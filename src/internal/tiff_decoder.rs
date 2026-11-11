// TIFF 6.0 baseline decoder. Pure function from a complete TIFF
// byte stream to a uniform RGBA8 pixel buffer for the first IFD's
// image. Composes the frozen `flate`, `lzw`, and `runlength`
// primitives for the four lossless compressions in scope (none,
// LZW=5, Deflate=8, PackBits=32773).
//
// Scope: strips only (no tiles); 1bpp/4bpp/8bpp samples;
// BlackIsZero / RGB / Palette photometrics; SPP=1/3/4; alpha
// only via SamplesPerPixel=4 + ExtraSamples=2. Out of scope:
// CCITT / JPEG compressions, WhiteIsZero, CMYK / YCbCr,
// PlanarConfiguration=2, predictor != 1.

use std::fmt;

use super::flate;
use super::lzw;
use super::runlength;

const TAG_IMAGE_WIDTH: u16 = 256;
const TAG_IMAGE_LENGTH: u16 = 257;
const TAG_BITS_PER_SAMPLE: u16 = 258;
const TAG_COMPRESSION: u16 = 259;
const TAG_PHOTOMETRIC: u16 = 262;
const TAG_STRIP_OFFSETS: u16 = 273;
const TAG_SAMPLES_PER_PIXEL: u16 = 277;
const TAG_ROWS_PER_STRIP: u16 = 278;
const TAG_STRIP_BYTE_COUNTS: u16 = 279;
const TAG_COLOR_MAP: u16 = 320;
const TAG_EXTRA_SAMPLES: u16 = 338;

const TYPE_BYTE: u16 = 1;
const TYPE_SHORT: u16 = 3;
const TYPE_LONG: u16 = 4;

const COMPRESSION_NONE: u16 = 1;
const COMPRESSION_LZW: u16 = 5;
const COMPRESSION_DEFLATE: u16 = 8;
const COMPRESSION_PACKBITS: u16 = 32773;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub components: u8, // always 4 (RGBA8)
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    NotTiff,
    BadHeader,
    MissingTag(u16),
    BadTagValue(&'static str),
    UnsupportedCompression(u16),
    UnsupportedPhotometric(u16),
    UnsupportedBps,
    UnsupportedSpp(u16),
    UnsupportedExtraSamples(u16),
    StripCountMismatch,
    DecompressFailed,
    OutOfBounds,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotTiff => write!(f, "tiff_decoder: not a TIFF (header mismatch)"),
            Error::BadHeader => write!(f, "tiff_decoder: malformed TIFF header"),
            Error::MissingTag(t) => write!(f, "tiff_decoder: missing required tag {t}"),
            Error::BadTagValue(name) => write!(f, "tiff_decoder: bad value for tag {name}"),
            Error::UnsupportedCompression(c) =>
                write!(f, "tiff_decoder: unsupported Compression {c}"),
            Error::UnsupportedPhotometric(p) =>
                write!(f, "tiff_decoder: unsupported Photometric {p}"),
            Error::UnsupportedBps =>
                write!(f, "tiff_decoder: unsupported BitsPerSample / SPP / Photometric combination"),
            Error::UnsupportedSpp(s) =>
                write!(f, "tiff_decoder: unsupported SamplesPerPixel {s}"),
            Error::UnsupportedExtraSamples(e) =>
                write!(f, "tiff_decoder: unsupported ExtraSamples {e}"),
            Error::StripCountMismatch =>
                write!(f, "tiff_decoder: StripOffsets/StripByteCounts length mismatch"),
            Error::DecompressFailed =>
                write!(f, "tiff_decoder: strip decompression failed"),
            Error::OutOfBounds =>
                write!(f, "tiff_decoder: read past end of input"),
        }
    }
}

impl std::error::Error for Error {}

// ============================================================
// Public API
// ============================================================

