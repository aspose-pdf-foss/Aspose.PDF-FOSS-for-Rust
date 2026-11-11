// PNG (Portable Network Graphics) encoder — single image,
// 8-bit samples, three colour types (Grayscale, Rgb, Rgba),
// non-interlaced, filter type 0 (None) on every scanline,
// single IDAT chunk. Composes the frozen `flate` primitive
// for the IDAT zlib stream. std-only.
//
// Mirrors `png_decoder`'s read scope minus the inverse-only
// inputs (multi-filter / interlaced); the encode → decode
// round trip is byte-equality on RGBA8 (per-colour-type
// coercions documented in the shared spec yaml).
//
// All multi-byte integers in the PNG wire format are
// big-endian — distinct from TIFF.

use std::fmt;
use std::sync::OnceLock;

use super::flate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorType {
    #[allow(dead_code)] // retained for a future-phase public-API binding
    Grayscale = 0,
    Rgb = 2,
    Rgba = 6,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub color_type: ColorType,
}

impl Default for Options {
    fn default() -> Self {
        Self { color_type: ColorType::Rgba }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    ZeroDimension,
    DimensionTooLarge,
    PixelLengthMismatch { expected: usize, got: usize },
    NonGrayscaleInput { pixel_index: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ZeroDimension =>
                write!(f, "png_encoder: width or height is zero"),
            Error::DimensionTooLarge =>
                write!(f, "png_encoder: width or height exceeds 2^31 - 1"),
            Error::PixelLengthMismatch { expected, got } =>
                write!(f, "png_encoder: rgba8 length {got} != width*height*4 = {expected}"),
            Error::NonGrayscaleInput { pixel_index } =>
                write!(f,
                    "png_encoder: Grayscale color_type requires R=G=B but pixel {pixel_index} disagrees"),
        }
    }
}

impl std::error::Error for Error {}

const SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// Encode a single RGBA8 image into a PNG byte stream.
pub fn encode(
    width: u32,
    height: u32,
    rgba8: &[u8],
    options: &Options,
) -> Result<Vec<u8>, Error> {
    if width == 0 || height == 0 {
        return Err(Error::ZeroDimension);
    }
    if width > i32::MAX as u32 || height > i32::MAX as u32 {
        return Err(Error::DimensionTooLarge);
    }
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or(Error::DimensionTooLarge)?;
    if rgba8.len() != expected {
        return Err(Error::PixelLengthMismatch { expected, got: rgba8.len() });
    }

    let bpp = match options.color_type {
        ColorType::Grayscale => 1usize,
        ColorType::Rgb => 3usize,
        ColorType::Rgba => 4usize,
    };
    let row_samples = (width as usize) * bpp;
    let filtered_len = (height as usize) * (1 + row_samples);

    let mut filtered = Vec::with_capacity(filtered_len);
    for y in 0..(height as usize) {
        filtered.push(0); // filter type None
        let src_row = y * (width as usize) * 4;
        match options.color_type {
            ColorType::Rgba => {
                filtered.extend_from_slice(
                    &rgba8[src_row..src_row + (width as usize) * 4]);
            }
            ColorType::Rgb => {
                for x in 0..(width as usize) {
                    let base = src_row + x * 4;
                    filtered.push(rgba8[base]);
                    filtered.push(rgba8[base + 1]);
                    filtered.push(rgba8[base + 2]);
                }
            }
            ColorType::Grayscale => {
                for x in 0..(width as usize) {
                    let base = src_row + x * 4;
                    let r = rgba8[base];
                    let g = rgba8[base + 1];
                    let b = rgba8[base + 2];
                    if r != g || g != b {
                        return Err(Error::NonGrayscaleInput {
                            pixel_index: y * (width as usize) + x,
                        });
                    }
                    filtered.push(r);
                }
            }
        }
    }

    let idat_payload = flate::encode(&filtered);

    let mut out = Vec::with_capacity(8 + 25 + 12 + idat_payload.len() + 12);
    out.extend_from_slice(&SIGNATURE);

    let mut ihdr_payload = [0u8; 13];
    ihdr_payload[0..4].copy_from_slice(&width.to_be_bytes());
    ihdr_payload[4..8].copy_from_slice(&height.to_be_bytes());
    ihdr_payload[8] = 8; // bit depth
    ihdr_payload[9] = options.color_type as u8;
    ihdr_payload[10] = 0; // compression: Deflate
    ihdr_payload[11] = 0; // filter method 0
    ihdr_payload[12] = 0; // interlace none
    write_chunk(&mut out, *b"IHDR", &ihdr_payload);
    write_chunk(&mut out, *b"IDAT", &idat_payload);
    write_chunk(&mut out, *b"IEND", &[]);

    Ok(out)
}

fn write_chunk(out: &mut Vec<u8>, chunk_type: [u8; 4], payload: &[u8]) {
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    let crc_start = out.len();
    out.extend_from_slice(&chunk_type);
    out.extend_from_slice(payload);
    let crc = crc32(&out[crc_start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

fn crc32_table() -> &'static [u32; 256] {
    static TABLE: OnceLock<[u32; 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for n in 0..256u32 {
            let mut c = n;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 };
            }
            t[n as usize] = c;
        }
        t
    })
}

fn crc32(bytes: &[u8]) -> u32 {
    let table = crc32_table();
    let mut c: u32 = 0xFFFFFFFF;
    for &b in bytes {
        c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFFFFFF
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_dimension() {
        let err = encode(0, 1, &[], &Options::default()).unwrap_err();
        assert_eq!(err, Error::ZeroDimension);
    }

    #[test]
    fn rejects_pixel_length_mismatch() {
        let err = encode(2, 2, &[0; 8], &Options::default()).unwrap_err();
        assert!(matches!(err, Error::PixelLengthMismatch { .. }));
    }

    #[test]
    fn grayscale_rejects_non_grayscale() {
        let mut pixels = vec![0u8; 4];
        pixels[1] = 5; // R=0 G=5 B=0 A=0 — not grayscale
        let err = encode(
            1, 1, &pixels,
            &Options { color_type: ColorType::Grayscale },
        ).unwrap_err();
        assert!(matches!(err, Error::NonGrayscaleInput { .. }));
    }

    #[test]
    fn signature_and_chunk_sequence_are_valid() {
        let pixels = vec![0u8; 4];
        let png = encode(1, 1, &pixels, &Options::default()).unwrap();
        assert_eq!(&png[0..8], &SIGNATURE);
        // IHDR length should be 13.
        let ihdr_len =
            u32::from_be_bytes([png[8], png[9], png[10], png[11]]);
        assert_eq!(ihdr_len, 13);
        assert_eq!(&png[12..16], b"IHDR");
    }

    #[test]
    fn crc32_known_value() {
        // Known: CRC32 over ASCII "123456789" = 0xCBF43926.
        assert_eq!(crc32(b"123456789"), 0xCBF43926);
    }
}
