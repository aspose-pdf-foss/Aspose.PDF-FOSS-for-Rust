//! `Aspose.Pdf.Facades.PdfContentEditor` — a facade for editing page
//! content (links / annotations / attachments / text / stamps), per
//! `capabilities/pdf_content_editor.yaml`.
//!
//! Extends [`SaveableFacade`](crate::SaveableFacade). The viewer-
//! preference accessors and the document-action name constants are
//! real; the annotation-creation overloads cascade-drop (they take
//! the BCL `System.Drawing.Rectangle` / `Color`). Content-editing
//! operations (replace text / delete image / stamps / attachments)
//! ship as no-op stubs — real content-stream editing + save-through
//! is a follow-on (matching the reference's base behaviour:
//! `ReplaceText` returns `false`, the void editors no-op). The
//! `GetStamps` (`StampInfo[]`) method and `ReplaceTextStrategy` are
//! dropped; the three text-options properties ship as configurable
//! option bags.

use crate::public::annotation::Annotation;
use crate::public::i_facade::{impl_facade, FacadeBase};
use crate::public::i_saveable_facade::impl_saveable_facade;
use crate::public::text_edit_options::TextEditOptions;
use crate::public::text_replace_options::TextReplaceOptions;
use crate::public::text_search_options::TextSearchOptions;
use crate::public::text_state::TextState;
use crate::internal::pending::PENDING;

/// A page-content editing facade.
#[derive(Debug, Default)]
pub struct PdfContentEditor {
    base: FacadeBase,
    viewer_preference: i32,
    text_search_options: TextSearchOptions,
    text_edit_options: TextEditOptions,
    text_replace_options: TextReplaceOptions,
}

impl PdfContentEditor {
    /// Document-action event: document open.
    pub const DOCUMENT_OPEN: &'static str = "DO";
    /// Document-action event: document close.
    pub const DOCUMENT_CLOSE: &'static str = "WC";
    /// Document-action event: document will save.
    pub const DOCUMENT_WILL_SAVE: &'static str = "WS";
    /// Document-action event: document saved.
    pub const DOCUMENT_SAVED: &'static str = "DS";
    /// Document-action event: document will print.
    pub const DOCUMENT_WILL_PRINT: &'static str = "WP";
    /// Document-action event: document printed.
    pub const DOCUMENT_PRINTED: &'static str = "DP";

    /// Construct an unbound editor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct bound to an in-memory document.
    pub fn with_document(document: crate::public::document::Document) -> Self {
        Self {
            base: FacadeBase { document: Some(document) },
            ..Self::default()
        }
    }

    /// The link annotations of the bound document, every page.
    ///
    /// Each comes back as a standalone copy carrying every entry its
    /// source would be written with — its `/A` action included — so it
    /// can be written into another document unchanged. It is not a
    /// [`LinkAnnotation`](crate::LinkAnnotation): nothing can build one
    /// of those from a `&dyn Annotation`, so the typed accessors are
    /// not there and the dictionary is.
    pub fn extract_link(&self) -> Vec<Box<dyn Annotation>> {
        let mut out: Vec<Box<dyn Annotation>> = Vec::new();
        let Some(doc) = self.base.document.as_ref() else {
            return out;
        };
        for number in 1..=doc.pages().count() {
            let Ok(page) = doc.pages().get(number) else {
                continue;
            };
            let annots = page.annotations();
            for index in 0..annots.count() {
                if let Ok(a) = annots.get(index) {
                    if a.annotation_type() == crate::public::annotation_type::AnnotationType::Link {
                        out.push(Box::new(
                            crate::public::annotation::LoadedAnnotation::from_model(&*a),
                        ));
                    }
                }
            }
        }
        out
    }

    /// Set the document's viewer-preference flags.
    pub fn change_viewer_preference(&mut self, preference: i32) {
        self.viewer_preference = preference;
    }
    /// The document's viewer-preference flags.
    pub fn get_viewer_preference(&self) -> i32 {
        self.viewer_preference
    }

    /// The text-search options used by content operations.
    pub fn text_search_options(&self) -> &TextSearchOptions {
        &self.text_search_options
    }

    /// The text-edit options used by content operations.
    pub fn text_edit_options(&self) -> &TextEditOptions {
        &self.text_edit_options
    }

    /// The text-replace options used by content operations.
    pub fn text_replace_options(&self) -> &TextReplaceOptions {
        &self.text_replace_options
    }

