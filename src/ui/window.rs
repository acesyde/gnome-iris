//! Root application window.

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use crate::fl;

/// Root window model.
pub struct Window {
    /// Whether the sidebar is shown.
    sidebar_visible: bool,
}

/// Input messages for [`Window`].
#[derive(Debug)]
pub enum Controls {
    /// Toggle the sidebar visibility.
    ToggleSidebar,
}

#[allow(missing_docs)]
#[relm4::component(pub)]
impl SimpleComponent for Window {
    type Init = ();
    type Input = Controls;
    type Output = ();

    view! {
        adw::ApplicationWindow {
            set_title: Some(&fl!("app-title")),
            set_default_width: 1000,
            set_default_height: 700,

            adw::OverlaySplitView {
                #[watch]
                set_show_sidebar: model.sidebar_visible,

                #[wrap(Some)]
                set_sidebar = &adw::NavigationPage {
                    set_title: &fl!("app-title"),
                    set_width_request: 260,

                    adw::ToolbarView {
                        add_top_bar = &adw::HeaderBar {
                            pack_start = &gtk::Button {
                                set_icon_name: "folder-open-symbolic",
                                set_tooltip_text: Some(&fl!("add-game")),
                            },
                        },

                        gtk::Label {
                            set_label: "Game list placeholder",
                            set_vexpand: true,
                        },
                    },
                },

                #[wrap(Some)]
                set_content = &adw::NavigationPage {
                    set_title: "Detail",

                    adw::ToolbarView {
                        add_top_bar = &adw::HeaderBar {},

                        adw::StatusPage {
                            set_title: &fl!("select-a-game"),
                            set_icon_name: Some("view-list-symbolic"),
                        },
                    },
                },
            },
        }
    }

    fn init(
        _: (),
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            sidebar_visible: true,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, _sender: ComponentSender<Self>) {
        match msg {
            Controls::ToggleSidebar => self.sidebar_visible = !self.sidebar_visible,
        }
    }
}
