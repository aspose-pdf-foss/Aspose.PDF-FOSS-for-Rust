//! `Aspose.Pdf.Facades.PdfExtractor` — a facade that extracts text
//! (and, as stubs, images / attachments) from a document, per
//! `capabilities/pdf_extractor.yaml`.
//!
//! Extends [`Facade`](crate::Facade). Text extraction is real
//! (through the foundation text path, honouring `start_page` /
//! `end_page`); image and attachment extraction are stubs, matching
//! the reference subset. The `Encoding` / `ImageFormat` overloads and
//! the `MemoryStream[]` `GetAttachment` are dropped; the
//! `TextSearchOptions` and `ExtractImageMode` properties ship (get-only).

use crate::internal::text_extractor;
use crate::public::error::Error;
use crate::public::extract_image_mode::ExtractImageMode;
use crate::public::file_specification::FileSpecification;
use crate::public::text_search_options::TextSearchOptions;
use crate::public::i_facade::{impl_facade, FacadeBase};
use std::io::Write;
use std::path::Path;

/// A text / image / attachment extraction facade.
#[derive(Debug, Default)]
pub struct PdfExtractor {
    base: FacadeBase,
    start_page: i32,
    end_page: i32,
    extract_text_mode: i32,
    resolution: i32,
    password: String,
    is_bidi: bool,
    cached: Option<Vec<String>>,
    cursor: usize,
    /// Lazily-built list of the bound document's embedded image bytes
    /// (the raw `/XObject /Image` data — JPEG verbatim for `/DCTDecode`).
    images: Option<Vec<Vec<u8>>>,
    image_cursor: usize,
    extract_image_mode: ExtractImageMode,
    text_search_options: TextSearchOptions,
    /// Which attachment the next [`get_attachment`](PdfExtractor::get_attachment)
    /// writes: `None` until an extraction is asked for, then either one
    /// file name or every attachment.
    attachment_selection: Option<AttachmentSelection>,
}

/// What an `ExtractAttachment` call selected.
#[derive(Debug, Clone, PartialEq, Eq)]
enum AttachmentSelection {
    /// `ExtractAttachment()` — every embedded file.
    All,
    /// `ExtractAttachment(name)` — the one with that file name.
    Named(String),
}

impl PdfExtractor {
    /// Construct an unbound extractor with default settings.
    pub fn new() -> Self {
        Self { start_page: 1, resolution: 150, ..Self::default() }
    }

    /// Construct bound to an in-memory document.
    pub fn with_document(document: crate::public::document::Document) -> Self {
        Self { base: FacadeBase { document: Some(document) }, ..Self::new() }
    }

    /// Which images the extractor collects (resources vs actually-used).
    pub fn extract_image_mode(&self) -> ExtractImageMode {
        self.extract_image_mode
    }

    /// The text-search options used by search-based extraction.
    pub fn text_search_options(&self) -> &TextSearchOptions {
        &self.text_search_options
    }

    /// Extract all in-range page text into the internal buffer.
    pub fn extract_text(&mut self) {
        self.ensure_extracted();
    }

    /// Write the extracted text to a file (lazily extracting first).
    pub fn get_text(&mut self, path: impl AsRef<Path>) -> Result<(), Error> {
        self.ensure_extracted();
        std::fs::write(path, self.joined_text())?;
        Ok(())
    }

    /// Write the extracted text to a byte sink (lazily extracting).
    pub fn get_text_to_writer(&mut self, mut writer: impl Write) -> Result<(), Error> {
        self.ensure_extracted();
        writer.write_all(self.joined_text().as_bytes())?;
        Ok(())
    }

    /// Whether another page's text remains in the page cursor.
    pub fn has_next_page_text(&mut self) -> bool {
        self.ensure_extracted();
        self.cursor < self.cached.as_ref().map_or(0, |p| p.len())
    }

    /// Write the next page's text to a file, advancing the cursor.
    pub fn get_next_page_text(&mut self, path: impl AsRef<Path>) -> Result<(), Error> {
        let page = self.next_page_text();
        std::fs::write(path, page)?;
        Ok(())
    }

    /// Write the next page's text to a byte sink, advancing.
    pub fn get_next_page_text_to_writer(
        &mut self,
        mut writer: impl Write,
    ) -> Result<(), Error> {
        let page = self.next_page_text();
        writer.write_all(page.as_bytes())?;
        Ok(())
    }

    // -- image extraction -------------------------------------------

    /// Begin image extraction: collect the bound document's embedded
    /// images and reset the image cursor.
    pub fn extract_image(&mut self) {
        self.images = Some(self.collect_images());
        self.image_cursor = 0;
    }

    /// Whether another embedded image remains in the image cursor
    /// (collecting lazily on first read).
    pub fn has_next_image(&mut self) -> bool {
        self.ensure_images();
        self.image_cursor < self.images.as_ref().map_or(0, |i| i.len())
    }

