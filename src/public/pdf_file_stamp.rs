//! `Aspose.Pdf.Facades.PdfFileStamp` — a facade for stamping headers,
//! footers, and page numbers onto a bound document, per
//! `capabilities/pdf_file_stamp.yaml`.
//!
//! Extends [`SaveableFacade`](crate::SaveableFacade). The string-keyed
//! header / footer / page-number stamping methods and the staged
//! properties (optimize / security / rotation / numbering / format)
//! ship; `Save` persists the bound document through the inherited
//! [`ISaveableFacade`](crate::public::i_saveable_facade::ISaveableFacade).
//!
//! Dropped (outside the shipped subset, mirroring the C++ reference):
//! the `Stream` constructors / `InputStream` / `OutputStream`, the
//! `FormattedText`- and `Stream`-shaped stamping overloads (those types
//! are not in the catalog), and `AddStamp(Stamp)`.
//!
//! Behaviour note: the stamping methods draw Helvetica text into each
//! bound page's content stream immediately (via the page text-render
//! path); `Save` then persists those edits. Page numbers are formatted
//! per page — the `#` in the format string is replaced with the page's
//! number, written in the
//! [`numbering_style`](PdfFileStamp::numbering_style) (§12.4.2), and
//! turned by [`page_number_rotation`](PdfFileStamp::page_number_rotation)
//! where the rotation applies to the number alone, not to a header or
//! footer. When no document is bound the methods are no-ops.
//!
//! **Carried, not written.**
//! [`optimize_size`](PdfFileStamp::optimize_size) asks for an output
//! with its unused objects dropped, which needs a writer that rebuilds
//! the whole file; saving here appends an incremental update.
//! [`keep_security`](PdfFileStamp::keep_security) asks for the source's
//! encryption to carry into the output, which that same rewrite would
//! have to re-encrypt. [`stamp_id`](PdfFileStamp::stamp_id) identifies
//! a stamp so it can be found again; a stamp drawn into a content
//! stream is just marks on the page, with no object to carry an id.
//!
//! [`input_file`](PdfFileStamp::input_file) and
//! [`output_file`](PdfFileStamp::output_file) are a second channel for
//! what this facade already takes elsewhere: the input is the document
//! [`bind_pdf`](crate::IFacade::bind_pdf) opened, and the output is the
//! path `Save` is given. Setting either names a file nothing reads.
//! [`set_convert_to`](PdfFileStamp::set_convert_to) asks for the output to be
//! written to a different PDF level or profile, which needs the same
//! whole-file rewrite `optimize_size` does.

use crate::public::document::Document;
use crate::public::error::Error;
use crate::public::i_facade::{impl_facade, FacadeBase};
use crate::public::i_saveable_facade::impl_saveable_facade;
use crate::public::numbering_style::NumberingStyle;
use crate::public::pdf_format::PdfFormat;
use crate::public::position::Position;
use crate::public::text_builder::TextBuilder;
use crate::public::text_fragment::TextFragment;
use std::path::Path;

/// Default stamp font size, in points.
const STAMP_FONT_SIZE: f32 = 12.0;
/// Left inset for header / footer text, in points.
const STAMP_LEFT: f64 = 72.0;

/// Default US-Letter page dimensions (PDF points), used when no
/// document is bound.
const LETTER_WIDTH: f32 = 612.0;
const LETTER_HEIGHT: f32 = 792.0;

/// A facade that stamps headers / footers / page numbers onto a PDF.
#[derive(Debug, Default)]
pub struct PdfFileStamp {
    base: FacadeBase,
    optimize_size: bool,
    keep_security: bool,
    input_file: String,
    output_file: String,
    page_number_rotation: f32,
    convert_to: PdfFormat,
    starting_number: i32,
    numbering_style: NumberingStyle,
    stamp_id: i32,
}

impl PdfFileStamp {
    // ---- Stamp position presets (canonical 0..7) ----

    /// Bottom-middle stamp position.
    pub const POS_BOTTOM_MIDDLE: i32 = 0;
    /// Bottom-right stamp position.
    pub const POS_BOTTOM_RIGHT: i32 = 1;
    /// Upper-right stamp position.
    pub const POS_UPPER_RIGHT: i32 = 2;
    /// Right-sides stamp position.
    pub const POS_SIDES_RIGHT: i32 = 3;
    /// Upper-middle stamp position.
    pub const POS_UPPER_MIDDLE: i32 = 4;
    /// Bottom-left stamp position.
    pub const POS_BOTTOM_LEFT: i32 = 5;
    /// Left-sides stamp position.
    pub const POS_SIDES_LEFT: i32 = 6;
    /// Upper-left stamp position.
    pub const POS_UPPER_LEFT: i32 = 7;

