# Design: Version Picker on Game Install

**Date:** 2026-03-13
**Status:** Approved

## Overview

When a user clicks "Install ReShade" on the game detail page, a dialog appears prompting them to select which locally-cached ReShade version to install. If no versions are cached, the Install button is disabled and an info banner explains that a version must be installed via Preferences first.

## Architecture

Approach A: all install-related UX lives inside `GameDetail`, which owns the new `PickReshadeVersionDialog` component. `Window` stores its own `installed_versions: Vec<String>`, passes it down to `GameDetail` whenever a game is selected or the list changes, and forwards the updated `version: String` field through its existing message chain to the install worker.

## Version Key Format

`Window.installed_versions` is populated from `list_installed_versions()`, which reads directory names verbatim from `~/.local/share/iris/reshade/`. Those directories are created by `do_download_version` using `dir_key = version.to_owned()` or `format!("{version}-Addon")`, where `version` is the GitHub tag name passed through from `handle_version_download_requested`. GitHub tag names carry a `v`-prefix (e.g. `"v6.3.0"`), and `handle_version_download_requested` does **not** strip this prefix before handing the string to the worker, so disk directories are named `"v6.3.0"`, `"v6.3.0-Addon"`, etc.

**All version keys throughout this feature carry the `v`-prefix. No stripping or transformation is performed at any boundary.**

Note: `parse_version_key()` in `reshade.rs` treats the first dot-separated component as a u64; for a v-prefixed key like `"v6.3.0"` the first component `"v6"` fails to parse and becomes 0, causing incorrect sort order. This is a pre-existing bug unrelated to this feature and is left for a separate fix.

## New Files

- `src/ui/pick_reshade_version_dialog.rs` (new)

## Changed Files

- `src/ui/mod.rs`
- `src/ui/game_detail.rs`
- `src/ui/install_worker.rs`
- `src/ui/window/mod.rs`
- `src/ui/window/panel_games.rs`
- `src/ui/window/panel_preferences.rs`
- `i18n/en-US/gnome_iris.ftl`
- `i18n/es-ES/gnome_iris.ftl`
- `i18n/fr-FR/gnome_iris.ftl`
- `i18n/it-IT/gnome_iris.ftl`
- `i18n/pt-BR/gnome_iris.ftl`

---

## New Component: `src/ui/pick_reshade_version_dialog.rs`

A `SimpleComponent` wrapping an `adw::Dialog`, following the same pattern as `install_version_dialog.rs`.

**Model fields:**
```rust
versions: Vec<String>,       // populated on Open
selected: Option<String>,    // currently checked version key (v-prefixed)
dialog: adw::Dialog,         // stored ref for close()
list_box: gtk::ListBox,      // stored ref for clearing rows on re-open
```

**Controls:**
```rust
pub enum Controls {
    /// Open the dialog: clear + repopulate rows, reset selection, present.
    /// `parent` must be a widget currently in the window tree.
    Open { versions: Vec<String>, parent: gtk::Widget },
    /// A row's radio button was toggled active.
    SelectVersion(String),
    /// Install button clicked.
    Confirm,
    /// Cancel button clicked or dialog dismissed via Escape/close.
    Cancel,
}
```

**Signal:**
```rust
pub enum Signal {
    /// Emitted on Confirm with the chosen version key (e.g. `"v6.3.0"`).
    VersionChosen(String),
    // Cancel emits no signal — dialog simply closes.
}
```

**Layout (view! macro):**
```
adw::Dialog { set_title: fl!("pick-version-dialog-title"), set_content_width: 380 }
  adw::ToolbarView
    adw::HeaderBar
    gtk::Box { orientation: Vertical, margin_all: 24, spacing: 12 }
      adw::PreferencesGroup
        #[name(list_box)]
        gtk::ListBox { selection_mode: None, add_css_class: "boxed-list" }
      gtk::Box { orientation: Horizontal, halign: End, spacing: 8 }
        gtk::Button { label: fl!("pick-version-cancel-btn"), add_css_class: "flat" }
        #[name(install_btn)]
        gtk::Button {
            label: fl!("pick-version-install-btn"),
            add_css_class: "suggested-action",
            #[watch] set_sensitive: model.selected.is_some(),
        }
```

**`init()`:** Store `widgets.dialog`, `widgets.list_box` on the model. Connect Cancel button and Install button to `Controls::Cancel` and `Controls::Confirm` respectively.

**`Controls::Open { versions, parent }` handler:**

```rust
// 1. Clear existing rows.
while let Some(child) = self.list_box.first_child() {
    self.list_box.remove(&child);
}
// 2. Reset selection.
self.selected = None;
self.versions = versions.clone();
// 3. Guard: empty list means Install button in GameDetail was disabled; skip present.
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
    // Emit SelectVersion only when this button becomes active (not when deactivated).
    {
        let s = sender.clone();
        let k = key.clone();
        check.connect_toggled(move |btn| {
            if btn.is_active() {
                s.input(Controls::SelectVersion(k.clone()));
            }
        });
    }
    // Format display: "v6.3.0-Addon" → "6.3.0 — Addon Support"; "v6.3.0" → "v6.3.0".
    // Uses the same logic as `display_title` in `preferences.rs`. Either duplicate the
    // function here (private, ~4 lines) or move it to a shared `ui::util` module.
    let title = version_display_title(key);
    let row = adw::ActionRow::new();
    row.set_title(&title);
    row.add_suffix(&check);
    self.list_box.append(&row);
}
// 5. Present.
self.dialog.present(Some(&parent));
```

