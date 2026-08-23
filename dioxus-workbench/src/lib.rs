//! # dioxus-workbench
//!
//! A dockable panel workbench for [Dioxus](https://dioxuslabs.com): the
//! editor-style shell — tabbed panel groups, drag-to-dock, resizable splits,
//! an activity rail, and a status bar — without a line of JavaScript.
//!
//! Applications register content as [`Panel`] values and describe only their
//! preferred starting arrangement. [`PanelWorkspace`] owns the durable layout,
//! tab activation, tiling, split resizing, drag previews, keyboard
//! equivalents, and the common panel chrome. [`Workbench`], [`ActivityRail`],
//! and [`StatusBar`] wrap it in the surrounding application frame.
//!
//! ```ignore
//! use dioxus::prelude::*;
//! use dioxus_workbench::prelude::*;
//!
//! #[component]
//! fn App() -> Element {
//!     let panels = vec![
//!         Panel::new("files", "Files", "side", rsx! { FileTree {} }),
//!         Panel::new("editor", "Editor", "main", rsx! { Editor {} }),
//!         Panel::new("terminal", "Terminal", "bottom", rsx! { Terminal {} }),
//!     ];
//!     let layout = PanelLayout::new(LayoutNode::split(
//!         "root",
//!         SplitAxis::Horizontal,
//!         0.25,
//!         LayoutNode::tile("side", ["files"]),
//!         LayoutNode::split(
//!             "main-rows",
//!             SplitAxis::Vertical,
//!             0.7,
//!             LayoutNode::tile("main", ["editor"]),
//!             LayoutNode::tile("bottom", ["terminal"]),
//!         ),
//!     ));
//!
//!     rsx! {
//!         div { style: "width: 100vw; height: 100vh;",
//!             Workbench {
//!                 rail: rsx! {
//!                     ActivityRail {
//!                         ActivityButton { label: "Code", active: true, onclick: move |_| {} }
//!                     }
//!                 },
//!                 status: rsx! {
//!                     StatusBar {
//!                         left: rsx! { StatusItem { StatusDot { tone: StatusTone::Good } "Ready" } },
//!                     }
//!                 },
//!                 PanelWorkspace { panels, initial_layout: layout }
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! Panel ids identify content; tile and split ids identify layout positions.
//! All three should remain stable across releases when persisted layouts need
//! to survive. A panel's `home` tile is used only when that panel is newly
//! discovered and absent from a restored layout.
//!
//! At runtime:
//!
//! - drag a tab to a tile center to attach it as a tab;
//! - drag to an edge to create a split;
//! - use the tab-bar actions to split right or down;
//! - use `Alt+Shift+Arrow` to split from the keyboard;
//! - use `Alt+Shift+PageUp/PageDown` to move across tile groups;
//! - drag or arrow-key a separator to resize it; double-click resets it.

mod dom;
mod icons;
mod model;
mod shell;
mod style;
mod workspace;

pub use dom::focus_after_render;
pub use model::{
    DockZone, LayoutNode, PanelId, PanelLayout, PanelPlacement, SplitAxis, SplitId, Tile, TileId,
};
pub use shell::{
    ActivityButton, ActivityRail, StatusBar, StatusDot, StatusItem, StatusMessage, StatusTone,
    Workbench,
};
pub use style::STYLE as STYLESHEET;
pub use workspace::{Panel, PanelWorkspace};

/// Everything you typically need.
pub mod prelude {
    pub use crate::{
        ActivityButton, ActivityRail, DockZone, LayoutNode, Panel, PanelId, PanelLayout,
        PanelPlacement, PanelWorkspace, SplitAxis, StatusBar, StatusDot, StatusItem, StatusMessage,
        StatusTone, TileId, Workbench,
    };
}
