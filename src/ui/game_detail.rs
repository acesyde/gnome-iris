//! Detail pane showing a single game's `ReShade` status and controls.

use relm4::adw::prelude::*;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, adw, gtk,
};

use crate::fl;
use crate::reshade::config::{ShaderOverrides, ShaderRepo};
use crate::reshade::game::{DllOverride, ExeArch, Game, InstallStatus};
use crate::ui::pick_reshade_version_dialog;

/// Game detail pane model.
pub struct GameDetail {
    game: Option<Game>,
    progress_message: Option<String>,
    shader_repos: Vec<ShaderRepo>,
    shader_list: gtk::ListBox,
    /// Locally-cached `ReShade` version keys (e.g. `"v6.3.0"`), set by Window.
    installed_versions: Vec<String>,
    /// Dialog for picking which cached version to install.
    pick_version_dialog: relm4::Controller<pick_reshade_version_dialog::PickReshadeVersionDialog>,
}

/// Input messages for [`GameDetail`].
#[derive(Debug)]
pub enum Controls {
    /// Load a game into the pane.
    SetGame(Game),
    /// Clear the pane (no game selected).
    Clear,
    /// Show a progress message in the banner.
    SetProgress(String),
    /// Hide the progress banner.
    ClearProgress,
    /// Mark the currently shown game as installed.
    MarkInstalled {
        /// The installed version string.
        version: String,
        /// The DLL override used.
        dll: DllOverride,
        /// The detected architecture.
        arch: ExeArch,
    },
    /// Mark the currently shown game as uninstalled.
    MarkUninstalled,
    /// Replace shader rows for the displayed game.
    SetShaderData {
        /// Available shader repositories.
        repos: Vec<ShaderRepo>,
        /// Per-game shader overrides.
        overrides: ShaderOverrides,
    },
    /// Internal: install button clicked — `update()` reads `self.game`.
    InstallRequested,
    /// Internal: uninstall button clicked — `update()` reads `self.game`.
    UninstallRequested,
    /// Internal: open-folder button clicked — opens the game directory.
    OpenFolderRequested,
    /// Refresh the list of locally-cached `ReShade` versions available for install.
    SetInstalledVersions(Vec<String>),
    /// Internal: version picker dialog confirmed a version choice.
    VersionChosen(String),
    /// Internal: shader switch toggled — `update()` emits the output signal.
    ShaderToggled {
        /// Repository local name.
        repo_name: String,
        /// New enabled state.
        enabled: bool,
    },
}

/// Output signals from [`GameDetail`].
#[derive(Debug)]
pub enum Signal {
    /// User requested installation with these parameters.
    Install {
        /// Stable game ID.
        game_id: String,
        /// DLL override chosen.
        dll: DllOverride,
        /// Architecture detected/chosen.
        arch: ExeArch,
        /// The cached version key to install, e.g. `"v6.3.0"`.
        version: String,
    },
    /// User requested uninstallation.
    Uninstall {
        /// Stable game ID.
        game_id: String,
        /// Current DLL override.
        dll: DllOverride,
    },
    /// Per-game shader repo toggled.
    ShaderToggled {
        /// Stable game ID.
        game_id: String,
        /// Repository local name.
        repo_name: String,
        /// New enabled state.
        enabled: bool,
    },
}

impl GameDetail {
    /// Build the metadata subtitle string shown below the game title.
    fn metadata_subtitle(&self) -> String {
        let Some(game) = &self.game else {
            return String::new();
        };
        match &game.status {
            InstallStatus::NotInstalled => fl!("not-installed"),
            InstallStatus::Installed { dll, arch, version } => {
                let arch_str = match arch {
                    ExeArch::X86 => "x86",
                    ExeArch::X86_64 => "x86_64",
                };
                let version_str = version.as_deref().unwrap_or("?");
                format!("{arch_str} \u{00B7} {dll} \u{00B7} ReShade {version_str}")
            },
        }
    }

