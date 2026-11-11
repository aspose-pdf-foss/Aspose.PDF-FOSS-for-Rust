// Decode-only adapter for the BMP (Microsoft Windows Bitmap)
// image container. Reads a complete BMP byte stream and returns
// a uniform RGBA8 pixel buffer, row-major top-to-bottom.
//
// Scope is narrow by design — only two (biBitCount,
// biCompression) pairs are accepted:
//
//   * (24, BI_RGB)         — 3 bytes per pixel, B G R per
//                            scanline, rows padded up to the
//                            next 4-byte boundary, alpha widened
//                            to 0xFF on output.
//   * (32, BI_BITFIELDS)   — 4 bytes per pixel, B G R A per
//                            scanline, no row padding, with the
//                            three canonical channel masks
//                            R=0x00FF0000 / G=0x0000FF00 /
//                            B=0x000000FF immediately after
//                            BITMAPINFOHEADER. Alpha at offset +3
//                            is preserved verbatim.
//
// Anything else (palette BMPs, 16-bit RGB555/RGB565, RLE
// variants, JPEG/PNG payloads, top-down layouts, BITMAPCOREHEADER
// / BITMAPV4HEADER / BITMAPV5HEADER, non-canonical channel masks,
// 4-mask BITFIELDS) is rejected with a precise Error variant.
//
// BMP scanlines are stored bottom-to-top in the file (positive
// biHeight); this decoder reverses that order on output so the
// emitted RGBA buffer is the row-major top-to-bottom shape every
// lossy primitive's DecodedImage carries.

use std::fmt;

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const BI_RGB: u32 = 0;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const BI_BITFIELDS: u32 = 3;

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const FILE_HEADER_SIZE: usize = 14;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const INFO_HEADER_SIZE: usize = 40;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const BITFIELDS_BLOCK_SIZE: usize = 12;

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const STD_RED_MASK: u32 = 0x00FF_0000;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const STD_GREEN_MASK: u32 = 0x0000_FF00;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const STD_BLUE_MASK: u32 = 0x0000_00FF;

/// Result of [`decode`] — uniform RGBA8 pixel buffer, row-major
/// top-to-bottom (R before G before B before A). `components` is
/// always 4; the field exists for parity with peer
/// `DecodedImage` shapes carried by other lossy primitives.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub struct DecodedImage {
    pub width: usize,
    pub height: usize,
    pub components: u8,
    pub pixels: Vec<u8>,
}

