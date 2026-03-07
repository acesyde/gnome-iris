# Relm4 Component Patterns

Relm4 has three component traits. Choose based on whether the component has a widget tree and whether it needs async tasks.

| Trait | Has Widget Tree | Async | Typical Use |
|-------|----------------|-------|-------------|
| `SimpleComponent` | Yes | No | Dialogs, static views |
| `Component` | Yes | No (but has `CommandOutput`) | Orchestrators with one-shot commands |
| `AsyncComponent` / `SimpleAsyncComponent` | Optional | Yes | Background workers, audio player, library loader |

---

## 1. `SimpleComponent` — Dialogs and Static Views

Use for components that only respond to messages synchronously and own a GTK widget tree.

**Real example:** `About`, `Settings`, `Window` (root).

```rust
//! My dialog component.

use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw};
use crate::ui::window::ApplicationActionGroup;
use crate::fl;

/// Declare an action that lives in the app-wide action group.
relm4::new_stateless_action!(pub MyDialogAction, ApplicationActionGroup, "my-dialog");

/// Component model.
pub struct MyDialog {
    /// Some reactive state.
    is_active: bool,
}

/// Input messages — always named `Controls` by convention.
#[derive(Debug)]
pub enum Controls {
    /// Toggle the active state.
    Toggle,
    /// No-op (used for cancelled dialogs, etc.).
    Ignore,
}

#[relm4::component(pub)]
impl SimpleComponent for MyDialog {
    /// Data passed to `init`. Use `()` for dialogs that need no setup data.
    type Init = ();
    type Input = Controls;
    /// Type of messages this component emits to its parent.
    type Output = ();

    view! {
        /// The `#[name(dialog)]` attribute lets parent code do `.widgets().dialog`.
        #[name(dialog)]
        adw::Dialog {
            set_title: fl!("my-dialog-title"),

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {},

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 12,

                    gtk::Switch {
                        /// `#[watch]` re-evaluates this property whenever `update` is called.
                        #[watch]
                        set_active: model.is_active,

                        connect_active_notify[sender] => move |sw| {
                            if sw.is_active() {
                                sender.input_sender().emit(Controls::Toggle);
                            }
                        },
                    },
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self { is_active: false };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            Controls::Toggle => self.is_active = !self.is_active,
            Controls::Ignore => {},
        }
    }
}
```

### Wiring a `SimpleComponent` from a Parent

```rust
// Launch and detach (no output forwarding — used for dialogs):
let about = About::builder().launch(()).detach();

// Present when an action fires:
let about_action = RelmAction::<AboutAction>::new_stateless(glib::clone!(
    #[weak] window,
    move |_| about.widgets().dialog.present(Some(&window))
));

// Launch with output forwarded to the parent's input:
let settings = Settings::builder()
    .launch(())
    .forward(parent_sender.input_sender(), |msg| msg);
```

---

## 2. `Component` — Orchestrators with `CommandOutput`

Use when the component needs to fire one-shot async work (e.g., spawning a background task that sends back a single result) but otherwise behaves synchronously.

**Real example:** `Header` (coordinates all children, routes messages).

```rust
use relm4::{Component, ComponentParts, ComponentSender, adw};

pub struct MyOrchestrator {
    some_child: relm4::Controller<SomeChild>,
}

pub enum Signal {
    DoSomething,
}

#[relm4::component(pub)]
impl Component for MyOrchestrator {
    /// `!` means no command output; replace with a type if needed.
    type CommandOutput = !;
    type Init = ();
    type Input = Signal;
    type Output = ();

    view! {
        #[root]
        adw::ApplicationWindow {
            set_content: Some(model.some_child.widget()),
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let some_child = SomeChild::builder()
            .launch(())
            .forward(sender.input_sender(), |_| Signal::DoSomething);

        let model = Self { some_child };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(
        &mut self,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            Signal::DoSomething => { /* ... */ },
        }
    }
}
```

---

## 3. `AsyncComponent` / `SimpleAsyncComponent` — Background Workers

Use when the component needs `async fn update`. `SimpleAsyncComponent` is for headless workers (no widget tree); `AsyncComponent` is for async components with widgets.

**Real example:** `Library` (headless async worker), `Player`, `Picker`, `Grid`.

### Headless Worker (`SimpleAsyncComponent`)

```rust
use relm4::prelude::{AsyncComponentParts, SimpleAsyncComponent};
use relm4::AsyncComponentSender;

/// Headless background worker — no widget tree.
pub struct MyWorker {
    cache: crate::<domain>::cache::Shared<MyCache>,
}

#[derive(Debug)]
pub enum Controls {
    LoadData(std::path::PathBuf),
    DataLoaded(MyData),
}

#[derive(Debug)]
pub enum Output {
    Finished(MyData),
}

impl SimpleAsyncComponent for MyWorker {
    type Init = crate::<domain>::cache::Shared<MyCache>;
    type Input = Controls;
    type Output = Output;
    /// Headless: root and widgets are both `()`.
    type Root = ();
    type Widgets = ();