pub fn decode(input: &[u8]) -> Result<DecodedImage, Error> {
    if input.len() < 8 {
        return Err(Error::NotTiff);
    }
    let big_endian = match &input[..2] {
        b"II" => false,
        b"MM" => true,
        _ => return Err(Error::NotTiff),
    };
    let magic = read_u16(input, 2, big_endian)?;
    if magic != 42 {
        return Err(Error::BadHeader);
    }
    let ifd_offset = read_u32(input, 4, big_endian)? as usize;

    // IFD walk.
    let entries = read_ifd_entries(input, ifd_offset, big_endian)?;
    let ifd = parse_ifd(input, &entries, big_endian)?;
    validate_ifd(&ifd)?;

    // Strip decompress + concat.
    let raw = read_and_decompress_strips(input, &ifd, big_endian)?;

    // Photometric expansion.
    let pixels = expand_to_rgba8(&raw, &ifd)?;

    Ok(DecodedImage {
        width: ifd.width,
        height: ifd.height,
        components: 4,
        pixels,
    })
}

// ============================================================
// IFD parsing
// ============================================================

#[derive(Debug, Clone)]
struct IfdEntry {
    tag: u16,
    field_type: u16,
    count: u32,
    /// Raw bytes of the 4-byte value slot — the caller decides
    /// whether to read inline or follow as an offset.
    value_slot: [u8; 4],
}

fn read_ifd_entries(
    input: &[u8],
    offset: usize,
    big_endian: bool,
) -> Result<Vec<IfdEntry>, Error> {
    if offset + 2 > input.len() {
        return Err(Error::OutOfBounds);
    }
    let n = read_u16(input, offset, big_endian)? as usize;
    let entries_start = offset + 2;
    let entries_end = entries_start + n * 12;
    if entries_end > input.len() {
        return Err(Error::OutOfBounds);
    }
    let mut entries = Vec::with_capacity(n);
    for i in 0..n {
        let p = entries_start + i * 12;
        let tag = read_u16(input, p, big_endian)?;
        let field_type = read_u16(input, p + 2, big_endian)?;
        let count = read_u32(input, p + 4, big_endian)?;
        let value_slot = [
            input[p + 8],
            input[p + 9],
            input[p + 10],
            input[p + 11],
        ];
        entries.push(IfdEntry { tag, field_type, count, value_slot });
    }
    Ok(entries)
}

#[derive(Debug, Clone)]
struct Ifd {
    width: u32,
    height: u32,
    bits_per_sample: u32,
    compression: u16,
    photometric: u16,
    samples_per_pixel: u16,
    rows_per_strip: u32,
    strip_offsets: Vec<u32>,
    strip_byte_counts: Vec<u32>,
    color_map: Vec<u16>,
    extra_samples: Option<u16>,
}

fn parse_ifd(
    input: &[u8],
    entries: &[IfdEntry],
    be: bool,
) -> Result<Ifd, Error> {
    // Default samples_per_pixel = 1 (TIFF 6.0 spec default).
    let mut width: Option<u32> = None;
    let mut height: Option<u32> = None;
    let mut bps_array: Option<Vec<u32>> = None;
    let mut compression: Option<u16> = None;
    let mut photometric: Option<u16> = None;
    let mut samples_per_pixel: u16 = 1;
    let mut rows_per_strip: Option<u32> = None;
    let mut strip_offsets: Option<Vec<u32>> = None;
    let mut strip_byte_counts: Option<Vec<u32>> = None;
    let mut color_map: Vec<u16> = Vec::new();
    let mut extra_samples: Option<u16> = None;

    for e in entries {
        match e.tag {
            TAG_IMAGE_WIDTH => {
                width = Some(read_unsigned_scalar(input, e, be)?);
            }
            TAG_IMAGE_LENGTH => {
                height = Some(read_unsigned_scalar(input, e, be)?);
            }
            TAG_BITS_PER_SAMPLE => {
                bps_array = Some(read_unsigned_array(input, e, be)?);
            }
            TAG_COMPRESSION => {
                compression = Some(read_unsigned_scalar(input, e, be)? as u16);
            }
            TAG_PHOTOMETRIC => {
                photometric = Some(read_unsigned_scalar(input, e, be)? as u16);
            }
            TAG_STRIP_OFFSETS => {
                strip_offsets = Some(
                    read_unsigned_array(input, e, be)?
                        .into_iter()
                        .collect()
                );
            }
            TAG_SAMPLES_PER_PIXEL => {
                samples_per_pixel = read_unsigned_scalar(input, e, be)? as u16;
            }
            TAG_ROWS_PER_STRIP => {
                rows_per_strip = Some(read_unsigned_scalar(input, e, be)?);
            }
            TAG_STRIP_BYTE_COUNTS => {
                strip_byte_counts = Some(
                    read_unsigned_array(input, e, be)?
                        .into_iter()
                        .collect()
                );
            }
            TAG_COLOR_MAP => {
                let arr = read_unsigned_array(input, e, be)?;
                color_map = arr.into_iter().map(|v| v as u16).collect();
            }
            TAG_EXTRA_SAMPLES => {
                let arr = read_unsigned_array(input, e, be)?;
                extra_samples = arr.into_iter().next().map(|v| v as u16);
            }
            _ => {} // ignore other tags
        }
    }

    let bps_array = bps_array.ok_or(Error::MissingTag(TAG_BITS_PER_SAMPLE))?;
    if bps_array.len() != samples_per_pixel as usize {
        return Err(Error::BadTagValue("BitsPerSample"));
    }
    let bps = bps_array[0];
    if !bps_array.iter().all(|&v| v == bps) {
        return Err(Error::BadTagValue("BitsPerSample"));
    }

    Ok(Ifd {
        width: width.ok_or(Error::MissingTag(TAG_IMAGE_WIDTH))?,
        height: height.ok_or(Error::MissingTag(TAG_IMAGE_LENGTH))?,
        bits_per_sample: bps,
        compression: compression.ok_or(Error::MissingTag(TAG_COMPRESSION))?,
        photometric: photometric.ok_or(Error::MissingTag(TAG_PHOTOMETRIC))?,
        samples_per_pixel,
        rows_per_strip: rows_per_strip.ok_or(Error::MissingTag(TAG_ROWS_PER_STRIP))?,
        strip_offsets: strip_offsets.ok_or(Error::MissingTag(TAG_STRIP_OFFSETS))?,
        strip_byte_counts: strip_byte_counts.ok_or(Error::MissingTag(TAG_STRIP_BYTE_COUNTS))?,
        color_map,
        extra_samples,
    })
}