**`Controls::SelectVersion(v)`:** `self.selected = Some(v);`

**`Controls::Cancel`:**
```rust
self.selected = None;
self.dialog.close();
```

**`Controls::Confirm`:**
```rust
if let Some(version) = self.selected.take() {
    sender.output(Signal::VersionChosen(version)).ok();
}
self.dialog.close();
```

**Helper (private to this module):**
```rust
/// Format a version key for display: strips the leading `v`, converts
/// `"-Addon"` suffix to `" — Addon Support"`.
fn version_display_title(key: &str) -> String {
    let base = key.strip_prefix('v').unwrap_or(key);
    if let Some(ver) = base.strip_suffix("-Addon") {
        format!("{ver} — {}", fl!("addon-support"))
    } else {
        base.to_owned()
    }
}
```
(The `addon-support` i18n key already exists.)

---

## Changes to `src/ui/mod.rs`

Add:
```rust
pub mod pick_reshade_version_dialog;
```

---

## Changes to `src/ui/game_detail.rs`

### Model additions
```rust
installed_versions: Vec<String>,
pick_version_dialog: Controller<pick_reshade_version_dialog::PickReshadeVersionDialog>,
```

Initialised in `init()`:
```rust
let pick_version_dialog =
    pick_reshade_version_dialog::PickReshadeVersionDialog::builder()
        .launch(())
        .forward(sender.input_sender(), |sig| match sig {
            pick_reshade_version_dialog::Signal::VersionChosen(v) => Controls::VersionChosen(v),
        });
// In the model struct literal:
installed_versions: Vec::new(),
pick_version_dialog,
```

### New `Controls` variants
```rust
/// Refresh the list of locally-cached ReShade versions.
SetInstalledVersions(Vec<String>),
/// Internal: version picker dialog confirmed a choice.
VersionChosen(String),
```

### Install button — updated `set_sensitive`
```rust
set_sensitive: !model.installed_versions.is_empty()
    && model.game.as_ref().map(|g| !g.status.is_installed()).unwrap_or(true),
```

### Info banner — new widget, inserted above the action-buttons box
```rust
adw::Banner {
    #[watch]
    set_title: &fl!("no-versions-banner"),
    #[watch]
    set_revealed: model.game.is_some() && model.installed_versions.is_empty(),
}
```
Non-dismissable, no button. Note: the existing `no-versions-subtitle` key is used in Preferences; `no-versions-banner` is a distinct key for this different context.

### `Controls::InstallRequested` handler — updated

`GameDetail` already stores `shader_list: gtk::ListBox` on the model. That widget is part of the window tree once the detail page is pushed. `WidgetExt::root()` returns `Option<gtk::Root>`; upcast to `gtk::Widget` for the message:

```rust
Controls::InstallRequested => {
    use relm4::gtk::prelude::*;
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

Inside the dialog's `Controls::Open` handler, `parent: gtk::Widget` is passed to `self.dialog.present(Some(&parent))`. `adw::Dialog::present` takes `Option<&impl IsA<gtk::Widget>>`, so this is type-correct.

### `Controls::VersionChosen(version)` handler
```rust
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
            .output(Signal::Install { game_id: game.id.clone(), dll, arch, version })
            .ok();
    }
}
```

### `Controls::SetInstalledVersions(versions)` handler
```rust
Controls::SetInstalledVersions(versions) => {
    self.installed_versions = versions;
}
```

### `Signal::Install` — updated definition
```rust
Install {
    game_id: String,
    dll: DllOverride,
    arch: ExeArch,
    version: String,   // ← new: chosen version key, e.g. "v6.3.0"
}
```

### `Controls::MarkInstalled` — no change needed

`MarkInstalled { dll, arch, .. }` already ignores its `version` field (used only for display in the subtitle via `reshade_version` which comes from `SetShaderData`). No change required.

---

## Changes to `src/ui/window/mod.rs`

### `Window` struct — add field
Add to the `pub struct Window { ... }` definition:
```rust
/// Locally-cached ReShade version keys (e.g. `"v6.3.0"`), kept in sync with the Preferences panel.
installed_versions: Vec<String>,
```

In `init()`, compute `list_installed_versions` once and use the result for both `PreferencesInit.installed_versions` and the model struct initialiser:
```rust
let installed_versions = list_installed_versions(&app_state.data_dir).unwrap_or_else(...);
// ... PreferencesInit uses installed_versions.clone()
// ... model struct literal: installed_versions,
```

### `Controls::Install` — updated definition
```rust
Install {
    game_id: String,
    dll: DllOverride,
    arch: ExeArch,
    version: String,   // ← new
}
```

### `update()` match arm — updated
```rust
Controls::Install { game_id, dll, arch, version } => {
    panel_games::handle_install(self, game_id, dll, arch, version);
}
```

### Signal forwarding closure in `init()` — updated
```rust
game_detail::Signal::Install { game_id, dll, arch, version } => {
    Controls::Install { game_id, dll, arch, version }
}
```

---

## Changes to `src/ui/window/panel_games.rs`

### `handle_game_selected` — emit installed versions to detail pane
After the existing `SetShaderData` emit:
```rust
model.game_detail.emit(
    game_detail::Controls::SetInstalledVersions(model.installed_versions.clone())
);
```

### `handle_install` — updated signature and body
```rust
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

