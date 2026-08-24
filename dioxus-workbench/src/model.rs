use std::collections::HashSet;

use serde::{Deserialize, Serialize};

const LAYOUT_VERSION: u8 = 1;
pub(crate) const MIN_SPLIT_RATIO: f64 = 0.1;
pub(crate) const MAX_SPLIT_RATIO: f64 = 0.9;

/// A stable application-owned identity for panel content.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PanelId(String);

impl PanelId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for PanelId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for PanelId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for PanelId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// A stable identity for a tab group in the layout tree.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TileId(String);

impl TileId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for TileId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for TileId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for TileId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// A stable identity for a resizable branch in the layout tree.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SplitId(String);

impl SplitId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for SplitId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for SplitId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for SplitId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SplitAxis {
    /// Children sit left and right of one another.
    Horizontal,
    /// Children sit above and below one another.
    Vertical,
}

/// Why a persisted or hand-built layout was rejected.
///
/// Returned by [`PanelLayout::try_encode`], [`PanelLayout::try_decode`], and
/// [`PanelLayout::validate`]. The `Option`-returning `encode`/`decode` remain
/// for callers that only care about the happy path.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum LayoutError {
    /// The persisted value is not a layout at all — malformed JSON or the
    /// wrong shape.
    Malformed,
    /// The value was written by a different format version. This is the
    /// migration hook: the raw string is still yours, so translate it and
    /// decode again — or fall back to the default arrangement.
    Version { found: u8 },
    /// The tree violates an invariant; the message names the first violation
    /// found (duplicate ids, an active tab the tile does not contain, an
    /// out-of-range ratio).
    Invalid(String),
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed => write!(formatter, "not a panel layout"),
            Self::Version { found } => write!(
                formatter,
                "layout format version {found} (this crate reads version {LAYOUT_VERSION})"
            ),
            Self::Invalid(reason) => write!(formatter, "invalid layout: {reason}"),
        }
    }
}

impl std::error::Error for LayoutError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DockZone {
    Center,
    Left,
    Right,
    Top,
    Bottom,
}

impl DockZone {
    pub const fn axis(self) -> Option<SplitAxis> {
        match self {
            Self::Center => None,
            Self::Left | Self::Right => Some(SplitAxis::Horizontal),
            Self::Top | Self::Bottom => Some(SplitAxis::Vertical),
        }
    }

    pub const fn before(self) -> bool {
        matches!(self, Self::Left | Self::Top)
    }
}

/// One tab group. Only an explicit operator-created Split remains when empty.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tile {
    pub id: TileId,
    pub panels: Vec<PanelId>,
    pub active: Option<PanelId>,
    #[serde(default)]
    preserve_when_empty: bool,
}

impl Tile {
    pub fn new<I, P>(id: impl Into<TileId>, panels: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PanelId>,
    {
        let panels = panels.into_iter().map(Into::into).collect::<Vec<_>>();
        let active = panels.first().cloned();
        Self {
            id: id.into(),
            panels,
            active,
            preserve_when_empty: false,
        }
    }

    pub fn empty(id: impl Into<TileId>) -> Self {
        let mut tile = Self::new(id, std::iter::empty::<PanelId>());
        tile.preserve_when_empty = true;
        tile
    }
}

/// The recursive tiling tree. Ratios describe the first child's share.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LayoutNode {
    Tile(Tile),
    Split {
        id: SplitId,
        axis: SplitAxis,
        ratio: f64,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
}

impl LayoutNode {
    pub fn tile<I, P>(id: impl Into<TileId>, panels: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PanelId>,
    {
        Self::Tile(Tile::new(id, panels))
    }

    pub fn empty_tile(id: impl Into<TileId>) -> Self {
        Self::Tile(Tile::empty(id))
    }

    pub fn split(
        id: impl Into<SplitId>,
        axis: SplitAxis,
        ratio: f64,
        first: Self,
        second: Self,
    ) -> Self {
        Self::Split {
            id: id.into(),
            axis,
            ratio: clamp_ratio(ratio),
            first: Box::new(first),
            second: Box::new(second),
        }
    }
}

/// Where a dynamically registered panel should appear if persistence has not
/// seen it before. This matters for content that arrives at runtime, such as
/// hot-plugged devices or documents opened by the user.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PanelPlacement {
    pub panel: PanelId,
    pub home: TileId,
    pub zone: DockZone,
}

impl PanelPlacement {
    pub fn new(panel: impl Into<PanelId>, home: impl Into<TileId>) -> Self {
        Self {
            panel: panel.into(),
            home: home.into(),
            zone: DockZone::Center,
        }
    }

