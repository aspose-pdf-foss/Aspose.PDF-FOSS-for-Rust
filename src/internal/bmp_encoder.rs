// Encode-only adapter for the BMP (Microsoft Windows Bitmap)
// image container. Mirrors the in-scope variants of the paired
// bmp_decoder primitive:
//
//   * 24-bit BGR with biCompression = BI_RGB. Three bytes per
//     pixel, rows padded up to the next 4-byte boundary, alpha
//     dropped on emit (round-trip widens it back to 0xFF on
//     decode).
//   * 32-bit BGRA with biCompression = BI_BITFIELDS plus the
//     canonical R/G/B masks (0x00FF0000, 0x0000FF00,
//     0x000000FF). Four bytes per pixel, no row padding, alpha
//     preserved end-to-end.
//
// Output layout: BITMAPFILEHEADER + BITMAPINFOHEADER (40-byte) +
// optional 12-byte BITFIELDS block + bottom-up scanlines.

use std::fmt;

const FILE_HEADER_SIZE: u32 = 14;
const INFO_HEADER_SIZE: u32 = 40;
const BITFIELDS_BLOCK_SIZE: u32 = 12;

const BI_RGB: u32 = 0;
const BI_BITFIELDS: u32 = 3;

const RED_MASK: u32 = 0x00FF_0000;
const GREEN_MASK: u32 = 0x0000_FF00;
const BLUE_MASK: u32 = 0x0000_00FF;

// 2835 pixels-per-meter ≈ 72 DPI — the de-facto value mature BMP
// writers emit. Optional for BI_RGB, required for BI_BITFIELDS;
// emitting non-zero is the maximum-compatibility choice.
const PELS_PER_METER: u32 = 2835;

// BMP biWidth / biHeight are signed 32-bit fields with positive
// values reserved for the in-scope (bottom-up) layout.
const BMP_MAX_DIM: u64 = 0x7FFF_FFFF;

/// BMP output variant. Discriminants match the BITMAPINFOHEADER
/// `biBitCount` byte for the variant — same wire-byte pun the
/// .bencfix fixture format uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ColorType {
    Bgr = 24,
    Bgra = 32,
}

/// Encoder-side options. `color_type` defaults to
/// [`ColorType::Bgra`] (the alpha-preserving path) — the
/// middle-of-the-road safe choice for callers that haven't
/// thought about alpha yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Options {
    pub color_type: Option<ColorType>,
}

/// Errors returned by [`encode`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    ZeroWidth,
    ZeroHeight,
    WidthOutOfRange { width: u64 },
    HeightOutOfRange { height: u64 },
    WrongRgbaLength { expected: usize, got: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ZeroWidth => write!(
                f, "bmp_encoder: width is zero"),
            Error::ZeroHeight => write!(
                f, "bmp_encoder: height is zero"),
            Error::WidthOutOfRange { width } => write!(
                f, "bmp_encoder: width {width} not in 1..=2^31 - 1"),
            Error::HeightOutOfRange { height } => write!(
                f, "bmp_encoder: height {height} not in 1..=2^31 - 1"),
            Error::WrongRgbaLength { expected, got } => write!(
                f, "bmp_encoder: rgba8 length {got} != {expected}"),
        }
    }
}

impl std::error::Error for Error {}

