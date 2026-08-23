use std::collections::HashMap;
use std::rc::Rc;

use dioxus::prelude::*;

use crate::dom;
use crate::icons::{CloseIcon, DockCompassIcon, SplitDownIcon, SplitRightIcon};
use crate::model::{MAX_SPLIT_RATIO, MIN_SPLIT_RATIO};
use crate::style::{use_style_owner, STYLE};
use crate::{
    DockZone, LayoutNode, PanelId, PanelLayout, PanelPlacement, SplitAxis, SplitId, Tile, TileId,
};

const RESIZE_KEYBOARD_STEP: f64 = 0.025;
const RESIZE_KEYBOARD_LARGE_STEP: f64 = 0.08;

/// Application content registered with a [`PanelWorkspace`].
#[derive(Clone, PartialEq)]
pub struct Panel {
    pub id: PanelId,
    pub title: String,
    pub home: TileId,
    pub home_zone: DockZone,
    pub class: String,
    pub closable: bool,
    pub content: Element,
}

impl Panel {
    pub fn new(
        id: impl Into<PanelId>,
        title: impl Into<String>,
        home: impl Into<TileId>,
        content: Element,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            home: home.into(),
            home_zone: DockZone::Center,
            class: String::new(),
            closable: false,
            content,
        }
    }

    /// Add a class to the panel's content container, for panel-specific
    /// styling such as its own background or padding behavior.
    pub fn with_class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Give this panel an explicit close affordance in its tab.
    /// The application remains responsible for removing it from the registry.
    pub fn with_closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// Split a newly registered panel beside its occupied home instead of
    /// hiding it as another tab. Current session placement still wins.
    pub fn with_home_zone(mut self, zone: DockZone) -> Self {
        self.home_zone = zone;
        self
    }

    fn placement(&self) -> PanelPlacement {
        PanelPlacement::new(self.id.clone(), self.home.clone()).with_zone(self.home_zone)
    }
}

#[derive(Clone, Debug, PartialEq)]
struct DropPreview {
    tile: TileId,
    zone: DockZone,
}

#[derive(Clone, Debug, PartialEq)]
struct ResizeDrag {
    split: SplitId,
    axis: SplitAxis,
    pointer_origin: f64,
    start_ratio: f64,
    span: f64,
}

#[derive(Clone, PartialEq)]
struct RenderContext {
    panels: Vec<Panel>,
    placements: Vec<PanelPlacement>,
    layout: Signal<PanelLayout>,
    reset_layout: PanelLayout,
    display_tile_count: usize,
    dragging: Signal<Option<PanelId>>,
    drop_preview: Signal<Option<DropPreview>>,
    resizing: Signal<Option<ResizeDrag>>,
    split_mounts: Signal<HashMap<SplitId, Rc<MountedData>>>,
    on_layout_change: EventHandler<PanelLayout>,
    on_panel_activate: EventHandler<PanelId>,
    on_panel_close: EventHandler<PanelId>,
    on_resize: EventHandler<()>,
}

