//! Global preferences dialog — shader repos, update interval, INI toggle.

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use crate::fl;
use crate::reshade::config::GlobalConfig;

/// Preferences dialog model.
pub struct Preferences {
    config: GlobalConfig,
}

/// Input messages for [`Preferences`].
#[derive(Debug)]
pub enum Controls {
    /// Open the dialog.
    Open,
    /// Update the displayed configuration.
    SetConfig(GlobalConfig),
}

/// Output signals from [`Preferences`].
#[derive(Debug)]
pub enum Signal {
    /// User changed and saved the configuration.
    ConfigChanged(GlobalConfig),
}

/// Widgets for the [`Preferences`] component.
pub struct PreferencesWidgets {
    #[allow(dead_code)]
    dialog: adw::PreferencesDialog,
    merge_row: adw::SwitchRow,
    ini_row: adw::SwitchRow,
}

#[allow(missing_docs)]
impl SimpleComponent for Preferences {
    type Init = GlobalConfig;
    type Input = Controls;
    type Output = Signal;
    type Root = adw::PreferencesDialog;
    type Widgets = PreferencesWidgets;

    fn init_root() -> Self::Root {
        adw::PreferencesDialog::new()
    }

    fn init(
        config: GlobalConfig,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        root.set_title(&fl!("preferences"));

        // Shaders page
        let shaders_page = adw::PreferencesPage::new();
        shaders_page.set_title("Shaders");
        shaders_page.set_icon_name(Some("preferences-system-symbolic"));

        let shader_group = adw::PreferencesGroup::new();
        shader_group.set_title("Shader Repositories");
        shader_group
            .set_description(Some(
                "Repos are cloned in order; first match wins on name collision.",
            ));

        let merge_row = adw::SwitchRow::new();
        merge_row.set_title("Merge shaders");
        merge_row.set_subtitle("Combine all repos into a single directory");
        merge_row.set_active(config.merge_shaders);

        let ini_row = adw::SwitchRow::new();
        ini_row.set_title("Global ReShade.ini");
        ini_row.set_subtitle("Share one config file across all games");
        ini_row.set_active(config.global_ini);

        shader_group.add(&merge_row);
        shader_group.add(&ini_row);
        shaders_page.add(&shader_group);
        root.add(&shaders_page);

        // Updates page
        let updates_page = adw::PreferencesPage::new();
        updates_page.set_title("Updates");
        updates_page.set_icon_name(Some("software-update-available-symbolic"));

        let update_group = adw::PreferencesGroup::new();
        update_group.set_title("Update Check");

        let adjustment = gtk::Adjustment::new(
            f64::from(u32::try_from(config.update_interval_hours).unwrap_or(4)),
            1.0,
            168.0,
            1.0,
            0.0,
            0.0,
        );
        let spin_row = adw::SpinRow::new(Some(&adjustment), 1.0, 0);
        spin_row.set_title("Check interval (hours)");
        spin_row.set_subtitle("How often to check for a new ReShade release");

        update_group.add(&spin_row);
        updates_page.add(&update_group);
        root.add(&updates_page);

        let model = Self { config };
        let widgets = PreferencesWidgets {
            dialog: root,
            merge_row,
            ini_row,
        };
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, _sender: ComponentSender<Self>) {
        match msg {
            Controls::Open => {}
            Controls::SetConfig(config) => {
                self.config = config;
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        widgets.merge_row.set_active(self.config.merge_shaders);
        widgets.ini_row.set_active(self.config.global_ini);
    }
}
