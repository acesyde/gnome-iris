# Game List Update-Available Pill — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show an "Update available" pill badge on a game row when its installed ReShade version is older than the latest known version.

**Architecture:** A `latest_version: Option<String>` field is added to `Window`; it is populated when `LatestVersionFetched` fires and broadcast via the existing `SetGameStatus` message (extended with a `latest_version` field) to `GameList`, which shows/hides a `gtk::Label` pill per row. Version comparison lives in the domain layer (`src/reshade/reshade.rs`) as a pure, unit-tested function.

**Tech Stack:** Rust stable 1.94, GTK4, Relm4, libadwaita, `semver = "1"`, Fluent (i18n), `mise exec` for all commands.

---

## Chunk 1: Domain helper + i18n + CSS

### Task 1: `semver` dependency + `is_version_outdated` (TDD)

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/reshade/reshade.rs`

- [ ] **Step 1.1 — Add `semver` to Cargo.toml**

  In `Cargo.toml`, inside the `[dependencies]` section (after the existing `tempfile = "3"` line is in `[dev-dependencies]`; find the right section), add:

  ```toml
  semver = "1"
  ```

  The `[dependencies]` section already contains entries like `relm4`, `anyhow`, `log`, etc. Add `semver = "1"` there.

- [ ] **Step 1.2 — Write failing tests**

  At the bottom of `src/reshade/reshade.rs`, inside the existing `#[cfg(test)] mod tests { ... }` block (after the last test, before the closing `}`), add:

  ```rust
  #[test]
  fn version_outdated_when_installed_is_older() {
      assert!(is_version_outdated("6.3.0", "v6.7.3"));
  }

  #[test]
  fn version_not_outdated_when_equal() {
      assert!(!is_version_outdated("6.7.3", "v6.7.3"));
  }

  #[test]
  fn version_not_outdated_when_newer() {
      assert!(!is_version_outdated("6.8.0", "v6.7.3"));
  }

  #[test]
  fn version_not_outdated_on_parse_failure() {
      assert!(!is_version_outdated("unknown", "v6.7.3"));
      assert!(!is_version_outdated("6.7.3", "unknown"));
  }

  #[test]
  fn version_outdated_strips_v_prefix() {
      // installed bare, latest v-prefixed
      assert!(is_version_outdated("6.3.0", "v6.7.3"));
      // both bare
      assert!(is_version_outdated("6.3.0", "6.7.3"));
      // both v-prefixed
      assert!(is_version_outdated("v6.3.0", "v6.7.3"));
  }
  ```

- [ ] **Step 1.3 — Run tests to confirm they fail**

  ```bash
  mise exec -- cargo test --lib reshade::reshade 2>&1 | tail -20
  ```

  Expected: compilation error — `is_version_outdated` is not yet defined.

- [ ] **Step 1.4 — Implement `is_version_outdated`**

  In `src/reshade/reshade.rs`, add this function **before** the `#[cfg(test)]` block (after the `list_installed_versions` function, around line 136):

  ```rust
  /// Returns `true` if `installed` is strictly older than `latest`.
  ///
  /// Both strings may optionally carry a leading `v` (e.g. `"v6.3.0"` or `"6.3.0"`).
  /// Returns `false` if either string cannot be parsed as a semver version.
  #[must_use]
  pub fn is_version_outdated(installed: &str, latest: &str) -> bool {
      use semver::Version;
      let strip = |s: &str| s.strip_prefix('v').unwrap_or(s);
      match (Version::parse(strip(installed)), Version::parse(strip(latest))) {
          (Ok(i), Ok(l)) => i < l,
          _ => false,
      }
  }
  ```

- [ ] **Step 1.5 — Run tests to confirm they pass**

  ```bash
  mise exec -- cargo test --lib reshade::reshade 2>&1 | tail -20
  ```

  Expected: all tests pass, including the 5 new ones.

