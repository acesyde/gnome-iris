//! Dialog for adding a custom shader git repository.

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, adw, gtk};

use crate::fl;
use crate::reshade::config::ShaderRepo;

/// Dialog model for adding a custom shader repository.
pub struct AddShaderRepoDialog {
    name: String,
    url: String,
    git_ref: String,
    /// Widget refs stored for programmatic reset on confirm.
    name_entry: adw::EntryRow,
    url_entry: adw::EntryRow,
    ref_entry: adw::EntryRow,
    dialog: adw::Dialog,
}

/// Input messages for [`AddShaderRepoDialog`].
#[derive(Debug)]
pub enum Controls {
    /// Present the dialog over the given root widget.
    Open,
    /// Name field changed.
    SetName(String),
    /// URL field changed.
    SetUrl(String),
    /// Branch/tag field changed.
    SetRef(String),
    /// User clicked the confirm button.
    Confirm,
}

/// Output signals from [`AddShaderRepoDialog`].
#[derive(Debug)]
pub enum Signal {
    /// User confirmed — contains the constructed repo.
    RepoAdded(ShaderRepo),
}

#[allow(missing_docs)]
#[relm4::component(pub)]
impl SimpleComponent for AddShaderRepoDialog {
    type Init = ();
    type Input = Controls;
    type Output = Signal;

    view! {
        #[name(dialog)]
        adw::Dialog {
            set_title: &fl!("add-custom-repo"),
            set_content_width: 400,

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {},

                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 24,
                    set_spacing: 12,

                    adw::PreferencesGroup {
                        #[name(name_entry)]
                        adw::EntryRow {
                            set_title: &fl!("dialog-name"),
                        },

                        #[name(url_entry)]
                        adw::EntryRow {
                            set_title: &fl!("dialog-url"),
                        },

                        #[name(ref_entry)]
                        adw::EntryRow {
                            set_title: &fl!("dialog-branch-tag"),
                        },
                    },

                    #[name(confirm_btn)]
                    gtk::Button {
                        set_label: &fl!("dialog-add"),
                        add_css_class: "suggested-action",
                        #[watch]
                        set_sensitive: !model.name.is_empty() && !model.url.is_empty(),
                    },
                },
            },
        }
    }

    fn init((): (), root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let mut model = Self {
            name: String::new(),
            url: String::new(),
            git_ref: String::new(),
            name_entry: adw::EntryRow::new(),
            url_entry: adw::EntryRow::new(),
            ref_entry: adw::EntryRow::new(),
            dialog: adw::Dialog::new(),
        };

        let widgets = view_output!();

        // Store real widget refs for programmatic access in update().
        model.name_entry = widgets.name_entry.clone();
        model.url_entry = widgets.url_entry.clone();
        model.ref_entry = widgets.ref_entry.clone();
        model.dialog = widgets.dialog.clone();

        widgets.name_entry.connect_changed({
            let s = sender.clone();
            move |e| s.input(Controls::SetName(e.text().to_string()))
        });
        widgets.url_entry.connect_changed({
            let s = sender.clone();
            move |e| s.input(Controls::SetUrl(e.text().to_string()))
        });
        widgets.ref_entry.connect_changed({
            let s = sender.clone();
            move |e| s.input(Controls::SetRef(e.text().to_string()))
        });
        widgets.confirm_btn.connect_clicked(move |_| sender.input(Controls::Confirm));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, sender: ComponentSender<Self>) {
        match msg {
            Controls::Open => {},
            Controls::SetName(v) => self.name = v,
            Controls::SetUrl(v) => self.url = v,
            Controls::SetRef(v) => self.git_ref = v,
            Controls::Confirm => {
                let name = self.name.trim().to_owned();
                let url = self.url.trim().to_owned();
                if name.is_empty() || url.is_empty() {
                    return;
                }
                let branch = {
                    let r = self.git_ref.trim().to_owned();
                    if r.is_empty() { None } else { Some(r) }
                };
                let repo = ShaderRepo {
                    url,
                    local_name: name,
                    branch,
                    enabled_by_default: false,
                };
                sender.output(Signal::RepoAdded(repo)).ok();
                // Reset fields.
                self.name = String::new();
                self.url = String::new();
                self.git_ref = String::new();
                self.name_entry.set_text("");
                self.url_entry.set_text("");
                self.ref_entry.set_text("");
                self.dialog.close();
            },
        }
    }
}
