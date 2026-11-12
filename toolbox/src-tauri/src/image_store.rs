use crate::contracts::{ImageArtifactRef, ToolboxError};
use crate::docker_cli::{DockerRunner, normalize_architecture};
use crate::workspace::{error, open_private, private_dir, reject_links};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::io::AsyncWriteExt;

pub struct ImageContext {
    pub cache_root: PathBuf,
    pub cancelled: Arc<AtomicBool>,
}
impl ImageContext {
    async fn cancellation(&self) {
        while !self.cancelled.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}
#[derive(Debug, Clone)]
pub struct VerifiedImage {
    pub source_id: String,
    pub target_id: String,
    pub platform: String,
    pub image_id: String,
    pub runtime_reference: String,
}
fn digest(value: &str, prefix: bool) -> bool {
    let raw = if prefix {
        let Some(raw) = value.strip_prefix("sha256:") else {
            return false;
        };
        raw
    } else {
        value
    };
    raw.len() == 64
        && raw
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub fn validate_image_ref(
    image: &ImageArtifactRef,
    architecture: &str,
) -> Result<(), ToolboxError> {
    if image.platform != format!("linux/{}", normalize_architecture(architecture)?) {
        return Err(error("ImageArchitectureMismatch"));
    }
    if image.status != "resolved" {
        return Err(error("UnresolvedImageArtifact"));
    }
    match image.kind.as_str() {
        "registry" => {
            if image
                .registry_ref
                .as_deref()
                .is_none_or(|s| s.is_empty() || s.contains(['$', '\n', ' ', '@']))
                || !image
                    .manifest_digest
                    .as_deref()
                    .is_some_and(|s| digest(s, true))
                || image.archive_url.is_some()
                || image.archive_sha256.is_some()
                || image.expected_image_id.is_some()
                || image.local_content_tag.is_some()
            {
                return Err(error("InvalidImageArtifact"));
            }
        }
        "release-image-archive" => {
            let url = reqwest::Url::parse(
                image
                    .archive_url
                    .as_deref()
                    .ok_or_else(|| error("UnresolvedImageArtifact"))?,
            )
            .map_err(|_| error("UnsafeArtifactSource"))?;
            if url.scheme() != "https"
                || url.host_str() != Some("github.com")
                || !url
                    .path()
                    .starts_with("/StephenQiu30/code-ark/releases/download/")
                || !url.username().is_empty()
                || url.password().is_some()
                || url.fragment().is_some()
            {
                return Err(error("UnsafeArtifactSource"));
            }
            if !image
                .archive_sha256
                .as_deref()
                .is_some_and(|s| digest(s, false))
                || !image
                    .expected_image_id
                    .as_deref()
                    .is_some_and(|s| digest(s, true))
                || image.registry_ref.is_some()
                || image.manifest_digest.is_some()
            {
                return Err(error("InvalidImageArtifact"));
            }
            let suffix = image
                .expected_image_id
                .as_deref()
                .unwrap()
                .trim_start_matches("sha256:");
            if !image.local_content_tag.as_deref().is_some_and(|s| {
                s.starts_with("codeark/")
                    && s.ends_with(&format!(":sha256-{suffix}"))
                    && !s.contains(['$', '\n', ' '])
            }) {
                return Err(error("InvalidImageArtifact"));
            }
        }
        _ => return Err(error("InvalidImageArtifact")),
    }
    Ok(())
}
fn file_hash(path: &Path) -> Result<String, ToolboxError> {
    let mut file = File::open(path).map_err(|_| error("ArchiveUnavailable"))?;
    let mut hash = Sha256::new();
    let mut chunk = [0u8; 65536];
    loop {
        let read = file
            .read(&mut chunk)
            .map_err(|_| error("ArchiveUnavailable"))?;
        if read == 0 {
            break;
        }
        hash.update(&chunk[..read]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
#[derive(Debug, Clone)]
struct VerifiedArchiveIdentity {
    ids: std::collections::BTreeSet<String>,
    archive_sha256: String,
    locked_id: String,
    platform: String,
    tag: String,
}
impl VerifiedArchiveIdentity {
    fn permits(&self, id: &str, image: &ImageArtifactRef) -> bool {
        image.archive_sha256.as_deref() == Some(&self.archive_sha256)
            && image.expected_image_id.as_deref() == Some(&self.locked_id)
            && image.local_content_tag.as_deref() == Some(&self.tag)
            && image.platform == self.platform
            && self.ids.contains(id)
    }
}
struct ArchiveMember {
    size: u64,
    digest: String,
    // JSON/config metadata is bounded; large layer bytes are hashed, never buffered.
    bytes: Option<Vec<u8>>,
}
fn member_name(path: &Path) -> Result<String, ToolboxError> {
    let mut names = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(name) => {
                let name = name.to_str().ok_or_else(|| error("UnsafeImageArchive"))?;
                if name.contains('\\') {
                    return Err(error("UnsafeImageArchive"));
                }
                names.push(name);
            }
            std::path::Component::CurDir => (),
            _ => return Err(error("UnsafeImageArchive")),
        }
    }
    Ok(names.join("/"))
}
fn archive_json(
    members: &BTreeMap<String, ArchiveMember>,
    name: &str,
) -> Result<serde_json::Value, ToolboxError> {
    let member = members
        .get(name)
        .ok_or_else(|| error("InvalidImageArchive"))?;
    serde_json::from_slice(
        member
            .bytes
            .as_deref()
            .ok_or_else(|| error("ArchiveTooLarge"))?,
    )
    .map_err(|_| error("InvalidImageArchive"))
}
fn descriptor_member<'a>(
    members: &'a BTreeMap<String, ArchiveMember>,
    descriptor: &serde_json::Value,
) -> Result<(&'a ArchiveMember, String), ToolboxError> {
    let digest = descriptor["digest"]
        .as_str()
        .filter(|s| self::digest(s, true))
        .ok_or_else(|| error("InvalidImageArchive"))?;
    let size = descriptor["size"]
        .as_u64()
        .ok_or_else(|| error("InvalidImageArchive"))?;
    if descriptor
        .get("urls")
        .is_some_and(|v| !v.as_array().is_some_and(Vec::is_empty))
    {
        return Err(error("UnsafeImageArchive"));
    }
    let path = format!("blobs/sha256/{}", digest.trim_start_matches("sha256:"));
    let member = members
        .get(&path)
        .ok_or_else(|| error("InvalidImageArchive"))?;
    if member.digest != digest || member.size != size {
        return Err(error("ImageIdentityMismatch"));
    }
    if let Some(data) = descriptor.get("data") {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data.as_str().ok_or_else(|| error("InvalidImageArchive"))?)
            .map_err(|_| error("InvalidImageArchive"))?;
        if bytes.len() as u64 != size || format!("sha256:{:x}", Sha256::digest(&bytes)) != digest {
            return Err(error("ImageIdentityMismatch"));
        }
    }
    Ok((member, path))
}
struct RunnableArchiveImage {
    ids: std::collections::BTreeSet<String>,
    config_digest: String,
    manifest_digest: String,
    layer_paths: Vec<String>,
}
struct ArchiveGraph<'a> {
    members: &'a BTreeMap<String, ArchiveMember>,
    platform: &'a str,
    active: std::collections::BTreeSet<String>,
    visits: usize,
    runnable: Vec<RunnableArchiveImage>,
    attestation_targets: Vec<String>,
}
impl ArchiveGraph<'_> {
    fn walk(
        &mut self,
        descriptor: &serde_json::Value,
        ancestors: &std::collections::BTreeSet<String>,
        depth: usize,
    ) -> Result<(), ToolboxError> {
        self.visits += 1;
        if depth > 16 || self.visits > 10000 {
            return Err(error("ArchiveTooLarge"));
        }
        let (_, path) = descriptor_member(self.members, descriptor)?;
        let id = descriptor["digest"].as_str().unwrap().to_owned();
        if !self.active.insert(id.clone()) {
            return Err(error("UnsafeImageArchive"));
        }
        let value = archive_json(self.members, &path)?;
        let media = descriptor["mediaType"]
            .as_str()
            .ok_or_else(|| error("InvalidImageArchive"))?;
        if value["schemaVersion"] != 2 || value.get("mediaType").is_some_and(|v| v != media) {
            return Err(error("InvalidImageArchive"));
        }
        let mut chain = ancestors.clone();
        chain.insert(id.clone());
        match media {
            "application/vnd.oci.image.index.v1+json"
            | "application/vnd.docker.distribution.manifest.list.v2+json" => {
                let children = value["manifests"]
                    .as_array()
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| error("InvalidImageArchive"))?;
                for child in children {
                    self.walk(child, &chain, depth + 1)?;
                }
            }
            "application/vnd.oci.image.manifest.v1+json"
            | "application/vnd.docker.distribution.manifest.v2+json" => {
                let (config_member, config_path) =
                    descriptor_member(self.members, &value["config"])?;
                let config = archive_json(self.members, &config_path)?;
                let layers = value["layers"]
                    .as_array()
                    .ok_or_else(|| error("InvalidImageArchive"))?;
                let mut layer_paths = Vec::new();
                for layer in layers {
                    layer_paths.push(descriptor_member(self.members, layer)?.1);
                }
                let attestation = descriptor["annotations"]["vnd.docker.reference.type"]
                    == "attestation-manifest";
                if attestation {
                    if value
                        .get("artifactType")
                        .is_some_and(|v| v != "application/vnd.docker.attestation.manifest.v1+json")
                        || descriptor["platform"]["os"] != "unknown"
                        || descriptor["platform"]["architecture"] != "unknown"
                        || !config.as_object().is_some_and(|v| v.is_empty())
                    {
                        return Err(error("UnsafeImageArchive"));
                    }
                    let target = descriptor["annotations"]["vnd.docker.reference.digest"]
                        .as_str()
                        .filter(|s| digest(s, true))
                        .ok_or_else(|| error("InvalidImageArchive"))?
                        .to_owned();
                    if let Some(subject) = value.get("subject") {
                        descriptor_member(self.members, subject)?;
                        if subject["digest"] != target {
                            return Err(error("ImageIdentityMismatch"));
                        }
                    }
                    if layers.is_empty() {
                        return Err(error("InvalidImageArchive"));
                    }
                    for (layer, path) in layers.iter().zip(&layer_paths) {
                        if layer["mediaType"] != "application/vnd.in-toto+json" {
                            return Err(error("UnsafeImageArchive"));
                        }
                        let statement = archive_json(self.members, path)?;
                        if !statement["subject"].as_array().is_some_and(|subjects| {
                            !subjects.is_empty()
                                && subjects.iter().all(|s| {
                                    s["digest"]["sha256"] == target.trim_start_matches("sha256:")
                                })
                        }) {
                            return Err(error("ImageIdentityMismatch"));
                        }
                    }
                    self.attestation_targets.push(target);
                } else {
                    if value.get("artifactType").is_some()
                        || !matches!(
                            value["config"]["mediaType"].as_str(),
                            Some(
                                "application/vnd.oci.image.config.v1+json"
                                    | "application/vnd.docker.container.image.v1+json"
                            )
                        )
                    {
                        return Err(error("InvalidImageArchive"));
                    }
                    let architecture =
                        normalize_architecture(config["architecture"].as_str().unwrap_or(""))
                            .map_err(|_| error("ImageArchitectureMismatch"))?;
                    if config["os"] != "linux" || format!("linux/{architecture}") != self.platform {
                        return Err(error("ImageArchitectureMismatch"));
                    }
                    if let Some(platform) = descriptor.get("platform") {
                        let declared =
                            normalize_architecture(platform["architecture"].as_str().unwrap_or(""))
                                .map_err(|_| error("ImageArchitectureMismatch"))?;
                        if platform["os"] != "linux"
                            || declared != architecture
                            || platform
                                .get("variant")
                                .is_some_and(|v| architecture != "arm64" || v != "v8")
                        {
                            return Err(error("ImageArchitectureMismatch"));
                        }
                    }
                    for layer in layers {
                        if !matches!(
                            layer["mediaType"].as_str(),
                            Some(
                                "application/vnd.oci.image.layer.v1.tar"
                                    | "application/vnd.oci.image.layer.v1.tar+gzip"
                                    | "application/vnd.oci.image.layer.v1.tar+zstd"
                                    | "application/vnd.docker.image.rootfs.diff.tar"
                                    | "application/vnd.docker.image.rootfs.diff.tar.gzip"
                            )
                        ) {
                            return Err(error("UnsafeImageArchive"));
                        }
                    }
                    chain.insert(config_member.digest.clone());
                    self.runnable.push(RunnableArchiveImage {
                        ids: chain,
                        config_digest: config_member.digest.clone(),
                        manifest_digest: id.clone(),
                        layer_paths,
                    });
                }
            }
            _ => return Err(error("InvalidImageArchive")),
        }
        self.active.remove(&id);
        Ok(())
    }
}
pub fn validate_archive(path: &Path, image: &ImageArtifactRef) -> Result<(), ToolboxError> {
    validate_archive_identity(path, image).map(|_| ())
}
fn validate_archive_identity(
    path: &Path,
    image: &ImageArtifactRef,
) -> Result<VerifiedArchiveIdentity, ToolboxError> {
    reject_links(path)?;
    if std::fs::metadata(path)
        .map_err(|_| error("ArchiveUnavailable"))?
        .len()
        > 8 * 1024 * 1024 * 1024
    {
        return Err(error("ArchiveTooLarge"));
    }
    if file_hash(path)? != image.archive_sha256.as_deref().unwrap_or("") {
        return Err(error("ArchiveHashMismatch"));
    }
    let mut archive = tar::Archive::new(File::open(path).map_err(|_| error("ArchiveUnavailable"))?);
    let mut members = BTreeMap::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut metadata_bytes = 0u64;
    for (count, entry) in archive
        .entries()
        .map_err(|_| error("InvalidImageArchive"))?
        .enumerate()
    {
        if count >= 10000 {
            return Err(error("ArchiveTooLarge"));
        }
        let mut entry = entry.map_err(|_| error("InvalidImageArchive"))?;
        let name = member_name(&entry.path().map_err(|_| error("InvalidImageArchive"))?)?;
        if !seen.insert(name.clone()) {
            return Err(error("UnsafeImageArchive"));
        }
        if entry.header().entry_type().is_dir() {
            continue;
        }
        if name.is_empty() || !entry.header().entry_type().is_file() {
            return Err(error("UnsafeImageArchive"));
        }
        let size = entry.size();
        let mut bytes = if size <= 2 * 1024 * 1024 {
            metadata_bytes += size;
            if metadata_bytes > 32 * 1024 * 1024 {
                return Err(error("ArchiveTooLarge"));
            }
            Some(Vec::with_capacity(size as usize))
        } else {
            None
        };
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 65536];
        let mut read_size = 0u64;
        loop {
            let read = entry
                .read(&mut buffer)
                .map_err(|_| error("InvalidImageArchive"))?;
            if read == 0 {
                break;
            }
            read_size += read as u64;
            hash.update(&buffer[..read]);
            if let Some(bytes) = bytes.as_mut() {
                bytes.extend_from_slice(&buffer[..read]);
            }
        }
        if read_size != size {
            return Err(error("InvalidImageArchive"));
        }
        let digest = format!("sha256:{:x}", hash.finalize());
        if let Some(raw) = name.strip_prefix("blobs/sha256/")
            && (!self::digest(raw, false) || digest != format!("sha256:{raw}"))
        {
            return Err(error("ImageIdentityMismatch"));
        }
        members.insert(
            name,
            ArchiveMember {
                size,
                digest,
                bytes,
            },
        );
    }
    let manifest = archive_json(&members, "manifest.json")?;
    let entries = manifest
        .as_array()
        .filter(|a| a.len() == 1)
        .ok_or_else(|| error("UnsafeImageArchive"))?;
    let entry = &entries[0];
    let tags = entry["RepoTags"]
        .as_array()
        .filter(|a| a.len() == 1)
        .ok_or_else(|| error("UnsafeImageArchive"))?;
    let tag = image
        .local_content_tag
        .as_deref()
        .ok_or_else(|| error("InvalidImageArtifact"))?;
    if tags[0].as_str() != Some(tag) {
        return Err(error("UnsafeImageArchive"));
    }
    let config_path = entry["Config"]
        .as_str()
        .ok_or_else(|| error("InvalidImageArchive"))?;
    let config_member = members
        .get(config_path)
        .ok_or_else(|| error("InvalidImageArchive"))?;
    let config = archive_json(&members, config_path)?;
    let architecture = normalize_architecture(config["architecture"].as_str().unwrap_or(""))
        .map_err(|_| error("ImageArchitectureMismatch"))?;
    if config["os"] != "linux" || format!("linux/{architecture}") != image.platform {
        return Err(error("ImageArchitectureMismatch"));
    }
    let legacy_layers = entry["Layers"]
        .as_array()
        .ok_or_else(|| error("InvalidImageArchive"))?;
    let layer_paths = legacy_layers
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .ok_or_else(|| error("InvalidImageArchive"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    for path in &layer_paths {
        if !members.contains_key(path) {
            return Err(error("InvalidImageArchive"));
        }
    }
    let mut ids = std::collections::BTreeSet::from([config_member.digest.clone()]);
    if members.contains_key("index.json") {
        if archive_json(&members, "oci-layout")?["imageLayoutVersion"] != "1.0.0" {
            return Err(error("InvalidImageArchive"));
        }
        let index = archive_json(&members, "index.json")?;
        if index["schemaVersion"] != 2
            || index["mediaType"] != "application/vnd.oci.image.index.v1+json"
        {
            return Err(error("InvalidImageArchive"));
        }
        let roots = index["manifests"]
            .as_array()
            .filter(|v| v.len() == 1)
            .ok_or_else(|| error("UnsafeImageArchive"))?;
        let annotations = &roots[0]["annotations"];
        if annotations
            .get("io.containerd.image.name")
            .is_some_and(|v| {
                v.as_str()
                    .is_none_or(|s| s.strip_prefix("docker.io/").unwrap_or(s) != tag)
            })
            || annotations
                .get("org.opencontainers.image.ref.name")
                .is_some_and(|v| v.as_str() != Some(tag) && v.as_str() != tag.rsplit(':').next())
        {
            return Err(error("UnsafeImageArchive"));
        }
        let mut graph = ArchiveGraph {
            members: &members,
            platform: &image.platform,
            active: Default::default(),
            visits: 0,
            runnable: Vec::new(),
            attestation_targets: Vec::new(),
        };
        graph.walk(&roots[0], &Default::default(), 0)?;
        if graph.runnable.len() != 1 {
            return Err(error("UnsafeImageArchive"));
        }
        let selected = &graph.runnable[0];
        if selected.config_digest != config_member.digest
            || selected.layer_paths != layer_paths
            || graph
                .attestation_targets
                .iter()
                .any(|target| target != &selected.manifest_digest)
        {
            return Err(error("ImageIdentityMismatch"));
        }
        ids = selected.ids.clone();
    } else {
        if members.contains_key("oci-layout") {
            return Err(error("InvalidImageArchive"));
        }
        // Classic Docker saves use uncompressed layer.tar files. If diff IDs
        // are supplied they must link the config to these exact ordered bytes.
        if let Some(diff_ids) = config["rootfs"].get("diff_ids") {
            let diff_ids = diff_ids
                .as_array()
                .filter(|v| v.len() == layer_paths.len())
                .ok_or_else(|| error("InvalidImageArchive"))?;
            if layer_paths
                .iter()
                .zip(diff_ids)
                .any(|(path, id)| id.as_str() != Some(&members[path].digest))
            {
                return Err(error("ImageIdentityMismatch"));
            }
        }
    }
    let locked_id = image
        .expected_image_id
        .as_deref()
        .ok_or_else(|| error("InvalidImageArtifact"))?;
    if !ids.contains(locked_id) {
        return Err(error("ImageIdentityMismatch"));
    }
    Ok(VerifiedArchiveIdentity {
        ids,
        archive_sha256: image.archive_sha256.clone().unwrap(),
        locked_id: locked_id.into(),
        platform: image.platform.clone(),
        tag: tag.into(),
    })
}
async fn inspect(
    runner: &DockerRunner,
    reference: &str,
    image: &ImageArtifactRef,
    archive_identity: Option<&VerifiedArchiveIdentity>,
) -> Result<VerifiedImage, ToolboxError> {
    let output = runner.run_read(&["image", "inspect", reference]).await?;
    let json: serde_json::Value =
        serde_json::from_slice(&output).map_err(|_| error("DockerOutputInvalid"))?;
    let items = json
        .as_array()
        .filter(|items| items.len() == 1)
        .ok_or_else(|| error("DockerOutputInvalid"))?;
    let item = &items[0];
    if item["Os"] != "linux"
        || format!(
            "linux/{}",
            normalize_architecture(item["Architecture"].as_str().unwrap_or(""))
                .map_err(|_| error("ImageArchitectureMismatch"))?
        ) != image.platform
    {
        return Err(error("ImageArchitectureMismatch"));
    }
    let id = item["Id"]
        .as_str()
        .filter(|s| digest(s, true))
        .ok_or_else(|| error("ImageIdentityMismatch"))?;
    if image.kind == "release-image-archive"
        && (!archive_identity.is_some_and(|verified| verified.permits(id, image))
            || !item["RepoTags"].as_array().is_some_and(|tags| {
                tags.iter().any(|tag| {
                    tag.as_str().is_some_and(|tag| {
                        Some(tag.strip_prefix("docker.io/").unwrap_or(tag))
                            == image.local_content_tag.as_deref()
                    })
                })
            }))
    {
        return Err(error("ImageIdentityMismatch"));
    }
    if image.kind == "registry"
        && !item["RepoDigests"].as_array().is_some_and(|refs| {
            refs.iter().any(|r| {
                r.as_str().is_some_and(|r| {
                    r.ends_with(image.manifest_digest.as_deref().unwrap_or("unresolved"))
                })
            })
        })
    {
        return Err(error("ImageIdentityMismatch"));
    }
    Ok(VerifiedImage {
        source_id: image.source_id.clone(),
        target_id: runner.target.target_id.clone(),
        platform: image.platform.clone(),
        image_id: id.into(),
        runtime_reference: reference.into(),
    })
}
pub async fn ensure_service_image(
    runner: &DockerRunner,
    image: &ImageArtifactRef,
    context: &ImageContext,
) -> Result<VerifiedImage, ToolboxError> {
    validate_image_ref(image, &runner.target.architecture)?;
    runner.verify_identity().await?;
    if context.cancelled.load(Ordering::Relaxed) {
        return Err(error("Cancelled"));
    }
    let reference = if image.kind == "registry" {
        format!(
            "{}@{}",
            image.registry_ref.as_ref().unwrap(),
            image.manifest_digest.as_ref().unwrap()
        )
    } else {
        image.local_content_tag.clone().unwrap()
    };
    let mut verified_archive = None;
    if image.kind == "registry" {
        if let Ok(verified) = inspect(runner, &reference, image, None).await {
            return Ok(verified);
        }
        let args = vec![
            "image".into(),
            "pull".into(),
            "--quiet".into(),
            "--platform".into(),
            image.platform.clone().into(),
            reference.clone().into(),
        ];
        tokio::select! {result=runner.run_args(&args,Duration::from_secs(1800))=>{result?;},_=context.cancellation()=>{return Err(error("Cancelled"));}}
    } else {
        private_dir(&context.cache_root)?;
        let sha = image.archive_sha256.as_ref().unwrap();
        let archive = context.cache_root.join(format!("{sha}.tar"));
        let lock = open_private(&context.cache_root.join(format!("{sha}.lock")), false)?;
        let start = std::time::Instant::now();
        while lock.try_lock().is_err() {
            if context.cancelled.load(Ordering::Relaxed) {
                return Err(error("Cancelled"));
            }
            if start.elapsed() > Duration::from_secs(1800) {
                return Err(error("TimedOut"));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        if !archive.exists() {
            let temporary = context
                .cache_root
                .join(format!("{sha}.partial-{}", crate::workspace::random_id()?));
            let file = open_private(&temporary, true)?;
            let mut file = tokio::fs::File::from_std(file);
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(1800))
                .redirect(reqwest::redirect::Policy::custom(|attempt| {
                    let url = attempt.url();
                    if attempt.previous().len() < 5
                        && url.scheme() == "https"
                        && matches!(
                            url.host_str(),
                            Some(
                                "github.com"
                                    | "release-assets.githubusercontent.com"
                                    | "objects.githubusercontent.com"
                            )
                        )
                    {
                        attempt.follow()
                    } else {
                        attempt.stop()
                    }
                }))
                .build()
                .map_err(|_| error("DownloadUnavailable"))?;
            let result = async {
                let mut response = client
                    .get(image.archive_url.as_ref().unwrap())
                    .send()
                    .await
                    .map_err(|_| error("ArtifactDownloadFailed"))?;
                if response.status() == reqwest::StatusCode::UNAUTHORIZED
                    || response.status() == reqwest::StatusCode::FORBIDDEN
                {
                    return Err(error("ArtifactAuthenticationRequired"));
                }
                if !response.status().is_success() {
                    return Err(error("ArtifactDownloadFailed"));
                }
                let mut size = 0u64;
                while let Some(chunk) = response
                    .chunk()
                    .await
                    .map_err(|_| error("ArtifactDownloadFailed"))?
                {
                    size += chunk.len() as u64;
                    if size > 8 * 1024 * 1024 * 1024 {
                        return Err(error("ArchiveTooLarge"));
                    }
                    file.write_all(&chunk)
                        .await
                        .map_err(|_| error("InsufficientStorage"))?;
                }
                file.sync_all()
                    .await
                    .map_err(|_| error("StateWriteFailed"))?;
                Ok(())
            };
            let result =
                tokio::select! {r=result=>r,_=context.cancellation()=>Err(error("Cancelled"))};
            drop(file);
            if let Err(e) = result {
                let _ = tokio::fs::remove_file(&temporary).await;
                return Err(e);
            }
            let candidate = temporary.clone();
            let spec = image.clone();
            let result = tokio::task::spawn_blocking(move || validate_archive(&candidate, &spec))
                .await
                .map_err(|_| error("InvalidImageArchive"))?;
            if let Err(e) = result {
                let _ = tokio::fs::remove_file(&temporary).await;
                return Err(e);
            }
            tokio::fs::rename(&temporary, &archive)
                .await
                .map_err(|_| error("StateWriteFailed"))?;
        }
        let candidate = archive.clone();
        let spec = image.clone();
        verified_archive = Some(
            tokio::task::spawn_blocking(move || validate_archive_identity(&candidate, &spec))
                .await
                .map_err(|_| error("InvalidImageArchive"))??,
        );
        runner.verify_identity().await?;
        // A cached local tag is usable only after this exact archive SHA and
        // descriptor chain have been verified; inspect output cannot mint IDs.
        if let Ok(verified) = inspect(runner, &reference, image, verified_archive.as_ref()).await {
            return Ok(verified);
        }
        let args = vec![
            "image".into(),
            "load".into(),
            "--input".into(),
            archive.into_os_string(),
        ];
        tokio::select! {r=runner.run_args(&args,Duration::from_secs(1800))=>{r?;},_=context.cancellation()=>{return Err(error("Cancelled"));}}
    }
    runner.verify_identity().await?;
    inspect(runner, &reference, image, verified_archive.as_ref()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    fn image() -> ImageArtifactRef {
        ImageArtifactRef {
            source_id: "test".into(),
            kind: "release-image-archive".into(),
            platform: "linux/arm64".into(),
            registry_ref: None,
            manifest_digest: None,
            archive_url: Some(
                "https://github.com/StephenQiu30/code-ark/releases/download/v0/test.tar".into(),
            ),
            archive_sha256: Some("a".repeat(64)),
            expected_image_id: Some(format!("sha256:{}", "b".repeat(64))),
            local_content_tag: Some(format!("codeark/test:sha256-{}", "b".repeat(64))),
            status: "resolved".into(),
        }
    }
    #[test]
    fn bad_platform_source_and_mutually_exclusive_fields_fail() {
        let mut i = image();
        assert!(validate_image_ref(&i, "aarch64").is_ok());
        assert_eq!(
            validate_image_ref(&i, "amd64").unwrap_err().code,
            "ImageArchitectureMismatch"
        );
        i.archive_url = Some("https://evil.example/test.tar".into());
        assert!(validate_image_ref(&i, "arm64").is_err());
        i = image();
        i.registry_ref = Some("redis:latest".into());
        assert!(validate_image_ref(&i, "arm64").is_err());
        i = image();
        i.expected_image_id = None;
        assert!(validate_image_ref(&i, "arm64").is_err());
    }
    fn append(builder: &mut tar::Builder<std::fs::File>, name: &str, bytes: &[u8]) {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o600);
        header.set_cksum();
        builder.append_data(&mut header, name, bytes).unwrap();
    }
    #[test]
    fn archive_checks_hash_config_id_cpu_and_exact_own_tag() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("image.tar");
        let config = br#"{"architecture":"arm64","os":"linux"}"#;
        let image_id = format!("sha256:{:x}", Sha256::digest(config));
        let tag = format!(
            "codeark/test:sha256-{}",
            image_id.trim_start_matches("sha256:")
        );
        let manifest = serde_json::json!([{"Config":"config.json","RepoTags":[tag],"Layers":[]}]);
        let mut builder = tar::Builder::new(std::fs::File::create(&file).unwrap());
        append(&mut builder, "config.json", config);
        append(
            &mut builder,
            "manifest.json",
            manifest.to_string().as_bytes(),
        );
        builder.finish().unwrap();
        drop(builder);
        let mut i = image();
        i.expected_image_id = Some(image_id);
        i.local_content_tag = Some(tag);
        i.archive_sha256 = Some(format!(
            "{:x}",
            Sha256::digest(std::fs::read(&file).unwrap())
        ));
        assert!(validate_archive(&file, &i).is_ok());
        i.expected_image_id = Some(format!("sha256:{}", "0".repeat(64)));
        assert!(validate_archive(&file, &i).is_err());
        i.archive_sha256 = Some("0".repeat(64));
        assert_eq!(
            validate_archive(&file, &i).unwrap_err().code,
            "ArchiveHashMismatch"
        );
    }
    struct OciFixture {
        _root: tempfile::TempDir,
        path: PathBuf,
        spec: ImageArtifactRef,
        config_id: String,
        manifest_id: String,
        index_id: String,
        decoy_id: String,
    }
    fn fixture_digest(bytes: &[u8]) -> String {
        format!("sha256:{:x}", Sha256::digest(bytes))
    }
    fn fixture_descriptor(media: &str, bytes: &[u8]) -> serde_json::Value {
        serde_json::json!({"mediaType":media,"digest":fixture_digest(bytes),"size":bytes.len()})
    }
    // Mirrors Docker's dual OCI+legacy save layout; every digest is calculated
    // over the actual fixture bytes, never through the validator under test.
    fn oci_fixture(anchor: &str, corrupt_layer: bool, split_legacy: bool) -> OciFixture {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("image.tar");
        let layer = b"fixture layer bytes";
        let config = br#"{"architecture":"arm64","os":"linux"}"#;
        let decoy = br#"{"architecture":"arm64","os":"linux","config":{"Env":["DECOY=1"]}}"#;
        let config_id = fixture_digest(config);
        let decoy_id = fixture_digest(decoy);
        let manifest = serde_json::json!({"schemaVersion":2,"mediaType":"application/vnd.oci.image.manifest.v1+json","config":fixture_descriptor("application/vnd.oci.image.config.v1+json",config),"layers":[fixture_descriptor("application/vnd.oci.image.layer.v1.tar",layer)]}).to_string().into_bytes();
        let manifest_id = fixture_digest(&manifest);
        let mut manifest_descriptor =
            fixture_descriptor("application/vnd.oci.image.manifest.v1+json", &manifest);
        manifest_descriptor["platform"] = serde_json::json!({"os":"linux","architecture":"arm64"});
        let index = serde_json::json!({"schemaVersion":2,"mediaType":"application/vnd.oci.image.index.v1+json","manifests":[manifest_descriptor]}).to_string().into_bytes();
        let index_id = fixture_digest(&index);
        let selected = match anchor {
            "config" => &config_id,
            "manifest" => &manifest_id,
            "decoy" => &decoy_id,
            _ => &index_id,
        };
        let tag = format!(
            "codeark/test:sha256-{}",
            selected.trim_start_matches("sha256:")
        );
        let mut root_descriptor =
            fixture_descriptor("application/vnd.oci.image.index.v1+json", &index);
        root_descriptor["annotations"] = serde_json::json!({"io.containerd.image.name":format!("docker.io/{tag}"),"org.opencontainers.image.ref.name":tag.split(':').next_back().unwrap()});
        let top_index = serde_json::json!({"schemaVersion":2,"mediaType":"application/vnd.oci.image.index.v1+json","manifests":[root_descriptor]});
        let blob_path = |id: &str| format!("blobs/sha256/{}", id.trim_start_matches("sha256:"));
        let legacy = serde_json::json!([{"Config":blob_path(if split_legacy{&decoy_id}else{&config_id}),"RepoTags":[tag],"Layers":[blob_path(&fixture_digest(layer))]}]);
        let mut builder = tar::Builder::new(File::create(&path).unwrap());
        append(
            &mut builder,
            "oci-layout",
            br#"{"imageLayoutVersion":"1.0.0"}"#,
        );
        append(&mut builder, "index.json", top_index.to_string().as_bytes());
        append(&mut builder, "manifest.json", legacy.to_string().as_bytes());
        for (id, bytes) in [
            (&config_id, config.as_slice()),
            (&decoy_id, decoy.as_slice()),
            (&manifest_id, manifest.as_slice()),
            (&index_id, index.as_slice()),
        ] {
            append(&mut builder, &blob_path(id), bytes);
        }
        append(
            &mut builder,
            &blob_path(&fixture_digest(layer)),
            if corrupt_layer {
                b"corrupted layer bytes"
            } else {
                layer
            },
        );
        builder.finish().unwrap();
        drop(builder);
        let mut spec = image();
        spec.expected_image_id = Some(selected.clone());
        spec.local_content_tag = Some(tag);
        spec.archive_sha256 = Some(file_hash(&path).unwrap());
        OciFixture {
            _root: root,
            path,
            spec,
            config_id,
            manifest_id,
            index_id,
            decoy_id,
        }
    }
    #[test]
    fn oci_archive_accepts_the_locked_index_manifest_or_config_on_one_chain() {
        for anchor in ["index", "manifest", "config"] {
            let f = oci_fixture(anchor, false, false);
            assert!(
                validate_archive(&f.path, &f.spec).is_ok(),
                "{anchor} is a verified identity on the same image chain"
            );
        }
    }
    #[test]
    fn oci_archive_rejects_corrupt_layers_and_a_detached_legacy_config() {
        let f = oci_fixture("config", true, false);
        assert!(
            validate_archive(&f.path, &f.spec).is_err(),
            "archive-level SHA cannot replace layer descriptor verification"
        );
        let f = oci_fixture("decoy", false, true);
        assert!(
            validate_archive(&f.path, &f.spec).is_err(),
            "legacy Config must match the OCI image chain"
        );
    }
    #[test]
    fn oci_archive_never_accepts_an_unreferenced_config_as_an_identity() {
        let f = oci_fixture("decoy", false, false);
        assert!(validate_archive(&f.path, &f.spec).is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn cached_image_cannot_skip_archive_hash_and_graph_validation() {
        use std::os::unix::fs::PermissionsExt;
        let f = oci_fixture("config", false, false);
        let cache = f._root.path().join("cache");
        std::fs::create_dir(&cache).unwrap();
        std::fs::write(
            cache.join(format!("{}.tar", f.spec.archive_sha256.as_ref().unwrap())),
            b"broken cached archive",
        )
        .unwrap();
        let cli = f._root.path().join("fake docker");
        let inspected = serde_json::json!([{"Id":f.config_id,"Os":"linux","Architecture":"arm64","RepoTags":[f.spec.local_content_tag]}]);
        std::fs::write(&cli,format!("#!/bin/sh\nif [ \"$3\" = info ]; then printf '%s' '{{\"ID\":\"fixture\",\"OSType\":\"linux\"}}'; elif [ \"$3\" = context ]; then printf '%s' '[{{\"Endpoints\":{{\"docker\":{{\"Host\":\"unix:///fixture\"}}}}}}]'; else printf '%s' '{inspected}'; fi\n")).unwrap();
        std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o700)).unwrap();
        let runner = DockerRunner {
            target: crate::contracts::DockerTarget {
                target_id: "fixture-target".into(),
                cli_path: cli.to_str().unwrap().into(),
                context_name: "fixture".into(),
                endpoint_kind: "unix".into(),
                daemon_id: "fixture".into(),
                os_type: "linux".into(),
                architecture: "arm64".into(),
            },
        };
        let result = ensure_service_image(
            &runner,
            &f.spec,
            &ImageContext {
                cache_root: cache,
                cancelled: Arc::new(AtomicBool::new(false)),
            },
        )
        .await;
        assert_eq!(result.unwrap_err().code, "ArchiveHashMismatch");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn verified_cached_archive_accepts_config_manifest_or_index_store_ids_only() {
        use std::os::unix::fs::PermissionsExt;
        for store_id in ["config", "manifest", "index", "decoy"] {
            let f = oci_fixture("index", false, false);
            let cache = f._root.path().join("cache");
            std::fs::create_dir(&cache).unwrap();
            std::fs::copy(
                &f.path,
                cache.join(format!("{}.tar", f.spec.archive_sha256.as_ref().unwrap())),
            )
            .unwrap();
            let cli = f._root.path().join("fake docker");
            let id = match store_id {
                "config" => &f.config_id,
                "manifest" => &f.manifest_id,
                "index" => &f.index_id,
                _ => &f.decoy_id,
            };
            let inspected = serde_json::json!([{"Id":id,"Os":"linux","Architecture":"arm64","RepoTags":[f.spec.local_content_tag]}]);
            std::fs::write(&cli,format!("#!/bin/sh\nif [ \"$3\" = info ]; then printf '%s' '{{\"ID\":\"fixture\",\"OSType\":\"linux\"}}'; elif [ \"$3\" = context ]; then printf '%s' '[{{\"Endpoints\":{{\"docker\":{{\"Host\":\"unix:///fixture\"}}}}}}]'; elif [ \"$4\" = inspect ]; then printf '%s' '{inspected}'; else exit 19; fi\n")).unwrap();
            std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o700)).unwrap();
            let runner = DockerRunner {
                target: crate::contracts::DockerTarget {
                    target_id: "fixture-target".into(),
                    cli_path: cli.to_str().unwrap().into(),
                    context_name: "fixture".into(),
                    endpoint_kind: "unix".into(),
                    daemon_id: "fixture".into(),
                    os_type: "linux".into(),
                    architecture: "arm64".into(),
                },
            };
            let result = ensure_service_image(
                &runner,
                &f.spec,
                &ImageContext {
                    cache_root: cache,
                    cancelled: Arc::new(AtomicBool::new(false)),
                },
            )
            .await;
            if store_id == "decoy" {
                assert!(
                    result.is_err(),
                    "an unreferenced ARM config is never an allowed store identity"
                );
            } else {
                assert_eq!(
                    result.unwrap().image_id,
                    *id,
                    "cached {store_id} identity must avoid Docker load after verifying the same archive"
                );
            }
        }
    }

    #[test]
    #[ignore = "read-only real ARM producer archives; requires ignored .tools/artifacts"]
    fn native_producer_archives_verify_all_three_image_store_identities() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        for id in ["xxjob", "es-ik"] {
            let path = root.join(format!(".tools/artifacts/{id}-arm64.tar"));
            let receipt: serde_json::Value =
                serde_json::from_slice(&std::fs::read(path.with_extension("tar.json")).unwrap())
                    .unwrap();
            let mut spec = image();
            spec.source_id = id.into();
            spec.platform = receipt["platform"].as_str().unwrap().into();
            spec.expected_image_id = Some(receipt["expectedImageId"].as_str().unwrap().into());
            spec.archive_sha256 = Some(receipt["archiveSha256"].as_str().unwrap().into());
            spec.local_content_tag = Some(receipt["localContentTag"].as_str().unwrap().into());
            let verified = validate_archive_identity(&path, &spec).unwrap();
            assert_eq!(verified.ids.len(), 3);
            for kind in ["inspectIndexId", "platformManifestDigest", "configDigest"] {
                assert!(
                    verified.permits(receipt["archiveIdentity"][kind].as_str().unwrap(), &spec),
                    "{id}: {kind}"
                );
            }
        }
    }
    fn rewrite_fixture_metadata(
        mut f: OciFixture,
        name: &str,
        update: impl FnOnce(&mut serde_json::Value),
        duplicate: bool,
    ) -> OciFixture {
        let mut entries = tar::Archive::new(File::open(&f.path).unwrap())
            .entries()
            .unwrap()
            .map(|entry| {
                let mut entry = entry.unwrap();
                let path = entry.path().unwrap().to_str().unwrap().to_owned();
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes).unwrap();
                (path, bytes)
            })
            .collect::<Vec<_>>();
        let (_, bytes) = entries.iter_mut().find(|(path, _)| path == name).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        update(&mut value);
        *bytes = value.to_string().into_bytes();
        let duplicate_bytes = bytes.clone();
        if duplicate {
            entries.push((name.into(), duplicate_bytes));
        }
        let path = f._root.path().join("rewritten.tar");
        let mut builder = tar::Builder::new(File::create(&path).unwrap());
        for (name, bytes) in entries {
            append(&mut builder, &name, &bytes);
        }
        builder.finish().unwrap();
        drop(builder);
        std::fs::rename(&path, &f.path).unwrap();
        f.spec.archive_sha256 = Some(file_hash(&f.path).unwrap());
        f
    }
    #[test]
    fn oci_top_index_requires_the_declared_index_media_type() {
        let f = rewrite_fixture_metadata(
            oci_fixture("config", false, false),
            "index.json",
            |v| v["mediaType"] = "application/vnd.oci.image.manifest.v1+json".into(),
            false,
        );
        assert!(validate_archive(&f.path, &f.spec).is_err());
    }
    #[test]
    fn archive_rejects_duplicate_paths_wrong_descriptor_size_and_wrong_root_tag() {
        let f = rewrite_fixture_metadata(
            oci_fixture("config", false, false),
            "manifest.json",
            |_| {},
            true,
        );
        assert_eq!(
            validate_archive(&f.path, &f.spec).unwrap_err().code,
            "UnsafeImageArchive"
        );
        let f = rewrite_fixture_metadata(
            oci_fixture("config", false, false),
            "index.json",
            |v| v["manifests"][0]["size"] = 1.into(),
            false,
        );
        assert!(validate_archive(&f.path, &f.spec).is_err());
        let f = rewrite_fixture_metadata(
            oci_fixture("config", false, false),
            "index.json",
            |v| {
                v["manifests"][0]["annotations"]["io.containerd.image.name"] =
                    "docker.io/unrelated/image:tag".into()
            },
            false,
        );
        assert_eq!(
            validate_archive(&f.path, &f.spec).unwrap_err().code,
            "UnsafeImageArchive"
        );
        let mut f = oci_fixture("config", false, false);
        f.spec.platform = "linux/amd64".into();
        assert_eq!(
            validate_archive(&f.path, &f.spec).unwrap_err().code,
            "ImageArchitectureMismatch"
        );
    }
}