- [ ] **Step 1.6 — Check clippy**

  ```bash
  mise exec -- cargo clippy 2>&1 | tail -20
  ```

  Expected: no warnings or errors.

- [ ] **Step 1.7 — Commit**

  ```bash
  git add Cargo.toml Cargo.lock src/reshade/reshade.rs
  git commit -m "feat(reshade): add is_version_outdated domain helper"
  ```

---

### Task 2: i18n strings

**Files:**
- Modify: `i18n/en-US/gnome_iris.ftl`
- Modify: `i18n/en-US/iris.ftl`
- Modify: `i18n/es-ES/gnome_iris.ftl`
- Modify: `i18n/es-ES/iris.ftl`
- Modify: `i18n/fr-FR/gnome_iris.ftl`
- Modify: `i18n/fr-FR/iris.ftl`
- Modify: `i18n/it-IT/gnome_iris.ftl`
- Modify: `i18n/it-IT/iris.ftl`
- Modify: `i18n/pt-BR/gnome_iris.ftl`
- Modify: `i18n/pt-BR/iris.ftl`

- [ ] **Step 2.1 — Add key to all FTL files**

  In each file, add the following line in the **Game list sections** block (near `remove-game`):

  ```
  update-available = Update available
  ```

  In `gnome_iris.ftl` files, insert after the `remove-game` line (around the `# Game list sections` comment).
  In `iris.ftl` files, insert after the `remove-game` line.

  Non-English locale files (`es-ES`, `fr-FR`, `it-IT`, `pt-BR`) use the English string as a placeholder — translators update it later. The `fl!()` macro panics at startup if the key is absent from any loaded locale.

- [ ] **Step 2.2 — Verify cargo check still passes**

  ```bash
  mise exec -- cargo check 2>&1 | tail -10
  ```

  Expected: no errors (the i18n key is validated at runtime, not compile time, but this confirms no regressions).

- [ ] **Step 2.3 — Commit**

  ```bash
  git add i18n/
  git commit -m "i18n: add update-available key to all locales"
  ```

---

### Task 3: Global CSS

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 3.1 — Add CSS call in `main()`**

  In `src/main.rs`, in the `main()` function, insert this line immediately before `app.run::<Window>(())`:

  ```rust
  relm4::set_global_css("label.pill { border-radius: 9999px; padding: 2px 8px; }");
  ```

  The function already has `let app = RelmApp::new(APPLICATION_ID);` on the line before `app.run`. Insert between those two.

- [ ] **Step 3.2 — Verify it compiles**

  ```bash
  mise exec -- cargo check 2>&1 | tail -10
  ```

  Expected: no errors. Relm4 is compiled with the `css` feature, so `relm4::set_global_css` is available.

- [ ] **Step 3.3 — Commit**

  ```bash
  git add src/main.rs
  git commit -m "feat(ui): register global pill CSS style"
  ```

---

## Chunk 2: GameList pill infrastructure + Window field

### Task 4: Add `latest_version` field to `Window`

**Files:**
- Modify: `src/ui/window/mod.rs`

- [ ] **Step 4.1 — Add field to the `Window` struct**

  In `src/ui/window/mod.rs`, in the `Window` struct definition (around line 26), add after the `installed_versions` field:

  ```rust
  /// Latest known `ReShade` version fetched from GitHub (or read from cache).
  latest_version: Option<String>,
  ```

- [ ] **Step 4.2 — Initialize to `None` in `init()`**

  In the `model = Self { ... }` block (around line 367), add after `installed_versions`:

  ```rust
  latest_version: None,
  ```

- [ ] **Step 4.3 — Verify it compiles**

  ```bash
  mise exec -- cargo check 2>&1 | tail -10
  ```

  Expected: no errors.

- [ ] **Step 4.4 — Commit**

  ```bash
  git add src/ui/window/mod.rs
  git commit -m "feat(window): add latest_version field"
  ```

