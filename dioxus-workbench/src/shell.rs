//! The low-frequency chrome around a workspace: the application frame, the
//! activity rail, and the status bar.
//!
//! These components own arrangement and interaction states only. What an
//! activity is, what a status item says, and what happens when either is
//! pressed belongs to the application.

use dioxus::prelude::*;

use crate::style::{use_style_owner, STYLE};

/// The application frame: an optional [`ActivityRail`] on the left, the main
/// region in the middle, and an optional [`StatusBar`] along the bottom that
/// spans the full width.
///
/// The frame fills its parent, so give the parent a size — typically the
/// viewport (`width: 100vw; height: 100vh`).
#[component]
pub fn Workbench(
    /// The left rail, usually an [`ActivityRail`].
    #[props(default)]
    rail: Option<Element>,
    /// The bottom bar, usually a [`StatusBar`].
    #[props(default)]
    status: Option<Element>,
    children: Element,
) -> Element {
    let owns_style = use_style_owner();
    rsx! {
        if owns_style {
            document::Style { {STYLE} }
        }
        div { class: "wb-shell",
            div { class: "wb-shell-body",
                if let Some(rail) = rail {
                    {rail}
                }
                main { class: "wb-main", {children} }
            }
            if let Some(status) = status {
                {status}
            }
        }
    }
}

/// A vertical strip of top-level activities, in the position editors reserve
/// for their activity bar. Children are usually [`ActivityButton`]s.
#[component]
pub fn ActivityRail(
    /// Accessible name for the rail's navigation landmark.
    #[props(default = String::from("Activities"))]
    label: String,
    children: Element,
) -> Element {
    rsx! {
        nav { class: "wb-rail", "aria-label": label, {children} }
    }
}

/// One activity in the rail: an icon over a short label. `active` renders the
/// selected wash and edge bar; `bottom` pins the item (and any that follow it)
/// to the rail's end, the conventional place for settings.
#[component]
pub fn ActivityButton(
    label: String,
    #[props(default)] icon: Option<Element>,
    #[props(default)] active: bool,
    #[props(default)] bottom: bool,
    #[props(default)] id: Option<String>,
    #[props(default)] title: Option<String>,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    let class = match (active, bottom) {
        (true, true) => "wb-rail-item wb-rail-item-active wb-rail-item-bottom",
        (true, false) => "wb-rail-item wb-rail-item-active",
        (false, true) => "wb-rail-item wb-rail-item-bottom",
        (false, false) => "wb-rail-item",
    };
    rsx! {
        button {
            r#type: "button",
            id,
            class,
            "aria-pressed": active,
            title,
            onclick: move |event| onclick.call(event),
            if let Some(icon) = icon {
                {icon}
            }
            span { "{label}" }
        }
    }
}

/// The color a status entry speaks in. Neutral is soft ink; the others exist
/// for facts worth a color — reserve them for state, not decoration.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StatusTone {
    #[default]
    Neutral,
    Accent,
    Good,
    Caution,
    Danger,
}

impl StatusTone {
    fn class(self) -> &'static str {
        match self {
            Self::Neutral => "",
            Self::Accent => "wb-tone-accent",
            Self::Good => "wb-tone-good",
            Self::Caution => "wb-tone-caution",
            Self::Danger => "wb-tone-danger",
        }
    }
}

/// A single-row bar of small facts. The left group carries context, the right
/// group carries state, and the message column between them holds the latest
/// notification; when space runs out, the left group is what gives way.
#[component]
pub fn StatusBar(
    #[props(default)] left: Option<Element>,
    #[props(default)] message: Option<Element>,
    #[props(default)] right: Option<Element>,
    /// Accessible name for the bar's contentinfo landmark.
    #[props(default = String::from("Status"))]
    label: String,
) -> Element {
    rsx! {
        footer { class: "wb-status-bar", "aria-label": label,
            div { class: "wb-status-group",
                if let Some(left) = left {
                    {left}
                }
            }
            if let Some(message) = message {
                {message}
            }
            div { class: "wb-status-group wb-status-group-right",
                if let Some(right) = right {
                    {right}
                }
            }
        }
    }
}

/// One fact in the status bar. With `onclick` it renders as a button and
/// gains hover and focus treatment; without, it is inert text.
#[component]
pub fn StatusItem(
    #[props(default)] tone: StatusTone,
    #[props(default)] title: Option<String>,
    #[props(default)] onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let class = if tone == StatusTone::Neutral {
        "wb-status-item".to_owned()
    } else {
        format!("wb-status-item {}", tone.class())
    };
    rsx! {
        if let Some(onclick) = onclick {
            button {
                r#type: "button",
                class: "{class}",
                title,
                onclick: move |event| onclick.call(event),
                {children}
            }
        } else {
            span { class: "{class}", title, {children} }
        }
    }
}

/// A small colored dot for connection-style facts, sized to sit inside a
/// [`StatusItem`] before its text.
#[component]
pub fn StatusDot(#[props(default)] tone: StatusTone) -> Element {
    let class = if tone == StatusTone::Neutral {
        "wb-status-dot".to_owned()
    } else {
        format!("wb-status-dot {}", tone.class())
    };
    rsx! {
        span { class: "{class}", "aria-hidden": "true" }
    }
}

/// The latest transient notification, announced politely — or assertively
/// when the tone is [`StatusTone::Danger`]. Render it in the [`StatusBar`]
/// `message` slot.
#[component]
pub fn StatusMessage(
    #[props(default)] tone: StatusTone,
    #[props(default)] title: Option<String>,
    children: Element,
) -> Element {
    let class = if tone == StatusTone::Neutral {
        "wb-status-message".to_owned()
    } else {
        format!("wb-status-message {}", tone.class())
    };
    let alert = tone == StatusTone::Danger;
    rsx! {
        span {
            class: "{class}",
            title,
            role: if alert { "alert" } else { "status" },
            "aria-live": if alert { "assertive" } else { "polite" },
            {children}
        }
    }
}