    /// Choose how a newly registered panel joins an occupied home tile.
    /// Existing panels keep their persisted position regardless of this hint.
    pub fn with_zone(mut self, zone: DockZone) -> Self {
        self.zone = zone;
        self
    }
}

/// Serializable, mutation-safe state for a complete panel workspace.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PanelLayout {
    version: u8,
    next_id: u64,
    pub root: LayoutNode,
}

impl PanelLayout {
    pub fn new(root: LayoutNode) -> Self {
        Self {
            version: LAYOUT_VERSION,
            next_id: 1,
            root,
        }
    }

    pub fn single(tile: impl Into<TileId>, panel: impl Into<PanelId>) -> Self {
        Self::new(LayoutNode::tile(tile, [panel.into()]))
    }

    /// A first-open arrangement derived from panel homes alone: one tile per
    /// distinct `home`, in first-registration order, split left to right in
    /// equal shares, with each panel attached through the same reconciliation
    /// that places late arrivals (so `with_home_zone` hints apply).
    ///
    /// [`PanelWorkspace`](crate::PanelWorkspace) uses this when no
    /// `initial_layout` is given; it also makes a sensible `reset_layout` for
    /// applications that never hand-author a tree.
    pub fn from_homes(placements: &[PanelPlacement]) -> Self {
        let mut homes = Vec::<TileId>::new();
        for placement in placements {
            if !homes.contains(&placement.home) {
                homes.push(placement.home.clone());
            }
        }
        let mut layout = match homes.split_last() {
            None => Self::new(LayoutNode::empty_tile("wb-home")),
            Some((last, rest)) => {
                let mut node = LayoutNode::tile(last.clone(), std::iter::empty::<PanelId>());
                let total = homes.len();
                for (index, home) in rest.iter().enumerate().rev() {
                    node = LayoutNode::split(
                        format!("wb-home-split-{}", index + 1),
                        SplitAxis::Horizontal,
                        1.0 / (total - index) as f64,
                        LayoutNode::tile(home.clone(), std::iter::empty::<PanelId>()),
                        node,
                    );
                }
                Self::new(node)
            }
        };
        layout.reconcile(placements);
        layout
    }

    pub fn encode(&self) -> Option<String> {
        self.try_encode().ok()
    }

    /// Encode for persistence, or say precisely why the tree cannot be
    /// persisted. Layouts mutated only through this type's methods always
    /// encode; [`LayoutError::Invalid`] appears when a tree was assembled
    /// directly through the public node fields and broke an invariant.
    pub fn try_encode(&self) -> Result<String, LayoutError> {
        self.validate()?;
        serde_json::to_string(self).map_err(|_| LayoutError::Malformed)
    }

    pub fn decode(value: &str) -> Option<Self> {
        Self::try_decode(value).ok()
    }

    /// Decode a persisted layout, distinguishing corruption from version
    /// drift. [`LayoutError::Version`] carries the version that wrote the
    /// value, so an application can migrate the raw string instead of
    /// silently discarding every saved arrangement.
    pub fn try_decode(value: &str) -> Result<Self, LayoutError> {
        let layout = serde_json::from_str::<Self>(value).map_err(|_| LayoutError::Malformed)?;
        layout.validate()?;
        Ok(layout)
    }

    pub fn valid(&self) -> bool {
        self.validate().is_ok()
    }

    /// Check every invariant: format version, unique tile/split/panel ids,
    /// active tabs that exist, and finite in-range ratios.
    pub fn validate(&self) -> Result<(), LayoutError> {
        if self.version != LAYOUT_VERSION {
            return Err(LayoutError::Version {
                found: self.version,
            });
        }
        let mut tiles = HashSet::new();
        let mut splits = HashSet::new();
        let mut panels = HashSet::new();
        validate_node(&self.root, &mut tiles, &mut splits, &mut panels)
    }

    pub fn reconciled(&self, placements: &[PanelPlacement]) -> Self {
        let mut next = self.clone();
        next.reconcile(placements);
        next
    }

