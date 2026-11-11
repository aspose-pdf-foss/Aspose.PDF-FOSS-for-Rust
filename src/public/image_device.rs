//! `Aspose.Pdf.Devices.ImageDevice` — abstract intermediate base
//! for raster-image rendering devices ([`PngDevice`](crate::PngDevice) today; more
//! formats in later beats).
//!
//! Rust expresses the canonical `ImageDevice` abstract class as
//! a trait surfacing the read-only geometry accessors
//! (`resolution` / `width` / `height`) common to every raster
//! device, plus the shared default DPI used when the resolution
//! is unset, plus a [`get_bitmap`](ImageDevice::get_bitmap)
//! default method that rasterises a [`Page`] through the
//! `internal::page_renderer` primitive and returns a flat
//! [`BitmapInfo`]. The canonical type also carries
//! `CoordinateType` / `RenderingOptions` / `FormPresentationMode`;
//! those are explicitly dropped from the v1 Rust surface via
//! `Feedstock/PDF/API/targets/rust/drops.yaml` (cascades through
//! `RenderingOptions` / `FormPresentationMode` /
//! `PageCoordinateType` which are not yet shipped on the Rust
//! track). [`PageSize`](crate::PageSize)-driven dimensions are
//! supported via twin ctors on each concrete leaf
//! ([`PngDevice`](crate::PngDevice),
//! [`JpegDevice`](crate::JpegDevice),
//! [`BmpDevice`](crate::BmpDevice),
//! [`TiffDevice`](crate::TiffDevice)) — the canonical algorithm
//! is a truncating cast of
//! [`PageSize::width`](crate::PageSize::width) /
//! [`PageSize::height`](crate::PageSize::height) (user-space
//! units) straight into the `ImageDeviceState` width/height
//! pixel fields — same shape the C++ and C# peers ship, and
//! what the .NET surface's `(int)pageSize.Width` cast produces.

use super::bitmap_info::BitmapInfo;
use super::pixel_format::PixelFormat;
use super::device::PageDevice;
use super::error::Error;
use super::form_presentation_mode::FormPresentationMode;
use super::page::Page;
use super::page_coordinate_type::PageCoordinateType;
use super::rendering_options::RenderingOptions;
use super::resolution::Resolution;

use crate::internal::page_renderer;

/// Default DPI applied when no [`Resolution`] is supplied — the
/// 1:1 user-space-to-device-pixel ratio used as the
/// `page_renderer` freeze-gate oracle baseline.
pub(crate) const DEFAULT_DPI: f64 = 72.0;

/// Abstract raster-image rendering device. Concrete
/// implementors ([`PngDevice`](crate::PngDevice)) inherit
/// `process` from [`PageDevice`] and add format-specific
/// configuration (e.g. `transparent_background` on `PngDevice`).
pub trait ImageDevice: PageDevice {
    /// Output resolution as configured at construction, or
    /// `None` when the device should render at the
    /// implementation-default DPI (72 — see `DEFAULT_DPI`).
    fn resolution(&self) -> Option<Resolution>;

    /// Output width in pixels as configured at construction.
    /// `0` when no explicit width was provided — the renderer
    /// derives width from the page's MediaBox at the effective
    /// DPI in that case.
    fn width(&self) -> i32;

    /// Output height in pixels as configured at construction.
    /// `0` when no explicit height was provided — same
    /// MediaBox-derived behaviour as [`ImageDevice::width`].
    fn height(&self) -> i32;

    /// Page-box selector consulted when interpreting
    /// [`ImageDevice::width`] / [`ImageDevice::height`] against
    /// the page's MediaBox / CropBox. Read-write on the canonical
    /// surface; mutable via [`set_coordinate_type`](ImageDevice::set_coordinate_type).
    /// The v1 rasteriser does not yet honour this knob at encode
    /// time — the carrier round-trips so calling code can
    /// configure as planned and pick it up when the rendering-
    /// options-on-devices semantics land.
    fn coordinate_type(&self) -> PageCoordinateType;

    /// Set [`coordinate_type`](ImageDevice::coordinate_type).
    fn set_coordinate_type(&mut self, value: PageCoordinateType);

    /// Shared reference to the device's [`RenderingOptions`] bag
    /// — the fine-grained rendering policy consulted by the
    /// raster pipeline. Read-write on the canonical surface; in
    /// Rust the read path returns a shared reference and the
    /// mutation paths are [`rendering_options_mut`](ImageDevice::rendering_options_mut)
    /// (in-place tweaks) and [`set_rendering_options`](ImageDevice::set_rendering_options)
    /// (replace by value). The v1 raster path does not yet
    /// honour any of the bag's knobs at encode time.
    fn rendering_options(&self) -> &RenderingOptions;

    /// Mutable [`RenderingOptions`] borrow — allows in-place
    /// tweaks like `device.rendering_options_mut().set_analyze_fonts(true)`.
    fn rendering_options_mut(&mut self) -> &mut RenderingOptions;

