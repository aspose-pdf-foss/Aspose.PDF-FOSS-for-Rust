// PNG baseline decoder (PNG 1.2 / ISO/IEC 15948:2003). Pure
// function from a complete PNG byte stream to a uniform RGBA8
// pixel buffer. Composes the frozen `flate` primitive for IDAT
// zlib decompression.
//
// Scope: 8-bit samples; color types 0 (Grayscale), 2 (RGB), 6
// (RGBA); non-interlaced; all five filter inverses (None, Sub,
// Up, Average, Paeth). Out of scope: 1/2/4/16-bit depths,
// palette (color type 3), grayscale-with-alpha (color type 4),
// Adam7 interlace.

#![allow(clippy::needless_range_loop)] // index-arithmetic numeric kernels

use std::fmt;

use super::flate;

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub components: u8, // always 4 (RGBA8)
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub enum Error {
    NotPng,
    BadChunk,
    CrcMismatch,
    MissingIhdr,
    MissingIdat,
    MissingIend,
    UnsupportedDepth(u8),
    UnsupportedColorType(u8),
    UnsupportedCompression(u8),
    UnsupportedFilterMethod(u8),
    UnsupportedInterlace(u8),
    ZeroDimension,
    DecompressFailed,
    ScanlineLengthMismatch,
    UnknownFilterType(u8),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotPng => write!(f, "png_decoder: not a PNG (signature mismatch)"),
            Error::BadChunk => write!(f, "png_decoder: malformed chunk header"),
            Error::CrcMismatch => write!(f, "png_decoder: chunk CRC mismatch"),
            Error::MissingIhdr => write!(f, "png_decoder: missing IHDR chunk"),
            Error::MissingIdat => write!(f, "png_decoder: missing IDAT chunk"),
            Error::MissingIend => write!(f, "png_decoder: missing IEND chunk"),
            Error::UnsupportedDepth(d) =>
                write!(f, "png_decoder: unsupported bit depth {d} (8 only)"),
            Error::UnsupportedColorType(c) =>
                write!(f, "png_decoder: unsupported color type {c} (0/2/6 only)"),
            Error::UnsupportedCompression(c) =>
                write!(f, "png_decoder: unsupported compression method {c} (0 only)"),
            Error::UnsupportedFilterMethod(m) =>
                write!(f, "png_decoder: unsupported filter method {m} (0 only)"),
            Error::UnsupportedInterlace(i) =>
                write!(f, "png_decoder: unsupported interlace {i} (non-interlaced only)"),
            Error::ZeroDimension => write!(f, "png_decoder: zero width or height"),
            Error::DecompressFailed => write!(f, "png_decoder: IDAT zlib decompress failed"),
            Error::ScanlineLengthMismatch =>
                write!(f, "png_decoder: decompressed length != expected scanline bytes"),
            Error::UnknownFilterType(t) =>
                write!(f, "png_decoder: unknown filter type byte {t}"),
        }
    }
}

impl std::error::Error for Error {}

// ============================================================
// Public API
// ============================================================

