use crate::{contracts::*, workspace::error};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptanceConfig {
    pub target_id: String,
    pub expected_daemon_id: String,
    pub cli_path: PathBuf,
    pub context_name: String,
    pub workspace_root: PathBuf,
    pub control_root: PathBuf,
    pub run_id: String,
    pub port_min: u16,
    pub port_max: u16,
    pub allow_create: bool,
    pub allow_destructive_owned_only: bool,
}
impl AcceptanceConfig {
    pub fn validate(&self, target: &DockerTarget) -> Result<(), ToolboxError> {
        crate::docker_cli::validate_target(target)?;
        if !self.allow_create
            || !self.allow_destructive_owned_only
            || self.target_id != target.target_id
            || self.expected_daemon_id != target.daemon_id
            || self.context_name != target.context_name
            || self.cli_path != std::path::Path::new(&target.cli_path)
        {
            return Err(error("AcceptanceTargetMismatch"));
        }
        if self.run_id.len() != 32
            || !self.run_id.bytes().all(|b| b.is_ascii_hexdigit())
            || self.port_min < 20000
            || self.port_max <= self.port_min
            // Nacos requires its main port plus offsets 1000, 1001 and 1002.
            || self.port_max - self.port_min > 2048
            || !self.workspace_root.is_absolute()
            || !self.control_root.is_absolute()
            || self.control_root.starts_with(&self.workspace_root)
            || self.workspace_root.starts_with(&self.control_root)
        {
            return Err(error("AcceptanceScopeInvalid"));
        }
        crate::workspace::reject_links(&self.workspace_root)?;
        crate::workspace::reject_links(&self.control_root)?;
        let workspace: WorkspaceIdentity =
            crate::workspace::read_json(&self.workspace_root.join("workspace.json"))?;
        if workspace.workspace_id != self.run_id
            || std::path::Path::new(&workspace.root) != self.workspace_root
        {
            return Err(error("AcceptanceScopeInvalid"));
        }
        if !self.workspace_root.starts_with(
            std::env::temp_dir()
                .canonicalize()
                .map_err(|_| error("AcceptanceWorkspaceNotIsolated"))?,
        ) && !self
            .workspace_root
            .components()
            .any(|c| c.as_os_str() == ".tools")
        {
            return Err(error("AcceptanceWorkspaceNotIsolated"));
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn acceptance_refuses_unknown_target_and_unbounded_scope() {
        let d = tempfile::tempdir().unwrap();
        let w = crate::workspace::open_workspace(&d.path().join("w")).unwrap();
        let target = DockerTarget {
            target_id: "target".into(),
            cli_path: "/usr/local/bin/docker".into(),
            context_name: "desktop-linux".into(),
            endpoint_kind: "unix".into(),
            daemon_id: "daemon".into(),
            os_type: "linux".into(),
            architecture: "arm64".into(),
        };
        let mut c = AcceptanceConfig {
            target_id: target.target_id.clone(),
            expected_daemon_id: target.daemon_id.clone(),
            cli_path: target.cli_path.clone().into(),
            context_name: target.context_name.clone(),
            workspace_root: w.root.into(),
            control_root: d.path().join("control"),
            run_id: w.workspace_id,
            port_min: 27000,
            port_max: 27999,
            allow_create: true,
            allow_destructive_owned_only: true,
        };
        c.validate(&target).unwrap();
        c.expected_daemon_id = "changed".into();
        assert!(c.validate(&target).is_err());
        c.expected_daemon_id = target.daemon_id.clone();
        c.allow_create = false;
        assert!(c.validate(&target).is_err());
        c.allow_create = true;
        c.port_max = c.port_min + 1999;
        c.validate(&target).unwrap();
        c.port_max = c.port_min + 2049;
        assert!(c.validate(&target).is_err());
    }
}
