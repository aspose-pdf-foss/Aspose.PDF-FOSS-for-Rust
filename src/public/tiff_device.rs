//! `Aspose.Pdf.Devices.TiffDevice` — concrete [`DocumentDevice`]
//! that renders a single [`Page`] or a (possibly multi-page)
//! [`Document`] range to a TIFF byte stream via the foundation
//! `internal::tiff_encoder` primitive.
//!
//! v1 Rust surface ships the 24bpp RGB photometric path with
//! [`TiffSettings::compression`](crate::TiffSettings::compression)-
//! driven strip compression: `LZW` / `Rle` / `None` map directly
//! to the matching `internal::tiff_encoder` codec.
//!
//! [`ColorDepth::Format1bpp`](crate::ColorDepth) selects a
//! BlackIsZero 1bpp encoding shape (the rendered RGBA8 buffer
//! is luma-thresholded at 128 inside the foundation encoder).
//! When 1bpp is selected `Ccitt3` / `Ccitt4` are honoured and
//! map to `internal::tiff_encoder::Compression::Ccitt3` /
//! `Ccitt4`, which dispatch to `internal::ccitt::encode_group3` /
//! `encode_group4`. The Rust track lands CCITT encoding ahead
//! of the cpp / csharp peers (both raise `NotSupported` on
//! `CompressionType::CCITT3/4`).
//!
//! Mixing `Ccitt3` / `Ccitt4` with any depth other than
//! `Format1bpp` falls back to uncompressed strips so RGB /
//! palette workflows continue to produce valid TIFF bytes
//! when the caller forgets to set `Depth = Format1bpp`.
//!
//! The remaining `TiffSettings` knobs (`Shape`, `Brightness`,
//! `CoordinateType`) and the canonical surface's
//! `RenderingOptions` / `FormPresentationMode` are stored as
//! configuration on the carrier but not yet honoured at encode
//! time; the cluster lands when the rendering-options-on-devices
//! semantics open.
//!
//! Beat 11 added the [`IIndexBitmapConverter`]-taking ctors —
//! four overloads paralleling the `with_settings*` family. The
//! converter is stored on the device but not invoked during v1
//! 24-bpp RGB / 1bpp encoding; future beats wire it into the
//! indexed-mode output path alongside
//! [`ColorDepth::Format4bpp`](crate::ColorDepth) /
//! [`ColorDepth::Format8bpp`](crate::ColorDepth) honouring.
//!
//! [`TiffDevice::binarize_bradley`] reads a TIFF byte stream,
//! collapses RGBA to ITU-R BT.601 luminance, runs Bradley/Roth
//! adaptive thresholding (2007) with an integral-image O(1)
//! window mean, and emits the binarised result back as a TIFF
//! byte stream. Output shape mirrors both peer libraries
//! (RGBA8 strips with every pixel `(0,0,0,255)` or
//! `(255,255,255,255)`).
//!
//! Like the rest of the device family, the rasteriser produces
//! RGBA8; TIFF's `Rgb` photometric drops alpha, so partial-alpha
//! source pixels are composited onto an opaque white background
//! before encoding. Multi-page output emits one TIFF IFD per
//! requested page; `internal::tiff_encoder` handles IFD chaining.

use std::sync::Arc;

use super::color_depth::ColorDepth;
use super::compression_type::CompressionType;
use super::device::{Device, DocumentDevice, PageDevice};
use super::document::Document;
use super::error::Error;
use super::form_presentation_mode::FormPresentationMode;
use super::i_index_bitmap_converter::IIndexBitmapConverter;
use super::image_device::ImageDeviceState;
use super::page::Page;
use super::page_size::PageSize;
use super::rendering_options::RenderingOptions;
use super::resolution::Resolution;
use super::tiff_settings::TiffSettings;

use crate::internal::page_renderer;
use crate::internal::tiff_decoder;
use crate::internal::tiff_encoder;

/// Concrete [`DocumentDevice`] emitting (multi-page) TIFF bytes.
#[derive(Clone)]
pub struct TiffDevice {
    state: ImageDeviceState,
    settings: TiffSettings,
    converter: Option<Arc<dyn IIndexBitmapConverter>>,
}

impl std::fmt::Debug for TiffDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TiffDevice")
            .field("state", &self.state)
            .field("settings", &self.settings)
            .field(
                "converter",
                &self.converter.as_ref().map(|_| "<dyn IIndexBitmapConverter>"),
            )
            .finish()
    }
}

