use crate::{
    bindings::{self, AuthorizedBinding, Credentials},
    contracts::*,
    env_store::SecretValues,
    image_store::ImageContext,
    orchestrator::Engine,
    runtime_state::RuntimeInstance,
    workspace::{atomic_write, error, read_json, reject_links},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, path::Path};
use zeroize::Zeroizing;

pub(crate) struct Dependency {
    binding: AuthorizedBinding,
    credentials: Credentials,
    image: ImageArtifactRef,
    pub inherited: SecretValues,
    managed: bool,
}

pub(crate) fn databases(
    service: &str,
    instance: &InstanceIdentity,
) -> Result<Vec<String>, ToolboxError> {
    if instance.service_id != service || !valid_id(&instance.instance_id) {
        return Err(error("IdentityMismatch"));
    }
    if !matches!(service, "xxjob" | "seata" | "temporal" | "canal") {
        return Err(error("UnsupportedInitializationSchema"));
    }
    let suffix = &instance.instance_id[..12];
    Ok(if service == "temporal" {
        vec!["temporal".into(), "temporal_visibility".into()]
    } else {
        vec![format!("ca_{suffix}_{service}")]
    })
}
fn valid_id(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn check_grant_scope(
    service: &str,
    binding: &DatabaseBinding,
    grant: &InitializationGrant,
    names: &[String],
) -> Result<(), ToolboxError> {
    let operation = if service == "canal" {
        "ObserveReplication"
    } else {
        "InitializeNamedResources"
    };
    if grant.operations != [operation]
        || binding.grant != operation
        || !grant.allowed_accounts.is_empty()
        || (service != "canal" && grant.allowed_databases != names)
    {
        return Err(error("InitializationGrantOutOfScope"));
    }
    Ok(())
}
fn env(instance: &RuntimeInstance, stack: &Path) -> Result<SecretValues, ToolboxError> {
    let applied = instance
        .applied
        .as_ref()
        .ok_or_else(|| error("DependencyUnavailable"))?;
    if applied.generation_id.len() != 32
        || !applied.generation_id.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(error("IdentityMismatch"));
    }
    let path = stack
        .join(".runtime")
        .join(&applied.generation_id)
        .join("effective.env");
    reject_links(&path)?;
    let raw = Zeroizing::new(
        std::fs::read_to_string(path).map_err(|_| error("BindingCredentialRequired"))?,
    );
    Ok(SecretValues(crate::env_store::parse_env(&raw)?))
}
fn required(values: &SecretValues, key: &str) -> Result<String, ToolboxError> {
    values
        .0
        .get(key)
        .filter(|v| !v.is_empty())
        .cloned()
        .ok_or_else(|| error("BindingCredentialRequired"))
}

