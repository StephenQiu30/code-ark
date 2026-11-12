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
pub fn validate_archive(path: &Path, image: &ImageArtifactRef) -> Result<(), ToolboxError> {
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
    let mut metadata = BTreeMap::new();
    let mut total = 0u64;
    let mut count = 0;
    for entry in archive
        .entries()
        .map_err(|_| error("InvalidImageArchive"))?
    {
        let mut entry = entry.map_err(|_| error("InvalidImageArchive"))?;
        count += 1;
        if count > 10000 {
            return Err(error("ArchiveTooLarge"));
        }
        let path = entry
            .path()
            .map_err(|_| error("InvalidImageArchive"))?
            .into_owned();
        if path.is_absolute()
            || path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            || entry.header().entry_type().is_symlink()
            || entry.header().entry_type().is_hard_link()
        {
            return Err(error("UnsafeImageArchive"));
        }
        if entry.size() <= 2 * 1024 * 1024 && entry.header().entry_type().is_file() {
            total += entry.size();
            if total > 32 * 1024 * 1024 {
                return Err(error("ArchiveTooLarge"));
            }
            let mut bytes = vec![];
            entry
                .read_to_end(&mut bytes)
                .map_err(|_| error("InvalidImageArchive"))?;
            metadata.insert(path.to_string_lossy().into_owned(), bytes);
        }
    }
    let manifest: serde_json::Value = serde_json::from_slice(
        metadata
            .get("manifest.json")
            .ok_or_else(|| error("InvalidImageArchive"))?,
    )
    .map_err(|_| error("InvalidImageArchive"))?;
    let entries = manifest
        .as_array()
        .filter(|a| a.len() == 1)
        .ok_or_else(|| error("UnsafeImageArchive"))?;
    let entry = &entries[0];
    let tags = entry["RepoTags"]
        .as_array()
        .filter(|a| a.len() == 1)
        .ok_or_else(|| error("UnsafeImageArchive"))?;
    if tags[0].as_str() != image.local_content_tag.as_deref() {
        return Err(error("UnsafeImageArchive"));
    }
    let config = metadata
        .get(
            entry["Config"]
                .as_str()
                .ok_or_else(|| error("InvalidImageArchive"))?,
        )
        .ok_or_else(|| error("InvalidImageArchive"))?;
    if Some(format!("sha256:{:x}", Sha256::digest(config))) != image.expected_image_id {
        return Err(error("ImageIdentityMismatch"));
    }
    let config: serde_json::Value =
        serde_json::from_slice(config).map_err(|_| error("InvalidImageArchive"))?;
    if config["os"] != "linux"
        || format!("linux/{}", config["architecture"].as_str().unwrap_or("")) != image.platform
    {
        return Err(error("ImageArchitectureMismatch"));
    }
    Ok(())
}
async fn inspect(
    runner: &DockerRunner,
    reference: &str,
    image: &ImageArtifactRef,
) -> Result<VerifiedImage, ToolboxError> {
    let output = runner.run_read(&["image", "inspect", reference]).await?;
    let json: serde_json::Value =
        serde_json::from_slice(&output).map_err(|_| error("DockerOutputInvalid"))?;
    let item = &json[0];
    if item["Os"] != "linux"
        || format!("linux/{}", item["Architecture"].as_str().unwrap_or("")) != image.platform
    {
        return Err(error("ImageArchitectureMismatch"));
    }
    let id = item["Id"]
        .as_str()
        .filter(|s| digest(s, true))
        .ok_or_else(|| error("ImageIdentityMismatch"))?;
    if image.kind == "release-image-archive" && Some(id) != image.expected_image_id.as_deref() {
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
    if let Ok(verified) = inspect(runner, &reference, image).await {
        return Ok(verified);
    }
    if image.kind == "registry" {
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
        tokio::task::spawn_blocking(move || validate_archive(&candidate, &spec))
            .await
            .map_err(|_| error("InvalidImageArchive"))??;
        runner.verify_identity().await?;
        let args = vec![
            "image".into(),
            "load".into(),
            "--input".into(),
            archive.into_os_string(),
        ];
        tokio::select! {r=runner.run_args(&args,Duration::from_secs(1800))=>{r?;},_=context.cancellation()=>{return Err(error("Cancelled"));}}
    }
    runner.verify_identity().await?;
    inspect(runner, &reference, image).await
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
}