fn validate_ifd(ifd: &Ifd) -> Result<(), Error> {
    if ifd.width == 0 || ifd.height == 0 || ifd.rows_per_strip == 0 {
        return Err(Error::BadTagValue("Image dimensions"));
    }
    if ifd.strip_offsets.len() != ifd.strip_byte_counts.len() {
        return Err(Error::StripCountMismatch);
    }
    match ifd.compression {
        COMPRESSION_NONE | COMPRESSION_LZW | COMPRESSION_DEFLATE | COMPRESSION_PACKBITS => {}
        _ => return Err(Error::UnsupportedCompression(ifd.compression)),
    }
    match ifd.photometric {
        1..=3 => {}
        _ => return Err(Error::UnsupportedPhotometric(ifd.photometric)),
    }
    match ifd.samples_per_pixel {
        1 | 3 | 4 => {}
        s => return Err(Error::UnsupportedSpp(s)),
    }
    // BPS / SPP / Photometric value-set check.
    let ok = match (ifd.samples_per_pixel, ifd.photometric, ifd.bits_per_sample) {
        (1, 1, 1) | (1, 1, 8) => true,           // BlackIsZero
        (1, 3, 4) | (1, 3, 8) => true,           // Palette
        (3, 2, 8) => true,                        // RGB
        (4, 2, 8) => true,                        // RGBA
        _ => false,
    };
    if !ok {
        return Err(Error::UnsupportedBps);
    }
    if ifd.samples_per_pixel == 4 && ifd.photometric == 2 {
        match ifd.extra_samples {
            Some(2) => {}
            Some(other) => return Err(Error::UnsupportedExtraSamples(other)),
            None => return Err(Error::UnsupportedExtraSamples(0)),
        }
    }
    if ifd.photometric == 3 {
        let expected = 3 * (1usize << ifd.bits_per_sample);
        if ifd.color_map.len() != expected {
            return Err(Error::BadTagValue("ColorMap"));
        }
    }
    Ok(())
}

// ============================================================
// IFD value readers (inline 4-byte slot vs out-of-line offset)
// ============================================================

fn type_size(field_type: u16) -> usize {
    match field_type {
        TYPE_BYTE => 1,
        TYPE_SHORT => 2,
        TYPE_LONG => 4,
        _ => 0,
    }
}