    /// Remove unavailable panel ids, repair active tabs, and attach any newly
    /// registered panels to their preferred home tile (or the first tile).
    ///
    /// Unclaimed empty tiles are collapsed, unless the tile is a named home for
    /// the next registry. Explicit empty tiles created by the operator carry a
    /// durable marker and remain valid drop targets across persistence.
    pub fn reconcile(&mut self, placements: &[PanelPlacement]) {
        let available = placements
            .iter()
            .map(|placement| placement.panel.clone())
            .collect::<HashSet<_>>();
        let protected_homes = placements
            .iter()
            .map(|placement| placement.home.clone())
            .collect::<HashSet<_>>();
        let fallback_tile = first_tile_id(&self.root).clone();
        let mut seen = HashSet::new();
        reconcile_node(&mut self.root, &available, &mut seen);
        self.root = prune_unclaimed_empty_tiles(self.root.clone(), &protected_homes)
            .unwrap_or_else(|| LayoutNode::empty_tile(fallback_tile));

        for placement in placements {
            if seen.contains(&placement.panel) {
                continue;
            }
            let target = if contains_tile(&self.root, &placement.home) {
                placement.home.clone()
            } else {
                first_tile_id(&self.root).clone()
            };
            let home_is_empty = self
                .tile(&target)
                .is_some_and(|tile| tile.panels.is_empty());
            let attached = if placement.zone == DockZone::Center || home_is_empty {
                attach_to_tile(&mut self.root, &target, placement.panel.clone())
            } else {
                let tile_id = self.allocate_tile_id();
                let split_id = self.allocate_split_id();
                insert_split(
                    &mut self.root,
                    &target,
                    split_id,
                    Tile::new(tile_id, [placement.panel.clone()]),
                    placement.zone,
                )
            };
            if attached {
                seen.insert(placement.panel.clone());
            }
        }
    }

    pub fn activate(&mut self, panel: &PanelId) -> bool {
        activate_panel(&mut self.root, panel)
    }

    pub fn tile_for_panel(&self, panel: &PanelId) -> Option<TileId> {
        find_panel_tile(&self.root, panel).cloned()
    }

    pub fn tile(&self, id: &TileId) -> Option<&Tile> {
        find_tile(&self.root, id)
    }

    pub fn tile_count(&self) -> usize {
        tile_ids(&self.root).len()
    }

    /// Every split id in the tree, in first-child order.
    pub fn split_ids(&self) -> Vec<SplitId> {
        let mut ids = Vec::new();
        collect_split_ids(&self.root, &mut ids);
        ids
    }

    pub fn split_ratio(&self, id: &SplitId) -> Option<f64> {
        find_split(&self.root, id).map(|(_, ratio)| ratio)
    }

    pub fn set_split_ratio(&mut self, id: &SplitId, ratio: f64) -> bool {
        let Some((_, current)) = find_split_mut(&mut self.root, id) else {
            return false;
        };
        let next = clamp_ratio(ratio);
        if (next - *current).abs() <= f64::EPSILON {
            return false;
        }
        *current = next;
        true
    }

    /// Move a panel into a tile or create an edge split around that tile.
    pub fn dock_panel(&mut self, panel: &PanelId, target: &TileId, zone: DockZone) -> bool {
        let Some(source) = self.tile_for_panel(panel) else {
            return false;
        };
        if !contains_tile(&self.root, target) {
            return false;
        }
        if source == *target && zone == DockZone::Center {
            return self.activate(panel);
        }
        if source == *target
            && self
                .tile(&source)
                .is_some_and(|tile| tile.panels.len() == 1)
        {
            return false;
        }

        let Some(next_root) = remove_panel(self.root.clone(), panel) else {
            return false;
        };
        self.root = next_root;

        if zone == DockZone::Center {
            return attach_to_tile(&mut self.root, target, panel.clone());
        }

        let tile_id = self.allocate_tile_id();
        let split_id = self.allocate_split_id();
        insert_split(
            &mut self.root,
            target,
            split_id,
            Tile::new(tile_id, [panel.clone()]),
            zone,
        )
    }

    /// Split a tile without fabricating panel content. The empty sibling is a
    /// visible drop target and can be closed if the operator changes their mind.
    pub fn split_tile(&mut self, target: &TileId, zone: DockZone) -> Option<TileId> {
        zone.axis()?;
        if !contains_tile(&self.root, target) {
            return None;
        }
        let tile_id = self.allocate_tile_id();
        let split_id = self.allocate_split_id();
        let inserted = insert_split(
            &mut self.root,
            target,
            split_id,
            Tile::empty(tile_id.clone()),
            zone,
        );
        inserted.then_some(tile_id)
    }

    /// Match editor-style Split: move the active tab when a group has several;
    /// otherwise open an empty sibling ready to receive another panel.
    pub fn split_active(&mut self, tile: &TileId, zone: DockZone) -> bool {
        let Some(active) = self.tile(tile).and_then(|tile| tile.active.clone()) else {
            return self.split_tile(tile, zone).is_some();
        };
        if self.tile(tile).is_some_and(|tile| tile.panels.len() > 1) {
            self.dock_panel(&active, tile, zone)
        } else {
            self.split_tile(tile, zone).is_some()
        }
    }

