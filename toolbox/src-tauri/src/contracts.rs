use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/types/generated/")]
pub struct ToolboxError {
    pub code: String,
    pub retryable: bool,
    pub redacted_context: String,
}
impl ToolboxError {
    pub fn new(code: &str) -> Self { Self { code: code.into(), retryable: false, redacted_context: String::new() } }
}
impl std::fmt::Display for ToolboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.code) }
}
impl std::error::Error for ToolboxError {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dto_wire_contract() {
        let value = ToolboxError::new("ProbeFailed");
        assert_eq!(serde_json::to_value(value).unwrap(), serde_json::json!({"code":"ProbeFailed", "retryable":false,"redactedContext":""}));
        assert!(ToolboxError::decl(&ts_rs::Config::default()).contains("redactedContext: string"));
        assert!(ServiceSpec::decl(&ts_rs::Config::default()).contains("minMemoryBytes: number"));
    }
}

macro_rules! dto {
    ($name:ident { $($(#[$meta:meta])* $field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
        #[serde(rename_all = "camelCase")]
        #[ts(export, export_to = "../../src/types/generated/")]
        pub struct $name { $( $(#[$meta])* pub $field: $ty),* }
    };
}


dto!(WorkspaceIdentity { workspace_id: String, root: String });
dto!(InstanceIdentity { workspace_id: String, service_id: String, instance_id: String, project_name: String, target_id: String, daemon_id: String });
dto!(SecretRef { key: String });
dto!(PortMapping { component_id: String, host_ip: String, host_port: u16, container_port: u16, protocol: String });
dto!(ComponentSpec { id: String, role: String, image_ref: String });
dto!(EnvFieldSpec { key: String, public_default: String, sensitive: bool, required: bool, secret_policy: SecretPolicy, secret_format: Option<String> });
dto!(ProbeSpec { probe_id: String, component_id: String, role: String, kind: String, timeout_secs: u32, attempts: u32, required: bool });
dto!(ServiceSpec { id: String, name: String, category: String, source_dir: String, source_project: String, assets: Vec<String>, data_paths: Vec<String>, image_refs: Vec<String>, components: Vec<ComponentSpec>, ports: Vec<PortMapping>, dependencies: Vec<String>, env_schema: Vec<EnvFieldSpec>, probes: Vec<ProbeSpec>, #[ts(type = "number")] min_memory_bytes: u64, #[ts(type = "number | null")] min_workspace_free_bytes: Option<u64>, #[ts(type = "number | null")] min_daemon_free_bytes: Option<u64>, #[ts(type = "number | null")] estimated_pull_bytes: Option<u64>, capacity_status: String });
dto!(SceneSpec { id: String, name: String, services: Vec<String> });
dto!(HostGuide { id: String, name: String, source_dir: String, assets: Vec<String>, data_paths: Vec<String> });
dto!(ServiceCatalog { schema_version: u32, services: Vec<ServiceSpec>, scenes: Vec<SceneSpec>, guides: Vec<HostGuide> });
dto!(DockerTarget { target_id: String, cli_path: String, context_name: String, endpoint_kind: String, daemon_id: String, os_type: String, architecture: String });
dto!(Endpoint { label: String, component_id: String, host: String, port: u16, protocol: String });
dto!(EffectiveConfig { generation_id: String, target_id: String, instance_id: String, ports: Vec<PortMapping>, endpoints: Vec<Endpoint>, secret_refs: Vec<SecretRef>, config_digest: String, image_artifacts: Vec<ImageArtifactRef> });
dto!(ImageArtifactRef { source_id: String, kind: String, platform: String, registry_ref: Option<String>, manifest_digest: Option<String>, archive_url: Option<String>, archive_sha256: Option<String>, expected_image_id: Option<String>, local_content_tag: Option<String>, status: String });

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/generated/")]
pub enum SecretPolicy { GenerateOnFirstManagedCreate, InheritBinding, PreserveExisting, OptionalEmpty }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/generated/")]
pub enum ServicePhase { Uninitialized, Stopped, Starting, RunningUnverified, Ready, Degraded, Failed, AwaitingAction, ExternalObserve }

dto!(DatabaseBinding { binding_id: String, target_id: String, daemon_id: String, engine: String, lifecycle_owner: String, container_id: String, network_name: String, network_alias: String, container_port: u16, host_endpoint: Option<Endpoint>, credential_ref: SecretRef, allowed_databases: Vec<String>, grant: String, server_fingerprint: String });
pub type BindingSet = Vec<DatabaseBinding>;
