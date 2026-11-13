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
    verified_owned_ids: &BTreeSet<String>,
) -> Result<PreflightReport, ToolboxError> {
    let available = available_daemon_memory(runner, verified_owned_ids).await?;
    // A local socket can front a Desktop VM or proxy. Until the daemon's host namespace
    // is positively identified, a host DockerRootDir never proves daemon free space.
    let capacity = crate::storage_probe::capacity(workspace, None)?;
    Ok(assess(
        service,
        effective,
        &capacity,
        available,
        verified_owned_ports,
    ))
}
async fn available_daemon_memory(
    runner: &DockerRunner,
    owned: &BTreeSet<String>,
) -> Result<u64, ToolboxError> {
    // An unknown observation proves no capacity. Re-read the complete sample a
    // bounded number of times, keeping target errors and persistent unknowns fatal.
    for attempt in 0..3 {
        runner.verify_identity().await?;
        let sample = async {
            let info = runner.run_read(&["info", "--format", "{{json .}}"]).await?;
            let info: serde_json::Value =
                serde_json::from_slice(&info).map_err(|_| error("DockerOutputInvalid"))?;
            if info["ID"] != runner.target.daemon_id || info["OSType"] != "linux" {
                return Err(error("TargetChanged"));
            }
            let stats = runner
                .run_read(&[
                    "stats",
                    "--no-stream",
                    "--no-trunc",
                    "--format",
                    "{{json .}}",
                ])
                .await?;
            let used = running_memory_bytes(
                std::str::from_utf8(&stats).map_err(|_| error("DockerOutputInvalid"))?,
                owned,
            )?;
            Ok::<u64, ToolboxError>(
                info["MemTotal"]
                    .as_u64()
                    .ok_or_else(|| error("UnknownMemory"))?
                    .saturating_sub(used)
                    .saturating_sub(512 * 1024 * 1024),
            )
        }
        .await;
        match sample {
            Err(error) if error.code == "UnknownMemory" && attempt < 2 => {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
            result => return result,
        }
    }
    Err(error("UnknownMemory"))
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
        // Reserve both protocols: a free TCP port says nothing about UDP occupancy.
        let (tcp, udp) = (0..32)
            .find_map(|_| {
                let tcp = std::net::TcpListener::bind("127.0.0.1:0").ok()?;
                let udp = std::net::UdpSocket::bind(tcp.local_addr().ok()?).ok()?;
                Some((tcp, udp))
            })
            .expect("an isolated TCP/UDP pair");
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
        let tcp_owned = BTreeSet::from([(port, "tcp".into())]);
        assert!(
            !assess(&s, &c, &capacity, 64 * 1024 * 1024 * 1024, &tcp_owned)
                .issues
                .iter()
                .any(|i| i.code == "PortConflict")
        );
        c.ports[0].protocol = "udp".into();
        assert!(
            assess(&s, &c, &capacity, 64 * 1024 * 1024 * 1024, &tcp_owned)
                .issues
                .iter()
                .any(|i| i.code == "PortConflict")
        );
        assert!(
            !assess(
                &s,
                &c,
                &capacity,
                64 * 1024 * 1024 * 1024,
                &BTreeSet::from([(port, "udp".into())])
            )
            .issues
            .iter()
            .any(|i| i.code == "PortConflict")
        );
        assert_eq!(udp.local_addr().unwrap().port(), port);
    }
}