/// Render a session-owned tree of tabbed panels and resizable splits.
///
/// The application owns panel content and the initial tree. The workspace owns
/// subsequent layout mutation. Drag a tab onto the center of a tile to attach
/// it as a tab, or onto an edge to create a split. `Alt+Shift+Arrow` splits the
/// active tab from the keyboard; `Alt+Shift+PageUp/PageDown` attaches it to the
/// previous or next tile.
#[component]
pub fn PanelWorkspace(
    /// The available panels. Panels absent from a restored layout are
    /// attached to their `home` tile; panels removed from this registry
    /// disappear from the layout on the next render.
    panels: Vec<Panel>,
    /// The layout to open with — the canonical arrangement, or a restored
    /// session (see [`PanelLayout::decode`]).
    initial_layout: PanelLayout,
    /// The target of a double-click reset on a splitter. Defaults to
    /// `initial_layout`; pass the canonical arrangement when
    /// `initial_layout` carries a restored session, so reset means "as
    /// designed" rather than "as saved".
    #[props(default)]
    reset_layout: Option<PanelLayout>,
    /// A change of scope (for example another document or screen) reloads
    /// the initial layout instead of carrying the previous scope's edits.
    /// `layout_scope`, `initial_layout`, and `panels` must always describe
    /// the same scope within one render: when a scope's layout arrives
    /// asynchronously, hold the previous scope (or unmount) until it does.
    #[props(default)]
    layout_scope: Option<String>,
    /// Programmatic activation: setting this brings a registered panel's tab
    /// to the front through the same path as a click.
    #[props(default)]
    active_panel: Option<PanelId>,
    /// Fired after every layout mutation with the full serializable layout —
    /// the hook for persistence (see [`PanelLayout::encode`]).
    #[props(default)]
    on_layout_change: EventHandler<PanelLayout>,
    #[props(default)] on_panel_activate: EventHandler<PanelId>,
    /// Fired when a closable panel's close affordance is used. The
    /// application removes the panel from `panels` to complete the close.
    #[props(default)]
    on_panel_close: EventHandler<PanelId>,
    /// Fired when panel geometry changes (splits resized, tabs docked), for
    /// content that measures its container, such as canvases.
    #[props(default)]
    on_resize: EventHandler<()>,
) -> Element {
    let owns_style = use_style_owner();
    let placements = panels.iter().map(Panel::placement).collect::<Vec<_>>();
    let initial_placements = placements.clone();
    let initial_fallback = initial_layout.clone();
    let mut layout = use_signal(move || layout_for_registry(initial_fallback, &initial_placements));
    let initial_scope = layout_scope.clone();
    let mut current_layout_scope = use_signal(move || initial_scope);
    let mut dragging = use_signal(|| None::<PanelId>);
    let mut drop_preview = use_signal(|| None::<DropPreview>);
    let mut resizing = use_signal(|| None::<ResizeDrag>);
    let split_mounts = use_signal(HashMap::<SplitId, Rc<MountedData>>::new);

    use_effect(dom::install_drag_shim);

    // A workspace component may remain mounted while its host changes scope,
    // for example from one open document to another. Component keys are a
    // rendering hint, not an ownership boundary, so explicitly reload the
    // session layout for the new scope.
    let scope_dependency = layout_scope.clone();
    let next_fallback = initial_layout.clone();
    let next_placements = placements.clone();
    use_effect(use_reactive((&scope_dependency,), move |(next_scope,)| {
        if current_layout_scope.peek().as_ref() == next_scope.as_ref() {
            return;
        }
        let next = layout_for_registry(next_fallback.clone(), &next_placements);
        current_layout_scope.set(next_scope.clone());
        layout.set(next);
        dragging.set(None);
        drop_preview.set(None);
        resizing.set(None);
    }));

    // Consumers such as run histories can open a registered panel directly.
    // Keep the workspace as the owner of tab state so pointer, keyboard, and
    // programmatic activation all converge on the same session layout.
    let requested_panel = active_panel.clone();
    let requested_registry = placements.clone();
    let requested_layout_change = on_layout_change;
    let requested_resize = on_resize;
    use_effect(use_reactive(
        (&requested_panel, &requested_registry),
        move |(requested_panel, requested_registry)| {
            let Some(panel) = requested_panel else { return };
            if !requested_registry
                .iter()
                .any(|placement| placement.panel == panel)
            {
                return;
            }
            let mut next = layout();
            next.reconcile(&requested_registry);
            if next.activate(&panel) {
                layout.set(next.clone());
                requested_layout_change.call(next);
                requested_resize.call(());
            }
        },
    ));

    let display_layout = layout().reconciled(&placements);
    let context = RenderContext {
        panels,
        placements,
        layout,
        reset_layout: reset_layout.unwrap_or(initial_layout),
        display_tile_count: display_layout.tile_count(),
        dragging,
        drop_preview,
        resizing,
        split_mounts,
        on_layout_change,
        on_panel_activate,
        on_panel_close,
        on_resize,
    };
    let pointer_context = context.clone();
    let release_context = context.clone();
    let cancel_context = context.clone();
    let resizing_active = resizing().is_some();
    let dragging_active = dragging().is_some();

    rsx! {
        if owns_style {
            document::Style { {STYLE} }
        }
        section {
            class: "wb-workspace",
            "data-resizing": resizing_active,
            "data-dragging": dragging_active,
            onpointermove: move |event| {
                let Some(active) = resizing() else { return };
                let pointer = match active.axis {
                    SplitAxis::Horizontal => event.data().client_coordinates().x,
                    SplitAxis::Vertical => event.data().client_coordinates().y,
                };
                let next_ratio = active.start_ratio
                    + (pointer - active.pointer_origin) / active.span;
                let mut next = layout();
                if next.set_split_ratio(&active.split, next_ratio) {
                    layout.set(next.clone());
                    pointer_context.on_layout_change.call(next);
                    pointer_context.on_resize.call(());
                }
            },
            onpointerup: move |_| {
                if resizing().is_some() {
                    resizing.set(None);
                    let next = layout();
                    release_context.on_layout_change.call(next);
                    release_context.on_resize.call(());
                }
            },
            onpointercancel: move |_| {
                let Some(active) = resizing() else { return };
                resizing.set(None);
                let mut next = layout();
                if next.set_split_ratio(&active.split, active.start_ratio) {
                    layout.set(next.clone());
                    cancel_context.on_layout_change.call(next);
                    cancel_context.on_resize.call(());
                }
            },
            ondragend: move |_| {
                dragging.set(None);
                drop_preview.set(None);
            },
            {render_node(display_layout.root, context)}
        }
    }
}