impl Engine {
    /// Resolves a verified, explicitly granted external binding or the owned dependency
    /// that the same startup operation has already brought to Ready.
    pub(crate) async fn dependency(
        &self,
        runner: &crate::docker_cli::DockerRunner,
        service: &ServiceSpec,
        instance: &mut RuntimeInstance,
        context: &ImageContext,
    ) -> Result<Option<Dependency>, ToolboxError> {
        let Some(engine) = service.dependencies.first() else {
            return Ok(None);
        };
        if service.dependencies.len() != 1 || !["mysql", "pgsql"].contains(&engine.as_str()) {
            return Err(error("UnsupportedDependencyTopology"));
        }
        let store = self.store()?;
        let scoped_databases = databases(&service.id, &instance.identity)?;
        let existing = instance
            .bindings
            .iter()
            .find(|b| b.engine == *engine && b.lifecycle_owner != "Managed")
            .cloned();
        let managed = existing.is_none();
        let (binding, credentials, grant) = if let Some(binding) = existing {
            if binding.binding_id.len() != 32
                || !binding.binding_id.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(error("InvalidBinding"));
            }
            #[derive(Deserialize)]
            struct SavedCredentials {
                username: String,
                password: String,
            }
            let root = Path::new(&store.workspace.root);
            let saved: SavedCredentials = read_json(
                &root
                    .join("credentials")
                    .join(format!("{}.json", binding.binding_id)),
            )?;
            let grant_path = root.join("grants").join(format!(
                "{}-{}.json",
                binding.binding_id, instance.identity.instance_id
            ));
            let grant_path = if grant_path.exists() {
                grant_path
            } else {
                root.join("grants")
                    .join(format!("{}.json", binding.binding_id))
            };
            let grant: InitializationGrant = read_json(&grant_path)?;
            check_grant_scope(&service.id, &binding, &grant, &scoped_databases)?;
            (
                binding,
                Credentials {
                    username: saved.username,
                    password: Zeroizing::new(saved.password),
                },
                grant,
            )
        } else {
            let provider = store
                .list()?
                .into_iter()
                .find(|i| {
                    i.identity.service_id == *engine
                        && i.identity.target_id == runner.target.target_id
                        && i.identity.daemon_id == runner.target.daemon_id
                        && i.phase == ServicePhase::Ready
                })
                .ok_or_else(|| error("DependencyUnavailable"))?;
            let component = self
                .service(engine)?
                .components
                .into_iter()
                .find(|c| c.role == "daemon")
                .ok_or_else(|| error("InvalidCatalog"))?;
            let record = provider
                .containers
                .first()
                .ok_or_else(|| error("DependencyUnavailable"))?;
            if record.instance_id != provider.identity.instance_id
                || record.workspace_id != instance.identity.workspace_id
                || record.service_id != *engine
                || !valid_id(&provider.identity.instance_id)
            {
                return Err(error("OwnershipConflict"));
            }
            let inspected = runner
                .run_read(&["container", "inspect", &record.container_id])
                .await?;
            let inspected: Value =
                serde_json::from_slice(&inspected).map_err(|_| error("DockerOutputInvalid"))?;
            bindings::verify_ownership(&inspected[0], record, &runner.target)?;
            if inspected[0]["Config"]["Labels"]["com.docker.compose.service"] != component.id {
                return Err(error("UnsupportedDependencyTopology"));
            }
            let provider_image = self
                .images(&self.service(engine)?, &runner.target)?
                .into_iter()
                .find(|image| image.source_id == component.image_ref)
                .ok_or_else(|| error("InvalidCatalog"))?;
            let verified_image =
                crate::image_store::ensure_service_image(runner, &provider_image, context).await?;
            if inspected[0]["Image"] != verified_image.image_id {
                return Err(error("ImageIdentityMismatch"));
            }
            let alias = format!("{}-{}", provider.identity.project_name, component.id);
            let network = provider
                .applied_networks
                .iter()
                .find(|name| {
                    inspected[0]["NetworkSettings"]["Networks"][name.as_str()]["Aliases"]
                        .as_array()
                        .is_some_and(|a| a.iter().any(|v| v == &alias))
                })
                .ok_or_else(|| error("UnsupportedDependencyTopology"))?
                .clone();
            let values = env(&provider, &store.stack(engine)?)?;
            let username = if engine == "mysql" {
                "root".into()
            } else {
                required(&values, "PGSQL_USER")?
            };
            let password = required(
                &values,
                if engine == "mysql" {
                    "MYSQL_ROOT_PASSWORD"
                } else {
                    "PGSQL_PASSWORD"
                },
            )?;
            let port = if engine == "mysql" { 3306 } else { 5432 };
            let allowed_databases = if service.id == "canal" {
                vec![required(&values, "MYSQL_DATABASE")?]
            } else {
                scoped_databases.clone()
            };
            let binding = DatabaseBinding {
                binding_id: format!("managed-{}", provider.identity.instance_id),
                target_id: runner.target.target_id.clone(),
                daemon_id: runner.target.daemon_id.clone(),
                engine: engine.clone(),
                lifecycle_owner: "Managed".into(),
                container_id: record.container_id.clone(),
                network_name: network,
                network_alias: alias,
                container_port: port,
                host_endpoint: provider
                    .applied
                    .as_ref()
                    .and_then(|e| {
                        e.ports
                            .iter()
                            .find(|p| p.component_id == component.id && p.container_port == port)
                    })
                    .map(|p| Endpoint {
                        label: component.id.clone(),
                        component_id: component.id.clone(),
                        host: p.host_ip.clone(),
                        port: p.host_port,
                        protocol: "tcp".into(),
                    }),
                credential_ref: SecretRef {
                    key: format!("managed-{}", provider.identity.instance_id),
                },
                allowed_databases,
                grant: "None".into(),
                server_fingerprint: String::new(),
            };
            let grant = InitializationGrant {
                target_id: runner.target.target_id.clone(),
                server_fingerprint: String::new(),
                allowed_databases: binding.allowed_databases.clone(),
                allowed_accounts: if service.id == "canal" {
                    vec![format!("ca_{}_canal", &instance.identity.instance_id[..12])]
                } else {
                    vec![]
                },
                operations: vec![
                    if service.id == "canal" {
                        "ObserveReplication"
                    } else {
                        "InitializeNamedResources"
                    }
                    .into(),
                ],
                confirmed_at: crate::orchestrator::now(),
            };
            (
                binding,
                Credentials {
                    username,
                    password: Zeroizing::new(password),
                },
                grant,
            )
        };
        let image = self
            .images(&self.service(engine)?, &runner.target)?
            .remove(0);
        let verified =
            bindings::verify_binding(runner, &binding, &credentials, Some(&image), context).await?;
        let mut grant = grant;
        if managed {
            grant.server_fingerprint = verified.view().server_fingerprint.clone();
        }
        let mut authorized = bindings::authorize_binding(&verified, &grant)?;
        if managed {
            authorized.binding.lifecycle_owner = "Managed".into();
        }
        let mut inherited = BTreeMap::new();
        match service.id.as_str() {
            "xxjob" | "seata" => {
                inherited.insert("MYSQL_HOST".into(), authorized.network_alias.clone());
                inherited.insert("MYSQL_PORT".into(), authorized.container_port.to_string());
                inherited.insert("MYSQL_DATABASE".into(), scoped_databases[0].clone());
                inherited.insert("MYSQL_USERNAME".into(), credentials.username.clone());
                inherited.insert("MYSQL_PASSWORD".into(), credentials.password.to_string());
            }
            "temporal" => {
                inherited.insert(
                    "TEMPORAL_POSTGRES_HOST".into(),
                    authorized.network_alias.clone(),
                );
                inherited.insert(
                    "PGSQL_DOCKER_NETWORK".into(),
                    authorized.network_name.clone(),
                );
                inherited.insert("PGSQL_USER".into(), credentials.username.clone());
                inherited.insert("PGSQL_PASSWORD".into(), credentials.password.to_string());
            }
            "canal" => {
                if authorized
                    .allowed_databases
                    .iter()
                    .any(|d| !bindings::valid_database_name(d))
                {
                    return Err(error("InitializationGrantOutOfScope"));
                }
                inherited.insert(
                    "CANAL_MYSQL_ADDRESS".into(),
                    format!("{}:{}", authorized.network_alias, authorized.container_port),
                );
                inherited.insert(
                    "CANAL_FILTER_REGEX".into(),
                    format!("({})[.].*", authorized.allowed_databases.join("|")),
                );
                let (username, password) = if managed {
                    let path = store.stack(&service.id)?.join(".env");
                    reject_links(&path)?;
                    let current = if path.exists() {
                        SecretValues(crate::env_store::parse_env(&Zeroizing::new(
                            std::fs::read_to_string(path).map_err(|_| error("StateUnavailable"))?,
                        ))?)
                    } else {
                        SecretValues::default()
                    };
                    let username = format!("ca_{}_canal", &instance.identity.instance_id[..12]);
                    if current
                        .0
                        .get("CANAL_MYSQL_USER")
                        .is_some_and(|value| value != &username)
                    {
                        return Err(error("CredentialRepairRequired"));
                    }
                    let password = current
                        .0
                        .get("CANAL_MYSQL_PASSWORD")
                        .cloned()
                        .unwrap_or(crate::env_store::generate_secret("alphanumeric32")?);
                    (username, password)
                } else {
                    (
                        credentials.username.clone(),
                        credentials.password.to_string(),
                    )
                };
                if password.len() > 32 {
                    return Err(error("UnsupportedCredentialValue"));
                }
                inherited.insert("CANAL_MYSQL_USER".into(), username);
                inherited.insert("CANAL_MYSQL_PASSWORD".into(), password);
            }
            _ => return Err(error("UnsupportedInitializationSchema")),
        }
        instance.bindings.retain(|b| b.engine != *engine);
        instance.bindings.push(authorized.binding.clone());
        store.save(instance)?;
        Ok(Some(Dependency {
            binding: authorized,
            credentials,
            image,
            inherited: SecretValues(inherited),
            managed,
        }))
    }

    pub(crate) async fn initialize_dependency(
        &self,
        runner: &crate::docker_cli::DockerRunner,
        service: &ServiceSpec,
        instance: &mut RuntimeInstance,
        setup: &Dependency,
        secrets: &SecretValues,
        context: &ImageContext,
    ) -> Result<(), ToolboxError> {
        let stack = self.store()?.stack(&service.id)?;
        let names = setup.binding.allowed_databases.clone();
        let _locks = crate::locks::acquire(
            &self.control.join("locks"),
            &names
                .iter()
                .map(|name| format!("database/{}/{name}", setup.binding.server_fingerprint))
                .collect::<Vec<_>>(),
        )?;
        if service.id == "canal" {
            return initialize_canal(runner, instance, setup, secrets, context, &stack).await;
        }
        let claim = if let Some(claim) = instance.claims.iter().find(|c| {
            c.server_fingerprint == setup.binding.server_fingerprint
                && c.database_names == names
                && c.owner_instance_id == instance.identity.instance_id
        }) {
            let mut claim = claim.clone();
            claim.binding_id = setup.binding.binding_id.clone();
            claim
        } else {
            let claim = crate::database_claims::plan_database_claim(
                &setup.binding,
                &instance.identity,
                &names,
            )?
            .claim;
            instance.claims.push(claim.clone());
            self.store()?.save(instance)?;
            claim
        };
        let claim = if service.id == "temporal" {
            initialize_temporal(runner, setup, claim, context).await?
        } else {
            let plan =
                crate::sql_provisioner::service_plan(service, &setup.binding, claim, secrets)?;
            let mut database = crate::sql_provisioner::SqlInitializationDatabase::new(
                runner,
                &setup.binding,
                &setup.credentials,
                &setup.image,
                context,
                &plan,
            )?;
            crate::provisioner::execute_initialization(
                &plan,
                &mut database,
                &crate::initialization_store::InitializationStore::new(
                    &stack.join(format!(".initialization-{}.json", plan.claim.claim_id)),
                ),
            )
            .await?
        };
        instance.claims.retain(|c| c.claim_id != claim.claim_id);
        instance.claims.push(claim);
        self.store()?.save(instance)?;
        Ok(())
    }
}

