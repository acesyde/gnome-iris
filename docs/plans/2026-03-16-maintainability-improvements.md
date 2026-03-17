# gnome-iris Maintainability Improvement Plan

**Date:** 2026-03-16
**Author:** Rust Engineer review
**Scope:** Second-pass analysis after `2026-03-15-architecture-improvements.md` — all items there are done.
**Method:** Static analysis of the full source tree (~6 200 lines across 30+ files).

---

## Context

The domain layer (`src/reshade/`) is clean, well-tested, and well-documented. The architecture improvements
plan has already been executed in full. This document focuses on what remains: test gaps in the service layer,
oversized UI modules, undocumented decisions, dependency hygiene, and a handful of type-safety issues that
reduce long-term confidence in the codebase.

The goal is not a rewrite — every item below is a targeted, independently shippable change.

---

## Improvement Items

### 1. ✅ Test the service trait default implementations

**Problem:** `src/reshade/services.rs` introduces `DefaultReShadeProvider`, `DefaultGameRepository`, and
`DefaultShaderSyncService` as thin wrappers around the domain free functions. These are the objects that the
UI workers actually call at runtime. They have **zero tests**. A bug introduced in these adapters — a missing
`with_context()`, a wrong function wired up, an incorrect field passed through — will not be caught before it
reaches users.

**What to test (per implementation):**

| Impl | Happy-path scenario to cover |
|------|------------------------------|
| `DefaultReShadeProvider` | `list_installed_versions()` returns correct paths from a temp dir |
| `DefaultReShadeProvider` | `download_and_extract()` writes expected files when given a mock zip |
| `DefaultGameRepository`  | `games()` reflects the in-memory list, `save_games()` persists to disk |
| `DefaultShaderSyncService` | `sync()` calls through without panicking on a bare-repo fixture |

Use `tempfile` (already a dev-dependency) for filesystem isolation. Mock network calls by implementing the
trait on a test double rather than reaching the real network.

**Files:** `src/reshade/services.rs`, new `tests/services.rs`
**Effort:** S — ~80 lines of test code
**Value:** High — only layer that bridges domain to UI with no coverage

---

### 2. Split `preferences.rs` into focused sub-components

**Problem:** `src/ui/preferences.rs` is 640 lines and handles three unrelated concerns inside one relm4
component:

1. **Config panel** — global config, DLL override defaults, addon API toggle.
2. **Version management** — list installed versions, set active, download new, remove old.
3. **Shader repos** — list configured repos, add/remove repos from the list.

Each concern has its own widgets, its own Controls variants, and its own update logic. The three are tangled
together, making it hard to reason about any one in isolation. The module is already the largest in the UI
layer.

**Fix:** Extract concerns 2 and 3 into standalone relm4 components:

```
src/ui/preferences/
    mod.rs              (≤ 200 lines — orchestration + config panel)
    panel_versions.rs   (version list, download, remove)
    panel_repos.rs      (shader repo list, add/remove)
```

`Preferences` becomes a thin host that owns `panel_versions::Versions` and `panel_repos::Repos` as child
components, forwarding their signals upward. Controls variants for versions/repos move to the appropriate
sub-enum, keeping `Preferences::Controls` focused.

**Acceptance:** Each resulting file ≤ 250 lines; `cargo clippy` still clean.

**Files:** `src/ui/preferences.rs` → `src/ui/preferences/mod.rs` + two new files
**Effort:** M — mostly mechanical split, but relm4 component wiring needs care
**Value:** High — largest single source of UI cognitive load

---

### 3. ✅ Document and track the relm4 git-pin

**Problem:** `Cargo.toml` pins relm4 (and `relm4-components`) to a specific Git commit
(`baa1c23ab35e3b8c4117714042671f7ed02aeabb`). There is no comment explaining:

- Why this specific commit is needed.
- What feature or bug fix it contains that is not in a released version.
- What the migration path to a stable release looks like.

If this commit is ever force-pushed or the repo reorganised, CI will fail with no obvious fix. A new
contributor has no way to know whether they can upgrade this dependency.

