# Game List — "Update Available" Pill

**Date:** 2026-03-14
**Status:** Approved

## Problem

When a game has ReShade installed but the installed version is older than the latest known version, the user has no visual cue on the game list. They must navigate into the game detail to discover an update is possible.

## Goal

Show a small pill badge on a game row when the game's installed ReShade version is behind the latest known version.

## Approach

Store `latest_version: Option<String>` in `Window`. Extend the existing `SetGameStatus` message in `GameList` to carry `latest_version`. `GameList` compares installed vs latest per row and shows/hides a pill widget accordingly.

## Version String Format

- `latest_version` (from `LatestVersionFetched`) carries a `v`-prefixed string, e.g. `"v6.3.0"`, as returned by `fetch_latest_version()` from the GitHub tags API.
- `InstallStatus::Installed { version, .. }` stores a bare string, e.g. `"6.3.0"` (no `v` prefix), as stored by the install worker.
- The `is_version_outdated` helper strips a leading `v` from both sides before comparing, normalising the difference.

## Data Flow

1. **`Window`** gains a `latest_version: Option<String>` field (initially `None`).
2. When `Controls::LatestVersionFetched(v)` is handled in `panel_preferences`:
   - Change `handle_latest_version_fetched` signature from `&Window` to `&mut Window` (matching all other mutating handlers in that file, e.g. `handle_config_changed`).
   - Store `v` in `model.latest_version`.
   - Forward to `Preferences` as today.
   - Re-emit `game_list::Controls::SetGameStatus` for every game in **`model.games`** (not `model.app_state.games` — `model.games` includes Steam-discovered games that are never stored in `app_state`) that has an `InstallStatus::Installed { version: Some(_), .. }`, passing the newly known `latest_version`. This ensures pills appear correctly even if the version arrives after the initial render.
   - **Borrow checker note:** You cannot iterate `model.games` (immutable borrow) and also call `model.game_list.emit(...)` (mutable borrow) in the same loop body — both fields live on the same `&mut Window`. Resolve by first collecting the needed data into a local `Vec<(String, Option<String>)>` (game ID + installed version), then iterating that local vec to emit.
3. **`game_list::Controls::SetGameStatus`** gains a new field:
   ```rust
   SetGameStatus {
       id: String,
       version: Option<String>,
       latest_version: Option<String>,  // NEW
   }
   ```
4. All call sites that emit `SetGameStatus` pass `model.latest_version.clone()`:
   - `panel_games::handle_install_complete`
   - `panel_games::handle_uninstall_complete`
   - The new broadcast loop in `panel_preferences::handle_latest_version_fetched`
5. **`AddGame` path:** `handle_game_added` in `panel_games.rs` must emit a follow-up `SetGameStatus` after `AddGame` **only if `model.latest_version.is_some()`**. This ensures a manually added game (which might already have ReShade installed) shows the pill immediately rather than waiting for the next `LatestVersionFetched`.

## Domain Helper

Place the version comparison logic in the **domain layer** as a free function in `src/reshade/reshade.rs` (zero GTK imports, unit-testable):

```rust
/// Returns `true` if `installed` is strictly older than `latest`.
///
/// Both strings may optionally carry a leading `v` (e.g. `"v6.3.0"` or `"6.3.0"`).
/// Returns `false` if either string cannot be parsed as a semver version.
pub fn is_version_outdated(installed: &str, latest: &str) -> bool {
    use semver::Version;
    let strip = |s: &str| s.strip_prefix('v').unwrap_or(s);
    match (Version::parse(strip(installed)), Version::parse(strip(latest))) {
        (Ok(i), Ok(l)) => i < l,
        _ => false,
    }
}
```

Add unit tests for this function alongside other domain tests (use `tempfile` pattern if needed, though this function needs no filesystem).

## GameList Changes

### State

Two new `HashMap`s alongside the existing row maps:

```rust
auto_update_pills: HashMap<String, gtk::Label>,
manual_update_pills: HashMap<String, gtk::Label>,
```

### `build_game_row` return type

Change `build_game_row` to return a tuple:

```rust
fn build_game_row(game: &Game, sender: &ComponentSender<GameList>) -> (adw::ActionRow, gtk::Label)
```

The `gtk::Label` is the pill widget. All call sites (in `init` and `update`) are updated to destructure the tuple and insert the pill into the appropriate map.

### Widget

In `build_game_row`, a `gtk::Label` pill is created and added as a suffix (inserted before the chevron):

```rust
let pill = gtk::Label::new(Some(&fl!("update-available")));
pill.add_css_class("pill");
pill.add_css_class("accent");
pill.set_visible(false);  // hidden by default
row.add_suffix(&pill);
```

