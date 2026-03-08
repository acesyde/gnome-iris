//! Shader catalog tab — lists known community shader repositories for download.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use crate::reshade::catalog::{CatalogEntry, KNOWN_REPOS};
use crate::reshade::config::ShaderRepo;

/// Initialisation data for [`ShaderCatalog`].
pub struct ShaderCatalogInit {
    /// Base data directory (e.g. `~/.local/share/iris/`).
    pub data_dir: PathBuf,
}

/// Shader catalog component model.
pub struct ShaderCatalog {
    /// Base data directory.
    data_dir: PathBuf,
    /// `local_name`s of repos already present on disk.
    installed: HashSet<String>,
    /// `local_name` of the repo currently being downloaded, if any.
    syncing: Option<String>,
    /// Download buttons keyed by `local_name` (only for not-yet-installed repos).
    row_buttons: HashMap<String, gtk::Button>,
}

/// Input messages for [`ShaderCatalog`].
#[derive(Debug)]
pub enum Controls {
    /// User clicked the download button for an entry.
    DownloadRepo(&'static CatalogEntry),
    /// Progress message forwarded from the shader worker.
    SyncProgress(String),
    /// The worker finished syncing the in-flight repo successfully.
    SyncComplete,
    /// The worker failed to sync the in-flight repo.
    SyncError(String),
}

/// Output signals from [`ShaderCatalog`].
#[derive(Debug)]
pub enum Signal {
    /// User wants to download this repo — parent should forward to the worker.
    DownloadRequested(ShaderRepo),
}

#[allow(missing_docs)]
#[relm4::component(pub)]
impl SimpleComponent for ShaderCatalog {
    type Init = ShaderCatalogInit;
    type Input = Controls;
    type Output = Signal;

    view! {
        gtk::ScrolledWindow {
            set_vexpand: true,
            set_hscrollbar_policy: gtk::PolicyType::Never,

            adw::PreferencesPage {
                set_title: "Shaders",

                #[name(catalog_group)]
                adw::PreferencesGroup {
                    set_title: "Known Repositories",
                    set_description: Some(
                        "Download shader packs to the global cache. \
                         Use the game detail pane to enable them per game."
                    ),
                },
            },
        }
    }

    fn init(
        init: ShaderCatalogInit,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let repos_dir = init.data_dir.join("ReShade_shaders");
        let installed: HashSet<String> = KNOWN_REPOS
            .iter()
            .filter(|e| repos_dir.join(e.local_name).is_dir())
            .map(|e| e.local_name.to_owned())
            .collect();

        let mut model = Self {
            data_dir: init.data_dir,
            installed,
            syncing: None,
            row_buttons: HashMap::new(),
        };

        let widgets = view_output!();

        // Build one ActionRow per catalog entry.
        for entry in KNOWN_REPOS {
            let row = adw::ActionRow::new();
            row.set_title(entry.name);
            row.set_subtitle(entry.description);

            if model.installed.contains(entry.local_name) {
                let btn = gtk::Button::from_icon_name("emblem-default-symbolic");
                btn.set_valign(gtk::Align::Center);
                btn.add_css_class("flat");
                btn.set_sensitive(false);
                btn.set_tooltip_text(Some("Downloaded"));
                row.add_suffix(&btn);
            } else {
                let btn = gtk::Button::from_icon_name("folder-download-symbolic");
                btn.set_valign(gtk::Align::Center);
                btn.add_css_class("flat");
                btn.set_tooltip_text(Some("Download"));
                {
                    let s = sender.clone();
                    btn.connect_clicked(move |_| s.input(Controls::DownloadRepo(entry)));
                }
                row.add_suffix(&btn);
                model.row_buttons.insert(entry.local_name.to_owned(), btn);
            }

            widgets.catalog_group.add(&row);
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, sender: ComponentSender<Self>) {
        match msg {
            Controls::DownloadRepo(entry) => {
                if self.syncing.is_some() {
                    return; // one download at a time
                }
                self.syncing = Some(entry.local_name.to_owned());
                if let Some(btn) = self.row_buttons.get(entry.local_name) {
                    btn.set_sensitive(false);
                }
                sender
                    .output(Signal::DownloadRequested(entry.to_shader_repo()))
                    .ok();
            }
            Controls::SyncProgress(msg) => {
                log::debug!("Shader sync: {msg}");
            }
            Controls::SyncComplete => {
                if let Some(local_name) = self.syncing.take() {
                    self.installed.insert(local_name.clone());
                    if let Some(btn) = self.row_buttons.get(&local_name) {
                        btn.set_icon_name("emblem-default-symbolic");
                        btn.set_tooltip_text(Some("Downloaded"));
                    }
                }
            }
            Controls::SyncError(e) => {
                if let Some(local_name) = self.syncing.take() {
                    log::error!("Failed to sync {local_name}: {e}");
                    if let Some(btn) = self.row_buttons.get(&local_name) {
                        btn.set_sensitive(true); // re-enable so user can retry
                    }
                }
            }
        }
    }
}
