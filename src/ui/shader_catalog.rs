//! Shader catalog tab — lists known community shader repositories for download.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use crate::fl;
use crate::reshade::catalog::{CatalogEntry, KNOWN_REPOS};
use crate::reshade::config::ShaderRepo;

/// Initialisation data for [`ShaderCatalog`].
#[allow(clippy::module_name_repetitions)]
pub struct ShaderCatalogInit {
    /// Base data directory (e.g. `~/.local/share/iris/`).
    pub data_dir: PathBuf,
    /// Custom repos from config (those not in `KNOWN_REPOS`) to pre-populate.
    pub custom_repos: Vec<ShaderRepo>,
}

/// Shader catalog component model.
pub struct ShaderCatalog {
    /// Base data directory.
    #[allow(dead_code)]
    data_dir: PathBuf,
    /// `local_name`s of repos already present on disk.
    installed: HashSet<String>,
    /// `local_name` of the repo currently being downloaded, if any.
    syncing: Option<String>,
    /// Download buttons keyed by `local_name` (only for not-yet-installed repos).
    row_buttons: HashMap<String, gtk::Button>,
    /// Spinner widgets keyed by `local_name` (known + custom repos).
    row_spinners: HashMap<String, gtk::Spinner>,
    /// The "Custom Repositories" group — stored for dynamic row insertion/removal.
    custom_group: adw::PreferencesGroup,
    /// Custom repo rows keyed by `local_name` — needed to remove them from the group.
    custom_rows: HashMap<String, adw::ActionRow>,
    /// Pending repos queued for sequential download.
    download_queue: VecDeque<ShaderRepo>,
}

