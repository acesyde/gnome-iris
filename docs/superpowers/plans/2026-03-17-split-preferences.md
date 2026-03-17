# Split preferences.rs Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split `src/ui/preferences.rs` (678 lines) into `src/ui/preferences/mod.rs` (thin config host) and `src/ui/preferences/panel_versions.rs` (full version management), each ≤ 250 lines.

**Architecture:** `panel_versions::Versions` becomes a standalone `SimpleComponent` whose root widget is `adw::PreferencesPage`. `Preferences` in `mod.rs` creates it as a `Controller<Versions>` child in `init()`, appends its widget to the content box, and forwards version-related `Controls` down / `Signal`s up. The public `preferences::Controls` and `preferences::Signal` types stay the same so `panel_preferences.rs` needs no changes.

**Tech Stack:** Rust, relm4 `SimpleComponent`, GTK4/libadwaita, existing codebase patterns.

---

### Task 1: Create `panel_versions.rs`

**Files:**
- Create: `src/ui/preferences/panel_versions.rs`

The new file takes all version-related state, helpers, and logic out of `preferences.rs`.

- [ ] **Step 1: Create `src/ui/preferences/` directory by creating the new file**

The file implements:
- `pub struct VersionsInit` — `data_dir`, `installed_versions`, `current_version`, `versions_in_use`
- `pub struct Versions` — all version-related model fields from current `Preferences`
- `pub enum Controls` — all version-related variants (see below)
- `pub enum Signal` — `InstallVersionRequested(String)`, `RemoveVersionRequested(String)`
- `impl SimpleComponent for Versions` with root `adw::PreferencesPage`
- All helper fns: `version_sort_key`, `display_title`, `subtitle_for_installed`, `build_uninstalled_row`, `build_installed_row`, `begin_version_op`, `finish_version_op`, `begin_install_op`, `finish_install_op`

