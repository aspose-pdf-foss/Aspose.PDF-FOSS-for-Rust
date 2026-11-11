//! `Aspose.Pdf.Facades.PdfConverter` — a facade that renders a bound
//! document to multi-page TIFF or page-by-page images, per
//! `capabilities/pdf_converter.yaml`.
//!
//! Extends [`Facade`](crate::public::facade::Facade). `SaveAsTIFF` /
//! `SaveAsTIFFClassF` render the bound document to a real multi-page
//! TIFF through [`TiffDevice`]; `DoConvert` /
//! `HasNextImage` / `GetNextImage` walk the page range and render each
//! page to a real PNG through [`PngDevice`].
//!
//! Dropped (outside the shipped subset, mirroring the C++ reference):
//! the `Stream`-output overloads, the `System.Drawing.Imaging.ImageFormat`
//! `GetNextImage` overloads (the BCL image-format bridge cascades off the
//! dropped list), and `MergeImages` / `MergeImagesAsTiff` (the
//! `ImageMergeMode` + `Aspose.Pdf.Drawing.ImageFormat` merge surface is
//! not in the catalog). The string-output rendering surface and the
//! staged render properties ship.
//!
//! **Carried, not consulted — and the two go together.** This crate's
//! renderer rasterises a page's `/MediaBox`, always. So
//! [`coordinate_type`](PdfConverter::coordinate_type) cannot take
//! effect — choosing `CropBox` would have to crop the canvas, which
//! means a box parameter plumbed through `render` and every device that
//! calls it — and
//! [`show_hidden_areas`](PdfConverter::show_hidden_areas) is in the
//! odd position of describing what already happens: with the whole
//! media box rendered, everything outside the crop box is shown. The
//! flag's other setting is the one that is missing, and it is missing
//! for the same reason.

use crate::public::color_depth::ColorDepth;
use crate::public::compression_type::CompressionType;
use crate::public::device::{DocumentDevice, PageDevice};
use crate::public::document::Document;
use crate::public::error::Error;
use crate::public::form_presentation_mode::FormPresentationMode;
use crate::public::i_facade::{impl_facade, FacadeBase};
use crate::public::i_index_bitmap_converter::IIndexBitmapConverter;
use crate::public::page_coordinate_type::PageCoordinateType;
use crate::public::page_size::PageSize;
use crate::public::png_device::PngDevice;
use crate::public::rendering_options::RenderingOptions;
use crate::public::resolution::Resolution;
use crate::public::tiff_device::TiffDevice;
use crate::public::tiff_settings::TiffSettings;
use std::path::Path;
use std::sync::Arc;

/// A facade that renders a PDF to TIFF / per-page images.
#[derive(Debug)]
pub struct PdfConverter {
    base: FacadeBase,
    coordinate_type: PageCoordinateType,
    show_hidden_areas: bool,
    rendering_options: RenderingOptions,
    form_presentation_mode: FormPresentationMode,
    resolution: Resolution,
    start_page: i32,
    end_page: i32,
    password: String,
    user_password: String,
    // 1-based page cursor for HasNextImage / GetNextImage; 0 = not
    // started (set by DoConvert).
    next_image: i32,
}

impl Default for PdfConverter {
    fn default() -> Self {
        Self {
            base: FacadeBase::default(),
            coordinate_type: PageCoordinateType::CropBox,
            show_hidden_areas: false,
            rendering_options: RenderingOptions::default(),
            form_presentation_mode: FormPresentationMode::Production,
            // 150 DPI is the canonical default render resolution.
            resolution: Resolution::new(150).expect("150 dpi is valid"),
            start_page: 1,
            end_page: 0,
            password: String::new(),
            user_password: String::new(),
            next_image: 0,
        }
    }
}

impl PdfConverter {
    /// Construct an unbound facade.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct and bind to an in-memory [`Document`].
    pub fn with_document(document: Document) -> Self {
        Self { base: FacadeBase { document: Some(document) }, ..Self::default() }
    }

    // ---- Page-range helpers ----

    /// The number of pages in the bound document (0 when unbound).
    pub fn page_count(&self) -> i32 {
        self.base.document.as_ref().map_or(0, |d| d.pages().count() as i32)
    }

    /// The inclusive 1-based `[first, last]` render range, clamped to
    /// the bound document's page count.
    fn page_range(&self) -> (usize, usize) {
        let count = self.page_count() as usize;
        let first = if self.start_page >= 1 { self.start_page as usize } else { 1 };
        let last = if self.end_page > 0 {
            (self.end_page as usize).min(count)
        } else {
            count
        };
        (first, last)
    }