/// Input messages for [`ShaderCatalog`].
#[derive(Debug)]
pub enum Controls {
    /// User clicked the download button for a known catalog entry.
    DownloadRepo(&'static CatalogEntry),
    /// User clicked "Download All" — queue all not-yet-installed known repos.
    DownloadAll,
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
                set_title: &fl!("shaders-section"),

                #[name(catalog_group)]
                adw::PreferencesGroup {
                    set_title: &fl!("shaders-section"),
                    set_description: Some(&fl!("shaders-description")),
                },

                #[name(custom_group)]
                adw::PreferencesGroup {
                    set_title: &fl!("custom-repos"),
                },
            },
        }
    }

    fn init(init: ShaderCatalogInit, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
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
            row_spinners: HashMap::new(),
            custom_group: adw::PreferencesGroup::new(),
            custom_rows: HashMap::new(),
            download_queue: VecDeque::new(),
        };

        let widgets = view_output!();

        model.custom_group = widgets.custom_group.clone();

        // Build one ActionRow per catalog entry.
        for entry in KNOWN_REPOS {
            let row = adw::ActionRow::new();
            row.set_title(entry.name);
            row.set_subtitle(entry.description);

            let (icon, tip) = if model.installed.contains(entry.local_name) {
                ("view-refresh-symbolic", fl!("redownload"))
            } else {
                ("folder-download-symbolic", fl!("download"))
            };
            let btn = gtk::Button::from_icon_name(icon);
            btn.set_valign(gtk::Align::Center);
            btn.add_css_class("flat");
            btn.set_tooltip_text(Some(&tip));
            {
                let s = sender.clone();
                btn.connect_clicked(move |_| s.input(Controls::DownloadRepo(entry)));
            }
            let spinner = gtk::Spinner::new();
            spinner.set_valign(gtk::Align::Center);

            let sync_stack = gtk::Stack::new();
            sync_stack.set_valign(gtk::Align::Center);
            sync_stack.add_named(&btn, Some("button"));
            sync_stack.add_named(&spinner, Some("spinner"));
            row.add_suffix(&sync_stack);

            model.row_buttons.insert(entry.local_name.to_owned(), btn);
            model.row_spinners.insert(entry.local_name.to_owned(), spinner);

            widgets.catalog_group.add(&row);
        }

        // Attach the "Download All" + "Open Folder" buttons to the catalog group header.
        let open_btn = gtk::Button::from_icon_name("folder-open-symbolic");
        open_btn.set_valign(gtk::Align::Center);
        open_btn.add_css_class("flat");
        open_btn.set_tooltip_text(Some(&fl!("open-shaders-folder")));
        {
            open_btn.connect_clicked(move |_| {
                let _ = std::fs::create_dir_all(&repos_dir);
                std::process::Command::new("xdg-open").arg(repos_dir.as_os_str()).spawn().ok();
            });
        }

        let download_all_btn = gtk::Button::from_icon_name("folder-download-symbolic");
        download_all_btn.set_valign(gtk::Align::Center);
        download_all_btn.add_css_class("flat");
        download_all_btn.set_tooltip_text(Some(&fl!("download-all")));
        {
            let s = sender.clone();
            download_all_btn.connect_clicked(move |_| s.input(Controls::DownloadAll));
        }

        let header_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        header_box.append(&download_all_btn);
        header_box.append(&open_btn);
        widgets.catalog_group.set_header_suffix(Some(&header_box));

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
            let is_installed = model.installed.contains(&repo.local_name);
            let (row, spinner) = build_custom_row(repo, is_installed, &mut model.row_buttons, &sender);
            model.row_spinners.insert(repo.local_name.clone(), spinner);
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
                begin_sync(entry.local_name, &self.row_buttons, &self.row_spinners);
                sender.output(Signal::DownloadRequested(entry.to_shader_repo())).ok();
            },
            Controls::DownloadAll => {
                if self.syncing.is_some() {
                    return;
                }
                self.download_queue.clear();
                for entry in KNOWN_REPOS {
                    if !self.installed.contains(entry.local_name) {
                        self.download_queue.push_back(entry.to_shader_repo());
                    }
                }
                if let Some(repo) = self.download_queue.pop_front() {
                    let name = repo.local_name.clone();
                    self.syncing = Some(name.clone());
                    begin_sync(&name, &self.row_buttons, &self.row_spinners);
                    sender.output(Signal::DownloadRequested(repo)).ok();
                }
            },
            Controls::AddCustomRepoRequested => {
                sender.output(Signal::AddCustomRepoRequested).ok();
            },
            Controls::AddCustomRepo(repo) => {
                let is_installed = self.installed.contains(&repo.local_name);
                let (row, spinner) = build_custom_row(&repo, is_installed, &mut self.row_buttons, &sender);
                self.row_spinners.insert(repo.local_name.clone(), spinner);
                self.custom_group.add(&row);
                self.custom_rows.insert(repo.local_name.clone(), row);
            },
            Controls::RemoveCustomRepoRequested(repo) => {
                sender.output(Signal::RemoveCustomRepoRequested(repo)).ok();
            },
            Controls::RemoveCustomRepo(repo) => {
                self.installed.remove(&repo.local_name);
                self.row_buttons.remove(&repo.local_name);
                self.row_spinners.remove(&repo.local_name);
                if let Some(row) = self.custom_rows.remove(&repo.local_name) {
                    self.custom_group.remove(&row);
                }
            },
            Controls::DownloadCustomRepo(repo) => {
                if self.syncing.is_some() {
                    return;
                }
                self.syncing = Some(repo.local_name.clone());
                begin_sync(&repo.local_name, &self.row_buttons, &self.row_spinners);
                sender.output(Signal::DownloadRequested(repo)).ok();
            },
            Controls::SyncProgress(msg) => {
                log::debug!("Shader sync: {msg}");
            },
            Controls::SyncComplete => {
                if let Some(local_name) = self.syncing.take() {
                    self.installed.insert(local_name.clone());
                    finish_sync(&local_name, true, &self.row_buttons, &self.row_spinners);
                    if let Some(next) = self.download_queue.pop_front() {
                        let name = next.local_name.clone();
                        self.syncing = Some(name.clone());
                        begin_sync(&name, &self.row_buttons, &self.row_spinners);
                        sender.output(Signal::DownloadRequested(next)).ok();
                    }
                }
            },
            Controls::SyncError(e) => {
                if let Some(local_name) = self.syncing.take() {
                    log::error!("Failed to sync {local_name}: {e}");
                    finish_sync(&local_name, false, &self.row_buttons, &self.row_spinners);
                    if let Some(next) = self.download_queue.pop_front() {
                        let name = next.local_name.clone();
                        self.syncing = Some(name.clone());
                        begin_sync(&name, &self.row_buttons, &self.row_spinners);
                        sender.output(Signal::DownloadRequested(next)).ok();
                    }
                }
            },
        }
    }
}

