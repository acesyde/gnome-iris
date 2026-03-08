//! Shader catalog tab — lists known community shader repositories for download.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use crate::fl;
use crate::reshade::catalog::{CatalogEntry, KNOWN_REPOS};
use crate::reshade::config::ShaderRepo;

/// Initialisation data for [`ShaderCatalog`].
pub struct ShaderCatalogInit {
    /// Base data directory (e.g. `~/.local/share/iris/`).
    pub data_dir: PathBuf,
    /// Custom repos from config (those not in KNOWN_REPOS) to pre-populate.
    pub custom_repos: Vec<ShaderRepo>,
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
    /// The "Custom Repositories" group — stored for dynamic row insertion/removal.
    custom_group: adw::PreferencesGroup,
    /// Custom repo rows keyed by `local_name` — needed to remove them from the group.
    custom_rows: HashMap<String, adw::ActionRow>,
}

/// Input messages for [`ShaderCatalog`].
#[derive(Debug)]
pub enum Controls {
    /// User clicked the download button for a known catalog entry.
    DownloadRepo(&'static CatalogEntry),
    /// Window confirmed a new custom repo — add a row to the custom group.
    AddCustomRepo(ShaderRepo),
    /// User clicked download on a custom repo row.
    DownloadCustomRepo(ShaderRepo),
    /// User clicked the trash button on a custom repo row.
    RemoveCustomRepoRequested(ShaderRepo),
    /// Window confirmed removal — remove the row and clean up internal state.
    RemoveCustomRepo(ShaderRepo),
    /// Progress message forwarded from the shader worker.
    SyncProgress(String),
    /// The worker finished syncing the in-flight repo successfully.
    SyncComplete,
    /// The worker failed to sync the in-flight repo.
    SyncError(String),
    /// The "+" button was clicked — window should open the add-repo dialog.
    AddCustomRepoRequested,
}

/// Output signals from [`ShaderCatalog`].
#[derive(Debug)]
pub enum Signal {
    /// User wants to download this repo — parent should forward to the worker.
    DownloadRequested(ShaderRepo),
    /// User clicked the "+" button — window should open the add-repo dialog.
    AddCustomRepoRequested,
    /// User clicked the trash button — window should remove from config and disk.
    RemoveCustomRepoRequested(ShaderRepo),
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
                    set_title: "Shaders",
                    set_description: Some(
                        "Download shader packs to the global cache. \
                         Use the game detail pane to enable them per game."
                    ),
                },

                #[name(custom_group)]
                adw::PreferencesGroup {
                    set_title: "Custom",
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

        let mut installed: HashSet<String> = KNOWN_REPOS
            .iter()
            .filter(|e| repos_dir.join(e.local_name).is_dir())
            .map(|e| e.local_name.to_owned())
            .collect();
        for repo in &init.custom_repos {
            if repos_dir.join(&repo.local_name).is_dir() {
                installed.insert(repo.local_name.clone());
            }
        }

        let mut model = Self {
            data_dir: init.data_dir,
            installed,
            syncing: None,
            row_buttons: HashMap::new(),
            custom_group: adw::PreferencesGroup::new(),
            custom_rows: HashMap::new(),
        };

        let widgets = view_output!();

        model.custom_group = widgets.custom_group.clone();

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

        // Attach the "+" button to the custom group header.
        let add_btn = gtk::Button::from_icon_name("list-add-symbolic");
        add_btn.set_valign(gtk::Align::Center);
        add_btn.add_css_class("flat");
        add_btn.set_tooltip_text(Some(&fl!("add-custom-repo")));
        {
            let s = sender.clone();
            add_btn.connect_clicked(move |_| s.input(Controls::AddCustomRepoRequested));
        }
        widgets.custom_group.set_header_suffix(Some(&add_btn));

        // Pre-populate custom repos.
        for repo in &init.custom_repos {
            let row = build_custom_row(repo, &mut model.row_buttons, &sender);
            widgets.custom_group.add(&row);
            model.custom_rows.insert(repo.local_name.clone(), row);
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
            Controls::AddCustomRepoRequested => {
                sender.output(Signal::AddCustomRepoRequested).ok();
            }
            Controls::AddCustomRepo(repo) => {
                let row = build_custom_row(&repo, &mut self.row_buttons, &sender);
                self.custom_group.add(&row);
                self.custom_rows.insert(repo.local_name.clone(), row);
            }
            Controls::RemoveCustomRepoRequested(repo) => {
                sender.output(Signal::RemoveCustomRepoRequested(repo)).ok();
            }
            Controls::RemoveCustomRepo(repo) => {
                self.installed.remove(&repo.local_name);
                self.row_buttons.remove(&repo.local_name);
                if let Some(row) = self.custom_rows.remove(&repo.local_name) {
                    self.custom_group.remove(&row);
                }
            }
            Controls::DownloadCustomRepo(repo) => {
                if self.syncing.is_some() {
                    return;
                }
                self.syncing = Some(repo.local_name.clone());
                if let Some(btn) = self.row_buttons.get(&repo.local_name) {
                    btn.set_sensitive(false);
                }
                sender.output(Signal::DownloadRequested(repo)).ok();
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

/// Build an [`adw::ActionRow`] for a custom repo with download and remove buttons.
fn build_custom_row(
    repo: &ShaderRepo,
    row_buttons: &mut HashMap<String, gtk::Button>,
    sender: &ComponentSender<ShaderCatalog>,
) -> adw::ActionRow {
    let row = adw::ActionRow::new();
    row.set_title(&repo.local_name);
    row.set_subtitle(&repo.url);

    // Download button.
    let dl_btn = gtk::Button::from_icon_name("folder-download-symbolic");
    dl_btn.set_valign(gtk::Align::Center);
    dl_btn.add_css_class("flat");
    dl_btn.set_tooltip_text(Some("Download"));
    {
        let s = sender.clone();
        let repo_clone = repo.clone();
        dl_btn.connect_clicked(move |_| {
            s.input(Controls::DownloadCustomRepo(repo_clone.clone()))
        });
    }
    row.add_suffix(&dl_btn);
    row_buttons.insert(repo.local_name.clone(), dl_btn);

    // Remove button.
    let rm_btn = gtk::Button::from_icon_name("user-trash-symbolic");
    rm_btn.set_valign(gtk::Align::Center);
    rm_btn.add_css_class("flat");
    rm_btn.add_css_class("destructive-action");
    rm_btn.set_tooltip_text(Some("Remove"));
    {
        let s = sender.clone();
        let repo_clone = repo.clone();
        rm_btn.connect_clicked(move |_| {
            s.input(Controls::RemoveCustomRepoRequested(repo_clone.clone()))
        });
    }
    row.add_suffix(&rm_btn);

    row
}
