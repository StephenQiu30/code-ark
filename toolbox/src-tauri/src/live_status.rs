use crate::{
    bindings::{OwnedContainerRecord, verify_ownership},
    contracts::{DockerTarget, ServicePhase, ServiceSpec, ToolboxError, WorkspaceIdentity},
    docker_cli::DockerRunner,
    env_store::{SecretValues, parse_env},
    orchestrator::{Engine, now},
    probes::{self, ProbeContext},
    runtime_state::{RuntimeInstance, RuntimeStore},
    workspace::{error, reject_links},
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs::File,
    io::{ErrorKind, Read},
    path::Path,
    sync::{Arc, Mutex, atomic::Ordering},
    time::{Duration, Instant},
};
use tokio::{sync::watch, task::JoinSet};
use zeroize::Zeroizing;

const REFRESH: Duration = Duration::from_secs(20);
const TTL: Duration = Duration::from_secs(55);
const PROBE_DEADLINE: Duration = Duration::from_secs(20);

#[derive(Clone, PartialEq, Eq)]
struct Basis {
    workspace: String,
    target: String,
    daemon: String,
    instance: String,
    service: String,
    project: String,
    generation: String,
    digest: String,
    desired: Option<String>,
    phase: ServicePhase,
    ids: BTreeSet<String>,
}
impl Basis {
    fn from_instance(i: &RuntimeInstance) -> Option<Self> {
        let id = &i.identity;
        let applied = i.applied.as_ref()?;
        if !matches!(
            i.phase,
            ServicePhase::Ready
                | ServicePhase::RunningUnverified
                | ServicePhase::Degraded
                | ServicePhase::Stopped
        ) || applied.instance_id != id.instance_id
            || applied.target_id != id.target_id
            || applied.generation_id.len() != 32
            || !applied.generation_id.bytes().all(|b| b.is_ascii_hexdigit())
            || i.containers.is_empty()
        {
            return None;
        }
        let mut ids = BTreeSet::new();
        for r in &i.containers {
            if !probes::valid_id(&r.container_id)
                || r.workspace_id != id.workspace_id
                || r.target_id != id.target_id
                || r.daemon_id != id.daemon_id
                || r.instance_id != id.instance_id
                || r.service_id != id.service_id
                || !ids.insert(r.container_id.clone())
            {
                return None;
            }
        }
        Some(Self {
            workspace: id.workspace_id.clone(),
            target: id.target_id.clone(),
            daemon: id.daemon_id.clone(),
            instance: id.instance_id.clone(),
            service: id.service_id.clone(),
            project: id.project_name.clone(),
            generation: applied.generation_id.clone(),
            digest: applied.config_digest.clone(),
            desired: i.desired.as_ref().map(|e| e.generation_id.clone()),
            phase: i.phase.clone(),
            ids,
        })
    }
}
type Snapshot = (ServicePhase, Option<String>, u64);
struct Sample {
    basis: Basis,
    value: Snapshot,
    observed: Instant,
}
#[derive(Default)]
struct Cache {
    epoch: u64,
    running: bool,
    last_batch: Option<Instant>,
    entries: BTreeMap<String, Sample>,
}
struct Inner {
    cache: Mutex<Cache>,
    invalidated: watch::Sender<u64>,
}
impl Default for Inner {
    fn default() -> Self {
        Self {
            cache: Mutex::default(),
            invalidated: watch::channel(0).0,
        }
    }
}
#[derive(Clone, Default)]
pub struct LiveStatus {
    inner: Arc<Inner>,
}