/// One-shot decode of a complete PNG byte stream.
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
#[allow(unused_assignments)] // stale local; annotated to keep the build clean
pub fn decode(input: &[u8]) -> Result<DecodedImage, Error> {
    if input.len() < 8 || input[..8] != PNG_SIGNATURE {
        return Err(Error::NotPng);
    }

    let mut p = 8usize;
    let mut ihdr: Option<Ihdr> = None;
    let mut idat_blob: Vec<u8> = Vec::new();
    let mut saw_iend = false;

    while p < input.len() {
        if p + 8 > input.len() {
            return Err(Error::BadChunk);
        }
        let length = u32::from_be_bytes([
            input[p], input[p + 1], input[p + 2], input[p + 3],
        ]) as usize;
        if length > 0x7FFF_FFFF {
            return Err(Error::BadChunk);
        }
        let kind = &input[p + 4..p + 8];
        let data_start = p + 8;
        let data_end = data_start + length;
        let crc_end = data_end + 4;
        if crc_end > input.len() {
            return Err(Error::BadChunk);
        }
        let data = &input[data_start..data_end];
        let crc_stored = u32::from_be_bytes([
            input[data_end], input[data_end + 1],
            input[data_end + 2], input[data_end + 3],
        ]);
        let crc_calc = crc32_pair(kind, data);
        if crc_calc != crc_stored {
            return Err(Error::CrcMismatch);
        }
        match kind {
            b"IHDR" => {
                if ihdr.is_some() {
                    return Err(Error::BadChunk);
                }
                ihdr = Some(parse_ihdr(data)?);
            }
            b"IDAT" => {
                if ihdr.is_none() {
                    return Err(Error::MissingIhdr);
                }
                idat_blob.extend_from_slice(data);
            }
            b"IEND" => {
                saw_iend = true;
                p = crc_end;
                break;
            }
            _ => {
                // Ancillary chunks (tEXt, pHYs, gAMA, etc.) —
                // silently skipped per the v1 spec scope.
            }
        }
        p = crc_end;
    }

    let ihdr = ihdr.ok_or(Error::MissingIhdr)?;
    if !saw_iend {
        return Err(Error::MissingIend);
    }
    if idat_blob.is_empty() {
        return Err(Error::MissingIdat);
    }

    // Decompress the concatenated IDAT.
    let raw_filtered = flate::decode(&idat_blob)
        .map_err(|_| Error::DecompressFailed)?;

    let bpp = match ihdr.color_type {
        0 => 1,
        2 => 3,
        6 => 4,
        _ => unreachable!("color type validated in parse_ihdr"),
    };
    let row_data_bytes = ihdr.width as usize * bpp;
    let expected = ihdr.height as usize * (1 + row_data_bytes);
    if raw_filtered.len() != expected {
        return Err(Error::ScanlineLengthMismatch);
    }

    // Inverse filters → raw row bytes.
    let raw = inverse_filter(
        &raw_filtered,
        ihdr.width as usize,
        ihdr.height as usize,
        bpp,
    )?;

    // Photometric expansion to RGBA8.
    let pixels = expand_to_rgba8(&raw, ihdr.color_type, bpp);

    Ok(DecodedImage {
        width: ihdr.width,
        height: ihdr.height,
        components: 4,
        pixels,
    })
}

// ============================================================
// IHDR + chunk parsing
// ============================================================

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
struct Ihdr {
    width: u32,
    height: u32,
    color_type: u8,
}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn parse_ihdr(data: &[u8]) -> Result<Ihdr, Error> {
    if data.len() != 13 {
        return Err(Error::BadChunk);
    }
    let width = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    let height = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    let bit_depth = data[8];
    let color_type = data[9];
    let compression = data[10];
    let filter = data[11];
    let interlace = data[12];
    if width == 0 || height == 0 {
        return Err(Error::ZeroDimension);
    }
    if bit_depth != 8 {
        return Err(Error::UnsupportedDepth(bit_depth));
    }
    match color_type {
        0 | 2 | 6 => {}
        _ => return Err(Error::UnsupportedColorType(color_type)),
    }
    if compression != 0 {
        return Err(Error::UnsupportedCompression(compression));
    }
    if filter != 0 {
        return Err(Error::UnsupportedFilterMethod(filter));
    }
    if interlace != 0 {
        return Err(Error::UnsupportedInterlace(interlace));
    }
    Ok(Ihdr { width, height, color_type })
}

// ============================================================
// CRC32 (IEEE 802.3 / 0xEDB88320 reversed)
// ============================================================

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn crc32_pair(a: &[u8], b: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in a.iter().chain(b.iter()) {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = if crc & 1 != 0 { 0xEDB8_8320 } else { 0 };
            crc = (crc >> 1) ^ mask;
        }
    }
    !crc
}