    fn init_root() -> Self::Root {}

    async fn init(
        cache: Self::Init,
        _root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let model = Self { cache };
        AsyncComponentParts { model, widgets: () }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            Controls::LoadData(path) => {
                let cache = self.cache.clone();
                let tx = sender.input_sender().clone();
                relm4::spawn(async move {
                    // Do async work...
                    let data = load(&path, cache).await.unwrap();
                    tx.emit(Controls::DataLoaded(data));
                });
            },
            Controls::DataLoaded(data) => {
                sender.output_sender().emit(Output::Finished(data));
            },
        }
    }
}
```

### Async Component with Widgets (`AsyncComponent`)

```rust
use relm4::prelude::{AsyncComponent, AsyncComponentParts};
use relm4::AsyncComponentSender;

pub struct MyAsyncView {
    items: Vec<String>,
}

#[derive(Debug)]
pub enum Controls { Refresh }

/// Command messages arrive via `update_cmd`, separate from user messages.
#[derive(Debug)]
pub enum CommandMsg { ItemsLoaded(Vec<String>) }

impl AsyncComponent for MyAsyncView {
    type CommandOutput = CommandMsg;
    type Init = ();
    type Input = Controls;
    type Output = ();

    view! {
        #[root]
        gtk::Box { /* ... */ }
    }

    async fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let model = Self { items: vec![] };
        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            Controls::Refresh => {
                sender.oneshot_command(async {
                    // This runs in a background task; result arrives in update_cmd.
                    let items = fetch_items().await;
                    CommandMsg::ItemsLoaded(items)
                });
            },
        }
    }

    async fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        _sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            CommandMsg::ItemsLoaded(items) => self.items = items,
        }
    }
}
```

---

## Wiring Patterns

### Standard: `.builder().launch(init).forward(sender, mapper)`

```rust
let child = MyChild::builder()
    .launch(init_data)
    .forward(sender.input_sender(), |child_output| {
        // Map child output to parent input:
        ParentInput::ChildDone(child_output)
    });
```

### Detached: `.builder().launch(init).detach()`

Used for dialogs that are shown imperatively (not driven by parent state). The parent keeps the `Controller` to call `.widgets()` and `.emit()`.

```rust
let about = About::builder().launch(()).detach();
// Later, in an action callback:
about.widgets().dialog.present(Some(&window));
```

### Shared child: `Arc<AsyncController<T>>`

When multiple siblings need to emit messages to the same child, wrap in `Arc`:

```rust
let library = Arc::new(
    Library::builder()
        .launch(init)
        .forward(sender.input_sender(), map_fn)
);

// Now both `albums` and `picker` can hold a clone:
let albums = Grid::builder()
    .launch((library.clone(), cache.clone()))
    .forward(sender.input_sender(), map_fn);
```

---

## Actions

Actions decouple menu items / keyboard shortcuts from component logic.

```rust
// In window.rs — declare the action group:
relm4::new_action_group!(pub ApplicationActionGroup, "app");

// In about.rs — declare a stateless action in that group:
relm4::new_stateless_action!(pub AboutAction, ApplicationActionGroup, "about");

// In window.rs init() — wire up and register:
let mut group = RelmActionGroup::<ApplicationActionGroup>::new();

let about_action = RelmAction::<AboutAction>::new_stateless(glib::clone!(
    #[weak] window,
    move |_| about.widgets().dialog.present(Some(&window))
));
group.add_action(about_action);
group.register_for_widget(&root);
```

The action name (`"app.about"`) can then be used in menus via the `menu!` macro.

---

## Naming Conventions

| Role | Name |
|------|------|
| Component input enum | `Controls` |
| Component output enum | `Signal` or `Output` |
| Async command output enum | `CommandMsg` |
| Root component | `Window` |
| Orchestrator with ViewStack | `Header` |
| Headless async worker | descriptive noun, e.g. `Library` |
