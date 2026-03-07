# Build System

## `rust-toolchain.toml`

Pin to the specific nightly that has all required features:

```toml
[toolchain]
channel = "nightly-2026-02-01"
components = ["rust-src", "rust-analyzer", "rustfmt"]
profile = "default"
```

Ratic uses these nightly features (add only those your project actually needs):

```rust
#![feature(adt_const_params)]
#![feature(never_type)]          // type CommandOutput = !;
#![feature(random)]
#![feature(range_into_bounds)]
#![feature(result_option_map_or_default)]
#![feature(unsized_const_params)]
#![allow(incomplete_features)]
```

---

## `rustfmt.toml`

```toml
edition = "2024"
style_edition = "2024"

# Enable features
format_code_in_doc_comments = true
normalize_doc_attributes = true
reorder_impl_items = true
wrap_comments = true

# Line width
chain_width = 90
comment_width = 120
max_width = 120
use_small_heuristics = "Max"

# Struct literal widths
struct_lit_width = 18
struct_variant_width = 35

# Import grouping: std / external / crate — each in its own block
group_imports = "StdExternalCrate"
imports_granularity = "Module"

# Style
match_block_trailing_comma = true
overflow_delimited_expr = true
```

---

## `Cargo.toml` — Lints, Profiles, and Key Dependencies

### Lint Section

All Clippy lint groups are denied. New code must be 100% Clippy-clean.

```toml
[lints.clippy]
complexity  = "deny"
correctness = "deny"
nursery     = "deny"
pedantic    = "deny"
perf        = "deny"
style       = "deny"
suspicious  = "deny"

module_name_repetitions = "deny"   # enforce by default in pedantic but worth calling out

[lints.rust]
missing_docs = "deny"   # every pub item and module must have a doc comment
```

### Profile Section

```toml
# Speed up compilation of dependencies in debug builds.
[profile.dev.package."*"]
opt-level = 2

# Small, fast release binary.
[profile.release]
lto = "thin"
codegen-units = 1
strip = true
```

### Key Dependencies

```toml
[dependencies]
# Error handling
anyhow = "1"

# Useful derives (Display, FromStr, From, etc.)
derive_more = { version = "2", features = ["full"] }

# XDG directories
directories = "6"

# Logging
env_logger = "0.11"
log = "0.4"

# GStreamer (omit if no audio)
gst        = { package = "gstreamer",       version = "0.24", features = ["log", "v1_26"] }
gst-audio  = { package = "gstreamer-audio", version = "0.24", features = ["v1_26"] }
gst-play   = { package = "gstreamer-play",  version = "0.24", features = ["v1_26"] }

# i18n
i18n-embed    = { version = "0.16", features = ["desktop-requester", "fluent", "fluent-system"] }
i18n-embed-fl = "0.10"

# Relm4 — pinned to a specific git revision for stability
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

# Asset embedding (for i18n .ftl files)
rust-embed = "8"

# Serialization
serde      = { version = "1", features = ["serde_derive"] }
serde_json = "1"

# Content hashing
sha2 = "0.10"

# Date/time
time = { version = "0.3", features = ["macros", "parsing", "serde"] }

# Async runtime (minimal footprint)
tokio = { version = "1", features = ["macros", "rt", "sync"] }

[build-dependencies]
glib-build-tools   = "0.21"
relm4-icons-build  = "0.10"
```

**About the relm4 git pin:** The released crate on crates.io may lag behind the GTK/libadwaita version you need. Pin to a specific rev that matches your GNOME target (`gnome_49` feature = GNOME 49 / libadwaita 1.7+).

---

## `build.rs` Template

```rust
//! Build script: compiles icons, GResources, and GSettings schemas.

use std::process::{Command, exit};

/// Output directory for compiled GSettings schemas (consumed by GLib at runtime).
const SCHEMAS_DIR: &str = "./target/share/glib-2.0/schemas/";

fn main() {
    // Re-run this script if either of these files changes.
    println!("cargo::rerun-if-changed=data/org.example.myapp.gschema.xml");
    println!("cargo::rerun-if-changed=data/icons/icons.gresource.xml");

    // 1. Bundle relm4-icons (generates icon_names.rs into OUT_DIR).
    relm4_icons_build::bundle_icons(
        "icon_names.rs",              // output filename in OUT_DIR
        Some("org.example.myapp"),    // application ID
        Some("/org/example/myapp"),   // GResource prefix
        Some("data/icons"),           // directory containing additional SVG icons
        [
            // Names of relm4-icons to include (without the "-symbolic" suffix):
            "play-large",
            "pause-large",
            "skip-backward-large",
            "skip-forward-large",
            "music-note-outline",
        ],
    );

    // 2. Compile the custom GResource bundle (SVG icons, etc.).
    glib_build_tools::compile_resources(
        &["data/icons"],                     // search paths
        "data/icons/icons.gresource.xml",    // manifest
        "icons.gresources",                  // output file in OUT_DIR
    );

    // 3. Compile GSettings schemas into the target directory.
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

The compiled schema directory must be on `GSETTINGS_SCHEMA_DIR` at runtime, or installed system-wide. In development, Nix/the shell environment handles this automatically.

---

## GResource XML Format

`data/icons/icons.gresource.xml`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<gresources>
  <gresource prefix="/org/example/myapp/icons/scalabale/actions/">
    <file preprocess="xml-stripblanks">hicolor/scalable/apps/logo.svg</file>
    <file preprocess="xml-stripblanks">hicolor/scalable/apps/logo_black.svg</file>
  </gresource>
</gresources>
```

Files under `prefix` are accessible at runtime via `gio::resources_lookup_data(path, ...)` or `gtk::IconTheme::add_resource_path`. The prefix must match `theme.add_resource_path(...)` in `main.rs`.

---

## Registering Resources at Runtime

In `main.rs`:

```rust
fn initialize_custom_resources() {
    // Embeds and registers the compiled GResource bundle.
    gio::resources_register_include!("icons.gresources").unwrap();

    // Adds the resource path to the default icon theme.
    let display = gdk::Display::default().unwrap();
    let theme = gtk::IconTheme::for_display(&display);
    theme.add_resource_path("/org/example/myapp");
}
```

Then in `main()`:

```rust
relm4_icons::initialize_icons(icon_names::GRESOURCE_BYTES, icon_names::RESOURCE_PREFIX);
initialize_custom_resources();
```

`relm4_icons::initialize_icons` must be called before `initialize_custom_resources` so that the relm4 icon bundle is registered first.
