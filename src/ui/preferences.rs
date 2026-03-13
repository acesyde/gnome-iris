//! Global preferences page — shader repos, update interval, INI toggle.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use crate::fl;
use crate::reshade::config::GlobalConfig;

/// Initialization payload for [`Preferences`].
pub struct PreferencesInit {
    /// App data directory (e.g. `~/.local/share/iris/`).
    pub data_dir: PathBuf,
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
    data_dir: PathBuf,
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
    /// Extra row shown when latest standard version is not locally installed.
    latest_uninstalled_row: Option<adw::ActionRow>,
    /// Download button in the "latest not installed" row.
    install_button: Option<gtk::Button>,
    /// Spinner for the standard install operation.
    install_spinner: Option<gtk::Spinner>,
    /// Extra row shown when latest Addon Support version is not locally installed.
    latest_addon_uninstalled_row: Option<adw::ActionRow>,
    /// Download button in the "latest addon not installed" row.
    install_addon_button: Option<gtk::Button>,
    /// Spinner for the addon install operation.
    install_addon_spinner: Option<gtk::Spinner>,
    /// Reference to the versions group widget for dynamic row management.
    versions_group: adw::PreferencesGroup,
    /// Placeholder row shown when no versions are installed.
    placeholder_row: Option<adw::ActionRow>,
    /// In-flight operation keys — allows standard and addon downloads to run concurrently.
    active_ops: HashSet<String>,
}

/// Input messages for [`Preferences`].
#[derive(Debug)]
pub enum Controls {
    /// Update the displayed configuration.
    SetConfig(GlobalConfig),
    /// User toggled the merge-shaders switch.
    MergeShadersChanged(bool),
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

