//! The UI node model.
//!
//! The tree is the renderer's only input: a frame is a pure function of the
//! tree plus the frozen token table (ADR-0018 §3). Every mutation is checked
//! against the declared budgets before it allocates, so a limit overrun is a
//! typed refusal rather than a truncated or half-built tree.

use crate::error::UiBackendError;

/// Maximum number of live nodes in one tree.
pub const MAX_NODES: usize = 4096;
/// Maximum depth of the node tree.
pub const MAX_DEPTH: usize = 64;
/// Maximum UTF-8 length of a single text run.
pub const MAX_TEXT_BYTES: usize = 64 * 1024;
/// Maximum number of options in one dropdown.
pub const MAX_OPTIONS: usize = 1024;
/// Maximum size of one image payload in bytes.
pub const MAX_IMAGE_BYTES: usize = 64 * 1024 * 1024;

/// The kind of a UI node.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    /// The window root. Exactly one per tree.
    Root,
    /// A vertical layout container.
    Column,
    /// A horizontal layout container.
    Row,
    /// A text run.
    Text,
    /// A push button.
    Button,
    /// A dropdown with a fixed option list.
    Dropdown,
    /// A fixed-size image preview.
    ImagePreview,
    /// A progress bar.
    Progress,
}

impl NodeKind {
    /// Whether a node of this kind may hold children.
    #[must_use]
    pub fn accepts_children(self) -> bool {
        matches!(self, NodeKind::Root | NodeKind::Column | NodeKind::Row)
    }

    /// Whether this kind accepts children at all.
    #[must_use]
    pub fn is_container(self) -> bool {
        self.accepts_children()
    }
}

/// A decoded image payload (BGRA8, row-major, no padding).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageData {
    /// Width in pixels.
    pub width: u64,
    /// Height in pixels.
    pub height: u64,
    /// BGRA8 pixel bytes.
    pub pixels: Vec<u8>,
}

/// One node of the UI tree.
#[derive(Clone, Debug)]
pub struct UiNode {
    /// The node kind.
    pub kind: NodeKind,
    /// The parent handle, absent for the root.
    pub parent: Option<u64>,
    /// Child handles, in insertion order.
    pub children: Vec<u64>,
    /// The text content of a text, button or dropdown node.
    pub text: String,
    /// The option list of a dropdown node.
    pub options: Vec<String>,
    /// The image payload of an image-preview node.
    pub image: Option<ImageData>,
    /// The `(done, of)` position of a progress node.
    pub progress: Option<(u64, u64)>,
}

impl UiNode {
    fn new(kind: NodeKind, parent: Option<u64>) -> Self {
        Self {
            kind,
            parent,
            children: Vec::new(),
            text: String::new(),
            options: Vec::new(),
            image: None,
            progress: None,
        }
    }
}

/// The UI tree: an ordered forest with exactly one root.
#[derive(Clone, Debug, Default)]
pub struct UiTree {
    nodes: Vec<UiNode>,
    root: Option<u64>,
}

impl UiTree {
    /// An empty tree.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The root handle, if a window root exists.
    #[must_use]
    pub fn root(&self) -> Option<u64> {
        self.root
    }

    /// The number of live nodes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the tree holds no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Borrows a node by handle.
    #[must_use]
    pub fn node(&self, handle: u64) -> Option<&UiNode> {
        self.nodes.get(Self::index(handle)?)
    }