async fn query(
    runner: &crate::docker_cli::DockerRunner,
    setup: &Dependency,
    context: &ImageContext,
    database: Option<&str>,
    sql: &str,
) -> Result<String, ToolboxError> {
    runner.verify_identity().await?;
    let raw = runner
        .run_read(&["container", "inspect", &setup.binding.container_id])
        .await?;
    let raw: Value = serde_json::from_slice(&raw).map_err(|_| error("DockerOutputInvalid"))?;
    bindings::binding_topology(&setup.binding, &runner.target, &raw[0])?;
    bindings::network_query_database(
        runner,
        &setup.binding,
        &setup.credentials,
        &setup.image,
        context,
        database,
        sql,
    )
    .await
}

async fn initialize_temporal(
    runner: &crate::docker_cli::DockerRunner,
    setup: &Dependency,
    mut claim: DatabaseClaim,
    context: &ImageContext,
) -> Result<DatabaseClaim, ToolboxError> {
    if !valid_id(&claim.claim_id)
        || !valid_id(&claim.owner_instance_id)
        || claim.database_names != ["temporal", "temporal_visibility"]
        || claim.server_fingerprint != setup.binding.server_fingerprint
    {
        return Err(error("RecoveryRequired"));
    }
    for name in &claim.database_names {
        if !bindings::valid_database_name(name) {
            return Err(error("InitializationGrantOutOfScope"));
        }
        let exists = query(runner, setup, context, None, &format!("SELECT CASE WHEN EXISTS(SELECT 1 FROM pg_database WHERE datname='{name}') THEN '1' ELSE '0' END")).await?;
        if exists == "0" {
            query(
                runner,
                setup,
                context,
                None,
                &format!("CREATE DATABASE \"{name}\""),
            )
            .await?;
            query(runner, setup, context, Some(name), &format!("CREATE TABLE public.codeark_resource_owner(marker_id INTEGER PRIMARY KEY,claim_id VARCHAR(64) NOT NULL,owner_instance_id VARCHAR(64) NOT NULL); INSERT INTO public.codeark_resource_owner VALUES(1,'{}','{}');", claim.claim_id, claim.owner_instance_id)).await?;
        } else if exists != "1" {
            return Err(error("DatabaseOutputInvalid"));
        }
        let marker = query(
            runner,
            setup,
            context,
            Some(name),
            "SELECT COALESCE(to_regclass('public.codeark_resource_owner')::text,'')",
        )
        .await?;
        if marker.is_empty() {
            return Err(error("RecoveryRequired"));
        }
        let marker = query(runner, setup, context, Some(name), "SELECT json_build_array(claim_id,owner_instance_id)::text FROM public.codeark_resource_owner WHERE marker_id=1").await?;
        let marker: Vec<String> =
            serde_json::from_str(&marker).map_err(|_| error("RecoveryRequired"))?;
        if marker.len() != 2 {
            return Err(error("RecoveryRequired"));
        }
        crate::database_claims::verify_claim_evidence(
            &claim,
            &setup.binding.server_fingerprint,
            &marker[0],
            &marker[1],
        )?;
    }
    claim.created_by_codeark = true;
    claim.status = "DatabaseCreated".into();
    claim.ddl_version = "temporal-1.32.2".into();
    use sha2::{Digest, Sha256};
    claim.ddl_checksum = format!(
        "{:x}",
        Sha256::digest(include_bytes!(
            "../../../services/temporal-start-local/scripts/setup-postgres.sh"
        ))
    );
    Ok(claim)
}

