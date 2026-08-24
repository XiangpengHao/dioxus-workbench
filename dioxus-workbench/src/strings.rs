//! The chrome's own words, gathered in one overridable place.
//!
//! The library draws a small amount of text itself: empty states, action
//! labels and tooltips, and accessible names. Applications that localize (or
//! simply disagree with the defaults) pass a [`WorkbenchStrings`] to
//! [`PanelWorkspace`](crate::PanelWorkspace); everything else keeps the
//! English defaults.

/// Every string the workspace chrome draws itself.
///
/// Construct with struct-update syntax over the defaults:
///
/// ```ignore
/// let strings = WorkbenchStrings {
///     empty_tile_hint: "Déposez un onglet ici, ou fermez ce groupe.".into(),
///     ..Default::default()
/// };
/// ```
///
/// Fields containing `{title}` or `{panel}` are templates; the placeholder is
/// replaced with the panel's title.
#[derive(Clone, Debug, PartialEq)]
pub struct WorkbenchStrings {
    /// Accessible name for a tab strip whose group is empty.
    pub tab_group_label: String,
    /// Accessible name for a tab strip, naming the active panel.
    /// `{panel}` is replaced with the active panel's title.
    pub tab_group_labelled: String,
    /// The teaching tooltip appended to every tab's title.
    pub tab_hint: String,
    /// Label and tooltip for a closable tab's close affordance.
    /// `{title}` is replaced with the panel's title.
    pub close_tab: String,
    /// Tab-strip placeholder when a group holds no panels.
    pub empty_group_label: String,
    /// Heading of an empty group's body.
    pub empty_tile_title: String,
    /// Guidance under an empty group's heading.
    pub empty_tile_hint: String,
    /// The keyboard vocabulary, taught where keyboard users can read it.
    pub empty_tile_keys: String,
    /// Accessible name for the split-right action.
    pub split_right_label: String,
    /// Tooltip for the split-right action.
    pub split_right_hint: String,
    /// Accessible name for the split-down action.
    pub split_down_label: String,
    /// Tooltip for the split-down action.
    pub split_down_hint: String,
    /// Accessible name for closing an empty group.
    pub close_empty_label: String,
    /// Tooltip for closing an empty group.
    pub close_empty_hint: String,
    /// Accessible name for a splitter between groups.
    pub splitter_label: String,
    /// Tooltip for a splitter between groups.
    pub splitter_hint: String,
}

impl Default for WorkbenchStrings {
    fn default() -> Self {
        Self {
            tab_group_label: "Panel group".to_owned(),
            tab_group_labelled: "Panel group: {panel}".to_owned(),
            tab_hint: "Drag to dock. Alt+Shift+Arrows split; Alt+Shift+Page keys move.".to_owned(),
            close_tab: "Close {title}".to_owned(),
            empty_group_label: "Empty group".to_owned(),
            empty_tile_title: "Empty panel group".to_owned(),
            empty_tile_hint: "Drag a panel tab here, or close this group.".to_owned(),
            empty_tile_keys:
                "Alt+Shift+Arrows split · Alt+Shift+Page keys move · Arrows switch tabs".to_owned(),
            split_right_label: "Split panel group right".to_owned(),
            split_right_hint: "Split right (Alt+Shift+Right)".to_owned(),
            split_down_label: "Split panel group down".to_owned(),
            split_down_hint: "Split down (Alt+Shift+Down)".to_owned(),
            close_empty_label: "Close empty panel group".to_owned(),
            close_empty_hint: "Close empty group".to_owned(),
            splitter_label: "Resize panel groups".to_owned(),
            splitter_hint:
                "Drag to resize. Arrow keys resize; Shift moves farther; double-click resets."
                    .to_owned(),
        }
    }
}
