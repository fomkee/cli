use std::cmp::Ordering;
use std::env::consts;
use std::io::{self, Write};
use std::path::Path;
use std::str;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::CliError;

pub mod cache;
mod github;
mod install;
use cache::{CacheEntry, FileUpdateCache, UpdateCache};
pub use github::GitHubReleases;
pub use install::{ExecutableInstaller, RunningExecutable};

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const CHECK_INTERVAL: u64 = 86_400;

/// GitHub release metadata needed for version selection and asset verification.
#[derive(Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub draft: bool,
    pub prerelease: bool,
    pub assets: Vec<Asset>,
}
/// Published asset metadata; URLs must match the canonical repository before download.
#[derive(Clone, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}
/// External release discovery and bounded asset downloads.
#[async_trait]
pub trait ReleaseSource: Send + Sync {
    /// Obtain GitHub's latest stable release.
    async fn latest(&self) -> Result<Release, CliError>;
    /// Download an asset with a hard byte limit.
    async fn download(&self, asset: &Asset, limit: usize) -> Result<Vec<u8>, CliError>;
}
/// Explicit version check or installation outcome, usable without workspace credentials.
#[derive(Debug, Serialize)]
pub struct UpdateResult {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub updated: bool,
}

fn version(value: &str) -> Result<Version, CliError> {
    Version::parse(value.strip_prefix('v').unwrap_or(value))
        .map_err(|_| CliError::InvalidInput("release version is not valid SemVer".into()))
}
fn release_version(release: &Release) -> Result<Version, CliError> {
    let version = version(&release.tag_name)?;
    if release.draft
        || release.prerelease
        || !version.pre.is_empty()
        || release.tag_name != format!("v{version}")
    {
        return Err(CliError::InvalidInput(
            "GitHub release must be a stable vVERSION tag".into(),
        ));
    }
    Ok(version)
}
/// Check or install the latest stable release; never downgrade or install an unchecked asset.
pub async fn execute(
    source: &impl ReleaseSource,
    installer: &impl ExecutableInstaller,
    current: &str,
    platform: &str,
    check_only: bool,
) -> Result<UpdateResult, CliError> {
    let release = source.latest().await?;
    let latest = release_version(&release)?;
    let newer = latest.cmp_precedence(&version(current)?) == Ordering::Greater;
    let mut result = UpdateResult {
        current_version: current.into(),
        latest_version: latest.to_string(),
        update_available: newer,
        updated: false,
    };
    if check_only || !newer {
        return Ok(result);
    }
    let binary = asset(&release, platform)?;
    let checksum = asset(&release, &format!("{platform}.sha256"))?;
    let expected = source.download(checksum, 1024).await?;
    let bytes = source.download(binary, 128 * 1024 * 1024).await?;
    if u64::try_from(bytes.len()).ok() != Some(binary.size) {
        return Err(CliError::InvalidInput(
            "release binary size does not match GitHub metadata; nothing installed".into(),
        ));
    }
    verify_checksum(&bytes, &expected, platform)?;
    installer.install(&bytes)?;
    result.updated = true;
    Ok(result)
}
fn asset<'a>(release: &'a Release, name: &str) -> Result<&'a Asset, CliError> {
    let mut matching = release.assets.iter().filter(|asset| asset.name == name);
    let asset = matching.next().ok_or_else(|| {
        CliError::InvalidInput(format!(
            "GitHub release has no {name} asset; nothing installed"
        ))
    })?;
    let expected = format!(
        "https://github.com/fomkee/cli/releases/download/{}/{name}",
        release.tag_name
    );
    if matching.next().is_some() || asset.browser_download_url != expected || asset.size == 0 {
        return Err(CliError::InvalidInput(
            "invalid or duplicate GitHub release asset; nothing installed".into(),
        ));
    }
    Ok(asset)
}
fn verify_checksum(bytes: &[u8], checksum: &[u8], name: &str) -> Result<(), CliError> {
    let error = || {
        CliError::InvalidInput(
            "release checksum is invalid or does not match; nothing installed".into(),
        )
    };
    let checksum = str::from_utf8(checksum).map_err(|_| error())?;
    let mut fields = checksum.split_whitespace();
    let hash = fields.next().ok_or_else(error)?;
    let filename = fields.next().ok_or_else(error)?;
    if fields.next().is_some()
        || filename.trim_start_matches('*') != name
        || hash.len() != 64
        || !hash.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(error());
    }
    let actual: String = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if !actual.eq_ignore_ascii_case(hash) {
        return Err(error());
    }
    Ok(())
}
fn platform() -> Result<&'static str, CliError> {
    match (consts::OS, consts::ARCH) {
        ("linux", "x86_64") => Ok("fomkeecli-Linux-x86_64"),
        ("macos", "aarch64") => Ok("fomkeecli-Darwin-arm64"),
        ("macos", "x86_64") => Ok("fomkeecli-Darwin-x86_64"),
        ("windows", "x86_64") => Ok("fomkeecli-Windows-x86_64.exe"),
        _ => Err(CliError::InvalidInput(
            "GitHub has no supported binary for this platform; build from source".into(),
        )),
    }
}
pub(crate) async fn command(check_only: bool) -> Result<UpdateResult, CliError> {
    execute(
        &GitHubReleases::new()?,
        &RunningExecutable,
        CURRENT_VERSION,
        platform()?,
        check_only,
    )
    .await
}

/// Return a cached update notification; clock values are supplied so cooldown behavior is testable.
pub async fn notification(
    source: &impl ReleaseSource,
    cache: &impl UpdateCache,
    current: &str,
    now: u64,
) -> Result<Option<String>, CliError> {
    let existing = cache.load()?;
    let fresh = existing.as_ref().is_some_and(|entry| {
        now.checked_sub(entry.checked_at)
            .is_some_and(|elapsed| elapsed < CHECK_INTERVAL)
    });
    let entry = if fresh {
        existing
    } else {
        // Persist the attempt before network I/O so offline invocations share the cooldown.
        let mut entry = CacheEntry {
            checked_at: now,
            latest_version: None,
        };
        cache.save(&entry)?;
        let release = source.latest().await?;
        entry.latest_version = Some(release_version(&release)?.to_string());
        cache.save(&entry)?;
        Some(entry)
    };
    let latest = entry.and_then(|entry| entry.latest_version);
    match latest {
        Some(latest)
            if version(&latest)?.cmp_precedence(&version(current)?) == Ordering::Greater =>
        {
            Ok(Some(format!(
                "fomkeecli {latest} is available (installed: {current}). Run fomkeecli self-update."
            )))
        }
        _ => Ok(None),
    }
}
pub(crate) async fn notify(directory: &Path) {
    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return;
    };
    let Ok(source) = GitHubReleases::new() else {
        return;
    };
    let cache = FileUpdateCache::new(directory.join("update-check.json"));
    let Ok(_lock) = cache.try_lock() else {
        return;
    };
    // Update discovery is best-effort and must not change the completed command's exit status.
    if let Ok(Ok(Some(message))) = tokio::time::timeout(
        Duration::from_secs(2),
        notification(&source, &cache, CURRENT_VERSION, now.as_secs()),
    )
    .await
    {
        let _ = writeln!(io::stderr().lock(), "{message}");
    }
}

#[cfg(test)]
mod tests;