#[derive(Serialize, Deserialize)]
struct ReplicaAccount {
    owner: String,
    server: String,
    username: String,
    databases: Vec<String>,
    completed: bool,
}
async fn initialize_canal(
    runner: &crate::docker_cli::DockerRunner,
    instance: &RuntimeInstance,
    setup: &Dependency,
    secrets: &SecretValues,
    context: &ImageContext,
    stack: &Path,
) -> Result<(), ToolboxError> {
    if query(
        runner,
        setup,
        context,
        None,
        "SELECT IF(@@log_bin=1 AND @@binlog_format='ROW','1','0')",
    )
    .await?
        != "1"
    {
        return Err(error("CanalBinlogRequired"));
    }
    if !setup.managed {
        let grants = query(
            runner,
            setup,
            context,
            None,
            "SHOW GRANTS FOR CURRENT_USER()",
        )
        .await?;
        if !canal_grants_allow_scope(&grants, &setup.binding.allowed_databases) {
            return Err(error("CanalReplicationGrantRequired"));
        }
        return Ok(());
    }
    let username = required(secrets, "CANAL_MYSQL_USER")?;
    let password = Zeroizing::new(required(secrets, "CANAL_MYSQL_PASSWORD")?);
    if setup.binding.grant.allowed_accounts != [username.clone()] {
        return Err(error("InitializationGrantOutOfScope"));
    }
    if username != format!("ca_{}_canal", &instance.identity.instance_id[..12])
        || password.is_empty()
        || password.len() > 32
        || !password.bytes().all(|b| b.is_ascii_alphanumeric())
    {
        return Err(error("UnsupportedCredentialValue"));
    }
    let path = stack.join(".replication-account.json");
    let previous: Option<ReplicaAccount> = if path.exists() {
        Some(read_json(&path)?)
    } else {
        None
    };
    if previous.as_ref().is_some_and(|p| {
        p.owner != instance.identity.instance_id
            || p.server != setup.binding.server_fingerprint
            || p.username != username
            || p.databases != setup.binding.allowed_databases
    }) {
        return Err(error("RecoveryRequired"));
    }
    let exists = query(
        runner,
        setup,
        context,
        None,
        &format!(
            "SELECT CAST(COUNT(*) AS CHAR) FROM mysql.user WHERE User='{username}' AND Host='%'"
        ),
    )
    .await?;
    if exists == "1" {
        if !previous.as_ref().is_some_and(|p| p.completed) {
            return Err(error("RecoveryRequired"));
        }
        let login = Credentials { username, password };
        bindings::verify_binding(runner, &setup.binding, &login, Some(&setup.image), context)
            .await?;
        let grants = bindings::network_query(
            runner,
            &setup.binding,
            &login,
            &setup.image,
            context,
            "SHOW GRANTS FOR CURRENT_USER()",
        )
        .await?;
        if !canal_grants_allow_scope(&grants, &setup.binding.allowed_databases) {
            return Err(error("CanalReplicationGrantRequired"));
        }
        return Ok(());
    }
    if exists != "0" || previous.as_ref().is_some_and(|p| p.completed) {
        return Err(error("RecoveryRequired"));
    }
    let partial_revokes = match query(
        runner,
        setup,
        context,
        None,
        "SELECT CAST(@@partial_revokes AS CHAR)",
    )
    .await?
    .as_str()
    {
        "0" => false,
        "1" => true,
        _ => return Err(error("DatabaseOutputInvalid")),
    };
    let mut record = ReplicaAccount {
        owner: instance.identity.instance_id.clone(),
        server: setup.binding.server_fingerprint.clone(),
        username: username.clone(),
        databases: setup.binding.allowed_databases.clone(),
        completed: false,
    };
    atomic_write(
        &path,
        &serde_json::to_vec(&record).map_err(|_| error("StateWriteFailed"))?,
    )?;
    query(runner, setup, context, None, &format!("CREATE USER '{username}'@'%' IDENTIFIED WITH mysql_native_password BY '{}'; GRANT REPLICATION SLAVE, REPLICATION CLIENT ON *.* TO '{username}'@'%';", password.as_str())).await?;
    for database in &record.databases {
        let scope = bindings::mysql_database_grant_scope(database, partial_revokes)?;
        query(
            runner,
            setup,
            context,
            None,
            &format!("GRANT SELECT ON {scope} TO '{username}'@'%'"),
        )
        .await?;
    }
    record.completed = true;
    atomic_write(
        &path,
        &serde_json::to_vec(&record).map_err(|_| error("StateWriteFailed"))?,
    )?;
    Ok(())
}

