//! Explicitly scoped real startup checks. Never falls back to the user's workspace.
use codeark_toolbox::{
    acceptance_guard::AcceptanceConfig,
    catalog::{load_embedded_catalog, resolve_startup_order},
    contracts::*,
    docker_cli::discover_target,
    orchestrator::Engine,
};
use std::{collections::BTreeSet, path::PathBuf, sync::Arc, time::Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: preset_startup <isolated-config> <service>...")?;
    let mut selected: Vec<String> = args.collect();
    let rebuild = selected.iter().any(|value| value == "--rebuild");
    let observe_live = selected.iter().any(|value| value == "--live");
    selected.retain(|value| !["--rebuild", "--live"].contains(&value.as_str()));
    if selected.is_empty() {
        return Err("explicit service selection required".into());
    }
    let config: AcceptanceConfig = serde_json::from_slice(&std::fs::read(path)?)?;
    let target = discover_target(Some(&config.cli_path), Some(&config.context_name)).await?;
    config.validate(&target)?;
    let catalog = load_embedded_catalog()?;
    let stop_only = selected == ["--stop"];
    let order = if stop_only {
        vec![]
    } else {
        resolve_startup_order(&catalog, &selected, &vec![])?
    };
    let engine = Arc::new(Engine::for_acceptance(
        &config,
        &target,
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources"),
    )?);
    *engine.target.write().map_err(|_| "target lock")? = Some(target.clone());
    engine.mark_interrupted_operations()?;
    let store = engine.store()?;
    if stop_only {
        let ids = store
            .list()?
            .iter()
            .filter(|i| !i.containers.is_empty())
            .map(|i| i.identity.instance_id.clone())
            .collect::<Vec<_>>();
        if !ids.is_empty() {
            let operation = engine
                .stop(StopRequest {
                    workspace_id: store.workspace.workspace_id.clone(),
                    target_id: target.target_id.clone(),
                    instance_ids: ids,
                    scene_id: None,
                })
                .await?;
            if operation.phase != "Stopped" {
                return Err(format!("scoped stop refused: {:?}", operation.error_code).into());
            }
        }
        println!("STOPPED verified isolated workspace containers; data retained");
        return Ok(());
    }
    let mut reserved = BTreeSet::new();
    for id in &order {
        let service = engine.service(id)?;
        let mut instance = engine.instance_for(&service, &target)?;
        if rebuild && let Some(effective) = instance.desired.as_mut() {
            let mut generation = [0u8; 16];
            getrandom::fill(&mut generation).map_err(|_| "random source unavailable")?;
            effective.generation_id = generation
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            effective.config_digest.clear();
            store.save(&instance)?;
        }
        if instance.desired.is_none() {
            let mut effective = engine.effective(&instance, &service, &target)?;
            let free = |port: u16| {
                !reserved.contains(&port)
                    && std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
                    && std::net::UdpSocket::bind(("127.0.0.1", port)).is_ok()
            };
            let nacos_main = if service.id == "nacos" {
                let last = config
                    .port_max
                    .checked_sub(1002)
                    .ok_or("isolated Nacos port range too small")?;
                let base = (config.port_min..=last)
                    .find(|base| {
                        [0, 1000, 1001, 1002]
                            .iter()
                            .all(|offset| free(base + offset))
                    })
                    .ok_or("no isolated Nacos coupled port group free")?;
                for offset in [0, 1000, 1001, 1002] {
                    reserved.insert(base + offset);
                }
                Some(base)
            } else {
                None
            };
            for mapping in &mut effective.ports {
                let port = if let Some(base) = nacos_main
                    .filter(|_| [8848, 9848, 9849, 9850].contains(&mapping.container_port))
                {
                    base + (mapping.container_port - 8848)
                } else {
                    (config.port_min..=config.port_max)
                        .find(|port| {
                            !reserved.contains(port)
                                && std::net::TcpListener::bind(("127.0.0.1", *port)).is_ok()
                                && std::net::UdpSocket::bind(("127.0.0.1", *port)).is_ok()
                        })
                        .ok_or("isolated port range exhausted")?
                };
                reserved.insert(port);
                let old = mapping.host_port;
                mapping.host_port = port;
                for endpoint in effective.endpoints.iter_mut().filter(|e| {
                    e.component_id == mapping.component_id
                        && e.port == old
                        && e.protocol == mapping.protocol
                }) {
                    endpoint.port = port;
                }
            }
            instance.desired = Some(effective);
            store.save(&instance)?;
        } else {
            for mapping in &instance.desired.as_ref().unwrap().ports {
                if !(config.port_min..=config.port_max).contains(&mapping.host_port)
                    || !reserved.insert(mapping.host_port)
                {
                    return Err("existing config escaped isolated port range".into());
                }
            }
        }
    }
    let mut failed = false;
    for id in selected {
        let request = StartRequest {
            workspace_id: store.workspace.workspace_id.clone(),
            target_id: target.target_id.clone(),
            service_ids: vec![id.clone()],
            scene_id: None,
            accept_unknown_capacity: true,
        };
        let mut previous = String::new();
        for pass in 1..=2 {
            println!("START {id} pass={pass}");
            let outcome = async {
                let op = engine.start(request.clone()).await?;
                let started = std::time::Instant::now();
                loop {
                    let current = engine.operations()?.load(&op.operation_id)?;
                    let progress = format!("{}:{}", current.phase, current.seq);
                    if progress != previous {
                        println!(
                            "PROGRESS {id} pass={pass} phase={} steps={} error={}",
                            current.phase,
                            current.steps.len(),
                            current.error_code.as_deref().unwrap_or("none")
                        );
                        previous = progress;
                    }
                    if !["Queued", "Running"].contains(&current.phase.as_str()) {
                        return if current.phase == "Ready" {
                            if observe_live {
                                // Discard startup evidence and exercise the same
                                // fresh background observation as the desktop.
                                engine.live_status.invalidate();
                                let observing = std::time::Instant::now();
                                loop {
                                    engine.live_status.request_poll(engine.clone());
                                    let snapshot = engine.snapshot(None)?;
                                    let live = snapshot.services.iter().find(|service| service.service_id == id)
                                        .ok_or_else(|| ToolboxError::new("UnknownService"))?;
                                    if live.phase == ServicePhase::Ready {
                                        println!("LIVE_PASS {id} pass={pass} fresh_protocol_observation=ready");
                                        break;
                                    }
                                    if [ServicePhase::Degraded, ServicePhase::Failed].contains(&live.phase) {
                                        return Err(ToolboxError::new(live.message_code.as_deref().unwrap_or("LiveReadinessFailed")));
                                    }
                                    if observing.elapsed() > Duration::from_secs(300) {
                                        return Err(ToolboxError::new("LiveReadinessTimeout"));
                                    }
                                    tokio::time::sleep(Duration::from_millis(500)).await;
                                }
                            }
                            Ok(())
                        } else {
                            Err(ToolboxError::new(
                                current.error_code.as_deref().unwrap_or("StartupFailed"),
                            ))
                        };
                    }
                    if started.elapsed() > Duration::from_secs(900) {
                        let _ = engine.cancel(&op.operation_id);
                        return Err(ToolboxError::new("AcceptanceTimeoutReconcileRequired"));
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
            .await;
            // Keep data and only stop instances owned by this isolated workspace.
            let ids = store
                .list()?
                .iter()
                .filter(|i| !i.containers.is_empty())
                .map(|i| i.identity.instance_id.clone())
                .collect::<Vec<_>>();
            if !ids.is_empty() {
                let stopped = engine
                    .stop(StopRequest {
                        workspace_id: store.workspace.workspace_id.clone(),
                        target_id: target.target_id.clone(),
                        instance_ids: ids,
                        scene_id: None,
                    })
                    .await?;
                if stopped.phase != "Stopped" {
                    return Err(format!("scoped cleanup refused: {:?}", stopped.error_code).into());
                }
            }
            match outcome {
                Ok(()) => println!(
                    "PASS {id} pass={pass} actual_protocol_probes=ready data_retained=true"
                ),
                Err(error) => {
                    println!("FAIL {id} pass={pass} code={}", error.code);
                    failed = true;
                    break;
                }
            }
        }
    }
    if failed {
        Err("one or more real startup checks failed".into())
    } else {
        Ok(())
    }
}