    /// Write the next embedded image to a file, advancing the cursor.
    /// Returns `false` (without writing) when no image remains. The
    /// bytes are the raw embedded image data — a `/DCTDecode` image's
    /// verbatim JPEG.
    pub fn get_next_image(&mut self, path: impl AsRef<Path>) -> bool {
        match self.next_image() {
            Some(bytes) => std::fs::write(path, bytes).is_ok(),
            None => false,
        }
    }

    /// Write the next embedded image to a byte sink, advancing the
    /// cursor. Returns `false` when no image remains.
    pub fn get_next_image_to_writer(&mut self, mut writer: impl Write) -> bool {
        match self.next_image() {
            Some(bytes) => writer.write_all(&bytes).is_ok(),
            None => false,
        }
    }

    // -- attachment extraction --------------------------------------

    /// The file names of the document's embedded files, in name-tree
    /// order. Empty when the editor is unbound or the document carries
    /// none.
    pub fn get_attach_names(&self) -> Vec<String> {
        self.attachments().into_iter().map(|a| a.name).collect()
    }

    /// Select every attachment, so the next
    /// [`get_attachment`](Self::get_attachment) writes them all.
    pub fn extract_attachment(&mut self) {
        self.attachment_selection = Some(AttachmentSelection::All);
    }

    /// Select the attachment whose file name is `attachment_file_name`.
    pub fn extract_attachment_named(&mut self, attachment_file_name: &str) {
        self.attachment_selection =
            Some(AttachmentSelection::Named(attachment_file_name.to_string()));
    }

    /// Write what the last `ExtractAttachment` call selected to
    /// `output_path`.
    ///
    /// The canonical parameter is a path, not a name, and what it means
    /// follows the selection: after
    /// [`extract_attachment_named`](Self::extract_attachment_named) it
    /// is the file to write that one attachment to; after
    /// [`extract_attachment`](Self::extract_attachment) it is a
    /// directory, created if missing, and each attachment is written
    /// inside it under its own name. Writing several files into one
    /// path would lose all but the last.
    ///
    /// Nothing happens when no extraction was asked for, when the
    /// document has no such attachment, or when the attachment's filter
    /// chain is one this crate cannot undo — an encoded blob written
    /// out as if it were the file would be worse than no file.
    pub fn get_attachment(&mut self, output_path: &str) {
        let Some(selection) = self.attachment_selection.clone() else {
            return;
        };
        let Some(bytes) = self.base.document.as_ref().map(|d| d.internal_bytes().to_vec()) else {
            return;
        };
        match selection {
            AttachmentSelection::Named(name) => {
                if let Some(data) =
                    crate::internal::embedded_file_io::extract_contents(&bytes, &name)
                {
                    let _ = std::fs::write(output_path, data);
                }
            }
            AttachmentSelection::All => {
                let dir = Path::new(output_path);
                if std::fs::create_dir_all(dir).is_err() {
                    return;
                }
                for name in self.get_attach_names() {
                    if let Some(data) =
                        crate::internal::embedded_file_io::extract_contents(&bytes, &name)
                    {
                        let _ = std::fs::write(dir.join(&name), data);
                    }
                }
            }
        }
    }

    /// The file specification of each attachment — its names, its
    /// description, its `/Params` metadata.
    pub fn get_attachment_info(&self) -> Vec<FileSpecification> {
        self.attachments()
            .into_iter()
            .map(|a| {
                let mut spec = FileSpecification::new();
                spec.set_name(a.name);
                spec.set_unicode_name(a.unicode_name);
                spec.set_description(a.description);
                spec.set_mime_type(a.mime_type);
                spec
            })
            .collect()
    }

    /// The bound document's embedded-file entries, or none when
    /// unbound.
    fn attachments(&self) -> Vec<crate::internal::embedded_file_io::EmbeddedFileData> {
        match self.base.document.as_ref() {
            Some(doc) => crate::internal::embedded_file_io::parse(doc.internal_bytes()),
            None => Vec::new(),
        }
    }

    // -- properties --------------------------------------------------

    /// The first page to extract (1-based).
    pub fn start_page(&self) -> i32 {
        self.start_page
    }
    /// Set the first page to extract (1-based).
    pub fn set_start_page(&mut self, value: i32) {
        self.start_page = value;
        self.invalidate();
    }

    /// The last page to extract (0 = through the end).
    pub fn end_page(&self) -> i32 {
        self.end_page
    }
    /// Set the last page to extract (0 = through the end).
    pub fn set_end_page(&mut self, value: i32) {
        self.end_page = value;
        self.invalidate();
    }

    /// The text-extraction mode.
    pub fn extract_text_mode(&self) -> i32 {
        self.extract_text_mode
    }
    /// Set the text-extraction mode.
    pub fn set_extract_text_mode(&mut self, value: i32) {
        self.extract_text_mode = value;
    }

