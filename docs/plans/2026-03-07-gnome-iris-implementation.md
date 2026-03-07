# gnome-iris Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a GTK4 + Relm4 + Rust GNOME desktop app that installs and manages ReShade for Wine/Proton games on Linux, replacing the `reshade-steam-proton` bash script.

**Architecture:** Clean domain/UI split — `src/reshade/` is pure Rust with zero GTK imports (fully unit tested), `src/ui/` is Relm4 components. Async operations (download, git) run in `AsyncComponent` workers and emit progress signals to the UI.

**Tech Stack:** Rust (nightly-2026-02-01), GTK4, libadwaita, Relm4 (pinned git rev), reqwest, git2, keyvalues-parser, sevenz-rust, zip, serde_json, sha2, tempfile (tests)

**Reference docs (read before starting each domain):**
- `docs/build-system.md` — toolchain pin, Cargo.toml, build.rs templates
- `docs/project-structure.md` — directory layout, main.rs template
- `docs/relm4-components.md` — SimpleComponent, Component, AsyncComponent patterns
- `docs/data-model.md` — Shared<T>, async patterns
- `docs/ui-patterns.md` — GSettings, toasts, dialogs, OverlaySplitView
- `docs/plans/2026-03-07-reshade-linux-design.md` — approved design

---

## Task 1: Project Scaffolding

**Files:**
- Create: `rust-toolchain.toml`
- Create: `rustfmt.toml`
- Create: `Cargo.toml`
- Create: `build.rs`
- Create: `data/org.gnome.Iris.gschema.xml`
- Create: `data/icons/icons.gresource.xml`
- Create: `data/icons/hicolor/scalable/apps/iris.svg`
- Create: `i18n.toml`
- Create: `i18n/en-US/iris.ftl`
- Create: `src/main.rs`
- Create: `src/localization.rs`
- Create: `src/reshade/mod.rs`
- Create: `src/ui/mod.rs`

**Step 1: Create `rust-toolchain.toml`**

```toml
[toolchain]
channel = "nightly-2026-02-01"
components = ["rust-src", "rust-analyzer", "rustfmt"]
profile = "default"
```

**Step 2: Create `rustfmt.toml`**

```toml
edition = "2024"
style_edition = "2024"
format_code_in_doc_comments = true
normalize_doc_attributes = true
reorder_impl_items = true
wrap_comments = true
chain_width = 90
comment_width = 120
max_width = 120
use_small_heuristics = "Max"
struct_lit_width = 18
struct_variant_width = 35
group_imports = "StdExternalCrate"
imports_granularity = "Module"
match_block_trailing_comma = true
overflow_delimited_expr = true
```

**Step 3: Create `Cargo.toml`**

```toml
[package]
name = "gnome-iris"
version = "0.1.0"
edition = "2024"
description = "ReShade manager for Wine/Proton games on Linux"
license = "GPL-2.0"

[lints.clippy]
complexity  = "deny"
correctness = "deny"
nursery     = "deny"
pedantic    = "deny"
perf        = "deny"
style       = "deny"
suspicious  = "deny"
module_name_repetitions = "deny"

[lints.rust]
missing_docs = "deny"

[profile.dev.package."*"]
opt-level = 2

[profile.release]
lto = "thin"
codegen-units = 1
strip = true

[dependencies]
anyhow = "1"
derive_more = { version = "2", features = ["full"] }
directories = "6"
env_logger = "0.11"
log = "0.4"

i18n-embed    = { version = "0.16", features = ["desktop-requester", "fluent", "fluent-system"] }
i18n-embed-fl = "0.10"

relm4 = {
    git = "https://github.com/relm4/relm4",
    rev = "baa1c23ab35e3b8c4117714042671f7ed02aeabb",
    default-features = false,
    features = ["adw", "css", "gnome_49", "macros"],
}
relm4-components = {
    git = "https://github.com/relm4/relm4",
    rev = "baa1c23ab35e3b8c4117714042671f7ed02aeabb",
    features = ["libadwaita"],
}
relm4-icons = "0.10"

rust-embed = "8"
serde      = { version = "1", features = ["serde_derive"] }
serde_json = "1"
sha2       = "0.10"
tokio      = { version = "1", features = ["macros", "rt", "sync"] }

# Domain-specific
reqwest         = { version = "0.12", features = ["stream"] }
git2            = "0.19"
keyvalues-parser = "0.2"
sevenz-rust     = "0.6"
zip             = "2"
regex           = "1"

[dev-dependencies]
tempfile = "3"

[build-dependencies]
glib-build-tools  = "0.21"
relm4-icons-build = "0.10"
```

**Step 4: Create `build.rs`**

```rust
//! Build script: compiles icons, GResources, and GSettings schemas.

use std::process::{Command, exit};

/// Output directory for compiled GSettings schemas.
const SCHEMAS_DIR: &str = "./target/share/glib-2.0/schemas/";

fn main() {
    println!("cargo::rerun-if-changed=data/org.gnome.Iris.gschema.xml");
    println!("cargo::rerun-if-changed=data/icons/icons.gresource.xml");

    relm4_icons_build::bundle_icons(
        "icon_names.rs",
        Some("org.gnome.Iris"),
        Some("/org/gnome/Iris"),
        Some("data/icons"),
        ["view-list-symbolic", "folder-open-symbolic", "preferences-system-symbolic"],
    );

    glib_build_tools::compile_resources(
        &["data/icons"],
        "data/icons/icons.gresource.xml",
        "icons.gresources",
    );

    std::fs::create_dir_all(SCHEMAS_DIR).expect("Could not create schemas output dir");
    let status = Command::new("glib-compile-schemas")
        .arg("data")
        .arg("--targetdir")
        .arg(SCHEMAS_DIR)
        .spawn()
        .expect("Failed to spawn glib-compile-schemas")
        .wait()
        .unwrap_or_else(|err| {
            eprintln!("Couldn't compile GLib schemas: {err}");
            exit(1);
        });
    assert!(status.success(), "glib-compile-schemas failed");
}
```

**Step 5: Create `data/org.gnome.Iris.gschema.xml`**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<schemalist>
  <schema id="org.gnome.Iris" path="/org/gnome/Iris/">
    <key name="window-width" type="i">
      <default>1000</default>
      <summary>Window width</summary>
    </key>
    <key name="window-height" type="i">
      <default>700</default>
      <summary>Window height</summary>
    </key>
    <key name="window-maximized" type="b">
      <default>false</default>
      <summary>Whether the window is maximized</summary>
    </key>
  </schema>
</schemalist>
```

**Step 6: Create `data/icons/icons.gresource.xml`**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<gresources>
  <gresource prefix="/org/gnome/Iris/icons/scalable/apps/">
    <file preprocess="xml-stripblanks">hicolor/scalable/apps/iris.svg</file>
  </gresource>
</gresources>
```

**Step 7: Create a placeholder `data/icons/hicolor/scalable/apps/iris.svg`**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">
  <circle cx="32" cy="32" r="30" fill="#3584e4"/>
  <text x="32" y="40" text-anchor="middle" fill="white" font-size="28" font-family="sans-serif">I</text>
</svg>
```

**Step 8: Create `i18n.toml`**

```toml
fallback_language = "en-US"

[fluent]
assets_dir = "i18n"
```

**Step 9: Create `i18n/en-US/iris.ftl`**

```ftl
# Window
app-title = Iris
add-game = Add Game
preferences = Preferences
about = About Iris

# Game detail
install = Install ReShade
uninstall = Uninstall ReShade
reinstall = Reinstall ReShade
not-installed = Not installed
installed = Installed
dll-override = DLL Override
architecture = Architecture
preset = Preset
no-preset = None

# Shaders
shaders-section = Shaders
global-default = global default

# Errors
error-title = Error
error-install = Failed to install ReShade
error-shader-update = Failed to update shader repository

# Status messages
downloading-reshade = Downloading ReShade...
extracting-reshade = Extracting ReShade...
updating-shaders = Updating shaders...
```

**Step 10: Create `src/localization.rs`**

```rust
//! Localization setup using i18n-embed and Fluent.

use i18n_embed::{
    DesktopLanguageRequester,
    fluent::{FluentLanguageLoader, fluent_language_loader},
};
use rust_embed::RustEmbed;

use anyhow::Result;

#[derive(RustEmbed)]
#[folder = "i18n"]
struct Localizations;

/// Global Fluent language loader.
pub static LANGUAGE_LOADER: std::sync::LazyLock<FluentLanguageLoader> =
    std::sync::LazyLock::new(|| {
        let loader = fluent_language_loader!();
        let requested = DesktopLanguageRequester::requested_languages();
        i18n_embed::select(&loader, &Localizations, &requested)
            .expect("Failed to load localizations");
        loader
    });

/// Initializes the localization system.
///
/// Must be called before any `fl!()` macro invocation.
pub fn setup() -> Result<()> {
    std::sync::LazyLock::force(&LANGUAGE_LOADER);
    Ok(())
}

/// Macro to look up a localized string by message ID.
#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!($crate::localization::LANGUAGE_LOADER, $message_id)
    }};
    ($message_id:literal, $($args:expr),*) => {{
        i18n_embed_fl::fl!($crate::localization::LANGUAGE_LOADER, $message_id, $($args),*)
    }};
}
```

**Step 11: Create skeleton `src/reshade/mod.rs`**

```rust
//! Core domain logic for gnome-iris — ReShade management for Wine/Proton games.
//!
//! This module is GTK-free. All types here are pure Rust.

