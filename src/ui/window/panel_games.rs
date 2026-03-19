//! Handler functions for the Games panel, called from [`super::Window::update`].

use std::path::PathBuf;

use relm4::ComponentController;
use relm4::adw;
use relm4::adw::prelude::*;

use crate::reshade::app_state::iris_data_dir;
use crate::reshade::catalog::KNOWN_REPOS;
use crate::reshade::config::ShaderOverrides;
use crate::reshade::game::{DllOverride, ExeArch, Game, GameSource, InstallStatus};
use crate::ui::worker_types::ProgressEvent;
use crate::ui::{add_game_dialog, game_detail, game_list, install_worker};

use super::Window;

/// Messages handled by the Games panel.
#[derive(Debug)]
pub enum GamesMsg {
    /// A game row was activated; `String` is the stable game ID.
    ///
    /// Pushes the detail page onto the navigation view.
    GameSelected(String),
    /// User clicked the trash button on a manually added game; `String` is the game ID.
    ///
    /// Removes the game from persisted state and the list.
    GameRemoveRequested(String),
    /// `GameDetail` requested installation.
    Install {
        /// Stable game ID.
        game_id: String,
        /// DLL override type chosen by the user.
        dll: DllOverride,
        /// Architecture of the game executable.
        arch: ExeArch,
        /// The cached version key chosen by the user, e.g. `"v6.3.0"`.
        version: String,
    },
    /// `GameDetail` requested uninstallation.
    Uninstall {
        /// Stable game ID.
        game_id: String,
        /// DLL override type to remove.
        dll: DllOverride,
    },
    /// `InstallWorker` reported progress.
    Progress(ProgressEvent),
    /// `InstallWorker` finished installation.
    InstallComplete {
        /// Version key that was installed.
        version: String,
        /// DLL override that was installed.
        dll: DllOverride,
        /// Executable architecture that was targeted.
        arch: ExeArch,
    },
    /// `InstallWorker` finished uninstallation.
    UninstallComplete,
    /// `InstallWorker` reported a fatal error; `String` is a human-readable message.
    ///
    /// Logged and shown as a progress message in the detail pane.
    WorkerError(String),
    /// User clicked the Add Game button.
    AddGameRequested,
    /// User confirmed adding a game via the dialog.
    GameAdded {
        /// Display name.
        name: String,
        /// Game directory.
        path: PathBuf,
        /// Architecture detected or chosen in the dialog.
        arch: ExeArch,
    },
    /// Per-game shader repo toggle forwarded from the detail pane.
    ShaderToggled {
        /// Stable game ID.
        game_id: String,
        /// Repository local name.
        repo_name: String,
        /// New enabled state.
        enabled: bool,
    },
    /// Async startup task detected the install status for a Steam-discovered game.
    GameStatusDetected {
        /// Stable game ID.
        id: String,
        /// Detected install status.
        status: InstallStatus,
    },
}

/// Dispatch a [`GamesMsg`] to the appropriate handler.
pub(super) fn handle(model: &mut Window, msg: GamesMsg, root: &adw::ApplicationWindow) {
    match msg {
        GamesMsg::GameSelected(id) => handle_game_selected(model, id),
        GamesMsg::GameRemoveRequested(id) => handle_game_remove(model, id),
        GamesMsg::Install {
            game_id,
            dll,
            arch,
            version,
        } => handle_install(model, &game_id, dll, arch, version),
        GamesMsg::Uninstall { game_id, dll } => handle_uninstall(model, &game_id, dll),
        GamesMsg::Progress(event) => handle_progress(model, &event),
        GamesMsg::InstallComplete { version, dll, arch } => handle_install_complete(model, &version, dll, arch),
        GamesMsg::UninstallComplete => handle_uninstall_complete(model),
        GamesMsg::WorkerError(e) => handle_worker_error(model, &e),
        GamesMsg::AddGameRequested => handle_add_game_requested(model, root),
        GamesMsg::GameAdded { name, path, arch } => handle_game_added(model, name, path, arch),
        GamesMsg::ShaderToggled {
            game_id,
            repo_name,
            enabled,
        } => handle_shader_toggled(model, &game_id, &repo_name, enabled),
        GamesMsg::GameStatusDetected { id, status } => handle_game_status_detected(model, id, status),
    }
}

/// Navigate to the detail page for the selected game.
pub(super) fn handle_game_selected(model: &mut Window, id: String) {
    if let Some(game) = model.games.iter().find(|g| g.id == id).cloned() {
        model.game_detail.emit(game_detail::Controls::SetGame(game.clone()));
        send_shader_data(model, &game.id, &game.shader_overrides);
        model
            .game_detail
            .emit(game_detail::Controls::SetInstalledVersions(model.installed_versions.clone()));
        model.nav_view.push(model.game_detail.widget());
        model.current_game_id = Some(id);
    }
}

