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
use crate::ui::add_shader_repo_dialog;
use crate::ui::game_detail;
use crate::ui::game_list;
use crate::ui::install_worker;
use crate::ui::preferences;
use crate::ui::shader_catalog;
use crate::ui::shader_worker;

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
    /// Shader catalog tab component.
    shader_catalog: relm4::Controller<shader_catalog::ShaderCatalog>,
    /// Preferences page child component.
    preferences: Controller<preferences::Preferences>,
    /// Background install/uninstall worker.
    install_worker: WorkerController<install_worker::InstallWorker>,
    /// Background shader sync worker (used for single-repo downloads).
    shader_worker: WorkerController<shader_worker::ShaderWorker>,
    /// Dialog for adding a custom shader repository.
    add_shader_repo_dialog: relm4::Controller<add_shader_repo_dialog::AddShaderRepoDialog>,
    /// Navigation view — used to push game detail page.
    nav_view: adw::NavigationView,
    /// Toast overlay — used to surface brief error/info messages.
    toast_overlay: adw::ToastOverlay,
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
    /// User requested downloading a shader repo from the catalog.
    ShaderDownloadRequested(crate::reshade::config::ShaderRepo),
    /// Shader worker reported download progress.
    ShaderProgress(String),
    /// Shader worker finished syncing a catalog repo.
    ShaderSyncComplete,
    /// Shader worker reported an error syncing a catalog repo.
    ShaderSyncError(String),
    /// Shader catalog "+" button clicked — present add-repo dialog.
    ShaderAddCustomRepoRequested,
    /// User confirmed new custom repo in dialog.
    ShaderRepoAdded(crate::reshade::config::ShaderRepo),
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
            toast_overlay -> adw::ToastOverlay {},
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

        let known_names: std::collections::HashSet<&str> =
            crate::reshade::catalog::KNOWN_REPOS.iter().map(|e| e.local_name).collect();
        let custom_repos: Vec<_> = app_state
            .config
            .shader_repos
            .iter()
            .filter(|r| !known_names.contains(r.local_name.as_str()))
            .cloned()
            .collect();

        let shader_catalog = shader_catalog::ShaderCatalog::builder()
            .launch(shader_catalog::ShaderCatalogInit {
                data_dir: app_state.data_dir.clone(),
                custom_repos,
            })
            .forward(sender.input_sender(), |sig| match sig {
                shader_catalog::Signal::DownloadRequested(repo) => {
                    Controls::ShaderDownloadRequested(repo)
                }
                shader_catalog::Signal::AddCustomRepoRequested => {
                    Controls::ShaderAddCustomRepoRequested
                }
            });

        let add_shader_repo_dialog = add_shader_repo_dialog::AddShaderRepoDialog::builder()
            .launch(())
            .forward(sender.input_sender(), |sig| match sig {
                add_shader_repo_dialog::Signal::RepoAdded(repo) => {
                    Controls::ShaderRepoAdded(repo)
                }
            });

        let shader_worker = shader_worker::ShaderWorker::builder()
            .detach_worker(())
            .forward(sender.input_sender(), |sig| match sig {
                shader_worker::Signal::Progress(msg) => Controls::ShaderProgress(msg),
                shader_worker::Signal::Complete => Controls::ShaderSyncComplete,
                shader_worker::Signal::RepoError { error, .. } => {
                    Controls::ShaderSyncError(error)
                }
                shader_worker::Signal::Error(e) => Controls::ShaderSyncError(e),
            });

        // Build ViewStack.
        let view_stack = adw::ViewStack::new();
        view_stack.add_titled(game_list.widget(), Some("my-games"), &fl!("my-games"));
        view_stack.add_titled(
            shader_catalog.widget(),
            Some("shaders"),
            &fl!("shaders-section"),
        );
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

        // Wrap in a ToastOverlay so we can surface brief notifications.
        let toast_overlay = adw::ToastOverlay::new();

        let model = Self {
            app_state,
            games,
            game_list,
            game_detail,
            shader_catalog,
            preferences,
            install_worker,
            shader_worker,
            add_shader_repo_dialog,
            nav_view: nav_view.clone(),
            toast_overlay: toast_overlay.clone(),
        };

        let nav_view = &nav_view;
        let toast_overlay = &toast_overlay;
        let widgets = view_output!();
        // Wire nav_view as the toast overlay's child (must be done after view_output!).
        widgets.toast_overlay.set_child(Some(nav_view));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, _sender: ComponentSender<Self>, root: &Self::Root) {
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

            Controls::ShaderDownloadRequested(repo) => {
                let data_dir = self.app_state.data_dir.clone();
                self.shader_worker
                    .emit(shader_worker::Controls::SyncOne { repo, data_dir });
            }
            Controls::ShaderProgress(msg) => {
                self.shader_catalog
                    .emit(shader_catalog::Controls::SyncProgress(msg));
            }
            Controls::ShaderSyncComplete => {
                self.shader_catalog
                    .emit(shader_catalog::Controls::SyncComplete);
            }
            Controls::ShaderSyncError(e) => {
                self.shader_catalog
                    .emit(shader_catalog::Controls::SyncError(e));
            }
            Controls::ShaderAddCustomRepoRequested => {
                self.add_shader_repo_dialog.widget().present(Some(root));
            }
            Controls::ShaderRepoAdded(repo) => {
                let in_catalog = crate::reshade::catalog::KNOWN_REPOS
                    .iter()
                    .any(|e| e.url == repo.url || e.local_name == repo.local_name);
                if in_catalog {
                    self.toast_overlay.add_toast(adw::Toast::new(
                        "This repository is already in the known catalog.",
                    ));
                    return;
                }
                let already = self
                    .app_state
                    .config
                    .shader_repos
                    .iter()
                    .any(|r| r.url == repo.url || r.local_name == repo.local_name);
                if already {
                    self.toast_overlay
                        .add_toast(adw::Toast::new("This repository has already been added."));
                    return;
                }
                self.app_state.config.shader_repos.push(repo.clone());
                if let Err(e) = self.app_state.save() {
                    log::error!("Failed to save config after adding custom repo: {e}");
                }
                self.shader_catalog
                    .emit(shader_catalog::Controls::AddCustomRepo(repo));
            }
        }
    }
}
