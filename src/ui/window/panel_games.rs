//! Handler functions for the Games panel, called from [`super::Window::update`].

use std::path::PathBuf;

use relm4::ComponentController;
use relm4::adw;
use relm4::adw::prelude::*;

use crate::reshade::app_state::iris_data_dir;
use crate::reshade::catalog::KNOWN_REPOS;
use crate::reshade::game::{DllOverride, ExeArch, Game, GameSource, InstallStatus};
use crate::ui::worker_types::ProgressEvent;
use crate::ui::{add_game_dialog, game_detail, game_list, install_worker};

use super::Window;

/// Navigate to the detail page for the selected game.
pub(super) fn handle_game_selected(model: &mut Window, id: String) {
    if let Some(game) = model.games.iter().find(|g| g.id == id).cloned() {
        model.game_detail.emit(game_detail::Controls::SetGame(game.clone()));
        let repos_dir = model.app_state.data_dir.join("ReShade_shaders");
        let known_names: std::collections::HashSet<&str> = KNOWN_REPOS.iter().map(|e| e.local_name.as_str()).collect();
        let downloaded_repos = KNOWN_REPOS
            .iter()
            .filter(|e| repos_dir.join(&e.local_name).is_dir())
            .map(crate::reshade::catalog::CatalogEntry::to_shader_repo)
            .chain(
                model
                    .app_state
                    .config
                    .shader_repos
                    .iter()
                    .filter(|r| !known_names.contains(r.local_name.as_str()) && repos_dir.join(&r.local_name).is_dir())
                    .cloned(),
            )
            .collect();
        model.game_detail.emit(game_detail::Controls::SetShaderData {
            repos: downloaded_repos,
            overrides: game.shader_overrides,
        });
        model
            .game_detail
            .emit(game_detail::Controls::SetInstalledVersions(model.installed_versions.clone()));
        model.nav_view.push(model.game_detail.widget());
        model.current_game_id = Some(id);
    }
}

/// Dispatch an install job to the worker using the pre-cached version.
pub(super) fn handle_install(model: &mut Window, game_id: &str, dll: DllOverride, arch: ExeArch, version: String) {
    if let Some(game) = model.games.iter().find(|g| g.id == game_id) {
        let data_dir = iris_data_dir();
        model.install_worker.emit(install_worker::Controls::Install {
            data_dir,
            game_dir: game.path.clone(),
            dll,
            arch,
            version,
        });
        model.pending_install = Some((dll, arch));
    }
}

/// Dispatch an uninstall job to the worker.
pub(super) fn handle_uninstall(model: &Window, game_id: &str, dll: DllOverride) {
    if let Some(game) = model.games.iter().find(|g| g.id == game_id) {
        model.install_worker.emit(install_worker::Controls::Uninstall {
            game_dir: game.path.clone(),
            dll,
        });
    }
}

/// Forward install worker progress to the detail pane.
pub(super) fn handle_progress(model: &Window, event: &ProgressEvent) {
    model.game_detail.emit(game_detail::Controls::SetProgress(event.to_string()));
}

/// Clear progress and mark the game as installed.
pub(super) fn handle_install_complete(model: &mut Window, version: &str) {
    let (dll, arch) = model.pending_install.take().unwrap_or((DllOverride::Dxgi, ExeArch::X86_64));
    model.game_detail.emit(game_detail::Controls::ClearProgress);
    model.game_detail.emit(game_detail::Controls::MarkInstalled { version: version.to_string(), dll, arch });
    if let Some(id) = &model.current_game_id {
        let status = InstallStatus::Installed { dll, arch, version: Some(version.to_string()) };
        if let Some(game) = model.games.iter_mut().find(|g| &g.id == id) {
            game.status = status.clone();
        }
        if let Some(game) = model.app_state.games.iter_mut().find(|g| &g.id == id) {
            game.status = status;
            if let Err(e) = model.app_state.save() {
                log::error!("Failed to save game status after install: {e}");
            }
        }
        model.game_list.emit(game_list::Controls::SetGameStatus {
            id: id.clone(),
            version: Some(version.to_string()),
            latest_version: model.latest_version.clone(),
        });
    }
}