/// Errors returned by [`decode`] on out-of-scope or structurally
/// malformed BMP input. Variants name the offending field so
/// callers can surface a precise diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub enum Error {
    /// Input shorter than 14 bytes (BITMAPFILEHEADER won't fit)
    /// or its first two bytes are not "BM".
    NotABmp,
    /// Input shorter than 14 + 40 = 54 bytes (BITMAPINFOHEADER
    /// won't fit).
    InfoHeaderTruncated,
    /// `biSize` not equal to 40. BITMAPCOREHEADER (12),
    /// BITMAPV4HEADER (108), BITMAPV5HEADER (124) and any other
    /// header size are out of scope.
    UnsupportedInfoHeaderSize { bi_size: u32 },
    /// `biWidth` not strictly positive.
    InvalidWidth { bi_width: i32 },
    /// `biHeight` not strictly positive (top-down layouts with
    /// negative `biHeight` are out of scope).
    InvalidHeight { bi_height: i32 },
    /// `biPlanes` not equal to 1.
    InvalidPlanes { bi_planes: u16 },
    /// `biBitCount` not in {24, 32}.
    UnsupportedBitCount { bi_bit_count: u16 },
    /// `(biBitCount, biCompression)` is not one of the two
    /// in-scope pairs (24, BI_RGB) or (32, BI_BITFIELDS).
    UnsupportedCompression { bi_bit_count: u16, bi_compression: u32 },
    /// `biClrUsed` non-zero — palette BMPs are out of scope.
    PaletteNotSupported { bi_clr_used: u32 },
    /// `biCompression == BI_BITFIELDS` but the 12-byte channel
    /// mask block immediately after BITMAPINFOHEADER does not
    /// fit in the input.
    BitfieldsBlockTruncated,
    /// BITFIELDS channel masks are not the canonical
    /// (R=0x00FF0000, G=0x0000FF00, B=0x000000FF) triple.
    UnsupportedChannelMasks { red: u32, green: u32, blue: u32 },
    /// `bfOffBits` smaller than the on-disk header extent
    /// (54 bytes for BI_RGB, 66 bytes for BI_BITFIELDS).
    InvalidPixelOffset { bf_off_bits: u32, minimum: u32 },
    /// Declared pixel-data extent (`bfOffBits + height *
    /// row_stride`) runs past EOF.
    PixelDataTruncated { required: usize, got: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotABmp => write!(
                f, "bmp_decoder: not a BMP (input too short or bfType != 'BM')"),
            Error::InfoHeaderTruncated => write!(
                f, "bmp_decoder: BITMAPINFOHEADER does not fit in input"),
            Error::UnsupportedInfoHeaderSize { bi_size } => write!(
                f, "bmp_decoder: unsupported biSize {bi_size} (only 40 is in scope)"),
            Error::InvalidWidth { bi_width } => write!(
                f, "bmp_decoder: biWidth {bi_width} must be > 0"),
            Error::InvalidHeight { bi_height } => write!(
                f, "bmp_decoder: biHeight {bi_height} must be > 0 (top-down BMPs are out of scope)"),
            Error::InvalidPlanes { bi_planes } => write!(
                f, "bmp_decoder: biPlanes {bi_planes} must be 1"),
            Error::UnsupportedBitCount { bi_bit_count } => write!(
                f, "bmp_decoder: biBitCount {bi_bit_count} not in {{24, 32}}"),
            Error::UnsupportedCompression { bi_bit_count, bi_compression } => write!(
                f, "bmp_decoder: unsupported (biBitCount={bi_bit_count}, biCompression={bi_compression}) pair (only (24, BI_RGB) and (32, BI_BITFIELDS) are in scope)"),
            Error::PaletteNotSupported { bi_clr_used } => write!(
                f, "bmp_decoder: biClrUsed {bi_clr_used} != 0 (palette BMPs are out of scope)"),
            Error::BitfieldsBlockTruncated => write!(
                f, "bmp_decoder: BITFIELDS 12-byte channel mask block does not fit in input"),
            Error::UnsupportedChannelMasks { red, green, blue } => write!(
                f, "bmp_decoder: unsupported BITFIELDS channel masks (R=0x{red:08X}, G=0x{green:08X}, B=0x{blue:08X}); only the canonical (0x00FF0000, 0x0000FF00, 0x000000FF) is in scope"),
            Error::InvalidPixelOffset { bf_off_bits, minimum } => write!(
                f, "bmp_decoder: bfOffBits {bf_off_bits} < minimum {minimum}"),
            Error::PixelDataTruncated { required, got } => write!(
                f, "bmp_decoder: pixel data extent {required} exceeds input length {got}"),
        }
    }
}

impl std::error::Error for Error {}