    /// Clear and repopulate the shader list box from `self.shader_repos`.
    fn rebuild_shader_rows(&self, sender: &ComponentSender<Self>) {
        while let Some(child) = self.shader_list.first_child() {
            self.shader_list.remove(&child);
        }
        if self.game.is_none() {
            return;
        }
        let disabled = self.game.as_ref().map(|g| g.shader_overrides.disabled_repos.clone()).unwrap_or_default();
        for repo in &self.shader_repos {
            let row = adw::SwitchRow::new();
            row.set_title(&repo.local_name);
            // Set initial state before connecting the notify handler so no
            // spurious ShaderToggled messages are emitted on load.
            row.set_active(!disabled.contains(&repo.local_name));
            let s = sender.clone();
            let name = repo.local_name.clone();
            row.connect_active_notify(move |r| {
                s.input(Controls::ShaderToggled {
                    repo_name: name.clone(),
                    enabled: r.is_active(),
                });
            });
            self.shader_list.append(&row);
        }
    }
}

#[allow(missing_docs)]
#[relm4::component(pub)]
impl SimpleComponent for GameDetail {
    type Init = ();
    type Input = Controls;
    type Output = Signal;

    view! {
        adw::NavigationPage {
            #[watch]
            set_title: model.game.as_ref().map_or("Game", |g| g.name.as_str()),

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {},

                #[wrap(Some)]
                set_content = &gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hscrollbar_policy: gtk::PolicyType::Never,

                    #[wrap(Some)]
                    set_child = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_margin_all: 24,
                        set_spacing: 24,

                        // Empty state
                        adw::StatusPage {
                            set_title: &fl!("select-a-game"),
                            set_icon_name: Some("view-list-symbolic"),
                            set_vexpand: true,
                            #[watch]
                            set_visible: model.game.is_none(),
                        },

                        // Content box
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 24,
                            #[watch]
                            set_visible: model.game.is_some(),

                            // Header (centered)
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_halign: gtk::Align::Center,
                                set_spacing: 4,

                                gtk::Label {
                                    add_css_class: "title-1",
                                    set_halign: gtk::Align::Center,
                                    #[watch]
                                    set_label: model.game.as_ref().map_or("", |g| g.name.as_str()),
                                },

                                gtk::Label {
                                    add_css_class: "caption",
                                    add_css_class: "dim-label",
                                    set_halign: gtk::Align::Center,
                                    #[watch]
                                    set_label: &model.metadata_subtitle(),
                                },
                            },

                            // Progress banner
                            adw::Banner {
                                #[watch]
                                set_title: model.progress_message.as_deref().unwrap_or(""),
                                #[watch]
                                set_revealed: model.progress_message.is_some(),
                                #[watch]
                                set_visible: model.progress_message.is_some(),
                            },

                            // No-versions warning banner
                            adw::Banner {
                                #[watch]
                                set_title: &fl!("no-versions-banner"),
                                #[watch]
                                set_revealed: model.game.is_some() && model.installed_versions.is_empty(),
                                #[watch]
                                set_visible: model.game.is_some() && model.installed_versions.is_empty(),
                            },

                            // Action buttons (centered, pill-shaped)
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_halign: gtk::Align::Center,
                                set_spacing: 12,

                                gtk::Button {
                                    set_label: &fl!("install"),
                                    add_css_class: "suggested-action",
                                    add_css_class: "pill",
                                    #[watch]
                                    set_visible: model
                                        .game
                                        .as_ref()
                                        .is_none_or(|g| !g.status.is_installed()),
                                    #[watch]
                                    set_sensitive: !model.installed_versions.is_empty()
                                        && model.game.as_ref().is_none_or(|g| !g.status.is_installed()),
                                    connect_clicked[sender] => move |_| {
                                        sender.input(Controls::InstallRequested);
                                    },
                                },

                                gtk::Button {
                                    set_label: &fl!("uninstall"),
                                    add_css_class: "destructive-action",
                                    add_css_class: "pill",
                                    #[watch]
                                    set_visible: model
                                        .game
                                        .as_ref()
                                        .is_some_and(|g| g.status.is_installed()),
                                    connect_clicked[sender] => move |_| {
                                        sender.input(Controls::UninstallRequested);
                                    },
                                },

                                gtk::Button {
                                    set_icon_name: "folder-open-symbolic",
                                    add_css_class: "flat",
                                    set_tooltip_text: Some(&fl!("open-game-folder")),
                                    set_valign: gtk::Align::Center,
                                    #[watch]
                                    set_visible: model.game.is_some(),
                                    connect_clicked[sender] => move |_| {
                                        sender.input(Controls::OpenFolderRequested);
                                    },
                                },
                            },

