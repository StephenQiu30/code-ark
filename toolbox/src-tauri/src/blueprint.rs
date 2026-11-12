use crate::contracts::*;
use crate::env_store::{SecretValues, runtime_env};
use crate::workspace::{atomic_write, error, private_dir, reject_links};
use serde::Serialize;
use serde_json::{Value, json};
use serde_saphyr::Tagged;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub struct OverrideArtifact {
    pub override_path: PathBuf,
    pub env_path: PathBuf,
    pub config_digest: String,
}
impl std::fmt::Debug for OverrideArtifact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OverrideArtifact([restricted paths])")
    }
}
#[derive(Serialize)]
struct Overlay {
    services: BTreeMap<String, ServiceOverlay>,
    networks: Tagged<BTreeMap<String, Value>>,
    volumes: Tagged<BTreeMap<String, Value>>,
}
#[derive(Serialize)]
struct ServiceOverlay {
    container_name: String,
    ports: Tagged<Vec<Value>>,
    labels: BTreeMap<String, String>,
    networks: Tagged<BTreeMap<String, Value>>,
    environment: BTreeMap<String, Value>,
    image: String,
    pull_policy: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    volumes: Option<Tagged<Vec<Value>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    build: Option<Tagged<Option<String>>>,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn safe_token(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
fn literal(value: &str) -> String {
    value.replace('$', "$$")
}
fn json_env(value: &Value) -> BTreeMap<String, Value> {
    match value {
        Value::Object(map) => map.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        Value::Array(array) => array
            .iter()
            .filter_map(|v| v.as_str()?.split_once('='))
            .map(|(k, v)| (k.into(), Value::String(v.into())))
            .collect(),
        _ => BTreeMap::new(),
    }
}
fn read_compose(stack: &Path) -> Result<(Value, String), ToolboxError> {
    let path = stack.join("docker-compose.yml");
    reject_links(&path)?;
    let metadata = fs::metadata(&path).map_err(|_| error("BlueprintMissing"))?;
    if metadata.len() > 2 * 1024 * 1024 {
        return Err(error("BlueprintTooLarge"));
    }
    let raw = fs::read_to_string(path).map_err(|_| error("BlueprintMissing"))?;
    let value: Value = serde_saphyr::from_str(&raw).map_err(|_| error("InvalidBlueprint"))?;
    if value.get("include").is_some()
        || value["services"].as_object().is_none()
        || value["services"]
            .as_object()
            .unwrap()
            .values()
            .any(|s| s.get("extends").is_some())
    {
        return Err(error("UnsafeBlueprintReference"));
    }
    Ok((value, hash(raw.as_bytes())))
}
fn reference(image: &ImageArtifactRef) -> Result<(String, String), ToolboxError> {
    let sha = |s: &str| {
        s.starts_with("sha256:")
            && s.len() == 71
            && s[7..]
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    };
    if image.status != "resolved" {
        return Err(error("UnresolvedImageArtifact"));
    }
    if image.kind == "registry" {
        let registry = image
            .registry_ref
            .as_deref()
            .filter(|s| !s.contains(['$', ' ', '\n', '@']))
            .ok_or_else(|| error("InvalidImageArtifact"))?;
        let digest = image
            .manifest_digest
            .as_deref()
            .filter(|s| sha(s))
            .ok_or_else(|| error("InvalidImageArtifact"))?;
        if image.archive_url.is_some() || image.expected_image_id.is_some() {
            return Err(error("InvalidImageArtifact"));
        }
        Ok((format!("{registry}@{digest}"), "missing".into()))
    } else if image.kind == "release-image-archive" {
        let tag = image
            .local_content_tag
            .as_deref()
            .filter(|s| s.starts_with("codeark/") && !s.contains(['$', ' ', '\n']))
            .ok_or_else(|| error("InvalidImageArtifact"))?;
        if !image.expected_image_id.as_deref().is_some_and(sha)
            || !image
                .archive_sha256
                .as_deref()
                .is_some_and(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            || image.registry_ref.is_some()
        {
            return Err(error("InvalidImageArtifact"));
        }
        Ok((tag.into(), "never".into()))
    } else {
        Err(error("InvalidImageArtifact"))
    }
}
pub fn render_override(
    stack: &Path,
    service: &ServiceSpec,
    instance: &InstanceIdentity,
    effective: &EffectiveConfig,
    secrets: &SecretValues,
    bindings: &BindingSet,
) -> Result<OverrideArtifact, ToolboxError> {
    if effective.target_id != instance.target_id
        || effective.instance_id != instance.instance_id
        || instance.service_id != service.id
    {
        return Err(error("IdentityMismatch"));
    }
    if !safe_token(&effective.generation_id) || !safe_token(&instance.project_name) {
        return Err(error("UnsafePath"));
    }
    reject_links(stack)?;
    let (source, source_sha) = read_compose(stack)?;
    let runtime = stack.join(".runtime").join(&effective.generation_id);
    private_dir(&runtime)?;
    let mut values = SecretValues(secrets.0.clone());
    for field in &service.env_schema {
        if !values.0.contains_key(&field.key) {
            if field.sensitive && field.required {
                return Err(error("CredentialRepairRequired"));
            }
            values
                .0
                .insert(field.key.clone(), field.public_default.clone());
        }
    }
    for binding in bindings {
        if binding.target_id != instance.target_id || binding.daemon_id != instance.daemon_id {
            return Err(error("TargetChanged"));
        }
        if !safe_token(&binding.network_alias) || binding.network_name.is_empty() {
            return Err(error("InvalidBinding"));
        }
        if binding.engine == "mysql" {
            values
                .0
                .insert("MYSQL_HOST".into(), binding.network_alias.clone());
            values
                .0
                .insert("MYSQL_PORT".into(), binding.container_port.to_string());
        }
        if binding.engine == "pgsql" {
            values.0.insert(
                "TEMPORAL_POSTGRES_HOST".into(),
                binding.network_alias.clone(),
            );
            values
                .0
                .insert("PGSQL_DOCKER_NETWORK".into(), binding.network_name.clone());
        }
    }
    let required = |key: &str| {
        values
            .0
            .get(key)
            .cloned()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| error("RequiredEnvFieldMissing"))
    };
    let mut networks = BTreeMap::new();
    if let Some(source_networks) = source["networks"].as_object() {
        for key in source_networks.keys() {
            networks.insert(key.clone(),json!({"name":format!("{}-{key}",instance.project_name),"driver":"bridge","labels":{"io.codeark.workspace-id":instance.workspace_id,"io.codeark.instance-id":instance.instance_id,"io.codeark.service-id":service.id,"io.codeark.blueprint-sha":source_sha}}));
        }
    }
    if networks.is_empty() {
        networks.insert(
            "default".into(),
            json!({"name":format!("{}-default",instance.project_name),"driver":"bridge","labels":{"io.codeark.workspace-id":instance.workspace_id,"io.codeark.instance-id":instance.instance_id,"io.codeark.service-id":service.id,"io.codeark.blueprint-sha":source_sha}}),
        );
    }
    for (index, binding) in bindings.iter().enumerate() {
        networks.insert(
            format!("codeark-dependency-{index}"),
            json!({"name":binding.network_name,"external":true}),
        );
    }
    let mut volumes = BTreeMap::new();
    if let Some(source_volumes) = source["volumes"].as_object() {
        for key in source_volumes.keys() {
            volumes.insert(key.clone(),json!({"name":format!("{}-{key}",instance.project_name),"labels":{"io.codeark.workspace-id":instance.workspace_id,"io.codeark.instance-id":instance.instance_id,"io.codeark.service-id":service.id,"io.codeark.blueprint-sha":source_sha}}));
        }
    }
    let mut services = BTreeMap::new();
    for component in &service.components {
        let original = &source["services"][&component.id];
        if original.is_null() {
            return Err(error("ComponentMissing"));
        }
        let image = effective
            .image_artifacts
            .iter()
            .find(|r| r.source_id == component.image_ref)
            .ok_or_else(|| error("UnresolvedImageArtifact"))?;
        let (image, pull_policy) = reference(image)?;
        let mut ports = vec![];
        for port in effective
            .ports
            .iter()
            .filter(|p| p.component_id == component.id)
        {
            if port.host_ip != "127.0.0.1"
                || port.host_port == 0
                || !matches!(port.protocol.as_str(), "tcp" | "udp")
            {
                return Err(error("InvalidPort"));
            }
            let target = if service.id == "rocketmq"
                && component.id == "console"
                && port.container_port == 8080
            {
                8082
            } else {
                port.container_port
            };
            ports.push(json!({"target":target,"published":port.host_port.to_string(),"host_ip":"127.0.0.1","protocol":port.protocol}));
        }
        let host_port = |target: u16| {
            effective
                .ports
                .iter()
                .find(|p| p.component_id == component.id && p.container_port == target)
                .map(|p| p.host_port)
                .ok_or_else(|| error("MissingPort"))
        };
        let mut environment = BTreeMap::new();
        let mut command = None;
        let mut mounts = None;
        let mut build = None;
        if service.id == "xxjob" {
            let host = required("MYSQL_HOST")?;
            let port = required("MYSQL_PORT")?;
            let db = required("MYSQL_DATABASE")?;
            let user = required("MYSQL_USERNAME")?;
            let password = required("MYSQL_PASSWORD")?;
            let token = required("XXL_JOB_ACCESS_TOKEN")?;
            if [
                host.as_str(),
                port.as_str(),
                db.as_str(),
                user.as_str(),
                password.as_str(),
                token.as_str(),
            ]
            .iter()
            .any(|s| s.chars().any(char::is_whitespace))
            {
                return Err(error("UnsupportedParameterValue"));
            }
            let params = format!(
                "--spring.datasource.url=jdbc:mysql://{host}:{port}/{db}?useUnicode=true&characterEncoding=UTF-8&autoReconnect=true&serverTimezone=Asia/Shanghai --spring.datasource.username={user} --spring.datasource.password={password} --spring.datasource.driver-class-name=com.mysql.cj.jdbc.Driver --xxl.job.accessToken={token} --server.port=8080"
            );
            environment.insert("PARAMS".into(), json!(literal(&params)));
        }
        if service.id == "seata" {
            let mut application: Value = serde_saphyr::from_str(
                &fs::read_to_string(stack.join("seata-config/application.yml"))
                    .map_err(|_| error("BlueprintMissing"))?,
            )
            .map_err(|_| error("InvalidBlueprint"))?;
            application["seata"]["store"]["db"]["url"] = json!(format!(
                "jdbc:mysql://{}:{}/{}?useUnicode=true&characterEncoding=utf8&useSSL=false&serverTimezone=Asia/Shanghai",
                required("MYSQL_HOST")?,
                required("MYSQL_PORT")?,
                required("MYSQL_DATABASE")?
            ));
            application["seata"]["store"]["db"]["user"] = json!(required("MYSQL_USERNAME")?);
            application["seata"]["store"]["db"]["password"] = json!(required("MYSQL_PASSWORD")?);
            application["console"]["user"]["password"] = json!(required("SEATA_CONSOLE_PASSWORD")?);
            application["seata"]["security"]["secretKey"] =
                json!(required("SEATA_SECURITY_SECRET")?);
            let path = runtime.join("application.toolbox.yml");
            atomic_write(
                &path,
                serde_saphyr::to_string(&application)
                    .map_err(|_| error("InvalidBlueprint"))?
                    .as_bytes(),
            )?;
            let mut list = vec![];
            for volume in original["volumes"].as_array().into_iter().flatten() {
                if let Some(s) = volume.as_str() {
                    let pieces = s.split(':').collect::<Vec<_>>();
                    let target = pieces
                        .get(1)
                        .copied()
                        .ok_or_else(|| error("InvalidMount"))?;
                    if target == "/seata-server/resources/application.yml" {
                        list.push(json!({"type":"bind","source":path,"target":target,"read_only":true,"bind":{"create_host_path":false}}));
                    } else {
                        list.push(volume.clone());
                    }
                } else {
                    list.push(volume.clone());
                }
            }
            mounts = Some(Tagged(list, Some("!override".into())));
        }
        if service.id == "kafka" && component.id == "kafka" {
            environment.insert(
                "KAFKA_ADVERTISED_LISTENERS".into(),
                json!(format!(
                    "EXTERNAL://127.0.0.1:{},INTERNAL://kafka:29092",
                    host_port(9092)?
                )),
            );
        }
        if service.id == "monitoring" && component.id == "grafana" {
            environment.insert(
                "GF_SERVER_ROOT_URL".into(),
                json!(format!("http://127.0.0.1:{}", host_port(3000)?)),
            );
        }
        if service.id == "searxng" {
            environment.insert(
                "SEARXNG_BASE_URL".into(),
                json!(format!("http://127.0.0.1:{}/", host_port(8080)?)),
            );
        }
        if service.id == "nacos" {
            environment.insert(
                "NACOS_AUTH_PASSWORD".into(),
                json!(literal(&required("NACOS_AUTH_PASSWORD")?)),
            );
            environment.insert("NACOS_AUTH_ADMIN_ENABLE".into(), json!("true"));
            environment.insert("NACOS_AUTH_CONSOLE_ENABLE".into(), json!("true"));
            for key in ["NACOS_AUTH_IDENTITY_KEY", "NACOS_AUTH_IDENTITY_VALUE"] {
                environment.insert(key.into(), json!(literal(&required(key)?)));
            }
            let base = host_port(8848)?;
            for target in [9848u16, 9849, 9850] {
                if host_port(target)? as u32 != base as u32 + (target - 8848) as u32 {
                    return Err(error("InvalidCoupledPorts"));
                }
            }
        }
        if service.id == "rocketmq" && component.id == "broker" {
            ports.push(json!({"target":8081,"published":effective.ports.iter().find(|p|p.component_id==component.id&&p.container_port==8081).map_or(18082,|p|p.host_port).to_string(),"host_ip":"127.0.0.1","protocol":"tcp"}));
            command = Some(vec![
                "sh".into(),
                "mqbroker".into(),
                "-c".into(),
                "/home/rocketmq/conf/broker.conf".into(),
                "--enable-proxy".into(),
            ]);
        }
        if service.id == "elastic" {
            let password = required("ES_LOCAL_PASSWORD")?;
            let kibana_password = required("KIBANA_LOCAL_PASSWORD")?;
            if component.id == "elasticsearch" || component.id == "logstash" {
                environment.insert("ELASTIC_PASSWORD".into(), json!(literal(&password)));
            }
            if component.id == "kibana" {
                environment.insert(
                    "ELASTICSEARCH_PASSWORD".into(),
                    json!(literal(&kibana_password)),
                );
                let es_port = effective
                    .ports
                    .iter()
                    .find(|p| p.component_id == "elasticsearch" && p.container_port == 9200)
                    .ok_or_else(|| error("MissingPort"))?
                    .host_port;
                environment.insert(
                    "ELASTICSEARCH_PUBLICBASEURL".into(),
                    json!(format!("http://127.0.0.1:{es_port}")),
                );
            }
            if component.id == "kibana_settings" {
                // The legacy source embeds credentials in a shell string. The managed job
                // uses readonly files, so secrets never become shell or curl arguments.
                let auth = runtime.join("elastic-auth.curlrc");
                let user = format!("elastic:{password}")
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"");
                atomic_write(
                    &auth,
                    format!("user = \"{user}\"\nsilent\nshow-error\nfail\n").as_bytes(),
                )?;
                let payload = runtime.join("kibana-password.json");
                atomic_write(
                    &payload,
                    &serde_json::to_vec(&json!({"password":kibana_password}))
                        .map_err(|_| error("InvalidConfig"))?,
                )?;
                let script = runtime.join("setup-elastic.sh");
                atomic_write(&script,br##"#!/bin/sh
set -eu
curl --config /run/codeark/auth.curlrc --request POST --header 'Content-Type: application/json' --data-binary @/run/codeark/password.json http://elasticsearch:9200/_security/user/kibana_system/_password >/dev/null
curl --config /run/codeark/auth.curlrc http://elasticsearch:9200/_license | grep -Eq '"type"[[:space:]]*:[[:space:]]*"basic"'
"##)?;
                mounts = Some(Tagged(
                    vec![
                        json!({"type":"bind","source":auth,"target":"/run/codeark/auth.curlrc","read_only":true,"bind":{"create_host_path":false}}),
                        json!({"type":"bind","source":payload,"target":"/run/codeark/password.json","read_only":true,"bind":{"create_host_path":false}}),
                        json!({"type":"bind","source":script,"target":"/run/codeark/setup.sh","read_only":true,"bind":{"create_host_path":false}}),
                    ],
                    Some("!override".into()),
                ));
                command = Some(vec!["/bin/sh".into(), "/run/codeark/setup.sh".into()]);
            }
        }
        if service.id == "elastic" && component.id == "elasticsearch" {
            build = Some(Tagged(None, Some("!reset".into())));
        }
        let attached = networks
            .keys()
            .map(|key| {
                (
                    key.clone(),
                    json!({"aliases":[format!("{}-{}",instance.project_name,component.id)]}),
                )
            })
            .collect();
        let labels = BTreeMap::from([
            (
                "io.codeark.workspace-id".into(),
                instance.workspace_id.clone(),
            ),
            (
                "io.codeark.instance-id".into(),
                instance.instance_id.clone(),
            ),
            ("io.codeark.service-id".into(), service.id.clone()),
            ("io.codeark.blueprint-sha".into(), source_sha.clone()),
            (
                "io.codeark.generation-id".into(),
                effective.generation_id.clone(),
            ),
        ]);
        // Validate environment shape while retaining source interpolation as the authority.
        let _ = json_env(&original["environment"]);
        services.insert(
            component.id.clone(),
            ServiceOverlay {
                container_name: format!("{}-{}", instance.project_name, component.id),
                ports: Tagged(ports, Some("!override".into())),
                labels,
                networks: Tagged(attached, Some("!override".into())),
                environment,
                image,
                pull_policy,
                volumes: mounts,
                command,
                build,
            },
        );
    }
    let overlay = Overlay {
        services,
        networks: Tagged(networks, Some("!override".into())),
        volumes: Tagged(volumes, Some("!override".into())),
    };
    let encoded = serde_saphyr::to_string(&overlay).map_err(|_| error("InvalidBlueprint"))?;
    if service.id == "elastic" {
        // Per-file Compose parsing happens before merge. Escape only the obsolete
        // shell-string inputs; all live credential consumers are overridden above.
        for key in ["ES_LOCAL_PASSWORD", "KIBANA_LOCAL_PASSWORD"] {
            if let Some(value) = values.0.get_mut(key) {
                *value = value.replace('\'', "'\\''");
            }
        }
    }
    let env = runtime_env(&values.0)?;
    let env_path = runtime.join("candidate.runtime.env");
    let override_path = runtime.join("compose.toolbox.override.yml");
    atomic_write(&env_path, env.as_bytes())?;
    atomic_write(&override_path, encoded.as_bytes())?;
    Ok(OverrideArtifact {
        override_path,
        env_path,
        config_digest: hash(format!("{encoded}{}", hash(env.as_bytes())).as_bytes()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    fn inputs(
        id: &str,
    ) -> (
        tempfile::TempDir,
        ServiceSpec,
        InstanceIdentity,
        EffectiveConfig,
        SecretValues,
    ) {
        let s = crate::catalog::load_embedded_catalog()
            .unwrap()
            .services
            .into_iter()
            .find(|s| s.id == id)
            .unwrap();
        let temp = tempfile::tempdir().unwrap();
        let src = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(&s.source_dir);
        for asset in &s.assets {
            let dst = temp.path().join(asset);
            std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
            std::fs::copy(src.join(asset), dst).unwrap();
        }
        let identity = InstanceIdentity {
            workspace_id: "workspace-a".into(),
            service_id: id.into(),
            instance_id: "instance-a".into(),
            project_name: format!("ca-workspace-a-{id}"),
            target_id: "target-a".into(),
            daemon_id: "daemon-a".into(),
        };
        let config = EffectiveConfig {
            generation_id: "generation-1".into(),
            target_id: "target-a".into(),
            instance_id: "instance-a".into(),
            ports: s
                .ports
                .iter()
                .map(|p| PortMapping {
                    host_port: p.host_port + 10000,
                    ..p.clone()
                })
                .collect(),
            endpoints: vec![],
            secret_refs: vec![],
            config_digest: "".into(),
            image_artifacts: s
                .image_refs
                .iter()
                .map(|id| ImageArtifactRef {
                    source_id: id.clone(),
                    kind: "registry".into(),
                    platform: "linux/arm64".into(),
                    registry_ref: Some("docker.io/library/redis".into()),
                    manifest_digest: Some(format!("sha256:{}", "a".repeat(64))),
                    archive_url: None,
                    archive_sha256: None,
                    expected_image_id: None,
                    local_content_tag: None,
                    status: "resolved".into(),
                })
                .collect(),
        };
        let mut values: BTreeMap<String, String> = s
            .env_schema
            .iter()
            .map(|f| {
                (
                    f.key.clone(),
                    if f.sensitive {
                        "test$#='secret".into()
                    } else {
                        f.public_default.clone()
                    },
                )
            })
            .collect();
        values.extend([
            ("MYSQL_HOST".into(), "bound-mysql".into()),
            ("MYSQL_PORT".into(), "3307".into()),
            ("MYSQL_DATABASE".into(), "ca_test_xxl".into()),
            ("MYSQL_USERNAME".into(), "service_user".into()),
            ("MYSQL_PASSWORD".into(), "test$#='secret".into()),
            ("XXL_JOB_ACCESS_TOKEN".into(), "test-token".into()),
        ]);
        (temp, s, identity, config, SecretValues(values))
    }
    fn model(root: &Path, a: &OverrideArtifact) -> serde_json::Value {
        let output = std::process::Command::new("docker")
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .args(["compose", "--project-directory"])
            .arg(root)
            .args(["-p", "codeark-config-test", "--env-file"])
            .arg(&a.env_path)
            .arg("-f")
            .arg(root.join("docker-compose.yml"))
            .arg("-f")
            .arg(&a.override_path)
            .args(["config", "--format", "json"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Compose config validation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
    #[test]
    fn xxl_effective_params() {
        let (dir, s, i, c, secrets) = inputs("xxjob");
        let a = render_override(dir.path(), &s, &i, &c, &secrets, &vec![]).unwrap();
        let m = model(dir.path(), &a);
        let params = m["services"]["xxl-job-admin"]["environment"]["PARAMS"]
            .as_str()
            .unwrap();
        assert!(params.contains("jdbc:mysql://bound-mysql:3307/ca_test_xxl"));
        assert!(params.contains("--spring.datasource.username=service_user"));
        assert!(params.contains("--spring.datasource.password=test$$#='secret"));
    }
    #[test]
    fn seata_effective_jdbc() {
        let (dir, s, i, c, secrets) = inputs("seata");
        let a = render_override(dir.path(), &s, &i, &c, &secrets, &vec![]).unwrap();
        let m = model(dir.path(), &a);
        let volumes = m["services"]["seata-server"]["volumes"].as_array().unwrap();
        let file = volumes
            .iter()
            .find(|v| v["target"] == "/seata-server/resources/application.yml")
            .unwrap();
        assert_eq!(file["read_only"], true);
        let value: serde_json::Value = serde_saphyr::from_str(
            &std::fs::read_to_string(file["source"].as_str().unwrap()).unwrap(),
        )
        .unwrap();
        assert!(
            value["seata"]["store"]["db"]["url"]
                .as_str()
                .unwrap()
                .starts_with("jdbc:mysql://bound-mysql:3307/ca_test_xxl")
        );
        assert_eq!(value["seata"]["store"]["db"]["password"], "test$#='secret");
    }
    #[test]
    fn all_ports_and_names_replaced() {
        let (dir, s, i, c, secrets) = inputs("nacos");
        let a = render_override(dir.path(), &s, &i, &c, &secrets, &vec![]).unwrap();
        let m = model(dir.path(), &a);
        assert_eq!(
            m["services"]["nacos"]["container_name"],
            "ca-workspace-a-nacos-nacos"
        );
        let ports = m["services"]["nacos"]["ports"].as_array().unwrap();
        assert_eq!(ports.len(), s.ports.len());
        assert!(
            ports
                .iter()
                .all(|p| p["published"].as_str().unwrap().parse::<u16>().unwrap() > 18000)
        );
        assert!(m["volumes"].as_object().unwrap().values().all(|v| {
            v["name"]
                .as_str()
                .unwrap()
                .starts_with("ca-workspace-a-nacos-")
        }));
    }
    #[test]
    fn rocket_proxy_keeps_internal_routes() {
        let (dir, s, i, c, secrets) = inputs("rocketmq");
        let a = render_override(dir.path(), &s, &i, &c, &secrets, &vec![]).unwrap();
        let m = model(dir.path(), &a);
        let command = m["services"]["broker"]["command"].as_array().unwrap();
        assert!(command.iter().any(|v| v == "--enable-proxy"));
        assert!(
            m["services"]["broker"]["ports"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["target"] == 8081)
        );
        assert!(
            m["services"]["console"]["ports"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["target"] == 8082)
        );
    }
    #[test]
    fn every_source_component_gets_owned_names_and_complete_ports() {
        for id in crate::catalog::load_embedded_catalog()
            .unwrap()
            .services
            .into_iter()
            .map(|s| s.id)
        {
            let (dir, s, i, c, secrets) = inputs(&id);
            let a = render_override(dir.path(), &s, &i, &c, &secrets, &vec![]).unwrap();
            let m = model(dir.path(), &a);
            for component in &s.components {
                assert!(
                    m["services"][&component.id]["container_name"]
                        .as_str()
                        .unwrap()
                        .starts_with(&i.project_name)
                );
                assert!(m["services"][&component.id].get("build").is_none());
            }
        }
    }
    #[test]
    fn unresolved_image_and_foreign_identity_fail_closed() {
        let (dir, s, i, mut c, secrets) = inputs("redis");
        c.image_artifacts.clear();
        assert!(render_override(dir.path(), &s, &i, &c, &secrets, &vec![]).is_err());
        c.instance_id = "foreign".into();
        assert!(render_override(dir.path(), &s, &i, &c, &secrets, &vec![]).is_err());
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintManifest {
    pub schema_version: u32,
    pub service_id: String,
    pub source_commit: String,
    pub compatibility: String,
    pub files: BTreeMap<String, String>,
}
#[derive(Debug)]
pub struct MaterializeResult {
    pub manifest: BlueprintManifest,
    pub created_paths: Vec<String>,
    pub conflicts: Vec<String>,
}
#[derive(Debug)]
pub struct BlueprintUpdatePlan {
    pub unchanged: Vec<String>,
    pub safe_updates: Vec<String>,
    pub conflicts: Vec<String>,
    pub compatibility_gate: Option<String>,
}
fn asset_path(path: &str) -> Result<(), ToolboxError> {
    if path.is_empty()
        || Path::new(path).is_absolute()
        || path.contains('\\')
        || path.split('/').any(|p| {
            p.is_empty()
                || p == "."
                || p == ".."
                || p == ".env"
                || p == ".runtime"
                || p == "data"
                || p == "logs"
                || (p.starts_with(".env.") && p != ".env.example")
        })
    {
        return Err(error("UnsafeAssetPath"));
    }
    Ok(())
}
fn declared_path(path: &str) -> Result<(), ToolboxError> {
    if path.is_empty()
        || Path::new(path).is_absolute()
        || path.contains('\\')
        || path
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == ".." || p == ".env" || p == ".runtime")
    {
        return Err(error("UnsafePath"));
    }
    Ok(())
}
pub fn materialize_blueprint(
    bundle: &Path,
    stack: &Path,
    service: &ServiceSpec,
    identity: &InstanceIdentity,
) -> Result<MaterializeResult, ToolboxError> {
    if identity.service_id != service.id {
        return Err(error("IdentityMismatch"));
    }
    reject_links(bundle)?;
    reject_links(stack)?;
    if stack.starts_with(bundle) || bundle.starts_with(stack) {
        return Err(error("UnsafePath"));
    }
    let distribution: Value = crate::workspace::read_json(&bundle.join("manifest.json"))?;
    if distribution["producer"] != "CodeArkBlueprintSync" || distribution["schemaVersion"] != 1 {
        return Err(error("InvalidManifest"));
    }
    let mut manifest = BlueprintManifest {
        schema_version: 1,
        service_id: service.id.clone(),
        source_commit: distribution["sourceCommit"]
            .as_str()
            .ok_or_else(|| error("InvalidManifest"))?
            .into(),
        compatibility: service.data_compatibility.clone(),
        files: BTreeMap::new(),
    };
    let mut assets = vec![];
    for file in &service.assets {
        asset_path(file)?;
        let source = bundle.join(&service.id).join(file);
        reject_links(&source)?;
        let bytes = fs::read(&source).map_err(|_| error("BlueprintMissing"))?;
        let expected = distribution["files"]
            .as_array()
            .ok_or_else(|| error("InvalidManifest"))?
            .iter()
            .find(|f| f["path"] == format!("{}/{file}", service.id))
            .and_then(|f| f["sha256"].as_str())
            .ok_or_else(|| error("InvalidManifest"))?;
        if hash(&bytes) != expected {
            return Err(error("AssetHashMismatch"));
        }
        manifest.files.insert(file.clone(), expected.into());
        assets.push((file, bytes));
    }
    // Validate every source and destination before writing any file.
    for (file, _) in &assets {
        reject_links(&stack.join(file))?;
    }
    private_dir(stack)?;
    let mut created_paths = vec![];
    let mut conflicts = vec![];
    let previous = if stack.join(".blueprint-manifest.json").exists() {
        Some(crate::workspace::read_json::<BlueprintManifest>(
            &stack.join(".blueprint-manifest.json"),
        )?)
    } else {
        None
    };
    if let Some(ref old) = previous
        && (old.service_id != service.id || old.compatibility != manifest.compatibility)
    {
        return Err(error("SchemaIncompatible"));
    }
    crate::managed_data::prepare(stack, service, identity)?;
    for (file, bytes) in assets {
        let path = stack.join(file);
        if path.exists() {
            let actual = hash(&fs::read(&path).map_err(|_| error("AssetUnavailable"))?);
            if actual != manifest.files[file] {
                conflicts.push(file.clone());
            } // no automatic updates
            continue;
        }
        private_dir(path.parent().ok_or_else(|| error("UnsafePath"))?)?;
        atomic_write(&path, &bytes)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                &path,
                fs::Permissions::from_mode(if file.ends_with(".sh") { 0o755 } else { 0o644 }),
            )
            .map_err(|_| error("PermissionDenied"))?;
            let mut parent = path.parent();
            while let Some(dir) = parent {
                if dir == stack {
                    break;
                }
                fs::set_permissions(dir, fs::Permissions::from_mode(0o755))
                    .map_err(|_| error("PermissionDenied"))?;
                parent = dir.parent();
            }
        }
        created_paths.push(file.clone());
    }
    for data in &service.data_paths {
        declared_path(data)?;
        reject_links(&stack.join(data))?;
    }
    if previous.is_none() && conflicts.is_empty() {
        atomic_write(
            &stack.join(".blueprint-manifest.json"),
            &serde_json::to_vec(&manifest).map_err(|_| error("InvalidManifest"))?,
        )?;
    }
    Ok(MaterializeResult {
        manifest,
        created_paths,
        conflicts,
    })
}
pub fn plan_blueprint_update(
    stack: &Path,
    old: &BlueprintManifest,
    new: &BlueprintManifest,
) -> Result<BlueprintUpdatePlan, ToolboxError> {
    reject_links(stack)?;
    if old.service_id != new.service_id {
        return Err(error("IdentityMismatch"));
    }
    for file in old.files.keys().chain(new.files.keys()) {
        asset_path(file)?;
        reject_links(&stack.join(file))?;
    }
    let compatibility_gate =
        if old.schema_version != new.schema_version || old.compatibility != new.compatibility {
            Some("SchemaIncompatible".into())
        } else {
            None
        };
    let mut plan = BlueprintUpdatePlan {
        unchanged: vec![],
        safe_updates: vec![],
        conflicts: vec![],
        compatibility_gate,
    };
    for (file, new_hash) in &new.files {
        let path = stack.join(file);
        if !path.exists() {
            if old.files.contains_key(file) {
                plan.conflicts.push(file.clone());
            } else {
                plan.safe_updates.push(file.clone());
            }
            continue;
        }
        let actual = hash(&fs::read(&path).map_err(|_| error("AssetUnavailable"))?);
        if old
            .files
            .get(file)
            .is_none_or(|expected| expected != &actual)
        {
            plan.conflicts.push(file.clone());
        } else if &actual == new_hash {
            plan.unchanged.push(file.clone());
        } else {
            plan.safe_updates.push(file.clone());
        }
    }
    // Deletions are conflicts until explicitly reviewed; user data is never inferred.
    for file in old.files.keys().filter(|f| !new.files.contains_key(*f)) {
        plan.conflicts.push(file.clone());
    }
    Ok(plan)
}

#[cfg(test)]
mod update_tests {
    use super::*;
    fn manifests() -> (BlueprintManifest, BlueprintManifest) {
        let old = BlueprintManifest {
            schema_version: 1,
            service_id: "redis".into(),
            source_commit: "old".into(),
            compatibility: "redis-8".into(),
            files: BTreeMap::from([("docker-compose.yml".into(), hash(b"original"))]),
        };
        let mut new = old.clone();
        new.source_commit = "new".into();
        new.files
            .insert("docker-compose.yml".into(), hash(b"updated"));
        (old, new)
    }
    #[test]
    fn user_edit_blocks_update() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("docker-compose.yml"), b"user edited").unwrap();
        let (old, new) = manifests();
        let p = plan_blueprint_update(dir.path(), &old, &new).unwrap();
        assert_eq!(p.conflicts, vec!["docker-compose.yml"]);
        assert!(p.safe_updates.is_empty());
        assert_eq!(
            std::fs::read(dir.path().join("docker-compose.yml")).unwrap(),
            b"user edited"
        );
    }
    #[test]
    fn unchanged_source_yields_candidate_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("docker-compose.yml"), b"original").unwrap();
        let (old, new) = manifests();
        let p = plan_blueprint_update(dir.path(), &old, &new).unwrap();
        assert_eq!(p.safe_updates, vec!["docker-compose.yml"]);
        assert_eq!(
            std::fs::read(dir.path().join("docker-compose.yml")).unwrap(),
            b"original"
        );
    }
    #[test]
    fn incompatible_upgrade_is_gated() {
        let dir = tempfile::tempdir().unwrap();
        let (old, mut new) = manifests();
        new.compatibility = "redis-7".into();
        assert!(
            plan_blueprint_update(dir.path(), &old, &new)
                .unwrap()
                .compatibility_gate
                .is_some()
        );
        new.files.insert(".env".into(), hash(b"secret"));
        assert!(plan_blueprint_update(dir.path(), &old, &new).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn reject_escaping_update_path() {
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        std::fs::write(other.path().join("original"), b"other data").unwrap();
        std::os::unix::fs::symlink(
            other.path().join("original"),
            dir.path().join("docker-compose.yml"),
        )
        .unwrap();
        let (old, new) = manifests();
        assert!(plan_blueprint_update(dir.path(), &old, &new).is_err());
        assert_eq!(
            std::fs::read(other.path().join("original")).unwrap(),
            b"other data"
        );
    }
    #[test]
    fn materialize_preserves_existing_env_and_user_files() {
        let catalog = crate::catalog::load_embedded_catalog().unwrap();
        let service = catalog.services.iter().find(|s| s.id == "redis").unwrap();
        let root = tempfile::tempdir().unwrap();
        let stack = root.path().join("stack");
        std::fs::create_dir_all(&stack).unwrap();
        std::fs::write(stack.join(".env"), b"PRIVATE=preserved").unwrap();
        let identity = InstanceIdentity {
            workspace_id: "w".into(),
            service_id: "redis".into(),
            instance_id: "i".into(),
            project_name: "ca-w-redis".into(),
            target_id: "t".into(),
            daemon_id: "d".into(),
        };
        let bundle = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/blueprints");
        let first = materialize_blueprint(&bundle, &stack, service, &identity).unwrap();
        assert!(first.conflicts.is_empty());
        std::fs::write(stack.join("docker-compose.yml"), b"user edited").unwrap();
        let second = materialize_blueprint(&bundle, &stack, service, &identity).unwrap();
        assert!(second.conflicts.contains(&"docker-compose.yml".to_string()));
        assert_eq!(
            std::fs::read(stack.join(".env")).unwrap(),
            b"PRIVATE=preserved"
        );
        assert_eq!(
            std::fs::read(stack.join("docker-compose.yml")).unwrap(),
            b"user edited"
        );
    }
}
