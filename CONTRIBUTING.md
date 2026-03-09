# Contributing to Iris

## Requirements

- [mise](https://mise.jdx.dev/) — manages the pinned Rust toolchain (`stable 1.94.0`)
- Internet access for the initial ReShade download and shader repo cloning

## Building

```bash
# Install the pinned toolchain
mise install

# Check (no display required)
mise exec -- cargo check

# Build
mise exec -- cargo build

# Run (schema dir must be set in dev)
GSETTINGS_SCHEMA_DIR=./target/share/glib-2.0/schemas mise exec -- cargo run
```

The `build.rs` script compiles the GSettings schema and icon resources automatically.

## Testing

```bash
mise exec -- cargo test   # domain layer only — no GTK required
```

## Data layout

All application data is stored under `$XDG_DATA_HOME/iris/` (typically `~/.local/share/iris/`):

```
~/.local/share/iris/
├── config.json               # Global settings
├── games.json                # Saved game list
├── reshade_state.json        # Version cache and update timestamps
├── reshade/
│   ├── 6.x.x/               # Standard variant DLLs
│   │   ├── ReShade32.dll
│   │   └── ReShade64.dll
│   └── 6.x.x-Addon/         # Addon Support variant DLLs
│       ├── ReShade32.dll
│       └── ReShade64.dll
├── ReShade_shaders/
│   ├── Merged/Shaders/       # Merged shader symlinks (optional)
│   └── <repo-name>/          # Cloned shader repositories
├── d3dcompiler_47.dll.32
└── d3dcompiler_47.dll.64
```

## Architecture

```
src/
├── reshade/    # Domain layer — pure Rust, no GTK imports, fully unit tested
│   ├── reshade.rs      # Version fetching, download, extraction
│   ├── install.rs      # Game install / uninstall logic
│   ├── cache.rs        # Persisted version state
│   ├── shaders.rs      # Shader repo sync and merging
│   ├── steam.rs        # Steam library discovery
│   ├── catalog.rs      # Curated shader repo list
│   ├── config.rs       # GlobalConfig and ShaderRepo types
│   ├── game.rs         # Game model and install status
│   └── app_state.rs    # Top-level persisted state
└── ui/         # Relm4 components — GTK only here
    ├── window.rs               # Root window, wires all components
    ├── preferences.rs          # Settings and version management page
    ├── game_list.rs            # Left-hand game list
    ├── game_detail.rs          # Install / uninstall pane
    ├── shader_catalog.rs       # Shader repo browser
    ├── install_worker.rs       # Async download / install worker
    └── shader_worker.rs        # Async shader sync worker
```

## Code conventions

- `src/reshade/` is the domain layer — **no GTK imports**, fully unit tested
- `src/ui/` is the UI layer — Relm4 components, GTK only here
- Input message types: `Controls`, output: `Signal`
- All `pub` items need `///` doc comments; modules need `//!`
- Clippy denies all lint groups — must be 100% clean (`cargo clippy`)
- Errors: `anyhow::Result<T>`; logging: `log` crate
- Commit style: `type(scope): message`