fn render_node(node: LayoutNode, context: RenderContext) -> Element {
    match node {
        LayoutNode::Tile(tile) => render_tile(tile, context),
        LayoutNode::Split {
            id,
            axis,
            ratio,
            first,
            second,
        } => {
            let splitter_dom_id = format!("wb-splitter-{}", safe_id(id.as_str()));
            let orientation = match axis {
                SplitAxis::Horizontal => "vertical",
                SplitAxis::Vertical => "horizontal",
            };
            let split_class = match axis {
                SplitAxis::Horizontal => "wb-split wb-split-horizontal",
                SplitAxis::Vertical => "wb-split wb-split-vertical",
            };
            let first_style = format!("flex-basis: {:.5}%;", ratio * 100.0);
            let first_context = context.clone();
            let second_context = context.clone();
            let mut split_mounts = context.split_mounts;
            let pointer_resizing = context.resizing;
            let pointer_mounts = context.split_mounts;
            let keyboard_context = context.clone();
            let reset_context = context.clone();
            let mounted_split = id.clone();
            let pointer_split = id.clone();
            let keyboard_split = id.clone();
            let reset_split = id.clone();
            let reset_ratio = context.reset_layout.split_ratio(&id).unwrap_or(0.5);
            let splitter_class = if (context.resizing)()
                .as_ref()
                .is_some_and(|active| active.split == id)
            {
                "wb-splitter wb-splitter-active"
            } else {
                "wb-splitter"
            };

            rsx! {
                div {
                    class: "{split_class}",
                    onmounted: move |event| {
                        split_mounts.write().insert(mounted_split.clone(), event.data());
                    },
                    div { class: "wb-split-child wb-split-first", style: "{first_style}",
                        {render_node(*first, first_context)}
                    }
                    div {
                        id: "{splitter_dom_id}",
                        class: "{splitter_class}",
                        role: "separator",
                        tabindex: "0",
                        "aria-label": "Resize panel groups",
                        "aria-orientation": "{orientation}",
                        "aria-valuemin": (MIN_SPLIT_RATIO * 100.0) as i64,
                        "aria-valuemax": (MAX_SPLIT_RATIO * 100.0) as i64,
                        "aria-valuenow": (ratio * 100.0).round() as i64,
                        title: "Drag to resize. Arrow keys resize; Shift moves farther; double-click resets.",
                        onpointerdown: {
                            let splitter_dom_id = splitter_dom_id.clone();
                            move |event: PointerEvent| {
                                let Some(mount) = pointer_mounts.peek().get(&pointer_split).cloned() else {
                                    return;
                                };
                                event.prevent_default();
                                dom::capture_pointer(&splitter_dom_id, event.data().pointer_id());
                                let pointer = match axis {
                                    SplitAxis::Horizontal => event.data().client_coordinates().x,
                                    SplitAxis::Vertical => event.data().client_coordinates().y,
                                };
                                let split = pointer_split.clone();
                                let mut resizing = pointer_resizing;
                                spawn(async move {
                                    let Ok(bounds) = mount.get_client_rect().await else { return };
                                    let span = match axis {
                                        SplitAxis::Horizontal => bounds.width(),
                                        SplitAxis::Vertical => bounds.height(),
                                    };
                                    if !span.is_finite() || span <= 0.0 {
                                        return;
                                    }
                                    resizing.set(Some(ResizeDrag {
                                        split,
                                        axis,
                                        pointer_origin: pointer,
                                        start_ratio: ratio,
                                        span,
                                    }));
                                });
                            }
                        },
                        ondoubleclick: move |_| {
                            mutate_layout(reset_context.clone(), |layout| {
                                layout.set_split_ratio(&reset_split, reset_ratio)
                            });
                        },
                        onkeydown: move |event| {
                            let step = if event.modifiers().shift() {
                                RESIZE_KEYBOARD_LARGE_STEP
                            } else {
                                RESIZE_KEYBOARD_STEP
                            };
                            let delta = match (axis, event.key()) {
                                (SplitAxis::Horizontal, Key::ArrowLeft)
                                | (SplitAxis::Vertical, Key::ArrowUp) => -step,
                                (SplitAxis::Horizontal, Key::ArrowRight)
                                | (SplitAxis::Vertical, Key::ArrowDown) => step,
                                _ => return,
                            };
                            event.prevent_default();
                            mutate_layout(keyboard_context.clone(), |layout| {
                                let Some(current) = layout.split_ratio(&keyboard_split) else {
                                    return false;
                                };
                                layout.set_split_ratio(&keyboard_split, current + delta)
                            });
                        },
                    }
                    div { class: "wb-split-child wb-split-second",
                        {render_node(*second, second_context)}
                    }
                }
            }
        }
    }
}