impl TiffDevice {
    /// Default-construct — render at implementation-default DPI (72)
    /// with default [`TiffSettings`] and no converter.
    pub fn new() -> Self {
        Self {
            state: ImageDeviceState::empty(),
            settings: TiffSettings::new(),
            converter: None,
        }
    }

    /// Construct with an explicit output resolution.
    pub fn with_resolution(resolution: Resolution) -> Self {
        Self {
            state: ImageDeviceState::with_resolution(resolution),
            settings: TiffSettings::new(),
            converter: None,
        }
    }

    /// Construct with explicit pixel dimensions only. Same
    /// MediaBox-derived caveat as
    /// [`PngDevice::with_dimensions`](crate::PngDevice::with_dimensions).
    pub fn with_dimensions(width: i32, height: i32) -> Self {
        Self {
            state: ImageDeviceState::with_dimensions(width, height),
            settings: TiffSettings::new(),
            converter: None,
        }
    }

    /// Construct with explicit pixel dimensions and an output
    /// resolution.
    pub fn with_dimensions_and_resolution(
        width: i32,
        height: i32,
        resolution: Resolution,
    ) -> Self {
        Self {
            state: ImageDeviceState::with_dimensions_and_resolution(
                width, height, resolution,
            ),
            settings: TiffSettings::new(),
            converter: None,
        }
    }

    /// Construct with explicit [`TiffSettings`]; render at the
    /// implementation-default DPI.
    pub fn with_settings(settings: TiffSettings) -> Self {
        Self {
            state: ImageDeviceState::empty(),
            settings,
            converter: None,
        }
    }

    /// Construct with an explicit output resolution and
    /// [`TiffSettings`].
    pub fn with_resolution_and_settings(
        resolution: Resolution,
        settings: TiffSettings,
    ) -> Self {
        Self {
            state: ImageDeviceState::with_resolution(resolution),
            settings,
            converter: None,
        }
    }

    /// Construct with explicit pixel dimensions and
    /// [`TiffSettings`].
    pub fn with_dimensions_and_settings(
        width: i32,
        height: i32,
        settings: TiffSettings,
    ) -> Self {
        Self {
            state: ImageDeviceState::with_dimensions(width, height),
            settings,
            converter: None,
        }
    }

    /// Construct with explicit pixel dimensions, an output
    /// resolution, and [`TiffSettings`].
    pub fn with_dimensions_resolution_and_settings(
        width: i32,
        height: i32,
        resolution: Resolution,
        settings: TiffSettings,
    ) -> Self {
        Self {
            state: ImageDeviceState::with_dimensions_and_resolution(
                width, height, resolution,
            ),
            settings,
            converter: None,
        }
    }

    /// Construct with explicit [`TiffSettings`] and a pluggable
    /// [`IIndexBitmapConverter`] strategy.
    ///
    /// On the v1 Rust track the converter is stored on the
    /// device but not invoked at encode time (the encoder
    /// always emits 24-bpp RGB strips today). Future beats wire
    /// it into the indexed-mode output path alongside
    /// [`ColorDepth::Format1bpp`](crate::ColorDepth) honouring.
    pub fn with_settings_and_converter(
        settings: TiffSettings,
        converter: Arc<dyn IIndexBitmapConverter>,
    ) -> Self {
        Self {
            state: ImageDeviceState::empty(),
            settings,
            converter: Some(converter),
        }
    }

    /// Construct with an explicit output resolution,
    /// [`TiffSettings`], and converter. See
    /// [`with_settings_and_converter`](TiffDevice::with_settings_and_converter).
    pub fn with_resolution_settings_and_converter(
        resolution: Resolution,
        settings: TiffSettings,
        converter: Arc<dyn IIndexBitmapConverter>,
    ) -> Self {
        Self {
            state: ImageDeviceState::with_resolution(resolution),
            settings,
            converter: Some(converter),
        }
    }

    /// Construct with explicit pixel dimensions, [`TiffSettings`],
    /// and converter.
    pub fn with_dimensions_settings_and_converter(
        width: i32,
        height: i32,
        settings: TiffSettings,
        converter: Arc<dyn IIndexBitmapConverter>,
    ) -> Self {
        Self {
            state: ImageDeviceState::with_dimensions(width, height),
            settings,
            converter: Some(converter),
        }
    }