pub mod cache;
pub mod config;
pub mod game;
pub mod install;
pub mod reshade;
pub mod shaders;
pub mod steam;
```

**Step 12: Create skeleton `src/ui/mod.rs`**

```rust
//! User interface for gnome-iris.

pub mod about;
pub mod add_game_dialog;
pub mod game_detail;
pub mod game_list;
pub mod game_shader_overrides;
pub mod install_worker;
pub mod preferences;
pub mod shader_worker;
pub mod window;
```

**Step 13: Create `src/main.rs`**

```rust
//! # gnome-iris
//!
//! GTK4 + Relm4 GUI for managing ReShade under Wine/Proton on Linux.

#![feature(never_type)]

use std::io::Write;

use anyhow::Result;

#[allow(missing_docs)]
#[allow(clippy::doc_markdown)]
pub mod icon_names {
    pub use shipped::*;
    include!(concat!(env!("OUT_DIR"), "/icon_names.rs"));
}

pub mod localization;
pub mod reshade;
pub mod ui;

use relm4::gtk::{gdk, gio};
use relm4::{RELM_THREADS, RelmApp, gtk};

use crate::ui::window::Window;

/// D-Bus application ID. Must match the GSettings schema id and GResource prefix.
pub const APPLICATION_ID: &str = "org.gnome.Iris";

fn initialize_custom_resources() {
    gio::resources_register_include!("icons.gresources").unwrap();
    let display = gdk::Display::default().unwrap();
    let theme = gtk::IconTheme::for_display(&display);
    theme.add_resource_path("/org/gnome/Iris");
}

