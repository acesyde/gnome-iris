//! Root application window — wires all UI components together.

mod panel_games;
mod panel_preferences;
mod panel_shaders;

use relm4::adw::prelude::*;
use relm4::{Component, ComponentController, ComponentParts, ComponentSender, Controller, WorkerController, adw, gtk};

use crate::fl;
use crate::reshade::app_state::AppState;
use crate::reshade::game::{Game, InstallStatus};
use crate::reshade::install::detect_install_status;
use crate::reshade::reshade::list_installed_versions;
use crate::ui::add_game_dialog;
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
    /// Dialog for manually adding a game.
    add_game_dialog: relm4::Controller<add_game_dialog::AddGameDialog>,
    /// Navigation view — used to push game detail page.
    nav_view: adw::NavigationView,
    /// Toast overlay — used to surface brief error/info messages.
    toast_overlay: adw::ToastOverlay,
    /// DLL + arch of the in-flight install (fixes hardcoded values in `handle_install_complete`).
    pending_install: Option<(crate::reshade::game::DllOverride, crate::reshade::game::ExeArch)>,
    /// ID of the game currently shown in the detail pane (for config refresh).
    current_game_id: Option<String>,
    /// Locally-cached `ReShade` version keys; kept in sync with Preferences add/remove.
    installed_versions: Vec<String>,
    /// Latest known `ReShade` version fetched from GitHub (or read from cache).
    latest_version: Option<String>,
}

/// Input messages for [`Window`], grouped by panel domain.
#[derive(Debug)]
pub enum Controls {
    /// Messages routed to the Games panel.
    Games(panel_games::GamesMsg),
    /// Messages routed to the Shaders panel.
    Shaders(panel_shaders::ShadersMsg),
    /// Messages routed to the Preferences panel.
    Prefs(panel_preferences::PrefsMsg),
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