---

### Task 5: GameList pill maps + `build_game_row` tuple return

**Files:**
- Modify: `src/ui/game_list.rs`

This task adds the pill widget infrastructure to `GameList` without yet wiring up the version comparison (that comes in Task 6 when `SetGameStatus` gains its new field).

- [ ] **Step 5.1 — Add pill maps to the `GameList` struct**

  In `src/ui/game_list.rs`, in the `GameList` struct (around line 12), add after `manual_rows`:

  ```rust
  /// Pill labels for auto-detected game rows, keyed by game ID.
  auto_update_pills: HashMap<String, gtk::Label>,
  /// Pill labels for manually added game rows, keyed by game ID.
  manual_update_pills: HashMap<String, gtk::Label>,
  ```

- [ ] **Step 5.2 — Initialize pill maps in `init()`**

  In the `let mut model = Self { ... }` block (around line 116), add after `manual_rows: HashMap::new()`:

  ```rust
  auto_update_pills: HashMap::new(),
  manual_update_pills: HashMap::new(),
  ```

- [ ] **Step 5.3 — Change `build_game_row` to return a tuple**

  Change the function signature (around line 193) from:

  ```rust
  fn build_game_row(game: &Game, sender: &ComponentSender<GameList>) -> adw::ActionRow {
  ```

  to:

  ```rust
  fn build_game_row(game: &Game, sender: &ComponentSender<GameList>) -> (adw::ActionRow, gtk::Label) {
  ```

  Inside the function body, before the final `row` return, add the pill widget (insert before the chevron suffix, i.e. before `let chevron = ...`):

  ```rust
  let pill = gtk::Label::new(Some(&fl!("update-available")));
  pill.add_css_class("pill");
  pill.add_css_class("accent");
  pill.set_visible(false);
  row.add_suffix(&pill);
  ```

  Change the final return from `row` to:

  ```rust
  (row, pill)
  ```

- [ ] **Step 5.4 — Update `init()` loop to destructure the tuple**

  In the `for game in &games { ... }` loop in `init()` (around line 128), change:

  ```rust
  let row = build_game_row(game, &sender);
  if matches!(game.source, GameSource::Manual) {
      widgets.manual_list_box.append(&row);
      model.manual_rows.insert(game.id.clone(), row);
  } else {
      widgets.auto_list_box.append(&row);
      model.auto_rows.insert(game.id.clone(), row);
  }
  ```

  to:

  ```rust
  let (row, pill) = build_game_row(game, &sender);
  if matches!(game.source, GameSource::Manual) {
      widgets.manual_list_box.append(&row);
      model.manual_rows.insert(game.id.clone(), row);
      model.manual_update_pills.insert(game.id.clone(), pill);
  } else {
      widgets.auto_list_box.append(&row);
      model.auto_rows.insert(game.id.clone(), row);
      model.auto_update_pills.insert(game.id.clone(), pill);
  }
  ```

- [ ] **Step 5.5 — Update `AddGame` arm in `update()` to destructure the tuple**

  In `update()`, in the `Controls::AddGame(game)` arm (around line 155), change:

  ```rust
  let row = build_game_row(&game, &sender);
  if matches!(game.source, GameSource::Manual) {
      self.manual_list_box.append(&row);
      self.manual_rows.insert(game.id.clone(), row);
  } else {
      self.auto_list_box.append(&row);
      self.auto_rows.insert(game.id.clone(), row);
  }
  ```

  to:

  ```rust
  let (row, pill) = build_game_row(&game, &sender);
  if matches!(game.source, GameSource::Manual) {
      self.manual_list_box.append(&row);
      self.manual_rows.insert(game.id.clone(), row);
      self.manual_update_pills.insert(game.id.clone(), pill);
  } else {
      self.auto_list_box.append(&row);
      self.auto_rows.insert(game.id.clone(), row);
      self.auto_update_pills.insert(game.id.clone(), pill);
  }
  ```

