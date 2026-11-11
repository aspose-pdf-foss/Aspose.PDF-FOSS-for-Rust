//! `Aspose.Pdf.Facades.PdfPageEditor` — a facade for page-level
//! edits (move / rotate / zoom / align + page transitions), per
//! `capabilities/pdf_page_editor.yaml`.
//!
//! Extends [`SaveableFacade`](crate::SaveableFacade). `get_pages` /
//! `get_page_size` / `get_page_rotation` read the bound document;
//! `move_position` / `apply_changes` and the transform settings
//! stage edits (applying them to the saved document is a follow-on).
//! The `GetPageBoxSize` (`System.Drawing.Rectangle`), `PageRotations`
//! (`Dictionary`), and `AlignmentType` / `VerticalAlignmentType`
//! (Facades enums) members are dropped.
//!
//! **What "stage" means for the transform settings.**
//! [`zoom`](PdfPageEditor::zoom),
//! [`page_size`](PdfPageEditor::page_size),
//! [`vertical_alignment_type`](PdfPageEditor::vertical_alignment_type)
//! and [`process_pages`](PdfPageEditor::process_pages) are the
//! arguments of [`apply_changes`](PdfPageEditor::apply_changes), which
//! declares itself unimplemented: scaling or re-boxing a page means
//! rewriting its content stream around a new transform, which this
//! crate does not author. They are stored and read back exactly, and
//! nothing else reads them. The page transitions beside them
//! ([`transition_type`](PdfPageEditor::transition_type) and the two
//! durations) are §12.4.4 `/Trans` entries and wait on the same call.

use crate::public::horizontal_alignment::HorizontalAlignment;
use crate::public::i_facade::{impl_facade, FacadeBase};
use crate::public::i_saveable_facade::impl_saveable_facade;
use crate::public::page_size::PageSize;
use crate::public::vertical_alignment::VerticalAlignment;
use crate::internal::pending::PENDING;

/// A page-level edit facade.
#[derive(Debug)]
pub struct PdfPageEditor {
    base: FacadeBase,
    transition_duration: i32,
    transition_type: i32,
    display_duration: i32,
    process_pages: Vec<i32>,
    rotation: i32,
    zoom: f32,
    page_size: PageSize,
    horizontal_alignment: HorizontalAlignment,
    vertical_alignment_type: VerticalAlignment,
}

impl Default for PdfPageEditor {
    fn default() -> Self {
        Self {
            base: FacadeBase::default(),
            transition_duration: 0,
            transition_type: 0,
            display_duration: 0,
            process_pages: Vec::new(),
            rotation: 0,
            zoom: 1.0,
            page_size: PageSize::new(612.0, 792.0),
            horizontal_alignment: HorizontalAlignment::None,
            vertical_alignment_type: VerticalAlignment::None,
        }
    }
}

impl PdfPageEditor {
    /// Transition style: vertical split outward.
    pub const SPLITVOUT: i32 = 1;
    /// Transition style: horizontal split outward.
    pub const SPLITHOUT: i32 = 2;
    /// Transition style: vertical split inward.
    pub const SPLITVIN: i32 = 3;
    /// Transition style: horizontal split inward.
    pub const SPLITHIN: i32 = 4;
    /// Transition style: vertical blinds.
    pub const BLINDV: i32 = 5;
    /// Transition style: horizontal blinds.
    pub const BLINDH: i32 = 6;
    /// Transition style: box inward.
    pub const INBOX: i32 = 7;
    /// Transition style: box outward.
    pub const OUTBOX: i32 = 8;
    /// Transition style: left-to-right wipe.
    pub const LRWIPE: i32 = 9;
    /// Transition style: right-to-left wipe.
    pub const RLWIPE: i32 = 10;
    /// Transition style: bottom-to-top wipe.
    pub const BTWIPE: i32 = 11;
    /// Transition style: top-to-bottom wipe.
    pub const TBWIPE: i32 = 12;
    /// Transition style: dissolve.
    pub const DISSOLVE: i32 = 13;
    /// Transition style: left-to-right glitter.
    pub const LRGLITTER: i32 = 14;
    /// Transition style: top-to-bottom glitter.
    pub const TBGLITTER: i32 = 15;
    /// Transition style: diagonal glitter.
    pub const DGLITTER: i32 = 16;

    /// Construct an unbound editor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct bound to an in-memory document.
    pub fn with_document(document: crate::public::document::Document) -> Self {
        Self { base: FacadeBase { document: Some(document) }, ..Self::default() }
    }

    /// The number of pages (0 when unbound).
    pub fn get_pages(&self) -> i32 {
        self.base.document.as_ref().map_or(0, |d| d.pages().count() as i32)
    }

    /// The size of a 1-based page (US-Letter when out of range).
    pub fn get_page_size(&self, page: i32) -> PageSize {
        self.page(page)
            .map(|p| PageSize::new(p.width() as f32, p.height() as f32))
            .unwrap_or_else(|| PageSize::new(612.0, 792.0))
    }

    /// The rotation of a 1-based page, in degrees (0 if out of range).
    pub fn get_page_rotation(&self, page: i32) -> i32 {
        self.page(page).map_or(0, |p| p.rotation())
    }

