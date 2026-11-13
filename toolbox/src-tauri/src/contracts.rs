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
    pub fn new(code: &str) -> Self {
        Self {
            code: code.into(),
            retryable: false,
            redacted_context: String::new(),
        }
    }
}
impl std::fmt::Display for ToolboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code)
    }
}
impl std::error::Error for ToolboxError {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dto_wire_contract() {
        let value = ToolboxError::new("ProbeFailed");
        assert_eq!(
            serde_json::to_value(value).unwrap(),
            serde_json::json!({"code":"ProbeFailed", "retryable":false,"redactedContext":""})
        );
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

dto!(WorkspaceIdentity {
    workspace_id: String,
    root: String
});
dto!(InstanceIdentity {
    workspace_id: String,
    service_id: String,
    instance_id: String,
    project_name: String,
    target_id: String,
    daemon_id: String
});
dto!(SecretRef { key: String });
dto!(PortMapping {
    component_id: String,
    host_ip: String,
    host_port: u16,
    container_port: u16,
    protocol: String
});
dto!(ComponentSpec {
    id: String,
    role: String,
    image_ref: String
});
dto!(EnvFieldSpec { key: String, public_default: String, sensitive: bool, required: bool, secret_policy: SecretPolicy, secret_format: Option<String> });
dto!(ProbeSpec {
    probe_id: String,
    component_id: String,
    role: String,
    kind: String,
    timeout_secs: u32,
    attempts: u32,
    required: bool
});
dto!(ServiceSpec { id: String, name: String, category: String, source_dir: String, source_project: String, assets: Vec<String>, data_paths: Vec<String>, image_refs: Vec<String>, components: Vec<ComponentSpec>, ports: Vec<PortMapping>, dependencies: Vec<String>, env_schema: Vec<EnvFieldSpec>, probes: Vec<ProbeSpec>, #[ts(type = "number")] min_memory_bytes: u64, #[ts(type = "number | null")] min_workspace_free_bytes: Option<u64>, #[ts(type = "number | null")] min_daemon_free_bytes: Option<u64>, #[ts(type = "number | null")] estimated_pull_bytes: Option<u64>, capacity_status: String, data_compatibility: String });
dto!(SceneSpec { id: String, name: String, services: Vec<String> });
dto!(BindingRequirements { service_id: String, engine: String, database_names: Vec<String>, editable_databases: bool });
dto!(HostGuide { id: String, name: String, source_dir: String, assets: Vec<String>, data_paths: Vec<String> });
dto!(ServiceCatalog { schema_version: u32, services: Vec<ServiceSpec>, scenes: Vec<SceneSpec>, guides: Vec<HostGuide> });
dto!(DockerTarget {
    target_id: String,
    cli_path: String,
    context_name: String,
    endpoint_kind: String,
    daemon_id: String,
    os_type: String,
    architecture: String
});
dto!(Endpoint {
    label: String,
    component_id: String,
    host: String,
    port: u16,
    protocol: String
});
dto!(EffectiveConfig { generation_id: String, target_id: String, instance_id: String, ports: Vec<PortMapping>, endpoints: Vec<Endpoint>, secret_refs: Vec<SecretRef>, config_digest: String, image_artifacts: Vec<ImageArtifactRef> });
dto!(ImageArtifactRef { source_id: String, kind: String, platform: String, registry_ref: Option<String>, manifest_digest: Option<String>, archive_url: Option<String>, archive_sha256: Option<String>, expected_image_id: Option<String>, local_content_tag: Option<String>, status: String });

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/generated/")]
pub enum SecretPolicy {
    GenerateOnFirstManagedCreate,
    InheritBinding,
    PreserveExisting,
    OptionalEmpty,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/generated/")]
pub enum ServicePhase {
    Uninitialized,
    Stopped,
    Starting,
    RunningUnverified,
    Ready,
    Degraded,
    Failed,
    AwaitingAction,
    ExternalObserve,
}

dto!(DatabaseBinding { binding_id: String, target_id: String, daemon_id: String, engine: String, lifecycle_owner: String, container_id: String, network_name: String, network_alias: String, container_port: u16, host_endpoint: Option<Endpoint>, credential_ref: SecretRef, allowed_databases: Vec<String>, grant: String, server_fingerprint: String });
pub type BindingSet = Vec<DatabaseBinding>;

dto!(StorageCapacity { #[ts(type="number | null")] workspace_free_bytes:Option<u64>, #[ts(type="number | null")] daemon_free_bytes:Option<u64>, daemon_status:String });
dto!(PreflightIssue { code:String, field:Option<String>, severity:String });
dto!(PreflightReport { issues:Vec<PreflightIssue>, capacity:StorageCapacity, #[ts(type="number")] required_memory_bytes:u64, #[ts(type="number")] daemon_memory_bytes:u64, requires_capacity_consent:bool });

dto!(InitializationGrant { target_id:String,server_fingerprint:String,allowed_databases:Vec<String>,allowed_accounts:Vec<String>,operations:Vec<String>,#[ts(type="number")] confirmed_at:u64 });
dto!(DatabaseClaim {claim_id:String,binding_id:String,daemon_id:String,server_fingerprint:String,database_names:Vec<String>,owner_instance_id:String,created_by_codeark:bool,ddl_version:String,ddl_checksum:String,status:String});

dto!(ServiceSnapshot {service_id:String,instance_id:Option<String>,phase:ServicePhase,ownership:String,desired_generation:Option<String>,applied_generation:Option<String>,last_ready_generation:Option<String>,message_code:Option<String>});
dto!(OperationStep {step_id:String,phase:String,message_code:String,#[ts(type="number")] elapsed_ms:u64,#[ts(type="number | null")] deadline_at:Option<u64>});
dto!(OperationRecord {operation_id:String,target_id:String,instance_ids:Vec<String>,request_digest:String,desired_generation:Option<String>,applied_generation:Option<String>,last_ready_generation:Option<String>,phase:String,steps:Vec<OperationStep>,created_resource_ids:Vec<String>,error_code:Option<String>,allowed_actions:Vec<String>,#[ts(type="number")] created_at:u64,#[ts(type="number")] seq:u64});
dto!(DashboardSnapshot {workspace:WorkspaceIdentity,target:Option<DockerTarget>,services:Vec<ServiceSnapshot>,operations:Vec<OperationRecord>,bindings:Vec<DatabaseBinding>,#[ts(type="number")] sampled_at:u64,environment_error:Option<String>});
dto!(StartRequest {workspace_id:String,target_id:String,service_ids:Vec<String>,scene_id:Option<String>,accept_unknown_capacity:bool});
dto!(StopRequest {workspace_id:String,target_id:String,instance_ids:Vec<String>,scene_id:Option<String>});
dto!(CredentialHandle {
    handle: String,
    label: String,
    sensitive: bool
});
dto!(ConnectionSnippet {
    label: String,
    body: String
});
dto!(ConnectionView {instance_id:String,applied_generation:String,host_endpoints:Vec<Endpoint>,container_endpoints:Vec<Endpoint>,required_networks:Vec<String>,credential_handles:Vec<CredentialHandle>,snippets:Vec<ConnectionSnippet>});
dto!(DraftField {key:String,value:Option<String>,sensitive:bool,editable:bool});
dto!(ConfigDraftView {draft_id:String,instance_id:String,base_generation:String,fields:Vec<DraftField>,validation_errors:Vec<String>,apply_impact:Vec<String>,requires_rebuild:bool,preflight:Option<PreflightReport>});
#[derive(Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/types/generated/")]
pub struct ConfigDraft {
    pub draft_id: String,
    pub instance_id: String,
    pub base_generation: String,
    pub values: std::collections::BTreeMap<String, String>,
}
dto!(ResetPlan {plan_id:String,target_id:String,instance_id:String,service_id:String,exact_resource_ids:Vec<String>,database_claims:Vec<DatabaseClaim>,impact:Vec<String>,fingerprint:String,#[ts(type="number")] expires_at:u64});
dto!(ResetConfirmation {
    plan_id: String,
    fingerprint: String,
    typed_service_id: String
});
dto!(DiagnosticsPreview {preview_id:String,sections:std::collections::BTreeMap<String,String>,excluded_fields:Vec<String>,risk_flags:Vec<String>});
dto!(DiagnosticsConfirmation {preview_id:String,excluded_sections:Vec<String>,include_logs:bool});
dto!(ExportResult {
    export_id: String,
    path: String
});
dto!(GuideSpec {guide_id:String,service_id:Option<String>,title:String,client_family:String,locked_client_version:Option<String>,required_tools:Vec<String>,source_files:Vec<String>,invocation:String,expected_output:String,cleanup:String,scope_notice:String,status:String});
dto!(LogStreamRequest {
    instance_id: String,
    tail_lines: u32
});
dto!(LogBatch {stream_id:String,#[ts(type="number")] batch_id:u64,#[ts(type="number")] from_seq:u64,lines:Vec<String>,#[ts(type="number")] dropped_count:u64});
dto!(LogAck {
    stream_id: String,
    #[ts(type = "number")]
    batch_id: u64
});
dto!(LogStreamHandle {
    stream_id: String,
    phase: String
});
dto!(DatabaseCandidate {candidate_id:String,container_id:String,name:String,engine:String,network_names:Vec<String>,host_endpoints:Vec<Endpoint>});
#[derive(Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/types/generated/")]
pub struct VerifyBindingRequest {
    pub candidate_id: String,
    pub username: String,
    pub password: String,
    pub allowed_databases: Vec<String>,
}
dto!(BindingGrantPreview {
    preview_id: String,
    binding_id: String,
    service_id: String,
    grant: InitializationGrant,
    scope_notice: String
});

dto!(ProbeResult {probe_id:String,component_id:String,success:bool,error_code:Option<String>,warning_code:Option<String>,#[ts(type="number")] sampled_at:u64});
dto!(StepProgressEvent {
    operation_id: String,
    #[ts(type = "number")]
    seq: u64,
    service_id: String,
    step_id: String,
    phase: String,
    message_code: String
});

dto!(LogStreamStateEvent {stream_id:String,phase:String,message_code:Option<String>});
