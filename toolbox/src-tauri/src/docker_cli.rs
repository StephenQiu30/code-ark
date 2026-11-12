use crate::blueprint::OverrideArtifact;
use crate::contracts::{DockerTarget, InstanceIdentity, ToolboxError};
use crate::workspace::{error, reject_links};
use sha2::{Digest, Sha256};
use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Command,
};

pub struct DockerRunner {
    pub target: DockerTarget,
}
fn command(cli: &Path, args: &[OsString]) -> Command {
    let mut cmd = Command::new(cli);
    cmd.args(args).stdin(Stdio::null()).kill_on_drop(true);
    cmd.env_clear();
    for (key, value) in std::env::vars_os() {
        let name = key.to_string_lossy().to_ascii_uppercase();
        if matches!(
            name.as_str(),
            "HOME"
                | "PATH"
                | "DOCKER_CONFIG"
                | "SSH_AUTH_SOCK"
                | "XDG_RUNTIME_DIR"
                | "DBUS_SESSION_BUS_ADDRESS"
                | "LANG"
                | "LC_ALL"
                | "TMPDIR"
                | "TMP"
                | "TEMP"
                | "USERPROFILE"
                | "APPDATA"
                | "LOCALAPPDATA"
                | "SYSTEMROOT"
                | "WINDIR"
                | "PATHEXT"
        ) {
            cmd.env(key, value);
        }
    }
    for key in [
        "DOCKER_HOST",
        "DOCKER_CONTEXT",
        "DOCKER_DEFAULT_PLATFORM",
        "COMPOSE_FILE",
        "COMPOSE_PROJECT_NAME",
        "COMPOSE_ENV_FILES",
    ] {
        cmd.env_remove(key);
    }
    #[cfg(unix)]
    cmd.process_group(0);
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000);
    }
    cmd
}
async fn read_bounded(reader: impl AsyncRead + Unpin) -> Result<Vec<u8>, ToolboxError> {
    let mut bytes = Vec::new();
    reader
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| error("DockerOutputUnavailable"))?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err(error("DockerOutputTooLarge"));
    }
    Ok(bytes)
}
// Only an ID spawned into our fresh Unix process group is ever signalled.
// Windows currently retains leader kill_on_drop; JobObject tree cleanup is
// pending native verification and must not be advertised as equivalent.
struct OwnedProcessGroup {
    #[cfg(unix)]
    id: Option<i32>,
}
impl OwnedProcessGroup {
    fn new(_child: &tokio::process::Child) -> Self {
        Self {
            #[cfg(unix)]
            id: _child
                .id()
                .and_then(|id| i32::try_from(id).ok())
                .filter(|id| *id > 0),
        }
    }
    fn disarm(&mut self) {
        #[cfg(unix)]
        {
            self.id = None;
        }
    }
    fn terminate(&mut self) {
        #[cfg(unix)]
        if let Some(id) = self.id.take() {
            // command() sets process_group(0), so this is precisely our child
            // and its descendants, never the caller's or a global Docker group.
            unsafe {
                libc::kill(-id, libc::SIGKILL);
            }
        }
    }
}
impl Drop for OwnedProcessGroup {
    fn drop(&mut self) {
        self.terminate();
    }
}
async fn invoke_cli_capture(
    cli: &Path,
    args: &[OsString],
    input: Option<&[u8]>,
    deadline: Duration,
    include_stderr: bool,
) -> Result<Vec<u8>, ToolboxError> {
    if input.is_some_and(|input| input.len() > 128 * 1024) {
        return Err(error("InputTooLarge"));
    }
    let mut child = command(cli, args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| error("DockerMissing"))?;
    // This guard is declared after child, so cancellation/drop kills the group
    // while the leader is still owned and has not been reaped by Child::drop.
    let mut group = OwnedProcessGroup::new(&child);
    let stdin = child.stdin.take();
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| error("DockerOutputUnavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| error("DockerOutputUnavailable"))?;
    let result = tokio::time::timeout(deadline, async {
        let write = async {
            if let Some(input) = input {
                let mut stdin = stdin.ok_or_else(|| error("DockerInputUnavailable"))?;
                stdin
                    .write_all(input)
                    .await
                    .map_err(|_| error("DockerInputUnavailable"))?;
                stdin
                    .shutdown()
                    .await
                    .map_err(|_| error("DockerInputUnavailable"))?;
            }
            Ok(())
        };
        let (_, mut output, errors) =
            tokio::try_join!(write, read_bounded(stdout), read_bounded(stderr))?;
        let status = child
            .wait()
            .await
            .map_err(|_| error("DockerCommandFailed"))?;
        // No pending process work remains after normal completion. Disarm
        // immediately after reaping, before the PID could ever be reused.
        group.disarm();
        if !status.success() {
            return Err(error("DockerCommandFailed"));
        }
        if include_stderr {
            let separator =
                usize::from(!output.is_empty() && !errors.is_empty() && !output.ends_with(b"\n"));
            if output.len() + errors.len() + separator > 2 * 1024 * 1024 {
                return Err(error("DockerOutputTooLarge"));
            }
            if separator != 0 {
                output.push(b'\n');
            }
            output.extend_from_slice(&errors);
        }
        Ok(output)
    })
    .await;
    match result {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(error)) => {
            group.terminate();
            let _ = child.kill().await;
            Err(error)
        }
        Err(_) => {
            group.terminate();
            let _ = child.kill().await;
            Err(error("TimedOut"))
        }
    }
}
pub(crate) async fn invoke_cli(
    cli: &Path,
    args: &[OsString],
    deadline: Duration,
) -> Result<Vec<u8>, ToolboxError> {
    invoke_cli_capture(cli, args, None, deadline, false).await
}
fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}
pub fn compose_supported(version: &str) -> bool {
    let text = version
        .trim()
        .trim_start_matches('v')
        .split('+')
        .next()
        .unwrap_or("");
    let (core, suffix) = text.split_once('-').unwrap_or((text, ""));
    if !suffix.is_empty() && !suffix.starts_with("desktop.") {
        return false;
    }
    let pieces = core
        .split('.')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>();
    matches!(pieces,Ok(v) if v.len()==3&&(v[0],v[1],v[2])>=(2,24,4))
}
pub fn normalize_architecture(value: &str) -> Result<&'static str, ToolboxError> {
    match value {
        "amd64" | "x86_64" => Ok("amd64"),
        "arm64" | "aarch64" | "arm64/v8" => Ok("arm64"),
        _ => Err(error("UnsupportedArchitecture")),
    }
}
pub fn validate_target(target: &DockerTarget) -> Result<(), ToolboxError> {
    if target.os_type != "linux"
        || !matches!(target.endpoint_kind.as_str(), "unix" | "npipe")
        || target.daemon_id.is_empty()
        || !Path::new(&target.cli_path).is_absolute()
    {
        return Err(error("UnsupportedTarget"));
    }
    normalize_architecture(&target.architecture)?;
    Ok(())
}
fn candidates(hint: Option<&Path>) -> Vec<PathBuf> {
    let mut paths = vec![];
    if let Some(path) = hint {
        paths.push(path.to_path_buf());
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            paths.push(dir.join(if cfg!(windows) {
                "docker.exe"
            } else {
                "docker"
            }));
        }
    }
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".docker/bin/docker"));
        paths.push(home.join(".orbstack/bin/docker"));
        paths.push(home.join(".rd/bin/docker"));
    }
    for path in [
        "/usr/local/bin/docker",
        "/opt/homebrew/bin/docker",
        "/usr/bin/docker",
        "/Applications/Docker.app/Contents/Resources/bin/docker",
        "/Applications/OrbStack.app/Contents/MacOS/bin/docker",
    ] {
        paths.push(PathBuf::from(path));
    }
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        paths.push(PathBuf::from(program_files).join("Docker/Docker/resources/bin/docker.exe"));
    }
    paths
}
pub async fn discover_target(
    hint: Option<&Path>,
    context: Option<&str>,
) -> Result<DockerTarget, ToolboxError> {
    let cli = candidates(hint)
        .into_iter()
        .find(|p| p.is_file())
        .and_then(|p| p.canonicalize().ok())
        .ok_or_else(|| error("DockerMissing"))?;
    let context = if let Some(name) = context {
        name.to_owned()
    } else {
        String::from_utf8(
            invoke_cli(&cli, &args(&["context", "show"]), Duration::from_secs(10)).await?,
        )
        .map_err(|_| error("DockerOutputInvalid"))?
        .trim()
        .to_owned()
    };
    if context.is_empty() || context.len() > 128 {
        return Err(error("InvalidDockerContext"));
    }
    let data = invoke_cli(
        &cli,
        &args(&["context", "inspect", &context]),
        Duration::from_secs(10),
    )
    .await?;
    let inspected: serde_json::Value =
        serde_json::from_slice(&data).map_err(|_| error("DockerOutputInvalid"))?;
    let endpoint = inspected[0]["Endpoints"]["docker"]["Host"]
        .as_str()
        .ok_or_else(|| error("UnsupportedTarget"))?;
    let endpoint_kind = if endpoint.starts_with("unix://") {
        "unix"
    } else if endpoint.starts_with("npipe://") {
        "npipe"
    } else {
        return Err(error("UnsupportedTarget"));
    };
    let data = invoke_cli(
        &cli,
        &args(&["--context", &context, "info", "--format", "{{json .}}"]),
        Duration::from_secs(30),
    )
    .await
    .map_err(|_| error("DaemonUnavailable"))?;
    let info: serde_json::Value =
        serde_json::from_slice(&data).map_err(|_| error("DockerOutputInvalid"))?;
    let daemon_id = info["ID"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| error("DaemonUnavailable"))?
        .to_owned();
    let target = DockerTarget {
        target_id: format!(
            "{:x}",
            Sha256::digest(format!("{}|{context}|{daemon_id}", cli.to_string_lossy()).as_bytes())
        ),
        cli_path: cli
            .to_str()
            .ok_or_else(|| error("UnsupportedPathEncoding"))?
            .into(),
        context_name: context,
        endpoint_kind: endpoint_kind.into(),
        daemon_id,
        os_type: info["OSType"].as_str().unwrap_or("").into(),
        architecture: info["Architecture"].as_str().unwrap_or("").into(),
    };
    validate_target(&target)?;
    let runner = DockerRunner {
        target: target.clone(),
    };
    let version = runner.run_read(&["compose", "version", "--short"]).await?;
    if !compose_supported(&String::from_utf8_lossy(&version)) {
        return Err(error("ComposeTooOld"));
    }
    // Probe the actual YAML merge capability without creating any Docker resource.
    let temporary = std::env::temp_dir().join(format!(
        "codeark-compose-probe-{}",
        crate::workspace::random_id()?
    ));
    crate::workspace::private_dir(&temporary)?;
    let config = temporary.join("capability.yml");
    crate::workspace::atomic_write(
        &config,
        b"services:\n  probe:\n    image: scratch\n    ports: !override []\n",
    )?;
    let capability = runner
        .run_args(
            &[
                OsString::from("compose"),
                OsString::from("--env-file"),
                OsString::from(if cfg!(windows) { "NUL" } else { "/dev/null" }),
                OsString::from("-f"),
                config.into_os_string(),
                OsString::from("config"),
                OsString::from("--quiet"),
            ],
            Duration::from_secs(10),
        )
        .await;
    let _ = std::fs::remove_dir_all(&temporary);
    capability.map_err(|_| error("ComposeCapabilityMissing"))?;
    Ok(target)
}
impl DockerRunner {
    pub async fn connect(target: DockerTarget) -> Result<Self, ToolboxError> {
        validate_target(&target)?;
        let runner = Self { target };
        runner.verify_identity().await?;
        Ok(runner)
    }
    pub(crate) async fn run_args(
        &self,
        values: &[OsString],
        timeout: Duration,
    ) -> Result<Vec<u8>, ToolboxError> {
        let mut args = vec![
            OsString::from("--context"),
            OsString::from(&self.target.context_name),
        ];
        args.extend_from_slice(values);
        invoke_cli(Path::new(&self.target.cli_path), &args, timeout).await
    }
    pub(crate) async fn run_read(&self, values: &[&str]) -> Result<Vec<u8>, ToolboxError> {
        self.run_args(&args(values), Duration::from_secs(30)).await
    }
    pub async fn verify_identity(&self) -> Result<(), ToolboxError> {
        validate_target(&self.target)?;
        let output = self
            .run_read(&["info", "--format", "{{json .}}"])
            .await
            .map_err(|_| error("DaemonUnavailable"))?;
        let info: serde_json::Value =
            serde_json::from_slice(&output).map_err(|_| error("DockerOutputInvalid"))?;
        if info["ID"].as_str() != Some(&self.target.daemon_id)
            || info["OSType"].as_str() != Some("linux")
        {
            return Err(error("TargetChanged"));
        }
        let inspected = self
            .run_read(&["context", "inspect", &self.target.context_name])
            .await?;
        let inspected: serde_json::Value =
            serde_json::from_slice(&inspected).map_err(|_| error("DockerOutputInvalid"))?;
        let endpoint = inspected[0]["Endpoints"]["docker"]["Host"]
            .as_str()
            .ok_or_else(|| error("UnsupportedTarget"))?;
        if !(endpoint.starts_with("unix://") && self.target.endpoint_kind == "unix"
            || endpoint.starts_with("npipe://") && self.target.endpoint_kind == "npipe")
        {
            return Err(error("UnsupportedTarget"));
        }
        Ok(())
    }
    pub async fn compose_run(
        &self,
        stack: &Path,
        instance: &InstanceIdentity,
        artifact: &OverrideArtifact,
        values: &[OsString],
    ) -> Result<Vec<u8>, ToolboxError> {
        if instance.target_id != self.target.target_id
            || instance.daemon_id != self.target.daemon_id
        {
            return Err(error("TargetChanged"));
        }
        reject_links(stack)?;
        reject_links(&artifact.env_path)?;
        reject_links(&artifact.override_path)?;
        if !artifact.env_path.starts_with(stack) || !artifact.override_path.starts_with(stack) {
            return Err(error("UnsafePath"));
        }
        self.verify_identity().await?;
        let allowed = ["config", "up", "stop", "ps", "logs"];
        if !values
            .first()
            .and_then(|v| v.to_str())
            .is_some_and(|v| allowed.contains(&v))
        {
            return Err(error("UnsupportedOperation"));
        }
        if values.iter().any(|v| {
            [
                OsStr::new("--volumes"),
                OsStr::new("-v"),
                OsStr::new("--remove-orphans"),
                OsStr::new("--build"),
            ]
            .contains(&v.as_os_str())
        }) {
            return Err(error("UnsupportedOperation"));
        }
        let mut command = vec![
            "compose".into(),
            "--project-directory".into(),
            stack.as_os_str().to_owned(),
            "-p".into(),
            instance.project_name.clone().into(),
            "--env-file".into(),
            artifact.env_path.as_os_str().to_owned(),
            "-f".into(),
            stack.join("docker-compose.yml").into_os_string(),
            "-f".into(),
            artifact.override_path.as_os_str().to_owned(),
        ];
        command.extend_from_slice(values);
        self.run_args(&command, Duration::from_secs(600)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target() -> DockerTarget {
        DockerTarget {
            target_id: "t".into(),
            cli_path: "/usr/local/bin/docker".into(),
            context_name: "desktop-linux".into(),
            endpoint_kind: "unix".into(),
            daemon_id: "original".into(),
            os_type: "linux".into(),
            architecture: "aarch64".into(),
        }
    }
    #[test]
    fn compose_minimum_and_new_major() {
        for v in ["2.20.0", "v2.24.3", "invalid", "2.24.4-rc1"] {
            assert!(!compose_supported(v));
        }
        for v in ["2.24.4", "v2.30.1", "5.7.0"] {
            assert!(compose_supported(v));
        }
    }
    #[test]
    fn remote_and_windows_containers_are_rejected() {
        assert!(validate_target(&target()).is_ok());
        let mut t = target();
        t.endpoint_kind = "ssh".into();
        assert_eq!(validate_target(&t).unwrap_err().code, "UnsupportedTarget");
        t.endpoint_kind = "unix".into();
        t.os_type = "windows".into();
        assert!(validate_target(&t).is_err());
    }
    #[test]
    fn inherited_docker_and_compose_overrides_are_removed() {
        let cmd = command(Path::new("/usr/local/bin/docker"), &[]);
        let vars = cmd.as_std().get_envs().collect::<Vec<_>>();
        for key in [
            "DOCKER_HOST",
            "DOCKER_CONTEXT",
            "DOCKER_DEFAULT_PLATFORM",
            "COMPOSE_FILE",
            "COMPOSE_PROJECT_NAME",
            "COMPOSE_ENV_FILES",
        ] {
            // env_clear omits inherited removals from get_envs; absence is also safe.
            assert!(
                !vars
                    .iter()
                    .any(|(name, value)| *name == OsStr::new(key) && value.is_some())
            );
        }
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn context_becoming_remote_is_rejected_even_with_same_daemon() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let cli = root.path().join("docker");
        std::fs::write(&cli,r##"#!/bin/sh
if [ "$3" = "info" ]; then printf '%s' '{"ID":"original","OSType":"linux","Architecture":"aarch64"}'; else printf '%s' '[{"Endpoints":{"docker":{"Host":"ssh://remote"}}}]'; fi
"##).unwrap();
        std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut t = target();
        t.cli_path = cli.to_str().unwrap().into();
        let runner = DockerRunner { target: t };
        assert_eq!(
            runner.verify_identity().await.unwrap_err().code,
            "UnsupportedTarget"
        );
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn excessive_cli_output_is_bounded() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let cli = root.path().join("docker");
        std::fs::write(&cli, "#!/bin/sh\nyes x | head -c 2200000\n").unwrap();
        std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            invoke_cli(&cli, &[], Duration::from_secs(3))
                .await
                .unwrap_err()
                .code,
            "DockerOutputTooLarge"
        );
    }
    #[tokio::test]
    #[ignore = "read-only local Docker native check; no containers created"]
    async fn local_environment_capability_probe() {
        let target = discover_target(None, None).await.unwrap();
        let runner = DockerRunner::connect(target).await.unwrap();
        runner.verify_identity().await.unwrap();
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn changed_daemon_is_rejected_before_mutation() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let cli = root.path().join("docker with spaces");
        std::fs::write(&cli,"#!/bin/sh\nprintf '%s' '{\"ID\":\"replacement\",\"OSType\":\"linux\",\"Architecture\":\"aarch64\"}'\n").unwrap();
        std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut t = target();
        t.cli_path = cli.to_str().unwrap().into();
        let runner = DockerRunner { target: t };
        assert_eq!(
            runner.verify_identity().await.unwrap_err().code,
            "TargetChanged"
        );
    }
}

pub(crate) async fn invoke_cli_input(
    cli: &Path,
    args: &[OsString],
    input: &[u8],
    deadline: Duration,
) -> Result<Vec<u8>, ToolboxError> {
    invoke_cli_capture(cli, args, Some(input), deadline, false).await
}
impl DockerRunner {
    #[cfg_attr(not(feature = "desktop"), allow(dead_code))]
    pub(crate) async fn container_logs(
        &self,
        proof: &crate::bindings::VerifiedOwnedContainer,
        tail: u32,
        since: Option<&str>,
    ) -> Result<Vec<u8>, ToolboxError> {
        if proof.record.target_id != self.target.target_id
            || proof.record.daemon_id != self.target.daemon_id
            || proof.container_id().len() != 64
            || !proof
                .container_id()
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(error("OwnershipConflict"));
        }
        if tail > 2000 {
            return Err(error("InvalidLogLimit"));
        }
        if since.is_some_and(|value| {
            value.is_empty()
                || value.len() > 64
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_digit() || b"-:+.TZ".contains(&b))
        }) {
            return Err(error("InvalidLogCursor"));
        }
        tokio::time::timeout(Duration::from_secs(15), async {
            self.verify_identity().await?;
            let mut values: Vec<OsString> = vec![
                "--context".into(),
                self.target.context_name.clone().into(),
                "container".into(),
                "logs".into(),
                "--timestamps".into(),
                "--tail".into(),
                tail.to_string().into(),
            ];
            if let Some(since) = since {
                values.extend(["--since".into(), since.into()]);
            }
            values.extend(["--".into(), proof.container_id().into()]);
            // Each pipe is bounded by read_bounded; merged total is also 2 MiB.
            // Failure diagnostics are deliberately discarded and expose only codes.
            invoke_cli_capture(
                Path::new(&self.target.cli_path),
                &values,
                None,
                Duration::from_secs(15),
                true,
            )
            .await
        })
        .await
        .map_err(|_| error("TimedOut"))?
    }
    pub(crate) async fn run_probe_input(
        &self,
        values: &[OsString],
        input: &[u8],
        deadline: Duration,
    ) -> Result<Vec<u8>, ToolboxError> {
        self.verify_identity().await?;
        let mut args = vec!["--context".into(), self.target.context_name.clone().into()];
        args.extend_from_slice(values);
        invoke_cli_input(Path::new(&self.target.cli_path), &args, input, deadline).await
    }
}
#[cfg(all(test, unix))]
mod stdin_tests {
    use super::*;
    #[tokio::test]
    async fn bounded_stdin_is_delivered_without_shell_or_secret_arguments() {
        let root = tempfile::tempdir().unwrap();
        let program = root.path().join("fake docker");
        std::fs::write(&program, "#!/bin/sh\ncat\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            invoke_cli_input(
                &program,
                &[],
                b"private-test-payload",
                Duration::from_secs(2)
            )
            .await
            .unwrap(),
            b"private-test-payload"
        );
        assert_eq!(
            invoke_cli_input(
                &program,
                &[],
                &vec![0; 128 * 1024 + 1],
                Duration::from_secs(2)
            )
            .await
            .unwrap_err()
            .code,
            "InputTooLarge"
        );
    }
}
#[cfg(all(test, unix))]
mod environment_tests {
    use super::*;
    #[tokio::test]
    async fn child_environment_does_not_leak_unlisted_compose_variables() {
        let out = command(Path::new("/usr/bin/env"), &[])
            .output()
            .await
            .unwrap();
        let text = String::from_utf8(out.stdout).unwrap();
        for line in text.lines() {
            let key = line.split_once('=').unwrap().0;
            assert!(
                matches!(
                    key,
                    "HOME"
                        | "PATH"
                        | "DOCKER_CONFIG"
                        | "SSH_AUTH_SOCK"
                        | "XDG_RUNTIME_DIR"
                        | "DBUS_SESSION_BUS_ADDRESS"
                        | "LANG"
                        | "LC_ALL"
                        | "TMPDIR"
                        | "TMP"
                        | "TEMP"
                ),
                "unexpected child environment key {key}"
            );
        }
    }
}

