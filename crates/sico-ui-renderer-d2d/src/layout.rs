//! The pure deterministic layout + point→handle hit-testing layer (STEP-0592).
//!
//! This is the missing piece of the `ui@1` `click(ui-handle)` /
//! `select(ui-selection)` contract (RFC-0052 §2, `wit/script-ui-v1/ui.wit:59-63`):
//! the window shell captures a pointer press as client **coordinates**
//! ([`crate::events::WindowEvent::Clicked`]) because the renderer had no layout
//! to map a point to a node. This module is that layout, and it is a **pure
//! function** of the UI tree plus the client size — no OS window, no renderer,
//! no environment value.
//!
//! # Determinism contract
//!
//! Every measurement comes from the frozen token table ([`crate::tokens`]).
//! In particular the layout does **not** use DirectWrite glyph metrics, which
//! are host-dependent (the glyph rasteriser of the freezing machine): text
//! advance is approximated as one `BODY_SIZE` em per UTF-16 code unit, capped
//! at the frozen `TEXT_MAX_WIDTH`. That makes the laid-out rectangles a pure
//! function of the tree + client size and byte-identical on every machine,
//! which is what lets the corpus in `tests/layout_corpus.rs` hash them across
//! processes (mirroring the frame and event corpora).
//!
//! # Divergence from the renderer's internal layout
//!
//! The offscreen renderer (`render/win.rs::place`) uses **degenerate** cells
//! for containers: `natural_size(Root | Column | Row)` is `(0.0, 0.0)`, so a
//! container's own box is 0×0 and a sibling after it would be placed with no
//! room (a latent overlap for two sibling containers that the frozen frame
//! corpus does not exercise). The hit-test layout here instead gives every
//! container its **true content extent** (the union of its children), because
//! "the point is in the container but in no child" must resolve to the
//! container. The root additionally covers the **whole client area** so a
//! click in the window padding resolves to the root, and a click outside the
//! client resolves to [`Hit::Miss`]. This is a deliberate, documented
//! divergence: the renderer's drawing cells are not the hit-testing regions.

// The casts below are behaviour-neutral and bounded (client side ≤ 16 384,
// tree ≤ 4096 nodes, image dimensions well under 2^32): the values are
// exactly representable or the truncation is impossible in range, so the
// pedantic cast lints are noise here. Mirrors `render/win.rs`.
#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::module_name_repetitions
)]

use std::fmt::Write as _;

use sha2::{Digest, Sha256};

use crate::error::UiBackendError;
use crate::tokens;
use crate::tree::{MAX_DEPTH, NodeKind, UiTree};

/// A node rectangle in 96-DPI logical pixels.
///
/// Coordinates are **client** coordinates: the origin is the client-area
/// top-left, exactly the space [`crate::events::WindowEvent::Clicked`]
/// reports.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

impl Rect {
    /// Whether the point is inside this rectangle.
    ///
    /// The rectangle is half-open: `[x, x+width) × [y, y+height)`. The right
    /// and bottom edges are exclusive, so `(width, height)` is **outside** the
    /// `(0, 0, width, height)` rectangle.
    #[must_use]
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    /// The point at the geometric centre of the rectangle.
    #[must_use]
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    fn to_bits_bytes(self) -> [u8; 16] {
        let mut out = [0u8; 16];
        out[0..4].copy_from_slice(&self.x.to_bits().to_le_bytes());
        out[4..8].copy_from_slice(&self.y.to_bits().to_le_bytes());
        out[8..12].copy_from_slice(&self.width.to_bits().to_le_bytes());
        out[12..16].copy_from_slice(&self.height.to_bits().to_le_bytes());
        out
    }
}

/// The result of one point → node hit test.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Hit {
    /// The deepest node whose rectangle contains the point.
    ///
    /// `Node(root)` is the window root: it means the point is inside the
    /// client area but not inside any child node (e.g. the window padding, or
    /// the empty part of a container). This is **not** the "attribute any
    /// click to the root" behaviour that STEP-0585 rejected — here the root is
    /// the deepest node because **no** child rectangle covers the point.
    Node(u64),
    /// The point is outside the window (or no window exists).
    Miss,
}

