use crate::contracts::{SecretPolicy, ServiceSpec, ToolboxError};
use base64::Engine;
use std::collections::BTreeMap;
use zeroize::Zeroize;

#[derive(Default)]
pub struct SecretValues(pub BTreeMap<String, String>);
impl std::fmt::Debug for SecretValues {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretValues([REDACTED])")
    }
}
impl Drop for SecretValues {
    fn drop(&mut self) {
        for value in self.0.values_mut() {
            value.zeroize();
        }
    }
}
pub struct EnvCandidate {
    pub document: String,
    pub changed_keys: Vec<String>,
    pub validation_errors: Vec<String>,
}
impl std::fmt::Debug for EnvCandidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnvCandidate")
            .field("changed_keys", &self.changed_keys)
            .field("validation_errors", &self.validation_errors)
            .finish()
    }
}
impl Drop for EnvCandidate {
    fn drop(&mut self) {
        self.document.zeroize();
    }
}
fn err(code: &str) -> ToolboxError {
    ToolboxError::new(code)
}
fn valid_key(key: &str) -> bool {
    let mut it = key.bytes();
    matches!(it.next(), Some(b'a'..=b'z' | b'A'..=b'Z' | b'_'))
        && it.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
fn decode(value: &str) -> Result<String, ToolboxError> {
    let value = value.trim();
    let mut out = String::new();
    if let Some(quote) = value.chars().next().filter(|c| *c == '\'' || *c == '"') {
        let mut chars = value[1..].chars();
        let mut closed = false;
        while let Some(c) = chars.next() {
            if c == quote {
                closed = true;
                break;
            }
            if c == '\\' {
                let next = chars.next().ok_or_else(|| err("InvalidEnvValue"))?;
                if next == quote || (quote == '"' && next == '\\') {
                    out.push(next);
                } else if quote == '"' {
                    match next {
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        '$' => out.push('$'),
                        _ => {
                            out.push('\\');
                            out.push(next);
                        }
                    }
                } else {
                    out.push('\\');
                    out.push(next);
                }
            } else {
                out.push(c);
            }
        }
        let tail = chars.collect::<String>();
        if !closed || (!tail.trim().is_empty() && !tail.trim_start().starts_with('#')) {
            return Err(err("InvalidEnvValue"));
        }
    } else {
        // A # begins an unquoted comment only when preceded by whitespace.
        let mut previous_space = false;
        for c in value.chars() {
            if c == '#' && previous_space {
                break;
            }
            out.push(c);
            previous_space = c.is_whitespace();
        }
        out = out.trim_end().into();
    }
    if out.contains(['\0', '\n', '\r']) {
        return Err(err("UnsupportedEnvValue"));
    }
    Ok(out)
}
pub fn parse_env(document: &str) -> Result<BTreeMap<String, String>, ToolboxError> {
    if document.len() > 1024 * 1024 {
        return Err(err("EnvTooLarge"));
    }
    let mut values = BTreeMap::new();
    for raw in document.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| err("InvalidEnvDocument"))?;
        let key = key.trim();
        if !valid_key(key) {
            return Err(err("InvalidEnvKey"));
        }
        if values.contains_key(key) {
            return Err(err("AmbiguousEnvKey"));
        }
        values.insert(key.into(), decode(value)?);
    }
    Ok(values)
}
fn encode(value: &str) -> Result<String, ToolboxError> {
    if value.contains(['\0', '\n', '\r']) {
        return Err(err("UnsupportedEnvValue"));
    }
    // Double quoted dotenv handles trailing backslashes; escape interpolation explicitly.
    let encoded = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$");
    Ok(format!("\"{encoded}\""))
}
pub fn runtime_env(values: &BTreeMap<String, String>) -> Result<String, ToolboxError> {
    let mut document = String::new();
    for (key, value) in values {
        if !valid_key(key) {
            return Err(err("InvalidEnvKey"));
        }
        document.push_str(&format!("{key}={}\n", encode(value)?));
    }
    Ok(document)
}
pub fn generate_secret(format: &str) -> Result<String, ToolboxError> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| err("RandomSourceUnavailable"))?;
    match format {
        "base64" => Ok(base64::engine::general_purpose::STANDARD.encode(bytes)),
        "hex64" => Ok(bytes.iter().map(|b| format!("{b:02x}")).collect()),
        "alphanumeric24" | "alphanumeric32" => {
            const ALPHABET: &[u8] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
            let len = if format == "alphanumeric24" { 24 } else { 32 };
            let mut result = String::new();
            while result.len() < len {
                getrandom::fill(&mut bytes).map_err(|_| err("RandomSourceUnavailable"))?;
                for b in bytes {
                    if b < 248 {
                        result.push(ALPHABET[(b % 62) as usize] as char);
                        if result.len() == len {
                            break;
                        }
                    }
                }
            }
            bytes.zeroize();
            Ok(result)
        }
        _ => Err(err("InvalidSecretFormat")),
    }
}
pub fn merge_env(
    existing: Option<&str>,
    sample: &str,
    service: &ServiceSpec,
    secrets: &SecretValues,
    is_new: bool,
) -> Result<EnvCandidate, ToolboxError> {
    let current = SecretValues(parse_env(existing.unwrap_or(""))?);
    let defaults = SecretValues(parse_env(sample)?);
    let mut document = existing.unwrap_or("").to_owned();
    let mut changed_keys = vec![];
    let newline = if document.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    for field in &service.env_schema {
        if !valid_key(&field.key) {
            return Err(err("InvalidEnvKey"));
        }
        if let Some(value) = current.0.get(&field.key).filter(|s| !s.is_empty()) {
            // Preserve explicit existing values, including a public example value.
            let _ = encode(value)?;
            continue;
        }
        if field.sensitive && !is_new && field.secret_policy != SecretPolicy::OptionalEmpty {
            return Err(err("CredentialRepairRequired"));
        }
        let value = if field.sensitive {
            match field.secret_policy {
                SecretPolicy::GenerateOnFirstManagedCreate => {
                    generate_secret(field.secret_format.as_deref().unwrap_or("alphanumeric32"))?
                }
                SecretPolicy::InheritBinding => secrets
                    .0
                    .get(&field.key)
                    .cloned()
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| err("BindingCredentialRequired"))?,
                SecretPolicy::PreserveExisting => return Err(err("CredentialRepairRequired")),
                SecretPolicy::OptionalEmpty => String::new(),
            }
        } else {
            secrets
                .0
                .get(&field.key)
                .or_else(|| defaults.0.get(&field.key))
                .cloned()
                .unwrap_or_else(|| field.public_default.clone())
        };
        if field.required && value.is_empty() {
            return Err(err("RequiredEnvFieldMissing"));
        }
        let encoded = encode(&value)?;
        // An empty existing assignment must be replaced at its position, not duplicated.
        if current.0.contains_key(&field.key) {
            document = document
                .split_inclusive('\n')
                .map(|line| {
                    let raw = line.trim();
                    let raw = raw.strip_prefix("export ").unwrap_or(raw);
                    if raw
                        .split_once('=')
                        .is_some_and(|(key, _)| key.trim() == field.key)
                    {
                        let ending = if line.ends_with("\r\n") {
                            "\r\n"
                        } else if line.ends_with('\n') {
                            "\n"
                        } else {
                            ""
                        };
                        format!("{}={}{ending}", field.key, encoded)
                    } else {
                        line.to_owned()
                    }
                })
                .collect();
        } else {
            if !document.is_empty() && !document.ends_with('\n') {
                document.push_str(newline);
            }
            document.push_str(&format!("{}={}{newline}", field.key, encoded));
        }
        changed_keys.push(field.key.clone());
    }
    Ok(EnvCandidate {
        document,
        changed_keys,
        validation_errors: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn spec(id: &str) -> ServiceSpec {
        crate::catalog::load_embedded_catalog()
            .unwrap()
            .services
            .into_iter()
            .find(|s| s.id == id)
            .unwrap()
    }
    #[test]
    fn dotenv_roundtrip() {
        assert_eq!(
            parse_env(r"PATH_VALUE='a\\b'").unwrap()["PATH_VALUE"],
            r"a\\b"
        );
        let raw = "# preserved\r\nVALUE='a=b$#\\\'quoted'\r\nOTHER=tail";
        let value = parse_env(raw).unwrap();
        assert_eq!(value["VALUE"], "a=b$#'quoted");
        assert_eq!(value["OTHER"], "tail");
        let mut s = spec("redis");
        s.env_schema.clear();
        assert_eq!(
            merge_env(Some(raw), "", &s, &SecretValues::default(), false)
                .unwrap()
                .document,
            raw
        );
        let encoded = runtime_env(&value).unwrap();
        assert_eq!(parse_env(&encoded).unwrap(), value);
    }
    #[test]
    fn new_instance_secret_policies() {
        let a = merge_env(None, "", &spec("nacos"), &SecretValues::default(), true).unwrap();
        let fields = parse_env(&a.document).unwrap();
        let token = fields
            .iter()
            .find(|(key, _)| key.contains("TOKEN"))
            .unwrap()
            .1;
        use base64::Engine;
        assert!(
            base64::engine::general_purpose::STANDARD
                .decode(token)
                .unwrap()
                .len()
                >= 32
        );
        let a = merge_env(None, "", &spec("searxng"), &SecretValues::default(), true).unwrap();
        let fields = parse_env(&a.document).unwrap();
        let secret = fields["SEARXNG_SECRET"].clone();
        assert_eq!(secret.len(), 64);
        assert!(
            secret
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        );
        let a = generate_secret("alphanumeric24").unwrap();
        assert_eq!(a.len(), 24);
        assert!(a.bytes().all(|b| b.is_ascii_alphanumeric()));
        assert_ne!(a, generate_secret("alphanumeric24").unwrap());
    }
    #[test]
    fn existing_secret_is_preserved() {
        let raw = "SEARXNG_SECRET='same-as-public-but-explicit'\n";
        let a = merge_env(
            Some(raw),
            "",
            &spec("searxng"),
            &SecretValues::default(),
            false,
        )
        .unwrap();
        assert_eq!(
            parse_env(&a.document).unwrap()["SEARXNG_SECRET"],
            "same-as-public-but-explicit"
        );
        assert!(!format!("{a:?}").contains("same-as-public-but-explicit"));
        let mut initialized = spec("mysql");
        initialized.env_schema.retain(|f| f.sensitive);
        assert_eq!(
            merge_env(Some(""), "", &initialized, &SecretValues::default(), false)
                .unwrap_err()
                .code,
            "CredentialRepairRequired"
        );
    }
    #[test]
    fn binding_secret_is_inherited() {
        let mut s = spec("temporal");
        s.env_schema.retain(|f| f.sensitive);
        let secret = SecretValues(BTreeMap::from([(
            "PGSQL_PASSWORD".into(),
            "bound$#='pass".into(),
        )]));
        let a = merge_env(None, "", &s, &secret, true).unwrap();
        assert_eq!(
            parse_env(&a.document).unwrap()["PGSQL_PASSWORD"],
            "bound$#='pass"
        );
        assert!(merge_env(None, "", &s, &SecretValues::default(), true).is_err());
        assert!(!format!("{secret:?}").contains("bound$"));
    }
    #[test]
    fn literal_secret_survives_interpolation() {
        let temp = tempfile::tempdir().unwrap();
        let compose = temp.path().join("compose.yml");
        std::fs::write(
            &compose,
            "services:\n  demo:\n    image: redis:7\n    environment:\n      VALUE: ${PASSWORD}\n",
        )
        .unwrap();
        for literal in [
            "a=b$#'quoted\\path",
            "a\\\\b",
            "\"double\"$${name}",
            "ends\\",
        ] {
            let env = runtime_env(&BTreeMap::from([("PASSWORD".into(), literal.into())])).unwrap();
            let file = temp.path().join("runtime.env");
            std::fs::write(&file, env).unwrap();
            let output = std::process::Command::new("docker")
                .env_remove("PASSWORD")
                .args(["compose", "--project-directory"])
                .arg(temp.path())
                .arg("--env-file")
                .arg(&file)
                .arg("-f")
                .arg(&compose)
                .args(["config", "--format", "json"])
                .output()
                .expect("Docker Compose CLI required for config-only validation");
            assert!(
                output.status.success(),
                "Compose config-only validation failed"
            );
            let model: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            // Compose config re-escapes dollars for a re-loadable model. This is not container payload escaping.
            assert_eq!(
                model["services"]["demo"]["environment"]["VALUE"],
                literal.replace('$', "$$")
            );
        }
    }
    #[test]
    fn dotenv_rejects_ambiguous_or_unsafe_values() {
        assert!(parse_env("PASSWORD='unterminated").is_err());
        assert!(runtime_env(&BTreeMap::from([("BAD KEY".into(), "value".into())])).is_err());
        assert!(runtime_env(&BTreeMap::from([("KEY".into(), "line\nbreak".into())])).is_err());
    }
}
