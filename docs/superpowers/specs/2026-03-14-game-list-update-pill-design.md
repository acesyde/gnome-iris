# Game List — "Update Available" Pill

**Date:** 2026-03-14
**Status:** Approved

## Problem

When a game has ReShade installed but the installed version is older than the latest known version, the user has no visual cue on the game list. They must navigate into the game detail to discover an update is possible.

## Goal

Show a small pill badge on a game row when the game's installed ReShade version is behind the latest known version.

## Approach

Store `latest_version: Option<String>` in `Window`. Extend the existing `SetGameStatus` message in `GameList` to carry `latest_version`. `GameList` compares installed vs latest per row and shows/hides a pill widget accordingly.

## Data Flow

1. **`Window`** gains a `latest_version: Option<String>` field (initially `None`).
2. When `Controls::LatestVersionFetched(v)` is handled in `panel_preferences`:
   - Store `v` in `model.latest_version`.
   - Forward to `Preferences` as today.
   - Re-emit `game_list::Controls::SetGameStatus` for every game in `model.games` that has an `InstallStatus::Installed { version: Some(_), .. }`, passing the newly known `latest_version`. This ensures pills appear correctly even if the version arrives after the initial render.
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

## GameList Changes

### State

Two new `HashMap`s alongside the existing row maps:

```rust
auto_update_pills: HashMap<String, gtk::Label>,
manual_update_pills: HashMap<String, gtk::Label>,
```

### Widget

In `build_game_row`, a `gtk::Label` pill is created and added as a suffix (inserted before the chevron):

```rust
let pill = gtk::Label::new(Some(&fl!("update-available")));
pill.add_css_class("pill");
pill.add_css_class("accent");
pill.set_visible(false);  // hidden by default
row.add_suffix(&pill);
```

The pill widget reference is stored in the appropriate `*_update_pills` map.

### Visibility Logic

A helper function `should_show_pill(installed: &str, latest: &str) -> bool`:
- Strip leading `v` from both strings.
- Parse both as `semver::Version`.
- Return `installed_ver < latest_ver`.
- On any parse failure, return `false` (fail-safe: no pill).

In `update` for `SetGameStatus`:
- Look up the row's pill from the maps.
- If both `version` and `latest_version` are `Some`, call `should_show_pill` and set pill visibility.
- Otherwise hide the pill.

## i18n

Add to `i18n/en-US/gnome_iris.ftl` (and `iris.ftl`):

```
update-available = Update
```

## Edge Cases

| Scenario | Behaviour |
|---|---|
| `latest_version` not yet fetched | Pill hidden (no false positives) |
| Game not installed | No pill (subtitle shows "not installed") |
| Installed version == latest | No pill |
| Semver parse failure | No pill (fail-safe) |
| Version removed from Preferences cache | No effect on pill (pill tracks game install vs latest, not cached versions) |

## Semver dependency

Check if `semver` is already in `Cargo.toml`. If not, add it. If the crate is unavailable, fall back to a simple split-based numeric comparison (`split('.').map(parse::<u64>)`).

## Files to Change

| File | Change |
|---|---|
| `src/ui/window/mod.rs` | Add `latest_version: Option<String>` field |
| `src/ui/window/panel_preferences.rs` | Store latest version, broadcast `SetGameStatus` to all installed games |
| `src/ui/window/panel_games.rs` | Pass `latest_version` to `SetGameStatus` calls |
| `src/ui/game_list.rs` | Extend `SetGameStatus`, add pill maps, `build_game_row` pill widget, `should_show_pill` helper |
| `i18n/en-US/gnome_iris.ftl` | Add `update-available` key |
| `i18n/en-US/iris.ftl` | Add `update-available` key (if mirror) |
| `Cargo.toml` | Add `semver` if not present |
