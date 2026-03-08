//! Global preferences page — shader repos, update interval, INI toggle.

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use crate::reshade::config::GlobalConfig;

/// Initialization payload for [`Preferences`].
pub struct PreferencesInit {
    /// Current global configuration.
    pub config: GlobalConfig,
    /// All locally installed ReShade version directories.
    pub installed_versions: Vec<String>,
    /// The currently active version (from `LVERS`), if any.
    pub current_version: Option<String>,
}

/// Preferences page model.
pub struct Preferences {
    config: GlobalConfig,
    installed_versions: Vec<String>,
    current_version: Option<String>,
}

/// Input messages for [`Preferences`].
#[derive(Debug)]
pub enum Controls {
    /// Update the displayed configuration.
    SetConfig(GlobalConfig),
    /// User toggled the merge-shaders switch.
    MergeShadersChanged(bool),
    /// User toggled the global-INI switch.
    GlobalIniChanged(bool),
    /// User changed the update-interval spin row.
    UpdateIntervalChanged(f64),
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
    type Init = PreferencesInit;
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

                        #[name(spin_row)]
                        adw::SpinRow {
                            set_title: "Check interval (hours)",
                            set_subtitle: "How often to check for a new ReShade release",
                            set_adjustment: Some(&gtk::Adjustment::new(4.0, 1.0, 168.0, 1.0, 0.0, 0.0)),
                            set_digits: 0,
                            set_snap_to_ticks: true,
                            #[watch]
                            set_value: model.config.update_interval_hours as f64,
                        },
                    },
                },

                adw::PreferencesPage {
                    set_title: "ReShade",
                    set_icon_name: Some("application-x-executable-symbolic"),

                    #[name(versions_group)]
                    adw::PreferencesGroup {
                        set_title: "Installed Versions",
                        set_description: Some("Versions downloaded to the local cache."),
                    },
                },
            },
        }
    }

    fn init(
        init: PreferencesInit,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            config: init.config,
            installed_versions: init.installed_versions,
            current_version: init.current_version,
        };
        let widgets = view_output!();

        widgets.merge_row.connect_active_notify({
            let s = sender.clone();
            move |row| s.input(Controls::MergeShadersChanged(row.is_active()))
        });
        widgets.ini_row.connect_active_notify({
            let s = sender.clone();
            move |row| s.input(Controls::GlobalIniChanged(row.is_active()))
        });
        widgets.spin_row.connect_value_notify({
            let s = sender.clone();
            move |row: &adw::SpinRow| s.input(Controls::UpdateIntervalChanged(row.value()))
        });

        // Populate installed versions rows imperatively (runtime data, can't use view! macro).
        if model.installed_versions.is_empty() {
            let row = adw::ActionRow::new();
            row.set_title("No versions installed");
            row.set_subtitle("Install ReShade from the game detail pane");
            widgets.versions_group.add(&row);
        } else {
            for version in &model.installed_versions {
                let row = adw::ActionRow::new();
                row.set_title(version);
                if model.current_version.as_deref() == Some(version.as_str()) {
                    row.set_subtitle("current");
                    let icon = gtk::Image::from_icon_name("emblem-default-symbolic");
                    row.add_suffix(&icon);
                }
                widgets.versions_group.add(&row);
            }
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, sender: ComponentSender<Self>) {
        match msg {
            Controls::SetConfig(config) => {
                self.config = config;
            }
            Controls::MergeShadersChanged(val) => {
                if self.config.merge_shaders != val {
                    self.config.merge_shaders = val;
                    sender.output(Signal::ConfigChanged(self.config.clone())).ok();
                }
            }
            Controls::GlobalIniChanged(val) => {
                if self.config.global_ini != val {
                    self.config.global_ini = val;
                    sender.output(Signal::ConfigChanged(self.config.clone())).ok();
                }
            }
            Controls::UpdateIntervalChanged(val) => {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let hours = val as u64;
                if self.config.update_interval_hours != hours {
                    self.config.update_interval_hours = hours;
                    sender.output(Signal::ConfigChanged(self.config.clone())).ok();
                }
            }
        }
    }
}
