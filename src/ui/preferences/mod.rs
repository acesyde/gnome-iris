//! Global preferences page — config panel (shaders, update interval) with
//! the [`panel_versions`] child component for `ReShade` version management.
#![allow(clippy::cast_precision_loss)]

pub mod panel_versions;

use std::collections::HashSet;
use std::path::PathBuf;

use relm4::adw::prelude::*;
use relm4::{Component, ComponentController, ComponentParts, ComponentSender, Controller, SimpleComponent, adw, gtk};

use crate::fl;
use crate::reshade::config::GlobalConfig;

/// Initialization payload for [`Preferences`].
#[allow(clippy::module_name_repetitions)]
pub struct PreferencesInit {
    /// App data directory (e.g. `~/.local/share/iris/`).
    pub data_dir: PathBuf,
    /// Current global configuration.
    pub config: GlobalConfig,
    /// All locally installed `ReShade` version directories.
    pub installed_versions: Vec<String>,
    /// The currently active version (from `LVERS`), if any.
    pub current_version: Option<String>,
    /// Versions currently symlinked by at least one game — cannot be removed.
    pub versions_in_use: HashSet<String>,
}

/// Preferences page model.
pub struct Preferences {
    config: GlobalConfig,
    versions: Controller<panel_versions::Versions>,
}

/// Input messages for [`Preferences`].
#[derive(Debug)]
pub enum Controls {
    /// Replace the displayed configuration with `config`.
    ///
    /// Purely a model update — does **not** emit [`Signal::ConfigChanged`];
    /// use this for programmatic refreshes, not user-initiated changes.
    SetConfig(GlobalConfig),
    /// User toggled the merge-shaders switch; `bool` is the new enabled state.
    ///
    /// Emits [`Signal::ConfigChanged`] only when the value actually changes.
    MergeShadersChanged(bool),
    /// User changed the update-interval spin row; `f64` is the new interval in **hours**.
    ///
    /// Emits [`Signal::ConfigChanged`] only when the value actually changes.
    UpdateIntervalChanged(f64),
    /// The latest available `ReShade` version (e.g. `"6.3.0"`) was fetched.
    ///
    /// Forwarded to the versions panel to add "install latest" rows for variants
    /// not already in the local cache.
    SetLatestVersion(String),
    /// Open the manual version install dialog.
    ///
    /// Forwarded to the versions panel.
    OpenInstallVersionDialog,
    /// The version download initiated by the versions panel completed.
    ///
    /// Forwarded to the versions panel to add an installed row and remove
    /// the corresponding "not installed" row.
    VersionDownloadComplete(String),
    /// The version removal initiated by the versions panel completed.
    ///
    /// Forwarded to the versions panel to remove the installed row and restore
    /// the "latest not installed" row if needed.
    VersionRemoveComplete(String),
    /// A version install or remove operation failed; `msg` is a human-readable error.
    ///
    /// Forwarded to the versions panel to reset all in-flight spinners.
    VersionOpError(String),
    /// Bridge: versions panel emitted [`panel_versions::Signal::InstallVersionRequested`].
    ///
    /// Re-emitted as [`Signal::InstallVersionRequested`] so the window handler can
    /// forward the job to the install worker.
    VersionInstallRequested(String),
    /// Bridge: versions panel emitted [`panel_versions::Signal::RemoveVersionRequested`].
    ///
    /// Re-emitted as [`Signal::RemoveVersionRequested`] so the window handler can
    /// delete the cached directory.
    VersionRemoveRequested(String),
}

/// Output signals from [`Preferences`].
#[derive(Debug)]
pub enum Signal {
    /// The user changed a setting; `config` is the full updated configuration.
    ///
    /// The window handler should persist this to disk.
    ConfigChanged(GlobalConfig),
    /// The user requested that a version be downloaded to the local cache.
    ///
    /// `version_key` is a full version key, e.g. `"v6.1.0"` or `"v6.1.0-Addon"`.
    /// The window handler should forward this to the install worker, then respond
    /// with [`Controls::VersionDownloadComplete`] or [`Controls::VersionOpError`].
    InstallVersionRequested(String),
    /// The user requested that a cached version be deleted from disk.
    ///
    /// `version_key` is the version to remove, e.g. `"v6.1.0"`.
    /// The window handler should delete the directory, then respond with
    /// [`Controls::VersionRemoveComplete`] or [`Controls::VersionOpError`].
    RemoveVersionRequested(String),
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

