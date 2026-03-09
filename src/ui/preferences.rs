//! Global preferences page — shader repos, update interval, INI toggle.

use std::collections::{HashMap, HashSet};

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
    /// Versions currently symlinked by at least one game — cannot be removed.
    pub versions_in_use: HashSet<String>,
}

/// Preferences page model.
pub struct Preferences {
    config: GlobalConfig,
    installed_versions: Vec<String>,
    current_version: Option<String>,
    /// Versions locked by at least one game symlink — remove button disabled.
    versions_in_use: HashSet<String>,
    /// GTK row widgets for each installed version, keyed by version string.
    version_rows: HashMap<String, adw::ActionRow>,
    /// Remove buttons keyed by version string (used to toggle spinner).
    version_buttons: HashMap<String, gtk::Button>,
    /// Spinners keyed by version string.
    version_spinners: HashMap<String, gtk::Spinner>,
    /// The latest known version string (set by `SetLatestVersion`).
    latest_version: Option<String>,
    /// Extra row shown when latest version is not locally installed.
    latest_uninstalled_row: Option<adw::ActionRow>,
    /// Download button in the "latest not installed" row.
    install_button: Option<gtk::Button>,
    /// Spinner for the install operation.
    install_spinner: Option<gtk::Spinner>,
    /// Reference to the versions group widget for dynamic row management.
    versions_group: adw::PreferencesGroup,
    /// Placeholder row shown when no versions are installed.
    placeholder_row: Option<adw::ActionRow>,
    /// Tracks which version's action button is currently active (spinning).
    active_version_op: Option<String>,
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
    /// The latest available version was fetched from GitHub.
    SetLatestVersion(String),
    /// User clicked the install button for the latest uninstalled version.
    InstallLatestVersion(String),
    /// User clicked the remove button for an installed version.
    RemoveVersion(String),
    /// Worker completed the version download.
    VersionDownloadComplete(String),
    /// Inline removal (in window) completed successfully.
    VersionRemoveComplete(String),
    /// A version operation (install or remove) failed.
    VersionOpError(String),
}

/// Output signals from [`Preferences`].
#[derive(Debug)]
pub enum Signal {
    /// User changed and saved the configuration.
    ConfigChanged(GlobalConfig),
    /// Forward install request up to window.
    InstallVersionRequested(String),
    /// Forward remove request up to window.
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
            versions_in_use: init.versions_in_use,
            version_rows: HashMap::new(),
            version_buttons: HashMap::new(),
            version_spinners: HashMap::new(),
            latest_version: None,
            latest_uninstalled_row: None,
            install_button: None,
            install_spinner: None,
            versions_group: adw::PreferencesGroup::new(), // replaced below after view_output!
            placeholder_row: None,
            active_version_op: None,
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
        let (version_rows, version_buttons, version_spinners, placeholder_row) =
            if model.installed_versions.is_empty() {
                let row = adw::ActionRow::new();
                row.set_title("No versions installed");
                row.set_subtitle("Install ReShade from the game detail pane");
                widgets.versions_group.add(&row);
                (HashMap::new(), HashMap::new(), HashMap::new(), Some(row))
            } else {
                let mut rows = HashMap::new();
                let mut buttons = HashMap::new();
                let mut spinners = HashMap::new();
                for version in &model.installed_versions {
                    let sub = subtitle_for_installed(
                        version,
                        model.current_version.as_deref(),
                        false,
                    );
                    let is_in_use = model.versions_in_use.contains(version);
                    let (row, btn, spinner) =
                        build_installed_row(version, &sub, is_in_use, &sender);
                    widgets.versions_group.add(&row);
                    rows.insert(version.clone(), row);
                    buttons.insert(version.clone(), btn);
                    spinners.insert(version.clone(), spinner);
                }
                (rows, buttons, spinners, None)
            };