#[cfg(all(test, unix))]
mod process_group_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    fn program(root: &Path) -> PathBuf {
        let cli = root.join("owned cli");
        std::fs::write(&cli,format!("#!/bin/sh\n(sh -c 'printf started > \"{}/started\"; sleep 1.5; printf continued > \"{}/escaped\"') &\nwait\n",root.display(),root.display())).unwrap();
        std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o700)).unwrap();
        cli
    }
    #[tokio::test]
    async fn timeout_kills_the_owned_plugin_process_group() {
        let root = tempfile::tempdir().unwrap();
        let cli = program(root.path());
        assert_eq!(
            invoke_cli(&cli, &[], Duration::from_millis(750))
                .await
                .unwrap_err()
                .code,
            "TimedOut"
        );
        assert!(root.path().join("started").exists());
        tokio::time::sleep(Duration::from_millis(1700)).await;
        assert!(
            !root.path().join("escaped").exists(),
            "the CLI's child plugin must not continue after timeout"
        );
    }
    #[tokio::test]
    async fn aborting_a_stdin_cli_future_kills_its_owned_plugin_group() {
        let root = tempfile::tempdir().unwrap();
        let cli = program(root.path());
        let job = tokio::spawn(async move {
            invoke_cli_input(&cli, &[], b"test", Duration::from_secs(5)).await
        });
        for _ in 0..100 {
            if root.path().join("started").exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(root.path().join("started").exists());
        job.abort();
        let _ = job.await;
        tokio::time::sleep(Duration::from_millis(1700)).await;
        assert!(
            !root.path().join("escaped").exists(),
            "dropping the future must kill its descendants"
        );
    }
    #[tokio::test]
    async fn ordinary_cli_output_does_not_include_stderr() {
        let root = tempfile::tempdir().unwrap();
        let cli = root.path().join("cli");
        std::fs::write(&cli, "#!/bin/sh\nprintf stdout; printf stderr >&2\n").unwrap();
        std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            invoke_cli(&cli, &[], Duration::from_secs(2)).await.unwrap(),
            b"stdout"
        );
    }
}

