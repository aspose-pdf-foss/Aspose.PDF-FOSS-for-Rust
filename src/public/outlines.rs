//! `Aspose.Pdf.Outlines` — the abstract base of the bookmark tree,
//! per `capabilities/outlines.yaml`, plus the shared tree internals
//! backing [`OutlineCollection`] and
//! [`OutlineItemCollection`].
//!
//! The canonical `Outlines` base is realised as the [`Outlines`]
//! enum over its two concrete forms (the document root collection
//! and an outline item). The tree itself is a shared
//! `Rc<RefCell<…>>` structure with `Weak` parent links, so handles
//! returned by the navigation accessors stay live and mutations are
//! visible through every handle to the same node.

use crate::public::i_appointment::IAppointment;
use crate::public::outline_collection::OutlineCollection;
use crate::public::outline_item_collection::OutlineItemCollection;
use crate::public::pdf_action::PdfAction;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A shared handle to one tree node.
pub(crate) type Link = Rc<RefCell<Node>>;

/// One node of the outline tree. The root collection node has
/// `item == None`; every bookmark item carries `Some(ItemData)`.
pub(crate) struct Node {
    pub(crate) item: Option<ItemData>,
    pub(crate) children: Vec<Link>,
    pub(crate) parent: Option<Weak<RefCell<Node>>>,
    /// Set when the tree is mutated after load. Meaningful only on the
    /// root node; [`mark_dirty`] always walks up to set it there, so
    /// the document's [`Document::save`](crate::Document::save) knows
    /// to (re)write the catalog's `/Outlines`.
    pub(crate) dirty: bool,
}

/// The per-item bookmark payload.
pub(crate) struct ItemData {
    pub(crate) title: String,
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    pub(crate) open: bool,
    pub(crate) destination: Option<Rc<dyn IAppointment>>,
    pub(crate) action: Option<Rc<dyn PdfAction>>,
    /// The `/A` dictionary a loaded bookmark was opened with, when this
    /// model cannot name it. Kept so the save does not strip it — see
    /// `outline_io::OutlineData::carried_action`. Empty for a bookmark
    /// built in memory.
    pub(crate) carried_action: Option<crate::internal::objects::Value>,
}

impl Node {
    /// A fresh root-collection node.
    pub(crate) fn new_root() -> Link {
        Rc::new(RefCell::new(Node {
            item: None,
            children: Vec::new(),
            parent: None,
            dirty: false,
        }))
    }

    /// A fresh detached item node (open by default).
    pub(crate) fn new_item() -> Link {
        Rc::new(RefCell::new(Node {
            item: Some(ItemData {
                title: String::new(),
                bold: false,
                italic: false,
                open: true,
                destination: None,
                action: None,
                carried_action: None,
            }),
            children: Vec::new(),
            parent: None,
            dirty: false,
        }))
    }
}

// ---------------------------------------------------------------------------
// Shared tree operations over a Link. Each keeps borrow scopes tight to
// avoid overlapping RefCell borrows.
// ---------------------------------------------------------------------------

/// Walk up to the tree's root node from `node`.
fn root_of(node: &Link) -> Link {
    let mut cur = Rc::clone(node);
    loop {
        let parent = cur.borrow().parent.as_ref().and_then(|w| w.upgrade());
        match parent {
            Some(p) => cur = p,
            None => return cur,
        }
    }
}

/// Flag the tree (via its root) as mutated since load.
pub(crate) fn mark_dirty(node: &Link) {
    root_of(node).borrow_mut().dirty = true;
}

/// Whether the tree rooted at `root` has been mutated since load.
pub(crate) fn is_dirty(root: &Link) -> bool {
    root.borrow().dirty
}

/// Reset the root's dirty flag (used after building a loaded tree).
pub(crate) fn clear_dirty(root: &Link) {
    root.borrow_mut().dirty = false;
}

/// Append `child` under `parent`, wiring the parent back-link.
pub(crate) fn attach(parent: &Link, child: &Link) {
    child.borrow_mut().parent = Some(Rc::downgrade(parent));
    parent.borrow_mut().children.push(Rc::clone(child));
    mark_dirty(parent);
}

/// Insert `child` under `parent` at a 0-based index (clamped).
pub(crate) fn attach_at(parent: &Link, index: usize, child: &Link) {
    child.borrow_mut().parent = Some(Rc::downgrade(parent));
    {
        let mut p = parent.borrow_mut();
        let at = index.min(p.children.len());
        p.children.insert(at, Rc::clone(child));
    }
    mark_dirty(parent);
}

