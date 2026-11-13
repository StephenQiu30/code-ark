use crate::{
    blueprint::*,
    contracts::*,
    docker_cli::DockerRunner,
    image_store::{ImageContext, ensure_service_image},
    operation_store::OperationStore,
    runtime_state::*,
    workspace::*,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
#[derive(Serialize, Deserialize)]
struct ImageLock {
    sources: Vec<ImageArtifactRef>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestEnvelope {
    schema_version: u32,
    request: StartRequest,
    generations: BTreeMap<String, String>,
    preserve_scope: bool,
}
#[derive(Clone)]
pub struct Engine {
    pub workspace: Arc<RwLock<WorkspaceIdentity>>,
    pub control: PathBuf,
    pub resources: PathBuf,
    pub target: Arc<RwLock<Option<DockerTarget>>>,
    pub busy: Arc<AtomicBool>,
    pub environment_error: Arc<RwLock<Option<String>>>,
    native_test_scope: bool,
    pub live_status: crate::live_status::LiveStatus,
    pub cancellations: Arc<Mutex<BTreeMap<String, Arc<AtomicBool>>>>,
    pub emit: Arc<dyn Fn(StepProgressEvent) + Send + Sync>,
}
impl Engine {
    pub fn new(
        workspace: WorkspaceIdentity,
        control: PathBuf,
        resources: PathBuf,
    ) -> Result<Self, ToolboxError> {
        private_dir(&control)?;
        Ok(Self {
            workspace: Arc::new(RwLock::new(workspace)),
            control,
            resources,
            target: Arc::new(RwLock::new(None)),
            busy: Arc::new(AtomicBool::new(false)),
            environment_error: Arc::new(RwLock::new(None)),
            native_test_scope: false,
            live_status: crate::live_status::LiveStatus::default(),
            cancellations: Arc::new(Mutex::new(BTreeMap::new())),
            emit: Arc::new(|_| {}),
        })
    }
    pub fn store(&self) -> Result<RuntimeStore, ToolboxError> {
        Ok(RuntimeStore::new(
            self.workspace
                .read()
                .map_err(|_| error("StateUnavailable"))?
                .clone(),
        ))
    }
    pub fn operations(&self) -> Result<OperationStore, ToolboxError> {
        OperationStore::new(&Path::new(&self.store()?.workspace.root).join("operations"))
    }
    pub fn target(&self) -> Result<DockerTarget, ToolboxError> {
        self.target
            .read()
            .map_err(|_| error("StateUnavailable"))?
            .clone()
            .ok_or_else(|| error("EnvironmentNotDetected"))
    }
    pub fn service(&self, id: &str) -> Result<ServiceSpec, ToolboxError> {
        crate::catalog::load_embedded_catalog()?
            .services
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| error("UnknownService"))
    }
    pub fn images(
        &self,
        service: &ServiceSpec,
        target: &DockerTarget,
    ) -> Result<Vec<ImageArtifactRef>, ToolboxError> {
        let lock: ImageLock = serde_json::from_str(include_str!("../../catalog/images.lock.json"))
            .map_err(|_| error("InvalidImageLock"))?;
        let platform = format!(
            "linux/{}",
            crate::docker_cli::normalize_architecture(&target.architecture)?
        );
        service
            .image_refs
            .iter()
            .map(|id| {
                let image = lock
                    .sources
                    .iter()
                    .find(|s| s.source_id == *id && s.platform == platform)
                    .cloned()
                    .ok_or_else(|| error("UnresolvedImageArtifact"))?;
                crate::image_store::validate_image_ref(&image, &target.architecture)?;
                Ok(image)
            })
            .collect()
    }
    pub fn tool_image(&self, target: &DockerTarget) -> Result<ImageArtifactRef, ToolboxError> {
        let service = ServiceSpec {
            image_refs: vec!["probe-jre17".into()],
            ..self.service("redis")?
        };
        self.images(&service, target)?
            .into_iter()
            .next()
            .ok_or_else(|| error("ProbeImageUnavailable"))
    }
    pub fn snapshot(
        &self,
        environment_error: Option<String>,
    ) -> Result<DashboardSnapshot, ToolboxError> {
        let store = self.store()?;
        let instances = store.list()?;
        let target = self
            .target
            .read()
            .map_err(|_| error("StateUnavailable"))?
            .clone();
        let bindings = self.load_bindings()?;
        let services = crate::catalog::load_embedded_catalog()?
            .services
            .into_iter()
            .map(|s| {
                if let Some(binding) = bindings.iter().find(|b| {
                    b.engine == s.id
                        && target.as_ref().is_some_and(|t| {
                            t.target_id == b.target_id && t.daemon_id == b.daemon_id
                        })
                }) && !instances
                    .iter()
                    .any(|i| i.identity.service_id == s.id && !i.containers.is_empty())
                {
                    return ServiceSnapshot {
                        service_id: s.id,
                        instance_id: Some(format!("binding-{}", binding.binding_id)),
                        phase: ServicePhase::ExternalObserve,
                        ownership: "ExternalConnected".into(),
                        desired_generation: None,
                        applied_generation: None,
                        last_ready_generation: None,
                        message_code: None,
                    };
                }
                if let Some(i) = instances.iter().find(|i| i.identity.service_id == s.id) {
                    let observation = self.live_status.service_snapshot(i, target.as_ref());
                    let phase = observation
                        .as_ref()
                        .map(|v| v.0.clone())
                        .unwrap_or_else(|| {
                            if i.phase == ServicePhase::Ready {
                                ServicePhase::RunningUnverified
                            } else {
                                i.phase.clone()
                            }
                        });
                    let message = observation
                        .as_ref()
                        .map(|v| v.1.clone())
                        .unwrap_or_else(|| {
                            if i.phase == ServicePhase::Ready {
                                Some("ReadinessRecheckRequired".into())
                            } else {
                                i.message_code.clone()
                            }
                        });
                    let last_ready = if phase == ServicePhase::Ready {
                        i.applied.as_ref().map(|e| e.generation_id.clone())
                    } else {
                        i.last_ready_generation.clone()
                    };
                    ServiceSnapshot {
                        service_id: s.id,
                        instance_id: Some(i.identity.instance_id.clone()),
                        phase,
                        ownership: "Managed".into(),
                        desired_generation: i.desired.as_ref().map(|e| e.generation_id.clone()),
                        applied_generation: i.applied.as_ref().map(|e| e.generation_id.clone()),
                        last_ready_generation: last_ready,
                        message_code: message,
                    }
                } else {
                    ServiceSnapshot {
                        service_id: s.id,
                        instance_id: None,
                        phase: ServicePhase::Uninitialized,
                        ownership: "None".into(),
                        desired_generation: None,
                        applied_generation: None,
                        last_ready_generation: None,
                        message_code: None,
                    }
                }
            })
            .collect();
        Ok(DashboardSnapshot {
            workspace: store.workspace,
            target,
            services,
            operations: self.operations()?.list()?,
            bindings,
            sampled_at: now(),
            environment_error: environment_error.or(self
                .environment_error
                .read()
                .map_err(|_| error("StateUnavailable"))?
                .clone()),
        })
    }
    pub fn load_bindings(&self) -> Result<Vec<DatabaseBinding>, ToolboxError> {
        let p = Path::new(&self.store()?.workspace.root).join("bindings.json");
        if p.exists() {
            read_json(&p)
        } else {
            Ok(vec![])
        }
    }
    pub async fn detect(&self) -> Result<DashboardSnapshot, ToolboxError> {
        let _busy = self.reserve()?;
        let result = async {
            let target = crate::docker_cli::discover_target(None, None).await?;
            let runner = DockerRunner::connect(target.clone()).await?;
            *self.target.write().map_err(|_| error("StateUnavailable"))? = Some(target);
            *self
                .environment_error
                .write()
                .map_err(|_| error("StateUnavailable"))? = None;
            for mut i in self.store()?.list()? {
                if i.identity.target_id != runner.target.target_id {
                    continue;
                }
                match self.reconcile_runtime(&runner, &mut i).await {
                    Ok(state) => {
                        i.phase = recovered_phase(&i, state.any_running);
                        i.message_code = if state.any_running {
                            Some("ReadinessRecheckRequired".into())
                        } else {
                            None
                        };
                    }
                    Err(_) => {
                        i.phase = ServicePhase::AwaitingAction;
                        i.message_code = Some("RecoveryRequired".into());
                    }
                }
                self.store()?.save(&i)?;
            }
            self.recover_interrupted_operations(&runner).await?;
            Ok::<(), ToolboxError>(())
        }
        .await;
        if let Err(e) = result {
            self.record_detection_failure(&e.code)?;
        }
        self.snapshot(None)
    }
    pub(crate) fn reserve(&self) -> Result<BusyLease, ToolboxError> {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| error("ResourceBusy"))?;
        self.live_status.invalidate();
        Ok(BusyLease::new(self.busy.clone()))
    }
    pub async fn apply_configuration(
        self: &Arc<Self>,
        draft: crate::config_apply::Draft,
        consent: bool,
    ) -> Result<OperationRecord, ToolboxError> {
        let i = self.store()?.instance(&draft.view.instance_id)?;
        let request = StartRequest {
            workspace_id: i.identity.workspace_id,
            target_id: i.identity.target_id,
            service_ids: vec![i.identity.service_id],
            scene_id: None,
            accept_unknown_capacity: consent,
        };
        self.start_internal(request, Some(draft), true).await
    }
    pub fn record_detection_failure(&self, code: &str) -> Result<(), ToolboxError> {
        *self.target.write().map_err(|_| error("StateUnavailable"))? = None;
        *self
            .environment_error
            .write()
            .map_err(|_| error("StateUnavailable"))? = Some(code.into());
        Ok(())
    }
    pub fn for_acceptance(
        config: &crate::acceptance_guard::AcceptanceConfig,
        target: &DockerTarget,
        resources: PathBuf,
    ) -> Result<Self, ToolboxError> {
        config.validate(target)?;
        let mut engine = Self::new(
            open_workspace(&config.workspace_root)?,
            config.control_root.clone(),
            resources,
        )?;
        engine.native_test_scope = true;
        Ok(engine)
    }
    pub(crate) fn verify_native_runtime(&self) -> Result<(), ToolboxError> {
        if !cfg!(all(target_os = "macos", target_arch = "aarch64")) && !self.native_test_scope {
            return Err(error("NativeRuntimeAcceptanceRequired"));
        }
        Ok(())
    }
    pub fn instance_for(
        &self,
        service: &ServiceSpec,
        target: &DockerTarget,
    ) -> Result<RuntimeInstance, ToolboxError> {
        let store = self.store()?;
        if let Some(i) = store
            .list()?
            .into_iter()
            .find(|i| i.identity.service_id == service.id)
        {
            if i.identity.target_id != target.target_id || i.identity.daemon_id != target.daemon_id
            {
                return Err(error("WorkspaceTargetConflict"));
            }
            return Ok(i);
        }
        let identity = allocate_instance(&store.workspace, service, target)?;
        let value = RuntimeInstance {
            identity,
            phase: ServicePhase::Uninitialized,
            desired: None,
            applied: None,
            last_ready_generation: None,
            containers: vec![],
            claims: vec![],
            active_presets: vec![],
            bindings: vec![],
            message_code: None,
            intents: vec![],
            applied_networks: vec![],
            pending_env: None,
        };
        store.save(&value)?;
        Ok(value)
    }
    pub fn effective(
        &self,
        instance: &RuntimeInstance,
        service: &ServiceSpec,
        target: &DockerTarget,
    ) -> Result<EffectiveConfig, ToolboxError> {
        if let Some(e) = &instance.desired {
            return Ok(e.clone());
        }
        Ok(EffectiveConfig {
            generation_id: random_id()?,
            target_id: target.target_id.clone(),
            instance_id: instance.identity.instance_id.clone(),
            ports: service.ports.clone(),
            endpoints: service
                .ports
                .iter()
                .map(|p| Endpoint {
                    label: format!("{}:{}", p.component_id, p.container_port),
                    component_id: p.component_id.clone(),
                    host: "127.0.0.1".into(),
                    port: p.host_port,
                    protocol: p.protocol.clone(),
                })
                .collect(),
            secret_refs: service
                .env_schema
                .iter()
                .filter(|f| f.sensitive)
                .map(|f| SecretRef { key: f.key.clone() })
                .collect(),
            config_digest: String::new(),
            image_artifacts: self.images(service, target)?,
        })
    }
    pub async fn preflight(&self, id: &str) -> Result<PreflightReport, ToolboxError> {
        self.verify_native_runtime()?;
        let target = self.target()?;
        let runner = DockerRunner::connect(target.clone()).await?;
        let service = self.service(id)?;
        let instance = self.instance_for(&service, &target)?;
        let global = self.load_bindings()?;
        if service.dependencies.iter().any(|dependency| {
            global
                .iter()
                .any(|b| b.engine == *dependency && b.target_id == target.target_id)
                && !instance.bindings.iter().any(|b| b.engine == *dependency)
        }) {
            return Err(error("DependencyBindingRequired"));
        }
        let mut external: Vec<DatabaseBinding> = instance
            .bindings
            .iter()
            .filter(|b| b.lifecycle_owner != "Managed")
            .cloned()
            .collect();
        if instance.containers.is_empty() {
            external.extend(global.into_iter().filter(|b| {
                b.engine == id && b.target_id == target.target_id && b.daemon_id == target.daemon_id
            }));
        }
        if external.iter().any(|b| b.engine == id) {
            return Ok(PreflightReport {
                issues: vec![],
                capacity: crate::storage_probe::capacity(
                    Path::new(&self.store()?.workspace.root),
                    None,
                )?,
                required_memory_bytes: 0,
                daemon_memory_bytes: serde_json::from_slice::<Value>(
                    &runner.run_read(&["info", "--format", "{{json .}}"]).await?,
                )
                .map_err(|_| error("DockerOutputInvalid"))?["MemTotal"]
                    .as_u64()
                    .ok_or_else(|| error("UnknownMemory"))?,
                requires_capacity_consent: false,
            });
        }
        let order = crate::catalog::resolve_startup_order(
            &crate::catalog::load_embedded_catalog()?,
            &[id.into()],
            &external,
        )?;
        let mut combined: Option<PreflightReport> = None;
        for id in order {
            let service = self.service(&id)?;
            let instance = self.instance_for(&service, &target)?;
            if ["mysql", "pgsql"].contains(&service.id.as_str())
                && instance.containers.is_empty()
                && self
                    .database_candidates(&runner)
                    .await?
                    .iter()
                    .any(|candidate| candidate.engine == service.id)
            {
                return Err(error("ExistingDatabaseBindingRequired"));
            }
            let effective = self.effective(&instance, &service, &target)?;
            let (ports, ids) = self.owned(&runner, &instance).await?;
            let report = crate::preflight::run_preflight(
                &runner,
                &service,
                &effective,
                Path::new(&self.store()?.workspace.root),
                &ports,
                &ids,
            )
            .await?;
            if let Some(total) = &mut combined {
                total.required_memory_bytes = total
                    .required_memory_bytes
                    .saturating_add(report.required_memory_bytes);
                total.daemon_memory_bytes =
                    total.daemon_memory_bytes.min(report.daemon_memory_bytes);
                total.requires_capacity_consent |= report.requires_capacity_consent;
                total.issues.extend(report.issues);
            } else {
                combined = Some(report);
            }
        }
        let mut report = combined.ok_or_else(|| error("UnknownService"))?;
        if report.required_memory_bytes > report.daemon_memory_bytes
            && !report.issues.iter().any(|i| i.code == "InsufficientMemory")
        {
            report.issues.push(PreflightIssue {
                code: "InsufficientMemory".into(),
                field: None,
                severity: "error".into(),
            });
        }
        Ok(report)
    }
    pub async fn preflight_configuration(
        &self,
        instance_id: &str,
        effective: &EffectiveConfig,
    ) -> Result<PreflightReport, ToolboxError> {
        self.verify_native_runtime()?;
        if self.busy.load(Ordering::Acquire) {
            return Err(error("ResourceBusy"));
        }
        let target = self.target()?;
        let i = self.store()?.instance(instance_id)?;
        if effective.instance_id != i.identity.instance_id
            || effective.target_id != target.target_id
        {
            return Err(error("TargetChanged"));
        }
        let service = self.service(&i.identity.service_id)?;
        let runner = DockerRunner::connect(target).await?;
        let (ports, ids) = self.owned(&runner, &i).await?;
        crate::preflight::run_preflight(
            &runner,
            &service,
            effective,
            Path::new(&self.store()?.workspace.root),
            &ports,
            &ids,
        )
        .await
    }
    async fn owned(
        &self,
        runner: &DockerRunner,
        instance: &RuntimeInstance,
    ) -> Result<(BTreeSet<(u16, String)>, BTreeSet<String>), ToolboxError> {
        let mut ports = BTreeSet::new();
        let mut ids = BTreeSet::new();
        for record in &instance.containers {
            let raw = runner
                .run_read(&["container", "inspect", &record.container_id])
                .await?;
            let v: Value =
                serde_json::from_slice(&raw).map_err(|_| error("DockerOutputInvalid"))?;
            crate::bindings::verify_ownership(&v[0], record, &runner.target)?;
            if v[0]["State"]["Running"] == true {
                ids.insert(record.container_id.clone());
                if let Some(e) = &instance.applied {
                    let component = v[0]["Config"]["Labels"]["com.docker.compose.service"]
                        .as_str()
                        .ok_or_else(|| error("OwnershipConflict"))?;
                    for p in e.ports.iter().filter(|p| p.component_id == component) {
                        let actual = &v[0]["NetworkSettings"]["Ports"]
                            [format!("{}/{}", p.container_port, p.protocol)];
                        if actual.as_array().is_some_and(|a| {
                            a.iter().any(|m| {
                                m["HostIp"] == p.host_ip
                                    && m["HostPort"]
                                        .as_str()
                                        .and_then(|value| value.parse::<u16>().ok())
                                        == Some(p.host_port)
                            })
                        }) {
                            ports.insert((p.host_port, p.protocol.clone()));
                        }
                    }
                }
            }
        }
        Ok((ports, ids))
    }
    fn progress(
        &self,
        op: &mut OperationRecord,
        service: &str,
        step: &str,
        phase: &str,
        message: &str,
    ) -> Result<(), ToolboxError> {
        op.seq += 1;
        if let Some(s) = op
            .steps
            .iter_mut()
            .find(|s| s.step_id == format!("{service}/{step}"))
        {
            s.phase = phase.into();
            s.message_code = message.into();
            s.elapsed_ms = now().saturating_sub(op.created_at);
        } else {
            op.steps.push(OperationStep {
                step_id: format!("{service}/{step}"),
                phase: phase.into(),
                message_code: message.into(),
                elapsed_ms: 0,
                deadline_at: Some(now() + 600000),
            });
        }
        self.operations()?.persist(op)?;
        (self.emit)(StepProgressEvent {
            operation_id: op.operation_id.clone(),
            seq: op.seq,
            service_id: service.into(),
            step_id: step.into(),
            phase: phase.into(),
            message_code: message.into(),
        });
        Ok(())
    }
    pub async fn start(
        self: &Arc<Self>,
        request: StartRequest,
    ) -> Result<OperationRecord, ToolboxError> {
        self.start_internal(request, None, false).await
    }
    async fn start_internal(
        self: &Arc<Self>,
        request: StartRequest,
        configured: Option<crate::config_apply::Draft>,
        preserve_scope: bool,
    ) -> Result<OperationRecord, ToolboxError> {
        let mut reservation = self.reserve()?;
        let store = self.store()?;
        let target = self.target()?;
        if request.workspace_id != store.workspace.workspace_id
            || request.target_id != target.target_id
        {
            return Err(error("TargetChanged"));
        }
        self.verify_native_runtime()?;
        let selected = validate_selection(&crate::catalog::load_embedded_catalog()?, &request)?;
        let mut bindings = vec![];
        for id in &selected {
            bindings.extend(
                self.instance_for(&self.service(id)?, &target)?
                    .bindings
                    .into_iter()
                    .filter(|b| b.lifecycle_owner != "Managed"),
            );
        }
        for binding in self.load_bindings()? {
            if selected.contains(&binding.engine)
                && binding.target_id == target.target_id
                && binding.daemon_id == target.daemon_id
                && !store
                    .list()?
                    .iter()
                    .any(|i| i.identity.service_id == binding.engine && !i.containers.is_empty())
            {
                bindings.push(binding);
            }
        }
        let order = crate::catalog::resolve_startup_order(
            &crate::catalog::load_embedded_catalog()?,
            &selected,
            &bindings,
        )?;
        // Resolve the entire request first: a pending artifact cannot leave half a scene deployed.
        let mut instances = vec![];
        for id in &order {
            let service = self.service(id)?;
            self.images(&service, &target)?;
            instances.push(self.instance_for(&service, &target)?);
        }
        if let Some(draft) = configured {
            let staged = crate::config_apply::stage(
                self,
                draft,
                request.accept_unknown_capacity,
                &reservation,
            )?;
            let row = instances
                .iter_mut()
                .find(|i| i.identity.instance_id == staged.identity.instance_id)
                .ok_or_else(|| error("StaleDraft"))?;
            *row = staged;
        }
        for i in &mut instances {
            if i.desired.is_none() {
                i.desired =
                    Some(self.effective(i, &self.service(&i.identity.service_id)?, &target)?);
                store.save(i)?;
            }
        }
        let envelope = RequestEnvelope {
            schema_version: 1,
            request: request.clone(),
            generations: instances
                .iter()
                .map(|i| {
                    (
                        i.identity.instance_id.clone(),
                        i.desired
                            .as_ref()
                            .map(|e| e.generation_id.clone())
                            .unwrap_or_default(),
                    )
                })
                .collect(),
            preserve_scope,
        };
        let op = OperationRecord {
            operation_id: random_id()?,
            target_id: target.target_id.clone(),
            instance_ids: instances
                .iter()
                .map(|i| i.identity.instance_id.clone())
                .collect(),
            request_digest: format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(&envelope).map_err(|_| error("InvalidRequest"))?)
            ),
            desired_generation: None,
            applied_generation: None,
            last_ready_generation: None,
            phase: "Queued".into(),
            steps: vec![],
            created_resource_ids: vec![],
            error_code: None,
            allowed_actions: vec!["cancel".into()],
            created_at: now(),
            seq: 1,
        };
        let result = (|| {
            let operations = self.operations()?;
            operations.persist(&op)?;
            let p = Path::new(&store.workspace.root)
                .join("requests")
                .join(format!("{}.json", op.operation_id));
            private_dir(p.parent().ok_or_else(|| error("UnsafePath"))?)?;
            atomic_write(
                &p,
                &serde_json::to_vec(&envelope).map_err(|_| error("InvalidRequest"))?,
            )
        })();
        if let Err(e) = result {
            self.busy.store(false, Ordering::Release);
            return Err(e);
        }
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancellations
            .lock()
            .map_err(|_| error("StateUnavailable"))?
            .insert(op.operation_id.clone(), cancel.clone());
        let engine = self.clone();
        let job = op.clone();
        tokio::spawn(async move {
            engine
                .run_start(job, request, instances, cancel, preserve_scope)
                .await;
        });
        reservation.disarm();
        Ok(op)
    }
    async fn run_start(
        &self,
        mut op: OperationRecord,
        request: StartRequest,
        mut instances: Vec<RuntimeInstance>,
        cancel: Arc<AtomicBool>,
        preserve_scope: bool,
    ) {
        let _busy = BusyLease::new(self.busy.clone());
        let result = async {
            let target = self.target()?;
            let runner = DockerRunner::connect(target).await?;
            let store = self.store()?;
            let keys = instances
                .iter()
                .map(|i| format!("{}/{}", i.identity.daemon_id, i.identity.instance_id))
                .collect::<Vec<_>>();
            let _locks = crate::locks::acquire(&self.control.join("locks"), &keys)?;
            op.phase = "Running".into();
            let context = ImageContext {
                cache_root: self.control.join("image-cache"),
                cancelled: cancel.clone(),
            };
            let usage = crate::usage_index::UsageIndex::new(&self.control.join("usage"))?;
            let mut ready = BTreeSet::new();
            for i in &mut instances {
                if cancel.load(Ordering::Acquire) {
                    return Err(error("Cancelled"));
                }
                self.reconcile_runtime(&runner, i).await?;
                let service = self.service(&i.identity.service_id)?;
                for dependency in &service.dependencies {
                    if !ready.contains(dependency)
                        && !i.bindings.iter().any(|b| b.engine == *dependency)
                    {
                        return Err(error("DependencyBindingRequired"));
                    }
                }
                self.progress(
                    &mut op,
                    &service.id,
                    "preflight",
                    "Running",
                    "CheckingEnvironment",
                )?;
                let mut effective = self.effective(i, &service, &runner.target)?;
                let (ports, ids) = self.owned(&runner, i).await?;
                let report = crate::preflight::run_preflight(
                    &runner,
                    &service,
                    &effective,
                    Path::new(&store.workspace.root),
                    &ports,
                    &ids,
                )
                .await?;
                if let Some(issue) = report.issues.iter().find(|p| p.severity == "error") {
                    return Err(error(&issue.code));
                }
                if report.requires_capacity_consent && !request.accept_unknown_capacity {
                    return Err(error("UnknownCapacity"));
                }
                if ["mysql", "pgsql"].contains(&service.id.as_str())
                    && i.containers.is_empty()
                    && self
                        .database_candidates(&runner)
                        .await?
                        .iter()
                        .any(|c| c.engine == service.id)
                {
                    return Err(error("ExistingDatabaseBindingRequired"));
                }
                let stack = store.stack(&service.id)?;
                let outcome = materialize_blueprint(
                    &self.resources.join("blueprints"),
                    &stack,
                    &service,
                    &i.identity,
                )?;
                if !outcome.conflicts.is_empty() {
                    return Err(error("BlueprintConflict"));
                }
                let dependency = self.dependency(&runner, &service, i, &context).await?;
                let envpath = stack.join(".env");
                reject_links(&envpath)?;
                let applying = i.pending_env.is_some();
                let pending = crate::config_apply::pending_document(&stack, i)?;
                let live = if pending.is_none() && envpath.exists() {
                    Some(zeroize::Zeroizing::new(
                        std::fs::read_to_string(&envpath).map_err(|_| error("StateUnavailable"))?,
                    ))
                } else {
                    None
                };
                let existing = pending.as_ref().or(live.as_ref()).map(|s| s.as_str());
                let sample = if stack.join(".env.example").exists() {
                    std::fs::read_to_string(stack.join(".env.example"))
                        .map_err(|_| error("BlueprintMissing"))?
                } else {
                    String::new()
                };
                let candidate = crate::env_store::merge_env(
                    existing,
                    &sample,
                    &service,
                    dependency
                        .as_ref()
                        .map(|setup| &setup.inherited)
                        .unwrap_or(&crate::env_store::SecretValues::default()),
                    existing.is_none(),
                )?;
                if !candidate.validation_errors.is_empty() {
                    return Err(error("CredentialRepairRequired"));
                }
                let generation_dir = stack.join(".runtime").join(&effective.generation_id);
                private_dir(&generation_dir)?;
                let effective_env = generation_dir.join("effective.env");
                reject_links(&effective_env)?;
                if effective_env.exists() {
                    let old = zeroize::Zeroizing::new(
                        std::fs::read_to_string(&effective_env)
                            .map_err(|_| error("AppliedEnvironmentUnavailable"))?,
                    );
                    if old.as_str() != candidate.document {
                        return Err(error("ConfigurationApplyRequired"));
                    }
                } else {
                    atomic_write(&effective_env, candidate.document.as_bytes())?;
                }
                if !applying {
                    atomic_write(&envpath, candidate.document.as_bytes())?;
                }
                let secrets = crate::env_store::SecretValues(crate::env_store::parse_env(
                    &candidate.document,
                )?);
                if let Some(setup) = &dependency {
                    self.progress(
                        &mut op,
                        &service.id,
                        "initialize",
                        "Running",
                        "InitializingScopedDatabase",
                    )?;
                    self.initialize_dependency(&runner, &service, i, setup, &secrets, &context)
                        .await?;
                    self.progress(
                        &mut op,
                        &service.id,
                        "initialize",
                        "Completed",
                        "DependencyInitialized",
                    )?;
                }
                let artifact = render_override(
                    &stack,
                    &service,
                    &i.identity,
                    &effective,
                    &secrets,
                    &i.bindings,
                )?;
                effective.config_digest = artifact.config_digest.clone();
                i.desired = Some(effective.clone());
                i.phase = ServicePhase::Starting;
                i.message_code = None;
                store.save(i)?;
                op.desired_generation = Some(effective.generation_id.clone());
                let model = runner
                    .compose_run(
                        &stack,
                        &i.identity,
                        &artifact,
                        &["config".into(), "--format".into(), "json".into()],
                    )
                    .await?;
                let model: Value =
                    serde_json::from_slice(&model).map_err(|_| error("InvalidComposeModel"))?;
                self.progress(
                    &mut op,
                    &service.id,
                    "images",
                    "Running",
                    "PreparingLockedImages",
                )?;
                for image in &effective.image_artifacts {
                    ensure_service_image(&runner, image, &context).await?;
                }
                let needs_tool = crate::probes::requires_tool(&service);
                let tool = if needs_tool {
                    let tool = self.tool_image(&runner.target)?;
                    ensure_service_image(&runner, &tool, &context).await?;
                    Some(tool)
                } else {
                    None
                };
                if cancel.load(Ordering::Acquire) {
                    return Err(error("Cancelled"));
                }
                if !preserve_scope || i.active_presets.is_empty() {
                    let scope = request.scene_id.clone().unwrap_or_else(|| "manual".into());
                    if !i.active_presets.contains(&scope) {
                        i.active_presets.push(scope);
                    }
                }
                store.save(i)?;
                usage.register(crate::usage_index::UsageRegistration {
                    target_id: i.identity.target_id.clone(),
                    provider_instance_id: i.identity.instance_id.clone(),
                    consumer_workspace_id: i.identity.workspace_id.clone(),
                    consumer_instance_id: i.identity.instance_id.clone(),
                    operation_id: op.operation_id.clone(),
                    preset_refs: i.active_presets.clone(),
                    status: "Pending".into(),
                })?;
                for binding in i.bindings.iter().filter(|b| b.lifecycle_owner == "Managed") {
                    if let Some(provider) = store.list()?.iter().find(|p| {
                        p.containers
                            .iter()
                            .any(|c| c.container_id == binding.container_id)
                    }) {
                        usage.register(crate::usage_index::UsageRegistration {
                            target_id: i.identity.target_id.clone(),
                            provider_instance_id: provider.identity.instance_id.clone(),
                            consumer_workspace_id: i.identity.workspace_id.clone(),
                            consumer_instance_id: i.identity.instance_id.clone(),
                            operation_id: op.operation_id.clone(),
                            preset_refs: i.active_presets.clone(),
                            status: "Active".into(),
                        })?;
                    }
                }
                self.progress(
                    &mut op,
                    &service.id,
                    "deploy",
                    "Running",
                    "StartingManagedContainers",
                )?;
                let intent =
                    crate::recovery::intent_from_model(&service, &i.identity, &effective, &model)?;
                let keep = i.applied.as_ref().map(|e| e.generation_id.as_str());
                i.intents.retain(|p| Some(p.generation_id.as_str()) == keep);
                i.intents
                    .retain(|p| p.generation_id != intent.generation_id);
                i.intents.push(intent);
                store.save(i)?;
                let before = i
                    .containers
                    .iter()
                    .map(|c| c.container_id.clone())
                    .collect::<BTreeSet<_>>();
                let deployed = runner
                    .compose_run(
                        &stack,
                        &i.identity,
                        &artifact,
                        &[
                            "up".into(),
                            "-d".into(),
                            "--no-build".into(),
                            "--pull".into(),
                            "never".into(),
                        ],
                    )
                    .await;
                let observed = self.reconcile_runtime(&runner, i).await?;
                if observed.desired_complete {
                    i.applied = Some(effective.clone());
                } else {
                    i.phase = ServicePhase::RunningUnverified;
                    i.message_code = Some("RecoveryRequired".into());
                }
                store.save(i)?;
                deployed?;
                if !observed.desired_complete {
                    return Err(error("RecoveryRequired"));
                }
                op.created_resource_ids.extend(
                    i.containers
                        .iter()
                        .filter(|r| !before.contains(&r.container_id))
                        .map(|r| r.container_id.clone()),
                );
                i.applied = Some(effective.clone());
                i.phase = ServicePhase::RunningUnverified;
                store.save(i)?;
                op.applied_generation = Some(effective.generation_id.clone());
                if service.id == "nacos" {
                    self.progress(
                        &mut op,
                        &service.id,
                        "initialize",
                        "Running",
                        "InitializingManagedAdministrator",
                    )?;
                    crate::service_bootstrap::nacos(self, &runner, i, &effective, &secrets, &cancel)
                        .await?;
                    self.progress(
                        &mut op,
                        &service.id,
                        "initialize",
                        "Completed",
                        "ManagedAdministratorVerified",
                    )?;
                }
                self.progress(
                    &mut op,
                    &service.id,
                    "readiness",
                    "Running",
                    "CheckingProtocols",
                )?;
                let started = std::time::Instant::now();
                let results = loop {
                    let results = crate::probes::evaluate(&crate::probes::ProbeContext {
                        control_root: &self.control,
                        runner: &runner,
                        instance: &i.identity,
                        effective: &effective,
                        service: &service,
                        secrets: &secrets,
                        bindings: &i.bindings,
                        tool_image: tool.as_ref(),
                        resources_root: &self.resources.join("probe-clients"),
                    })
                    .await?;
                    if crate::probes::ready(&service, &results)
                        || started.elapsed() > Duration::from_secs(180)
                        || cancel.load(Ordering::Acquire)
                    {
                        break results;
                    }
                    tokio::time::sleep(Duration::from_secs(2)).await;
                };
                if cancel.load(Ordering::Acquire) {
                    return Err(error("Cancelled"));
                }
                if !crate::probes::ready(&service, &results) {
                    i.phase = ServicePhase::Degraded;
                    i.message_code = results.iter().find_map(|p| p.error_code.clone());
                    store.save(i)?;
                    return Err(error(i.message_code.as_deref().unwrap_or("ProbeFailed")));
                }
                crate::config_apply::commit_pending(&stack, i)?;
                i.phase = ServicePhase::Ready;
                for claim in &mut i.claims {
                    if claim.status == "DatabaseCreated" {
                        claim.status = "Initialized".into();
                    }
                }
                i.message_code = results.iter().find_map(|p| p.warning_code.clone());
                i.last_ready_generation = Some(effective.generation_id.clone());
                usage.register(crate::usage_index::UsageRegistration {
                    target_id: i.identity.target_id.clone(),
                    provider_instance_id: i.identity.instance_id.clone(),
                    consumer_workspace_id: i.identity.workspace_id.clone(),
                    consumer_instance_id: i.identity.instance_id.clone(),
                    operation_id: op.operation_id.clone(),
                    preset_refs: i.active_presets.clone(),
                    status: "Active".into(),
                })?;
                store.save(i)?;
                self.live_status.mark_ready(i);
                ready.insert(service.id.clone());
                op.last_ready_generation = Some(effective.generation_id);
                self.progress(
                    &mut op,
                    &service.id,
                    "readiness",
                    "Completed",
                    "ServiceReady",
                )?;
            }
            Ok(())
        }
        .await;
        match result {
            Ok(()) => {
                op.phase = "Ready".into();
                op.allowed_actions.clear();
            }
            Err(e) => {
                op.phase = if e.code == "Cancelled" {
                    "Canceled"
                } else if e.code.ends_with("Required") || e.code == "UnknownCapacity" {
                    "AwaitingAction"
                } else {
                    "Failed"
                }
                .into();
                op.error_code = Some(e.code.clone());
                op.allowed_actions = vec!["retry".into()];
                if let Ok(store) = self.store() {
                    for i in &mut instances {
                        if matches!(i.phase, ServicePhase::Starting) {
                            i.phase = ServicePhase::RunningUnverified;
                            i.message_code = Some("RecoveryRequired".into());
                            let _ = store.save(i);
                        }
                    }
                }
            }
        }
        op.seq += 1;
        let _ = self.operations().and_then(|s| s.persist(&op));
        (self.emit)(StepProgressEvent {
            operation_id: op.operation_id.clone(),
            seq: op.seq,
            service_id: String::new(),
            step_id: "operation".into(),
            phase: op.phase.clone(),
            message_code: op.error_code.clone().unwrap_or_else(|| "Completed".into()),
        });
        if let Ok(mut c) = self.cancellations.lock() {
            c.remove(&op.operation_id);
        }
        self.busy.store(false, Ordering::Release);
    }
    pub async fn database_candidates(
        &self,
        runner: &DockerRunner,
    ) -> Result<Vec<DatabaseCandidate>, ToolboxError> {
        runner.verify_identity().await?;
        let raw = runner
            .run_read(&["ps", "--no-trunc", "--format", "{{json .}} "])
            .await?;
        let mut out = vec![];
        for line in std::str::from_utf8(&raw)
            .map_err(|_| error("DockerOutputInvalid"))?
            .lines()
        {
            let v: Value =
                serde_json::from_str(line.trim()).map_err(|_| error("DockerOutputInvalid"))?;
            let image = v["Image"].as_str().unwrap_or("");
            let engine = if image.starts_with("mysql:") || image.starts_with("mysql@") {
                "mysql"
            } else if image.starts_with("postgres:") || image.starts_with("postgres@") {
                "pgsql"
            } else {
                continue;
            };
            let id = v["ID"]
                .as_str()
                .ok_or_else(|| error("DockerOutputInvalid"))?;
            let raw = runner.run_read(&["container", "inspect", id]).await?;
            let raw: Value =
                serde_json::from_slice(&raw).map_err(|_| error("DockerOutputInvalid"))?;
            let c = &raw[0];
            let port = if engine == "mysql" { 3306 } else { 5432 };
            let mut endpoints = vec![];
            if let Some(list) = c["NetworkSettings"]["Ports"][format!("{port}/tcp")].as_array() {
                for p in list {
                    if matches!(p["HostIp"].as_str(), Some("127.0.0.1" | "0.0.0.0" | ""))
                        && let Some(port) =
                            p["HostPort"].as_str().and_then(|p| p.parse::<u16>().ok())
                    {
                        endpoints.push(Endpoint {
                            label: engine.into(),
                            component_id: engine.into(),
                            host: "127.0.0.1".into(),
                            port,
                            protocol: "tcp".into(),
                        });
                    }
                }
            }
            out.push(DatabaseCandidate {
                candidate_id: format!(
                    "{:x}",
                    Sha256::digest(format!("{}|{id}", runner.target.target_id))
                ),
                container_id: id.into(),
                name: v["Names"].as_str().unwrap_or(engine).into(),
                engine: engine.into(),
                network_names: c["NetworkSettings"]["Networks"]
                    .as_object()
                    .map(|v| v.keys().cloned().collect())
                    .unwrap_or_default(),
                host_endpoints: endpoints,
            });
        }
        Ok(out)
    }
    pub fn cancel(&self, id: &str) -> Result<(), ToolboxError> {
        let op = self.operations()?.load(id)?;
        if !op.allowed_actions.iter().any(|a| a == "cancel") {
            return Err(error("OperationNotCancelable"));
        }
        let c = self
            .cancellations
            .lock()
            .map_err(|_| error("StateUnavailable"))?;
        c.get(id)
            .ok_or_else(|| error("RecoveryRequired"))?
            .store(true, Ordering::Release);
        Ok(())
    }
    pub async fn retry(self: &Arc<Self>, id: &str) -> Result<OperationRecord, ToolboxError> {
        let old = self.operations()?.load(id)?;
        if !old.allowed_actions.iter().any(|a| a == "retry") {
            return Err(error("OperationNotRetryable"));
        }
        let envelope: RequestEnvelope = read_json(
            &Path::new(&self.store()?.workspace.root)
                .join("requests")
                .join(format!("{id}.json")),
        )?;
        self.validate_envelope(&old, &envelope)?;
        self.start_internal(envelope.request, None, envelope.preserve_scope)
            .await
    }
    fn validate_envelope(
        &self,
        op: &OperationRecord,
        envelope: &RequestEnvelope,
    ) -> Result<(), ToolboxError> {
        let store = self.store()?;
        let hash = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(envelope).map_err(|_| error("InvalidRequest"))?)
        );
        if envelope.schema_version != 1
            || hash != op.request_digest
            || envelope.request.target_id != op.target_id
            || envelope.request.workspace_id != store.workspace.workspace_id
            || envelope.generations.len() != op.instance_ids.len()
        {
            return Err(error("StaleOperation"));
        }
        for id in &op.instance_ids {
            let i = store.instance(id)?;
            if i.desired.as_ref().map(|e| &e.generation_id) != envelope.generations.get(id) {
                return Err(error("StaleOperation"));
            }
        }
        Ok(())
    }
}
pub(crate) struct BusyLease {
    busy: Arc<AtomicBool>,
    armed: bool,
}
impl BusyLease {
    pub(crate) fn controls(&self, engine: &Engine) -> bool {
        self.armed && Arc::ptr_eq(&self.busy, &engine.busy)
    }
    fn new(busy: Arc<AtomicBool>) -> Self {
        Self { busy, armed: true }
    }
    fn disarm(&mut self) {
        self.armed = false;
    }
}
impl Drop for BusyLease {
    fn drop(&mut self) {
        if self.armed {
            self.busy.store(false, Ordering::Release);
        }
    }
}