fn read_unsigned_scalar(
    input: &[u8],
    e: &IfdEntry,
    be: bool,
) -> Result<u32, Error> {
    let arr = read_unsigned_array(input, e, be)?;
    if arr.is_empty() {
        return Err(Error::BadTagValue("scalar tag has zero count"));
    }
    Ok(arr[0])
}

fn read_unsigned_array(
    input: &[u8],
    e: &IfdEntry,
    be: bool,
) -> Result<Vec<u32>, Error> {
    let elem = type_size(e.field_type);
    if elem == 0 {
        return Err(Error::BadTagValue("unsupported field type"));
    }
    let total = elem * e.count as usize;
    let bytes: &[u8] = if total <= 4 {
        &e.value_slot[..total]
    } else {
        // Slot holds an offset to the value array.
        let offset = read_u32_inplace(&e.value_slot, be) as usize;
        if offset + total > input.len() {
            return Err(Error::OutOfBounds);
        }
        &input[offset..offset + total]
    };
    let mut out = Vec::with_capacity(e.count as usize);
    for i in 0..e.count as usize {
        let v = match e.field_type {
            TYPE_BYTE => bytes[i] as u32,
            TYPE_SHORT => read_u16_inplace(&bytes[i * 2..i * 2 + 2], be) as u32,
            TYPE_LONG => read_u32_inplace(&bytes[i * 4..i * 4 + 4], be),
            _ => unreachable!(),
        };
        out.push(v);
    }
    Ok(out)
}

// ============================================================
// Strip decompression
// ============================================================

fn read_and_decompress_strips(
    input: &[u8],
    ifd: &Ifd,
    _be: bool,
) -> Result<Vec<u8>, Error> {
    let row_bytes = on_wire_row_bytes(ifd);
    let total_rows = ifd.height as usize;
    let mut raw: Vec<u8> = Vec::with_capacity(total_rows * row_bytes);
    let mut rows_remaining = total_rows;

    for (i, &offset) in ifd.strip_offsets.iter().enumerate() {
        let offset = offset as usize;
        let count = ifd.strip_byte_counts[i] as usize;
        if offset + count > input.len() {
            return Err(Error::OutOfBounds);
        }
        let body = &input[offset..offset + count];
        let strip_rows = rows_remaining.min(ifd.rows_per_strip as usize);
        let expected_bytes = strip_rows * row_bytes;
        let decompressed = match ifd.compression {
            COMPRESSION_NONE => body.to_vec(),
            COMPRESSION_LZW => lzw::decode(body)
                .map_err(|_| Error::DecompressFailed)?,
            COMPRESSION_DEFLATE => flate::decode(body)
                .map_err(|_| Error::DecompressFailed)?,
            COMPRESSION_PACKBITS => runlength::decode(body)
                .map_err(|_| Error::DecompressFailed)?,
            other => return Err(Error::UnsupportedCompression(other)),
        };
        if decompressed.len() < expected_bytes {
            return Err(Error::DecompressFailed);
        }
        raw.extend_from_slice(&decompressed[..expected_bytes]);
        rows_remaining = rows_remaining.saturating_sub(strip_rows);
    }
    if rows_remaining != 0 {
        return Err(Error::DecompressFailed);
    }
    Ok(raw)
}

fn on_wire_row_bytes(ifd: &Ifd) -> usize {
    let w = ifd.width as usize;
    match ifd.bits_per_sample {
        1 => w.div_ceil(8),
        4 => w.div_ceil(2),
        8 => w * ifd.samples_per_pixel as usize,
        _ => 0,
    }
}

// ============================================================
// Photometric expansion → RGBA8
// ============================================================