/// One laid-out node: its rectangle, depth, kind and parent handle.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Entry {
    rect: Rect,
    depth: u32,
    kind: NodeKind,
    parent: u64,
}

/// A computed layout: one rectangle per node, in client coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    /// Node rectangles, indexed by `handle - 1` (handles are `1..=len`).
    entries: Vec<Option<Entry>>,
    root: u64,
    width: u32,
    height: u32,
}

impl Layout {
    /// The window root handle.
    #[must_use]
    pub fn root(&self) -> u64 {
        self.root
    }

    /// The client width the layout was computed for.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The client height the layout was computed for.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The number of nodes the layout covers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the layout covers no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The rectangle of a node, if the handle is live.
    #[must_use]
    pub fn rect(&self, handle: u64) -> Option<Rect> {
        self.entry(handle).map(|entry| entry.rect)
    }

    /// The kind of a node, if the handle is live.
    #[must_use]
    pub fn kind(&self, handle: u64) -> Option<NodeKind> {
        self.entry(handle).map(|entry| entry.kind)
    }

    /// The depth of a node (root is 0), if the handle is live.
    #[must_use]
    pub fn depth(&self, handle: u64) -> Option<u32> {
        self.entry(handle).map(|entry| entry.depth)
    }

    /// The parent handle of a node, if the handle is live (0 for the root).
    #[must_use]
    pub fn parent(&self, handle: u64) -> Option<u64> {
        self.entry(handle).map(|entry| entry.parent)
    }

    /// Hit-tests one client point and returns the deepest node under it.
    ///
    /// A point outside the client area is always [`Hit::Miss`], even when a
    /// degenerate child rectangle (e.g. a huge image preview) extends beyond
    /// the window. A point inside the client area but not inside any child
    /// node resolves to [`Hit::Node`] with the deepest containing node — the
    /// root if the point is in the window padding.
    #[must_use]
    pub fn hit(&self, x: i32, y: i32) -> Hit {
        let px = x as f32;
        let py = y as f32;
        if px < 0.0 || py < 0.0 || px >= self.width as f32 || py >= self.height as f32 {
            return Hit::Miss;
        }
        let mut best: Option<(u32, u64)> = None;
        for (index, entry) in self.entries.iter().enumerate() {
            let Some(entry) = entry else {
                continue;
            };
            if entry.rect.contains(px, py)
                && best.is_none_or(|(best_depth, _)| entry.depth > best_depth)
            {
                best = Some((entry.depth, index as u64 + 1));
            }
        }
        best.map_or(Hit::Miss, |(_, handle)| Hit::Node(handle))
    }

    /// The canonical SHA-256 digest of the laid-out rectangles.
    ///
    /// The serialization is a fixed, versioned wire form (the client size, the
    /// node count, then per node in handle order the handle, the kind
    /// discriminant, the four rectangle `f32` bit patterns and the depth). It
    /// is the identity of a layout: the same tree + client size always hashes
    /// to the same value on every process.
    #[must_use]
    pub fn digest(&self) -> String {
        let mut out = Vec::new();
        out.extend_from_slice(&self.width.to_le_bytes());
        out.extend_from_slice(&self.height.to_le_bytes());
        let mut count = 0u32;
        for entry in &self.entries {
            if entry.is_some() {
                count += 1;
            }
        }
        out.extend_from_slice(&count.to_le_bytes());
        for (index, entry) in self.entries.iter().enumerate() {
            let Some(entry) = entry else {
                continue;
            };
            out.extend_from_slice(&((index as u64 + 1).to_le_bytes()));
            out.push(kind_tag(entry.kind));
            out.extend_from_slice(&entry.rect.to_bits_bytes());
            out.extend_from_slice(&entry.depth.to_le_bytes());
        }
        sha256_hex(&out)
    }