/// Number of direct children.
pub(crate) fn child_count(node: &Link) -> usize {
    node.borrow().children.len()
}

/// The child at a 0-based index, if present.
pub(crate) fn child_at(node: &Link, index: usize) -> Option<Link> {
    node.borrow().children.get(index).map(Rc::clone)
}

/// Remove every direct child.
pub(crate) fn clear_children(node: &Link) {
    node.borrow_mut().children.clear();
    mark_dirty(node);
}

/// Whether `target` is a direct child of `node`.
pub(crate) fn contains_child(node: &Link, target: &Link) -> bool {
    node.borrow().children.iter().any(|c| Rc::ptr_eq(c, target))
}

/// Remove `target` from `node`'s direct children; returns whether it
/// was present.
pub(crate) fn remove_child(node: &Link, target: &Link) -> bool {
    let before = node.borrow().children.len();
    node.borrow_mut().children.retain(|c| !Rc::ptr_eq(c, target));
    let changed = node.borrow().children.len() != before;
    if changed {
        mark_dirty(node);
    }
    changed
}

/// Remove the direct child at a 0-based index (no-op when out of
/// range).
pub(crate) fn remove_child_at(node: &Link, index: usize) {
    let removed = {
        let mut n = node.borrow_mut();
        if index < n.children.len() {
            n.children.remove(index);
            true
        } else {
            false
        }
    };
    if removed {
        mark_dirty(node);
    }
}

/// Remove the first direct child whose title equals `name`.
pub(crate) fn remove_child_named(node: &Link, name: &str) {
    let pos = node
        .borrow()
        .children
        .iter()
        .position(|c| c.borrow().item.as_ref().map(|i| i.title.as_str()) == Some(name));
    if let Some(i) = pos {
        node.borrow_mut().children.remove(i);
        mark_dirty(node);
    }
}

/// Total visible descendants of `node`: each child counts once, plus
/// (when it is open) its own visible descendants, recursively.
pub(crate) fn visible_descendants(node: &Link) -> i32 {
    let children: Vec<Link> = node.borrow().children.iter().map(Rc::clone).collect();
    let mut total = 0;
    for c in &children {
        total += 1;
        let open = c.borrow().item.as_ref().map(|i| i.open).unwrap_or(true);
        if open {
            total += visible_descendants(c);
        }
    }
    total
}

// ---------------------------------------------------------------------------
// Outlines base — the enum over the two concrete forms.
// ---------------------------------------------------------------------------

/// The base of the outline hierarchy: either the document's root
/// [`OutlineCollection`] or an [`OutlineItemCollection`] node.
///
/// Returned by [`OutlineItemCollection::parent`](crate::OutlineItemCollection::parent),
/// matching the canonical `Outlines` return type.
pub enum Outlines {
    /// The document root outline collection.
    Collection(OutlineCollection),
    /// An outline item (which is itself a collection of children).
    Item(OutlineItemCollection),
}

impl Outlines {
    /// The number of direct children.
    pub fn count(&self) -> usize {
        match self {
            Outlines::Collection(c) => c.count(),
            Outlines::Item(i) => i.count(),
        }
    }

    /// The number of visible descendants when expanded per the open
    /// flags.
    pub fn visible_count(&self) -> i32 {
        match self {
            Outlines::Collection(c) => c.visible_count(),
            Outlines::Item(i) => i.visible_count(),
        }
    }

    /// Append an item under this node.
    pub fn add(&self, item: &OutlineItemCollection) {
        match self {
            Outlines::Collection(c) => c.add(item),
            Outlines::Item(i) => i.add(item),
        }
    }

    /// Remove every child.
    pub fn clear(&self) {
        match self {
            Outlines::Collection(c) => c.clear(),
            Outlines::Item(i) => i.clear(),
        }
    }

    /// Whether `item` is a direct child.
    pub fn contains(&self, item: &OutlineItemCollection) -> bool {
        match self {
            Outlines::Collection(c) => c.contains(item),
            Outlines::Item(i) => i.contains(item),
        }
    }

    /// Remove `item` from the direct children; returns whether it was
    /// present.
    pub fn remove(&self, item: &OutlineItemCollection) -> bool {
        match self {
            Outlines::Collection(c) => c.remove(item),
            Outlines::Item(i) => i.remove(item),
        }
    }
}