/// Sends the current downloaded shader repo list and per-game overrides to the
/// detail pane.
///
/// Used both when navigating to a game and when a shader sync completes while
/// the detail pane is already open.
pub(super) fn send_shader_data(model: &Window, game_id: &str, overrides: &ShaderOverrides) {
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
        // Exclude per-game dirs that happen to share the prefix used for repo names.
        .filter(|r| {
            use crate::reshade::paths::GAME_SHADER_DIR_PREFIX;
            !r.local_name.starts_with(GAME_SHADER_DIR_PREFIX)
        })
        .collect();
    model.game_detail.emit(game_detail::Controls::SetShaderData {
        repos: downloaded_repos,
        overrides: overrides.clone(),
    });
    let _ = game_id; // retained for future per-game filtering if needed
}

/// Dispatch an install job to the worker using the pre-cached version.
pub(super) fn handle_install(model: &Window, game_id: &str, dll: DllOverride, arch: ExeArch, version: String) {
    if let Some(game) = model.games.iter().find(|g| g.id == game_id) {
        let data_dir = iris_data_dir();
        model.install_worker.emit(install_worker::Controls::Install {
            data_dir,
            game_dir: game.path.clone(),
            game_id: game.id.clone(),
            disabled_repos: game.shader_overrides.disabled_repos.clone(),
            dll,
            arch,
            version,
        });
    }
}

/// Dispatch an uninstall job to the worker.
pub(super) fn handle_uninstall(model: &Window, game_id: &str, dll: DllOverride) {
    if let Some(game) = model.games.iter().find(|g| g.id == game_id) {
        model.install_worker.emit(install_worker::Controls::Uninstall {
            data_dir: iris_data_dir(),
            game_dir: game.path.clone(),
            game_id: game.id.clone(),
            dll,
        });
    }
}

/// Forward install worker progress to the detail pane.
pub(super) fn handle_progress(model: &Window, event: &ProgressEvent) {
    model.game_detail.emit(game_detail::Controls::SetProgress(event.to_string()));
}

/// Clear progress and mark the game as installed.
pub(super) fn handle_install_complete(model: &mut Window, version: &str, dll: DllOverride, arch: ExeArch) {
    model.game_detail.emit(game_detail::Controls::ClearProgress);
    model.game_detail.emit(game_detail::Controls::MarkInstalled {
        version: version.to_string(),
        dll,
        arch,
    });
    if let Some(id) = model.current_game_id.clone() {
        let status = InstallStatus::Installed {
            dll,
            arch,
            version: Some(version.to_string()),
        };
        if let Some(game) = model.games.iter_mut().find(|g| g.id == id) {
            game.status = status.clone();
        }
        if let Some(game) = model.app_state.games.iter_mut().find(|g| g.id == id) {
            game.status = status;
        }
        model.save_or_toast();
        model.game_list.emit(game_list::Controls::SetGameStatus {
            id,
            version: Some(version.to_string()),
            latest_version: model.latest_version.clone(),
        });
    }
}

/// Clear progress and mark the game as uninstalled.
pub(super) fn handle_uninstall_complete(model: &mut Window) {
    model.game_detail.emit(game_detail::Controls::ClearProgress);
    model.game_detail.emit(game_detail::Controls::MarkUninstalled);
    if let Some(id) = model.current_game_id.clone() {
        if let Some(game) = model.games.iter_mut().find(|g| g.id == id) {
            game.status = InstallStatus::NotInstalled;
        }
        if let Some(game) = model.app_state.games.iter_mut().find(|g| g.id == id) {
            game.status = InstallStatus::NotInstalled;
        }
        model.save_or_toast();
        model.game_list.emit(game_list::Controls::SetGameStatus {
            id,
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
    model.save_or_toast();
    model.game_list.emit(game_list::Controls::RemoveGame(id));
}

/// Persist a per-game shader repo toggle and rebuild the per-game shader directory.
pub(super) fn handle_shader_toggled(model: &mut Window, game_id: &str, repo_name: &str, enabled: bool) {
    let update = |overrides: &mut ShaderOverrides| {
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
        model.save_or_toast();
    }

    // Rebuild the per-game shader dir so the toggle takes effect immediately.
    // This is fast (symlink-only) and safe to run on the main thread.
    if let Some(game) = model.games.iter().find(|g| g.id == game_id)
        && let Err(e) = crate::reshade::shaders::rebuild_game_merged(
            &model.app_state.data_dir,
            &game.id,
            &game.shader_overrides.disabled_repos,
        )
    {
        log::warn!("Shader rebuild failed for game {game_id}: {e}");
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
    model.save_or_toast();
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
