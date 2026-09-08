use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use dioxus::html::geometry::WheelDelta;
use dioxus::prelude::*;

use crate::dom;
use crate::icons::{CloseIcon, DockCompassIcon, SplitDownIcon, SplitRightIcon};
use crate::model::{MAX_SPLIT_RATIO, MIN_SPLIT_RATIO};
use crate::strings::WorkbenchStrings;
use crate::style::{use_style_owner, WorkbenchStyle};
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
    pub tab_icon: Option<Element>,
    pub tab_accessory: Option<Element>,
}

/// A context-menu request on a panel tab, delivered through
/// [`PanelWorkspace`]'s `on_tab_menu`. The library never draws a menu — what
/// a tab's menu says is application territory; this carries where and about
/// what.
#[derive(Clone, Debug, PartialEq)]
pub struct TabMenuRequest {
    pub panel: PanelId,
    /// The tile the panel currently sits in.
    pub tile: TileId,
    /// Pointer position in client coordinates, for menu placement.
    pub x: f64,
    pub y: f64,
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
            tab_icon: None,
            tab_accessory: None,
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

    /// A small element rendered before the tab's label — typically a file or
    /// content icon. Decorative: it is hidden from assistive technology, so
    /// the label must carry the meaning.
    pub fn with_tab_icon(mut self, icon: Element) -> Self {
        self.tab_icon = Some(icon);
        self
    }

