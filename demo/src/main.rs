//! The demo app: a small fake IDE that exercises every part of the kit —
//! activities with their own persisted workspace layouts, dockable and
//! closable panels, and a live status bar.

use dioxus::document;
use dioxus::prelude::*;
use dioxus_workbench::prelude::*;

mod icons;
mod panels;

use icons::{BookIcon, CodeIcon, DataIcon, GearIcon};
use panels::{Chart, Editor, FileTree, Problems, Query, QuickStart, Readme, Records, Terminal};

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Activity {
    Code,
    Data,
    Docs,
    Settings,
}

impl Activity {
    const fn label(self) -> &'static str {
        match self {
            Self::Code => "Code",
            Self::Data => "Data",
            Self::Docs => "Docs",
            Self::Settings => "Settings",
        }
    }

    const fn scope(self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Data => "data",
            Self::Docs => "docs",
            Self::Settings => "settings",
        }
    }
}

#[component]
fn App() -> Element {
    let mut activity = use_signal(|| Activity::Code);
    // The latest thing that happened, spoken once in the status bar.
    let mut message = use_signal(String::new);
    // The Preview panel can be closed from its tab and reopened from the
    // editor, exercising a registry that changes at runtime.
    let mut preview_open = use_signal(|| false);

    let current = activity();
    let rail = rsx! {
        ActivityRail {
            ActivityButton {
                label: "Code",
                icon: rsx! { CodeIcon {} },
                active: current == Activity::Code,
                onclick: move |_| {
                    activity.set(Activity::Code);
                    message.set(String::new());
                },
            }
            ActivityButton {
                label: "Data",
                icon: rsx! { DataIcon {} },
                active: current == Activity::Data,
                onclick: move |_| {
                    activity.set(Activity::Data);
                    message.set(String::new());
                },
            }
            ActivityButton {
                label: "Docs",
                icon: rsx! { BookIcon {} },
                active: current == Activity::Docs,
                onclick: move |_| {
                    activity.set(Activity::Docs);
                    message.set(String::new());
                },
            }
            ActivityButton {
                label: "Settings",
                icon: rsx! { GearIcon {} },
                active: current == Activity::Settings,
                bottom: true,
                onclick: move |_| {
                    activity.set(Activity::Settings);
                    message.set(String::new());
                },
            }
        }
    };

    let latest = message();
    let status = rsx! {
        StatusBar {
            left: rsx! {
                StatusItem { title: "Everything in this demo is fake but the workbench",
                    StatusDot { tone: StatusTone::Good }
                    "Ready"
                }
                StatusItem { "demo-project" }
                StatusItem { "{current.label()}" }
            },
            message: if latest.is_empty() { None } else { Some(rsx! {
                StatusMessage { "{latest}" }
            }) },
            right: rsx! {
                if current == Activity::Code {
                    StatusItem { tone: StatusTone::Caution, "2 problems" }
                }
                StatusItem { tone: StatusTone::Accent, "Layouts persist per activity" }
                StatusItem {
                    title: "Clear the saved layouts and reload",
                    onclick: move |_| {
                        spawn(async move {
                            let _ = document::eval(
                                "for (const key of Object.keys(window.localStorage)) { \
                                     if (key.startsWith('dioxus-workbench-demo/')) { \
                                         window.localStorage.removeItem(key); \
                                     } \
                                 } \
                                 window.location.reload();",
                            )
                            .await;
                        });
                    },
                    "Reset layouts"
                }
            },
        }
    };

    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        div { class: "demo-viewport",
            Workbench {
                rail,
                status,
                if current == Activity::Settings {
                    SettingsScreen {}
                } else {
                    ActivityWorkspace {
                        activity: current,
                        preview_open,
                        on_event: move |note: String| message.set(note),
                        on_preview_change: move |open| preview_open.set(open),
                    }
                }
            }
        }
    }
}

/// One workspace per activity, each restored from local storage and saved on
/// every mutation. The workspace stays mounted across activity switches;
/// `layout_scope` is what reloads it.
#[component]
fn ActivityWorkspace(
    activity: Activity,
    preview_open: ReadSignal<bool>,
    on_event: EventHandler<String>,
    on_preview_change: EventHandler<bool>,
) -> Element {
    let storage_key = format!("dioxus-workbench-demo/{}", activity.scope());
    let restore_key = storage_key.clone();
    let restored = use_resource(use_reactive((&restore_key,), |(key,)| async move {
        let layout = load_layout(&key).await;
        (key, layout)
    }));

    // Wait for storage before the first render so a saved layout is the
    // initial one, not a correction applied after the default flashes. The
    // key check matters: while a switch is loading, the resource still holds
    // the previous activity's value, and `initial_layout` and `layout_scope`
    // must always describe the same activity.
    let saved = match restored.read().as_ref() {
        Some((key, saved)) if key == &storage_key => saved.clone(),
        _ => {
            return rsx! {
                div { class: "demo-loading" }
            };
        }
    };

    let (panels, default_layout) = match activity {
        Activity::Code => code_workspace(preview_open(), on_event, on_preview_change),
        Activity::Data => data_workspace(),
        Activity::Docs | Activity::Settings => docs_workspace(),
    };
    let initial_layout = saved.unwrap_or_else(|| default_layout.clone());

    rsx! {
        PanelWorkspace {
            panels,
            initial_layout,
            reset_layout: default_layout,
            layout_scope: activity.scope().to_owned(),
            on_layout_change: move |layout: PanelLayout| {
                save_layout(&storage_key, &layout);
            },
            on_panel_activate: move |panel: PanelId| {
                on_event.call(format!("Opened {panel}"));
            },
            on_panel_close: move |panel: PanelId| {
                if panel.as_str() == "preview" {
                    on_preview_change.call(false);
                    on_event.call("Closed Preview".to_owned());
                }
            },
        }
    }
}

