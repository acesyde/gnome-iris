# UI Patterns

## GSettings — Persistent App Preferences

### Schema XML

Place at `data/org.example.myapp.gschema.xml`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<schemalist>
  <schema id="org.example.myapp" path="/org/example/myapp/">
    <!-- String setting with XDG env-var substitution -->
    <key name="default-library-dir" type="s">
      <default>"$XDG_MUSIC_DIR"</default>
      <summary>Default directory scanned for music files</summary>
    </key>

    <!-- Boolean setting -->
    <key name="dynamic-background" type="b">
      <default>true</default>
      <summary>Whether to show a blurred cover as background</summary>
    </key>

    <!-- Enum-like string with allowed values -->
    <key name="play-mode" type="s">
      <default>"classic"</default>
      <summary>Playback continuation mode</summary>
      <choices>
        <choice value="stop" />
        <choice value="classic" />
        <choice value="loop" />
        <choice value="repeat" />
      </choices>
    </key>

    <!-- Double setting -->
    <key name="volume" type="d">
      <default>0.5</default>
      <summary>Audio volume (0.0 – 1.0)</summary>
    </key>
  </schema>
</schemalist>
```

Type codes: `s` = string, `b` = boolean, `i` = int32, `d` = double.

### Reading / Writing in Rust

```rust
use relm4::gtk::gio;
use relm4::gtk::gio::prelude::{SettingsExt, SettingsExtManual};

let settings = gio::Settings::new(APPLICATION_ID);

// Read:
let vol: f64   = settings.double("volume");
let flag: bool = settings.boolean("dynamic-background");
let mode: String = settings.string("play-mode").to_string();

// Read a raw GVariant (useful for typed extraction):
let raw: String = settings.value("default-library-dir").get::<String>().unwrap();

// Write (returns Err if the key does not exist or type mismatches):
settings.set_boolean("dynamic-background", true).unwrap();
settings.set_string("play-mode", "loop").unwrap();
settings.set_double("volume", 0.8).unwrap();
```

### `$XDG_*` Env-Var Substitution Pattern

GSettings stores the literal string `"$XDG_MUSIC_DIR"`. Expand it at read time:

```rust
let mut dir = settings.string("default-library-dir").to_string();
if let Some(var_name) = dir.strip_prefix('$') {
    dir = std::env::var(var_name).unwrap_or_default();
}
let path = std::path::PathBuf::from(dir);
```

---

## Responsive Breakpoints

Ratic uses a 700 sp breakpoint to switch from the wide ViewSwitcher to the narrow ViewSwitcherBar.

```rust
use relm4::adw::{BreakpointConditionLengthType, LengthUnit};

// Create the breakpoint (store in the component model so it lives long enough):
let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
    BreakpointConditionLengthType::MaxWidth,
    700.0,
    LengthUnit::Sp,
));

// In the view! macro, attach to the ApplicationWindow:
adw::ApplicationWindow {
    add_breakpoint: model.breakpoint.clone(),
    // ...
}
```

Add setters to the breakpoint to show/hide widgets when the condition is met:

```rust
breakpoint.add_setter(&widgets.bottom_bar, "reveal", &true.to_value());
breakpoint.add_setter(&widgets.top_menu,   "visible", &false.to_value());
```

---

## Toasts — Non-Intrusive Notifications

Wrap your content in an `adw::ToastOverlay`:

```rust
// In the model:
pub toasts: adw::ToastOverlay,

// In init():
let toasts = adw::ToastOverlay::new();
// The overlay's child is set after view_output!() so the widget tree is available:
toasts.set_child(Some(&widgets.content));

// In the view! macro, add the overlay as an overlay on top of your content:
gtk::Overlay {
    gtk::Picture { /* background */ },
    add_overlay = &model.toasts.clone(),
}
```

Show a toast (can be called from `update`):

```rust
self.toasts.add_toast(adw::Toast::new("Library loaded"));
// Or with a localized string:
self.toasts.add_toast(adw::Toast::new(fl!("library-load-end")));
```

---

## Dynamic CSS

Apply CSS that can change at runtime (e.g., tinting background based on cover art):

```rust
use relm4::gtk;

// In the model:
pub content_provider: gtk::CssProvider,

// In init(), register the provider for the widget's display:
let content_display = widgets.content.display();
gtk::style_context_add_provider_for_display(
    &content_display,
    &model.content_provider,
    gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
);

