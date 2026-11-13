pub mod catalog;
pub mod contracts;

pub mod workspace;

#[cfg(feature = "desktop")]
mod desktop;
#[cfg(feature = "desktop")]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(desktop::setup)
        .invoke_handler(tauri::generate_handler![
            desktop::service_catalog,
            desktop::dashboard_snapshot,
            desktop::detect_environment,
            desktop::preflight_service,
            desktop::start_services,
            desktop::stop_services,
            desktop::retry_operation,
            desktop::cancel_operation,
            desktop::connection_view,
            desktop::reveal_secret,
            desktop::config_draft,
            desktop::validate_draft,
            desktop::apply_draft,
            desktop::database_candidates,
            desktop::verify_database_binding,
            desktop::binding_grant_preview,
            desktop::binding_requirements,
            desktop::authorize_database_binding,
            desktop::preview_diagnostics,
            desktop::export_diagnostics,
            desktop::list_guides,
            desktop::read_guide,
            desktop::export_guide_sources,
            desktop::open_official,
            desktop::open_service_console,
            desktop::choose_workspace,
            desktop::start_log_stream,
            desktop::ack_log_batch,
            desktop::stop_log_stream,
            desktop::plan_reset,
            desktop::execute_reset
        ])
        .run(tauri::generate_context!())
        .expect("Code Ark could not start");
}

pub mod env_store;

pub mod blueprint;

pub mod docker_cli;

pub mod preflight;
pub mod storage_probe;

pub mod image_store;

pub mod bindings;

pub mod database_claims;

pub mod initialization_store;
pub mod provisioner;
mod dependencies;

pub(crate) mod probe_clients;
pub mod probes;

pub mod acceptance_guard;
pub mod config_apply;
pub mod locks;
pub mod log_stream;
pub mod operation_store;
pub mod orchestrator;
pub mod reset;
pub mod runtime_state;
pub mod sql_provisioner;
pub mod sql_schema;
pub mod usage_index;

pub mod endpoints;
pub mod live_status;
pub mod managed_data;
pub(crate) mod service_bootstrap;
pub mod recovery;
#[cfg(windows)]
mod windows_permissions;