                adw::PreferencesPage {
                    set_title: "ReShade",
                    set_icon_name: Some("application-x-executable-symbolic"),

                    #[name(versions_group)]
                    adw::PreferencesGroup {
                        set_title: &fl!("installed-versions"),
                        set_description: Some(&fl!("installed-versions-description")),
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
            data_dir: init.data_dir,
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
            latest_addon_uninstalled_row: None,
            install_addon_button: None,
            install_addon_spinner: None,
            versions_group: adw::PreferencesGroup::new(), // replaced below after view_output!
            placeholder_row: None,
            active_ops: HashSet::new(),
        };
        let widgets = view_output!();

        widgets.merge_row.connect_active_notify({
            let s = sender.clone();
            move |row| s.input(Controls::MergeShadersChanged(row.is_active()))
        });
        widgets.spin_row.connect_value_notify({
            let s = sender.clone();
            move |row: &adw::SpinRow| s.input(Controls::UpdateIntervalChanged(row.value()))
        });

        // Attach an "open folder" button to the versions group header.
        {
            let reshade_dir = model.data_dir.join("reshade");
            let open_btn = gtk::Button::from_icon_name("folder-open-symbolic");
            open_btn.set_valign(gtk::Align::Center);
            open_btn.add_css_class("flat");
            open_btn.set_tooltip_text(Some(&fl!("open-reshade-folder")));
            open_btn.connect_clicked(move |_| {
                let _ = std::fs::create_dir_all(&reshade_dir);
                std::process::Command::new("xdg-open")
                    .arg(reshade_dir.as_os_str())
                    .spawn()
                    .ok();
            });
            widgets.versions_group.set_header_suffix(Some(&open_btn));
        }

        // Populate installed versions rows imperatively (runtime data, can't use view! macro).
        let (version_rows, version_buttons, version_spinners, placeholder_row) =
            if model.installed_versions.is_empty() {
                let row = adw::ActionRow::new();
                row.set_title(&fl!("no-versions-installed"));
                row.set_subtitle(&fl!("no-versions-subtitle"));
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
            latest_addon_uninstalled_row: None,
            install_addon_button: None,
            install_addon_spinner: None,
            active_ops: HashSet::new(),
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
                let addon_key = format!("{version}-Addon");

                // Remove placeholder once before adding any new rows.
                if let Some(ph) = self.placeholder_row.take() {
                    self.versions_group.remove(&ph);
                }

                // Standard variant.
                if let Some(row) = self.version_rows.get(&version) {
                    let sub = subtitle_for_installed(
                        &version,
                        self.current_version.as_deref(),
                        true,
                    );
                    row.set_subtitle(&sub);
                } else if self.latest_uninstalled_row.is_none() {
                    let (row, btn, spinner) = build_uninstalled_row(&version, &sender);
                    self.versions_group.add(&row);
                    self.latest_uninstalled_row = Some(row);
                    self.install_button = Some(btn);
                    self.install_spinner = Some(spinner);
                }

                // Addon Support variant.
                if let Some(row) = self.version_rows.get(&addon_key) {
                    let sub = subtitle_for_installed(
                        &addon_key,
                        self.current_version.as_deref(),
                        true,
                    );
                    row.set_subtitle(&sub);
                } else if self.latest_addon_uninstalled_row.is_none() {
                    let (row, btn, spinner) = build_uninstalled_row(&addon_key, &sender);
                    self.versions_group.add(&row);
                    self.latest_addon_uninstalled_row = Some(row);
                    self.install_addon_button = Some(btn);
                    self.install_addon_spinner = Some(spinner);
                }
            }
            Controls::InstallLatestVersion(version_key) => {
                if self.active_ops.contains(&version_key) {
                    return;
                }
                self.active_ops.insert(version_key.clone());
                if version_key.ends_with("-Addon") {
                    begin_install_op(&self.install_addon_button, &self.install_addon_spinner);
                } else {
                    begin_install_op(&self.install_button, &self.install_spinner);
                }
                sender
                    .output(Signal::InstallVersionRequested(version_key))
                    .ok();
            }
            Controls::RemoveVersion(version) => {
                if self.active_ops.contains(&version) {
                    return;
                }
                self.active_ops.insert(version.clone());
                begin_version_op(&version, &self.version_buttons, &self.version_spinners);
                sender.output(Signal::RemoveVersionRequested(version)).ok();
            }
            Controls::VersionDownloadComplete(version_key) => {
                let is_addon = version_key.ends_with("-Addon");

                // Stop the correct install spinner and remove the correct uninstalled row.
                if is_addon {
                    finish_install_op(&self.install_addon_button, &self.install_addon_spinner);
                    if let Some(old) = self.latest_addon_uninstalled_row.take() {
                        self.versions_group.remove(&old);
                        self.install_addon_button = None;
                        self.install_addon_spinner = None;
                    }
                } else {
                    finish_install_op(&self.install_button, &self.install_spinner);
                    if let Some(old) = self.latest_uninstalled_row.take() {
                        self.versions_group.remove(&old);
                        self.install_button = None;
                        self.install_spinner = None;
                    }
                }

                // Remove placeholder if present.
                if let Some(ph) = self.placeholder_row.take() {
                    self.versions_group.remove(&ph);
                }

                let sub = subtitle_for_installed(
                    &version_key,
                    self.current_version.as_deref(),
                    true,
                );
                let is_in_use = self.versions_in_use.contains(&version_key);
                let (row, btn, spinner) =
                    build_installed_row(&version_key, &sub, is_in_use, &sender);
                self.versions_group.add(&row);
                self.version_rows.insert(version_key.clone(), row);
                self.version_buttons.insert(version_key.clone(), btn);
                self.version_spinners.insert(version_key.clone(), spinner);
                self.installed_versions.push(version_key.clone());
                self.active_ops.remove(&version_key);
            }
            Controls::VersionRemoveComplete(version) => {
                finish_version_op(&version, &self.version_buttons, &self.version_spinners);
                if let Some(row) = self.version_rows.remove(&version) {
                    self.versions_group.remove(&row);
                }
                self.version_buttons.remove(&version);
                self.version_spinners.remove(&version);
                self.installed_versions.retain(|v| v != &version);

                // Restore the download row for the variant that was just removed.
                let is_addon = version.ends_with("-Addon");
                if let Some(latest) = self.latest_version.clone() {
                    if is_addon && self.latest_addon_uninstalled_row.is_none() {
                        let addon_key = format!("{latest}-Addon");
                        let (row, btn, spinner) = build_uninstalled_row(&addon_key, &sender);
                        self.versions_group.add(&row);
                        self.latest_addon_uninstalled_row = Some(row);
                        self.install_addon_button = Some(btn);
                        self.install_addon_spinner = Some(spinner);
                    } else if !is_addon && self.latest_uninstalled_row.is_none() {
                        let (row, btn, spinner) = build_uninstalled_row(&latest, &sender);
                        self.versions_group.add(&row);
                        self.latest_uninstalled_row = Some(row);
                        self.install_button = Some(btn);
                        self.install_spinner = Some(spinner);
                    }
                } else if self.version_rows.is_empty() {
                    // Latest version unknown and nothing left — show placeholder.
                    let ph = adw::ActionRow::new();
                    ph.set_title(&fl!("no-versions-installed"));
                    ph.set_subtitle(&fl!("no-versions-subtitle"));
                    self.versions_group.add(&ph);
                    self.placeholder_row = Some(ph);
                }
                self.active_ops.remove(&version);
            }
            Controls::VersionOpError(e) => {
                log::error!("Version operation failed: {e}");
                self.active_ops.clear();
                let versions: Vec<String> = self.version_spinners.keys().cloned().collect();
                for v in &versions {
                    finish_version_op(v, &self.version_buttons, &self.version_spinners);
                }
                finish_install_op(&self.install_button, &self.install_spinner);
                finish_install_op(&self.install_addon_button, &self.install_addon_spinner);
            }
        }
    }
}