    /// Replace the device's [`RenderingOptions`] bag by value.
    fn set_rendering_options(&mut self, value: RenderingOptions);

    /// AcroForm-widget presentation policy. Read-write on the
    /// canonical surface; not yet honoured by the v1 raster path.
    fn form_presentation_mode(&self) -> FormPresentationMode;

    /// Set [`form_presentation_mode`](ImageDevice::form_presentation_mode).
    fn set_form_presentation_mode(&mut self, value: FormPresentationMode);

    /// Rasterise `page` at the device's effective DPI (the
    /// horizontal axis of [`ImageDevice::resolution`] when set,
    /// otherwise the 72-DPI default) and return a flat
    /// [`BitmapInfo`] carrying the RGBA8 pixel buffer in
    /// [`PixelFormat::Rgba32`] layout.
    ///
    /// Default implementation routes through the
    /// `internal::page_renderer` primitive — the canonical
    /// abstract `ImageDevice` exposes the same accessor and
    /// every concrete `ImageDevice` inherits the same body, so
    /// the Rust trait surfaces it as a default method rather
    /// than a per-leaf override.
    fn get_bitmap(&self, page: &Page) -> Result<BitmapInfo, Error> {
        let dpi = self.resolution().map(|r| r.x() as f64).unwrap_or(DEFAULT_DPI);
        // ImageDevice's default get_bitmap renders with the opaque
        // canvas (transparent_background=false). PngDevice overrides
        // its own render path to plumb its transparent_background
        // flag through.
        let rendered = page_renderer::render(
            page.internal_doc_bytes(),
            page.require_persisted_leaf()?,
            dpi,
            false,
        )
        .map_err(|e| Error::InvalidPdf(e.to_string()))?;
        Ok(BitmapInfo::new(
            rendered.pixels,
            rendered.width as i32,
            rendered.height as i32,
            PixelFormat::Rgba32,
        ))
    }
}

/// Shared geometry + rendering-policy state for every concrete
/// raster device. Held inline on the concrete type (e.g.
/// [`PngDevice`](crate::PngDevice)); the [`ImageDevice`] trait surfaces it through
/// the accessor methods.
///
/// Not part of the canonical .NET surface — the C# / TypeScript
/// peers reuse this state via class inheritance, which Rust
/// expresses as struct composition. Beat 13 grew this state with
/// `coordinate_type`, `rendering_options`, and
/// `form_presentation_mode` so every concrete leaf shares the
/// same canonical defaults (`CropBox`,
/// engine-default-RenderingOptions, `Production`).
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ImageDeviceState {
    pub(crate) resolution: Option<Resolution>,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) coordinate_type: PageCoordinateType,
    pub(crate) rendering_options: RenderingOptions,
    pub(crate) form_presentation_mode: FormPresentationMode,
}

impl ImageDeviceState {
    pub(crate) fn empty() -> Self {
        Self::default()
    }

    pub(crate) fn with_resolution(resolution: Resolution) -> Self {
        Self { resolution: Some(resolution), ..Self::default() }
    }

    pub(crate) fn with_dimensions(width: i32, height: i32) -> Self {
        Self { width, height, ..Self::default() }
    }

    pub(crate) fn with_dimensions_and_resolution(
        width: i32,
        height: i32,
        resolution: Resolution,
    ) -> Self {
        Self {
            resolution: Some(resolution),
            width,
            height,
            ..Self::default()
        }
    }

    /// Derive width / height from a [`PageSize`](crate::PageSize)
    /// via a truncating cast of its user-space-unit width / height.
    /// Mirrors `(int)pageSize.Width` on the .NET surface and the
    /// `static_cast<int>(pageSize.Width())` body the C++ peer
    /// emits — both peers agree on this cast.
    pub(crate) fn with_page_size(page_size: super::page_size::PageSize) -> Self {
        Self {
            width: page_size.width() as i32,
            height: page_size.height() as i32,
            ..Self::default()
        }
    }

    /// Derive width / height from a [`PageSize`](crate::PageSize)
    /// and pin the output [`Resolution`]. See
    /// [`with_page_size`](ImageDeviceState::with_page_size).
    pub(crate) fn with_page_size_and_resolution(
        page_size: super::page_size::PageSize,
        resolution: Resolution,
    ) -> Self {
        Self {
            resolution: Some(resolution),
            width: page_size.width() as i32,
            height: page_size.height() as i32,
            ..Self::default()
        }
    }

    /// Effective DPI used to drive `page_renderer`. Honours the
    /// configured horizontal axis when a [`Resolution`] is set;
    /// otherwise falls back to `DEFAULT_DPI`.
    pub(crate) fn effective_dpi(&self) -> f64 {
        match self.resolution {
            Some(r) => r.x() as f64,
            None => DEFAULT_DPI,
        }
    }
}