    /// Attach the file at `path`, described by `description`. The
    /// attachment reaches the document's [`EmbeddedFileCollection`](crate::EmbeddedFileCollection)
    /// and is embedded when the document is saved.
    pub fn add_document_attachment(&mut self, path: &str, description: &str) {
        let Some(doc) = self.base.document.as_mut() else { return };
        doc.embedded_files_mut()
            .add(crate::public::file_specification::FileSpecification::with_file_and_description(
                path.to_string(),
                description.to_string(),
            ));
    }
    /// Attach a file from bytes.
    ///
    /// [`FileSpecification`](crate::FileSpecification) embeds from a
    /// file path; the shipped subset carries no byte-content path onto
    /// an attachment, so there is nowhere for these bytes to go. Write
    /// them to a file and use
    /// [`add_document_attachment`](Self::add_document_attachment).
    ///
    /// Not implemented — pending `file_specification_contents`. Calling
    /// this panics.
    pub fn add_document_attachment_bytes(
        &mut self,
        _bytes: &[u8],
        _name: &str,
        _description: &str,
    ) {
        PENDING("file_specification_contents")
    }

    /// Delete every document attachment. The removal reaches the
    /// document's [`EmbeddedFileCollection`](crate::EmbeddedFileCollection) and persists on save.
    pub fn delete_attachments(&mut self) {
        let Some(doc) = self.base.document.as_mut() else { return };
        doc.embedded_files_mut().delete_all();
    }

    /// Add a document additional action.
    ///
    /// Not implemented — pending `document_additional_actions`. Calling
    /// this panics.
    pub fn add_document_additional_action(&mut self, _action_name: &str, _javascript: &str) {
        PENDING("document_additional_actions")
    }
    /// Remove the document open action.
    ///
    /// Not implemented — pending `document_additional_actions`. Calling
    /// this panics.
    pub fn remove_document_open_action(&mut self) {
        PENDING("document_additional_actions")
    }

    /// Replace the image at **1-based** `image_index` on **1-based**
    /// `page` with the file at `path`. The replacement reaches the
    /// page's live [`XImageCollection`](crate::XImageCollection) and
    /// persists when the document is saved. A path that cannot be read
    /// leaves the page untouched.
    pub fn replace_image(&mut self, page: i32, image_index: i32, path: &str) {
        if image_index < 1 {
            return;
        }
        let Ok(bytes) = std::fs::read(path) else { return };
        let Some(doc) = self.base.document.as_ref() else { return };
        let Ok(page) = doc.pages().get(page.max(0) as usize) else { return };
        page.resources()
            .images()
            .replace((image_index - 1) as usize, &bytes);
    }

    /// Delete images by **1-based** index on a **1-based** page. The
    /// removals reach the page's live [`XImageCollection`](crate::XImageCollection) (so a bound
    /// [`PdfExtractor`](crate::PdfExtractor) sees the lower count) and
    /// persist when the document is saved.
    pub fn delete_image(&mut self, page: i32, indices: &[i32]) {
        let Some(doc) = self.base.document.as_ref() else { return };
        let Ok(page) = doc.pages().get(page.max(0) as usize) else { return };
        let images = page.resources().images();
        // Remove highest index first so earlier indices stay valid.
        let mut ordered: Vec<i32> = indices.to_vec();
        ordered.sort_unstable();
        ordered.dedup();
        for &idx in ordered.iter().rev() {
            if idx >= 1 {
                images.delete_at((idx - 1) as usize);
            }
        }
    }

    /// Delete every image on every page. The removals reach each page's
    /// live [`XImageCollection`](crate::XImageCollection) and persist on save.
    pub fn delete_image_all(&mut self) {
        let Some(doc) = self.base.document.as_ref() else { return };
        let pages = doc.pages();
        for n in 1..=pages.count() {
            if let Ok(page) = pages.get(n) {
                page.resources().images().delete_all();
            }
        }
    }

    /// Replace every occurrence of `old` with `new`.
    ///
    /// Not implemented — pending `content_stream_authoring`. Calling
    /// this panics.
    pub fn replace_text(&mut self, _old: &str, _new: &str) -> bool {
        PENDING("content_stream_authoring")
    }
    /// Replace on a specific 1-based page.
    ///
    /// Not implemented — pending `content_stream_authoring`. Calling
    /// this panics.
    pub fn replace_text_on_page(&mut self, _old: &str, _page: i32, _new: &str) -> bool {
        PENDING("content_stream_authoring")
    }
    /// Replace with a styling state.
    ///
    /// Not implemented — pending `content_stream_authoring`. Calling
    /// this panics.
    pub fn replace_text_styled(&mut self, _old: &str, _new: &str, _state: TextState) -> bool {
        PENDING("content_stream_authoring")
    }
    /// Replace on a page with a styling state.
    ///
    /// Not implemented — pending `content_stream_authoring`. Calling
    /// this panics.
    pub fn replace_text_on_page_styled(
        &mut self,
        _old: &str,
        _page: i32,
        _new: &str,
        _state: TextState,
    ) -> bool {
        PENDING("content_stream_authoring")
    }
    /// Replace up to `count` occurrences.
    ///
    /// Not implemented — pending `content_stream_authoring`. Calling
    /// this panics.
    pub fn replace_text_count(&mut self, _old: &str, _new: &str, _count: i32) -> bool {
        PENDING("content_stream_authoring")
    }