fn canal_grants_allow_scope(grants: &str, databases: &[String]) -> bool {
    bindings::mysql_grants_allow(grants, "REPLICATION SLAVE", None)
        && bindings::mysql_grants_allow(grants, "REPLICATION CLIENT", None)
        && databases
            .iter()
            .all(|database| bindings::mysql_grants_allow(grants, "SELECT", Some(database)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(service: &str) -> InstanceIdentity {
        InstanceIdentity {
            workspace_id: "workspace".into(),
            service_id: service.into(),
            instance_id: "0123456789abcdef0123456789abcdef".into(),
            project_name: format!("ca-workspace-{service}"),
            target_id: "target".into(),
            daemon_id: "daemon".into(),
        }
    }

    #[test]
    fn database_names_are_instance_scoped_but_temporal_uses_its_fixed_pair() {
        assert_eq!(
            databases("xxjob", &identity("xxjob")).unwrap(),
            ["ca_0123456789ab_xxjob"]
        );
        assert_eq!(
            databases("seata", &identity("seata")).unwrap(),
            ["ca_0123456789ab_seata"]
        );
        assert_eq!(
            databases("temporal", &identity("temporal")).unwrap(),
            ["temporal", "temporal_visibility"]
        );
        let mut other = identity("xxjob");
        other.instance_id.replace_range(..1, "f");
        assert_ne!(
            databases("xxjob", &identity("xxjob")).unwrap(),
            databases("xxjob", &other).unwrap()
        );
    }

    #[test]
    fn database_derivation_rejects_truncated_or_sql_bearing_identity_and_service_mismatch() {
        for invalid in [
            "0123456789ab",
            "0123456789ab';DROP DATABASE x;--xx",
            "0123456789ABCDEF0123456789ABCDEF",
            "éééééééééééééééé",
        ] {
            let mut i = identity("temporal");
            i.instance_id = invalid.into();
            assert_eq!(
                databases("temporal", &i).unwrap_err().code,
                "IdentityMismatch"
            );
        }
        assert!(databases("xxjob", &identity("seata")).is_err());
        assert!(databases("xxjob`", &identity("xxjob`")).is_err());
    }

    #[test]
    fn explicit_binding_requires_exact_named_databases_and_operation() {
        let names = databases("seata", &identity("seata")).unwrap();
        let binding = DatabaseBinding {
            binding_id: "binding".into(),
            target_id: "target".into(),
            daemon_id: "daemon".into(),
            engine: "mysql".into(),
            lifecycle_owner: "ExternalConnected".into(),
            container_id: "container".into(),
            network_name: "network".into(),
            network_alias: "mysql".into(),
            container_port: 3306,
            host_endpoint: None,
            credential_ref: SecretRef {
                key: "secret".into(),
            },
            allowed_databases: names.clone(),
            grant: "InitializeNamedResources".into(),
            server_fingerprint: "server".into(),
        };
        let grant = InitializationGrant {
            target_id: binding.target_id.clone(),
            server_fingerprint: binding.server_fingerprint.clone(),
            allowed_databases: names.clone(),
            allowed_accounts: vec![],
            operations: vec!["InitializeNamedResources".into()],
            confirmed_at: 1,
        };
        assert!(check_grant_scope("seata", &binding, &grant, &names).is_ok());
        let mut wrong = grant.clone();
        wrong.allowed_databases.push("existing_business".into());
        assert!(check_grant_scope("seata", &binding, &wrong, &names).is_err());
        wrong = grant.clone();
        wrong.operations = vec!["ObserveReplication".into()];
        assert!(check_grant_scope("seata", &binding, &wrong, &names).is_err());
        wrong = grant.clone();
        wrong.allowed_accounts = vec!["unrelated_account".into()];
        assert!(check_grant_scope("seata", &binding, &wrong, &names).is_err());
        assert!(check_grant_scope("canal", &binding, &grant, &names).is_err());
    }

    #[test]
    fn canal_requires_global_replication_and_select_on_every_selected_database() {
        let names = vec!["orders".into(), "billing".into()];
        let grants = "GRANT REPLICATION SLAVE, REPLICATION CLIENT ON *.* TO `replica`@`%`\nGRANT SELECT ON `orders`.* TO `replica`@`%`\nGRANT SELECT ON `billing`.* TO `replica`@`%`";
        assert!(canal_grants_allow_scope(grants, &names));
        assert!(canal_grants_allow_scope(
            "GRANT ALL PRIVILEGES ON *.* TO `root`@`%` WITH GRANT OPTION",
            &names
        ));
        assert!(!canal_grants_allow_scope(
            grants,
            &["other_business".into()]
        ));
        assert!(!canal_grants_allow_scope(
            "GRANT REPLICATION SLAVE, REPLICATION CLIENT ON *.* TO `replica`@`%`",
            &names
        ));
        assert!(!canal_grants_allow_scope(
            "GRANT SELECT ON `orders`.* TO `REPLICATION SLAVE REPLICATION CLIENT`@`%`",
            &["orders".into()]
        ));
        assert!(!canal_grants_allow_scope(
            "GRANT SELECT ON `ALL PRIVILEGES ON *.*`.* TO `replica`@`%`",
            &names
        ));
    }

    #[test]
    fn canal_database_grants_do_not_expand_underscore_wildcards() {
        assert_eq!(
            bindings::mysql_database_grant_scope("ca_jobs", false).unwrap(),
            "`ca\\_jobs`.*"
        );
        assert_eq!(
            bindings::mysql_database_grant_scope("ca_jobs", true).unwrap(),
            "`ca_jobs`.*"
        );
        assert!(bindings::mysql_database_grant_scope("ca_jobs`.* TO 'other'", false).is_err());
        let grants = "GRANT REPLICATION SLAVE, REPLICATION CLIENT ON *.* TO `replica`@`%`\nGRANT SELECT ON `ca\\_jobs`.* TO `replica`@`%`";
        assert!(canal_grants_allow_scope(grants, &["ca_jobs".into()]));
        assert!(!canal_grants_allow_scope(grants, &["caxjobs".into()]));
        assert!(!canal_grants_allow_scope(
            "GRANT ALL PRIVILEGES ON *.* TO `replica`@`%`\nREVOKE SELECT ON `ca_jobs`.* FROM `replica`@`%`",
            &["ca_jobs".into()]
        ));
    }
}
