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
/// Accepts direct SHOW GRANTS evidence for one privilege and an optional
/// database. A missing database requires a global privilege, as replication does.
pub(crate) fn mysql_grants_allow(grants: &str, privilege: &str, database: Option<&str>) -> bool {
    // Partial revokes narrow global permissions; GRANT rows alone are then
    // insufficient evidence. Role-based permissions also need separate evidence.
    if grants.lines().any(|line| line.starts_with("REVOKE ")) {
        return false;
    }
    grants.lines().any(|line| {
        let Some((privileges, scope)) = line
            .strip_prefix("GRANT ")
            .and_then(|line| line.split_once(" ON "))
        else {
            return false;
        };
        let Some((scope, _)) = scope.split_once(" TO ") else {
            return false;
        };
        let scope_matches = scope == "*.*"
            || database.is_some_and(|name| {
                !name.is_empty()
                    && (scope == mysql_database_scope(name, true)
                        || scope == mysql_database_scope(name, false))
            });
        scope_matches
            && privileges
                .split(", ")
                .any(|value| value == privilege || value == "ALL PRIVILEGES")
    })
}

pub(crate) fn mysql_database_grant_scope(
    database: &str,
    partial_revokes: bool,
) -> Result<String, ToolboxError> {
    if !valid_database_name(database) {
        return Err(error("InitializationGrantOutOfScope"));
    }
    Ok(mysql_database_scope(database, partial_revokes))
}
fn mysql_database_scope(database: &str, partial_revokes: bool) -> String {
    // Identifier quotes do not disable MySQL database-level GRANT wildcards.
    let database = database.replace('`', "``");
    let database = if partial_revokes {
        database
    } else {
        database.replace('_', "\\_").replace('%', "\\%")
    };
    format!("`{database}`.*")
}
pub fn binding_topology(
    binding: &DatabaseBinding,
    target: &DockerTarget,
    container: &Value,
) -> Result<AuthRoute, ToolboxError> {
    if binding.container_id.is_empty()
        || binding.container_port == 0
        || binding.network_alias.is_empty()
        || !binding
            .network_alias
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
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
/// Expanding an authentication scope requires the backend-issued preview and an
/// explicit confirmation. This helper never accepts arbitrary frontend SQL.
pub(crate) fn authorize_preview(
    verified: &VerifiedBinding,
    grant: &InitializationGrant,
) -> Result<AuthorizedBinding, ToolboxError> {
    let mut scoped = verified.binding.clone();
    scoped.allowed_databases = grant.allowed_databases.clone();
    authorize_binding(&VerifiedBinding { binding: scoped }, grant)
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
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != grant.allowed_databases.len()
        || grant
            .allowed_databases
            .iter()
            .any(|db| !valid_database_name(db) || !binding.allowed_databases.contains(db))
        || ![
            vec!["InitializeNamedResources".to_owned()],
            vec!["ObserveReplication".to_owned()],
        ]
        .contains(&grant.operations)
        || (grant.operations == ["ObserveReplication"] && binding.engine != "mysql")
    {
        return Err(error("InitializationGrantOutOfScope"));
    }
    if grant
        .allowed_accounts
        .iter()
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        != grant.allowed_accounts.len()
        || grant.allowed_accounts.iter().any(|account| {
            account.is_empty()
                || account.len() > 32
                || !account
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        })
    {
        return Err(error("InitializationGrantOutOfScope"));
    }
    let mut result = binding.clone();
    result.grant = grant.operations[0].clone();
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
    network_query_database(runner, binding, credentials, image, context, None, sql).await
}
pub(crate) async fn network_query_database(
    runner: &DockerRunner,
    binding: &DatabaseBinding,
    credentials: &Credentials,
    image: &ImageArtifactRef,
    context: &ImageContext,
    database: Option<&str>,
    sql: &str,
) -> Result<String, ToolboxError> {
    if database.is_some_and(|name| {
        !valid_database_name(name)
            || !binding
                .allowed_databases
                .iter()
                .any(|allowed| allowed == name)
    }) {
        return Err(error("InitializationGrantOutOfScope"));
    }
    recover_sql_clients(runner, &context.cache_root).await?;
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
        // The owner-checked cleanup below is the sole remover. Docker's --rm
        // races its ps/inspect verification when a short SELECT exits quickly.
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
            format!("--dbname={}", database.unwrap_or("postgres")).into(),
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
    let cleanup_result = cleanup.finish().await;
    // A failed SQL command remains the primary failure; cleanup records its
    // independent pending work durably before returning.
    let result = result.map_err(|_| error("DatabaseQueryFailed"))?;
    cleanup_result?;
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
    #[test]
    fn initialization_grants_reject_wrong_targets_operations_and_duplicate_scope() {
        let verified = VerifiedBinding { binding: binding() };
        let grant = InitializationGrant {
            target_id: "t".into(),
            server_fingerprint: "server-fingerprint".into(),
            allowed_databases: vec!["temporal".into()],
            allowed_accounts: vec![],
            operations: vec!["InitializeNamedResources".into()],
            confirmed_at: 1,
        };
        let mut bad = grant.clone();
        bad.target_id = "other-target".into();
        assert_eq!(
            authorize_binding(&verified, &bad).err().unwrap().code,
            "ServerIdentityMismatch"
        );
        for operations in [
            vec![],
            vec!["ReadOnlyConnect".into()],
            vec!["ObserveReplication".into()],
            vec![
                "InitializeNamedResources".into(),
                "ObserveReplication".into(),
            ],
        ] {
            bad = grant.clone();
            bad.operations = operations;
            assert_eq!(
                authorize_binding(&verified, &bad).err().unwrap().code,
                "InitializationGrantOutOfScope"
            );
        }
        bad = grant.clone();
        bad.allowed_databases.push("temporal".into());
        assert!(authorize_binding(&verified, &bad).is_err());
        bad = grant.clone();
        bad.allowed_accounts = vec!["replica".into(), "replica".into()];
        assert!(authorize_binding(&verified, &bad).is_err());
        bad = grant;
        bad.allowed_databases = vec!["temporal';DROP DATABASE x;--".into()];
        assert!(authorize_binding(&verified, &bad).is_err());
    }
    #[test]
    fn network_topology_rejects_control_characters_and_zero_port() {
        let mut b = binding();
        b.container_port = 0;
        assert!(binding_topology(&b, &target(), &container()).is_err());
        b = binding();
        b.network_alias = "postgres\nMYSQL_HOST=other".into();
        let mut inspected = container();
        inspected["NetworkSettings"]["Networks"]["database-net"]["Aliases"] =
            serde_json::json!([b.network_alias]);
        assert!(binding_topology(&b, &target(), &inspected).is_err());
    }
    #[test]
    fn mysql_readiness_accepts_escaped_database_scope_but_not_other_grants() {
        assert!(mysql_grants_allow(
            "GRANT SELECT ON `dev-db`.* TO `app`@`%`",
            "SELECT",
            Some("dev-db")
        ));
        let grants =
            "GRANT USAGE ON *.* TO `app`@`%`\nGRANT ALL PRIVILEGES ON `test\\_db`.* TO `app`@`%`";
        assert!(mysql_grants_allow(grants, "SELECT", Some("test_db")));
        assert!(!mysql_grants_allow(grants, "SELECT", Some("testxdb")));
        assert!(!mysql_grants_allow(grants, "SELECT", Some("other_db")));
        assert!(!mysql_grants_allow(grants, "REPLICATION CLIENT", None));
        assert!(mysql_grants_allow(
            "GRANT SELECT ON `test_db`.* TO `app`@`%`",
            "SELECT",
            Some("test_db")
        ));
        assert!(mysql_grants_allow(
            "GRANT SELECT ON *.* TO `app`@`%`",
            "SELECT",
            Some("test_db")
        ));
        assert!(!mysql_grants_allow(
            "GRANT INSERT ON `test\\_db`.* TO `SELECT`@`%`",
            "SELECT",
            Some("test_db")
        ));
        assert!(!mysql_grants_allow(
            "GRANT SELECT ON `test\\_db`.`other_table` TO `app`@`%`",
            "SELECT",
            Some("test_db")
        ));
        assert!(!mysql_grants_allow(
            "GRANT ALL PRIVILEGES ON *.* TO `app`@`%`\nREVOKE SELECT ON `test_db`.* FROM `app`@`%`",
            "SELECT",
            Some("test_db")
        ));
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
/// Returns host paths for comparison only; never changes a mount or a data path.
pub(crate) fn container_bind_sources(
    container: &Value,
    target: &DockerTarget,
) -> Result<Vec<String>, ToolboxError> {
    container["Mounts"]
        .as_array()
        .ok_or_else(|| error("OwnershipConflict"))?
        .iter()
        .filter(|mount| mount["Type"] == "bind")
        .map(|mount| {
            let source = mount["Source"]
                .as_str()
                .filter(|source| !source.is_empty())
                .ok_or_else(|| error("OwnershipConflict"))?;
            let desktop = cfg!(target_os = "macos")
                && target.os_type == "linux"
                && target.context_name == "desktop-linux"
                && target.endpoint_kind == "unix"
                && target.cli_path == "/Applications/Docker.app/Contents/Resources/bin/docker";
            if !desktop || !source.starts_with("/host_mnt/Users/") {
                return Ok(source.to_owned());
            }
            let path = std::path::Path::new(source);
            if path.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir | std::path::Component::CurDir
                )
            }) {
                return Err(error("OwnershipConflict"));
            }
            let host = source
                .strip_prefix("/host_mnt")
                .ok_or_else(|| error("OwnershipConflict"))?;
            let destination = mount["Destination"]
                .as_str()
                .filter(|destination| destination.starts_with('/'))
                .ok_or_else(|| error("OwnershipConflict"))?;
            let same_source = |configured: &str| configured == source || configured == host;
            let configured_mount =
                container["HostConfig"]["Mounts"]
                    .as_array()
                    .is_some_and(|mounts| {
                        mounts.iter().any(|mount| {
                            mount["Type"] == "bind"
                                && mount["Target"] == destination
                                && mount["Source"].as_str().is_some_and(same_source)
                        })
                    });
            let configured_bind =
                container["HostConfig"]["Binds"]
                    .as_array()
                    .is_some_and(|binds| {
                        binds.iter().filter_map(Value::as_str).any(|bind| {
                            // Split from the destination side, preserving ':' in macOS paths.
                            let mut parts = bind.rsplitn(3, ':');
                            let _mode = parts.next();
                            parts.next() == Some(destination)
                                && parts.next().is_some_and(same_source)
                        })
                    });
            if !configured_mount && !configured_bind {
                return Err(error("OwnershipConflict"));
            }
            Ok(host.to_owned())
        })
        .collect()
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
    let actual = container_bind_sources(container, target)?
        .into_iter()
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
    #[test]
    fn desktop_alias_requires_local_macos_target_and_matching_mount_configuration() {
        let target = DockerTarget {
            target_id: "target".into(),
            cli_path: "/Applications/Docker.app/Contents/Resources/bin/docker".into(),
            context_name: "desktop-linux".into(),
            endpoint_kind: "unix".into(),
            daemon_id: "daemon".into(),
            os_type: "linux".into(),
            architecture: "arm64".into(),
        };
        let source = "/Users/local/project/config.yml";
        let alias = format!("/host_mnt{source}");
        let record = OwnedContainerRecord {
            container_id: "container".into(),
            daemon_id: "daemon".into(),
            target_id: "target".into(),
            workspace_id: "workspace".into(),
            instance_id: "instance".into(),
            service_id: "monitoring".into(),
            blueprint_sha: "sha".into(),
            bind_sources: vec![source.into()],
        };
        let inspected = serde_json::json!({"Id":"container","Config":{"Labels":{"io.codeark.workspace-id":"workspace","io.codeark.instance-id":"instance","io.codeark.service-id":"monitoring","io.codeark.blueprint-sha":"sha"}},"Mounts":[{"Type":"bind","Source":alias,"Destination":"/etc/config.yml"}],"HostConfig":{"Binds":[format!("{alias}:/etc/config.yml:ro")]}});
        assert_eq!(
            verify_ownership(&inspected, &record, &target).is_ok(),
            cfg!(target_os = "macos")
        );
        for field in ["context", "cli", "endpoint"] {
            let mut other = target.clone();
            match field {
                "context" => other.context_name = "other-local".into(),
                "cli" => other.cli_path = "/usr/bin/docker".into(),
                _ => other.endpoint_kind = "npipe".into(),
            }
            assert!(verify_ownership(&inspected, &record, &other).is_err());
        }
        for alternate in [
            "/host_mnt/Users/local/project/other.yml",
            "/host_mnt/Users/local/../project/config.yml",
            "/run/desktop/mnt/host/Users/local/project/config.yml",
            "/host_mnt/Users2/local/project/config.yml",
        ] {
            let mut wrong = inspected.clone();
            wrong["Mounts"][0]["Source"] = serde_json::json!(alternate);
            wrong["HostConfig"]["Binds"] =
                serde_json::json!([format!("{alternate}:/etc/config.yml:ro")]);
            assert!(verify_ownership(&wrong, &record, &target).is_err());
        }
        let mut wrong = inspected.clone();
        wrong["HostConfig"] = serde_json::Value::Null;
        assert!(verify_ownership(&wrong, &record, &target).is_err());
        wrong = inspected.clone();
        wrong["HostConfig"]["Binds"] = serde_json::json!([format!("{alias}:/etc/other.yml:ro")]);
        assert!(verify_ownership(&wrong, &record, &target).is_err());
        wrong = inspected.clone();
        wrong["HostConfig"]["Binds"] =
            serde_json::json!(["/Users/other/config.yml:/etc/config.yml:ro"]);
        assert!(verify_ownership(&wrong, &record, &target).is_err());
        let mut long_mount = inspected;
        long_mount["HostConfig"] = serde_json::json!({"Mounts":[{"Type":"bind","Source":source,"Target":"/etc/config.yml"}]});
        assert_eq!(
            verify_ownership(&long_mount, &record, &target).is_ok(),
            cfg!(target_os = "macos")
        );
    }
}

