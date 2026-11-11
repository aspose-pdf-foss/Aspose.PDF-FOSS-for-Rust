//! `Aspose.Pdf.XImage` — an embedded image XObject, per
//! `capabilities/x_image.yaml`.
//!
//! A descriptor over an embedded image's bytes: its dimensions,
//! transparency, name, and raw data. Created by an
//! [`XImageCollection`](crate::XImageCollection); the dimensions and
//! transparency are read from the image header (PNG / JPEG / GIF /
//! BMP). There is no public constructor.

/// How an [`XImage`]'s `raw` bytes are stored — distinguishes an
/// encoded blob (PNG / JPEG / GIF / BMP) from already-rasterised
/// `RGBA8` samples, which the save path needs to embed correctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImageKind {
    /// `raw` is an encoded image file (PNG / JPEG / GIF / BMP).
    Encoded,
    /// `raw` is `width * height * 4` `RGBA8` samples, row-major.
    RasterRgba8,
}

use crate::public::color_type::ColorType;
use crate::public::image_filter_type::ImageFilterType;

/// An embedded image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XImage {
    raw: Vec<u8>,
    width: i32,
    height: i32,
    name: String,
    image_mask: bool,
    contains_transparency: bool,
    kind: ImageKind,
    color_type: ColorType,
}

impl XImage {
    /// Build an image descriptor from its raw bytes and resource
    /// name, reading the dimensions / transparency from the header.
    pub(crate) fn from_bytes(raw: Vec<u8>, name: impl Into<String>) -> Self {
        let (width, height, contains_transparency) = probe_image(&raw);
        let color_type = classify_color(&raw);
        Self {
            raw,
            width,
            height,
            name: name.into(),
            image_mask: false,
            contains_transparency,
            kind: ImageKind::Encoded,
            color_type,
        }
    }

    /// Build an image descriptor from already-known fields (e.g. a
    /// rasterised [`BitmapInfo`](crate::BitmapInfo)).
    pub(crate) fn from_raster(
        raw: Vec<u8>,
        width: i32,
        height: i32,
        name: impl Into<String>,
        contains_transparency: bool,
    ) -> Self {
        Self {
            raw,
            width,
            height,
            name: name.into(),
            image_mask: false,
            contains_transparency,
            kind: ImageKind::RasterRgba8,
            // A widened raster carries full-colour samples; the load
            // path overrides this with the source `/ColorSpace` facts
            // via `with_color_type`.
            color_type: ColorType::Rgb,
        }
    }

    /// Override the colour classification from the source PDF image
    /// dictionary's `/ColorSpace` / `/BitsPerComponent` / `/ImageMask`
    /// — the load path uses this so a grayscale or bilevel image keeps
    /// its true [`ColorType`] after its samples are widened to `RGBA8`.
    pub(crate) fn with_color_type(mut self, color_type: ColorType) -> Self {
        self.color_type = color_type;
        self
    }

    /// Map a source image dictionary's colour facts to a [`ColorType`].
    /// An `/ImageMask` stencil is bilevel; a gray space is grayscale
    /// (or bilevel at one bit); any device / calibrated / indexed /
    /// ICC colour space is `Rgb`; anything else is `Undefined`.
    pub(crate) fn color_type_from_pdf(
        color_space: Option<&[u8]>,
        bits_per_component: i32,
        image_mask: bool,
    ) -> ColorType {
        if image_mask {
            return ColorType::BlackAndWhite;
        }
        match color_space {
            Some(b"DeviceGray") | Some(b"CalGray") | Some(b"G") => {
                if bits_per_component == 1 {
                    ColorType::BlackAndWhite
                } else {
                    ColorType::Grayscale
                }
            }
            Some(b"DeviceRGB") | Some(b"RGB") | Some(b"CalRGB") | Some(b"Lab")
            | Some(b"DeviceCMYK") | Some(b"CMYK") | Some(b"ICCBased")
            | Some(b"Indexed") | Some(b"I") | Some(b"Separation")
            | Some(b"DeviceN") => ColorType::Rgb,
            _ => ColorType::Undefined,
        }
    }

    /// How `raw` is stored (encoded blob vs raster samples).
    pub(crate) fn kind(&self) -> ImageKind {
        self.kind
    }

    /// The raw bytes without cloning — for the save path.
    pub(crate) fn raw_bytes(&self) -> &[u8] {
        &self.raw
    }

    /// The raw embedded image bytes.
    pub fn get_raw_image_data(&self) -> Vec<u8> {
        self.raw.clone()
    }

