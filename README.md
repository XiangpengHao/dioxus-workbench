# dioxus-workbench

[![crates.io](https://img.shields.io/crates/v/dioxus-workbench.svg)](https://crates.io/crates/dioxus-workbench)
[![CI](https://github.com/XiangpengHao/dioxus-workbench/actions/workflows/ci.yml/badge.svg)](https://github.com/XiangpengHao/dioxus-workbench/actions/workflows/ci.yml)

A dockable panel workbench for [Dioxus](https://dioxuslabs.com): tabbed panel
groups, drag-to-dock, resizable splits, an activity rail, and a status bar.

<!-- Absolute URL on purpose: the published crate's readme is `../README.md`, so
     crates.io resolves relative paths against `dioxus-workbench/`, not the repo root. -->
![dioxus-workbench demo](https://raw.githubusercontent.com/XiangpengHao/dioxus-workbench/main/assets/screenshot.png)

*The demo is a small fake IDE — three activities, each with its own persisted
layout, plus a settings screen. Run it with `cd demo && dx serve`.*

## Features

- **Tabbed panel groups** — click a tab to activate, drag it to rearrange.
- **Drag-to-dock** — drop on a group's center to attach a tab, on an edge to
  split, with live drop previews.
- **Resizable splits** — drag, arrow-key (Shift for bigger steps), or
  double-click to reset.
- **Serializable layouts** — the split tree is a small serde value:
  `encode()` on change, `decode()` next session.
- **Layout reconciliation** — vanished panels are pruned, late arrivals join
  their home tile, stale empty groups collapse.
- **Keyboard & accessibility** — real tab/tablist/tabpanel semantics,
  focusable separators with ARIA values, `Alt+Shift+Arrow` to split,
  `Alt+Shift+PageUp/PageDown` to move between groups, `prefers-reduced-motion`.
- **Renderer-agnostic** — no `web-sys`, no JavaScript dependencies:
  measurement and pointer capture go through Dioxus's own document APIs, so
  web and desktop behave the same.
- **Dark mode** out of the box, themable through `--wb-*` variables.

## Quickstart

Two concepts get you a working workbench: panels, and the workspace that
arranges them.

```rust
use dioxus::prelude::*;
use dioxus_workbench::prelude::*;

#[component]
fn App() -> Element {
    let panels = vec![
        Panel::new("files", "Files", "side", rsx! { FileTree {} }),
        Panel::new("editor", "Editor", "main", rsx! { Editor {} }),
        Panel::new("terminal", "Terminal", "bottom", rsx! { Terminal {} }),
    ];

    rsx! {
        div { style: "width: 100vw; height: 100vh;",
            PanelWorkspace { panels }
        }
    }
}
```

The workspace fills its parent, so give the parent a size. With no
`initial_layout` it derives one from the panels' homes — one tile per distinct
`home`, split left to right (`PanelLayout::from_homes`). Describe the tree
yourself when you outgrow that:

```rust
let layout = PanelLayout::new(LayoutNode::split(
    "root",
    SplitAxis::Horizontal,
    0.25,
    LayoutNode::tile("side", ["files"]),
    LayoutNode::split(
        "main-rows",
        SplitAxis::Vertical,
        0.7,
        LayoutNode::tile("main", ["editor"]),
        LayoutNode::tile("bottom", ["terminal"]),
    ),
));

rsx! { PanelWorkspace { panels, initial_layout: layout } }
```

The application owns panel content and the first-open arrangement; the
workspace owns every later dock, tab, and resize.

Panel ids identify content; tile and split ids identify layout positions. Keep
all three stable across releases when persisted layouts need to survive. A
panel's `home` is used only when that panel is newly discovered and absent
from a restored layout.

## The shell

`Workbench` frames the workspace with an optional activity rail and status
bar. Both are slots — the library draws the chrome, you say what it means:

```rust
rsx! {
    Workbench {
        rail: rsx! {
            ActivityRail {
                ActivityButton {
                    label: "Code",
                    icon: rsx! { CodeIcon {} },
                    active: activity() == Activity::Code,
                    onclick: move |_| activity.set(Activity::Code),
                }
                ActivityButton { label: "Settings", bottom: true, onclick: move |_| { /* … */ } }
            }
        },
        status: rsx! {
            StatusBar {
                left: rsx! { StatusItem { StatusDot { tone: StatusTone::Good } "Connected" } },
                message: rsx! { StatusMessage { "Saved layout" } },
                right: rsx! { StatusItem { tone: StatusTone::Accent, "main · 12:04" } },
            }
        },
        PanelWorkspace { panels }
    }
}
```

`ActivityButton { bottom: true }` pins an item (and any after it) to the
rail's end — the conventional place for settings. A `StatusItem` with an
`onclick` renders as a button; without one it is inert text.

## Persistence

Layouts serialize to a compact JSON string. Where it lives is up to you —
local storage, a settings file, a server:

```rust
PanelWorkspace {
    panels,
    initial_layout: saved.unwrap_or_else(|| default_layout.clone()),
    // Splitter double-click resets to the canonical arrangement,
    // not to whatever happened to be saved.
    reset_layout: default_layout,
    on_layout_change: move |layout: PanelLayout| {
        if let Some(encoded) = layout.encode() {
            save_somewhere("workspace-layout", encoded);
        }
    },
}
```

`on_layout_change` fires once per *settled* mutation — a drop, a keyboard
step, a finished splitter drag — never per pointer move, so persisting inside
it is safe.

On restore, `decode` rejects stale or corrupt values and the workspace
reconciles the tree against the current registry. When "it didn't decode" is
not enough, `try_decode` and `try_encode` return a `LayoutError` naming the
cause; `LayoutError::Version { found }` is the migration hook — the raw string
is still yours, so translate an old format instead of silently discarding
every user's saved arrangement.

The `layout_scope` prop reloads the layout when the host switches context
(another document, another screen) while the component stays mounted. The demo
persists one layout per activity this way.

## Dynamic panels

The `panels` prop is the registry: render it from state and panels come and go
at runtime. `.with_closable(true)` gives a tab a close affordance; handle
`on_panel_close(PanelId)` by removing the panel from your registry.
Keeping the panel registered leaves selection and focus unchanged. Removal completes
the close, including after asynchronous confirmation. Workbench chooses
the right neighbor, then the left, in the panel's actual group. If reconciliation
prunes that group, focus moves to the active tab of the group inheriting its space.

`active_panel` brings a tab to the front programmatically — a notification's
"show me" action. It is edge-triggered with memory: it applies once when the
value changes, and a standing `Some` never re-asserts itself when the registry
changes or the user picks another tab. Pass `None` in between to request the
same panel twice.

## Panel state

Panel content lives in a stable host keyed by panel ID. Tab switches, structural
docks, and splits preserve component signals, form values, and scroll positions.
Removing a panel from the registry unmounts it. Reopening creates fresh state.
Keep IDs unique and keep the content component type stable for this guarantee.

The recursive chrome tree contains measured content slots; stable hosts follow
those slots without reparenting application DOM. Measurements use Dioxus's
`document::eval` bridge and ResizeObserver on both web and desktop, with no
web-sys dependency. Observers reconnect after structural moves and disconnect
when hidden or removed. This requires a document-capable renderer.

### Panel lifecycle

Inside panel content, `PanelContext::current()` provides reactive geometry:

```rust,ignore
let panel = PanelContext::current();
use_effect(move || {
    let box_size = panel.geometry();
    if box_size.visible() {
        // Resize your renderer using box_size.width() / box_size.height().
    }
});
```

Dimensions are CSS pixels for the panel's content area. Invisible panels report
zero dimensions. Updates cover ancestor visibility, tab activation, window
resizing, and live splitter drags. Nested charts can still observe their own
smaller element boxes; the panel does not dictate their internal arrangement.
`panel.request_close()` uses the same application acceptance path as the tab's
close control. Closing remains an application decision.

### Group toolbar

`group_toolbar` accepts a callback from `GroupContext` to `Element`. The context
exposes `tile()` and `active_panel()`. It is rendered beside the native tab strip;
workbench owns its sizing, overflow, and placement when groups are docked.
Return an empty element for groups without actions. Toolbar content belongs to
the group chrome and may remount during docking; keep durable state in the panel
or application, not inside a toolbar control.

```rust,ignore
PanelWorkspace {
    panels,
    group_toolbar: move |group: GroupContext| rsx! {
        button { onclick: move |_| open_for(group.tile()), "Load" }
    },
    on_panel_close: move |closing: PanelId| {
        opened.write().retain(|panel| panel.id != closing);
    },
}
```

## Runtime control

For a View menu, a command palette, or a reset command, hand the workspace an
application-owned signal and mutate it with `PanelLayout`'s own methods:

```rust
let mut layout = use_signal(|| default_layout.clone());

rsx! {
    button {
        // "Move the query panel in beside the records table."
        onclick: move |_| {
            layout.write().dock_panel(
                &PanelId::from("query"),
                &TileId::from("table"),
                DockZone::Right,
            );
        },
        "Query beside records"
    }
    PanelWorkspace { panels, layout }
}
```

The signal must stay the same signal for the component's lifetime. The
workspace writes its own mutations (drags, keyboard, tab clicks) to it and
still reports them through `on_layout_change`; your own writes are not echoed
back, since you already know about them.

## Extending the chrome

Tabs take two application-owned slots — a leading icon and a trailing
accessory (a dirty dot, an unread badge, a spinner):

```rust
Panel::new("editor", "main.rs", "main", rsx! { Editor {} })
    .with_tab_icon(rsx! { RustFileIcon {} })
    .with_tab_accessory(rsx! { span { class: "dirty-dot", "aria-label": "Unsaved changes" } })
```

`on_tab_menu` delivers context-menu gestures on tabs — panel, tile, and
pointer position — and suppresses the native menu when provided. Drawing the
menu stays your job. Every string the chrome draws itself (empty states,
labels, tooltips, accessible names) is overridable through `WorkbenchStrings`:

```rust
PanelWorkspace {
    panels,
    on_tab_menu: move |request: TabMenuRequest| menu.set(Some(request)),
    strings: WorkbenchStrings {
        empty_tile_hint: "Déposez un onglet ici, ou fermez ce groupe.".into(),
        ..Default::default()
    },
}
```

## Theming

Every color and structural size flows through a `--wb-*` variable. Override
them on `.wb-shell` / `.wb-workspace` or any ancestor:

```css
.wb-shell {
  --wb-accent: #10b981;
  --wb-canvas: #fdfdfc;
  --wb-rail-size: 4.5rem;
  /* see dioxus-workbench/src/style.css for the full list */
}
```

Dark mode follows `prefers-color-scheme` automatically. The stylesheet is
compiled into the crate and injected once per tree; to serve it yourself, it
is exported as `dioxus_workbench::STYLESHEET`.

## Events

| Prop | Fires when |
| --- | --- |
| `on_layout_change` | a settled layout mutation — dock, split, activation, finished resize — with the full serializable layout |
| `on_panel_activate` | a panel's tab comes to the front, by pointer, keyboard, drop, or `active_panel` |
| `on_panel_close` | a close is requested; remove it from `panels` to complete closing |
| `on_resize` | panel geometry settled, for content that measures its container |
| `on_tab_menu` | a context-menu gesture on a tab, with panel, tile, and pointer position |

## Development

The repo is a workspace: `dioxus-workbench/` is the library, `demo/` is the
demo app. The Nix flake provides everything (Rust with the wasm target, `dx`,
`wasm-opt`); CI runs fmt, clippy, tests, and a wasm check through the same
flake.

```bash
nix develop                        # or direnv
cargo test -p dioxus-workbench
cd demo && dx serve                # http://127.0.0.1:8080
```

For release builds, pass `--debug-symbols false` — the nixpkgs `wasm-opt` is
newer than the one dx pins and crashes on the debug info dx keeps by default:
`dx build --release --debug-symbols false`.

## Roadmap

- Floating panels and maximize-a-group
- Tab overflow menus for very crowded groups
- Optional tab reordering within a group
- Replaceable chrome icons

Keyboard focus bridges the stable host layer: Tab from a group's tablist
focuses its active tabpanel; Shift+Tab on the tabpanel itself returns to its tab.
Descendant controls retain their own keyboard behavior. Sequential traversal
out of panel content still follows host DOM order rather than visual group order.
