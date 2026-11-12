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
    io::{AsyncRead, AsyncReadExt},
    process::Command,
};

pub struct DockerRunner {
    pub target: DockerTarget,
}
fn command(cli: &Path, args: &[OsString]) -> Command {
    let mut cmd = Command::new(cli);
    cmd.args(args).stdin(Stdio::null()).kill_on_drop(true);
    for (key, _) in std::env::vars_os() {
        let name = key.to_string_lossy();
        if name.starts_with("COMPOSE_") || (name.starts_with("DOCKER_") && name != "DOCKER_CONFIG")
        {
            cmd.env_remove(key);
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
pub(crate) async fn invoke_cli(
    cli: &Path,
    args: &[OsString],
    deadline: Duration,
) -> Result<Vec<u8>, ToolboxError> {
    let mut child = command(cli, args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| error("DockerMissing"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| error("DockerOutputUnavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| error("DockerOutputUnavailable"))?;
    let result = tokio::time::timeout(deadline, async {
        let (output, _stderr) = tokio::try_join!(read_bounded(stdout), read_bounded(stderr))?;
        let status = child
            .wait()
            .await
            .map_err(|_| error("DockerCommandFailed"))?;
        if !status.success() {
            return Err(error("DockerCommandFailed"));
        }
        Ok(output)
    })
    .await;
    match result {
        Ok(Ok(bytes)) => Ok(bytes),
        Ok(Err(e)) => {
            let _ = child.kill().await;
            Err(e)
        }
        Err(_) => {
            let _ = child.kill().await;
            Err(error("TimedOut"))
        }
    }
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
            assert!(
                vars.iter()
                    .any(|(name, value)| *name == OsStr::new(key) && value.is_none())
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
