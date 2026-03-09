//! Scrollable list of game cards.

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, adw, gtk};

use crate::fl;
use crate::reshade::game::Game;

/// Game list model.
pub struct GameList {
    /// All games to display.
    games: Vec<Game>,
    /// Widget ref for dynamically appending rows.
    list_box: gtk::ListBox,
}

/// Input messages for [`GameList`].
#[derive(Debug)]
pub enum Controls {
    /// Replace the full game list.
    SetGames(Vec<Game>),
    /// Append a single game row.
    AddGame(Game),
}

/// Output signals from [`GameList`].
#[derive(Debug)]
pub enum Signal {
    /// User selected a game by its stable ID.
    GameSelected(String),
}

#[allow(missing_docs)]
#[relm4::component(pub)]
impl SimpleComponent for GameList {
    type Init = Vec<Game>;
    type Input = Controls;
    type Output = Signal;

    view! {
        gtk::ScrolledWindow {
            set_vexpand: true,
            set_hscrollbar_policy: gtk::PolicyType::Never,

            #[name(list_box)]
            gtk::ListBox {
                set_selection_mode: gtk::SelectionMode::None,
                add_css_class: "boxed-list",
                set_margin_all: 12,
                set_valign: gtk::Align::Start,
            },
        }
    }

    fn init(
        games: Vec<Game>,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = Self {
            games: games.clone(),
            list_box: gtk::ListBox::new(),
        };
        let widgets = view_output!();
        model.list_box = widgets.list_box.clone();

        // Populate initial rows
        for game in &games {
            let row = make_game_card(game);
            widgets.list_box.append(&row);
        }

        // Emit selection signal when a row is activated
        let sender2 = sender.clone();
        widgets.list_box.connect_row_activated(move |_, row| {
            let id = row.widget_name().to_string();
            sender2.output(Signal::GameSelected(id)).ok();
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, _sender: ComponentSender<Self>) {
        match msg {
            Controls::SetGames(games) => self.games = games,
            Controls::AddGame(game) => {
                let row = make_game_card(&game);
                self.list_box.append(&row);
                self.games.push(game);
            }
        }
    }
}

/// Builds an `adw::ActionRow` card for a single game.
fn make_game_card(game: &Game) -> adw::ActionRow {
    let row = adw::ActionRow::new();
    row.set_widget_name(&game.id);
    row.set_title(&game.name);
    let subtitle = if game.status.is_installed() {
        String::from("ReShade installed")
    } else {
        fl!("not-installed")
    };
    row.set_subtitle(&subtitle);
    row.set_activatable(true);

    let prefix = gtk::Image::from_icon_name("application-x-executable-symbolic");
    row.add_prefix(&prefix);

    let suffix = gtk::Image::from_icon_name("go-next-symbolic");
    row.add_suffix(&suffix);

    row
}