    /// The canonical SHA-256 digest of the hit results for a fixed point list.
    ///
    /// The serialization carries the client size, the point count, then per
    /// point the `i32` coordinates and the hit result (a tag byte plus the
    /// handle). It is the identity of the hit behaviour for those points.
    #[must_use]
    pub fn hit_digest(&self, points: &[(i32, i32)]) -> String {
        let mut out = Vec::new();
        out.extend_from_slice(&self.width.to_le_bytes());
        out.extend_from_slice(&self.height.to_le_bytes());
        out.extend_from_slice(&(points.len() as u32).to_le_bytes());
        for (x, y) in points {
            out.extend_from_slice(&x.to_le_bytes());
            out.extend_from_slice(&y.to_le_bytes());
            match self.hit(*x, *y) {
                Hit::Miss => out.push(0),
                Hit::Node(handle) => {
                    out.push(1);
                    out.extend_from_slice(&handle.to_le_bytes());
                }
            }
        }
        sha256_hex(&out)
    }

    fn entry(&self, handle: u64) -> Option<Entry> {
        if handle == 0 {
            return None;
        }
        let index = usize::try_from(handle - 1).ok()?;
        self.entries.get(index).copied().flatten()
    }
}

/// Computes the layout of a tree at a client size.
///
/// The layout is a pure function of `tree` + `width` + `height`. It is
/// bounded by the tree's declared node-count and depth budgets (the tree
/// enforces them on insert); a malformed tree, an unknown handle or a
/// depth overrun is a typed refusal, never a panic or a truncated result.
///
/// # Errors
///
/// Returns [`UiBackendError::NoWindow`] when the tree has no window root,
/// [`UiBackendError::InvalidArgument`] when `width` or `height` is `0`, and
/// the tree's typed refusals ([`UiBackendError::UnknownHandle`],
/// [`UiBackendError::Limit`]) for a malformed or over-deep tree.
pub fn compute(tree: &UiTree, width: u32, height: u32) -> Result<Layout, UiBackendError> {
    let root = tree.root().ok_or(UiBackendError::NoWindow)?;
    if width == 0 || height == 0 {
        return Err(UiBackendError::InvalidArgument {
            which: "client-size",
        });
    }
    let mut entries = vec![None; tree.len()];
    let w = width as f32;
    let h = height as f32;

    // The window root covers the whole client area.
    set_entry(
        &mut entries,
        root,
        Entry {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: w,
                height: h,
            },
            depth: 0,
            kind: NodeKind::Root,
            parent: 0,
        },
    )?;

    let root_node = tree.node(root).ok_or(UiBackendError::UnknownHandle)?;
    let padding = tokens::geometry::WINDOW_PADDING;
    let content_x = padding;
    let content_y = padding;
    let content_width = (w - padding * 2.0).max(0.0);
    let mut cursor = content_y;
    for child in &root_node.children {
        let (_, child_height) = place(
            tree,
            *child,
            content_x,
            cursor,
            content_width,
            true,
            1,
            &mut entries,
        )?;
        cursor += child_height + tokens::geometry::SPACING;
    }

    Ok(Layout {
        entries,
        root,
        width,
        height,
    })
}

