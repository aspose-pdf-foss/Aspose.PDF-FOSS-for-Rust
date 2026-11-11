//! `Aspose.Pdf.PageCollection` — the pages of a
//! [`Document`](crate::Document), with mutation.
//!
//! v1 surface was read-only (page count + 1-based indexer). This adds the
//! canonical mutators — [`PageCollection::add`] (blank or copy),
//! [`PageCollection::insert`], and the three [`PageCollection::delete`]
//! overloads — backed by page-tree save-through: a mutated collection is
//! flagged dirty and the document's save path rewrites the `/Pages`
//! `/Kids` + `/Count` (and emits the new / copied leaf objects) as an
//! incremental update.
//!
//! Interior mutability — the page list lives behind an `Rc<RefCell<…>>`,
//! so `doc.pages()` keeps returning a borrowed `&PageCollection` while the
//! mutators take `&self`. `get` and `iter` hand back **owned** [`Page`]
//! handles; a `Page` is a cheap Rc-sharing clone, so this is not a deep
//! copy. The canonical `ICollection<Page>` infrastructure (`Clear` aliases
//! `Delete()`, `IndexOf` / `CopyTo` / `GetEnumerator` / `Accept` / …) is
//! dropped.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use super::error::Error;
use super::page::Page;

/// The pages of a [`Document`](crate::Document).
///
/// Indexing is 1-based to match the canonical `Aspose.Pdf.Page` `Number`
/// invariant — the first page is `pages.get(1)?`.
#[derive(Debug, Clone)]
pub struct PageCollection {
    pages: Rc<RefCell<Vec<Page>>>,
    /// The owning document's source bytes and content-edit buffer, stamped
    /// into every page synthesised here — a page created in-session is
    /// wired to the same document as a loaded one.
    doc_bytes: Arc<Vec<u8>>,
    edits: crate::public::page_edits::EditBuffer,
    /// Set by any mutator. When set, the document's save path rewrites the
    /// `/Pages` tree (`/Kids` + `/Count`) and emits the added / copied leaf
    /// objects as an incremental update. An untouched collection resaves
    /// the page tree verbatim.
    dirty: Rc<Cell<bool>>,
}

impl PartialEq for PageCollection {
    fn eq(&self, other: &Self) -> bool {
        *self.pages.borrow() == *other.pages.borrow()
    }
}

impl PageCollection {
    pub(crate) fn new(
        pages: Vec<Page>,
        doc_bytes: Arc<Vec<u8>>,
        edits: crate::public::page_edits::EditBuffer,
    ) -> Self {
        Self {
            pages: Rc::new(RefCell::new(pages)),
            doc_bytes,
            edits,
            dirty: Rc::new(Cell::new(false)),
        }
    }

    /// Number of pages currently in the document. Zero for an
    /// empty document.
    pub fn count(&self) -> usize {
        self.pages.borrow().len()
    }

    /// Look up a page by its 1-based index. Returns
    /// [`Error::PageIndexOutOfRange`] when `index < 1` or
    /// `index > count()`.
    ///
    /// The returned [`Page`] is an owned handle — a cheap Rc-sharing clone
    /// of the stored page, so mutations through it (annotations, images,
    /// geometry, …) still reach the document on save.
    pub fn get(&self, index: usize) -> Result<Page, Error> {
        let pages = self.pages.borrow();
        if index == 0 || index > pages.len() {
            return Err(Error::PageIndexOutOfRange {
                index,
                count: pages.len(),
            });
        }
        Ok(pages[index - 1].clone())
    }

    /// Owned iterator over the document's pages in order. Each yielded
    /// [`Page`] is a cheap Rc-sharing clone.
    pub fn iter(&self) -> std::vec::IntoIter<Page> {
        self.pages.borrow().clone().into_iter()
    }

    /// Append a new blank page (612×792, US Letter) and return a handle to
    /// it. Mirrors the canonical `PageCollection.Add()`.
    pub fn add(&self) -> Page {
        let page = self.synthesize_blank();
        self.pages.borrow_mut().push(page);
        self.renumber();
        self.dirty.set(true);
        // Hand back a handle taken AFTER renumbering: the returned page
        // carries its real Number, which is what the annotation / field
        // constructors bind to.
        self.pages.borrow().last().cloned().expect("just pushed")
    }