impl Engine {
    pub async fn stop(&self, request: StopRequest) -> Result<OperationRecord, ToolboxError> {
        self.verify_native_runtime()?;
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| error("ResourceBusy"))?;
        let _busy = BusyLease::new(self.busy.clone());
        self.live_status.invalidate();
        let store = self.store()?;
        let target = self.target()?;
        if request.workspace_id != store.workspace.workspace_id
            || request.target_id != target.target_id
            || request.instance_ids.is_empty()
        {
            return Err(error("TargetChanged"));
        }
        let runner = DockerRunner::connect(target.clone()).await?;
        let mut instances = request
            .instance_ids
            .iter()
            .map(|id| store.instance(id))
            .collect::<Result<Vec<_>, _>>()?;
        if instances.iter().any(|i| {
            i.identity.target_id != target.target_id || i.identity.daemon_id != target.daemon_id
        }) {
            return Err(error("TargetChanged"));
        }
        let selected = instances
            .iter()
            .map(|i| i.identity.service_id.clone())
            .collect::<Vec<_>>();
        let order = crate::catalog::resolve_startup_order(
            &crate::catalog::load_embedded_catalog()?,
            &selected,
            &vec![],
        )?;
        instances.sort_by_key(|i| {
            std::cmp::Reverse(
                order
                    .iter()
                    .position(|s| s == &i.identity.service_id)
                    .unwrap_or(0),
            )
        });
        let keys = instances
            .iter()
            .map(|i| format!("{}/{}", i.identity.daemon_id, i.identity.instance_id))
            .collect::<Vec<_>>();
        let _locks = crate::locks::acquire(&self.control.join("locks"), &keys)?;
        let usage = crate::usage_index::UsageIndex::new(&self.control.join("usage"))?;
        let mut op = OperationRecord {
            operation_id: random_id()?,
            target_id: target.target_id.clone(),
            instance_ids: request.instance_ids.clone(),
            request_digest: format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(&request).map_err(|_| error("InvalidRequest"))?)
            ),
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
        self.operations()?.persist(&op)?;
        let result = async {
            for i in &mut instances {
                self.reconcile_runtime(&runner, i).await?;
                let remaining = scopes_after_stop(&i.active_presets, request.scene_id.as_deref())?;
                if request
                    .scene_id
                    .as_ref()
                    .is_some_and(|scene| !i.active_presets.contains(scene))
                {
                    continue;
                }
                if !remaining.is_empty() {
                    i.active_presets = remaining.clone();
                    store.save(i)?;
                    usage.replace_own_presets(
                        &target.target_id,
                        &store.workspace.workspace_id,
                        &i.identity.instance_id,
                        &remaining,
                    )?;
                    continue;
                }
                let consumers = usage.consumers(&target.target_id, &i.identity.instance_id)?;
                if consumers
                    .iter()
                    .any(|c| c.consumer_instance_id != i.identity.instance_id)
                {
                    return Err(error("SharedDependencyInUse"));
                }
                for record in &i.containers {
                    runner.verify_identity().await?;
                    let raw = runner
                        .run_read(&["container", "inspect", &record.container_id])
                        .await?;
                    let v: Value =
                        serde_json::from_slice(&raw).map_err(|_| error("DockerOutputInvalid"))?;
                    let proof = crate::bindings::verify_ownership(&v[0], record, &target)?;
                    if v[0]["State"]["Running"] == true {
                        runner
                            .run_read(&["container", "stop", "--time", "20", proof.container_id()])
                            .await?;
                    }
                    let raw = runner
                        .run_read(&["container", "inspect", proof.container_id()])
                        .await?;
                    let v: Value =
                        serde_json::from_slice(&raw).map_err(|_| error("DockerOutputInvalid"))?;
                    if v[0]["State"]["Running"] != false {
                        return Err(error("StopNotConfirmed"));
                    }
                }
                let observed = self.reconcile_runtime(&runner, i).await?;
                if observed.any_running {
                    return Err(error("StopNotConfirmed"));
                }
                i.phase = ServicePhase::Stopped;
                i.active_presets.clear();
                store.save(i)?;
                usage.unregister_stopped(
                    &target.target_id,
                    &store.workspace.workspace_id,
                    &i.identity.instance_id,
                )?;
                self.progress(
                    &mut op,
                    &i.identity.service_id,
                    "stop",
                    "Completed",
                    "StoppedDataRetained",
                )?;
            }
            Ok::<(), ToolboxError>(())
        }
        .await;
        match result {
            Ok(()) => op.phase = "Stopped".into(),
            Err(e) => {
                op.phase = "Failed".into();
                op.error_code = Some(e.code);
            }
        }
        op.seq += 1;
        self.operations()?.persist(&op)?;
        Ok(op)
    }
    pub fn connection(&self, id: &str) -> Result<ConnectionView, ToolboxError> {
        if let Some(binding_id) = id.strip_prefix("binding-") {
            let binding = self
                .load_bindings()?
                .into_iter()
                .find(|b| b.binding_id == binding_id)
                .ok_or_else(|| error("UnknownBinding"))?;
            let target = self.target()?;
            if binding.target_id != target.target_id || binding.daemon_id != target.daemon_id {
                return Err(error("TargetChanged"));
            }
            return Ok(ConnectionView {
                instance_id: id.into(),
                applied_generation: "verified-binding".into(),
                host_endpoints: binding.host_endpoint.into_iter().collect(),
                container_endpoints: vec![Endpoint {
                    label: binding.engine.clone(),
                    component_id: binding.engine,
                    host: binding.network_alias,
                    port: binding.container_port,
                    protocol: "tcp".into(),
                }],
                required_networks: vec![binding.network_name],
                credential_handles: vec![],
                snippets: vec![],
            });
        }
        let i = self.store()?.instance(id)?;
        let e = i.applied.ok_or_else(|| error("NoAppliedConfiguration"))?;
        let service = self.service(&i.identity.service_id)?;
        Ok(ConnectionView {
            instance_id: id.into(),
            applied_generation: e.generation_id,
            host_endpoints: e
                .ports
                .iter()
                .map(|p| Endpoint {
                    label: format!("{}:{}", p.component_id, p.container_port),
                    component_id: p.component_id.clone(),
                    host: p.host_ip.clone(),
                    port: p.host_port,
                    protocol: crate::endpoints::console_protocol(
                        &service.id,
                        &p.component_id,
                        p.container_port,
                    )
                    .unwrap_or(&p.protocol)
                    .into(),
                })
                .collect(),
            container_endpoints: e
                .ports
                .iter()
                .map(|p| Endpoint {
                    label: format!("{}:{}", p.component_id, p.container_port),
                    component_id: p.component_id.clone(),
                    host: format!("{}-{}", i.identity.project_name, p.component_id),
                    port: p.container_port,
                    protocol: crate::endpoints::console_protocol(
                        &service.id,
                        &p.component_id,
                        p.container_port,
                    )
                    .unwrap_or(&p.protocol)
                    .into(),
                })
                .collect(),
            required_networks: i.applied_networks,
            credential_handles: service
                .env_schema
                .iter()
                .filter(|f| f.sensitive)
                .map(|f| CredentialHandle {
                    handle: f.key.clone(),
                    label: f.key.clone(),
                    sensitive: true,
                })
                .collect(),
            snippets: vec![],
        })
    }
    pub fn reveal(&self, id: &str, key: &str) -> Result<String, ToolboxError> {
        let store = self.store()?;
        let i = store.instance(id)?;
        let service = self.service(&i.identity.service_id)?;
        if !service
            .env_schema
            .iter()
            .any(|f| f.key == key && f.sensitive)
        {
            return Err(error("UnknownCredentialHandle"));
        }
        let stack = store.stack(&service.id)?;
        let env = if let Some(applied) = &i.applied {
            if applied.generation_id.len() != 32
                || !applied.generation_id.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(error("GenerationMismatch"));
            }
            let p = stack
                .join(".runtime")
                .join(&applied.generation_id)
                .join("effective.env");
            if p.exists() { p } else { stack.join(".env") }
        } else {
            stack.join(".env")
        };
        reject_links(&env)?;
        let doc = zeroize::Zeroizing::new(
            std::fs::read_to_string(env).map_err(|_| error("CredentialRepairRequired"))?,
        );
        let values = crate::env_store::SecretValues(crate::env_store::parse_env(&doc)?);
        values
            .0
            .get(key)
            .cloned()
            .ok_or_else(|| error("CredentialRepairRequired"))
    }
}

