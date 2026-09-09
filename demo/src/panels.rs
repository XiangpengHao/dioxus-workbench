//! Panel content for the demo. Everything here is application territory:
//! the workbench only ever sees these as opaque `Element`s.

use dioxus::prelude::*;

#[component]
pub fn FileTree() -> Element {
    let mut selected = use_signal(|| "main.rs");
    let files = [
        ("src/", true),
        ("main.rs", false),
        ("panels.rs", false),
        ("icons.rs", false),
        ("assets/", true),
        ("main.css", false),
        ("Cargo.toml", false),
        ("README.md", false),
    ];
    rsx! {
        div { class: "demo-pane demo-files",
            ul {
                for (name, is_dir) in files {
                    li {
                        button {
                            r#type: "button",
                            class: if selected() == name { "demo-file demo-file-active" } else { "demo-file" },
                            class: if is_dir { "demo-file-dir" },
                            disabled: is_dir,
                            onclick: move |_| selected.set(name),
                            "{name}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Editor(preview_open: bool, on_open_preview: EventHandler<()>) -> Element {
    rsx! {
        div { class: "demo-pane demo-editor",
            div { class: "demo-editor-toolbar",
                span { "src/main.rs" }
                button {
                    r#type: "button",
                    class: "demo-button",
                    disabled: preview_open,
                    onclick: move |_| on_open_preview.call(()),
                    if preview_open { "Preview is open" } else { "Open preview" }
                }
            }
            pre { class: "demo-code",
                code {
                    span { class: "demo-code-comment", "// Panels are ordinary Dioxus elements." }
                    "\n"
                    span { class: "demo-code-keyword", "use" }
                    " dioxus_workbench::prelude::*;\n\n"
                    span { class: "demo-code-comment", "// Drag this file's tab onto another group," }
                    "\n"
                    span { class: "demo-code-comment", "// or onto an edge of one, to re-dock it." }
                    "\n"
                    span { class: "demo-code-keyword", "fn" }
                    " "
                    span { class: "demo-code-fn", "main" }
                    "() {{\n    dioxus::launch(App);\n}}\n"
                }
            }
        }
    }
}

#[component]
pub fn Readme() -> Element {
    rsx! {
        div { class: "demo-pane demo-prose",
            h2 { "Try the workbench" }
            ul {
                li { "Drag a tab onto the center of a group to attach it as a tab." }
                li { "Drag a tab onto an edge of a group to split it." }
                li { "Drag the separators to resize; double-click one to reset it." }
                li { "Alt+Shift+Arrow splits the focused tab from the keyboard." }
                li { "Every edit is saved: reload the page and the layout survives." }
            }
        }
    }
}

#[component]
pub fn Preview() -> Element {
    rsx! {
        div { class: "demo-pane demo-preview",
            div { class: "demo-preview-card",
                h3 { "Preview" }
                p {
                    "This panel joined at runtime with "
                    code { "DockZone::Right" }
                    " as its home hint, and its tab carries a close affordance."
                }
            }
        }
    }
}

#[component]
pub fn Terminal() -> Element {
    rsx! {
        div { class: "demo-pane demo-terminal",
            pre {
                span { class: "demo-terminal-prompt", "$ " }
                "cargo run\n"
                span { class: "demo-terminal-dim", "   Compiling demo v0.1.0\n" }
                span { class: "demo-terminal-dim", "    Finished dev [unoptimized] in 1.42s\n" }
                span { class: "demo-terminal-dim", "     Running target/debug/demo\n" }
                "workbench ready\n"
                span { class: "demo-terminal-prompt", "$ " }
                span { class: "demo-terminal-cursor" }
            }
        }
    }
}

#[component]
pub fn Problems() -> Element {
    rsx! {
        div { class: "demo-pane demo-problems",
            ul {
                li {
                    span { class: "demo-problem-warning", "warning" }
                    span { "unused variable: `layout` — main.rs:42" }
                }
                li {
                    span { class: "demo-problem-warning", "warning" }
                    span { "field `home_zone` is never read — panels.rs:17" }
                }
            }
        }
    }
}

#[component]
pub fn Records() -> Element {
    let rows = [
        ("ep-0141", "pick-place", "34.2 s", "ok"),
        ("ep-0140", "pick-place", "31.8 s", "ok"),
        ("ep-0139", "stack-cups", "58.1 s", "review"),
        ("ep-0138", "stack-cups", "44.0 s", "ok"),
        ("ep-0137", "pour-water", "71.5 s", "failed"),
        ("ep-0136", "pour-water", "66.3 s", "ok"),
    ];
    rsx! {
        div { class: "demo-pane demo-table",
            table {
                thead {
                    tr {
                        th { "Episode" }
                        th { "Task" }
                        th { "Duration" }
                        th { "Status" }
                    }
                }
                tbody {
                    for (id, task, duration, status) in rows {
                        tr {
                            td { code { "{id}" } }
                            td { "{task}" }
                            td { "{duration}" }
                            td {
                                span { class: "demo-status demo-status-{status}", "{status}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Chart() -> Element {
    let samples = [
        18.0, 24.0, 21.0, 30.0, 27.0, 34.0, 31.0, 38.0, 36.0, 44.0, 41.0, 47.0,
    ];
    let max = 50.0;
    let step = 100.0 / (samples.len() as f64 - 1.0);
    let points = samples
        .iter()
        .enumerate()
        .map(|(i, value)| format!("{:.1},{:.1}", i as f64 * step, 40.0 - value / max * 40.0))
        .collect::<Vec<_>>()
        .join(" ");
    rsx! {
        div { class: "demo-pane demo-chart",
            span { class: "demo-chart-title", "Episodes per week" }
            svg {
                class: "demo-chart-plot",
                view_box: "0 0 100 40",
                preserve_aspect_ratio: "none",
                polyline { points: "{points}" }
            }
        }
    }
}

/// Editor state is panel-owned and survives structural docking.
#[component]
pub fn Query() -> Element {
    let mut query_text = use_signal(|| {
        "SELECT task, count(*) AS episodes\nFROM records\nGROUP BY task\nORDER BY episodes DESC;"
            .to_owned()
    });
    rsx! {
        div { class: "demo-pane demo-query",
            textarea {
                spellcheck: false,
                value: "{query_text}",
                oninput: move |event| *query_text.write() = event.value(),
            }
        }
    }
}

#[component]
pub fn QuickStart() -> Element {
    rsx! {
        div { class: "demo-pane demo-prose",
            h2 { "dioxus-workbench in three steps" }
            ol {
                li {
                    "Register content: "
                    code { "Panel::new(\"editor\", \"Editor\", \"main\", rsx! {{ … }})" }
                }
                li {
                    "Describe the first-open tree with "
                    code { "PanelLayout" }
                    " and "
                    code { "LayoutNode::split" }
                    " — after that, the workspace owns every dock, tab, and resize."
                }
                li {
                    "Persist "
                    code { "on_layout_change" }
                    " with "
                    code { "layout.encode()" }
                    " and hand it back as "
                    code { "initial_layout" }
                    " next session."
                }
            }
            p {
                "The rail on the left and the bar below are the optional shell: "
                code { "Workbench" }
                ", "
                code { "ActivityRail" }
                ", and "
                code { "StatusBar" }
                ". Each activity here keeps its own layout — switch and come back."
            }
        }
    }
}
