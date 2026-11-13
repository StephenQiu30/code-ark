use crate::contracts::{ImageArtifactRef, ProbeResult, ProbeSpec, ToolboxError};
use crate::docker_cli::DockerRunner;
use crate::probes::{ProbeContext, valid_id};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};
use zeroize::Zeroizing;

fn err(code: &str) -> ToolboxError {
    ToolboxError::new(code)
}
const LOCK: &str = include_str!("../../probe-clients/artifacts.lock.json");
fn bundle(root: &Path, module: &str, control: &Path) -> Result<PathBuf, ToolboxError> {
    let lock: BTreeMap<String, BTreeMap<String, String>> =
        serde_json::from_str(LOCK).map_err(|_| err("ProbeArtifactLockInvalid"))?;
    let expected = lock
        .get(module)
        .filter(|v| v.contains_key("probe.jar"))
        .ok_or_else(|| err("ProbeClientArtifactMissing"))?;
    stage_bundle(&root.join(module), module, expected, control)
}
fn verify_bundle(dir: &Path, expected: &BTreeMap<String, String>) -> Result<PathBuf, ToolboxError> {
    if !expected.contains_key("probe.jar")
        || expected.iter().any(|(file, digest)| {
            let allowed_file = file == "probe.jar"
                || file.strip_prefix("lib/").is_some_and(|name| {
                    name.ends_with(".jar")
                        && name
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b".-_".contains(&b))
                });
            !allowed_file
                || digest.len() != 64
                || !digest
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        })
    {
        return Err(err("ProbeArtifactLockInvalid"));
    }
    crate::workspace::reject_links(dir)?;
    crate::workspace::reject_links(&dir.join("lib"))?;
    for (file, digest) in expected {
        let path = dir.join(file);
        crate::workspace::reject_links(&path)?;
        let mut reader = open_artifact(&path)?;
        let mut buffer = [0u8; 65536];
        let mut hash = Sha256::new();
        loop {
            let n = reader
                .read(&mut buffer)
                .map_err(|_| err("ProbeArtifactReadFailed"))?;
            if n == 0 {
                break;
            }
            hash.update(&buffer[..n]);
        }
        if format!("{:x}", hash.finalize()) != *digest {
            return Err(err("ProbeArtifactDigestMismatch"));
        }
    }
    // Java's wildcard classpath would also load an unlisted jar.
    for (subdir, root) in [("", dir), ("lib/", dir.join("lib").as_path())] {
        for item in fs::read_dir(root).map_err(|_| err("ProbeClientArtifactMissing"))? {
            let entry = item.map_err(|_| err("ProbeArtifactReadFailed"))?;
            let kind = entry
                .file_type()
                .map_err(|_| err("ProbeArtifactReadFailed"))?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| err("ProbeArtifactUnexpectedFile"))?;
            if subdir.is_empty() && name == "lib" {
                if !kind.is_dir() {
                    return Err(err("ProbeArtifactUnexpectedFile"));
                }
                continue;
            }
            if !kind.is_file() || !expected.contains_key(&format!("{subdir}{name}")) {
                return Err(err("ProbeArtifactUnexpectedFile"));
            }
        }
    }
    dir.canonicalize()
        .map_err(|_| err("ProbeClientArtifactMissing"))
}
fn open_artifact(path: &Path) -> Result<fs::File, ToolboxError> {
    crate::workspace::reject_links(path)?;
    if !fs::symlink_metadata(path)
        .map_err(|_| err("ProbeClientArtifactMissing"))?
        .is_file()
    {
        return Err(err("ProbeArtifactUnexpectedFile"));
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options
        .open(path)
        .map_err(|_| err("ProbeClientArtifactMissing"))?;
    if !file
        .metadata()
        .map_err(|_| err("ProbeArtifactReadFailed"))?
        .is_file()
    {
        return Err(err("ProbeArtifactUnexpectedFile"));
    }
    Ok(file)
}
fn stage_bundle(
    source: &Path,
    module: &str,
    expected: &BTreeMap<String, String>,
    control: &Path,
) -> Result<PathBuf, ToolboxError> {
    if module.is_empty()
        || module.len() > 32
        || !module
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(err("ProbeArtifactLockInvalid"));
    }
    // The packaged source remains authoritative even when a cached copy exists.
    verify_bundle(source, expected)?;
    let descriptor = serde_json::to_vec(expected).map_err(|_| err("ProbeArtifactLockInvalid"))?;
    let digest = format!("{:x}", Sha256::digest(descriptor));
    let root = control.join("probe-clients");
    let bundles = root.join("bundles");
    crate::workspace::private_dir(&bundles)?;
    let scope = format!("probe-bundle/{module}/{digest}");
    let started = std::time::Instant::now();
    let _lock = loop {
        match crate::locks::acquire(&root.join("bundle-locks"), std::slice::from_ref(&scope)) {
            Ok(lock) => break lock,
            Err(error)
                if error.code == "ResourceBusy" && started.elapsed() < Duration::from_secs(15) =>
            {
                std::thread::sleep(Duration::from_millis(20))
            }
            Err(error) => return Err(error),
        }
    };
    let source = verify_bundle(source, expected)?;
    let destination = bundles.join(format!("{module}-{digest}"));
    crate::workspace::reject_links(&destination)?;
    if destination.exists() {
        return verify_staged(&destination, expected);
    }
    let temporary = bundles.join(format!(
        ".staging-{module}-{}",
        crate::workspace::random_id()?
    ));
    crate::workspace::private_dir(&temporary)?;
    let result = (|| {
        crate::workspace::private_dir(&temporary.join("lib"))?;
        // Copy exactly the descriptor entries, never a recursive directory tree.
        for relative in expected.keys() {
            let mut input = open_artifact(&source.join(relative))?;
            let output = temporary.join(relative);
            let mut output_file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&output)
                .map_err(|_| err("ProbeArtifactReadFailed"))?;
            std::io::copy(&mut input, &mut output_file)
                .map_err(|_| err("ProbeArtifactReadFailed"))?;
            output_file
                .sync_all()
                .map_err(|_| err("ProbeArtifactReadFailed"))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&output, fs::Permissions::from_mode(0o644))
                    .map_err(|_| err("ProbeArtifactReadFailed"))?;
            }
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for directory in [&temporary, &temporary.join("lib")] {
                fs::set_permissions(directory, fs::Permissions::from_mode(0o755))
                    .map_err(|_| err("ProbeArtifactReadFailed"))?;
            }
        }
        verify_staged(&temporary, expected)?;
        verify_bundle(&source, expected)?;
        // Only publish a new content directory. Existing mounted directories are
        // verified and reused above; never delete, repair or replace them in place.
        fs::rename(&temporary, &destination).map_err(|_| err("ProbeArtifactReadFailed"))?;
        verify_staged(&destination, expected)
    })();
    if temporary.exists() && crate::workspace::reject_links(&temporary).is_ok() {
        let _ = fs::remove_dir_all(&temporary);
    }
    result
}
fn verify_staged(
    path: &Path,
    expected: &BTreeMap<String, String>,
) -> Result<PathBuf, ToolboxError> {
    let canonical = verify_bundle(path, expected)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for directory in [path.to_path_buf(), path.join("lib")] {
            if fs::metadata(directory)
                .map_err(|_| err("ProbeArtifactReadFailed"))?
                .permissions()
                .mode()
                & 0o777
                != 0o755
            {
                return Err(err("ProbeArtifactReadFailed"));
            }
        }
        for relative in expected.keys() {
            if fs::metadata(path.join(relative))
                .map_err(|_| err("ProbeArtifactReadFailed"))?
                .permissions()
                .mode()
                & 0o777
                != 0o644
            {
                return Err(err("ProbeArtifactReadFailed"));
            }
        }
    }
    Ok(canonical)
}

