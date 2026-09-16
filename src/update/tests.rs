use super::*;
use cache::{InMemoryUpdateCache, UpdateCache};
use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

const PLATFORM: &str = "fomkeecli-Linux-x86_64";
const BINARY: &[u8] = b"hello";
const CHECKSUM: &[u8] = b"2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824\n";
struct Source {
    release: Release,
    checksum: Vec<u8>,
    calls: AtomicUsize,
    fail: bool,
}
impl Source {
    fn stable() -> Self {
        Self {
            release: Release {
                tag_name: "v0.2.0".into(),
                draft: false,
                prerelease: false,
                assets: vec![
                    an_asset(PLATFORM, 5),
                    an_asset(&format!("{PLATFORM}.sha256"), 100),
                ],
            },
            checksum: format!("{}  {PLATFORM}\n", String::from_utf8_lossy(CHECKSUM).trim())
                .into_bytes(),
            calls: AtomicUsize::new(0),
            fail: false,
        }
    }
}
fn an_asset(name: &str, size: u64) -> Asset {
    Asset {
        name: name.into(),
        size,
        browser_download_url: format!(
            "https://github.com/fomkee/cli/releases/download/v0.2.0/{name}"
        ),
    }
}
#[async_trait]
impl ReleaseSource for Source {
    async fn latest(&self) -> Result<Release, CliError> {
        self.calls.fetch_add(1, AtomicOrdering::SeqCst);
        if self.fail {
            return Err(CliError::Configuration("offline".into()));
        }
        Ok(self.release.clone())
    }
    async fn download(&self, asset: &Asset, _: usize) -> Result<Vec<u8>, CliError> {
        Ok(if asset.name.ends_with(".sha256") {
            self.checksum.clone()
        } else {
            BINARY.to_vec()
        })
    }
}
#[derive(Default)]
struct Installer(RefCell<Vec<u8>>);
impl ExecutableInstaller for Installer {
    fn install(&self, bytes: &[u8]) -> Result<(), CliError> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(())
    }
}
fn assert_no_install(installer: &Installer) {
    assert!(installer.0.borrow().is_empty());
}

