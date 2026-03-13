//! Build script: compiles icons, `GResources`, and `GSettings` schemas.

use std::process::{Command, exit};

/// Output directory for compiled `GSettings` schemas.
const SCHEMAS_DIR: &str = "./target/share/glib-2.0/schemas/";

fn main() {
    println!("cargo::rerun-if-changed=data/org.gnome.Iris.gschema.xml");
    println!("cargo::rerun-if-changed=data/icons/icons.gresource.xml");

    relm4_icons_build::bundle_icons(
        "icon_names.rs",
        Some("org.gnome.Iris"),
        Some("/org/gnome/Iris"),
        Some("data/icons"),
        ["view-list", "folder-open"],
    );

    glib_build_tools::compile_resources(
        &["data/icons"],
        "data/icons/icons.gresource.xml",
        "icons.gresources",
    );

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