#[cfg(all(test, unix))]
mod container_log_tests {
    use super::*;
    use crate::bindings::{OwnedContainerRecord, VerifiedOwnedContainer, verify_ownership};
    use std::os::unix::fs::PermissionsExt;
    fn fixture(root: &Path, body: &str) -> (DockerRunner, VerifiedOwnedContainer) {
        let cli = root.join("log cli");
        let cid = "c".repeat(64);
        let source = format!(
            "#!/bin/sh\nif [ \"$3\" = info ]; then printf '%s' '{{\"ID\":\"daemon\",\"OSType\":\"linux\"}}'; elif [ \"$3\" = context ]; then printf '%s' '[{{\"Endpoints\":{{\"docker\":{{\"Host\":\"unix:///fixture\"}}}}}}]'; else\n{body}\nfi\n"
        );
        std::fs::write(&cli, source).unwrap();
        std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o700)).unwrap();
        let target = DockerTarget {
            target_id: "target".into(),
            daemon_id: "daemon".into(),
            cli_path: cli.to_str().unwrap().into(),
            context_name: "local-fixture".into(),
            endpoint_kind: "unix".into(),
            os_type: "linux".into(),
            architecture: "arm64".into(),
        };
        let record = OwnedContainerRecord {
            container_id: cid.clone(),
            daemon_id: "daemon".into(),
            target_id: "target".into(),
            workspace_id: "workspace".into(),
            instance_id: "instance".into(),
            service_id: "redis".into(),
            blueprint_sha: "blueprint".into(),
            bind_sources: vec![],
        };
        let container = serde_json::json!({"Id":cid,"Config":{"Labels":{"io.codeark.workspace-id":"workspace","io.codeark.instance-id":"instance","io.codeark.service-id":"redis","io.codeark.blueprint-sha":"blueprint"}},"Mounts":[]});
        let proof = verify_ownership(&container, &record, &target).unwrap();
        (DockerRunner { target }, proof)
    }
    #[tokio::test]
    async fn log_deadline_covers_identity_checks_as_well_as_the_log_process() {
        let root = tempfile::tempdir().unwrap();
        let (runner, proof) = fixture(root.path(), "printf output");
        let text = std::fs::read_to_string(&runner.target.cli_path)
            .unwrap()
            .replace(
                "if [ \"$3\" = info ]; then printf",
                "if [ \"$3\" = info ]; then sleep 17; printf",
            );
        std::fs::write(&runner.target.cli_path, text).unwrap();
        let started = std::time::Instant::now();
        assert_eq!(
            runner
                .container_logs(&proof, 10, None)
                .await
                .unwrap_err()
                .code,
            "TimedOut"
        );
        assert!(started.elapsed() < Duration::from_secs(16));
    }
    #[tokio::test]
    async fn owned_logs_merge_both_streams_using_only_fixed_context_and_container_arguments() {
        let root = tempfile::tempdir().unwrap();
        let cid = "c".repeat(64);
        let body = format!(
            "[ \"$1\" = --context ] && [ \"$2\" = local-fixture ] && [ \"$3\" = container ] && [ \"$4\" = logs ] && [ \"$5\" = --timestamps ] && [ \"$6\" = --tail ] && [ \"$7\" = 7 ] && [ \"$8\" = --since ] && [ \"$9\" = 2026-11-12T10:00:00Z ] && [ \"${{10}}\" = -- ] && [ \"${{11}}\" = {cid} ] || exit 9\nprintf stdout-log; printf stderr-log >&2"
        );
        let (runner, proof) = fixture(root.path(), &body);
        let output = runner
            .container_logs(&proof, 7, Some("2026-11-12T10:00:00Z"))
            .await
            .unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("stdout-log"));
        assert!(text.contains("stderr-log"));
        assert!(text.contains('\n'));
    }
    #[tokio::test]
    async fn owned_logs_reject_cross_target_limits_and_unsafe_cursors() {
        let root = tempfile::tempdir().unwrap();
        let (runner, mut proof) = fixture(root.path(), "exit 77");
        assert_eq!(
            runner
                .container_logs(&proof, 2001, None)
                .await
                .unwrap_err()
                .code,
            "InvalidLogLimit"
        );
        assert_eq!(
            runner
                .container_logs(&proof, 10, Some("now\n--follow"))
                .await
                .unwrap_err()
                .code,
            "InvalidLogCursor"
        );
        proof.record.target_id = "foreign".into();
        assert_eq!(
            runner
                .container_logs(&proof, 10, None)
                .await
                .unwrap_err()
                .code,
            "OwnershipConflict"
        );
    }
    #[tokio::test]
    async fn owned_log_overflow_and_command_errors_never_return_raw_error_text() {
        let root = tempfile::tempdir().unwrap();
        let (runner, proof) = fixture(root.path(), "printf PRIVATE_ERROR >&2; exit 8");
        let error = runner.container_logs(&proof, 10, None).await.unwrap_err();
        assert_eq!(error.code, "DockerCommandFailed");
        assert!(error.redacted_context.is_empty());
        assert!(!error.to_string().contains("PRIVATE_ERROR"));
        let root = tempfile::tempdir().unwrap();
        let (runner, proof) = fixture(
            root.path(),
            "head -c 1200000 /dev/zero; head -c 1200000 /dev/zero >&2",
        );
        assert_eq!(
            runner
                .container_logs(&proof, 10, None)
                .await
                .unwrap_err()
                .code,
            "DockerOutputTooLarge"
        );
    }
}