`pending_install: Option<(DllOverride, ExeArch)>` does not need to store `version` — `handle_install_complete` takes its `version` from `Signal::InstallComplete { version }`, which is emitted by `do_install` using the version it was called with.

---

## Changes to `src/ui/window/panel_preferences.rs`

### `handle_version_download_complete` — sync `Window.installed_versions`

The `version` parameter here is the `version_key` string (e.g. `"v6.3.0"`) forwarded from `Signal::DownloadVersionComplete { version_key }`. Push it to `installed_versions` and, if a game is currently open in the detail pane, refresh its version list:

```rust
pub(super) fn handle_version_download_complete(model: &mut Window, version: String) {
    model.installed_versions.push(version.clone());
    if model.current_game_id.is_some() {
        model.game_detail.emit(
            game_detail::Controls::SetInstalledVersions(model.installed_versions.clone())
        );
    }
    model.preferences.emit(preferences::Controls::VersionDownloadComplete(version));
}
```

### `handle_version_remove_requested` — sync `Window.installed_versions` on success

The existing function returns early on `remove_dir_all` failure (before emitting `VersionRemoveComplete`), so the new lines are only reached on the success path. Add after the `VersionRemoveComplete` emit:

```rust
model.installed_versions.retain(|v| v != &version);
if model.current_game_id.is_some() {
    model.game_detail.emit(
        game_detail::Controls::SetInstalledVersions(model.installed_versions.clone())
    );
}
```

---

## Changes to `src/ui/install_worker.rs`

### `Controls::Install` — updated definition
```rust
Install {
    data_dir: PathBuf,
    game_dir: PathBuf,
    dll: DllOverride,
    arch: ExeArch,
    version: String,   // ← new; no longer fetched from GitHub
}
```

### `Worker::update()` match arm — updated
```rust
Controls::Install { data_dir, game_dir, dll, arch, version } => {
    let sender2 = sender.clone();
    relm4::spawn(async move {
        if let Err(e) = do_install(&data_dir, &game_dir, dll, arch, &version, &sender2).await {
            sender2.output(Signal::Error(e.to_string())).ok();
        }
    });
}
```

### `do_install` — simplified, no network
```rust
async fn do_install(
    data_dir: &Path,
    game_dir: &Path,
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

    // `cache.add_installed` is intentionally omitted: the version is guaranteed
    // to already be registered in the cache because `do_install` is only reachable
    // after the user selects a version from `PickReshadeVersionDialog`, which
    // is only populated from `installed_versions` — a list derived from
    // `list_installed_versions()` (disk) which only contains versions previously
    // downloaded via `do_download_version` (which calls `cache.add_installed`).

    sender
        .output(Signal::InstallComplete { version: version.to_owned() })
        .ok();
    Ok(())
}
```

`fetch_latest_version()` and the download block are **removed** from `do_install`. They remain only in `do_download_version` (the Preferences cache-download path).

---

## i18n

New keys in all five locale files (`i18n/en-US/gnome_iris.ftl`, `i18n/es-ES/gnome_iris.ftl`, `i18n/fr-FR/gnome_iris.ftl`, `i18n/it-IT/gnome_iris.ftl`, `i18n/pt-BR/gnome_iris.ftl`):

```fluent
no-versions-banner = Install a ReShade version in Preferences before installing to a game.
pick-version-dialog-title = Choose ReShade Version
pick-version-install-btn = Install
pick-version-cancel-btn = Cancel
```

Note: `addon-support` is an existing key reused by `version_display_title`.

---

## Error Handling

If `do_install` is dispatched with a version whose directory is missing or DLL absent, it returns an `anyhow::Error`. The existing `Signal::Error` → `Controls::WorkerError` → `handle_worker_error` path surfaces this in the detail pane progress banner — no new error paths needed.

---

## Testing

- Domain layer: no new pure-Rust logic; existing `install_reshade` and `detect_install_status` tests cover the symlink path.
- `do_install` no longer performs network I/O, making it easier to unit test in future.
- Manual smoke test: cache a version via Preferences → open a game → confirm Install button is enabled → click Install → pick the version in the dialog → confirm → verify the correct DLL is symlinked and the subtitle updates. Also verify: with no cached versions, Install button is insensitive and the banner is visible.
