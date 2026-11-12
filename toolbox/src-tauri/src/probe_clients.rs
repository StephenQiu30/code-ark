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
fn bundle(root: &Path, module: &str) -> Result<PathBuf, ToolboxError> {
    let lock: BTreeMap<String, BTreeMap<String, String>> =
        serde_json::from_str(LOCK).map_err(|_| err("ProbeArtifactLockInvalid"))?;
    let expected = lock
        .get(module)
        .filter(|v| v.contains_key("probe.jar"))
        .ok_or_else(|| err("ProbeClientArtifactMissing"))?;
    verify_bundle(&root.join(module), expected)
}
fn verify_bundle(dir: &Path, expected: &BTreeMap<String, String>) -> Result<PathBuf, ToolboxError> {
    crate::workspace::reject_links(dir)?;
    for (file, digest) in expected {
        let path = dir.join(file);
        crate::workspace::reject_links(&path)?;
        let mut reader = fs::File::open(&path).map_err(|_| err("ProbeClientArtifactMissing"))?;
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
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| err("ProbeArtifactUnexpectedFile"))?;
            if subdir.is_empty() && name == "lib" {
                continue;
            }
            if !expected.contains_key(&format!("{subdir}{name}")) {
                return Err(err("ProbeArtifactUnexpectedFile"));
            }
        }
    }
    dir.canonicalize()
        .map_err(|_| err("ProbeClientArtifactMissing"))
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
        .await?;
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
struct Cleanup {
    runner: DockerRunner,
    owner: String,
}
impl Cleanup {
    async fn remove(&self) {
        if self.runner.verify_identity().await.is_err() {
            return;
        }
        let owner = format!("label=io.codeark.probe-owner={}", self.owner);
        if let Ok(raw) = self
            .runner
            .run_read(&["ps", "-aq", "--no-trunc", "--filter", &owner])
            .await
            && let Ok(id) = std::str::from_utf8(&raw)
            && valid_id(id.trim())
        {
            let _ = self.runner.run_read(&["rm", "-f", id.trim()]).await;
        }
    }
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        if self.owner.is_empty() {
            return;
        }
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let mut guard = Cleanup {
                runner: DockerRunner {
                    target: self.runner.target.clone(),
                },
                owner: std::mem::take(&mut self.owner),
            };
            handle.spawn(async move {
                guard.remove().await;
                guard.owner.clear();
            });
        }
    }
}
async fn run(
    cx: &ProbeContext<'_>,
    id: &str,
    args: Vec<OsString>,
    stdin: &[u8],
) -> Result<Vec<u8>, ToolboxError> {
    if !valid_id(id) {
        return Err(err("ComponentIdentityUnverified"));
    }
    let owner = crate::workspace::random_id()?;
    let mut guard = Cleanup {
        runner: DockerRunner {
            target: cx.runner.target.clone(),
        },
        owner: owner.clone(),
    };
    let mut parameters: Vec<OsString> = [
        "run",
        "--rm",
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
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    parameters.extend(args);
    let result = cx
        .runner
        .run_probe_input(&parameters, stdin, Duration::from_secs(20))
        .await;
    if result.is_err() {
        guard.remove().await;
    }
    guard.owner.clear();
    result
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
    let dir = tokio::task::spawn_blocking(move || bundle(&root, module))
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
    let raw = run(cx, component_id, args, &bytes).await?;
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
        let raw = run(cx, id, args, &[]).await?;
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
