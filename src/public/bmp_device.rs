//! `Aspose.Pdf.Devices.BmpDevice` — concrete [`ImageDevice`] that
//! renders a single PDF [`Page`] to an uncompressed BMP byte stream
//! via the foundation `internal::bmp_encoder` primitive.
//!
//! Output is 24-bit BGR (the most-interoperable BMP variant — no
//! alpha channel, matching the canonical .NET surface). The
//! rasteriser produces RGBA8; partial-alpha pixels are composited
//! onto an opaque white background before encoding (same policy
//! `PngDevice` applies when `transparent_background=false`). The
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

use crate::internal::bmp_encoder;
use crate::internal::page_renderer;

/// Concrete [`ImageDevice`] emitting uncompressed BMP bytes.
#[derive(Debug, Clone)]
pub struct BmpDevice {
    state: ImageDeviceState,
}

impl BmpDevice {
    /// Default-construct — render at implementation-default DPI (72).
    pub fn new() -> Self {
        Self { state: ImageDeviceState::empty() }
    }

    /// Construct with an explicit output resolution.
    pub fn with_resolution(resolution: Resolution) -> Self {
        Self { state: ImageDeviceState::with_resolution(resolution) }
    }

    /// Construct with explicit pixel dimensions only. Same
    /// MediaBox-derived caveat as
    /// [`PngDevice::with_dimensions`](crate::PngDevice::with_dimensions).
    pub fn with_dimensions(width: i32, height: i32) -> Self {
        Self { state: ImageDeviceState::with_dimensions(width, height) }
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
        }
    }

    /// Construct with [`PageSize`]-derived pixel dimensions. The
    /// canonical algorithm is `(int)pageSize.Width` /
    /// `(int)pageSize.Height` — a truncating cast straight from
    /// the user-space-unit page geometry into the device's
    /// width / height pixel fields. Same MediaBox-derived caveat
    /// as [`BmpDevice::with_dimensions`].
    pub fn with_page_size(page_size: PageSize) -> Self {
        Self { state: ImageDeviceState::with_page_size(page_size) }
    }

    /// Construct with [`PageSize`]-derived pixel dimensions and an
    /// output resolution. See [`BmpDevice::with_page_size`].
    pub fn with_page_size_and_resolution(
        page_size: PageSize,
        resolution: Resolution,
    ) -> Self {
        Self {
            state: ImageDeviceState::with_page_size_and_resolution(
                page_size,
                resolution,
            ),
        }
    }
}

impl Default for BmpDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl Device for BmpDevice {}

impl PageDevice for BmpDevice {
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

        let encoded = bmp_encoder::encode(
            rendered.width,
            rendered.height,
            &composed,
            bmp_encoder::Options {
                color_type: Some(bmp_encoder::ColorType::Bgr),
            },
        )
        .map_err(|e| Error::InvalidPdf(e.to_string()))?;

        output.write_all(&encoded)?;
        Ok(())
    }
}

impl ImageDevice for BmpDevice {
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

// Same pre-composite policy as PngDevice's opaque path and
// JpegDevice — bmp_encoder reads RGBA8 and drops alpha for the
// Bgr ColorType, so partial-alpha source pixels are composited
// onto opaque white first.
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