/// Places one node and its subtree, returning the node's content extent.
///
/// `stretch_width` is set for the children of a vertical container (root or
/// column): text and progress nodes take the full available width while
/// controls keep their intrinsic width. Children of a horizontal container
/// never stretch.
#[allow(clippy::too_many_arguments)]
fn place(
    tree: &UiTree,
    handle: u64,
    x: f32,
    y: f32,
    available_width: f32,
    stretch_width: bool,
    depth: u32,
    entries: &mut Vec<Option<Entry>>,
) -> Result<(f32, f32), UiBackendError> {
    if depth as usize > MAX_DEPTH {
        return Err(UiBackendError::Limit {
            which: "tree-depth",
        });
    }
    let node = tree.node(handle).ok_or(UiBackendError::UnknownHandle)?;
    let (natural_width, natural_height) = natural_size(node);
    let stretches = stretch_width && matches!(node.kind, NodeKind::Text | NodeKind::Progress);
    let own_width = if stretches {
        available_width
    } else {
        natural_width
    };
    let own_height = natural_height;

    let (extent_width, extent_height) = match node.kind {
        NodeKind::Root | NodeKind::Column => {
            // A nested Root is impossible (insert rejects it); both are
            // vertical: children stacked from y, all at x.
            let mut extent_width: f32 = 0.0;
            let mut cursor: f32 = y;
            let mut last_bottom: f32 = y;
            for child in &node.children {
                let (child_width, child_height) = place(
                    tree,
                    *child,
                    x,
                    cursor,
                    available_width,
                    true,
                    depth + 1,
                    entries,
                )?;
                cursor += child_height + tokens::geometry::SPACING;
                extent_width = extent_width.max(child_width);
                last_bottom = cursor - tokens::geometry::SPACING;
            }
            (extent_width, last_bottom - y)
        }
        NodeKind::Row => {
            // Horizontal: children side by side, all at y.
            let mut extent_height: f32 = 0.0;
            let mut cursor: f32 = x;
            let mut last_right: f32 = x;
            for child in &node.children {
                let (child_width, child_height) = place(
                    tree,
                    *child,
                    cursor,
                    y,
                    available_width,
                    false,
                    depth + 1,
                    entries,
                )?;
                cursor += child_width + tokens::geometry::SPACING;
                extent_height = extent_height.max(child_height);
                last_right = cursor - tokens::geometry::SPACING;
            }
            (last_right - x, extent_height)
        }
        NodeKind::Text
        | NodeKind::Button
        | NodeKind::Dropdown
        | NodeKind::ImagePreview
        | NodeKind::Progress => (own_width, own_height),
    };

    set_entry(
        entries,
        handle,
        Entry {
            rect: Rect {
                x,
                y,
                width: extent_width,
                height: extent_height,
            },
            depth,
            kind: node.kind,
            parent: node.parent.unwrap_or(0),
        },
    )?;
    Ok((extent_width, extent_height))
}

/// The intrinsic size of a node before it is placed.
///
/// Every measurement is a frozen token; no environment value (DPI, system
/// font metrics, time) is consulted, so the size is cross-machine identical.
fn natural_size(node: &crate::tree::UiNode) -> (f32, f32) {
    match node.kind {
        NodeKind::Root | NodeKind::Column | NodeKind::Row => (0.0, 0.0),
        NodeKind::Text => (
            text_advance(&node.text),
            tokens::typography::BODY_LINE_HEIGHT,
        ),
        NodeKind::Button => (
            (text_advance(&node.text) + tokens::geometry::CONTROL_PADDING_H * 2.0)
                .max(tokens::geometry::MIN_BUTTON_WIDTH),
            tokens::geometry::CONTROL_HEIGHT,
        ),
        NodeKind::Dropdown => (
            (text_advance(&node.text)
                + tokens::geometry::DROPDOWN_CHEVRON_WIDTH
                + tokens::geometry::CONTROL_PADDING_H * 2.0)
                .max(tokens::geometry::MIN_BUTTON_WIDTH),
            tokens::geometry::CONTROL_HEIGHT,
        ),
        NodeKind::ImagePreview => {
            let image = node.image.as_ref();
            (
                image.map_or(0.0, |image| image.width as f32),
                image.map_or(0.0, |image| image.height as f32),
            )
        }
        NodeKind::Progress => (0.0, tokens::geometry::PROGRESS_HEIGHT),
    }
}

/// The deterministic text advance in logical pixels.
///
/// One body-size em per UTF-16 code unit, capped at the frozen maximum
/// rendered width. This is a **token-derived proxy**, not a font measurement:
/// DirectWrite glyph metrics are host-dependent, so the layout substitutes a
/// pure, environment-free estimate that is byte-identical on every machine.
fn text_advance(text: &str) -> f32 {
    let units = text.encode_utf16().count() as f32;
    (units * tokens::typography::BODY_SIZE).min(tokens::geometry::TEXT_MAX_WIDTH)
}

fn set_entry(
    entries: &mut [Option<Entry>],
    handle: u64,
    entry: Entry,
) -> Result<(), UiBackendError> {
    if handle == 0 {
        return Err(UiBackendError::UnknownHandle);
    }
    let index = usize::try_from(handle - 1).map_err(|_| UiBackendError::UnknownHandle)?;
    let slot = entries
        .get_mut(index)
        .ok_or(UiBackendError::UnknownHandle)?;
    *slot = Some(entry);
    Ok(())
}