```rust
//! Version management panel — lists installed ReShade versions and handles
//! install/remove operations.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use relm4::adw::prelude::*;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, SimpleComponent,
    adw, gtk,
};

use crate::fl;
use crate::ui::install_version_dialog;

/// Initialization payload for [`Versions`].
pub struct VersionsInit {
    /// App data directory (e.g. `~/.local/share/iris/`).
    pub data_dir: PathBuf,
    /// All locally installed ReShade version directories.
    pub installed_versions: Vec<String>,
    /// The currently active version (from `LVERS`), if any.
    pub current_version: Option<String>,
    /// Versions currently symlinked by at least one game — cannot be removed.
    pub versions_in_use: HashSet<String>,
}

/// Version management panel model.
pub struct Versions {
    data_dir: PathBuf,
    installed_versions: Vec<String>,
    current_version: Option<String>,
    versions_in_use: HashSet<String>,
    version_rows: HashMap<String, adw::ActionRow>,
    version_buttons: HashMap<String, gtk::Button>,
    version_spinners: HashMap<String, gtk::Spinner>,
    latest_version: Option<String>,
    latest_uninstalled_row: Option<adw::ActionRow>,
    install_button: Option<gtk::Button>,
    install_spinner: Option<gtk::Spinner>,
    latest_addon_uninstalled_row: Option<adw::ActionRow>,
    install_addon_button: Option<gtk::Button>,
    install_addon_spinner: Option<gtk::Spinner>,
    versions_group: adw::PreferencesGroup,
    placeholder_row: Option<adw::ActionRow>,
    active_ops: HashSet<String>,
    install_version_dialog: Controller<install_version_dialog::InstallVersionDialog>,
}

/// Input messages for [`Versions`].
#[derive(Debug)]
pub enum Controls {
    /// The latest available ReShade version was fetched; adds install rows for
    /// standard and Addon Support variants not already in the local cache.
    SetLatestVersion(String),
    /// Download `version_key` to the local cache; shows spinner and emits
    /// [`Signal::InstallVersionRequested`]. Ignored if that key is in flight.
    InstallLatestVersion(String),
    /// Open the manual version install dialog.
    OpenInstallVersionDialog,
    /// Remove `version_key` from the local cache; shows spinner and emits
    /// [`Signal::RemoveVersionRequested`]. Ignored if that key is in flight.
    RemoveVersion(String),
    /// The download of `version_key` completed — adds an installed row and
    /// removes the corresponding "not installed" row.
    VersionDownloadComplete(String),
    /// The removal of `version` completed — removes the installed row and
    /// restores the "latest not installed" row if needed.
    VersionRemoveComplete(String),
    /// An install or remove operation failed; resets all in-flight spinners.
    VersionOpError(String),
}

/// Output signals from [`Versions`].
#[derive(Debug)]
pub enum Signal {
    /// The user requested that `version_key` be downloaded to the local cache.
    ///
    /// The window handler should forward this to the install worker.
    InstallVersionRequested(String),
    /// The user requested that a cached `version_key` be deleted from disk.
    ///
    /// The window handler should delete the directory.
    RemoveVersionRequested(String),
}

#[allow(missing_docs)]
#[relm4::component(pub)]
impl SimpleComponent for Versions {
    type Init = VersionsInit;
    type Input = Controls;
    type Output = Signal;

    view! {
        adw::PreferencesPage {
            set_title: "ReShade",
            set_icon_name: Some("application-x-executable-symbolic"),

            #[name(versions_group)]
            adw::PreferencesGroup {
                set_title: &fl!("installed-versions"),
                set_description: Some(&fl!("installed-versions-description")),
            },
        }
    }

    #[allow(clippy::too_many_lines)]
    fn init(init: VersionsInit, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let install_version_dialog =
            install_version_dialog::InstallVersionDialog::builder()
                .launch(())
                .forward(sender.input_sender(), |sig| match sig {
                    install_version_dialog::Signal::InstallRequested(key) => {
                        Controls::InstallLatestVersion(key)
                    },
                });

        let model = Self {
            data_dir: init.data_dir,
            installed_versions: init.installed_versions,
            current_version: init.current_version,
            versions_in_use: init.versions_in_use,
            version_rows: HashMap::new(),
            version_buttons: HashMap::new(),
            version_spinners: HashMap::new(),
            latest_version: None,
            latest_uninstalled_row: None,
            install_button: None,
            install_spinner: None,
            latest_addon_uninstalled_row: None,
            install_addon_button: None,
            install_addon_spinner: None,
            versions_group: adw::PreferencesGroup::new(),
            placeholder_row: None,
            active_ops: HashSet::new(),
            install_version_dialog,
        };
        let widgets = view_output!();

        // Attach action buttons to the versions group header.
        {
            let reshade_dir = model.data_dir.join("reshade");
            let open_btn = gtk::Button::from_icon_name("folder-open-symbolic");
            open_btn.set_valign(gtk::Align::Center);
            open_btn.add_css_class("flat");
            open_btn.set_tooltip_text(Some(&fl!("open-reshade-folder")));
            open_btn.connect_clicked(move |_| {
                let _ = std::fs::create_dir_all(&reshade_dir);
                std::process::Command::new("xdg-open")
                    .arg(reshade_dir.as_os_str())
                    .spawn()
                    .ok();
            });

            let add_version_btn = gtk::Button::from_icon_name("list-add-symbolic");
            add_version_btn.set_valign(gtk::Align::Center);
            add_version_btn.add_css_class("flat");
            add_version_btn.set_tooltip_text(Some(&fl!("install-version-button-tooltip")));
            {
                let s = sender.clone();
                add_version_btn.connect_clicked(move |_| {
                    s.input(Controls::OpenInstallVersionDialog);
                });
            }

            let header_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
            header_box.append(&add_version_btn);
            header_box.append(&open_btn);
            widgets.versions_group.set_header_suffix(Some(&header_box));
        }

        // Populate installed version rows imperatively.
        let (version_rows, version_buttons, version_spinners, placeholder_row) =
            if model.installed_versions.is_empty() {
                let row = adw::ActionRow::new();
                row.set_title(&fl!("no-versions-installed"));
                row.set_subtitle(&fl!("no-versions-subtitle"));
                widgets.versions_group.add(&row);
                (HashMap::new(), HashMap::new(), HashMap::new(), Some(row))
            } else {
                let mut rows = HashMap::new();
                let mut buttons = HashMap::new();
                let mut spinners = HashMap::new();
                let mut sorted = model.installed_versions.clone();
                sorted.sort_by(|a, b| {
                    let (ma, mia, pa, aa) = version_sort_key(a);
                    let (mb, mib, pb, ab) = version_sort_key(b);
                    (mb, mib, pb).cmp(&(ma, mia, pa)).then(aa.cmp(&ab))
                });
                for version in &sorted {
                    let sub =
                        subtitle_for_installed(version, model.current_version.as_deref(), false);
                    let is_in_use = model.versions_in_use.contains(version);
                    let (row, btn, spinner) =
                        build_installed_row(version, &sub, is_in_use, &sender);
                    widgets.versions_group.add(&row);
                    rows.insert(version.clone(), row);
                    buttons.insert(version.clone(), btn);
                    spinners.insert(version.clone(), spinner);
                }
                (rows, buttons, spinners, None)
            };

        let model = Self {
            version_rows,
            version_buttons,
            version_spinners,
            placeholder_row,
            versions_group: widgets.versions_group.clone(),
            latest_uninstalled_row: None,
            install_button: None,
            install_spinner: None,
            latest_addon_uninstalled_row: None,
            install_addon_button: None,
            install_addon_spinner: None,
            active_ops: HashSet::new(),
            ..model
        };

        ComponentParts { model, widgets }
    }

    #[allow(clippy::too_many_lines)]
    fn update(&mut self, msg: Controls, sender: ComponentSender<Self>) {
        // (exact copy of version-related arms from preferences.rs::update)
        match msg {
            Controls::SetLatestVersion(version) => { /* ... copy from preferences.rs ... */ },
            Controls::OpenInstallVersionDialog => { /* ... */ },
            Controls::InstallLatestVersion(version_key) => { /* ... */ },
            Controls::RemoveVersion(version) => { /* ... */ },
            Controls::VersionDownloadComplete(version_key) => { /* ... */ },
            Controls::VersionRemoveComplete(version) => { /* ... */ },
            Controls::VersionOpError(e) => { /* ... */ },
        }
    }
}

// --- helper functions (copied verbatim from preferences.rs) ---
fn version_sort_key(key: &str) -> (u64, u64, u64, bool) { /* ... */ }
fn display_title(version_key: &str) -> String { /* ... */ }
fn build_uninstalled_row(version_key: &str, sender: &ComponentSender<Versions>) -> (adw::ActionRow, gtk::Button, gtk::Spinner) { /* ... */ }
fn subtitle_for_installed(version: &str, current: Option<&str>, is_latest: bool) -> String { /* ... */ }
fn build_installed_row(version: &str, subtitle: &str, is_in_use: bool, sender: &ComponentSender<Versions>) -> (adw::ActionRow, gtk::Button, gtk::Spinner) { /* ... */ }
fn begin_version_op(version: &str, buttons: &HashMap<String, gtk::Button>, spinners: &HashMap<String, gtk::Spinner>) { /* ... */ }
fn finish_version_op(version: &str, buttons: &HashMap<String, gtk::Button>, spinners: &HashMap<String, gtk::Spinner>) { /* ... */ }
fn begin_install_op(button: Option<&gtk::Button>, spinner: Option<&gtk::Spinner>) { /* ... */ }
fn finish_install_op(button: Option<&gtk::Button>, spinner: Option<&gtk::Spinner>) { /* ... */ }
```

