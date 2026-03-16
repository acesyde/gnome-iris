//! Handler functions for the Preferences panel, called from [`super::Window::update`].

use relm4::ComponentController;
use relm4::adw;

use crate::reshade::cache::UpdateCache;
use crate::reshade::config::GlobalConfig;
use crate::reshade::game::InstallStatus;
use crate::ui::{game_detail, game_list, install_worker, preferences};

use super::Window;

/// Messages handled by the Preferences panel.
#[derive(Debug)]
pub enum PrefsMsg {
    /// Preferences emitted a config change; carry updated config.
    ConfigChanged(GlobalConfig),
    /// Latest `ReShade` version was fetched from GitHub; forward to Preferences.
    LatestVersionFetched(String),
    /// Preferences requested downloading a version to the local cache.
    VersionDownloadRequested(String),
    /// Install worker completed a version-only download.
    VersionDownloadComplete(String),
    /// Install worker failed a version-only download.
    VersionDownloadError(String),
    /// Preferences requested removing a cached version.
    VersionRemoveRequested(String),
}

/// Dispatch a [`PrefsMsg`] to the appropriate handler.
pub(super) fn handle(model: &mut Window, msg: PrefsMsg) {
    match msg {
        PrefsMsg::ConfigChanged(config) => handle_config_changed(model, config),
        PrefsMsg::LatestVersionFetched(version) => handle_latest_version_fetched(model, &version),
        PrefsMsg::VersionDownloadRequested(version_key) => handle_version_download_requested(model, &version_key),
        PrefsMsg::VersionDownloadComplete(version) => handle_version_download_complete(model, version),
        PrefsMsg::VersionDownloadError(e) => handle_version_download_error(model, &e),
        PrefsMsg::VersionRemoveRequested(version) => handle_version_remove_requested(model, &version),
    }
}

/// Persist an updated global config.
pub(super) fn handle_config_changed(model: &mut Window, config: GlobalConfig) {
    model.app_state.config = config;
    model.save_or_toast();
}

/// Store the latest version, forward to Preferences, and refresh pill visibility on all installed games.
pub(super) fn handle_latest_version_fetched(model: &mut Window, version: &str) {
    let version_owned = version.to_string();
    model.latest_version = Some(version_owned.clone());
    model.preferences.emit(preferences::Controls::SetLatestVersion(version_owned.clone()));

    // Collect before emitting to satisfy the borrow checker:
    // iterating `model.games` (immutable) and calling `model.game_list.emit` (mutable)
    // cannot happen in the same loop body on the same `&mut Window`.
    let installed: Vec<(String, Option<String>)> = model
        .games
        .iter()
        .filter_map(|g| match &g.status {
            InstallStatus::Installed { version: v, .. } => Some((g.id.clone(), v.clone())),
            InstallStatus::NotInstalled => None,
        })
        .collect();

    for (id, installed_version) in installed {
        model.game_list.emit(game_list::Controls::SetGameStatus {
            id,
            version: installed_version,
            latest_version: Some(version_owned.clone()),
        });
    }
}

/// Dispatch a version download job to the install worker.
pub(super) fn handle_version_download_requested(model: &Window, version_key: &str) {
    let (version, addon) = version_key
        .strip_suffix("-Addon")
        .map_or_else(|| (version_key.to_owned(), false), |base| (base.to_owned(), true));
    model.install_worker.emit(install_worker::Controls::DownloadVersion { version, addon });
}

/// Notify Preferences that a version download completed; also sync Window's version list.
pub(super) fn handle_version_download_complete(model: &mut Window, version: String) {
    model.installed_versions.push(version.clone());
    if model.current_game_id.is_some() {
        model
            .game_detail
            .emit(game_detail::Controls::SetInstalledVersions(model.installed_versions.clone()));
    }
    model.preferences.emit(preferences::Controls::VersionDownloadComplete(version));
}

/// Log and surface a version download error.
pub(super) fn handle_version_download_error(model: &Window, e: &str) {
    log::error!("Version download failed: {e}");
    model.preferences.emit(preferences::Controls::VersionOpError(e.to_owned()));
    model.toast_overlay.add_toast(adw::Toast::new(&format!("Download failed: {e}")));
}

/// Remove a cached `ReShade` version from disk and notify Preferences.
pub(super) fn handle_version_remove_requested(model: &mut Window, version: &str) {
    let data_dir = &model.app_state.data_dir;
    let version_dir = crate::reshade::reshade::version_dir(data_dir, version);
    if version_dir.exists()
        && let Err(e) = std::fs::remove_dir_all(&version_dir)
    {
        log::error!("Failed to remove ReShade version {version}: {e}");
        model.preferences.emit(preferences::Controls::VersionOpError(e.to_string()));
        return;
    }
    let cache = UpdateCache::new(data_dir.clone());
    if let Err(e) = cache.remove_installed(version) {
        log::warn!("Could not update installed versions cache after removal: {e}");
    }
    model.preferences.emit(preferences::Controls::VersionRemoveComplete(version.to_owned()));
    // Sync Window's version list and refresh the detail pane if open.
    model.installed_versions.retain(|v| v != version);
    if model.current_game_id.is_some() {
        model
            .game_detail
            .emit(game_detail::Controls::SetInstalledVersions(model.installed_versions.clone()));
    }
}