    pub fn remove_empty_tile(&mut self, tile: &TileId) -> bool {
        if self
            .tile(tile)
            .is_none_or(|candidate| !candidate.panels.is_empty())
        {
            return false;
        }
        let Some(root) = remove_tile(self.root.clone(), tile) else {
            return false;
        };
        self.root = root;
        true
    }

    pub fn move_panel_by_tile(&mut self, panel: &PanelId, delta: isize) -> bool {
        let tiles = tile_ids(&self.root);
        if tiles.len() < 2 {
            return false;
        }
        let Some(source) = self.tile_for_panel(panel) else {
            return false;
        };
        let Some(index) = tiles.iter().position(|tile| tile == &source) else {
            return false;
        };
        let target_index = (index as isize + delta).rem_euclid(tiles.len() as isize) as usize;
        self.dock_panel(panel, &tiles[target_index], DockZone::Center)
    }

    fn allocate_tile_id(&mut self) -> TileId {
        loop {
            let id = TileId::new(format!("tile-{}", self.next_id));
            self.next_id += 1;
            if !contains_tile(&self.root, &id) {
                return id;
            }
        }
    }

    fn allocate_split_id(&mut self) -> SplitId {
        loop {
            let id = SplitId::new(format!("split-{}", self.next_id));
            self.next_id += 1;
            if find_split(&self.root, &id).is_none() {
                return id;
            }
        }
    }
}

fn clamp_ratio(ratio: f64) -> f64 {
    if ratio.is_finite() {
        ratio.clamp(MIN_SPLIT_RATIO, MAX_SPLIT_RATIO)
    } else {
        0.5
    }
}

fn validate_node(
    node: &LayoutNode,
    tiles: &mut HashSet<TileId>,
    splits: &mut HashSet<SplitId>,
    panels: &mut HashSet<PanelId>,
) -> Result<(), LayoutError> {
    match node {
        LayoutNode::Tile(tile) => {
            if !tiles.insert(tile.id.clone()) {
                return Err(LayoutError::Invalid(format!(
                    "duplicate tile id `{}`",
                    tile.id
                )));
            }
            for panel in &tile.panels {
                if !panels.insert(panel.clone()) {
                    return Err(LayoutError::Invalid(format!(
                        "panel `{panel}` appears in more than one tile"
                    )));
                }
            }
            match &tile.active {
                Some(active) if !tile.panels.contains(active) => {
                    Err(LayoutError::Invalid(format!(
                        "tile `{}` activates `{active}`, which it does not contain",
                        tile.id
                    )))
                }
                None if !tile.panels.is_empty() => Err(LayoutError::Invalid(format!(
                    "tile `{}` holds panels but activates none of them",
                    tile.id
                ))),
                _ => Ok(()),
            }
        }
        LayoutNode::Split {
            id,
            ratio,
            first,
            second,
            ..
        } => {
            if !splits.insert(id.clone()) {
                return Err(LayoutError::Invalid(format!("duplicate split id `{id}`")));
            }
            if !ratio.is_finite() || !(MIN_SPLIT_RATIO..=MAX_SPLIT_RATIO).contains(ratio) {
                return Err(LayoutError::Invalid(format!(
                    "split `{id}` ratio {ratio} is outside {MIN_SPLIT_RATIO}..={MAX_SPLIT_RATIO}"
                )));
            }
            validate_node(first, tiles, splits, panels)?;
            validate_node(second, tiles, splits, panels)
        }
    }
}

fn reconcile_node(
    node: &mut LayoutNode,
    available: &HashSet<PanelId>,
    seen: &mut HashSet<PanelId>,
) {
    match node {
        LayoutNode::Tile(tile) => {
            tile.panels
                .retain(|panel| available.contains(panel) && seen.insert(panel.clone()));
            if tile
                .active
                .as_ref()
                .is_none_or(|active| !tile.panels.contains(active))
            {
                tile.active = tile.panels.first().cloned();
            }
        }
        LayoutNode::Split {
            ratio,
            first,
            second,
            ..
        } => {
            // Trees assembled directly through the public node fields can
            // carry ratios the constructors would have clamped. Heal them at
            // the boundary instead of rendering a degenerate pane and then
            // refusing to persist it.
            *ratio = clamp_ratio(*ratio);
            reconcile_node(first, available, seen);
            reconcile_node(second, available, seen);
        }
    }
}