                            // Shaders section
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 8,
                                #[watch]
                                set_visible: !model.shader_repos.is_empty(),

                                gtk::Label {
                                    add_css_class: "heading",
                                    set_halign: gtk::Align::Start,
                                    set_label: &fl!("shaders-section"),
                                },

                                #[name(shader_list)]
                                gtk::ListBox {
                                    set_selection_mode: gtk::SelectionMode::None,
                                    add_css_class: "boxed-list",
                                },
                            },
                        },
                    },
                },
            },
        }
    }

    fn init((): (), root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let pick_version_dialog = pick_reshade_version_dialog::PickReshadeVersionDialog::builder().launch(()).forward(
            sender.input_sender(),
            |sig| match sig {
                pick_reshade_version_dialog::Signal::VersionChosen(v) => Controls::VersionChosen(v),
            },
        );
        let mut model = Self {
            game: None,
            progress_message: None,
            shader_repos: Vec::new(),
            shader_list: gtk::ListBox::new(),
            installed_versions: Vec::new(),
            pick_version_dialog,
        };
        let widgets = view_output!();
        // Replace the placeholder with the real widget from the view tree.
        model.shader_list = widgets.shader_list.clone();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, sender: ComponentSender<Self>) {
        match msg {
            Controls::SetGame(game) => {
                self.game = Some(game);
                self.progress_message = None;
                self.rebuild_shader_rows(&sender);
            },
            Controls::Clear => self.game = None,
            Controls::SetProgress(msg) => self.progress_message = Some(msg),
            Controls::ClearProgress => self.progress_message = None,
            Controls::MarkInstalled { dll, arch, version } => {
                if let Some(game) = &mut self.game {
                    game.status = InstallStatus::Installed {
                        dll,
                        arch,
                        version: Some(version),
                    };
                }
            },
            Controls::MarkUninstalled => {
                if let Some(game) = &mut self.game {
                    game.status = InstallStatus::NotInstalled;
                }
            },
            Controls::SetShaderData { repos, overrides } => {
                self.shader_repos = repos;
                if let Some(game) = &mut self.game {
                    game.shader_overrides = overrides;
                }
                self.rebuild_shader_rows(&sender);
            },
            Controls::InstallRequested => {
                use relm4::gtk::prelude::WidgetExt;
                if let Some(root) = self.shader_list.root() {
                    self.pick_version_dialog.emit(pick_reshade_version_dialog::Controls::Open {
                        versions: self.installed_versions.clone(),
                        parent: root.upcast::<gtk::Widget>(),
                    });
                }
            },
            Controls::SetInstalledVersions(versions) => {
                self.installed_versions = versions;
            },
            Controls::VersionChosen(version) => {
                if let Some(game) = &self.game {
                    let (dll, arch) = match &game.status {
                        InstallStatus::Installed { dll, arch, .. } => (*dll, *arch),
                        InstallStatus::NotInstalled => {
                            (DllOverride::Dxgi, game.preferred_arch.unwrap_or(ExeArch::X86_64))
                        },
                    };
                    sender
                        .output(Signal::Install {
                            game_id: game.id.clone(),
                            dll,
                            arch,
                            version,
                        })
                        .ok();
                }
            },
            Controls::UninstallRequested => {
                if let Some(game) = &self.game
                    && let InstallStatus::Installed { dll, .. } = &game.status
                {
                    sender
                        .output(Signal::Uninstall {
                            game_id: game.id.clone(),
                            dll: *dll,
                        })
                        .ok();
                }
            },
            Controls::ShaderToggled { repo_name, enabled } => {
                if let Some(game) = &self.game {
                    sender
                        .output(Signal::ShaderToggled {
                            game_id: game.id.clone(),
                            repo_name,
                            enabled,
                        })
                        .ok();
                }
            },
            Controls::OpenFolderRequested => {
                if let Some(game) = &self.game {
                    let file = gtk::gio::File::for_path(&game.path);
                    let launcher = gtk::FileLauncher::new(Some(&file));
                    launcher.launch(gtk::Window::NONE, gtk::gio::Cancellable::NONE, |_| {});
                }
            },
        }
    }
}