/// Show spinner in place of button — call before emitting `Signal::DownloadRequested`.
fn begin_sync(
    local_name: &str,
    row_buttons: &HashMap<String, gtk::Button>,
    row_spinners: &HashMap<String, gtk::Spinner>,
) {
    if let Some(sp) = row_spinners.get(local_name) {
        sp.start();
    }
    if let Some(btn) = row_buttons.get(local_name)
        && let Some(stack) = btn.parent().and_then(|p| p.downcast::<gtk::Stack>().ok())
    {
        stack.set_visible_child_name("spinner");
    }
}

/// Restore button in place of spinner — call after the worker responds.
fn finish_sync(
    local_name: &str,
    success: bool,
    row_buttons: &HashMap<String, gtk::Button>,
    row_spinners: &HashMap<String, gtk::Spinner>,
) {
    if let Some(btn) = row_buttons.get(local_name) {
        btn.set_sensitive(true);
        if success {
            btn.set_icon_name("view-refresh-symbolic");
            btn.set_tooltip_text(Some(&fl!("redownload")));
        }
        if let Some(stack) = btn.parent().and_then(|p| p.downcast::<gtk::Stack>().ok()) {
            stack.set_visible_child_name("button");
        }
    }
    if let Some(sp) = row_spinners.get(local_name) {
        sp.stop();
    }
}

/// Build an [`adw::ActionRow`] for a custom repo with download and remove buttons.
///
/// Returns `(row, spinner)` so the caller can register the spinner in `row_spinners`.
fn build_custom_row(
    repo: &ShaderRepo,
    is_installed: bool,
    row_buttons: &mut HashMap<String, gtk::Button>,
    sender: &ComponentSender<ShaderCatalog>,
) -> (adw::ActionRow, gtk::Spinner) {
    let row = adw::ActionRow::new();
    row.set_title(&repo.local_name);
    row.set_subtitle(&repo.url);

    // Download button — shows refresh icon when already installed.
    let (icon, tip) = if is_installed {
        ("view-refresh-symbolic", fl!("redownload"))
    } else {
        ("folder-download-symbolic", fl!("download"))
    };
    let dl_btn = gtk::Button::from_icon_name(icon);
    dl_btn.set_valign(gtk::Align::Center);
    dl_btn.add_css_class("flat");
    dl_btn.set_tooltip_text(Some(&tip));
    {
        let s = sender.clone();
        let repo_clone = repo.clone();
        dl_btn.connect_clicked(move |_| {
            s.input(Controls::DownloadCustomRepo(repo_clone.clone()));
        });
    }
    // Spinner (hidden until syncing) — stacked with the download button.
    let spinner = gtk::Spinner::new();
    spinner.set_valign(gtk::Align::Center);

    let sync_stack = gtk::Stack::new();
    sync_stack.set_valign(gtk::Align::Center);
    sync_stack.add_named(&dl_btn, Some("button"));
    sync_stack.add_named(&spinner, Some("spinner"));
    row.add_suffix(&sync_stack);

    row_buttons.insert(repo.local_name.clone(), dl_btn);

    // Remove button.
    let rm_btn = gtk::Button::from_icon_name("user-trash-symbolic");
    rm_btn.set_valign(gtk::Align::Center);
    rm_btn.add_css_class("flat");
    rm_btn.add_css_class("destructive-action");
    rm_btn.set_tooltip_text(Some(&fl!("remove")));
    {
        let s = sender.clone();
        let repo_clone = repo.clone();
        rm_btn.connect_clicked(move |_| {
            s.input(Controls::RemoveCustomRepoRequested(repo_clone.clone()));
        });
    }
    row.add_suffix(&rm_btn);

    (row, spinner)
}