fn prune_unclaimed_empty_tiles(
    node: LayoutNode,
    protected_homes: &HashSet<TileId>,
) -> Option<LayoutNode> {
    match node {
        LayoutNode::Tile(tile) => (!tile.panels.is_empty()
            || tile.preserve_when_empty
            || protected_homes.contains(&tile.id))
        .then_some(LayoutNode::Tile(tile)),
        LayoutNode::Split {
            id,
            axis,
            ratio,
            first,
            second,
        } => match (
            prune_unclaimed_empty_tiles(*first, protected_homes),
            prune_unclaimed_empty_tiles(*second, protected_homes),
        ) {
            (Some(first), Some(second)) => Some(LayoutNode::Split {
                id,
                axis,
                ratio,
                first: Box::new(first),
                second: Box::new(second),
            }),
            (Some(node), None) | (None, Some(node)) => Some(node),
            (None, None) => None,
        },
    }
}

fn contains_tile(node: &LayoutNode, id: &TileId) -> bool {
    find_tile(node, id).is_some()
}

fn first_tile_id(node: &LayoutNode) -> &TileId {
    match node {
        LayoutNode::Tile(tile) => &tile.id,
        LayoutNode::Split { first, .. } => first_tile_id(first),
    }
}

fn find_tile<'a>(node: &'a LayoutNode, id: &TileId) -> Option<&'a Tile> {
    match node {
        LayoutNode::Tile(tile) => (&tile.id == id).then_some(tile),
        LayoutNode::Split { first, second, .. } => {
            find_tile(first, id).or_else(|| find_tile(second, id))
        }
    }
}

fn attach_to_tile(node: &mut LayoutNode, id: &TileId, panel: PanelId) -> bool {
    match node {
        LayoutNode::Tile(tile) if &tile.id == id => {
            if !tile.panels.contains(&panel) {
                tile.panels.push(panel.clone());
            }
            tile.active = Some(panel);
            true
        }
        LayoutNode::Tile(_) => false,
        LayoutNode::Split { first, second, .. } => {
            attach_to_tile(first, id, panel.clone()) || attach_to_tile(second, id, panel)
        }
    }
}

fn activate_panel(node: &mut LayoutNode, panel: &PanelId) -> bool {
    match node {
        LayoutNode::Tile(tile) if tile.panels.contains(panel) => {
            let changed = tile.active.as_ref() != Some(panel);
            tile.active = Some(panel.clone());
            changed
        }
        LayoutNode::Tile(_) => false,
        LayoutNode::Split { first, second, .. } => {
            activate_panel(first, panel) || activate_panel(second, panel)
        }
    }
}

fn find_panel_tile<'a>(node: &'a LayoutNode, panel: &PanelId) -> Option<&'a TileId> {
    match node {
        LayoutNode::Tile(tile) => tile.panels.contains(panel).then_some(&tile.id),
        LayoutNode::Split { first, second, .. } => {
            find_panel_tile(first, panel).or_else(|| find_panel_tile(second, panel))
        }
    }
}

fn find_split(node: &LayoutNode, id: &SplitId) -> Option<(SplitAxis, f64)> {
    match node {
        LayoutNode::Tile(_) => None,
        LayoutNode::Split {
            id: split_id,
            axis,
            ratio,
            first,
            second,
        } => {
            if split_id == id {
                Some((*axis, *ratio))
            } else {
                find_split(first, id).or_else(|| find_split(second, id))
            }
        }
    }
}

fn find_split_mut<'a>(node: &'a mut LayoutNode, id: &SplitId) -> Option<(SplitAxis, &'a mut f64)> {
    match node {
        LayoutNode::Tile(_) => None,
        LayoutNode::Split {
            id: split_id,
            axis,
            ratio,
            first,
            second,
        } => {
            if split_id == id {
                Some((*axis, ratio))
            } else {
                find_split_mut(first, id).or_else(|| find_split_mut(second, id))
            }
        }
    }
}

fn remove_panel(node: LayoutNode, panel: &PanelId) -> Option<LayoutNode> {
    match node {
        LayoutNode::Tile(mut tile) => {
            tile.panels.retain(|candidate| candidate != panel);
            if tile.panels.is_empty() {
                None
            } else {
                if tile
                    .active
                    .as_ref()
                    .is_none_or(|active| !tile.panels.contains(active))
                {
                    tile.active = tile.panels.first().cloned();
                }
                Some(LayoutNode::Tile(tile))
            }
        }
        LayoutNode::Split {
            id,
            axis,
            ratio,
            first,
            second,
        } => match (remove_panel(*first, panel), remove_panel(*second, panel)) {
            (Some(first), Some(second)) => Some(LayoutNode::Split {
                id,
                axis,
                ratio,
                first: Box::new(first),
                second: Box::new(second),
            }),
            (Some(node), None) | (None, Some(node)) => Some(node),
            (None, None) => None,
        },
    }
}