async fn locked_image(
    cx: &ProbeContext<'_>,
    image: &ImageArtifactRef,
) -> Result<String, ToolboxError> {
    crate::image_store::validate_image_ref(image, &cx.runner.target.architecture)?;
    if image.kind != "registry" {
        return Err(err("ProbeImageUnavailable"));
    }
    let reference = format!(
        "{}@{}",
        image
            .registry_ref
            .as_deref()
            .ok_or_else(|| err("ProbeImageUnavailable"))?,
        image
            .manifest_digest
            .as_deref()
            .ok_or_else(|| err("ProbeImageUnavailable"))?
    );
    let raw = cx
        .runner
        .run_read(&[
            "image",
            "inspect",
            "--format",
            "{{.Os}}/{{.Architecture}} {{.Id}}",
            &reference,
        ])
        .await
        .map_err(|_| err("ProbeImageUnavailable"))?;
    let (platform, id) = std::str::from_utf8(&raw)
        .ok()
        .map(str::trim)
        .and_then(|s| s.split_once(' '))
        .ok_or_else(|| err("ProbeImageUnavailable"))?;
    if platform != image.platform || !id.strip_prefix("sha256:").is_some_and(valid_id) {
        return Err(err("ProbeImageUnavailable"));
    }
    Ok(id.into())
}
const ROLE: &str = "protocol-probe";
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProbeOwner {
    schema_version: u32,
    owner: String,
    target_id: String,
    daemon_id: String,
    instance_id: String,
    component_id: String,
    network_container_id: String,
    module: String,
    status: String,
    code: Option<String>,
    cleanup_code: Option<String>,
    container_id: Option<String>,
    oom_killed: Option<bool>,
    exit_code: Option<i64>,
}
impl ProbeOwner {
    fn slot(&self) -> String {
        format!(
            "probe/{}/{}/{}",
            self.target_id, self.instance_id, self.component_id
        )
    }
    fn path(&self, root: &Path) -> PathBuf {
        root.join("active").join(format!("{}.json", self.owner))
    }
    fn save(&self, root: &Path) -> Result<(), ToolboxError> {
        crate::workspace::atomic_write(
            &self.path(root),
            &serde_json::to_vec(self).map_err(|_| err("ProbeDiagnosticUnavailable"))?,
        )
    }
    fn failure(&self, root: &Path) -> Result<(), ToolboxError> {
        // One bounded, safe summary per control directory; never persist CLI output.
        if self.code.is_some() {
            crate::workspace::atomic_write(
                &root.join("last-failure.json"),
                &serde_json::to_vec(self).map_err(|_| err("ProbeDiagnosticUnavailable"))?,
            )?;
        }
        Ok(())
    }
}
struct ClientState {
    id: String,
    running: bool,
    oom: bool,
    exit: i64,
    runtime_error: bool,
}
struct Cleanup {
    runner: DockerRunner,
    root: PathBuf,
    record: ProbeOwner,
    slot: Option<crate::locks::ResourceGuard>,
    active: bool,
}
impl Cleanup {
    async fn snapshot(&self) -> Result<Option<ClientState>, ToolboxError> {
        self.runner.verify_identity().await?;
        let name = format!("codeark-probe-{}", self.record.owner);
        const TEMPLATE: &str = r#"{"Id":{{json .Id}},"Labels":{{json .Config.Labels}},"Running":{{json .State.Running}},"OOMKilled":{{json .State.OOMKilled}},"ExitCode":{{json .State.ExitCode}},"RuntimeError":{{if .State.Error}}true{{else}}false{{end}}}"#;
        let raw = self
            .runner
            .run_read(&["container", "inspect", "--format", TEMPLATE, &name])
            .await;
        let raw = match raw {
            Ok(raw) => raw,
            Err(_) => {
                // A failed inspect alone never proves that cleanup completed.
                self.runner.verify_identity().await?;
                let owner = format!("label=io.codeark.probe-owner={}", self.record.owner);
                let role = format!("label=io.codeark.role={ROLE}");
                let target = format!("label=io.codeark.target-id={}", self.record.target_id);
                let daemon = format!("label=io.codeark.daemon-id={}", self.record.daemon_id);
                let ids = self
                    .runner
                    .run_read(&[
                        "ps",
                        "-aq",
                        "--no-trunc",
                        "--filter",
                        &owner,
                        "--filter",
                        &role,
                        "--filter",
                        &target,
                        "--filter",
                        &daemon,
                    ])
                    .await?;
                if ids.iter().all(u8::is_ascii_whitespace) {
                    return Ok(None);
                }
                return Err(err("ProbeClientStateUnavailable"));
            }
        };
        let value: Value =
            serde_json::from_slice(&raw).map_err(|_| err("ProbeClientStateUnavailable"))?;
        let labels = &value["Labels"];
        if labels["io.codeark.probe-owner"] != self.record.owner
            || labels["io.codeark.role"] != ROLE
            || labels["io.codeark.target-id"] != self.record.target_id
            || labels["io.codeark.daemon-id"] != self.record.daemon_id
        {
            return Err(err("ProbeClientOwnershipConflict"));
        }
        Ok(Some(ClientState {
            id: value["Id"]
                .as_str()
                .filter(|id| valid_id(id))
                .ok_or_else(|| err("ProbeClientStateUnavailable"))?
                .into(),
            running: value["Running"]
                .as_bool()
                .ok_or_else(|| err("ProbeClientStateUnavailable"))?,
            oom: value["OOMKilled"]
                .as_bool()
                .ok_or_else(|| err("ProbeClientStateUnavailable"))?,
            exit: value["ExitCode"]
                .as_i64()
                .ok_or_else(|| err("ProbeClientStateUnavailable"))?,
            runtime_error: value["RuntimeError"]
                .as_bool()
                .ok_or_else(|| err("ProbeClientStateUnavailable"))?,
        }))
    }
    async fn finish(&mut self, run_error: Option<&str>) -> Result<(), ToolboxError> {
        let observation = self.snapshot().await;
        let mut code = match &observation {
            Ok(Some(state)) => {
                self.record.container_id = Some(state.id.clone());
                self.record.oom_killed = Some(state.oom);
                self.record.exit_code = Some(state.exit);
                classify(run_error, Some(state))
            }
            Ok(None) => classify(run_error, None),
            Err(error) => Some(error.code.clone()),
        };
        self.record.status = "CleanupPending".into();
        self.record.code = code.clone();
        self.record.save(&self.root)?;
        let cleanup = match observation {
            Ok(Some(state)) => {
                let result = async {
                    self.runner.verify_identity().await?;
                    // Re-read all owner labels immediately before deleting the exact ID.
                    let current = self.snapshot().await?;
                    if let Some(current) = current {
                        if current.id != state.id {
                            return Err(err("ProbeClientOwnershipConflict"));
                        }
                        self.runner
                            .run_read(&["container", "rm", "-f", &current.id])
                            .await?;
                    }
                    Ok::<(), ToolboxError>(())
                }
                .await;
                if result.is_err() && matches!(self.snapshot().await, Ok(None)) {
                    Ok(())
                } else {
                    result
                }
            }
            Ok(None) => Ok(()),
            Err(error) => Err(error),
        };
        if let Err(error) = cleanup {
            self.record.cleanup_code = Some(error.code);
            code.get_or_insert_with(|| "ProbeClientCleanupPending".into());
            self.record.code = code.clone();
            self.record.save(&self.root)?;
        } else {
            self.record.status = "Cleaned".into();
            let path = self.record.path(&self.root);
            crate::workspace::reject_links(&path)?;
            fs::remove_file(path).map_err(|_| err("ProbeDiagnosticUnavailable"))?;
        }
        self.record.failure(&self.root)?;
        self.active = false;
        code.map_or(Ok(()), |code| Err(err(&code)))
    }
}
fn classify(run_error: Option<&str>, state: Option<&ClientState>) -> Option<String> {
    let code = if state.is_some_and(|state| state.oom) {
        "ProbeClientOomKilled"
    } else if run_error == Some("TimedOut") {
        "ProbeClientTimedOut"
    } else if let Some(
        code @ ("ProbeClientCancelled"
        | "ProbeClientOomKilled"
        | "ProbeClientTimedOut"
        | "ProbeClientExitFailed"
        | "ProbeClientLaunchFailed"
        | "ProbeClientCommandFailed"
        | "ProbeClientStateUnavailable"
        | "ProbeClientOwnershipConflict"
        | "ProbeClientCleanupPending"),
    ) = run_error
    {
        code
    } else if state.is_none() {
        "ProbeClientLaunchFailed"
    } else if state.is_some_and(|state| state.runtime_error) {
        "ProbeClientLaunchFailed"
    } else if state.is_some_and(|state| state.running) {
        "ProbeClientStateUnavailable"
    } else if state.is_some_and(|state| state.exit != 0) {
        "ProbeClientExitFailed"
    } else if run_error.is_some() {
        "ProbeClientCommandFailed"
    } else {
        return None;
    };
    Some(code.into())
}
fn nonce(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
async fn recover(
    cx: &ProbeContext<'_>,
    root: &Path,
    current_slot: &str,
) -> Result<(), ToolboxError> {
    for entry in fs::read_dir(root.join("active")).map_err(|_| err("ProbeDiagnosticUnavailable"))? {
        let path = entry.map_err(|_| err("ProbeDiagnosticUnavailable"))?.path();
        let Some(owner) = path
            .file_stem()
            .and_then(|name| name.to_str())
            .filter(|owner| nonce(owner))
        else {
            return Err(err("ProbeClientCleanupPending"));
        };
        let record: ProbeOwner = crate::workspace::read_json(&path)?;
        if record.target_id != cx.runner.target.target_id
            || record.daemon_id != cx.runner.target.daemon_id
        {
            continue;
        }
        if record.schema_version != 1
            || record.owner != owner
            || !valid_id(&record.network_container_id)
            || !["Running", "CleanupPending"].contains(&record.status.as_str())
        {
            return Err(err("ProbeClientCleanupPending"));
        }
        let slot_key = record.slot();
        let slot = match crate::locks::acquire(&root.join("locks"), &[slot_key.clone()]) {
            Ok(slot) => slot,
            Err(error) if error.code == "ResourceBusy" && slot_key != current_slot => continue,
            Err(_) => return Err(err("ProbeClientBusy")),
        };
        if !path.exists() {
            continue;
        }
        let mut cleanup = Cleanup {
            runner: DockerRunner {
                target: cx.runner.target.clone(),
            },
            root: root.into(),
            record,
            slot: Some(slot),
            active: false,
        };
        let previous_code = cleanup
            .record
            .code
            .clone()
            .unwrap_or_else(|| "ProbeClientCancelled".into());
        let _ = tokio::time::timeout(
            Duration::from_secs(60),
            cleanup.finish(Some(&previous_code)),
        )
        .await;
        if path.exists() {
            return Err(err("ProbeClientCleanupPending"));
        }
    }
    Ok(())
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        self.record.status = "CleanupPending".into();
        self.record.code = Some("ProbeClientCancelled".into());
        let _ = self.record.save(&self.root);
        let _ = self.record.failure(&self.root);
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let mut cleanup = Cleanup {
                runner: DockerRunner {
                    target: self.runner.target.clone(),
                },
                root: self.root.clone(),
                record: self.record.clone(),
                slot: self.slot.take(),
                active: false,
            };
            handle.spawn(async move {
                let _ = tokio::time::timeout(
                    Duration::from_secs(60),
                    cleanup.finish(Some("ProbeClientCancelled")),
                )
                .await;
            });
        }
    }
}
async fn run(
    cx: &ProbeContext<'_>,
    id: &str,
    component: &str,
    module: &str,
    args: Vec<OsString>,
    stdin: &[u8],
) -> Result<Vec<u8>, ToolboxError> {
    if !valid_id(id) {
        return Err(err("ComponentIdentityUnverified"));
    }
    let root = cx.control_root.join("probe-clients");
    crate::workspace::private_dir(&root)?;
    crate::workspace::private_dir(&root.join("active"))?;
    let owner = crate::workspace::random_id()?;
    let record = ProbeOwner {
        schema_version: 1,
        owner: owner.clone(),
        target_id: cx.runner.target.target_id.clone(),
        daemon_id: cx.runner.target.daemon_id.clone(),
        instance_id: cx.instance.instance_id.clone(),
        component_id: component.into(),
        network_container_id: id.into(),
        module: module.into(),
        status: "Running".into(),
        code: None,
        cleanup_code: None,
        container_id: None,
        oom_killed: None,
        exit_code: None,
    };
    recover(cx, &root, &record.slot()).await?;
    let slot = crate::locks::acquire(&root.join("locks"), &[record.slot()])
        .map_err(|_| err("ProbeClientBusy"))?;
    record.save(&root)?;
    let mut guard = Cleanup {
        runner: DockerRunner {
            target: cx.runner.target.clone(),
        },
        root,
        record,
        slot: Some(slot),
        active: true,
    };
    let mut parameters: Vec<OsString> = [
        "run",
        "--name",
        &format!("codeark-probe-{owner}"),
        "--pull=never",
        "--interactive",
        "--read-only",
        "--cap-drop=ALL",
        "--security-opt=no-new-privileges",
        "--pids-limit=96",
        "--memory=256m",
        "--tmpfs",
        "/tmp:rw,nosuid,nodev,size=32m",
        "--network",
        &format!("container:{id}"),
        "--label",
        &format!("io.codeark.probe-owner={owner}"),
        "--label",
        &format!("io.codeark.role={ROLE}"),
        "--label",
        &format!("io.codeark.target-id={}", cx.runner.target.target_id),
        "--label",
        &format!("io.codeark.daemon-id={}", cx.runner.target.daemon_id),
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    parameters.extend(args);
    let result = cx
        .runner
        .run_probe_input(&parameters, stdin, Duration::from_secs(20))
        .await;
    let run_error = result.as_ref().err().map(|error| error.code.as_str());
    guard.finish(run_error).await?;
    result.map_err(|_| err("ProbeClientCommandFailed"))
}
fn bind_mount(dir: &Path) -> Result<String, ToolboxError> {
    let path = dir
        .to_str()
        .filter(|s| !s.contains(['\r', '\n']))
        .ok_or_else(|| err("UnsupportedResourcePath"))?;
    let field = format!("source={}", path.replace('"', "\"\""));
    Ok(format!(
        "type=bind,\"{field}\",target=/opt/codeark,readonly"
    ))
}
pub(crate) async fn java(
    cx: &ProbeContext<'_>,
    probe: &ProbeSpec,
    component_id: &str,
    module: &'static str,
    mut input: Value,
) -> Result<(), ToolboxError> {
    let image = cx
        .tool_image
        .filter(|i| i.source_id == "probe-jre17")
        .ok_or_else(|| err("ProbeImageUnavailable"))?;
    let image = locked_image(cx, image).await?;
    let root = cx.resources_root.to_owned();
    let control = cx.control_root.to_owned();
    let dir = tokio::task::spawn_blocking(move || bundle(&root, module, &control))
        .await
        .map_err(|_| err("ProbeArtifactReadFailed"))??;
    let mount = bind_mount(&dir)?;
    let args: Vec<OsString> = [
        "--mount",
        &mount,
        "--user",
        "65534:65534",
        "--env",
        "HOME=/tmp",
        "--workdir",
        "/tmp",
        "--entrypoint",
        "java",
        &image,
        "-Xms16m",
        "-Xmx128m",
        "-XX:ActiveProcessorCount=2",
        "-Duser.home=/tmp",
        "-Djava.io.tmpdir=/tmp",
        "-cp",
        "/opt/codeark/probe.jar:/opt/codeark/lib/*",
        "io.codeark.ProbeMain",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    input["probeId"] = probe.probe_id.clone().into();
    input["componentId"] = probe.component_id.clone().into();
    let bytes = Zeroizing::new(serde_json::to_vec(&input).map_err(|_| err("ProbeInputInvalid"))?);
    if bytes.len() > 16384 {
        return Err(err("ProbeInputTooLarge"));
    }
    let raw = run(cx, component_id, &probe.component_id, module, args, &bytes).await?;
    if raw.len() > 2048 {
        return Err(err("ProbeResponseTooLarge"));
    }
    let r: ProbeResult = serde_json::from_slice(&raw).map_err(|_| err("ProbeResponseInvalid"))?;
    if r.probe_id != probe.probe_id || r.component_id != probe.component_id || r.sampled_at == 0 {
        return Err(err("ProbeResponseInvalid"));
    }
    if !r.success && r.error_code.as_deref() == Some("ProbeClientIncompatible") {
        return Err(err("ProbeClientIncompatible"));
    }
    if !r.success || r.error_code.is_some() || r.warning_code.is_some() {
        return Err(err("ProtocolUnavailable"));
    }
    Ok(())
}

pub(crate) async fn temporal(
    cx: &ProbeContext<'_>,
    id: &str,
    namespace: &str,
) -> Result<(), ToolboxError> {
    let image = cx
        .effective
        .image_artifacts
        .iter()
        .find(|i| i.source_id == "temporal-temporal-namespace-setup")
        .ok_or_else(|| err("ProbeImageUnavailable"))?;
    let image = locked_image(cx, image).await?;
    for (index, command) in [
        vec!["operator", "cluster", "health"],
        vec![
            "operator",
            "namespace",
            "describe",
            "--namespace",
            namespace,
        ],
    ]
    .into_iter()
    .enumerate()
    {
        let mut args: Vec<OsString> = ["--entrypoint", "temporal", &image]
            .into_iter()
            .map(Into::into)
            .collect();
        args.extend(command.into_iter().map(OsString::from));
        args.extend(
            [
                "--address",
                "127.0.0.1:7233",
                "--command-timeout",
                "5s",
                "--output",
                "json",
            ]
            .into_iter()
            .map(OsString::from),
        );
        let raw = run(cx, id, "temporal", "temporal", args, &[]).await?;
        let value: Value = serde_json::from_slice(&raw).map_err(|_| err("ProbeResponseInvalid"))?;
        let ok = if index == 0 {
            value["status"] == "SERVING"
        } else {
            value["namespaceInfo"]["name"] == namespace
                && value["namespaceInfo"]["id"]
                    .as_str()
                    .is_some_and(|s| !s.is_empty())
        };
        if !ok {
            return Err(err("ProtocolUnavailable"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn staged_fixture(root: &Path) -> (PathBuf, PathBuf, BTreeMap<String, String>) {
        let source = root.join("packaged/canal");
        fs::create_dir_all(source.join("lib")).unwrap();
        let mut expected = BTreeMap::new();
        for relative in ["probe.jar", "lib/driver.jar"] {
            fs::write(source.join(relative), b"reviewed public jar bytes").unwrap();
            expected.insert(
                relative.into(),
                format!("{:x}", Sha256::digest(b"reviewed public jar bytes")),
            );
        }
        (source, root.join("control"), expected)
    }
    #[test]
    fn immutable_bundle_reuses_its_path_and_new_descriptors_never_replace_old_mounts() {
        let temporary = tempfile::tempdir().unwrap();
        let (source, control, mut expected) = staged_fixture(temporary.path());
        let first = stage_bundle(&source, "canal", &expected, &control).unwrap();
        assert!(
            first.starts_with(
                control
                    .canonicalize()
                    .unwrap()
                    .join("probe-clients/bundles")
            )
        );
        assert_ne!(first, source.canonicalize().unwrap());
        #[cfg(unix)]
        let inode = {
            use std::os::unix::fs::MetadataExt;
            fs::metadata(&first).unwrap().ino()
        };
        // Simulate resource preparation replacing the source tree with identical artifacts.
        fs::remove_dir_all(&source).unwrap();
        let (_, _, same_descriptor) = staged_fixture(temporary.path());
        assert_eq!(expected, same_descriptor);
        assert_eq!(
            stage_bundle(&source, "canal", &expected, &control).unwrap(),
            first
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::{MetadataExt, PermissionsExt};
            assert_eq!(fs::metadata(&first).unwrap().ino(), inode);
            for directory in [&first, &first.join("lib")] {
                assert_eq!(
                    fs::metadata(directory).unwrap().permissions().mode() & 0o777,
                    0o755
                );
            }
            for relative in expected.keys() {
                assert_eq!(
                    fs::metadata(first.join(relative))
                        .unwrap()
                        .permissions()
                        .mode()
                        & 0o777,
                    0o644
                );
            }
        }
        fs::write(source.join("probe.jar"), b"new reviewed probe bytes").unwrap();
        expected.insert(
            "probe.jar".into(),
            format!("{:x}", Sha256::digest(b"new reviewed probe bytes")),
        );
        let second = stage_bundle(&source, "canal", &expected, &control).unwrap();
        assert_ne!(first, second);
        assert_eq!(
            fs::read(first.join("probe.jar")).unwrap(),
            b"reviewed public jar bytes"
        );
        assert_eq!(
            fs::read(second.join("probe.jar")).unwrap(),
            b"new reviewed probe bytes"
        );
    }
    #[test]
    fn staging_rejects_modified_sources_caches_extra_jars_and_incomplete_publication() {
        let temporary = tempfile::tempdir().unwrap();
        let (source, control, expected) = staged_fixture(temporary.path());
        let destination = stage_bundle(&source, "canal", &expected, &control).unwrap();
        fs::write(source.join("probe.jar"), b"source changed").unwrap();
        assert_eq!(
            stage_bundle(&source, "canal", &expected, &control)
                .unwrap_err()
                .code,
            "ProbeArtifactDigestMismatch"
        );
        fs::write(source.join("probe.jar"), b"reviewed public jar bytes").unwrap();
        fs::write(source.join("lib/unlisted.jar"), b"unlisted").unwrap();
        assert_eq!(
            stage_bundle(&source, "canal", &expected, &control)
                .unwrap_err()
                .code,
            "ProbeArtifactUnexpectedFile"
        );
        fs::remove_file(source.join("lib/unlisted.jar")).unwrap();
        fs::write(destination.join("lib/unlisted.jar"), b"unlisted").unwrap();
        assert_eq!(
            stage_bundle(&source, "canal", &expected, &control)
                .unwrap_err()
                .code,
            "ProbeArtifactUnexpectedFile"
        );
        fs::remove_file(destination.join("lib/unlisted.jar")).unwrap();
        fs::write(destination.join("probe.jar"), b"cache changed").unwrap();
        assert_eq!(
            stage_bundle(&source, "canal", &expected, &control)
                .unwrap_err()
                .code,
            "ProbeArtifactDigestMismatch"
        );
        assert_eq!(
            fs::read(destination.join("probe.jar")).unwrap(),
            b"cache changed",
            "Never repair a possibly mounted cache directory in place"
        );
        fs::remove_file(destination.join("probe.jar")).unwrap();
        assert!(stage_bundle(&source, "canal", &expected, &control).is_err());
        assert!(!destination.join("probe.jar").exists());
        let mut traversal = expected;
        traversal.insert("lib/../../outside.jar".into(), "a".repeat(64));
        assert_eq!(
            stage_bundle(&source, "canal", &traversal, &control)
                .unwrap_err()
                .code,
            "ProbeArtifactLockInvalid"
        );
    }
    #[test]
    fn concurrent_stagers_publish_one_verified_directory() {
        let temporary = tempfile::tempdir().unwrap();
        let (source, control, expected) = staged_fixture(temporary.path());
        let workers = (0..4)
            .map(|_| {
                let (source, control, expected) =
                    (source.clone(), control.clone(), expected.clone());
                std::thread::spawn(move || {
                    stage_bundle(&source, "canal", &expected, &control).unwrap()
                })
            })
            .collect::<Vec<_>>();
        let paths = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        assert!(paths.iter().all(|path| path == &paths[0]));
        assert_eq!(
            fs::read_dir(control.join("probe-clients/bundles"))
                .unwrap()
                .count(),
            1
        );
    }
    #[cfg(unix)]
    #[test]
    fn staged_and_packaged_artifact_links_are_never_followed() {
        use std::os::unix::fs::symlink;
        let temporary = tempfile::tempdir().unwrap();
        let (source, control, expected) = staged_fixture(temporary.path());
        let destination = stage_bundle(&source, "canal", &expected, &control).unwrap();
        fs::remove_file(source.join("probe.jar")).unwrap();
        symlink(destination.join("probe.jar"), source.join("probe.jar")).unwrap();
        assert!(stage_bundle(&source, "canal", &expected, &control).is_err());
        fs::remove_file(source.join("probe.jar")).unwrap();
        fs::write(source.join("probe.jar"), b"reviewed public jar bytes").unwrap();
        fs::remove_file(destination.join("lib/driver.jar")).unwrap();
        symlink(
            source.join("lib/driver.jar"),
            destination.join("lib/driver.jar"),
        )
        .unwrap();
        assert!(stage_bundle(&source, "canal", &expected, &control).is_err());
        fs::remove_file(destination.join("lib/driver.jar")).unwrap();
        fs::write(
            destination.join("lib/driver.jar"),
            b"reviewed public jar bytes",
        )
        .unwrap();
        let moved = temporary.path().join("moved-cache");
        fs::rename(&destination, &moved).unwrap();
        symlink(&moved, &destination).unwrap();
        assert!(stage_bundle(&source, "canal", &expected, &control).is_err());
    }
    #[test]
    fn client_exit_codes_do_not_invent_oom_evidence() {
        let mut state = ClientState {
            id: "c".repeat(64),
            running: false,
            oom: false,
            exit: 137,
            runtime_error: false,
        };
        assert_eq!(
            classify(Some("DockerCommandFailed"), Some(&state)).as_deref(),
            Some("ProbeClientExitFailed")
        );
        state.oom = true;
        assert_eq!(
            classify(Some("DockerCommandFailed"), Some(&state)).as_deref(),
            Some("ProbeClientOomKilled")
        );
        state.oom = false;
        assert_eq!(
            classify(Some("TimedOut"), Some(&state)).as_deref(),
            Some("ProbeClientTimedOut")
        );
        assert_eq!(
            classify(Some("DockerCommandFailed"), None).as_deref(),
            Some("ProbeClientLaunchFailed")
        );
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn owned_client_lifecycle_records_only_safe_state_and_recovers_after_cancellation() {
        use crate::contracts::*;
        let root = tempfile::tempdir().unwrap();
        let program = root.path().join("fake-docker");
        fs::write(&program, r#"#!/usr/bin/env python3
import sys,json,time
from pathlib import Path
root=Path(__file__).parent
a=sys.argv[3:]
state=root/'client.json'
mode=(root/'mode').read_text()
if a[0]=='info': print('{"ID":"daemon","OSType":"linux"}')
elif a[:2]==['context','inspect']: print('[{"Endpoints":{"docker":{"Host":"unix:///fake.sock"}}}]')
elif a[0]=='run':
 assert '--rm' not in a and '--memory=256m' in a
 payload=sys.stdin.read()
 assert 'private-password' in payload
 assert not any('private-password' in value for value in a)
 if mode=='launch': sys.exit(125)
 labels=dict(a[i+1].split('=',1) for i,v in enumerate(a) if v=='--label')
 value={'Id':'c'*64,'Labels':labels,'Running':mode=='hang','OOMKilled':mode=='oom','ExitCode':137 if mode in ['oom','exit137'] else 0,'RuntimeError':False}
 state.write_text(json.dumps(value))
 if mode=='hang': time.sleep(120)
 if mode in ['oom','exit137']:
  print('private-password header.payload.signature',file=sys.stderr)
  sys.exit(137)
 print('{"safe":true}')
elif a[:2]==['container','inspect']:
 if not state.exists(): sys.exit(1)
 value=json.loads(state.read_text())
 if mode=='forged': value['Labels']['io.codeark.probe-owner']='different-owner'
 if mode=='unknown': del value['OOMKilled']
 print(json.dumps(value))
elif a[0]=='ps':
 if state.exists(): print('c'*64)
elif a[:3]==['container','rm','-f']:
 if mode=='cleanup-fail': sys.exit(1)
 if mode=='hang': time.sleep(.5)
 assert a[3]=='c'*64
 state.unlink()
 (root/'removed').write_text('yes')
else: sys.exit(1)
"#).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&program, fs::Permissions::from_mode(0o700)).unwrap();
        let runner = DockerRunner {
            target: DockerTarget {
                target_id: "target".into(),
                daemon_id: "daemon".into(),
                cli_path: program.to_str().unwrap().into(),
                context_name: "fake".into(),
                endpoint_kind: "unix".into(),
                os_type: "linux".into(),
                architecture: "arm64".into(),
            },
        };
        let instance = InstanceIdentity {
            workspace_id: "workspace".into(),
            service_id: "canal".into(),
            instance_id: "b".repeat(32),
            project_name: "project".into(),
            target_id: "target".into(),
            daemon_id: "daemon".into(),
        };
        let effective = EffectiveConfig {
            generation_id: "a".repeat(32),
            instance_id: instance.instance_id.clone(),
            target_id: "target".into(),
            ports: vec![],
            endpoints: vec![],
            secret_refs: vec![],
            config_digest: String::new(),
            image_artifacts: vec![],
        };
        let service = crate::catalog::load_embedded_catalog()
            .unwrap()
            .services
            .into_iter()
            .find(|service| service.id == "canal")
            .unwrap();
        let secrets = crate::env_store::SecretValues::default();
        let control = root.path().join("control");
        let cx = ProbeContext {
            control_root: &control,
            runner: &runner,
            instance: &instance,
            effective: &effective,
            service: &service,
            secrets: &secrets,
            bindings: &[],
            tool_image: None,
            resources_root: root.path(),
        };
        let id = "d".repeat(64);
        let input = b"private-password header.payload.signature";
        for (mode, expected) in [
            ("success", None),
            ("oom", Some("ProbeClientOomKilled")),
            ("exit137", Some("ProbeClientExitFailed")),
            ("launch", Some("ProbeClientLaunchFailed")),
            ("forged", Some("ProbeClientOwnershipConflict")),
            ("unknown", Some("ProbeClientStateUnavailable")),
            ("cleanup-fail", Some("ProbeClientCleanupPending")),
        ] {
            fs::write(root.path().join("mode"), mode).unwrap();
            let result = run(
                &cx,
                &id,
                "canal-server",
                "canal",
                vec!["fake-image".into()],
                input,
            )
            .await;
            if let Some(code) = expected {
                assert_eq!(result.unwrap_err().code, code, "{mode}");
                let summary =
                    fs::read_to_string(control.join("probe-clients/last-failure.json")).unwrap();
                assert!(
                    !summary.contains("private-password")
                        && !summary.contains("header.payload.signature")
                );
                let summary: Value = serde_json::from_str(&summary).unwrap();
                assert_eq!(summary["code"], code);
                if mode == "exit137" {
                    assert_eq!(summary["oomKilled"], false);
                    assert_eq!(summary["exitCode"], 137);
                }
            } else {
                assert!(result.is_ok());
            }
            if ["forged", "unknown", "cleanup-fail"].contains(&mode) {
                assert!(
                    root.path().join("client.json").exists(),
                    "Unproven state must not be deleted"
                );
                fs::write(root.path().join("mode"), "success").unwrap();
                assert!(
                    run(
                        &cx,
                        &id,
                        "canal-server",
                        "canal",
                        vec!["fake-image".into()],
                        input
                    )
                    .await
                    .is_ok()
                );
            }
            assert!(!root.path().join("client.json").exists());
            assert_eq!(
                fs::read_dir(control.join("probe-clients/active"))
                    .unwrap()
                    .count(),
                0
            );
        }
        fs::write(root.path().join("mode"), "hang").unwrap();
        let mut future = Box::pin(run(
            &cx,
            &id,
            "canal-server",
            "canal",
            vec!["fake-image".into()],
            input,
        ));
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                tokio::select! {
                    result = &mut future => panic!("client unexpectedly completed: {}", result.is_ok()),
                    _ = tokio::time::sleep(Duration::from_millis(10)) => if root.path().join("client.json").exists() { break; }
                }
            }
        }).await.unwrap();
        drop(future);
        assert_eq!(
            run(
                &cx,
                &id,
                "canal-server",
                "canal",
                vec!["fake-image".into()],
                input
            )
            .await
            .unwrap_err()
            .code,
            "ProbeClientBusy"
        );
        tokio::time::timeout(Duration::from_secs(10), async {
            while fs::read_dir(control.join("probe-clients/active"))
                .unwrap()
                .count()
                != 0
            {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        assert!(!root.path().join("client.json").exists());
        let summary: Value =
            crate::workspace::read_json(&control.join("probe-clients/last-failure.json")).unwrap();
        assert_eq!(summary["code"], "ProbeClientCancelled");
    }
    #[test]
    fn mount_escapes_csv_metacharacters_in_resource_paths() {
        let p = Path::new("/用户/SDK, \"test\"/bundle");
        assert_eq!(
            bind_mount(p).unwrap(),
            r#"type=bind,"source=/用户/SDK, ""test""/bundle",target=/opt/codeark,readonly"#
        );
    }
    #[test]
    fn every_jar_and_the_whole_classpath_must_match() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir(root.join("lib")).unwrap();
        let mut expected = BTreeMap::new();
        for f in ["probe.jar", "lib/driver.jar"] {
            fs::write(root.join(f), b"fixture jar bytes").unwrap();
            expected.insert(
                f.into(),
                format!("{:x}", Sha256::digest(b"fixture jar bytes")),
            );
        }
        assert!(verify_bundle(root, &expected).is_ok());
        fs::write(root.join("lib/other.jar"), b"extra").unwrap();
        assert_eq!(
            verify_bundle(root, &expected).unwrap_err().code,
            "ProbeArtifactUnexpectedFile"
        );
        fs::remove_file(root.join("lib/other.jar")).unwrap();
        fs::write(root.join("lib/driver.jar"), b"changed").unwrap();
        assert_eq!(
            verify_bundle(root, &expected).unwrap_err().code,
            "ProbeArtifactDigestMismatch"
        );
    }
}
