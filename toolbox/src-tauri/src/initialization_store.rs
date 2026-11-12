use crate::contracts::ToolboxError;
use crate::workspace::{atomic_write, error, read_json};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializationJournal {
    pub schema_version: u32,
    pub claim_id: String,
    pub plan_digest: String,
    pub steps: BTreeMap<String, String>,
    pub phase: String,
    pub error_code: Option<String>,
}
pub struct InitializationStore {
    path: PathBuf,
}
impl InitializationStore {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_owned(),
        }
    }
    pub fn load(&self) -> Result<Option<InitializationJournal>, ToolboxError> {
        if self.path.exists() {
            read_json(&self.path).map(Some)
        } else {
            Ok(None)
        }
    }
    pub fn persist(&self, journal: &InitializationJournal) -> Result<(), ToolboxError> {
        if journal.schema_version != 1
            || journal.claim_id.is_empty()
            || journal.plan_digest.is_empty()
            || journal.steps.len() > 4096
        {
            return Err(error("InvalidInitializationJournal"));
        }
        if let Some(old) = self.load()?
            && (old.claim_id != journal.claim_id || old.plan_digest != journal.plan_digest)
        {
            return Err(error("SchemaIncompatible"));
        }
        atomic_write(
            &self.path,
            &serde_json::to_vec(journal).map_err(|_| error("InvalidInitializationJournal"))?,
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn journal_reopens_partial_state_without_secret_payloads() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("initialization.json");
        let store = InitializationStore::new(&path);
        let j = InitializationJournal {
            schema_version: 1,
            claim_id: "claim".into(),
            plan_digest: "plan".into(),
            steps: BTreeMap::from([
                ("table-a".into(), "Completed".into()),
                ("table-b".into(), "Running".into()),
            ]),
            phase: "Partial".into(),
            error_code: Some("InitializationPartial".into()),
        };
        store.persist(&j).unwrap();
        assert_eq!(
            InitializationStore::new(&path)
                .load()
                .unwrap()
                .unwrap()
                .steps["table-b"],
            "Running"
        );
    }
}