    /// Construct an unbound facade.
    pub fn new() -> Self {
        Self { starting_number: 1, ..Self::default() }
    }

    /// Open `input_file` and stage `output_file` as the save target.
    pub fn with_files(
        input_file: impl AsRef<Path>,
        output_file: impl Into<String>,
    ) -> Result<Self, Error> {
        Self::with_files_and_security(input_file, output_file, false)
    }

    /// Open `input_file`, stage `output_file`, and set whether the
    /// source document's security is preserved on save.
    pub fn with_files_and_security(
        input_file: impl AsRef<Path>,
        output_file: impl Into<String>,
        keep_security: bool,
    ) -> Result<Self, Error> {
        let path = input_file.as_ref();
        let document = Document::open(path)?;
        Ok(Self {
            base: FacadeBase { document: Some(document) },
            keep_security,
            input_file: path.to_string_lossy().into_owned(),
            output_file: output_file.into(),
            starting_number: 1,
            ..Self::default()
        })
    }

    /// Bind to an in-memory [`Document`].
    pub fn with_document(document: Document) -> Self {
        Self {
            base: FacadeBase { document: Some(document) },
            starting_number: 1,
            ..Self::default()
        }
    }

    /// Bind to an in-memory [`Document`] and stage `output_file`.
    pub fn with_document_and_output(
        document: Document,
        output_file: impl Into<String>,
    ) -> Self {
        Self {
            base: FacadeBase { document: Some(document) },
            output_file: output_file.into(),
            starting_number: 1,
            ..Self::default()
        }
    }

    // ---- Stamping (draws into each bound page on the spot) ----

    /// Stamp a page number using `format` (e.g. `"Page #"`) — the `#`
    /// is replaced with each page's number (from `starting_number`).
    pub fn add_page_number(&mut self, format: &str) {
        self.stamp_page_numbers(format, STAMP_LEFT, 30.0);
    }

    /// Stamp a page number starting at `starting_number`.
    pub fn add_page_number_from(&mut self, format: &str, starting_number: i32) {
        self.starting_number = starting_number;
        self.stamp_page_numbers(format, STAMP_LEFT, 30.0);
    }

    /// Stamp a page number at `(left, top)`.
    pub fn add_page_number_at(&mut self, format: &str, left: f32, top: f32) {
        let left = left as f64;
        self.for_each_page(self.page_number_rotation as f64, |i, _w, h| {
            Some((
                render_number(format, self.starting_number + i, self.numbering_style),
                left,
                h - top as f64,
            ))
        });
    }

    /// Stamp a page number within `(llx, lly, urx, ury)`, starting at
    /// `starting_number`.
    pub fn add_page_number_in_rect(
        &mut self,
        format: &str,
        starting_number: i32,
        llx: f32,
        lly: f32,
        _urx: f32,
        _ury: f32,
    ) {
        self.starting_number = starting_number;
        let (llx, lly) = (llx as f64, lly as f64);
        self.for_each_page(self.page_number_rotation as f64, |i, _w, _h| {
            Some((
                render_number(format, self.starting_number + i, self.numbering_style),
                llx,
                lly,
            ))
        });
    }

    /// Stamp `text` as a header `top_margin` points from the top.
    pub fn add_header(&mut self, text: &str, top_margin: f32) {
        let top = top_margin as f64;
        self.for_each_page(0.0, |_i, _w, h| Some((text.to_string(), STAMP_LEFT, h - top)));
    }

    /// Stamp `text` as a header with explicit top / left / right margins.
    pub fn add_header_with_margins(
        &mut self,
        text: &str,
        top_margin: f32,
        left_margin: f32,
        _right_margin: f32,
    ) {
        let (top, left) = (top_margin as f64, left_margin as f64);
        self.for_each_page(0.0, |_i, _w, h| Some((text.to_string(), left, h - top)));
    }

    /// Stamp `text` as a footer `bottom_margin` points from the bottom.
    pub fn add_footer(&mut self, text: &str, bottom_margin: f32) {
        let bottom = bottom_margin as f64;
        self.for_each_page(0.0, |_i, _w, _h| Some((text.to_string(), STAMP_LEFT, bottom)));
    }

    /// Stamp `text` as a footer with explicit bottom / left / right
    /// margins.
    pub fn add_footer_with_margins(
        &mut self,
        text: &str,
        bottom_margin: f32,
        left_margin: f32,
        _right_margin: f32,
    ) {
        let (bottom, left) = (bottom_margin as f64, left_margin as f64);
        self.for_each_page(0.0, |_i, _w, _h| Some((text.to_string(), left, bottom)));
    }

