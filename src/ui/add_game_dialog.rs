//! Dialog for manually adding a game by path.

use std::path::PathBuf;

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, adw, gtk};

use crate::fl;

/// Add game dialog model.
pub struct AddGameDialog {
    selected_path: Option<PathBuf>,
}

/// Input messages for [`AddGameDialog`].
#[derive(Debug)]
pub enum Controls {
    /// Open the dialog attached to the given window.
    Open,
    /// Set the selected path from a file chooser.
    SetPath(PathBuf),
}

/// Output signals from [`AddGameDialog`].
#[derive(Debug)]
pub enum Signal {
    /// User confirmed a game path.
    GamePathSelected(PathBuf),
}

#[allow(missing_docs)]
#[relm4::component(pub)]
impl SimpleComponent for AddGameDialog {
    type Init = ();
    type Input = Controls;
    type Output = Signal;

    view! {
        #[name(dialog)]
        adw::Dialog {
            set_title: &fl!("add-game"),
            set_content_width: 400,

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {},

                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 24,
                    set_spacing: 12,

                    gtk::Label {
                        set_label: "Select the directory containing the game .exe",
                        set_wrap: true,
                        set_xalign: 0.0,
                    },

                    #[name(path_label)]
                    gtk::Label {
                        add_css_class: "caption",
                        set_xalign: 0.0,
                        set_ellipsize: gtk::pango::EllipsizeMode::Middle,
                        #[watch]
                        set_label: model
                            .selected_path
                            .as_ref()
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "(none selected)".into())
                            .as_str(),
                    },

                    gtk::Button {
                        set_label: &fl!("add-game"),
                        add_css_class: "suggested-action",
                        #[watch]
                        set_sensitive: model.selected_path.is_some(),
                        connect_clicked[sender] => move |_| {
                            // Confirm is handled via output — path already set
                            let _ = sender.input_sender();
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
        let model = Self { selected_path: None };
        let widgets = view_output!();
        let _ = sender; // suppress unused warning
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, sender: ComponentSender<Self>) {
        match msg {
            Controls::Open => {}
            Controls::SetPath(path) => {
                self.selected_path = Some(path.clone());
                sender.output(Signal::GamePathSelected(path)).ok();
            }
        }
    }
}
