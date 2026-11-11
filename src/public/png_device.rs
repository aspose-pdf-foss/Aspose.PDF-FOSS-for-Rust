//! `Aspose.Pdf.Devices.PngDevice` — concrete [`ImageDevice`] that
//! renders a single PDF [`Page`] to PNG bytes.
//!
//! Composes the foundation primitives
//! `internal::page_renderer` (rasteriser) +
//! `internal::png_encoder` (PNG writer). Honours
//! [`PngDevice::transparent_background`]: when `true`, alpha is
//! preserved through the PNG `Rgba` color type; when `false`,
//! source pixels are composited onto an opaque white background
//! and emitted via the `Rgb` color type. The
//! [`Resolution`] configured at construction
//! drives the raster DPI (72 default when unset).

use super::device::{Device, PageDevice};
use super::error::Error;
use super::form_presentation_mode::FormPresentationMode;
use super::image_device::{ImageDevice, ImageDeviceState};
use super::page::Page;
use super::page_coordinate_type::PageCoordinateType;
use super::page_size::PageSize;
use super::rendering_options::RenderingOptions;
use super::resolution::Resolution;

use crate::internal::page_renderer;
use crate::internal::png_encoder;

/// Concrete [`ImageDevice`] emitting PNG bytes.
#[derive(Debug, Clone)]
pub struct PngDevice {
    state: ImageDeviceState,
    transparent_background: bool,
}

impl PngDevice {
    /// Default-construct — render at `DEFAULT_DPI`
    /// (72), opaque background.
    pub fn new() -> Self {
        Self {
            state: ImageDeviceState::empty(),
            transparent_background: false,
        }
    }

    /// Construct with an explicit output resolution.
    pub fn with_resolution(resolution: Resolution) -> Self {
        Self {
            state: ImageDeviceState::with_resolution(resolution),
            transparent_background: false,
        }
    }

    /// Construct with explicit pixel dimensions only. The
    /// configured width and height are stored for the
    /// [`ImageDevice::width`] / [`ImageDevice::height`]
    /// accessors; the v1 rasteriser still derives the actual
    /// output size from the page's MediaBox at the effective
    /// DPI — explicit-dimension rendering (independent of
    /// MediaBox) is a future-beat scope expansion.
    pub fn with_dimensions(width: i32, height: i32) -> Self {
        Self {
            state: ImageDeviceState::with_dimensions(width, height),
            transparent_background: false,
        }
    }

    /// Construct with explicit pixel dimensions and an output
    /// resolution. Same dimension caveat as
    /// [`PngDevice::with_dimensions`].
    pub fn with_dimensions_and_resolution(
        width: i32,
        height: i32,
        resolution: Resolution,
    ) -> Self {
        Self {
            state: ImageDeviceState::with_dimensions_and_resolution(
                width,
                height,
                resolution,
            ),
            transparent_background: false,
        }
    }

    /// Construct with [`PageSize`]-derived pixel dimensions. The
    /// canonical algorithm is `(int)pageSize.Width` /
    /// `(int)pageSize.Height` — a truncating cast straight from
    /// the user-space-unit page geometry into the device's
    /// width / height pixel fields. Same MediaBox-derived caveat
    /// as [`PngDevice::with_dimensions`].
    pub fn with_page_size(page_size: PageSize) -> Self {
        Self {
            state: ImageDeviceState::with_page_size(page_size),
            transparent_background: false,
        }
    }

    /// Construct with [`PageSize`]-derived pixel dimensions and an
    /// output resolution. See
    /// [`PngDevice::with_page_size`].
    pub fn with_page_size_and_resolution(
        page_size: PageSize,
        resolution: Resolution,
    ) -> Self {
        Self {
            state: ImageDeviceState::with_page_size_and_resolution(
                page_size,
                resolution,
            ),
            transparent_background: false,
        }
    }

    /// `true` when this device emits a PNG with alpha
    /// pass-through (color type `Rgba`); `false` (the default)
    /// when source-alpha pixels are composited onto an opaque
    /// white background and emitted as `Rgb`.
    pub fn transparent_background(&self) -> bool {
        self.transparent_background
    }

    /// Toggle the transparent-background mode. See
    /// [`PngDevice::transparent_background`].
    pub fn set_transparent_background(&mut self, value: bool) {
        self.transparent_background = value;
    }
}

impl Default for PngDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl Device for PngDevice {}

impl PageDevice for PngDevice {
    fn process(
        &self,
        page: &Page,
        output: &mut dyn std::io::Write,
    ) -> Result<(), Error> {
        // transparent_background plumbs through to the renderer's
        // canvas-init step (0,0,0,0 vs 255,255,255,255) — matches
        // python / csharp / cpp / ts peer behaviour.
        let rendered = page_renderer::render(
            page.internal_doc_bytes(),
            page.require_persisted_leaf()?,
            self.state.effective_dpi(),
            self.transparent_background,
        )
        .map_err(|e| Error::InvalidPdf(e.to_string()))?;

        let (rgba, color_type) = if self.transparent_background {
            (rendered.pixels, png_encoder::ColorType::Rgba)
        } else {
            (
                composite_alpha_on_white(
                    &rendered.pixels,
                    rendered.width,
                    rendered.height,
                ),
                png_encoder::ColorType::Rgb,
            )
        };

        let encoded = png_encoder::encode(
            rendered.width,
            rendered.height,
            &rgba,
            &png_encoder::Options { color_type },
        )
        .map_err(|e| Error::InvalidPdf(e.to_string()))?;

        output.write_all(&encoded)?;
        Ok(())
    }
}

impl ImageDevice for PngDevice {
    fn resolution(&self) -> Option<Resolution> {
        self.state.resolution
    }

    fn width(&self) -> i32 {
        self.state.width
    }

    fn height(&self) -> i32 {
        self.state.height
    }

    fn coordinate_type(&self) -> PageCoordinateType {
        self.state.coordinate_type
    }

    fn set_coordinate_type(&mut self, value: PageCoordinateType) {
        self.state.coordinate_type = value;
    }

    fn rendering_options(&self) -> &RenderingOptions {
        &self.state.rendering_options
    }

    fn rendering_options_mut(&mut self) -> &mut RenderingOptions {
        &mut self.state.rendering_options
    }

    fn set_rendering_options(&mut self, value: RenderingOptions) {
        self.state.rendering_options = value;
    }

    fn form_presentation_mode(&self) -> FormPresentationMode {
        self.state.form_presentation_mode
    }

    fn set_form_presentation_mode(&mut self, value: FormPresentationMode) {
        self.state.form_presentation_mode = value;
    }
}

/// Composite source RGBA pixels onto an opaque white background,
/// keeping the RGBA8 layout (alpha forced to 255 on output).
/// `png_encoder` always takes RGBA8 input regardless of the
/// output `ColorType`; this pre-composite ensures partial-alpha
/// source pixels render correctly when the encoder drops the
/// alpha channel for `Rgb` output.
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