    /// A small element rendered after the tab's label — a dirty dot, an
    /// unread badge, a spinner. The application owns its meaning and, when it
    /// carries one, its accessible name.
    pub fn with_tab_accessory(mut self, accessory: Element) -> Self {
        self.tab_accessory = Some(accessory);
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
    /// DOM id of the split's first child, the element whose flex-basis the
    /// drag writes directly while the gesture is live.
    first_child_dom: String,
}

/// Copyable handles shared by every node of one workspace tree.
///
/// Provided through context rather than cloned down the recursion: signals
/// and event handlers are `Copy`, so subscribing components pay only for what
/// they actually read. Render code must not read `placements` or
/// `reset_layout` — they exist for event handlers, which `peek` them.
#[derive(Clone, Copy)]
struct WorkspaceShared {
    layout: Signal<PanelLayout>,
    placements: Signal<Vec<PanelPlacement>>,
    reset_layout: Signal<PanelLayout>,
    strings: Signal<WorkbenchStrings>,
    dragging: Signal<Option<PanelId>>,
    drop_preview: Signal<Option<DropPreview>>,
    resizing: Signal<Option<ResizeDrag>>,
    split_mounts: Signal<HashMap<SplitId, Rc<MountedData>>>,
    on_layout_change: EventHandler<PanelLayout>,
    on_panel_activate: EventHandler<PanelId>,
    on_panel_close: EventHandler<PanelId>,
    on_resize: EventHandler<()>,
    on_tab_menu: Option<EventHandler<TabMenuRequest>>,
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
    /// session (see [`PanelLayout::decode`]). When omitted, an arrangement is
    /// derived from panel homes: one tile per distinct `home`, split left to
    /// right ([`PanelLayout::from_homes`]).
    #[props(default)]
    initial_layout: Option<PanelLayout>,
    /// Controlled mode: an application-owned signal as the source of truth.
    /// Mutate it at any time with [`PanelLayout`]'s methods — `dock_panel`,
    /// `split_active`, `activate` — to drive the layout programmatically (a
    /// View menu, a reset command). The workspace writes its own mutations to
    /// the same signal and still reports them through `on_layout_change`;
    /// writes the application makes itself are not echoed back. Must remain
    /// the same signal for the component's lifetime. Without it, the
    /// workspace owns layout state internally, seeded from `initial_layout`.
    #[props(default)]
    layout: Option<Signal<PanelLayout>>,
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
    ///
    /// Edge-triggered with memory: the workspace acts when the value changes
    /// to a panel it has not yet applied, then leaves tab state alone — a
    /// standing `Some` never re-asserts itself when the registry changes or
    /// the user picks another tab. To request the same panel twice in a row,
    /// pass `None` in between. A request naming a panel that is not yet
    /// registered stays pending until that panel arrives.
    #[props(default)]
    active_panel: Option<PanelId>,
    /// Fired after every settled layout mutation — docking, splitting, tab
    /// activation, a finished resize — with the full serializable layout: the
    /// hook for persistence (see [`PanelLayout::encode`]). Not fired for every
    /// pointer move while a splitter drag is still in flight.
    #[props(default)]
    on_layout_change: EventHandler<PanelLayout>,
    #[props(default)] on_panel_activate: EventHandler<PanelId>,
    /// Fired when a closable panel's close affordance is used. The
    /// application removes the panel from `panels` to complete the close.
    #[props(default)]
    on_panel_close: EventHandler<PanelId>,
    /// Fired when panel geometry settles (a dock, a finished resize), for
    /// content that measures its container, such as canvases.
    #[props(default)]
    on_resize: EventHandler<()>,
    /// Fired on a context-menu gesture (right-click, menu key) on a tab.
    /// Providing this suppresses the native browser menu on tabs; the
    /// application draws its own from the request's panel, tile, and
    /// position. Absent, tabs keep the native menu.
    #[props(default)]
    on_tab_menu: Option<EventHandler<TabMenuRequest>>,
    /// Overrides for the chrome's own text — see [`WorkbenchStrings`].
    #[props(default)]
    strings: Option<WorkbenchStrings>,
) -> Element {
    let owns_style = use_style_owner();
    let placements = panels.iter().map(Panel::placement).collect::<Vec<_>>();
    use_context_provider(WorkspaceDomIds::new);
    // With no explicit first arrangement, derive one from panel homes.
    let resolved_initial = initial_layout.unwrap_or_else(|| PanelLayout::from_homes(&placements));
    let initial_placements = placements.clone();
    let initial_fallback = resolved_initial.clone();
    let internal_layout =
        use_signal(move || layout_for_registry(initial_fallback, &initial_placements));
    // Controlled mode: the application's signal is the source of truth. It
    // must remain the same signal for the component's lifetime.
    let mut layout = layout.unwrap_or(internal_layout);
    let initial_scope = layout_scope.clone();
    let mut current_layout_scope = use_signal(move || initial_scope);
    let mut dragging = use_signal(|| None::<PanelId>);
    let mut drop_preview = use_signal(|| None::<DropPreview>);
    let mut resizing = use_signal(|| None::<ResizeDrag>);
    let split_mounts = use_signal(HashMap::<SplitId, Rc<MountedData>>::new);
    let resolved_reset = reset_layout.unwrap_or_else(|| resolved_initial.clone());
    let resolved_strings = strings.unwrap_or_default();
    let placements_mirror = use_signal({
        let initial = placements.clone();
        move || initial
    });
    let reset_mirror = use_signal({
        let initial = resolved_reset.clone();
        move || initial
    });
    let strings_mirror = use_signal({
        let initial = resolved_strings.clone();
        move || initial
    });

    let shared = use_context_provider(|| WorkspaceShared {
        layout,
        placements: placements_mirror,
        reset_layout: reset_mirror,
        strings: strings_mirror,
        dragging,
        drop_preview,
        resizing,
        split_mounts,
        on_layout_change,
        on_panel_activate,
        on_panel_close,
        on_resize,
        on_tab_menu,
    });

    // Registrations that cannot render are repaired silently; say so once at
    // mount, and again whenever the registry changes into a bad state.
    {
        let mount_placements = placements.clone();
        use_hook(move || warn_duplicate_panels(&mount_placements));
    }

    // Event handlers read the registry through these mirrors, so they must
    // follow the props on every render. Only the strings mirror has render
    // subscribers, and it changes about never.
    let sync_placements = placements.clone();
    use_effect(use_reactive(
        (&sync_placements, &resolved_reset, &resolved_strings),
        move |(placements, reset, strings)| {
            let mut placements_mirror = placements_mirror;
            let mut reset_mirror = reset_mirror;
            let mut strings_mirror = strings_mirror;
            if *placements_mirror.peek() != placements {
                warn_duplicate_panels(&placements);
                placements_mirror.set(placements);
            }
            if *reset_mirror.peek() != reset {
                reset_mirror.set(reset);
            }
            if *strings_mirror.peek() != strings {
                strings_mirror.set(strings);
            }
        },
    ));

    use_effect(dom::install_drag_shim);

    // A workspace component may remain mounted while its host changes scope,
    // for example from one open document to another. Component keys are a
    // rendering hint, not an ownership boundary, so explicitly reload the
    // session layout for the new scope.
    let scope_dependency = layout_scope.clone();
    let next_fallback = resolved_initial.clone();
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
    // programmatic activation all converge on the same session layout. The
    // request is an edge, not a level: remembering what was last applied
    // keeps an unrelated registry change from yanking the user's tab
    // selection back to a stale `Some`.
    let mut applied_active = use_signal(|| None::<PanelId>);
    let requested_panel = active_panel.clone();
    let requested_registry = placements.clone();
    use_effect(use_reactive(
        (&requested_panel, &requested_registry),
        move |(requested_panel, requested_registry)| {
            let Some(panel) = requested_panel else {
                applied_active.set(None);
                return;
            };
            if applied_active.peek().as_ref() == Some(&panel) {
                return;
            }
            if !requested_registry
                .iter()
                .any(|placement| placement.panel == panel)
            {
                // Not registered yet: leave the request pending so the panel
                // comes to the front when it arrives.
                return;
            }
            let mut next = layout.peek().clone();
            next.reconcile(&requested_registry);
            if next.activate(&panel) {
                layout.set(next.clone());
                on_layout_change.call(next);
                on_resize.call(());
            }
            applied_active.set(Some(panel));
        },
    ));

    // The reconciled tree is what actually renders. Memoized so a layout
    // write that reconciles to the same tree re-renders nothing.
    let display_placements = placements;
    let display_layout = use_memo(use_reactive((&display_placements,), move |(placements,)| {
        layout().reconciled(&placements)
    }));

    let display = display_layout();
    let display_tile_count = display.tile_count();
    let resizing_active = resizing().is_some();
    let dragging_active = dragging().is_some();

    rsx! {
        if owns_style {
            WorkbenchStyle {}
        }
        section {
            class: "wb-workspace",
            "data-resizing": resizing_active,
            "data-dragging": dragging_active,
            // While a splitter drag is live, pointer moves write flex-basis
            // straight to the DOM. Committing the layout per move would
            // re-render the workspace and invite consumers to persist at
            // pointer frequency; the settled commit happens on release.
            onpointermove: move |event| {
                let Some(active) = resizing.peek().clone() else { return };
                let pointer = match active.axis {
                    SplitAxis::Horizontal => event.data().client_coordinates().x,
                    SplitAxis::Vertical => event.data().client_coordinates().y,
                };
                let next_ratio = drag_ratio(&active, pointer);
                dom::set_flex_basis(&active.first_child_dom, next_ratio * 100.0);
            },
            onpointerup: move |event| {
                let Some(active) = resizing.peek().clone() else { return };
                resizing.set(None);
                let pointer = match active.axis {
                    SplitAxis::Horizontal => event.data().client_coordinates().x,
                    SplitAxis::Vertical => event.data().client_coordinates().y,
                };
                let next_ratio = drag_ratio(&active, pointer);
                let committed = mutate_layout(shared, |layout| {
                    layout.set_split_ratio(&active.split, next_ratio)
                });
                if !committed {
                    // No net movement: the render above never happens, so put
                    // back the style the transient drag wrote.
                    dom::set_flex_basis(&active.first_child_dom, active.start_ratio * 100.0);
                }
            },
            onpointercancel: move |_| {
                let Some(active) = resizing.peek().clone() else { return };
                resizing.set(None);
                dom::set_flex_basis(&active.first_child_dom, active.start_ratio * 100.0);
            },
            ondragend: move |_| {
                dragging.set(None);
                drop_preview.set(None);
            },
            NodeView { node: display.root, panels, display_tile_count }
        }
    }
}

/// One node of the reconciled tree. A dedicated component (rather than a
/// plain function) keeps signal subscriptions inside the subtree that needs
/// them and lets unchanged branches skip re-rendering entirely.
#[component]
fn NodeView(node: LayoutNode, panels: Vec<Panel>, display_tile_count: usize) -> Element {
    match node {
        LayoutNode::Tile(tile) => rsx! {
            TileView { tile, panels, display_tile_count }
        },
        LayoutNode::Split {
            id,
            axis,
            ratio,
            first,
            second,
        } => rsx! {
            SplitView {
                id,
                axis,
                ratio,
                first: *first,
                second: *second,
                panels,
                display_tile_count,
            }
        },
    }
}

#[component]
fn SplitView(
    id: SplitId,
    axis: SplitAxis,
    ratio: f64,
    first: LayoutNode,
    second: LayoutNode,
    panels: Vec<Panel>,
    display_tile_count: usize,
) -> Element {
    let shared = use_context::<WorkspaceShared>();
    let dom_ids = use_context::<WorkspaceDomIds>();
    let strings = shared.strings.read().clone();
    let splitter_dom_id = dom_ids.element("splitter", id.as_str());
    let first_child_dom = dom_ids.element("split-first", id.as_str());
    let orientation = match axis {
        SplitAxis::Horizontal => "vertical",
        SplitAxis::Vertical => "horizontal",
    };
    let split_class = match axis {
        SplitAxis::Horizontal => "wb-split wb-split-horizontal",
        SplitAxis::Vertical => "wb-split wb-split-vertical",
    };
    let first_style = format!("flex-basis: {:.5}%;", ratio * 100.0);
    // Only this splitter re-renders when a drag starts or ends on it.
    let splitter_active = use_memo(use_reactive((&id,), move |(id,)| {
        shared
            .resizing
            .read()
            .as_ref()
            .is_some_and(|active| active.split == id)
    }));
    let splitter_class = if splitter_active() {
        "wb-splitter wb-splitter-active"
    } else {
        "wb-splitter"
    };
    let mut split_mounts = shared.split_mounts;
    let mounted_split = id.clone();
    let pointer_split = id.clone();
    let keyboard_split = id.clone();
    let reset_split = id.clone();

    rsx! {
        div {
            class: "{split_class}",
            onmounted: move |event| {
                split_mounts.write().insert(mounted_split.clone(), event.data());
            },
            div {
                id: "{first_child_dom}",
                class: "wb-split-child wb-split-first",
                style: "{first_style}",
                NodeView { node: first, panels: panels.clone(), display_tile_count }
            }
            div {
                id: "{splitter_dom_id}",
                class: "{splitter_class}",
                role: "separator",
                tabindex: "0",
                "aria-label": "{strings.splitter_label}",
                "aria-orientation": "{orientation}",
                "aria-valuemin": (MIN_SPLIT_RATIO * 100.0) as i64,
                "aria-valuemax": (MAX_SPLIT_RATIO * 100.0) as i64,
                "aria-valuenow": (ratio * 100.0).round() as i64,
                title: "{strings.splitter_hint}",
                onpointerdown: {
                    let splitter_dom_id = splitter_dom_id.clone();
                    let first_child_dom = first_child_dom.clone();
                    move |event: PointerEvent| {
                        let Some(mount) = shared.split_mounts.peek().get(&pointer_split).cloned() else {
                            return;
                        };
                        event.prevent_default();
                        dom::capture_pointer(&splitter_dom_id, event.data().pointer_id());
                        let pointer = match axis {
                            SplitAxis::Horizontal => event.data().client_coordinates().x,
                            SplitAxis::Vertical => event.data().client_coordinates().y,
                        };
                        let split = pointer_split.clone();
                        let first_child_dom = first_child_dom.clone();
                        let mut resizing = shared.resizing;
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
                                first_child_dom,
                            }));
                        });
                    }
                },
                ondoubleclick: move |_| {
                    let target = shared
                        .reset_layout
                        .peek()
                        .split_ratio(&reset_split)
                        .unwrap_or(0.5);
                    mutate_layout(shared, |layout| layout.set_split_ratio(&reset_split, target));
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
                    mutate_layout(shared, |layout| {
                        let Some(current) = layout.split_ratio(&keyboard_split) else {
                            return false;
                        };
                        layout.set_split_ratio(&keyboard_split, current + delta)
                    });
                },
            }
            div { class: "wb-split-child wb-split-second",
                NodeView { node: second, panels, display_tile_count }
            }
        }
    }
}