    /// Construct with explicit pixel dimensions, output
    /// resolution, [`TiffSettings`], and converter.
    pub fn with_dimensions_resolution_settings_and_converter(
        width: i32,
        height: i32,
        resolution: Resolution,
        settings: TiffSettings,
        converter: Arc<dyn IIndexBitmapConverter>,
    ) -> Self {
        Self {
            state: ImageDeviceState::with_dimensions_and_resolution(
                width, height, resolution,
            ),
            settings,
            converter: Some(converter),
        }
    }

    /// Construct with [`PageSize`]-derived pixel dimensions. The
    /// canonical algorithm is `(int)pageSize.Width` /
    /// `(int)pageSize.Height` — a truncating cast straight from
    /// the user-space-unit page geometry into the device's
    /// width / height pixel fields. Same MediaBox-derived caveat
    /// as [`TiffDevice::with_dimensions`]. Matches the C++ peer's
    /// per-ctor body (C# peer delegates and leaves Width/Height
    /// unset — peer disagreement, C++ peer wins per the Rust
    /// track's resolution rule).
    pub fn with_page_size(page_size: PageSize) -> Self {
        Self {
            state: ImageDeviceState::with_page_size(page_size),
            settings: TiffSettings::new(),
            converter: None,
        }
    }

    /// Construct with [`PageSize`]-derived pixel dimensions and an
    /// output resolution. See [`TiffDevice::with_page_size`].
    pub fn with_page_size_and_resolution(
        page_size: PageSize,
        resolution: Resolution,
    ) -> Self {
        Self {
            state: ImageDeviceState::with_page_size_and_resolution(
                page_size, resolution,
            ),
            settings: TiffSettings::new(),
            converter: None,
        }
    }

    /// Construct with [`PageSize`]-derived pixel dimensions and
    /// [`TiffSettings`]. See [`TiffDevice::with_page_size`].
    pub fn with_page_size_and_settings(
        page_size: PageSize,
        settings: TiffSettings,
    ) -> Self {
        Self {
            state: ImageDeviceState::with_page_size(page_size),
            settings,
            converter: None,
        }
    }

    /// Construct with [`PageSize`]-derived pixel dimensions, an
    /// output resolution, and [`TiffSettings`]. See
    /// [`TiffDevice::with_page_size`].
    pub fn with_page_size_resolution_and_settings(
        page_size: PageSize,
        resolution: Resolution,
        settings: TiffSettings,
    ) -> Self {
        Self {
            state: ImageDeviceState::with_page_size_and_resolution(
                page_size, resolution,
            ),
            settings,
            converter: None,
        }
    }

    /// Construct with [`PageSize`]-derived pixel dimensions,
    /// [`TiffSettings`], and a pluggable
    /// [`IIndexBitmapConverter`] strategy. See
    /// [`TiffDevice::with_page_size`] and
    /// [`TiffDevice::with_settings_and_converter`].
    pub fn with_page_size_settings_and_converter(
        page_size: PageSize,
        settings: TiffSettings,
        converter: Arc<dyn IIndexBitmapConverter>,
    ) -> Self {
        Self {
            state: ImageDeviceState::with_page_size(page_size),
            settings,
            converter: Some(converter),
        }
    }

    /// Construct with [`PageSize`]-derived pixel dimensions, an
    /// output resolution, [`TiffSettings`], and a converter. See
    /// [`TiffDevice::with_page_size`] and
    /// [`TiffDevice::with_settings_and_converter`].
    pub fn with_page_size_resolution_settings_and_converter(
        page_size: PageSize,
        resolution: Resolution,
        settings: TiffSettings,
        converter: Arc<dyn IIndexBitmapConverter>,
    ) -> Self {
        Self {
            state: ImageDeviceState::with_page_size_and_resolution(
                page_size, resolution,
            ),
            settings,
            converter: Some(converter),
        }
    }

    /// TIFF-specific encoding knobs configured at construction.
    /// Read-only on the canonical surface — set via the
    /// [`with_settings`](TiffDevice::with_settings) family of
    /// ctors. See [`TiffSettings`].
    pub fn settings(&self) -> TiffSettings {
        self.settings
    }

    /// Output resolution configured at construction, or `None`
    /// when the device renders at the implementation-default DPI.
    /// Canonical .NET surface carries `Resolution` as a property
    /// initialised to `new Resolution(72)`; the Rust target uses
    /// `Option<Resolution>` for the same default-fallback
    /// semantics, paralleling the [`ImageDevice`](crate::ImageDevice)
    /// hierarchy. (`TiffDevice` is a `DocumentDevice`, not an
    /// `ImageDevice`, in the canonical class graph — these
    /// accessors are declared directly on `TiffDevice`.)
    pub fn resolution(&self) -> Option<Resolution> {
        self.state.resolution
    }

