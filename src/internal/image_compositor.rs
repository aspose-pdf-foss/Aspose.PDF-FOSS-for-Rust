// XObject placement + alpha compositing onto a `rasterizer::Raster`.
// PDF 32000-1:2008 §8.9.5 (Image XObjects, generic case) plus §11.3.4
// (src-over compositing math); Foley/van Dam §10.10 (sub-pixel
// super-sampling for resampled images); Porter & Duff 1984 (the
// src-over operator).
//
// The bridge between PDF's Image XObjects (a decoded RGBA pixel buffer
// + a CTM placing the unit rectangle [0,1]² onto the page) and the
// rasterizer's Raster. PageRenderer at the public-API layer assembles
// Image XObjects (via dctdecode / flate / ccitt / jbig2) and calls
// `composite`; this primitive owns nothing about decoding the source
// pixels.
//
// Composes:
//   * transform::{Matrix, Point, apply, inverse}
//   * colorspace::quantize
//   * rasterizer::Raster
//
// Algorithm: walk DESTINATION pixels and apply `inverse(ctm)` to each
// of 16 sub-samples on a 4×4 centred grid (offsets 1/8, 3/8, 5/8,
// 7/8) to map back into source unit-space. Sub-samples that land
// inside [0, 1)² look up the source pixel by nearest-neighbour with
// a drift clamp and contribute to running RGBA sums; sub-samples
// outside contribute zero to BOTH numerator and alpha — the divisor
// stays 16, which gives partial-coverage edges their correct softened
// alpha. Average, then src-over composite the averaged source onto
// the destination with `eff_a = avg_a * alpha`. Quantise per
// `colorspace::quantize`.

use std::fmt;

use super::colorspace::quantize;
use super::rasterizer::Raster;
use super::transform::{apply, inverse, Matrix, Point};

/// Source RGBA8 pixel buffer. `pixels.len()` must equal
/// `width * height * 4`; a malformed buffer no-ops (matching
/// `rasterizer::fill`'s policy on a malformed dst).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImageSource {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Standard lossy-primitive return shape — mirrors
/// `rasterizer::DecodedImage` and `glyph_rasterizer::DecodedImage` so
/// the gate template drives all three uniformly. `components` is
/// always 4 (RGBA8).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // retained for a future-phase public-API binding
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub components: u8,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // retained for a future-phase public-API binding
pub enum Error {
    /// Input ended before a fixed-size header field or the source
    /// pixel payload was fully read.
    Truncated,
    /// First byte was not 0xCC.
    BadMagic,
    /// Version byte was not 1.
    UnsupportedVersion(u8),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated => write!(
                f, "image_compositor: truncated input"),
            Error::BadMagic => write!(
                f, "image_compositor: bad magic (expected 0xCC)"),
            Error::UnsupportedVersion(v) => write!(
                f, "image_compositor: unsupported version {v} (expected 1)"),
        }
    }
}

impl std::error::Error for Error {}

/// 4×4 centred sub-pixel grid offsets — identical convention to
/// `rasterizer::fill` and `glyph_rasterizer::draw`. Sub-sample i is
/// at `(i + 0.5) / 4` of the destination pixel.
const SUB_OFFSETS: [f64; 4] = [0.125, 0.375, 0.625, 0.875];

/// Clamp `v` to [0, 1] using the same convention `colorspace::quantize`
/// applies internally — NaN → 0, < 0 → 0, > 1 → 1. Used on the
/// global-alpha multiplier so out-of-domain inputs behave identically
/// to every other rendering primitive.
#[allow(clippy::manual_clamp, clippy::if_same_then_else)]
fn clamp01(v: f64) -> f64 {
    if v.is_nan() {
        0.0
    } else if v < 0.0 {
        0.0
    } else if v > 1.0 {
        1.0
    } else {
        v
    }
}

