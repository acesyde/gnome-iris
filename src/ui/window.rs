//! Root application window.

use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw};
use relm4::adw::prelude::*;

/// Root window model.
pub struct Window;

/// Input messages for [`Window`].
#[derive(Debug)]
pub enum Controls {}

#[allow(missing_docs)]
#[relm4::component(pub)]
impl SimpleComponent for Window {
    type Init = ();
    type Input = Controls;
    type Output = ();

    view! {
        adw::ApplicationWindow {
            set_title: Some("Iris"),
            set_default_width: 1000,
            set_default_height: 700,
        }
    }

    fn init(_: (), _root: Self::Root, _sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = Self;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, _msg: Controls, _sender: ComponentSender<Self>) {}
}