    // ---- Page-by-page image cursor ----

    /// Reset the page cursor to the first page of the render range.
    pub fn do_convert(&mut self) {
        self.next_image = self.page_range().0 as i32;
    }

    /// Whether another page is available from the cursor.
    pub fn has_next_image(&self) -> bool {
        if self.base.document.is_none() {
            return false;
        }
        let (first, last) = self.page_range();
        self.next_image >= first as i32 && self.next_image <= last as i32
    }

    /// Render the current page to a PNG file and advance the cursor.
    pub fn get_next_image(&mut self, output_file: impl AsRef<Path>) -> Result<(), Error> {
        let device = PngDevice::with_resolution(self.resolution);
        self.render_next_png(output_file, device)
    }

    /// Render the current page to a PNG file at `page_size` and advance
    /// the cursor.
    pub fn get_next_image_with_size(
        &mut self,
        output_file: impl AsRef<Path>,
        page_size: PageSize,
    ) -> Result<(), Error> {
        let device = PngDevice::with_page_size(page_size);
        self.render_next_png(output_file, device)
    }

    fn render_next_png(
        &mut self,
        output_file: impl AsRef<Path>,
        device: PngDevice,
    ) -> Result<(), Error> {
        let idx = self.next_image;
        {
            let doc = self.bound()?;
            let page = doc.pages().get(idx as usize)?;
            let mut f = std::fs::File::create(output_file)?;
            device.process(&page, &mut f)?;
        }
        self.next_image += 1;
        Ok(())
    }

    // ---- TIFF rendering (real, multi-page) ----

    /// Render the whole render range to a multi-page TIFF.
    pub fn save_as_tiff(&self, output_file: impl AsRef<Path>) -> Result<(), Error> {
        self.render_tiff(output_file, TiffDevice::with_resolution(self.resolution))
    }

    /// Render to a TIFF with an explicit `compression`.
    pub fn save_as_tiff_with_compression(
        &self,
        output_file: impl AsRef<Path>,
        compression: CompressionType,
    ) -> Result<(), Error> {
        let settings = TiffSettings::with_compression(compression);
        self.render_tiff(
            output_file,
            TiffDevice::with_resolution_and_settings(self.resolution, settings),
        )
    }

    /// Render to a TIFF at an explicit `x_resolution` / `y_resolution`.
    pub fn save_as_tiff_with_axes(
        &self,
        output_file: impl AsRef<Path>,
        x_resolution: i32,
        y_resolution: i32,
    ) -> Result<(), Error> {
        let res = Resolution::with_axes(x_resolution, y_resolution)?;
        self.render_tiff(output_file, TiffDevice::with_resolution(res))
    }

    /// Render to a TIFF sized to `page_size`.
    pub fn save_as_tiff_with_page_size(
        &self,
        output_file: impl AsRef<Path>,
        page_size: PageSize,
    ) -> Result<(), Error> {
        self.render_tiff(output_file, TiffDevice::with_page_size(page_size))
    }

    /// Render to a TIFF sized to `page_size` with explicit `settings`.
    pub fn save_as_tiff_with_page_size_and_settings(
        &self,
        output_file: impl AsRef<Path>,
        page_size: PageSize,
        settings: TiffSettings,
    ) -> Result<(), Error> {
        self.render_tiff(
            output_file,
            TiffDevice::with_page_size_and_settings(page_size, settings),
        )
    }

    /// Render to a TIFF at `x_resolution` / `y_resolution` with explicit
    /// `compression`.
    pub fn save_as_tiff_with_axes_and_compression(
        &self,
        output_file: impl AsRef<Path>,
        x_resolution: i32,
        y_resolution: i32,
        compression: CompressionType,
    ) -> Result<(), Error> {
        let res = Resolution::with_axes(x_resolution, y_resolution)?;
        let settings = TiffSettings::with_compression(compression);
        self.render_tiff(
            output_file,
            TiffDevice::with_resolution_and_settings(res, settings),
        )
    }

    /// Render to a TIFF at `x_resolution` / `y_resolution` with explicit
    /// `settings`.
    pub fn save_as_tiff_with_axes_and_settings(
        &self,
        output_file: impl AsRef<Path>,
        x_resolution: i32,
        y_resolution: i32,
        settings: TiffSettings,
    ) -> Result<(), Error> {
        let res = Resolution::with_axes(x_resolution, y_resolution)?;
        self.render_tiff(
            output_file,
            TiffDevice::with_resolution_and_settings(res, settings),
        )
    }

