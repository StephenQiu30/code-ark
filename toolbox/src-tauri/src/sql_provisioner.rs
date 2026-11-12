use crate::{
    bindings::AuthorizedBinding, contracts::*, provisioner::InitializationPlan, workspace::error,
};
pub fn service_plan(
    service: &ServiceSpec,
    binding: &AuthorizedBinding,
    mut claim: DatabaseClaim,
    secrets: &crate::env_store::SecretValues,
) -> Result<InitializationPlan, ToolboxError> {
    use sha2::{Digest, Sha256};
    if binding.engine != "mysql"
        || claim.binding_id != binding.binding_id
        || claim.server_fingerprint != binding.server_fingerprint
        || claim
            .database_names
            .iter()
            .any(|n| !binding.grant.allowed_databases.contains(n))
    {
        return Err(error("InitializationGrantOutOfScope"));
    }
    let (source, expected, version) = match service.id.as_str() {
        "xxjob" => (
            include_str!("../../../services/xxjob-start-local/tables_xxl_job.sql"),
            "8f29210cfb49800daa93408395159f42f12d76d88cee35f70e7cc59838d7c0a3",
            "xxl-job-3.5.0",
        ),
        "seata" => (
            include_str!("../resources/sql/seata_1_8_0_mysql.sql"),
            "9313cf0d0e9fd117d3eb2902c1a2174f67f01a2adffbc40026d83720dd54919f",
            "seata-1.8.0",
        ),
        _ => return Err(error("UnsupportedInitializationSchema")),
    };
    let checksum = format!("{:x}", Sha256::digest(source));
    if checksum != expected {
        return Err(error("SchemaSourceMismatch"));
    }
    let mut steps = crate::provisioner::parse_schema_steps(source)?;
    if service.id == "xxjob" {
        let password = secrets
            .0
            .get("XXL_ADMIN_PASSWORD")
            .filter(|s| !s.is_empty())
            .ok_or_else(|| error("MissingCredential"))?;
        let token = secrets
            .0
            .get("XXL_JOB_ACCESS_TOKEN")
            .filter(|s| !s.is_empty())
            .ok_or_else(|| error("MissingCredential"))?;
        for step in &mut steps {
            if step.kind != "seed" {
                continue;
            }
            if step.object_name == "xxl_job_user" {
                step.statement = step.statement.replace(
                    "8d969eef6ecad3c29a3a629280e686cf0c3f5d5a86aff3ca12020c923adc6c92",
                    &format!("{:x}", Sha256::digest(password)),
                );
            }
            step.statement = step
                .statement
                .replace("'default_token'", &mysql_text(token));
        }
    }
    claim.ddl_version = version.into();
    claim.ddl_checksum = checksum;
    let mut hash = Sha256::new();
    hash.update(claim.claim_id.as_bytes());
    for step in &steps {
        hash.update(step.statement.as_bytes());
        hash.update([0]);
    }
    Ok(InitializationPlan {
        claim,
        plan_digest: format!("{:x}", hash.finalize()),
        steps,
    })
}
fn mysql_text(value: &str) -> String {
    format!(
        "CONVERT(0x{} USING utf8mb4)",
        value
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}

/// Scoped driver accepts backend-issued grants and only the checksummed plan statements.
/// Queries travel through a short-lived locked SQL client on the selected Docker network.
pub struct SqlInitializationDatabase<'a> {
    runner: &'a crate::docker_cli::DockerRunner,
    binding: &'a AuthorizedBinding,
    credentials: &'a crate::bindings::Credentials,
    image: &'a ImageArtifactRef,
    context: &'a crate::image_store::ImageContext,
    claim: &'a DatabaseClaim,
    steps: &'a [crate::provisioner::InitializationStep],
}
impl<'a> SqlInitializationDatabase<'a> {
    pub fn new(
        runner: &'a crate::docker_cli::DockerRunner,
        binding: &'a AuthorizedBinding,
        credentials: &'a crate::bindings::Credentials,
        image: &'a ImageArtifactRef,
        context: &'a crate::image_store::ImageContext,
        plan: &'a InitializationPlan,
    ) -> Result<Self, ToolboxError> {
        if binding.engine != "mysql"
            || binding.target_id != runner.target.target_id
            || binding.daemon_id != runner.target.daemon_id
            || plan.claim.binding_id != binding.binding_id
            || plan.claim.server_fingerprint != binding.server_fingerprint
            || plan
                .claim
                .database_names
                .iter()
                .any(|n| !binding.grant.allowed_databases.contains(n))
        {
            return Err(error("InitializationGrantOutOfScope"));
        }
        Ok(Self {
            runner,
            binding,
            credentials,
            image,
            context,
            claim: &plan.claim,
            steps: &plan.steps,
        })
    }
    fn scope(&self, database: &str) -> Result<(), ToolboxError> {
        if !crate::bindings::valid_database_name(database)
            || !self.claim.database_names.iter().any(|n| n == database)
        {
            Err(error("InitializationGrantOutOfScope"))
        } else {
            Ok(())
        }
    }
    async fn query(&self, sql: &str) -> Result<String, ToolboxError> {
        self.runner.verify_identity().await?;
        let raw = self
            .runner
            .run_read(&["container", "inspect", &self.binding.container_id])
            .await?;
        let raw: serde_json::Value =
            serde_json::from_slice(&raw).map_err(|_| error("DockerOutputInvalid"))?;
        crate::bindings::binding_topology(self.binding, &self.runner.target, &raw[0])?;
        crate::bindings::network_query(
            self.runner,
            self.binding,
            self.credentials,
            self.image,
            self.context,
            sql,
        )
        .await
    }
    fn checked_step(
        &self,
        step: &crate::provisioner::InitializationStep,
    ) -> Result<(), ToolboxError> {
        if self.steps.iter().any(|s| {
            s.id == step.id
                && s.statement == step.statement
                && s.object_name == step.object_name
                && s.kind == step.kind
        }) {
            Ok(())
        } else {
            Err(error("SchemaSourceMismatch"))
        }
    }
}
impl crate::provisioner::InitializationDatabase for SqlInitializationDatabase<'_> {
    async fn identity(&mut self) -> Result<String, ToolboxError> {
        self.query("SELECT @@server_uuid").await.map(|id| {
            crate::bindings::server_fingerprint("mysql", &self.runner.target.daemon_id, &id)
        })
    }
    async fn database_exists(&mut self, database: &str) -> Result<bool, ToolboxError> {
        self.scope(database)?;
        Ok(self
            .query(&format!(
                "SELECT COUNT(*) FROM information_schema.schemata WHERE schema_name={}",
                mysql_text(database)
            ))
            .await?
            == "1")
    }
    async fn create_database(&mut self, database: &str) -> Result<(), ToolboxError> {
        self.scope(database)?;
        self.query(&format!(
            "CREATE DATABASE `{database}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci"
        ))
        .await?;
        Ok(())
    }
    async fn owner(&mut self, database: &str) -> Result<Option<(String, String)>, ToolboxError> {
        self.scope(database)?;
        let exists=self.query(&format!("SELECT COUNT(*) FROM information_schema.tables WHERE table_schema={} AND table_name='codeark_resource_owner'",mysql_text(database))).await?;
        if exists == "0" {
            return Ok(None);
        }
        if exists != "1" {
            return Err(error("RecoveryRequired"));
        }
        let row=self.query(&format!("SELECT JSON_ARRAY(claim_id,owner_instance_id) FROM `{database}`.codeark_resource_owner WHERE marker_id=1")).await?;
        let pair: Vec<String> =
            serde_json::from_str(&row).map_err(|_| error("RecoveryRequired"))?;
        if pair.len() != 2 || pair.iter().any(|s| s.is_empty()) {
            return Err(error("RecoveryRequired"));
        }
        Ok(Some((pair[0].clone(), pair[1].clone())))
    }
    async fn write_owner(
        &mut self,
        database: &str,
        claim: &DatabaseClaim,
    ) -> Result<(), ToolboxError> {
        self.scope(database)?;
        if claim != self.claim {
            return Err(error("RecoveryRequired"));
        }
        self.query(&format!("CREATE TABLE `{database}`.codeark_resource_owner(marker_id INT NOT NULL PRIMARY KEY, claim_id VARCHAR(64) NOT NULL,owner_instance_id VARCHAR(64) NOT NULL); INSERT INTO `{database}`.codeark_resource_owner VALUES(1,{},{});",mysql_text(&claim.claim_id),mysql_text(&claim.owner_instance_id))).await?;
        Ok(())
    }
    async fn observe(
        &mut self,
        database: &str,
        step: &crate::provisioner::InitializationStep,
    ) -> Result<crate::provisioner::ObservedStep, ToolboxError> {
        use crate::provisioner::ObservedStep::*;
        self.scope(database)?;
        self.checked_step(step)?;
        if step.kind == "table" {
            let raw=self.query(&format!("SELECT JSON_ARRAY(column_name,LOWER(column_type),is_nullable,column_key) FROM information_schema.columns WHERE table_schema={} AND table_name={} ORDER BY ordinal_position",mysql_text(database),mysql_text(&step.object_name))).await?;
            if raw.is_empty() {
                return Ok(Missing);
            }
            let expected = crate::sql_schema::expected_columns(step)?;
            let actual = raw.lines().collect::<Vec<_>>();
            if actual.len() != expected.len() {
                return Ok(Incompatible);
            }
            for (row, column) in actual.iter().zip(expected) {
                let fields: Vec<String> =
                    serde_json::from_str(row).map_err(|_| error("DatabaseOutputInvalid"))?;
                if fields.len() != 4
                    || fields[0] != column.name
                    || crate::sql_schema::normalize_type(&fields[1]) != column.kind
                    || (fields[2] == "YES") != column.nullable
                    || (fields[3] == "PRI") != column.primary
                {
                    return Ok(Incompatible);
                }
            }
            Ok(Compatible)
        } else if step.kind == "seed" {
            let table = self
                .steps
                .iter()
                .find(|s| s.kind == "table" && s.object_name == step.object_name)
                .ok_or_else(|| error("InvalidSchema"))?;
            let (key, value) = crate::sql_schema::seed_key(step, table)?;
            // A pre-existing seed key is retained even when users edited its other values.
            let count = self
                .query(&format!(
                    "SELECT COUNT(*) FROM `{database}`.`{}` WHERE `{key}`={value}",
                    step.object_name
                ))
                .await?;
            match count.as_str() {
                "0" => Ok(Missing),
                "1" => Ok(Compatible),
                _ => Ok(Incompatible),
            }
        } else {
            Err(error("UnsupportedSchemaStatement"))
        }
    }
    async fn apply(
        &mut self,
        database: &str,
        step: &crate::provisioner::InitializationStep,
    ) -> Result<(), ToolboxError> {
        self.scope(database)?;
        self.checked_step(step)?;
        if crate::bindings::server_fingerprint(
            "mysql",
            &self.runner.target.daemon_id,
            &self.query("SELECT @@server_uuid").await?,
        ) != self.claim.server_fingerprint
        {
            return Err(error("ServerIdentityMismatch"));
        }
        self.query(&format!("USE `{database}`; {}", step.statement))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_schema_is_locked_and_admin_seed_uses_instance_secret() {
        let catalog = crate::catalog::load_embedded_catalog().unwrap();
        let service = catalog.services.iter().find(|s| s.id == "xxjob").unwrap();
        let binding = AuthorizedBinding {
            binding: DatabaseBinding {
                binding_id: "b".into(),
                target_id: "t".into(),
                daemon_id: "d".into(),
                engine: "mysql".into(),
                lifecycle_owner: "Managed".into(),
                container_id: "c".into(),
                network_name: "n".into(),
                network_alias: "mysql".into(),
                container_port: 3306,
                host_endpoint: None,
                credential_ref: SecretRef { key: "k".into() },
                allowed_databases: vec!["ca_jobs".into()],
                grant: "InitializeNamedResources".into(),
                server_fingerprint: "server".into(),
            },
            grant: InitializationGrant {
                target_id: "t".into(),
                server_fingerprint: "server".into(),
                allowed_databases: vec!["ca_jobs".into()],
                allowed_accounts: vec![],
                operations: vec!["InitializeNamedResources".into()],
                confirmed_at: 0,
            },
        };
        let claim = DatabaseClaim {
            claim_id: "claim".into(),
            binding_id: "b".into(),
            daemon_id: "d".into(),
            server_fingerprint: "server".into(),
            database_names: vec!["ca_jobs".into()],
            owner_instance_id: "owner".into(),
            created_by_codeark: false,
            ddl_version: "".into(),
            ddl_checksum: "".into(),
            status: "Reserved".into(),
        };
        let secret = crate::env_store::SecretValues(std::collections::BTreeMap::from([
            (
                "XXL_ADMIN_PASSWORD".into(),
                "random-private-password".into(),
            ),
            ("XXL_JOB_ACCESS_TOKEN".into(), "private-token".into()),
        ]));
        let plan = service_plan(service, &binding, claim, &secret).unwrap();
        assert!(!plan.claim.ddl_checksum.is_empty());
        let user = plan
            .steps
            .iter()
            .find(|s| s.kind == "seed" && s.object_name == "xxl_job_user")
            .unwrap();
        assert!(!user.statement.contains("8d969eef"));
        assert!(!user.statement.contains("random-private-password"));
    }
}
