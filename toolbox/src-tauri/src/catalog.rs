use crate::contracts::{ServiceCatalog, ToolboxError};
pub fn load_embedded_catalog() -> Result<ServiceCatalog, ToolboxError> {
    serde_json::from_str(include_str!("../../catalog/services.json"))
        .map_err(|_| ToolboxError::new("InvalidCatalog"))
}

pub fn resolve_startup_order(
    catalog: &ServiceCatalog,
    selected: &[String],
    bindings: &crate::contracts::BindingSet,
) -> Result<Vec<String>, ToolboxError> {
    use std::collections::{BTreeMap, BTreeSet};
    let services: BTreeMap<_, _> = catalog
        .services
        .iter()
        .map(|s| (s.id.as_str(), s))
        .collect();
    if services.len() != catalog.services.len() {
        return Err(ToolboxError::new("InvalidCatalog"));
    }
    let bound: BTreeSet<&str> = bindings.iter().map(|b| b.engine.as_str()).collect();
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut ordered = vec![];
    fn visit<'a>(
        id: &'a str,
        services: &BTreeMap<&'a str, &'a crate::contracts::ServiceSpec>,
        bound: &BTreeSet<&str>,
        visiting: &mut BTreeSet<&'a str>,
        visited: &mut BTreeSet<&'a str>,
        ordered: &mut Vec<String>,
    ) -> Result<(), ToolboxError> {
        if visited.contains(id) {
            return Ok(());
        }
        let service = services
            .get(id)
            .ok_or_else(|| ToolboxError::new("UnknownService"))?;
        if !visiting.insert(service.id.as_str()) {
            return Err(ToolboxError::new("DependencyCycle"));
        }
        for dependency in &service.dependencies {
            if !bound.contains(dependency.as_str()) {
                visit(dependency, services, bound, visiting, visited, ordered)?;
            }
        }
        visiting.remove(service.id.as_str());
        visited.insert(service.id.as_str());
        ordered.push(service.id.clone());
        Ok(())
    }
    for id in selected {
        let canonical = services
            .get(id.as_str())
            .ok_or_else(|| ToolboxError::new("UnknownService"))?
            .id
            .as_str();
        visit(
            canonical,
            &services,
            &bound,
            &mut visiting,
            &mut visited,
            &mut ordered,
        )?;
    }
    Ok(ordered)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packaged_catalog_has_complete_sources() {
        let c = load_embedded_catalog().unwrap();
        assert_eq!(c.services.len(), 17);
        assert_eq!(c.guides.len(), 1);
        assert_eq!(c.scenes.len(), 7);
        assert!(
            c.services
                .iter()
                .all(|s| s.source_dir.starts_with("services/")
                    && !s.source_project.starts_with("services/"))
        );
    }
    #[test]
    fn dag_uses_selected_bindings() {
        let c = load_embedded_catalog().unwrap();
        assert_eq!(
            resolve_startup_order(&c, &["temporal".into()], &vec![]).unwrap(),
            vec!["pgsql", "temporal"]
        );
        let binding = crate::contracts::DatabaseBinding {
            binding_id: "pg-existing".into(),
            target_id: "t".into(),
            daemon_id: "d".into(),
            engine: "pgsql".into(),
            lifecycle_owner: "ExternalConnected".into(),
            container_id: "pg".into(),
            network_name: "pg-network".into(),
            network_alias: "pg".into(),
            container_port: 5432,
            host_endpoint: None,
            credential_ref: crate::contracts::SecretRef {
                key: "pg-secret".into(),
            },
            allowed_databases: vec![],
            grant: "ReadOnlyConnect".into(),
            server_fingerprint: "verified".into(),
        };
        assert_eq!(
            resolve_startup_order(&c, &["temporal".into()], &vec![binding]).unwrap(),
            vec!["temporal"]
        );
    }
    #[test]
    fn dag_rejects_cycles_and_unknown_services() {
        let mut c = load_embedded_catalog().unwrap();
        c.services
            .iter_mut()
            .find(|s| s.id == "pgsql")
            .unwrap()
            .dependencies = vec!["temporal".into()];
        assert_eq!(
            resolve_startup_order(&c, &["temporal".into()], &vec![])
                .unwrap_err()
                .code,
            "DependencyCycle"
        );
        assert!(resolve_startup_order(&c, &["unknown".into()], &vec![]).is_err());
    }
}