/// Clear progress and mark the game as uninstalled.
pub(super) fn handle_uninstall_complete(model: &mut Window) {
    model.game_detail.emit(game_detail::Controls::ClearProgress);
    model.game_detail.emit(game_detail::Controls::MarkUninstalled);
    if let Some(id) = &model.current_game_id {
        if let Some(game) = model.games.iter_mut().find(|g| &g.id == id) {
            game.status = InstallStatus::NotInstalled;
        }
        if let Some(game) = model.app_state.games.iter_mut().find(|g| &g.id == id) {
            game.status = InstallStatus::NotInstalled;
            if let Err(e) = model.app_state.save() {
                log::error!("Failed to save game status after uninstall: {e}");
            }
        }
        model.game_list.emit(game_list::Controls::SetGameStatus {
            id: id.clone(),
            version: None,
            latest_version: model.latest_version.clone(),
        });
    }
}

/// Log and surface a worker error in the detail pane.
pub(super) fn handle_worker_error(model: &Window, e: &str) {
    log::error!("Install worker error: {e}");
    model.game_detail.emit(game_detail::Controls::SetProgress(format!("Error: {e}")));
}

/// Open the Add Game dialog.
pub(super) fn handle_add_game_requested(model: &Window, root: &adw::ApplicationWindow) {
    model.add_game_dialog.emit(add_game_dialog::Controls::Open);
    model.add_game_dialog.widget().present(Some(root));
}

/// Remove a manually added game from the list and persisted state.
pub(super) fn handle_game_remove(model: &mut Window, id: String) {
    model.games.retain(|g| g.id != id);
    model.app_state.games.retain(|g| g.id != id);
    if let Err(e) = model.app_state.save() {
        log::error!("Failed to save games after removal: {e}");
    }
    model.game_list.emit(game_list::Controls::RemoveGame(id));
}

/// Persist a per-game shader repo toggle.
pub(super) fn handle_shader_toggled(model: &mut Window, game_id: &str, repo_name: &str, enabled: bool) {
    let update = |overrides: &mut crate::reshade::config::ShaderOverrides| {
        if enabled {
            overrides.disabled_repos.retain(|r| r != repo_name);
        } else if !overrides.disabled_repos.contains(&repo_name.to_owned()) {
            overrides.disabled_repos.push(repo_name.to_owned());
        }
    };

    if let Some(game) = model.games.iter_mut().find(|g| g.id == game_id) {
        update(&mut game.shader_overrides);
    }
    if let Some(game) = model.app_state.games.iter_mut().find(|g| g.id == game_id) {
        update(&mut game.shader_overrides);
        if let Err(e) = model.app_state.save() {
            log::error!("Failed to save shader overrides: {e}");
        }
    }
}

/// Update a game's install status after async startup detection completes.
pub(super) fn handle_game_status_detected(model: &mut Window, id: String, status: InstallStatus) {
    if let Some(game) = model.games.iter_mut().find(|g| g.id == id) {
        game.status = status.clone();
    }
    let version = match status {
        InstallStatus::Installed { version, .. } => version,
        InstallStatus::NotInstalled => None,
    };
    model.game_list.emit(game_list::Controls::SetGameStatus {
        id,
        version,
        latest_version: model.latest_version.clone(),
    });
}

/// Persist the new game and add it to the list.
pub(super) fn handle_game_added(model: &mut Window, name: String, path: PathBuf, arch: ExeArch) {
    let mut game = Game::new(name, path, GameSource::Manual);
    game.preferred_arch = Some(arch);
    model.app_state.games.push(game.clone());
    if let Err(e) = model.app_state.save() {
        log::error!("Failed to save games: {e}");
    }
    model.games.push(game.clone());
    model.game_list.emit(game_list::Controls::AddGame(game));
    // Prime the pill immediately if the latest version is already known.
    // game was moved by AddGame; read back from model.games which was pushed above.
    if model.latest_version.is_some()
        && let Some(g) = model.games.last()
    {
        let installed_version = match &g.status {
            InstallStatus::Installed { version: v, .. } => v.clone(),
            InstallStatus::NotInstalled => None,
        };
        model.game_list.emit(game_list::Controls::SetGameStatus {
            id: g.id.clone(),
            version: installed_version,
            latest_version: model.latest_version.clone(),
        });
    }
    // Keep the dialog's duplicate-detection list in sync.
    let paths = model.games.iter().map(|g| g.path.clone()).collect();
    model.add_game_dialog.emit(add_game_dialog::Controls::UpdateExistingPaths(paths));
}