#[tokio::test]
async fn test_installs_only_after_checksum_verification() {
    // Arrange
    let installer = Installer::default();
    // Act
    let result = execute(&Source::stable(), &installer, "0.1.0", PLATFORM, false)
        .await
        .unwrap();
    // Assert
    assert!(result.updated && result.update_available);
    assert_eq!(*installer.0.borrow(), BINARY);
}
#[tokio::test]
async fn test_checksum_mismatch_leaves_executable_unchanged() {
    // Arrange
    let mut source = Source::stable();
    source.checksum = format!("{}  {PLATFORM}", "0".repeat(64)).into_bytes();
    let installer = Installer::default();
    // Act
    let result = execute(&source, &installer, "0.1.0", PLATFORM, false).await;
    // Assert
    assert!(result.is_err());
    assert_no_install(&installer);
}
#[tokio::test]
async fn test_check_only_does_not_install() {
    let installer = Installer::default();
    let result = execute(&Source::stable(), &installer, "0.1.0", PLATFORM, true)
        .await
        .unwrap();
    assert!(result.update_available && !result.updated);
    assert_no_install(&installer);
}
#[tokio::test]
async fn test_never_downgrades_a_newer_installed_version() {
    let installer = Installer::default();
    let result = execute(&Source::stable(), &installer, "0.3.0", PLATFORM, false)
        .await
        .unwrap();
    assert!(!result.update_available && !result.updated);
    assert_no_install(&installer);
}
#[tokio::test]
async fn test_ignores_build_metadata_when_comparing_versions() {
    let installer = Installer::default();
    let result = execute(
        &Source::stable(),
        &installer,
        "0.2.0+local",
        PLATFORM,
        false,
    )
    .await
    .unwrap();
    assert!(!result.update_available);
    assert_no_install(&installer);
}
#[tokio::test]
async fn test_rejects_unpublished_or_prerelease_metadata() {
    let mut source = Source::stable();
    source.release.prerelease = true;
    let installer = Installer::default();
    assert!(
        execute(&source, &installer, "0.1.0", PLATFORM, false)
            .await
            .is_err()
    );
    assert_no_install(&installer);
}
#[tokio::test]
async fn test_missing_platform_asset_does_not_install() {
    let installer = Installer::default();
    assert!(
        execute(&Source::stable(), &installer, "0.1.0", "unsupported", false)
            .await
            .is_err()
    );
    assert_no_install(&installer);
}
#[tokio::test]
async fn test_rejects_asset_from_another_repository() {
    let mut source = Source::stable();
    source
        .release
        .assets
        .first_mut()
        .unwrap()
        .browser_download_url = "https://example.com/cli".into();
    let installer = Installer::default();
    assert!(
        execute(&source, &installer, "0.1.0", PLATFORM, false)
            .await
            .is_err()
    );
    assert_no_install(&installer);
}
#[tokio::test]
async fn test_rejects_wrong_download_size() {
    let mut source = Source::stable();
    source.release.assets.first_mut().unwrap().size = 6;
    let installer = Installer::default();
    assert!(
        execute(&source, &installer, "0.1.0", PLATFORM, false)
            .await
            .is_err()
    );
    assert_no_install(&installer);
}
#[tokio::test]
async fn test_reuses_daily_notification_without_network() {
    let source = Source::stable();
    let cache = InMemoryUpdateCache::default();
    notification(&source, &cache, "0.1.0", 100).await.unwrap();
    let result = notification(&source, &cache, "0.1.0", 200).await.unwrap();
    assert!(result.unwrap().contains("self-update"));
    assert_eq!(source.calls.load(AtomicOrdering::SeqCst), 1);
}
#[tokio::test]
async fn test_refreshes_after_daily_cooldown() {
    let source = Source::stable();
    let cache = InMemoryUpdateCache::default();
    cache
        .save(&CacheEntry {
            checked_at: 100,
            latest_version: None,
        })
        .unwrap();
    assert!(
        notification(&source, &cache, "0.1.0", 86_500)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(source.calls.load(AtomicOrdering::SeqCst), 1);
}
#[tokio::test]
async fn test_offline_failure_does_not_retry_on_every_command() {
    let mut source = Source::stable();
    source.fail = true;
    let cache = InMemoryUpdateCache::default();
    assert!(notification(&source, &cache, "0.1.0", 100).await.is_err());
    assert!(
        notification(&source, &cache, "0.1.0", 200)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(source.calls.load(AtomicOrdering::SeqCst), 1);
}
#[test]
fn test_file_and_memory_cache_preserve_entries() {
    let directory = tempfile::tempdir().unwrap();
    assert_cache_round_trip(&FileUpdateCache::new(directory.path().join("update.json")));
    assert_cache_round_trip(&InMemoryUpdateCache::default());
}
fn assert_cache_round_trip(cache: &impl UpdateCache) {
    assert!(cache.load().unwrap().is_none());
    cache
        .save(&CacheEntry {
            checked_at: 123,
            latest_version: Some("0.2.0".into()),
        })
        .unwrap();
    let loaded = cache.load().unwrap().unwrap();
    assert_eq!(
        (loaded.checked_at, loaded.latest_version.as_deref()),
        (123, Some("0.2.0"))
    );
}

#[test]
fn test_concurrent_update_checks_do_not_share_the_discovery_lock() {
    // Arrange
    let directory = tempfile::tempdir().unwrap();
    let cache = FileUpdateCache::new(directory.path().join("update.json"));
    let lock = cache.try_lock().unwrap();
    // Act
    let competing = cache.try_lock();
    // Assert
    assert!(competing.is_err());
    drop(lock);
    assert!(cache.try_lock().is_ok());
}

#[tokio::test]
async fn test_keeps_development_build_when_published_release_has_older_minor_version() {
    // Arrange
    let mut source = Source::stable();
    source.release.tag_name = "v0.1.3".into();
    let installer = Installer::default();

    // Act
    let result = execute(&source, &installer, "0.2.0-dev.1", PLATFORM, false)
        .await
        .unwrap();

    // Assert
    assert!(!result.update_available && !result.updated);
    assert_no_install(&installer);
}

#[tokio::test]
async fn test_development_build_can_upgrade_to_its_stable_release() {
    // Arrange
    let installer = Installer::default();

    // Act
    let result = execute(
        &Source::stable(),
        &installer,
        "0.2.0-dev.1",
        PLATFORM,
        false,
    )
    .await
    .unwrap();

    // Assert
    assert!(result.update_available && result.updated);
    assert_eq!(*installer.0.borrow(), BINARY);
}