/// Decode a complete BMP byte stream into an RGBA8 pixel buffer.
/// Returns a [`DecodedImage`] whose `pixels` field is
/// `width * height * 4` bytes, row-major top-to-bottom, R before
/// G before B before A. See the [`Error`] enum for the exhaustive
/// rejection model.
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub fn decode(input: &[u8]) -> Result<DecodedImage, Error> {
    // ---- BITMAPFILEHEADER (14 bytes) ----
    if input.len() < FILE_HEADER_SIZE {
        return Err(Error::NotABmp);
    }
    if input[0] != b'B' || input[1] != b'M' {
        return Err(Error::NotABmp);
    }
    // bfSize (bytes 2..6) is advisory; real-world writers
    // occasionally miscompute it. bfReserved1/2 (bytes 6..10)
    // are ignored.
    let bf_off_bits = read_u32_le(input, 10);

    // ---- BITMAPINFOHEADER (40 bytes, starting at offset 14) ----
    if input.len() < FILE_HEADER_SIZE + INFO_HEADER_SIZE {
        return Err(Error::InfoHeaderTruncated);
    }
    let bi_size = read_u32_le(input, 14);
    if bi_size != INFO_HEADER_SIZE as u32 {
        return Err(Error::UnsupportedInfoHeaderSize { bi_size });
    }
    let bi_width = read_i32_le(input, 18);
    if bi_width <= 0 {
        return Err(Error::InvalidWidth { bi_width });
    }
    let bi_height = read_i32_le(input, 22);
    if bi_height <= 0 {
        return Err(Error::InvalidHeight { bi_height });
    }
    let bi_planes = read_u16_le(input, 26);
    if bi_planes != 1 {
        return Err(Error::InvalidPlanes { bi_planes });
    }
    let bi_bit_count = read_u16_le(input, 28);
    if bi_bit_count != 24 && bi_bit_count != 32 {
        return Err(Error::UnsupportedBitCount { bi_bit_count });
    }
    let bi_compression = read_u32_le(input, 30);
    let valid_pair = (bi_bit_count == 24 && bi_compression == BI_RGB)
        || (bi_bit_count == 32 && bi_compression == BI_BITFIELDS);
    if !valid_pair {
        return Err(Error::UnsupportedCompression { bi_bit_count, bi_compression });
    }
    // biSizeImage at +34, biXPelsPerMeter at +38, biYPelsPerMeter
    // at +42 are advisory / ignored.
    let bi_clr_used = read_u32_le(input, 46);
    if bi_clr_used != 0 {
        return Err(Error::PaletteNotSupported { bi_clr_used });
    }
    // biClrImportant at +50 is ignored.

    // ---- BI_BITFIELDS masks (12 bytes at offset 54) ----
    let header_extent = if bi_compression == BI_BITFIELDS {
        if input.len() < FILE_HEADER_SIZE + INFO_HEADER_SIZE + BITFIELDS_BLOCK_SIZE {
            return Err(Error::BitfieldsBlockTruncated);
        }
        let red = read_u32_le(input, 54);
        let green = read_u32_le(input, 58);
        let blue = read_u32_le(input, 62);
        if red != STD_RED_MASK || green != STD_GREEN_MASK || blue != STD_BLUE_MASK {
            return Err(Error::UnsupportedChannelMasks { red, green, blue });
        }
        FILE_HEADER_SIZE + INFO_HEADER_SIZE + BITFIELDS_BLOCK_SIZE
    } else {
        FILE_HEADER_SIZE + INFO_HEADER_SIZE
    };

    // ---- Pixel offset bounds ----
    if (bf_off_bits as usize) < header_extent {
        return Err(Error::InvalidPixelOffset {
            bf_off_bits,
            minimum: header_extent as u32,
        });
    }

    // ---- Pixel data layout ----
    let width = bi_width as usize;
    let height = bi_height as usize;
    let bytes_per_pixel = (bi_bit_count / 8) as usize;
    let row_data_bytes = width * bytes_per_pixel;
    let row_padding = (4 - (row_data_bytes % 4)) % 4;
    let row_stride = row_data_bytes + row_padding;

    let pixel_extent = (bf_off_bits as usize)
        .saturating_add(height.saturating_mul(row_stride));
    if pixel_extent > input.len() {
        return Err(Error::PixelDataTruncated {
            required: pixel_extent,
            got: input.len(),
        });
    }

    // ---- Channel translation + bottom-up row inversion ----
    //
    // BMP stores scanlines bottom-to-top; the on-disk row at
    // offset `bfOffBits + 0 * row_stride` is the IMAGE BOTTOM,
    // and the row at `bfOffBits + (height - 1) * row_stride` is
    // the IMAGE TOP. We emit row-major top-to-bottom, so when
    // walking the on-disk rows from k=0..height we place each
    // into output row `(height - 1 - k)`.
    let mut pixels = vec![0u8; width * height * 4];
    let pixel_base = bf_off_bits as usize;

    match bi_bit_count {
        24 => {
            // BI_RGB: B G R per pixel, alpha widened to 0xFF.
            for k in 0..height {
                let row_off = pixel_base + k * row_stride;
                let out_y = height - 1 - k;
                let out_base = out_y * width * 4;
                for x in 0..width {
                    let s = row_off + x * 3;
                    let d = out_base + x * 4;
                    pixels[d]     = input[s + 2]; // R
                    pixels[d + 1] = input[s + 1]; // G
                    pixels[d + 2] = input[s];     // B
                    pixels[d + 3] = 0xFF;
                }
            }
        }
        32 => {
            // BI_BITFIELDS w/ standard masks: B G R A per pixel,
            // alpha at offset +3 preserved verbatim.
            for k in 0..height {
                let row_off = pixel_base + k * row_stride;
                let out_y = height - 1 - k;
                let out_base = out_y * width * 4;
                for x in 0..width {
                    let s = row_off + x * 4;
                    let d = out_base + x * 4;
                    pixels[d]     = input[s + 2]; // R
                    pixels[d + 1] = input[s + 1]; // G
                    pixels[d + 2] = input[s];     // B
                    pixels[d + 3] = input[s + 3]; // A
                }
            }
        }
        _ => unreachable!(),
    }

    Ok(DecodedImage {
        width,
        height,
        components: 4,
        pixels,
    })
}