- [ ] **Step 2: Verify it compiles**

```bash
mise exec -- cargo check 2>&1 | head -40
```

Expected: errors only about `panel_versions` not being declared yet (no type errors within the new file itself).

---

### Task 2: Rewrite `preferences/mod.rs` as thin host

**Files:**
- Modify: `src/ui/preferences.rs` → becomes `src/ui/preferences/mod.rs`

The file keeps:
- `PreferencesInit` (add nothing for versions — those go to `VersionsInit`)
- `Preferences` model: just `config: GlobalConfig` + `versions: Controller<panel_versions::Versions>`
- `Controls` enum with ALL variants (both config and version-related, for backward compat)
- `Signal` enum unchanged
- `impl SimpleComponent for Preferences`: thin `view!` with just the config pages; appends `versions.widget()` in `init()`; `update()` forwards version Controls to child, handles config Controls locally

Key wiring in `init()`:
```rust
let versions = panel_versions::Versions::builder()
    .launch(panel_versions::VersionsInit { ... })
    .forward(sender.input_sender(), |sig| match sig {
        panel_versions::Signal::InstallVersionRequested(k) => Controls::InstallLatestVersion(k),
        panel_versions::Signal::RemoveVersionRequested(v) => Controls::RemoveVersion(v),
    });
widgets.content_box.append(versions.widget());
```