    /// Stamp the page number on every page at `(left, bottom)`.
    fn stamp_page_numbers(&self, format: &str, left: f64, bottom: f64) {
        self.for_each_page(self.page_number_rotation as f64, |i, _w, _h| {
            Some((
                render_number(format, self.starting_number + i, self.numbering_style),
                left,
                bottom,
            ))
        });
    }

    /// Draw a Helvetica text run on every bound page. `make` receives
    /// the 0-based page index plus the page width / height and returns
    /// the `(text, x, baseline_y)` to draw, or `None` to skip the page.
    /// A no-op when no document is bound.
    fn for_each_page(
        &self,
        rotation: f64,
        make: impl Fn(i32, f64, f64) -> Option<(String, f64, f64)>,
    ) {
        let Some(doc) = self.base.document.as_ref() else {
            return;
        };
        let count = doc.pages().count();
        for i in 0..count {
            let Ok(page) = doc.pages().get(i + 1) else {
                continue;
            };
            if let Some((text, x, y)) = make(i as i32, page.width(), page.height()) {
                if text.is_empty() {
                    continue;
                }
                let mut frag = TextFragment::with_text(text);
                frag.set_position(Position::new(x, y));
                frag.text_state_mut().set_font_size(STAMP_FONT_SIZE);
                TextBuilder::new(&page).append_text_rotated(&frag, rotation);
            }
        }
    }

    // ---- Properties ----

    /// Whether the output is size-optimised.
    pub fn optimize_size(&self) -> bool {
        self.optimize_size
    }
    /// Set whether the output is size-optimised.
    pub fn set_optimize_size(&mut self, value: bool) {
        self.optimize_size = value;
    }

    /// Whether the source document's security is preserved on save.
    pub fn keep_security(&self) -> bool {
        self.keep_security
    }
    /// Set whether the source document's security is preserved on save.
    pub fn set_keep_security(&mut self, value: bool) {
        self.keep_security = value;
    }

    /// The staged input file path. Carried, not read — the facade's
    /// input is the document [`bind_pdf`](crate::IFacade::bind_pdf)
    /// opened.
    pub fn input_file(&self) -> &str {
        &self.input_file
    }
    /// Set the input file path.
    pub fn set_input_file(&mut self, value: impl Into<String>) {
        self.input_file = value.into();
    }

    /// The staged output file path. Carried, not read — the output is
    /// the path `Save` is given.
    pub fn output_file(&self) -> &str {
        &self.output_file
    }
    /// Set the output file path.
    pub fn set_output_file(&mut self, value: impl Into<String>) {
        self.output_file = value.into();
    }

    /// The page-number rotation, in degrees.
    pub fn page_number_rotation(&self) -> f32 {
        self.page_number_rotation
    }
    /// Set the page-number rotation, in degrees.
    pub fn set_page_number_rotation(&mut self, value: f32) {
        self.page_number_rotation = value;
    }

    /// Set the target format the document is converted to on save.
    pub fn set_convert_to(&mut self, value: PdfFormat) {
        self.convert_to = value;
    }

    /// The first bound page's height (US-Letter default when unbound).
    pub fn page_height(&self) -> f32 {
        self.first_page_dim(|p| p.height() as f32, LETTER_HEIGHT)
    }
    /// The first bound page's width (US-Letter default when unbound).
    pub fn page_width(&self) -> f32 {
        self.first_page_dim(|p| p.width() as f32, LETTER_WIDTH)
    }

    fn first_page_dim(
        &self,
        f: impl Fn(&crate::public::page::Page) -> f32,
        fallback: f32,
    ) -> f32 {
        self.base
            .document
            .as_ref()
            .and_then(|d| d.pages().get(1).ok())
            .map_or(fallback, |p| f(&p))
    }

    /// The starting page number for page-number stamps.
    pub fn starting_number(&self) -> i32 {
        self.starting_number
    }
    /// Set the starting page number for page-number stamps.
    pub fn set_starting_number(&mut self, value: i32) {
        self.starting_number = value;
    }

    /// The numbering style for page-number stamps.
    pub fn numbering_style(&self) -> NumberingStyle {
        self.numbering_style
    }
    /// Set the numbering style for page-number stamps.
    pub fn set_numbering_style(&mut self, value: NumberingStyle) {
        self.numbering_style = value;
    }

