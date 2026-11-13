//! Explicit, owned-service initialization; readiness probes remain read-only.
use crate::{
    contracts::*, docker_cli::DockerRunner, env_store::SecretValues, orchestrator::Engine,
    runtime_state::RuntimeInstance, workspace::error,
};
use serde::Deserialize;
use serde_json::Value;
use std::{
    future::Future,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use zeroize::{Zeroize, Zeroizing};

trait BootstrapGuard {
    fn verify(&self) -> impl Future<Output = Result<(), ToolboxError>> + Send;
}

#[derive(Debug, PartialEq)]
enum Outcome {
    Initialized,
    AlreadyInitialized,
}

async fn initialize_admin<G: BootstrapGuard>(
    port: u16,
    password: &str,
    guard: &G,
    cancelled: &AtomicBool,
) -> Result<Outcome, ToolboxError> {
    if password.is_empty() || password.len() > 72 || port == 0 {
        return Err(error("CredentialRepairRequired"));
    }
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|_| error("NacosBootstrapUnavailable"))?;
    let base = format!("http://127.0.0.1:{port}/nacos/v3/auth/user");
    let started = std::time::Instant::now();
    let mut initialized = false;
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Err(error("Cancelled"));
        }
        guard.verify().await?;
        let login = client
            .post(format!("{base}/login"))
            .form(&[("username", "nacos"), ("password", password)])
            .send()
            .await;
        match login {
            Ok(response) if response.status().is_success() => {
                #[derive(Deserialize)]
                struct Login {
                    #[serde(rename = "accessToken")]
                    access_token: Option<String>,
                }
                let raw = bounded(response).await?;
                let mut login: Login = serde_json::from_slice(&raw)
                    .map_err(|_| error("NacosBootstrapResponseInvalid"))?;
                let authenticated = login.access_token.as_ref().is_some_and(|s| !s.is_empty());
                login.access_token.zeroize();
                if !authenticated {
                    return Err(error("NacosBootstrapResponseInvalid"));
                }
                guard.verify().await?;
                return Ok(if initialized {
                    Outcome::Initialized
                } else {
                    Outcome::AlreadyInitialized
                });
            }
            Ok(response) if matches!(response.status().as_u16(), 401 | 403) && !initialized => {
                if cancelled.load(Ordering::Acquire) {
                    return Err(error("Cancelled"));
                }
                guard.verify().await?;
                // Bootstrap-only endpoint, never password update/reset. Nacos 3.3.0
                // UserControllerV3#createAdminUser refuses existing global admins (code 409).
                let response = client
                    .post(format!("{base}/admin"))
                    .form(&[("password", password)])
                    .send()
                    .await
                    .map_err(|_| error("NacosBootstrapUnavailable"))?;
                let http_status = response.status().as_u16();
                #[derive(Deserialize)]
                struct ResultCode {
                    code: i64,
                }
                let raw = bounded(response).await?;
                let result: ResultCode = serde_json::from_slice(&raw)
                    .map_err(|_| error("NacosBootstrapResponseInvalid"))?;
                if result.code == 409 || http_status == 409 {
                    // Concurrent initialization may have used the same saved password.
                    // Recheck once; a mismatch is repair-required and never changes an account.
                    if cancelled.load(Ordering::Acquire) {
                        return Err(error("Cancelled"));
                    }
                    guard.verify().await?;
                    let response = client
                        .post(format!("{base}/login"))
                        .form(&[("username", "nacos"), ("password", password)])
                        .send()
                        .await
                        .map_err(|_| error("NacosBootstrapUnavailable"))?;
                    if matches!(response.status().as_u16(), 401 | 403) {
                        return Err(error("CredentialRepairRequired"));
                    }
                    if !response.status().is_success() {
                        return Err(error("NacosBootstrapUnavailable"));
                    }
                    #[derive(Deserialize)]
                    struct Login {
                        #[serde(rename = "accessToken")]
                        access_token: Option<String>,
                    }
                    let raw = bounded(response).await?;
                    let mut login: Login = serde_json::from_slice(&raw)
                        .map_err(|_| error("NacosBootstrapResponseInvalid"))?;
                    let authenticated = login.access_token.as_ref().is_some_and(|s| !s.is_empty());
                    login.access_token.zeroize();
                    if !authenticated {
                        return Err(error("CredentialRepairRequired"));
                    }
                    guard.verify().await?;
                    return Ok(Outcome::AlreadyInitialized);
                }
                if !(200..300).contains(&http_status) || result.code != 0 {
                    return Err(error("NacosBootstrapFailed"));
                }
                initialized = true;
            }
            Ok(response) if matches!(response.status().as_u16(), 401 | 403) => {} // Allow the auth cache to catch up after creation.
            Ok(response) if response.status().is_server_error() => {}
            Err(_) => {}
            _ => return Err(error("NacosBootstrapResponseInvalid")),
        }
        if started.elapsed() >= Duration::from_secs(180) {
            return Err(error("NacosBootstrapUnavailable"));
        }
        if !initialized {
            tokio::time::sleep(Duration::from_secs(2)).await;
        } else {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

async fn bounded(mut response: reqwest::Response) -> Result<Zeroizing<Vec<u8>>, ToolboxError> {
    let mut bytes = Zeroizing::new(Vec::new());
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| error("NacosBootstrapResponseInvalid"))?
    {
        if bytes.len() + chunk.len() > 64 * 1024 {
            return Err(error("NacosBootstrapResponseInvalid"));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

struct OwnedNacos<'a> {
    engine: &'a Engine,
    runner: &'a DockerRunner,
    instance: &'a RuntimeInstance,
    effective: &'a EffectiveConfig,
    port: u16,
}
impl BootstrapGuard for OwnedNacos<'_> {
    async fn verify(&self) -> Result<(), ToolboxError> {
        let identity = &self.instance.identity;
        let current = self.engine.target()?;
        if identity.service_id != "nacos"
            || current.target_id != identity.target_id
            || current.daemon_id != identity.daemon_id
            || current != self.runner.target
            || self.effective.instance_id != identity.instance_id
            || self.effective.target_id != identity.target_id
        {
            return Err(error("TargetChanged"));
        }
        self.runner.verify_identity().await?;
        if self.instance.containers.len() != 1 {
            return Err(error("OwnershipConflict"));
        }
        let record = &self.instance.containers[0];
        if record.workspace_id != identity.workspace_id
            || record.instance_id != identity.instance_id
            || record.service_id != "nacos"
        {
            return Err(error("OwnershipConflict"));
        }
        let raw = self
            .runner
            .run_read(&["container", "inspect", &record.container_id])
            .await?;
        let inspected: Vec<Value> =
            serde_json::from_slice(&raw).map_err(|_| error("DockerOutputInvalid"))?;
        let container = inspected
            .first()
            .filter(|_| inspected.len() == 1)
            .ok_or_else(|| error("OwnershipConflict"))?;
        crate::bindings::verify_ownership(container, record, &current)?;
        if container["State"]["Running"] != true
            || container["Config"]["Labels"]["io.codeark.generation-id"]
                != self.effective.generation_id
            || container["Config"]["Labels"]["com.docker.compose.project"] != identity.project_name
            || container["Config"]["Labels"]["com.docker.compose.service"] != "nacos"
        {
            return Err(error("OwnershipConflict"));
        }
        let published = container["NetworkSettings"]["Ports"]["8848/tcp"]
            .as_array()
            .ok_or_else(|| error("ProbeEndpointChanged"))?;
        if published.len() != 1
            || published[0]["HostIp"] != "127.0.0.1"
            || published[0]["HostPort"]
                .as_str()
                .and_then(|s| s.parse::<u16>().ok())
                != Some(self.port)
        {
            return Err(error("ProbeEndpointChanged"));
        }
        let volume_name = format!("{}-nacos-data", identity.project_name);
        let data_mounts = container["Mounts"]
            .as_array()
            .ok_or_else(|| error("OwnershipConflict"))?
            .iter()
            .filter(|v| v["Destination"] == "/home/nacos/data")
            .collect::<Vec<_>>();
        if data_mounts.len() != 1
            || data_mounts[0]["Type"] != "volume"
            || data_mounts[0]["Name"] != volume_name
            || data_mounts[0]["RW"] != true
        {
            return Err(error("OwnershipConflict"));
        }
        let raw = self
            .runner
            .run_read(&["volume", "inspect", &volume_name])
            .await?;
        let volumes: Vec<Value> =
            serde_json::from_slice(&raw).map_err(|_| error("DockerOutputInvalid"))?;
        let volume = volumes
            .first()
            .filter(|_| volumes.len() == 1)
            .ok_or_else(|| error("OwnershipConflict"))?;
        if volume["Name"] != volume_name {
            return Err(error("OwnershipConflict"));
        }
        for (key, value) in [
            ("io.codeark.workspace-id", &identity.workspace_id),
            ("io.codeark.instance-id", &identity.instance_id),
            ("io.codeark.service-id", &identity.service_id),
            ("io.codeark.blueprint-sha", &record.blueprint_sha),
        ] {
            if volume["Labels"][key].as_str() != Some(value) {
                return Err(error("OwnershipConflict"));
            }
        }
        Ok(())
    }
}

pub(crate) async fn nacos(
    engine: &Engine,
    runner: &DockerRunner,
    instance: &RuntimeInstance,
    effective: &EffectiveConfig,
    secrets: &SecretValues,
    cancelled: &AtomicBool,
) -> Result<(), ToolboxError> {
    if secrets
        .0
        .get("NACOS_AUTH_USERNAME")
        .map(String::as_str)
        .unwrap_or("nacos")
        != "nacos"
    {
        return Err(error("CredentialRepairRequired"));
    }
    let password = secrets
        .0
        .get("NACOS_AUTH_PASSWORD")
        .ok_or_else(|| error("CredentialRepairRequired"))?;
    let ports = effective
        .ports
        .iter()
        .filter(|p| p.component_id == "nacos" && p.container_port == 8848 && p.protocol == "tcp")
        .collect::<Vec<_>>();
    if ports.len() != 1 || ports[0].host_ip != "127.0.0.1" {
        return Err(error("ProbeEndpointChanged"));
    }
    let guard = OwnedNacos {
        engine,
        runner,
        instance,
        effective,
        port: ports[0].host_port,
    };
    initialize_admin(guard.port, password, &guard, cancelled).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex, atomic::AtomicUsize};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    struct Guard {
        calls: AtomicUsize,
        reject_at: usize,
    }
    impl Guard {
        fn valid() -> Self {
            Self {
                calls: AtomicUsize::new(0),
                reject_at: usize::MAX,
            }
        }
    }
    impl BootstrapGuard for Guard {
        async fn verify(&self) -> Result<(), ToolboxError> {
            if self.calls.fetch_add(1, Ordering::AcqRel) + 1 == self.reject_at {
                return Err(error("TargetChanged"));
            }
            Ok(())
        }
    }
    async fn server(
        replies: Vec<(u16, &'static str)>,
    ) -> (u16, Arc<Mutex<Vec<String>>>, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(vec![]));
        let captured = requests.clone();
        let task = tokio::spawn(async move {
            for (status, body) in replies {
                let (mut stream, _) =
                    tokio::time::timeout(Duration::from_secs(5), listener.accept())
                        .await
                        .unwrap()
                        .unwrap();
                let mut request = vec![];
                loop {
                    let mut buffer = [0; 4096];
                    let count = stream.read(&mut buffer).await.unwrap();
                    assert!(count > 0);
                    request.extend_from_slice(&buffer[..count]);
                    if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&request[..end]);
                        let length = headers
                            .lines()
                            .find_map(|line| {
                                line.to_lowercase()
                                    .strip_prefix("content-length: ")
                                    .and_then(|v| v.parse::<usize>().ok())
                            })
                            .unwrap_or(0);
                        if request.len() >= end + 4 + length {
                            break;
                        }
                    }
                }
                captured
                    .lock()
                    .unwrap()
                    .push(String::from_utf8(request).unwrap());
                let response = format!(
                    "HTTP/1.1 {status} Response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).await.unwrap();
            }
        });
        (port, requests, task)
    }
    const LOGIN: &str = r#"{"accessToken":"fixture-token","username":"nacos","globalAdmin":true}"#;
    #[tokio::test]
    async fn fresh_initialization_sets_saved_password_once_and_restart_only_logs_in() {
        let (port, requests, task) = server(vec![
            (403, "User not found!"),
            (
                200,
                r#"{"code":0,"data":{"username":"nacos","password":"fixture-password"}}"#,
            ),
            (200, LOGIN),
            (200, LOGIN),
        ])
        .await;
        let guard = Guard::valid();
        let cancelled = AtomicBool::new(false);
        assert_eq!(
            initialize_admin(port, "fixture-password", &guard, &cancelled)
                .await
                .unwrap(),
            Outcome::Initialized
        );
        assert_eq!(
            initialize_admin(port, "fixture-password", &guard, &cancelled)
                .await
                .unwrap(),
            Outcome::AlreadyInitialized
        );
        task.await.unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(
            requests
                .iter()
                .filter(|r| r.starts_with("POST /nacos/v3/auth/user/admin "))
                .count(),
            1
        );
        assert!(requests[1].ends_with("password=fixture-password"));
        assert!(requests.iter().all(|r| !r.starts_with("PUT ")));
    }
    #[tokio::test]
    async fn existing_wrong_password_is_never_overwritten_even_when_conflict_has_http_200() {
        let (port, requests, task) = server(vec![
            (403, "User not found!"),
            (
                200,
                r#"{"code":409,"message":"have admin user cannot use it.","data":null}"#,
            ),
            (403, "User not found!"),
        ])
        .await;
        let result = initialize_admin(
            port,
            "fixture-wrong-password",
            &Guard::valid(),
            &AtomicBool::new(false),
        )
        .await;
        assert_eq!(result.unwrap_err().code, "CredentialRepairRequired");
        task.await.unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 3);
        assert!(
            requests
                .iter()
                .all(|r| !r.starts_with("PUT ") && !r.starts_with("DELETE "))
        );
    }
    #[tokio::test]
    async fn target_change_after_login_rejection_blocks_the_initialization_write() {
        let (port, requests, task) = server(vec![(403, "User not found!")]).await;
        let guard = Guard {
            calls: AtomicUsize::new(0),
            reject_at: 2,
        };
        assert_eq!(
            initialize_admin(port, "fixture-password", &guard, &AtomicBool::new(false))
                .await
                .unwrap_err()
                .code,
            "TargetChanged"
        );
        task.await.unwrap();
        assert_eq!(requests.lock().unwrap().len(), 1);
    }
    #[tokio::test]
    async fn successful_login_never_calls_the_bootstrap_endpoint() {
        let (port, requests, task) = server(vec![(200, LOGIN)]).await;
        assert_eq!(
            initialize_admin(
                port,
                "fixture-password",
                &Guard::valid(),
                &AtomicBool::new(false)
            )
            .await
            .unwrap(),
            Outcome::AlreadyInitialized
        );
        task.await.unwrap();
        assert_eq!(requests.lock().unwrap().len(), 1);
    }
    #[tokio::test]
    async fn cancellation_blocks_all_http_requests() {
        assert_eq!(
            initialize_admin(
                1,
                "fixture-password",
                &Guard::valid(),
                &AtomicBool::new(true)
            )
            .await
            .unwrap_err()
            .code,
            "Cancelled"
        );
    }
    #[tokio::test]
    async fn malformed_successful_login_does_not_authorize_initialization() {
        let (port, requests, task) = server(vec![(
            200,
            r#"{"data":{"accessToken":"unexpected-shape"}}"#,
        )])
        .await;
        assert_eq!(
            initialize_admin(
                port,
                "fixture-password",
                &Guard::valid(),
                &AtomicBool::new(false)
            )
            .await
            .unwrap_err()
            .code,
            "NacosBootstrapResponseInvalid"
        );
        task.await.unwrap();
        assert_eq!(requests.lock().unwrap().len(), 1);
    }
    #[tokio::test]
    async fn owned_guard_rejects_a_switched_engine_target_before_docker_or_http() {
        let directory = tempfile::tempdir().unwrap();
        let engine = Engine::new(
            crate::workspace::open_workspace(&directory.path().join("workspace")).unwrap(),
            directory.path().join("control"),
            directory.path().join("unused-resources"),
        )
        .unwrap();
        let target = DockerTarget {
            target_id: "original-target".into(),
            daemon_id: "original-daemon".into(),
            cli_path: directory
                .path()
                .join("must-not-execute")
                .to_string_lossy()
                .into(),
            context_name: "fixture".into(),
            endpoint_kind: "unix".into(),
            os_type: "linux".into(),
            architecture: "arm64".into(),
        };
        let instance = engine
            .instance_for(&engine.service("nacos").unwrap(), &target)
            .unwrap();
        let effective = EffectiveConfig {
            target_id: target.target_id.clone(),
            instance_id: instance.identity.instance_id.clone(),
            generation_id: "generation".into(),
            ports: vec![],
            endpoints: vec![],
            secret_refs: vec![],
            config_digest: String::new(),
            image_artifacts: vec![],
        };
        *engine.target.write().unwrap() = Some(DockerTarget {
            target_id: "another-target".into(),
            ..target.clone()
        });
        let runner = DockerRunner { target };
        let guard = OwnedNacos {
            engine: &engine,
            runner: &runner,
            instance: &instance,
            effective: &effective,
            port: 1,
        };
        assert_eq!(guard.verify().await.unwrap_err().code, "TargetChanged");
    }
}
