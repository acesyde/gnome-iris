# gnome-iris Architecture Improvement Plan

## Context

This document captures architectural issues found by analysing the codebase without consulting the docs/ folder.
The project is a GTK4 + Relm4 + Rust GNOME app (~4 000 lines). The domain layer (`src/reshade/`) is clean and
well-tested. Most issues are in the UI orchestration layer and in domain-layer duplication / missing abstractions.

The goal is not a rewrite — it is a set of targeted, prioritised improvements that make the codebase easier to
extend and maintain over time.

---

## Improvement Items

### 1. Deduplicate `parse_version_key()` — **Quick win / high value** ✅ Done

**Problem:** `parse_version_key()` is defined identically in both `src/reshade/cache.rs` and
`src/reshade/reshade.rs`. Any fix or change must be applied in two places; they will inevitably drift.

**Fix:** Move the function into a new `src/reshade/version.rs` module (or into `reshade.rs` and re-export from
`cache.rs`). Both callers import the single canonical copy.

**Files:** `src/reshade/cache.rs`, `src/reshade/reshade.rs`

---

### 2. Extract domain path constants — **Quick win / medium value** ✅ Done

**Problem:** Magic string literals for paths and file names are scattered across modules:

- `"Merged"` — in `shaders.rs`
- `"ReShade_shaders"` — in `shaders.rs` and `install.rs`
- `"LVERS"`, `"LASTUPDATED"` — in `cache.rs`
- `"reshade_state.json"`, `"config.json"`, `"games.json"` — in `app_state.rs` and `cache.rs`

**Fix:** Create `src/reshade/paths.rs` with a single `IrisPaths` struct or module-level `pub const` declarations
that all modules import. No path string should appear in more than one place.

**Files:** `src/reshade/shaders.rs`, `src/reshade/install.rs`, `src/reshade/cache.rs`, `src/reshade/app_state.rs`

---

### 3. Introduce service traits for the domain layer — **Medium effort / high long-term value** ✅ Done

**Problem:** All domain operations are free functions. The UI calls them directly, which:

- Makes the UI layer untestable in isolation.
- Makes it impossible to swap a mock or alternative implementation.
- Couples the UI to concrete modules rather than abstractions.

**Fix:** Define traits in `src/reshade/`:

```rust
pub trait ReShadeProvider {
    async fn fetch_latest_version(&self) -> Result<String>;
    async fn download_and_extract(&self, version: &str, addon: bool) -> Result<()>;
    fn list_installed_versions(&self) -> Result<Vec<String>>;
}

pub trait GameRepository {
    fn games(&self) -> &[Game];
    fn save_games(&mut self, games: &[Game]) -> Result<()>;
}
```

Provide a `DefaultReShadeProvider` and `DefaultGameRepository` that wrap the current free functions.
Workers and Window receive the trait object (or generic parameter) rather than calling functions directly.

This is groundwork for future unit-testing of UI handlers.

**Files:** new `src/reshade/services.rs`, `src/ui/install_worker.rs`, `src/ui/shader_worker.rs`

---

### 4. Replace raw-string progress messages with a typed `Progress` enum — **Medium effort / medium value** ✅ Done

**Problem:** `InstallWorker` and `ShaderWorker` both emit `Signal::Progress(String)`. The string content is
ad-hoc and checked nowhere by the compiler. Adding a new progress stage means grepping for string literals.

**Fix:** Define a `ProgressEvent` enum in the domain layer or in a shared `ui/worker_types.rs`:

```rust
pub enum ProgressEvent {
    Downloading { version: String, bytes_total: Option<u64>, bytes_done: u64 },
    Extracting { version: String },
    SyncingRepo { name: String },
    Installing,
    Done,
}
```

Workers emit `Signal::Progress(ProgressEvent)`. The detail pane converts the enum to a localised string for
display. This decouples progress semantics from presentation.

**Files:** `src/ui/install_worker.rs`, `src/ui/shader_worker.rs`, `src/ui/game_detail.rs`

---

### 5. Surface persistence errors to the user — **Medium effort / high UX value** ✅ Done

**Problem:** Every `AppState::save()` call swallows failures with `log::error!()` only:

```rust
if let Err(e) = model.app_state.save() {
    log::error!("Failed to save...: {e}");
}
```

The user has no indication that their config or game list was not persisted. Data loss is silent.

**Fix:** Propagate save errors to the Window's `ToastOverlay`. Introduce a helper in `window/mod.rs`:

```rust
fn save_or_toast(model: &mut Window, sender: &Sender<Controls>) {
    if let Err(e) = model.app_state.save() {
        sender.input(Controls::ShowToast(format!("Failed to save: {e}")));
    }
}
```