Wait — that would create a loop. The signal forwarding should map panel_versions::Signal to the parent's output Signal (not to Controls). Let me reconsider.

Actually, `panel_versions::Signal::InstallVersionRequested` should be forwarded to `preferences::Signal::InstallVersionRequested`. The `.forward()` maps the child's Signal to the parent's Input (Controls). Then in `update()`, the parent re-emits it as output Signal.

OR, use `connect_receiver()` / output directly. The cleaner approach: in `update()`:
```rust
Controls::InstallLatestVersion(k) => {
    // re-emitted from child signal via forward
    sender.output(Signal::InstallVersionRequested(k)).ok();
},
Controls::RemoveVersion(v) => {
    sender.output(Signal::RemoveVersionRequested(v)).ok();
},
```

And the forward closure:
```rust
.forward(sender.input_sender(), |sig| match sig {
    panel_versions::Signal::InstallVersionRequested(k) => Controls::InstallLatestVersion(k),
    panel_versions::Signal::RemoveVersionRequested(v) => Controls::RemoveVersion(v),
})
```

This way the parent's `update()` sees these as Controls and can re-emit them as Signal.

- [ ] **Step 3: Convert `preferences.rs` to `preferences/mod.rs`**

The final mod.rs should look like:

```rust
//! Global preferences page — config panel (shaders, update interval)
//! orchestrating the [`panel_versions`] child component.
#![allow(clippy::cast_precision_loss)]

pub mod panel_versions;

use relm4::adw::prelude::*;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, SimpleComponent,
    adw, gtk,
};

use crate::fl;
use crate::reshade::config::GlobalConfig;
use crate::ui::preferences::panel_versions::Versions;

pub use panel_versions::Controls as VersionControls; // NOT needed, just forward all variants

/// (same PreferencesInit as before — data_dir, config, installed_versions, etc.)

/// Preferences page model.
pub struct Preferences {
    config: GlobalConfig,
    versions: Controller<Versions>,
}

/// Input messages — unchanged from old preferences.rs for backward compat.
#[derive(Debug)]
pub enum Controls {
    SetConfig(GlobalConfig),
    MergeShadersChanged(bool),
    UpdateIntervalChanged(f64),
    // Version controls — forwarded to child:
    SetLatestVersion(String),
    InstallLatestVersion(String),
    OpenInstallVersionDialog,
    RemoveVersion(String),
    VersionDownloadComplete(String),
    VersionRemoveComplete(String),
    VersionOpError(String),
}

/// Output signals — unchanged.
#[derive(Debug)]
pub enum Signal {
    ConfigChanged(GlobalConfig),
    InstallVersionRequested(String),
    RemoveVersionRequested(String),
}

#[allow(missing_docs)]
#[relm4::component(pub)]
impl SimpleComponent for Preferences {
    type Init = PreferencesInit;
    type Input = Controls;
    type Output = Signal;

    view! {
        gtk::ScrolledWindow {
            set_vexpand: true,
            set_hscrollbar_policy: gtk::PolicyType::Never,

            #[name(content_box)]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 0,

                adw::PreferencesPage {
                    set_title: &fl!("shaders-section"),
                    set_icon_name: Some("preferences-system-symbolic"),

                    adw::PreferencesGroup {
                        set_title: &fl!("shader-repositories"),
                        set_description: Some(&fl!("shader-repos-description")),

                        #[name(merge_row)]
                        adw::SwitchRow {
                            set_title: &fl!("merge-shaders"),
                            set_subtitle: &fl!("merge-shaders-subtitle"),
                            #[watch]
                            set_active: model.config.merge_shaders,
                        },
                    },
                },

                adw::PreferencesPage {
                    set_title: &fl!("updates"),
                    set_icon_name: Some("software-update-available-symbolic"),

                    adw::PreferencesGroup {
                        set_title: &fl!("update-check"),

                        #[name(spin_row)]
                        adw::SpinRow {
                            set_title: &fl!("update-interval"),
                            set_subtitle: &fl!("update-interval-subtitle"),
                            set_adjustment: Some(&gtk::Adjustment::new(4.0, 1.0, 168.0, 1.0, 0.0, 0.0)),
                            set_digits: 0,
                            set_snap_to_ticks: true,
                            #[watch]
                            set_value: model.config.update_interval_hours as f64,
                        },
                    },
                },
            },
        }
    }

    fn init(init: PreferencesInit, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let versions = panel_versions::Versions::builder()
            .launch(panel_versions::VersionsInit {
                data_dir: init.data_dir,
                installed_versions: init.installed_versions,
                current_version: init.current_version,
                versions_in_use: init.versions_in_use,
            })
            .forward(sender.input_sender(), |sig| match sig {
                panel_versions::Signal::InstallVersionRequested(k) => Controls::InstallLatestVersion(k),
                panel_versions::Signal::RemoveVersionRequested(v) => Controls::RemoveVersion(v),
            });

        let model = Self { config: init.config, versions };
        let widgets = view_output!();

        widgets.merge_row.connect_active_notify({
            let s = sender.clone();
            move |row| s.input(Controls::MergeShadersChanged(row.is_active()))
        });
        widgets.spin_row.connect_value_notify({
            let s = sender.clone();
            move |row: &adw::SpinRow| s.input(Controls::UpdateIntervalChanged(row.value()))
        });

        widgets.content_box.append(model.versions.widget());

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, sender: ComponentSender<Self>) {
        match msg {
            Controls::SetConfig(config) => { self.config = config; },
            Controls::MergeShadersChanged(val) => {
                if self.config.merge_shaders != val {
                    self.config.merge_shaders = val;
                    sender.output(Signal::ConfigChanged(self.config.clone())).ok();
                }
            },
            Controls::UpdateIntervalChanged(val) => {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let hours = val as u64;
                if self.config.update_interval_hours != hours {
                    self.config.update_interval_hours = hours;
                    sender.output(Signal::ConfigChanged(self.config.clone())).ok();
                }
            },
            // Version controls — forwarded to child:
            Controls::SetLatestVersion(v) => {
                self.versions.emit(panel_versions::Controls::SetLatestVersion(v));
            },
            Controls::InstallLatestVersion(k) => {
                // Could be re-routed from child signal OR from window panel.
                // Distinguish: if it comes from child signal it was already handled there.
                // But since forward maps child Signal → this Controls variant,
                // we re-emit as output Signal here.
                sender.output(Signal::InstallVersionRequested(k)).ok();
            },
            Controls::OpenInstallVersionDialog => {
                self.versions.emit(panel_versions::Controls::OpenInstallVersionDialog);
            },
            Controls::RemoveVersion(v) => {
                sender.output(Signal::RemoveVersionRequested(v)).ok();
            },
            Controls::VersionDownloadComplete(v) => {
                self.versions.emit(panel_versions::Controls::VersionDownloadComplete(v));
            },
            Controls::VersionRemoveComplete(v) => {
                self.versions.emit(panel_versions::Controls::VersionRemoveComplete(v));
            },
            Controls::VersionOpError(e) => {
                self.versions.emit(panel_versions::Controls::VersionOpError(e));
            },
        }
    }
}
```

