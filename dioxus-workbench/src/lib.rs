//! # dioxus-workbench
//!
//! A dockable panel workbench for [Dioxus](https://dioxuslabs.com): the
//! editor-style shell — tabbed panel groups, drag-to-dock, resizable splits,
//! an activity rail, and a status bar — in pure Rust. No `web-sys`, no
//! JavaScript dependencies: the only scripting is a handful of
//! renderer-agnostic `document::eval` calls for pointer capture, focus, and
//! drag priming, so web and desktop behave the same.
//!
//! Applications register content as [`Panel`] values and describe only their
//! preferred starting arrangement. [`PanelWorkspace`] owns the durable layout,
//! tab activation, tiling, split resizing, drag previews, keyboard
//! equivalents, and the common panel chrome. [`Workbench`], [`ActivityRail`],
//! and [`StatusBar`] wrap it in the surrounding application frame.
//!
//! ```no_run
//! use dioxus::prelude::*;
//! use dioxus_workbench::prelude::*;
//!
//! #[component]
//! fn FileTree() -> Element { rsx! { "files" } }
//! #[component]
//! fn Editor() -> Element { rsx! { "editor" } }
//! #[component]
//! fn Terminal() -> Element { rsx! { "terminal" } }
//!
//! #[component]
//! fn App() -> Element {
//!     let panels = vec![
//!         Panel::new("files", "Files", "side", rsx! { FileTree {} }),
//!         Panel::new("editor", "Editor", "main", rsx! { Editor {} }),
//!         Panel::new("terminal", "Terminal", "bottom", rsx! { Terminal {} }),
//!     ];
//!
//!     // With no `initial_layout`, the workspace derives one from panel
//!     // homes: one tile per distinct home, left to right. Pass a
//!     // `PanelLayout` to control the first-open tree; wrap in `Workbench`
//!     // with an `ActivityRail` and `StatusBar` for the full shell.
//!     rsx! {
//!         div { style: "width: 100vw; height: 100vh;",
//!             PanelWorkspace { panels }
//!         }
//!     }
//! }
//! # fn main() {}
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
//!
//! ## Panel state
//!
//! Panel content stays mounted while tabs switch within a group, so scroll
//! positions, form values, and internal signals survive activation. A
//! structural move — docking a panel into another group, an edge split
//! replacing a tile — rebuilds that region of the element tree and remounts
//! the panels involved. State that must survive docking belongs outside the
//! panel: a signal owned by the application, a context, or a store the
//! panel's content reads.

mod dom;
mod icons;
mod model;
mod shell;
mod strings;
mod style;
mod workspace;

pub use dom::focus_after_render;
pub use model::{
    DockZone, LayoutError, LayoutNode, PanelId, PanelLayout, PanelPlacement, SplitAxis, SplitId,
    Tile, TileId,
};
pub use shell::{
    ActivityButton, ActivityRail, StatusBar, StatusDot, StatusItem, StatusMessage, StatusTone,
    Workbench,
};
pub use strings::WorkbenchStrings;
pub use style::STYLE as STYLESHEET;
pub use workspace::{Panel, PanelWorkspace, TabMenuRequest};

/// Everything you typically need.
pub mod prelude {
    pub use crate::{
        ActivityButton, ActivityRail, DockZone, LayoutError, LayoutNode, Panel, PanelId,
        PanelLayout, PanelPlacement, PanelWorkspace, SplitAxis, StatusBar, StatusDot, StatusItem,
        StatusMessage, StatusTone, TabMenuRequest, TileId, Workbench, WorkbenchStrings,
    };
}
