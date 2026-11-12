use crate::contracts::*;
use crate::initialization_store::*;
use crate::workspace::error;
use std::future::Future;
#[derive(Clone, PartialEq)]
pub enum ObservedStep {
    Missing,
    Compatible,
    Incompatible,
}
#[derive(Clone)]
pub struct InitializationStep {
    pub id: String,
    pub object_name: String,
    pub statement: String,
    pub kind: String,
}
pub struct InitializationPlan {
    pub claim: DatabaseClaim,
    pub plan_digest: String,
    pub steps: Vec<InitializationStep>,
}
pub trait InitializationDatabase: Send {
    fn identity(&mut self) -> impl Future<Output = Result<String, ToolboxError>> + Send;
    fn database_exists(
        &mut self,
        database: &str,
    ) -> impl Future<Output = Result<bool, ToolboxError>> + Send;
    fn create_database(
        &mut self,
        database: &str,
    ) -> impl Future<Output = Result<(), ToolboxError>> + Send;
    fn owner(
        &mut self,
        database: &str,
    ) -> impl Future<Output = Result<Option<(String, String)>, ToolboxError>> + Send;
    fn write_owner(
        &mut self,
        database: &str,
        claim: &DatabaseClaim,
    ) -> impl Future<Output = Result<(), ToolboxError>> + Send;
    fn observe(
        &mut self,
        database: &str,
        step: &InitializationStep,
    ) -> impl Future<Output = Result<ObservedStep, ToolboxError>> + Send;
    fn apply(
        &mut self,
        database: &str,
        step: &InitializationStep,
    ) -> impl Future<Output = Result<(), ToolboxError>> + Send;
}
pub async fn execute_initialization<D: InitializationDatabase>(
    plan: &InitializationPlan,
    database: &mut D,
    store: &InitializationStore,
) -> Result<DatabaseClaim, ToolboxError> {
    let identity = database.identity().await?;
    if identity != plan.claim.server_fingerprint {
        return Err(error("ServerIdentityMismatch"));
    }
    if plan.claim.database_names.is_empty()
        || !plan
            .claim
            .database_names
            .iter()
            .all(|d| crate::bindings::valid_database_name(d))
    {
        return Err(error("InitializationGrantOutOfScope"));
    }
    let mut journal = if let Some(journal) = store.load()? {
        if journal.schema_version != 1
            || journal.claim_id != plan.claim.claim_id
            || journal.plan_digest != plan.plan_digest
        {
            return Err(error("SchemaIncompatible"));
        }
        journal
    } else {
        InitializationJournal {
            schema_version: 1,
            claim_id: plan.claim.claim_id.clone(),
            plan_digest: plan.plan_digest.clone(),
            steps: std::collections::BTreeMap::new(),
            phase: "Running".into(),
            error_code: None,
        }
    };
    store.persist(&journal)?;
    let result = async {
        for name in &plan.claim.database_names {
            // Persist intent first. An existing database without its marker is never adopted,
            // including after a crash between CREATE DATABASE and marker creation.
            if database.database_exists(name).await? {
                let owner = database
                    .owner(name)
                    .await?
                    .ok_or_else(|| error("RecoveryRequired"))?;
                crate::database_claims::verify_claim_evidence(
                    &plan.claim,
                    &identity,
                    &owner.0,
                    &owner.1,
                )?;
            } else {
                let key = format!("{name}/create");
                journal.steps.insert(key.clone(), "Running".into());
                store.persist(&journal)?;
                database.create_database(name).await?;
                database.write_owner(name, &plan.claim).await?;
                let marker = database
                    .owner(name)
                    .await?
                    .ok_or_else(|| error("RecoveryRequired"))?;
                crate::database_claims::verify_claim_evidence(
                    &plan.claim,
                    &identity,
                    &marker.0,
                    &marker.1,
                )?;
                journal.steps.insert(key, "Completed".into());
                store.persist(&journal)?;
            }
            for step in &plan.steps {
                if database.identity().await? != identity {
                    return Err(error("ServerIdentityMismatch"));
                }
                let key = format!("{name}/{}", step.id);
                match database.observe(name, step).await? {
                    ObservedStep::Incompatible => return Err(error("SchemaIncompatible")),
                    ObservedStep::Compatible => {
                        journal.steps.insert(key, "Completed".into());
                        store.persist(&journal)?;
                        continue;
                    }
                    ObservedStep::Missing => {
                        if journal.steps.get(&key).is_some_and(|s| s == "Completed") {
                            return Err(error("RecoveryRequired"));
                        }
                    }
                }
                journal.steps.insert(key.clone(), "Running".into());
                store.persist(&journal)?;
                database.apply(name, step).await?;
                if database.observe(name, step).await? != ObservedStep::Compatible {
                    return Err(error("InitializationPartial"));
                }
                journal.steps.insert(key, "Completed".into());
                store.persist(&journal)?;
            }
        }
        Ok(())
    }
    .await;
    if let Err(e) = result {
        journal.phase = if e.code == "RecoveryRequired" {
            "RecoveryRequired"
        } else {
            "Partial"
        }
        .into();
        journal.error_code = Some(e.code.clone());
        store.persist(&journal)?;
        return Err(e);
    }
    journal.phase = "Completed".into();
    journal.error_code = None;
    store.persist(&journal)?;
    let mut claim = plan.claim.clone();
    claim.created_by_codeark = true;
    claim.status = "Initialized".into();
    Ok(claim)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    struct Database {
        exists: bool,
        owner: Option<(String, String)>,
        tables: BTreeMap<String, String>,
        fail_on: Option<String>,
    }
    impl InitializationDatabase for Database {
        async fn identity(&mut self) -> Result<String, ToolboxError> {
            Ok("server-a".into())
        }
        async fn database_exists(&mut self, _: &str) -> Result<bool, ToolboxError> {
            Ok(self.exists)
        }
        async fn create_database(&mut self, _: &str) -> Result<(), ToolboxError> {
            self.exists = true;
            Ok(())
        }
        async fn owner(&mut self, _: &str) -> Result<Option<(String, String)>, ToolboxError> {
            Ok(self.owner.clone())
        }
        async fn write_owner(
            &mut self,
            _: &str,
            claim: &DatabaseClaim,
        ) -> Result<(), ToolboxError> {
            self.owner = Some((claim.claim_id.clone(), claim.owner_instance_id.clone()));
            Ok(())
        }
        async fn observe(
            &mut self,
            _: &str,
            step: &InitializationStep,
        ) -> Result<ObservedStep, ToolboxError> {
            Ok(match self.tables.get(&step.object_name) {
                None => ObservedStep::Missing,
                Some(value) if value == "compatible" => ObservedStep::Compatible,
                _ => ObservedStep::Incompatible,
            })
        }
        async fn apply(&mut self, _: &str, step: &InitializationStep) -> Result<(), ToolboxError> {
            if self.fail_on.as_deref() == Some(&step.object_name) {
                return Err(error("InitializationPartial"));
            }
            self.tables
                .insert(step.object_name.clone(), "compatible".into());
            Ok(())
        }
    }
    fn plan() -> InitializationPlan {
        InitializationPlan {
            claim: DatabaseClaim {
                claim_id: "claim-a".into(),
                binding_id: "binding".into(),
                daemon_id: "daemon".into(),
                server_fingerprint: "server-a".into(),
                database_names: vec!["ca_test_seata".into()],
                owner_instance_id: "instance-a".into(),
                created_by_codeark: false,
                ddl_version: "1".into(),
                ddl_checksum: "source-hash".into(),
                status: "Reserved".into(),
            },
            plan_digest: "plan-a".into(),
            steps: vec![
                InitializationStep {
                    id: "a".into(),
                    object_name: "table_a".into(),
                    statement: "source schema a".into(),
                    kind: "table".into(),
                },
                InitializationStep {
                    id: "b".into(),
                    object_name: "table_b".into(),
                    statement: "source schema b".into(),
                    kind: "table".into(),
                },
            ],
        }
    }
    #[tokio::test]
    async fn each_ddl_interrupt_resumes_without_replaying_completed_tables() {
        let d = tempfile::tempdir().unwrap();
        let store = InitializationStore::new(&d.path().join("journal.json"));
        let mut db = Database {
            exists: false,
            owner: None,
            tables: BTreeMap::new(),
            fail_on: Some("table_b".into()),
        };
        assert!(
            execute_initialization(&plan(), &mut db, &store)
                .await
                .is_err()
        );
        assert_eq!(
            db.tables.get("table_a").map(String::as_str),
            Some("compatible")
        );
        db.fail_on = None;
        let c = execute_initialization(&plan(), &mut db, &store)
            .await
            .unwrap();
        assert_eq!(c.status, "Initialized");
        assert_eq!(db.tables.len(), 2);
        assert_eq!(store.load().unwrap().unwrap().phase, "Completed");
    }
    #[tokio::test]
    async fn unknown_existing_database_requires_recovery() {
        let d = tempfile::tempdir().unwrap();
        let store = InitializationStore::new(&d.path().join("j.json"));
        let mut db = Database {
            exists: true,
            owner: None,
            tables: BTreeMap::from([("business_rows".into(), "preserved".into())]),
            fail_on: None,
        };
        assert_eq!(
            execute_initialization(&plan(), &mut db, &store)
                .await
                .unwrap_err()
                .code,
            "RecoveryRequired"
        );
        assert_eq!(db.tables["business_rows"], "preserved");
        assert!(db.owner.is_none());
    }
    #[tokio::test]
    async fn incompatible_schema_is_never_overwritten() {
        let d = tempfile::tempdir().unwrap();
        let store = InitializationStore::new(&d.path().join("j.json"));
        let mut db = Database {
            exists: true,
            owner: Some(("claim-a".into(), "instance-a".into())),
            tables: BTreeMap::from([("table_a".into(), "incompatible".into())]),
            fail_on: None,
        };
        assert_eq!(
            execute_initialization(&plan(), &mut db, &store)
                .await
                .unwrap_err()
                .code,
            "SchemaIncompatible"
        );
        assert_eq!(db.tables["table_a"], "incompatible");
    }
}