All four call sites in `panel_games.rs` and `panel_preferences.rs` use this helper.

**Files:** `src/ui/window/mod.rs`, `src/ui/window/panel_games.rs`, `src/ui/window/panel_preferences.rs`

---

### 6. Slim down the Window `Controls` enum — **Medium effort / high maintainability value** ✅ Done

**Problem:** `window/mod.rs` has **64+ Controls variants**. The `update()` function routes them all. This makes
the root component a god object that is hard to navigate and increasingly risky to extend.

**Fix:** Group variants by domain using nested enums, then flatten in `update()` with a thin match arm:

```rust
pub enum Controls {
    Games(GamesMsg),
    Shaders(ShadersMsg),
    Prefs(PrefsMsg),
    Toast(String),
}
```

Each panel handler file becomes responsible for its own message type. `window/mod.rs::update()` dispatches in
~4 lines instead of 64. New features add a variant to the appropriate sub-enum, not to the root.

**Files:** `src/ui/window/mod.rs`, `src/ui/window/panel_games.rs`, `src/ui/window/panel_preferences.rs`,
`src/ui/window/panel_shaders.rs`

---

### 7. Move `catalog.rs` static data to an embedded data file — **Low effort / medium evolvability value** ✅ Done

**Problem:** `src/reshade/catalog.rs` is 406 lines of hand-written Rust `CatalogEntry` structs for 45 shader
repos. Adding or updating a repo requires a Rust recompile and a code change.

**Fix:** Encode the catalog as `data/catalog.toml` (or JSON), embed it with `rust-embed` (already a dependency),
and deserialise at startup with `serde`. The `CatalogEntry` type stays; only the source of truth moves.

**Files:** `src/reshade/catalog.rs` → `data/catalog.toml` + thin loader, `build.rs` (no change needed — rust-embed handles it)

---

### 8. Add domain-layer integration tests — **Medium effort / high confidence value** ✅ Done

**Problem:** Unit tests cover individual functions in isolation but there are no end-to-end domain tests that
exercise a full flow (e.g., download → extract → install → detect_status → uninstall).

**Fix:** Add `tests/` at the crate root (Rust integration test convention). One file per major flow:

- `tests/install_flow.rs` — download a mock zip, extract, install symlinks, detect status, uninstall
- `tests/shader_sync.rs` — clone a bare test repo, rebuild merged, verify symlinks
- `tests/app_state.rs` — load/save roundtrip with full AppState

Use `tempfile` (already a dev-dependency) for isolation.

**Files:** new `tests/install_flow.rs`, `tests/shader_sync.rs`, `tests/app_state.rs`

---

### 9. Defer `detect_install_status()` out of the synchronous UI init path — **Low effort / UX value** ✅ Done

**Problem:** `Window::init()` calls `detect_install_status()` synchronously for every game before the window
appears. This does blocking filesystem I/O on the main thread, making startup lag proportional to the number
of games.

**Fix:** Move the per-game status detection into the startup async task that already fetches the latest version.
The Window shows games immediately with `InstallStatus::NotInstalled` as a placeholder, then a
`Controls::GameStatusDetected { id, status }` message updates each row once detection completes.

**Files:** `src/ui/window/mod.rs`

---

## Priority Order

| #   | Item                              | Effort | Value  | Do first? |
| --- | --------------------------------- | ------ | ------ | --------- |
| 1   | Deduplicate `parse_version_key()` | XS     | High   | ✅ Yes    |
| 2   | Extract path constants            | S      | Medium | ✅ Yes    |
| 5   | Surface save errors to user       | S      | High   | ✅ Done   |
| 9   | Async startup detection           | S      | Medium | ✅ Done   |
| 6   | Slim down `Controls` enum         | M      | High   | ✅ Done   |
| 4   | Typed `Progress` enum             | M      | Medium | ✅ Yes    |
| 7   | Catalog as data file              | M      | Medium | ✅ Yes    |
| 8   | Integration tests                 | M      | High   | ✅ Done   |
| 3   | Service traits                    | L      | High   | ✅ Done   |

---

## Verification

For each item, verify with:

```bash
mise exec -- cargo check          # type-checks without GTK display
mise exec -- cargo test           # domain unit tests
mise exec -- cargo clippy         # must be 100% clean (all groups denied)
mise exec -- cargo fmt --check    # formatting
```

Manual smoke test (requires display):

```bash
GSETTINGS_SCHEMA_DIR=./target/share/glib-2.0/schemas mise exec -- cargo run
```