fn remove_tile(node: LayoutNode, target: &TileId) -> Option<LayoutNode> {
    match node {
        LayoutNode::Tile(tile) => (&tile.id != target).then_some(LayoutNode::Tile(tile)),
        LayoutNode::Split {
            id,
            axis,
            ratio,
            first,
            second,
        } => match (remove_tile(*first, target), remove_tile(*second, target)) {
            (Some(first), Some(second)) => Some(LayoutNode::Split {
                id,
                axis,
                ratio,
                first: Box::new(first),
                second: Box::new(second),
            }),
            (Some(node), None) | (None, Some(node)) => Some(node),
            (None, None) => None,
        },
    }
}

fn insert_split(
    node: &mut LayoutNode,
    target: &TileId,
    split_id: SplitId,
    tile: Tile,
    zone: DockZone,
) -> bool {
    match node {
        LayoutNode::Tile(existing) if &existing.id == target => {
            let Some(axis) = zone.axis() else {
                return false;
            };
            let old = LayoutNode::Tile(existing.clone());
            let new = LayoutNode::Tile(tile);
            let (first, second) = if zone.before() {
                (new, old)
            } else {
                (old, new)
            };
            *node = LayoutNode::split(split_id, axis, 0.5, first, second);
            true
        }
        LayoutNode::Tile(_) => false,
        LayoutNode::Split { first, second, .. } => {
            insert_split(first, target, split_id.clone(), tile.clone(), zone)
                || insert_split(second, target, split_id, tile, zone)
        }
    }
}

fn tile_ids(node: &LayoutNode) -> Vec<TileId> {
    let mut ids = Vec::new();
    collect_tile_ids(node, &mut ids);
    ids
}

fn collect_tile_ids(node: &LayoutNode, ids: &mut Vec<TileId>) {
    match node {
        LayoutNode::Tile(tile) => ids.push(tile.id.clone()),
        LayoutNode::Split { first, second, .. } => {
            collect_tile_ids(first, ids);
            collect_tile_ids(second, ids);
        }
    }
}

