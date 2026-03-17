//! Handler functions for the Shaders panel, called from [`super::Window::update`].

use relm4::ComponentController;
use relm4::adw;
use relm4::adw::prelude::*;

use crate::fl;
use crate::reshade::config::ShaderRepo;
use crate::ui::worker_types::ProgressEvent;
use crate::ui::{add_shader_repo_dialog, shader_catalog, shader_worker};

use super::Window;

/// Messages handled by the Shaders panel.
#[derive(Debug)]
pub enum ShadersMsg {
    /// User clicked the download button for a catalog or custom repo.
    ///
    /// Handler forwards to the shader worker as a single-repo sync job.
    DownloadRequested(ShaderRepo),
    /// Shader worker reported a progress event for the in-flight sync.
    Progress(ProgressEvent),
    /// Shader worker finished syncing the in-flight repo.
    ///
    /// Forwarded to the catalog so it can stop the spinner and mark the repo installed.
    SyncComplete,
    /// Shader worker failed to sync the in-flight repo; `String` is the error message.
    ///
    /// Forwarded to the catalog so it can stop the spinner.
    SyncError(String),
    /// Shader catalog "+" button clicked — present the add-repo dialog.
    AddCustomRepoRequested,
    /// User confirmed a new custom repo in the add-repo dialog.
    ///
    /// Handler validates uniqueness (rejects duplicates and known-catalog entries),
    /// persists to config, and notifies the catalog to add a row.
    RepoAdded(ShaderRepo),
    /// User clicked the trash button on a custom repo row.
    ///
    /// Handler removes the repo from config and deletes the cloned directory from disk.
    RemoveCustomRepoRequested(ShaderRepo),
}

/// Dispatch a [`ShadersMsg`] to the appropriate handler.
pub(super) fn handle(model: &mut Window, msg: ShadersMsg, root: &adw::ApplicationWindow) {
    match msg {
        ShadersMsg::DownloadRequested(repo) => handle_download_requested(model, repo),
        ShadersMsg::Progress(event) => handle_progress(model, &event),
        ShadersMsg::SyncComplete => handle_sync_complete(model),
        ShadersMsg::SyncError(e) => handle_sync_error(model, e),
        ShadersMsg::AddCustomRepoRequested => handle_add_custom_repo_requested(model, root),
        ShadersMsg::RepoAdded(repo) => handle_repo_added(model, repo),
        ShadersMsg::RemoveCustomRepoRequested(repo) => handle_remove_custom_repo_requested(model, repo),
    }
}

/// Dispatch a single-repo sync job to the shader worker.
pub(super) fn handle_download_requested(model: &Window, repo: ShaderRepo) {
    let data_dir = model.app_state.data_dir.clone();
    model.shader_worker.emit(shader_worker::Controls::SyncOne { repo, data_dir });
}

/// Forward shader worker progress to the catalog.
pub(super) fn handle_progress(model: &Window, event: &ProgressEvent) {
    model.shader_catalog.emit(shader_catalog::Controls::SyncProgress(event.to_string()));
}

/// Notify the catalog that a sync completed.
pub(super) fn handle_sync_complete(model: &Window) {
    model.shader_catalog.emit(shader_catalog::Controls::SyncComplete);
}

/// Notify the catalog that a sync failed.
pub(super) fn handle_sync_error(model: &Window, e: String) {
    model.shader_catalog.emit(shader_catalog::Controls::SyncError(e));
}

/// Present the Add Custom Repo dialog, pre-loading existing URLs for duplicate detection.
pub(super) fn handle_add_custom_repo_requested(model: &Window, root: &adw::ApplicationWindow) {
    let existing_urls = model.app_state.config.shader_repos.iter().map(|r| r.url.clone()).collect();
    model.add_shader_repo_dialog.emit(add_shader_repo_dialog::Controls::UpdateExistingUrls(existing_urls));
    model.add_shader_repo_dialog.widget().present(Some(root));
}

/// Remove a custom repo from config and disk, then update the catalog.
pub(super) fn handle_remove_custom_repo_requested(model: &mut Window, repo: ShaderRepo) {
    // Remove from persisted config.
    model.app_state.config.shader_repos.retain(|r| r.local_name != repo.local_name);
    if let Err(e) = model.app_state.save() {
        log::error!("Failed to save config after removing custom repo: {e}");
    }
    // Delete cloned data from disk.
    let repo_dir = model.app_state.data_dir.join("ReShade_shaders").join(&repo.local_name);
    if repo_dir.exists()
        && let Err(e) = std::fs::remove_dir_all(&repo_dir)
    {
        log::error!("Failed to delete repo directory {}: {e}", repo_dir.display());
    }
    // Tell the catalog to remove the row.
    model.shader_catalog.emit(shader_catalog::Controls::RemoveCustomRepo(repo));
}

/// Validate and persist a newly added custom repo.
pub(super) fn handle_repo_added(model: &mut Window, repo: ShaderRepo) {
    let in_catalog = crate::reshade::catalog::KNOWN_REPOS
        .iter()
        .any(|e| e.url.as_str() == repo.url || e.local_name.as_str() == repo.local_name);
    if in_catalog {
        model.toast_overlay.add_toast(adw::Toast::new(&fl!("toast-repo-in-catalog")));
        return;
    }
    let already = model
        .app_state
        .config
        .shader_repos
        .iter()
        .any(|r| r.url == repo.url || r.local_name == repo.local_name);
    if already {
        model.toast_overlay.add_toast(adw::Toast::new(&fl!("toast-repo-duplicate")));
        return;
    }
    model.app_state.config.shader_repos.push(repo.clone());
    if let Err(e) = model.app_state.save() {
        log::error!("Failed to save config after adding custom repo: {e}");
    }
    model.shader_catalog.emit(shader_catalog::Controls::AddCustomRepo(repo));
}