impl LiveStatus {
    pub fn invalidate(&self) {
        if let Ok(mut c) = self.inner.cache.lock() {
            c.epoch = c.epoch.wrapping_add(1);
            c.entries.clear();
            c.last_batch = None;
            self.inner.invalidated.send_replace(c.epoch);
        }
    }
    fn epoch(&self) -> u64 {
        self.inner.cache.lock().map_or(u64::MAX, |c| c.epoch)
    }
    fn record(
        &self,
        epoch: u64,
        i: &RuntimeInstance,
        phase: ServicePhase,
        code: Option<String>,
        timestamp: u64,
        observed: Instant,
    ) {
        if let Some(basis) = Basis::from_instance(i)
            && let Ok(mut c) = self.inner.cache.lock()
            && c.epoch == epoch
        {
            c.entries.insert(
                basis.instance.clone(),
                Sample {
                    basis,
                    value: (phase, code, timestamp),
                    observed,
                },
            );
        }
    }
    pub fn mark_ready(&self, i: &RuntimeInstance) {
        if i.phase == ServicePhase::Ready
            && i.applied
                .as_ref()
                .is_some_and(|e| Some(&e.generation_id) == i.last_ready_generation.as_ref())
        {
            self.record(
                self.epoch(),
                i,
                ServicePhase::Ready,
                i.message_code.clone(),
                now(),
                Instant::now(),
            );
        }
    }
    pub fn service_snapshot(
        &self,
        i: &RuntimeInstance,
        target: Option<&DockerTarget>,
    ) -> Option<Snapshot> {
        let target = target?;
        let basis = Basis::from_instance(i)?;
        if target.target_id != basis.target || target.daemon_id != basis.daemon {
            return None;
        }
        let c = self.inner.cache.lock().ok()?;
        let sample = c.entries.get(&basis.instance)?;
        (basis == sample.basis && sample.observed.elapsed() < TTL).then(|| sample.value.clone())
    }
    pub fn request_poll(&self, engine: Arc<Engine>) {
        if engine.busy.load(Ordering::Acquire) || engine.verify_native_runtime().is_err() {
            return;
        }
        let (Ok(handle), Ok(store), Ok(target)) = (
            tokio::runtime::Handle::try_current(),
            engine.store(),
            engine.target(),
        ) else {
            return;
        };
        let Ok(mut c) = self.inner.cache.lock() else {
            return;
        };
        if c.running
            || c.last_batch
                .is_some_and(|t| t.elapsed() < Duration::from_secs(2))
        {
            return;
        }
        let epoch = c.epoch;
        let mut cancelled = self.inner.invalidated.subscribe();
        c.running = true;
        c.last_batch = Some(Instant::now());
        drop(c);
        let live = self.clone();
        handle.spawn(async move{
            let _lease=PollLease(live.clone());
            tokio::select!{
                _=cancelled.changed()=>{},
                _=tokio::time::timeout(Duration::from_secs(25),live.poll(&engine,store,target,epoch))=>{}
            }
        });
    }
    fn current(
        &self,
        engine: &Engine,
        workspace: &WorkspaceIdentity,
        target: &DockerTarget,
        epoch: u64,
    ) -> bool {
        !engine.busy.load(Ordering::Acquire)
            && self.epoch() == epoch
            && engine.workspace.read().is_ok_and(|s| *s == *workspace)
            && engine.target().is_ok_and(|s| s == *target)
    }
    async fn poll(
        &self,
        engine: &Arc<Engine>,
        store: RuntimeStore,
        target: DockerTarget,
        epoch: u64,
    ) {
        let workspace = store.workspace.clone();
        let Ok(Ok(instances)) = tokio::task::spawn_blocking(move || store.list()).await else {
            return;
        };
        if !self.current(engine, &workspace, &target, epoch) {
            return;
        }
        let Ok(catalog) = crate::catalog::load_embedded_catalog() else {
            return;
        };
        let mut candidates = Vec::new();
        {
            let Ok(c) = self.inner.cache.lock() else {
                return;
            };
            for i in instances {
                let Some(basis) = Basis::from_instance(&i) else {
                    continue;
                };
                if basis.target != target.target_id
                    || basis.daemon != target.daemon_id
                    || basis.workspace != workspace.workspace_id
                {
                    continue;
                }
                let old = c.entries.get(&basis.instance).filter(|s| s.basis == basis);
                if old.is_some_and(|s| s.observed.elapsed() < REFRESH) {
                    continue;
                }
                if let Some(service) = catalog.services.iter().find(|s| s.id == basis.service) {
                    candidates.push((old.map(|s| s.observed), i, service.clone()));
                }
            }
        }
        candidates.sort_by_key(|(sampled, _, _)| *sampled);
        let mut workers = JoinSet::new();
        let (mut heavy, mut native) = (0, 0);
        for (_, instance, service) in candidates {
            if probes::requires_tool(&service) || service.id == "temporal" {
                if heavy == 1 {
                    continue;
                }
                heavy += 1;
            } else {
                if native == 3 {
                    continue;
                }
                native += 1;
            }
            let own = engine.clone();
            let endpoint = target.clone();
            let root = workspace.clone();
            workers.spawn(async move {
                let check = tokio::time::timeout(
                    PROBE_DEADLINE,
                    observe(&own, root, endpoint, &instance, &service),
                )
                .await;
                let sample = match check {
                    Ok(Ok(sample)) => sample,
                    Ok(Err(e)) => (ServicePhase::Degraded, Some(e.code)),
                    Err(_) => (ServicePhase::Degraded, Some("ProbeTimedOut".into())),
                };
                (instance, sample)
            });
        }
        while let Some(out) = workers.join_next().await {
            let Ok((i, (phase, code))) = out else {
                continue;
            };
            if !self.current(engine, &workspace, &target, epoch) {
                return;
            }
            // The persisted basis can also change in another process. service_snapshot checks it
            // on every read; recording this exact old basis can never validate a new generation.
            self.record(epoch, &i, phase, code, now(), Instant::now());
        }
    }
}
struct PollLease(LiveStatus);
impl Drop for PollLease {
    fn drop(&mut self) {
        if let Ok(mut c) = self.0.inner.cache.lock() {
            c.running = false;
        }
    }
}

