//! Handler functions for the Shaders panel, called from [`super::Window::update`].

use relm4::adw;
use relm4::adw::prelude::*;
use relm4::ComponentController;

use crate::fl;
use crate::reshade::config::ShaderRepo;
use crate::ui::{shader_catalog, shader_worker};

use super::Window;

/// Dispatch a single-repo sync job to the shader worker.
pub(super) fn handle_download_requested(model: &mut Window, repo: ShaderRepo) {
    let data_dir = model.app_state.data_dir.clone();
    model
        .shader_worker
        .emit(shader_worker::Controls::SyncOne { repo, data_dir });
}

/// Forward shader worker progress to the catalog.
pub(super) fn handle_progress(model: &mut Window, msg: String) {
    model
        .shader_catalog
        .emit(shader_catalog::Controls::SyncProgress(msg));
}

/// Notify the catalog that a sync completed.
pub(super) fn handle_sync_complete(model: &mut Window) {
    model
        .shader_catalog
        .emit(shader_catalog::Controls::SyncComplete);
}

/// Notify the catalog that a sync failed.
pub(super) fn handle_sync_error(model: &mut Window, e: String) {
    model
        .shader_catalog
        .emit(shader_catalog::Controls::SyncError(e));
}

/// Present the Add Custom Repo dialog.
pub(super) fn handle_add_custom_repo_requested(model: &mut Window, root: &adw::ApplicationWindow) {
    model.add_shader_repo_dialog.widget().present(Some(root));
}

/// Remove a custom repo from config and disk, then update the catalog.
pub(super) fn handle_remove_custom_repo_requested(model: &mut Window, repo: ShaderRepo) {
    // Remove from persisted config.
    model
        .app_state
        .config
        .shader_repos
        .retain(|r| r.local_name != repo.local_name);
    if let Err(e) = model.app_state.save() {
        log::error!("Failed to save config after removing custom repo: {e}");
    }
    // Delete cloned data from disk.
    let repo_dir = model
        .app_state
        .data_dir
        .join("ReShade_shaders")
        .join(&repo.local_name);
    if repo_dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(&repo_dir) {
            log::error!("Failed to delete repo directory {}: {e}", repo_dir.display());
        }
    }
    // Tell the catalog to remove the row.
    model
        .shader_catalog
        .emit(shader_catalog::Controls::RemoveCustomRepo(repo));
}

/// Validate and persist a newly added custom repo.
pub(super) fn handle_repo_added(model: &mut Window, repo: ShaderRepo) {
    let in_catalog = crate::reshade::catalog::KNOWN_REPOS
        .iter()
        .any(|e| e.url == repo.url || e.local_name == repo.local_name);
    if in_catalog {
        model.toast_overlay.add_toast(adw::Toast::new(
            &fl!("toast-repo-in-catalog"),
        ));
        return;
    }
    let already = model
        .app_state
        .config
        .shader_repos
        .iter()
        .any(|r| r.url == repo.url || r.local_name == repo.local_name);
    if already {
        model
            .toast_overlay
            .add_toast(adw::Toast::new(&fl!("toast-repo-duplicate")));
        return;
    }
    model.app_state.config.shader_repos.push(repo.clone());
    if let Err(e) = model.app_state.save() {
        log::error!("Failed to save config after adding custom repo: {e}");
    }
    model
        .shader_catalog
        .emit(shader_catalog::Controls::AddCustomRepo(repo));
}
