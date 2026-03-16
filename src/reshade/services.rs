//! Service traits for the domain layer.
//!
//! These traits decouple the UI layer from concrete domain implementations,
//! enabling future unit-testing of UI handlers by injecting mock services.

use std::future::Future;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::reshade::config::ShaderRepo;
use crate::reshade::game::Game;

/// Provides access to `ReShade` version and download operations.
///
/// Implement this trait to supply a mock or alternative backend for testing.
pub trait ReShadeProvider: Send + Sync + 'static {
    /// Fetches the latest `ReShade` version string from the GitHub tags API.
    ///
    /// # Errors
    /// Returns an error if the network request fails or the API returns an empty list.
    #[must_use]
    fn fetch_latest_version(&self) -> impl Future<Output = Result<String>> + Send + 'static;

    /// Downloads and extracts a `ReShade` version to the local cache.
    ///
    /// `version` is a bare version string (e.g. `"6.7.3"`); `addon` selects the
    /// Addon Support variant. Skips the download if the DLL files are already present.
    ///
    /// # Errors
    /// Returns an error if the network request or extraction fails.
    #[must_use]
    fn download_and_extract(&self, version: &str, addon: bool) -> impl Future<Output = Result<()>> + Send + 'static;

    /// Lists all installed `ReShade` version keys found in the local cache.
    ///
    /// # Errors
    /// Returns an error if the versions directory cannot be read.
    fn list_installed_versions(&self) -> Result<Vec<String>>;
}

/// Provides access to the game list and its persistence.
///
/// Implement this trait to supply a mock or alternative storage backend.
pub trait GameRepository: Send + 'static {
    /// Returns all known games.
    fn games(&self) -> &[Game];

    /// Replaces the stored game list and persists it to disk.
    ///
    /// # Errors
    /// Returns an error if serialisation or the disk write fails.
    fn save_games(&mut self, games: &[Game]) -> Result<()>;
}

/// Provides shader repository sync operations.
///
/// Implement this trait to supply a mock or alternative sync backend.
pub trait ShaderSyncService: Send + 'static {
    /// Clones or fast-forward-updates a single shader repository.
    ///
    /// # Errors
    /// Returns an error if the git operation fails.
    fn sync_repo(&self, repo: &ShaderRepo, repos_dir: &Path) -> Result<()>;

    /// Rebuilds the `Merged/` directory from all enabled repos in `repos_dir`.
    ///
    /// # Errors
    /// Returns an error if directory creation or symlinking fails.
    fn rebuild_merged(&self, repos_dir: &Path, disabled_repos: &[String]) -> Result<()>;
}

// ── Default implementations ─────────────────────────────────────────────────

/// Default [`ReShadeProvider`] backed by the `reshade` domain module.
pub struct DefaultReShadeProvider {
    data_dir: PathBuf,
}

impl DefaultReShadeProvider {
    /// Creates a new provider rooted at `data_dir`.
    #[must_use]
    pub const fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }
}

impl ReShadeProvider for DefaultReShadeProvider {
    fn fetch_latest_version(&self) -> impl Future<Output = Result<String>> + Send + 'static {
        crate::reshade::reshade::fetch_latest_version()
    }

    fn download_and_extract(&self, version: &str, addon: bool) -> impl Future<Output = Result<()>> + Send + 'static {
        use crate::reshade::cache::UpdateCache;
        use crate::reshade::game::ExeArch;
        use crate::reshade::reshade;

        let dir_key = if addon { format!("{version}-Addon") } else { version.to_owned() };
        let version = version.to_owned();
        let data_dir = self.data_dir.clone();

        async move {
            let version_dir = reshade::version_dir(&data_dir, &dir_key);
            if !version_dir.join(ExeArch::X86_64.reshade_dll()).exists() {
                let url = reshade::download_url(&version, addon);
                reshade::download_and_extract(&url, &version_dir).await?;
            }
            let cache = UpdateCache::new(data_dir);
            if let Err(e) = cache.add_installed(&dir_key) {
                log::warn!("Could not update installed versions cache: {e}");
            }
            Ok(())
        }
    }

    fn list_installed_versions(&self) -> Result<Vec<String>> {
        crate::reshade::reshade::list_installed_versions(&self.data_dir)
    }
}

/// Default [`ShaderSyncService`] backed by the `shaders` domain module.
pub struct DefaultShaderSyncService;

impl ShaderSyncService for DefaultShaderSyncService {
    fn sync_repo(&self, repo: &ShaderRepo, repos_dir: &Path) -> Result<()> {
        crate::reshade::shaders::sync_repo(repo, repos_dir)
    }

    fn rebuild_merged(&self, repos_dir: &Path, disabled_repos: &[String]) -> Result<()> {
        crate::reshade::shaders::rebuild_merged(repos_dir, disabled_repos)
    }
}

/// Default [`GameRepository`] backed by [`crate::reshade::app_state::AppState`].
pub struct DefaultGameRepository {
    app_state: crate::reshade::app_state::AppState,
}

impl DefaultGameRepository {
    /// Creates a new repository wrapping the given `app_state`.
    #[must_use]
    pub const fn new(app_state: crate::reshade::app_state::AppState) -> Self {
        Self { app_state }
    }
}

impl GameRepository for DefaultGameRepository {
    fn games(&self) -> &[Game] {
        &self.app_state.games
    }

    fn save_games(&mut self, games: &[Game]) -> Result<()> {
        self.app_state.games = games.to_vec();
        self.app_state.save()
    }
}