#[cfg(all(test, unix))]
mod sql_client_tests {
    use super::*;
    #[tokio::test]
    async fn postgres_client_selects_the_scoped_database_and_retains_it_until_owned_cleanup() {
        let root = tempfile::tempdir().unwrap();
        let program = root.path().join("fake-docker");
        std::fs::write(&program, r#"#!/usr/bin/env python3
import sys,json
from pathlib import Path
root=Path(__file__).parent
a=sys.argv[3:]
state=root/'client.json'
if a[0]=='info': print(json.dumps({'ID':'daemon','OSType':'linux'}))
elif a[:2]==['context','inspect']: print('[{"Endpoints":{"docker":{"Host":"unix:///fake.sock"}}}]')
elif a[:2]==['image','inspect']: print(json.dumps([{'Id':'sha256:'+'a'*64,'Os':'linux','Architecture':'arm64','RepoDigests':['postgres@sha256:'+'b'*64]}]))
elif a[0]=='run':
 assert not any('private-password' in value for value in a)
 assert '--no-psqlrc' in a and '--set=ON_ERROR_STOP=1' in a
 mount=a[a.index('--mount')+1]
 sql=Path(mount.split('source=',1)[1].split(',target=',1)[0]).read_text()
 assert sql in ['SELECT current_database()', 'SELECT failing_query()']
 owner=next(value.split('=',1)[1] for value in a if value.startswith('io.codeark.sql-owner='))
 state.write_text(json.dumps({'owner':owner,'autoRemoved':'--rm' in a}))
 if sql=='SELECT failing_query()':
  print('private-password', file=sys.stderr)
  sys.exit(1)
 print(next(value.split('=',1)[1] for value in a if value.startswith('--dbname=')))
elif a[0]=='ps':
 assert 'label=io.codeark.role=sql-probe' in a
 if state.exists() and 'label=io.codeark.sql-owner='+json.loads(state.read_text())['owner'] in a: print('c'*64)
elif a[:2]==['container','inspect']:
 s=json.loads(state.read_text())
 if s['autoRemoved']: sys.exit(1)
 print(json.dumps({'io.codeark.sql-owner':s['owner'],'io.codeark.role':'sql-probe'}))
elif a[:3]==['container','rm','-f']:
 assert a[3]=='c'*64
 if (root/'deny-cleanup').exists(): sys.exit(1)
 if (root/'slow-remove-before-delete').exists():
  import time
  time.sleep(15)
 state.unlink()
 (root/'cleaned').write_text('yes')
 if (root/'slow-remove-response').exists():
  import time
  time.sleep(15)
else: sys.exit(1)
"#).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        let runner = DockerRunner {
            target: DockerTarget {
                target_id: "target".into(),
                cli_path: program.to_str().unwrap().into(),
                context_name: "fake".into(),
                endpoint_kind: "unix".into(),
                daemon_id: "daemon".into(),
                os_type: "linux".into(),
                architecture: "arm64".into(),
            },
        };
        let binding = DatabaseBinding {
            binding_id: "binding".into(),
            target_id: "target".into(),
            daemon_id: "daemon".into(),
            engine: "pgsql".into(),
            lifecycle_owner: "Managed".into(),
            container_id: "d".repeat(64),
            network_name: "pg-network".into(),
            network_alias: "pg".into(),
            container_port: 5432,
            host_endpoint: None,
            credential_ref: SecretRef {
                key: "credential".into(),
            },
            allowed_databases: vec!["temporal".into(), "temporal_visibility".into()],
            grant: "InitializeNamedResources".into(),
            server_fingerprint: "server".into(),
        };
        let credentials = Credentials {
            username: "user".into(),
            password: Zeroizing::new("private-password".into()),
        };
        let image = ImageArtifactRef {
            source_id: "pg".into(),
            kind: "registry".into(),
            platform: "linux/arm64".into(),
            registry_ref: Some("postgres".into()),
            manifest_digest: Some(format!("sha256:{}", "b".repeat(64))),
            archive_url: None,
            archive_sha256: None,
            expected_image_id: None,
            local_content_tag: None,
            status: "resolved".into(),
        };
        let context = ImageContext {
            cache_root: root.path().join("cache"),
            cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };
        for (database, expected) in [
            (None, "postgres"),
            (Some("temporal"), "temporal"),
            (Some("temporal_visibility"), "temporal_visibility"),
        ] {
            assert_eq!(
                network_query_database(
                    &runner,
                    &binding,
                    &credentials,
                    &image,
                    &context,
                    database,
                    "SELECT current_database()"
                )
                .await
                .unwrap(),
                expected
            );
            assert!(root.path().join("cleaned").exists());
            assert!(!root.path().join("client.json").exists());
        }
        assert_eq!(
            network_query_database(
                &runner,
                &binding,
                &credentials,
                &image,
                &context,
                Some("other_business"),
                "SELECT current_database()"
            )
            .await
            .unwrap_err()
            .code,
            "InitializationGrantOutOfScope"
        );
        assert_eq!(
            network_query_database(
                &runner,
                &binding,
                &credentials,
                &image,
                &context,
                None,
                "SELECT failing_query()"
            )
            .await
            .unwrap_err()
            .code,
            "DatabaseQueryFailed"
        );
        std::fs::write(root.path().join("deny-cleanup"), "yes").unwrap();
        // Both fail: retain the SQL failure and a credential-free cleanup receipt.
        assert_eq!(
            network_query_database(
                &runner,
                &binding,
                &credentials,
                &image,
                &context,
                None,
                "SELECT failing_query()"
            )
            .await
            .unwrap_err()
            .code,
            "DatabaseQueryFailed"
        );
        let receipts = std::fs::read_dir(&context.cache_root)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("cleanup-")
            })
            .collect::<Vec<_>>();
        assert_eq!(receipts.len(), 1);
        let raw = std::fs::read_to_string(&receipts[0]).unwrap();
        assert!(!raw.contains("private-password"));
        let receipt: SqlCleanupReceipt = serde_json::from_str(&raw).unwrap();
        assert_eq!(receipt.status, "CleanupPending");
        // A live owner lock prevents replay from interfering with its cleanup.
        let lock = crate::locks::acquire(
            &context.cache_root.join("sql-cleanup-locks"),
            &[format!("sql-probe/{}", receipt.owner)],
        )
        .unwrap();
        recover_sql_clients(&runner, &context.cache_root)
            .await
            .unwrap();
        assert!(root.path().join("client.json").exists());
        drop(lock);
        std::fs::remove_file(root.path().join("deny-cleanup")).unwrap();
        // A new query replays only the durable failed cleanup before proceeding.
        assert_eq!(
            network_query_database(
                &runner,
                &binding,
                &credentials,
                &image,
                &context,
                None,
                "SELECT current_database()"
            )
            .await
            .unwrap(),
            "postgres"
        );
        assert!(!receipts[0].exists());
        assert!(!root.path().join("client.json").exists());
        // A successful query whose cleanup fails must still fail explicitly.
        std::fs::write(root.path().join("deny-cleanup"), "yes").unwrap();
        assert!(
            network_query_database(
                &runner,
                &binding,
                &credentials,
                &image,
                &context,
                None,
                "SELECT current_database()"
            )
            .await
            .is_err()
        );
        assert!(root.path().join("client.json").exists());
        std::fs::remove_file(root.path().join("deny-cleanup")).unwrap();
        let receipt_path = std::fs::read_dir(&context.cache_root)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("cleanup-")
            })
            .unwrap();
        let saved = std::fs::read_to_string(&receipt_path).unwrap();
        let mut foreign: Value = serde_json::from_str(&saved).unwrap();
        foreign["daemonId"] = serde_json::json!("another-daemon");
        atomic_write(&receipt_path, &serde_json::to_vec(&foreign).unwrap()).unwrap();
        recover_sql_clients(&runner, &context.cache_root)
            .await
            .unwrap();
        assert!(root.path().join("client.json").exists());
        foreign = serde_json::from_str(&saved).unwrap();
        foreign["role"] = serde_json::json!("external-service");
        atomic_write(&receipt_path, &serde_json::to_vec(&foreign).unwrap()).unwrap();
        assert_eq!(
            recover_sql_clients(&runner, &context.cache_root)
                .await
                .unwrap_err()
                .code,
            "CleanupPending"
        );
        assert!(root.path().join("client.json").exists());
        atomic_write(&receipt_path, saved.as_bytes()).unwrap();
        recover_sql_clients(&runner, &context.cache_root)
            .await
            .unwrap();
        assert!(!root.path().join("client.json").exists());
        for (flag, removed) in [
            ("slow-remove-response", true),
            ("slow-remove-before-delete", false),
        ] {
            let owner = "e".repeat(32);
            let directory = context
                .cache_root
                .join(format!("sql-client-delayed-{owner}"));
            private_dir(&directory).unwrap();
            atomic_write(&directory.join("client.env"), b"private-password").unwrap();
            atomic_write(
                &root.path().join("client.json"),
                &serde_json::to_vec(&serde_json::json!({"owner":owner,"autoRemoved":false}))
                    .unwrap(),
            )
            .unwrap();
            std::fs::write(root.path().join(flag), "yes").unwrap();
            let _lock = crate::locks::acquire(
                &context.cache_root.join("sql-cleanup-locks"),
                &[format!("sql-probe/{owner}")],
            )
            .unwrap();
            let mut cleanup = SqlClientCleanup {
                runner: DockerRunner {
                    target: runner.target.clone(),
                },
                nonce: owner.clone(),
                directory: directory.clone(),
                started: true,
            };
            let result = cleanup
                .finish_with_deadlines(Duration::from_secs(5), Duration::from_secs(3))
                .await;
            let receipt_path = context.cache_root.join(format!("cleanup-{owner}.json"));
            if removed {
                assert!(
                    result.is_ok(),
                    "A verified empty owner set proves delayed rm completed"
                );
                assert!(!receipt_path.exists());
            } else {
                assert_eq!(result.unwrap_err().code, "CleanupTimedOut");
                let receipt: Value = crate::workspace::read_json(&receipt_path).unwrap();
                assert_eq!(receipt["reasonCode"], "CleanupTimedOut");
                assert!(!receipt.to_string().contains("private-password"));
                assert!(root.path().join("client.json").exists());
            }
            assert!(!directory.exists());
            std::fs::remove_file(root.path().join(flag)).unwrap();
            drop(_lock);
            recover_sql_clients(&runner, &context.cache_root)
                .await
                .unwrap();
            assert!(!receipt_path.exists());
            assert!(!root.path().join("client.json").exists());
        }
    }
}

