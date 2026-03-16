# gnome-iris

GTK4 + Relm4 + Rust GNOME app for managing ReShade under Wine/Proton on Linux.

## Commands

```bash
# Check (no GTK display needed)
mise exec -- cargo check

# Build
mise exec -- cargo build

# Run (schema dir required in dev)
GSETTINGS_SCHEMA_DIR=./target/share/glib-2.0/schemas mise exec -- cargo run

# Test (domain layer only — no GTK)
mise exec -- cargo test

# Format
mise exec -- cargo fmt

# Lint
mise exec -- cargo clippy
```

## Architecture

- `src/reshade/` — domain layer, **pure Rust, zero GTK imports**, fully unit tested
- `src/ui/` — Relm4 components, GTK only here
- Domain is tested; UI is smoke-tested manually

## Key Conventions

- Input message types: `Controls`, output: `Signal`
- `Shared<T> = Arc<RwLock<T>>` for shared state (defined in `app_state.rs`)
- All `pub` items need `///` doc comments; modules need `//!`
- Add `#[allow(missing_docs)]` before `#[relm4::component(pub)]` — macro generates public `Widgets` structs
- Clippy denies all lint groups — must be 100% clean
- Errors: `anyhow::Result<T>`; logging: `log` crate
- Commit style: `type(scope): message`

## Gotchas

- **GSettings schema**: must be compiled before running — `build.rs` handles it automatically
- **`GSETTINGS_SCHEMA_DIR`**: set to `./target/share/glib-2.0/schemas` when running in dev
- **Rust toolchain**: managed by mise (`mise.toml` pins stable 1.94.0) — no nightly features
- **relm4 git pin**: pinned to `baa1c23ab35e3b8c4117714042671f7ed02aeabb` — don't update without testing
- **`.ftl` filename**: `fluent_language_loader!()` derives the filename from the crate name (`gnome-iris` → `gnome_iris.ftl`) — one `gnome_iris.ftl` per language directory; legacy `iris.ftl` files have been removed
- **`connect_clicked[sender]`**: in `view!` macro, requires `sender` (not `_sender`) in `init()` signature
- **`adw::PreferencesDialog`**: doesn't support relm4 container extensions — build dialog tree manually in `init()` rather than via the `view!` macro
- **Domain tests**: use `tempfile` crate for isolated filesystem operations

## Data Layout

App data stored at `$XDG_DATA_HOME/iris/` (typically `~/.local/share/iris/`):

```
~/.local/share/iris/
├── config.json               # GlobalConfig
├── games.json                # Saved games list
├── LVERS                     # Last known ReShade version
├── LASTUPDATED               # Timestamp of last update check
├── reshade/
│   ├── latest -> 6.x.x/      # Symlink to current version
│   └── 6.x.x/
│       ├── ReShade32.dll
│       └── ReShade64.dll
├── ReShade_shaders/
│   ├── Merged/Shaders/       # Symlinked merged shaders
│   └── <repo-name>/          # Cloned shader repos
├── d3dcompiler_47.dll.32
└── d3dcompiler_47.dll.64
```
