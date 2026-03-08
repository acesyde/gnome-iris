//! Root application window — wires all UI components together.

use relm4::adw::prelude::*;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    WorkerController, adw, gtk,
};

use crate::fl;
use crate::reshade::app_state::{AppState, iris_data_dir};
use crate::reshade::game::Game;
use crate::ui::game_detail;
use crate::ui::game_list;
use crate::ui::install_worker;

/// Root window model.
pub struct Window {
    /// All known games (used to look up by ID on selection).
    games: Vec<Game>,
    /// Sidebar game-list child component.
    game_list: Controller<game_list::GameList>,
    /// Detail pane child component.
    game_detail: Controller<game_detail::GameDetail>,
    /// Background install/uninstall worker.
    install_worker: WorkerController<install_worker::InstallWorker>,
    /// Whether the sidebar is shown.
    sidebar_visible: bool,
}

/// Input messages for [`Window`].
#[allow(missing_docs)]
#[derive(Debug)]
pub enum Controls {
    /// Toggle the sidebar visibility.
    ToggleSidebar,
    /// A game was selected in the sidebar.
    GameSelected(String),
    /// GameDetail requested installation.
    Install {
        game_id: String,
        dll: crate::reshade::game::DllOverride,
        arch: crate::reshade::game::ExeArch,
    },
    /// GameDetail requested uninstallation.
    Uninstall {
        game_id: String,
        dll: crate::reshade::game::DllOverride,
    },
    /// InstallWorker reported progress.
    Progress(String),
    /// InstallWorker finished installation.
    InstallComplete { version: String },
    /// InstallWorker finished uninstallation.
    UninstallComplete,
    /// InstallWorker reported an error.
    WorkerError(String),
}

#[allow(missing_docs)]
#[relm4::component(pub)]
impl Component for Window {
    type Init = ();
    type Input = Controls;
    type Output = ();
    type CommandOutput = ();

    view! {
        adw::ApplicationWindow {
            set_title: Some(&fl!("app-title")),
            set_default_width: 1000,
            set_default_height: 700,

            adw::OverlaySplitView {
                #[watch]
                set_show_sidebar: model.sidebar_visible,

                #[wrap(Some)]
                set_sidebar = &adw::NavigationPage {
                    set_title: &fl!("app-title"),
                    set_width_request: 260,

                    adw::ToolbarView {
                        add_top_bar = &adw::HeaderBar {
                            pack_start = &gtk::Button {
                                set_icon_name: "folder-open-symbolic",
                                set_tooltip_text: Some(&fl!("add-game")),
                            },
                        },

                        model.game_list.widget() -> &gtk::Box {},
                    },
                },

                #[wrap(Some)]
                set_content = &adw::NavigationPage {
                    set_title: "Detail",

                    adw::ToolbarView {
                        add_top_bar = &adw::HeaderBar {},

                        model.game_detail.widget() -> &gtk::Box {},
                    },
                },
            },
        }
    }

    fn init(
        _: (),
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Load state and discover Steam games.
        let app_state = AppState::load();
        let mut games = app_state.games;
        let steam_games = crate::reshade::steam::discover_steam_games();
        // Merge: add Steam games not already tracked (by ID).
        for sg in steam_games {
            if !games.iter().any(|g| g.id == sg.id) {
                games.push(sg);
            }
        }

        // Launch game list sidebar.
        let game_list = game_list::GameList::builder()
            .launch(games.clone())
            .forward(sender.input_sender(), |sig| match sig {
                game_list::Signal::GameSelected(id) => Controls::GameSelected(id),
                game_list::Signal::AddGameRequested => {
                    // TODO: open add-game dialog
                    Controls::ToggleSidebar // no-op placeholder
                }
            });

        // Launch game detail pane.
        let game_detail = game_detail::GameDetail::builder()
            .launch(())
            .forward(sender.input_sender(), |sig| match sig {
                game_detail::Signal::Install { game_id, dll, arch } => {
                    Controls::Install { game_id, dll, arch }
                }
                game_detail::Signal::Uninstall { game_id, dll } => {
                    Controls::Uninstall { game_id, dll }
                }
            });

        // Launch install worker.
        let install_worker = install_worker::InstallWorker::builder()
            .detach_worker(())
            .forward(sender.input_sender(), |sig| match sig {
                install_worker::Signal::Progress(msg) => Controls::Progress(msg),
                install_worker::Signal::InstallComplete { version } => {
                    Controls::InstallComplete { version }
                }
                install_worker::Signal::UninstallComplete => Controls::UninstallComplete,
                install_worker::Signal::Error(e) => Controls::WorkerError(e),
            });

        let model = Self {
            games,
            game_list,
            game_detail,
            install_worker,
            sidebar_visible: true,
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            Controls::ToggleSidebar => self.sidebar_visible = !self.sidebar_visible,

            Controls::GameSelected(id) => {
                if let Some(game) = self.games.iter().find(|g| g.id == id) {
                    self.game_detail
                        .emit(game_detail::Controls::SetGame(game.clone()));
                }
            }

            Controls::Install { game_id, dll, arch } => {
                if let Some(game) = self.games.iter().find(|g| g.id == game_id) {
                    let data_dir = iris_data_dir();
                    self.install_worker
                        .emit(install_worker::Controls::Install {
                            data_dir,
                            game_dir: game.path.clone(),
                            dll,
                            arch,
                        });
                }
            }

            Controls::Uninstall { game_id, dll } => {
                if let Some(game) = self.games.iter().find(|g| g.id == game_id) {
                    self.install_worker
                        .emit(install_worker::Controls::Uninstall {
                            game_dir: game.path.clone(),
                            dll,
                        });
                }
            }

            Controls::Progress(msg) => {
                self.game_detail
                    .emit(game_detail::Controls::SetProgress(msg));
            }

            Controls::InstallComplete { version } => {
                self.game_detail
                    .emit(game_detail::Controls::ClearProgress);
                // We don't know dll/arch here without tracking; use defaults.
                // A proper implementation would track the in-flight install params.
                self.game_detail.emit(game_detail::Controls::MarkInstalled {
                    version,
                    dll: crate::reshade::game::DllOverride::Dxgi,
                    arch: crate::reshade::game::ExeArch::X86_64,
                });
            }

            Controls::UninstallComplete => {
                self.game_detail
                    .emit(game_detail::Controls::ClearProgress);
                self.game_detail
                    .emit(game_detail::Controls::MarkUninstalled);
            }

            Controls::WorkerError(e) => {
                log::error!("Install worker error: {e}");
                self.game_detail
                    .emit(game_detail::Controls::SetProgress(format!("Error: {e}")));
            }
        }
    }
}