fn collect_split_ids(node: &LayoutNode, ids: &mut Vec<SplitId>) {
    match node {
        LayoutNode::Tile(_) => {}
        LayoutNode::Split {
            id, first, second, ..
        } => {
            ids.push(id.clone());
            collect_split_ids(first, ids);
            collect_split_ids(second, ids);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_tiles() -> PanelLayout {
        PanelLayout::new(LayoutNode::split(
            "root",
            SplitAxis::Horizontal,
            0.6,
            LayoutNode::tile("left", ["editor", "outline"]),
            LayoutNode::tile("right", ["terminal"]),
        ))
    }

    #[test]
    fn moving_the_last_panel_collapses_its_source_branch() {
        let mut layout = two_tiles();
        assert!(layout.dock_panel(
            &PanelId::from("terminal"),
            &TileId::from("left"),
            DockZone::Center,
        ));
        assert!(layout.tile(&TileId::from("right")).is_none());
        assert_eq!(
            layout.tile(&TileId::from("left")).unwrap().panels,
            ["editor", "outline", "terminal"]
                .into_iter()
                .map(PanelId::from)
                .collect::<Vec<_>>()
        );
        assert!(layout.valid());
    }

    #[test]
    fn edge_docking_splits_the_target_and_keeps_panel_identity_unique() {
        let mut layout = two_tiles();
        assert!(layout.dock_panel(
            &PanelId::from("outline"),
            &TileId::from("right"),
            DockZone::Bottom,
        ));
        assert_ne!(
            layout.tile_for_panel(&PanelId::from("outline")),
            Some(TileId::from("left"))
        );
        assert!(layout.valid());
    }

    #[test]
    fn split_single_panel_creates_an_explicit_empty_drop_target() {
        let mut layout = PanelLayout::single("editor-group", "editor");
        assert!(layout.split_active(&TileId::from("editor-group"), DockZone::Right));
        assert_eq!(tile_ids(&layout.root).len(), 2);
        assert!(tile_ids(&layout.root)
            .iter()
            .any(|id| layout.tile(id).is_some_and(|tile| tile.panels.is_empty())));
    }

    #[test]
    fn a_new_panel_rejoins_its_named_home_tile() {
        let mut layout = two_tiles();
        layout.reconcile(&[
            PanelPlacement::new("editor", "left"),
            PanelPlacement::new("terminal", "right"),
            PanelPlacement::new("problems", "right"),
        ]);
        assert_eq!(
            layout.tile_for_panel(&PanelId::from("problems")),
            Some(TileId::from("right"))
        );
        assert!(layout.tile_for_panel(&PanelId::from("outline")).is_none());
    }

    #[test]
    fn a_split_home_hint_keeps_late_arrivals_visible() {
        let mut layout = PanelLayout::single("documents", "document-empty");
        layout.reconcile(&[
            PanelPlacement::new("document-a", "documents").with_zone(DockZone::Bottom),
            PanelPlacement::new("document-b", "documents").with_zone(DockZone::Bottom),
        ]);

        let first = layout.tile_for_panel(&PanelId::from("document-a"));
        let second = layout.tile_for_panel(&PanelId::from("document-b"));
        assert_eq!(first, Some(TileId::from("documents")));
        assert_ne!(first, second);
        assert!(matches!(
            layout.root,
            LayoutNode::Split {
                axis: SplitAxis::Vertical,
                ..
            }
        ));
        assert!(layout.valid());
    }

    #[test]
    fn registry_changes_prune_only_the_rows_they_empty() {
        let mut layout = PanelLayout::new(LayoutNode::split(
            "root",
            SplitAxis::Horizontal,
            0.7,
            LayoutNode::tile("primary", ["editor"]),
            LayoutNode::split(
                "document-rows-1",
                SplitAxis::Vertical,
                0.5,
                LayoutNode::tile("documents", ["session-a"]),
                LayoutNode::split(
                    "document-rows-2",
                    SplitAxis::Vertical,
                    0.5,
                    LayoutNode::tile("documents-2", ["session-b"]),
                    LayoutNode::tile("documents-3", ["session-c"]),
                ),
            ),
        ));

        layout.reconcile(&[
            PanelPlacement::new("editor", "primary"),
            PanelPlacement::new("archive:a", "documents").with_zone(DockZone::Bottom),
            PanelPlacement::new("archive:b", "documents").with_zone(DockZone::Bottom),
        ]);

        assert_eq!(layout.tile_count(), 3);
        assert_eq!(
            layout.tile_for_panel(&PanelId::from("archive:a")),
            Some(TileId::from("documents"))
        );
        assert_ne!(
            layout.tile_for_panel(&PanelId::from("archive:a")),
            layout.tile_for_panel(&PanelId::from("archive:b"))
        );
        assert!(tile_ids(&layout.root)
            .iter()
            .all(|id| layout.tile(id).is_some_and(|tile| !tile.panels.is_empty())));
        assert!(layout.valid());
    }

    #[test]
    fn registry_changes_preserve_operator_created_empty_drop_targets() {
        let mut layout = PanelLayout::single("editor-group", "editor");
        let empty = layout
            .split_tile(&TileId::from("editor-group"), DockZone::Right)
            .expect("empty drop target");
        let encoded = layout.encode().expect("serializable layout");
        let mut layout = PanelLayout::decode(&encoded).expect("persisted layout");

        layout.reconcile(&[PanelPlacement::new("editor", "editor-group")]);

        assert!(layout
            .tile(&empty)
            .is_some_and(|tile| tile.panels.is_empty()));
        assert!(layout.valid());
    }

    #[test]
    fn persisted_unmarked_empty_groups_are_repaired_as_registry_artifacts() {
        let mut layout = PanelLayout::new(LayoutNode::split(
            "root",
            SplitAxis::Horizontal,
            0.5,
            LayoutNode::tile("editor-group", ["editor"]),
            LayoutNode::tile("stale-row", std::iter::empty::<PanelId>()),
        ));

        layout.reconcile(&[PanelPlacement::new("editor", "editor-group")]);

        assert_eq!(layout.tile_count(), 1);
        assert!(layout.tile(&TileId::from("stale-row")).is_none());
        assert!(layout.valid());
    }

    #[test]
    fn stale_and_invalid_persisted_layouts_are_rejected() {
        let layout = two_tiles();
        let encoded = layout.encode().unwrap();
        assert_eq!(PanelLayout::decode(&encoded), Some(layout));
        assert!(PanelLayout::decode(&encoded.replace("\"version\":1", "\"version\":2")).is_none());
        assert!(PanelLayout::decode(&encoded.replace("0.6", "1.4")).is_none());
    }

    #[test]
    fn from_homes_builds_one_tile_per_distinct_home_in_registration_order() {
        let layout = PanelLayout::from_homes(&[
            PanelPlacement::new("files", "side"),
            PanelPlacement::new("editor", "main"),
            PanelPlacement::new("readme", "main"),
            PanelPlacement::new("terminal", "bottom"),
        ]);
        assert_eq!(layout.tile_count(), 3);
        assert_eq!(tile_ids(&layout.root)[0], TileId::from("side"));
        assert_eq!(
            layout.tile(&TileId::from("main")).unwrap().panels,
            vec![PanelId::from("editor"), PanelId::from("readme")]
        );
        assert!(layout.valid());
        assert!(layout.encode().is_some());
    }

    #[test]
    fn from_homes_honors_home_zone_hints_and_survives_emptiness() {
        let layout = PanelLayout::from_homes(&[
            PanelPlacement::new("editor", "main"),
            PanelPlacement::new("preview", "main").with_zone(DockZone::Right),
        ]);
        assert_ne!(
            layout.tile_for_panel(&PanelId::from("editor")),
            layout.tile_for_panel(&PanelId::from("preview"))
        );
        assert!(layout.valid());

        let empty = PanelLayout::from_homes(&[]);
        assert_eq!(empty.tile_count(), 1);
        assert!(empty.valid());
    }

    #[test]
    fn decode_errors_name_their_cause() {
        let encoded = two_tiles().encode().unwrap();
        assert_eq!(
            PanelLayout::try_decode(&encoded.replace("\"version\":1", "\"version\":2")),
            Err(LayoutError::Version { found: 2 })
        );
        assert_eq!(
            PanelLayout::try_decode("not a layout"),
            Err(LayoutError::Malformed)
        );
        assert!(matches!(
            PanelLayout::try_decode(&encoded.replace("0.6", "1.4")),
            Err(LayoutError::Invalid(_))
        ));
    }

    #[test]
    fn hand_built_invariant_violations_fail_encoding_with_a_reason() {
        let layout = PanelLayout::new(LayoutNode::Split {
            id: SplitId::from("root"),
            axis: SplitAxis::Horizontal,
            ratio: 0.5,
            first: Box::new(LayoutNode::tile("same", ["editor"])),
            second: Box::new(LayoutNode::tile("same", ["terminal"])),
        });
        let error = layout.try_encode().unwrap_err();
        assert!(matches!(&error, LayoutError::Invalid(reason) if reason.contains("same")));
    }

    #[test]
    fn reconcile_heals_out_of_range_ratios_from_public_construction() {
        let mut layout = PanelLayout::new(LayoutNode::Split {
            id: SplitId::from("root"),
            axis: SplitAxis::Horizontal,
            ratio: 5.0,
            first: Box::new(LayoutNode::tile("left", ["editor"])),
            second: Box::new(LayoutNode::tile("right", ["terminal"])),
        });
        assert!(!layout.valid());
        layout.reconcile(&[
            PanelPlacement::new("editor", "left"),
            PanelPlacement::new("terminal", "right"),
        ]);
        assert_eq!(
            layout.split_ratio(&SplitId::from("root")),
            Some(MAX_SPLIT_RATIO)
        );
        assert!(layout.valid());
    }

    #[test]
    fn keyboard_tile_moves_wrap_and_collapse_sources() {
        let mut layout = two_tiles();
        assert!(layout.move_panel_by_tile(&PanelId::from("editor"), -1));
        assert_eq!(
            layout.tile_for_panel(&PanelId::from("editor")),
            Some(TileId::from("right"))
        );
        assert!(layout.valid());
    }

    #[test]
    fn docking_into_an_edge_of_the_same_multi_tab_tile_splits_it() {
        let mut layout = PanelLayout::new(LayoutNode::tile("only", ["editor", "outline"]));
        assert!(layout.dock_panel(
            &PanelId::from("outline"),
            &TileId::from("only"),
            DockZone::Right,
        ));
        assert_eq!(layout.tile_count(), 2);
        assert_ne!(
            layout.tile_for_panel(&PanelId::from("outline")),
            layout.tile_for_panel(&PanelId::from("editor"))
        );
        assert!(layout.valid());
    }

    #[test]
    fn the_last_panel_cannot_be_docked_out_of_existence() {
        let mut layout = PanelLayout::single("only", "editor");
        assert!(!layout.dock_panel(
            &PanelId::from("editor"),
            &TileId::from("only"),
            DockZone::Right,
        ));
        assert_eq!(layout.tile_count(), 1);
        assert!(layout.valid());
    }
}
