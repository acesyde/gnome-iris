# Version Picker on Game Install — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When the user clicks "Install ReShade" on the game detail page, show a dialog to choose which locally-cached version to install; disable the button and show an info banner when no versions are cached.

**Architecture:** `GameDetail` owns a new `PickReshadeVersionDialog` component and forwards `Signal::Install` with a chosen `version: String`. `Window` stores `installed_versions: Vec<String>` and propagates it down to `GameDetail` on game-select and on Preferences add/remove events. `InstallWorker` uses the passed-in version instead of fetching from GitHub.

**Tech Stack:** Rust 1.94 (stable), Relm4 (git pin), GTK4 + libadwaita, i18n-embed / Fluent

**Spec:** `docs/superpowers/specs/2026-03-13-version-picker-on-install-design.md`

---

## Chunk 1: i18n strings + install_worker refactor

### Task 1: Add i18n keys to all locale files

**Files:**
- Modify: `i18n/en-US/gnome_iris.ftl`
- Modify: `i18n/es-ES/gnome_iris.ftl`
- Modify: `i18n/fr-FR/gnome_iris.ftl`
- Modify: `i18n/it-IT/gnome_iris.ftl`
- Modify: `i18n/pt-BR/gnome_iris.ftl`

- [ ] **Step 1: Add keys to the English locale**

Open `i18n/en-US/gnome_iris.ftl` and append below the existing `# Preferences — versions` block (after line 72, `install-version-already-installed`):

```fluent
# Version picker dialog (game install)
no-versions-banner = Install a ReShade version in Preferences before installing to a game.
pick-version-dialog-title = Choose ReShade Version
pick-version-install-btn = Install
pick-version-cancel-btn = Cancel
```

- [ ] **Step 2: Add the same keys to es-ES**

Open `i18n/es-ES/gnome_iris.ftl` and append (use the same English strings for now — will be translated separately):

```fluent
# Version picker dialog (game install)
no-versions-banner = Install a ReShade version in Preferences before installing to a game.
pick-version-dialog-title = Choose ReShade Version
pick-version-install-btn = Install
pick-version-cancel-btn = Cancel
```

- [ ] **Step 3: Add the same keys to fr-FR**

Open `i18n/fr-FR/gnome_iris.ftl` and append:

```fluent
# Version picker dialog (game install)
no-versions-banner = Install a ReShade version in Preferences before installing to a game.
pick-version-dialog-title = Choose ReShade Version
pick-version-install-btn = Install
pick-version-cancel-btn = Cancel
```

- [ ] **Step 4: Add the same keys to it-IT**

Open `i18n/it-IT/gnome_iris.ftl` and append:

```fluent
# Version picker dialog (game install)
no-versions-banner = Install a ReShade version in Preferences before installing to a game.
pick-version-dialog-title = Choose ReShade Version
pick-version-install-btn = Install
pick-version-cancel-btn = Cancel
```

- [ ] **Step 5: Add the same keys to pt-BR**

Open `i18n/pt-BR/gnome_iris.ftl` and append:

```fluent
# Version picker dialog (game install)
no-versions-banner = Install a ReShade version in Preferences before installing to a game.
pick-version-dialog-title = Choose ReShade Version
pick-version-install-btn = Install
pick-version-cancel-btn = Cancel
```

- [ ] **Step 6: Cargo check — no GTK needed**

```bash
mise exec -- cargo check
```