/// Encode `rgba8` (row-major top-to-bottom, R G B A) into a
/// complete BMP byte stream. Returns [`Error`] on out-of-range
/// dimensions or wrong buffer length; otherwise infallible.
pub fn encode(
    width: u32,
    height: u32,
    rgba8: &[u8],
    options: Options,
) -> Result<Vec<u8>, Error> {
    if width == 0 {
        return Err(Error::ZeroWidth);
    }
    if height == 0 {
        return Err(Error::ZeroHeight);
    }
    // u32 max is 2^32 - 1, BMP biWidth/biHeight cap is 2^31 - 1.
    if (width as u64) > BMP_MAX_DIM {
        return Err(Error::WidthOutOfRange { width: width as u64 });
    }
    if (height as u64) > BMP_MAX_DIM {
        return Err(Error::HeightOutOfRange { height: height as u64 });
    }
    let expected_rgba = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .expect("width*height*4 fits in usize after dim bounds check");
    if rgba8.len() != expected_rgba {
        return Err(Error::WrongRgbaLength {
            expected: expected_rgba,
            got: rgba8.len(),
        });
    }

    let color_type = options.color_type.unwrap_or(ColorType::Bgra);
    let bytes_per_pixel: u32 = match color_type {
        ColorType::Bgr => 3,
        ColorType::Bgra => 4,
    };
    let row_data_bytes = width * bytes_per_pixel;
    let row_padding = (4 - (row_data_bytes % 4)) % 4;
    let row_stride = row_data_bytes + row_padding;
    let pixel_data_bytes = height * row_stride;
    let bitfields_block = match color_type {
        ColorType::Bgr => 0,
        ColorType::Bgra => BITFIELDS_BLOCK_SIZE,
    };
    let bf_off_bits = FILE_HEADER_SIZE + INFO_HEADER_SIZE + bitfields_block;
    let bf_size = bf_off_bits + pixel_data_bytes;

    let mut out = Vec::with_capacity(bf_size as usize);

    // BITMAPFILEHEADER (14 bytes).
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&bf_size.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // bfReserved1
    out.extend_from_slice(&0u16.to_le_bytes()); // bfReserved2
    out.extend_from_slice(&bf_off_bits.to_le_bytes());

    // BITMAPINFOHEADER (40 bytes).
    out.extend_from_slice(&INFO_HEADER_SIZE.to_le_bytes());
    out.extend_from_slice(&(width as i32).to_le_bytes());
    out.extend_from_slice(&(height as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());                       // biPlanes
    out.extend_from_slice(&((bytes_per_pixel * 8) as u16).to_le_bytes()); // biBitCount
    let bi_compression = match color_type {
        ColorType::Bgr => BI_RGB,
        ColorType::Bgra => BI_BITFIELDS,
    };
    out.extend_from_slice(&bi_compression.to_le_bytes());
    out.extend_from_slice(&pixel_data_bytes.to_le_bytes());           // biSizeImage
    out.extend_from_slice(&PELS_PER_METER.to_le_bytes());             // biXPelsPerMeter
    out.extend_from_slice(&PELS_PER_METER.to_le_bytes());             // biYPelsPerMeter
    out.extend_from_slice(&0u32.to_le_bytes());                       // biClrUsed
    out.extend_from_slice(&0u32.to_le_bytes());                       // biClrImportant

    // BITFIELDS block (12 bytes, BI_BITFIELDS only).
    if matches!(color_type, ColorType::Bgra) {
        out.extend_from_slice(&RED_MASK.to_le_bytes());
        out.extend_from_slice(&GREEN_MASK.to_le_bytes());
        out.extend_from_slice(&BLUE_MASK.to_le_bytes());
    }

    // Pixel data — bottom-up scanline order.
    let width_us = width as usize;
    let height_us = height as usize;
    let row_padding_us = row_padding as usize;

    match color_type {
        ColorType::Bgr => {
            for k in 0..height_us {
                // file row 0 = image bottom; file row height-1 = image top.
                let input_row = height_us - 1 - k;
                let mut src = input_row * width_us * 4;
                for _ in 0..width_us {
                    out.push(rgba8[src + 2]); // B
                    out.push(rgba8[src + 1]); // G
                    out.push(rgba8[src]);     // R
                    src += 4;
                }
                out.resize(out.len() + row_padding_us, 0);
            }
        }
        ColorType::Bgra => {
            for k in 0..height_us {
                let input_row = height_us - 1 - k;
                let mut src = input_row * width_us * 4;
                for _ in 0..width_us {
                    out.push(rgba8[src + 2]); // B
                    out.push(rgba8[src + 1]); // G
                    out.push(rgba8[src]);     // R
                    out.push(rgba8[src + 3]); // A
                    src += 4;
                }
            }
        }
    }

    debug_assert_eq!(out.len(), bf_size as usize);
    Ok(out)
}

#[cfg(test)]
mod tests {
    // Unit tests are deliberately self-contained — no cross-module
    // references — because this file is also pulled into the
    // integration-test crate via `#[path = ".../bmp_encoder.rs"]`,
    // where `crate::internal::bmp_decoder` does not resolve. The
    // round-trip pairing with bmp_decoder lives in the gate
    // driver under tests/internal/bmp_encoder_test.rs.
    use super::*;

    #[test]
    fn default_options_select_bgra() {
        let src: Vec<u8> = vec![1, 2, 3, 0x77];
        let bmp = encode(1, 1, &src, Options::default()).unwrap();
        // BITMAPINFOHEADER.biBitCount at offset 28..30 must be 32.
        assert_eq!(u16::from_le_bytes([bmp[28], bmp[29]]), 32);
    }

    #[test]
    fn zero_dim_rejected() {
        assert_eq!(encode(0, 1, &[0u8; 4], Options::default()).unwrap_err(),
                   Error::ZeroWidth);
        assert_eq!(encode(1, 0, &[0u8; 4], Options::default()).unwrap_err(),
                   Error::ZeroHeight);
    }

    #[test]
    fn wrong_rgba_length_rejected() {
        let err = encode(2, 2, &[0u8; 15], Options::default()).unwrap_err();
        assert_eq!(err, Error::WrongRgbaLength { expected: 16, got: 15 });
    }

    #[test]
    fn header_layout_bgra() {
        let src: Vec<u8> = vec![0u8; 4];
        let bmp = encode(1, 1, &src,
                         Options { color_type: Some(ColorType::Bgra) }).unwrap();
        assert_eq!(&bmp[..2], b"BM");
        // bfOffBits = 14 + 40 + 12 = 66.
        assert_eq!(u32::from_le_bytes([bmp[10], bmp[11], bmp[12], bmp[13]]), 66);
        // biCompression = BI_BITFIELDS (3).
        assert_eq!(u32::from_le_bytes([bmp[30], bmp[31], bmp[32], bmp[33]]), 3);
        // RED_MASK at offset 54.
        assert_eq!(u32::from_le_bytes([bmp[54], bmp[55], bmp[56], bmp[57]]),
                   RED_MASK);
    }

    #[test]
    fn header_layout_bgr_no_bitfields_block() {
        let src: Vec<u8> = vec![0u8; 4];
        let bmp = encode(1, 1, &src,
                         Options { color_type: Some(ColorType::Bgr) }).unwrap();
        // bfOffBits = 14 + 40 = 54.
        assert_eq!(u32::from_le_bytes([bmp[10], bmp[11], bmp[12], bmp[13]]), 54);
        // biCompression = BI_RGB (0).
        assert_eq!(u32::from_le_bytes([bmp[30], bmp[31], bmp[32], bmp[33]]), 0);
    }

    #[test]
    fn bgr_padding_widths() {
        // width=3 → 9 data bytes/row + 3 padding = 12 stride.
        // Total file = 14 + 40 + 2 * 12 = 78 bytes.
        let src: Vec<u8> = (0..24).collect(); // 3*2*4 = 24 bytes
        let bmp = encode(3, 2, &src,
                         Options { color_type: Some(ColorType::Bgr) }).unwrap();
        assert_eq!(bmp.len(), 14 + 40 + 24);
        // bfSize at offset 2 must equal output length.
        assert_eq!(u32::from_le_bytes([bmp[2], bmp[3], bmp[4], bmp[5]]),
                   bmp.len() as u32);
    }

    #[test]
    fn bottom_up_first_emitted_row_is_image_bottom() {
        // Distinct rows: red top, blue bottom. The first emitted
        // pixel-data byte for color_type Bgr should be blue's B (255).
        let red = [255u8, 0, 0, 0xFF];
        let blue = [0u8, 0, 255, 0xFF];
        let mut src = Vec::new();
        src.extend_from_slice(&red);   // y=0 (top)
        src.extend_from_slice(&blue);  // y=1 (bottom)
        let bmp = encode(1, 2, &src,
                         Options { color_type: Some(ColorType::Bgr) }).unwrap();
        // bfOffBits = 14 + 40 = 54. First pixel byte at 54 is the
        // image-bottom row's first B byte; bottom row is blue
        // → B = 255.
        assert_eq!(bmp[54], 255);
    }
}