**Fix (no code change — documentation only):**

1. Add an inline comment in `Cargo.toml` above the git dependency:
   ```toml
   # Pinned to commit baa1c23 because <reason>.
   # Upstream issue/PR tracking stable release: <url>
   # Last verified working: 2026-03-16
   # To upgrade: run `cargo update -p relm4` and smoke-test the full UI.
   relm4 = { git = "https://github.com/relm4/relm4", rev = "baa1c23...", ... }
   ```
2. If a GitHub issue exists on the relm4 repo tracking the unreleased feature, link it.
3. Open a tracking issue in this repo: "Migrate relm4 from git-pin to released version".

**Files:** `Cargo.toml`, optionally a new GitHub issue
**Effort:** XS
**Value:** High — prevents mystery build failures and unblocks future upgrades

---

### 4. ✅ Remove or justify `sevenz-rust` dependency

**Problem:** `Cargo.toml` declares `sevenz-rust = "0.6"` as a runtime dependency. A search of all `.rs`
source files finds **no `use sevenz_rust`** or `extern crate sevenz_rust` anywhere. It is either:

- A leftover from a feature that was removed without cleaning up `Cargo.toml`.
- An intentional placeholder for future 7z support that was never documented.

An unused dependency increases compile time, binary size, and the attack surface of the supply chain.

**Fix:**

1. Run `cargo check` after removing `sevenz-rust` from `Cargo.toml`. If it compiles cleanly, the dependency
   is genuinely unused — remove it.
2. If it turns out to be needed (e.g., behind a `cfg` or a future feature gate), add a comment explaining
   why it is declared but not yet imported.

**Files:** `Cargo.toml`
**Effort:** XS
**Value:** Medium — cleaner dependency tree, faster CI

---

### 5. ✅ Add doc comments to all public UI Controls and Signal variants

**Problem:** `CLAUDE.md` and `Cargo.toml` both deny `missing_docs` at the crate level. The domain layer
follows this well. However several UI `Controls` and `Signal` enum variants have either no doc comment or
only a one-word description that doesn't explain intent:

Examples from the current codebase (paraphrased):

```rust
/// Update config.
SetConfig(GlobalConfig),         // ← What happens if the config is unchanged?

/// Install version.
InstallVersion(String),          // ← What is the String — semver? Path? Display name?

/// Show a toast.
ShowToast(String),               // ← Is this info, warning, or error? Duration?
```

For a message-passing UI, variant documentation is the primary way a new contributor understands what
triggers what. Missing context forces them to trace the entire call chain.

**Fix:** For every `pub enum Controls` and `pub enum Signal` variant in the UI layer, write a doc comment
that answers:

- What data does this variant carry?
- What observable effect does handling it produce?
- Any invariants the caller must satisfy?

**Files:** all `src/ui/**/*.rs` files that define Controls/Signal enums
**Effort:** S — documentation-only, no logic changes
**Value:** Medium — onboarding time, prevents misuse

---

### 6. ✅ Consolidate the dual `.ftl` translation files

**Problem:** Every language directory under `i18n/` contains two Fluent files:

```
i18n/en-US/
    gnome_iris.ftl    ← loaded by fluent_language_loader!() (crate name → filename)
    iris.ftl          ← origin unknown; possibly a leftover from an earlier crate name
```

Per `CLAUDE.md`: _"Both `iris.ftl` and `gnome_iris.ftl` exist"_ — the macro loads only `gnome_iris.ftl`.
`iris.ftl` is either a legacy file that is silently ignored, or it was supposed to be loaded separately and
isn't. Either way, maintaining translations in two files wastes translator effort and risks the files
diverging silently.

**Fix:**

1. Determine empirically which file the running app actually loads (add a temporary `log::debug!` in
   `localization.rs`, or audit the `i18n-embed` documentation).
2. If `iris.ftl` is unused: remove it from all language directories and update `build.rs`/`i18n.toml` if
   needed.
3. If `iris.ftl` should be loaded alongside `gnome_iris.ftl`: fix the loader configuration so both are
   always in sync and document why two files are needed.