fn main() -> Result<()> {
    relm4_icons::initialize_icons(icon_names::GRESOURCE_BYTES, icon_names::RESOURCE_PREFIX);
    initialize_custom_resources();

    env_logger::builder()
        .format(|fmt, record| writeln!(fmt, "{}: {}", record.level(), record.args()))
        .init();

    localization::setup()?;

    RELM_THREADS.set(4).expect("Could not set thread count");

    let app = RelmApp::new(APPLICATION_ID);
    app.run::<Window>(());

    Ok(())
}
```

**Step 14: Verify it compiles (stub UI files will be needed)**

Create minimal stub files for each `src/ui/*.rs` and `src/reshade/*.rs` module so the project compiles. Each stub is just:

```rust
//! Module stub.
```

Run:
```bash
cargo check 2>&1 | head -40
```
Expected: compiles or only "unused import" / "missing items" warnings. Fix any hard errors.

**Step 15: Commit**

```bash
git add -A
git commit -m "feat: scaffold project structure"
```

---

## Task 2: Domain — `config.rs`

**Files:**
- Create: `src/reshade/config.rs`
- Create: `tests/reshade/config_test.rs` (or inline `#[cfg(test)]`)

**Step 1: Write failing tests (add to bottom of `src/reshade/config.rs`)**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> GlobalConfig {
        GlobalConfig {
            shader_repos: vec![
                ShaderRepo {
                    url: "https://github.com/crosire/reshade-shaders".into(),
                    local_name: "reshade-shaders".into(),
                    branch: Some("slim".into()),
                    enabled_by_default: true,
                },
            ],
            global_ini: true,
            merge_shaders: true,
            update_interval_hours: 4,
        }
    }

    #[test]
    fn roundtrip_global_config() {
        let config = default_config();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: GlobalConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.shader_repos.len(), 1);
        assert_eq!(decoded.shader_repos[0].local_name, "reshade-shaders");
        assert_eq!(decoded.update_interval_hours, 4);
        assert!(decoded.global_ini);
        assert!(decoded.merge_shaders);
    }

    #[test]
    fn default_shader_repos_are_not_empty() {
        let config = GlobalConfig::default();
        assert!(!config.shader_repos.is_empty());
    }

    #[test]
    fn roundtrip_shader_overrides() {
        let overrides = ShaderOverrides {
            disabled_repos: vec!["sweetfx-shaders".into()],
        };
        let json = serde_json::to_string(&overrides).unwrap();
        let decoded: ShaderOverrides = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.disabled_repos, vec!["sweetfx-shaders"]);
    }
}
```

**Step 2: Run to verify failure**

```bash
cargo test reshade::config 2>&1 | tail -20
```
Expected: compile error — types not defined yet.

**Step 3: Implement `src/reshade/config.rs`**

```rust
//! Application configuration types — serialized as JSON to `$XDG_DATA_HOME/iris/`.

use serde::{Deserialize, Serialize};

/// Global application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Ordered list of shader repositories.
    pub shader_repos: Vec<ShaderRepo>,
    /// When true, a shared `ReShade.ini` is used for all games.
    pub global_ini: bool,
    /// When true, shaders from all repos are merged into a single `Merged/` directory.
    pub merge_shaders: bool,
    /// How many hours between automatic update checks.
    pub update_interval_hours: u64,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            shader_repos: default_shader_repos(),
            global_ini: true,
            merge_shaders: true,
            update_interval_hours: 4,
        }
    }
}

/// A remote Git repository containing ReShade shaders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderRepo {
    /// Remote URL (HTTPS).
    pub url: String,
    /// Local directory name under `ReShade_shaders/`.
    pub local_name: String,
    /// Optional branch name; clones the default branch when `None`.
    pub branch: Option<String>,
    /// Whether this repo is enabled for new games by default.
    pub enabled_by_default: bool,
}

/// Per-game shader repository overrides.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShaderOverrides {
    /// `local_name`s of repos disabled for this game.
    pub disabled_repos: Vec<String>,
}

fn default_shader_repos() -> Vec<ShaderRepo> {
    vec![
        ShaderRepo {
            url: "https://github.com/crosire/reshade-shaders".into(),
            local_name: "reshade-shaders".into(),
            branch: Some("slim".into()),
            enabled_by_default: true,
        },
        ShaderRepo {
            url: "https://github.com/martymcmodding/qUINT".into(),
            local_name: "martymc-shaders".into(),
            branch: None,
            enabled_by_default: true,
        },
        ShaderRepo {
            url: "https://github.com/CeeJayDK/SweetFX".into(),
            local_name: "sweetfx-shaders".into(),
            branch: None,
            enabled_by_default: true,
        },
        ShaderRepo {
            url: "https://github.com/BlueSkyDefender/AstrayFX".into(),
            local_name: "astrayfx-shaders".into(),
            branch: None,
            enabled_by_default: true,
        },
        ShaderRepo {
            url: "https://github.com/prod80/prod80-ReShade-Repository".into(),
            local_name: "prod80-shaders".into(),
            branch: None,
            enabled_by_default: true,
        },
    ]
}
```

**Step 4: Run tests**

```bash
cargo test reshade::config 2>&1
```
Expected: all tests pass.

**Step 5: Commit**

```bash
git add src/reshade/config.rs
git commit -m "feat: add GlobalConfig and ShaderRepo types"
```

---

## Task 3: Domain — `game.rs`

**Files:**
- Create: `src/reshade/game.rs`

**Step 1: Write failing tests (inline in `game.rs`)**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dll_override_symlink_name() {
        assert_eq!(DllOverride::Dxgi.symlink_name(), "dxgi.dll");
        assert_eq!(DllOverride::D3d9.symlink_name(), "d3d9.dll");
        assert_eq!(DllOverride::OpenGl32.symlink_name(), "opengl32.dll");
    }

    #[test]
    fn game_id_is_deterministic() {
        let path = std::path::PathBuf::from("/home/user/.steam/game");
        let id1 = Game::make_id(&path);
        let id2 = Game::make_id(&path);
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 128); // SHA-512 hex = 128 chars
    }

    #[test]
    fn game_id_differs_for_different_paths() {
        let a = Game::make_id(&std::path::PathBuf::from("/game/a"));
        let b = Game::make_id(&std::path::PathBuf::from("/game/b"));
        assert_ne!(a, b);
    }

    #[test]
    fn install_status_is_installed() {
        let status = InstallStatus::Installed {
            dll: DllOverride::Dxgi,
            arch: ExeArch::X86_64,
        };
        assert!(status.is_installed());
        assert!(!InstallStatus::NotInstalled.is_installed());
    }
}
```

**Step 2: Run to verify failure**

```bash
cargo test reshade::game 2>&1 | tail -10
```

**Step 3: Implement `src/reshade/game.rs`**

```rust
//! Core game data model.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};

use crate::reshade::config::ShaderOverrides;

/// A game known to the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    /// SHA-512 hex of the canonical game path — used as a stable identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Directory containing the game's `.exe`.
    pub path: PathBuf,
    /// Where this game was discovered.
    pub source: GameSource,
    /// Current ReShade install status.
    pub status: InstallStatus,
    /// Per-game shader repo opt-outs.
    pub shader_overrides: ShaderOverrides,
}

impl Game {
    /// Creates a new uninstalled game entry.
    pub fn new(name: String, path: PathBuf, source: GameSource) -> Self {
        let id = Self::make_id(&path);
        Self {
            id,
            name,
            path,
            source,
            status: InstallStatus::NotInstalled,
            shader_overrides: ShaderOverrides::default(),
        }
    }

    /// Derives a stable ID from the game path (SHA-512 hex).
    pub fn make_id(path: &PathBuf) -> String {
        let mut hasher = Sha512::new();
        hasher.update(path.to_string_lossy().as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// How a game was discovered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameSource {
    /// Discovered from the Steam library.
    Steam {
        /// Steam App ID.
        app_id: u32,
    },
    /// Added manually by the user.
    Manual,
}

/// ReShade installation status for a game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallStatus {
    /// ReShade is not installed for this game.
    NotInstalled,
    /// ReShade is installed with these settings.
    Installed {
        /// The DLL that ReShade is masquerading as.
        dll: DllOverride,
        /// Detected executable architecture.
        arch: ExeArch,
    },
}

impl InstallStatus {
    /// Returns `true` if ReShade is currently installed.
    pub fn is_installed(&self) -> bool {
        matches!(self, Self::Installed { .. })
    }
}

/// The Windows DLL name that ReShade replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DllOverride {
    /// `d3d8.dll`
    D3d8,
    /// `d3d9.dll`
    D3d9,
    /// `d3d11.dll`
    D3d11,
    /// `ddraw.dll`
    Ddraw,
    /// `dinput8.dll`
    Dinput8,
    /// `dxgi.dll` (default for 64-bit DirectX games)
    Dxgi,
    /// `opengl32.dll`
    OpenGl32,
}

impl DllOverride {
    /// Returns the filename used for the symlink in the game directory.
    pub fn symlink_name(self) -> &'static str {
        match self {
            Self::D3d8 => "d3d8.dll",
            Self::D3d9 => "d3d9.dll",
            Self::D3d11 => "d3d11.dll",
            Self::Ddraw => "ddraw.dll",
            Self::Dinput8 => "dinput8.dll",
            Self::Dxgi => "dxgi.dll",
            Self::OpenGl32 => "opengl32.dll",
        }
    }

    /// All supported DLL overrides (for UI dropdown).
    pub fn all() -> &'static [Self] {
        &[
            Self::D3d8,
            Self::D3d9,
            Self::D3d11,
            Self::Ddraw,
            Self::Dinput8,
            Self::Dxgi,
            Self::OpenGl32,
        ]
    }
}

impl std::fmt::Display for DllOverride {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.symlink_name())
    }
}

/// Executable architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExeArch {
    /// 32-bit (x86).
    X86,
    /// 64-bit (x86-64).
    X86_64,
}

impl ExeArch {
    /// Returns the ReShade DLL filename for this architecture.
    pub fn reshade_dll(self) -> &'static str {
        match self {
            Self::X86 => "ReShade32.dll",
            Self::X86_64 => "ReShade64.dll",
        }
    }

    /// Returns the d3dcompiler suffix for this architecture.
    pub fn d3dcompiler_suffix(self) -> &'static str {
        match self {
            Self::X86 => "32",
            Self::X86_64 => "64",
        }
    }
}
```

**Step 4: Run tests**

```bash
cargo test reshade::game 2>&1
```
Expected: all pass.

**Step 5: Commit**

```bash
git add src/reshade/game.rs
git commit -m "feat: add Game, DllOverride, ExeArch types"
```

---

## Task 4: Domain — `steam.rs`

**Files:**
- Create: `src/reshade/steam.rs`
- Create: `tests/fixtures/libraryfolders.vdf`

**Step 1: Create fixture VDF**

`tests/fixtures/libraryfolders.vdf`:
```
"libraryfolders"
{
	"0"
	{
		"path"		"/home/user/.local/share/Steam"
		"label"		""
		"contentid"		"1234567890"
		"totalsize"		"500000000000"
		"update_clean_bytes_tally"		"0"
		"time_last_update_corruption"		"0"
		"apps"
		{
			"570"		"8000000"
			"730"		"15000000"
		}
	}
	"1"
	{
		"path"		"/mnt/games/Steam"
		"label"		"Games Drive"
		"contentid"		"9876543210"
		"totalsize"		"2000000000000"
		"update_clean_bytes_tally"		"0"
		"time_last_update_corruption"		"0"
		"apps"
		{
			"379720"		"40000000"
		}
	}
}
```

**Step 2: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_vdf() -> &'static str {
        include_str!("../../tests/fixtures/libraryfolders.vdf")
    }

    #[test]
    fn parse_library_folders() {
        let folders = parse_library_folders_vdf(fixture_vdf()).unwrap();
        assert_eq!(folders.len(), 2);
        assert!(folders[0].ends_with(".local/share/Steam"));
        assert!(folders[1].ends_with("mnt/games/Steam"));
    }

    #[test]
    fn detect_arch_from_elf_header_x86_64() {
        // ELF magic + x86-64 machine type (0x3e)
        let mut header = [0u8; 20];
        header[0..4].copy_from_slice(b"\x7fELF");
        header[4] = 2; // EI_CLASS: 64-bit
        header[18] = 0x3e; // e_machine low byte: x86-64
        header[19] = 0x00;
        assert_eq!(arch_from_elf(&header), Some(ExeArch::X86_64));
    }

    #[test]
    fn detect_arch_from_elf_header_x86() {
        let mut header = [0u8; 20];
        header[0..4].copy_from_slice(b"\x7fELF");
        header[4] = 1; // EI_CLASS: 32-bit
        header[18] = 0x03; // e_machine low byte: x86
        assert_eq!(arch_from_elf(&header), Some(ExeArch::X86));
    }
}
```

**Step 3: Run to verify failure**

```bash
cargo test reshade::steam 2>&1 | tail -10
```

**Step 4: Implement `src/reshade/steam.rs`**

```rust
//! Steam library discovery — reads `libraryfolders.vdf` to find installed games.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use keyvalues_parser::Vdf;

use crate::reshade::game::{ExeArch, Game, GameSource};

/// Returns all Steam library root paths found on this system.
pub fn find_steam_libraries() -> Result<Vec<PathBuf>> {
    let steam_root = steam_root().context("Steam not found")?;
    let vdf_path = steam_root.join("steamapps/libraryfolders.vdf");
    let vdf_str = std::fs::read_to_string(&vdf_path)
        .with_context(|| format!("Cannot read {}", vdf_path.display()))?;
    parse_library_folders_vdf(&vdf_str)
}

/// Parses the `libraryfolders.vdf` content and returns library root paths.
pub fn parse_library_folders_vdf(vdf_str: &str) -> Result<Vec<PathBuf>> {
    let vdf = Vdf::parse(vdf_str).context("Invalid VDF")?;
    let root = vdf.value.get_obj().context("VDF root is not an object")?;
    let mut paths = Vec::new();
    for (_key, values) in root {
        for value in values {
            if let Some(obj) = value.get_obj() {
                for (k, vs) in obj {
                    if k.as_ref() == "path" {
                        if let Some(v) = vs.first().and_then(|v| v.get_str()) {
                            paths.push(PathBuf::from(v.as_ref()));
                        }
                    }
                }
            }
        }
    }
    Ok(paths)
}

/// Scans all Steam libraries and returns discovered games.
pub fn discover_steam_games() -> Vec<Game> {
    let Ok(libraries) = find_steam_libraries() else {
        return vec![];
    };
    let mut games = Vec::new();
    for library in libraries {
        if let Ok(entries) = std::fs::read_dir(library.join("steamapps/common")) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    games.push(Game::new(name, path, GameSource::Steam { app_id: 0 }));
                }
            }
        }
    }
    games
}

/// Detects the architecture of a PE `.exe` by reading its header bytes.
///
/// Returns `None` if the file cannot be read or is not a valid PE.
pub fn detect_exe_arch(exe_path: &Path) -> Option<ExeArch> {
    use std::io::Read;
    let mut file = std::fs::File::open(exe_path).ok()?;
    let mut dos_header = [0u8; 64];
    file.read_exact(&mut dos_header).ok()?;
    // Check MZ magic
    if &dos_header[0..2] != b"MZ" {
        return None;
    }
    // PE offset is at 0x3c
    let pe_offset = u32::from_le_bytes(dos_header[60..64].try_into().ok()?) as usize;
    let mut buf = vec![0u8; pe_offset + 6];
    let mut file2 = std::fs::File::open(exe_path).ok()?;
    file2.read_exact(&mut buf).ok()?;
    // Check PE signature
    if &buf[pe_offset..pe_offset + 4] != b"PE\0\0" {
        return None;
    }
    // Machine type is 2 bytes after PE signature
    let machine = u16::from_le_bytes(buf[pe_offset + 4..pe_offset + 6].try_into().ok()?);
    match machine {
        0x014c => Some(ExeArch::X86),
        0x8664 => Some(ExeArch::X86_64),
        _ => None,
    }
}

/// Detect arch from a raw ELF header (used in tests).
pub fn arch_from_elf(header: &[u8]) -> Option<ExeArch> {
    if header.len() < 20 || &header[0..4] != b"\x7fELF" {
        return None;
    }
    match header[4] {
        1 => Some(ExeArch::X86),
        2 => Some(ExeArch::X86_64),
        _ => None,
    }
}

fn steam_root() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let candidates = [
        format!("{home}/.local/share/Steam"),
        format!("{home}/.steam/steam"),
        format!("{home}/.steam/root"),
    ];
    candidates.into_iter().map(PathBuf::from).find(|p| p.exists())
}
```

**Step 5: Run tests**

```bash
cargo test reshade::steam 2>&1
```
Expected: all pass.

**Step 6: Commit**

```bash
git add src/reshade/steam.rs tests/fixtures/libraryfolders.vdf
git commit -m "feat: add Steam library discovery"
```

---

## Task 5: Domain — `cache.rs`

**Files:**
- Create: `src/reshade/cache.rs`

**Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn write_and_read_version() {
        let dir = tempdir().unwrap();
        let cache = UpdateCache::new(dir.path().to_path_buf());
        cache.write_version("6.1.0").unwrap();
        assert_eq!(cache.read_version().unwrap().as_deref(), Some("6.1.0"));
    }

    #[test]
    fn needs_update_when_no_timestamp() {
        let dir = tempdir().unwrap();
        let cache = UpdateCache::new(dir.path().to_path_buf());
        assert!(cache.needs_update(4));
    }

    #[test]
    fn does_not_need_update_when_recent() {
        let dir = tempdir().unwrap();
        let cache = UpdateCache::new(dir.path().to_path_buf());
        cache.touch().unwrap();
        assert!(!cache.needs_update(4));
    }
}
```

**Step 2: Run to verify failure**

```bash
cargo test reshade::cache 2>&1 | tail -10
```

**Step 3: Implement `src/reshade/cache.rs`**

```rust
//! Update tracking — stores the last known ReShade version and update timestamp.

use std::path::PathBuf;

use anyhow::Result;

/// Manages the version and timestamp files under the iris data directory.
pub struct UpdateCache {
    base: PathBuf,
}

impl UpdateCache {
    /// Creates a new cache pointing at the given directory.
    pub fn new(base: PathBuf) -> Self {
        Self { base }
    }

    /// Returns the last recorded ReShade version, or `None` if unknown.
    pub fn read_version(&self) -> Result<Option<String>> {
        let path = self.base.join("LVERS");
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(std::fs::read_to_string(path)?.trim().to_owned()))
    }

    /// Writes the current ReShade version to disk.
    pub fn write_version(&self, version: &str) -> Result<()> {
        std::fs::create_dir_all(&self.base)?;
        std::fs::write(self.base.join("LVERS"), version)?;
        Ok(())
    }

    /// Writes the current timestamp to the `LASTUPDATED` file.
    pub fn touch(&self) -> Result<()> {
        std::fs::create_dir_all(&self.base)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        std::fs::write(self.base.join("LASTUPDATED"), now.to_string())?;
        Ok(())
    }

    /// Returns `true` if more than `interval_hours` have passed since the last update.
    pub fn needs_update(&self, interval_hours: u64) -> bool {
        let path = self.base.join("LASTUPDATED");
        let Ok(content) = std::fs::read_to_string(path) else {
            return true;
        };
        let Ok(ts) = content.trim().parse::<u64>() else {
            return true;
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now.saturating_sub(ts) >= interval_hours * 3600
    }
}
```

**Step 4: Run tests**

```bash
cargo test reshade::cache 2>&1
```

**Step 5: Commit**

```bash
git add src/reshade/cache.rs
git commit -m "feat: add UpdateCache for version and timestamp tracking"
```

---

## Task 6: Domain — `reshade.rs` (version fetching + extraction)

**Files:**
- Create: `src/reshade/reshade.rs`

This module handles fetching the ReShade version number from `reshade.me` and extracting the downloaded `.exe` (self-extracting zip).

**Step 1: Write failing tests (pure parsing tests, no network)**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_from_html() {
        // Minimal HTML that matches what reshade.me serves
        let html = r#"<html><body>
            <a href="/downloads/ReShade_6.1.0.exe">Download ReShade 6.1.0</a>
            </body></html>"#;
        let version = parse_version_from_html(html).unwrap();
        assert_eq!(version, "6.1.0");
    }

    #[test]
    fn parse_version_addon_from_html() {
        let html = r#"<a href="/downloads/ReShade_6.1.0_Addon.exe">Download</a>"#;
        let version = parse_version_from_html(html).unwrap();
        assert_eq!(version, "6.1.0");
    }

    #[test]
    fn build_download_url_standard() {
        let url = download_url("6.1.0", false);
        assert_eq!(url, "https://reshade.me/downloads/ReShade_6.1.0.exe");
    }

    #[test]
    fn build_download_url_addon() {
        let url = download_url("6.1.0", true);
        assert_eq!(url, "https://reshade.me/downloads/ReShade_6.1.0_Addon.exe");
    }
}
```

**Step 2: Run to verify failure**

```bash
cargo test reshade::reshade 2>&1 | tail -10
```

**Step 3: Implement `src/reshade/reshade.rs`**

```rust
//! ReShade version fetching and extraction.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use regex::Regex;

/// Fetches the latest ReShade version string from `reshade.me`.
pub async fn fetch_latest_version() -> Result<String> {
    let html = reqwest::get("https://reshade.me")
        .await
        .context("Failed to connect to reshade.me")?
        .text()
        .await?;
    parse_version_from_html(&html).context("Could not parse ReShade version from reshade.me")
}

/// Parses the ReShade version string from the HTML of `reshade.me`.
pub fn parse_version_from_html(html: &str) -> Result<String> {
    let re = Regex::new(r#"/downloads/ReShade_(\d+\.\d+\.\d+)(?:_Addon)?\.exe"#)
        .expect("static regex");
    re.captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_owned())
        .ok_or_else(|| anyhow!("No ReShade version found in HTML"))
}

/// Builds the download URL for a given version.
pub fn download_url(version: &str, addon_support: bool) -> String {
    if addon_support {
        format!("https://reshade.me/downloads/ReShade_{version}_Addon.exe")
    } else {
        format!("https://reshade.me/downloads/ReShade_{version}.exe")
    }
}

/// Downloads a ReShade `.exe` and extracts it to `dest_dir`.
///
/// The `.exe` is a self-extracting zip. We extract it directly with the `zip` crate.
pub async fn download_and_extract(url: &str, dest_dir: &Path) -> Result<()> {
    let bytes = reqwest::get(url)
        .await
        .with_context(|| format!("Failed to download {url}"))?
        .bytes()
        .await?;
    std::fs::create_dir_all(dest_dir)?;
    extract_zip_from_bytes(&bytes, dest_dir)?;
    Ok(())
}

/// Extracts all entries from a zip archive contained in `bytes` into `dest_dir`.
pub fn extract_zip_from_bytes(bytes: &[u8], dest_dir: &Path) -> Result<()> {
    use std::io::Cursor;
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).context("Not a valid zip archive")?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_owned();
        // Only extract DLL files we care about
        if !name.ends_with(".dll") {
            continue;
        }
        let dest = dest_dir.join(&name);
        let mut out = std::fs::File::create(&dest)
            .with_context(|| format!("Cannot create {}", dest.display()))?;
        std::io::copy(&mut entry, &mut out)?;
    }
    Ok(())
}

/// Returns the versioned directory for a given ReShade version.
pub fn version_dir(base: &Path, version: &str) -> PathBuf {
    base.join("reshade").join(version)
}

/// Updates the `latest` symlink to point to `version_dir`.
pub fn update_latest_symlink(base: &Path, version: &str) -> Result<()> {
    let latest = base.join("reshade/latest");
    let target = PathBuf::from(version);
    if latest.exists() || latest.is_symlink() {
        std::fs::remove_file(&latest)?;
    }
    std::os::unix::fs::symlink(target, latest)?;
    Ok(())
}
```

**Step 4: Run tests**

```bash
cargo test reshade::reshade 2>&1
```

**Step 5: Commit**

```bash
git add src/reshade/reshade.rs
git commit -m "feat: add ReShade version fetching and extraction"
```

---

## Task 7: Domain — `shaders.rs`

**Files:**
- Create: `src/reshade/shaders.rs`

**Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn merge_creates_symlinks_for_fx_files() {
        let dir = tempdir().unwrap();
        let src_shaders = dir.path().join("repo/Shaders");
        let merged = dir.path().join("Merged/Shaders");
        std::fs::create_dir_all(&src_shaders).unwrap();
        std::fs::create_dir_all(&merged).unwrap();
        std::fs::write(src_shaders.join("test.fx"), "// shader").unwrap();

        link_shader_files(&src_shaders, &merged).unwrap();

        let link = merged.join("test.fx");
        assert!(link.exists(), "symlink should exist");
        assert!(link.is_symlink(), "should be a symlink");
    }

    #[test]
    fn merge_does_not_overwrite_existing_symlink() {
        let dir = tempdir().unwrap();
        let src1 = dir.path().join("repo1/Shaders");
        let src2 = dir.path().join("repo2/Shaders");
        let merged = dir.path().join("Merged/Shaders");
        std::fs::create_dir_all(&src1).unwrap();
        std::fs::create_dir_all(&src2).unwrap();
        std::fs::create_dir_all(&merged).unwrap();
        std::fs::write(src1.join("common.fx"), "// v1").unwrap();
        std::fs::write(src2.join("common.fx"), "// v2").unwrap();

        link_shader_files(&src1, &merged).unwrap();
        link_shader_files(&src2, &merged).unwrap(); // should not panic

        // Should still point to src1 version (first wins)
        let target = std::fs::read_link(merged.join("common.fx")).unwrap();
        assert!(target.to_string_lossy().contains("repo1"));
    }
}
```

**Step 2: Run to verify failure**

```bash
cargo test reshade::shaders 2>&1 | tail -10
```

**Step 3: Implement `src/reshade/shaders.rs`**

```rust
//! Shader repository management: clone, update, and merge into a unified directory.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::reshade::config::ShaderRepo;

/// Clones or updates a shader repository.
///
/// If the local directory does not exist, it is cloned from `repo.url`.
/// If it does exist, `git pull` (fetch + merge) is performed.
pub fn sync_repo(repo: &ShaderRepo, repos_dir: &Path) -> Result<()> {
    let dest = repos_dir.join(&repo.local_name);
    if dest.exists() {
        let git_repo = git2::Repository::open(&dest)
            .with_context(|| format!("Cannot open repo at {}", dest.display()))?;
        fetch_and_merge(&git_repo)?;
    } else {
        let mut opts = git2::FetchOptions::new();
        opts.download_tags(git2::AutotagOption::None);
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(opts);
        if let Some(branch) = &repo.branch {
            builder.branch(branch);
        }
        builder
            .clone(&repo.url, &dest)
            .with_context(|| format!("Failed to clone {}", repo.url))?;
    }
    Ok(())
}

fn fetch_and_merge(repo: &git2::Repository) -> Result<()> {
    let mut remote = repo.find_remote("origin")?;
    remote.fetch(&[] as &[&str], None, None)?;
    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
    let (analysis, _) = repo.merge_analysis(&[&fetch_commit])?;
    if analysis.is_fast_forward() {
        let mut reference = repo.find_reference("HEAD")?;
        reference.set_target(fetch_commit.id(), "Fast-forward")?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    }
    Ok(())
}

/// Rebuilds the `Merged/` directory by symlinking all unique shader/texture files.
///
/// Priority is determined by order in `repos`: first repo wins on name collision.
pub fn rebuild_merged(repos_dir: &Path, disabled_repos: &[String]) -> Result<()> {
    let merged_shaders = repos_dir.join("Merged/Shaders");
    let merged_textures = repos_dir.join("Merged/Textures");
    std::fs::create_dir_all(&merged_shaders)?;
    std::fs::create_dir_all(&merged_textures)?;

    let entries = std::fs::read_dir(repos_dir)
        .context("Cannot read repos dir")?
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name != "Merged" && !disabled_repos.contains(&name)
        });

    for entry in entries {
        let shaders_src = entry.path().join("Shaders");
        let textures_src = entry.path().join("Textures");
        if shaders_src.exists() {
            link_shader_files(&shaders_src, &merged_shaders)?;
        }
        if textures_src.exists() {
            link_shader_files(&textures_src, &merged_textures)?;
        }
    }
    Ok(())
}

/// Creates symlinks in `dest_dir` for each file in `src_dir`.
/// Skips files that already have a symlink in `dest_dir` (first-wins).
pub fn link_shader_files(src_dir: &Path, dest_dir: &Path) -> Result<()> {
    for entry in std::fs::read_dir(src_dir).context("Cannot read shader dir")?.flatten() {
        let src = entry.path();
        if !src.is_file() {
            continue;
        }
        let file_name = entry.file_name();
        let dest = dest_dir.join(&file_name);
        if dest.exists() || dest.is_symlink() {
            continue; // first repo wins
        }
        std::os::unix::fs::symlink(&src, &dest)
            .with_context(|| format!("Cannot link {} -> {}", src.display(), dest.display()))?;
    }
    Ok(())
}

/// Returns the path to the merged shaders directory.
pub fn merged_shaders_dir(base: &Path) -> PathBuf {
    base.join("ReShade_shaders/Merged/Shaders")
}

/// Returns the path to the merged textures directory.
pub fn merged_textures_dir(base: &Path) -> PathBuf {
    base.join("ReShade_shaders/Merged/Textures")
}
```

**Step 4: Run tests**

```bash
cargo test reshade::shaders 2>&1
```

**Step 5: Commit**

```bash
git add src/reshade/shaders.rs
git commit -m "feat: add shader repo sync and merge logic"
```

---

## Task 8: Domain — `install.rs`

**Files:**
- Create: `src/reshade/install.rs`

**Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup_fake_reshade(base: &std::path::Path, version: &str, arch: ExeArch) {
        let dll_name = arch.reshade_dll();
        let versioned = base.join("reshade").join(version);
        std::fs::create_dir_all(&versioned).unwrap();
        std::fs::write(versioned.join(dll_name), "fake dll").unwrap();
        // also write d3dcompiler
        let suffix = arch.d3dcompiler_suffix();
        std::fs::write(base.join(format!("d3dcompiler_47.dll.{suffix}")), "fake").unwrap();
    }

    #[test]
    fn install_creates_symlinks() {
        let base = tempdir().unwrap();
        let game_dir = tempdir().unwrap();
        let version = "6.1.0";
        let arch = ExeArch::X86_64;
        let dll = DllOverride::Dxgi;
        setup_fake_reshade(base.path(), version, arch);

        install_reshade(base.path(), game_dir.path(), version, dll, arch).unwrap();

        assert!(game_dir.path().join("dxgi.dll").is_symlink());
        assert!(game_dir.path().join("d3dcompiler_47.dll").is_symlink());
    }

    #[test]
    fn uninstall_removes_symlinks() {
        let base = tempdir().unwrap();
        let game_dir = tempdir().unwrap();
        let arch = ExeArch::X86_64;
        let dll = DllOverride::Dxgi;
        setup_fake_reshade(base.path(), "6.1.0", arch);

        install_reshade(base.path(), game_dir.path(), "6.1.0", dll, arch).unwrap();
        uninstall_reshade(game_dir.path(), dll).unwrap();

        assert!(!game_dir.path().join("dxgi.dll").exists());
        assert!(!game_dir.path().join("d3dcompiler_47.dll").exists());
    }
}
```

**Step 2: Run to verify failure**

```bash
cargo test reshade::install 2>&1 | tail -10
```

**Step 3: Implement `src/reshade/install.rs`**

```rust
//! Install and uninstall ReShade into a game directory via symlinks.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::reshade::game::{DllOverride, ExeArch};

/// Installs ReShade into `game_dir` by creating symlinks.
///
/// Links:
/// - `ReShade{32,64}.dll` → `<dll>.dll`
/// - `d3dcompiler_47.dll.<arch>` → `d3dcompiler_47.dll`
/// - `ReShade_shaders/Merged` → `reshade-shaders`
pub fn install_reshade(
    base: &Path,
    game_dir: &Path,
    version: &str,
    dll: DllOverride,
    arch: ExeArch,
) -> Result<()> {
    // ReShade DLL → <dll>.dll
    let reshade_src = base
        .join("reshade")
        .join(version)
        .join(arch.reshade_dll());
    let dll_dest = game_dir.join(dll.symlink_name());
    symlink_force(&reshade_src, &dll_dest)?;

    // d3dcompiler
    let d3dc_src = base.join(format!("d3dcompiler_47.dll.{}", arch.d3dcompiler_suffix()));
    let d3dc_dest = game_dir.join("d3dcompiler_47.dll");
    symlink_force(&d3dc_src, &d3dc_dest)?;

    // Shaders dir
    let shaders_src = base.join("ReShade_shaders/Merged");
    let shaders_dest = game_dir.join("reshade-shaders");
    if shaders_src.exists() && !shaders_dest.exists() {
        symlink_force(&shaders_src, &shaders_dest)?;
    }

    Ok(())
}

/// Removes all ReShade symlinks from `game_dir`.
pub fn uninstall_reshade(game_dir: &Path, dll: DllOverride) -> Result<()> {
    let files = [
        dll.symlink_name().to_owned(),
        "d3dcompiler_47.dll".into(),
        "reshade-shaders".into(),
        "ReShade.ini".into(),
        "ReShade32.json".into(),
        "ReShade64.json".into(),
    ];
    for name in &files {
        let path = game_dir.join(name);
        if path.is_symlink() {
            std::fs::remove_file(&path)
                .with_context(|| format!("Cannot remove {}", path.display()))?;
        }
    }
    Ok(())
}

/// Creates a symlink at `dest` pointing to `src`, removing any existing entry first.
fn symlink_force(src: &Path, dest: &Path) -> Result<()> {
    if dest.exists() || dest.is_symlink() {
        std::fs::remove_file(dest)?;
    }
    std::os::unix::fs::symlink(src, dest)
        .with_context(|| format!("Cannot create symlink {} -> {}", dest.display(), src.display()))
}

/// Detects the default DLL override for a given architecture.
pub fn default_dll_for_arch(arch: ExeArch) -> DllOverride {
    match arch {
        ExeArch::X86 => DllOverride::D3d9,
        ExeArch::X86_64 => DllOverride::Dxgi,
    }
}

/// Returns all `.exe` files in `game_dir`.
pub fn find_exes(game_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(game_dir) else {
        return vec![];
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "exe").unwrap_or(false))
        .collect()
}
```

**Step 4: Run tests**

```bash
cargo test reshade::install 2>&1
```

**Step 5: Commit**

```bash
git add src/reshade/install.rs
git commit -m "feat: add ReShade install/uninstall logic"
```

---

## Task 9: Shared App State

**Files:**
- Create: `src/reshade/app_state.rs`
- Modify: `src/reshade/mod.rs` (add `pub mod app_state;`)

**Step 1: Implement `src/reshade/app_state.rs`**

No unit tests needed — this is a pure data container wiring together the domain types.

```rust
//! Shared application state passed between UI components.

use std::path::PathBuf;
use std::sync::Arc;

use directories::ProjectDirs;
use tokio::sync::RwLock;

use crate::reshade::config::GlobalConfig;
use crate::reshade::game::Game;

/// Shared mutable application state.
pub type Shared<T> = Arc<RwLock<T>>;

/// Top-level application state shared across all Relm4 components.
#[derive(Debug)]
pub struct AppState {
    /// All games known to the application.
    pub games: Vec<Game>,
    /// Currently installed ReShade version, if any.
    pub reshade_version: Option<String>,
    /// Global configuration.
    pub config: GlobalConfig,
    /// Root data directory (`$XDG_DATA_HOME/iris/`).
    pub data_dir: PathBuf,
}

impl AppState {
    /// Initializes app state from disk (or defaults if first run).
    pub fn load() -> Self {
        let data_dir = data_dir();
        let config = load_config(&data_dir);
        let games = load_games(&data_dir);
        let reshade_version = load_reshade_version(&data_dir);
        Self { games, reshade_version, config, data_dir }
    }

    /// Persists the current state to disk.
    pub fn save(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        let config_json = serde_json::to_string_pretty(&self.config)?;
        std::fs::write(self.data_dir.join("config.json"), config_json)?;
        let games_json = serde_json::to_string_pretty(&self.games)?;
        std::fs::write(self.data_dir.join("games.json"), games_json)?;
        Ok(())
    }
}

fn data_dir() -> PathBuf {
    ProjectDirs::from("org", "gnome", "Iris")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("~/.local/share/iris"))
}

fn load_config(data_dir: &PathBuf) -> GlobalConfig {
    let path = data_dir.join("config.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn load_games(data_dir: &PathBuf) -> Vec<Game> {
    let path = data_dir.join("games.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn load_reshade_version(data_dir: &PathBuf) -> Option<String> {
    let cache = crate::reshade::cache::UpdateCache::new(data_dir.clone());
    cache.read_version().ok().flatten()
}
```

**Step 2: Add module to `src/reshade/mod.rs`**

```rust
pub mod app_state;
pub mod cache;
pub mod config;
pub mod game;
pub mod install;
pub mod reshade;
pub mod shaders;
pub mod steam;
```

**Step 3: Verify compile**

```bash
cargo check 2>&1 | head -30
```

**Step 4: Commit**

```bash
git add src/reshade/app_state.rs src/reshade/mod.rs
git commit -m "feat: add AppState with load/save"
```

---

## Task 10: UI — Root Window

**Files:**
- Create: `src/ui/window.rs`
- Create: `src/ui/about.rs`

Read `docs/relm4-components.md` (SimpleComponent section) and `docs/ui-patterns.md` (OverlaySplitView section) before implementing.

**Step 1: Implement `src/ui/about.rs`**

```rust
//! About dialog for gnome-iris.

use relm4::adw;
use relm4::{ComponentParts, ComponentSender, SimpleComponent};

/// About dialog component.
pub struct AboutDialog;

/// Input messages for [`AboutDialog`].
#[derive(Debug)]
pub enum Controls {
    /// Show the dialog.
    Show,
}

#[relm4::component(pub)]
impl SimpleComponent for AboutDialog {
    type Init = ();
    type Input = Controls;
    type Output = ();

    view! {
        #[name(dialog)]
        adw::AboutDialog {
            set_application_name: "Iris",
            set_application_icon: "iris",
            set_developer_name: "gnome-iris contributors",
            set_version: env!("CARGO_PKG_VERSION"),
            set_license_type: gtk::License::Gpl20,
            set_comments: "ReShade manager for Wine/Proton games on Linux",
        }
    }

    fn init(_: (), _root: Self::Root, _sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = Self;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, _sender: ComponentSender<Self>) {
        match msg {
            Controls::Show => {}
        }
    }
}
```

**Step 2: Implement `src/ui/window.rs`**

```rust
//! Root application window.

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use crate::fl;

/// Root window model.
pub struct Window {
    /// Whether the sidebar is shown (used for narrow screens).
    sidebar_visible: bool,
}

/// Input messages for [`Window`].
#[derive(Debug)]
pub enum Controls {
    /// Toggle the sidebar.
    ToggleSidebar,
}

#[relm4::component(pub)]
impl SimpleComponent for Window {
    type Init = ();
    type Input = Controls;
    type Output = ();

    view! {
        adw::ApplicationWindow {
            set_title: Some(&fl!("app-title")),
            set_default_width: 1000,
            set_default_height: 700,

            adw::OverlaySplitView {
                #[watch]
                set_show_sidebar: model.sidebar_visible,

                #[wrap(Some)]
                set_sidebar = &adw::NavigationPage {
                    set_title: &fl!("app-title"),
                    set_width_request: 260,

                    adw::ToolbarView {
                        add_top_bar = &adw::HeaderBar {
                            pack_start = &gtk::Button {
                                set_icon_name: "folder-open-symbolic",
                                set_tooltip_text: Some(&fl!("add-game")),
                            },
                        },
                        // GameList will go here in Task 11
                        gtk::Label {
                            set_label: "Game list placeholder",
                        },
                    },
                },

                #[wrap(Some)]
                set_content = &adw::NavigationPage {
                    set_title: "Detail",

                    adw::ToolbarView {
                        add_top_bar = &adw::HeaderBar {},
                        // GameDetail will go here in Task 12
                        adw::StatusPage {
                            set_title: "Select a game",
                            set_icon_name: Some("view-list-symbolic"),
                        },
                    },
                },
            },
        }
    }

    fn init(_: (), _root: Self::Root, _sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = Self { sidebar_visible: true };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, _sender: ComponentSender<Self>) {
        match msg {
            Controls::ToggleSidebar => self.sidebar_visible = !self.sidebar_visible,
        }
    }
}
```

**Step 3: Verify compile**

```bash
cargo check 2>&1 | head -30
```

**Step 4: Commit**

```bash
git add src/ui/window.rs src/ui/about.rs
git commit -m "feat: add root window with OverlaySplitView skeleton"
```

---

## Task 11: UI — Game List Sidebar

**Files:**
- Create: `src/ui/game_list.rs`

**Step 1: Implement `src/ui/game_list.rs`**

```rust
//! Sidebar listing all known games.

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use crate::reshade::game::{Game, InstallStatus};

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
    /// User selected a game by its ID.
    GameSelected(String),
    /// User clicked the Add Game button.
    AddGameRequested,
}

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

    fn init(games: Vec<Game>, _root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = Self { games: games.clone() };
        let widgets = view_output!();
        // Populate initial rows
        for game in &games {
            let row = make_game_row(game);
            widgets.list_box.append(&row);
        }
        // Emit selection signal
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
```

**Step 2: Verify compile**

```bash
cargo check 2>&1 | head -30
```

**Step 3: Commit**

```bash
git add src/ui/game_list.rs
git commit -m "feat: add game list sidebar component"
```

---

## Task 12: UI — `install_worker.rs` and `shader_worker.rs`

**Files:**
- Create: `src/ui/install_worker.rs`
- Create: `src/ui/shader_worker.rs`

Read `docs/relm4-components.md` (AsyncComponent section) before implementing.

**Step 1: Implement `src/ui/install_worker.rs`**

```rust
//! Async worker for downloading and installing ReShade.

use std::path::PathBuf;

use anyhow::Result;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, Worker};

use crate::reshade::game::{DllOverride, ExeArch};
use crate::reshade::{install, reshade};

/// Input commands for the install worker.
#[derive(Debug)]
pub enum Controls {
    /// Download the latest ReShade and install into the given game dir.
    Install {
        /// App data directory.
        data_dir: PathBuf,
        /// Game directory to install into.
        game_dir: PathBuf,
        /// DLL override to use.
        dll: DllOverride,
        /// Executable architecture.
        arch: ExeArch,
    },
    /// Remove ReShade from the given game dir.
    Uninstall {
        /// Game directory to uninstall from.
        game_dir: PathBuf,
        /// The DLL override currently in use.
        dll: DllOverride,
    },
}

/// Output signals from the install worker.
#[derive(Debug)]
pub enum Signal {
    /// A step completed with a status message.
    Progress(String),
    /// Installation finished successfully.
    InstallComplete {
        /// The installed ReShade version.
        version: String,
    },
    /// Uninstall finished successfully.
    UninstallComplete,
    /// An error occurred.
    Error(String),
}

/// Background install worker (no widget tree).
pub struct InstallWorker;

impl Worker for InstallWorker {
    type Init = ();
    type Input = Controls;
    type Output = Signal;

    fn init(_: (), _sender: ComponentSender<Self>) -> Self {
        Self
    }

    fn update(&mut self, msg: Controls, sender: ComponentSender<Self>) {
        match msg {
            Controls::Install { data_dir, game_dir, dll, arch } => {
                let sender = sender.clone();
                relm4::spawn(async move {
                    if let Err(e) = do_install(&data_dir, &game_dir, dll, arch, &sender).await {
                        sender.output(Signal::Error(e.to_string())).ok();
                    }
                });
            }
            Controls::Uninstall { game_dir, dll } => {
                match install::uninstall_reshade(&game_dir, dll) {
                    Ok(()) => { sender.output(Signal::UninstallComplete).ok(); }
                    Err(e) => { sender.output(Signal::Error(e.to_string())).ok(); }
                }
            }
        }
    }
}

async fn do_install(
    data_dir: &std::path::Path,
    game_dir: &std::path::Path,
    dll: DllOverride,
    arch: ExeArch,
    sender: &ComponentSender<InstallWorker>,
) -> Result<()> {
    sender.output(Signal::Progress("Fetching latest ReShade version...".into())).ok();
    let version = reshade::fetch_latest_version().await?;

    let version_dir = reshade::version_dir(data_dir, &version);
    if !version_dir.join(arch.reshade_dll()).exists() {
        sender.output(Signal::Progress(format!("Downloading ReShade {version}..."))).ok();
        let url = reshade::download_url(&version, false);
        reshade::download_and_extract(&url, &version_dir).await?;
        reshade::update_latest_symlink(data_dir, &version)?;
    }

    sender.output(Signal::Progress("Installing...".into())).ok();
    install::install_reshade(data_dir, game_dir, &version, dll, arch)?;

    sender.output(Signal::InstallComplete { version }).ok();
    Ok(())
}
```

**Step 2: Implement `src/ui/shader_worker.rs`**

```rust
//! Async worker for cloning and updating shader repositories.

use std::path::PathBuf;

use relm4::{ComponentSender, Worker};

use crate::reshade::config::ShaderRepo;
use crate::reshade::shaders;

/// Input commands for the shader worker.
#[derive(Debug)]
pub enum Controls {
    /// Clone/update all given repos and rebuild the Merged directory.
    SyncAll {
        /// Repos to sync.
        repos: Vec<ShaderRepo>,
        /// Base directory containing `ReShade_shaders/`.
        data_dir: PathBuf,
        /// Repo names to exclude from the merge.
        disabled_repos: Vec<String>,
    },
}

/// Output signals from the shader worker.
#[derive(Debug)]
pub enum Signal {
    /// Currently syncing this repo.
    Progress(String),
    /// All repos synced successfully.
    Complete,
    /// A non-fatal error on one repo (sync continues).
    RepoError {
        /// The repo's local name.
        repo_name: String,
        /// Error message.
        error: String,
    },
    /// A fatal error stopping all syncing.
    Error(String),
}

/// Background shader sync worker.
pub struct ShaderWorker;

impl Worker for ShaderWorker {
    type Init = ();
    type Input = Controls;
    type Output = Signal;

    fn init(_: (), _sender: ComponentSender<Self>) -> Self {
        Self
    }

    fn update(&mut self, msg: Controls, sender: ComponentSender<Self>) {
        match msg {
            Controls::SyncAll { repos, data_dir, disabled_repos } => {
                let repos_dir = data_dir.join("ReShade_shaders");
                if let Err(e) = std::fs::create_dir_all(&repos_dir) {
                    sender.output(Signal::Error(e.to_string())).ok();
                    return;
                }
                for repo in &repos {
                    sender.output(Signal::Progress(format!("Syncing {}...", repo.local_name))).ok();
                    if let Err(e) = shaders::sync_repo(repo, &repos_dir) {
                        sender
                            .output(Signal::RepoError {
                                repo_name: repo.local_name.clone(),
                                error: e.to_string(),
                            })
                            .ok();
                    }
                }
                if let Err(e) = shaders::rebuild_merged(&repos_dir, &disabled_repos) {
                    sender.output(Signal::Error(e.to_string())).ok();
                    return;
                }
                sender.output(Signal::Complete).ok();
            }
        }
    }
}
```

**Step 3: Verify compile**

```bash
cargo check 2>&1 | head -30
```

**Step 4: Commit**

```bash
git add src/ui/install_worker.rs src/ui/shader_worker.rs
git commit -m "feat: add async install and shader worker components"
```

---

## Task 13: UI — Game Detail Pane

**Files:**
- Create: `src/ui/game_shader_overrides.rs`
- Create: `src/ui/game_detail.rs`

**Step 1: Implement `src/ui/game_shader_overrides.rs`**

```rust
//! Per-game shader repo override panel.

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, gtk};

use crate::reshade::config::{GlobalConfig, ShaderOverrides};

/// Per-game shader override panel.
pub struct GameShaderOverrides {
    overrides: ShaderOverrides,
    config: GlobalConfig,
}

/// Input messages.
#[derive(Debug)]
pub enum Controls {
    /// Update displayed data.
    SetData(GlobalConfig, ShaderOverrides),
}

/// Output signals.
#[derive(Debug)]
pub enum Signal {
    /// User toggled a repo override.
    OverrideChanged(ShaderOverrides),
}

#[relm4::component(pub)]
impl SimpleComponent for GameShaderOverrides {
    type Init = (GlobalConfig, ShaderOverrides);
    type Input = Controls;
    type Output = Signal;

    view! {
        #[name(list_box)]
        gtk::ListBox {
            set_selection_mode: gtk::SelectionMode::None,
            add_css_class: "boxed-list",
        }
    }

    fn init(
        (config, overrides): (GlobalConfig, ShaderOverrides),
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self { overrides, config };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, _sender: ComponentSender<Self>) {
        match msg {
            Controls::SetData(config, overrides) => {
                self.config = config;
                self.overrides = overrides;
            }
        }
    }
}
```

**Step 2: Implement `src/ui/game_detail.rs`**

```rust
//! Detail pane showing a single game's ReShade status and controls.

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use crate::reshade::game::{DllOverride, ExeArch, Game, InstallStatus};
use crate::fl;

/// Game detail pane model.
pub struct GameDetail {
    game: Option<Game>,
    progress_message: Option<String>,
}

/// Input messages for [`GameDetail`].
#[derive(Debug)]
pub enum Controls {
    /// Load a game into the pane.
    SetGame(Game),
    /// Clear the pane (no game selected).
    Clear,
    /// Show a progress message.
    SetProgress(String),
    /// Clear the progress message.
    ClearProgress,
    /// Mark the game as installed.
    MarkInstalled { version: String, dll: DllOverride, arch: ExeArch },
    /// Mark the game as uninstalled.
    MarkUninstalled,
}

/// Output signals from [`GameDetail`].
#[derive(Debug)]
pub enum Signal {
    /// User requested install with these parameters.
    Install { game_id: String, dll: DllOverride, arch: ExeArch },
    /// User requested uninstall.
    Uninstall { game_id: String, dll: DllOverride },
}

#[relm4::component(pub)]
impl SimpleComponent for GameDetail {
    type Init = ();
    type Input = Controls;
    type Output = Signal;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_margin_all: 24,
            set_spacing: 16,

            #[name(status_page)]
            adw::StatusPage {
                set_title: &fl!("select-a-game"),
                set_icon_name: Some("view-list-symbolic"),
                #[watch]
                set_visible: model.game.is_none(),
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 12,
                #[watch]
                set_visible: model.game.is_some(),

                #[name(game_name_label)]
                gtk::Label {
                    add_css_class: "title-1",
                    #[watch]
                    set_label: model.game.as_ref().map(|g| g.name.as_str()).unwrap_or(""),
                    set_xalign: 0.0,
                },

                #[name(game_path_label)]
                gtk::Label {
                    add_css_class: "caption",
                    #[watch]
                    set_label: model.game.as_ref()
                        .map(|g| g.path.to_string_lossy().into_owned())
                        .unwrap_or_default()
                        .as_str(),
                    set_xalign: 0.0,
                },

                #[name(progress_bar)]
                adw::Banner {
                    #[watch]
                    set_title: model.progress_message.as_deref().unwrap_or(""),
                    #[watch]
                    set_revealed: model.progress_message.is_some(),
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 8,

                    #[name(install_btn)]
                    gtk::Button {
                        set_label: &fl!("install"),
                        add_css_class: "suggested-action",
                        #[watch]
                        set_visible: model.game.as_ref()
                            .map(|g| !g.status.is_installed())
                            .unwrap_or(true),
                        connect_clicked[sender] => move |_| {
                            // Defaults; in a real impl these come from dropdowns
                            sender.output(Signal::Install {
                                game_id: String::new(),
                                dll: DllOverride::Dxgi,
                                arch: ExeArch::X86_64,
                            }).ok();
                        },
                    },

                    #[name(uninstall_btn)]
                    gtk::Button {
                        set_label: &fl!("uninstall"),
                        add_css_class: "destructive-action",
                        #[watch]
                        set_visible: model.game.as_ref()
                            .map(|g| g.status.is_installed())
                            .unwrap_or(false),
                        connect_clicked[sender] => move |_| {
                            sender.output(Signal::Uninstall {
                                game_id: String::new(),
                                dll: DllOverride::Dxgi,
                            }).ok();
                        },
                    },
                },
            },
        }
    }

    fn init(_: (), _root: Self::Root, _sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = Self { game: None, progress_message: None };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, _sender: ComponentSender<Self>) {
        match msg {
            Controls::SetGame(game) => self.game = Some(game),
            Controls::Clear => self.game = None,
            Controls::SetProgress(msg) => self.progress_message = Some(msg),
            Controls::ClearProgress => self.progress_message = None,
            Controls::MarkInstalled { dll, arch, .. } => {
                if let Some(game) = &mut self.game {
                    game.status = InstallStatus::Installed { dll, arch };
                }
            }
            Controls::MarkUninstalled => {
                if let Some(game) = &mut self.game {
                    game.status = InstallStatus::NotInstalled;
                }
            }
        }
    }
}
```

**Step 3: Verify compile**

```bash
cargo check 2>&1 | head -30
```

**Step 4: Commit**

```bash
git add src/ui/game_detail.rs src/ui/game_shader_overrides.rs
git commit -m "feat: add game detail pane and shader override panel"
```

---

## Task 14: UI — Add Game Dialog and Preferences

**Files:**
- Create: `src/ui/add_game_dialog.rs`
- Create: `src/ui/preferences.rs`

**Step 1: Implement `src/ui/add_game_dialog.rs`**

```rust
//! Dialog for manually adding a game by path.

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use crate::fl;

/// Add game dialog model.
pub struct AddGameDialog;

/// Input messages.
#[derive(Debug)]
pub enum Controls {
    /// Open the dialog attached to the given window.
    Open,
}

/// Output signals.
#[derive(Debug)]
pub enum Signal {
    /// User confirmed a game path.
    GamePathSelected(std::path::PathBuf),
}

#[relm4::component(pub)]
impl SimpleComponent for AddGameDialog {
    type Init = ();
    type Input = Controls;
    type Output = Signal;

    view! {
        #[name(dialog)]
        adw::Dialog {
            set_title: &fl!("add-game"),

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {},

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 24,
                    set_spacing: 12,

                    gtk::Label {
                        set_label: "Select the directory containing the game .exe",
                        set_wrap: true,
                        set_xalign: 0.0,
                    },

                    #[name(path_row)]
                    adw::ActionRow {
                        set_title: "Game directory",
                        set_subtitle: "(none selected)",

                        add_suffix = &gtk::Button {
                            set_label: "Browse",
                            set_valign: gtk::Align::Center,
                            connect_clicked[sender] => move |_| {
                                // File chooser wired in init
                                let _ = sender.input_sender();
                            },
                        },
                    },

                    gtk::Button {
                        set_label: &fl!("add-game"),
                        add_css_class: "suggested-action",
                    },
                },
            },
        }
    }

    fn init(_: (), _root: Self::Root, _sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = Self;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, _msg: Controls, _sender: ComponentSender<Self>) {}
}
```

**Step 2: Implement `src/ui/preferences.rs`**

```rust
//! Global preferences dialog — shader repos, update interval, INI toggle.

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use crate::reshade::config::GlobalConfig;
use crate::fl;

/// Preferences dialog model.
pub struct Preferences {
    config: GlobalConfig,
}

/// Input messages.
#[derive(Debug)]
pub enum Controls {
    /// Open the dialog.
    Open,
    /// Update displayed config.
    SetConfig(GlobalConfig),
}

/// Output signals.
#[derive(Debug)]
pub enum Signal {
    /// User saved changes.
    ConfigChanged(GlobalConfig),
}

#[relm4::component(pub)]
impl SimpleComponent for Preferences {
    type Init = GlobalConfig;
    type Input = Controls;
    type Output = Signal;

    view! {
        #[name(dialog)]
        adw::PreferencesDialog {
            set_title: &fl!("preferences"),

            adw::PreferencesPage {
                set_title: "Shaders",

                adw::PreferencesGroup {
                    set_title: "Shader Repositories",
                    set_description: Some("Repositories are cloned in order; first match wins on name collision."),

                    adw::SwitchRow {
                        set_title: "Merge shaders",
                        set_subtitle: "Combine all repos into a single directory",
                        #[watch]
                        set_active: model.config.merge_shaders,
                    },

                    adw::SwitchRow {
                        set_title: "Global ReShade.ini",
                        set_subtitle: "Share one config file across all games",
                        #[watch]
                        set_active: model.config.global_ini,
                    },
                },
            },

            adw::PreferencesPage {
                set_title: "Updates",

                adw::PreferencesGroup {
                    set_title: "Update Check",

                    adw::SpinRow {
                        set_title: "Check interval (hours)",
                        set_adjustment: &gtk::Adjustment::new(
                            model.config.update_interval_hours as f64,
                            1.0, 168.0, 1.0, 0.0, 0.0,
                        ),
                    },
                },
            },
        }
    }

    fn init(config: GlobalConfig, _root: Self::Root, _sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = Self { config };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Controls, _sender: ComponentSender<Self>) {
        match msg {
            Controls::Open => {}
            Controls::SetConfig(config) => self.config = config,
        }
    }
}
```

**Step 3: Verify compile**

```bash
cargo check 2>&1 | head -30
```

**Step 4: Commit**

```bash
git add src/ui/add_game_dialog.rs src/ui/preferences.rs
git commit -m "feat: add AddGameDialog and Preferences dialog"
```

---

## Task 15: Wire Everything Together

**Files:**
- Modify: `src/ui/window.rs` — embed GameList, GameDetail, workers, handle signals
- Modify: `src/main.rs` — pass AppState to Window

**Step 1: Update `src/main.rs` to load AppState**

Replace the `app.run::<Window>(())` line:

```rust
let state = Arc::new(RwLock::new(crate::reshade::app_state::AppState::load()));
let app = RelmApp::new(APPLICATION_ID);
app.run::<Window>(state);
```

Add at top of `main.rs`:
```rust
use std::sync::Arc;
use tokio::sync::RwLock;
```

**Step 2: Update `src/ui/window.rs`** to wire GameList + GameDetail

Replace the stub components with real component connectors. The Window `Init` type becomes `Shared<AppState>`:

```rust
type Init = crate::reshade::app_state::Shared<crate::reshade::app_state::AppState>;
```

Add child component connectors (see `docs/relm4-components.md` "Wiring" section for the exact connector pattern). Wire:
- `GameList::Signal::GameSelected(id)` → load game from state → `GameDetail::Controls::SetGame(...)`
- `GameDetail::Signal::Install { ... }` → `InstallWorker::Controls::Install { ... }`
- `InstallWorker::Signal::Progress(msg)` → `GameDetail::Controls::SetProgress(msg)`
- `InstallWorker::Signal::InstallComplete { version }` → update state + `GameDetail::Controls::MarkInstalled { ... }`
- `ShaderWorker::Signal::RepoError { repo_name, error }` → show `adw::Toast`

**Step 3: Verify compile and run**

```bash
cargo check 2>&1 | head -30
GSETTINGS_SCHEMA_DIR=./target/share/glib-2.0/schemas cargo run
```
Expected: window opens with sidebar and empty detail pane.

**Step 4: Commit**

```bash
git add src/ui/window.rs src/main.rs
git commit -m "feat: wire UI components together"
```

---

## Task 16: Steam Library Integration and Initial Game Load

**Files:**
- Modify: `src/ui/window.rs` — trigger Steam discovery on startup
- Modify: `src/reshade/app_state.rs` — merge Steam games on load

**Step 1: On `AppState::load()`, discover Steam games and merge with saved games**

In `app_state.rs`, after loading saved games:

```rust
// Merge Steam-discovered games that aren't already saved
let steam_games = crate::reshade::steam::discover_steam_games();
for sg in steam_games {
    if !games.iter().any(|g| g.path == sg.path) {
        games.push(sg);
    }
}
```

**Step 2: Pass game list to `GameList` on window init**

In `window.rs` init, read the game list from `AppState` and send to `GameList` via `Controls::SetGames`.

**Step 3: Test with real Steam library**

```bash
GSETTINGS_SCHEMA_DIR=./target/share/glib-2.0/schemas cargo run
```
Expected: sidebar shows installed Steam games.

**Step 4: Commit**

```bash
git add src/reshade/app_state.rs src/ui/window.rs
git commit -m "feat: load Steam games on startup"
```

---

## Task 17: CLAUDE.md and Final Docs

**Files:**
- Create: `CLAUDE.md`
- Modify: `docs/README.md` — add gnome-iris section

**Step 1: Create `CLAUDE.md`**

```markdown
# gnome-iris

GTK4 + Relm4 + Rust GNOME app for managing ReShade under Wine/Proton.

## Commands

```bash
# Build
cargo build

# Run (schema dir required in dev)
GSETTINGS_SCHEMA_DIR=./target/share/glib-2.0/schemas cargo run

# Test
cargo test

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt
```

## Architecture

- `src/reshade/` — domain layer, pure Rust, zero GTK imports, fully unit tested
- `src/ui/` — Relm4 components, GTK only here

## Key Conventions

- Input message types: `Controls`, output: `Signal`
- `Shared<T> = Arc<RwLock<T>>` for all shared state
- All `pub` items need `///` doc comments; modules need `//!`
- Clippy denies all lint groups — must be 100% clean
- Errors: `anyhow::Result<T>`; logging: `log` crate
- Commit style: `type(scope): message`

## Gotchas

- GSettings schemas must be compiled before running: `build.rs` handles this
- `GSETTINGS_SCHEMA_DIR=./target/share/glib-2.0/schemas` needed in dev
- relm4 is pinned to a specific git rev — do not update without testing
- Domain tests use `tempfile` crate for isolated filesystem operations
```

**Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: add CLAUDE.md"
```

---

**Plan complete and saved to `docs/plans/2026-03-07-gnome-iris-implementation.md`.**

Two execution options:

**1. Subagent-Driven (this session)** — I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** — Open a new session with `executing-plans`, batch execution with checkpoints

Which approach?
