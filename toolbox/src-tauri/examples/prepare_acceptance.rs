use codeark_toolbox::{
    acceptance_guard::AcceptanceConfig, docker_cli::discover_target, workspace::open_workspace,
};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .ok_or("usage: prepare_acceptance <new-config-file>")?;
    if output.exists() {
        return Err("configuration already exists".into());
    }
    let target = discover_target(None, None).await?;
    let parent = output.parent().ok_or("configuration parent required")?;
    std::fs::create_dir_all(parent)?;
    let root = parent.canonicalize()?;
    let workspace = open_workspace(&root.join("workspace"))?;
    let config = AcceptanceConfig {
        target_id: target.target_id,
        expected_daemon_id: target.daemon_id,
        cli_path: target.cli_path.into(),
        context_name: target.context_name,
        workspace_root: workspace.root.into(),
        control_root: root
            .join("control")
            .canonicalize()
            .unwrap_or_else(|_| root.join("control")),
        run_id: workspace.workspace_id,
        port_min: 27000,
        port_max: 28999,
        allow_create: true,
        allow_destructive_owned_only: true,
    };
    let bytes = serde_json::to_vec_pretty(&config)?;
    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(&output)?.write_all(&bytes)?;
    println!("{}", output.display());
    Ok(())
}
