//! Root application window — wires all UI components together.

use relm4::adw::prelude::*;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    WorkerController, adw, gtk,
};

use crate::fl;
use crate::reshade::app_state::{AppState, iris_data_dir};
use crate::reshade::config::GlobalConfig;
use crate::reshade::game::Game;
use crate::reshade::reshade::list_installed_versions;
use crate::ui::game_detail;
use crate::ui::game_list;
use crate::ui::install_worker;
use crate::ui::preferences;

/// Root window model.
pub struct Window {
    /// Persisted application state (config + games) — used for saving.
    app_state: AppState,
    /// All known games (used to look up by ID on selection).
    games: Vec<Game>,
    /// Game-list child component.
    game_list: Controller<game_list::GameList>,
    /// Detail pane child component.
    game_detail: Controller<game_detail::GameDetail>,
    /// Preferences page child component.
    preferences: Controller<preferences::Preferences>,
    /// Background install/uninstall worker.
    install_worker: WorkerController<install_worker::InstallWorker>,
    /// Navigation view — used to push game detail page.
    nav_view: adw::NavigationView,
}

/// Input messages for [`Window`].
#[allow(missing_docs)]
#[derive(Debug)]
pub enum Controls {
    /// A game was selected in the list.
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
    /// User clicked the Add Game button.
    AddGameRequested,
    /// Preferences emitted a config change; carry updated config.
    ConfigChanged(GlobalConfig),
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

            #[local_ref]
            nav_view -> adw::NavigationView {},
        }
    }

    fn init(
        _: (),
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Load state and discover Steam games.
        let app_state = AppState::load();
        let mut games = app_state.games.clone();
        let steam_games = crate::reshade::steam::discover_steam_games();
        for sg in steam_games {
            if !games.iter().any(|g| g.id == sg.id) {
                games.push(sg);
            }
        }

        // Launch child components.
        let game_list = game_list::GameList::builder()
            .launch(games.clone())
            .forward(sender.input_sender(), |sig| match sig {
                game_list::Signal::GameSelected(id) => Controls::GameSelected(id),
            });

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

        let installed_versions = list_installed_versions(&app_state.data_dir)
            .unwrap_or_else(|e| {
                log::warn!("Could not list ReShade versions: {e}");
                Vec::new()
            });
        let preferences_init = preferences::PreferencesInit {
            config: app_state.config.clone(),
            installed_versions,
            current_version: app_state.reshade_version.clone(),
        };
        let preferences = preferences::Preferences::builder()
            .launch(preferences_init)
            .forward(sender.input_sender(), |sig| match sig {
                preferences::Signal::ConfigChanged(config) => Controls::ConfigChanged(config),
            });

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

        // Build ViewStack.
        let view_stack = adw::ViewStack::new();
        view_stack.add_titled(game_list.widget(), Some("my-games"), &fl!("my-games"));
        view_stack.add_titled(
            preferences.widget(),
            Some("preferences"),
            &fl!("preferences"),
        );

        // Build ViewSwitcher wired to stack.
        let view_switcher = adw::ViewSwitcher::new();
        view_switcher.set_policy(adw::ViewSwitcherPolicy::Wide);
        view_switcher.set_stack(Some(&view_stack));

        // Build HeaderBar.
        let add_button = gtk::Button::from_icon_name("list-add-symbolic");
        add_button.set_tooltip_text(Some(&fl!("add-game")));
        add_button.connect_clicked({
            let s = sender.clone();
            move |_| s.input(Controls::AddGameRequested)
        });
        let header_bar = adw::HeaderBar::new();
        header_bar.pack_start(&add_button);
        header_bar.set_title_widget(Some(&view_switcher));

        // Build ToolbarView.
        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header_bar);
        toolbar_view.set_content(Some(&view_stack));

        // Build home NavigationPage.
        let home_page = adw::NavigationPage::new(&toolbar_view, &fl!("app-title"));

        // Build NavigationView.
        let nav_view = adw::NavigationView::new();
        nav_view.push(&home_page);

        let model = Self {
            app_state,
            games,
            game_list,
            game_detail,
            preferences,
            install_worker,
            nav_view: nav_view.clone(),
        };

        let nav_view = &nav_view;
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            Controls::GameSelected(id) => {
                if let Some(game) = self.games.iter().find(|g| g.id == id) {
                    self.game_detail
                        .emit(game_detail::Controls::SetGame(game.clone()));
                    self.nav_view.push(self.game_detail.widget());
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

            Controls::AddGameRequested => {
                // TODO: open AddGameDialog
            }

            Controls::ConfigChanged(config) => {
                self.app_state.config = config;
                if let Err(e) = self.app_state.save() {
                    log::error!("Failed to save config: {e}");
                }
            }
        }
    }
}
