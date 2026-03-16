//! About dialog for gnome-iris.

use relm4::adw;
use relm4::gtk;
use relm4::{ComponentParts, ComponentSender, SimpleComponent};

/// About dialog component.
#[allow(clippy::module_name_repetitions)]
pub struct AboutDialog;

/// Input messages for [`AboutDialog`].
#[derive(Debug)]
pub enum Controls {
    /// Present the about dialog.
    Show,
}

#[allow(missing_docs)]
#[relm4::component(pub)]
impl SimpleComponent for AboutDialog {
    type Init = ();
    type Input = Controls;
    type Output = ();

    view! {
        #[name(dialog)]
        adw::AboutDialog {
            set_application_name: "Iris",
            set_application_icon: "iris",
            set_developer_name: "gnome-iris contributors",
            set_version: env!("CARGO_PKG_VERSION"),
            set_license_type: gtk::License::Gpl20,
            set_comments: "ReShade manager for Wine/Proton games on Linux",
        }
    }

    fn init((): (), root: Self::Root, _sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = Self;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, _sender: ComponentSender<Self>) {
        match msg {
            Controls::Show => {},
        }
    }
}
