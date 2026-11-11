//! `Aspose.Pdf.OutlineItemCollection` — a bookmark item, which is
//! itself a collection of child bookmarks, per
//! `capabilities/outline_item_collection.yaml`.
//!
//! Carries the item's title / style / open flag / destination /
//! action, the child-collection operations, and tree navigation
//! (first / last / next / prev / parent / level). The `Color`
//! property (`System.Drawing.Color`) and the .NET collection
//! infrastructure are dropped (see `targets/rust/drops.yaml`).

use crate::public::i_appointment::IAppointment;
use crate::public::outline_collection::OutlineCollection;
use crate::public::outlines::{self, Link, Node, Outlines};
use crate::public::pdf_action::PdfAction;
use std::rc::Rc;

/// A bookmark item and the collection of its child bookmarks.
#[derive(Clone)]
pub struct OutlineItemCollection(Link);

impl OutlineItemCollection {
    /// Construct a fresh (detached) outline item against the
    /// document's root collection. The item is added to the tree by
    /// [`OutlineCollection::add`] or [`OutlineItemCollection::add`].
    pub fn new(_root: &OutlineCollection) -> Self {
        OutlineItemCollection(Node::new_item())
    }

    pub(crate) fn from_link(link: Link) -> Self {
        OutlineItemCollection(link)
    }

    /// The `/A` dictionary this bookmark was opened with, when the
    /// model cannot name it. Crate-internal: the save path writes it
    /// back verbatim so a tree rewrite does not strip it.
    pub(crate) fn carried_action(&self) -> Option<crate::internal::objects::Value> {
        self.0.borrow().item.as_ref().expect("outline item").carried_action.clone()
    }

    /// Record the `/A` dictionary a loaded bookmark carried.
    pub(crate) fn set_carried_action(&self, value: crate::internal::objects::Value) {
        self.0.borrow_mut().item.as_mut().expect("outline item").carried_action = Some(value);
    }

    pub(crate) fn link(&self) -> &Link {
        &self.0
    }

    /// Whether two handles refer to the same item.
    pub fn ptr_eq(&self, other: &OutlineItemCollection) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    // -- item payload -------------------------------------------------

    /// The item title.
    pub fn title(&self) -> String {
        self.0.borrow().item.as_ref().expect("outline item").title.clone()
    }
    /// Set the item title.
    pub fn set_title(&self, value: String) {
        self.0.borrow_mut().item.as_mut().expect("outline item").title = value;
        outlines::mark_dirty(&self.0);
    }

    /// Whether the title is bold.
    pub fn bold(&self) -> bool {
        self.0.borrow().item.as_ref().expect("outline item").bold
    }
    /// Set the bold flag.
    pub fn set_bold(&self, value: bool) {
        self.0.borrow_mut().item.as_mut().expect("outline item").bold = value;
        outlines::mark_dirty(&self.0);
    }

    /// Whether the title is italic.
    pub fn italic(&self) -> bool {
        self.0.borrow().item.as_ref().expect("outline item").italic
    }
    /// Set the italic flag.
    pub fn set_italic(&self, value: bool) {
        self.0.borrow_mut().item.as_mut().expect("outline item").italic = value;
        outlines::mark_dirty(&self.0);
    }

    /// Whether the item is expanded (its children are visible).
    pub fn open(&self) -> bool {
        self.0.borrow().item.as_ref().expect("outline item").open
    }
    /// Set the open (expanded) flag.
    pub fn set_open(&self, value: bool) {
        self.0.borrow_mut().item.as_mut().expect("outline item").open = value;
        outlines::mark_dirty(&self.0);
    }

    /// The target destination, if any.
    pub fn destination(&self) -> Option<Rc<dyn IAppointment>> {
        self.0.borrow().item.as_ref().expect("outline item").destination.clone()
    }
    /// Set the target destination.
    pub fn set_destination(&self, destination: Box<dyn IAppointment>) {
        self.0.borrow_mut().item.as_mut().expect("outline item").destination = Some(Rc::from(destination));
        outlines::mark_dirty(&self.0);
    }

    /// The action triggered by the item, if any.
    pub fn action(&self) -> Option<Rc<dyn PdfAction>> {
        self.0.borrow().item.as_ref().expect("outline item").action.clone()
    }
    /// Set the action triggered by the item.
    pub fn set_action(&self, action: Box<dyn PdfAction>) {
        self.0.borrow_mut().item.as_mut().expect("outline item").action = Some(Rc::from(action));
        outlines::mark_dirty(&self.0);
    }

    // -- child collection --------------------------------------------