    /// Append a copy of `source` and return a handle to it. The copy
    /// shares `source`'s content and resources (it renders the same text /
    /// graphics). Mirrors the canonical `PageCollection.Add(Page)`.
    pub fn add_page(&self, source: &Page) -> Page {
        let page = self.synthesize_copy(source);
        self.pages.borrow_mut().push(page);
        self.renumber();
        self.dirty.set(true);
        self.pages.borrow().last().cloned().expect("just pushed")
    }

    /// Insert a new blank page at the 1-based `page_number` and return a
    /// handle to it. `page_number` is clamped into `[1, count()+1]`.
    /// Mirrors the canonical `PageCollection.Insert(int)`.
    pub fn insert(&self, page_number: usize) -> Page {
        let page = self.synthesize_blank();
        let idx = {
            let mut pages = self.pages.borrow_mut();
            let len = pages.len();
            let idx = page_number.clamp(1, len + 1) - 1;
            pages.insert(idx, page);
            idx
        };
        self.renumber();
        self.dirty.set(true);
        self.pages.borrow()[idx].clone()
    }

    /// Delete the page at the 1-based `index`. Out-of-range indices are
    /// ignored (the canonical indexer throws; the FOSS posture is
    /// forgiving). Mirrors the canonical `PageCollection.Delete(int)`.
    pub fn delete(&self, index: usize) {
        {
            let mut pages = self.pages.borrow_mut();
            if index >= 1 && index <= pages.len() {
                pages.remove(index - 1);
            }
        }
        self.renumber();
        self.dirty.set(true);
    }

    /// Delete the pages at the given 1-based `indexes`. Removal proceeds in
    /// descending order so earlier removals don't shift later indices.
    /// Mirrors the canonical `PageCollection.Delete(int[])`.
    pub fn delete_indexes(&self, indexes: &[i32]) {
        {
            let mut pages = self.pages.borrow_mut();
            let mut sorted: Vec<usize> =
                indexes.iter().filter(|&&i| i >= 1).map(|&i| i as usize).collect();
            sorted.sort_unstable();
            sorted.dedup();
            for &idx in sorted.iter().rev() {
                if idx <= pages.len() {
                    pages.remove(idx - 1);
                }
            }
        }
        self.renumber();
        self.dirty.set(true);
    }

    /// Delete every page. Mirrors the canonical `PageCollection.Delete()`
    /// (and the `ICollection<Page>.Clear` alias).
    pub fn delete_all(&self) {
        self.pages.borrow_mut().clear();
        self.dirty.set(true);
    }

    // ---- save-through plumbing (crate-internal) ----------------------

    /// Whether any mutator ran since load / last save.
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    /// The current page set as a snapshot (cheap Rc-sharing clones). Used
    /// by the document save path to rebuild the `/Pages` `/Kids` and to
    /// emit added leaf objects.
    pub(crate) fn snapshot(&self) -> Vec<Page> {
        self.pages.borrow().clone()
    }

    /// Re-derive each page's 1-based `number` and 0-based `leaf_index` from
    /// its current position, so `Page::number` and `TextAbsorber` (which
    /// indexes by `leaf_index`) stay correct after a mutation.
    fn renumber(&self) {
        let mut pages = self.pages.borrow_mut();
        for (i, page) in pages.iter_mut().enumerate() {
            page.set_position(i as u64);
        }
    }

    /// Build a blank 612×792 page handle that shares the document's source
    /// bytes / edit buffer with the existing pages. Carries `leaf_id == 0`
    /// (assigned a fresh object id by the save path) and a fresh content /
    /// resource / geometry state.
    fn synthesize_blank(&self) -> Page {
        Page::new_blank(Arc::clone(&self.doc_bytes), Rc::clone(&self.edits))
    }

    /// Build a page that reuses `source`'s content + resources (so the copy
    /// renders the same text). The copy shares `source`'s `leaf_id` for
    /// content reuse on save, and its `leaf_index` so an in-session
    /// `TextAbsorber::visit_page` reads `source`'s text.
    fn synthesize_copy(&self, source: &Page) -> Page {
        Page::new_copy(source)
    }
}

impl IntoIterator for &PageCollection {
    type Item = Page;
    type IntoIter = std::vec::IntoIter<Page>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
