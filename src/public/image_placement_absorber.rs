//! `Aspose.Pdf.ImagePlacementAbsorber` — an image-placement visitor,
//! per `capabilities/image_placement_absorber.yaml`.
//!
//! Walks a [`Document`] or [`Page`], finds every place an image XObject
//! is drawn (`/Name Do`), and exposes the results as
//! [`ImagePlacement`]s carrying the transformation matrix, page-space
//! rectangle, rotation, resolution, image, and page. The image analog
//! of [`TextFragmentAbsorber`](crate::TextFragmentAbsorber).
//!
//! Canonically extends `Object`. The whole cataloged surface ships: the
//! default constructor, the [`is_read_only_mode`](ImagePlacementAbsorber::is_read_only_mode)
//! flag, the [`image_placements`](ImagePlacementAbsorber::image_placements)
//! accessor, and `Visit(Document)` / `Visit(Page)`. `Visit` returns
//! `Result` (the underlying parse is fallible), following the
//! established convention on the text absorbers.

use crate::internal::image_placement_io;
use crate::internal::ximage_io;
use crate::public::blend_mode::BlendMode;
use crate::public::compositing_parameters::CompositingParameters;
use crate::public::document::Document;
use crate::public::error::Error;
use crate::public::image_filter_type::ImageFilterType;
use crate::public::image_placement::ImagePlacement;
use crate::public::image_placement_collection::ImagePlacementCollection;
use crate::public::matrix::Matrix;
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;
use crate::public::resolution::Resolution;
use crate::public::x_image::XImage;
use std::collections::HashMap;

/// Image-placement visitor producing located [`ImagePlacement`]s.
#[derive(Debug, Default)]
pub struct ImagePlacementAbsorber {
    is_read_only_mode: bool,
    placements: ImagePlacementCollection,
}

impl ImagePlacementAbsorber {
    /// Construct an absorber with the default (read/write) mode.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the absorber operates in read-only mode (it never
    /// mutates the document; the flag round-trips for API parity).
    pub fn is_read_only_mode(&self) -> bool {
        self.is_read_only_mode
    }

    /// Set the read-only mode flag.
    pub fn set_is_read_only_mode(&mut self, value: bool) {
        self.is_read_only_mode = value;
    }

    /// The placements found across prior visits.
    pub fn image_placements(&self) -> &ImagePlacementCollection {
        &self.placements
    }

    /// Visit every page of `pdf`; placements accumulate in
    /// [`image_placements`](ImagePlacementAbsorber::image_placements).
    pub fn visit_document(&mut self, pdf: &Document) -> Result<(), Error> {
        let pages = pdf.pages();
        for n in 1..=pages.count() {
            let page = pages.get(n)?;
            self.absorb_page(&page);
        }
        Ok(())
    }

    /// Visit a single `page`; placements accumulate in
    /// [`image_placements`](ImagePlacementAbsorber::image_placements).
    pub fn visit_page(&mut self, page: &Page) -> Result<(), Error> {
        self.absorb_page(page);
        Ok(())
    }

    fn absorb_page(&mut self, page: &Page) {
        let bytes = page.internal_doc_bytes();
        let leaf_id = page.internal_leaf_id();

        // Name → loaded image, so a placement can carry its XImage and
        // compute resolution from the source pixel dimensions.
        let mut images: HashMap<String, ximage_io::LoadedImage> = HashMap::new();
        for img in ximage_io::parse_page_images(bytes, leaf_id) {
            images.insert(img.name.clone(), img);
        }

        for placement in image_placement_io::parse_page_placements(bytes, leaf_id) {
            let Some(loaded) = images.get(&placement.name) else { continue };
            let m = placement.matrix;
            let matrix = Matrix::new(m[0], m[1], m[2], m[3], m[4], m[5]);
            let rectangle = unit_square_bbox(&m);
            let rotation = m[1].atan2(m[0]).to_degrees() as f32;
            let resolution = placed_resolution(&m, loaded.width, loaded.height);
            let image = build_ximage(loaded);
            let compositing = CompositingParameters::with_mask(
                blend_mode(&placement.blend),
                filter_type(placement.filter.as_deref()),
                placement.is_masked,
            );
            self.placements.push(ImagePlacement::new(
                matrix,
                rectangle,
                rotation,
                resolution,
                image,
                page.clone(),
                compositing,
            ));
        }
    }
}