    /// The image's bytes as a readable buffer. In the shipped subset a
    /// `System.IO.Stream` return is surfaced as an in-memory `Vec<u8>`,
    /// so this yields the same underlying data as
    /// [`Self::get_raw_image_data`].
    pub fn to_stream(&self) -> Vec<u8> {
        self.get_raw_image_data()
    }

    /// Whether the image carries an alpha / transparency channel.
    pub fn contains_transparency(&self) -> bool {
        self.contains_transparency
    }

    /// The image width, in pixels.
    pub fn width(&self) -> i32 {
        self.width
    }

    /// The image height, in pixels.
    pub fn height(&self) -> i32 {
        self.height
    }

    /// The image's resource name.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Set the image's resource name.
    pub fn set_name(&mut self, value: impl Into<String>) {
        self.name = value.into();
    }

    /// Rename the image's resource. Equivalent to setting [`Self::set_name`];
    /// the new name becomes the image's `/XObject` key on the next save.
    pub fn rename(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// The image's name as a key in its owning
    /// [`XImageCollection`](crate::XImageCollection) — the `/XObject`
    /// resource key the image is stored under.
    pub fn get_name_in_collection(&self) -> String {
        self.name.clone()
    }

    /// Whether `other` refers to the same image. In the value model an
    /// image is identified by its content and name, so this is a
    /// structural comparison — matching how an
    /// [`XImageCollection`](crate::XImageCollection) locates an image.
    pub fn is_the_same_object(&self, other: &XImage) -> bool {
        self == other
    }

    /// Whether the image is a stencil mask.
    pub fn image_mask(&self) -> bool {
        self.image_mask
    }

    /// The compression filter the image is stored with. A JPEG blob is
    /// [`ImageFilterType::Jpeg`] (`/DCTDecode`); every other image is
    /// re-encoded with [`ImageFilterType::Flate`] (`/FlateDecode`) in
    /// the shipped subset.
    pub fn filter_type(&self) -> ImageFilterType {
        if self.kind == ImageKind::Encoded && is_jpeg(&self.raw) {
            ImageFilterType::Jpeg
        } else {
            ImageFilterType::Flate
        }
    }

    /// The image's colour classification (colour / grayscale / bilevel
    /// / undefined). For a loaded image this reflects the source
    /// `/ColorSpace`; for an in-memory image it is read from the
    /// encoded header.
    pub fn get_color_type(&self) -> ColorType {
        self.color_type
    }
}

fn is_jpeg(b: &[u8]) -> bool {
    b.len() >= 2 && b[0] == 0xFF && b[1] == 0xD8
}

/// Classify an encoded image's colour type from its header (PNG / JPEG
/// / GIF / BMP). Returns [`ColorType::Undefined`] for an unrecognised
/// or truncated header.
fn classify_color(b: &[u8]) -> ColorType {
    // PNG: colour-type byte @25, bit-depth @24.
    if b.len() >= 26 && b[..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        let bit_depth = b[24];
        return match b[25] {
            // 0 = grayscale, 4 = grayscale + alpha.
            0 | 4 => {
                if bit_depth == 1 {
                    ColorType::BlackAndWhite
                } else {
                    ColorType::Grayscale
                }
            }
            // 2 = truecolour, 3 = indexed, 6 = truecolour + alpha.
            2 | 3 | 6 => ColorType::Rgb,
            _ => ColorType::Undefined,
        };
    }
    // JPEG: component count from the SOF marker (1 = gray, else colour).
    if is_jpeg(b) {
        return match jpeg_components(b) {
            1 => ColorType::Grayscale,
            0 => ColorType::Undefined,
            _ => ColorType::Rgb,
        };
    }
    // GIF: always an indexed (colour) palette.
    if b.len() >= 6 && &b[..3] == b"GIF" {
        return ColorType::Rgb;
    }
    // BMP: bit-depth @28 (LE u16).
    if b.len() >= 30 && &b[..2] == b"BM" {
        return match le16(b, 28).unwrap_or(0) {
            1 => ColorType::BlackAndWhite,
            _ => ColorType::Rgb,
        };
    }
    ColorType::Undefined
}

/// Component count declared in a JPEG SOF marker (1 = grayscale,
/// 3 = YCbCr/RGB, 4 = CMYK). Returns 0 when no SOF is found.
fn jpeg_components(b: &[u8]) -> i32 {
    let mut i = 2;
    while i + 9 <= b.len() {
        if b[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = b[i + 1];
        if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            return *b.get(i + 9).unwrap_or(&0) as i32;
        }
        let seg = be16(b, i + 2).unwrap_or(0);
        if seg < 2 {
            break;
        }
        i += 2 + seg as usize;
    }
    0
}

fn be16(b: &[u8], o: usize) -> Option<i32> {
    Some(((*b.get(o)? as i32) << 8) | *b.get(o + 1)? as i32)
}
fn be32(b: &[u8], o: usize) -> Option<i32> {
    Some(
        ((*b.get(o)? as i32) << 24)
            | ((*b.get(o + 1)? as i32) << 16)
            | ((*b.get(o + 2)? as i32) << 8)
            | *b.get(o + 3)? as i32,
    )
}
fn le16(b: &[u8], o: usize) -> Option<i32> {
    Some((*b.get(o)? as i32) | ((*b.get(o + 1)? as i32) << 8))
}
fn le32(b: &[u8], o: usize) -> Option<i32> {
    Some(
        (*b.get(o)? as i32)
            | ((*b.get(o + 1)? as i32) << 8)
            | ((*b.get(o + 2)? as i32) << 16)
            | ((*b.get(o + 3)? as i32) << 24),
    )
}

/// Read `(width, height, has_transparency)` from an image header.
/// Returns zeros for an unrecognised / truncated header.
fn probe_image(b: &[u8]) -> (i32, i32, bool) {
    // PNG: 8-byte signature, then IHDR (width@16, height@20,
    // colour-type@25; 4 / 6 carry alpha).
    if b.len() >= 26 && b[..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
    {
        let w = be32(b, 16).unwrap_or(0);
        let h = be32(b, 20).unwrap_or(0);
        let alpha = matches!(b[25], 4 | 6);
        return (w, h, alpha);
    }
    // JPEG: scan segments for an SOF marker.
    if b.len() >= 2 && b[0] == 0xFF && b[1] == 0xD8 {
        let mut i = 2;
        while i + 9 <= b.len() {
            if b[i] != 0xFF {
                i += 1;
                continue;
            }
            let marker = b[i + 1];
            // SOF0..SOF15 except DHT(C4)/JPG(C8)/DAC(CC).
            if (0xC0..=0xCF).contains(&marker)
                && !matches!(marker, 0xC4 | 0xC8 | 0xCC)
            {
                let h = be16(b, i + 5).unwrap_or(0);
                let w = be16(b, i + 7).unwrap_or(0);
                return (w, h, false);
            }
            let seg = be16(b, i + 2).unwrap_or(0);
            if seg < 2 {
                break;
            }
            i += 2 + seg as usize;
        }
        return (0, 0, false);
    }
    // GIF: width@6 / height@8 (LE u16).
    if b.len() >= 10 && (&b[..3] == b"GIF") {
        return (le16(b, 6).unwrap_or(0), le16(b, 8).unwrap_or(0), true);
    }
    // BMP: width@18 / height@22 (LE i32); 32bpp may carry alpha.
    if b.len() >= 30 && &b[..2] == b"BM" {
        let w = le32(b, 18).unwrap_or(0);
        let h = le32(b, 22).unwrap_or(0).abs();
        let alpha = le16(b, 28).unwrap_or(0) == 32;
        return (w, h, alpha);
    }
    (0, 0, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probes_png_dimensions_and_alpha() {
        // Minimal PNG header: sig + IHDR length/type + 13-byte IHDR.
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        png.extend_from_slice(&[0, 0, 0, 13]); // IHDR length
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&[0, 0, 0, 4]); // width 4
        png.extend_from_slice(&[0, 0, 0, 3]); // height 3
        png.push(8); // bit depth
        png.push(6); // colour type 6 (RGBA → alpha)
        png.extend_from_slice(&[0, 0, 0]); // compression/filter/interlace
        let img = XImage::from_bytes(png, "Im1");
        assert_eq!(img.width(), 4);
        assert_eq!(img.height(), 3);
        assert!(img.contains_transparency());
        assert_eq!(img.name(), "Im1");
        assert!(!img.image_mask());
    }

    #[test]
    fn probes_jpeg_dimensions() {
        // SOI + APP0 stub + SOF0 with height 7, width 11.
        let mut jpg = vec![0xFF, 0xD8];
        jpg.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 0x08]); // SOF0, len 17, prec 8
        jpg.extend_from_slice(&[0x00, 0x07]); // height 7
        jpg.extend_from_slice(&[0x00, 0x0B]); // width 11
        let img = XImage::from_bytes(jpg.clone(), "Im1");
        assert_eq!(img.width(), 11);
        assert_eq!(img.height(), 7);
        assert!(!img.contains_transparency());
        assert_eq!(img.get_raw_image_data(), jpg);
    }