fn code_workspace(
    preview_open: bool,
    on_event: EventHandler<String>,
    on_preview_change: EventHandler<bool>,
) -> (Vec<Panel>, PanelLayout) {
    let mut panels = vec![
        Panel::new("files", "Files", "side", rsx! { FileTree {} }),
        Panel::new(
            "editor",
            "main.rs",
            "main",
            rsx! {
                Editor {
                    preview_open,
                    on_open_preview: move |_| {
                        on_preview_change.call(true);
                        on_event.call("Opened Preview beside the editor".to_owned());
                    },
                }
            },
        ),
        Panel::new("readme", "README.md", "main", rsx! { Readme {} }),
        Panel::new("terminal", "Terminal", "bottom", rsx! { Terminal {} }),
        Panel::new("problems", "Problems", "bottom", rsx! { Problems {} }),
    ];
    if preview_open {
        panels.push(
            Panel::new("preview", "Preview", "main", rsx! { panels::Preview {} })
                .with_home_zone(DockZone::Right)
                .with_closable(true),
        );
    }

    let layout = PanelLayout::new(LayoutNode::split(
        "root",
        SplitAxis::Horizontal,
        0.22,
        LayoutNode::tile("side", ["files"]),
        LayoutNode::split(
            "main-rows",
            SplitAxis::Vertical,
            0.68,
            LayoutNode::tile("main", ["editor", "readme"]),
            LayoutNode::tile("bottom", ["terminal", "problems"]),
        ),
    ));
    (panels, layout)
}

fn data_workspace() -> (Vec<Panel>, PanelLayout) {
    let panels = vec![
        Panel::new("records", "Records", "table", rsx! { Records {} }),
        Panel::new("chart", "Chart", "chart", rsx! { Chart {} }),
        Panel::new("query", "Query", "query", rsx! { Query {} }),
    ];
    let layout = PanelLayout::new(LayoutNode::split(
        "root",
        SplitAxis::Vertical,
        0.6,
        LayoutNode::split(
            "top-columns",
            SplitAxis::Horizontal,
            0.55,
            LayoutNode::tile("table", ["records"]),
            LayoutNode::tile("chart", ["chart"]),
        ),
        LayoutNode::tile("query", ["query"]),
    ));
    (panels, layout)
}

fn docs_workspace() -> (Vec<Panel>, PanelLayout) {
    let panels = vec![Panel::new(
        "quickstart",
        "Quick start",
        "docs",
        rsx! { QuickStart {} },
    )];
    (panels, PanelLayout::single("docs", "quickstart"))
}

#[component]
fn SettingsScreen() -> Element {
    rsx! {
        div { class: "demo-settings",
            h1 { "Settings" }
            p {
                "The main region is an ordinary slot: this screen replaces the "
                "panel workspace entirely while the rail and status bar stay put."
            }
            label { class: "demo-settings-row",
                input { r#type: "checkbox", checked: true }
                span { "Restore panel layouts between sessions" }
            }
            label { class: "demo-settings-row",
                input { r#type: "checkbox" }
                span { "Confirm before closing panels" }
            }
        }
    }
}

/// Where the demo keeps layouts. The library stays storage-agnostic:
/// `PanelLayout::encode`/`decode` produce the string, the application owns
/// where it lives.
async fn load_layout(key: &str) -> Option<PanelLayout> {
    let script = format!(
        "return window.localStorage.getItem({});",
        serde_json_string(key)
    );
    let value = document::eval(&script).await.ok()?;
    PanelLayout::decode(value.as_str()?)
}

fn save_layout(key: &str, layout: &PanelLayout) {
    let Some(encoded) = layout.encode() else {
        return;
    };
    let script = format!(
        "window.localStorage.setItem({}, {});",
        serde_json_string(key),
        serde_json_string(&encoded)
    );
    spawn(async move {
        let _ = document::eval(&script).await;
    });
}

/// JSON string syntax is a subset of JavaScript's, making interpolation safe.
fn serde_json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            control if (control as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", control as u32));
            }
            other => out.push(other),
        }
    }
    out.push('"');
    out
}
