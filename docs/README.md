# Architecture Documentation

This directory contains reusable architectural documentation extracted from Ratic, a Rust/GTK4/libadwaita desktop music player. The goal is to enable future Claude instances (or humans) to scaffold a new similar application from these patterns without reading all of the source code.

## What This Is Not

This is not a user guide or API reference. It is a **scaffolding kit** — concrete templates and rationale that let you build a new GTK4+Relm4 Rust desktop application fast.

## Files and Reading Order

Apply these in order when bootstrapping a new app:

| Step | File | What It Covers |
|------|------|----------------|
| 1 | [`build-system.md`](build-system.md) | Toolchain pin, `Cargo.toml` lint/profile config, `build.rs` for icons + schemas + resources, key crate versions |
| 2 | [`project-structure.md`](project-structure.md) | Directory layout, module conventions, `main.rs` and `src/ui/mod.rs` templates |
| 3 | [`relm4-components.md`](relm4-components.md) | All three component trait patterns (`SimpleComponent`, `Component`, `AsyncComponent`/`SimpleAsyncComponent`) with full templates and wiring |
| 4 | [`data-model.md`](data-model.md) | `Shared<T>`, cache pattern, SHA-512 UIDs, `Target` enum, async cache access, streaming library loading |
| 5 | [`i18n.md`](i18n.md) | `i18n.toml`, `localization.rs` template, `.ftl` syntax, `fl!` macro |
| 6 | [`ui-patterns.md`](ui-patterns.md) | GSettings, breakpoints, toasts, CSS, background blending, menus, `OpenDialog`, ViewStack navigation, `OverlaySplitView` |

## How to Bootstrap a New App

1. **Copy the build system.** Use the templates in `build-system.md` to create `rust-toolchain.toml`, `rustfmt.toml`, `Cargo.toml` (lints + profiles + deps), and `build.rs`.

2. **Create the directory skeleton.** Follow `project-structure.md`: `src/`, `src/ui/`, `src/<domain>/`, `data/`, `data/icons/`, `i18n/<lang>/`.

3. **Write `main.rs` and `src/ui/mod.rs`** using the templates in `project-structure.md`.

4. **Define your root component** using the `SimpleComponent` template in `relm4-components.md` (analogous to `Window`), then build child components as needed.

5. **Define your data model** using `data-model.md`: create your cache struct, `Shared<T>` aliases, and UID functions.

6. **Set up i18n** with the `localization.rs` template and `.ftl` files.

7. **Wire UI patterns** as needed from `ui-patterns.md`: GSettings schema, breakpoints, toasts, responsive navigation.

## Key Conventions Across All Files

- **Input message type is named `Controls`**, output is named `Signal` or `Output`.
- **`Shared<T> = Arc<RwLock<T>>`** is the universal shared-state primitive.
- **All public items have doc comments** (`///` for items, `//!` for modules) — clippy denies `missing_docs`.
- **Commit style:** `type(scope): message` — type is `feat`, `fix`, or `chore`.
- **Error propagation:** `anyhow::Result<T>` everywhere; logging via `log` crate.
