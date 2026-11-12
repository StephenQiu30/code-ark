use crate::{
    bindings::verify_ownership,
    contracts::*,
    docker_cli::DockerRunner,
    orchestrator::{Engine, now},
    workspace::{error, random_id, reject_links},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};
pub struct VerifiedResetPlan {
    pub view: ResetPlan,
    observations: BTreeMap<String, Value>,
    data_dirs: Vec<PathBuf>,
}
pub fn verify_confirmation(
    plan: &ResetPlan,
    request: &ResetConfirmation,
    at: u64,
    fingerprint: &str,
) -> Result<(), ToolboxError> {
    if at >= plan.expires_at {
        return Err(error("ResetPlanExpired"));
    }
    if request.plan_id != plan.plan_id
        || request.fingerprint != plan.fingerprint
        || fingerprint != plan.fingerprint
        || request.typed_service_id != plan.service_id
    {
        return Err(error("ResetPlanChanged"));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reset_requires_fresh_unchanged_backend_plan_and_exact_service_text() {
        let p = ResetPlan {
            plan_id: "id".into(),
            target_id: "target".into(),
            instance_id: "i".into(),
            service_id: "redis".into(),
            exact_resource_ids: vec!["container:exact-id".into()],
            database_claims: vec![],
            impact: vec![],
            fingerprint: "fingerprint".into(),
            expires_at: 100,
        };
        let mut r = ResetConfirmation {
            plan_id: "id".into(),
            fingerprint: "fingerprint".into(),
            typed_service_id: "redis".into(),
        };
        verify_confirmation(&p, &r, 50, "fingerprint").unwrap();
        assert!(verify_confirmation(&p, &r, 101, "fingerprint").is_err());
        assert!(verify_confirmation(&p, &r, 50, "changed").is_err());
        r.typed_service_id = "other".into();
        assert!(verify_confirmation(&p, &r, 50, "fingerprint").is_err());
    }
}
fn fingerprint(observations: &BTreeMap<String, Value>) -> Result<String, ToolboxError> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(observations).map_err(|_| error("ResetUnavailable"))?)
    ))
}
fn walk_safe(path: &Path, count: &mut usize) -> Result<(), ToolboxError> {
    reject_links(path)?;
    let meta = std::fs::symlink_metadata(path).map_err(|_| error("ResetPathUnavailable"))?;
    if meta.file_type().is_symlink() {
        return Err(error("UnsafePath"));
    }
    *count += 1;
    if *count > 100000 {
        return Err(error("ResetTreeTooLarge"));
    }
    if meta.is_dir() {
        for entry in std::fs::read_dir(path).map_err(|_| error("ResetPathUnavailable"))? {
            walk_safe(
                &entry.map_err(|_| error("ResetPathUnavailable"))?.path(),
                count,
            )?;
        }
    }
    Ok(())
}
async fn observe(
    engine: &Engine,
    id: &str,
) -> Result<(BTreeMap<String, Value>, Vec<PathBuf>), ToolboxError> {
    let store = engine.store()?;
    let i = store.instance(id)?;
    let target = engine.target()?;
    if i.identity.target_id != target.target_id || i.identity.daemon_id != target.daemon_id {
        return Err(error("TargetChanged"));
    }
    if ["mysql", "pgsql"].contains(&i.identity.service_id.as_str()) {
        return Err(error("UnknownBusinessResources"));
    }
    if !i.claims.is_empty() {
        return Err(error("DatabaseClaimResetRequired"));
    }
    if i.containers.is_empty() {
        return Err(error("NoManagedResources"));
    }
    let runner = DockerRunner::connect(target.clone()).await?;
    let usage = crate::usage_index::UsageIndex::new(&engine.control.join("usage"))?;
    if !i.active_presets.is_empty() || !usage.consumers(&target.target_id, id)?.is_empty() {
        return Err(error("SharedDependencyInUse"));
    }
    let owned = i
        .containers
        .iter()
        .map(|r| r.container_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut observations = BTreeMap::new();
    for record in &i.containers {
        let raw = runner
            .run_read(&["container", "inspect", &record.container_id])
            .await?;
        let v: Value = serde_json::from_slice(&raw).map_err(|_| error("DockerOutputInvalid"))?;
        let c = &v[0];
        verify_ownership(c, record, &target)?;
        observations.insert(format!("container:{}",record.container_id),json!({"id":c["Id"],"image":c["Image"],"labels":c["Config"]["Labels"],"mounts":c["Mounts"],"ports":c["NetworkSettings"]["Ports"]}));
        for mount in c["Mounts"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|m| m["Type"] == "volume")
        {
            let name = mount["Name"]
                .as_str()
                .ok_or_else(|| error("OwnershipConflict"))?;
            let raw = runner.run_read(&["volume", "inspect", name]).await?;
            let raw: Value =
                serde_json::from_slice(&raw).map_err(|_| error("DockerOutputInvalid"))?;
            let v = &raw[0];
            verify_resource_labels(&v["Labels"], &i.identity)?;
            if v["Driver"] != "local" || v["Options"].as_object().is_some_and(|v| !v.is_empty()) {
                return Err(error("UnsupportedVolumeDriver"));
            }
            let users = runner
                .run_read(&[
                    "ps",
                    "-aq",
                    "--no-trunc",
                    "--filter",
                    &format!("volume={name}"),
                ])
                .await?;
            if std::str::from_utf8(&users)
                .map_err(|_| error("DockerOutputInvalid"))?
                .lines()
                .any(|id| !owned.contains(id))
            {
                return Err(error("SharedResourceInUse"));
            }
            observations.insert(format!("volume:{name}"), v.clone());
        }
        if let Some(networks) = c["NetworkSettings"]["Networks"].as_object() {
            for network in networks.values() {
                let network_id = network["NetworkID"]
                    .as_str()
                    .ok_or_else(|| error("OwnershipConflict"))?;
                let raw = runner.run_read(&["network", "inspect", network_id]).await?;
                let raw: Value =
                    serde_json::from_slice(&raw).map_err(|_| error("DockerOutputInvalid"))?;
                let n = &raw[0];
                if n["Labels"]["io.codeark.instance-id"] != id {
                    continue;
                }
                verify_resource_labels(&n["Labels"], &i.identity)?;
                if n["Containers"]
                    .as_object()
                    .is_some_and(|c| c.keys().any(|id| !owned.contains(id.as_str())))
                {
                    return Err(error("SharedResourceInUse"));
                }
                observations.insert(format!("network:{network_id}"),json!({"id":n["Id"],"name":n["Name"],"labels":n["Labels"],"created":n["Created"]}));
            }
        }
    }
    let stack = store.stack(&i.identity.service_id)?;
    crate::managed_data::verify(
        &stack,
        &engine.service(&i.identity.service_id)?,
        &i.identity,
    )?;
    let mut paths = vec![];
    for path in engine.service(&i.identity.service_id)?.data_paths {
        let path = stack.join(path);
        reject_links(&path)?;
        if !path.exists() {
            continue;
        }
        if !path.starts_with(&stack) || path == stack {
            return Err(error("UnsafePath"));
        }
        walk_safe(&path, &mut 0)?;
        let identity = json!(crate::managed_data::identity(&path)?);
        let token = format!(
            "path:{:x}",
            Sha256::digest(path.to_string_lossy().as_bytes())
        );
        observations.insert(token, identity);
        paths.push(path);
    }
    let mut shared_paths = paths.clone();
    for (key, v) in &observations {
        if key.starts_with("volume:") {
            shared_paths.push(PathBuf::from(
                v["Mountpoint"]
                    .as_str()
                    .ok_or_else(|| error("OwnershipConflict"))?,
            ));
        }
    }
    let owned_ids = owned.iter().map(|s| s.to_string()).collect();
    assert_no_bind_consumers(&runner, &shared_paths, &owned_ids).await?;
    Ok((observations, paths))
}
fn verify_resource_labels(labels: &Value, i: &InstanceIdentity) -> Result<(), ToolboxError> {
    for (key, value) in [
        ("io.codeark.workspace-id", &i.workspace_id),
        ("io.codeark.instance-id", &i.instance_id),
        ("io.codeark.service-id", &i.service_id),
    ] {
        if labels[key].as_str() != Some(value) {
            return Err(error("OwnershipConflict"));
        }
    }
    Ok(())
}
pub async fn plan(engine: &Engine, id: &str) -> Result<VerifiedResetPlan, ToolboxError> {
    if engine.busy.load(Ordering::Acquire) {
        return Err(error("ResourceBusy"));
    }
    let (observations, data_dirs) = observe(engine, id).await?;
    let i = engine.store()?.instance(id)?;
    let view = ResetPlan {
        plan_id: random_id()?,
        target_id: i.identity.target_id.clone(),
        instance_id: id.into(),
        service_id: i.identity.service_id.clone(),
        exact_resource_ids: observations.keys().cloned().collect(),
        database_claims: vec![],
        impact: vec![
            format!("停止并删除 {} 个已核验容器。", i.containers.len()),
            format!(
                "删除 {} 个本实例数据目录及清单中的专属卷；保留蓝图与 .env。",
                data_dirs.len()
            ),
            "此操作会永久删除当前服务数据。".into(),
        ],
        fingerprint: fingerprint(&observations)?,
        expires_at: now() + 5 * 60 * 1000,
    };
    Ok(VerifiedResetPlan {
        view,
        observations,
        data_dirs,
    })
}
pub async fn execute(
    engine: &Engine,
    plan: VerifiedResetPlan,
    request: ResetConfirmation,
) -> Result<OperationRecord, ToolboxError> {
    engine.verify_native_runtime()?;
    let _busy = engine.reserve()?;

    execute_locked(engine, plan, request).await
}
async fn execute_locked(
    engine: &Engine,
    plan: VerifiedResetPlan,
    request: ResetConfirmation,
) -> Result<OperationRecord, ToolboxError> {
    let store = engine.store()?;
    let mut i = store.instance(&plan.view.instance_id)?;
    let target = engine.target()?;
    let _lock = crate::locks::acquire(
        &engine.control.join("locks"),
        &[format!("{}/{}", target.daemon_id, i.identity.instance_id)],
    )?;
    if target.target_id != plan.view.target_id {
        return Err(error("TargetChanged"));
    }
    let (current, paths) = observe(engine, &i.identity.instance_id).await?;
    verify_confirmation(&plan.view, &request, now(), &fingerprint(&current)?)?;
    if current != plan.observations || paths != plan.data_dirs {
        return Err(error("ResetPlanChanged"));
    }
    let runner = DockerRunner::connect(target.clone()).await?;
    let mut op = OperationRecord {
        operation_id: random_id()?,
        target_id: target.target_id,
        instance_ids: vec![i.identity.instance_id.clone()],
        request_digest: plan.view.fingerprint.clone(),
        desired_generation: None,
        applied_generation: None,
        last_ready_generation: None,
        phase: "Running".into(),
        steps: vec![],
        created_resource_ids: vec![],
        error_code: None,
        allowed_actions: vec![],
        created_at: now(),
        seq: 1,
    };
    let operations = engine.operations()?;
    operations.persist(&op)?;
    let result = async {
        for record in &i.containers {
            runner.verify_identity().await?;
            let raw = runner
                .run_read(&["container", "inspect", &record.container_id])
                .await?;
            let v: Value =
                serde_json::from_slice(&raw).map_err(|_| error("DockerOutputInvalid"))?;
            let proof = verify_ownership(&v[0], record, &runner.target)?;
            runner
                .run_read(&["container", "rm", "-f", proof.container_id()])
                .await?;
        }
        for key in plan
            .observations
            .keys()
            .filter(|k| k.starts_with("volume:") || k.starts_with("network:"))
        {
            let (kind, id) = key
                .split_once(':')
                .ok_or_else(|| error("ResetPlanChanged"))?;
            runner.verify_identity().await?;
            let raw=runner.run_read(&[kind,"inspect",id]).await?;let raw:Value=serde_json::from_slice(&raw).map_err(|_|error("DockerOutputInvalid"))?;
            let observed=if kind=="network"{json!({"id":raw[0]["Id"],"name":raw[0]["Name"],"labels":raw[0]["Labels"],"created":raw[0]["Created"]})}else{raw[0].clone()};
            if plan.observations.get(key)!=Some(&observed){return Err(error("ResetPlanChanged"));}
            if kind=="volume"{let path=PathBuf::from(observed["Mountpoint"].as_str().ok_or_else(||error("OwnershipConflict"))?);assert_no_bind_consumers(&runner,&[path],&std::collections::BTreeSet::new()).await?;let users=runner.run_read(&["ps","-aq","--no-trunc","--filter",&format!("volume={id}")]).await?;if !users.is_empty(){return Err(error("SharedResourceInUse"));}}
            runner.run_read(&[kind, "rm", id]).await?;
        }
        let quarantine=store.stack(&i.identity.service_id)?.join(format!(".reset-{}",plan.view.plan_id));crate::workspace::private_dir(&quarantine)?;
        for (index,path) in plan.data_dirs.iter().enumerate(){
            assert_no_bind_consumers(&runner,std::slice::from_ref(path),&std::collections::BTreeSet::new()).await?;
            let token=format!("path:{:x}",Sha256::digest(path.to_string_lossy().as_bytes()));
            let current=crate::managed_data::identity(path)?;
            if plan.observations.get(&token)!=Some(&json!(current)){return Err(error("ResetPlanChanged"));}
            walk_safe(path,&mut 0)?;
            let moved=quarantine.join(index.to_string());std::fs::rename(path,&moved).map_err(|_|error("ResetPathUnavailable"))?;
            if crate::managed_data::identity(&moved)?!=current{if !path.exists(){let _=std::fs::rename(&moved,path);}return Err(error("ResetPlanChanged"));}
            if let Err(e)=assert_no_bind_consumers(&runner,&[path.clone(),moved.clone()],&std::collections::BTreeSet::new()).await{if !path.exists(){let _=std::fs::rename(&moved,path);}return Err(e);}
            walk_safe(&moved,&mut 0)?;std::fs::remove_dir_all(&moved).map_err(|_|error("ResetPathUnavailable"))?;
        }
        let _=std::fs::remove_dir(quarantine);
        i.containers.clear();
        i.applied = None;
        i.desired = None;
        i.last_ready_generation = None;
        i.phase = ServicePhase::Stopped;
        i.message_code = Some("ResetCompleted".into());
        store.save(&i)?;
        Ok::<(), ToolboxError>(())
    }
    .await;
    op.seq += 1;
    match result {
        Ok(()) => op.phase = "Stopped".into(),
        Err(e) => {
            op.phase = "RecoveryRequired".into();
            op.error_code = Some(e.code);
        }
    }
    operations.persist(&op)?;
    Ok(op)
}
pub fn paths_overlap(a: &Path, b: &Path) -> Result<bool, ToolboxError> {
    fn normal(path: &Path) -> Result<PathBuf, ToolboxError> {
        if !path.is_absolute()
            || path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(error("UnprovenBindTopology"));
        }
        let p = path.canonicalize().unwrap_or_else(|_| path.to_owned());
        #[cfg(windows)]
        let p = PathBuf::from(p.to_string_lossy().to_lowercase());
        Ok(p)
    }
    let a = normal(a)?;
    let b = normal(b)?;
    Ok(a.starts_with(&b) || b.starts_with(&a))
}
async fn assert_no_bind_consumers(
    runner: &DockerRunner,
    paths: &[PathBuf],
    owned: &std::collections::BTreeSet<String>,
) -> Result<(), ToolboxError> {
    runner.verify_identity().await?;
    let raw = runner.run_read(&["ps", "-aq", "--no-trunc"]).await?;
    let ids = std::str::from_utf8(&raw).map_err(|_| error("DockerOutputInvalid"))?;
    for id in ids.lines().filter(|id| !owned.contains(*id)) {
        if !crate::probes::valid_id(id) {
            return Err(error("DockerOutputInvalid"));
        }
        let raw = runner
            .run_read(&["container", "inspect", "--format", "{{json .Mounts}}", id])
            .await?;
        let mounts: Vec<Value> =
            serde_json::from_slice(&raw).map_err(|_| error("DockerOutputInvalid"))?;
        for mount in mounts.iter().filter(|m| m["Type"] == "bind") {
            let source = mount["Source"]
                .as_str()
                .ok_or_else(|| error("UnprovenBindTopology"))?;
            for path in paths {
                if paths_overlap(path, Path::new(source))? {
                    return Err(error("SharedBindMountInUse"));
                }
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod bind_tests {
    use super::*;
    #[test]
    fn bind_ancestors_descendants_and_exact_paths_are_shared_resources() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().canonicalize().unwrap();
        assert!(paths_overlap(&root.join("data"), &root).unwrap());
        assert!(paths_overlap(&root.join("data"), &root.join("data/subtree")).unwrap());
        assert!(paths_overlap(&root.join("data"), &root.join("data")).unwrap());
        assert!(!paths_overlap(&root.join("data"), &root.join("data-other")).unwrap());
        assert!(paths_overlap(&root, Path::new("../outside")).is_err());
    }
}