fn render_tile(tile: Tile, context: RenderContext) -> Element {
    let tile_id = tile.id.clone();
    let preview = (context.drop_preview)().filter(|preview| preview.tile == tile.id);
    let mut tile_classes = vec!["wb-tile"];
    if preview.is_some() {
        tile_classes.push("wb-tile-drop-active");
    }
    if tile.id.as_str().starts_with("tile-") {
        tile_classes.push("wb-tile-enter");
    }
    let tile_class = tile_classes.join(" ");
    let right_context = context.clone();
    let down_context = context.clone();
    let close_context = context.clone();
    let right_tile = tile.id.clone();
    let down_tile = tile.id.clone();
    let close_tile = tile.id.clone();

    rsx! {
        section { key: "{tile.id}", class: "{tile_class}", "data-tile-id": "{tile.id}",
            header { class: "wb-tab-bar wb-hover-host",
                div { class: "wb-tabs", role: "tablist", "aria-label": "Panel group",
                    for panel_id in &tile.panels {
                        if let Some(panel) = context.panels.iter().find(|panel| &panel.id == panel_id) {
                            {render_tab(panel.clone(), &tile, context.clone())}
                        }
                    }
                    if tile.panels.is_empty() {
                        span { class: "wb-empty-label", "Empty group" }
                    }
                }
                div { class: "wb-tile-actions",
                    button {
                        r#type: "button",
                        class: "wb-tile-action wb-icon-btn wb-hover-action",
                        "aria-label": "Split panel group right",
                        title: "Split right (Alt+Shift+Right)",
                        onclick: move |_| {
                            mutate_layout(right_context.clone(), |layout| {
                                layout.split_active(&right_tile, DockZone::Right)
                            });
                        },
                        SplitRightIcon {}
                    }
                    button {
                        r#type: "button",
                        class: "wb-tile-action wb-icon-btn wb-hover-action",
                        "aria-label": "Split panel group down",
                        title: "Split down (Alt+Shift+Down)",
                        onclick: move |_| {
                            mutate_layout(down_context.clone(), |layout| {
                                layout.split_active(&down_tile, DockZone::Bottom)
                            });
                        },
                        SplitDownIcon {}
                    }
                    if tile.panels.is_empty() && context.display_tile_count > 1 {
                        button {
                            r#type: "button",
                            class: "wb-tile-action wb-icon-btn wb-hover-action",
                            "aria-label": "Close empty panel group",
                            title: "Close empty group",
                            onclick: move |_| {
                                mutate_layout(close_context.clone(), |layout| {
                                    layout.remove_empty_tile(&close_tile)
                                });
                            },
                            CloseIcon {}
                        }
                    }
                }
            }
            div { class: "wb-panel-frame",
                for panel_id in &tile.panels {
                    if let Some(panel) = context.panels.iter().find(|panel| &panel.id == panel_id) {
                        {
                            let is_active = tile.active.as_ref() == Some(&panel.id);
                            let panel_dom = panel_dom_id(&panel.id);
                            let labelled_by = tab_dom_id(&panel.id);
                            rsx! {
                                div {
                                    key: "{panel.id}",
                                    id: "{panel_dom}",
                                    class: "wb-panel-content {panel.class}",
                                    role: "tabpanel",
                                    tabindex: "0",
                                    hidden: !is_active,
                                    "aria-labelledby": "{labelled_by}",
                                    {panel.content.clone()}
                                }
                            }
                        }
                    }
                }
                if tile.panels.is_empty() {
                    div { class: "wb-empty-tile",
                        strong { "Empty panel group" }
                        span { "Drag a panel tab here, or close this group." }
                    }
                }
            }
            if (context.dragging)().is_some() {
                DockOverlay { tile: tile_id, preview, context }
            }
        }
    }
}

