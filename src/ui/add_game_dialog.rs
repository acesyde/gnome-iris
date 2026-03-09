//! Dialog for manually adding a game by name and path.

use std::path::PathBuf;

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, adw, gtk};

use crate::reshade::game::ExeArch;

/// Add game dialog model.
pub struct AddGameDialog {
    name: String,
    path: Option<PathBuf>,
    arch: ExeArch,
    /// Paths already in the library — used for duplicate detection.
    existing_paths: Vec<PathBuf>,
    /// Whether the currently selected path is already in the library.
    duplicate: bool,
    /// Widget refs for programmatic control.
    name_entry: adw::EntryRow,
    path_row: adw::ActionRow,
    dialog: adw::Dialog,
    arch_x64_btn: gtk::CheckButton,
    arch_x86_btn: gtk::CheckButton,
}

/// Input messages for [`AddGameDialog`].
#[derive(Debug)]
pub enum Controls {
    /// Present the dialog.
    Open,
    /// Name field changed.
    SetName(String),
    /// File or folder selected via the browse button.
    ///
    /// If the path is a file, the directory is inferred from its parent and arch
    /// is auto-detected from the PE header (defaults to x64 on failure).
    /// If the path is a directory, it is used as-is with no arch change.
    FileSelected(PathBuf),
    /// Architecture selected via radio button.
    SetArch(ExeArch),
    /// Refresh the list of already-added game paths for duplicate detection.
    UpdateExistingPaths(Vec<PathBuf>),
    /// User clicked the confirm button.
    Confirm,
}

/// Output signals from [`AddGameDialog`].
#[derive(Debug)]
pub enum Signal {
    /// User confirmed the new game entry.
    GameAdded {
        /// Display name entered by the user.
        name: String,
        /// Directory path selected by the user.
        path: PathBuf,
        /// Architecture detected or chosen by the user.
        arch: ExeArch,
    },
}

#[allow(missing_docs)]
#[relm4::component(pub)]
impl SimpleComponent for AddGameDialog {
    type Init = Vec<PathBuf>;
    type Input = Controls;
    type Output = Signal;

