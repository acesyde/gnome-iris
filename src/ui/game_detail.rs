//! Detail pane showing a single game's ReShade status and controls.

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, adw, gtk};

use crate::fl;
use crate::reshade::game::{DllOverride, ExeArch, Game, InstallStatus};

/// Game detail pane model.
pub struct GameDetail {
    game: Option<Game>,
    progress_message: Option<String>,
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
    },
    /// User requested uninstallation.
    Uninstall {
        /// Stable game ID.
        game_id: String,
        /// Current DLL override.
        dll: DllOverride,
    },
}

#[allow(missing_docs)]
#[relm4::component(pub)]
impl SimpleComponent for GameDetail {
    type Init = ();
    type Input = Controls;
    type Output = Signal;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_margin_all: 24,
            set_spacing: 16,

            // Empty state
            adw::StatusPage {
                set_title: &fl!("select-a-game"),
                set_icon_name: Some("view-list-symbolic"),
                set_vexpand: true,
                #[watch]
                set_visible: model.game.is_none(),
            },

            // Game content
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 12,
                set_vexpand: true,
                #[watch]
                set_visible: model.game.is_some(),

                #[name(game_name_label)]
                gtk::Label {
                    add_css_class: "title-1",
                    #[watch]
                    set_label: model.game.as_ref().map(|g| g.name.as_str()).unwrap_or(""),
                    set_xalign: 0.0,
                },

                #[name(game_path_label)]
                gtk::Label {
                    add_css_class: "caption",
                    set_xalign: 0.0,
                    set_ellipsize: gtk::pango::EllipsizeMode::Middle,
                    #[watch]
                    set_label: model
                        .game
                        .as_ref()
                        .map(|g| g.path.to_string_lossy().into_owned())
                        .unwrap_or_default()
                        .as_str(),
                },

                // Progress banner
                adw::Banner {
                    #[watch]
                    set_title: model.progress_message.as_deref().unwrap_or(""),
                    #[watch]
                    set_revealed: model.progress_message.is_some(),
                },

                // Action buttons
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 8,

                    gtk::Button {
                        set_label: &fl!("install"),
                        add_css_class: "suggested-action",
                        #[watch]
                        set_visible: model
                            .game
                            .as_ref()
                            .map(|g| !g.status.is_installed())
                            .unwrap_or(true),
                        connect_clicked[sender] => move |_| {
                            sender
                                .output(Signal::Install {
                                    game_id: String::new(),
                                    dll: DllOverride::Dxgi,
                                    arch: ExeArch::X86_64,
                                })
                                .ok();
                        },
                    },

                    gtk::Button {
                        set_label: &fl!("uninstall"),
                        add_css_class: "destructive-action",
                        #[watch]
                        set_visible: model
                            .game
                            .as_ref()
                            .map(|g| g.status.is_installed())
                            .unwrap_or(false),
                        connect_clicked[sender] => move |_| {
                            sender
                                .output(Signal::Uninstall {
                                    game_id: String::new(),
                                    dll: DllOverride::Dxgi,
                                })
                                .ok();
                        },
                    },
                },
            },
        }
    }

    fn init(
        _: (),
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            game: None,
            progress_message: None,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, _sender: ComponentSender<Self>) {
        match msg {
            Controls::SetGame(game) => self.game = Some(game),
            Controls::Clear => self.game = None,
            Controls::SetProgress(msg) => self.progress_message = Some(msg),
            Controls::ClearProgress => self.progress_message = None,
            Controls::MarkInstalled { dll, arch, .. } => {
                if let Some(game) = &mut self.game {
                    game.status = InstallStatus::Installed { dll, arch };
                }
            }
            Controls::MarkUninstalled => {
                if let Some(game) = &mut self.game {
                    game.status = InstallStatus::NotInstalled;
                }
            }
        }
    }
}