        // Rebuild model with actual widget references needed for dynamic updates.
        let model = Self {
            version_rows,
            version_buttons,
            version_spinners,
            placeholder_row,
            versions_group: widgets.versions_group.clone(),
            latest_uninstalled_row: None,
            install_button: None,
            install_spinner: None,
            active_version_op: None,
            ..model
        };

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
            Controls::SetLatestVersion(version) => {
                self.latest_version = Some(version.clone());
                if let Some(row) = self.version_rows.get(&version) {
                    // Already installed — update subtitle to reflect it is the latest.
                    let sub = subtitle_for_installed(
                        &version,
                        self.current_version.as_deref(),
                        true,
                    );
                    row.set_subtitle(&sub);
                } else {
                    // Not installed — remove placeholder if present, add/replace latest row.
                    if let Some(ph) = self.placeholder_row.take() {
                        self.versions_group.remove(&ph);
                    }
                    if let Some(old) = self.latest_uninstalled_row.take() {
                        self.versions_group.remove(&old);
                        self.install_button = None;
                        self.install_spinner = None;
                    }
                    let row = adw::ActionRow::new();
                    row.set_title(&version);
                    row.set_subtitle("latest available — not installed");

                    let btn = gtk::Button::from_icon_name("folder-download-symbolic");
                    btn.set_valign(gtk::Align::Center);
                    btn.add_css_class("flat");
                    btn.set_tooltip_text(Some("Download to cache"));
                    {
                        let v = version.clone();
                        let s = sender.clone();
                        btn.connect_clicked(move |_| {
                            s.input(Controls::InstallLatestVersion(v.clone()));
                        });
                    }

                    let spinner = gtk::Spinner::new();
                    spinner.set_valign(gtk::Align::Center);

                    let stack = gtk::Stack::new();
                    stack.set_valign(gtk::Align::Center);
                    stack.add_named(&btn, Some("button"));
                    stack.add_named(&spinner, Some("spinner"));
                    row.add_suffix(&stack);

                    self.versions_group.add(&row);
                    self.latest_uninstalled_row = Some(row);
                    self.install_button = Some(btn);
                    self.install_spinner = Some(spinner);
                }
            }
            Controls::InstallLatestVersion(version) => {
                if self.active_version_op.is_some() {
                    return;
                }
                self.active_version_op = Some(version.clone());
                begin_install_op(&self.install_button, &self.install_spinner);
                sender.output(Signal::InstallVersionRequested(version)).ok();
            }
            Controls::RemoveVersion(version) => {
                if self.active_version_op.is_some() {
                    return;
                }
                self.active_version_op = Some(version.clone());
                begin_version_op(&version, &self.version_buttons, &self.version_spinners);
                sender.output(Signal::RemoveVersionRequested(version)).ok();
            }
            Controls::VersionDownloadComplete(version) => {
                // Stop install spinner and remove the "not installed" row.
                finish_install_op(&self.install_button, &self.install_spinner);
                if let Some(old) = self.latest_uninstalled_row.take() {
                    self.versions_group.remove(&old);
                    self.install_button = None;
                    self.install_spinner = None;
                }
                // Add a new installed row for the downloaded version.
                if let Some(ph) = self.placeholder_row.take() {
                    self.versions_group.remove(&ph);
                }
                let sub = subtitle_for_installed(
                    &version,
                    self.current_version.as_deref(),
                    true,
                );
                let is_in_use = self.versions_in_use.contains(&version);
                let (row, btn, spinner) =
                    build_installed_row(&version, &sub, is_in_use, &sender);
                self.versions_group.add(&row);
                self.version_rows.insert(version.clone(), row);
                self.version_buttons.insert(version.clone(), btn);
                self.version_spinners.insert(version.clone(), spinner);
                self.installed_versions.push(version);
                self.active_version_op = None;
            }
            Controls::VersionRemoveComplete(version) => {
                finish_version_op(&version, &self.version_buttons, &self.version_spinners);
                if let Some(row) = self.version_rows.remove(&version) {
                    self.versions_group.remove(&row);
                }
                self.version_buttons.remove(&version);
                self.version_spinners.remove(&version);
                self.installed_versions.retain(|v| v != &version);
                // When the list is now empty, either restore the "latest not installed"
                // download row (if we know the latest version) or show the placeholder.
                if self.version_rows.is_empty() && self.latest_uninstalled_row.is_none() {
                    if let Some(latest) = self.latest_version.clone() {
                        // Re-add the "latest available — not installed" row with download button.
                        let row = adw::ActionRow::new();
                        row.set_title(&latest);
                        row.set_subtitle("latest available — not installed");

                        let btn = gtk::Button::from_icon_name("folder-download-symbolic");
                        btn.set_valign(gtk::Align::Center);
                        btn.add_css_class("flat");
                        btn.set_tooltip_text(Some("Download to cache"));
                        {
                            let v = latest.clone();
                            let s = sender.clone();
                            btn.connect_clicked(move |_| {
                                s.input(Controls::InstallLatestVersion(v.clone()));
                            });
                        }
                        let spinner = gtk::Spinner::new();
                        spinner.set_valign(gtk::Align::Center);
                        let stack = gtk::Stack::new();
                        stack.set_valign(gtk::Align::Center);
                        stack.add_named(&btn, Some("button"));
                        stack.add_named(&spinner, Some("spinner"));
                        row.add_suffix(&stack);

                        self.versions_group.add(&row);
                        self.latest_uninstalled_row = Some(row);
                        self.install_button = Some(btn);
                        self.install_spinner = Some(spinner);
                    } else {
                        let ph = adw::ActionRow::new();
                        ph.set_title("No versions installed");
                        ph.set_subtitle("Install ReShade from the game detail pane");
                        self.versions_group.add(&ph);
                        self.placeholder_row = Some(ph);
                    }
                }
                self.active_version_op = None;
            }
            Controls::VersionOpError(e) => {
                log::error!("Version operation failed: {e}");
                if let Some(version) = self.active_version_op.take() {
                    finish_version_op(&version, &self.version_buttons, &self.version_spinners);
                }
                finish_install_op(&self.install_button, &self.install_spinner);
            }
        }
    }
}

