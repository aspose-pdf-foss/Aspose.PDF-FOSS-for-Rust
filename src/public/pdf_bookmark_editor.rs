//! `Aspose.Pdf.Facades.PdfBookmarkEditor` — a facade for creating /
//! extracting / editing document bookmarks, per
//! `capabilities/pdf_bookmark_editor.yaml`.
//!
//! Extends [`SaveableFacade`](crate::SaveableFacade). The editor
//! manages an in-memory [`Bookmarks`] list (create / extract /
//! delete / modify), and
//! [`create_bookmarks`](PdfBookmarkEditor::create_bookmarks) fills that
//! list from the bound document's own `/Outlines`. The XML / HTML
//! import-export entry points are no-op stubs. The `Color` create
//! overload and the `Stream` XML overloads are dropped.

use crate::public::bookmark::Bookmark;
use crate::public::bookmarks::Bookmarks;
use crate::public::error::Error;
use crate::public::i_facade::{impl_facade, FacadeBase};
use crate::public::i_saveable_facade::impl_saveable_facade;
use std::path::Path;
use crate::internal::pending::PENDING;

/// A document bookmark editor.
#[derive(Debug, Default)]
pub struct PdfBookmarkEditor {
    base: FacadeBase,
    bookmarks: Bookmarks,
}

impl PdfBookmarkEditor {
    /// Construct an unbound editor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct bound to an in-memory document.
    pub fn with_document(document: crate::public::document::Document) -> Self {
        Self { base: FacadeBase { document: Some(document) }, bookmarks: Bookmarks::new() }
    }

    /// Fill the editor's list from the bound document's outline — the
    /// canonical parameterless `CreateBookmarks`, the one that reads
    /// the document rather than being told what to add.
    ///
    /// Each bookmark takes the outline item's title, style and open
    /// flag, its nesting level (1-based), and the page its destination
    /// names. A title-only item, or one whose action this crate does
    /// not model, gets page 0 — there is no page to name. Nothing
    /// happens when the editor is unbound.
    pub fn create_bookmarks(&mut self) {
        let Some(doc) = self.base.document.as_ref() else {
            return;
        };
        let outlines = doc.outlines();
        for index in 1..=outlines.count() as i32 {
            let item = outlines.get(index);
            self.bookmarks.add(Self::bookmark_of(&item, 1));
        }
    }

    /// One outline item as a [`Bookmark`], recursing its children.
    fn bookmark_of(
        item: &crate::public::outline_item_collection::OutlineItemCollection,
        level: i32,
    ) -> Bookmark {
        let mut bm = Bookmark::new();
        bm.set_title(item.title());
        bm.set_bold_flag(item.bold());
        bm.set_italic_flag(item.italic());
        bm.set_open(item.open());
        bm.set_level(level);
        bm.set_page_number(Self::page_of(item));
        let mut children = Bookmarks::new();
        for index in 1..=item.count() as i32 {
            children.add(Self::bookmark_of(&item.get(index), level + 1));
        }
        bm.set_child_items(children);
        bm
    }

    /// The page an item's destination names, or 0 when it names none.
    ///
    /// A destination renders as the canonical string whose first token
    /// is the page number (`"1 XYZ 0 792 0"`), which is where this
    /// reads it from.
    fn page_of(
        item: &crate::public::outline_item_collection::OutlineItemCollection,
    ) -> i32 {
        item.destination()
            .and_then(|d| {
                d.to_string()
                    .split_whitespace()
                    .next()
                    .and_then(|first| first.parse::<i32>().ok())
            })
            .unwrap_or(0)
    }

    /// Add a bookmark to `page` with the given `title`.
    pub fn create_bookmark_of_page(&mut self, title: impl Into<String>, page: i32) {
        let mut bm = Bookmark::new();
        bm.set_title(title);
        bm.set_page_number(page);
        self.bookmarks.add(bm);
    }

    /// Add a bookmark per `(title, page)` pair.
    pub fn create_bookmark_of_pages(&mut self, titles: &[&str], pages: &[i32]) {
        for (t, p) in titles.iter().zip(pages.iter()) {
            self.create_bookmark_of_page(*t, *p);
        }
    }