/// Composite `src` onto `dst` under `ctm` with global alpha multiplier
/// `alpha`. `ctm` maps the source unit-rectangle [0,1]² to destination
/// pixel coordinates (PDF row-vector convention). In-place mutation
/// of `dst.pixels`.
///
/// Total — there is no error condition for well-formed input. Edge
/// cases that no-op (leaving `dst` untouched):
///   * Zero-sized destination (`dst.width == 0` or `dst.height == 0`).
///   * Zero-sized source (`src.width == 0` or `src.height == 0`).
///   * Malformed destination (`dst.pixels.len() != width * height * 4`).
///   * Malformed source (`src.pixels.len() != width * height * 4`).
///   * Singular CTM (`inverse` returns `None` — det below 1e-12 floor
///     or NaN-bearing matrix).
///   * `alpha` clamps to 0 (NaN, ≤ 0).
pub fn composite(
    dst: &mut Raster,
    src: &ImageSource,
    ctm: Matrix,
    alpha: f64,
    clip: Option<&super::rasterizer::ClipRect>,
) {
    if dst.width == 0 || dst.height == 0 {
        return;
    }
    let dst_pixel_count =
        (dst.width as usize) * (dst.height as usize) * 4;
    if dst.pixels.len() != dst_pixel_count {
        return;
    }
    if src.width == 0 || src.height == 0 {
        return;
    }
    let src_pixel_count =
        (src.width as usize) * (src.height as usize) * 4;
    if src.pixels.len() != src_pixel_count {
        return;
    }

    let inv = match inverse(ctm) {
        Some(m) => m,
        None => return,
    };

    let alpha_clamped = clamp01(alpha);
    if alpha_clamped == 0.0 {
        return;
    }

    let src_w_f = src.width as f64;
    let src_h_f = src.height as f64;
    let src_w_max = (src.width - 1) as usize;
    let src_h_max = (src.height - 1) as usize;
    let src_stride = (src.width as usize) * 4;
    let dst_stride = (dst.width as usize) * 4;

    // The clip narrows the scan, exactly as it does for a fill: a
    // clipped image XObject lands only inside the region.
    let (mut x0, mut y0) = (0i64, 0i64);
    let (mut x1, mut y1) = (dst.width as i64, dst.height as i64);
    if let Some(c) = clip {
        x0 = x0.max(c.x0);
        y0 = y0.max(c.y0);
        x1 = x1.min(c.x1);
        y1 = y1.min(c.y1);
    }
    if x0 >= x1 || y0 >= y1 {
        return;
    }

    for py in y0 as u32..y1 as u32 {
        for px in x0 as u32..x1 as u32 {
            let mut sum_r = 0.0_f64;
            let mut sum_g = 0.0_f64;
            let mut sum_b = 0.0_f64;
            let mut sum_a = 0.0_f64;

            for &jo in &SUB_OFFSETS {
                let sy = py as f64 + jo;
                for &io in &SUB_OFFSETS {
                    let sx = px as f64 + io;
                    let uv = apply(inv, Point { x: sx, y: sy });
                    if uv.x < 0.0 || uv.x >= 1.0 {
                        continue;
                    }
                    if uv.y < 0.0 || uv.y >= 1.0 {
                        continue;
                    }

                    // Nearest-neighbour with a drift clamp at the
                    // right/bottom edge. Strict `< 1.0` already
                    // filters the typical case; the clamp is belt-
                    // and-suspenders for IEEE-754 drift in the
                    // inverse × apply composition.
                    let mut u_pix = (uv.x * src_w_f) as usize;
                    let mut v_pix = (uv.y * src_h_f) as usize;
                    if u_pix > src_w_max {
                        u_pix = src_w_max;
                    }
                    if v_pix > src_h_max {
                        v_pix = src_h_max;
                    }

                    let s_idx = v_pix * src_stride + u_pix * 4;
                    let sr = src.pixels[s_idx] as f64;
                    let sg = src.pixels[s_idx + 1] as f64;
                    let sb = src.pixels[s_idx + 2] as f64;
                    let sa = src.pixels[s_idx + 3] as f64;
                    sum_r += sr / 255.0;
                    sum_g += sg / 255.0;
                    sum_b += sb / 255.0;
                    sum_a += sa / 255.0;
                }
            }

            let avg_r = sum_r / 16.0;
            let avg_g = sum_g / 16.0;
            let avg_b = sum_b / 16.0;
            let avg_a = sum_a / 16.0;

            let clip_coverage =
                clip.map_or(1.0, |c| clip_coverage_at(c, px as i64, py as i64));
            let eff_a = avg_a * alpha_clamped * clip_coverage;
            if eff_a == 0.0 {
                continue;
            }

            let d_idx = (py as usize) * dst_stride + (px as usize) * 4;
            let dr = dst.pixels[d_idx] as f64 / 255.0;
            let dg = dst.pixels[d_idx + 1] as f64 / 255.0;
            let db = dst.pixels[d_idx + 2] as f64 / 255.0;
            let da = dst.pixels[d_idx + 3] as f64 / 255.0;
            let inv_eff = 1.0 - eff_a;

            let out_r = avg_r * eff_a + dr * inv_eff;
            let out_g = avg_g * eff_a + dg * inv_eff;
            let out_b = avg_b * eff_a + db * inv_eff;
            let out_a = eff_a + da * inv_eff;

            dst.pixels[d_idx]     = quantize(out_r);
            dst.pixels[d_idx + 1] = quantize(out_g);
            dst.pixels[d_idx + 2] = quantize(out_b);
            dst.pixels[d_idx + 3] = quantize(out_a);
        }
    }
}