pub(crate) fn fragments(input: &str, delimiter: u8) -> Result<Vec<String>, ToolboxError> {
    let b = input.as_bytes();
    let mut out = vec![];
    let mut part = Vec::new();
    let mut i = 0;
    let mut quote = 0u8;
    let mut depth = 0usize;
    while i < b.len() {
        let c = b[i];
        if quote != 0 {
            part.push(c);
            if c == b'\\' && quote != b'`' {
                i += 1;
                if i >= b.len() {
                    return Err(error("InvalidSchema"));
                }
                part.push(b[i]);
            } else if c == quote {
                if b.get(i + 1) == Some(&quote) {
                    i += 1;
                    part.push(b[i]);
                } else {
                    quote = 0;
                }
            }
            i += 1;
            continue;
        }
        if b"'\"`".contains(&c) {
            quote = c;
            part.push(c);
            i += 1;
            continue;
        }
        if c == b'-'
            && b.get(i + 1) == Some(&b'-')
            && b.get(i + 2).is_none_or(u8::is_ascii_whitespace)
        {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            part.push(b' ');
            continue;
        }
        if c == b'#' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            part.push(b' ');
            continue;
        }
        if c == b'/' && b.get(i + 1) == Some(&b'*') {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 >= b.len() {
                return Err(error("InvalidSchema"));
            }
            i += 2;
            part.push(b' ');
            continue;
        }
        if c == delimiter && depth == 0 {
            let s =
                String::from_utf8(std::mem::take(&mut part)).map_err(|_| error("InvalidSchema"))?;
            if !s.trim().is_empty() {
                out.push(s.trim().into());
            }
            i += 1;
            continue;
        }
        if c == b'(' {
            depth += 1;
        } else if c == b')' {
            depth = depth.checked_sub(1).ok_or_else(|| error("InvalidSchema"))?;
        }
        part.push(c);
        i += 1;
    }
    if quote != 0 || depth != 0 {
        return Err(error("InvalidSchema"));
    }
    let s = String::from_utf8(part).map_err(|_| error("InvalidSchema"))?;
    if !s.trim().is_empty() {
        out.push(s.trim().into());
    }
    Ok(out)
}
pub(crate) fn identifier(input: &str) -> Result<(String, &str), ToolboxError> {
    let text = input.trim_start();
    let (name, rest) = if let Some(text) = text.strip_prefix('`') {
        let at = text.find('`').ok_or_else(|| error("InvalidSchema"))?;
        (&text[..at], &text[at + 1..])
    } else {
        let at = text
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .unwrap_or(text.len());
        (&text[..at], &text[at..])
    };
    if !crate::bindings::valid_database_name(name) {
        return Err(error("InvalidSchema"));
    }
    Ok((name.into(), rest))
}
pub fn parse_schema_steps(sql: &str) -> Result<Vec<InitializationStep>, ToolboxError> {
    if sql.len() > 1024 * 1024 {
        return Err(error("InvalidSchema"));
    }
    let mut steps = vec![];
    let mut tables = std::collections::BTreeSet::new();
    for statement in fragments(sql, b';')? {
        let upper = statement.to_ascii_uppercase();
        if upper.starts_with("CREATE DATABASE ")
            || upper.starts_with("USE ")
            || upper == "START TRANSACTION"
            || upper == "COMMIT"
        {
            continue;
        }
        if upper.starts_with("CREATE TABLE ") {
            let mut rest = &statement[13..];
            if rest.to_ascii_uppercase().starts_with("IF NOT EXISTS ") {
                rest = &rest[14..];
            }
            let (name, body) = identifier(rest)?;
            if !body.trim_start().starts_with('(') || !tables.insert(name.clone()) {
                return Err(error("InvalidSchema"));
            }
            steps.push(InitializationStep {
                id: format!("table/{name}"),
                object_name: name,
                statement,
                kind: "table".into(),
            });
        } else if upper.starts_with("INSERT INTO ") {
            let (name, rest) = identifier(&statement[12..])?;
            if !tables.contains(&name) {
                return Err(error("InvalidSchema"));
            }
            // VALUES is outside the explicit column list. Locate it without interpreting literals.
            let mut depth = 0i32;
            let mut at = None;
            let bytes = rest.as_bytes();
            let mut quote = false;
            for i in 0..bytes.len() {
                let c = bytes[i];
                if c == b'`' {
                    quote = !quote;
                }
                if quote {
                    continue;
                }
                if c == b'(' {
                    depth += 1;
                }
                if c == b')' {
                    depth -= 1;
                }
                if depth == 0
                    && bytes
                        .get(i..i + 6)
                        .is_some_and(|s| s.eq_ignore_ascii_case(b"VALUES"))
                {
                    at = Some(i);
                    break;
                }
            }
            let at = at.ok_or_else(|| error("InvalidSchema"))?;
            let columns = rest[..at].trim();
            if !columns.starts_with('(') || !columns.ends_with(')') {
                return Err(error("InvalidSchema"));
            }
            for row in fragments(&rest[at + 6..], b',')? {
                if !row.starts_with('(') || !row.ends_with(')') {
                    return Err(error("InvalidSchema"));
                }
                let index = steps
                    .iter()
                    .filter(|s| s.object_name == name && s.kind == "seed")
                    .count();
                steps.push(InitializationStep {
                    id: format!("seed/{name}/{index}"),
                    object_name: name.clone(),
                    statement: format!("INSERT INTO `{name}` {columns} VALUES {row}"),
                    kind: "seed".into(),
                });
            }
        } else {
            return Err(error("UnsupportedSchemaStatement"));
        }
    }
    if tables.is_empty() {
        return Err(error("InvalidSchema"));
    }
    Ok(steps)
}