    /// Append a child item.
    pub fn add(&self, item: &OutlineItemCollection) {
        outlines::attach(&self.0, item.link());
    }

    /// Insert a child item at a 1-based index.
    pub fn insert(&self, index: i32, item: &OutlineItemCollection) {
        let i = if index >= 1 { (index - 1) as usize } else { 0 };
        outlines::attach_at(&self.0, i, item.link());
    }

    /// Remove every child item.
    pub fn clear(&self) {
        outlines::clear_children(&self.0);
    }

    /// Remove this item from its parent.
    pub fn delete(&self) {
        let parent = self.0.borrow().parent.as_ref().and_then(|w| w.upgrade());
        if let Some(p) = parent {
            outlines::remove_child(&p, &self.0);
        }
    }

    /// Remove the first child whose title equals `name`.
    pub fn delete_by_name(&self, name: &str) {
        outlines::remove_child_named(&self.0, name);
    }

    /// Whether `item` is a direct child.
    pub fn contains(&self, item: &OutlineItemCollection) -> bool {
        outlines::contains_child(&self.0, item.link())
    }

    /// Remove `item` from the direct children; returns whether it was
    /// present.
    pub fn remove(&self, item: &OutlineItemCollection) -> bool {
        outlines::remove_child(&self.0, item.link())
    }

    /// Remove the child at a 1-based index (no-op when out of range).
    pub fn remove_at(&self, index: i32) {
        if index >= 1 {
            outlines::remove_child_at(&self.0, (index - 1) as usize);
        }
    }

    /// The number of direct children.
    pub fn count(&self) -> usize {
        outlines::child_count(&self.0)
    }

    /// The number of visible descendants when expanded per the open
    /// flags.
    pub fn visible_count(&self) -> i32 {
        outlines::visible_descendants(&self.0)
    }

    /// The child at a 1-based index.
    ///
    /// # Panics
    /// Panics when `index` is out of range, matching the canonical
    /// indexer's bounds throw.
    pub fn get(&self, index: i32) -> OutlineItemCollection {
        assert!(index >= 1, "outline index is 1-based");
        outlines::child_at(&self.0, (index - 1) as usize)
            .map(OutlineItemCollection::from_link)
            .expect("outline index out of range")
    }

    // -- navigation ---------------------------------------------------

    /// The first child, if any.
    pub fn first(&self) -> Option<OutlineItemCollection> {
        outlines::child_at(&self.0, 0).map(OutlineItemCollection::from_link)
    }

    /// The last child, if any.
    pub fn last(&self) -> Option<OutlineItemCollection> {
        let n = self.count();
        if n == 0 {
            None
        } else {
            outlines::child_at(&self.0, n - 1).map(OutlineItemCollection::from_link)
        }
    }

    /// The next sibling, if any.
    pub fn next(&self) -> Option<OutlineItemCollection> {
        self.sibling(1)
    }

    /// The previous sibling, if any.
    pub fn prev(&self) -> Option<OutlineItemCollection> {
        self.sibling(-1)
    }

    /// Whether a next sibling exists.
    pub fn has_next(&self) -> bool {
        self.next().is_some()
    }

    fn sibling(&self, delta: i32) -> Option<OutlineItemCollection> {
        let parent = self.0.borrow().parent.as_ref().and_then(|w| w.upgrade())?;
        let idx = parent
            .borrow()
            .children
            .iter()
            .position(|c| Rc::ptr_eq(c, &self.0))?;
        let target = idx as i32 + delta;
        if target < 0 {
            return None;
        }
        outlines::child_at(&parent, target as usize).map(OutlineItemCollection::from_link)
    }

    /// This item's parent — either the root collection or the parent
    /// item.
    pub fn parent(&self) -> Option<Outlines> {
        let parent = self.0.borrow().parent.as_ref().and_then(|w| w.upgrade())?;
        let is_item = parent.borrow().item.is_some();
        if is_item {
            Some(Outlines::Item(OutlineItemCollection::from_link(parent)))
        } else {
            Some(Outlines::Collection(OutlineCollection::from_link(parent)))
        }
    }

    /// The 1-based nesting depth (a top-level item is level 1).
    pub fn level(&self) -> i32 {
        let mut lvl = 1;
        let mut cur = Rc::clone(&self.0);
        loop {
            let next = cur.borrow().parent.as_ref().and_then(|w| w.upgrade());
            match next {
                Some(p) => {
                    if p.borrow().item.is_some() {
                        lvl += 1;
                    }
                    cur = p;
                }
                None => break,
            }
        }
        lvl
    }
}