    /// Output width in pixels as configured at construction
    /// (`0` when unspecified).
    pub fn width(&self) -> i32 {
        self.state.width
    }

    /// Output height in pixels as configured at construction
    /// (`0` when unspecified).
    pub fn height(&self) -> i32 {
        self.state.height
    }

    /// Shared reference to the device's [`RenderingOptions`] bag.
    /// Read-write on the canonical surface; in Rust the read path
    /// returns `&RenderingOptions` and mutation rides through
    /// [`rendering_options_mut`](TiffDevice::rendering_options_mut)
    /// (in-place) or
    /// [`set_rendering_options`](TiffDevice::set_rendering_options)
    /// (replace). The v1 raster path does not yet honour any of
    /// the bag's knobs at encode time — carrier round-trips so
    /// calling code can configure as planned and pick it up when
    /// the rendering-options-on-devices semantics land.
    pub fn rendering_options(&self) -> &RenderingOptions {
        &self.state.rendering_options
    }

    /// Mutable [`RenderingOptions`] borrow — enables in-place
    /// tweaks like
    /// `device.rendering_options_mut().set_analyze_fonts(true)`.
    pub fn rendering_options_mut(&mut self) -> &mut RenderingOptions {
        &mut self.state.rendering_options
    }

    /// Replace the device's [`RenderingOptions`] bag by value.
    pub fn set_rendering_options(&mut self, value: RenderingOptions) {
        self.state.rendering_options = value;
    }

    /// AcroForm-widget presentation policy. Read-write on the
    /// canonical surface; not yet honoured by the v1 raster path.
    pub fn form_presentation_mode(&self) -> FormPresentationMode {
        self.state.form_presentation_mode
    }

    /// Set [`form_presentation_mode`](TiffDevice::form_presentation_mode).
    pub fn set_form_presentation_mode(&mut self, value: FormPresentationMode) {
        self.state.form_presentation_mode = value;
    }
}

impl Default for TiffDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl Device for TiffDevice {}

impl PageDevice for TiffDevice {
    fn process(
        &self,
        page: &Page,
        output: &mut dyn std::io::Write,
    ) -> Result<(), Error> {
        let rendered = page_renderer::render(
            page.internal_doc_bytes(),
            page.require_persisted_leaf()?,
            self.state.effective_dpi(),
            false,
        )
        .map_err(|e| Error::InvalidPdf(e.to_string()))?;

        let composed = composite_alpha_on_white(
            &rendered.pixels,
            rendered.width,
            rendered.height,
        );

        let pages = [tiff_encoder::Page {
            width: rendered.width,
            height: rendered.height,
            rgba8: composed,
        }];
        let encoded = encode_pages(&pages, &self.settings, self.effective_dpi_u32())?;
        output.write_all(&encoded)?;
        Ok(())
    }
}

impl DocumentDevice for TiffDevice {
    fn process_document_range(
        &self,
        document: &Document,
        from_page: usize,
        to_page: usize,
        output: &mut dyn std::io::Write,
    ) -> Result<(), Error> {
        let count = document.pages().count();
        if from_page < 1 || from_page > count {
            return Err(Error::PageIndexOutOfRange { index: from_page, count });
        }
        if to_page < from_page || to_page > count {
            return Err(Error::PageIndexOutOfRange { index: to_page, count });
        }

        let dpi = self.state.effective_dpi();
        let mut pages = Vec::with_capacity(to_page - from_page + 1);
        for n in from_page..=to_page {
            let page = document.pages().get(n)?;
            let rendered = page_renderer::render(
                page.internal_doc_bytes(),
                page.require_persisted_leaf()?,
                dpi,
                false,
            )
            .map_err(|e| Error::InvalidPdf(e.to_string()))?;
            let composed = composite_alpha_on_white(
                &rendered.pixels,
                rendered.width,
                rendered.height,
            );
            if self.settings.skip_blank_pages() && is_blank(&composed) {
                continue;
            }
            pages.push(tiff_encoder::Page {
                width: rendered.width,
                height: rendered.height,
                rgba8: composed,
            });
        }
        if pages.is_empty() {
            // Every page in the range was blank. A TIFF with no image
            // at all is not a TIFF, so the first page is kept: the
            // output stays readable and still says the range was empty.
            let page = document.pages().get(from_page)?;
            let rendered = page_renderer::render(
                page.internal_doc_bytes(),
                page.require_persisted_leaf()?,
                dpi,
                false,
            )
            .map_err(|e| Error::InvalidPdf(e.to_string()))?;
            let composed = composite_alpha_on_white(
                &rendered.pixels,
                rendered.width,
                rendered.height,
            );
            pages.push(tiff_encoder::Page {
                width: rendered.width,
                height: rendered.height,
                rgba8: composed,
            });
        }

        let encoded = encode_pages(&pages, &self.settings, self.effective_dpi_u32())?;
        output.write_all(&encoded)?;
        Ok(())
    }
}