    view! {
        #[name(dialog)]
        adw::Dialog {
            set_title: "Add Game",
            set_content_width: 420,

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    pack_start = &gtk::Button {
                        set_label: "Cancel",
                        connect_clicked[dialog] => move |_| {
                            dialog.close();
                        },
                    },

                    pack_end = &gtk::Button {
                        set_label: "Add",
                        add_css_class: "suggested-action",
                        #[watch]
                        set_sensitive: !model.name.trim().is_empty()
                            && model.path.is_some()
                            && !model.duplicate,
                        connect_clicked[sender] => move |_| {
                            sender.input(Controls::Confirm);
                        },
                    },
                },

                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 24,
                    set_spacing: 24,

                    adw::PreferencesGroup {
                        #[name(name_entry)]
                        adw::EntryRow {
                            set_title: "Name",
                        },
                    },

                    gtk::Label {
                        set_label: "This game is already in your library.",
                        add_css_class: "error",
                        set_xalign: 0.0,
                        set_wrap: true,
                        #[watch]
                        set_visible: model.duplicate,
                    },

                    adw::PreferencesGroup {
                        #[name(path_row)]
                        adw::ActionRow {
                            set_title: "Directory",

                            add_suffix = &gtk::Button {
                                set_icon_name: "folder-open-symbolic",
                                set_valign: gtk::Align::Center,
                                add_css_class: "flat",
                                set_tooltip_text: Some("Browse — select a folder or a .exe for auto-detection"),
                                connect_clicked[sender] => move |btn| {
                                    let sender = sender.clone();
                                    let fd = gtk::FileDialog::new();
                                    fd.set_title("Select game folder or executable");
                                    let filter = gtk::FileFilter::new();
                                    filter.set_name(Some("Windows executables (*.exe)"));
                                    filter.add_pattern("*.exe");
                                    filter.add_pattern("*.EXE");
                                    fd.set_default_filter(Some(&filter));
                                    let parent = btn.root().and_downcast::<gtk::Window>();
                                    gtk::glib::MainContext::default().spawn_local(async move {
                                        if let Ok(file) = fd.open_future(parent.as_ref()).await {
                                            if let Some(path) = file.path() {
                                                sender.input(Controls::FileSelected(path));
                                            }
                                        }
                                    });
                                },
                            },
                        },
                    },

                    #[name(arch_group)]
                    adw::PreferencesGroup {
                        set_title: "Architecture",
                    },
                },
            },
        }
    }

    fn init(
        existing_paths: Vec<PathBuf>,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = Self {
            name: String::new(),
            path: None,
            arch: ExeArch::X86_64,
            existing_paths,
            duplicate: false,
            name_entry: adw::EntryRow::new(),
            path_row: adw::ActionRow::new(),
            dialog: adw::Dialog::new(),
            arch_x64_btn: gtk::CheckButton::new(),
            arch_x86_btn: gtk::CheckButton::new(),
        };

        let widgets = view_output!();

        model.name_entry = widgets.name_entry.clone();
        model.path_row = widgets.path_row.clone();
        model.dialog = widgets.dialog.clone();

        widgets.name_entry.connect_changed({
            let s = sender.clone();
            move |e| s.input(Controls::SetName(e.text().to_string()))
        });

        // Build arch radio buttons.
        let arch_x64_btn = gtk::CheckButton::new();
        arch_x64_btn.set_active(true);
        let arch_x86_btn = gtk::CheckButton::new();
        arch_x86_btn.set_group(Some(&arch_x64_btn));

        let x64_row = adw::ActionRow::new();
        x64_row.set_title("64-bit (x86-64)");
        x64_row.set_activatable_widget(Some(&arch_x64_btn));
        x64_row.add_prefix(&arch_x64_btn);

        let x86_row = adw::ActionRow::new();
        x86_row.set_title("32-bit (x86)");
        x86_row.set_activatable_widget(Some(&arch_x86_btn));
        x86_row.add_prefix(&arch_x86_btn);

        widgets.arch_group.add(&x64_row);
        widgets.arch_group.add(&x86_row);

        arch_x64_btn.connect_toggled({
            let s = sender.clone();
            move |btn| {
                if btn.is_active() {
                    s.input(Controls::SetArch(ExeArch::X86_64));
                }
            }
        });
        arch_x86_btn.connect_toggled({
            let s = sender.clone();
            move |btn| {
                if btn.is_active() {
                    s.input(Controls::SetArch(ExeArch::X86));
                }
            }
        });

        model.arch_x64_btn = arch_x64_btn;
        model.arch_x86_btn = arch_x86_btn;

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, sender: ComponentSender<Self>) {
        match msg {
            Controls::Open => {
                self.name = String::new();
                self.path = None;
                self.arch = ExeArch::X86_64;
                self.duplicate = false;
                self.name_entry.set_text("");
                self.path_row.set_subtitle("");
                self.path_row.remove_css_class("error");
                self.arch_x64_btn.set_active(true);
            }
            Controls::SetName(v) => self.name = v,
            Controls::SetArch(arch) => self.arch = arch,
            Controls::UpdateExistingPaths(paths) => {
                self.existing_paths = paths;
                // Re-evaluate duplicate status for the currently selected path.
                self.refresh_duplicate();
            }
            Controls::FileSelected(path) => {
                let dir = if path.is_file() {
                    // Detect arch from PE header; default to x64 on failure.
                    let detected = crate::reshade::steam::detect_exe_arch(&path)
                        .unwrap_or(ExeArch::X86_64);
                    self.arch = detected;
                    match detected {
                        ExeArch::X86_64 => self.arch_x64_btn.set_active(true),
                        ExeArch::X86 => self.arch_x86_btn.set_active(true),
                    }
                    path.parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or(path)
                } else {
                    path
                };

                self.path = Some(dir.clone());
                self.refresh_duplicate();

                let subtitle = if self.duplicate {
                    format!("{} — already in library", dir.display())
                } else {
                    dir.to_string_lossy().into_owned()
                };
                self.path_row.set_subtitle(&subtitle);
                if self.duplicate {
                    self.path_row.add_css_class("error");
                } else {
                    self.path_row.remove_css_class("error");
                }
            }
            Controls::Confirm => {
                let name = self.name.trim().to_owned();
                let Some(path) = self.path.clone() else { return };
                if name.is_empty() || self.duplicate {
                    return;
                }
                sender
                    .output(Signal::GameAdded { name, path, arch: self.arch })
                    .ok();
                // Reset fields.
                self.name = String::new();
                self.path = None;
                self.arch = ExeArch::X86_64;
                self.duplicate = false;
                self.name_entry.set_text("");
                self.path_row.set_subtitle("");
                self.path_row.remove_css_class("error");
                self.arch_x64_btn.set_active(true);
                self.dialog.close();
            }
        }
    }
}

impl AddGameDialog {
    /// Recomputes `self.duplicate` from `self.path` and `self.existing_paths`.
    fn refresh_duplicate(&mut self) {
        self.duplicate = self
            .path
            .as_ref()
            .map(|p| self.existing_paths.contains(p))
            .unwrap_or(false);
    }
}