#[inline]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn read_u16_le(buf: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([buf[off], buf[off + 1]])
}

#[inline]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn read_u32_le(buf: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]])
}

#[inline]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn read_i32_le(buf: &[u8], off: usize) -> i32 {
    i32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]])
}

// =====================  Tests  ==============================

#[cfg(test)]
mod tests {
    use super::*;

    // Hand-rolled 24-bit BI_RGB writer mirroring
    // generate_fixtures.py — used so unit tests stay independent
    // of vendored fixtures.
    fn write_bgr24(width: usize, height: usize, rgba: &[u8]) -> Vec<u8> {
        assert_eq!(rgba.len(), width * height * 4);
        let row_data = width * 3;
        let pad = (4 - (row_data % 4)) % 4;
        let stride = row_data + pad;
        let pixel_bytes = height * stride;
        let bf_off = (FILE_HEADER_SIZE + INFO_HEADER_SIZE) as u32;
        let bf_size = bf_off + pixel_bytes as u32;

        let mut out = Vec::with_capacity(bf_size as usize);
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&bf_size.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&bf_off.to_le_bytes());

        out.extend_from_slice(&(INFO_HEADER_SIZE as u32).to_le_bytes());
        out.extend_from_slice(&(width as i32).to_le_bytes());
        out.extend_from_slice(&(height as i32).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&24u16.to_le_bytes());
        out.extend_from_slice(&BI_RGB.to_le_bytes());
        out.extend_from_slice(&(pixel_bytes as u32).to_le_bytes());
        out.extend_from_slice(&2835i32.to_le_bytes());
        out.extend_from_slice(&2835i32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());

