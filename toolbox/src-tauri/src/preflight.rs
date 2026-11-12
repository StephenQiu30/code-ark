use crate::contracts::*;
use crate::docker_cli::DockerRunner;
use crate::workspace::error;
use std::{collections::BTreeSet, path::Path};
pub fn assess(
    service: &ServiceSpec,
    effective: &EffectiveConfig,
    capacity: &StorageCapacity,
    daemon_memory: u64,
    owned_ports: &BTreeSet<(u16, String)>,
) -> PreflightReport {
    let mut issues = vec![];
    let mut issue = |code: &str, field: Option<String>, severity: &str| {
        issues.push(PreflightIssue {
            code: code.into(),
            field,
            severity: severity.into(),
        })
    };
    if daemon_memory < service.min_memory_bytes {
        issue("InsufficientMemory", None, "error");
    }
    let mut seen = BTreeSet::new();
    for port in &effective.ports {
        if port.host_ip != "127.0.0.1"
            || port.host_port == 0
            || !matches!(port.protocol.as_str(), "tcp" | "udp")
        {
            issue("InvalidPort", Some(port.component_id.clone()), "error");
            continue;
        }
        let key = (port.host_port, port.protocol.clone());
        if !seen.insert(key.clone()) {
            issue("DuplicatePort", Some(port.component_id.clone()), "error");
            continue;
        }
        if owned_ports.contains(&key) {
            continue;
        }
        let free = if port.protocol == "tcp" {
            std::net::TcpListener::bind(("127.0.0.1", port.host_port)).is_ok()
        } else {
            std::net::UdpSocket::bind(("127.0.0.1", port.host_port)).is_ok()
        };
        if !free {
            issue(
                "PortConflict",
                Some(format!("{}/{}", port.host_port, port.protocol)),
                "error",
            );
        }
    }
    if let (Some(available), Some(required)) = (
        capacity.workspace_free_bytes,
        service.min_workspace_free_bytes,
    ) && available < required
    {
        issue("InsufficientStorage", Some("workspace".into()), "error");
    }
    if let (Some(available), Some(required)) =
        (capacity.daemon_free_bytes, service.min_daemon_free_bytes)
        && available < required
    {
        issue("InsufficientStorage", Some("daemon".into()), "error");
    }
    let requires_capacity_consent = capacity.daemon_free_bytes.is_none()
        || capacity.workspace_free_bytes.is_none()
        || service.estimated_pull_bytes.is_none();
    if requires_capacity_consent {
        issue("UnknownCapacity", None, "action");
    }
    PreflightReport {
        issues,
        capacity: capacity.clone(),
        required_memory_bytes: service.min_memory_bytes,
        daemon_memory_bytes: daemon_memory,
        requires_capacity_consent,
    }
}
pub async fn run_preflight(
    runner: &DockerRunner,
    service: &ServiceSpec,
    effective: &EffectiveConfig,
    workspace: &Path,
    verified_owned_ports: &BTreeSet<(u16, String)>,
) -> Result<PreflightReport, ToolboxError> {
    runner.verify_identity().await?;
    let info = runner.run_read(&["info", "--format", "{{json .}}"]).await?;
    let info: serde_json::Value =
        serde_json::from_slice(&info).map_err(|_| error("DockerOutputInvalid"))?;
    let local_root = if cfg!(target_os = "linux") {
        info["DockerRootDir"].as_str().map(Path::new)
    } else {
        None
    };
    let capacity = crate::storage_probe::capacity(workspace, local_root)?;
    Ok(assess(
        service,
        effective,
        &capacity,
        info["MemTotal"]
            .as_u64()
            .ok_or_else(|| error("UnknownMemory"))?,
        verified_owned_ports,
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> EffectiveConfig {
        EffectiveConfig {
            generation_id: "g".into(),
            target_id: "t".into(),
            instance_id: "i".into(),
            ports: vec![],
            endpoints: vec![],
            secret_refs: vec![],
            config_digest: "".into(),
            image_artifacts: vec![],
        }
    }
    #[test]
    fn elastic_needs_six_gib_and_unknown_storage_consent() {
        let s = crate::catalog::load_embedded_catalog()
            .unwrap()
            .services
            .into_iter()
            .find(|s| s.id == "elastic")
            .unwrap();
        let c = StorageCapacity {
            workspace_free_bytes: Some(100 * 1024 * 1024 * 1024),
            daemon_free_bytes: None,
            daemon_status: "UnknownCapacity".into(),
        };
        let r = assess(&s, &config(), &c, 4 * 1024 * 1024 * 1024, &BTreeSet::new());
        assert!(r.issues.iter().any(|i| i.code == "InsufficientMemory"));
        assert!(r.requires_capacity_consent);
    }
    #[test]
    fn tcp_and_udp_conflicts_are_distinct() {
        let s = crate::catalog::load_embedded_catalog()
            .unwrap()
            .services
            .remove(0);
        let tcp = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = tcp.local_addr().unwrap().port();
        let mut c = config();
        c.ports.push(PortMapping {
            component_id: "test".into(),
            host_ip: "127.0.0.1".into(),
            host_port: port,
            container_port: 1234,
            protocol: "tcp".into(),
        });
        let capacity = StorageCapacity {
            workspace_free_bytes: Some(u64::MAX / 2),
            daemon_free_bytes: Some(u64::MAX / 2),
            daemon_status: "Known".into(),
        };
        let r = assess(&s, &c, &capacity, 64 * 1024 * 1024 * 1024, &BTreeSet::new());
        assert!(r.issues.iter().any(|i| i.code == "PortConflict"));
        c.ports[0].protocol = "udp".into();
        assert!(
            !assess(&s, &c, &capacity, 64 * 1024 * 1024 * 1024, &BTreeSet::new())
                .issues
                .iter()
                .any(|i| i.code == "PortConflict")
        );
    }
}
