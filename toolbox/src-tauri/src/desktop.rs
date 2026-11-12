use crate::{
    config_apply::Draft,
    contracts::*,
    docker_cli::DockerRunner,
    image_store::ImageContext,
    log_stream::{LogWindow, Redactor},
    orchestrator::{Engine, now},
    workspace::{
        atomic_write, error, open_private, private_dir, random_id, read_json, reject_links,
    },
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager, State, ipc::Channel};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
struct Stream {
    window: LogWindow,
    cancel: Arc<AtomicBool>,
    done: tokio::sync::watch::Receiver<bool>,
}
struct Bound {
    verified: crate::bindings::VerifiedBinding,
    _credentials: crate::bindings::Credentials,
}
pub struct AppState {
    pub engine: Arc<Engine>,
    _app_lock: std::fs::File,
    drafts: Mutex<BTreeMap<String, Draft>>,
    streams: Arc<Mutex<BTreeMap<String, Stream>>>,
    previews: Mutex<BTreeMap<String, DiagnosticsPreview>>,
    bound: Mutex<BTreeMap<String, Bound>>,
    grants: Mutex<BTreeMap<String, BindingGrantPreview>>,
    resets: Mutex<BTreeMap<String, crate::reset::VerifiedResetPlan>>,
}
impl AppState {
    pub fn new(engine: Engine) -> Result<Self, ToolboxError> {
        let lock = open_private(&engine.control.join("app.lock"), false)?;
        lock.try_lock()
            .map_err(|_| error("ApplicationAlreadyRunning"))?;
        engine.mark_interrupted_operations()?;
        Ok(Self {
            engine: Arc::new(engine),
            _app_lock: lock,
            drafts: Mutex::new(BTreeMap::new()),
            streams: Arc::new(Mutex::new(BTreeMap::new())),
            previews: Mutex::new(BTreeMap::new()),
            bound: Mutex::new(BTreeMap::new()),
            grants: Mutex::new(BTreeMap::new()),
            resets: Mutex::new(BTreeMap::new()),
        })
    }
}
#[tauri::command]
pub fn service_catalog() -> Result<ServiceCatalog, ToolboxError> {
    crate::catalog::load_embedded_catalog()
}
#[tauri::command]
pub async fn dashboard_snapshot(
    state: State<'_, AppState>,
) -> Result<DashboardSnapshot, ToolboxError> {
    state.engine.live_status.request_poll(state.engine.clone());
    state.engine.snapshot(None)
}
#[tauri::command]
pub async fn detect_environment(
    state: State<'_, AppState>,
) -> Result<DashboardSnapshot, ToolboxError> {
    state.engine.detect().await
}
#[tauri::command]
pub async fn preflight_service(
    state: State<'_, AppState>,
    service_id: String,
) -> Result<PreflightReport, ToolboxError> {
    state.engine.preflight(&service_id).await
}
#[tauri::command]
pub async fn start_services(
    state: State<'_, AppState>,
    request: StartRequest,
) -> Result<OperationRecord, ToolboxError> {
    state.engine.start(request).await
}
#[tauri::command]
pub async fn stop_services(
    state: State<'_, AppState>,
    request: StopRequest,
) -> Result<OperationRecord, ToolboxError> {
    state.engine.stop(request).await
}
#[tauri::command]
pub async fn retry_operation(
    state: State<'_, AppState>,
    operation_id: String,
) -> Result<OperationRecord, ToolboxError> {
    state.engine.retry(&operation_id).await
}
#[tauri::command]
pub fn cancel_operation(
    state: State<'_, AppState>,
    operation_id: String,
) -> Result<(), ToolboxError> {
    state.engine.cancel(&operation_id)
}
#[tauri::command]
pub fn connection_view(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<ConnectionView, ToolboxError> {
    state.engine.connection(&instance_id)
}
#[tauri::command]
pub fn reveal_secret(
    state: State<'_, AppState>,
    instance_id: String,
    handle: String,
) -> Result<String, ToolboxError> {
    state.engine.reveal(&instance_id, &handle)
}
#[tauri::command]
pub fn config_draft(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<ConfigDraftView, ToolboxError> {
    let draft = crate::config_apply::draft(&state.engine, &instance_id)?;
    let view = draft.view.clone();
    let mut drafts = state.drafts.lock().map_err(|_| error("StateUnavailable"))?;
    drafts.retain(|_, d| d.expires_at > now());
    if drafts.len() >= 32 {
        return Err(error("TooManyDrafts"));
    }
    drafts.insert(view.draft_id.clone(), draft);
    Ok(view)
}
#[tauri::command]
pub async fn validate_draft(
    state: State<'_, AppState>,
    request: ConfigDraft,
) -> Result<ConfigDraftView, ToolboxError> {
    let id = request.draft_id.clone();
    let (view, effective) = {
        let mut drafts = state.drafts.lock().map_err(|_| error("StateUnavailable"))?;
        let draft = drafts.get_mut(&id).ok_or_else(|| error("StaleDraft"))?;
        let view = crate::config_apply::validate(&state.engine, draft, request)?;
        (view, draft.effective.clone())
    };
    if !view.validation_errors.is_empty() {
        return Ok(view);
    }
    let report = state
        .engine
        .preflight_configuration(&view.instance_id, &effective)
        .await?;
    let mut drafts = state.drafts.lock().map_err(|_| error("StateUnavailable"))?;
    let draft = drafts.get_mut(&id).ok_or_else(|| error("StaleDraft"))?;
    if draft.effective.generation_id != effective.generation_id {
        return Err(error("StaleDraft"));
    }
    draft.view.validation_errors.extend(
        report
            .issues
            .iter()
            .filter(|i| i.severity == "error")
            .map(|i| i.code.clone()),
    );
    draft.view.preflight = Some(report);
    Ok(draft.view.clone())
}
#[tauri::command]
pub async fn apply_draft(
    state: State<'_, AppState>,
    draft_id: String,
    accept_unknown_capacity: bool,
) -> Result<OperationRecord, ToolboxError> {
    let draft = state
        .drafts
        .lock()
        .map_err(|_| error("StateUnavailable"))?
        .remove(&draft_id)
        .ok_or_else(|| error("StaleDraft"))?;
    crate::config_apply::apply(&state.engine, draft, accept_unknown_capacity).await
}
#[tauri::command]
pub async fn database_candidates(
    state: State<'_, AppState>,
) -> Result<Vec<DatabaseCandidate>, ToolboxError> {
    let runner = DockerRunner::connect(state.engine.target()?).await?;
    state.engine.database_candidates(&runner).await
}
#[tauri::command]
pub async fn verify_database_binding(
    state: State<'_, AppState>,
    request: VerifyBindingRequest,
) -> Result<DatabaseBinding, ToolboxError> {
    state.engine.verify_native_runtime()?;
    if request
        .allowed_databases
        .iter()
        .any(|n| !crate::bindings::valid_database_name(n))
        || request.username.is_empty()
        || request.username.len() > 128
        || request.password.len() > 16 * 1024
    {
        return Err(error("InvalidBinding"));
    }
    let target = state.engine.target()?;
    let runner = DockerRunner::connect(target.clone()).await?;
    let candidate = state
        .engine
        .database_candidates(&runner)
        .await?
        .into_iter()
        .find(|c| c.candidate_id == request.candidate_id)
        .ok_or_else(|| error("DependencyUnavailable"))?;
    let bytes = runner
        .run_read(&["container", "inspect", &candidate.container_id])
        .await?;
    let v: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| error("DockerOutputInvalid"))?;
    let mut network = None;
    for name in &candidate.network_names {
        if let Some(alias) = v[0]["NetworkSettings"]["Networks"][name]["Aliases"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str())
            .find(|s| {
                !s.is_empty()
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            })
        {
            network = Some((name.clone(), alias.to_owned()));
            break;
        }
    }
    let (network_name, network_alias) =
        network.ok_or_else(|| error("UnsupportedDependencyTopology"))?;
    let binding_id = random_id()?;
    let credentials = crate::bindings::Credentials {
        username: request.username,
        password: zeroize::Zeroizing::new(request.password),
    };
    let binding = DatabaseBinding {
        binding_id: binding_id.clone(),
        target_id: target.target_id.clone(),
        daemon_id: target.daemon_id.clone(),
        engine: candidate.engine.clone(),
        lifecycle_owner: "ExternalObserve".into(),
        container_id: candidate.container_id,
        network_name,
        network_alias,
        container_port: if candidate.engine == "mysql" {
            3306
        } else {
            5432
        },
        host_endpoint: candidate.host_endpoints.first().cloned(),
        credential_ref: SecretRef {
            key: format!("binding-{binding_id}"),
        },
        allowed_databases: request.allowed_databases,
        grant: "None".into(),
        server_fingerprint: String::new(),
    };
    let service = state.engine.service(&binding.engine)?;
    let image = state.engine.images(&service, &target)?.remove(0);
    let context = ImageContext {
        cache_root: state.engine.control.join("image-cache"),
        cancelled: Arc::new(AtomicBool::new(false)),
    };
    let verified =
        crate::bindings::verify_binding(&runner, &binding, &credentials, Some(&image), &context)
            .await?;
    let view = verified.view().clone();
    let mut bindings = state.engine.load_bindings()?;
    bindings.retain(|b| b.engine != view.engine);
    bindings.push(view.clone());
    atomic_write(
        &Path::new(&state.engine.store()?.workspace.root).join("bindings.json"),
        &serde_json::to_vec(&bindings).map_err(|_| error("StateWriteFailed"))?,
    )?;
    let credential_dir = Path::new(&state.engine.store()?.workspace.root).join("credentials");
    private_dir(&credential_dir)?;
    atomic_write(&credential_dir.join(format!("{binding_id}.json")),&serde_json::to_vec(&serde_json::json!({"username":credentials.username,"password":credentials.password.as_str()})).map_err(|_|error("StateWriteFailed"))?)?;
    state
        .bound
        .lock()
        .map_err(|_| error("StateUnavailable"))?
        .insert(
            binding_id,
            Bound {
                verified,
                _credentials: credentials,
            },
        );
    Ok(view)
}
#[tauri::command]
pub fn binding_grant_preview(
    state: State<'_, AppState>,
    binding_id: String,
    service_id: String,
) -> Result<BindingGrantPreview, ToolboxError> {
    let service = state.engine.service(&service_id)?;
    let bound = state.bound.lock().map_err(|_| error("StateUnavailable"))?;
    let bound = bound
        .get(&binding_id)
        .ok_or_else(|| error("BindingReverificationRequired"))?;
    if !service.dependencies.contains(&bound.verified.view().engine) {
        return Err(error("InitializationGrantOutOfScope"));
    }
    let binding = bound.verified.view();
    let instance = state
        .engine
        .instance_for(&service, &state.engine.target()?)?;
    let suffix = &instance.identity.instance_id[..12];
    let databases = if service_id == "temporal" {
        vec!["temporal".into(), "temporal_visibility".into()]
    } else {
        vec![format!("ca_{suffix}_{service_id}")]
    };
    if databases
        .iter()
        .any(|d| !binding.allowed_databases.contains(d))
    {
        return Err(error("InitializationGrantOutOfScope"));
    }
    let accounts = if service_id == "temporal" {
        vec![]
    } else {
        vec![format!("ca_{suffix}_{service_id}")]
    };
    let view = BindingGrantPreview {
        preview_id: random_id()?,
        binding_id,
        service_id,
        grant: InitializationGrant {
            target_id: binding.target_id.clone(),
            server_fingerprint: binding.server_fingerprint.clone(),
            allowed_databases: databases,
            allowed_accounts: accounts,
            operations: vec!["InitializeNamedResources".into()],
            confirmed_at: now(),
        },
        scope_notice: "仅初始化清单中的专属库和账号；不授予外部容器启停、删除或整库重置权限。"
            .into(),
    };
    state
        .grants
        .lock()
        .map_err(|_| error("StateUnavailable"))?
        .insert(view.preview_id.clone(), view.clone());
    Ok(view)
}
#[tauri::command]
pub fn authorize_database_binding(
    state: State<'_, AppState>,
    preview_id: String,
) -> Result<DatabaseBinding, ToolboxError> {
    let preview = state
        .grants
        .lock()
        .map_err(|_| error("StateUnavailable"))?
        .remove(&preview_id)
        .ok_or_else(|| error("StaleGrant"))?;
    if now().saturating_sub(preview.grant.confirmed_at) > 5 * 60 * 1000
        || state.engine.target()?.target_id != preview.grant.target_id
    {
        return Err(error("StaleGrant"));
    }
    let bound = state.bound.lock().map_err(|_| error("StateUnavailable"))?;
    let bound = bound
        .get(&preview.binding_id)
        .ok_or_else(|| error("BindingReverificationRequired"))?;
    let authorized = crate::bindings::authorize_binding(&bound.verified, &preview.grant)?;
    let mut i = state.engine.instance_for(
        &state.engine.service(&preview.service_id)?,
        &state.engine.target()?,
    )?;
    i.bindings.retain(|b| b.engine != authorized.engine);
    i.bindings.push(authorized.binding.clone());
    state.engine.store()?.save(&i)?;
    let dir = Path::new(&state.engine.store()?.workspace.root).join("grants");
    private_dir(&dir)?;
    atomic_write(
        &dir.join(format!("{}.json", preview.binding_id)),
        &serde_json::to_vec(&preview.grant).map_err(|_| error("StateWriteFailed"))?,
    )?;
    Ok(authorized.binding)
}
#[tauri::command]
pub fn preview_diagnostics(state: State<'_, AppState>) -> Result<DiagnosticsPreview, ToolboxError> {
    let snapshot = state.engine.snapshot(None)?;
    let mut sections = BTreeMap::new();
    sections.insert(
        "application".into(),
        format!(
            "Code Ark {}\nservices={}\noperations={}",
            env!("CARGO_PKG_VERSION"),
            snapshot.services.len(),
            snapshot.operations.len()
        ),
    );
    if let Some(target) = snapshot.target {
        sections.insert(
            "environment".into(),
            format!(
                "target={}\nos={}\narchitecture={}",
                target.target_id, target.os_type, target.architecture
            ),
        );
    }
    sections.insert(
        "services".into(),
        serde_json::to_string_pretty(&snapshot.services)
            .map_err(|_| error("DiagnosticUnavailable"))?,
    );
    sections.insert(
        "operations".into(),
        serde_json::to_string_pretty(&snapshot.operations)
            .map_err(|_| error("DiagnosticUnavailable"))?,
    );
    let preview = DiagnosticsPreview {
        preview_id: random_id()?,
        sections,
        excluded_fields: vec![
            ".env".into(),
            "credentials".into(),
            "expanded-compose".into(),
            "full-inspect".into(),
            "process-arguments".into(),
            "workspace-path".into(),
            "logs (默认不收集)".into(),
        ],
        risk_flags: vec![
            "导出前检查每个章节，可排除不希望共享的内容；日志只在明确选择后另行采集。".into(),
        ],
    };
    let mut previews = state
        .previews
        .lock()
        .map_err(|_| error("StateUnavailable"))?;
    if previews.len() > 16 {
        previews.clear();
    }
    previews.insert(preview.preview_id.clone(), preview.clone());
    Ok(preview)
}
#[tauri::command]
pub async fn export_diagnostics(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    confirmation: DiagnosticsConfirmation,
) -> Result<ExportResult, ToolboxError> {
    let mut preview = state
        .previews
        .lock()
        .map_err(|_| error("StateUnavailable"))?
        .remove(&confirmation.preview_id)
        .ok_or_else(|| error("StaleDiagnosticPreview"))?;
    for name in confirmation.excluded_sections {
        if preview.sections.remove(&name).is_none() {
            return Err(error("UnknownDiagnosticSection"));
        }
    }
    if confirmation.include_logs {
        return Err(error("DiagnosticsLogPreviewRequired"));
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_file_name("codeark-diagnostics.json")
        .save_file(move |path| {
            let _ = tx.send(path);
        });
    let path = rx
        .await
        .map_err(|_| error("DialogUnavailable"))?
        .ok_or_else(|| error("Canceled"))?
        .into_path()
        .map_err(|_| error("UnsafePath"))?;
    reject_links(&path)?;
    atomic_write(
        &path,
        &serde_json::to_vec_pretty(&preview.sections)
            .map_err(|_| error("DiagnosticUnavailable"))?,
    )?;
    Ok(ExportResult {
        export_id: preview.preview_id,
        path: path.to_string_lossy().into(),
    })
}
#[tauri::command]
pub fn list_guides(state: State<'_, AppState>) -> Result<Vec<GuideSpec>, ToolboxError> {
    let manifest: serde_json::Value =
        read_json(&state.engine.resources.join("guides/manifest.json"))?;
    if manifest["schemaVersion"] != 1 {
        return Err(error("InvalidGuideManifest"));
    }
    serde_json::from_value(manifest["guides"].clone()).map_err(|_| error("InvalidGuideManifest"))
}
#[tauri::command]
pub fn read_guide(state: State<'_, AppState>, guide_id: String) -> Result<String, ToolboxError> {
    let guides = list_guides(state.clone())?;
    let guide = guides
        .iter()
        .find(|g| g.guide_id == guide_id)
        .ok_or_else(|| error("UnknownGuide"))?;
    let file = guide
        .source_files
        .first()
        .filter(|s| s.starts_with("guides/") && !s.contains("..") && !s.contains('\\'))
        .ok_or_else(|| error("InvalidGuideManifest"))?;
    let path = state.engine.resources.join(file);
    reject_links(&path)?;
    let value = std::fs::read_to_string(path).map_err(|_| error("GuideUnavailable"))?;
    if value.len() > 512 * 1024 {
        return Err(error("GuideTooLarge"));
    }
    Ok(value)
}
#[tauri::command]
pub fn open_official(app: tauri::AppHandle, link_id: String) -> Result<(), ToolboxError> {
    let url = match link_id.as_str() {
        "docker-install" => "https://docs.docker.com/get-started/get-docker/",
        "docker-compose" => "https://docs.docker.com/compose/",
        "tauri-prerequisites" => "https://v2.tauri.app/start/prerequisites/",
        _ => return Err(error("UnknownOfficialLink")),
    };
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|_| error("BrowserUnavailable"))
}
#[tauri::command]
pub async fn open_service_console(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    endpoint_label: String,
) -> Result<(), ToolboxError> {
    let i = state.engine.store()?.instance(&instance_id)?;
    let effective = i
        .applied
        .as_ref()
        .ok_or_else(|| error("NoAppliedConfiguration"))?;
    let endpoint = state
        .engine
        .connection(&instance_id)?
        .host_endpoints
        .into_iter()
        .find(|e| e.label == endpoint_label)
        .ok_or_else(|| error("UnknownEndpoint"))?;
    let mapping = effective
        .ports
        .iter()
        .find(|p| {
            p.component_id == endpoint.component_id
                && format!("{}:{}", p.component_id, p.container_port) == endpoint_label
        })
        .ok_or_else(|| error("UnknownEndpoint"))?;
    let protocol = crate::endpoints::console_protocol(
        &i.identity.service_id,
        &mapping.component_id,
        mapping.container_port,
    )
    .ok_or_else(|| error("EndpointIsNotConsole"))?;
    if endpoint.host != "127.0.0.1" || mapping.host_ip != "127.0.0.1" {
        return Err(error("UnsafeEndpoint"));
    }
    let runner = DockerRunner::connect(state.engine.target()?).await?;
    let mut matched = false;
    for record in &i.containers {
        let raw = runner
            .run_read(&["container", "inspect", &record.container_id])
            .await?;
        let v: serde_json::Value =
            serde_json::from_slice(&raw).map_err(|_| error("DockerOutputInvalid"))?;
        crate::bindings::verify_ownership(&v[0], record, &runner.target)?;
        let c = &v[0];
        if c["Config"]["Labels"]["com.docker.compose.service"] != mapping.component_id {
            continue;
        }
        let internal = if i.identity.service_id == "rocketmq" && mapping.component_id == "console" {
            8082
        } else {
            mapping.container_port
        };
        if c["State"]["Running"] != true
            || c["Config"]["Labels"]["io.codeark.generation-id"] != effective.generation_id
            || !c["NetworkSettings"]["Ports"][format!("{internal}/tcp")]
                .as_array()
                .is_some_and(|a| {
                    a.iter().any(|p| {
                        p["HostIp"] == "127.0.0.1"
                            && p["HostPort"]
                                .as_str()
                                .and_then(|value| value.parse::<u16>().ok())
                                == Some(mapping.host_port)
                    })
                })
        {
            return Err(error("EndpointUnavailable"));
        }
        matched = true;
    }
    if !matched {
        return Err(error("EndpointUnavailable"));
    }
    let suffix = if i.identity.service_id == "xxjob" {
        "/xxl-job-admin"
    } else {
        "/"
    };
    app.opener()
        .open_url(
            format!("{protocol}://127.0.0.1:{}{suffix}", endpoint.port),
            None::<&str>,
        )
        .map_err(|_| error("BrowserUnavailable"))
}
#[tauri::command]
pub async fn choose_workspace(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<DashboardSnapshot, ToolboxError> {
    if state.engine.busy.load(Ordering::Acquire) {
        return Err(error("ResourceBusy"));
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("选择 Code Ark 工作区（保留原工作区数据及容器）")
        .pick_folder(move |path| {
            let _ = tx.send(path);
        });
    let path = rx
        .await
        .map_err(|_| error("DialogUnavailable"))?
        .ok_or_else(|| error("Canceled"))?
        .into_path()
        .map_err(|_| error("UnsafePath"))?;
    let _busy = state.engine.reserve()?;
    let workspace = crate::workspace::open_workspace(&path)?;
    atomic_write(
        &state.engine.control.join("workspace-choice.json"),
        &serde_json::to_vec(&workspace.root).map_err(|_| error("StateWriteFailed"))?,
    )?;
    for stream in state
        .streams
        .lock()
        .map_err(|_| error("StateUnavailable"))?
        .values()
    {
        stream.cancel.store(true, Ordering::Release);
    }
    state
        .drafts
        .lock()
        .map_err(|_| error("StateUnavailable"))?
        .clear();
    state
        .bound
        .lock()
        .map_err(|_| error("StateUnavailable"))?
        .clear();
    state
        .grants
        .lock()
        .map_err(|_| error("StateUnavailable"))?
        .clear();
    *state
        .engine
        .workspace
        .write()
        .map_err(|_| error("StateUnavailable"))? = workspace;
    state.engine.snapshot(None)
}
#[tauri::command]
pub async fn start_log_stream(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request: LogStreamRequest,
    channel: Channel<LogBatch>,
) -> Result<LogStreamHandle, ToolboxError> {
    let i = state.engine.store()?.instance(&request.instance_id)?;
    let target = state.engine.target()?;
    if i.identity.target_id != target.target_id || i.containers.is_empty() {
        return Err(error("OwnershipConflict"));
    }
    if request.tail_lines > 2000 {
        return Err(error("InvalidLogLimit"));
    }
    let id = random_id()?;
    let cancel = Arc::new(AtomicBool::new(false));
    let path = state
        .engine
        .store()?
        .stack(&i.identity.service_id)?
        .join(".env");
    reject_links(&path)?;
    let doc = zeroize::Zeroizing::new(
        std::fs::read_to_string(path).map_err(|_| error("CredentialRepairRequired"))?,
    );
    let secrets = crate::env_store::SecretValues(crate::env_store::parse_env(&doc)?);
    let service = state.engine.service(&i.identity.service_id)?;
    let redactor = Redactor::new(
        service
            .env_schema
            .iter()
            .filter(|f| f.sensitive)
            .filter_map(|f| secrets.0.get(&f.key).cloned()),
    );
    let (done_tx, done_rx) = tokio::sync::watch::channel(false);
    let streams = state.streams.clone();
    {
        let mut map = streams.lock().map_err(|_| error("StateUnavailable"))?;
        if map.len() >= 8 {
            return Err(error("TooManyLogStreams"));
        }
        map.insert(
            id.clone(),
            Stream {
                window: LogWindow::new(id.clone()),
                cancel: cancel.clone(),
                done: done_rx,
            },
        );
    }
    let stream_id = id.clone();
    tokio::spawn(async move {
        let started = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));
        let reader = async {
            let runner = DockerRunner::connect(target).await?;
            let mut cursors = BTreeMap::<String, String>::new();
            let mut seen_queue = std::collections::VecDeque::<[u8; 32]>::new();
            let mut seen = std::collections::HashSet::<[u8; 32]>::new();
            loop {
                if paused.load(Ordering::Acquire) {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
                for record in &i.containers {
                    let raw = runner
                        .run_read(&["container", "inspect", &record.container_id])
                        .await?;
                    let v: serde_json::Value =
                        serde_json::from_slice(&raw).map_err(|_| error("DockerOutputInvalid"))?;
                    let proof = crate::bindings::verify_ownership(&v[0], record, &runner.target)?;
                    let raw = zeroize::Zeroizing::new(
                        runner
                            .container_logs(
                                &proof,
                                request.tail_lines,
                                cursors.get(&record.container_id).map(String::as_str),
                            )
                            .await?,
                    );
                    let text = zeroize::Zeroizing::new(String::from_utf8_lossy(&raw).into_owned());
                    started.store(true, Ordering::Release);
                    for line in text.lines() {
                        use sha2::{Digest, Sha256};
                        let mut hash = Sha256::new();
                        hash.update(record.container_id.as_bytes());
                        hash.update([0]);
                        hash.update(line.as_bytes());
                        let digest: [u8; 32] = hash.finalize().into();
                        if !seen.insert(digest) {
                            continue;
                        }
                        seen_queue.push_back(digest);
                        if seen_queue.len() > 2000
                            && let Some(old) = seen_queue.pop_front()
                        {
                            seen.remove(&old);
                        }
                        if let Some((timestamp, _)) = line.split_once(' ')
                            && timestamp.len() <= 64
                            && timestamp.ends_with('Z')
                            && timestamp.len() >= 20
                            && timestamp
                                .bytes()
                                .all(|b| b.is_ascii_digit() || b"-:+.TZ".contains(&b))
                        {
                            let replace = cursors
                                .get(&record.container_id)
                                .is_none_or(|old| timestamp_key(timestamp) > timestamp_key(old));
                            if replace {
                                cursors.insert(record.container_id.clone(), timestamp.into());
                            }
                        }
                        let line = redactor.redact(line);
                        streams
                            .lock()
                            .map_err(|_| error("StateUnavailable"))?
                            .get_mut(&stream_id)
                            .ok_or_else(|| error("LogStreamExpired"))?
                            .window
                            .push(line);
                    }
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            #[allow(unreachable_code)]
            Ok::<(), ToolboxError>(())
        };
        let deliver = async {
            let mut phase = "Starting";
            loop {
                let batch = {
                    let mut map = streams.lock().map_err(|_| error("StateUnavailable"))?;
                    map.get_mut(&stream_id)
                        .ok_or_else(|| error("LogStreamExpired"))?
                        .window
                        .batch(Instant::now())
                };
                match batch {
                    Err(e) if e.code == "AwaitingConsumer" => {
                        paused.store(true, Ordering::Release);
                        if phase != "AwaitingConsumer" {
                            phase = "AwaitingConsumer";
                            let _ = app.emit(
                                "log-stream-state",
                                LogStreamStateEvent {
                                    stream_id: stream_id.clone(),
                                    phase: phase.into(),
                                    message_code: Some(e.code),
                                },
                            );
                        }
                    }
                    Err(e) => return Err(e),
                    Ok(batch) => {
                        paused.store(false, Ordering::Release);
                        if started.load(Ordering::Acquire) && phase != "Active" {
                            phase = "Active";
                            let _ = app.emit(
                                "log-stream-state",
                                LogStreamStateEvent {
                                    stream_id: stream_id.clone(),
                                    phase: phase.into(),
                                    message_code: None,
                                },
                            );
                        }
                        if let Some(batch) = batch
                            && channel.send(batch).is_err()
                        {
                            return Ok::<(), ToolboxError>(());
                        }
                    }
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        };
        let stop = async {
            while !cancel.load(Ordering::Acquire) {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        };
        let result = tokio::select! {result=reader=>result,result=deliver=>result,_=stop=>Ok(())};
        let user_stopped = cancel.load(Ordering::Acquire);
        cancel.store(true, Ordering::Release);
        let (phase, message) = match result {
            Err(e) if !user_stopped => {
                let phase = if e.code == "LogStreamExpired" {
                    "Expired"
                } else {
                    "Failed"
                };
                (phase, Some(e.code))
            }
            _ => ("Stopped", None),
        };
        let _ = app.emit(
            "log-stream-state",
            LogStreamStateEvent {
                stream_id: stream_id.clone(),
                phase: phase.into(),
                message_code: message,
            },
        );
        if let Ok(mut map) = streams.lock() {
            map.remove(&stream_id);
        }
        let _ = done_tx.send(true);
    });
    Ok(LogStreamHandle {
        stream_id: id,
        phase: "Starting".into(),
    })
}
fn timestamp_key(timestamp: &str) -> String {
    let Some(body) = timestamp.strip_suffix('Z') else {
        return String::new();
    };
    let (seconds, fraction) = body.split_once('.').unwrap_or((body, ""));
    format!("{seconds}.{:0<9}", fraction)
}
#[tauri::command]
pub fn ack_log_batch(state: State<'_, AppState>, ack: LogAck) -> Result<(), ToolboxError> {
    state
        .streams
        .lock()
        .map_err(|_| error("StateUnavailable"))?
        .get_mut(&ack.stream_id)
        .ok_or_else(|| error("LogStreamExpired"))?
        .window
        .ack(&ack)
}
#[tauri::command]
pub async fn stop_log_stream(
    state: State<'_, AppState>,
    stream_id: String,
) -> Result<(), ToolboxError> {
    let stream = state
        .streams
        .lock()
        .map_err(|_| error("StateUnavailable"))?
        .remove(&stream_id);
    if let Some(mut stream) = stream {
        stream.cancel.store(true, Ordering::Release);
        if !*stream.done.borrow() {
            tokio::time::timeout(Duration::from_secs(2), stream.done.changed())
                .await
                .map_err(|_| error("LogStreamCloseTimedOut"))?
                .map_err(|_| error("LogStreamClosed"))?;
        }
    }
    Ok(())
}
pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let base = dirs::data_local_dir().ok_or_else(|| error("UserDirectoryUnavailable"))?;
    let control = base
        .join(if cfg!(target_os = "linux") {
            "codeark"
        } else {
            "CodeArk"
        })
        .join("control");
    private_dir(&control)?;
    let selected = control.join("workspace-choice.json");
    let root = if selected.exists() {
        PathBuf::from(read_json::<String>(&selected)?)
    } else {
        dirs::home_dir()
            .ok_or_else(|| error("UserDirectoryUnavailable"))?
            .join(".code-ark")
    };
    let workspace = crate::workspace::open_workspace(&root)?;
    let resources = if cfg!(debug_assertions) {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("resources")
    } else {
        app.path().resource_dir()?.join("resources")
    };
    let mut engine = Engine::new(workspace, control, resources)?;
    let handle = app.handle().clone();
    engine.emit = Arc::new(move |event| {
        let _ = handle.emit("step-progress", event);
    });
    app.manage(AppState::new(engine)?);
    Ok(())
}

#[tauri::command]
pub async fn plan_reset(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<ResetPlan, ToolboxError> {
    let plan = crate::reset::plan(&state.engine, &instance_id).await?;
    let view = plan.view.clone();
    let mut plans = state.resets.lock().map_err(|_| error("StateUnavailable"))?;
    plans.retain(|_, p| p.view.expires_at > now());
    if plans.len() >= 16 {
        return Err(error("TooManyResetPlans"));
    }
    plans.insert(view.plan_id.clone(), plan);
    Ok(view)
}
#[tauri::command]
pub async fn execute_reset(
    state: State<'_, AppState>,
    request: ResetConfirmation,
) -> Result<OperationRecord, ToolboxError> {
    let plan = state
        .resets
        .lock()
        .map_err(|_| error("StateUnavailable"))?
        .remove(&request.plan_id)
        .ok_or_else(|| error("ResetPlanExpired"))?;
    crate::reset::execute(&state.engine, plan, request).await
}
#[tauri::command]
pub async fn export_guide_sources(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    guide_id: String,
) -> Result<ExportResult, ToolboxError> {
    let guides = list_guides(state.clone())?;
    let guide = guides
        .into_iter()
        .find(|g| g.guide_id == guide_id)
        .ok_or_else(|| error("UnknownGuide"))?;
    let manifest: serde_json::Value = read_json(
        &state
            .engine
            .resources
            .join(".codeark-resource-manifest.json"),
    )?;
    if manifest["producer"] != "CodeArkResources" || manifest["schemaVersion"] != 1 {
        return Err(error("InvalidResourceManifest"));
    }
    let mut files = vec![];
    for source in &guide.source_files {
        let p = Path::new(source);
        if p.is_absolute()
            || p.components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            || source.contains('\\')
            || !(source.starts_with("guides/")
                || source.starts_with("examples/")
                || source.starts_with("blueprints/mediacrawler/"))
        {
            return Err(error("UnsafePath"));
        }
        let path = state.engine.resources.join(source);
        reject_links(&path)?;
        let bytes = std::fs::read(&path).map_err(|_| error("GuideUnavailable"))?;
        if bytes.len() > 1024 * 1024 {
            return Err(error("GuideTooLarge"));
        }
        use sha2::{Digest, Sha256};
        if !source.starts_with("blueprints/") {
            let expected = manifest["files"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|f| f["path"] == *source)
                .and_then(|f| f["sha256"].as_str())
                .ok_or_else(|| error("InvalidResourceManifest"))?;
            if format!("{:x}", Sha256::digest(&bytes)) != expected {
                return Err(error("GuideDigestMismatch"));
            }
        } else {
            let blueprint: serde_json::Value =
                read_json(&state.engine.resources.join("blueprints/manifest.json"))?;
            let relative = source
                .strip_prefix("blueprints/")
                .ok_or_else(|| error("UnsafePath"))?;
            let expected = blueprint["files"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|f| f["path"] == relative)
                .and_then(|f| f["sha256"].as_str())
                .ok_or_else(|| error("InvalidResourceManifest"))?;
            if format!("{:x}", Sha256::digest(&bytes)) != expected {
                return Err(error("GuideDigestMismatch"));
            }
        }
        files.push((source.clone(), bytes));
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("选择教学源码导出位置（创建独立目录）")
        .pick_folder(move |path| {
            let _ = tx.send(path);
        });
    let parent = rx
        .await
        .map_err(|_| error("DialogUnavailable"))?
        .ok_or_else(|| error("Canceled"))?
        .into_path()
        .map_err(|_| error("UnsafePath"))?;
    reject_links(&parent)?;
    let export_id = random_id()?;
    let target = parent.join(format!("CodeArk-{}-{}", guide.guide_id, &export_id[..8]));
    if target.exists() {
        return Err(error("ExportDirectoryExists"));
    }
    private_dir(&target)?;
    for (source, bytes) in files {
        let path = target.join(source);
        private_dir(path.parent().ok_or_else(|| error("UnsafePath"))?)?;
        atomic_write(&path, &bytes)?;
    }
    Ok(ExportResult {
        export_id,
        path: target.to_string_lossy().into(),
    })
}