The `accent` class is standard Adwaita (highlighted text colour). The `pill` class is a custom CSS class defined in the application stylesheet (see CSS section below).

### Visibility Logic

In `update` for `SetGameStatus`:
- Look up the row's pill from the maps.
- If both `version` and `latest_version` are `Some`, call `reshade::reshade::is_version_outdated` and set pill visibility.
- Otherwise hide the pill.

### `SetGames` handler

`Controls::SetGames(Vec<Game>)` has **no call sites** in the current codebase — it is declared but never emitted. No pill handling is required for it. If it gains call sites in the future, the handler must be updated to also refresh pill visibility.

### `RemoveGame` cleanup

When `Controls::RemoveGame(id)` is handled, also remove the entry from `auto_update_pills` or `manual_update_pills` (whichever contains it) to prevent a map memory leak.

## CSS Loading

Relm4 is already compiled with the `css` feature, which provides `relm4::set_global_css(css: &str)`. In `main.rs`, call it before `app.run::<Window>(())`:

```rust
relm4::set_global_css("label.pill { border-radius: 9999px; padding: 2px 8px; }");
```

No GResource changes or extra files required.

## i18n

Add to **all** locale FTL files. Use the English string as a placeholder for non-English locales (translators can update later). The `fl!()` macro panics at startup if a key is missing from any loaded locale.

Files to update:
- `i18n/en-US/gnome_iris.ftl`
- `i18n/en-US/iris.ftl`
- `i18n/es-ES/gnome_iris.ftl`
- `i18n/es-ES/iris.ftl`
- `i18n/fr-FR/gnome_iris.ftl`
- `i18n/fr-FR/iris.ftl`
- `i18n/it-IT/gnome_iris.ftl`
- `i18n/it-IT/iris.ftl`
- `i18n/pt-BR/gnome_iris.ftl`
- `i18n/pt-BR/iris.ftl`

String to add to each file:

```
update-available = Update available
```

("Update available" is used instead of the terse "Update" for screen-reader accessibility.)

## Edge Cases

| Scenario | Behaviour |
|---|---|
| `latest_version` not yet fetched | Pill hidden (no false positives) |
| Game not installed | No pill (subtitle shows "not installed") |
| Installed version == latest | No pill |
| Semver parse failure | No pill (fail-safe) |
| Version removed from Preferences cache | No effect on pill (pill tracks game install vs latest, not cached versions) |
| Game manually added while `latest_version` known | Follow-up `SetGameStatus` emitted from `handle_game_added` |
| Game row removed while pill visible | Pill map entry removed alongside row map entry in `RemoveGame` handler |

## Semver dependency

Add `semver` to `Cargo.toml` unconditionally — it is a stable, widely available crate with no build-time requirements. Do not use a fallback split-based implementation; `semver` is the correct tool and avoids Clippy issues with manual parsing under the project's `deny(all lint groups)` configuration.

```toml
semver = "1"
```

## Files to Change

| File | Change |
|---|---|
| `Cargo.toml` | Add `semver = "1"` |
| `src/reshade/reshade.rs` | Add `is_version_outdated` free function + unit tests |
| `src/ui/window/mod.rs` | Add `latest_version: Option<String>` field |
| `src/ui/window/panel_preferences.rs` | Change to `&mut Window`; store latest version; broadcast `SetGameStatus` over `model.games` |
| `src/ui/window/panel_games.rs` | Pass `latest_version` to `SetGameStatus` calls; emit follow-up `SetGameStatus` in `handle_game_added` |
| `src/ui/game_list.rs` | Extend `SetGameStatus`; change `build_game_row` return to tuple; add pill maps; show/hide pill in `SetGameStatus` handler; clean up pill map in `RemoveGame` handler |
| `src/main.rs` | Call `relm4::set_global_css(...)` to define `label.pill` style |
| `i18n/en-US/gnome_iris.ftl` | Add `update-available` key |
| `i18n/en-US/iris.ftl` | Add `update-available` key |
| `i18n/es-ES/gnome_iris.ftl` | Add `update-available` key (English placeholder) |
| `i18n/es-ES/iris.ftl` | Add `update-available` key (English placeholder) |
| `i18n/fr-FR/gnome_iris.ftl` | Add `update-available` key (English placeholder) |
| `i18n/fr-FR/iris.ftl` | Add `update-available` key (English placeholder) |
| `i18n/it-IT/gnome_iris.ftl` | Add `update-available` key (English placeholder) |
| `i18n/it-IT/iris.ftl` | Add `update-available` key (English placeholder) |
| `i18n/pt-BR/gnome_iris.ftl` | Add `update-available` key (English placeholder) |
| `i18n/pt-BR/iris.ftl` | Add `update-available` key (English placeholder) |
