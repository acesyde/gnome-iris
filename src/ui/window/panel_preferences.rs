//! Handler functions for the Preferences panel, called from [`super::Window::update`].

use relm4::adw;
use relm4::ComponentController;

use crate::reshade::cache::UpdateCache;
use crate::reshade::config::GlobalConfig;
use crate::ui::{game_detail, install_worker, preferences};

use super::Window;

/// Persist an updated global config.
pub(super) fn handle_config_changed(model: &mut Window, config: GlobalConfig) {
    model.app_state.config = config;
    if let Err(e) = model.app_state.save() {
        log::error!("Failed to save config: {e}");
    }
}

/// Forward the latest fetched ReShade version to Preferences.
pub(super) fn handle_latest_version_fetched(model: &mut Window, version: String) {
    model
        .preferences
        .emit(preferences::Controls::SetLatestVersion(version));
}

/// Dispatch a version download job to the install worker.
pub(super) fn handle_version_download_requested(model: &mut Window, version_key: String) {
    let (version, addon) = if let Some(base) = version_key.strip_suffix("-Addon") {
        (base.to_owned(), true)
    } else {
        (version_key.clone(), false)
    };
    model
        .install_worker
        .emit(install_worker::Controls::DownloadVersion {
            data_dir: model.app_state.data_dir.clone(),
            version,
            addon,
        });
}

/// Notify Preferences that a version download completed; also sync Window's version list.
pub(super) fn handle_version_download_complete(model: &mut Window, version: String) {
    model.installed_versions.push(version.clone());
    if model.current_game_id.is_some() {
        model
            .game_detail
            .emit(game_detail::Controls::SetInstalledVersions(
                model.installed_versions.clone(),
            ));
    }
    model
        .preferences
        .emit(preferences::Controls::VersionDownloadComplete(version));
}

/// Log and surface a version download error.
pub(super) fn handle_version_download_error(model: &mut Window, e: String) {
    log::error!("Version download failed: {e}");
    model
        .preferences
        .emit(preferences::Controls::VersionOpError(e.clone()));
    model
        .toast_overlay
        .add_toast(adw::Toast::new(&format!("Download failed: {e}")));
}

/// Remove a cached ReShade version from disk and notify Preferences.
pub(super) fn handle_version_remove_requested(model: &mut Window, version: String) {
    let data_dir = &model.app_state.data_dir;
    let version_dir = crate::reshade::reshade::version_dir(data_dir, &version);
    if version_dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(&version_dir) {
            log::error!("Failed to remove ReShade version {version}: {e}");
            model
                .preferences
                .emit(preferences::Controls::VersionOpError(e.to_string()));
            return;
        }
    }
    let cache = UpdateCache::new(data_dir.clone());
    if let Err(e) = cache.remove_installed(&version) {
        log::warn!("Could not update installed versions cache after removal: {e}");
    }
    model
        .preferences
        .emit(preferences::Controls::VersionRemoveComplete(version.clone()));
    // Sync Window's version list and refresh the detail pane if open.
    model.installed_versions.retain(|v| v != &version);
    if model.current_game_id.is_some() {
        model
            .game_detail
            .emit(game_detail::Controls::SetInstalledVersions(
                model.installed_versions.clone(),
            ));
    }
}
