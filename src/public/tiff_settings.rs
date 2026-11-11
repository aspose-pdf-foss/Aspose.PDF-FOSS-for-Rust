//! `Aspose.Pdf.Devices.TiffSettings` — TIFF-specific encoding
//! knobs consumed by [`TiffDevice`](crate::TiffDevice).
//!
//! Public-API value bag per `capabilities/tiff_settings.yaml`:
//! 9 ctor shapes covering combinations of [`Margins`],
//! [`SkipBlankPages`](TiffSettings::skip_blank_pages),
//! [`Compression`](TiffSettings::compression) (TIFF compression
//! scheme), [`Depth`](TiffSettings::depth) (output colour
//! depth), and [`Shape`](TiffSettings::shape) (orientation
//! hint), plus six read-write properties + the read-only
//! `Margins` carrier.
//!
//! Default-constructed values match the canonical Aspose.PDF
//! 26.4.0 surface (`Compression = LZW`, `Depth = Default`,
//! `Shape = None`, `Brightness = 0.5`, `CoordinateType =
//! CropBox`, `SkipBlankPages = false`, `Margins = zero on all
//! four sides`).
//!
//! On the v1 Rust track, [`TiffDevice`](crate::TiffDevice)
//! honours `Compression` (LZW / RLE / None — see
//! [`CompressionType`] doc for the CCITT fallback), `Depth`, and
//! [`skip_blank_pages`](TiffSettings::skip_blank_pages): a page that
//! took no ink is left out of a multi-page TIFF, with "no ink" meaning
//! every pixel is the white it was composited onto. The last page
//! standing is kept even when it is blank, because a TIFF with no image
//! in it is not a TIFF.
//!
//! **Carried, not consulted:** `shape`, `brightness` and
//! `coordinate_type`. The first two need stages the encoder does not
//! have — a page-shape transform and a brightness pass — and the third
//! needs the renderer to rasterise a box other than the media box,
//! which is the same contract question
//! [`RenderingOptions`](crate::RenderingOptions) names.

use super::color_depth::ColorDepth;
use super::compression_type::CompressionType;
use super::margins::Margins;
use super::page_coordinate_type::PageCoordinateType;
use super::shape_type::ShapeType;

/// TIFF-output configuration carrier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TiffSettings {
    margins: Margins,
    skip_blank_pages: bool,
    compression: CompressionType,
    depth: ColorDepth,
    shape: ShapeType,
    brightness: f32,
    coordinate_type: PageCoordinateType,
}

impl TiffSettings {
    /// Default-construct with the canonical Aspose.PDF baseline
    /// (LZW compression, default depth, no shape override,
    /// brightness 0.5, CropBox coordinate system, no skip of
    /// blank pages, zero margins).
    pub fn new() -> Self {
        Self {
            margins: Margins::new(),
            skip_blank_pages: false,
            compression: CompressionType::Lzw,
            depth: ColorDepth::Default,
            shape: ShapeType::None,
            brightness: 0.5,
            coordinate_type: PageCoordinateType::CropBox,
        }
    }

    /// Construct overriding only the [`Shape`](TiffSettings::shape).
    pub fn with_shape(shape: ShapeType) -> Self {
        Self { shape, ..Self::new() }
    }

    /// Construct overriding only the
    /// [`Compression`](TiffSettings::compression).
    pub fn with_compression(compression: CompressionType) -> Self {
        Self { compression, ..Self::new() }
    }

    /// Construct overriding only the
    /// [`Depth`](TiffSettings::depth).
    pub fn with_depth(depth: ColorDepth) -> Self {
        Self { depth, ..Self::new() }
    }

    /// Construct overriding only the
    /// [`Margins`](TiffSettings::margins).
    pub fn with_margins(margins: Margins) -> Self {
        Self { margins, ..Self::new() }
    }

    /// Construct overriding only the
    /// [`SkipBlankPages`](TiffSettings::skip_blank_pages) flag.
    pub fn with_skip_blank_pages(skip_blank_pages: bool) -> Self {
        Self { skip_blank_pages, ..Self::new() }
    }

    /// Construct overriding [`Compression`](TiffSettings::compression),
    /// [`Depth`](TiffSettings::depth), and
    /// [`Margins`](TiffSettings::margins) together.
    pub fn with_compression_depth_margins(
        compression: CompressionType,
        depth: ColorDepth,
        margins: Margins,
    ) -> Self {
        Self { compression, depth, margins, ..Self::new() }
    }

    /// Construct overriding `Compression` + `Depth` + `Margins`
    /// + [`SkipBlankPages`](TiffSettings::skip_blank_pages).
    pub fn with_compression_depth_margins_skip(
        compression: CompressionType,
        depth: ColorDepth,
        margins: Margins,
        skip_blank_pages: bool,
    ) -> Self {
        Self {
            compression,
            depth,
            margins,
            skip_blank_pages,
            ..Self::new()
        }
    }

    /// Construct overriding `Compression` + `Depth` + `Margins`
    /// + `SkipBlankPages` + [`Shape`](TiffSettings::shape).
    pub fn with_compression_depth_margins_skip_shape(
        compression: CompressionType,
        depth: ColorDepth,
        margins: Margins,
        skip_blank_pages: bool,
        shape: ShapeType,
    ) -> Self {
        Self {
            compression,
            depth,
            margins,
            skip_blank_pages,
            shape,
            ..Self::new()
        }
    }

    /// Page margins, in pixels. Carried, not applied — see
    /// [`Margins`] for why the canvas is the page's own
    /// size.
    /// Read-only — set via the appropriate constructor.
    pub fn margins(&self) -> Margins {
        self.margins
    }

    /// `true` when blank pages are elided from the output TIFF.
    pub fn skip_blank_pages(&self) -> bool {
        self.skip_blank_pages
    }

    /// Replace the [`skip_blank_pages`](TiffSettings::skip_blank_pages)
    /// flag.
    pub fn set_skip_blank_pages(&mut self, value: bool) {
        self.skip_blank_pages = value;
    }

    /// TIFF compression scheme. See [`CompressionType`].
    pub fn compression(&self) -> CompressionType {
        self.compression
    }

    /// Replace the [`compression`](TiffSettings::compression).
    pub fn set_compression(&mut self, value: CompressionType) {
        self.compression = value;
    }

    /// Output colour depth. See [`ColorDepth`].
    pub fn depth(&self) -> ColorDepth {
        self.depth
    }

    /// Replace the [`depth`](TiffSettings::depth).
    pub fn set_depth(&mut self, value: ColorDepth) {
        self.depth = value;
    }

    /// Page-orientation hint. See [`ShapeType`].
    pub fn shape(&self) -> ShapeType {
        self.shape
    }

    /// Replace the [`shape`](TiffSettings::shape).
    pub fn set_shape(&mut self, value: ShapeType) {
        self.shape = value;
    }

    /// Output brightness in `[0.0, 1.0]`. Canonical default
    /// 0.5.
    pub fn brightness(&self) -> f32 {
        self.brightness
    }

    /// Replace the [`brightness`](TiffSettings::brightness).
    pub fn set_brightness(&mut self, value: f32) {
        self.brightness = value;
    }

    /// Page-box selector for the rendering pipeline. See
    /// [`PageCoordinateType`].
    pub fn coordinate_type(&self) -> PageCoordinateType {
        self.coordinate_type
    }

    /// Replace the [`coordinate_type`](TiffSettings::coordinate_type).
    pub fn set_coordinate_type(&mut self, value: PageCoordinateType) {
        self.coordinate_type = value;
    }
}

impl Default for TiffSettings {
    fn default() -> Self {
        Self::new()
    }
}