⚠️ **Issue with the forwarding design above**: When `panel_versions` emits `Signal::InstallVersionRequested(k)`, it gets forwarded to `Preferences::Controls::InstallLatestVersion(k)`. But `InstallLatestVersion` is also sent directly from `panel_preferences.rs` calling `preferences.emit(Controls::InstallLatestVersion(...))`.

This creates ambiguity: we'd double-emit. The solution: rename the signal path.

**Revised approach** — map child Signals directly to parent output Signals by using a bridge:

```rust
// In panel_versions forward closure, map to a dedicated parent Controls variant:
.forward(sender.input_sender(), |sig| match sig {
    panel_versions::Signal::InstallVersionRequested(k) => Controls::VersionInstallRequested(k),
    panel_versions::Signal::RemoveVersionRequested(v) => Controls::VersionRemoveRequested(v),
})
```

And add two private Controls variants `VersionInstallRequested` / `VersionRemoveRequested` that just emit the output Signal. Keep `InstallLatestVersion` for the inbound window-panel path which forwards to the child directly.

Actually, the cleaner fix: the window panel (`panel_preferences.rs`) calls `preferences.emit(Controls::InstallLatestVersion(...))` which is triggered by `preferences::Signal::InstallVersionRequested`. So the flow is:

```
user clicks download button in panel_versions
→ panel_versions emits Signal::InstallVersionRequested(k)
→ forwarded as Preferences::Controls::VersionInstallRequested(k)   ← new variant
→ Preferences::update() emits Signal::InstallVersionRequested(k)
→ panel_preferences.rs receives it, calls install worker
→ install worker completes
→ panel_preferences.rs calls preferences.emit(Controls::VersionDownloadComplete(k))
→ Preferences::update() forwards Controls::VersionDownloadComplete(k) to child
```

