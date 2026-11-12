use crate::{
    bindings::{OwnedContainerRecord, verify_ownership},
    contracts::*,
    workspace::error,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Serialize, Deserialize)]
pub struct ComponentIntent {
    pub component_id: String,
    pub container_name: String,
    pub generation_id: String,
    pub blueprint_sha: String,
    pub image_ref: String,
    pub bind_sources: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct DeploymentIntent {
    pub generation_id: String,
    pub components: Vec<ComponentIntent>,
}
pub struct Reconciled {
    pub containers: Vec<OwnedContainerRecord>,
    pub desired_complete: bool,
    pub any_running: bool,
    pub networks: Vec<String>,
}
pub fn reconcile(
    instance: &InstanceIdentity,
    target: &DockerTarget,
    intents: &[DeploymentIntent],
    desired: Option<&str>,
    actual: &[Value],
) -> Result<Reconciled, ToolboxError> {
    if instance.target_id != target.target_id || instance.daemon_id != target.daemon_id {
        return Err(error("TargetChanged"));
    }
    let mut records = vec![];
    let mut seen = std::collections::BTreeSet::new();
    let mut running = false;
    let mut networks = std::collections::BTreeSet::new();
    let mut desired_components = std::collections::BTreeSet::new();
    for c in actual {
        let labels = &c["Config"]["Labels"];
        let generation = labels["io.codeark.generation-id"]
            .as_str()
            .ok_or_else(|| error("RecoveryRequired"))?;
        let component = labels["com.docker.compose.service"]
            .as_str()
            .ok_or_else(|| error("RecoveryRequired"))?;
        let intent = intents
            .iter()
            .find(|i| i.generation_id == generation)
            .and_then(|i| i.components.iter().find(|p| p.component_id == component))
            .ok_or_else(|| error("RecoveryRequired"))?;
        let id = c["Id"]
            .as_str()
            .filter(|s| crate::probes::valid_id(s))
            .ok_or_else(|| error("RecoveryRequired"))?;
        if !seen.insert(id) {
            return Err(error("RecoveryRequired"));
        }
        if c["Name"].as_str().map(|s| s.trim_start_matches('/'))
            != Some(intent.container_name.as_str())
            || c["Config"]["Image"] != intent.image_ref
            || labels["com.docker.compose.project"] != instance.project_name
        {
            return Err(error("RecoveryRequired"));
        }
        let record = OwnedContainerRecord {
            container_id: id.into(),
            daemon_id: instance.daemon_id.clone(),
            target_id: instance.target_id.clone(),
            workspace_id: instance.workspace_id.clone(),
            instance_id: instance.instance_id.clone(),
            service_id: instance.service_id.clone(),
            blueprint_sha: intent.blueprint_sha.clone(),
            bind_sources: intent.bind_sources.clone(),
        };
        verify_ownership(c, &record, target)?;
        running |= c["State"]["Running"] == true;
        if let Some(ns) = c["NetworkSettings"]["Networks"].as_object() {
            networks.extend(ns.keys().cloned());
        }
        if Some(generation) == desired {
            desired_components.insert(component.to_string());
        }
        records.push(record);
    }
    let complete = desired
        .and_then(|g| intents.iter().find(|i| i.generation_id == g))
        .is_some_and(|i| {
            !i.components.is_empty()
                && i.components
                    .iter()
                    .all(|c| desired_components.contains(&c.component_id))
                && records.len() == i.components.len()
        });
    Ok(Reconciled {
        containers: records,
        desired_complete: complete,
        any_running: running,
        networks: networks.into_iter().collect(),
    })
}

pub fn intent_from_model(
    service: &ServiceSpec,
    identity: &InstanceIdentity,
    effective: &EffectiveConfig,
    model: &Value,
) -> Result<DeploymentIntent, ToolboxError> {
    let mut components = vec![];
    for component in &service.components {
        let c = &model["services"][&component.id];
        let name = c["container_name"]
            .as_str()
            .ok_or_else(|| error("InvalidComposeModel"))?;
        if name != format!("{}-{}", identity.project_name, component.id)
            || c["labels"]["io.codeark.generation-id"] != effective.generation_id
        {
            return Err(error("InvalidComposeModel"));
        }
        components.push(ComponentIntent {
            component_id: component.id.clone(),
            container_name: name.into(),
            generation_id: effective.generation_id.clone(),
            blueprint_sha: c["labels"]["io.codeark.blueprint-sha"]
                .as_str()
                .ok_or_else(|| error("InvalidComposeModel"))?
                .into(),
            image_ref: c["image"]
                .as_str()
                .ok_or_else(|| error("InvalidComposeModel"))?
                .into(),
            bind_sources: c["volumes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|v| v["type"] == "bind")
                .map(|v| {
                    v["source"]
                        .as_str()
                        .map(String::from)
                        .ok_or_else(|| error("InvalidComposeModel"))
                })
                .collect::<Result<Vec<_>, _>>()?,
        });
    }
    Ok(DeploymentIntent {
        generation_id: effective.generation_id.clone(),
        components,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> (
        InstanceIdentity,
        DockerTarget,
        Vec<DeploymentIntent>,
        Vec<Value>,
    ) {
        let i = InstanceIdentity {
            workspace_id: "workspace".into(),
            service_id: "redis".into(),
            instance_id: "instance".into(),
            project_name: "project".into(),
            target_id: "target".into(),
            daemon_id: "daemon".into(),
        };
        let t = DockerTarget {
            target_id: "target".into(),
            cli_path: "/usr/local/bin/docker".into(),
            context_name: "local".into(),
            endpoint_kind: "unix".into(),
            daemon_id: "daemon".into(),
            os_type: "linux".into(),
            architecture: "arm64".into(),
        };
        let intent = DeploymentIntent {
            generation_id: "generation".into(),
            components: vec![ComponentIntent {
                component_id: "redis".into(),
                container_name: "project-redis".into(),
                generation_id: "generation".into(),
                blueprint_sha: "blueprint".into(),
                image_ref: "redis@sha256:locked".into(),
                bind_sources: vec!["/owned/data".into()],
            }],
        };
        let c = serde_json::json!({"Id":"a".repeat(64),"Name":"/project-redis","Image":"sha256:image","Config":{"Image":"redis@sha256:locked","Labels":{"io.codeark.workspace-id":"workspace","io.codeark.instance-id":"instance","io.codeark.service-id":"redis","io.codeark.blueprint-sha":"blueprint","io.codeark.generation-id":"generation","com.docker.compose.project":"project","com.docker.compose.service":"redis"}},"Mounts":[{"Type":"bind","Source":"/owned/data"}],"State":{"Running":true},"NetworkSettings":{"Networks":{"project-default":{}}}});
        (i, t, vec![intent], vec![c])
    }
    #[test]
    fn exact_intent_recovers_a_container_created_before_registry_commit() {
        let (i, t, intents, actual) = input();
        let r = reconcile(&i, &t, &intents, Some("generation"), &actual).unwrap();
        assert!(r.desired_complete);
        assert!(r.any_running);
        assert_eq!(r.containers.len(), 1);
        let r = reconcile(&i, &t, &intents, Some("generation"), &[]).unwrap();
        assert!(!r.desired_complete);
        assert!(r.containers.is_empty());
    }
    #[test]
    fn unknown_generation_or_mounts_cannot_be_recovered_as_own() {
        let (i, t, intents, mut actual) = input();
        actual[0]["Config"]["Labels"]["io.codeark.generation-id"] = serde_json::json!("unknown");
        assert!(reconcile(&i, &t, &intents, Some("generation"), &actual).is_err());
        let (i, t, intents, mut actual) = input();
        actual[0]["Mounts"][0]["Source"] = serde_json::json!("/business/data");
        assert!(reconcile(&i, &t, &intents, Some("generation"), &actual).is_err());
    }
}
