use crate::contracts::*;
use crate::docker_cli::DockerRunner;
use crate::image_store::{ImageContext, ensure_service_image};
use crate::workspace::{atomic_write, error, private_dir, random_id};
use serde_json::Value;
use sqlx::Connection;
use std::time::Duration;
use zeroize::Zeroizing;
#[derive(Debug, Clone, PartialEq)]
pub enum AuthRoute {
    Host,
    Network,
}
pub struct Credentials {
    pub username: String,
    pub password: Zeroizing<String>,
}
impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Credentials([REDACTED])")
    }
}
pub fn server_fingerprint(engine: &str, daemon: &str, id: &str) -> String {
    format!("{engine}:{daemon}:{}", id.trim())
}
pub struct VerifiedBinding {
    pub(crate) binding: DatabaseBinding,
}
impl VerifiedBinding {
    pub fn view(&self) -> &DatabaseBinding {
        &self.binding
    }
}
pub fn valid_database_name(name: &str) -> bool {
    let system = [
        "mysql",
        "information_schema",
        "performance_schema",
        "sys",
        "postgres",
        "template0",
        "template1",
    ];
    !name.is_empty()
        && name.len() <= 63
        && !system.contains(&name.to_ascii_lowercase().as_str())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && name.as_bytes()[0].is_ascii_alphabetic()
}
pub fn binding_topology(
    binding: &DatabaseBinding,
    target: &DockerTarget,
    container: &Value,
) -> Result<AuthRoute, ToolboxError> {
    if binding.container_id.is_empty()
        || binding.target_id != target.target_id
        || binding.daemon_id != target.daemon_id
        || !matches!(binding.engine.as_str(), "mysql" | "pgsql")
    {
        return Err(error("UnsupportedDependencyTopology"));
    }
    if container["Id"] != binding.container_id || container["State"]["Running"] != true {
        return Err(error("DependencyUnavailable"));
    }
    let network = &container["NetworkSettings"]["Networks"][&binding.network_name];
    if network["NetworkID"].as_str().is_none_or(str::is_empty)
        || !network["Aliases"]
            .as_array()
            .is_some_and(|a| a.iter().any(|v| v == &binding.network_alias))
    {
        return Err(error("UnsupportedDependencyTopology"));
    }
    if let Some(endpoint) = &binding.host_endpoint {
        if !matches!(endpoint.host.as_str(), "127.0.0.1" | "localhost")
            || endpoint.protocol != "tcp"
        {
            return Err(error("UnsupportedDependencyTopology"));
        }
        let key = format!("{}/tcp", binding.container_port);
        if !container["NetworkSettings"]["Ports"][key]
            .as_array()
            .is_some_and(|a| {
                a.iter().any(|p| {
                    p["HostPort"].as_str() == Some(endpoint.port.to_string().as_str())
                        && p["HostIp"]
                            .as_str()
                            .is_some_and(|s| matches!(s, "127.0.0.1" | "0.0.0.0" | ""))
                })
            })
        {
            return Err(error("UnsupportedDependencyTopology"));
        }
        Ok(AuthRoute::Host)
    } else {
        Ok(AuthRoute::Network)
    }
}
pub struct AuthorizedBinding {
    pub(crate) binding: DatabaseBinding,
    pub(crate) grant: InitializationGrant,
}
impl std::ops::Deref for AuthorizedBinding {
    type Target = DatabaseBinding;
    fn deref(&self) -> &Self::Target {
        &self.binding
    }
}
pub fn authorize_binding(
    verified: &VerifiedBinding,
    grant: &InitializationGrant,
) -> Result<AuthorizedBinding, ToolboxError> {
    let binding = &verified.binding;
    if binding.server_fingerprint.is_empty()
        || grant.target_id != binding.target_id
        || grant.server_fingerprint != binding.server_fingerprint
    {
        return Err(error("ServerIdentityMismatch"));
    }
    if grant.allowed_databases.is_empty()
        || grant
            .allowed_databases
            .iter()
            .any(|db| !valid_database_name(db) || !binding.allowed_databases.contains(db))
        || grant.operations != ["InitializeNamedResources"]
    {
        return Err(error("InitializationGrantOutOfScope"));
    }
    if grant.allowed_accounts.iter().any(|account| {
        account.is_empty()
            || account.len() > 32
            || !account
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    }) {
        return Err(error("InitializationGrantOutOfScope"));
    }
    let mut result = binding.clone();
    result.grant = "InitializeNamedResources".into();
    result.allowed_databases = grant.allowed_databases.clone();
    Ok(AuthorizedBinding {
        binding: result,
        grant: grant.clone(),
    })
}
pub(crate) enum SqlConnection {
    Mysql(sqlx::MySqlConnection),
    Postgres(sqlx::PgConnection),
}
impl SqlConnection {
    pub(crate) async fn connect(
        binding: &DatabaseBinding,
        credentials: &Credentials,
        database: Option<&str>,
    ) -> Result<Self, ToolboxError> {
        let endpoint = binding
            .host_endpoint
            .as_ref()
            .ok_or_else(|| error("NetworkClientRequired"))?;
        let connection = async {
            if binding.engine == "mysql" {
                let mut options = sqlx::mysql::MySqlConnectOptions::new()
                    .host(&endpoint.host)
                    .port(endpoint.port)
                    .username(&credentials.username)
                    .password(&credentials.password);
                if let Some(database) = database {
                    options = options.database(database);
                }
                sqlx::MySqlConnection::connect_with(&options)
                    .await
                    .map(Self::Mysql)
            } else {
                let options = sqlx::postgres::PgConnectOptions::new()
                    .host(&endpoint.host)
                    .port(endpoint.port)
                    .username(&credentials.username)
                    .password(&credentials.password)
                    .database(database.unwrap_or("postgres"));
                sqlx::PgConnection::connect_with(&options)
                    .await
                    .map(Self::Postgres)
            }
        };
        tokio::time::timeout(Duration::from_secs(5), connection)
            .await
            .map_err(|_| error("TimedOut"))?
            .map_err(|_| error("CredentialMismatch"))
    }
    pub(crate) async fn scalar(&mut self, sql: &'static str) -> Result<String, ToolboxError> {
        tokio::time::timeout(Duration::from_secs(5), async {
            match self {
                Self::Mysql(c) => sqlx::query_scalar::<_, String>(sql).fetch_one(c).await,
                Self::Postgres(c) => sqlx::query_scalar::<_, String>(sql).fetch_one(c).await,
            }
        })
        .await
        .map_err(|_| error("TimedOut"))?
        .map_err(|_| error("DatabaseQueryFailed"))
    }
}
pub(crate) async fn network_query(
    runner: &DockerRunner,
    binding: &DatabaseBinding,
    credentials: &Credentials,
    image: &ImageArtifactRef,
    context: &ImageContext,
    sql: &str,
) -> Result<String, ToolboxError> {
    let image = ensure_service_image(runner, image, context).await?;
    let temporary = context
        .cache_root
        .join(format!("sql-client-{}", random_id()?));
    private_dir(&temporary)?;
    let nonce = random_id()?;
    let mut cleanup = SqlClientCleanup {
        runner: DockerRunner {
            target: runner.target.clone(),
        },
        nonce: nonce.clone(),
        directory: temporary.clone(),
        started: false,
    };
    let env = temporary.join("client.env");
    if credentials.password.contains(['\n', '\r', '\0']) {
        return Err(error("UnsupportedCredentialValue"));
    }
    let values = if binding.engine == "pgsql" {
        format!(
            "PGPASSWORD={}\nPGCONNECT_TIMEOUT=5\n",
            credentials.password.as_str()
        )
    } else {
        format!("MYSQL_PWD={}\n", credentials.password.as_str())
    };
    atomic_write(&env, values.as_bytes())?;
    let query_file = temporary.join("query.sql");
    atomic_write(&query_file, sql.as_bytes())?;
    let name = format!("ca-sql-probe-{}", random_id()?);
    let mut args = vec![
        "run".into(),
        "--rm".into(),
        "--name".into(),
        name.into(),
        "--network".into(),
        format!("container:{}", binding.container_id).into(),
        "--env-file".into(),
        env.into_os_string(),
        "--mount".into(),
        format!(
            "type=bind,source={},target=/run/codeark/query.sql,readonly",
            query_file.display()
        )
        .into(),
        "--label".into(),
        "io.codeark.role=sql-probe".into(),
        "--label".into(),
        format!("io.codeark.sql-owner={nonce}").into(),
        image.runtime_reference.into(),
    ];
    if binding.engine == "pgsql" {
        args.extend([
            "psql".into(),
            "--no-psqlrc".into(),
            "--tuples-only".into(),
            "--no-align".into(),
            "--set=ON_ERROR_STOP=1".into(),
            "--host=127.0.0.1".into(),
            format!("--port={}", binding.container_port).into(),
            format!("--username={}", credentials.username).into(),
            "--dbname=postgres".into(),
            "--file=/run/codeark/query.sql".into(),
        ]);
    } else {
        args.extend([
            "mysql".into(),
            "--batch".into(),
            "--raw".into(),
            "--skip-column-names".into(),
            "--connect-timeout=5".into(),
            "--host=127.0.0.1".into(),
            format!("--port={}", binding.container_port).into(),
            format!("--user={}", credentials.username).into(),
            "--execute=source /run/codeark/query.sql".into(),
        ]);
    }
    runner.verify_identity().await?;
    cleanup.started = true;
    let result = runner.run_args(&args, Duration::from_secs(30)).await;
    cleanup.finish().await?;
    let result = result.map_err(|_| error("DatabaseQueryFailed"))?;
    String::from_utf8(result)
        .map(|s| s.trim().to_owned())
        .map_err(|_| error("DatabaseOutputInvalid"))
}
pub async fn verify_binding(
    runner: &DockerRunner,
    binding: &DatabaseBinding,
    credentials: &Credentials,
    client: Option<&ImageArtifactRef>,
    context: &ImageContext,
) -> Result<VerifiedBinding, ToolboxError> {
    runner.verify_identity().await?;
    let inspected = runner
        .run_read(&["container", "inspect", &binding.container_id])
        .await?;
    let inspected: Value =
        serde_json::from_slice(&inspected).map_err(|_| error("DockerOutputInvalid"))?;
    let route = binding_topology(binding, &runner.target, &inspected[0])?;
    let fingerprint_sql = if binding.engine == "mysql" {
        "SELECT @@server_uuid"
    } else {
        "SELECT system_identifier::text FROM pg_control_system()"
    };
    let fingerprint = match route {
        AuthRoute::Host => {
            let mut sql = SqlConnection::connect(binding, credentials, None).await?;
            let auth = if binding.engine == "mysql" {
                "SELECT CAST(1 AS CHAR)"
            } else {
                "SELECT '1'::text"
            };
            if sql.scalar(auth).await? != "1" {
                return Err(error("CredentialMismatch"));
            }
            sql.scalar(fingerprint_sql).await.ok()
        }
        AuthRoute::Network => {
            let client = client.ok_or_else(|| error("NetworkClientArtifactRequired"))?;
            let auth = if binding.engine == "mysql" {
                "SELECT CAST(1 AS CHAR)"
            } else {
                "SELECT '1'::text"
            };
            if network_query(runner, binding, credentials, client, context, auth).await? != "1" {
                return Err(error("CredentialMismatch"));
            }
            network_query(
                runner,
                binding,
                credentials,
                client,
                context,
                fingerprint_sql,
            )
            .await
            .ok()
        }
    };
    let mut verified = binding.clone();
    verified.server_fingerprint = fingerprint
        .map(|id| server_fingerprint(&binding.engine, &runner.target.daemon_id, &id))
        .unwrap_or_default();
    if !binding.server_fingerprint.is_empty()
        && binding.server_fingerprint != verified.server_fingerprint
    {
        return Err(error("ServerIdentityMismatch"));
    }
    verified.lifecycle_owner = "ExternalConnected".into();
    verified.grant = "ReadOnlyConnect".into();
    Ok(VerifiedBinding { binding: verified })
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn binding() -> DatabaseBinding {
        DatabaseBinding {
            binding_id: "b".into(),
            target_id: "t".into(),
            daemon_id: "d".into(),
            engine: "pgsql".into(),
            lifecycle_owner: "ExternalConnected".into(),
            container_id: "abcdef0123456789".into(),
            network_name: "database-net".into(),
            network_alias: "postgres".into(),
            container_port: 5432,
            host_endpoint: None,
            credential_ref: SecretRef {
                key: "db-login".into(),
            },
            allowed_databases: vec!["temporal".into(), "temporal_visibility".into()],
            grant: "ReadOnlyConnect".into(),
            server_fingerprint: "server-fingerprint".into(),
        }
    }
    fn target() -> DockerTarget {
        DockerTarget {
            target_id: "t".into(),
            cli_path: "/usr/local/bin/docker".into(),
            context_name: "desktop-linux".into(),
            endpoint_kind: "unix".into(),
            daemon_id: "d".into(),
            os_type: "linux".into(),
            architecture: "arm64".into(),
        }
    }
    fn container() -> Value {
        serde_json::json!({"Id":"abcdef0123456789","State":{"Running":true},"NetworkSettings":{"Networks":{"database-net":{"NetworkID":"network-id","Aliases":["postgres"]}},"Ports":{"5432/tcp":[{"HostIp":"127.0.0.1","HostPort":"15432"}]}}})
    }
    #[test]
    fn unpublished_container_authenticates_on_network() {
        assert_eq!(
            binding_topology(&binding(), &target(), &container()).unwrap(),
            AuthRoute::Network
        );
    }
    #[test]
    fn unsupported_topology_is_explicit() {
        let mut b = binding();
        b.container_id = "".into();
        assert_eq!(
            binding_topology(&b, &target(), &container())
                .unwrap_err()
                .code,
            "UnsupportedDependencyTopology"
        );
        let mut b = binding();
        b.target_id = "remote".into();
        assert!(binding_topology(&b, &target(), &container()).is_err());
    }
    #[test]
    fn host_auth_requires_actual_published_port() {
        let mut b = binding();
        b.host_endpoint = Some(Endpoint {
            label: "database".into(),
            component_id: "postgres".into(),
            host: "127.0.0.1".into(),
            port: 15432,
            protocol: "tcp".into(),
        });
        assert_eq!(
            binding_topology(&b, &target(), &container()).unwrap(),
            AuthRoute::Host
        );
        b.host_endpoint.as_mut().unwrap().port = 5432;
        assert!(binding_topology(&b, &target(), &container()).is_err());
    }
    #[test]
    fn external_grant_is_scoped() {
        let verified = VerifiedBinding { binding: binding() };
        let grant = InitializationGrant {
            target_id: "t".into(),
            server_fingerprint: "server-fingerprint".into(),
            allowed_databases: vec!["temporal".into()],
            allowed_accounts: vec![],
            operations: vec!["InitializeNamedResources".into()],
            confirmed_at: 1,
        };
        let result = authorize_binding(&verified, &grant).unwrap();
        assert_eq!(result.lifecycle_owner, "ExternalConnected");
        let mut evil = grant.clone();
        evil.allowed_databases.push("other_business".into());
        assert!(authorize_binding(&verified, &evil).is_err());
        evil = grant;
        evil.server_fingerprint = "different-server".into();
        assert!(authorize_binding(&verified, &evil).is_err());
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnedContainerRecord {
    pub container_id: String,
    pub daemon_id: String,
    pub target_id: String,
    pub workspace_id: String,
    pub instance_id: String,
    pub service_id: String,
    pub blueprint_sha: String,
    pub bind_sources: Vec<String>,
}
pub struct VerifiedOwnedContainer {
    pub(crate) record: OwnedContainerRecord,
}
impl VerifiedOwnedContainer {
    pub fn container_id(&self) -> &str {
        &self.record.container_id
    }
}
pub fn verify_ownership(
    container: &Value,
    record: &OwnedContainerRecord,
    target: &DockerTarget,
) -> Result<VerifiedOwnedContainer, ToolboxError> {
    if target.target_id != record.target_id
        || target.daemon_id != record.daemon_id
        || container["Id"] != record.container_id
    {
        return Err(error("OwnershipConflict"));
    }
    let labels = &container["Config"]["Labels"];
    for (key, expected) in [
        ("io.codeark.workspace-id", &record.workspace_id),
        ("io.codeark.instance-id", &record.instance_id),
        ("io.codeark.service-id", &record.service_id),
        ("io.codeark.blueprint-sha", &record.blueprint_sha),
    ] {
        if expected.is_empty() || labels[key].as_str() != Some(expected) {
            return Err(error("OwnershipConflict"));
        }
    }
    let actual = container["Mounts"]
        .as_array()
        .ok_or_else(|| error("OwnershipConflict"))?
        .iter()
        .filter(|m| m["Type"] == "bind")
        .map(|m| m["Source"].as_str().unwrap_or("").to_owned())
        .collect::<std::collections::BTreeSet<_>>();
    let expected = record
        .bind_sources
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    if actual != expected {
        return Err(error("OwnershipConflict"));
    }
    Ok(VerifiedOwnedContainer {
        record: record.clone(),
    })
}
#[cfg(test)]
mod ownership_tests {
    use super::*;
    #[test]
    fn ownership_requires_registry_labels_daemon_and_mounts() {
        let record = OwnedContainerRecord {
            container_id: "container".into(),
            daemon_id: "daemon".into(),
            target_id: "target".into(),
            workspace_id: "workspace".into(),
            instance_id: "instance".into(),
            service_id: "redis".into(),
            blueprint_sha: "sha".into(),
            bind_sources: vec!["/owned/stack/data".into()],
        };
        let target = DockerTarget {
            target_id: "target".into(),
            cli_path: "/usr/local/bin/docker".into(),
            context_name: "desktop-linux".into(),
            endpoint_kind: "unix".into(),
            daemon_id: "daemon".into(),
            os_type: "linux".into(),
            architecture: "arm64".into(),
        };
        let json = serde_json::json!({"Id":"container","Config":{"Labels":{"io.codeark.workspace-id":"workspace","io.codeark.instance-id":"instance","io.codeark.service-id":"redis","io.codeark.blueprint-sha":"sha"}},"Mounts":[{"Type":"bind","Source":"/owned/stack/data"}]});
        assert!(verify_ownership(&json, &record, &target).is_ok());
        let mut wrong = json.clone();
        wrong["Config"]["Labels"] = serde_json::json!({});
        assert!(verify_ownership(&wrong, &record, &target).is_err());
        wrong = json.clone();
        wrong["Mounts"][0]["Source"] = serde_json::json!("/other/business");
        assert!(verify_ownership(&wrong, &record, &target).is_err());
        let mut wrong_record = record;
        wrong_record.container_id = "old-container-id".into();
        assert!(verify_ownership(&json, &wrong_record, &target).is_err());
    }
}

struct SqlClientCleanup {
    runner: DockerRunner,
    nonce: String,
    directory: std::path::PathBuf,
    started: bool,
}
impl SqlClientCleanup {
    async fn finish(&mut self) -> Result<(), ToolboxError> {
        if self.nonce.is_empty() {
            return Ok(());
        }
        let result = if self.started {
            tokio::time::timeout(Duration::from_secs(15), async {
                self.runner.verify_identity().await?;
                let filter = format!("label=io.codeark.sql-owner={}", self.nonce);
                let raw = self
                    .runner
                    .run_read(&[
                        "ps",
                        "-aq",
                        "--no-trunc",
                        "--filter",
                        &filter,
                        "--filter",
                        "label=io.codeark.role=sql-probe",
                    ])
                    .await?;
                let ids = std::str::from_utf8(&raw).map_err(|_| error("DockerOutputInvalid"))?;
                for id in ids.lines() {
                    if !crate::probes::valid_id(id) {
                        return Err(error("CleanupPending"));
                    }
                    let raw = self
                        .runner
                        .run_read(&[
                            "container",
                            "inspect",
                            "--format",
                            "{{json .Config.Labels}}",
                            id,
                        ])
                        .await?;
                    let labels: Value =
                        serde_json::from_slice(&raw).map_err(|_| error("DockerOutputInvalid"))?;
                    if labels["io.codeark.sql-owner"] != self.nonce
                        || labels["io.codeark.role"] != "sql-probe"
                    {
                        return Err(error("OwnershipConflict"));
                    }
                    self.runner.verify_identity().await?;
                    self.runner.run_read(&["container", "rm", "-f", id]).await?;
                }
                Ok::<(), ToolboxError>(())
            })
            .await
            .unwrap_or_else(|_| Err(error("CleanupPending")))
        } else {
            Ok(())
        };
        if result.is_err()
            && let Some(parent) = self.directory.parent()
        {
            let receipt = serde_json::json!({"schemaVersion":1,"targetId":self.runner.target.target_id,"daemonId":self.runner.target.daemon_id,"owner":self.nonce,"role":"sql-probe","status":"CleanupPending"});
            let _ = atomic_write(
                &parent.join(format!("cleanup-{}.json", self.nonce)),
                &serde_json::to_vec(&receipt).map_err(|_| error("StateWriteFailed"))?,
            );
        }
        if crate::workspace::reject_links(&self.directory).is_ok() {
            let _ = std::fs::remove_dir_all(&self.directory);
        }
        self.nonce.clear();
        result
    }
}
impl Drop for SqlClientCleanup {
    fn drop(&mut self) {
        if self.nonce.is_empty() {
            return;
        }
        if !self.started {
            if crate::workspace::reject_links(&self.directory).is_ok() {
                let _ = std::fs::remove_dir_all(&self.directory);
            }
            return;
        }
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let mut owned = Self {
                runner: DockerRunner {
                    target: self.runner.target.clone(),
                },
                nonce: std::mem::take(&mut self.nonce),
                directory: self.directory.clone(),
                started: self.started,
            };
            handle.spawn(async move {
                let _ = owned.finish().await;
            });
        }
    }
}