fn process_evidence(role: &str, state: &Value) -> Result<bool, ToolboxError> {
    if state["OOMKilled"] != false || state["Error"] != "" || state["Paused"] != false {
        return Err(error("ContainerFailed"));
    }
    match (role, state["Status"].as_str(), state["Running"].as_bool()) {
        ("job", Some("exited"), Some(false)) if state["ExitCode"].as_i64() == Some(0) => Ok(true),
        ("job", _, _) => Err(error("JobIncomplete")),
        ("daemon", Some("running"), Some(true))
            if state["Health"].is_null() || state["Health"]["Status"] == "healthy" =>
        {
            Ok(true)
        }
        ("daemon", Some("exited"), Some(false))
            if matches!(state["ExitCode"].as_i64(), Some(0 | 130 | 143)) =>
        {
            Ok(false)
        }
        ("daemon", _, _) => Err(error("DaemonNotHealthy")),
        _ => Err(error("UnknownComponentRole")),
    }
}

#[derive(PartialEq, Eq)]
struct Processes {
    started: BTreeMap<String, String>,
    stopped: bool,
}
async fn inspect_processes(
    runner: &DockerRunner,
    instance: &RuntimeInstance,
    service: &ServiceSpec,
    effective: &crate::contracts::EffectiveConfig,
) -> Result<Processes, ToolboxError> {
    let records: BTreeMap<&str, &OwnedContainerRecord> = instance
        .containers
        .iter()
        .map(|r| (r.container_id.as_str(), r))
        .collect();
    if records.len() != service.components.len()
        || records.len() != instance.containers.len()
        || records.keys().any(|id| !probes::valid_id(id))
    {
        return Err(error("ComponentIdentityUnverified"));
    }
    // Project only ownership and process fields. Docker inspect can also contain plaintext environment and health logs.
    const TEMPLATE: &str = r#"{"Id":{{json .Id}},"Mounts":{{json .Mounts}},"Config":{"Labels":{{json .Config.Labels}}},"State":{"Status":{{json .State.Status}},"Running":{{json .State.Running}},"OOMKilled":{{json .State.OOMKilled}},"Paused":{{json .State.Paused}},"Error":{{if .State.Error}}"ContainerFailed"{{else}}""{{end}},"ExitCode":{{json .State.ExitCode}},"StartedAt":{{json .State.StartedAt}},"Health":{{if .State.Health}}{"Status":{{json .State.Health.Status}}}{{else}}null{{end}}}}"#;
    let mut args: Vec<OsString> = ["container", "inspect", "--format", TEMPLATE]
        .into_iter()
        .map(Into::into)
        .collect();
    args.extend(records.keys().map(OsString::from));
    let bytes = runner.run_args(&args, Duration::from_secs(5)).await?;
    let mut started = BTreeMap::new();
    let (mut alive, mut stopped) = (0, 0);
    for value in serde_json::Deserializer::from_slice(&bytes).into_iter::<Value>() {
        let v = value.map_err(|_| error("ProbeObservationInvalid"))?;
        let record = v["Id"]
            .as_str()
            .and_then(|s| records.get(s))
            .ok_or_else(|| error("ComponentIdentityUnverified"))?;
        verify_ownership(&v, record, &runner.target)?;
        let labels = &v["Config"]["Labels"];
        let component = service
            .components
            .iter()
            .find(|c| labels["com.docker.compose.service"] == c.id)
            .ok_or_else(|| error("ComponentIdentityUnverified"))?;
        if labels["io.codeark.generation-id"] != effective.generation_id
            || labels["com.docker.compose.project"] != instance.identity.project_name
        {
            return Err(error("GenerationMismatch"));
        }
        let time = v["State"]["StartedAt"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| error("ProbeObservationInvalid"))?;
        if started.insert(component.id.clone(), time.into()).is_some() {
            return Err(error("ComponentIdentityUnverified"));
        }
        let active = process_evidence(&component.role, &v["State"])?;
        if component.role == "daemon" {
            if active { alive += 1 } else { stopped += 1 }
        }
    }
    if started.len() != records.len() || alive + stopped == 0 {
        return Err(error("ComponentIdentityUnverified"));
    }
    if stopped > 0 && alive > 0 {
        return Err(error("DaemonNotHealthy"));
    }
    Ok(Processes {
        started,
        stopped: alive == 0,
    })
}

