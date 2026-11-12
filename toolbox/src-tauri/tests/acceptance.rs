use codeark_toolbox::{
    acceptance_guard::AcceptanceConfig,
    contracts::*,
    docker_cli::{DockerRunner, discover_target},
    orchestrator::Engine,
    workspace::open_workspace,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
fn load(path: Option<PathBuf>) -> Result<AcceptanceConfig, Box<dyn std::error::Error>> {
    let path = path
        .ok_or("CODE_ARK_ACCEPTANCE_CONFIG is required; no default daemon or workspace fallback")?;
    let bytes = std::fs::read(path)?;
    if bytes.len() > 64 * 1024 {
        return Err("acceptance configuration too large".into());
    }
    Ok(serde_json::from_slice(&bytes)?)
}
#[test]
fn isolation_guard() {
    assert!(load(None).is_err());
}
async fn complete(
    engine: &Engine,
    op: OperationRecord,
) -> Result<OperationRecord, Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    loop {
        let op = engine.operations()?.load(&op.operation_id)?;
        if !["Queued", "Running"].contains(&op.phase.as_str()) {
            if op.phase != "Ready" {
                return Err(format!("operation failed: {} {:?}", op.phase, op.error_code).into());
            }
            return Ok(op);
        }
        if started.elapsed() > Duration::from_secs(240) {
            let _ = engine.cancel(&op.operation_id);
            return Err("operation timed out; reconciliation required".into());
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}
async fn redis(port: u16, parts: &[&str]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port)).await?;
    let mut command = format!("*{}\r\n", parts.len());
    for p in parts {
        command.push_str(&format!("${}\r\n{}\r\n", p.len(), p));
    }
    stream.write_all(command.as_bytes()).await?;
    let mut out = vec![0; 4096];
    let n = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut out)).await??;
    out.truncate(n);
    Ok(out)
}
#[tokio::test]
#[ignore = "requires an explicit isolated config; creates and deletes only verified resources from that run"]
async fn redis_lifecycle_retains_data_and_resets_exact_owned_resources()
-> Result<(), Box<dyn std::error::Error>> {
    let config = load(std::env::var_os("CODE_ARK_ACCEPTANCE_CONFIG").map(PathBuf::from))?;
    let target = discover_target(Some(&config.cli_path), Some(&config.context_name)).await?;
    config.validate(&target)?;
    let workspace = open_workspace(&config.workspace_root)?;
    let engine = Arc::new(Engine::for_acceptance(
        &config,
        &target,
        Path::new(env!("CARGO_MANIFEST_DIR")).join("resources"),
    )?);
    *engine.target.write().map_err(|_| "target lock")? = Some(target.clone());
    let runner = DockerRunner::connect(target.clone()).await?;
    runner.verify_identity().await?;
    let service = engine.service("redis")?;
    let mut i = engine.instance_for(&service, &target)?;
    let port = (config.port_min..config.port_max)
        .find(|p| std::net::TcpListener::bind(("127.0.0.1", *p)).is_ok())
        .ok_or("no isolated port free")?;
    let mut effective = engine.effective(&i, &service, &target)?;
    effective.ports[0].host_port = port;
    effective.endpoints[0].port = port;
    i.desired = Some(effective);
    engine.store()?.save(&i)?;
    let request = StartRequest {
        workspace_id: workspace.workspace_id.clone(),
        target_id: target.target_id.clone(),
        service_ids: vec!["redis".into()],
        scene_id: None,
        accept_unknown_capacity: true,
    };
    let key = format!("codeark:acceptance:{}", config.run_id);
    let result = async {
        complete(&engine, engine.start(request.clone()).await?).await?;
        assert_eq!(
            redis(port, &["SET", &key, &config.run_id]).await?,
            b"+OK\r\n"
        );
        let env_path = engine.store()?.stack("redis")?.join(".env");
        let original_env = std::fs::read(&env_path)?;
        let original = engine.store()?.instance(&i.identity.instance_id)?;
        let changed_port = ((port + 1)..=config.port_max)
            .find(|p| std::net::TcpListener::bind(("127.0.0.1", *p)).is_ok())
            .ok_or("no second isolated port free")?;
        // Exercise a real occupied listener before consenting to a new generation.
        let occupied = std::net::TcpListener::bind(("127.0.0.1", changed_port))?;
        let mut candidate = port_draft(&engine, &i.identity.instance_id, changed_port)?;
        candidate.view.preflight = Some(
            engine
                .preflight_configuration(&i.identity.instance_id, &candidate.effective)
                .await?,
        );
        let rejected = codeark_toolbox::config_apply::apply(&engine, candidate, true).await;
        assert_eq!(
            rejected.err().ok_or("occupied port was accepted")?.code,
            "PreflightFailed"
        );
        assert_eq!(std::fs::read(&env_path)?, original_env);
        assert_eq!(
            engine
                .store()?
                .instance(&i.identity.instance_id)?
                .desired
                .unwrap()
                .generation_id,
            original
                .desired
                .as_ref()
                .ok_or("no desired generation")?
                .generation_id
        );
        drop(occupied);
        let mut candidate = port_draft(&engine, &i.identity.instance_id, changed_port)?;
        candidate.view.preflight = Some(
            engine
                .preflight_configuration(&i.identity.instance_id, &candidate.effective)
                .await?,
        );
        assert!(
            candidate
                .view
                .preflight
                .as_ref()
                .unwrap()
                .requires_capacity_consent
        );
        let rejected = codeark_toolbox::config_apply::apply(&engine, candidate, false).await;
        assert_eq!(
            rejected
                .err()
                .ok_or("missing capacity consent was accepted")?
                .code,
            "UnknownCapacity"
        );
        assert_eq!(std::fs::read(&env_path)?, original_env);
        let mut candidate = port_draft(&engine, &i.identity.instance_id, changed_port)?;
        candidate.view.preflight = Some(
            engine
                .preflight_configuration(&i.identity.instance_id, &candidate.effective)
                .await?,
        );
        // A harmless backend fixture makes the pending/live document distinction observable.
        candidate
            .document
            .push_str("\n# isolated acceptance candidate\n");
        complete(
            &engine,
            codeark_toolbox::config_apply::apply(&engine, candidate, true).await?,
        )
        .await?;
        assert!(std::fs::read_to_string(&env_path)?.ends_with("# isolated acceptance candidate\n"));
        let applied = engine.store()?.instance(&i.identity.instance_id)?;
        assert!(applied.pending_env.is_none());
        assert_eq!(applied.active_presets, original.active_presets);
        assert_eq!(
            redis(changed_port, &["GET", &key]).await?,
            format!("${}\r\n{}\r\n", config.run_id.len(), config.run_id).as_bytes()
        );
        let before = engine
            .store()?
            .instance(&i.identity.instance_id)?
            .containers[0]
            .container_id
            .clone();
        // Simulate a missing post-up registry write while preserving the durable intent.
        let mut lost = engine.store()?.instance(&i.identity.instance_id)?;
        assert!(!lost.intents.is_empty());
        lost.containers.clear();
        engine.store()?.save(&lost)?;
        let stopped = engine
            .stop(StopRequest {
                workspace_id: workspace.workspace_id.clone(),
                target_id: target.target_id.clone(),
                instance_ids: vec![i.identity.instance_id.clone()],
                scene_id: None,
            })
            .await?;
        assert_eq!(stopped.phase, "Stopped");
        assert!(
            tokio::net::TcpStream::connect(("127.0.0.1", changed_port))
                .await
                .is_err()
        );
        let mut value = engine.store()?.instance(&i.identity.instance_id)?;
        let mut next = value.applied.clone().ok_or("no applied config")?;
        let mut generation = [0u8; 16];
        getrandom::fill(&mut generation).map_err(|_| "random source unavailable")?;
        next.generation_id = generation.iter().map(|b| format!("{b:02x}")).collect();
        next.config_digest.clear();
        value.desired = Some(next);
        engine.store()?.save(&value)?;
        complete(&engine, engine.start(request).await?).await?;
        let current = engine.store()?.instance(&i.identity.instance_id)?;
        assert_ne!(before, current.containers[0].container_id);
        assert_eq!(
            redis(changed_port, &["GET", &key]).await?,
            format!("${}\r\n{}\r\n", config.run_id.len(), config.run_id).as_bytes()
        );
        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;
    let stopped = engine
        .stop(StopRequest {
            workspace_id: workspace.workspace_id,
            target_id: target.target_id,
            instance_ids: vec![i.identity.instance_id.clone()],
            scene_id: None,
        })
        .await?;
    if stopped.phase != "Stopped" {
        return Err(format!("cleanup stop failed: {:?}", stopped.error_code).into());
    }
    let plan = codeark_toolbox::reset::plan(&engine, &i.identity.instance_id).await?;
    let confirmation = ResetConfirmation {
        plan_id: plan.view.plan_id.clone(),
        fingerprint: plan.view.fingerprint.clone(),
        typed_service_id: "redis".into(),
    };
    let reset = codeark_toolbox::reset::execute(&engine, plan, confirmation).await?;
    assert_eq!(reset.phase, "Stopped");
    assert!(!engine.store()?.stack("redis")?.join("data").exists());
    result?;
    println!(
        "Redis lifecycle passed: isolated write/read, occupied-port refusal, capacity consent, configuration apply, registry-gap stop recovery, data-preserving recreate and exact reset. No installer acceptance implied."
    );
    Ok(())
}

fn port_draft(
    engine: &Engine,
    id: &str,
    port: u16,
) -> Result<codeark_toolbox::config_apply::Draft, ToolboxError> {
    let mut draft = codeark_toolbox::config_apply::draft(engine, id)?;
    let field = draft
        .view
        .fields
        .iter()
        .find(|f| f.key.starts_with("port:"))
        .ok_or_else(|| ToolboxError::new("MissingPort"))?
        .key
        .clone();
    let request = ConfigDraft {
        draft_id: draft.view.draft_id.clone(),
        instance_id: id.into(),
        base_generation: draft.view.base_generation.clone(),
        values: std::collections::BTreeMap::from([(field, port.to_string())]),
    };
    codeark_toolbox::config_apply::validate(engine, &mut draft, request)?;
    Ok(draft)
}
