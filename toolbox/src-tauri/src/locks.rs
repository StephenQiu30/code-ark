use crate::{
    contracts::ToolboxError,
    workspace::{error, open_private, private_dir},
};
use std::{fs::File, path::Path};
pub struct ResourceGuard {
    _files: Vec<File>,
}
impl Drop for ResourceGuard {
    fn drop(&mut self) {
        // A concurrently spawned child can briefly inherit an open description
        // before exec. Release the lock explicitly instead of waiting for every
        // inherited descriptor to close.
        for file in &self._files {
            let _ = file.unlock();
        }
    }
}
pub fn acquire(root: &Path, keys: &[String]) -> Result<ResourceGuard, ToolboxError> {
    use sha2::{Digest, Sha256};
    private_dir(root)?;
    let mut keys = keys.to_vec();
    keys.sort();
    keys.dedup();
    if keys.is_empty() || keys.len() > 128 || keys.iter().any(|k| k.is_empty() || k.len() > 4096) {
        return Err(error("InvalidLockKey"));
    }
    let mut files = vec![];
    for key in keys {
        let name = format!("{:x}.lock", Sha256::digest(key.as_bytes()));
        let file = open_private(&root.join(name), false)?;
        file.try_lock().map_err(|_| error("ResourceBusy"))?;
        files.push(file);
    }
    Ok(ResourceGuard { _files: files })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independently_opened_handles_cannot_mutate_one_resource() {
        let d = tempfile::tempdir().unwrap();
        let key = "daemon/database/server/ca_jobs".to_owned();
        let a = acquire(d.path(), std::slice::from_ref(&key)).unwrap();
        assert!(acquire(d.path(), std::slice::from_ref(&key)).is_err());
        drop(a);
        assert!(acquire(d.path(), &[key]).is_ok());
    }
}
