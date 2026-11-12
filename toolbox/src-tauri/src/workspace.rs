use crate::contracts::{
    DockerTarget, InstanceIdentity, ServiceSpec, ToolboxError, WorkspaceIdentity,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path},
};

pub(crate) fn error(code: &str) -> ToolboxError {
    ToolboxError::new(code)
}
pub(crate) fn random_id() -> Result<String, ToolboxError> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| error("RandomSourceUnavailable"))?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
pub(crate) fn reject_links(path: &Path) -> Result<(), ToolboxError> {
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(error("UnsafePath"));
    }
    for ancestor in path.ancestors() {
        // macOS aliases are OS-owned canonical prefixes, never a user workspace link.
        if [Path::new("/var"), Path::new("/tmp")].contains(&ancestor) {
            continue;
        }
        match fs::symlink_metadata(ancestor) {
            Ok(m) if m.file_type().is_symlink() => return Err(error("UnsafePath")),
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => return Err(error("PathUnavailable")),
        }
    }
    Ok(())
}
pub(crate) fn private_dir(path: &Path) -> Result<(), ToolboxError> {
    reject_links(path)?;
    fs::create_dir_all(path).map_err(|_| error("WorkspaceNotWritable"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| error("PermissionDenied"))?;
    }
    Ok(())
}
pub(crate) fn open_private(path: &Path, create_new: bool) -> Result<File, ToolboxError> {
    reject_links(path)?;
    let mut o = OpenOptions::new();
    o.read(true).write(true);
    if create_new {
        o.create_new(true);
    } else {
        o.create(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o.open(path).map_err(|_| error("StateUnavailable"))
}
pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), ToolboxError> {
    reject_links(path)?;
    let parent = path.parent().ok_or_else(|| error("UnsafePath"))?;
    let temp = parent.join(format!(".write-{}", random_id()?));
    let result = (|| {
        let mut file = open_private(&temp, true)?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| error("StateWriteFailed"))?;
        fs::rename(&temp, path).map_err(|_| error("StateWriteFailed"))?;
        #[cfg(unix)]
        File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|_| error("StateWriteFailed"))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
pub(crate) fn read_json<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<T, ToolboxError> {
    reject_links(path)?;
    let file = File::open(path).map_err(|_| error("StateUnavailable"))?;
    if file
        .metadata()
        .map_err(|_| error("StateUnavailable"))?
        .len()
        > 2 * 1024 * 1024
    {
        return Err(error("StateTooLarge"));
    }
    let mut bytes = Vec::new();
    file.take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error("StateUnavailable"))?;
    serde_json::from_slice(&bytes).map_err(|_| error("StateCorrupt"))
}
fn locked(path: &Path) -> Result<File, ToolboxError> {
    let file = open_private(path, false)?;
    file.try_lock().map_err(|_| error("ResourceBusy"))?;
    Ok(file)
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Registry {
    schema_version: u32,
    workspace_id: String,
    instances: Vec<InstanceIdentity>,
}
pub fn open_workspace(path: &Path) -> Result<WorkspaceIdentity, ToolboxError> {
    reject_links(path)?;
    for ancestor in path.ancestors().skip(1) {
        if ancestor.join("workspace.json").exists() {
            return Err(error("NestedWorkspace"));
        }
    }
    private_dir(path)?;
    let path = path.canonicalize().map_err(|_| error("UnsafePath"))?;
    let _lock = locked(&path.join(".workspace.lock"))?;
    let identity_path = path.join("workspace.json");
    if identity_path.exists() {
        let identity: WorkspaceIdentity = read_json(&identity_path)?;
        if identity.root != path.to_string_lossy() {
            return Err(error("WorkspaceIdentityConflict"));
        }
        let registry: Registry = read_json(&path.join("registry.json"))?;
        if registry.schema_version != 1 || registry.workspace_id != identity.workspace_id {
            return Err(error("WorkspaceIdentityConflict"));
        }
        return Ok(identity);
    }
    if path.join("registry.json").exists() {
        return Err(error("RecoveryRequired"));
    }
    let identity = WorkspaceIdentity {
        workspace_id: random_id()?,
        root: path
            .to_str()
            .ok_or_else(|| error("UnsupportedPathEncoding"))?
            .into(),
    };
    let registry = Registry {
        schema_version: 1,
        workspace_id: identity.workspace_id.clone(),
        instances: vec![],
    };
    atomic_write(
        &path.join("registry.json"),
        &serde_json::to_vec(&registry).map_err(|_| error("StateWriteFailed"))?,
    )?;
    atomic_write(
        &identity_path,
        &serde_json::to_vec(&identity).map_err(|_| error("StateWriteFailed"))?,
    )?;
    Ok(identity)
}
pub fn allocate_instance(
    workspace: &WorkspaceIdentity,
    service: &ServiceSpec,
    target: &DockerTarget,
) -> Result<InstanceIdentity, ToolboxError> {
    if !service
        .id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        || service.id.is_empty()
    {
        return Err(error("InvalidServiceId"));
    }
    let root = Path::new(&workspace.root);
    reject_links(root)?;
    let _lock = locked(&root.join(".workspace.lock"))?;
    let stored: WorkspaceIdentity = read_json(&root.join("workspace.json"))?;
    if &stored != workspace {
        return Err(error("WorkspaceIdentityConflict"));
    }
    let mut registry: Registry = read_json(&root.join("registry.json"))?;
    if registry.workspace_id != workspace.workspace_id {
        return Err(error("WorkspaceIdentityConflict"));
    }
    if let Some(i) = registry
        .instances
        .iter()
        .find(|i| i.service_id == service.id && i.target_id == target.target_id)
    {
        if i.daemon_id != target.daemon_id {
            return Err(error("TargetChanged"));
        }
        return Ok(i.clone());
    }
    let compact = workspace.workspace_id.replace('-', "");
    let mut size = 12usize.min(compact.len());
    let project_name = loop {
        let name = format!("ca-{}-{}", &compact[..size], service.id);
        if !registry.instances.iter().any(|i| i.project_name == name) {
            break name;
        }
        if size >= compact.len() {
            return Err(error("IdentityCollision"));
        }
        size = (size + 4).min(compact.len());
    };
    let instance = InstanceIdentity {
        workspace_id: workspace.workspace_id.clone(),
        service_id: service.id.clone(),
        instance_id: random_id()?,
        project_name,
        target_id: target.target_id.clone(),
        daemon_id: target.daemon_id.clone(),
    };
    registry.instances.push(instance.clone());
    atomic_write(
        &root.join("registry.json"),
        &serde_json::to_vec(&registry).map_err(|_| error("StateWriteFailed"))?,
    )?;
    Ok(instance)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn service() -> ServiceSpec {
        crate::catalog::load_embedded_catalog()
            .unwrap()
            .services
            .into_iter()
            .find(|s| s.id == "nacos")
            .unwrap()
    }
    fn target() -> DockerTarget {
        DockerTarget {
            target_id: "desktop".into(),
            cli_path: "/usr/local/bin/docker".into(),
            context_name: "desktop-linux".into(),
            endpoint_kind: "unix".into(),
            daemon_id: "daemon-a".into(),
            os_type: "linux".into(),
            architecture: "aarch64".into(),
        }
    }
    #[test]
    fn reopen_preserves_identity() {
        let dir = tempfile::tempdir().unwrap();
        let a = open_workspace(dir.path()).unwrap();
        let b = open_workspace(dir.path()).unwrap();
        assert_eq!(a.workspace_id, b.workspace_id);
        let i = allocate_instance(&a, &service(), &target()).unwrap();
        assert_eq!(i, allocate_instance(&b, &service(), &target()).unwrap());
    }
    #[test]
    fn names_isolate_all_resources() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let a =
            allocate_instance(&open_workspace(a.path()).unwrap(), &service(), &target()).unwrap();
        let b =
            allocate_instance(&open_workspace(b.path()).unwrap(), &service(), &target()).unwrap();
        assert_ne!(a.project_name, b.project_name);
        assert_ne!(a.instance_id, b.instance_id);
        assert!(a.project_name.starts_with("ca-"));
        assert_ne!(a.project_name, "nacos-start-local");
    }
    #[test]
    fn special_paths_are_lossless() {
        let parent = tempfile::tempdir().unwrap();
        let path = parent.path().join("中文 工作区");
        let a = open_workspace(&path).unwrap();
        assert_eq!(a.root, path.canonicalize().unwrap().to_str().unwrap());
        assert!(open_workspace(&path.join("nested")).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn atomic_write_preserves_old_file() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("config");
        std::fs::write(&file, b"old bytes").unwrap();
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
        let result = atomic_write(&file, b"new bytes");
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(result.is_err());
        assert_eq!(std::fs::read(&file).unwrap(), b"old bytes");
    }
    #[test]
    fn collision_extends_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = open_workspace(dir.path()).unwrap();
        let first = allocate_instance(&workspace, &service(), &target()).unwrap();
        let mut other = target();
        other.target_id = "other-target".into();
        let second = allocate_instance(&workspace, &service(), &other).unwrap();
        assert_ne!(first.project_name, second.project_name);
        assert!(second.project_name.len() > first.project_name.len());
    }
    #[cfg(unix)]
    #[test]
    fn reject_escaping_path() {
        let parent = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let link = parent.path().join("link");
        std::os::unix::fs::symlink(external.path(), &link).unwrap();
        assert!(open_workspace(&link).is_err());
        assert!(!external.path().join("workspace.json").exists());
    }
}
