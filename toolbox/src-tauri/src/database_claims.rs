use crate::{
    bindings::{AuthorizedBinding, valid_database_name},
    contracts::*,
    workspace::{error, random_id},
};
use std::collections::BTreeSet;
#[derive(Debug)]
pub struct ClaimPlan {
    pub claim: DatabaseClaim,
}
pub fn plan_database_claim(
    binding: &AuthorizedBinding,
    identity: &InstanceIdentity,
    databases: &[String],
) -> Result<ClaimPlan, ToolboxError> {
    if identity.target_id != binding.target_id
        || identity.daemon_id != binding.daemon_id
        || binding.server_fingerprint.is_empty()
        || binding.grant.server_fingerprint != binding.server_fingerprint
    {
        return Err(error("ServerIdentityMismatch"));
    }
    if binding.binding.grant != "InitializeNamedResources"
        || databases.is_empty()
        || databases.iter().any(|db| {
            !valid_database_name(db)
                || !binding.binding.allowed_databases.contains(db)
                || !binding.grant.allowed_databases.contains(db)
        })
    {
        return Err(error("InitializationGrantOutOfScope"));
    }
    let unique = databases.iter().collect::<BTreeSet<_>>();
    if unique.len() != databases.len() {
        return Err(error("InvalidDatabaseClaim"));
    }
    if identity.service_id == "temporal"
        && (binding.engine != "pgsql"
            || unique
                != BTreeSet::from([&"temporal".to_owned(), &"temporal_visibility".to_owned()]))
    {
        return Err(error("TemporalDatabasePairRequired"));
    }
    Ok(ClaimPlan {
        claim: DatabaseClaim {
            claim_id: random_id()?,
            binding_id: binding.binding_id.clone(),
            daemon_id: binding.daemon_id.clone(),
            server_fingerprint: binding.server_fingerprint.clone(),
            database_names: databases.to_vec(),
            owner_instance_id: identity.instance_id.clone(),
            created_by_codeark: false,
            ddl_version: String::new(),
            ddl_checksum: String::new(),
            status: "Reserved".into(),
        },
    })
}
pub fn verify_claim_evidence(
    claim: &DatabaseClaim,
    server: &str,
    marker_claim: &str,
    marker_owner: &str,
) -> Result<(), ToolboxError> {
    if claim.server_fingerprint.is_empty()
        || claim.server_fingerprint != server
        || claim.claim_id.is_empty()
        || claim.claim_id != marker_claim
        || claim.owner_instance_id != marker_owner
        || !claim
            .database_names
            .iter()
            .all(|db| valid_database_name(db))
    {
        return Err(error("RecoveryRequired"));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn proof() -> AuthorizedBinding {
        AuthorizedBinding {
            binding: DatabaseBinding {
                binding_id: "b".into(),
                target_id: "t".into(),
                daemon_id: "d".into(),
                engine: "mysql".into(),
                lifecycle_owner: "ExternalConnected".into(),
                container_id: "c".into(),
                network_name: "n".into(),
                network_alias: "mysql".into(),
                container_port: 3306,
                host_endpoint: None,
                credential_ref: SecretRef { key: "k".into() },
                allowed_databases: vec!["ca_test_seata".into()],
                grant: "InitializeNamedResources".into(),
                server_fingerprint: "server-a".into(),
            },
            grant: InitializationGrant {
                target_id: "t".into(),
                server_fingerprint: "server-a".into(),
                allowed_databases: vec!["ca_test_seata".into()],
                allowed_accounts: vec!["ca_test_seata".into()],
                operations: vec!["InitializeNamedResources".into()],
                confirmed_at: 1,
            },
        }
    }
    fn identity() -> InstanceIdentity {
        InstanceIdentity {
            workspace_id: "w".into(),
            service_id: "seata".into(),
            instance_id: "i".into(),
            project_name: "ca-w-seata".into(),
            target_id: "t".into(),
            daemon_id: "d".into(),
        }
    }
    #[test]
    fn claim_requires_server_identity_and_exact_scope() {
        let p = plan_database_claim(&proof(), &identity(), &["ca_test_seata".into()]).unwrap();
        assert_eq!(p.claim.status, "Reserved");
        assert!(!p.claim.created_by_codeark);
        assert!(plan_database_claim(&proof(), &identity(), &["other_business".into()]).is_err());
        assert!(plan_database_claim(&proof(), &identity(), &["mysql".into()]).is_err());
    }
    #[test]
    fn unknown_database_and_forged_owner_are_not_claimed() {
        let mut c = plan_database_claim(&proof(), &identity(), &["ca_test_seata".into()])
            .unwrap()
            .claim;
        c.created_by_codeark = true;
        assert!(verify_claim_evidence(&c, "server-a", &c.claim_id, "i").is_ok());
        assert!(verify_claim_evidence(&c, "server-a", &c.claim_id, "someone-else").is_err());
        assert!(verify_claim_evidence(&c, "server-b", &c.claim_id, "i").is_err());
        assert!(verify_claim_evidence(&c, "server-a", "", "").is_err());
    }
    #[test]
    fn temporal_cannot_claim_only_half_the_pair() {
        let mut b = proof();
        b.binding.engine = "pgsql".into();
        b.binding.allowed_databases = vec!["temporal".into(), "temporal_visibility".into()];
        b.grant.allowed_databases = b.binding.allowed_databases.clone();
        let mut i = identity();
        i.service_id = "temporal".into();
        assert!(plan_database_claim(&b, &i, &["temporal".into()]).is_err());
        assert_eq!(
            plan_database_claim(&b, &i, &b.binding.allowed_databases)
                .unwrap()
                .claim
                .database_names
                .len(),
            2
        );
    }
}