/// The page-space bounding box of the unit square `[0,1]×[0,1]` mapped
/// through `m` — the region a PDF image is painted into.
fn unit_square_bbox(m: &[f64; 6]) -> Rectangle {
    let corner = |u: f64, v: f64| (m[0] * u + m[2] * v + m[4], m[1] * u + m[3] * v + m[5]);
    let pts = [corner(0.0, 0.0), corner(1.0, 0.0), corner(1.0, 1.0), corner(0.0, 1.0)];
    let (mut llx, mut lly) = (f64::INFINITY, f64::INFINITY);
    let (mut urx, mut ury) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for (x, y) in pts {
        llx = llx.min(x);
        lly = lly.min(y);
        urx = urx.max(x);
        ury = ury.max(y);
    }
    Rectangle::new(llx, lly, urx, ury, false)
}

/// The effective resolution: source pixels over placed size in inches
/// (1 PDF unit = 1/72 inch). Falls back to 72 DPI on a degenerate
/// placement.
fn placed_resolution(m: &[f64; 6], width_px: i32, height_px: i32) -> Resolution {
    let placed_w = (m[0] * m[0] + m[1] * m[1]).sqrt();
    let placed_h = (m[2] * m[2] + m[3] * m[3]).sqrt();
    let dpi = |px: i32, size: f64| -> i32 {
        if size <= 0.0 || px <= 0 {
            72
        } else {
            ((px as f64) * 72.0 / size).round().max(1.0) as i32
        }
    };
    Resolution::with_axes(dpi(width_px, placed_w), dpi(height_px, placed_h))
        .unwrap_or_else(|_| Resolution::new(72).unwrap())
}

/// Map a PDF `/BM` blend-mode name to [`BlendMode`]. Unrecognised names
/// (and the legacy `Compatible` alias) fall back to `Normal`.
fn blend_mode(name: &[u8]) -> BlendMode {
    match name {
        b"Multiply" => BlendMode::Multiply,
        b"Screen" => BlendMode::Screen,
        b"Overlay" => BlendMode::Overlay,
        b"Darken" => BlendMode::Darken,
        b"Lighten" => BlendMode::Lighten,
        b"ColorDodge" => BlendMode::ColorDodge,
        b"ColorBurn" => BlendMode::ColorBurn,
        b"HardLight" => BlendMode::HardLight,
        b"SoftLight" => BlendMode::SoftLight,
        b"Difference" => BlendMode::Difference,
        b"Exclusion" => BlendMode::Exclusion,
        b"Hue" => BlendMode::Hue,
        b"Saturation" => BlendMode::Saturation,
        b"Color" => BlendMode::Color,
        b"Luminosity" => BlendMode::Luminosity,
        _ => BlendMode::Normal,
    }
}

/// Map a PDF image `/Filter` codec name to [`ImageFilterType`]. Unfiltered
/// or unrecognised images fall back to `Jpeg`, matching the compositing
/// type's own default.
fn filter_type(codec: Option<&[u8]>) -> ImageFilterType {
    match codec {
        Some(b"JPXDecode") => ImageFilterType::Jpeg2000,
        Some(b"DCTDecode") => ImageFilterType::Jpeg,
        Some(b"FlateDecode") => ImageFilterType::Flate,
        Some(b"CCITTFaxDecode") => ImageFilterType::CCITTFax,
        _ => ImageFilterType::Jpeg,
    }
}

/// Rebuild the public [`XImage`] for a loaded image (encoded blob or
/// expanded raster), mirroring the document image-load path.
fn build_ximage(loaded: &ximage_io::LoadedImage) -> XImage {
    let color_type = XImage::color_type_from_pdf(
        loaded.color_space.as_deref(),
        loaded.bits_per_component,
        loaded.image_mask,
    );
    let ximage = if loaded.encoded {
        XImage::from_bytes(loaded.raw.clone(), loaded.name.clone())
    } else {
        XImage::from_raster(
            loaded.raw.clone(),
            loaded.width,
            loaded.height,
            loaded.name.clone(),
            loaded.has_alpha,
        )
    };
    ximage.with_color_type(color_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_empty_and_read_write() {
        let a = ImagePlacementAbsorber::new();
        assert!(!a.is_read_only_mode());
        assert_eq!(a.image_placements().count(), 0);
    }

    #[test]
    fn read_only_flag_round_trips() {
        let mut a = ImagePlacementAbsorber::new();
        a.set_is_read_only_mode(true);
        assert!(a.is_read_only_mode());
    }
}