    #[allow(clippy::too_many_lines)]
    fn init((): (), root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        // Load state and discover Steam games.
        let app_state = AppState::load();
        let mut games = app_state.games.clone();
        let steam_games = crate::reshade::steam::discover_steam_games();
        // Collect new Steam games without blocking on status detection; detection
        // runs in the startup async task so the window appears immediately.
        let mut new_steam_game_paths: Vec<(String, std::path::PathBuf)> = Vec::new();
        for sg in steam_games {
            if !games.iter().any(|g| g.id == sg.id) {
                new_steam_game_paths.push((sg.id.clone(), sg.path.clone()));
                games.push(sg);
            }
        }

        // Launch child components.
        let game_list = game_list::GameList::builder()
            .launch(games.clone())
            .forward(sender.input_sender(), |sig| match sig {
                game_list::Signal::GameSelected(id) => Controls::Games(panel_games::GamesMsg::GameSelected(id)),
                game_list::Signal::GameRemoveRequested(id) => Controls::Games(panel_games::GamesMsg::GameRemoveRequested(id)),
            });

        let game_detail =
            game_detail::GameDetail::builder()
                .launch(())
                .forward(sender.input_sender(), |sig| match sig {
                    game_detail::Signal::Install { game_id, dll, arch, version } => {
                        Controls::Games(panel_games::GamesMsg::Install { game_id, dll, arch, version })
                    },
                    game_detail::Signal::Uninstall { game_id, dll } => {
                        Controls::Games(panel_games::GamesMsg::Uninstall { game_id, dll })
                    },
                    game_detail::Signal::ShaderToggled { game_id, repo_name, enabled } => {
                        Controls::Games(panel_games::GamesMsg::ShaderToggled { game_id, repo_name, enabled })
                    },
                });

        let installed_versions = list_installed_versions(&app_state.data_dir).unwrap_or_else(|e| {
            log::warn!("Could not list ReShade versions: {e}");
            Vec::new()
        });
        let versions_in_use = compute_versions_in_use(&games, &app_state.data_dir);
        let preferences_init = preferences::PreferencesInit {
            data_dir: app_state.data_dir.clone(),
            config: app_state.config.clone(),
            installed_versions: installed_versions.clone(),
            current_version: app_state.reshade_version.clone(),
            versions_in_use,
        };
        let preferences = preferences::Preferences::builder().launch(preferences_init).forward(
            sender.input_sender(),
            |sig| match sig {
                preferences::Signal::ConfigChanged(config) => Controls::Prefs(panel_preferences::PrefsMsg::ConfigChanged(config)),
                preferences::Signal::InstallVersionRequested(v) => Controls::Prefs(panel_preferences::PrefsMsg::VersionDownloadRequested(v)),
                preferences::Signal::RemoveVersionRequested(v) => Controls::Prefs(panel_preferences::PrefsMsg::VersionRemoveRequested(v)),
            },
        );

        let install_worker = install_worker::InstallWorker::builder().detach_worker(()).forward(
            sender.input_sender(),
            |sig| match sig {
                install_worker::Signal::Progress(msg) => Controls::Games(panel_games::GamesMsg::Progress(msg)),
                install_worker::Signal::InstallComplete { version } => Controls::Games(panel_games::GamesMsg::InstallComplete { version }),
                install_worker::Signal::UninstallComplete => Controls::Games(panel_games::GamesMsg::UninstallComplete),
                install_worker::Signal::DownloadVersionComplete { version_key } => {
                    Controls::Prefs(panel_preferences::PrefsMsg::VersionDownloadComplete(version_key))
                },
                install_worker::Signal::DownloadVersionError(e) => Controls::Prefs(panel_preferences::PrefsMsg::VersionDownloadError(e)),
                install_worker::Signal::Error(e) => Controls::Games(panel_games::GamesMsg::WorkerError(e)),
            },
        );

        let known_names: std::collections::HashSet<&str> =
            crate::reshade::catalog::KNOWN_REPOS.iter().map(|e| e.local_name.as_str()).collect();
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
                shader_catalog::Signal::DownloadRequested(repo) => Controls::Shaders(panel_shaders::ShadersMsg::DownloadRequested(repo)),
                shader_catalog::Signal::AddCustomRepoRequested => Controls::Shaders(panel_shaders::ShadersMsg::AddCustomRepoRequested),
                shader_catalog::Signal::RemoveCustomRepoRequested(repo) => {
                    Controls::Shaders(panel_shaders::ShadersMsg::RemoveCustomRepoRequested(repo))
                },
            });

        let add_shader_repo_dialog =
            add_shader_repo_dialog::AddShaderRepoDialog::builder()
                .launch(())
                .forward(sender.input_sender(), |sig| match sig {
                    add_shader_repo_dialog::Signal::RepoAdded(repo) => Controls::Shaders(panel_shaders::ShadersMsg::RepoAdded(repo)),
                });

        let add_game_dialog = add_game_dialog::AddGameDialog::builder()
            .launch(games.iter().map(|g| g.path.clone()).collect())
            .forward(sender.input_sender(), |sig| match sig {
                add_game_dialog::Signal::GameAdded { name, path, arch } => {
                    Controls::Games(panel_games::GamesMsg::GameAdded { name, path, arch })
                },
            });

        let shader_worker =
            shader_worker::ShaderWorker::builder()
                .detach_worker(())
                .forward(sender.input_sender(), |sig| match sig {
                    shader_worker::Signal::Progress(msg) => Controls::Shaders(panel_shaders::ShadersMsg::Progress(msg)),
                    shader_worker::Signal::Complete => Controls::Shaders(panel_shaders::ShadersMsg::SyncComplete),
                    shader_worker::Signal::RepoError { error, .. } => Controls::Shaders(panel_shaders::ShadersMsg::SyncError(error)),
                    shader_worker::Signal::Error(e) => Controls::Shaders(panel_shaders::ShadersMsg::SyncError(e)),
                });

        // Capture values needed for version-check task before app_state is moved.
        let update_interval = app_state.config.update_interval_hours;
        let cache_data_dir = app_state.data_dir.clone();
        let steam_paths_for_detection = new_steam_game_paths;

        // Build ViewStack.
        let view_stack = adw::ViewStack::new();
        view_stack.add_titled_with_icon(
            game_list.widget(),
            Some("my-games"),
            &fl!("my-games"),
            "input-gaming-symbolic",
        );
        view_stack.add_titled_with_icon(
            shader_catalog.widget(),
            Some("shaders"),
            &fl!("shaders-section"),
            "preferences-color-symbolic",
        );
        view_stack.add_titled_with_icon(
            preferences.widget(),
            Some("preferences"),
            &fl!("preferences"),
            "preferences-system-symbolic",
        );

        // Build ViewSwitcher wired to stack.
        let view_switcher = adw::ViewSwitcher::new();
        view_switcher.set_policy(adw::ViewSwitcherPolicy::Wide);
        view_switcher.set_stack(Some(&view_stack));

        // Build About dialog and register a win.about action.
        let about_dialog = adw::AboutDialog::builder()
            .application_name("Iris")
            .application_icon("iris")
            .developer_name("gnome-iris contributors")
            .version(env!("CARGO_PKG_VERSION"))
            .license_type(gtk::License::Gpl20)
            .comments("ReShade manager for Wine/Proton games on Linux")
            .build();
        about_dialog.add_link("reshade-steam-proton", "https://github.com/kevinlekiller/reshade-steam-proton");
        about_dialog.add_link("ReShade", "https://reshade.me/");
        about_dialog.add_link("ratic (codebase)", "https://gitlab.gnome.org/ratcornu/ratic");
        {
            let win = root.clone();
            let about_action = gtk::gio::SimpleAction::new("about", None);
            about_action.connect_activate(move |_, _| about_dialog.present(Some(&win)));
            root.add_action(&about_action);
        }

        // Build primary menu (⋮ button).
        let primary_menu = gtk::gio::Menu::new();
        primary_menu.append(Some(&fl!("about")), Some("win.about"));
        let menu_btn = gtk::MenuButton::new();
        menu_btn.set_icon_name("open-menu-symbolic");
        menu_btn.set_menu_model(Some(&primary_menu));

        // Build HeaderBar.
        let add_button = gtk::Button::from_icon_name("list-add-symbolic");
        add_button.set_tooltip_text(Some(&fl!("add-game")));
        add_button.connect_clicked({
            let s = sender.clone();
            move |_| s.input(Controls::Games(panel_games::GamesMsg::AddGameRequested))
        });
        let header_bar = adw::HeaderBar::new();
        header_bar.pack_start(&add_button);
        header_bar.pack_end(&menu_btn);
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
            add_game_dialog,
            nav_view: nav_view.clone(),
            toast_overlay: toast_overlay.clone(),
            pending_install: None,
            current_game_id: None,
            installed_versions,
            latest_version: None,
        };

        let nav_view = &nav_view;
        let toast_overlay = &toast_overlay;
        let widgets = view_output!();
        // Wire nav_view as the toast overlay's child (must be done after view_output!).
        widgets.toast_overlay.set_child(Some(nav_view));

        // Spawn startup version check respecting the configured interval.
        {
            use crate::reshade::cache::UpdateCache;
            use crate::reshade::d3dcompiler;
            use crate::reshade::game::ExeArch;
            use crate::reshade::reshade::fetch_latest_version;
            relm4::spawn(async move {
                let d3dc_dir = cache_data_dir.clone();
                let cache = UpdateCache::new(cache_data_dir);
                let version = if cache.needs_update(update_interval) {
                    match fetch_latest_version().await {
                        Ok(v) => {
                            if let Err(e) = cache.write_version(&v) {
                                log::warn!("Could not write version cache: {e}");
                            }
                            if let Err(e) = cache.touch() {
                                log::warn!("Could not touch update cache: {e}");
                            }
                            Some(v)
                        },
                        Err(e) => {
                            log::warn!("ReShade version check failed: {e}");
                            cache.read_version().unwrap_or(None)
                        },
                    }
                } else {
                    cache.read_version().unwrap_or(None)
                };
                if let Some(v) = version {
                    sender.input(Controls::Prefs(panel_preferences::PrefsMsg::LatestVersionFetched(v)));
                }

                // Ensure both d3dcompiler DLLs are present in the data directory.
                for arch in [ExeArch::X86, ExeArch::X86_64] {
                    if let Err(e) = d3dcompiler::ensure(&d3dc_dir, arch) {
                        log::warn!("Could not install d3dcompiler_47.dll: {e}");
                    }
                }

                // Detect install status for newly discovered Steam games off the main thread.
                for (id, path) in steam_paths_for_detection {
                    let status = detect_install_status(&path);
                    sender.input(Controls::Games(panel_games::GamesMsg::GameStatusDetected { id, status }));
                }
            });
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, _sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            Controls::Games(msg) => panel_games::handle(self, msg, root),
            Controls::Shaders(msg) => panel_shaders::handle(self, msg, root),
            Controls::Prefs(msg) => panel_preferences::handle(self, msg),
        }
    }
}

/// Determine which cached `ReShade` versions are currently in use by at least one game.
///
/// A version is "in use" when a game's DLL symlink points into that version's directory.
fn compute_versions_in_use(games: &[Game], data_dir: &std::path::Path) -> std::collections::HashSet<String> {
    let reshade_dir = data_dir.join("reshade");
    let mut in_use = std::collections::HashSet::new();
    for game in games {
        if let InstallStatus::Installed { dll, .. } = &game.status {
            let link = game.path.join(dll.symlink_name());
            if let Ok(target) = std::fs::read_link(&link) {
                let abs = if target.is_absolute() { target } else { game.path.join(&target) };
                if let Ok(rel) = abs.strip_prefix(&reshade_dir)
                    && let Some(comp) = rel.components().next()
                {
                    in_use.insert(comp.as_os_str().to_string_lossy().into_owned());
                }
            }
        }
    }
    in_use
}