- [ ] **Step 5.6 — Update `RemoveGame` arm to also clean pill maps**

  In `update()`, in the `Controls::RemoveGame(id)` arm (around line 167), change:

  ```rust
  if let Some(row) = self.manual_rows.remove(&id) {
      self.manual_list_box.remove(&row);
  } else if let Some(row) = self.auto_rows.remove(&id) {
      self.auto_list_box.remove(&row);
  }
  ```

  to:

  ```rust
  if let Some(row) = self.manual_rows.remove(&id) {
      self.manual_list_box.remove(&row);
      self.manual_update_pills.remove(&id);
  } else if let Some(row) = self.auto_rows.remove(&id) {
      self.auto_list_box.remove(&row);
      self.auto_update_pills.remove(&id);
  }
  ```

- [ ] **Step 5.7 — Verify it compiles**

  ```bash
  mise exec -- cargo check 2>&1 | tail -10
  ```

  Expected: no errors.

- [ ] **Step 5.8 — Run clippy**

  ```bash
  mise exec -- cargo clippy 2>&1 | tail -20
  ```

  Expected: no warnings.

- [ ] **Step 5.9 — Commit**

  ```bash
  git add src/ui/game_list.rs
  git commit -m "feat(game-list): add pill widget infrastructure to game rows"
  ```

---

## Chunk 3: Full wiring

### Task 6: Extend `SetGameStatus` + wire pill visibility

**Files:**
- Modify: `src/ui/game_list.rs`
- Modify: `src/ui/window/panel_games.rs`

This task adds `latest_version` to `SetGameStatus` and updates all existing emit call sites atomically (required for the code to compile). It also adds the visibility logic in the `GameList` handler.

- [ ] **Step 6.1 — Add `latest_version` to the `SetGameStatus` enum variant**

  In `src/ui/game_list.rs`, in the `Controls` enum (around line 37), change `SetGameStatus` from:

  ```rust
  /// Update the install-status subtitle for a game row.
  SetGameStatus {
      /// Stable game ID.
      id: String,
      /// Installed version string, or `None` if `ReShade` is not installed.
      version: Option<String>,
  },
  ```

  to:

  ```rust
  /// Update the install-status subtitle and update-pill visibility for a game row.
  SetGameStatus {
      /// Stable game ID.
      id: String,
      /// Installed version string, or `None` if `ReShade` is not installed.
      version: Option<String>,
      /// Latest known `ReShade` version, or `None` if not yet fetched.
      latest_version: Option<String>,
  },
  ```

- [ ] **Step 6.2 — Update the `SetGameStatus` handler in `GameList::update()`**

  In `src/ui/game_list.rs`, in `update()`, replace the `Controls::SetGameStatus` arm (around line 176):

  ```rust
  Controls::SetGameStatus { id, version } => {
      let subtitle = match &version {
          Some(v) if !v.is_empty() => format!("ReShade {v}"),
          Some(_) => fl!("reshade-installed"),
          None => fl!("not-installed"),
      };
      if let Some(row) = self.auto_rows.get(&id).or_else(|| self.manual_rows.get(&id)) {
          row.set_subtitle(&subtitle);
      }
  },
  ```

  with:

  ```rust
  Controls::SetGameStatus { id, version, latest_version } => {
      let subtitle = match &version {
          Some(v) if !v.is_empty() => format!("ReShade {v}"),
          Some(_) => fl!("reshade-installed"),
          None => fl!("not-installed"),
      };
      if let Some(row) = self.auto_rows.get(&id).or_else(|| self.manual_rows.get(&id)) {
          row.set_subtitle(&subtitle);
      }
      let outdated = match (&version, &latest_version) {
          (Some(installed), Some(latest)) => {
              crate::reshade::reshade::is_version_outdated(installed, latest)
          },
          _ => false,
      };
      if let Some(pill) = self.auto_update_pills.get(&id).or_else(|| self.manual_update_pills.get(&id)) {
          pill.set_visible(outdated);
      }
  },
  ```