fn load_env(stack: &Path, generation: &str) -> Result<SecretValues, ToolboxError> {
    if generation.len() != 32 || !generation.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(error("GenerationMismatch"));
    }
    let snapshot = stack
        .join(".runtime")
        .join(generation)
        .join("effective.env");
    reject_links(&snapshot)?;
    let file = match File::open(snapshot) {
        Ok(f) => f,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            // Legacy managed stacks have only the private env in the workspace's materialized service directory.
            let path = stack.join(".env");
            reject_links(&path)?;
            File::open(path).map_err(|_| error("AppliedEnvironmentUnavailable"))?
        }
        Err(_) => return Err(error("AppliedEnvironmentUnavailable")),
    };
    if !file
        .metadata()
        .is_ok_and(|m| m.is_file() && m.len() <= 1024 * 1024)
    {
        return Err(error("AppliedEnvironmentUnavailable"));
    }
    let mut text = Zeroizing::new(String::new());
    file.take(1024 * 1024 + 1)
        .read_to_string(&mut text)
        .map_err(|_| error("AppliedEnvironmentUnavailable"))?;
    Ok(SecretValues(parse_env(&text)?))
}

async fn observe(
    engine: &Engine,
    workspace: WorkspaceIdentity,
    target: DockerTarget,
    instance: &RuntimeInstance,
    service: &ServiceSpec,
) -> Result<(ServicePhase, Option<String>), ToolboxError> {
    let effective = instance
        .applied
        .as_ref()
        .ok_or_else(|| error("NoAppliedConfiguration"))?;
    let runner = DockerRunner::connect(target).await?;
    let before = inspect_processes(&runner, instance, service, effective).await?;
    if before.stopped {
        return Ok((ServicePhase::Stopped, None));
    }
    if !service.dependencies.is_empty() {
        return Err(error("DependencyInitializationRequired"));
    }
    let stack = RuntimeStore::new(workspace).stack(&service.id)?;
    let generation = effective.generation_id.clone();
    let secrets = tokio::task::spawn_blocking(move || load_env(&stack, &generation))
        .await
        .map_err(|_| error("AppliedEnvironmentUnavailable"))??;
    let tool = if probes::requires_tool(service) {
        Some(engine.tool_image(&runner.target)?)
    } else {
        None
    };
    let results = probes::evaluate(&ProbeContext {
        runner: &runner,
        instance: &instance.identity,
        effective,
        service,
        secrets: &secrets,
        bindings: &instance.bindings,
        tool_image: tool.as_ref(),
        resources_root: &engine.resources.join("probe-clients"),
    })
    .await?;
    if !probes::ready(service, &results) {
        return Ok((
            ServicePhase::Degraded,
            Some(
                results
                    .iter()
                    .find_map(|r| r.error_code.clone())
                    .unwrap_or_else(|| "ProbeFailed".into()),
            ),
        ));
    }
    if before != inspect_processes(&runner, instance, service, effective).await? {
        return Err(error("ComponentChanged"));
    }
    Ok((
        ServicePhase::Ready,
        results.into_iter().find_map(|r| r.warning_code),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bindings::OwnedContainerRecord,
        contracts::{EffectiveConfig, InstanceIdentity},
    };
    fn sample() -> (RuntimeInstance, DockerTarget) {
        let identity = InstanceIdentity {
            workspace_id: "w".into(),
            instance_id: "i".into(),
            service_id: "redis".into(),
            project_name: "ca-test".into(),
            target_id: "t".into(),
            daemon_id: "d".into(),
        };
        let effective = EffectiveConfig {
            generation_id: "1".repeat(32),
            target_id: "t".into(),
            instance_id: "i".into(),
            ports: vec![],
            endpoints: vec![],
            secret_refs: vec![],
            config_digest: "a".into(),
            image_artifacts: vec![],
        };
        let target = DockerTarget {
            target_id: "t".into(),
            daemon_id: "d".into(),
            architecture: "arm64".into(),
            os_type: "linux".into(),
            context_name: "default".into(),
            cli_path: "docker".into(),
            endpoint_kind: "unix".into(),
        };
        let container = OwnedContainerRecord {
            container_id: "1".repeat(64),
            daemon_id: "d".into(),
            target_id: "t".into(),
            workspace_id: "w".into(),
            instance_id: "i".into(),
            service_id: "redis".into(),
            blueprint_sha: "a".into(),
            bind_sources: vec![],
        };
        let instance = RuntimeInstance {
            identity,
            phase: ServicePhase::Ready,
            desired: Some(effective.clone()),
            last_ready_generation: Some(effective.generation_id.clone()),
            applied: Some(effective),
            containers: vec![container],
            claims: vec![],
            active_presets: vec![],
            bindings: vec![],
            message_code: Some("WaitingForApplicationClient".into()),
            intents: vec![],
            applied_networks: vec![],
            pending_env: None,
        };
        (instance, target)
    }
    #[test]
    fn ready_evidence_is_bound_to_applied_identity_and_phase() {
        let live = LiveStatus::default();
        let (i, t) = sample();
        assert!(live.service_snapshot(&i, Some(&t)).is_none());
        live.mark_ready(&i);
        let (phase, warning, time) = live
            .service_snapshot(&i, Some(&t))
            .expect("just-probed instance");
        assert_eq!(phase, ServicePhase::Ready);
        assert!(time > 0);
        assert_eq!(warning.as_deref(), Some("WaitingForApplicationClient"));
        for change in 0..6 {
            let mut changed = i.clone();
            let mut target = t.clone();
            match change {
                0 => changed.applied.as_mut().unwrap().generation_id = "2".repeat(32),
                1 => changed.containers[0].container_id = "2".repeat(64),
                2 => changed.identity.workspace_id = "other".into(),
                3 => target.daemon_id = "other".into(),
                4 => changed.phase = ServicePhase::Stopped,
                _ => changed.identity.instance_id = "other".into(),
            }
            assert!(live.service_snapshot(&changed, Some(&target)).is_none());
        }
        assert!(live.service_snapshot(&i, None).is_none());
    }
    #[test]
    fn invalidation_rejects_late_completion_and_expired_samples() {
        let live = LiveStatus::default();
        let (i, t) = sample();
        let old = live.epoch();
        live.invalidate();
        live.record(old, &i, ServicePhase::Ready, None, 42, Instant::now());
        assert!(live.service_snapshot(&i, Some(&t)).is_none());
        live.record(
            live.epoch(),
            &i,
            ServicePhase::Ready,
            None,
            42,
            Instant::now() - TTL,
        );
        assert!(live.service_snapshot(&i, Some(&t)).is_none());
        live.record(
            live.epoch(),
            &i,
            ServicePhase::Degraded,
            Some("ProtocolUnavailable".into()),
            43,
            Instant::now(),
        );
        assert_eq!(
            live.service_snapshot(&i, Some(&t)),
            Some((
                ServicePhase::Degraded,
                Some("ProtocolUnavailable".into()),
                43
            ))
        );
    }
    #[test]
    fn a_failed_job_and_a_failed_healthcheck_invalidate_evidence() {
        let normal = serde_json::json!({"Status":"running","Running":true,"OOMKilled":false,"Paused":false,"Error":"","Health":{"Status":"healthy"}});
        assert!(process_evidence("daemon", &normal).unwrap());
        let mut unhealthy = normal.clone();
        unhealthy["Health"]["Status"] = "unhealthy".into();
        assert!(process_evidence("daemon", &unhealthy).is_err());
        let exited = serde_json::json!({"Status":"exited","Running":false,"ExitCode":0,"Error":"","OOMKilled":false,"Paused":false});
        assert!(!process_evidence("daemon", &exited).unwrap());
        assert!(process_evidence("job", &exited).unwrap());
        let mut failed = exited;
        failed["ExitCode"] = 1.into();
        assert!(process_evidence("job", &failed).is_err());
    }
}
