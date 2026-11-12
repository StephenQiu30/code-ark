pub mod catalog;
pub mod contracts;

pub mod workspace;

#[cfg(feature = "desktop")]
#[tauri::command]
fn service_catalog() -> Result<contracts::ServiceCatalog, contracts::ToolboxError> {
    catalog::load_embedded_catalog()
}
#[cfg(feature = "desktop")]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![service_catalog])
        .run(tauri::generate_context!())
        .expect("Code Ark could not start");
}

pub mod env_store;