- [ ] **Step 6.3 — Update `handle_install_complete` call site**

  In `src/ui/window/panel_games.rs`, in `handle_install_complete` (around line 94), change:

  ```rust
  model.game_list.emit(game_list::Controls::SetGameStatus {
      id: id.clone(),
      version: Some(version.clone()),
  });
  ```

  to:

  ```rust
  model.game_list.emit(game_list::Controls::SetGameStatus {
      id: id.clone(),
      version: Some(version.clone()),
      latest_version: model.latest_version.clone(),
  });
  ```

- [ ] **Step 6.4 — Update `handle_uninstall_complete` call site**

  In `src/ui/window/panel_games.rs`, in `handle_uninstall_complete` (around line 115), change:

  ```rust
  model.game_list.emit(game_list::Controls::SetGameStatus {
      id: id.clone(),
      version: None,
  });
  ```

  to:

  ```rust
  model.game_list.emit(game_list::Controls::SetGameStatus {
      id: id.clone(),
      version: None,
      latest_version: model.latest_version.clone(),
  });
  ```

- [ ] **Step 6.5 — Verify it compiles**

  ```bash
  mise exec -- cargo check 2>&1 | tail -10
  ```

  Expected: no errors. All `SetGameStatus` pattern matches and struct literals now include `latest_version`.

- [ ] **Step 6.6 — Run clippy**

  ```bash
  mise exec -- cargo clippy 2>&1 | tail -20
  ```

  Expected: no warnings.

- [ ] **Step 6.7 — Commit**

  ```bash
  git add src/ui/game_list.rs src/ui/window/panel_games.rs
  git commit -m "feat(game-list): wire update-available pill to SetGameStatus"
  ```

---

### Task 7: `handle_latest_version_fetched` broadcast

**Files:**
- Modify: `src/ui/window/panel_preferences.rs`