struct SqlClientCleanup {
    runner: DockerRunner,
    nonce: String,
    directory: std::path::PathBuf,
    started: bool,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SqlCleanupReceipt {
    schema_version: u32,
    target_id: String,
    daemon_id: String,
    owner: String,
    role: String,
    status: String,
}
fn cleanup_owner(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
async fn recover_sql_clients(
    runner: &DockerRunner,
    root: &std::path::Path,
) -> Result<(), ToolboxError> {
    private_dir(root)?;
    for entry in std::fs::read_dir(root).map_err(|_| error("StateUnavailable"))? {
        let entry = entry.map_err(|_| error("StateUnavailable"))?;
        let name = entry.file_name();
        let Some(owner) = name
            .to_str()
            .and_then(|name| name.strip_prefix("cleanup-"))
            .and_then(|name| name.strip_suffix(".json"))
            .filter(|owner| cleanup_owner(owner))
        else {
            continue;
        };
        let _lock = match crate::locks::acquire(
            &root.join("sql-cleanup-locks"),
            &[format!("sql-probe/{owner}")],
        ) {
            Ok(lock) => lock,
            Err(error) if error.code == "ResourceBusy" => continue,
            Err(error) => return Err(error),
        };
        if !entry.path().exists() {
            continue;
        }
        let receipt: SqlCleanupReceipt = crate::workspace::read_json(&entry.path())?;
        if receipt.target_id != runner.target.target_id
            || receipt.daemon_id != runner.target.daemon_id
        {
            continue;
        }
        if receipt.schema_version != 1
            || receipt.owner != owner
            || receipt.role != "sql-probe"
            || receipt.status != "CleanupPending"
        {
            return Err(error("CleanupPending"));
        }
        let mut cleanup = SqlClientCleanup {
            runner: DockerRunner {
                target: runner.target.clone(),
            },
            nonce: owner.into(),
            directory: root.join(format!("sql-client-recovery-{owner}")),
            started: true,
        };
        cleanup.finish_locked().await?;
    }
    Ok(())
}
impl SqlClientCleanup {
    async fn finish(&mut self) -> Result<(), ToolboxError> {
        if self.nonce.is_empty() {
            return Ok(());
        }
        let root = self
            .directory
            .parent()
            .ok_or_else(|| error("CleanupPending"))?;
        let _lock = crate::locks::acquire(
            &root.join("sql-cleanup-locks"),
            &[format!("sql-probe/{}", self.nonce)],
        )?;
        self.finish_locked().await
    }
    async fn finish_locked(&mut self) -> Result<(), ToolboxError> {
        // Total cleanup is bounded to 60 seconds, including a reserved absence
        // check if Docker removed the container but its response was interrupted.
        self.finish_with_deadlines(Duration::from_secs(45), Duration::from_secs(15))
            .await
    }
    async fn finish_with_deadlines(
        &mut self,
        primary: Duration,
        recheck: Duration,
    ) -> Result<(), ToolboxError> {
        if self.nonce.is_empty() {
            return Ok(());
        }
        let mut result = if self.started {
            tokio::time::timeout(primary, async {
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
            .unwrap_or_else(|_| Err(error("CleanupTimedOut")))
        } else {
            Ok(())
        };
        if result.as_ref().is_err_and(|error| {
            matches!(
                error.code.as_str(),
                "CleanupTimedOut" | "DockerCommandFailed"
            )
        }) {
            // Issuing rm is not proof of removal. Reconnect to the same daemon
            // and require an empty owner+role-filtered set before clearing evidence.
            let absent = tokio::time::timeout(recheck, async {
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
                Ok::<bool, ToolboxError>(ids.trim().is_empty())
            })
            .await;
            if matches!(absent, Ok(Ok(true))) {
                result = Ok(());
            }
        }
        if result.is_err()
            && let Some(parent) = self.directory.parent()
        {
            let receipt = serde_json::json!({"schemaVersion":1,"targetId":self.runner.target.target_id,"daemonId":self.runner.target.daemon_id,"owner":self.nonce,"role":"sql-probe","status":"CleanupPending","reasonCode":result.as_ref().err().map(|error| error.code.as_str()).unwrap_or("CleanupPending")});
            atomic_write(
                &parent.join(format!("cleanup-{}.json", self.nonce)),
                &serde_json::to_vec(&receipt).map_err(|_| error("StateWriteFailed"))?,
            )?;
        } else if result.is_ok()
            && let Some(parent) = self.directory.parent()
        {
            let path = parent.join(format!("cleanup-{}.json", self.nonce));
            crate::workspace::reject_links(&path)?;
            if path.exists() {
                std::fs::remove_file(path).map_err(|_| error("StateWriteFailed"))?;
            }
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
