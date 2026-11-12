use crate::{
    contracts::*,
    workspace::{atomic_write, error, private_dir, read_json},
};
use std::path::{Path, PathBuf};
pub struct OperationStore {
    root: PathBuf,
}
impl OperationStore {
    pub fn new(root: &Path) -> Result<Self, ToolboxError> {
        private_dir(root)?;
        Ok(Self { root: root.into() })
    }
    pub fn persist(&self, op: &OperationRecord) -> Result<(), ToolboxError> {
        valid_id(&op.operation_id)?;
        let _guard = crate::locks::acquire(
            &self.root.join("locks"),
            std::slice::from_ref(&op.operation_id),
        )?;
        if op.target_id.is_empty()
            || op.request_digest.is_empty()
            || op.seq == 0
            || op.steps.len() > 4096
            || op.created_resource_ids.len() > 4096
        {
            return Err(error("InvalidOperation"));
        }
        let terminal = matches!(
            op.phase.as_str(),
            "Ready"
                | "Stopped"
                | "Failed"
                | "Canceled"
                | "RecoveryRequired"
                | "AwaitingAction"
                | "Degraded"
        );
        if !terminal && op.phase != "Running" && op.phase != "Queued" {
            return Err(error("InvalidOperationPhase"));
        }
        if op.phase == "Ready"
            && (op.desired_generation.is_none()
                || op.applied_generation != op.desired_generation
                || op.last_ready_generation != op.applied_generation)
        {
            return Err(error("InvalidGeneration"));
        }
        if op
            .allowed_actions
            .iter()
            .any(|a| !matches!(a.as_str(), "retry" | "cancel"))
            || op.allowed_actions.iter().any(|a| {
                a == "cancel" && terminal
                    || a == "retry"
                        && !matches!(
                            op.phase.as_str(),
                            "Failed" | "Canceled" | "AwaitingAction" | "Degraded"
                        )
            })
        {
            return Err(error("InvalidOperationActions"));
        }
        let path = self.root.join(format!("{}.json", op.operation_id));
        if path.exists() {
            let old: OperationRecord = read_json(&path)?;
            if old.operation_id != op.operation_id
                || old.target_id != op.target_id
                || old.request_digest != op.request_digest
                || old.instance_ids != op.instance_ids
                || old.created_at != op.created_at
                || old.seq >= op.seq
            {
                return Err(error("StaleOperation"));
            }
        }
        atomic_write(
            &path,
            &serde_json::to_vec(op).map_err(|_| error("InvalidOperation"))?,
        )
    }
    pub fn load(&self, id: &str) -> Result<OperationRecord, ToolboxError> {
        valid_id(id)?;
        let record: OperationRecord = read_json(&self.root.join(format!("{id}.json")))?;
        if record.operation_id != id {
            return Err(error("StateCorrupt"));
        }
        Ok(record)
    }
    pub fn list(&self) -> Result<Vec<OperationRecord>, ToolboxError> {
        let mut out = vec![];
        let mut recent = std::collections::BTreeMap::new();
        for entry in std::fs::read_dir(&self.root).map_err(|_| error("StateUnavailable"))? {
            let entry = entry.map_err(|_| error("StateUnavailable"))?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                let id = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| error("StateCorrupt"))?;
                let record = self.load(id)?;
                if matches!(
                    record.phase.as_str(),
                    "Running" | "Queued" | "RecoveryRequired" | "AwaitingAction"
                ) {
                    out.push(record);
                } else {
                    recent.insert((record.created_at, record.operation_id.clone()), record);
                    if recent.len() > 100 {
                        recent.pop_first();
                    }
                }
            }
        }
        out.extend(recent.into_values());
        out.sort_by_key(|op| op.created_at);
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    pub fn record() -> OperationRecord {
        OperationRecord {
            operation_id: "0123456789abcdef0123456789abcdef".into(),
            target_id: "target".into(),
            instance_ids: vec!["instance".into()],
            request_digest: "digest".into(),
            desired_generation: Some("desired".into()),
            applied_generation: None,
            last_ready_generation: None,
            phase: "Running".into(),
            steps: vec![],
            created_resource_ids: vec![],
            error_code: None,
            allowed_actions: vec!["cancel".into()],
            created_at: 1,
            seq: 1,
        }
    }
    #[test]
    fn durable_operations_reject_stale_updates_and_claimed_ready() {
        let d = tempfile::tempdir().unwrap();
        let s = OperationStore::new(d.path()).unwrap();
        let mut op = record();
        s.persist(&op).unwrap();
        assert_eq!(s.load(&op.operation_id).unwrap(), op);
        op.seq = 0;
        assert!(s.persist(&op).is_err());
        op.seq = 2;
        op.phase = "Ready".into();
        assert!(s.persist(&op).is_err());
        op.applied_generation = op.desired_generation.clone();
        op.last_ready_generation = op.desired_generation.clone();
        op.allowed_actions.clear();
        s.persist(&op).unwrap();
    }
    #[test]
    fn path_is_an_id_not_a_frontend_path() {
        let d = tempfile::tempdir().unwrap();
        let s = OperationStore::new(d.path()).unwrap();
        assert!(s.load("../../other").is_err());
    }
}

fn valid_id(id: &str) -> Result<(), ToolboxError> {
    if id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        Ok(())
    } else {
        Err(error("InvalidOperationId"))
    }
}
#[cfg(test)]
mod history_tests {
    use super::*;
    #[test]
    fn long_history_never_breaks_dashboard_and_inflight_records_are_retained() {
        let d = tempfile::tempdir().unwrap();
        let store = OperationStore::new(d.path()).unwrap();
        for n in 0..2050u64 {
            let mut op = super::tests::record();
            op.operation_id = format!("{n:032x}");
            op.created_at = n + 1;
            op.phase = "Stopped".into();
            op.allowed_actions.clear();
            std::fs::write(
                d.path().join(format!("{}.json", op.operation_id)),
                serde_json::to_vec(&op).unwrap(),
            )
            .unwrap();
        }
        let mut active = super::tests::record();
        active.created_at = 0;
        store.persist(&active).unwrap();
        let list = store.list().unwrap();
        assert!(list.len() <= 101);
        assert!(list.iter().any(|op| op.operation_id == active.operation_id));
        assert!(list.iter().any(|op| op.created_at == 2050));
    }
}
