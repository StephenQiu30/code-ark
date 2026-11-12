use crate::{
    contracts::*,
    env_store::{SecretValues, merge_env, parse_env, runtime_env},
    orchestrator::{Engine, now},
    workspace::{atomic_write, error, random_id, reject_links},
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::Arc};
pub struct Draft {
    pub view: ConfigDraftView,
    pub document: zeroize::Zeroizing<String>,
    pub document_digest: String,
    pub effective: EffectiveConfig,
    pub expires_at: u64,
    pub values: SecretValues,
}
pub fn draft(engine: &Engine, id: &str) -> Result<Draft, ToolboxError> {
    let store = engine.store()?;
    let i = store.instance(id)?;
    if i.pending_env.is_some() {
        return Err(error("ConfigurationRecoveryRequired"));
    }
    let service = engine.service(&i.identity.service_id)?;
    let target = engine.target()?;
    if i.identity.target_id != target.target_id {
        return Err(error("TargetChanged"));
    }
    let stack = store.stack(&service.id)?;
    let outcome = crate::blueprint::materialize_blueprint(
        &engine.resources.join("blueprints"),
        &stack,
        &service,
        &i.identity,
    )?;
    if !outcome.conflicts.is_empty() {
        return Err(error("BlueprintConflict"));
    }
    let path = stack.join(".env");
    reject_links(&path)?;
    let old = if path.exists() {
        std::fs::read_to_string(&path).map_err(|_| error("StateUnavailable"))?
    } else {
        String::new()
    };
    let sample = if stack.join(".env.example").exists() {
        std::fs::read_to_string(stack.join(".env.example"))
            .map_err(|_| error("BlueprintMissing"))?
    } else {
        String::new()
    };
    let candidate = merge_env(
        Some(&old),
        &sample,
        &service,
        &SecretValues::default(),
        i.applied.is_none(),
    )?;
    let values = SecretValues(parse_env(&candidate.document)?);
    let effective = engine.effective(&i, &service, &target)?;
    let mut fields = service
        .env_schema
        .iter()
        .map(|f| DraftField {
            key: f.key.clone(),
            value: if f.sensitive {
                None
            } else {
                values.0.get(&f.key).cloned()
            },
            sensitive: f.sensitive,
            editable: !f.sensitive || i.applied.is_none(),
        })
        .collect::<Vec<_>>();
    for p in &effective.ports {
        fields.push(DraftField {
            key: format!("port:{}:{}", p.component_id, p.container_port),
            value: Some(p.host_port.to_string()),
            sensitive: false,
            editable: !(service.id == "nacos" && [9848, 9849, 9850].contains(&p.container_port)),
        });
    }
    let base = i
        .applied
        .as_ref()
        .map(|e| e.generation_id.clone())
        .unwrap_or_else(|| "unapplied".into());
    let view = ConfigDraftView {
        draft_id: random_id()?,
        instance_id: id.into(),
        base_generation: base,
        fields,
        validation_errors: candidate.validation_errors.clone(),
        apply_impact: vec![
            "生成候选配置；应用时核验目标、端口和依赖，重建当前服务容器，保留持久化数据。".into(),
        ],
        requires_rebuild: true,
        preflight: None,
    };
    Ok(Draft {
        view,
        document: zeroize::Zeroizing::new(candidate.document.clone()),
        document_digest: format!("{:x}", Sha256::digest(old.as_bytes())),
        effective,
        expires_at: now() + 15 * 60 * 1000,
        values,
    })
}
pub fn validate(
    engine: &Engine,
    draft: &mut Draft,
    request: ConfigDraft,
) -> Result<ConfigDraftView, ToolboxError> {
    let i = engine.store()?.instance(&draft.view.instance_id)?;
    let generation = i
        .applied
        .as_ref()
        .map(|e| e.generation_id.as_str())
        .unwrap_or("unapplied");
    if request.draft_id != draft.view.draft_id
        || request.instance_id != draft.view.instance_id
        || request.base_generation != draft.view.base_generation
        || generation != request.base_generation
        || now() > draft.expires_at
    {
        return Err(error("StaleDraft"));
    }
    draft.view.preflight = None;
    let mut changes = BTreeMap::new();
    let mut errors = vec![];
    let mut effective = draft.effective.clone();
    let mut values = SecretValues(draft.values.0.clone());
    for (key, value) in request.values {
        let field = draft
            .view
            .fields
            .iter()
            .find(|f| f.key == key)
            .ok_or_else(|| error("UnknownConfigField"))?;
        if value.len() > 16 * 1024 || value.contains(['\n', '\r', '\0']) {
            errors.push(format!("InvalidValue:{key}"));
            continue;
        }
        if let Some(port) = key.strip_prefix("port:") {
            let (component, target) = port.rsplit_once(':').ok_or_else(|| error("InvalidPort"))?;
            let mapping = effective
                .ports
                .iter_mut()
                .find(|p| p.component_id == component && p.container_port.to_string() == target)
                .ok_or_else(|| error("InvalidPort"))?;
            if !field.editable && value != mapping.host_port.to_string() {
                return Err(error("CoupledPortReadOnly"));
            }
            match value.parse::<u16>() {
                Ok(value) if value > 0 => mapping.host_port = value,
                _ => errors.push(format!("InvalidPort:{key}")),
            };
            continue;
        }
        if field.sensitive && i.applied.is_some() && draft.values.0.get(&key) != Some(&value) {
            return Err(error("CredentialRotationUnsupported"));
        }
        if !field.editable && draft.values.0.get(&key) != Some(&value) {
            return Err(error("ConfigFieldReadOnly"));
        }
        values.0.insert(key.clone(), value.clone());
        changes.insert(key, value);
    }
    if i.identity.service_id == "nacos" {
        let base = effective
            .ports
            .iter()
            .find(|p| p.container_port == 8848)
            .ok_or_else(|| error("MissingPort"))?
            .host_port;
        for p in &mut effective.ports {
            if [9848, 9849, 9850].contains(&p.container_port) {
                p.host_port = base
                    .checked_add(p.container_port - 8848)
                    .ok_or_else(|| error("InvalidCoupledPorts"))?;
            }
        }
    }
    let service = engine.service(&i.identity.service_id)?;
    for field in &service.env_schema {
        if field.required && values.0.get(&field.key).is_none_or(|s| s.is_empty()) {
            errors.push(format!("RequiredField:{}", field.key));
        }
    }
    let keys = effective
        .ports
        .iter()
        .map(|p| (p.host_port, &p.protocol))
        .collect::<std::collections::BTreeSet<_>>();
    if keys.len() != effective.ports.len() {
        errors.push("DuplicatePort".into());
    }
    draft.view.validation_errors = errors;
    if !draft.view.validation_errors.is_empty() {
        return Ok(draft.view.clone());
    }
    draft.document = zeroize::Zeroizing::new(update_document(&draft.document, &changes)?);
    draft.values = values;
    effective.generation_id = random_id()?;
    effective.endpoints = effective
        .ports
        .iter()
        .map(|p| Endpoint {
            label: format!("{}:{}", p.component_id, p.container_port),
            component_id: p.component_id.clone(),
            host: p.host_ip.clone(),
            port: p.host_port,
            protocol: p.protocol.clone(),
        })
        .collect();
    effective.config_digest = String::new();
    draft.effective = effective;
    for field in &mut draft.view.fields {
        if !field.sensitive {
            field.value = if let Some(port) = field.key.strip_prefix("port:") {
                let (component, target) =
                    port.rsplit_once(':').ok_or_else(|| error("InvalidPort"))?;
                draft
                    .effective
                    .ports
                    .iter()
                    .find(|p| p.component_id == component && p.container_port.to_string() == target)
                    .map(|p| p.host_port.to_string())
            } else {
                draft.values.0.get(&field.key).cloned()
            };
        }
    }
    Ok(draft.view.clone())
}
fn update_document(
    document: &str,
    changes: &BTreeMap<String, String>,
) -> Result<String, ToolboxError> {
    let mut pending = changes.clone();
    let newline = if document.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut result = String::new();
    for raw in document.split_inclusive('\n') {
        let line = raw.trim_end_matches(['\r', '\n']);
        let key = line
            .trim_start()
            .strip_prefix("export ")
            .unwrap_or(line.trim_start())
            .split_once('=')
            .map(|(k, _)| k.trim());
        if let Some((key, value)) = key.and_then(|key| pending.remove(key).map(|v| (key, v))) {
            let encoded = runtime_env(&BTreeMap::from([(key.to_owned(), value)]))?;
            let (_, encoded) = encoded
                .trim_end()
                .split_once('=')
                .ok_or_else(|| error("InvalidEnvDocument"))?;
            let prefix = line
                .split_once('=')
                .ok_or_else(|| error("InvalidEnvDocument"))?
                .0;
            result.push_str(prefix);
            result.push('=');
            result.push_str(encoded);
            if raw.ends_with('\n') {
                result.push_str(newline);
            }
        } else {
            result.push_str(raw);
        }
    }
    for (key, value) in pending {
        if !result.is_empty() && !result.ends_with('\n') {
            result.push_str(newline);
        }
        result.push_str(runtime_env(&BTreeMap::from([(key, value)]))?.trim_end());
        result.push_str(newline);
    }
    Ok(result)
}
pub async fn apply(
    engine: &Arc<Engine>,
    draft: Draft,
    consent: bool,
) -> Result<OperationRecord, ToolboxError> {
    engine.apply_configuration(draft, consent).await
}
pub(crate) fn stage(
    engine: &Engine,
    draft: Draft,
    consent: bool,
    guard: &crate::orchestrator::BusyLease,
) -> Result<crate::runtime_state::RuntimeInstance, ToolboxError> {
    if !guard.controls(engine) {
        return Err(error("ResourceBusy"));
    }
    if !draft.view.validation_errors.is_empty() || now() > draft.expires_at {
        return Err(error("InvalidDraft"));
    }
    let report = draft
        .view
        .preflight
        .as_ref()
        .ok_or_else(|| error("DraftNotValidated"))?;
    if report.issues.iter().any(|i| i.severity == "error") {
        return Err(error("PreflightFailed"));
    }
    if report.requires_capacity_consent && !consent {
        return Err(error("UnknownCapacity"));
    }
    let store = engine.store()?;
    let mut i = store.instance(&draft.view.instance_id)?;
    let base = i
        .applied
        .as_ref()
        .map(|e| e.generation_id.as_str())
        .unwrap_or("unapplied");
    if base != draft.view.base_generation
        || i.identity.target_id != engine.target()?.target_id
        || draft.effective.instance_id != i.identity.instance_id
        || draft.effective.target_id != i.identity.target_id
    {
        return Err(error("StaleDraft"));
    }
    if i.pending_env.is_some() {
        return Err(error("ConfigurationRecoveryRequired"));
    }
    let path = store.stack(&i.identity.service_id)?.join(".env");
    reject_links(&path)?;
    let old = if path.exists() {
        zeroize::Zeroizing::new(
            std::fs::read_to_string(&path).map_err(|_| error("StateUnavailable"))?,
        )
    } else {
        zeroize::Zeroizing::new(String::new())
    };
    if format!("{:x}", Sha256::digest(old.as_bytes())) != draft.document_digest {
        return Err(error("ConfigFileChanged"));
    }
    if draft.effective.generation_id.len() != 32
        || !draft
            .effective
            .generation_id
            .bytes()
            .all(|b| b.is_ascii_hexdigit())
    {
        return Err(error("InvalidGeneration"));
    }
    let dir = store
        .stack(&i.identity.service_id)?
        .join(".runtime")
        .join(&draft.effective.generation_id);
    crate::workspace::private_dir(&dir)?;
    atomic_write(&dir.join("draft.env"), draft.document.as_bytes())?;
    i.pending_env = Some(crate::runtime_state::PendingEnv {
        generation_id: draft.effective.generation_id.clone(),
        document_digest: format!("{:x}", Sha256::digest(draft.document.as_bytes())),
        base_env_digest: draft.document_digest.clone(),
    });
    i.desired = Some(draft.effective);
    store.save(&i)?;
    Ok(i)
}
/// Returns only a validated generation-local candidate; never a frontend path.
pub(crate) fn pending_document(
    stack: &std::path::Path,
    instance: &crate::runtime_state::RuntimeInstance,
) -> Result<Option<zeroize::Zeroizing<String>>, ToolboxError> {
    let Some(pending) = &instance.pending_env else {
        return Ok(None);
    };
    if pending.generation_id.len() != 32
        || !pending.generation_id.bytes().all(|b| b.is_ascii_hexdigit())
        || instance
            .desired
            .as_ref()
            .is_none_or(|e| e.generation_id != pending.generation_id)
    {
        return Err(error("ConfigurationRecoveryRequired"));
    }
    let path = stack
        .join(".runtime")
        .join(&pending.generation_id)
        .join("draft.env");
    reject_links(&path)?;
    let document = zeroize::Zeroizing::new(
        std::fs::read_to_string(&path).map_err(|_| error("ConfigurationRecoveryRequired"))?,
    );
    if document.len() > 1024 * 1024
        || format!("{:x}", Sha256::digest(document.as_bytes())) != pending.document_digest
    {
        return Err(error("ConfigFileChanged"));
    }
    let live = stack.join(".env");
    reject_links(&live)?;
    let current = if live.exists() {
        zeroize::Zeroizing::new(
            std::fs::read_to_string(&live).map_err(|_| error("StateUnavailable"))?,
        )
    } else {
        zeroize::Zeroizing::new(String::new())
    };
    let hash = format!("{:x}", Sha256::digest(current.as_bytes()));
    if hash != pending.base_env_digest && hash != pending.document_digest {
        return Err(error("ConfigFileChanged"));
    }
    Ok(Some(document))
}
pub(crate) fn commit_pending(
    stack: &std::path::Path,
    instance: &mut crate::runtime_state::RuntimeInstance,
) -> Result<(), ToolboxError> {
    if let Some(document) = pending_document(stack, instance)? {
        atomic_write(&stack.join(".env"), document.as_bytes())?;
        instance.pending_env = None;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_and_unknown_draft_fields_cannot_change_config() {
        let d = tempfile::tempdir().unwrap();
        let w = crate::workspace::open_workspace(&d.path().join("w")).unwrap();
        let engine = Engine::new(
            w,
            d.path().join("c"),
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources"),
        )
        .unwrap();
        let target = DockerTarget {
            target_id: "t".into(),
            cli_path: "/usr/local/bin/docker".into(),
            context_name: "test".into(),
            endpoint_kind: "unix".into(),
            daemon_id: "d".into(),
            os_type: "linux".into(),
            architecture: "arm64".into(),
        };
        *engine.target.write().unwrap() = Some(target.clone());
        let instance = engine
            .instance_for(&engine.service("redis").unwrap(), &target)
            .unwrap();
        let mut draft = draft(&engine, &instance.identity.instance_id).unwrap();
        let bad = ConfigDraft {
            draft_id: draft.view.draft_id.clone(),
            instance_id: instance.identity.instance_id,
            base_generation: "stale".into(),
            values: BTreeMap::from([("COMPOSE_FILE".into(), "/other".into())]),
        };
        assert_eq!(
            validate(&engine, &mut draft, bad).unwrap_err().code,
            "StaleDraft"
        );
    }
}
#[cfg(test)]
mod staging_tests {
    use super::*;
    fn setup() -> (tempfile::TempDir, Engine, String) {
        let d = tempfile::tempdir().unwrap();
        let workspace = crate::workspace::open_workspace(&d.path().join("workspace")).unwrap();
        let engine = Engine::new(
            workspace,
            d.path().join("control"),
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources"),
        )
        .unwrap();
        let target = DockerTarget {
            target_id: "target".into(),
            cli_path: "/usr/local/bin/docker".into(),
            context_name: "test".into(),
            endpoint_kind: "unix".into(),
            daemon_id: "daemon".into(),
            os_type: "linux".into(),
            architecture: "arm64".into(),
        };
        *engine.target.write().unwrap() = Some(target.clone());
        let i = engine
            .instance_for(&engine.service("redis").unwrap(), &target)
            .unwrap();
        let path = engine.store().unwrap().stack("redis").unwrap().join(".env");
        std::fs::write(path, "# last ready env\nCUSTOM=old\n").unwrap();
        (d, engine, i.identity.instance_id)
    }
    fn ready_draft(engine: &Engine, id: &str) -> Draft {
        let mut draft = draft(engine, id).unwrap();
        draft.document.push_str("STAGED_ONLY=new\n");
        draft.view.preflight = Some(PreflightReport {
            issues: vec![],
            capacity: StorageCapacity {
                workspace_free_bytes: None,
                daemon_free_bytes: None,
                daemon_status: "UnknownCapacity".into(),
            },
            required_memory_bytes: 1,
            daemon_memory_bytes: 1024,
            requires_capacity_consent: true,
        });
        draft
    }
    #[test]
    fn staging_requires_bound_consent_and_never_overwrites_live_env() {
        let (_d, engine, id) = setup();
        let d = ready_draft(&engine, &id);
        let guard = engine.reserve().unwrap();
        assert_eq!(
            stage(&engine, d, false, &guard).err().unwrap().code,
            "UnknownCapacity"
        );
        let d = ready_draft(&engine, &id);
        let desired = d.effective.generation_id.clone();
        let staged = stage(&engine, d, true, &guard).unwrap();
        assert_eq!(staged.desired.unwrap().generation_id, desired);
        let live =
            std::fs::read_to_string(engine.store().unwrap().stack("redis").unwrap().join(".env"))
                .unwrap();
        assert!(!live.contains("STAGED_ONLY"));
        assert!(staged.pending_env.is_some());
    }
}