impl TiffDevice {
    fn effective_dpi_u32(&self) -> u32 {
        self.state.effective_dpi().round().max(1.0) as u32
    }

    /// Read a TIFF byte stream from `input`, run Bradley/Roth
    /// adaptive thresholding (2007), and write the binarised
    /// result back as a TIFF byte stream to `output`.
    ///
    /// `threshold` is the Bradley `t` parameter — the fraction
    /// the local-mean comparand is scaled down by before each
    /// per-pixel decision (`luma < local_mean * (1 - threshold)`
    /// → foreground). Typical inputs sit in `0.10..=0.20`.
    ///
    /// Output shape matches the peer C++ / C# libraries: every
    /// pixel becomes `(0, 0, 0, 255)` or `(255, 255, 255, 255)`
    /// and is re-encoded as a 24-bpp RGB TIFF via the foundation
    /// `tiff_encoder` pass-through path. Future revisions may
    /// switch this to a 1bpp BlackIsZero TIFF now that the
    /// bilevel encode path is wired.
    pub fn binarize_bradley(
        &self,
        input: &mut dyn std::io::Read,
        output: &mut dyn std::io::Write,
        threshold: f64,
    ) -> Result<(), Error> {
        let mut buf: Vec<u8> = Vec::new();
        input.read_to_end(&mut buf)?;
        if buf.is_empty() {
            return Err(Error::InvalidPdf(
                "binarize_bradley: input TIFF is empty".to_string(),
            ));
        }
        if !(0.0..=1.0).contains(&threshold) {
            return Err(Error::InvalidPdf(format!(
                "binarize_bradley: threshold {threshold} not in [0, 1]"
            )));
        }
        let decoded = tiff_decoder::decode(&buf)
            .map_err(|e| Error::InvalidPdf(format!("binarize_bradley: {e}")))?;
        let w = decoded.width;
        let h = decoded.height;
        let n = (w as usize) * (h as usize);

        // Collapse RGBA to BT.601 luminance (integer cross-
        // multiplication with round-half-up via the +500 nudge).
        let rgba = &decoded.pixels;
        let mut grey = vec![0u8; n];
        for i in 0..n {
            let r = rgba[i * 4] as u32;
            let g = rgba[i * 4 + 1] as u32;
            let b = rgba[i * 4 + 2] as u32;
            grey[i] = ((r * 299 + g * 587 + b * 114 + 500) / 1000) as u8;
        }

        // Delegate to the foundation integer-only Bradley
        // primitive (shared spec yaml — every target produces
        // byte-identical bilevel output). Window size = max(2,
        // w / 8) per Bradley & Roth 2007; threshold maps to
        // integer percent via round-half-up.
        let window_size = std::cmp::max(2u32, w / 8);
        let threshold_percent = (threshold * 100.0).round() as u32;
        let bilevel =
            crate::internal::binarize_bradley::apply(
                &grey, w, h, window_size, threshold_percent,
            )
            .map_err(|e| Error::InvalidPdf(format!("binarize_bradley: {e}")))?;

        // Expand bilevel MSB-first → RGBA8 (BLACK = (0,0,0,255);
        // WHITE = (255,255,255,255)) so the existing
        // tiff_encoder::encode RGBA8 path emits the output.
        let stride_in = w.div_ceil(8) as usize;
        let mut rgba_out = vec![0u8; n * 4];
        for y in 0..h as usize {
            for x in 0..w as usize {
                let byte_val = bilevel[y * stride_in + (x >> 3)];
                let bit = (byte_val >> (7 - (x & 7))) & 1;
                let v: u8 = if bit == 0 { 0 } else { 0xFF };
                let pi = (y * w as usize + x) * 4;
                rgba_out[pi] = v;
                rgba_out[pi + 1] = v;
                rgba_out[pi + 2] = v;
                rgba_out[pi + 3] = 0xFF;
            }
        }

        let page = tiff_encoder::Page {
            width: w,
            height: h,
            rgba8: rgba_out,
        };
        let encoded = tiff_encoder::encode(
            std::slice::from_ref(&page),
            &tiff_encoder::Options {
                compression: tiff_encoder::Compression::None,
                photometric: tiff_encoder::Photometric::Rgb,
                samples_per_pixel: 3,
                bits_per_sample: 8,
                dpi: self.effective_dpi_u32(),
                color_map: None,
            },
        )
        .map_err(|e| Error::InvalidPdf(e.to_string()))?;

        output.write_all(&encoded)?;
        Ok(())
    }
}