    /// Add an existing bookmark.
    pub fn create_bookmarks_from(&mut self, bookmark: Bookmark) {
        self.bookmarks.add(bookmark);
    }

    /// Remove every editor bookmark.
    pub fn delete_bookmarks(&mut self) {
        self.bookmarks.clear();
    }

    /// Remove editor bookmarks with the given title.
    pub fn delete_bookmarks_named(&mut self, title: &str) {
        let kept: Vec<Bookmark> =
            self.bookmarks.iter().filter(|b| b.title() != title).cloned().collect();
        self.bookmarks = Bookmarks::new();
        for b in kept {
            self.bookmarks.add(b);
        }
    }

    /// Rename editor bookmarks from `old_title` to `new_title`.
    pub fn modify_bookmarks(&mut self, old_title: &str, new_title: &str) {
        for i in 0..self.bookmarks.count() {
            if let Some(b) = self.bookmarks.get_mut(i) {
                if b.title() == old_title {
                    b.set_title(new_title.to_string());
                }
            }
        }
    }

    /// Extract the editor's bookmarks.
    pub fn extract_bookmarks(&self) -> Bookmarks {
        self.bookmarks.clone()
    }

    /// Extract bookmarks, optionally including children (the flag is
    /// accepted; the editor list is returned either way).
    pub fn extract_bookmarks_with_children(&self, _include_children: bool) -> Bookmarks {
        self.bookmarks.clone()
    }

    /// Extract bookmarks matching `title`.
    pub fn extract_bookmarks_named(&self, title: &str) -> Bookmarks {
        let mut out = Bookmarks::new();
        for b in self.bookmarks.iter().filter(|b| b.title() == title) {
            out.add(b.clone());
        }
        out
    }

    /// Extract the child bookmarks of `parent`.
    pub fn extract_bookmarks_under(&self, parent: &Bookmark) -> Bookmarks {
        parent.child_items().clone()
    }

    // -- XML / HTML import-export (no-op stubs) ---------------------

    /// Export bookmarks to an XML file.
    ///
    /// Not implemented — pending `bookmark_serialization`. Calling this
    /// panics.
    pub fn export_bookmarks_to_xml(&self, _path: impl AsRef<Path>) -> Result<(), Error> {
        PENDING("bookmark_serialization")
    }
    /// Import bookmarks from an XML file.
    ///
    /// Not implemented — pending `bookmark_serialization`. Calling this
    /// panics.
    pub fn import_bookmarks_with_xml(&mut self, _path: impl AsRef<Path>) -> Result<(), Error> {
        PENDING("bookmark_serialization")
    }
    /// Export bookmarks to HTML.
    ///
    /// Not implemented — pending `bookmark_serialization`. Calling this
    /// panics.
    pub fn export_bookmarks_to_html(
        &self,
        _dir: impl AsRef<Path>,
        _file: impl AsRef<Path>,
    ) -> Result<(), Error> {
        PENDING("bookmark_serialization")
    }
    /// Extract bookmarks to HTML.
    ///
    /// Not implemented — pending `bookmark_serialization`. Calling this
    /// panics.
    pub fn extract_bookmarks_to_html(
        &self,
        _dir: impl AsRef<Path>,
        _file: impl AsRef<Path>,
    ) -> Result<(), Error> {
        PENDING("bookmark_serialization")
    }
}

impl_facade!(PdfBookmarkEditor);
impl_saveable_facade!(PdfBookmarkEditor);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::document::Document;

    #[test]
    fn create_extract_delete_modify() {
        let mut ed = PdfBookmarkEditor::with_document(Document::new());
        assert!(ed.extract_bookmarks().is_empty());

        ed.create_bookmark_of_page("Chapter 1", 1);
        ed.create_bookmark_of_page("Chapter 2", 2);
        assert_eq!(ed.extract_bookmarks().count(), 2);
        assert_eq!(ed.extract_bookmarks().get(0).unwrap().title(), "Chapter 1");

        ed.modify_bookmarks("Chapter 1", "Intro");
        assert_eq!(ed.extract_bookmarks_named("Intro").count(), 1);

        ed.delete_bookmarks_named("Chapter 2");
        assert_eq!(ed.extract_bookmarks().count(), 1);

        ed.delete_bookmarks();
        assert!(ed.extract_bookmarks().is_empty());
    }
}