Expected: compiles without errors (the new Fluent keys will only cause compile errors if referenced in Rust, which we haven't done yet).

- [ ] **Step 7: Commit**

```bash
git add i18n/
git commit -m "feat(i18n): add version picker and no-versions banner strings"
```

---

### Task 2: Refactor `install_worker::Controls::Install` — add `version` field

**Files:**
- Modify: `src/ui/install_worker.rs`

The `Controls::Install` variant currently fetches the version itself from GitHub. We add a `version: String` field and simplify `do_install` to use the pre-cached DLL.

- [ ] **Step 1: Update `Controls::Install` variant**

In `src/ui/install_worker.rs`, replace the existing `Controls::Install` variant (lines 15–25):

```rust
/// Install a pre-cached ReShade version into the given game directory.
Install {
    /// App data directory.
    data_dir: PathBuf,
    /// Game directory to install into.
    game_dir: PathBuf,
    /// DLL override to use.
    dll: DllOverride,
    /// Executable architecture.
    arch: ExeArch,
    /// The cached version key to install, e.g. `"v6.3.0"` or `"v6.3.0-Addon"`.
    version: String,
},
```

- [ ] **Step 2: Update `Worker::update()` match arm for `Controls::Install`**

In the `update()` method, replace the `Controls::Install` arm (lines 82–96):

```rust
Controls::Install {
    data_dir,
    game_dir,
    dll,
    arch,
    version,
} => {
    let sender2 = sender.clone();
    relm4::spawn(async move {
        if let Err(e) =
            do_install(&data_dir, &game_dir, dll, arch, &version, &sender2).await
        {
            sender2.output(Signal::Error(e.to_string())).ok();
        }
    });
}
```

- [ ] **Step 3: Rewrite `do_install` — remove network fetch, use pre-cached DLL**

Replace the entire `do_install` function (lines 158–204) with:

```rust
async fn do_install(
    data_dir: &std::path::Path,
    game_dir: &std::path::Path,
    dll: DllOverride,
    arch: ExeArch,
    version: &str,
    sender: &ComponentSender<InstallWorker>,
) -> anyhow::Result<()> {
    let version_dir = reshade::version_dir(data_dir, version);
    if !version_dir.join(arch.reshade_dll()).exists() {
        anyhow::bail!(
            "ReShade {version} is not cached locally — download it in Preferences first"
        );
    }

    if !d3dcompiler::is_installed(data_dir, arch) {
        sender
            .output(Signal::Progress("Installing d3dcompiler_47.dll...".into()))
            .ok();
    }
    d3dcompiler::ensure(data_dir, arch).context("Failed to install d3dcompiler_47.dll")?;

    sender.output(Signal::Progress("Installing...".into())).ok();
    install::install_reshade(data_dir, game_dir, version, dll, arch)?;

    // cache.add_installed intentionally omitted: the version is already
    // registered in the cache from the Preferences download step.

    sender
        .output(Signal::InstallComplete { version: version.to_owned() })
        .ok();
    Ok(())
}
```

Also remove the `use crate::reshade::cache::UpdateCache;` import at line 8 **only if** it is no longer used elsewhere in the file after this change. (It is used in `do_download_version`, so leave it.)

- [ ] **Step 4: Cargo check**

```bash
mise exec -- cargo check
```

Expected: compile errors in `src/ui/window/panel_games.rs` (caller of `Controls::Install` doesn't pass `version` yet). That's expected — we'll fix callers in later tasks. If there are _other_ unexpected errors in `install_worker.rs` itself, fix them now.

- [ ] **Step 5: Commit**

```bash
git add src/ui/install_worker.rs
git commit -m "feat(install-worker): pass pre-cached version to do_install, remove network fetch"
```

---

## Chunk 2: New dialog component

### Task 3: Create `src/ui/pick_reshade_version_dialog.rs`

**Files:**
- Create: `src/ui/pick_reshade_version_dialog.rs`

This is a `SimpleComponent` following the same pattern as `src/ui/install_version_dialog.rs`. It shows a list of cached ReShade versions as radio rows and emits the chosen one.

- [ ] **Step 1: Create the file with module doc, imports, model, enums**

Create `src/ui/pick_reshade_version_dialog.rs` with:

```rust
//! Dialog for choosing which cached ReShade version to install into a game.

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use crate::fl;

/// Model for the version-picker dialog.
pub struct PickReshadeVersionDialog {
    /// Version keys currently shown in the list (populated on `Open`).
    versions: Vec<String>,
    /// The version key currently selected by the user.
    selected: Option<String>,
    /// Stored ref so we can call `close()` from `update()`.
    dialog: adw::Dialog,
    /// Stored ref so we can clear and repopulate rows on re-open.
    list_box: gtk::ListBox,
}

/// Input messages for [`PickReshadeVersionDialog`].
#[derive(Debug)]
pub enum Controls {
    /// Populate the list and present the dialog.
    ///
    /// `parent` must be a widget currently in the window tree (used as the
    /// transient parent for `dialog.present()`).
    Open {
        /// Version keys to show (e.g. `"v6.3.0"`, `"v6.3.0-Addon"`).
        versions: Vec<String>,
        /// A widget in the window tree to use as the dialog parent.
        parent: gtk::Widget,
    },
    /// A radio button was toggled active — stores the version key.
    SelectVersion(String),
    /// Install button clicked.
    Confirm,
    /// Cancel button clicked or dialog dismissed.
    Cancel,
}

/// Output signals from [`PickReshadeVersionDialog`].
#[derive(Debug)]
pub enum Signal {
    /// Emitted on confirm with the chosen version key (e.g. `"v6.3.0"`).
    VersionChosen(String),
}
```

- [ ] **Step 2: Add the private display helper**

Append to the file (still before `impl SimpleComponent`):

```rust
impl PickReshadeVersionDialog {
    /// Format a version key for display.
    ///
    /// Strips the leading `v` and converts `"-Addon"` to `" — Addon Support"`.
    /// Examples: `"v6.3.0"` → `"6.3.0"`, `"v6.3.0-Addon"` → `"6.3.0 — Addon Support"`.
    fn display_title(key: &str) -> String {
        let base = key.strip_prefix('v').unwrap_or(key);
        if let Some(ver) = base.strip_suffix("-Addon") {
            format!("{ver} \u{2014} {}", fl!("addon-support"))
        } else {
            base.to_owned()
        }
    }
}
```

- [ ] **Step 3: Add the `view!` macro block and `impl SimpleComponent`**

Append to the file:

```rust
#[allow(missing_docs)]
#[relm4::component(pub)]
impl SimpleComponent for PickReshadeVersionDialog {
    type Init = ();
    type Input = Controls;
    type Output = Signal;

    view! {
        #[name(dialog)]
        adw::Dialog {
            set_title: &fl!("pick-version-dialog-title"),
            set_content_width: 380,

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {},

                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 24,
                    set_spacing: 12,

                    adw::PreferencesGroup {
                        #[name(list_box)]
                        gtk::ListBox {
                            set_selection_mode: gtk::SelectionMode::None,
                            add_css_class: "boxed-list",
                        },
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_halign: gtk::Align::End,
                        set_spacing: 8,

                        #[name(cancel_btn)]
                        gtk::Button {
                            set_label: &fl!("pick-version-cancel-btn"),
                            add_css_class: "flat",
                        },

                        #[name(install_btn)]
                        gtk::Button {
                            set_label: &fl!("pick-version-install-btn"),
                            add_css_class: "suggested-action",
                            #[watch]
                            set_sensitive: model.selected.is_some(),
                        },
                    },
                },
            },
        }
    }

    fn init(
        (): (),
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = Self {
            versions: Vec::new(),
            selected: None,
            dialog: adw::Dialog::new(),
            list_box: gtk::ListBox::new(),
        };

        let widgets = view_output!();

        model.dialog = widgets.dialog.clone();
        model.list_box = widgets.list_box.clone();

        widgets.cancel_btn.connect_clicked({
            let s = sender.clone();
            move |_| s.input(Controls::Cancel)
        });
        widgets.install_btn.connect_clicked({
            move |_| sender.input(Controls::Confirm)
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, sender: ComponentSender<Self>) {
        match msg {
            Controls::Open { versions, parent } => {
                // 1. Clear existing rows.
                while let Some(child) = self.list_box.first_child() {
                    self.list_box.remove(&child);
                }
                // 2. Reset selection.
                self.selected = None;
                self.versions = versions.clone();
                // 3. Guard: empty list should not happen (Install button is disabled
                //    in GameDetail when no versions exist), but be defensive.
                if versions.is_empty() {
                    return;
                }
                // 4. Build radio rows.
                let mut group_anchor: Option<gtk::CheckButton> = None;
                for key in &versions {
                    let check = gtk::CheckButton::new();
                    if let Some(anchor) = &group_anchor {
                        check.set_group(Some(anchor));
                    } else {
                        group_anchor = Some(check.clone());
                    }
                    {
                        let s = sender.clone();
                        let k = key.clone();
                        check.connect_toggled(move |btn| {
                            if btn.is_active() {
                                s.input(Controls::SelectVersion(k.clone()));
                            }
                        });
                    }
                    let title = Self::display_title(key);
                    let row = adw::ActionRow::new();
                    row.set_title(&title);
                    row.add_suffix(&check);
                    self.list_box.append(&row);
                }
                // 5. Present with the window as the transient parent.
                self.dialog.present(Some(&parent));
            }
            Controls::SelectVersion(v) => {
                self.selected = Some(v);
            }
            Controls::Cancel => {
                self.selected = None;
                self.dialog.close();
            }
            Controls::Confirm => {
                if let Some(version) = self.selected.take() {
                    sender.output(Signal::VersionChosen(version)).ok();
                }
                self.dialog.close();
            }
        }
    }
}
```

- [ ] **Step 4: Register the module in `src/ui/mod.rs`**

Open `src/ui/mod.rs` and add after the `install_version_dialog` line:

```rust
pub mod pick_reshade_version_dialog;
```

The file should now contain (relevant excerpt):
```rust
pub mod install_version_dialog;
pub mod install_worker;
pub mod pick_reshade_version_dialog;
```

- [ ] **Step 5: Cargo check**

```bash
mise exec -- cargo check
```

Expected: the new module compiles cleanly. Still expect errors in callers of `install_worker::Controls::Install` — those are not fixed yet.

- [ ] **Step 6: Commit**

```bash
git add src/ui/pick_reshade_version_dialog.rs src/ui/mod.rs
git commit -m "feat(ui): add PickReshadeVersionDialog component"
```

---

## Chunk 3: GameDetail — version awareness

### Task 4: Update `src/ui/game_detail.rs`

**Files:**
- Modify: `src/ui/game_detail.rs`

Add `installed_versions` to the model, a `PickReshadeVersionDialog` controller, two new `Controls` variants, a warning banner, update the Install button sensitivity, and reroute `InstallRequested` through the dialog.

- [ ] **Step 1: Add imports**

At the top of `src/ui/game_detail.rs`, the existing import block already has `relm4::{..., Controller, ...}` — verify `Controller` is imported. Add the new module import. The use block currently ends at line 8; add:

```rust
use crate::ui::pick_reshade_version_dialog;
```

- [ ] **Step 2: Add fields to the `GameDetail` struct**

The struct is defined at lines 11–17. Add two new fields:

```rust
pub struct GameDetail {
    game: Option<Game>,
    progress_message: Option<String>,
    shader_repos: Vec<ShaderRepo>,
    reshade_version: Option<String>,
    shader_list: gtk::ListBox,
    /// Locally-cached ReShade version keys (e.g. `"v6.3.0"`), set by Window.
    installed_versions: Vec<String>,
    /// Dialog for picking which cached version to install.
    pick_version_dialog: relm4::Controller<pick_reshade_version_dialog::PickReshadeVersionDialog>,
}
```

- [ ] **Step 3: Add two new `Controls` variants**

The `Controls` enum is defined at lines 21–63. Add after `OpenFolderRequested`:

```rust
/// Refresh the list of locally-cached ReShade versions available for install.
SetInstalledVersions(Vec<String>),
/// Internal: version picker dialog confirmed a version choice.
VersionChosen(String),
```

- [ ] **Step 4: Add `version` field to `Signal::Install`**

The `Signal::Install` variant (lines 68–77) currently has `game_id`, `dll`, `arch`. Add:

```rust
/// User requested installation with these parameters.
Install {
    /// Stable game ID.
    game_id: String,
    /// DLL override chosen.
    dll: DllOverride,
    /// Architecture detected/chosen.
    arch: ExeArch,
    /// The cached version key to install, e.g. `"v6.3.0"`.
    version: String,
},
```

- [ ] **Step 5: Add a warning banner to the `view!` macro**

In the `view!` macro (around line 219), the existing layout has a progress banner followed by action buttons. Add a new banner between the progress banner and the action-buttons box:

```rust
// No-versions warning banner
adw::Banner {
    #[watch]
    set_title: &fl!("no-versions-banner"),
    #[watch]
    set_revealed: model.game.is_some() && model.installed_versions.is_empty(),
},
```

- [ ] **Step 6: Update the Install button `set_sensitive`**

The Install button in the `view!` macro (around line 225) does not currently have a `set_sensitive` binding. Add one (keep the existing `set_visible` binding):

```rust
gtk::Button {
    set_label: &fl!("install"),
    add_css_class: "suggested-action",
    add_css_class: "pill",
    #[watch]
    set_sensitive: !model.installed_versions.is_empty()
        && model.game.as_ref().map(|g| !g.status.is_installed()).unwrap_or(true),
    #[watch]
    set_visible: model
        .game
        .as_ref()
        .map(|g| !g.status.is_installed())
        .unwrap_or(true),
    connect_clicked[sender] => move |_| {
        sender.input(Controls::InstallRequested);
    },
},
```

- [ ] **Step 7: Update `init()` — construct `pick_version_dialog` and initialise new fields**

The `init()` function is at lines 294–309. Replace it with:

```rust
fn init(
    _: (),
    _root: Self::Root,
    sender: ComponentSender<Self>,
) -> ComponentParts<Self> {
    let pick_version_dialog =
        pick_reshade_version_dialog::PickReshadeVersionDialog::builder()
            .launch(())
            .forward(sender.input_sender(), |sig| match sig {
                pick_reshade_version_dialog::Signal::VersionChosen(v) => {
                    Controls::VersionChosen(v)
                }
            });

    let mut model = Self {
        game: None,
        progress_message: None,
        shader_repos: Vec::new(),
        reshade_version: None,
        shader_list: gtk::ListBox::new(),
        installed_versions: Vec::new(),
        pick_version_dialog,
    };
    let widgets = view_output!();
    model.shader_list = widgets.shader_list.clone();
    ComponentParts { model, widgets }
}
```

- [ ] **Step 8: Update `Controls::InstallRequested` handler in `update()`**

The current handler (lines 339–350) directly emits `Signal::Install`. Replace it with a dialog open:

```rust
Controls::InstallRequested => {
    use relm4::gtk::prelude::WidgetExt;
    if let Some(root) = self.shader_list.root() {
        self.pick_version_dialog.emit(
            pick_reshade_version_dialog::Controls::Open {
                versions: self.installed_versions.clone(),
                parent: root.upcast::<gtk::Widget>(),
            },
        );
    }
}
```

- [ ] **Step 9: Add handlers for `VersionChosen` and `SetInstalledVersions`**

In the `update()` `match` block, add two new arms (place them after `Controls::OpenFolderRequested`):

```rust
Controls::SetInstalledVersions(versions) => {
    self.installed_versions = versions;
}
Controls::VersionChosen(version) => {
    if let Some(game) = &self.game {
        let (dll, arch) = match &game.status {
            InstallStatus::Installed { dll, arch } => (*dll, *arch),
            InstallStatus::NotInstalled => (
                DllOverride::Dxgi,
                game.preferred_arch.unwrap_or(ExeArch::X86_64),
            ),
        };
        sender
            .output(Signal::Install {
                game_id: game.id.clone(),
                dll,
                arch,
                version,
            })
            .ok();
    }
}
```

- [ ] **Step 10: Cargo check**

```bash
mise exec -- cargo check
```

Expected: errors only in `src/ui/window/mod.rs` and `src/ui/window/panel_games.rs` where `Signal::Install` is consumed (missing `version` field). Fix those in the next chunk. No errors inside `game_detail.rs` itself.

- [ ] **Step 11: Commit**

```bash
git add src/ui/game_detail.rs
git commit -m "feat(game-detail): add version picker dialog, disable install when no versions cached"
```

---

## Chunk 4: Window plumbing — propagate `installed_versions` and `version`

### Task 5: Update `src/ui/window/mod.rs`

**Files:**
- Modify: `src/ui/window/mod.rs`

Add `installed_versions` to the `Window` struct, update `Controls::Install`, update the `update()` dispatch arm, and update the signal forwarding closure.

- [ ] **Step 1: Add `installed_versions` to the `Window` struct**

The struct definition ends at line 58. Add after `current_game_id`:

```rust
/// Locally-cached ReShade version keys; kept in sync with Preferences add/remove.
installed_versions: Vec<String>,
```

- [ ] **Step 2: Update `Controls::Install` to include `version`**

The `Controls::Install` variant is at lines 69–73. Replace with:

```rust
/// GameDetail requested installation with a chosen version.
Install {
    game_id: String,
    dll: crate::reshade::game::DllOverride,
    arch: crate::reshade::game::ExeArch,
    /// The cached version key chosen by the user, e.g. `"v6.3.0"`.
    version: String,
},
```

- [ ] **Step 3a: Clone `installed_versions` for `preferences_init` (edit 1 of 2)**

The `list_installed_versions` call is at lines 192–196. Currently `installed_versions` is moved into `preferences_init`. We need it on the model too, so clone it for `preferences_init`. Replace lines 192–204:

```rust
let installed_versions = list_installed_versions(&app_state.data_dir)
    .unwrap_or_else(|e| {
        log::warn!("Could not list ReShade versions: {e}");
        Vec::new()
    });
let versions_in_use = compute_versions_in_use(&games, &app_state.data_dir);
let preferences_init = preferences::PreferencesInit {
    data_dir: app_state.data_dir.clone(),
    config: app_state.config.clone(),
    installed_versions: installed_versions.clone(),   // ← clone for preferences
    current_version: app_state.reshade_version.clone(),
    versions_in_use,
};
```

- [ ] **Step 3b: Add `installed_versions` to the model struct literal (edit 2 of 2)**

The model struct literal is at lines 375–390. Add the new field (the original `installed_versions` binding is moved here):

```rust
let model = Self {
    app_state,
    games,
    game_list,
    game_detail,
    shader_catalog,
    preferences,
    install_worker,
    shader_worker,
    add_shader_repo_dialog,
    add_game_dialog,
    nav_view: nav_view.clone(),
    toast_overlay: toast_overlay.clone(),
    pending_install: None,
    current_game_id: None,
    installed_versions,   // ← new: moves the original Vec
};
```

- [ ] **Step 4: Update signal forwarding closure for `game_detail`**

The closure at lines 180–190 matches `game_detail::Signal::Install { game_id, dll, arch }`. Update to destructure `version`:

```rust
game_detail::Signal::Install { game_id, dll, arch, version } => {
    Controls::Install { game_id, dll, arch, version }
}
```

- [ ] **Step 5: Update `update()` dispatch arm for `Controls::Install`**

Line 448 currently reads:
```rust
Controls::Install { game_id, dll, arch } => {
    panel_games::handle_install(self, game_id, dll, arch);
}
```

Replace with:
```rust
Controls::Install { game_id, dll, arch, version } => {
    panel_games::handle_install(self, game_id, dll, arch, version);
}
```

- [ ] **Step 6: Cargo check**

```bash
mise exec -- cargo check
```

Expected: errors in `panel_games::handle_install` (signature mismatch) and `panel_preferences` (if it references `installed_versions`). Fix those next.

- [ ] **Step 7: Commit**

```bash
git add src/ui/window/mod.rs
git commit -m "feat(window): add installed_versions field, thread version through Install control"
```

---

### Task 6: Update `src/ui/window/panel_games.rs`

**Files:**
- Modify: `src/ui/window/panel_games.rs`

- [ ] **Step 1: Update `handle_install` signature and body**

The function is at lines 53–69. Replace with:

```rust
/// Dispatch an install job to the worker using the pre-cached version.
pub(super) fn handle_install(
    model: &mut Window,
    game_id: String,
    dll: DllOverride,
    arch: ExeArch,
    version: String,
) {
    if let Some(game) = model.games.iter().find(|g| g.id == game_id) {
        let data_dir = iris_data_dir();
        model.install_worker.emit(install_worker::Controls::Install {
            data_dir,
            game_dir: game.path.clone(),
            dll,
            arch,
            version,
        });
        model.pending_install = Some((dll, arch));
    }
}
```

- [ ] **Step 2: Emit `SetInstalledVersions` in `handle_game_selected`**

The function `handle_game_selected` is at lines 17–50. After the existing `model.game_detail.emit(game_detail::Controls::SetShaderData { ... })` call (around line 42), add:

```rust
model.game_detail.emit(
    game_detail::Controls::SetInstalledVersions(model.installed_versions.clone()),
);
```

- [ ] **Step 3: Cargo check**

```bash
mise exec -- cargo check
```

Expected: compile errors only in `panel_preferences.rs` (missing `installed_versions` sync). All other files should be clean.

- [ ] **Step 4: Commit**

```bash
git add src/ui/window/panel_games.rs
git commit -m "feat(panel-games): pass chosen version to install worker, send versions to detail pane"
```

---

### Task 7: Update `src/ui/window/panel_preferences.rs`

**Files:**
- Modify: `src/ui/window/panel_preferences.rs`

- [ ] **Step 1: Update `handle_version_download_complete` to sync `installed_versions`**

The function is at lines 43–48. Replace with:

```rust
/// Notify Preferences that a version download completed; also sync Window's version list.
pub(super) fn handle_version_download_complete(model: &mut Window, version: String) {
    model.installed_versions.push(version.clone());
    if model.current_game_id.is_some() {
        model
            .game_detail
            .emit(game_detail::Controls::SetInstalledVersions(
                model.installed_versions.clone(),
            ));
    }
    model
        .preferences
        .emit(preferences::Controls::VersionDownloadComplete(version));
}
```

Add the missing import at the top of the file:
```rust
use crate::ui::game_detail;
```

- [ ] **Step 2: Update `handle_version_remove_requested` to sync `installed_versions`**

The function is at lines 62–81. Replace the entire function with the following (the new lines are added after the `VersionRemoveComplete` emit, inside the success path — the early `return` on error means they are unreachable on failure):

```rust
/// Remove a cached ReShade version from disk and notify Preferences.
pub(super) fn handle_version_remove_requested(model: &mut Window, version: String) {
    let data_dir = &model.app_state.data_dir;
    let version_dir = crate::reshade::reshade::version_dir(data_dir, &version);
    if version_dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(&version_dir) {
            log::error!("Failed to remove ReShade version {version}: {e}");
            model
                .preferences
                .emit(preferences::Controls::VersionOpError(e.to_string()));
            return;
        }
    }
    let cache = crate::reshade::cache::UpdateCache::new(data_dir.clone());
    if let Err(e) = cache.remove_installed(&version) {
        log::warn!("Could not update installed versions cache after removal: {e}");
    }
    model
        .preferences
        .emit(preferences::Controls::VersionRemoveComplete(version.clone()));
    // Sync Window's version list and refresh the detail pane if open.
    model.installed_versions.retain(|v| v != &version);
    if model.current_game_id.is_some() {
        model
            .game_detail
            .emit(game_detail::Controls::SetInstalledVersions(
                model.installed_versions.clone(),
            ));
    }
}
```

- [ ] **Step 3: Cargo check — expect clean**

```bash
mise exec -- cargo check
```

Expected: no errors. All callers now pass the correct types.

- [ ] **Step 4: Run tests**

```bash
mise exec -- cargo test
```

Expected: all existing tests pass (domain layer only; no new tests needed for this UI-only feature).

- [ ] **Step 5: Commit**

```bash
git add src/ui/window/panel_preferences.rs
git commit -m "feat(panel-preferences): sync installed_versions on version download and remove"
```

---

## Chunk 5: Final verification

### Task 8: Full build and smoke test checklist

**Files:** (none — verification only)

- [ ] **Step 1: Clean cargo check**

```bash
mise exec -- cargo check
```

Expected: zero warnings, zero errors.

- [ ] **Step 2: Clippy — must be 100% clean**

```bash
mise exec -- cargo clippy
```

Expected: no warnings or errors. If clippy flags anything (e.g. unused imports, unnecessary clones), fix them and re-run until clean.

- [ ] **Step 3: Format**

```bash
mise exec -- cargo fmt
```

Expected: no diff. If files were reformatted, stage and amend the relevant commit or create a cleanup commit.

- [ ] **Step 4: Run domain tests**

```bash
mise exec -- cargo test
```

Expected: all existing tests pass.

- [ ] **Step 5: Build**

```bash
mise exec -- cargo build
```

Expected: successful debug build.

- [ ] **Step 6: Manual smoke test — happy path**

Launch the app:
```bash
GSETTINGS_SCHEMA_DIR=./target/share/glib-2.0/schemas mise exec -- cargo run
```

Verify:
1. Open Preferences → download a version (e.g. `6.3.0`) — confirm it appears in "Installed Versions".
2. Select a game in the My Games tab.
3. Click "Install ReShade" — the `PickReshadeVersionDialog` should appear with the downloaded version listed.
4. Select the version and click Install.
5. Confirm the game subtitle updates to show the installed version.

- [ ] **Step 7: Manual smoke test — no-versions path**

1. Remove all cached versions via Preferences.
2. Select a game.
3. Confirm the Install button is grayed out (insensitive).
4. Confirm the warning banner is visible: *"Install a ReShade version in Preferences before installing to a game."*

- [ ] **Step 8: Final commit (if any fmt/clippy fixes were needed)**

Stage only the files that were reformatted or fixed by clippy, then commit:

```bash
git add src/ui/pick_reshade_version_dialog.rs src/ui/game_detail.rs   # adjust as needed
git commit -m "fix(ui): clippy and fmt cleanup for version picker feature"
```