- [ ] **Step 7.1 — Update the function**

  In `src/ui/window/panel_preferences.rs`, replace `handle_latest_version_fetched` entirely (around line 21):

  ```rust
  pub(super) fn handle_latest_version_fetched(model: &Window, version: String) {
      model.preferences.emit(preferences::Controls::SetLatestVersion(version));
  }
  ```

  with:

  ```rust
  /// Store the latest version, forward to Preferences, and refresh pill visibility on all installed games.
  pub(super) fn handle_latest_version_fetched(model: &mut Window, version: String) {
      model.latest_version = Some(version.clone());
      model.preferences.emit(preferences::Controls::SetLatestVersion(version.clone()));

      // Collect before emitting to satisfy the borrow checker:
      // iterating `model.games` (immutable) and calling `model.game_list.emit` (mutable)
      // cannot happen in the same loop body on the same `&mut Window`.
      use crate::reshade::game::InstallStatus;
      let installed: Vec<(String, Option<String>)> = model
          .games
          .iter()
          .filter_map(|g| match &g.status {
              InstallStatus::Installed { version: v, .. } => Some((g.id.clone(), v.clone())),
              InstallStatus::NotInstalled => None,
          })
          .collect();

      for (id, installed_version) in installed {
          model.game_list.emit(game_list::Controls::SetGameStatus {
              id,
              version: installed_version,
              latest_version: Some(version.clone()),
          });
      }
  }
  ```

  Also add `game_list` to the existing grouped import at the top of `panel_preferences.rs`. The current line is:

  ```rust
  use crate::ui::{game_detail, install_worker, preferences};
  ```

  Change it to:

  ```rust
  use crate::ui::{game_detail, game_list, install_worker, preferences};
  ```

  (Clippy rejects duplicate `use crate::ui` lines under the project's `deny(all lint groups)` — the merge into the existing grouped import is required.)

- [ ] **Step 7.2 — Verify it compiles**

  ```bash
  mise exec -- cargo check 2>&1 | tail -10
  ```

  Expected: no errors.

- [ ] **Step 7.3 — Run clippy**

  ```bash
  mise exec -- cargo clippy 2>&1 | tail -20
  ```

  Expected: no warnings.

- [ ] **Step 7.4 — Commit**

  ```bash
  git add src/ui/window/panel_preferences.rs
  git commit -m "feat(window): broadcast latest version to game list pills"
  ```

---

### Task 8: `handle_game_added` follow-up status

**Files:**
- Modify: `src/ui/window/panel_games.rs`

When a game is manually added, it may already have ReShade installed. If `latest_version` is already known at that moment, we prime the pill immediately rather than waiting for the next version fetch.

- [ ] **Step 8.1 — Add follow-up `SetGameStatus` in `handle_game_added`**

  In `src/ui/window/panel_games.rs`, in `handle_game_added` (around line 166), the current last three lines are:

  ```rust
  model.games.push(game.clone());
  model.game_list.emit(game_list::Controls::AddGame(game));
  // Keep the dialog's duplicate-detection list in sync.
  let paths = model.games.iter().map(|g| g.path.clone()).collect();
  model.add_game_dialog.emit(add_game_dialog::Controls::UpdateExistingPaths(paths));
  ```

  Note that `game` is **moved** into `AddGame(game)`. Capture the ID and status **before** that emit, then use the captured values in the follow-up:

  ```rust
  model.games.push(game.clone());
  // Capture before the move into AddGame.
  let game_id = game.id.clone();
  let game_status = game.status.clone();
  model.game_list.emit(game_list::Controls::AddGame(game));
  // Prime the pill for this game immediately if the latest version is already known.
  if model.latest_version.is_some() {
      let installed_version = match &game_status {
          InstallStatus::Installed { version: v, .. } => v.clone(),
          InstallStatus::NotInstalled => None,
      };
      model.game_list.emit(game_list::Controls::SetGameStatus {
          id: game_id,
          version: installed_version,
          latest_version: model.latest_version.clone(),
      });
  }
  // Keep the dialog's duplicate-detection list in sync.
  let paths = model.games.iter().map(|g| g.path.clone()).collect();
  model.add_game_dialog.emit(add_game_dialog::Controls::UpdateExistingPaths(paths));
  ```

  `InstallStatus` is already imported at the top of `panel_games.rs` (`use crate::reshade::game::{DllOverride, ExeArch, Game, GameSource, InstallStatus};`), so no import changes are needed.

- [ ] **Step 8.2 — Verify it compiles**

  ```bash
  mise exec -- cargo check 2>&1 | tail -10
  ```

  Expected: no errors.

- [ ] **Step 8.3 — Run all domain tests**

  ```bash
  mise exec -- cargo test 2>&1 | tail -20
  ```

  Expected: all tests pass (domain layer tests run; UI is not unit-tested per project convention).

- [ ] **Step 8.4 — Final clippy**

  ```bash
  mise exec -- cargo clippy 2>&1 | tail -20
  ```

  Expected: no warnings.

- [ ] **Step 8.5 — Commit**

  ```bash
  git add src/ui/window/panel_games.rs
  git commit -m "feat(game-list): prime update pill on game-added when version known"
  ```

---

## Manual Smoke Test

After all tasks are complete, run the app and verify:

```bash
GSETTINGS_SCHEMA_DIR=./target/share/glib-2.0/schemas mise exec -- cargo run
```

Check:
1. Games with an outdated installed ReShade show an "Update available" pill (accent-coloured, rounded).
2. Games with the latest version show no pill.
3. Games without ReShade installed show no pill.
4. After downloading a newer version in Preferences, any games using the old version now show the pill.
5. The pill disappears after reinstalling to the latest version via the game detail pane.
