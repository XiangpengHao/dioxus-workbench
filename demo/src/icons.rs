//! Rail icons for the demo. The library deliberately ships no icon set;
//! applications bring their own, and any 16×16 stroke SVG sits well.

use dioxus::prelude::*;

#[component]
pub fn CodeIcon() -> Element {
    rsx! {
        svg { view_box: "0 0 16 16", "aria-hidden": "true",
            path { d: "m5.5 4.5-3.5 3.5 3.5 3.5M10.5 4.5 14 8l-3.5 3.5" }
        }
    }
}

#[component]
pub fn DataIcon() -> Element {
    rsx! {
        svg { view_box: "0 0 16 16", "aria-hidden": "true",
            ellipse { cx: "8", cy: "3.75", rx: "5.5", ry: "2.25" }
            path { d: "M2.5 3.75v8.5c0 1.24 2.46 2.25 5.5 2.25s5.5-1.01 5.5-2.25v-8.5" }
            path { d: "M2.5 8c0 1.24 2.46 2.25 5.5 2.25S13.5 9.24 13.5 8" }
        }
    }
}

#[component]
pub fn BookIcon() -> Element {
    rsx! {
        svg { view_box: "0 0 16 16", "aria-hidden": "true",
            path { d: "M8 3.5C6.5 2.5 4.5 2.25 2.5 2.5v10.5c2-.25 4 0 5.5 1 1.5-1 3.5-1.25 5.5-1V2.5c-2-.25-4 0-5.5 1z" }
            path { d: "M8 3.5V14" }
        }
    }
}

#[component]
pub fn GearIcon() -> Element {
    rsx! {
        svg { view_box: "0 0 16 16", "aria-hidden": "true",
            circle { cx: "8", cy: "8", r: "2.25" }
            path { d: "M8 1.75v2M8 12.25v2M1.75 8h2M12.25 8h2M3.6 3.6l1.4 1.4M11 11l1.4 1.4M12.4 3.6 11 5M5 11l-1.4 1.4" }
        }
    }
}