#[cfg(test)]
mod schema_tests {
    use super::*;
    #[test]
    fn xxl_source_is_split_into_individual_tables_and_seed_rows() {
        let sql = include_str!("../../../services/xxjob-start-local/tables_xxl_job.sql");
        let steps = parse_schema_steps(sql).unwrap();
        let names = steps
            .iter()
            .filter(|s| s.kind == "table")
            .map(|s| s.object_name.as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"xxl_job_user"));
        assert!(names.contains(&"xxl_job_lock"));
        assert!(
            steps
                .iter()
                .filter(|s| s.kind == "seed" && s.object_name == "xxl_job_group")
                .count()
                >= 2
        );
        assert!(
            !steps
                .iter()
                .any(|s| s.statement.trim_start().starts_with("USE ")
                    || s.statement.trim_start().starts_with("CREATE DATABASE"))
        );
    }
    #[test]
    fn seata_four_tables_and_four_lock_seeds_are_complete() {
        let steps =
            parse_schema_steps(include_str!("../resources/sql/seata_1_8_0_mysql.sql")).unwrap();
        assert_eq!(steps.iter().filter(|s| s.kind == "table").count(), 4);
        assert_eq!(steps.iter().filter(|s| s.kind == "seed").count(), 4);
    }
    #[test]
    fn semicolon_in_literal_is_not_a_statement_boundary() {
        let steps=parse_schema_steps("CREATE TABLE `example` (`id` INT NOT NULL, PRIMARY KEY (`id`)); INSERT INTO `example` (`id`) VALUES (1), (2);").unwrap();
        assert_eq!(steps.len(), 3);
        assert!(parse_schema_steps("DROP DATABASE other_business;").is_err());
    }
}
