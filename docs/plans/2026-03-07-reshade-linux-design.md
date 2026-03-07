# gnome-iris Design Document

**Date:** 2026-03-07
**Status:** Approved

## Overview

gnome-iris is a GTK4 + Relm4 + Rust GNOME desktop application that replaces the
`reshade-steam-proton` bash script. It provides a GUI for installing and managing
ReShade for Windows games running under Wine/Proton on Linux.

**Application ID:** `org.gnome.Iris`

## Scope

Curated migration from the bash script:

**Included:**
- Install/uninstall ReShade for DirectX (8/9/11) and OpenGL games
- Automatic exe architecture detection (32/64-bit) and DLL selection
- Shader repository management (clone, update, merge)
- Global INI management
- Preset file linking
- Auto-update checking (configurable interval, default 4h)
- Steam library discovery + manual game path fallback
- Per-game shader repo overrides (global defaults + per-game opt-out)

**Excluded:**
- Vulkan/Wine registry support (noted as non-functional in original script)

## Architecture

**Approach:** Full Rust rewrite, monolithic single binary. Clean domain/UI separation
per the scaffolding docs — domain layer has zero GTK imports.

### Directory Layout

```
src/
├── main.rs
├── localization.rs
├── reshade/              # Domain layer — pure Rust, no GTK
│   ├── mod.rs
│   ├── cache.rs          # Version/update tracking (LVERS, LASTUPDATED)
│   ├── config.rs         # GlobalConfig + per-game config, serde_json
│   ├── game.rs           # Game struct, GameSource, InstallStatus
│   ├── install.rs        # Install/uninstall logic, symlinks, d3dcompiler
│   ├── reshade.rs        # ReShade version fetching and extraction
│   ├── shaders.rs        # Repo clone/update/merge (git2)
│   └── steam.rs          # Steam library discovery (libraryfolders.vdf)
└── ui/
    ├── mod.rs
    ├── window.rs          # Root SimpleComponent, OverlaySplitView
    ├── game_list.rs       # Sidebar: Steam games + manual entries
    ├── game_detail.rs     # Detail pane: status, install/uninstall, config
    ├── game_shader_overrides.rs  # Per-game repo opt-out panel
    ├── add_game_dialog.rs # File chooser for manual game paths
    ├── shader_worker.rs   # AsyncComponent: git clone/update
    ├── install_worker.rs  # AsyncComponent: ReShade download + extract
    ├── preferences.rs     # Global shader repos, INI toggle, update interval
    └── about.rs
```

### Persistent Storage

All files under `$XDG_DATA_HOME/iris/` (default `~/.local/share/iris/`):

```
~/.local/share/iris/
├── config.json               # GlobalConfig
├── games.json                # All known games
├── reshade/
│   ├── latest -> 6.x.x/      # symlink to current version
│   └── 6.x.x/
│       ├── ReShade32.dll
│       └── ReShade64.dll
├── ReShade_shaders/
│   ├── Merged/               # Symlinked merged shaders
│   └── <repo-name>/          # Cloned repos
├── External_shaders/         # User-dropped custom shaders
├── d3dcompiler_47.dll.32
├── d3dcompiler_47.dll.64
└── ReShade.ini               # Global INI (when enabled)
```

## Data Model

```rust
type Shared<T> = Arc<RwLock<T>>;

struct AppState {
    games: Vec<Game>,
    reshade_version: Option<String>,
    global_config: GlobalConfig,
}

struct Game {
    id: String,                      // SHA-512 of canonical path
    name: String,
    path: PathBuf,
    source: GameSource,
    status: InstallStatus,
    shader_overrides: ShaderOverrides,
}

enum GameSource {
    Steam { app_id: u32 },
    Manual,
}

enum InstallStatus {
    NotInstalled,
    Installed { dll: DllOverride, arch: ExeArch },
}

enum DllOverride { D3d8, D3d9, D3d11, Ddraw, Dinput8, Dxgi, OpenGl32 }
enum ExeArch { X86, X86_64 }

struct GlobalConfig {
    shader_repos: Vec<ShaderRepo>,
    global_ini: bool,
    merge_shaders: bool,
    update_interval_hours: u64,      // default: 4
}

struct ShaderRepo {
    url: String,
    local_name: String,
    branch: Option<String>,
    enabled_by_default: bool,
}

struct ShaderOverrides {
    disabled_repos: Vec<String>,     // local_names of disabled repos
}
```

**Conventions:**
- Component input types named `Controls`
- Component output types named `Signal`
- `Shared<T> = Arc<RwLock<T>>` for all shared state

## UI Layout

Root window uses `adw::OverlaySplitView` (sidebar collapses on narrow screens):

```
┌─────────────────────────────────────────────────────┐
│  [≡] Iris                          [•••]            │  ← HeaderBar
├──────────────┬──────────────────────────────────────┤
│              │                                       │
│  Steam Games │  Game Detail Pane                    │
│  ──────────  │  ─────────────────────────────────── │
│  ● Game A ✓  │  Game A                              │
│  ● Game B    │  ~/.steam/.../.../gameA/              │
│  ● Game C ✓  │                                       │
│              │  Status: ✓ Installed (dxgi, 64-bit)  │
│  Manual      │  ReShade 6.1.0                       │
│  ──────────  │                                       │
│  + Add Game  │  [ Uninstall ]  [ Reinstall ]        │
│              │                                       │
│              │  DLL Override: [dxgi ▾]  Arch: [64▾] │
│              │  Preset: [None ▾]                    │
│              │                                       │
│              │  Shaders ────────────────────────── │
│              │  ☑ reshade-shaders (global default)  │
│              │  ☑ martymc-shaders (global default)  │
│              │  ☐ sweetfx-shaders (disabled here)   │
└──────────────┴──────────────────────────────────────┘
```

## Components

| Component | Trait | Responsibility |
|---|---|---|
| `Window` | `SimpleComponent` | Root, hosts split view, owns `AppState` |
| `GameList` | `SimpleComponent` | Sidebar, emits `Signal::GameSelected(id)` |
| `GameDetail` | `Component` | Detail pane, install/uninstall commands |
| `AddGameDialog` | `SimpleComponent` | File chooser for manual game paths |
| `ShaderWorker` | `AsyncComponent` | Git clone/update in background |
| `InstallWorker` | `AsyncComponent` | Download ReShade, extract, symlink |
| `Preferences` | `SimpleComponent` | Global shader repos, INI toggle, update interval |

## Crate Replacements

| Bash dependency | Rust crate |
|---|---|
| `curl` | `reqwest` (async) |
| `7z` / zip extraction | `sevenz-rust` + `zip` |
| `git clone/pull` | `git2` |
| VDF parsing | `keyvalues-parser` |
| Symlink management | `std::os::unix::fs` |

## Error Handling

- All domain functions return `anyhow::Result<T>`
- Async workers emit `Signal::Error(String)` on failure → `adw::AlertDialog`
- Non-fatal warnings (e.g. shader repo update failed) → `adw::Toast`

## Testing

- Domain layer unit tested with `cargo test`
- `steam.rs` and `config.rs` tested against fixture files in `tests/fixtures/`
- `install.rs` tested with temp directories (`tempfile` crate)
- UI layer: manual smoke testing (standard Relm4 practice)