/// The stable wire discriminant of a node kind (declaration order).
fn kind_tag(kind: NodeKind) -> u8 {
    match kind {
        NodeKind::Root => 0,
        NodeKind::Column => 1,
        NodeKind::Row => 2,
        NodeKind::Text => 3,
        NodeKind::Button => 4,
        NodeKind::Dropdown => 5,
        NodeKind::ImagePreview => 6,
        NodeKind::Progress => 7,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// The frozen layout + hit corpus scenes and their probe points.
///
/// This is the single source of truth shared by `examples/layout_digest.rs`
/// (the regeneration tool) and `tests/layout_corpus.rs` (the frozen
/// assertions), so the two can never drift: the same scenes and the same
/// probe construction feed both.
#[doc(hidden)]
pub mod corpus {
    use super::{Layout, Rect};
    use crate::tree::{NodeKind, UiTree};

    /// One corpus scene: a named tree plus its client size.
    pub struct Scene {
        /// The scene name (the corpus key).
        pub name: &'static str,
        /// Client width in 96-DPI logical pixels.
        pub width: u32,
        /// Client height in 96-DPI logical pixels.
        pub height: u32,
        /// The built tree.
        pub tree: UiTree,
    }

    /// The four corpus scenes, in corpus order.
    ///
    /// The shapes mirror the frozen frame corpus scenes (`tests/frame_corpus.rs`)
    /// so the layout corpus and the frame corpus stay aligned.
    #[must_use]
    pub fn scenes() -> Vec<Scene> {
        vec![empty(), text(), controls(), row()]
    }

    fn empty() -> Scene {
        let mut tree = UiTree::new();
        tree.insert(NodeKind::Root, None).expect("root");
        Scene {
            name: "empty",
            width: 200,
            height: 120,
            tree,
        }
    }

    fn text() -> Scene {
        let mut tree = UiTree::new();
        let root = tree.insert(NodeKind::Root, None).expect("root");
        let column = tree.insert(NodeKind::Column, Some(root)).expect("column");
        tree.insert(NodeKind::Text, Some(column))
            .and_then(|handle| tree.set_text(handle, "Sico"))
            .expect("text");
        tree.insert(NodeKind::Text, Some(column))
            .and_then(|handle| tree.set_text(handle, "Fluent-subset renderer"))
            .expect("text");
        Scene {
            name: "text",
            width: 320,
            height: 160,
            tree,
        }
    }

    fn controls() -> Scene {
        let mut tree = UiTree::new();
        let root = tree.insert(NodeKind::Root, None).expect("root");
        let column = tree.insert(NodeKind::Column, Some(root)).expect("column");
        tree.insert(NodeKind::Text, Some(column))
            .and_then(|handle| tree.set_text(handle, "Convert"))
            .expect("text");
        tree.insert(NodeKind::Button, Some(column))
            .and_then(|handle| tree.set_text(handle, "Start"))
            .expect("button");
        tree.insert(NodeKind::Dropdown, Some(column))
            .and_then(|handle| tree.set_options(handle, &["PNG".to_owned(), "JPEG".to_owned()]))
            .expect("dropdown");
        tree.insert(NodeKind::Progress, Some(column))
            .and_then(|handle| tree.set_progress(handle, 3, 8))
            .expect("progress");
        tree.insert(NodeKind::ImagePreview, Some(column))
            .and_then(|handle| tree.set_image(handle, 32, 24, Vec::new()))
            .expect("image");
        Scene {
            name: "controls",
            width: 360,
            height: 280,
            tree,
        }
    }

    fn row() -> Scene {
        let mut tree = UiTree::new();
        let root = tree.insert(NodeKind::Root, None).expect("root");
        let row = tree.insert(NodeKind::Row, Some(root)).expect("row");
        tree.insert(NodeKind::Button, Some(row))
            .and_then(|handle| tree.set_text(handle, "OK"))
            .expect("button");
        tree.insert(NodeKind::Button, Some(row))
            .and_then(|handle| tree.set_text(handle, "Cancel"))
            .expect("button");
        Scene {
            name: "row",
            width: 320,
            height: 120,
            tree,
        }
    }

    /// The probe points for a scene layout.
    ///
    /// The list is deterministic (handle order) and covers the corner cases:
    /// the four window corners (in and out), every node's centre, and the
    /// spacing gap between consecutive siblings of every container (which must
    /// resolve to the container, not to a child).
    #[must_use]
    pub fn probes(layout: &Layout) -> Vec<(i32, i32)> {
        let mut points = vec![
            (-1, -1),
            (0, 0),
            (layout.width() as i32 - 1, layout.height() as i32 - 1),
            (layout.width() as i32, layout.height() as i32),
        ];
        for handle in 1..=layout.len() as u64 {
            let Some(rect) = layout.rect(handle) else {
                continue;
            };
            let (cx, cy) = rect.center();
            points.push((cx as i32, cy as i32));
        }
        for handle in 1..=layout.len() as u64 {
            let Some(kind) = layout.kind(handle) else {
                continue;
            };
            if !matches!(kind, NodeKind::Root | NodeKind::Column | NodeKind::Row) {
                continue;
            }
            let children: Vec<u64> = (1..=layout.len() as u64)
                .filter(|child| layout.parent(*child) == Some(handle))
                .collect();
            for pair in children.windows(2) {
                let Some(a) = layout.rect(pair[0]) else {
                    continue;
                };
                let Some(b) = layout.rect(pair[1]) else {
                    continue;
                };
                if kind == NodeKind::Row {
                    let x = (a.x + a.width + b.x) / 2.0;
                    let y = a.y + a.height / 2.0;
                    points.push((x as i32, y as i32));
                } else {
                    let x = a.x + a.width / 2.0;
                    let y = (a.y + a.height + b.y) / 2.0;
                    points.push((x as i32, y as i32));
                }
            }
        }
        points
    }

    /// The centre of a node as an integer probe point.
    #[must_use]
    pub fn center_of(rect: Rect) -> (i32, i32) {
        let (x, y) = rect.center();
        (x as i32, y as i32)
    }
}

#[cfg(test)]
mod tests {
    // The f32 assertions compare exact integer-valued logical pixels (56.0 =
    // 4 × 14.0, 4096.0 = TEXT_MAX_WIDTH, integral sums), so the strict float
    // comparison is exact and behaviour-neutral.
    #![allow(clippy::float_cmp)]

    use super::*;
    use crate::tree::{NodeKind, UiTree};

    fn small_tree() -> (UiTree, u64, u64, u64, u64, u64) {
        // window(320x160) > column > text "Sico", button "OK", image 32x24
        let mut tree = UiTree::new();
        let root = tree.insert(NodeKind::Root, None).unwrap();
        let column = tree.insert(NodeKind::Column, Some(root)).unwrap();
        let text = tree.insert(NodeKind::Text, Some(column)).unwrap();
        tree.set_text(text, "Sico").unwrap();
        let button = tree.insert(NodeKind::Button, Some(column)).unwrap();
        tree.set_text(button, "OK").unwrap();
        let image = tree.insert(NodeKind::ImagePreview, Some(column)).unwrap();
        tree.set_image(image, 32, 24, Vec::new()).unwrap();
        (tree, root, column, text, button, image)
    }

    #[test]
    fn the_same_tree_lays_out_the_same_way_twice() {
        let (tree, ..) = small_tree();
        let first = compute(&tree, 320, 160).unwrap();
        let second = compute(&tree, 320, 160).unwrap();
        assert_eq!(first.digest(), second.digest());
    }

    #[test]
    fn the_root_covers_the_whole_client_area() {
        let (tree, root, ..) = small_tree();
        let layout = compute(&tree, 320, 160).unwrap();
        let rect = layout.rect(root).unwrap();
        assert_eq!(rect.x, 0.0);
        assert_eq!(rect.y, 0.0);
        assert_eq!(rect.width, 320.0);
        assert_eq!(rect.height, 160.0);
    }

    #[test]
    fn the_window_padding_resolves_to_the_root() {
        let (tree, root, ..) = small_tree();
        let layout = compute(&tree, 320, 160).unwrap();
        // (8, 8) is inside the 16px window padding, in no child.
        assert_eq!(layout.hit(8, 8), Hit::Node(root));
    }

    #[test]
    fn a_point_on_a_leaf_resolves_to_the_leaf() {
        let (tree, _, _, _, button, _) = small_tree();
        let layout = compute(&tree, 320, 160).unwrap();
        let rect = layout.rect(button).unwrap();
        let (x, y) = rect.center();
        assert_eq!(layout.hit(x as i32, y as i32), Hit::Node(button));
    }

    #[test]
    fn a_point_outside_the_window_is_an_explicit_miss() {
        let (tree, ..) = small_tree();
        let layout = compute(&tree, 320, 160).unwrap();
        assert_eq!(layout.hit(-1, -1), Hit::Miss);
        assert_eq!(
            layout.hit(320, 160),
            Hit::Miss,
            "bottom-right edge is outside"
        );
        assert_eq!(layout.hit(400, 400), Hit::Miss);
        assert_eq!(layout.hit(10, -5), Hit::Miss);
    }

    #[test]
    fn the_gap_between_siblings_resolves_to_the_container() {
        let (tree, _, column, text, button, _) = small_tree();
        let layout = compute(&tree, 320, 160).unwrap();
        let text_rect = layout.rect(text).unwrap();
        let button_rect = layout.rect(button).unwrap();
        // The midpoint of the 8px gap below the text run.
        let gap_x = text_rect.center().0 as i32;
        let gap_y = (text_rect.y + text_rect.height + button_rect.y) / 2.0;
        assert_eq!(layout.hit(gap_x, gap_y as i32), Hit::Node(column));
    }

    #[test]
    fn the_deepest_node_wins() {
        let (tree, _, _, _, button, _) = small_tree();
        let layout = compute(&tree, 320, 160).unwrap();
        // The button is deeper than its column; a point on the button must
        // resolve to the button, not the column.
        let rect = layout.rect(button).unwrap();
        let (x, y) = rect.center();
        let hit = layout.hit(x as i32, y as i32);
        assert!(matches!(hit, Hit::Node(handle) if handle == button));
        assert!(
            layout.depth(button).unwrap() > layout.depth(layout.parent(button).unwrap()).unwrap()
        );
    }

    #[test]
    fn a_tree_without_a_root_is_refused() {
        assert_eq!(
            compute(&UiTree::new(), 200, 120),
            Err(UiBackendError::NoWindow)
        );
    }

    #[test]
    fn a_zero_client_size_is_refused() {
        let (tree, ..) = small_tree();
        assert_eq!(
            compute(&tree, 0, 120),
            Err(UiBackendError::InvalidArgument {
                which: "client-size"
            })
        );
        assert_eq!(
            compute(&tree, 200, 0),
            Err(UiBackendError::InvalidArgument {
                which: "client-size"
            })
        );
    }

    #[test]
    fn text_advance_is_token_derived_and_bounded() {
        // "Sico" is 4 UTF-16 code units => 4 * 14 = 56.
        assert_eq!(text_advance("Sico"), 56.0);
        // Empty text has no width.
        assert_eq!(text_advance(""), 0.0);
        // A very long run is capped at the frozen maximum rendered width.
        let long = "a".repeat(10_000);
        assert_eq!(text_advance(&long), tokens::geometry::TEXT_MAX_WIDTH);
    }

    #[test]
    fn a_row_places_children_side_by_side() {
        let mut tree = UiTree::new();
        let root = tree.insert(NodeKind::Root, None).unwrap();
        let row = tree.insert(NodeKind::Row, Some(root)).unwrap();
        let a = tree.insert(NodeKind::Button, Some(row)).unwrap();
        tree.set_text(a, "OK").unwrap();
        let b = tree.insert(NodeKind::Button, Some(row)).unwrap();
        tree.set_text(b, "Cancel").unwrap();
        let layout = compute(&tree, 320, 120).unwrap();
        let a_rect = layout.rect(a).unwrap();
        let b_rect = layout.rect(b).unwrap();
        // Side by side: b starts after a's right edge plus the spacing.
        assert_eq!(
            b_rect.x,
            a_rect.x + a_rect.width + tokens::geometry::SPACING
        );
        assert_eq!(a_rect.y, b_rect.y);
    }
}
