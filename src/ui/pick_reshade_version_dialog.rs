//! Dialog for choosing which cached ReShade version to install into a game.

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, adw, gtk};

use crate::fl;

/// Model for the version-picker dialog.
pub struct PickReshadeVersionDialog {
    /// The version key currently selected by the user.
    selected: Option<String>,
    /// Stored ref so we can call `close()` from `update()`.
    dialog: adw::Dialog,
    /// Stored ref so we can clear and repopulate rows on re-open.
    list_box: gtk::ListBox,
}

/// Input messages for [`PickReshadeVersionDialog`].
#[derive(Debug)]
pub enum Controls {
    /// Populate the list and present the dialog.
    ///
    /// `parent` must be a widget currently in the window tree (used as the
    /// transient parent for `dialog.present()`).
    Open {
        /// Version keys to show (e.g. `"v6.3.0"`, `"v6.3.0-Addon"`).
        versions: Vec<String>,
        /// A widget in the window tree to use as the dialog parent.
        parent: gtk::Widget,
    },
    /// A radio button was toggled active — stores the version key.
    SelectVersion(String),
    /// Install button clicked.
    Confirm,
    /// Cancel button clicked or dialog dismissed.
    Cancel,
}

/// Output signals from [`PickReshadeVersionDialog`].
#[derive(Debug)]
pub enum Signal {
    /// Emitted on confirm with the chosen version key (e.g. `"v6.3.0"`).
    VersionChosen(String),
}

impl PickReshadeVersionDialog {
    /// Format a version key for display.
    ///
    /// Strips the leading `v` and converts `"-Addon"` to `" — Addon Support"`.
    /// Examples: `"v6.3.0"` → `"6.3.0"`, `"v6.3.0-Addon"` → `"6.3.0 — Addon Support"`.
    fn display_title(key: &str) -> String {
        let base = key.strip_prefix('v').unwrap_or(key);
        if let Some(ver) = base.strip_suffix("-Addon") {
            format!("{ver} \u{2014} {}", fl!("addon-support"))
        } else {
            base.to_owned()
        }
    }
}

#[allow(missing_docs)]
#[relm4::component(pub)]
impl SimpleComponent for PickReshadeVersionDialog {
    type Init = ();
    type Input = Controls;
    type Output = Signal;

    view! {
        #[name(dialog)]
        adw::Dialog {
            set_title: &fl!("pick-version-dialog-title"),
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
                        #[name(list_box)]
                        gtk::ListBox {
                            set_selection_mode: gtk::SelectionMode::None,
                            add_css_class: "boxed-list",
                        },
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_halign: gtk::Align::End,
                        set_spacing: 8,

                        #[name(cancel_btn)]
                        gtk::Button {
                            set_label: &fl!("pick-version-cancel-btn"),
                            add_css_class: "flat",
                        },

                        #[name(install_btn)]
                        gtk::Button {
                            set_label: &fl!("pick-version-install-btn"),
                            add_css_class: "suggested-action",
                            #[watch]
                            set_sensitive: model.selected.is_some(),
                        },
                    },
                },
            },
        }
    }

    fn init(
        (): (),
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = Self {
            selected: None,
            dialog: adw::Dialog::new(),
            list_box: gtk::ListBox::new(),
        };

        let widgets = view_output!();

        model.dialog = widgets.dialog.clone();
        model.list_box = widgets.list_box.clone();

        widgets.cancel_btn.connect_clicked({
            let s = sender.clone();
            move |_| s.input(Controls::Cancel)
        });
        widgets.install_btn.connect_clicked({
            move |_| sender.input(Controls::Confirm)
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, sender: ComponentSender<Self>) {
        match msg {
            Controls::Open { versions, parent } => {
                // 1. Clear existing rows.
                while let Some(child) = self.list_box.first_child() {
                    self.list_box.remove(&child);
                }
                // 2. Reset selection.
                self.selected = None;
                // 3. Guard: empty list should not happen (Install button is disabled
                //    in GameDetail when no versions exist), but be defensive.
                if versions.is_empty() {
                    return;
                }
                // 4. Build radio rows.
                let mut group_anchor: Option<gtk::CheckButton> = None;
                for key in &versions {
                    let check = gtk::CheckButton::new();
                    if let Some(anchor) = &group_anchor {
                        check.set_group(Some(anchor));
                    } else {
                        group_anchor = Some(check.clone());
                    }
                    {
                        let s = sender.clone();
                        let k = key.clone();
                        check.connect_toggled(move |btn| {
                            if btn.is_active() {
                                s.input(Controls::SelectVersion(k.clone()));
                            }
                        });
                    }
                    let title = Self::display_title(key);
                    let row = adw::ActionRow::new();
                    row.set_title(&title);
                    row.add_suffix(&check);
                    self.list_box.append(&row);
                }
                // 5. Present with the window as the transient parent.
                self.dialog.present(Some(&parent));
            }
            Controls::SelectVersion(v) => {
                self.selected = Some(v);
            }
            Controls::Cancel => {
                self.selected = None;
                self.dialog.close();
            }
            Controls::Confirm => {
                if let Some(version) = self.selected.take() {
                    sender.output(Signal::VersionChosen(version)).ok();
                    self.dialog.close();
                }
            }
        }
    }
}