/// A clip's coverage at a device pixel, `0.0..=1.0` — the box on its
/// own, or the soft mask where the clip path was not a rectangle.
fn clip_coverage_at(
    clip: &super::rasterizer::ClipRect,
    px: i64,
    py: i64,
) -> f64 {
    if px < clip.x0 || px >= clip.x1 || py < clip.y0 || py >= clip.y1 {
        return 0.0;
    }
    match &clip.mask {
        None => 1.0,
        Some(mask) => {
            if clip.mask_stride == 0 {
                return 1.0;
            }
            let i = py as usize * clip.mask_stride + px as usize;
            mask.get(i).copied().unwrap_or(0) as f64 / 255.0
        }
    }
}

/// Test-only entry point used by the lossy_oracle_agreement freeze
/// gate. Parses `input` per the binary fixture wire format documented
/// in `Feedstock/PDF/Spec/image_compositor.yaml` (magic 0xCC),
/// allocates a freshly-zeroed Raster, calls `composite`, and returns
/// the result wrapped as a `DecodedImage`.
#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn decode(input: &[u8]) -> Result<DecodedImage, Error> {
    let mut r = Reader::new(input);

    let magic = r.read_u8()?;
    if magic != 0xCC {
        return Err(Error::BadMagic);
    }
    let version = r.read_u8()?;
    if version != 1 {
        return Err(Error::UnsupportedVersion(version));
    }
    let _reserved0 = r.read_u8()?;
    let _reserved1 = r.read_u8()?;

    let dst_w = r.read_u32_le()?;
    let dst_h = r.read_u32_le()?;
    let src_w = r.read_u32_le()?;
    let src_h = r.read_u32_le()?;

    let a = r.read_f64_le()?;
    let b = r.read_f64_le()?;
    let c = r.read_f64_le()?;
    let d = r.read_f64_le()?;
    let e = r.read_f64_le()?;
    let f = r.read_f64_le()?;
    let ctm = Matrix::new(a, b, c, d, e, f);

    let alpha = r.read_f64_le()?;

    let src_pixel_bytes =
        (src_w as usize) * (src_h as usize) * 4;
    let src_pixels = r.read_bytes(src_pixel_bytes)?;
    let src = ImageSource {
        width: src_w,
        height: src_h,
        pixels: src_pixels,
    };

    let dst_pixel_bytes =
        (dst_w as usize) * (dst_h as usize) * 4;
    let mut dst = Raster {
        pixels: vec![0u8; dst_pixel_bytes],
        width: dst_w,
        height: dst_h,
    };
    composite(&mut dst, &src, ctm, alpha, None);

    Ok(DecodedImage {
        width: dst.width,
        height: dst.height,
        components: 4,
        pixels: dst.pixels,
    })
}

// ---------------------------------------------------------------------
// Little-endian byte reader for the binary fixture format.
// ---------------------------------------------------------------------