impl Engine {
    async fn reconcile_runtime(
        &self,
        runner: &DockerRunner,
        i: &mut RuntimeInstance,
    ) -> Result<crate::recovery::Reconciled, ToolboxError> {
        runner.verify_identity().await?;
        let raw = runner
            .run_read(&[
                "ps",
                "-aq",
                "--no-trunc",
                "--filter",
                &format!("label=io.codeark.workspace-id={}", i.identity.workspace_id),
                "--filter",
                &format!("label=io.codeark.instance-id={}", i.identity.instance_id),
            ])
            .await?;
        let ids = std::str::from_utf8(&raw).map_err(|_| error("DockerOutputInvalid"))?;
        let mut actual = vec![];
        for id in ids.lines() {
            if !crate::probes::valid_id(id) {
                return Err(error("DockerOutputInvalid"));
            }
            let raw = runner.run_read(&["container", "inspect", id]).await?;
            let value: Vec<Value> =
                serde_json::from_slice(&raw).map_err(|_| error("DockerOutputInvalid"))?;
            if value.len() != 1 {
                return Err(error("DockerOutputInvalid"));
            }
            actual.push(value[0].clone());
        }
        let mut intents = i.intents.clone();
        // Compatibility for an already registered exact CID: never infer a new CID from its name.
        for c in &actual {
            let generation = c["Config"]["Labels"]["io.codeark.generation-id"]
                .as_str()
                .ok_or_else(|| error("RecoveryRequired"))?;
            let component = c["Config"]["Labels"]["com.docker.compose.service"]
                .as_str()
                .ok_or_else(|| error("RecoveryRequired"))?;
            if intents.iter().any(|p| {
                p.generation_id == generation
                    && p.components.iter().any(|p| p.component_id == component)
            }) {
                continue;
            }
            let id = c["Id"].as_str().ok_or_else(|| error("RecoveryRequired"))?;
            let record = i
                .containers
                .iter()
                .find(|r| r.container_id == id)
                .ok_or_else(|| error("RecoveryRequired"))?;
            crate::bindings::verify_ownership(c, record, &runner.target)?;
            if ![i.applied.as_ref(), i.desired.as_ref()]
                .into_iter()
                .flatten()
                .any(|e| e.generation_id == generation)
            {
                return Err(error("RecoveryRequired"));
            }
            let intent = crate::recovery::ComponentIntent {
                component_id: component.into(),
                container_name: format!("{}-{component}", i.identity.project_name),
                generation_id: generation.into(),
                blueprint_sha: record.blueprint_sha.clone(),
                image_ref: c["Config"]["Image"]
                    .as_str()
                    .ok_or_else(|| error("RecoveryRequired"))?
                    .into(),
                bind_sources: record.bind_sources.clone(),
            };
            if let Some(group) = intents.iter_mut().find(|p| p.generation_id == generation) {
                group.components.push(intent);
            } else {
                intents.push(crate::recovery::DeploymentIntent {
                    generation_id: generation.into(),
                    components: vec![intent],
                });
            }
        }
        let result = crate::recovery::reconcile(
            &i.identity,
            &runner.target,
            &intents,
            i.desired.as_ref().map(|e| e.generation_id.as_str()),
            &actual,
        )?;
        i.containers = result.containers.clone();
        i.applied_networks = result.networks.clone();
        self.store()?.save(i)?;
        Ok(result)
    }
    pub fn mark_interrupted_operations(&self) -> Result<(), ToolboxError> {
        for mut op in self.operations()?.list()? {
            if matches!(op.phase.as_str(), "Queued" | "Running") {
                op.phase = "RecoveryRequired".into();
                op.error_code = Some("OperationInterrupted".into());
                op.allowed_actions.clear();
                op.seq += 1;
                self.operations()?.persist(&op)?;
            }
        }
        Ok(())
    }
    async fn recover_interrupted_operations(
        &self,
        runner: &DockerRunner,
    ) -> Result<(), ToolboxError> {
        for mut op in self.operations()?.list()? {
            if op.target_id != runner.target.target_id
                || op.phase != "RecoveryRequired"
                || op.error_code.as_deref() != Some("OperationInterrupted")
            {
                continue;
            }
            let mut verified = true;
            for id in &op.instance_ids {
                let mut i = self.store()?.instance(id)?;
                if self.reconcile_runtime(runner, &mut i).await.is_err() {
                    verified = false;
                    break;
                }
            }
            if verified {
                let path = Path::new(&self.store()?.workspace.root)
                    .join("requests")
                    .join(format!("{}.json", op.operation_id));
                if let Ok(envelope) = read_json::<RequestEnvelope>(&path)
                    && self.validate_envelope(&op, &envelope).is_ok()
                {
                    op.phase = "Failed".into();
                    op.allowed_actions = vec!["retry".into()];
                    op.seq += 1;
                    self.operations()?.persist(&op)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_detection_revokes_target_and_survives_snapshot_refresh() {
        let d = tempfile::tempdir().unwrap();
        let w = open_workspace(&d.path().join("workspace")).unwrap();
        let e = Engine::new(w, d.path().join("control"), d.path().join("resources")).unwrap();
        *e.target.write().unwrap() = Some(DockerTarget {
            target_id: "previous".into(),
            cli_path: "/usr/local/bin/docker".into(),
            context_name: "previous".into(),
            endpoint_kind: "unix".into(),
            daemon_id: "previous".into(),
            os_type: "linux".into(),
            architecture: "arm64".into(),
        });
        e.record_detection_failure("UnsupportedTarget").unwrap();
        assert_eq!(e.target().unwrap_err().code, "EnvironmentNotDetected");
        let snapshot = e.snapshot(None).unwrap();
        assert!(snapshot.target.is_none());
        assert_eq!(
            snapshot.environment_error.as_deref(),
            Some("UnsupportedTarget")
        );
    }
    #[test]
    fn corrupt_instance_state_and_target_switch_never_discard_ownership() {
        let d = tempfile::tempdir().unwrap();
        let w = open_workspace(&d.path().join("workspace")).unwrap();
        let e = Engine::new(w, d.path().join("control"), d.path().join("resources")).unwrap();
        let service = e.service("redis").unwrap();
        let mut target = DockerTarget {
            target_id: "t".into(),
            cli_path: "/usr/local/bin/docker".into(),
            context_name: "test".into(),
            endpoint_kind: "unix".into(),
            daemon_id: "d".into(),
            os_type: "linux".into(),
            architecture: "arm64".into(),
        };
        let original = e.instance_for(&service, &target).unwrap();
        target.target_id = "other".into();
        assert_eq!(
            e.instance_for(&service, &target).err().unwrap().code,
            "WorkspaceTargetConflict"
        );
        target.target_id = "t".into();
        let path = e
            .store()
            .unwrap()
            .stack("redis")
            .unwrap()
            .join(".codeark-instance.json");
        std::fs::write(&path, "truncated").unwrap();
        assert!(e.instance_for(&service, &target).is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "truncated");
        assert!(!original.identity.instance_id.is_empty());
    }
}
