use crate::{
    contracts::ToolboxError,
    workspace::{atomic_write, error, private_dir, read_json},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageRegistration {
    pub target_id: String,
    pub provider_instance_id: String,
    pub consumer_workspace_id: String,
    pub consumer_instance_id: String,
    pub operation_id: String,
    pub preset_refs: Vec<String>,
    pub status: String,
}
#[derive(Serialize, Deserialize)]
struct Index {
    schema_version: u32,
    entries: Vec<UsageRegistration>,
}
pub struct UsageIndex {
    root: PathBuf,
}
impl UsageIndex {
    pub fn new(root: &Path) -> Result<Self, ToolboxError> {
        private_dir(root)?;
        let _guard = crate::locks::acquire(&root.join("locks"), &["usage-index".into()])?;
        let path = root.join("usage.json");
        let marker = root.join("index-created");
        if !path.exists() {
            if marker.exists() {
                return Err(error("UnknownUsage"));
            }
            atomic_write(&marker, b"1")?;
            atomic_write(
                &path,
                &serde_json::to_vec(&Index {
                    schema_version: 1,
                    entries: vec![],
                })
                .map_err(|_| error("UnknownUsage"))?,
            )?;
        }
        if !marker.exists() {
            return Err(error("UnknownUsage"));
        }
        let value = Self { root: root.into() };
        value.load()?;
        Ok(value)
    }
    fn load(&self) -> Result<Index, ToolboxError> {
        let index: Index =
            read_json(&self.root.join("usage.json")).map_err(|_| error("UnknownUsage"))?;
        if index.schema_version != 1
            || index.entries.len() > 16384
            || index.entries.iter().any(|e| {
                e.target_id.is_empty()
                    || e.provider_instance_id.is_empty()
                    || e.consumer_instance_id.is_empty()
                    || e.consumer_workspace_id.is_empty()
                    || !matches!(e.status.as_str(), "Active" | "Pending" | "Unknown")
            })
        {
            return Err(error("UnknownUsage"));
        }
        Ok(index)
    }
    pub fn register(&self, entry: UsageRegistration) -> Result<(), ToolboxError> {
        let _guard = crate::locks::acquire(&self.root.join("locks"), &["usage-index".into()])?;
        let mut index = self.load()?;
        if entry.target_id.is_empty()
            || entry.provider_instance_id.is_empty()
            || entry.consumer_instance_id.is_empty()
            || entry.consumer_workspace_id.is_empty()
            || !matches!(entry.status.as_str(), "Active" | "Pending")
        {
            return Err(error("InvalidUsage"));
        }
        if let Some(old) = index.entries.iter_mut().find(|e| {
            e.target_id == entry.target_id
                && e.provider_instance_id == entry.provider_instance_id
                && e.consumer_workspace_id == entry.consumer_workspace_id
                && e.consumer_instance_id == entry.consumer_instance_id
        }) {
            if old.status == "Unknown" {
                return Err(error("UnknownUsage"));
            }
            let refs = old
                .preset_refs
                .iter()
                .chain(&entry.preset_refs)
                .cloned()
                .collect::<std::collections::BTreeSet<_>>();
            *old = entry;
            old.preset_refs = refs.into_iter().collect();
        } else {
            index.entries.push(entry);
        }
        atomic_write(
            &self.root.join("usage.json"),
            &serde_json::to_vec(&index).map_err(|_| error("UnknownUsage"))?,
        )
    }
    pub fn consumers(
        &self,
        target: &str,
        provider: &str,
    ) -> Result<Vec<UsageRegistration>, ToolboxError> {
        let _guard = crate::locks::acquire(&self.root.join("locks"), &["usage-index".into()])?;
        let index = self.load()?;
        Ok(index
            .entries
            .into_iter()
            .filter(|e| e.target_id == target && e.provider_instance_id == provider)
            .collect())
    }
    pub(crate) fn unregister_stopped(
        &self,
        target: &str,
        workspace: &str,
        consumer: &str,
    ) -> Result<(), ToolboxError> {
        let _guard = crate::locks::acquire(&self.root.join("locks"), &["usage-index".into()])?;
        let mut index = self.load()?;
        if index.entries.iter().any(|e| {
            e.target_id == target
                && e.consumer_workspace_id == workspace
                && e.consumer_instance_id == consumer
                && e.status == "Unknown"
        }) {
            return Err(error("UnknownUsage"));
        }
        index.entries.retain(|e| {
            !(e.target_id == target
                && e.consumer_workspace_id == workspace
                && e.consumer_instance_id == consumer)
        });
        atomic_write(
            &self.root.join("usage.json"),
            &serde_json::to_vec(&index).map_err(|_| error("UnknownUsage"))?,
        )
    }
}

impl UsageIndex {
    pub(crate) fn replace_own_presets(
        &self,
        target: &str,
        workspace: &str,
        instance: &str,
        presets: &[String],
    ) -> Result<(), ToolboxError> {
        let _guard = crate::locks::acquire(&self.root.join("locks"), &["usage-index".into()])?;
        let mut index = self.load()?;
        let entry = index
            .entries
            .iter_mut()
            .find(|e| {
                e.target_id == target
                    && e.provider_instance_id == instance
                    && e.consumer_instance_id == instance
                    && e.consumer_workspace_id == workspace
            })
            .ok_or_else(|| error("UnknownUsage"))?;
        if entry.status == "Unknown" || presets.is_empty() {
            return Err(error("UnknownUsage"));
        }
        entry.preset_refs = presets.to_vec();
        atomic_write(
            &self.root.join("usage.json"),
            &serde_json::to_vec(&index).map_err(|_| error("UnknownUsage"))?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cross_workspace_usage_and_unknown_entries_block_lifecycle() {
        let d = tempfile::tempdir().unwrap();
        let a = UsageIndex::new(d.path()).unwrap();
        a.register(UsageRegistration {
            target_id: "t".into(),
            provider_instance_id: "p".into(),
            consumer_workspace_id: "workspace-b".into(),
            consumer_instance_id: "consumer-b".into(),
            operation_id: "op".into(),
            preset_refs: vec!["scene-b".into()],
            status: "Active".into(),
        })
        .unwrap();
        let b = UsageIndex::new(d.path()).unwrap();
        assert_eq!(b.consumers("t", "p").unwrap().len(), 1);
        std::fs::write(d.path().join("usage.json"), "broken").unwrap();
        assert!(a.consumers("t", "p").is_err());
    }
}
