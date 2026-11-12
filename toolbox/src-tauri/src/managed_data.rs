use crate::{contracts::*, workspace::*};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};
#[derive(Serialize, Deserialize)]
struct Claims {
    schema_version: u32,
    workspace_id: String,
    instance_id: String,
    directories: BTreeMap<String, String>,
}
pub fn identity(path: &Path) -> Result<String, ToolboxError> {
    reject_links(path)?;
    let m = std::fs::symlink_metadata(path).map_err(|_| error("DataDirectoryUnavailable"))?;
    if !m.is_dir() {
        return Err(error("UnsafePath"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(format!("{}:{}", m.dev(), m.ino()))
    }
    #[cfg(windows)]
    {
        crate::windows_permissions::file_identity(path)
    }
    #[cfg(not(any(unix, windows)))]
    {
        Err(error("UnsupportedPermissionModel"))
    }
}
pub fn prepare(
    stack: &Path,
    service: &ServiceSpec,
    instance: &InstanceIdentity,
) -> Result<(), ToolboxError> {
    reject_links(stack)?;
    let marker = stack.join(".managed-data.json");
    let mut claims = if marker.exists() {
        let c: Claims = read_json(&marker)?;
        if c.schema_version != 1
            || c.workspace_id != instance.workspace_id
            || c.instance_id != instance.instance_id
        {
            return Err(error("UnownedDataDirectory"));
        }
        c
    } else {
        Claims {
            schema_version: 1,
            workspace_id: instance.workspace_id.clone(),
            instance_id: instance.instance_id.clone(),
            directories: BTreeMap::new(),
        }
    };
    for relative in &service.data_paths {
        let path = Path::new(relative);
        if path.is_absolute()
            || path
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err(error("UnsafePath"));
        }
        let full = stack.join(path);
        reject_links(&full)?;
        if full.exists() && claims.directories.get(relative) != Some(&identity(&full)?) {
            return Err(error("UnownedDataDirectory"));
        }
    }
    for relative in &service.data_paths {
        let path = stack.join(relative);
        if !path.exists() {
            private_dir(&path)?;
            claims
                .directories
                .insert(relative.clone(), identity(&path)?);
        }
    }
    atomic_write(
        &marker,
        &serde_json::to_vec(&claims).map_err(|_| error("StateWriteFailed"))?,
    )
}
pub fn verify(
    stack: &Path,
    service: &ServiceSpec,
    instance: &InstanceIdentity,
) -> Result<(), ToolboxError> {
    let claims: Claims = read_json(&stack.join(".managed-data.json"))?;
    if claims.schema_version != 1
        || claims.workspace_id != instance.workspace_id
        || claims.instance_id != instance.instance_id
    {
        return Err(error("UnownedDataDirectory"));
    }
    for relative in &service.data_paths {
        let path = stack.join(relative);
        reject_links(&path)?;
        if path.exists() && claims.directories.get(relative) != Some(&identity(&path)?) {
            return Err(error("UnownedDataDirectory"));
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preexisting_data_is_not_adopted_and_owned_directory_identity_is_stable() {
        let d = tempfile::tempdir().unwrap();
        let service = crate::catalog::load_embedded_catalog()
            .unwrap()
            .services
            .into_iter()
            .find(|s| s.id == "redis")
            .unwrap();
        let i = InstanceIdentity {
            workspace_id: "workspace".into(),
            service_id: "redis".into(),
            instance_id: "instance".into(),
            project_name: "project".into(),
            target_id: "target".into(),
            daemon_id: "daemon".into(),
        };
        std::fs::create_dir(d.path().join("data")).unwrap();
        assert_eq!(
            prepare(d.path(), &service, &i).unwrap_err().code,
            "UnownedDataDirectory"
        );
        std::fs::remove_dir(d.path().join("data")).unwrap();
        prepare(d.path(), &service, &i).unwrap();
        let identity = identity(&d.path().join("data")).unwrap();
        prepare(d.path(), &service, &i).unwrap();
        assert_eq!(identity, super::identity(&d.path().join("data")).unwrap());
    }
}