**Files:** `i18n/*/iris.ftl`, `src/localization.rs`, `i18n.toml`
**Effort:** XS–S depending on what is found
**Value:** Medium — avoids translator confusion and silent untranslated strings

---

### 7. ✅ Add English fallback for missing Fluent translations

**Problem:** `localization.rs` initialises the `FluentLanguageLoader` and selects the system locale, but
does not explicitly configure a fallback. If a translation key is missing in a non-English locale (e.g.,
a new string added to `en-US` but not yet translated for `fr-FR`), the app will render an empty string or
the raw message ID instead of the English text.

```rust
// localization.rs — current
fl_loader.load_languages(&Localizations, &[requested_language]).ok();
// ← no fallback to en-US when a key is absent
```

**Fix:** Configure the loader to fall back to `en-US` for any missing key:

```rust
fl_loader
    .load_languages(&Localizations, &[requested_language, &langid!("en-US")])
    .ok();
```

Also add a `#[cfg(test)]` test that loads a non-existent key in a non-English locale and asserts the result
is the English string, not empty.

**Files:** `src/localization.rs`
**Effort:** XS
**Value:** Medium — prevents blank labels when translations are incomplete

---

### 8. ✅ Replace hardcoded DLL/arch values in the install completion signal

**Problem:** `src/ui/window/panel_games.rs` handles `InstallWorker::Signal::Done` with a hardcoded
placeholder for DLL override and architecture:

```rust
// panel_games.rs (paraphrased)
Signal::Done => sender.output(GamesMsg::InstallComplete {
    dll: DllOverride::D3D11,   // ← always D3D11, regardless of what was installed
    arch: ExeArch::X64,        // ← always x64, regardless of the game's actual arch
})
```

A comment in the code notes this is a workaround for a relm4 macro constraint. The values are wrong for
32-bit games and for non-D3D11 games. While the install itself uses the correct values internally, the
UI model is updated with wrong data, so the displayed status after install may not match reality until the
next reload.

**Fix:**

1. Add `dll: DllOverride` and `arch: ExeArch` fields to `InstallWorker::Signal::Done`.
2. The worker sets these fields from the `InstallRequest` it was given at startup.
3. `panel_games.rs` forwards the received values rather than hardcoding them.

This makes the install flow end-to-end type-safe with no implicit assumptions.

**Files:** `src/ui/worker_types.rs`, `src/ui/install_worker.rs`, `src/ui/window/panel_games.rs`
**Effort:** S
**Value:** Medium — correctness for 32-bit and non-D3D11 games; currently a latent bug

---

### 9. ✅ Add pre-flight validation to the Add Game and Add Shader Repo dialogs

**Problem:** Both dialogs do minimal validation before accepting user input:

- `add_game_dialog.rs`: Checks only that the selected path exists on disk. Does not verify it is a `.exe`,
  that it is readable, or that a game with the same path is not already in the list.
- `add_shader_repo_dialog.rs`: Checks that the URL is non-empty and appears to be a Git URL, but does not
  verify the remote is reachable before adding it to the config. Duplicate URLs are silently accepted.

This leads to broken state that is hard to recover from (a non-exe path in the game list, a duplicate repo
that causes a later sync conflict).

**Fix — Add Game:**
```rust
fn validate_game_path(path: &Path, existing: &[Game]) -> Result<(), String> {
    if path.extension().and_then(OsStr::to_str) != Some("exe") {
        return Err(fl!("error-not-an-exe"));
    }
    if existing.iter().any(|g| g.exe_path == path) {
        return Err(fl!("error-game-already-added"));
    }
    Ok(())
}
```

**Fix — Add Shader Repo:**
- Deduplicate check: reject if `config.shader_repos` already contains the same URL.
- Add a Fluent string for the "repo already added" error.

Display errors inline in the dialog (an `adw::Banner` or a label with `error` CSS class) rather than silently
doing nothing or allowing invalid state.

**Files:** `src/ui/add_game_dialog.rs`, `src/ui/add_shader_repo_dialog.rs`, `i18n/en-US/gnome_iris.ftl`
**Effort:** S
**Value:** Medium — prevents corrupted config that is hard to recover from in the UI