    /// Render to a TIFF at `x_resolution` / `y_resolution` with explicit
    /// `settings` and an index-bitmap `converter`.
    pub fn save_as_tiff_with_axes_settings_and_converter(
        &self,
        output_file: impl AsRef<Path>,
        x_resolution: i32,
        y_resolution: i32,
        settings: TiffSettings,
        converter: Arc<dyn IIndexBitmapConverter>,
    ) -> Result<(), Error> {
        let res = Resolution::with_axes(x_resolution, y_resolution)?;
        self.render_tiff(
            output_file,
            TiffDevice::with_resolution_settings_and_converter(res, settings, converter),
        )
    }

    /// Render to a TIFF with explicit `settings`.
    pub fn save_as_tiff_with_settings(
        &self,
        output_file: impl AsRef<Path>,
        settings: TiffSettings,
    ) -> Result<(), Error> {
        self.render_tiff(
            output_file,
            TiffDevice::with_resolution_and_settings(self.resolution, settings),
        )
    }

    /// Render to a TIFF with explicit `settings` and an index-bitmap
    /// `converter`.
    pub fn save_as_tiff_with_settings_and_converter(
        &self,
        output_file: impl AsRef<Path>,
        settings: TiffSettings,
        converter: Arc<dyn IIndexBitmapConverter>,
    ) -> Result<(), Error> {
        self.render_tiff(
            output_file,
            TiffDevice::with_resolution_settings_and_converter(
                self.resolution,
                settings,
                converter,
            ),
        )
    }

    /// Render to a fax-profile (Class F, CCITT-4 bilevel) TIFF.
    pub fn save_as_tiff_class_f(&self, output_file: impl AsRef<Path>) -> Result<(), Error> {
        self.render_tiff(
            output_file,
            TiffDevice::with_resolution_and_settings(self.resolution, Self::class_f_settings()),
        )
    }

    /// Render a Class F TIFF at `x_resolution` / `y_resolution`.
    pub fn save_as_tiff_class_f_with_axes(
        &self,
        output_file: impl AsRef<Path>,
        x_resolution: i32,
        y_resolution: i32,
    ) -> Result<(), Error> {
        let res = Resolution::with_axes(x_resolution, y_resolution)?;
        self.render_tiff(
            output_file,
            TiffDevice::with_resolution_and_settings(res, Self::class_f_settings()),
        )
    }

    /// Render a Class F TIFF sized to `page_size`.
    pub fn save_as_tiff_class_f_with_page_size(
        &self,
        output_file: impl AsRef<Path>,
        page_size: PageSize,
    ) -> Result<(), Error> {
        self.render_tiff(
            output_file,
            TiffDevice::with_page_size_and_settings(page_size, Self::class_f_settings()),
        )
    }

    fn class_f_settings() -> TiffSettings {
        let mut settings = TiffSettings::with_compression(CompressionType::Ccitt4);
        settings.set_depth(ColorDepth::Format1bpp);
        settings
    }

    fn render_tiff(
        &self,
        output_file: impl AsRef<Path>,
        device: TiffDevice,
    ) -> Result<(), Error> {
        let doc = self.bound()?;
        let (first, last) = self.page_range();
        let mut f = std::fs::File::create(output_file)?;
        device.process_document_range(doc, first, last, &mut f)
    }

    fn bound(&self) -> Result<&Document, Error> {
        self.base
            .document
            .as_ref()
            .ok_or_else(|| Error::InvalidPdf("facade has no bound document".into()))
    }

    // ---- Properties ----

    /// The page box used for rendering coordinates.
    pub fn coordinate_type(&self) -> PageCoordinateType {
        self.coordinate_type
    }
    /// Set the page box used for rendering coordinates.
    pub fn set_coordinate_type(&mut self, value: PageCoordinateType) {
        self.coordinate_type = value;
    }

    /// Whether hidden areas are rendered.
    pub fn show_hidden_areas(&self) -> bool {
        self.show_hidden_areas
    }
    /// Set whether hidden areas are rendered.
    pub fn set_show_hidden_areas(&mut self, value: bool) {
        self.show_hidden_areas = value;
    }

    /// The render options.
    pub fn rendering_options(&self) -> &RenderingOptions {
        &self.rendering_options
    }
    /// Set the render options.
    pub fn set_rendering_options(&mut self, value: RenderingOptions) {
        self.rendering_options = value;
    }

    /// The form presentation mode.
    pub fn form_presentation_mode(&self) -> FormPresentationMode {
        self.form_presentation_mode
    }
    /// Set the form presentation mode.
    pub fn set_form_presentation_mode(&mut self, value: FormPresentationMode) {
        self.form_presentation_mode = value;
    }