And separately:
```
window panel calls preferences.emit(Controls::SetLatestVersion(v))
→ Preferences::update() forwards to child panel_versions.emit(Controls::SetLatestVersion(v))
```

So `Controls::InstallLatestVersion` is no longer needed in `Preferences::Controls` — it lives only in `panel_versions::Controls`. The parent's Controls becomes cleaner:

```rust
pub enum Controls {
    // Config:
    SetConfig(GlobalConfig),
    MergeShadersChanged(bool),
    UpdateIntervalChanged(f64),
    // Forwarded to child:
    SetLatestVersion(String),
    OpenInstallVersionDialog,
    VersionDownloadComplete(String),
    VersionRemoveComplete(String),
    VersionOpError(String),
    // Bridge from child Signal to parent Signal:
    VersionInstallRequested(String),   // ← replaces old InstallLatestVersion at parent level
    VersionRemoveRequested(String),    // ← replaces old RemoveVersion at parent level
}
```

But this breaks `panel_preferences.rs` which calls `preferences::Controls::InstallLatestVersion`. We need to update that file too.

**Simplest backward-compatible approach**: keep `InstallLatestVersion` and `RemoveVersion` in `Preferences::Controls` but change their semantics — they no longer go to the child (the child handles them internally via its own Controls), they just re-emit as output Signals. The child's signal forward maps to new private variants.

Let me just go with adding two new variants for the bridge and updating `panel_preferences.rs` accordingly.

- [ ] **Step 4: Update `panel_preferences.rs` if any Controls names changed**

Check for references to `preferences::Controls::InstallLatestVersion` and `preferences::Controls::RemoveVersion` in `panel_preferences.rs` and update if needed.

- [ ] **Step 5: Verify compilation**

```bash
mise exec -- cargo check 2>&1
```

Expected: zero errors.

---

### Task 3: Run tests and clippy

- [ ] **Step 6: Run tests**

```bash
mise exec -- cargo test 2>&1
```

Expected: all tests pass.

- [ ] **Step 7: Run clippy**

```bash
mise exec -- cargo clippy 2>&1
```

Expected: zero warnings/errors.

- [ ] **Step 8: Check line counts**

```bash
wc -l src/ui/preferences/mod.rs src/ui/preferences/panel_versions.rs
```

Expected: each file ≤ 250 lines.

---

### Task 4: Mark plan item done and commit

- [ ] **Step 9: Mark item 2 done in the plan file**

In `docs/plans/2026-03-16-maintainability-improvements.md`, change:
```
### 2. Split `preferences.rs` into focused sub-components
```
to:
```
### 2. ✅ Split `preferences.rs` into focused sub-components
```

And update the priority table row from `| 2 | Split preferences.rs |` to `| 2 | ✅ Split preferences.rs |`.

- [ ] **Step 10: Commit**

```bash
git add src/ui/preferences/ src/ui/preferences.rs \
        docs/plans/2026-03-16-maintainability-improvements.md
git commit -m "refactor(ui): split preferences.rs into mod + panel_versions"
```
