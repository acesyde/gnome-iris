//! Scrollable list of game cards.

use std::collections::HashMap;

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, adw, gtk};

use crate::fl;
use crate::reshade::game::{Game, GameSource};

/// Game list model.
pub struct GameList {
    /// All games to display.
    games: Vec<Game>,
    /// Widget ref for dynamically appending rows.
    list_box: gtk::ListBox,
    /// Row widgets keyed by game ID — needed for imperative removal.
    rows: HashMap<String, adw::ActionRow>,
}

/// Input messages for [`GameList`].
#[derive(Debug)]
pub enum Controls {
    /// Replace the full game list.
    SetGames(Vec<Game>),
    /// Append a single game row.
    AddGame(Game),
    /// Remove the row for the given game ID.
    RemoveGame(String),
}

/// Output signals from [`GameList`].
#[derive(Debug)]
pub enum Signal {
    /// User selected a game by its stable ID.
    GameSelected(String),
    /// User clicked the remove button on a manually added game.
    GameRemoveRequested(String),
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
            rows: HashMap::new(),
        };
        let widgets = view_output!();
        model.list_box = widgets.list_box.clone();

        // Populate initial rows.
        for game in &games {
            let row = build_game_row(game, &sender);
            widgets.list_box.append(&row);
            model.rows.insert(game.id.clone(), row);
        }

        // Emit selection signal when a row is activated.
        widgets.list_box.connect_row_activated({
            let s = sender.clone();
            move |_, row| {
                let id = row.widget_name().to_string();
                s.output(Signal::GameSelected(id)).ok();
            }
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, sender: ComponentSender<Self>) {
        match msg {
            Controls::SetGames(games) => self.games = games,
            Controls::AddGame(game) => {
                let row = build_game_row(&game, &sender);
                self.list_box.append(&row);
                self.rows.insert(game.id.clone(), row);
                self.games.push(game);
            }
            Controls::RemoveGame(id) => {
                if let Some(row) = self.rows.remove(&id) {
                    self.list_box.remove(&row);
                }
                self.games.retain(|g| g.id != id);
            }
        }
    }
}

/// Builds an [`adw::ActionRow`] card for a single game.
///
/// Manual games get a trash button; Steam-discovered games get only the chevron.
fn build_game_row(game: &Game, sender: &ComponentSender<GameList>) -> adw::ActionRow {
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

    if matches!(game.source, GameSource::Manual) {
        let btn = gtk::Button::from_icon_name("user-trash-symbolic");
        btn.set_valign(gtk::Align::Center);
        btn.add_css_class("flat");
        btn.set_tooltip_text(Some("Remove game"));
        let id = game.id.clone();
        let s = sender.clone();
        btn.connect_clicked(move |_| {
            s.output(Signal::GameRemoveRequested(id.clone())).ok();
        });
        row.add_suffix(&btn);
    }

    let chevron = gtk::Image::from_icon_name("go-next-symbolic");
    row.add_suffix(&chevron);

    row
}
