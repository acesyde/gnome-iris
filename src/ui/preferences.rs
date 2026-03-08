//! Global preferences page — shader repos, update interval, INI toggle.

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use crate::reshade::config::GlobalConfig;

/// Preferences page model.
pub struct Preferences {
    config: GlobalConfig,
}

/// Input messages for [`Preferences`].
#[derive(Debug)]
pub enum Controls {
    /// Update the displayed configuration.
    SetConfig(GlobalConfig),
}

/// Output signals from [`Preferences`].
#[derive(Debug)]
pub enum Signal {
    /// User changed and saved the configuration.
    ConfigChanged(GlobalConfig),
}

#[allow(missing_docs)]
#[relm4::component(pub)]
impl SimpleComponent for Preferences {
    type Init = GlobalConfig;
    type Input = Controls;
    type Output = Signal;

    view! {
        gtk::ScrolledWindow {
            set_vexpand: true,
            set_hscrollbar_policy: gtk::PolicyType::Never,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 0,

                adw::PreferencesPage {
                    set_title: "Shaders",
                    set_icon_name: Some("preferences-system-symbolic"),

                    adw::PreferencesGroup {
                        set_title: "Shader Repositories",
                        set_description: Some(
                            "Repos are cloned in order; first match wins on name collision.",
                        ),

                        #[name(merge_row)]
                        adw::SwitchRow {
                            set_title: "Merge shaders",
                            set_subtitle: "Combine all repos into a single directory",
                            #[watch]
                            set_active: model.config.merge_shaders,
                        },

                        #[name(ini_row)]
                        adw::SwitchRow {
                            set_title: "Global ReShade.ini",
                            set_subtitle: "Share one config file across all games",
                            #[watch]
                            set_active: model.config.global_ini,
                        },
                    },
                },

                adw::PreferencesPage {
                    set_title: "Updates",
                    set_icon_name: Some("software-update-available-symbolic"),

                    adw::PreferencesGroup {
                        set_title: "Update Check",

                        adw::SpinRow {
                            set_title: "Check interval (hours)",
                            set_subtitle: "How often to check for a new ReShade release",
                            set_adjustment: Some(&gtk::Adjustment::new(4.0, 1.0, 168.0, 1.0, 0.0, 0.0)),
                            set_digits: 0,
                            set_snap_to_ticks: true,
                        },
                    },
                },
            },
        }
    }

    fn init(
        config: GlobalConfig,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self { config };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, _sender: ComponentSender<Self>) {
        match msg {
            Controls::SetConfig(config) => {
                self.config = config;
            }
        }
    }
}