#[component]
fn TileView(tile: Tile, panels: Vec<Panel>, display_tile_count: usize) -> Element {
    let shared = use_context::<WorkspaceShared>();
    let dom_ids = use_context::<WorkspaceDomIds>();
    // Memoized per tile: crossing drop zones mid-drag re-renders only the
    // tile whose preview state actually changed, not the whole tree.
    let drop_active = use_memo(use_reactive((&tile.id,), move |(tile_id,)| {
        shared
            .drop_preview
            .read()
            .as_ref()
            .is_some_and(|preview| preview.tile == tile_id)
    }));
    let dragging_active = use_memo(move || shared.dragging.read().is_some());
    let strings = shared.strings.read().clone();
    let mut tile_classes = vec!["wb-tile"];
    if drop_active() {
        tile_classes.push("wb-tile-drop-active");
    }
    if tile.id.as_str().starts_with("tile-") {
        tile_classes.push("wb-tile-enter");
    }
    let tile_class = tile_classes.join(" ");
    let right_tile = tile.id.clone();
    let down_tile = tile.id.clone();
    let close_tile = tile.id.clone();
    let tabs_dom_id = dom_ids.element("tabs", tile.id.as_str());
    let wheel_tabs_dom_id = tabs_dom_id.clone();
    // Name each tab strip after its active panel, so assistive technology
    // can tell four "Panel group"s apart.
    let group_label = tile
        .active
        .as_ref()
        .and_then(|active| panels.iter().find(|panel| &panel.id == active))
        .map(|panel| strings.tab_group_labelled.replace("{panel}", &panel.title))
        .unwrap_or_else(|| strings.tab_group_label.clone());

    rsx! {
        section { key: "{tile.id}", class: "{tile_class}", "data-tile-id": "{tile.id}",
            header { class: "wb-tab-bar wb-hover-host",
                div {
                    id: "{tabs_dom_id}",
                    class: "wb-tabs",
                    role: "tablist",
                    "aria-label": "{group_label}",
                    // A vertical wheel walks an overflowed strip sideways,
                    // the way editors train. Harmless when nothing overflows.
                    onwheel: move |event| {
                        let delta = match event.data().delta() {
                            WheelDelta::Pixels(vector) => vector.y,
                            WheelDelta::Lines(vector) => vector.y * 16.0,
                            WheelDelta::Pages(vector) => vector.y * 160.0,
                        };
                        dom::scroll_by_x(&wheel_tabs_dom_id, delta);
                    },
                    for panel_id in &tile.panels {
                        if let Some(panel) = panels.iter().find(|panel| &panel.id == panel_id) {
                            TabItem {
                                key: "{panel.id}",
                                panel: panel.clone(),
                                tile_id: tile.id.clone(),
                                is_active: tile.active.as_ref() == Some(&panel.id),
                                tab_order: tile.panels.clone(),
                            }
                        }
                    }
                    if tile.panels.is_empty() {
                        span { class: "wb-empty-label", "{strings.empty_group_label}" }
                    }
                }
                div { class: "wb-tile-actions",
                    button {
                        r#type: "button",
                        class: "wb-tile-action wb-icon-btn wb-hover-action",
                        "aria-label": "{strings.split_right_label}",
                        title: "{strings.split_right_hint}",
                        onclick: move |_| {
                            mutate_layout(shared, |layout| {
                                layout.split_active(&right_tile, DockZone::Right)
                            });
                        },
                        SplitRightIcon {}
                    }
                    button {
                        r#type: "button",
                        class: "wb-tile-action wb-icon-btn wb-hover-action",
                        "aria-label": "{strings.split_down_label}",
                        title: "{strings.split_down_hint}",
                        onclick: move |_| {
                            mutate_layout(shared, |layout| {
                                layout.split_active(&down_tile, DockZone::Bottom)
                            });
                        },
                        SplitDownIcon {}
                    }
                    if tile.panels.is_empty() && display_tile_count > 1 {
                        button {
                            r#type: "button",
                            class: "wb-tile-action wb-icon-btn wb-hover-action",
                            "aria-label": "{strings.close_empty_label}",
                            title: "{strings.close_empty_hint}",
                            onclick: move |_| {
                                mutate_layout(shared, |layout| {
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
                    if let Some(panel) = panels.iter().find(|panel| &panel.id == panel_id) {
                        {
                            let is_active = tile.active.as_ref() == Some(&panel.id);
                            let panel_dom = dom_ids.panel(&panel.id);
                            let labelled_by = dom_ids.tab(&panel.id);
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
                        strong { "{strings.empty_tile_title}" }
                        span { "{strings.empty_tile_hint}" }
                        span { class: "wb-empty-keys", "{strings.empty_tile_keys}" }
                    }
                }
            }
            if dragging_active() {
                DockOverlay { tile: tile.id.clone() }
            }
        }
    }
}

#[component]
fn TabItem(panel: Panel, tile_id: TileId, is_active: bool, tab_order: Vec<PanelId>) -> Element {
    let shared = use_context::<WorkspaceShared>();
    let dom_ids = use_context::<WorkspaceDomIds>();
    let strings = shared.strings.read().clone();
    let panel_id = panel.id.clone();
    let activate_id = panel.id.clone();
    let drag_id = panel.id.clone();
    let key_id = panel.id.clone();
    let close_id = panel.id.clone();
    let menu_id = panel.id.clone();
    let title = panel.title.clone();
    let close_label = strings.close_tab.replace("{title}", &title);
    let tab_id = dom_ids.tab(&panel.id);
    let controlled_panel_id = dom_ids.panel(&panel.id);
    let tab_index = if is_active { "0" } else { "-1" };
    let tab_item_class = if is_active {
        "wb-tab-item wb-tab-item-active"
    } else {
        "wb-tab-item"
    };
    let mut drag_signal = shared.dragging;
    let mut drag_preview_signal = shared.drop_preview;
    let drag_tile = tile_id.clone();
    let keyboard_tile = tile_id.clone();
    let menu_tile = tile_id.clone();

    // An activated tab may sit outside a crowded strip's viewport — after a
    // drop, a programmatic request, or a restored session. Bring it in.
    {
        let scroll_target = tab_id.clone();
        use_effect(use_reactive((&is_active,), move |(active,)| {
            if active {
                dom::scroll_into_view(&scroll_target);
            }
        }));
    }

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
                title: "{title}. {strings.tab_hint}",
                onclick: move |_| {
                    mutate_layout(shared, |layout| layout.activate(&activate_id));
                    shared.on_panel_activate.call(activate_id.clone());
                },
                oncontextmenu: move |event: MouseEvent| {
                    let Some(menu) = shared.on_tab_menu else { return };
                    event.prevent_default();
                    let point = event.data().client_coordinates();
                    menu.call(TabMenuRequest {
                        panel: menu_id.clone(),
                        tile: menu_tile.clone(),
                        x: point.x,
                        y: point.y,
                    });
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
                        let mutated = mutate_layout(shared, |layout| match action {
                            KeyboardDock::Split(zone) => {
                                layout.dock_panel(&key_id, &keyboard_tile, zone)
                                    || layout.split_tile(&keyboard_tile, zone).is_some()
                            }
                            KeyboardDock::Move(delta) => layout.move_panel_by_tile(&key_id, delta),
                        });
                        if mutated {
                            // The mutation re-renders the tab elsewhere in the
                            // tree; without this the keyboard flow strands
                            // focus on <body>.
                            dom::focus_after_render(dom_ids.tab(&key_id));
                        }
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
                    mutate_layout(shared, |layout| layout.activate(&target));
                    shared.on_panel_activate.call(target.clone());
                    dom::focus_after_render(dom_ids.tab(&target));
                },
                if let Some(icon) = panel.tab_icon.clone() {
                    span { class: "wb-tab-icon", "aria-hidden": "true", {icon} }
                }
                span { class: "wb-tab-label", "{title}" }
                if let Some(accessory) = panel.tab_accessory.clone() {
                    span { class: "wb-tab-accessory", {accessory} }
                }
            }
            if panel.closable {
                button {
                    r#type: "button",
                    class: "wb-tab-close wb-icon-btn wb-hover-action",
                    "aria-label": "{close_label}",
                    title: "{close_label}",
                    onclick: move |_| shared.on_panel_close.call(close_id.clone()),
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
fn DockOverlay(tile: TileId) -> Element {
    let shared = use_context::<WorkspaceShared>();
    // Value-gated per tile: only the overlays whose preview changed re-render
    // as the drag crosses zones.
    let preview = use_memo(use_reactive((&tile,), move |(tile,)| {
        shared
            .drop_preview
            .read()
            .clone()
            .filter(|preview| preview.tile == tile)
    }));
    let preview_value = preview();
    let preview_class = preview_value
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
                    let mut enter_preview_signal = shared.drop_preview;
                    let mut drop_dragging_signal = shared.dragging;
                    let mut drop_preview_signal = shared.drop_preview;
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
                                let Some(panel) = drop_dragging_signal.peek().clone() else { return };
                                mutate_layout(shared, |layout| {
                                    layout.dock_panel(&panel, &drop_tile, zone)
                                });
                                shared.on_panel_activate.call(panel);
                                drop_dragging_signal.set(None);
                                drop_preview_signal.set(None);
                            },
                        }
                    }
                }
            }
            if preview_value.is_some() {
                div { class: "{preview_class}" }
                div { class: "wb-drop-compass", DockCompassIcon {} }
            }
        }
    }
}

/// Reconcile against the current registry, apply one mutation, and — only if
/// it changed anything — commit the layout and fire the settled callbacks.
/// Returns whether a commit happened.
fn mutate_layout(shared: WorkspaceShared, mutation: impl FnOnce(&mut PanelLayout) -> bool) -> bool {
    let mut layout_signal = shared.layout;
    let mut next = layout_signal.peek().clone();
    next.reconcile(&shared.placements.peek());
    if !mutation(&mut next) {
        return false;
    }
    // Structural mutations can drop splits; release their mount handles so
    // the map does not grow for the session's lifetime.
    let live = next.split_ids().into_iter().collect::<HashSet<_>>();
    let mut split_mounts = shared.split_mounts;
    split_mounts.write().retain(|split, _| live.contains(split));
    layout_signal.set(next.clone());
    shared.on_layout_change.call(next);
    shared.on_resize.call(());
    true
}

fn drag_ratio(drag: &ResizeDrag, pointer: f64) -> f64 {
    let raw = drag.start_ratio + (pointer - drag.pointer_origin) / drag.span;
    if raw.is_finite() {
        raw.clamp(MIN_SPLIT_RATIO, MAX_SPLIT_RATIO)
    } else {
        drag.start_ratio
    }
}

fn layout_for_registry(initial_layout: PanelLayout, placements: &[PanelPlacement]) -> PanelLayout {
    let mut layout = initial_layout;
    if let Err(issue) = layout.validate() {
        tracing::warn!(
            "dioxus-workbench: the initial layout is invalid ({issue}); \
             reconciliation will repair what it can"
        );
    }
    layout.reconcile(placements);
    layout
}

fn warn_duplicate_panels(placements: &[PanelPlacement]) {
    let mut seen = HashSet::new();
    for placement in placements {
        if !seen.insert(&placement.panel) {
            tracing::warn!(
                "dioxus-workbench: duplicate panel id `{}` in the registry; \
                 only the first registration renders",
                placement.panel
            );
        }
    }
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

/// Logical panel and layout IDs are local to a workspace. DOM IDs must also
/// identify the owning instance so retained or side-by-side workspaces cannot
/// redirect each other's focus, pointer capture, or splitter updates.
#[derive(Clone, Copy)]
struct WorkspaceDomIds {
    scope: ScopeId,
}

impl WorkspaceDomIds {
    fn new() -> Self {
        Self {
            scope: dioxus::dioxus_core::current_scope_id(),
        }
    }

    fn element(self, kind: &str, key: &str) -> String {
        format!("wb-{}-{kind}-{}", self.scope.0, safe_id(key))
    }

    fn tab(self, panel: &PanelId) -> String {
        self.element("tab", panel.as_str())
    }

    fn panel(self, panel: &PanelId) -> String {
        self.element("panel", panel.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::dioxus_core::{AttributeValue, Mutation};

    struct WorkspacePair;
    impl WorkspacePair {
        fn render() -> Element {
            rsx! {
                for instance in ["first", "second"] {
                    PanelWorkspace {
                        key: "{instance}",
                        // All logical IDs intentionally overlap across instances.
                        panels: vec![
                            Panel::new("scene", "Scene", "primary", rsx! { "Scene content" }),
                            Panel::new("details", "Details", "secondary", rsx! { "Details content" }),
                        ],
                        initial_layout: PanelLayout::new(LayoutNode::split(
                            "columns", SplitAxis::Horizontal, 0.5,
                            LayoutNode::tile("primary", ["scene".to_owned()]),
                            LayoutNode::tile("secondary", ["details".to_owned()]),
                        )),
                    }
                }
            }
        }
    }

    #[test]
    fn mounted_workspaces_have_distinct_dom_ids_and_local_aria_targets() {
        let mut dom = VirtualDom::new(WorkspacePair::render);
        let mutations = dom.rebuild_to_vec();
        let mut ids = HashSet::new();
        let mut references = Vec::new();
        let mut elements = HashMap::new();
        for edit in mutations.edits {
            if let Mutation::SetAttribute {
                name,
                value: AttributeValue::Text(value),
                id,
                ..
            } = edit
            {
                if name == "id" {
                    assert!(ids.insert(value.clone()), "duplicate DOM id: {value}");
                    elements.insert(id, value);
                } else if matches!(name, "aria-controls" | "aria-labelledby") {
                    references.push((id, value));
                }
            }
        }
        assert_eq!(ids.iter().filter(|id| id.contains("-splitter-")).count(), 2);
        assert_eq!(ids.iter().filter(|id| id.contains("-tabs-")).count(), 4);
        assert_eq!(ids.iter().filter(|id| id.contains("-tab-")).count(), 4);
        assert_eq!(ids.iter().filter(|id| id.contains("-panel-")).count(), 4);
        assert_eq!(references.len(), 8);
        for (element, target) in references {
            assert!(ids.contains(&target), "missing ARIA target: {target}");
            let owner = elements[&element].split('-').nth(1).unwrap();
            assert_eq!(target.split('-').nth(1).unwrap(), owner);
        }
    }
}