    /// The render resolution.
    pub fn resolution(&self) -> &Resolution {
        &self.resolution
    }
    /// Set the render resolution.
    pub fn set_resolution(&mut self, value: Resolution) {
        self.resolution = value;
    }

    /// The first page of the render range (1-based).
    pub fn start_page(&self) -> i32 {
        self.start_page
    }
    /// Set the first page of the render range (1-based).
    pub fn set_start_page(&mut self, value: i32) {
        self.start_page = value;
    }

    /// The last page of the render range (0 = last page).
    pub fn end_page(&self) -> i32 {
        self.end_page
    }
    /// Set the last page of the render range (0 = last page).
    pub fn set_end_page(&mut self, value: i32) {
        self.end_page = value;
    }

    /// The owner password used to open the bound document.
    ///
    /// Carried, not used: a converter is bound to a [`Document`] that is
    /// already open, and a document that needed a password was
    /// authenticated when it was opened. Use
    /// [`Document::open_with_password`] for that.
    pub fn password(&self) -> &str {
        &self.password
    }
    /// Set the owner password.
    pub fn set_password(&mut self, value: impl Into<String>) {
        self.password = value.into();
    }

    /// The user password used to open the bound document.
    pub fn user_password(&self) -> &str {
        &self.user_password
    }
    /// Set the user password.
    pub fn set_user_password(&mut self, value: impl Into<String>) {
        self.user_password = value.into();
    }
}

impl_facade!(PdfConverter);

#[cfg(test)]
mod tests {
    use super::*;

    fn hello_world_pdf() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/text_extractor/hello_world.pdf")
    }

    fn bound() -> PdfConverter {
        PdfConverter::with_document(Document::open(hello_world_pdf()).unwrap())
    }

    fn head(path: &Path, n: usize) -> Vec<u8> {
        let bytes = std::fs::read(path).unwrap();
        bytes[..n.min(bytes.len())].to_vec()
    }

    #[test]
    fn page_count_real() {
        let conv = bound();
        let expected = Document::open(hello_world_pdf()).unwrap().pages().count() as i32;
        assert_eq!(conv.page_count(), expected);
        assert!(conv.page_count() > 0);
    }

    #[test]
    fn unbound_page_count_zero() {
        let conv = PdfConverter::new();
        assert_eq!(conv.page_count(), 0);
        assert!(!conv.has_next_image());
    }

    #[test]
    fn save_as_tiff_renders_real_file() {
        let conv = bound();
        let out = std::env::temp_dir().join("aspose_rust_conv_all.tiff");
        conv.save_as_tiff(&out).unwrap();
        assert!(out.exists());
        assert!(std::fs::metadata(&out).unwrap().len() > 0);
        let h = head(&out, 2);
        assert!(h == b"II" || h == b"MM", "head={h:?}");
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn save_as_tiff_with_compression() {
        let conv = bound();
        let out = std::env::temp_dir().join("aspose_rust_conv_lzw.tiff");
        conv.save_as_tiff_with_compression(&out, CompressionType::Lzw).unwrap();
        assert!(out.exists());
        assert!(std::fs::metadata(&out).unwrap().len() > 0);
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn get_next_image_renders_png() {
        let mut conv = bound();
        let total = conv.page_count();
        conv.do_convert();
        let mut rendered = 0;
        while conv.has_next_image() {
            let out = std::env::temp_dir()
                .join(format!("aspose_rust_conv_page{rendered}.png"));
            conv.get_next_image(&out).unwrap();
            assert!(out.exists());
            let h = head(&out, 4);
            assert_eq!(h[0], 0x89);
            assert_eq!(&h[1..4], b"PNG");
            let _ = std::fs::remove_file(&out);
            rendered += 1;
        }
        assert_eq!(rendered, total);
    }

    #[test]
    fn property_roundtrip() {
        let mut conv = PdfConverter::new();
        conv.set_start_page(2);
        conv.set_end_page(5);
        conv.set_show_hidden_areas(true);
        conv.set_coordinate_type(PageCoordinateType::MediaBox);
        conv.set_password("pw");
        conv.set_user_password("upw");
        conv.set_resolution(Resolution::new(300).unwrap());

        assert_eq!(conv.start_page(), 2);
        assert_eq!(conv.end_page(), 5);
        assert!(conv.show_hidden_areas());
        assert_eq!(conv.coordinate_type(), PageCoordinateType::MediaBox);
        assert_eq!(conv.password(), "pw");
        assert_eq!(conv.user_password(), "upw");
        assert_eq!(conv.resolution().x(), 300);
    }
}