fn expand_to_rgba8(raw: &[u8], ifd: &Ifd) -> Result<Vec<u8>, Error> {
    let w = ifd.width as usize;
    let h = ifd.height as usize;
    let row_bytes = on_wire_row_bytes(ifd);
    let mut out: Vec<u8> = Vec::with_capacity(w * h * 4);

    match (ifd.bits_per_sample, ifd.samples_per_pixel, ifd.photometric) {
        (1, 1, 1) => {
            // 1bpp BlackIsZero. MSB-first; bit=0 → 0x00, bit=1 → 0xFF.
            for r in 0..h {
                let row = &raw[r * row_bytes..(r + 1) * row_bytes];
                for x in 0..w {
                    let bit = (row[x / 8] >> (7 - (x % 8))) & 1;
                    let v = if bit == 0 { 0u8 } else { 0xFFu8 };
                    out.push(v); out.push(v); out.push(v); out.push(0xFF);
                }
            }
        }
        (4, 1, 3) => {
            // 4bpp Palette. HIGH nibble = first pixel of byte pair.
            for r in 0..h {
                let row = &raw[r * row_bytes..(r + 1) * row_bytes];
                for x in 0..w {
                    let byte = row[x / 2];
                    let idx = if x % 2 == 0 { byte >> 4 } else { byte & 0x0F };
                    push_palette_pixel(&mut out, idx as usize, ifd);
                }
            }
        }
        (8, 1, 1) => {
            // 8bpp grayscale.
            for r in 0..h {
                let row = &raw[r * row_bytes..(r + 1) * row_bytes];
                for &s in row.iter().take(w) {
                    out.push(s); out.push(s); out.push(s); out.push(0xFF);
                }
            }
        }
        (8, 1, 3) => {
            // 8bpp Palette.
            for r in 0..h {
                let row = &raw[r * row_bytes..(r + 1) * row_bytes];
                for &idx in row.iter().take(w) {
                    push_palette_pixel(&mut out, idx as usize, ifd);
                }
            }
        }
        (8, 3, 2) => {
            // 8bpp RGB.
            for r in 0..h {
                let row = &raw[r * row_bytes..(r + 1) * row_bytes];
                for x in 0..w {
                    out.push(row[x * 3]);
                    out.push(row[x * 3 + 1]);
                    out.push(row[x * 3 + 2]);
                    out.push(0xFF);
                }
            }
        }
        (8, 4, 2) => {
            // 8bpp RGBA (ExtraSamples=2 unassoc).
            for r in 0..h {
                let row = &raw[r * row_bytes..(r + 1) * row_bytes];
                for x in 0..w {
                    out.push(row[x * 4]);
                    out.push(row[x * 4 + 1]);
                    out.push(row[x * 4 + 2]);
                    out.push(row[x * 4 + 3]);
                }
            }
        }
        _ => return Err(Error::UnsupportedBps),
    }
    Ok(out)
}

fn push_palette_pixel(out: &mut Vec<u8>, idx: usize, ifd: &Ifd) {
    // ColorMap: 3 * 2^bps SHORTs — all R, then all G, then all B,
    // each 0..65535. Output byte = (entry >> 8).
    let n = 1usize << ifd.bits_per_sample;
    let r = (ifd.color_map[idx] >> 8) as u8;
    let g = (ifd.color_map[n + idx] >> 8) as u8;
    let b = (ifd.color_map[2 * n + idx] >> 8) as u8;
    out.push(r); out.push(g); out.push(b); out.push(0xFF);
}

// ============================================================
// Endian-aware byte reads
// ============================================================

fn read_u16(input: &[u8], offset: usize, be: bool) -> Result<u16, Error> {
    if offset + 2 > input.len() { return Err(Error::OutOfBounds); }
    Ok(read_u16_inplace(&input[offset..offset + 2], be))
}

fn read_u32(input: &[u8], offset: usize, be: bool) -> Result<u32, Error> {
    if offset + 4 > input.len() { return Err(Error::OutOfBounds); }
    Ok(read_u32_inplace(&input[offset..offset + 4], be))
}

fn read_u16_inplace(b: &[u8], be: bool) -> u16 {
    if be { u16::from_be_bytes([b[0], b[1]]) }
    else { u16::from_le_bytes([b[0], b[1]]) }
}

fn read_u32_inplace(b: &[u8], be: bool) -> u32 {
    if be { u32::from_be_bytes([b[0], b[1], b[2], b[3]]) }
    else { u32::from_le_bytes([b[0], b[1], b[2], b[3]]) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_errors() {
        assert_eq!(decode(&[]).unwrap_err(), Error::NotTiff);
    }

    #[test]
    fn bad_byte_order_errors() {
        let bad = [b'X', b'X', 0, 0, 0, 0, 0, 0];
        assert_eq!(decode(&bad).unwrap_err(), Error::NotTiff);
    }

    #[test]
    fn header_too_short_errors() {
        let short = [b'I', b'I', 42];
        assert_eq!(decode(&short).unwrap_err(), Error::NotTiff);
    }
}