    fn page(&self, page: i32) -> Option<crate::public::page::Page> {
        if page < 1 {
            return None;
        }
        self.base.document.as_ref().and_then(|d| d.pages().get(page as usize).ok())
    }

    /// Stage a content move.
    ///
    /// Not implemented — pending `content_stream_authoring`. Calling
    /// this panics.
    pub fn move_position(&mut self, _x: f32, _y: f32) {
        PENDING("content_stream_authoring")
    }

    /// Apply the staged edits.
    ///
    /// Not implemented — pending `content_stream_authoring`. Calling
    /// this panics.
    pub fn apply_changes(&mut self) {
        PENDING("content_stream_authoring")
    }

    /// The page-transition duration, in seconds.
    pub fn transition_duration(&self) -> i32 {
        self.transition_duration
    }
    /// Set the page-transition duration, in seconds.
    pub fn set_transition_duration(&mut self, value: i32) {
        self.transition_duration = value;
    }

    /// The page-transition style (a `SPLITVOUT`..`DGLITTER` constant).
    pub fn transition_type(&self) -> i32 {
        self.transition_type
    }
    /// Set the page-transition style.
    pub fn set_transition_type(&mut self, value: i32) {
        self.transition_type = value;
    }

    /// The page display duration, in seconds.
    pub fn display_duration(&self) -> i32 {
        self.display_duration
    }
    /// Set the page display duration, in seconds.
    pub fn set_display_duration(&mut self, value: i32) {
        self.display_duration = value;
    }

    /// The 1-based page numbers the edits apply to.
    pub fn process_pages(&self) -> &[i32] {
        &self.process_pages
    }
    /// Set the 1-based page numbers the edits apply to.
    pub fn set_process_pages(&mut self, value: Vec<i32>) {
        self.process_pages = value;
    }

    /// The rotation to apply, in degrees.
    pub fn rotation(&self) -> i32 {
        self.rotation
    }
    /// Set the rotation to apply, in degrees.
    pub fn set_rotation(&mut self, value: i32) {
        self.rotation = value;
    }

    /// The zoom factor to apply.
    pub fn zoom(&self) -> f32 {
        self.zoom
    }
    /// Set the zoom factor to apply.
    pub fn set_zoom(&mut self, value: f32) {
        self.zoom = value;
    }

    /// The page size to apply.
    pub fn page_size(&self) -> PageSize {
        self.page_size
    }
    /// Set the page size to apply.
    pub fn set_page_size(&mut self, value: PageSize) {
        self.page_size = value;
    }

    /// The horizontal alignment to apply.
    pub fn horizontal_alignment(&self) -> HorizontalAlignment {
        self.horizontal_alignment
    }
    /// Set the horizontal alignment to apply.
    pub fn set_horizontal_alignment(&mut self, value: HorizontalAlignment) {
        self.horizontal_alignment = value;
    }

    /// The vertical alignment to apply.
    pub fn vertical_alignment_type(&self) -> VerticalAlignment {
        self.vertical_alignment_type
    }
    /// Set the vertical alignment to apply.
    pub fn set_vertical_alignment_type(&mut self, value: VerticalAlignment) {
        self.vertical_alignment_type = value;
    }
}

impl_facade!(PdfPageEditor);
impl_saveable_facade!(PdfPageEditor);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::document::Document;

    #[test]
    fn transition_constants() {
        assert_eq!(PdfPageEditor::SPLITVOUT, 1);
        assert_eq!(PdfPageEditor::SPLITHIN, 4);
        assert_eq!(PdfPageEditor::INBOX, 7);
        assert_eq!(PdfPageEditor::DISSOLVE, 13);
        assert_eq!(PdfPageEditor::DGLITTER, 16);
    }

    #[test]
    fn get_pages_and_geometry() {
        let mut ed = PdfPageEditor::new();
        assert_eq!(ed.get_pages(), 0);
        ed = PdfPageEditor::with_document(Document::new());
        assert_eq!(ed.get_pages(), ed.get_pages()); // bound, no panic
        let sz = ed.get_page_size(1);
        assert!(sz.width() > 0.0 && sz.height() > 0.0);
    }

    #[test]
    #[should_panic(expected = "content_stream_authoring")]
    fn apply_changes_is_pending() {
        let mut ed = PdfPageEditor::with_document(Document::new());
        ed.apply_changes();
    }

    #[test]
    fn property_roundtrip() {
        let mut ed = PdfPageEditor::new();
        assert_eq!(ed.zoom(), 1.0);
        ed.set_transition_duration(5);
        ed.set_transition_type(PdfPageEditor::BLINDV);
        ed.set_display_duration(3);
        ed.set_process_pages(vec![1, 3, 5]);
        ed.set_rotation(90);
        ed.set_zoom(1.5);
        assert_eq!(ed.transition_duration(), 5);
        assert_eq!(ed.transition_type(), PdfPageEditor::BLINDV);
        assert_eq!(ed.process_pages().len(), 3);
        assert_eq!(ed.rotation(), 90);
        assert_eq!(ed.zoom(), 1.5);
    }
}