---

### 10. Extract and document the message-flow architecture

**Problem:** `src/ui/window/mod.rs::init()` wires together all components, workers, and signal forwarders
in ~350 lines with no high-level comment explaining the component hierarchy or data flow. A new contributor
reading the file must trace each `connect_*` call individually to understand the architecture.

The forwarding pattern — where a child component's `Signal` is forwarded to the parent's `Controls`, which
routes it to a panel, which may emit further signals — is non-obvious until you have seen it several times.

**Fix (documentation-only, no logic changes):**

1. Add a module-level `//!` comment to `src/ui/window/mod.rs` with an ASCII component diagram:

```
//! ## Component hierarchy
//!
//!  Window
//!  ├── GameList            → GamesMsg (via panel_games)
//!  │     └── GameDetail   → GameDetailMsg
//!  ├── ShaderCatalog       → ShadersMsg (via panel_shaders)
//!  ├── Preferences         → PrefsMsg (via panel_preferences)
//!  │     ├── Versions
//!  │     └── Repos
//!  ├── InstallWorker       → GamesMsg::InstallDone
//!  └── ShaderWorker        → ShadersMsg::SyncDone
//!
//! Signal flow: Child::Signal → parent forward_to! → Window::Controls
//! → panel_*.rs update() → child.emit(Controls::*)
```

2. Add section comments inside `init()` grouping the initialization steps:
   `// --- Component initialization ---`, `// --- Signal forwarding ---`, etc.

**Files:** `src/ui/window/mod.rs`
**Effort:** XS
**Value:** Medium — drastically reduces onboarding time for new contributors

---

## Priority Order

| # | Item | Effort | Value | Do First? |
|---|------|--------|-------|-----------|
| 3 | ✅ Document relm4 git-pin | XS | High | Yes — zero risk, high payoff |
| 4 | ✅ Remove sevenz-rust | XS | Medium | Yes — one-liner |
| 7 | ✅ i18n fallback to English | XS | Medium | Yes — one-liner |
| 10 | Document message-flow architecture | XS | Medium | Yes — documentation only |
| 6 | ✅ Consolidate dual .ftl files | XS–S | Medium | Yes — before next translation round |
| 5 | ✅ Doc comments on Controls/Signal variants | S | Medium | Before next feature work |
| 1 | ✅ Service trait tests | S | High | Before merging any future service changes |
| 9 | ✅ Add dialog input validation | S | Medium | Before next UX pass |
| 8 | ✅ Fix hardcoded DLL/arch in install signal | S | Medium | Before 32-bit game support |
| 2 | Split preferences.rs | M | High | When preferences.rs is next touched |

---

## Items Explicitly Out of Scope

The following were considered and deferred:

- **Relm4 upgrade from git-pin to released version** — blocked on upstream releasing the feature we need
  (see item 3). Tracked separately.
- **UI component unit tests** — Relm4 components are difficult to test without a real GTK display. The
  existing strategy of testing business logic at the domain layer and manually smoke-testing the UI is
  acceptable at this project scale. Revisit if headless GTK4 testing infrastructure matures.
- **Async Steam discovery** — Already addressed in item 9 of the 2026-03-15 plan (marked done).
- **Replacing `anyhow` with typed errors** — The codebase is an app, not a library. `anyhow` is appropriate
  here. Typed errors would add complexity without enabling callers to pattern-match on variants meaningfully.
- **Tokio thread count (hardcoded 4)** — Acceptable for a desktop GUI app; not a bottleneck.

---

## Verification

For every item, verify with:

```bash
mise exec -- cargo check          # type-checks without GTK display
mise exec -- cargo test           # domain + service unit + integration tests
mise exec -- cargo clippy         # must be 100% clean (all groups denied)
mise exec -- cargo fmt --check    # formatting
```

Manual smoke test (requires display):

```bash
GSETTINGS_SCHEMA_DIR=./target/share/glib-2.0/schemas mise exec -- cargo run
```