/// Format a version key for display: `"6.7.3-Addon"` → `"6.7.3 — Addon Support"`.
fn display_title(version_key: &str) -> String {
    if let Some(base) = version_key.strip_suffix("-Addon") {
        format!("{base} — {}", fl!("addon-support"))
    } else {
        version_key.to_owned()
    }
}

/// Build a "latest available — not installed" row with a download button/spinner stack.
fn build_uninstalled_row(
    version_key: &str,
    sender: &ComponentSender<Preferences>,
) -> (adw::ActionRow, gtk::Button, gtk::Spinner) {
    let row = adw::ActionRow::new();
    row.set_title(&display_title(version_key));
    row.set_subtitle(&fl!("latest-not-installed"));

    let btn = gtk::Button::from_icon_name("folder-download-symbolic");
    btn.set_valign(gtk::Align::Center);
    btn.add_css_class("flat");
    btn.set_tooltip_text(Some(&fl!("download-to-cache")));
    {
        let vk = version_key.to_owned();
        let s = sender.clone();
        btn.connect_clicked(move |_| {
            s.input(Controls::InstallLatestVersion(vk.clone()));
        });
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

/// Compute the subtitle for an installed version row.
fn subtitle_for_installed(version: &str, current: Option<&str>, is_latest: bool) -> String {
    match (current == Some(version), is_latest) {
        (true, true) => fl!("version-status-current-latest"),
        (true, false) => fl!("version-status-current"),
        (false, true) => fl!("version-status-latest"),
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
    row.set_title(&display_title(version));
    if !subtitle.is_empty() {
        row.set_subtitle(subtitle);
    }

    let btn = gtk::Button::from_icon_name("user-trash-symbolic");
    btn.set_valign(gtk::Align::Center);
    btn.add_css_class("flat");
    btn.set_sensitive(!is_in_use);
    let tip = if is_in_use {
        fl!("version-in-use")
    } else {
        fl!("remove-version")
    };
    btn.set_tooltip_text(Some(&tip));
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