fn render_tab(panel: Panel, tile: &Tile, context: RenderContext) -> Element {
    let panel_id = panel.id.clone();
    let activate_id = panel.id.clone();
    let drag_id = panel.id.clone();
    let key_id = panel.id.clone();
    let close_id = panel.id.clone();
    let title = panel.title.clone();
    let is_active = tile.active.as_ref() == Some(&panel.id);
    let tab_id = tab_dom_id(&panel.id);
    let controlled_panel_id = panel_dom_id(&panel.id);
    let tab_index = if is_active { "0" } else { "-1" };
    let tab_item_class = if is_active {
        "wb-tab-item wb-tab-item-active"
    } else {
        "wb-tab-item"
    };
    let activate_context = context.clone();
    let close_context = context.clone();
    let keyboard_context = context.clone();
    let mut drag_signal = context.dragging;
    let mut drag_preview_signal = context.drop_preview;
    let drag_tile = tile.id.clone();
    let keyboard_tile = tile.id.clone();
    let tab_order = tile.panels.clone();

    rsx! {
        div { key: "{panel_id}", class: "{tab_item_class} wb-hover-host",
            button {
                r#type: "button",
                id: "{tab_id}",
                class: "wb-tab",
                role: "tab",
                draggable: "true",
                tabindex: "{tab_index}",
                "aria-selected": is_active,
                "aria-controls": "{controlled_panel_id}",
                title: "{title}. Drag to dock; Alt+Shift+Arrow splits; Alt+Shift+Page Up or Down moves between groups.",
                onclick: move |_| {
                    mutate_layout(activate_context.clone(), |layout| layout.activate(&activate_id));
                    activate_context.on_panel_activate.call(activate_id.clone());
                },
                ondragstart: move |_: DragEvent| {
                    drag_signal.set(Some(drag_id.clone()));
                    drag_preview_signal.set(Some(DropPreview {
                        tile: drag_tile.clone(),
                        zone: DockZone::Center,
                    }));
                },
                onkeydown: move |event: KeyboardEvent| {
                    if event.modifiers().alt() && event.modifiers().shift() {
                        let action = match event.key() {
                            Key::ArrowLeft => Some(KeyboardDock::Split(DockZone::Left)),
                            Key::ArrowRight => Some(KeyboardDock::Split(DockZone::Right)),
                            Key::ArrowUp => Some(KeyboardDock::Split(DockZone::Top)),
                            Key::ArrowDown => Some(KeyboardDock::Split(DockZone::Bottom)),
                            Key::PageUp => Some(KeyboardDock::Move(-1)),
                            Key::PageDown => Some(KeyboardDock::Move(1)),
                            _ => None,
                        };
                        let Some(action) = action else { return };
                        event.prevent_default();
                        mutate_layout(keyboard_context.clone(), |layout| match action {
                            KeyboardDock::Split(zone) => {
                                layout.dock_panel(&key_id, &keyboard_tile, zone)
                                    || layout.split_tile(&keyboard_tile, zone).is_some()
                            }
                            KeyboardDock::Move(delta) => layout.move_panel_by_tile(&key_id, delta),
                        });
                        return;
                    }
                    let Some(current) = tab_order.iter().position(|panel| panel == &key_id) else {
                        return;
                    };
                    let target = match event.key() {
                        Key::ArrowLeft => (current + tab_order.len() - 1) % tab_order.len(),
                        Key::ArrowRight => (current + 1) % tab_order.len(),
                        Key::Home => 0,
                        Key::End => tab_order.len() - 1,
                        _ => return,
                    };
                    event.prevent_default();
                    let target = tab_order[target].clone();
                    mutate_layout(keyboard_context.clone(), |layout| layout.activate(&target));
                    keyboard_context.on_panel_activate.call(target.clone());
                    dom::focus_after_render(tab_dom_id(&target));
                },
                span { class: "wb-tab-label", "{title}" }
            }
            if panel.closable {
                button {
                    r#type: "button",
                    class: "wb-tab-close wb-icon-btn wb-hover-action",
                    "aria-label": "Close {title}",
                    title: "Close {title}",
                    onclick: move |_| close_context.on_panel_close.call(close_id.clone()),
                    CloseIcon {}
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum KeyboardDock {
    Split(DockZone),
    Move(isize),
}

#[component]
fn DockOverlay(tile: TileId, preview: Option<DropPreview>, context: RenderContext) -> Element {
    let preview_class = preview
        .as_ref()
        .map(|preview| {
            format!(
                "wb-drop-preview wb-drop-preview-{}",
                zone_name(preview.zone)
            )
        })
        .unwrap_or_else(|| "wb-drop-preview".to_owned());

    rsx! {
        div { class: "wb-dock-overlay", "aria-hidden": "true",
            for zone in [DockZone::Left, DockZone::Right, DockZone::Top, DockZone::Bottom, DockZone::Center] {
                {
                    let drop_context = context.clone();
                    let mut enter_preview_signal = context.drop_preview;
                    let mut drop_dragging_signal = context.dragging;
                    let mut drop_preview_signal = context.drop_preview;
                    let enter_tile = tile.clone();
                    let drop_tile = tile.clone();
                    rsx! {
                        div {
                            key: "{zone_name(zone)}",
                            class: "wb-drop-zone wb-drop-zone-{zone_name(zone)}",
                            ondragenter: move |event| {
                                event.prevent_default();
                                enter_preview_signal.set(Some(DropPreview {
                                    tile: enter_tile.clone(),
                                    zone,
                                }));
                            },
                            ondragover: move |event| event.prevent_default(),
                            ondrop: move |event| {
                                event.prevent_default();
                                let Some(panel) = drop_dragging_signal() else { return };
                                mutate_layout(drop_context.clone(), |layout| {
                                    layout.dock_panel(&panel, &drop_tile, zone)
                                });
                                drop_context.on_panel_activate.call(panel);
                                drop_dragging_signal.set(None);
                                drop_preview_signal.set(None);
                            },
                        }
                    }
                }
            }
            if preview.is_some() {
                div { class: "{preview_class}" }
                div { class: "wb-drop-compass", DockCompassIcon {} }
            }
        }
    }
}

fn mutate_layout(context: RenderContext, mutation: impl FnOnce(&mut PanelLayout) -> bool) {
    let mut layout_signal = context.layout;
    let mut next = layout_signal();
    next.reconcile(&context.placements);
    if !mutation(&mut next) {
        return;
    }
    layout_signal.set(next.clone());
    context.on_layout_change.call(next);
    context.on_resize.call(());
}

fn layout_for_registry(initial_layout: PanelLayout, placements: &[PanelPlacement]) -> PanelLayout {
    let mut layout = initial_layout;
    layout.reconcile(placements);
    layout
}

fn zone_name(zone: DockZone) -> &'static str {
    match zone {
        DockZone::Center => "center",
        DockZone::Left => "left",
        DockZone::Right => "right",
        DockZone::Top => "top",
        DockZone::Bottom => "bottom",
    }
}

fn safe_id(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn tab_dom_id(panel: &PanelId) -> String {
    format!("wb-tab-{}", safe_id(panel.as_str()))
}

fn panel_dom_id(panel: &PanelId) -> String {
    format!("wb-panel-{}", safe_id(panel.as_str()))
}