        for src_y in (0..height).rev() {
            for x in 0..width {
                let i = (src_y * width + x) * 4;
                out.push(rgba[i + 2]); // B
                out.push(rgba[i + 1]); // G
                out.push(rgba[i]);     // R
            }
            out.resize(out.len() + pad, 0);
        }
        out
    }

    fn write_bgra32(width: usize, height: usize, rgba: &[u8]) -> Vec<u8> {
        assert_eq!(rgba.len(), width * height * 4);
        let stride = width * 4;
        let pixel_bytes = height * stride;
        let bf_off = (FILE_HEADER_SIZE + INFO_HEADER_SIZE + BITFIELDS_BLOCK_SIZE) as u32;
        let bf_size = bf_off + pixel_bytes as u32;

        let mut out = Vec::with_capacity(bf_size as usize);
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&bf_size.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&bf_off.to_le_bytes());

        out.extend_from_slice(&(INFO_HEADER_SIZE as u32).to_le_bytes());
        out.extend_from_slice(&(width as i32).to_le_bytes());
        out.extend_from_slice(&(height as i32).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&BI_BITFIELDS.to_le_bytes());
        out.extend_from_slice(&(pixel_bytes as u32).to_le_bytes());
        out.extend_from_slice(&2835i32.to_le_bytes());
        out.extend_from_slice(&2835i32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());

        out.extend_from_slice(&STD_RED_MASK.to_le_bytes());
        out.extend_from_slice(&STD_GREEN_MASK.to_le_bytes());
        out.extend_from_slice(&STD_BLUE_MASK.to_le_bytes());

        for src_y in (0..height).rev() {
            for x in 0..width {
                let i = (src_y * width + x) * 4;
                out.push(rgba[i + 2]); // B
                out.push(rgba[i + 1]); // G
                out.push(rgba[i]);     // R
                out.push(rgba[i + 3]); // A
            }
        }
        out
    }

    // ------ Error paths.

    #[test]
    fn empty_input_rejected() {
        assert_eq!(decode(&[]).unwrap_err(), Error::NotABmp);
    }

    #[test]
    fn short_input_rejected() {
        // 13 bytes — one byte short of the file header.
        assert_eq!(decode(&[0u8; 13]).unwrap_err(), Error::NotABmp);
    }

    #[test]
    fn wrong_signature_rejected() {
        let mut buf = vec![0u8; 14];
        buf[0] = b'X';
        buf[1] = b'M';
        assert_eq!(decode(&buf).unwrap_err(), Error::NotABmp);
    }

    #[test]
    fn info_header_truncated() {
        // 14 bytes of valid file header, no info header.
        let mut buf = vec![0u8; 14];
        buf[0] = b'B';
        buf[1] = b'M';
        assert_eq!(decode(&buf).unwrap_err(), Error::InfoHeaderTruncated);
    }

    #[test]
    fn unsupported_info_header_size() {
        // BITMAPV4HEADER = 108. Build a valid BMP then poke
        // biSize so we exercise the rejection path.
        let mut buf = write_bgr24(2, 2, &[0u8; 16]);
        buf[14..18].copy_from_slice(&108u32.to_le_bytes());
        assert_eq!(
            decode(&buf).unwrap_err(),
            Error::UnsupportedInfoHeaderSize { bi_size: 108 });
    }

    #[test]
    fn negative_height_rejected() {
        let mut buf = write_bgr24(2, 2, &[0u8; 16]);
        buf[22..26].copy_from_slice(&(-2i32).to_le_bytes());
        assert_eq!(
            decode(&buf).unwrap_err(),
            Error::InvalidHeight { bi_height: -2 });
    }

    #[test]
    fn unsupported_bit_count() {
        let mut buf = write_bgr24(2, 2, &[0u8; 16]);
        buf[28..30].copy_from_slice(&8u16.to_le_bytes());
        assert_eq!(
            decode(&buf).unwrap_err(),
            Error::UnsupportedBitCount { bi_bit_count: 8 });
    }

    #[test]
    fn invalid_compression_pair() {
        // 24-bit + BI_BITFIELDS is not in scope.
        let mut buf = write_bgr24(2, 2, &[0u8; 16]);
        buf[30..34].copy_from_slice(&BI_BITFIELDS.to_le_bytes());
        assert_eq!(
            decode(&buf).unwrap_err(),
            Error::UnsupportedCompression {
                bi_bit_count: 24,
                bi_compression: BI_BITFIELDS,
            });
    }

    #[test]
    fn palette_not_supported() {
        let mut buf = write_bgr24(2, 2, &[0u8; 16]);
        buf[46..50].copy_from_slice(&8u32.to_le_bytes());
        assert_eq!(
            decode(&buf).unwrap_err(),
            Error::PaletteNotSupported { bi_clr_used: 8 });
    }

    #[test]
    fn nonstandard_channel_masks_rejected() {
        let mut buf = write_bgra32(2, 2, &[0u8; 16]);
        // Swap red/blue masks — out of scope.
        buf[54..58].copy_from_slice(&STD_BLUE_MASK.to_le_bytes());
        buf[62..66].copy_from_slice(&STD_RED_MASK.to_le_bytes());
        assert_eq!(
            decode(&buf).unwrap_err(),
            Error::UnsupportedChannelMasks {
                red: STD_BLUE_MASK,
                green: STD_GREEN_MASK,
                blue: STD_RED_MASK,
            });
    }

    #[test]
    fn pixel_data_truncated() {
        let mut buf = write_bgr24(4, 4, &[0u8; 64]);
        buf.truncate(buf.len() - 3); // cut off last few bytes
        match decode(&buf).unwrap_err() {
            Error::PixelDataTruncated { .. } => {}
            other => panic!("expected PixelDataTruncated, got {other:?}"),
        }
    }

    // ------ Happy paths.

    #[test]
    fn bgr24_roundtrip_widens_alpha() {
        // 2x2 image: arbitrary RGB, alpha varies but is dropped
        // by the encoder and widened back to 0xFF on decode.
        let src: Vec<u8> = vec![
            10, 20, 30, 0x11,    50, 60, 70, 0x22,
            90, 100, 110, 0x33,  130, 140, 150, 0x44,
        ];
        let bmp = write_bgr24(2, 2, &src);
        let decoded = decode(&bmp).unwrap();
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 2);
        assert_eq!(decoded.components, 4);
        // Alpha must be widened to 0xFF; RGB must round-trip.
        let expected: Vec<u8> = vec![
            10, 20, 30, 0xFF,    50, 60, 70, 0xFF,
            90, 100, 110, 0xFF,  130, 140, 150, 0xFF,
        ];
        assert_eq!(decoded.pixels, expected);
    }

    #[test]
    fn bgra32_preserves_alpha() {
        let src: Vec<u8> = vec![
            10, 20, 30, 0x11,    50, 60, 70, 0x22,
            90, 100, 110, 0x33,  130, 140, 150, 0x44,
        ];
        let bmp = write_bgra32(2, 2, &src);
        let decoded = decode(&bmp).unwrap();
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 2);
        assert_eq!(decoded.components, 4);
        // BI_BITFIELDS preserves alpha verbatim.
        assert_eq!(decoded.pixels, src);
    }

    #[test]
    fn bgr24_padded_width_3() {
        // width=3 → 9 data bytes/row + 3 padding = 12 stride.
        // Exercises the maximum row-padding case.
        let mut src = Vec::new();
        for y in 0..2u8 {
            for x in 0..3u8 {
                src.extend_from_slice(&[x * 80, y * 100, 50, 0xFF]);
            }
        }
        let bmp = write_bgr24(3, 2, &src);
        let decoded = decode(&bmp).unwrap();
        assert_eq!(decoded.pixels, src);
    }

    #[test]
    fn bgr24_padded_width_2() {
        // width=2 → 6 data bytes/row + 2 padding = 8 stride.
        let src: Vec<u8> = vec![
            5, 15, 25, 0xFF,   35, 45, 55, 0xFF,
            65, 75, 85, 0xFF,  95, 105, 115, 0xFF,
            125, 135, 145, 0xFF, 155, 165, 175, 0xFF,
        ];
        let bmp = write_bgr24(2, 3, &src);
        let decoded = decode(&bmp).unwrap();
        assert_eq!(decoded.pixels, src);
    }

    #[test]
    fn bgr24_padded_width_5() {
        // width=5 → 15 data bytes/row + 1 padding = 16 stride.
        let mut src = Vec::new();
        for y in 0..3u8 {
            for x in 0..5u8 {
                src.extend_from_slice(&[x * 50, y * 80, x * y, 0xFF]);
            }
        }
        let bmp = write_bgr24(5, 3, &src);
        let decoded = decode(&bmp).unwrap();
        assert_eq!(decoded.pixels, src);
    }

    #[test]
    fn bottom_up_row_order_inverted_on_decode() {
        // Distinct-row image: row 0 = red, row 1 = green, row 2
        // = blue. On disk BMP stores row 2 first, then row 1,
        // then row 0; the decoder must invert that ordering so
        // the output's row 0 IS the source's row 0.
        let red = [255u8, 0, 0, 0xFF];
        let green = [0u8, 255, 0, 0xFF];
        let blue = [0u8, 0, 255, 0xFF];
        let mut src = Vec::new();
        for c in [red, green, blue] {
            for _ in 0..4 { src.extend_from_slice(&c); }
        }
        let bmp = write_bgr24(4, 3, &src);
        let decoded = decode(&bmp).unwrap();
        // Row 0 stays red — confirms inversion happened.
        assert_eq!(&decoded.pixels[0..16],
                   &[255, 0, 0, 0xFF].repeat(4));
        // Row 2 stays blue.
        assert_eq!(&decoded.pixels[32..48],
                   &[0, 0, 255, 0xFF].repeat(4));
    }

    #[test]
    fn single_pixel_bgra32() {
        let src: Vec<u8> = vec![123, 45, 67, 89];
        let bmp = write_bgra32(1, 1, &src);
        let decoded = decode(&bmp).unwrap();
        assert_eq!(decoded.width, 1);
        assert_eq!(decoded.height, 1);
        assert_eq!(decoded.pixels, src);
    }
}
