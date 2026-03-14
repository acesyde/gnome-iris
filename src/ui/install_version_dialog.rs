//! Dialog for manually installing a specific `ReShade` version by version number.

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, adw, gtk};

use crate::fl;

/// Dialog model for installing a specific `ReShade` version.
pub struct InstallVersionDialog {
    version_text: String,
    addon: bool,
    installed_versions: Vec<String>,
    /// Stored ref for programmatic reset on confirm.
    entry: adw::EntryRow,
    /// Stored ref for programmatic reset on confirm.
    addon_check: gtk::CheckButton,
    /// Stored ref for `close()` on confirm.
    dialog: adw::Dialog,
}

/// Input messages for [`InstallVersionDialog`].
#[derive(Debug)]
pub enum Controls {
    /// Reset all fields and prepare dialog for a fresh open.
    Reset,
    /// Version entry text changed.
    SetVersion(String),
    /// Addon Support checkbox toggled.
    SetAddon(bool),
    /// Refresh the installed-versions list used for duplicate detection.
    UpdateInstalledVersions(Vec<String>),
    /// User clicked the Install button.
    Confirm,
}

/// Output signals from [`InstallVersionDialog`].
#[derive(Debug)]
pub enum Signal {
    /// User confirmed — contains the fully-formed version key
    /// (`"6.3.0"` or `"6.3.0-Addon"`).
    InstallRequested(String),
}

impl InstallVersionDialog {
    /// Returns `true` when `version_text` matches the `X.Y.Z` format
    /// (three dot-separated non-empty numeric parts).
    fn is_valid(&self) -> bool {
        let parts: Vec<&str> = self.version_text.split('.').collect();
        parts.len() == 3 && parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
    }

    /// Returns `true` when the would-be version key is already installed.
    fn is_duplicate(&self) -> bool {
        self.installed_versions.contains(&self.version_key())
    }

    /// Builds the version key from the current state, prefixing with `v`.
    fn version_key(&self) -> String {
        if self.addon { format!("v{}-Addon", self.version_text) } else { format!("v{}", self.version_text) }
    }
}

#[allow(missing_docs)]
#[relm4::component(pub)]
impl SimpleComponent for InstallVersionDialog {
    type Init = ();
    type Input = Controls;
    type Output = Signal;

    view! {
        #[name(dialog)]
        adw::Dialog {
            set_title: &fl!("install-version-dialog-title"),
            set_content_width: 380,

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {},

                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 24,
                    set_spacing: 12,

                    adw::PreferencesGroup {
                        #[name(entry)]
                        adw::EntryRow {
                            set_title: &fl!("install-version-entry-title"),
                        },
                    },

                    #[name(addon_check)]
                    gtk::CheckButton {
                        set_label: Some(&fl!("addon-support")),
                        set_margin_start: 4,
                    },

                    gtk::Label {
                        add_css_class: "error",
                        set_xalign: 0.0,
                        set_margin_start: 4,
                        #[watch]
                        set_visible: model.is_duplicate(),
                        set_label: &fl!("install-version-already-installed"),
                    },

                    #[name(confirm_btn)]
                    gtk::Button {
                        set_label: &fl!("install-version-install-btn"),
                        add_css_class: "suggested-action",
                        #[watch]
                        set_sensitive: model.is_valid() && !model.is_duplicate(),
                    },
                },
            },
        }
    }

    fn init((): (), root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let mut model = Self {
            version_text: String::new(),
            addon: false,
            installed_versions: Vec::new(),
            entry: adw::EntryRow::new(),
            addon_check: gtk::CheckButton::new(),
            dialog: adw::Dialog::new(),
        };

        let widgets = view_output!();

        model.entry = widgets.entry.clone();
        model.addon_check = widgets.addon_check.clone();
        model.dialog = widgets.dialog.clone();

        widgets.entry.connect_changed({
            let s = sender.clone();
            move |e| s.input(Controls::SetVersion(e.text().to_string()))
        });
        widgets.addon_check.connect_toggled({
            let s = sender.clone();
            move |check| s.input(Controls::SetAddon(check.is_active()))
        });
        widgets.confirm_btn.connect_clicked(move |_| sender.input(Controls::Confirm));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, sender: ComponentSender<Self>) {
        match msg {
            Controls::Reset => {
                self.version_text = String::new();
                self.addon = false;
                self.entry.set_text("");
                self.addon_check.set_active(false);
            },
            Controls::SetVersion(v) => {
                self.version_text = v;
            },
            Controls::SetAddon(v) => {
                self.addon = v;
            },
            Controls::UpdateInstalledVersions(v) => {
                self.installed_versions = v;
            },
            Controls::Confirm => {
                if !self.is_valid() || self.is_duplicate() {
                    return;
                }
                sender.output(Signal::InstallRequested(self.version_key())).ok();
                self.version_text = String::new();
                self.addon = false;
                self.entry.set_text("");
                self.addon_check.set_active(false);
                self.dialog.close();
            },
        }
    }
}
