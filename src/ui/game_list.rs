//! Scrollable list of game cards, split into auto-detected and manually added sections.

use std::collections::HashMap;

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, adw, gtk};

use crate::fl;
use crate::reshade::game::{Game, GameSource};

/// Game list model.
pub struct GameList {
    /// All games to display.
    games: Vec<Game>,
    /// Whether at least one manually added game exists (drives section visibility).
    has_manual: bool,
    /// List box for auto-detected games.
    auto_list_box: gtk::ListBox,
    /// List box for manually added games.
    manual_list_box: gtk::ListBox,
    /// Row widgets for auto-detected games, keyed by game ID.
    auto_rows: HashMap<String, adw::ActionRow>,
    /// Row widgets for manually added games, keyed by game ID.
    manual_rows: HashMap<String, adw::ActionRow>,
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
    /// Update the install-status subtitle for a game row.
    SetGameStatus {
        /// Stable game ID.
        id: String,
        /// Whether ReShade is now installed.
        installed: bool,
    },
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

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 18,
                set_margin_all: 12,

                // Auto-detected section
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,

                    gtk::Label {
                        add_css_class: "heading",
                        set_halign: gtk::Align::Start,
                        set_label: &fl!("autodetected"),
                    },

                    #[name(auto_list_box)]
                    gtk::ListBox {
                        set_selection_mode: gtk::SelectionMode::None,
                        add_css_class: "boxed-list",
                        set_valign: gtk::Align::Start,
                    },
                },

                // Manually added section — hidden until at least one game exists
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,
                    #[watch]
                    set_visible: model.has_manual,

                    gtk::Label {
                        add_css_class: "heading",
                        set_halign: gtk::Align::Start,
                        set_label: &fl!("manually-added"),
                    },

                    #[name(manual_list_box)]
                    gtk::ListBox {
                        set_selection_mode: gtk::SelectionMode::None,
                        add_css_class: "boxed-list",
                        set_valign: gtk::Align::Start,
                    },
                },
            },
        }
    }

    fn init(
        games: Vec<Game>,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let has_manual = games.iter().any(|g| matches!(g.source, GameSource::Manual));
        let mut model = Self {
            games: games.clone(),
            has_manual,
            auto_list_box: gtk::ListBox::new(),
            manual_list_box: gtk::ListBox::new(),
            auto_rows: HashMap::new(),
            manual_rows: HashMap::new(),
        };
        let widgets = view_output!();
        model.auto_list_box = widgets.auto_list_box.clone();
        model.manual_list_box = widgets.manual_list_box.clone();

        for game in &games {
            let row = build_game_row(game, &sender);
            if matches!(game.source, GameSource::Manual) {
                widgets.manual_list_box.append(&row);
                model.manual_rows.insert(game.id.clone(), row);
            } else {
                widgets.auto_list_box.append(&row);
                model.auto_rows.insert(game.id.clone(), row);
            }
        }

        let connect_selection = |list_box: &gtk::ListBox| {
            let s = sender.clone();
            list_box.connect_row_activated(move |_, row| {
                let id = row.widget_name().to_string();
                s.output(Signal::GameSelected(id)).ok();
            });
        };
        connect_selection(&widgets.auto_list_box);
        connect_selection(&widgets.manual_list_box);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, sender: ComponentSender<Self>) {
        match msg {
            Controls::SetGames(games) => self.games = games,
            Controls::AddGame(game) => {
                let row = build_game_row(&game, &sender);
                if matches!(game.source, GameSource::Manual) {
                    self.manual_list_box.append(&row);
                    self.manual_rows.insert(game.id.clone(), row);
                } else {
                    self.auto_list_box.append(&row);
                    self.auto_rows.insert(game.id.clone(), row);
                }
                self.games.push(game);
                self.has_manual = self.games.iter().any(|g| matches!(g.source, GameSource::Manual));
            }
            Controls::RemoveGame(id) => {
                if let Some(row) = self.manual_rows.remove(&id) {
                    self.manual_list_box.remove(&row);
                } else if let Some(row) = self.auto_rows.remove(&id) {
                    self.auto_list_box.remove(&row);
                }
                self.games.retain(|g| g.id != id);
                self.has_manual = self.games.iter().any(|g| matches!(g.source, GameSource::Manual));
            }
            Controls::SetGameStatus { id, installed } => {
                let subtitle = if installed {
                    fl!("reshade-installed")
                } else {
                    fl!("not-installed")
                };
                if let Some(row) =
                    self.auto_rows.get(&id).or_else(|| self.manual_rows.get(&id))
                {
                    row.set_subtitle(&subtitle);
                }
            }
        }
    }
}

/// Builds an [`adw::ActionRow`] card for a single game.
///
/// Manual games get a trash button; auto-detected games get only the chevron.
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
        btn.set_tooltip_text(Some(&fl!("remove-game")));
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
