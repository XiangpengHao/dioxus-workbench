//! Stable content ownership, independent of the recursive chrome tree.
use dioxus::prelude::*;
use serde::Deserialize;

use crate::workspace::{Panel, WorkspaceDomIds};
use crate::TileId;

/// The current content box in CSS pixels. Hidden panels report zero size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize)]
pub struct PanelGeometry {
    width: f64,
    height: f64,
    visible: bool,
}

impl PanelGeometry {
    pub fn width(self) -> f64 {
        self.width
    }
    pub fn height(self) -> f64 {
        self.height
    }
    pub fn visible(self) -> bool {
        self.visible
    }
}

/// Panel-local lifecycle information. Read inside an effect to react to
/// tab activation, ancestor visibility, window resizing, or splitter drags.
/// The application owns how its renderer responds to these measurements.
#[derive(Clone, Copy, PartialEq)]
pub struct PanelContext {
    geometry: ReadSignal<PanelGeometry>,
    on_close: EventHandler<()>,
}

impl PanelContext {
    pub fn current() -> Self {
        use_context()
    }
    pub fn geometry(self) -> PanelGeometry {
        *self.geometry.read()
    }
    /// Use the same acceptance and focus path as the tab's close control.
    pub fn request_close(self) {
        self.on_close.call(());
    }
}

#[component]
pub(crate) fn PanelHost(
    panel: Panel,
    tile: Option<TileId>,
    active: bool,
    on_close: EventHandler<()>,
    layout_identity: Vec<(crate::SplitId, bool)>,
) -> Element {
    let ids = use_context::<WorkspaceDomIds>();
    let mut geometry = use_signal(PanelGeometry::default);
    use_context_provider(|| PanelContext {
        geometry: geometry.into(),
        on_close,
    });
    let mut observer = use_signal(|| None::<document::Eval>);
    let mut subscription = use_signal(|| None::<dioxus::dioxus_core::Task>);
    let panel_dom = ids.panel(&panel.id);
    let root_dom = ids.element("workspace", "root");
    let label_dom = ids.tab(&panel.id);
    let keyboard_host = panel_dom.clone();
    let keyboard_tab = label_dom.clone();
    let slot_dom = tile
        .as_ref()
        .map(|tile| ids.element("content-slot", tile.as_str()));
    use_effect(use_reactive((&slot_dom, &active, &layout_identity), {
        let panel_dom = panel_dom.clone();
        move |(slot, active, _)| {
            if let Some(task) = subscription.take() {
                task.cancel();
            }
            if let Some(previous) = observer.take() {
                let _ = previous.send(());
            }
            geometry.set(PanelGeometry::default());
            let Some(slot) = slot.filter(|_| active) else {
                return;
            };
            let script = format!(
                "const hostId = {}; const slotId = {}; const rootId = {};\n{}",
                serde_json::to_string(&panel_dom).unwrap(),
                serde_json::to_string(&slot).unwrap(),
                serde_json::to_string(&root_dom).unwrap(),
                include_str!("panel_host.js")
            );
            // Defer until the reconciled chrome and stable hosts are committed.
            let task = spawn(async move {
                crate::dom::sleep_ms(0).await;
                let mut eval = document::eval(&script);
                observer.set(Some(eval));
                while let Ok(next) = eval.recv::<PanelGeometry>().await {
                    if *geometry.peek() != next {
                        geometry.set(next);
                    }
                }
            });
            subscription.set(Some(task));
        }
    }));
    use_drop(move || {
        if let Some(eval) = *observer.peek() {
            let _ = eval.send(());
        }
    });
    rsx! {
        div {
            id: panel_dom,
            class: "wb-panel-content wb-panel-host {panel.class}",
            role: "tabpanel",
            tabindex: "0",
            hidden: !active,
            "aria-labelledby": label_dom,
            onmounted: move |_| crate::dom::bridge_panel_tab(&keyboard_host, &keyboard_tab),
            {panel.content}
        }
    }
}