pub fn running_memory_bytes(stats: &str, owned: &BTreeSet<String>) -> Result<u64, ToolboxError> {
    let mut total = 0u64;
    let mut seen = BTreeSet::new();
    for line in stats.lines().filter(|l| !l.trim().is_empty()) {
        let value: serde_json::Value =
            serde_json::from_str(line).map_err(|_| error("UnknownMemory"))?;
        let id = value["ID"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| error("UnknownMemory"))?;
        if !seen.insert(id.to_string()) {
            return Err(error("UnknownMemory"));
        }
        if owned.contains(id) {
            continue;
        }
        let used = value["MemUsage"]
            .as_str()
            .and_then(|s| s.split('/').next())
            .ok_or_else(|| error("UnknownMemory"))?
            .trim();
        let at = used
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .ok_or_else(|| error("UnknownMemory"))?;
        let amount = used[..at]
            .parse::<f64>()
            .map_err(|_| error("UnknownMemory"))?;
        let unit = match used[at..].trim() {
            "B" => 1f64,
            "kB" => 1000.,
            "MB" => 1e6,
            "GB" => 1e9,
            "TB" => 1e12,
            "KiB" => 1024.,
            "MiB" => 1048576.,
            "GiB" => 1073741824.,
            "TiB" => 1099511627776.,
            _ => return Err(error("UnknownMemory")),
        };
        let bytes = (amount * unit).ceil();
        if !bytes.is_finite() || bytes < 0. || bytes >= u64::MAX as f64 {
            return Err(error("UnknownMemory"));
        }
        total = total
            .checked_add(bytes as u64)
            .ok_or_else(|| error("UnknownMemory"))?;
    }
    Ok(total)
}
#[cfg(test)]
mod load_tests {
    use super::*;
    #[test]
    fn active_container_memory_is_not_available_to_new_services() {
        let stats = "{\"ID\":\"external\",\"MemUsage\":\"1.5GiB / 8GiB\"}\n{\"ID\":\"owned\",\"MemUsage\":\"512MiB / 8GiB\"}";
        assert_eq!(
            running_memory_bytes(stats, &BTreeSet::new()).unwrap(),
            2 * 1024 * 1024 * 1024
        );
        assert_eq!(
            running_memory_bytes(stats, &BTreeSet::from(["owned".into()])).unwrap(),
            1536 * 1024 * 1024
        );
        assert!(running_memory_bytes("invalid-json", &BTreeSet::new()).is_err());
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn unknown_memory_is_resampled_without_assuming_capacity_or_ignoring_target_changes() {
        let root = tempfile::tempdir().unwrap();
        let program = root.path().join("fake-docker");
        std::fs::write(
            &program,
            r#"#!/usr/bin/env python3
import json,sys
from pathlib import Path
root=Path(__file__).parent
scenario=(root/'scenario').read_text()
count=int((root/'count').read_text()) if (root/'count').exists() else 0
a=sys.argv[3:]
if a[0]=='info':
 info_count=int((root/'info-count').read_text()) if (root/'info-count').exists() else 0
 (root/'info-count').write_text(str(info_count+1))
 changed=(scenario=='target-change' and count) or (scenario=='sample-target-change' and info_count)
 info={'ID':'changed' if changed else 'daemon','OSType':'linux','MemTotal':8*1024**3}
 if scenario=='missing-total' and not count: del info['MemTotal']
 print(json.dumps(info))
elif a[:2]==['context','inspect']: print('[{"Endpoints":{"docker":{"Host":"unix:///fake.sock"}}}]')
elif a[0]=='stats':
 (root/'count').write_text(str(count+1))
 unknown=scenario in ['persistent','target-change'] or (scenario=='transient' and not count)
 print(json.dumps({'ID':'external','MemUsage':'unknown' if unknown else '256MiB / 8GiB'}))
else: sys.exit(1)
"#,
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        let runner = DockerRunner {
            target: DockerTarget {
                target_id: "target".into(),
                cli_path: program.to_str().unwrap().into(),
                context_name: "fake".into(),
                endpoint_kind: "unix".into(),
                daemon_id: "daemon".into(),
                os_type: "linux".into(),
                architecture: "arm64".into(),
            },
        };
        for (scenario, expected, attempts) in [
            (
                "transient",
                Ok((8u64 * 1024 - 256 - 512) * 1024 * 1024),
                "2",
            ),
            (
                "missing-total",
                Ok((8u64 * 1024 - 256 - 512) * 1024 * 1024),
                "2",
            ),
            ("persistent", Err("UnknownMemory"), "3"),
            ("target-change", Err("TargetChanged"), "1"),
            ("sample-target-change", Err("TargetChanged"), "0"),
        ] {
            std::fs::write(root.path().join("scenario"), scenario).unwrap();
            std::fs::write(root.path().join("count"), "0").unwrap();
            std::fs::write(root.path().join("info-count"), "0").unwrap();
            let result = available_daemon_memory(&runner, &BTreeSet::new())
                .await
                .map_err(|error| error.code);
            match expected {
                Ok(bytes) => assert_eq!(result.unwrap(), bytes),
                Err(code) => assert_eq!(result.unwrap_err(), code),
            }
            assert_eq!(
                std::fs::read_to_string(root.path().join("count")).unwrap(),
                attempts,
                "{scenario}"
            );
        }
    }
}