    /// Iterates over every node in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (u64, &UiNode)> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (index as u64 + 1, node))
    }

    /// Inserts a node under `parent`.
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::Limit`] when the node count or the tree
    /// depth budget would be exceeded, [`UiBackendError::UnknownHandle`] for
    /// an unknown parent and [`UiBackendError::WrongParentKind`] when the
    /// parent cannot accept children.
    pub fn insert(&mut self, kind: NodeKind, parent: Option<u64>) -> Result<u64, UiBackendError> {
        if self.nodes.len() >= MAX_NODES {
            return Err(UiBackendError::Limit {
                which: "node-count",
            });
        }
        match parent {
            None => {
                if self.root.is_some() {
                    return Err(UiBackendError::WrongParentKind);
                }
            }
            Some(parent_handle) => {
                let parent_node = self
                    .node(parent_handle)
                    .ok_or(UiBackendError::UnknownHandle)?;
                if !parent_node.kind.accepts_children() {
                    return Err(UiBackendError::WrongParentKind);
                }
            }
        }
        if kind == NodeKind::Root && parent.is_some() {
            return Err(UiBackendError::WrongParentKind);
        }
        if kind != NodeKind::Root && parent.is_none() {
            return Err(UiBackendError::WrongParentKind);
        }
        if let Some(parent_handle) = parent
            && self.depth_of(parent_handle)? + 1 > MAX_DEPTH
        {
            return Err(UiBackendError::Limit {
                which: "tree-depth",
            });
        }
        self.nodes.push(UiNode::new(kind, parent));
        let handle = self.nodes.len() as u64;
        if kind == NodeKind::Root {
            self.root = Some(handle);
        }
        if let Some(parent_handle) = parent
            && let Some(parent_node) = self.nodes.get_mut(Self::index(parent_handle).unwrap_or(0))
        {
            parent_node.children.push(handle);
        }
        Ok(handle)
    }

    /// Replaces the text content of a node.
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::UnknownHandle`] for an unknown handle and
    /// [`UiBackendError::Limit`] when the text exceeds the declared budget.
    pub fn set_text(&mut self, handle: u64, content: &str) -> Result<(), UiBackendError> {
        if content.len() > MAX_TEXT_BYTES {
            return Err(UiBackendError::Limit {
                which: "text-bytes",
            });
        }
        let node = self
            .nodes
            .get_mut(Self::index(handle).ok_or(UiBackendError::UnknownHandle)?)
            .ok_or(UiBackendError::UnknownHandle)?;
        content.clone_into(&mut node.text);
        Ok(())
    }

    /// Sets the option list of a dropdown node.
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::UnknownHandle`] for an unknown handle,
    /// [`UiBackendError::WrongParentKind`] when the node is not a dropdown
    /// and [`UiBackendError::Limit`] when the option budget is exceeded.
    pub fn set_options(&mut self, handle: u64, options: &[String]) -> Result<(), UiBackendError> {
        if options.len() > MAX_OPTIONS {
            return Err(UiBackendError::Limit {
                which: "option-count",
            });
        }
        let total: usize = options.iter().map(String::len).sum();
        if total > MAX_TEXT_BYTES {
            return Err(UiBackendError::Limit {
                which: "text-bytes",
            });
        }
        let node = self
            .nodes
            .get_mut(Self::index(handle).ok_or(UiBackendError::UnknownHandle)?)
            .ok_or(UiBackendError::UnknownHandle)?;
        if node.kind != NodeKind::Dropdown {
            return Err(UiBackendError::WrongParentKind);
        }
        node.options = options.to_vec();
        Ok(())
    }

    /// Sets the `(done, of)` position of a progress node.
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::UnknownHandle`] for an unknown handle and
    /// [`UiBackendError::WrongParentKind`] when the node is not a progress
    /// bar.
    pub fn set_progress(&mut self, handle: u64, done: u64, of: u64) -> Result<(), UiBackendError> {
        let node = self
            .nodes
            .get_mut(Self::index(handle).ok_or(UiBackendError::UnknownHandle)?)
            .ok_or(UiBackendError::UnknownHandle)?;
        if node.kind != NodeKind::Progress {
            return Err(UiBackendError::WrongParentKind);
        }
        node.progress = Some((done, of));
        Ok(())
    }

    /// Attaches an image payload to an image-preview node.
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::UnknownHandle`] for an unknown handle,
    /// [`UiBackendError::WrongParentKind`] when the node is not an image
    /// preview and [`UiBackendError::Limit`] when the payload exceeds the
    /// declared size budget or does not match the declared dimensions.
    pub fn set_image(
        &mut self,
        handle: u64,
        width: u64,
        height: u64,
        pixels: Vec<u8>,
    ) -> Result<(), UiBackendError> {
        if pixels.len() > MAX_IMAGE_BYTES {
            return Err(UiBackendError::Limit {
                which: "image-bytes",
            });
        }
        let expected = width
            .checked_mul(height)
            .and_then(|area| area.checked_mul(4))
            .ok_or(UiBackendError::Limit {
                which: "image-dimensions",
            })?;
        // An empty payload declares the preview size without any content yet;
        // a non-empty payload must match the declared dimensions exactly.
        if !pixels.is_empty() && expected != pixels.len() as u64 {
            return Err(UiBackendError::InvalidArgument {
                which: "image-payload",
            });
        }
        let node = self
            .nodes
            .get_mut(Self::index(handle).ok_or(UiBackendError::UnknownHandle)?)
            .ok_or(UiBackendError::UnknownHandle)?;
        if node.kind != NodeKind::ImagePreview {
            return Err(UiBackendError::WrongParentKind);
        }
        node.image = Some(ImageData {
            width,
            height,
            pixels,
        });
        Ok(())
    }

    /// The depth of a node, with the root at depth 0.
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::UnknownHandle`] for an unknown handle.
    pub fn depth_of(&self, handle: u64) -> Result<usize, UiBackendError> {
        let mut depth = 0usize;
        let mut current = Some(handle);
        while let Some(node_handle) = current {
            let node = self
                .node(node_handle)
                .ok_or(UiBackendError::UnknownHandle)?;
            current = node.parent;
            depth += 1;
            if depth > MAX_DEPTH + 1 {
                return Err(UiBackendError::Limit {
                    which: "tree-depth",
                });
            }
        }
        Ok(depth.saturating_sub(1))
    }

    fn index(handle: u64) -> Option<usize> {
        if handle == 0 {
            return None;
        }
        let index = usize::try_from(handle - 1).ok()?;
        Some(index)
    }
}