// In update(), update the CSS string:
self.content_provider.load_from_string(&format!(
    ".content {{ background: rgba({r}, {g}, {b}, 0.3); }}"
));
```

For static global CSS (applied once at startup):

```rust
relm4::set_global_css("
    .sidebar-pane { background: rgba(0, 0, 0, 0); }
");
```

---

## Background Image Blending

Use `gtk::Overlay` with a `gtk::Picture` at the bottom and content widgets as overlays:

```rust
view! {
    gtk::Overlay {
        // Bottom layer: the background image.
        gtk::Picture {
            set_content_fit: ContentFit::Cover,
            set_hexpand: true,
            set_vexpand: true,
            // `#[watch]` re-evaluates when `update` is called.
            #[watch]
            set_paintable: model.background.as_ref(),
        },

        // Top layer: actual content.
        add_overlay = &model.toasts.clone(),
    }
}
```

Store the background as `Option<Texture>` in the model. Setting it to `None` hides the picture.

---

## Menus

```rust
use std::sync::LazyLock;
use crate::fl;

// Static labels (needed because menu! macro requires &str at compile time).
static MENU_SETTINGS: LazyLock<String> = LazyLock::new(|| fl!("main-menu-settings").to_owned());
static MENU_ABOUT: LazyLock<String>    = LazyLock::new(|| fl!("main-menu-about").to_owned());

// Inside view! (or just before it):
menu! {
    main_menu: {
        section! {
            &MENU_SETTINGS => SettingsAction,
            &MENU_ABOUT    => AboutAction,
        }
    }
}

// Attach to a MenuButton in view!:
gtk::MenuButton {
    set_primary: true,
    set_icon_name: "view-more",
    set_menu_model: Some(&main_menu),
}
```

---

## `OpenDialog` from relm4-components

Use for file/folder picker dialogs without writing GTK file-chooser boilerplate:

```rust
use relm4_components::open_dialog::{
    OpenDialog, OpenDialogMsg, OpenDialogResponse, OpenDialogSettings,
};

// In init():
let mut filter = gtk::FileFilter::new();
filter.add_mime_type("inode/directory");

let dialog_settings = OpenDialogSettings {
    folder_mode: true,
    cancel_label: fl!("cancel").to_owned(),
    accept_label: fl!("open").to_owned(),
    create_folders: true,
    is_modal: true,
    filters: vec![filter],
};

let dialog = OpenDialog::builder()
    .transient_for_native(&root)   // centers over the parent window
    .launch(dialog_settings)
    .forward(sender.input_sender(), |msg| match msg {
        OpenDialogResponse::Accept(path) => Controls::PathChosen(path),
        OpenDialogResponse::Cancel => Controls::Ignore,
    });

// Trigger it:
dialog.emit(OpenDialogMsg::Open);
```

---

## ViewStack + ViewSwitcher Responsive Navigation

The standard GNOME pattern: wide switcher in the header bar, narrow bar at the bottom, breakpoint toggles between them.

```rust
// In init():
let view_stack = adw::ViewStack::builder()
    .name("view_stack")
    .vexpand(true)
    .build();

// Add pages:
view_stack.add_titled_with_icon(
    picker.widget(),
    Some("library"),
    fl!("view-library"),
    "music-note-outline",
);
view_stack.add_titled_with_icon(
    albums.widget(),
    Some("albums"),
    fl!("view-albums"),
    "library-music",
);

// In view! (inside ToolbarView):
add_top_bar = &adw::HeaderBar {
    #[name(top_switcher)]
    #[wrap(Some)]
    set_title_widget = &adw::ViewSwitcher {
        set_policy: ViewSwitcherPolicy::Wide,
    },
},
#[name(bottom_bar)]
add_bottom_bar = &adw::ViewSwitcherBar {},

// After view_output!():
widgets.top_switcher.set_stack(Some(&view_stack));
widgets.bottom_bar.set_stack(Some(&view_stack));
```

---

## `adw::OverlaySplitView` — Sidebar Layout

```rust
// In view!:
adw::OverlaySplitView {
    // Main content fills the left/right area.
    #[wrap(Some)]
    set_content = &gtk::Box {
        set_orientation: Orientation::Vertical,
        append: &model.view_stack,
    },

    // Sidebar (e.g., the playback controls).
    set_sidebar: Some(model.player.widget()),

    set_sidebar_position: gtk::PackType::End,
    set_sidebar_width_fraction: 0.5,
    set_min_sidebar_width: 300.0,
    set_max_sidebar_width: 500.0,
}
```

Hide the sidebar with a breakpoint setter when the window is narrow:

```rust
breakpoint.add_setter(
    &widgets.split_view,
    "collapsed",
    &true.to_value(),
);
```

---

## `#[watch]` — Reactive Widget Bindings

Any property set with `#[watch]` in the `view!` macro is re-evaluated every time `update` or `update_with_view` is called. Use it for model-derived state:

```rust
view! {
    gtk::Label {
        #[watch]
        set_label: &model.current_title,
    }

    gtk::Switch {
        #[watch]
        set_active: model.is_enabled,
    }

    gtk::Picture {
        #[watch]
        set_paintable: model.background.as_ref(),
    }
}
```

Do not use `#[watch]` for properties that never change after `init` — it adds overhead.