#[allow(dead_code)] // retained for a future-phase public-API binding
struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    #[allow(dead_code)] // retained for a future-phase public-API binding
    fn new(buf: &'a [u8]) -> Self {
        Reader { buf, pos: 0 }
    }

    #[allow(dead_code)] // retained for a future-phase public-API binding
    fn read_u8(&mut self) -> Result<u8, Error> {
        if self.pos >= self.buf.len() {
            return Err(Error::Truncated);
        }
        let v = self.buf[self.pos];
        self.pos += 1;
        Ok(v)
    }

    #[allow(dead_code)] // retained for a future-phase public-API binding
    fn read_u32_le(&mut self) -> Result<u32, Error> {
        if self.pos + 4 > self.buf.len() {
            return Err(Error::Truncated);
        }
        let bytes: [u8; 4] = self.buf[self.pos..self.pos + 4]
            .try_into()
            .expect("4-byte slice");
        self.pos += 4;
        Ok(u32::from_le_bytes(bytes))
    }

    #[allow(dead_code)] // retained for a future-phase public-API binding
    fn read_f64_le(&mut self) -> Result<f64, Error> {
        if self.pos + 8 > self.buf.len() {
            return Err(Error::Truncated);
        }
        let bytes: [u8; 8] = self.buf[self.pos..self.pos + 8]
            .try_into()
            .expect("8-byte slice");
        self.pos += 8;
        Ok(f64::from_le_bytes(bytes))
    }

    #[allow(dead_code)] // retained for a future-phase public-API binding
    fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>, Error> {
        if self.pos + n > self.buf.len() {
            return Err(Error::Truncated);
        }
        let v = self.buf[self.pos..self.pos + n].to_vec();
        self.pos += n;
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_dst(width: u32, height: u32) -> Raster {
        Raster {
            pixels: vec![0u8; (width as usize) * (height as usize) * 4],
            width,
            height,
        }
    }

    fn red_src(width: u32, height: u32) -> ImageSource {
        let mut pixels = Vec::with_capacity(
            (width as usize) * (height as usize) * 4);
        for _ in 0..(width as usize) * (height as usize) {
            pixels.extend_from_slice(&[255, 0, 0, 255]);
        }
        ImageSource { width, height, pixels }
    }

    #[test]
    fn zero_sized_dst_no_ops() {
        let mut dst = solid_dst(0, 0);
        let src = red_src(4, 4);
        composite(&mut dst, &src,
                  Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0), 1.0, None);
        assert!(dst.pixels.is_empty());
    }

    #[test]
    fn zero_sized_src_leaves_dst_zeroed() {
        let mut dst = solid_dst(4, 4);
        let src = ImageSource::default();  // 0×0
        composite(&mut dst, &src,
                  Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0), 1.0, None);
        assert!(dst.pixels.iter().all(|&p| p == 0));
    }

    #[test]
    fn singular_ctm_is_no_op() {
        let mut dst = solid_dst(8, 8);
        let src = red_src(4, 4);
        // det = 0
        composite(&mut dst, &src,
                  Matrix::new(0.0, 0.0, 0.0, 8.0, 0.0, 0.0), 1.0, None);
        assert!(dst.pixels.iter().all(|&p| p == 0));
    }

    #[test]
    fn global_alpha_zero_is_no_op() {
        let mut dst = solid_dst(8, 8);
        let src = red_src(8, 8);
        composite(&mut dst, &src,
                  Matrix::new(8.0, 0.0, 0.0, 8.0, 0.0, 0.0), 0.0, None);
        assert!(dst.pixels.iter().all(|&p| p == 0));
    }

    #[test]
    fn axis_aligned_solid_red_paints_red() {
        let mut dst = solid_dst(4, 4);
        let src = red_src(4, 4);
        composite(&mut dst, &src,
                  Matrix::new(4.0, 0.0, 0.0, 4.0, 0.0, 0.0), 1.0, None);
        for chunk in dst.pixels.chunks_exact(4) {
            assert_eq!(chunk[0], 255);
            assert_eq!(chunk[1], 0);
            assert_eq!(chunk[2], 0);
            assert_eq!(chunk[3], 255);
        }
    }

    #[test]
    fn decode_rejects_bad_magic() {
        let bytes = vec![0xFFu8; 76];
        assert_eq!(decode(&bytes), Err(Error::BadMagic));
    }

    #[test]
    fn decode_rejects_unsupported_version() {
        let mut bytes = vec![0u8; 76];
        bytes[0] = 0xCC;
        bytes[1] = 9;
        assert_eq!(decode(&bytes), Err(Error::UnsupportedVersion(9)));
    }

    #[test]
    fn decode_rejects_truncated_header() {
        let bytes = vec![0xCCu8, 1, 0, 0]; // 4 bytes — header is 76
        assert_eq!(decode(&bytes), Err(Error::Truncated));
    }

    #[test]
    fn nan_alpha_clamps_to_zero_no_op() {
        let mut dst = solid_dst(4, 4);
        let src = red_src(4, 4);
        composite(&mut dst, &src,
                  Matrix::new(4.0, 0.0, 0.0, 4.0, 0.0, 0.0),
                  f64::NAN, None);
        assert!(dst.pixels.iter().all(|&p| p == 0));
    }
}