            #[name(content_box)]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 0,

                adw::PreferencesPage {
                    set_title: &fl!("shaders-section"),
                    set_icon_name: Some("preferences-system-symbolic"),

                    adw::PreferencesGroup {
                        set_title: &fl!("shader-repositories"),
                        set_description: Some(&fl!("shader-repos-description")),

                        #[name(merge_row)]
                        adw::SwitchRow {
                            set_title: &fl!("merge-shaders"),
                            set_subtitle: &fl!("merge-shaders-subtitle"),
                            #[watch]
                            set_active: model.config.merge_shaders,
                        },
                    },
                },

                adw::PreferencesPage {
                    set_title: &fl!("updates"),
                    set_icon_name: Some("software-update-available-symbolic"),

                    adw::PreferencesGroup {
                        set_title: &fl!("update-check"),

                        #[name(spin_row)]
                        adw::SpinRow {
                            set_title: &fl!("update-interval"),
                            set_subtitle: &fl!("update-interval-subtitle"),
                            set_adjustment: Some(&gtk::Adjustment::new(4.0, 1.0, 168.0, 1.0, 0.0, 0.0)),
                            set_digits: 0,
                            set_snap_to_ticks: true,
                            #[watch]
                            set_value: model.config.update_interval_hours as f64,
                        },
                    },
                },
            },
        }
    }

    fn init(init: PreferencesInit, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let versions = panel_versions::Versions::builder()
            .launch(panel_versions::VersionsInit {
                data_dir: init.data_dir,
                installed_versions: init.installed_versions,
                current_version: init.current_version,
                versions_in_use: init.versions_in_use,
            })
            .forward(sender.input_sender(), |sig| match sig {
                panel_versions::Signal::InstallVersionRequested(k) => Controls::VersionInstallRequested(k),
                panel_versions::Signal::RemoveVersionRequested(v) => Controls::VersionRemoveRequested(v),
            });

        let model = Self { config: init.config, versions };
        let widgets = view_output!();

        widgets.merge_row.connect_active_notify({
            let s = sender.clone();
            move |row| s.input(Controls::MergeShadersChanged(row.is_active()))
        });
        widgets.spin_row.connect_value_notify({
            move |row: &adw::SpinRow| sender.input(Controls::UpdateIntervalChanged(row.value()))
        });

        widgets.content_box.append(model.versions.widget());

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, sender: ComponentSender<Self>) {
        match msg {
            Controls::SetConfig(config) => {
                self.config = config;
            },
            Controls::MergeShadersChanged(val) => {
                if self.config.merge_shaders != val {
                    self.config.merge_shaders = val;
                    sender.output(Signal::ConfigChanged(self.config.clone())).ok();
                }
            },
            Controls::UpdateIntervalChanged(val) => {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let hours = val as u64;
                if self.config.update_interval_hours != hours {
                    self.config.update_interval_hours = hours;
                    sender.output(Signal::ConfigChanged(self.config.clone())).ok();
                }
            },
            Controls::SetLatestVersion(v) => {
                self.versions.emit(panel_versions::Controls::SetLatestVersion(v));
            },
            Controls::OpenInstallVersionDialog => {
                self.versions.emit(panel_versions::Controls::OpenInstallVersionDialog);
            },
            Controls::VersionDownloadComplete(v) => {
                self.versions.emit(panel_versions::Controls::VersionDownloadComplete(v));
            },
            Controls::VersionRemoveComplete(v) => {
                self.versions.emit(panel_versions::Controls::VersionRemoveComplete(v));
            },
            Controls::VersionOpError(e) => {
                self.versions.emit(panel_versions::Controls::VersionOpError(e));
            },
            Controls::VersionInstallRequested(k) => {
                sender.output(Signal::InstallVersionRequested(k)).ok();
            },
            Controls::VersionRemoveRequested(v) => {
                sender.output(Signal::RemoveVersionRequested(v)).ok();
            },
        }
    }
}
