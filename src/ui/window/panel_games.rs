//! Handler functions for the Games panel, called from [`super::Window::update`].

use std::path::PathBuf;

use relm4::adw;
use relm4::adw::prelude::*;
use relm4::ComponentController;

use crate::reshade::app_state::iris_data_dir;
use crate::reshade::game::{DllOverride, ExeArch, Game, GameSource};
use crate::ui::{add_game_dialog, game_detail, game_list, install_worker};

use super::Window;

/// Navigate to the detail page for the selected game.
pub(super) fn handle_game_selected(model: &mut Window, id: String) {
    if let Some(game) = model.games.iter().find(|g| g.id == id) {
        model
            .game_detail
            .emit(game_detail::Controls::SetGame(game.clone()));
        model.nav_view.push(model.game_detail.widget());
    }
}

/// Dispatch an install job to the worker.
pub(super) fn handle_install(
    model: &mut Window,
    game_id: String,
    dll: DllOverride,
    arch: ExeArch,
) {
    if let Some(game) = model.games.iter().find(|g| g.id == game_id) {
        let data_dir = iris_data_dir();
        model.install_worker.emit(install_worker::Controls::Install {
            data_dir,
            game_dir: game.path.clone(),
            dll,
            arch,
        });
    }
}

/// Dispatch an uninstall job to the worker.
pub(super) fn handle_uninstall(model: &mut Window, game_id: String, dll: DllOverride) {
    if let Some(game) = model.games.iter().find(|g| g.id == game_id) {
        model
            .install_worker
            .emit(install_worker::Controls::Uninstall {
                game_dir: game.path.clone(),
                dll,
            });
    }
}

/// Forward install worker progress to the detail pane.
pub(super) fn handle_progress(model: &mut Window, msg: String) {
    model
        .game_detail
        .emit(game_detail::Controls::SetProgress(msg));
}

/// Clear progress and mark the game as installed.
pub(super) fn handle_install_complete(model: &mut Window, version: String) {
    model
        .game_detail
        .emit(game_detail::Controls::ClearProgress);
    model.game_detail.emit(game_detail::Controls::MarkInstalled {
        version,
        dll: DllOverride::Dxgi,
        arch: ExeArch::X86_64,
    });
}

/// Clear progress and mark the game as uninstalled.
pub(super) fn handle_uninstall_complete(model: &mut Window) {
    model
        .game_detail
        .emit(game_detail::Controls::ClearProgress);
    model
        .game_detail
        .emit(game_detail::Controls::MarkUninstalled);
}

/// Log and surface a worker error in the detail pane.
pub(super) fn handle_worker_error(model: &mut Window, e: String) {
    log::error!("Install worker error: {e}");
    model
        .game_detail
        .emit(game_detail::Controls::SetProgress(format!("Error: {e}")));
}

/// Open the Add Game dialog.
pub(super) fn handle_add_game_requested(model: &mut Window, root: &adw::ApplicationWindow) {
    model.add_game_dialog.emit(add_game_dialog::Controls::Open);
    model.add_game_dialog.widget().present(Some(root));
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
    // Keep the dialog's duplicate-detection list in sync.
    let paths = model.games.iter().map(|g| g.path.clone()).collect();
    model
        .add_game_dialog
        .emit(add_game_dialog::Controls::UpdateExistingPaths(paths));
}