/// Compute the subtitle for an installed version row.
fn subtitle_for_installed(version: &str, current: Option<&str>, is_latest: bool) -> String {
    match (current == Some(version), is_latest) {
        (true, true) => "current · latest".to_owned(),
        (true, false) => "current".to_owned(),
        (false, true) => "latest".to_owned(),
        (false, false) => String::new(),
    }
}

/// Build an installed version row with a remove button (in a spinner stack).
///
/// Returns `(row, remove_button, spinner)` so the caller can register them.
fn build_installed_row(
    version: &str,
    subtitle: &str,
    is_in_use: bool,
    sender: &ComponentSender<Preferences>,
) -> (adw::ActionRow, gtk::Button, gtk::Spinner) {
    let row = adw::ActionRow::new();
    row.set_title(version);
    if !subtitle.is_empty() {
        row.set_subtitle(subtitle);
    }

    let btn = gtk::Button::from_icon_name("user-trash-symbolic");
    btn.set_valign(gtk::Align::Center);
    btn.add_css_class("flat");
    btn.set_sensitive(!is_in_use);
    btn.set_tooltip_text(Some(if is_in_use {
        "In use by a game — uninstall first"
    } else {
        "Remove version"
    }));
    {
        let v = version.to_owned();
        let s = sender.clone();
        btn.connect_clicked(move |_| s.input(Controls::RemoveVersion(v.clone())));
    }

    let spinner = gtk::Spinner::new();
    spinner.set_valign(gtk::Align::Center);

    let stack = gtk::Stack::new();
    stack.set_valign(gtk::Align::Center);
    stack.add_named(&btn, Some("button"));
    stack.add_named(&spinner, Some("spinner"));
    row.add_suffix(&stack);

    (row, btn, spinner)
}

/// Show spinner in place of remove button — call before emitting `RemoveVersionRequested`.
fn begin_version_op(
    version: &str,
    buttons: &HashMap<String, gtk::Button>,
    spinners: &HashMap<String, gtk::Spinner>,
) {
    if let Some(sp) = spinners.get(version) {
        sp.start();
    }
    if let Some(btn) = buttons.get(version) {
        if let Some(stack) = btn.parent().and_then(|p| p.downcast::<gtk::Stack>().ok()) {
            stack.set_visible_child_name("spinner");
        }
    }
}

/// Restore remove button in place of spinner — call after the operation completes.
fn finish_version_op(
    version: &str,
    buttons: &HashMap<String, gtk::Button>,
    spinners: &HashMap<String, gtk::Spinner>,
) {
    if let Some(btn) = buttons.get(version) {
        if let Some(stack) = btn.parent().and_then(|p| p.downcast::<gtk::Stack>().ok()) {
            stack.set_visible_child_name("button");
        }
    }
    if let Some(sp) = spinners.get(version) {
        sp.stop();
    }
}

/// Show spinner in place of the install button for the "latest not installed" row.
fn begin_install_op(button: &Option<gtk::Button>, spinner: &Option<gtk::Spinner>) {
    if let Some(sp) = spinner {
        sp.start();
    }
    if let Some(btn) = button {
        if let Some(stack) = btn.parent().and_then(|p| p.downcast::<gtk::Stack>().ok()) {
            stack.set_visible_child_name("spinner");
        }
    }
}

/// Restore the install button in place of the spinner.
fn finish_install_op(button: &Option<gtk::Button>, spinner: &Option<gtk::Spinner>) {
    if let Some(btn) = button {
        if let Some(stack) = btn.parent().and_then(|p| p.downcast::<gtk::Stack>().ok()) {
            stack.set_visible_child_name("button");
        }
    }
    if let Some(sp) = spinner {
        sp.stop();
    }
}
