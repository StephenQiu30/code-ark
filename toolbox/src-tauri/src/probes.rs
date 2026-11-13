use crate::contracts::{
    DatabaseBinding, EffectiveConfig, ImageArtifactRef, InstanceIdentity, ProbeResult, ServiceSpec,
};
use crate::docker_cli::DockerRunner;
use crate::env_store::SecretValues;

pub struct ProbeContext<'a> {
    pub control_root: &'a std::path::Path,
    pub runner: &'a DockerRunner,
    pub instance: &'a InstanceIdentity,
    pub effective: &'a EffectiveConfig,
    pub service: &'a ServiceSpec,
    pub secrets: &'a SecretValues,
    pub bindings: &'a [DatabaseBinding],
    pub tool_image: Option<&'a ImageArtifactRef>,
    pub resources_root: &'a std::path::Path,
}

use crate::contracts::{ProbeSpec, ToolboxError};
use crate::probe_clients;
use reqwest::{Client, RequestBuilder};
use serde_json::{Value, json};
use sqlx::Connection;
use std::collections::HashSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type Check = Result<Option<&'static str>, ToolboxError>;
fn err(code: &str) -> ToolboxError {
    ToolboxError::new(code)
}
fn checked(value: bool) -> Check {
    if value {
        Ok(None)
    } else {
        Err(err("ProtocolUnavailable"))
    }
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn requires_tool(service: &ServiceSpec) -> bool {
    service.probes.iter().any(|p| {
        matches!(
            p.kind.as_str(),
            "s3" | "amqp"
                | "kafka"
                | "canal"
                | "rocketmq"
                | "rocket-route"
                | "seata"
                | "xxjob"
                | "nacos"
        )
    })
}

fn supported(p: &ProbeSpec) -> bool {
    match p.role.as_str() {
        "job" => p.kind == "job-exit",
        "daemon" => matches!(
            p.kind.as_str(),
            "sql"
                | "redis"
                | "s3"
                | "kafka"
                | "canal"
                | "amqp"
                | "rocketmq"
                | "rocket-route"
                | "temporal"
                | "xxjob"
                | "nacos"
                | "http"
                | "seata"
                | "elasticsearch"
                | "monitoring"
                | "rss"
                | "search"
        ),
        _ => false,
    }
}
pub fn ready(service: &ServiceSpec, results: &[ProbeResult]) -> bool {
    if service.components.is_empty()
        || service.probes.is_empty()
        || results.len() != service.probes.len()
    {
        return false;
    }
    let ids: HashSet<_> = results
        .iter()
        .map(|r| (&r.component_id, &r.probe_id))
        .collect();
    ids.len() == results.len()
        && service.components.iter().all(|c| {
            service
                .probes
                .iter()
                .any(|p| p.component_id == c.id && p.role == c.role && p.required)
        })
        && service.probes.iter().all(|p| {
            supported(p)
                && p.required
                && service
                    .components
                    .iter()
                    .any(|c| c.id == p.component_id && c.role == p.role)
                && results.iter().any(|r| {
                    r.probe_id == p.probe_id
                        && r.component_id == p.component_id
                        && r.success
                        && r.error_code.is_none()
                        && r.sampled_at > 0
                })
        })
}

pub(crate) fn valid_id(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit())
}
fn verify_state(state: &Value, role: &str) -> Result<(), ToolboxError> {
    if state["OOMKilled"] != false || state["Error"] != "" {
        return Err(err("ContainerFailed"));
    }
    match role {
        "job" if state["Status"] == "exited" && state["ExitCode"].as_i64() == Some(0) => Ok(()),
        "job" => Err(err("JobIncomplete")),
        "daemon"
            if state["Status"] == "running"
                && state["Running"] == true
                && state["Paused"] == false
                && (state.get("Health").is_none() || state["Health"]["Status"] == "healthy") =>
        {
            Ok(())
        }
        "daemon" => Err(err("DaemonNotHealthy")),
        _ => Err(err("UnknownComponentRole")),
    }
}
struct Observed {
    id: String,
    started: String,
    ports: Value,
}
impl ProbeContext<'_> {
    fn field(&self, key: &str) -> Result<&str, ToolboxError> {
        self.secrets
            .0
            .get(key)
            .filter(|v| !v.is_empty())
            .map(String::as_str)
            .ok_or_else(|| err("ProbeCredentialOrSettingMissing"))
    }
    fn provided<'a>(&'a self, key: &str, fallback: &'a str) -> &'a str {
        self.secrets
            .0
            .get(key)
            .map(String::as_str)
            .unwrap_or(fallback)
    }
    async fn observe(&self, p: &ProbeSpec) -> Result<Observed, ToolboxError> {
        let i = self.instance;
        let pairs = [
            ("io.codeark.workspace-id", i.workspace_id.as_str()),
            ("io.codeark.instance-id", &i.instance_id),
            ("io.codeark.service-id", &i.service_id),
            ("com.docker.compose.project", &i.project_name),
            ("com.docker.compose.service", &p.component_id),
            ("io.codeark.generation-id", &self.effective.generation_id),
        ];
        let mut args = vec!["ps".into(), "-aq".into(), "--no-trunc".into()];
        for (key, value) in pairs {
            args.extend(["--filter".into(), format!("label={key}={value}").into()]);
        }
        let raw = self.runner.run_args(&args, Duration::from_secs(5)).await?;
        let id = std::str::from_utf8(&raw).unwrap_or("").trim();
        if !valid_id(id) {
            return Err(err("ComponentIdentityUnverified"));
        }
        let raw=self.runner.run_read(&["container","inspect","--format",r#"{"id":{{json .Id}},"state":{{json .State}},"ports":{{json .NetworkSettings.Ports}}}"#,id]).await?;
        let v: Value = serde_json::from_slice(&raw).map_err(|_| err("ProbeObservationInvalid"))?;
        if v["id"] != id {
            return Err(err("ComponentIdentityUnverified"));
        }
        verify_state(&v["state"], &p.role)?;
        let started = v["state"]["StartedAt"]
            .as_str()
            .ok_or_else(|| err("ProbeObservationInvalid"))?
            .to_owned();
        Ok(Observed {
            id: id.into(),
            started,
            ports: v["ports"].clone(),
        })
    }
    fn port(&self, p: &ProbeSpec, target: u16, o: &Observed) -> Result<u16, ToolboxError> {
        let mapping = self
            .effective
            .ports
            .iter()
            .find(|m| {
                m.component_id == p.component_id
                    && m.container_port == target
                    && m.protocol == "tcp"
                    && m.host_ip == "127.0.0.1"
                    && m.host_port != 0
            })
            .ok_or_else(|| err("ProbeEndpointUnavailable"))?;
        let actual =
            if self.service.id == "rocketmq" && p.component_id == "console" && target == 8080 {
                8082
            } else {
                target
            };
        if !o.ports[format!("{actual}/tcp")]
            .as_array()
            .is_some_and(|list| {
                list.iter().any(|m| {
                    m["HostIp"] == "127.0.0.1"
                        && m["HostPort"]
                            .as_str()
                            .and_then(|value| value.parse::<u16>().ok())
                            == Some(mapping.host_port)
                })
            })
        {
            return Err(err("ProbeEndpointChanged"));
        }
        Ok(mapping.host_port)
    }
}
pub async fn evaluate(cx: &ProbeContext<'_>) -> Result<Vec<ProbeResult>, ToolboxError> {
    if cx.instance.target_id != cx.runner.target.target_id
        || cx.instance.daemon_id != cx.runner.target.daemon_id
        || cx.instance.service_id != cx.service.id
        || cx.effective.target_id != cx.instance.target_id
        || cx.effective.instance_id != cx.instance.instance_id
        || cx.effective.generation_id.is_empty()
    {
        return Err(err("TargetChanged"));
    }
    cx.runner.verify_identity().await?;
    let client = Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|_| err("ProbeClientUnavailable"))?;
    let mut results = vec![];
    for p in &cx.service.probes {
        let mut result = Err(err("ProbePolicyInvalid"));
        if (1..=5).contains(&p.attempts) && (1..=60).contains(&p.timeout_secs) && p.required {
            for n in 0..p.attempts {
                result = tokio::time::timeout(Duration::from_secs(p.timeout_secs.into()), async {
                    let before = cx.observe(p).await?;
                    let warn = if p.role == "job" && p.kind == "job-exit" {
                        None
                    } else if p.role == "daemon" {
                        protocol(cx, p, &before, &client).await?
                    } else {
                        return Err(err("UnknownProbe"));
                    };
                    let after = cx.observe(p).await?;
                    if after.id != before.id || after.started != before.started {
                        return Err(err("ComponentChanged"));
                    }
                    Ok(warn)
                })
                .await
                .unwrap_or_else(|_| Err(err("ProbeTimedOut")));
                if result.is_ok() {
                    break;
                }
                if n + 1 < p.attempts {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
        let (success, error_code, warning_code) = match result {
            Ok(w) => (true, None, w.map(str::to_owned)),
            Err(e) => (false, Some(e.code), None),
        };
        results.push(ProbeResult {
            probe_id: p.probe_id.clone(),
            component_id: p.component_id.clone(),
            success,
            error_code,
            warning_code,
            sampled_at: now(),
        });
    }
    cx.runner.verify_identity().await?;
    Ok(results)
}

async fn response(
    req: RequestBuilder,
) -> Result<(reqwest::header::HeaderMap, Vec<u8>), ToolboxError> {
    let mut r = req
        .send()
        .await
        .map_err(|_| err("ProbeEndpointUnavailable"))?;
    if matches!(r.status().as_u16(), 401 | 403) {
        return Err(err("ProbeAuthenticationFailed"));
    }
    if !r.status().is_success() {
        return Err(err("BackendUnavailable"));
    }
    let headers = r.headers().clone();
    let mut b = Vec::new();
    while let Some(chunk) = r.chunk().await.map_err(|_| err("ProbeResponseInvalid"))? {
        if b.len() + chunk.len() > 1024 * 1024 {
            return Err(err("ProbeResponseTooLarge"));
        }
        b.extend_from_slice(&chunk);
    }
    Ok((headers, b))
}
async fn body(req: RequestBuilder) -> Result<Value, ToolboxError> {
    serde_json::from_slice(&response(req).await?.1).map_err(|_| err("ProbeResponseInvalid"))
}
fn nonempty_array(v: &Value) -> bool {
    v.as_array().is_some_and(|a| !a.is_empty())
}
async fn login_cookie(req: RequestBuilder, code: i32) -> Result<String, ToolboxError> {
    let (headers, b) = response(req).await?;
    let v: Value = serde_json::from_slice(&b).map_err(|_| err("ProbeResponseInvalid"))?;
    if v["code"] != code {
        return Err(err("ProbeAuthenticationFailed"));
    }
    let cookies = headers
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|h| h.to_str().ok()?.split(';').next())
        .filter(|s| s.contains('='))
        .collect::<Vec<_>>()
        .join("; ");
    if cookies.is_empty() {
        return Err(err("ProbeAuthenticationFailed"));
    }
    Ok(cookies)
}
async fn redis(port: u16) -> Check {
    let mut c = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .map_err(|_| err("ProbeEndpointUnavailable"))?;
    c.write_all(b"*1\r\n$4\r\nPING\r\n")
        .await
        .map_err(|_| err("ProtocolUnavailable"))?;
    let mut reply = [0; 7];
    c.read_exact(&mut reply)
        .await
        .map_err(|_| err("ProtocolUnavailable"))?;
    checked(&reply == b"+PONG\r\n")
}
async fn database(cx: &ProbeContext<'_>, port: u16) -> Check {
    if cx.service.id == "mysql" {
        let opts = sqlx::mysql::MySqlConnectOptions::new()
            .host("127.0.0.1")
            .port(port)
            .username(cx.field("MYSQL_USER")?)
            .password(cx.field("MYSQL_PASSWORD")?)
            .database(cx.field("MYSQL_DATABASE")?);
        let mut c = sqlx::MySqlConnection::connect_with(&opts)
            .await
            .map_err(|_| err("ProbeAuthenticationFailed"))?;
        let v:String=sqlx::query_scalar("SELECT IF(@@log_bin=1 AND @@binlog_format='ROW' AND DATABASE() IS NOT NULL, 'ok', 'failed')").fetch_one(&mut c).await.map_err(|_|err("DatabaseQueryFailed"))?;
        let grants: Vec<String> = sqlx::query_scalar("SHOW GRANTS FOR CURRENT_USER")
            .fetch_all(&mut c)
            .await
            .map_err(|_| err("DatabaseQueryFailed"))?;
        let permitted = crate::bindings::mysql_grants_allow(
            &grants.join("\n"),
            "SELECT",
            Some(cx.field("MYSQL_DATABASE")?),
        );
        checked(v == "ok" && permitted)
    } else {
        let opts = sqlx::postgres::PgConnectOptions::new()
            .host("127.0.0.1")
            .port(port)
            .username(cx.field("PGSQL_USER")?)
            .password(cx.field("PGSQL_PASSWORD")?)
            .database(cx.field("PGSQL_DATABASE")?);
        let mut c = sqlx::PgConnection::connect_with(&opts)
            .await
            .map_err(|_| err("ProbeAuthenticationFailed"))?;
        checked(
            sqlx::query_scalar::<_, i32>("SELECT 1")
                .fetch_one(&mut c)
                .await
                .map_err(|_| err("DatabaseQueryFailed"))?
                == 1,
        )
    }
}

fn rss_document(bytes: &[u8]) -> bool {
    use quick_xml::events::Event;
    let mut reader = quick_xml::Reader::from_reader(bytes);
    let mut path = Vec::<Vec<u8>>::new();
    let mut root = false;
    let mut item = false;
    let mut channel_title = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                if path.len() > 128 {
                    return false;
                }
                if path.is_empty() {
                    if root || e.name().as_ref() != "rss" {
                        return false;
                    }
                    root = true;
                }
                path.push(e.name().as_ref().as_bytes().to_owned());
            }
            Ok(Event::End(_)) => {
                if path.pop().is_none() {
                    return false;
                }
            }
            Ok(Event::Text(t))
                if !t.as_ref().is_empty()
                    && path.iter().map(Vec::as_slice).eq([
                        b"rss".as_slice(),
                        b"channel",
                        b"item",
                        b"title",
                    ]) =>
            {
                item = true;
            }
            Ok(Event::CData(t))
                if !t.as_ref().is_empty()
                    && path.iter().map(Vec::as_slice).eq([
                        b"rss".as_slice(),
                        b"channel",
                        b"item",
                        b"title",
                    ]) =>
            {
                item = true;
            }
            Ok(Event::Text(v))
                if path
                    .iter()
                    .map(Vec::as_slice)
                    .eq([b"rss".as_slice(), b"channel", b"title"]) =>
            {
                channel_title = v.as_ref() == "Test 1"
            }
            Ok(Event::CData(v))
                if path
                    .iter()
                    .map(Vec::as_slice)
                    .eq([b"rss".as_slice(), b"channel", b"title"]) =>
            {
                channel_title = v.as_ref() == "Test 1"
            }
            Ok(Event::DocType(_)) | Err(_) => return false,
            Ok(Event::Eof) => return root && path.is_empty() && channel_title && item,
            _ => {}
        }
    }
}
fn engines_available(v: &Value) -> bool {
    let Some(failed) = v["unresponsive_engines"].as_array() else {
        return false;
    };
    v["results"].as_array().is_some_and(|results| {
        results.iter().any(|r| {
            let Some(engine) = r["engine"].as_str().filter(|e| !e.is_empty()) else {
                return false;
            };
            r["url"]
                .as_str()
                .is_some_and(|s| s.starts_with("https://") || s.starts_with("http://"))
                && !failed.iter().any(|e| e[0] == engine)
        })
    })
}
fn safe_segment(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        && !matches!(s, "." | "..")
}
async fn dependency(cx: &ProbeContext<'_>, p: &ProbeSpec) -> Result<(), ToolboxError> {
    let mut bindings = cx.bindings.iter().filter(|b| b.engine == "mysql");
    let b = bindings.next().ok_or_else(|| err("BindingRequired"))?;
    if bindings.next().is_some() || !valid_id(&b.container_id) {
        return Err(err("BindingIdentityUnverified"));
    }
    let raw=cx.runner.run_read(&["container","inspect","--format",r#"{"Id":{{json .Id}},"State":{"Running":{{json .State.Running}}},"NetworkSettings":{"Networks":{{json .NetworkSettings.Networks}},"Ports":{{json .NetworkSettings.Ports}}}}"#,&b.container_id]).await?;
    let info = serde_json::from_slice(&raw).map_err(|_| err("ProbeObservationInvalid"))?;
    crate::bindings::binding_topology(b, &cx.runner.target, &info)?;
    let uuid = b
        .server_fingerprint
        .strip_prefix(&format!("mysql:{}:", cx.runner.target.daemon_id))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| err("BindingIdentityUnverified"))?;
    let canal = cx.service.id == "canal";
    let db = if canal {
        ""
    } else {
        cx.field("MYSQL_DATABASE")?
    };
    if !canal && !b.allowed_databases.iter().any(|d| d == db) {
        return Err(err("BindingDatabaseOutOfScope"));
    }
    let input = json!({"database":db,"serverUuid":uuid,"port":b.container_port,"purpose":cx.service.id,
        "username":cx.field(if canal{"CANAL_MYSQL_USER"}else{"MYSQL_USERNAME"})?,
        "password":cx.field(if canal{"CANAL_MYSQL_PASSWORD"}else{"MYSQL_PASSWORD"})?});
    probe_clients::java(cx, p, &b.container_id, "mysql", input).await
}
async fn protocol(cx: &ProbeContext<'_>, p: &ProbeSpec, o: &Observed, http: &Client) -> Check {
    let port = |target| cx.port(p, target, o);
    let base = |target| port(target).map(|n| format!("http://127.0.0.1:{n}"));
    let java = |module, input| probe_clients::java(cx, p, &o.id, module, input);
    match (
        cx.service.id.as_str(),
        p.component_id.as_str(),
        p.kind.as_str(),
    ) {
        ("pgsql", "postgres", "sql") => database(cx, port(5432)?).await,
        ("mysql", "mysql", "sql") => database(cx, port(3306)?).await,
        ("redis", "redis", "redis") => redis(port(6379)?).await,
        ("minio", "minio", "s3") => {
            response(http.get(format!("{}/minio/health/ready", base(9000)?))).await?;
            java("s3",json!({"port":9000,"username":cx.field("MINIO_ROOT_USER")?,"password":cx.field("MINIO_ROOT_PASSWORD")?})).await?;
            Ok(None)
        }
        ("kafka", "kafka", "kafka") => {
            java("kafka", json!({"port":29092})).await?;
            Ok(None)
        }
        ("kafka", "kafka-ui", "http") => {
            let data = body(http.get(format!(
                "{}/api/clusters/local-cluster/brokers",
                base(8080)?
            )))
            .await?;
            checked(data.as_array().is_some_and(|v| {
                v.iter().any(|b| {
                    b["id"].as_i64().is_some() && b["host"].as_str().is_some_and(|s| !s.is_empty())
                })
            }))
        }
        ("canal", "canal-server", "canal") => {
            java("canal", json!({"port":11111})).await?;
            dependency(cx, p).await?;
            Ok(None)
        }
        ("rabbitmq", "rabbitmq", "amqp") => {
            let (u, pw) = (
                cx.field("RABBITMQ_DEFAULT_USER")?,
                cx.field("RABBITMQ_DEFAULT_PASS")?,
            );
            java("amqp", json!({"port":5672,"username":u,"password":pw})).await?;
            checked(
                body(
                    http.get(format!("{}/api/whoami", base(15672)?))
                        .basic_auth(u, Some(pw)),
                )
                .await?["name"]
                    == u,
            )
        }
        ("rocketmq", "namesrv", "rocket-route") => {
            java("rocket-route", json!({"port":9876})).await?;
            Ok(None)
        }
        ("rocketmq", "broker", "rocketmq") => {
            java("rocket-proxy", json!({"port":8081})).await?;
            Ok(None)
        }
        ("rocketmq", "console", "http") => {
            let v = body(http.get(format!("{}/cluster/list.query", base(8080)?))).await?;
            checked(
                v["status"] == 0
                    && v["data"]["clusterInfo"]["brokerAddrTable"]
                        .as_object()
                        .is_some_and(|o| {
                            o.values().any(|b| {
                                b["brokerAddrs"].as_object().is_some_and(|m| !m.is_empty())
                            })
                        }),
            )
        }
        ("temporal", "temporal", "temporal") => {
            let ns = cx.field("TEMPORAL_NAMESPACE")?;
            if !safe_segment(ns) {
                return Err(err("InvalidNamespace"));
            }
            probe_clients::temporal(cx, &o.id, ns).await?;
            Ok(None)
        }
        ("temporal", "temporal-ui", "http") => {
            let ns = cx.field("TEMPORAL_NAMESPACE")?;
            if !safe_segment(ns) {
                return Err(err("InvalidNamespace"));
            }
            let v = body(http.get(format!("{}/api/v1/namespaces/{ns}", base(8080)?))).await?;
            checked(
                v["namespaceInfo"]["name"] == ns
                    && v["namespaceInfo"]["id"]
                        .as_str()
                        .is_some_and(|id| !id.is_empty()),
            )
        }
        ("xxjob", "xxl-job-admin", "xxjob") => {
            dependency(cx, p).await?;
            let url = format!("{}/xxl-job-admin", base(8080)?);
            let cookie = login_cookie(
                http.post(format!("{url}/auth/doLogin")).form(&[
                    ("userName", cx.provided("XXL_ADMIN_USERNAME", "admin")),
                    ("password", cx.provided("XXL_ADMIN_PASSWORD", "123456")),
                ]),
                200,
            )
            .await?;
            let v = body(
                http.post(format!("{url}/jobgroup/pageList"))
                    .header("Cookie", cookie)
                    .form(&[("offset", "0"), ("pagesize", "1")]),
            )
            .await?;
            checked(
                v["code"] == 200
                    && nonempty_array(&v["data"]["data"])
                    && v["data"]["total"].as_u64().is_some_and(|i| i > 0),
            )
        }
        ("nacos", "nacos", "nacos") => {
            let (u, pw) = (
                cx.provided("NACOS_AUTH_USERNAME", "nacos"),
                cx.provided("NACOS_AUTH_PASSWORD", "nacos"),
            );
            let v = body(
                http.post(format!("{}/nacos/v3/auth/user/login", base(8848)?))
                    .form(&[("username", u), ("password", pw)]),
            )
            .await?;
            let token = v["accessToken"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| err("ProbeAuthenticationFailed"))?;
            let console = body(
                http.get(format!("{}/v3/console/core/namespace/list", base(8080)?))
                    .bearer_auth(token),
            )
            .await?;
            if console["code"] != 0 || !nonempty_array(&console["data"]) {
                return Err(err("BackendUnavailable"));
            }
            let configs = body(
                http.get(format!("{}/v3/console/cs/config/list", base(8080)?))
                    .query(&[("pageNo", "1"), ("pageSize", "1")])
                    .bearer_auth(token),
            )
            .await?;
            if configs["code"] != 0
                || !configs["data"]["pageItems"].is_array()
                || !configs["data"]["totalCount"].is_u64()
            {
                return Err(err("ConfigurationBackendUnavailable"));
            }
            java("nacos", json!({"port":8848,"username":u,"password":pw})).await?;
            Ok(None)
        }
        ("sentinel", "sentinel", "http") => {
            let url = base(
                cx.provided("SENTINEL_DASHBOARD_PORT", "8858")
                    .parse()
                    .map_err(|_| err("ProbeEndpointUnavailable"))?,
            )?;
            let cookie = login_cookie(
                http.post(format!("{url}/auth/login")).form(&[
                    ("username", cx.provided("SENTINEL_USERNAME", "sentinel")),
                    ("password", cx.provided("SENTINEL_PASSWORD", "sentinel")),
                ]),
                0,
            )
            .await?;
            let v = body(
                http.get(format!("{url}/app/names.json"))
                    .header("Cookie", cookie),
            )
            .await?;
            if v["code"] != 0 {
                return Err(err("BackendUnavailable"));
            }
            let names = v["data"]
                .as_array()
                .ok_or_else(|| err("ProbeResponseInvalid"))?;
            Ok(
                if names
                    .iter()
                    .any(|n| n.as_str().is_some_and(|n| n != "sentinel-dashboard"))
                {
                    None
                } else {
                    Some("WaitingForApplicationClient")
                },
            )
        }
        ("seata", "seata-server", "seata") => {
            dependency(cx, p).await?;
            let b = base(7091)?;
            let (h,_)=response(http.post(format!("{b}/api/v1/auth/login")).json(&json!({"username":cx.provided("SEATA_CONSOLE_USERNAME","seata"),"password":cx.provided("SEATA_CONSOLE_PASSWORD","seata")}))).await?;
            if !h
                .get("Authorization")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|s| s.starts_with("Bearer ") && s.len() > 7)
            {
                return Err(err("ProbeAuthenticationFailed"));
            }
            java("seata", json!({"port":8091})).await?;
            Ok(None)
        }
        ("elastic", "elasticsearch", "elasticsearch") => {
            let b = base(9200)?;
            let pw = cx.field("ES_LOCAL_PASSWORD")?;
            let health = body(
                http.get(format!("{b}/_cluster/health"))
                    .basic_auth("elastic", Some(pw)),
            )
            .await?;
            if !matches!(health["status"].as_str(), Some("green" | "yellow"))
                || health["timed_out"] != false
            {
                return Err(err("ClusterUnavailable"));
            }
            let tokens = body(
                http.post(format!("{b}/_analyze"))
                    .basic_auth("elastic", Some(pw))
                    .json(&json!({"analyzer":"ik_smart","text":"中文分词"})),
            )
            .await?;
            checked(tokens["tokens"].as_array().is_some_and(|a| {
                a.iter()
                    .any(|t| t["token"].as_str().is_some_and(|s| !s.is_empty()))
            }))
        }
        ("elastic", "kibana", "http") => {
            let v = body(
                http.get(format!("{}/api/status", base(5601)?))
                    .basic_auth("elastic", Some(cx.field("ES_LOCAL_PASSWORD")?)),
            )
            .await?;
            checked(
                v["status"]["overall"]["level"] == "available"
                    && v["status"]["core"]["elasticsearch"]["level"] == "available",
            )
        }
        ("elastic", "logstash", "http") => {
            let v = body(http.get(format!("{}/_node/stats/pipelines", base(9600)?))).await?;
            checked(
                v["pipelines"]["main"]["events"]["in"].as_u64().is_some()
                    && nonempty_array(&v["pipelines"]["main"]["plugins"]["outputs"]),
            )
        }
        ("monitoring", "prometheus", "http") => {
            let b = base(9090)?;
            if !std::str::from_utf8(&response(http.get(format!("{b}/-/ready"))).await?.1)
                .unwrap_or("")
                .contains("Prometheus Server is Ready")
            {
                return Err(err("BackendUnavailable"));
            }
            let v = body(http.get(format!("{b}/api/v1/targets?state=active"))).await?;
            checked(
                v["status"] == "success"
                    && v["data"]["activeTargets"].as_array().is_some_and(|a| {
                        a.iter().any(|t| t["labels"]["job"] == "prometheus")
                            && a.iter()
                                .all(|t| t["health"] == "up" && t["lastError"] == "")
                    }),
            )
        }
        ("monitoring", "grafana", "monitoring") => {
            let b = base(3000)?;
            let get = |path: &str| {
                http.get(format!("{b}{path}")).basic_auth(
                    "admin",
                    Some(cx.provided("GRAFANA_ADMIN_PASSWORD", "admin")),
                )
            };
            let ds = body(get("/api/datasources/name/Prometheus")).await?;
            let uid = ds["uid"]
                .as_str()
                .filter(|u| safe_segment(u))
                .ok_or_else(|| err("DatasourceUnavailable"))?;
            if ds["type"] != "prometheus"
                || body(get(&format!("/api/datasources/uid/{uid}/health"))).await?["status"] != "OK"
            {
                return Err(err("DatasourceUnavailable"));
            }
            let q = body(
                get(&format!("/api/datasources/proxy/uid/{uid}/api/v1/query"))
                    .query(&[("query", "up{job=\"prometheus\"}")]),
            )
            .await?;
            checked(
                q["status"] == "success"
                    && q["data"]["result"]
                        .as_array()
                        .is_some_and(|a| a.iter().any(|r| r["value"][1] == "1")),
            )
        }
        ("rsshub", "rsshub", "rss") => {
            let b = base(1200)?;
            response(http.get(format!("{b}/healthz"))).await?;
            checked(rss_document(
                &response(http.get(format!("{b}/test/1"))).await?.1,
            ))
        }
        ("searxng", "searxng", "search") => {
            let v = body(
                http.get(format!("{}/search", base(8080)?))
                    .query(&[("format", "json"), ("q", "Docker documentation")]),
            )
            .await?;
            if engines_available(&v) {
                Ok(None)
            } else {
                Err(err("SearchUpstreamUnavailable"))
            }
        }
        _ => Err(err("UnknownProbe")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::load_embedded_catalog;
    #[test]
    fn every_packaged_service_requires_all_components_and_probes() {
        let services = load_embedded_catalog().unwrap().services;
        assert_eq!(services.len(), 17);
        for mut s in services {
            assert_eq!(
                requires_tool(&s),
                matches!(
                    s.id.as_str(),
                    "minio"
                        | "rabbitmq"
                        | "kafka"
                        | "canal"
                        | "rocketmq"
                        | "seata"
                        | "xxjob"
                        | "nacos"
                )
            );
            let results: Vec<_> = s
                .probes
                .iter()
                .map(|p| ProbeResult {
                    probe_id: p.probe_id.clone(),
                    component_id: p.component_id.clone(),
                    sampled_at: 1,
                    success: true,
                    warning_code: None,
                    error_code: None,
                })
                .collect();
            assert!(ready(&s, &results), "all checks succeeded for {}", s.id);
            for i in 0..results.len() {
                let mut failed = results.clone();
                failed[i].success = false;
                failed[i].error_code = Some("ProbeFailed".into());
                assert!(!ready(&s, &failed), "{}", s.id);
                failed.remove(i);
                assert!(!ready(&s, &failed), "{}", s.id);
            }
            let saved_kind = s.probes[0].kind.clone();
            s.probes[0].kind = "unknown-future-protocol".into();
            assert!(!ready(&s, &results), "unsupported protocol {}", s.id);
            s.probes[0].kind = saved_kind;
            let missing = s.probes[0].component_id.clone();
            s.probes.retain(|p| p.component_id != missing);
            assert!(!ready(&s, &results), "missing required component {missing}");
        }
    }

    #[test]
    fn deterministic_feed_rejects_wrong_route_and_html() {
        assert!(!rss_document(
            b"<html><head><title>login</title></head></html>"
        ));
        assert!(!rss_document(b"<rss><channel><title>unrelated feed</title><item><title>Title0</title></item></channel></rss>"));
        assert!(rss_document(b"<rss><channel><title><![CDATA[Test 1]]></title><item><title>Title0</title></item></channel></rss>"));
        assert!(!rss_document(
            b"<rss><channel><title>Test 1</title><item><title>Title0</title></item>"
        ));
    }
    #[test]
    fn job_requires_successful_exit_and_daemon_requires_running_health() {
        let mut job = json!({"Status":"exited","ExitCode":0,"Error":"","OOMKilled":false});
        assert!(verify_state(&job, "job").is_ok());
        job["ExitCode"] = 1.into();
        assert_eq!(verify_state(&job, "job").unwrap_err().code, "JobIncomplete");
        job["Status"] = "running".into();
        job["ExitCode"] = 0.into();
        assert!(verify_state(&job, "job").is_err());
        let mut d =
            json!({"Status":"running","Running":true,"Paused":false,"Error":"","OOMKilled":false});
        assert!(verify_state(&d, "daemon").is_ok());
        d["Health"] = json!({"Status":"unhealthy"});
        assert!(verify_state(&d, "daemon").is_err());
        assert!(verify_state(&d, "mystery").is_err());
    }
    #[test]
    fn search_requires_a_result_from_a_responsive_engine() {
        let mut v = json!({"results":[{"url":"https://example.org","engine":"local-engine"}],"unresponsive_engines":[]});
        assert!(engines_available(&v));
        v["unresponsive_engines"] = json!([["local-engine", "timeout"]]);
        assert!(!engines_available(&v));
        assert!(!engines_available(
            &json!({"results":[],"unresponsive_engines":[]})
        ));
    }
    #[tokio::test]
    async fn redis_accepts_only_a_complete_resp_pong() {
        for (reply, expected) in [
            ("+PONG\r\n", true),
            ("-NOAUTH required\r\n", false),
            ("+OK\r\n", false),
        ] {
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
                .await
                .unwrap();
            let port = listener.local_addr().unwrap().port();
            let server = tokio::spawn(async move {
                let (mut peer, _) = listener.accept().await.unwrap();
                let mut ping = [0u8; 14];
                peer.read_exact(&mut ping).await.unwrap();
                assert_eq!(&ping, b"*1\r\n$4\r\nPING\r\n");
                peer.write_all(reply.as_bytes()).await.unwrap();
            });
            assert_eq!(redis(port).await.is_ok(), expected);
            server.await.unwrap();
        }
    }
    // Run the same daemon dispatch and bounded HTTP readers used by evaluate(). Each response
    // is supplied over a real ephemeral loopback socket. No container or service image is used.
    async fn http_fixture(
        service_id: &str,
        component: &str,
        port: u16,
        responses: Vec<(&str, u16, &str, &str)>,
    ) -> Check {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let host_port = listener.local_addr().unwrap().port();
        let script: Vec<_> = responses
            .into_iter()
            .map(|(path, status, headers, body)| {
                (path.to_owned(), status, headers.to_owned(), body.to_owned())
            })
            .collect();
        let server = tokio::spawn(async move {
            for (path, status, headers, body) in script {
                let (mut peer, _) = listener.accept().await.unwrap();
                let mut raw = Vec::new();
                while !raw.ends_with(b"\r\n\r\n") && raw.len() < 16384 {
                    let mut b = [0];
                    if peer.read_exact(&mut b).await.is_err() {
                        break;
                    }
                    raw.push(b[0]);
                }
                let first = std::str::from_utf8(&raw).unwrap().lines().next().unwrap();
                assert!(
                    first.split_whitespace().nth(1).unwrap().starts_with(&path),
                    "unexpected API: {first}"
                );
                let reply = format!(
                    "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n{headers}\r\n{body}",
                    body.len()
                );
                peer.write_all(reply.as_bytes()).await.unwrap();
            }
        });
        let service = load_embedded_catalog()
            .unwrap()
            .services
            .into_iter()
            .find(|s| s.id == service_id)
            .unwrap();
        let p = service
            .probes
            .iter()
            .find(|p| p.component_id == component)
            .unwrap();
        let mut effective = EffectiveConfig {
            ports: vec![crate::contracts::PortMapping {
                component_id: component.into(),
                container_port: port,
                host_port,
                host_ip: "127.0.0.1".into(),
                protocol: "tcp".into(),
            }],
            endpoints: vec![],
            target_id: "t".into(),
            instance_id: "i".into(),
            generation_id: "g".into(),
            secret_refs: vec![],
            config_digest: String::new(),
            image_artifacts: vec![],
        };
        if service_id == "nacos" {
            effective.ports.push(crate::contracts::PortMapping {
                component_id: component.into(),
                container_port: 8080,
                host_port,
                host_ip: "127.0.0.1".into(),
                protocol: "tcp".into(),
            });
        }
        let target = crate::contracts::DockerTarget {
            target_id: "t".into(),
            context_name: "unused".into(),
            cli_path: "unused".into(),
            endpoint_kind: "unix".into(),
            daemon_id: "d".into(),
            os_type: "linux".into(),
            architecture: "arm64".into(),
        };
        let instance = InstanceIdentity {
            target_id: "t".into(),
            daemon_id: "d".into(),
            instance_id: "i".into(),
            workspace_id: "w".into(),
            service_id: service_id.into(),
            project_name: "p".into(),
        };
        let secrets = SecretValues(std::collections::BTreeMap::from([
            ("ES_LOCAL_PASSWORD".into(), "test-password".into()),
            ("TEMPORAL_NAMESPACE".into(), "default".into()),
        ]));
        let runner = DockerRunner { target };
        let ctx = ProbeContext {
            control_root: std::path::Path::new(""),
            runner: &runner,
            service: &service,
            instance: &instance,
            effective: &effective,
            secrets: &secrets,
            bindings: &[],
            tool_image: None,
            resources_root: std::path::Path::new(""),
        };
        let actual = if service_id == "rocketmq" && component == "console" {
            8082
        } else {
            port
        };
        let mut observed = Observed {
            id: String::new(),
            started: String::new(),
            ports: json!({format!("{actual}/tcp"):[{"HostIp":"127.0.0.1","HostPort":host_port.to_string()}]}),
        };
        if service_id == "nacos" {
            observed.ports["8080/tcp"] =
                json!([{"HostIp":"127.0.0.1","HostPort":host_port.to_string()}]);
        }
        let http = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let result = protocol(&ctx, p, &observed, &http).await;
        tokio::time::timeout(Duration::from_secs(3), server)
            .await
            .expect("probe skipped its expected API")
            .unwrap();
        result
    }

    #[tokio::test]
    async fn http_backends_require_protocol_evidence_even_with_200() {
        for (service, component, port, path, ok, broken) in [
            (
                "kafka",
                "kafka-ui",
                8080,
                "/api/clusters/local-cluster/brokers",
                r#"[{"id":1,"host":"kafka"}]"#,
                "[]",
            ),
            (
                "rocketmq",
                "console",
                8080,
                "/cluster/list.query",
                r#"{"status":0,"data":{"clusterInfo":{"brokerAddrTable":{"broker-a":{"brokerAddrs":{"0":"broker:10911"}}}}}}"#,
                r#"{"status":1}"#,
            ),
            (
                "temporal",
                "temporal-ui",
                8080,
                "/api/v1/namespaces/default",
                r#"{"namespaceInfo":{"name":"default","id":"ns"}}"#,
                r#"{"namespaces":[]}"#,
            ),
            (
                "elastic",
                "kibana",
                5601,
                "/api/status",
                r#"{"status":{"overall":{"level":"available"},"core":{"elasticsearch":{"level":"available"}}}}"#,
                r#"{"status":{"overall":{"level":"available"},"core":{"elasticsearch":{"level":"unavailable"}}}}"#,
            ),
            (
                "elastic",
                "logstash",
                9600,
                "/_node/stats/pipelines",
                r#"{"pipelines":{"main":{"events":{"in":0},"plugins":{"outputs":[{"id":"es"}]}}}}"#,
                r#"{"pipelines":{}}"#,
            ),
            (
                "searxng",
                "searxng",
                8080,
                "/search?",
                r#"{"results":[{"engine":"x","url":"https://example.org"}],"unresponsive_engines":[]}"#,
                r#"{"results":[],"unresponsive_engines":[["x","timeout"]]}"#,
            ),
        ] {
            assert!(
                http_fixture(service, component, port, vec![(path, 200, "", ok)])
                    .await
                    .is_ok(),
                "{component}"
            );
            assert!(
                http_fixture(service, component, port, vec![(path, 200, "", broken)])
                    .await
                    .is_err(),
                "{component}"
            );
            assert!(
                http_fixture(
                    service,
                    component,
                    port,
                    vec![(path, 200, "", "<html>login</html>")]
                )
                .await
                .is_err(),
                "{component}"
            );
        }
    }
    #[tokio::test]
    async fn authenticated_endpoints_wait_for_clients_and_reject_denied_credentials() {
        let cookie = "Set-Cookie: sid=example; HttpOnly\r\n";
        let result = http_fixture(
            "sentinel",
            "sentinel",
            8858,
            vec![
                ("/auth/login", 200, cookie, r#"{"code":0}"#),
                (
                    "/app/names.json",
                    200,
                    "",
                    r#"{"code":0,"data":["sentinel-dashboard"]}"#,
                ),
            ],
        )
        .await
        .unwrap();
        assert_eq!(result, Some("WaitingForApplicationClient"));
        assert!(
            http_fixture(
                "elastic",
                "elasticsearch",
                9200,
                vec![("/_cluster/health", 401, "", "")]
            )
            .await
            .is_err()
        );
        assert!(
            http_fixture(
                "elastic",
                "elasticsearch",
                9200,
                vec![
                    (
                        "/_cluster/health",
                        200,
                        "",
                        r#"{"status":"yellow","timed_out":false}"#
                    ),
                    ("/_analyze", 200, "", r#"{"tokens":[{"token":"中文"}]}"#)
                ]
            )
            .await
            .is_ok()
        );
    }
    #[tokio::test]
    async fn nacos_denied_login_is_never_readiness_success() {
        for status in [401, 403] {
            let result = http_fixture(
                "nacos",
                "nacos",
                8848,
                vec![("/nacos/v3/auth/user/login", status, "", "User not found!")],
            )
            .await;
            assert_eq!(result.unwrap_err().code, "ProbeAuthenticationFailed");
        }
    }
    #[tokio::test]
    async fn nacos_valid_login_cannot_mask_denied_console_access() {
        for status in [401, 403] {
            let result = http_fixture(
                "nacos",
                "nacos",
                8848,
                vec![
                    (
                        "/nacos/v3/auth/user/login",
                        200,
                        "",
                        r#"{"accessToken":"fixture-token"}"#,
                    ),
                    (
                        "/v3/console/core/namespace/list",
                        status,
                        "",
                        r#"{"code":403,"data":null}"#,
                    ),
                ],
            )
            .await;
            assert_eq!(result.unwrap_err().code, "ProbeAuthenticationFailed");
        }
    }
}