/// Selected (foundation Photometric, samples_per_pixel,
/// bits_per_sample) tuple for the encoder. Mirrors the peer
/// `ShapeFromDepth` helper.
#[derive(Debug, Clone, Copy)]
struct EncodingShape {
    photometric: tiff_encoder::Photometric,
    samples_per_pixel: u8,
    bits_per_sample: u8,
}

fn shape_from_depth(depth: ColorDepth) -> EncodingShape {
    match depth {
        ColorDepth::Format1bpp => EncodingShape {
            photometric: tiff_encoder::Photometric::BlackIsZero,
            samples_per_pixel: 1,
            bits_per_sample: 1,
        },
        // Default / 24bpp use the existing RGB pass-through.
        // 4bpp / 8bpp palette modes need the IIndexBitmapConverter
        // path which is not yet wired on the v1 Rust track —
        // fall back to RGB so the device still produces a valid
        // TIFF.
        _ => EncodingShape {
            photometric: tiff_encoder::Photometric::Rgb,
            samples_per_pixel: 3,
            bits_per_sample: 8,
        },
    }
}

/// Map the canonical [`CompressionType`] to the foundation
/// `tiff_encoder` codec. `Ccitt3` / `Ccitt4` are only legal
/// alongside 1bpp output; otherwise they degrade to
/// uncompressed strips so RGB workflows keep producing valid
/// TIFF when callers forget to set `Depth = Format1bpp`.
fn map_compression(
    c: CompressionType,
    depth: ColorDepth,
) -> tiff_encoder::Compression {
    match c {
        CompressionType::Lzw => tiff_encoder::Compression::Lzw,
        CompressionType::Rle => tiff_encoder::Compression::PackBits,
        CompressionType::None => tiff_encoder::Compression::None,
        CompressionType::Ccitt3 => {
            if depth == ColorDepth::Format1bpp {
                tiff_encoder::Compression::Ccitt3
            } else {
                tiff_encoder::Compression::None
            }
        }
        CompressionType::Ccitt4 => {
            if depth == ColorDepth::Format1bpp {
                tiff_encoder::Compression::Ccitt4
            } else {
                tiff_encoder::Compression::None
            }
        }
    }
}

fn encode_pages(
    pages: &[tiff_encoder::Page],
    settings: &TiffSettings,
    dpi: u32,
) -> Result<Vec<u8>, Error> {
    let shape = shape_from_depth(settings.depth());
    tiff_encoder::encode(
        pages,
        &tiff_encoder::Options {
            compression: map_compression(settings.compression(), settings.depth()),
            photometric: shape.photometric,
            samples_per_pixel: shape.samples_per_pixel,
            bits_per_sample: shape.bits_per_sample,
            dpi,
            color_map: None,
        },
    )
    .map_err(|e| Error::InvalidPdf(e.to_string()))
}

/// Whether a composited page took no ink at all: every pixel is the
/// white it was composited onto. A page with the faintest mark on it is
/// not blank — the test is exact, not a tolerance, because "nearly
/// blank" is a judgement this crate has no business making for a
/// caller.
fn is_blank(composed_rgba: &[u8]) -> bool {
    composed_rgba
        .chunks_exact(4)
        .all(|px| px[0] == 0xff && px[1] == 0xff && px[2] == 0xff)
}

fn composite_alpha_on_white(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let n = (width as usize) * (height as usize) * 4;
    debug_assert_eq!(rgba.len(), n);
    let mut out = Vec::with_capacity(n);
    for chunk in rgba.chunks_exact(4) {
        let r = chunk[0] as f64;
        let g = chunk[1] as f64;
        let b = chunk[2] as f64;
        let a = chunk[3] as f64 / 255.0;
        out.push(((r * a) + 255.0 * (1.0 - a)).round() as u8);
        out.push(((g * a) + 255.0 * (1.0 - a)).round() as u8);
        out.push(((b * a) + 255.0 * (1.0 - a)).round() as u8);
        out.push(0xff);
    }
    out
}