// ============================================================
// Filter inverse (PNG 1.2 §6.3 + §9)
// ============================================================

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn inverse_filter(
    filtered: &[u8],
    width: usize,
    height: usize,
    bpp: usize,
) -> Result<Vec<u8>, Error> {
    let row_data = width * bpp;
    let row_total = 1 + row_data;
    let mut raw = Vec::with_capacity(height * row_data);
    let mut prev_row: Vec<u8> = vec![0u8; row_data];

    for r in 0..height {
        let row = &filtered[r * row_total..(r + 1) * row_total];
        let filter_type = row[0];
        let data = &row[1..];
        let mut current: Vec<u8> = Vec::with_capacity(row_data);
        match filter_type {
            0 => {
                // None.
                current.extend_from_slice(data);
            }
            1 => {
                // Sub: raw[i] = filtered[i] + raw[i - bpp].
                for i in 0..row_data {
                    let left = if i >= bpp { current[i - bpp] } else { 0 };
                    current.push(data[i].wrapping_add(left));
                }
            }
            2 => {
                // Up: raw[i] = filtered[i] + prior[i].
                for i in 0..row_data {
                    current.push(data[i].wrapping_add(prev_row[i]));
                }
            }
            3 => {
                // Average: raw[i] = filtered[i] +
                //   floor((raw[i - bpp] + prior[i]) / 2)
                for i in 0..row_data {
                    let left = if i >= bpp { current[i - bpp] } else { 0 };
                    let up = prev_row[i];
                    let avg = ((left as u16 + up as u16) / 2) as u8;
                    current.push(data[i].wrapping_add(avg));
                }
            }
            4 => {
                // Paeth.
                for i in 0..row_data {
                    let a = if i >= bpp { current[i - bpp] } else { 0 };
                    let b = prev_row[i];
                    let c = if i >= bpp { prev_row[i - bpp] } else { 0 };
                    let p = paeth(a, b, c);
                    current.push(data[i].wrapping_add(p));
                }
            }
            other => return Err(Error::UnknownFilterType(other)),
        }
        raw.extend_from_slice(&current);
        prev_row = current;
    }
    Ok(raw)
}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn paeth(a: u8, b: u8, c: u8) -> u8 {
    // Paeth predictor — intermediate sum and absolute distances
    // computed in signed 32-bit so the comparison is exact;
    // result is the 8-bit byte that wins.
    let pa = (b as i32 - c as i32).abs();
    let pb = (a as i32 - c as i32).abs();
    let pc = (a as i32 + b as i32 - 2 * c as i32).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

// ============================================================
// Photometric expansion
// ============================================================

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn expand_to_rgba8(raw: &[u8], color_type: u8, bpp: usize) -> Vec<u8> {
    let pixel_count = raw.len() / bpp;
    let mut out = Vec::with_capacity(pixel_count * 4);
    match color_type {
        0 => {
            // Grayscale: replicate sample into RGB, alpha=0xFF.
            for i in 0..pixel_count {
                let s = raw[i];
                out.push(s);
                out.push(s);
                out.push(s);
                out.push(0xFF);
            }
        }
        2 => {
            // RGB: pass-through, append alpha=0xFF.
            for i in 0..pixel_count {
                out.push(raw[i * 3]);
                out.push(raw[i * 3 + 1]);
                out.push(raw[i * 3 + 2]);
                out.push(0xFF);
            }
        }
        6 => {
            // RGBA: pass-through.
            out.extend_from_slice(raw);
        }
        _ => unreachable!("color type validated in parse_ihdr"),
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_mismatch_errors() {
        let bad = [0u8; 16];
        assert_eq!(decode(&bad).unwrap_err(), Error::NotPng);
    }

    #[test]
    fn empty_input_errors() {
        assert_eq!(decode(&[]).unwrap_err(), Error::NotPng);
    }

    #[test]
    fn paeth_predictor_known_cases() {
        // PNG 1.2 §9.4 examples: Paeth of (0, 0, 0) → 0.
        assert_eq!(paeth(0, 0, 0), 0);
        // Paeth(10, 20, 15) — pa=|20-15|=5, pb=|10-15|=5,
        // pc=|10+20-30|=0. pc smallest? pa<=pb && pa<=pc?
        // pa=5, pc=0 → pa>pc → false. pb<=pc? 5<=0 → false.
        // → return c=15.
        assert_eq!(paeth(10, 20, 15), 15);
        // Paeth(100, 50, 80): pa=|50-80|=30, pb=|100-80|=20,
        // pc=|100+50-160|=10. pa<=pb && pa<=pc? 30<=20 false.
        // pb<=pc? 20<=10 false. → return c=80.
        assert_eq!(paeth(100, 50, 80), 80);
    }

    #[test]
    fn crc32_known_input() {
        // CRC32 of the IHDR chunk type alone (no data) is
        // 0x575E51F5 per the PNG spec test vector.
        // We compute (kind, data) — but the official test case
        // uses CRC32 of "IEND" + empty data = 0xAE426082.
        let crc = crc32_pair(b"IEND", b"");
        assert_eq!(crc, 0xAE42_6082);
    }
}