    /// Build a minimal PNG header with the given colour-type / bit-depth.
    fn png_header(color_type: u8, bit_depth: u8) -> Vec<u8> {
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        png.extend_from_slice(&[0, 0, 0, 13]);
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&[0, 0, 0, 2]); // width 2
        png.extend_from_slice(&[0, 0, 0, 2]); // height 2
        png.push(bit_depth);
        png.push(color_type);
        png.extend_from_slice(&[0, 0, 0]);
        png
    }

    #[test]
    fn filter_type_jpeg_vs_flate() {
        // A JPEG blob keeps DCTDecode → Jpeg.
        let jpg = vec![0xFF, 0xD8, 0xFF, 0xD9];
        assert_eq!(XImage::from_bytes(jpg, "j").filter_type(), ImageFilterType::Jpeg);
        // A PNG (re-encoded to FlateDecode) and a raster both → Flate.
        assert_eq!(
            XImage::from_bytes(png_header(6, 8), "p").filter_type(),
            ImageFilterType::Flate
        );
        let raster = XImage::from_raster(vec![0; 16], 2, 2, "r", false);
        assert_eq!(raster.filter_type(), ImageFilterType::Flate);
    }

    #[test]
    fn color_type_from_png_header() {
        // Truecolour + alpha → Rgb.
        assert_eq!(XImage::from_bytes(png_header(6, 8), "a").get_color_type(), ColorType::Rgb);
        // 8-bit grayscale → Grayscale.
        assert_eq!(
            XImage::from_bytes(png_header(0, 8), "g").get_color_type(),
            ColorType::Grayscale
        );
        // 1-bit grayscale → bilevel.
        assert_eq!(
            XImage::from_bytes(png_header(0, 1), "bw").get_color_type(),
            ColorType::BlackAndWhite
        );
    }

    #[test]
    fn color_type_from_jpeg_components() {
        // SOF0 with a single component → Grayscale.
        let mut gray = vec![0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 0x08];
        gray.extend_from_slice(&[0x00, 0x07, 0x00, 0x0B]); // height/width
        gray.push(0x01); // component count 1
        assert_eq!(XImage::from_bytes(gray, "g").get_color_type(), ColorType::Grayscale);
    }

    #[test]
    fn color_type_from_pdf_facts() {
        use XImage as X;
        assert_eq!(X::color_type_from_pdf(Some(b"DeviceRGB"), 8, false), ColorType::Rgb);
        assert_eq!(X::color_type_from_pdf(Some(b"ICCBased"), 8, false), ColorType::Rgb);
        assert_eq!(X::color_type_from_pdf(Some(b"Indexed"), 8, false), ColorType::Rgb);
        assert_eq!(
            X::color_type_from_pdf(Some(b"DeviceGray"), 8, false),
            ColorType::Grayscale
        );
        assert_eq!(
            X::color_type_from_pdf(Some(b"DeviceGray"), 1, false),
            ColorType::BlackAndWhite
        );
        // An /ImageMask stencil is bilevel regardless of colour space.
        assert_eq!(X::color_type_from_pdf(None, 1, true), ColorType::BlackAndWhite);
        // No colour space and no mask → undetermined.
        assert_eq!(X::color_type_from_pdf(None, 8, false), ColorType::Undefined);
    }

    #[test]
    fn to_stream_yields_raw_bytes() {
        let jpg = vec![0xFF, 0xD8, 0xFF, 0xD9];
        let img = XImage::from_bytes(jpg.clone(), "Im1");
        assert_eq!(img.to_stream(), jpg);
        assert_eq!(img.to_stream(), img.get_raw_image_data());
    }

    #[test]
    fn rename_and_name_in_collection() {
        let mut img = XImage::from_bytes(png_header(6, 8), "Im1");
        assert_eq!(img.get_name_in_collection(), "Im1");
        img.rename("Logo");
        assert_eq!(img.name(), "Logo");
        assert_eq!(img.get_name_in_collection(), "Logo");
    }

    #[test]
    fn is_the_same_object_is_structural() {
        let a = XImage::from_bytes(png_header(6, 8), "Im1");
        let clone = a.clone();
        assert!(a.is_the_same_object(&clone));
        // A different name is a different image.
        let mut renamed = a.clone();
        renamed.rename("Other");
        assert!(!a.is_the_same_object(&renamed));
        // Different content is a different image.
        let jpeg = XImage::from_bytes(vec![0xFF, 0xD8, 0xFF, 0xD9], "Im1");
        assert!(!a.is_the_same_object(&jpeg));
    }

    #[test]
    fn loaded_grayscale_keeps_color_type_after_widening() {
        // A widened raster reports Rgb by default, but the load path's
        // with_color_type override restores the source classification.
        let raster = XImage::from_raster(vec![0; 16], 2, 2, "g", false)
            .with_color_type(ColorType::Grayscale);
        assert_eq!(raster.get_color_type(), ColorType::Grayscale);
    }
}