    /// The image-extraction resolution, in DPI.
    pub fn resolution(&self) -> i32 {
        self.resolution
    }
    /// Set the image-extraction resolution, in DPI.
    pub fn set_resolution(&mut self, value: i32) {
        self.resolution = value;
    }

    /// The password used to open the document.
    ///
    /// Carried, not used: an extractor is bound to a [`Document`](crate::Document) that
    /// is already open, and a document that needed a password was
    /// authenticated when it was opened. Use
    /// [`Document::open_with_password`](crate::Document::open_with_password)
    /// for that.
    pub fn password(&self) -> &str {
        &self.password
    }
    /// Set the password used to open the document.
    pub fn set_password(&mut self, value: impl Into<String>) {
        self.password = value.into();
    }

    /// Whether bidirectional text reordering is applied.
    pub fn is_bidi(&self) -> bool {
        self.is_bidi
    }
    /// Set whether bidirectional text reordering is applied.
    pub fn set_is_bidi(&mut self, value: bool) {
        self.is_bidi = value;
    }

    // -- internals ---------------------------------------------------

    fn invalidate(&mut self) {
        self.cached = None;
        self.cursor = 0;
    }

    fn ensure_extracted(&mut self) {
        if self.cached.is_some() {
            return;
        }
        let pages = self
            .base
            .document
            .as_ref()
            .and_then(|d| text_extractor::parse(d.internal_bytes()).ok())
            .unwrap_or_default();
        let len = pages.len();
        let start = (self.start_page.max(1) as usize).saturating_sub(1);
        let end = if self.end_page <= 0 {
            len
        } else {
            (self.end_page as usize).min(len)
        };
        let ranged = if start < end { pages[start..end].to_vec() } else { Vec::new() };
        self.cached = Some(ranged);
        self.cursor = 0;
    }

    fn joined_text(&self) -> String {
        self.cached.as_ref().map(|p| p.join("\n")).unwrap_or_default()
    }

    fn ensure_images(&mut self) {
        if self.images.is_none() {
            self.images = Some(self.collect_images());
        }
    }

    /// Collect every embedded image's raw bytes from the bound
    /// document's live page collection (so freshly-added, not-yet-saved
    /// images are surfaced too).
    fn collect_images(&self) -> Vec<Vec<u8>> {
        let Some(doc) = self.base.document.as_ref() else {
            return Vec::new();
        };
        let pages = doc.pages();
        let mut out = Vec::new();
        for n in 1..=pages.count() {
            if let Ok(page) = pages.get(n) {
                let images = page.resources().images();
                for i in 0..images.count() {
                    if let Some(img) = images.get(i) {
                        out.push(img.get_raw_image_data());
                    }
                }
            }
        }
        out
    }

    fn next_image(&mut self) -> Option<Vec<u8>> {
        self.ensure_images();
        let bytes = self.images.as_ref().and_then(|i| i.get(self.image_cursor)).cloned();
        if bytes.is_some() {
            self.image_cursor += 1;
        }
        bytes
    }

    fn next_page_text(&mut self) -> String {
        self.ensure_extracted();
        let page = self
            .cached
            .as_ref()
            .and_then(|p| p.get(self.cursor))
            .cloned()
            .unwrap_or_default();
        self.cursor += 1;
        page
    }
}

impl_facade!(PdfExtractor);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::document::Document;

    fn two_pages_pdf() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/text_extractor/two_pages.pdf")
    }

    #[test]
    fn property_defaults() {
        let mut ex = PdfExtractor::new();
        assert_eq!(ex.start_page(), 1);
        assert_eq!(ex.end_page(), 0);
        assert_eq!(ex.extract_text_mode(), 0);
        assert_eq!(ex.resolution(), 150);
        assert!(ex.password().is_empty());
        assert!(!ex.is_bidi());
        assert!(!ex.has_next_image());
    }

    #[test]
    fn an_unbound_extractor_has_no_attachments() {
        let mut ex = PdfExtractor::new();
        assert!(ex.get_attach_names().is_empty());
        assert!(ex.get_attachment_info().is_empty());
        // Selecting and writing with no document bound does nothing.
        ex.extract_attachment();
        ex.get_attachment("/dev/null");
    }

    #[test]
    fn extracts_text_and_page_cursor() {
        let doc = Document::open(two_pages_pdf()).unwrap();
        let mut ex = PdfExtractor::with_document(doc);
        let mut all = Vec::new();
        ex.get_text_to_writer(&mut all).unwrap();
        let text = String::from_utf8_lossy(&all);
        assert!(text.contains("Page"), "got: {text:?}");

        // Page cursor walks each page then stops.
        let total_pages = {
            let mut n = 0;
            while ex.has_next_page_text() {
                let mut p = Vec::new();
                ex.get_next_page_text_to_writer(&mut p).unwrap();
                n += 1;
            }
            n
        };
        assert!(total_pages >= 1);
        assert!(!ex.has_next_page_text());
    }
}
