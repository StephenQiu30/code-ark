use crate::{contracts::*, workspace::*};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInstance {
    pub identity: InstanceIdentity,
    pub phase: ServicePhase,
    pub desired: Option<EffectiveConfig>,
    pub applied: Option<EffectiveConfig>,
    pub last_ready_generation: Option<String>,
    pub containers: Vec<crate::bindings::OwnedContainerRecord>,
    pub claims: Vec<DatabaseClaim>,
    #[serde(default)]
    pub active_presets: Vec<String>,
    pub bindings: Vec<DatabaseBinding>,
    pub message_code: Option<String>,
    #[serde(default)]
    pub intents: Vec<crate::recovery::DeploymentIntent>,
    #[serde(default)]
    pub applied_networks: Vec<String>,
    #[serde(default)]
    pub pending_env: Option<PendingEnv>,
}
pub struct RuntimeStore {
    pub workspace: WorkspaceIdentity,
}
impl RuntimeStore {
    pub fn new(workspace: WorkspaceIdentity) -> Self {
        Self { workspace }
    }
    pub fn stack(&self, service: &str) -> Result<PathBuf, ToolboxError> {
        if !crate::catalog::load_embedded_catalog()?
            .services
            .iter()
            .any(|s| s.id == service)
        {
            return Err(error("UnknownService"));
        }
        Ok(Path::new(&self.workspace.root).join("stacks").join(service))
    }
    pub fn save(&self, value: &RuntimeInstance) -> Result<(), ToolboxError> {
        if value.identity.workspace_id != self.workspace.workspace_id {
            return Err(error("WorkspaceIdentityConflict"));
        }
        let root = self.stack(&value.identity.service_id)?;
        if root.exists()
            && !root.join(".codeark-instance.json").exists()
            && std::fs::read_dir(&root)
                .map_err(|_| error("StateUnavailable"))?
                .next()
                .is_some()
        {
            return Err(error("UnownedStackDirectory"));
        }
        private_dir(&root)?;
        atomic_write(
            &root.join(".codeark-instance.json"),
            &serde_json::to_vec(value).map_err(|_| error("StateWriteFailed"))?,
        )
    }
    pub fn list(&self) -> Result<Vec<RuntimeInstance>, ToolboxError> {
        let mut values = vec![];
        for service in crate::catalog::load_embedded_catalog()?.services {
            let path = self.stack(&service.id)?.join(".codeark-instance.json");
            if path.exists() {
                let value: RuntimeInstance = read_json(&path)?;
                if value.identity.service_id != service.id
                    || value.identity.workspace_id != self.workspace.workspace_id
                {
                    return Err(error("StateCorrupt"));
                }
                values.push(value);
            }
        }
        Ok(values)
    }
    pub fn instance(&self, id: &str) -> Result<RuntimeInstance, ToolboxError> {
        self.list()?
            .into_iter()
            .find(|i| i.identity.instance_id == id)
            .ok_or_else(|| error("UnknownInstance"))
    }
}
pub fn validate_selection(
    catalog: &ServiceCatalog,
    request: &StartRequest,
) -> Result<Vec<String>, ToolboxError> {
    use std::collections::BTreeSet;
    let selected = request
        .service_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if selected.is_empty()
        || selected.len() != request.service_ids.len()
        || selected
            .iter()
            .any(|id| !catalog.services.iter().any(|s| s.id == *id))
    {
        return Err(error("UnknownService"));
    }
    if let Some(scene) = &request.scene_id {
        let scene = catalog
            .scenes
            .iter()
            .find(|s| s.id == *scene)
            .ok_or_else(|| error("UnknownScene"))?;
        let allowed = match scene.id.as_str() {
            "cdc-messaging" => {
                selected.len() == 3
                    && selected.contains("mysql")
                    && selected.contains("canal")
                    && ["kafka", "rabbitmq", "rocketmq"]
                        .iter()
                        .filter(|id| selected.contains(**id))
                        .count()
                        == 1
            }
            "search-observability" => selected
                .iter()
                .all(|id| ["elastic", "monitoring"].contains(id)),
            _ => selected == scene.services.iter().map(String::as_str).collect(),
        };
        if !allowed {
            return Err(error("InvalidSceneSelection"));
        }
    }
    Ok(request.service_ids.clone())
}
pub fn recovered_phase(value: &RuntimeInstance, running: bool) -> ServicePhase {
    if running {
        ServicePhase::RunningUnverified
    } else if matches!(
        value.phase,
        ServicePhase::Starting | ServicePhase::RunningUnverified
    ) {
        ServicePhase::AwaitingAction
    } else {
        ServicePhase::Stopped
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backend_validates_scene_alternatives_and_unknown_ids() {
        let c = crate::catalog::load_embedded_catalog().unwrap();
        let mut r = StartRequest {
            workspace_id: "w".into(),
            target_id: "t".into(),
            service_ids: vec!["mysql".into(), "canal".into(), "rabbitmq".into()],
            scene_id: Some("cdc-messaging".into()),
            accept_unknown_capacity: false,
        };
        assert_eq!(validate_selection(&c, &r).unwrap(), r.service_ids);
        r.service_ids.push("rocketmq".into());
        assert!(validate_selection(&c, &r).is_err());
        r.scene_id = None;
        r.service_ids = vec!["unknown".into()];
        assert!(validate_selection(&c, &r).is_err());
    }
}
pub fn scopes_after_stop(
    current: &[String],
    scene: Option<&str>,
) -> Result<Vec<String>, ToolboxError> {
    if scene.is_none() && current.iter().any(|s| s != "manual") {
        return Err(error("SharedPresetInUse"));
    }
    let stop = scene.unwrap_or("manual");
    Ok(current
        .iter()
        .filter(|s| s.as_str() != stop)
        .cloned()
        .collect())
}
#[cfg(test)]
mod scope_tests {
    use super::*;
    #[test]
    fn stopping_one_scene_keeps_prior_manual_and_other_scene_use() {
        let refs = vec!["manual".into(), "scene-a".into(), "scene-b".into()];
        assert_eq!(
            scopes_after_stop(&refs, Some("scene-a")).unwrap(),
            vec!["manual", "scene-b"]
        );
        assert!(scopes_after_stop(&refs, None).is_err());
        assert_eq!(
            scopes_after_stop(&["manual".into()], None).unwrap(),
            Vec::<String>::new()
        );
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PendingEnv {
    pub generation_id: String,
    pub document_digest: String,
    pub base_env_digest: String,
}
