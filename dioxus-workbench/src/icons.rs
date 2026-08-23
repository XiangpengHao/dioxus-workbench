//! The handful of icons the chrome itself needs. Application content brings
//! its own icon set; these stay private so the crate never becomes one.

use dioxus::prelude::*;

#[component]
pub(crate) fn CloseIcon() -> Element {
    rsx! {
        svg { class: "wb-icon", view_box: "0 0 16 16", "aria-hidden": "true",
            path { d: "M4 4l8 8M12 4l-8 8" }
        }
    }
}

#[component]
pub(crate) fn SplitRightIcon() -> Element {
    rsx! {
        svg { class: "wb-icon", view_box: "0 0 16 16", "aria-hidden": "true",
            rect { x: "2.5", y: "3", width: "11", height: "10", rx: "1" }
            path { d: "M9 3v10" }
        }
    }
}

#[component]
pub(crate) fn SplitDownIcon() -> Element {
    rsx! {
        svg { class: "wb-icon", view_box: "0 0 16 16", "aria-hidden": "true",
            rect { x: "2.5", y: "3", width: "11", height: "10", rx: "1" }
            path { d: "M2.5 8h11" }
        }
    }
}

#[component]
pub(crate) fn DockCompassIcon() -> Element {
    rsx! {
        svg { class: "wb-compass-icon", view_box: "0 0 28 28", "aria-hidden": "true",
            rect { x: "10", y: "10", width: "8", height: "8", rx: "1" }
            path { d: "M5 10h4v8H5zm14 0h4v8h-4zM10 5h8v4h-8zm0 14h8v4h-8z" }
        }
    }
}