    /// The stamp identifier.
    pub fn stamp_id(&self) -> i32 {
        self.stamp_id
    }
    /// Set the stamp identifier.
    pub fn set_stamp_id(&mut self, value: i32) {
        self.stamp_id = value;
    }
}

/// Format a page-number stamp: replace `#` in `format` with `n` (or, if
/// the format contains no `#`, append the number).
/// `format` with its `#` replaced by `n` written in `style` (§12.4.2),
/// or the number appended when the format names no place for it.
fn render_number(format: &str, n: i32, style: NumberingStyle) -> String {
    let number = style.format_number(n);
    if format.contains('#') {
        format.replace('#', &number)
    } else {
        format!("{format}{number}")
    }
}

impl_facade!(PdfFileStamp);
impl_saveable_facade!(PdfFileStamp);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::i_saveable_facade::ISaveableFacade;

    fn hello_world_pdf() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/text_extractor/hello_world.pdf")
    }

    #[test]
    fn position_constants() {
        assert_eq!(PdfFileStamp::POS_BOTTOM_MIDDLE, 0);
        assert_eq!(PdfFileStamp::POS_BOTTOM_RIGHT, 1);
        assert_eq!(PdfFileStamp::POS_UPPER_RIGHT, 2);
        assert_eq!(PdfFileStamp::POS_SIDES_RIGHT, 3);
        assert_eq!(PdfFileStamp::POS_UPPER_MIDDLE, 4);
        assert_eq!(PdfFileStamp::POS_BOTTOM_LEFT, 5);
        assert_eq!(PdfFileStamp::POS_SIDES_LEFT, 6);
        assert_eq!(PdfFileStamp::POS_UPPER_LEFT, 7);
    }

    #[test]
    fn defaults() {
        let st = PdfFileStamp::new();
        assert!(!st.optimize_size());
        assert!(!st.keep_security());
        assert!(st.input_file().is_empty());
        assert!(st.output_file().is_empty());
        assert_eq!(st.page_number_rotation(), 0.0);
        assert_eq!(st.starting_number(), 1);
        assert_eq!(st.stamp_id(), 0);
        assert_eq!(st.page_height(), 792.0);
        assert_eq!(st.page_width(), 612.0);
        assert_eq!(st.numbering_style(), NumberingStyle::NumeralsArabic);
    }

    #[test]
    fn property_roundtrip() {
        let mut st = PdfFileStamp::new();
        st.set_optimize_size(true);
        st.set_keep_security(true);
        st.set_input_file("in.pdf");
        st.set_output_file("out.pdf");
        st.set_page_number_rotation(90.0);
        st.set_starting_number(5);
        st.set_stamp_id(42);
        st.set_convert_to(PdfFormat::PDF_A_1B);
        st.set_numbering_style(NumberingStyle::NumeralsRomanUppercase);

        assert!(st.optimize_size());
        assert!(st.keep_security());
        assert_eq!(st.input_file(), "in.pdf");
        assert_eq!(st.output_file(), "out.pdf");
        assert_eq!(st.page_number_rotation(), 90.0);
        assert_eq!(st.starting_number(), 5);
        assert_eq!(st.stamp_id(), 42);
        assert_eq!(st.numbering_style(), NumberingStyle::NumeralsRomanUppercase);
    }

    #[test]
    fn stamp_stubs_do_not_panic_and_save_passes_through() {
        let mut st = PdfFileStamp::with_document(Document::open(hello_world_pdf()).unwrap());
        st.add_page_number("Page #");
        st.add_page_number_from("Page #", 1);
        st.add_page_number_at("Page #", 10.0, 20.0);
        st.add_page_number_in_rect("Page #", 1, 0.0, 0.0, 100.0, 20.0);
        st.add_header("Header", 36.0);
        st.add_header_with_margins("Header", 36.0, 36.0, 36.0);
        st.add_footer("Footer", 36.0);
        st.add_footer_with_margins("Footer", 36.0, 36.0, 36.0);

        let out = std::env::temp_dir().join("aspose_rust_filestamp_save.pdf");
        st.save(&out).unwrap();
        assert!(out.exists());
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn file_ctor_binds() {
        let out = std::env::temp_dir()
            .join("aspose_rust_filestamp_filector.pdf")
            .to_string_lossy()
            .into_owned();
        let st = PdfFileStamp::with_files_and_security(hello_world_pdf(), &out, true).unwrap();
        assert_eq!(st.output_file(), out);
        assert!(st.keep_security());
        st.save(&out).unwrap();
        assert!(Path::new(&out).exists());
        let _ = std::fs::remove_file(&out);
    }
}