    /// Delete stamps by id on a page.
    ///
    /// Not implemented — pending `stamp_authoring`. Calling this
    /// panics.
    pub fn delete_stamp(&mut self, _page: i32, _ids: &[i32]) {
        PENDING("stamp_authoring")
    }
    /// Delete stamps by id across the document.
    ///
    /// Not implemented — pending `stamp_authoring`. Calling this
    /// panics.
    pub fn delete_stamp_by_ids(&mut self, _ids: &[i32]) {
        PENDING("stamp_authoring")
    }
    /// Delete stamps by id on a page.
    ///
    /// Not implemented — pending `stamp_authoring`. Calling this
    /// panics.
    pub fn delete_stamp_by_ids_on_page(&mut self, _page: i32, _ids: &[i32]) {
        PENDING("stamp_authoring")
    }
    /// Delete a stamp by id on a page.
    ///
    /// Not implemented — pending `stamp_authoring`. Calling this
    /// panics.
    pub fn delete_stamp_by_id(&mut self, _page: i32, _id: i32) {
        PENDING("stamp_authoring")
    }
    /// Delete a stamp by id across the document.
    ///
    /// Not implemented — pending `stamp_authoring`. Calling this
    /// panics.
    pub fn delete_stamp_by_id_global(&mut self, _id: i32) {
        PENDING("stamp_authoring")
    }
    /// Hide a stamp by id.
    ///
    /// Not implemented — pending `stamp_authoring`. Calling this
    /// panics.
    pub fn hide_stamp_by_id(&mut self, _page: i32, _id: i32) {
        PENDING("stamp_authoring")
    }
    /// Show a stamp by id.
    ///
    /// Not implemented — pending `stamp_authoring`. Calling this
    /// panics.
    pub fn show_stamp_by_id(&mut self, _page: i32, _id: i32) {
        PENDING("stamp_authoring")
    }
    /// Move a stamp by id.
    ///
    /// Not implemented — pending `stamp_authoring`. Calling this
    /// panics.
    pub fn move_stamp_by_id(&mut self, _page: i32, _id: i32, _x: f64, _y: f64) {
        PENDING("stamp_authoring")
    }
    /// Move a stamp by index.
    ///
    /// Not implemented — pending `stamp_authoring`. Calling this
    /// panics.
    pub fn move_stamp(&mut self, _page: i32, _stamp_index: i32, _x: f64, _y: f64) {
        PENDING("stamp_authoring")
    }
}

impl_facade!(PdfContentEditor);
impl_saveable_facade!(PdfContentEditor);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::document::Document;

    #[test]
    fn document_action_constants() {
        assert_eq!(PdfContentEditor::DOCUMENT_OPEN, "DO");
        assert_eq!(PdfContentEditor::DOCUMENT_CLOSE, "WC");
        assert_eq!(PdfContentEditor::DOCUMENT_WILL_SAVE, "WS");
        assert_eq!(PdfContentEditor::DOCUMENT_SAVED, "DS");
        assert_eq!(PdfContentEditor::DOCUMENT_WILL_PRINT, "WP");
        assert_eq!(PdfContentEditor::DOCUMENT_PRINTED, "DP");
    }

    #[test]
    fn viewer_preference_round_trips() {
        let mut ed = PdfContentEditor::with_document(Document::new());
        ed.change_viewer_preference(4);
        assert_eq!(ed.get_viewer_preference(), 4);
        ed.delete_image_all();
        ed.delete_attachments();
    }

    #[test]
    #[should_panic(expected = "content_stream_authoring")]
    fn replace_text_is_pending() {
        let mut ed = PdfContentEditor::with_document(Document::new());
        ed.replace_text("a", "b");
    }

    #[test]
    #[should_panic(expected = "stamp_authoring")]
    fn stamp_removal_is_pending() {
        let mut ed = PdfContentEditor::with_document(Document::new());
        ed.delete_stamp_by_id(1, 1);
    }
}
