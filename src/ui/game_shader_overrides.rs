//! Per-game shader repository override panel.

use relm4::gtk::prelude::*;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, gtk};

use crate::reshade::config::{GlobalConfig, ShaderOverrides};

/// Per-game shader override panel.
pub struct GameShaderOverrides {
    overrides: ShaderOverrides,
    config: GlobalConfig,
}

/// Input messages for [`GameShaderOverrides`].
#[derive(Debug)]
pub enum Controls {
    /// Update the displayed config and overrides.
    SetData(GlobalConfig, ShaderOverrides),
}

/// Output signals from [`GameShaderOverrides`].
#[derive(Debug)]
pub enum Signal {
    /// User toggled a repo — carries the updated overrides.
    OverrideChanged(ShaderOverrides),
}

#[allow(missing_docs)]
#[relm4::component(pub)]
impl SimpleComponent for GameShaderOverrides {
    type Init = (GlobalConfig, ShaderOverrides);
    type Input = Controls;
    type Output = Signal;

    view! {
        #[name(list_box)]
        gtk::ListBox {
            set_selection_mode: gtk::SelectionMode::None,
            add_css_class: "boxed-list",
        }
    }

    fn init(
        (config, overrides): (GlobalConfig, ShaderOverrides),
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self { overrides, config };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, _sender: ComponentSender<Self>) {
        match msg {
            Controls::SetData(config, overrides) => {
                self.config = config;
                self.overrides = overrides;
            }
        }
    }
}
