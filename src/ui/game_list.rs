//! Sidebar component listing all known games.

use relm4::gtk::prelude::*;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, gtk};

use crate::reshade::game::Game;

/// Sidebar game list model.
pub struct GameList {
    /// All games to display.
    games: Vec<Game>,
}

/// Input messages for [`GameList`].
#[derive(Debug)]
pub enum Controls {
    /// Replace the full game list.
    SetGames(Vec<Game>),
}

/// Output signals from [`GameList`].
#[derive(Debug)]
pub enum Signal {
    /// User selected a game by its stable ID.
    GameSelected(String),
    /// User clicked the Add Game button.
    AddGameRequested,
}

#[allow(missing_docs)]
#[relm4::component(pub)]
impl SimpleComponent for GameList {
    type Init = Vec<Game>;
    type Input = Controls;
    type Output = Signal;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,

            gtk::ScrolledWindow {
                set_vexpand: true,
                set_hscrollbar_policy: gtk::PolicyType::Never,

                #[name(list_box)]
                gtk::ListBox {
                    set_selection_mode: gtk::SelectionMode::Single,
                    add_css_class: "navigation-sidebar",
                },
            },

            gtk::Separator {},

            gtk::Button {
                set_label: "+ Add Game",
                set_margin_all: 8,
                connect_clicked[sender] => move |_| {
                    sender.output(Signal::AddGameRequested).ok();
                },
            },
        }
    }

    fn init(
        games: Vec<Game>,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            games: games.clone(),
        };
        let widgets = view_output!();

        // Populate initial rows
        for game in &games {
            let row = make_game_row(game);
            widgets.list_box.append(&row);
        }

        // Emit selection signal when a row is activated
        let sender2 = sender.clone();
        widgets.list_box.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                let id = row.widget_name().to_string();
                sender2.output(Signal::GameSelected(id)).ok();
            }
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, _sender: ComponentSender<Self>) {
        match msg {
            Controls::SetGames(games) => self.games = games,
        }
    }
}

/// Builds a `ListBoxRow` for a single game.
fn make_game_row(game: &Game) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_widget_name(&game.id);

    let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    hbox.set_margin_all(8);

    let label = gtk::Label::new(Some(&game.name));
    label.set_hexpand(true);
    label.set_xalign(0.0);
    hbox.append(&label);

    if game.status.is_installed() {
        let check = gtk::Image::from_icon_name("emblem-ok-symbolic");
        hbox.append(&check);
    }

    row.set_child(Some(&hbox));
    row
}
