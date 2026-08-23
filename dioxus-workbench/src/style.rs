//! Stylesheet installation.
//!
//! The stylesheet is compiled into the library and injected as a
//! `document::Style`, so the crate works on every renderer without an asset
//! pipeline. A context marker keeps one workbench from installing it twice:
//! the outermost [`Workbench`](crate::Workbench) or
//! [`PanelWorkspace`](crate::PanelWorkspace) owns the tag and everything
//! nested below it skips its own.

use dioxus::prelude::*;

/// The complete stylesheet, for applications that prefer to serve or bundle
/// it themselves instead of letting the components inject it.
pub static STYLE: &str = include_str!("style.css");

#[derive(Clone, Copy)]
struct StyleInstalled;

/// Returns whether this component should render the style tag, and marks the
/// subtree so descendants do not render their own.
pub(crate) fn use_style_owner() -> bool {
    // Read before providing: our own provider must not answer the question.
    let inherited = use_hook(|| try_consume_context::<StyleInstalled>().is_some());
    use_context_provider(|| StyleInstalled);
    !inherited
}
