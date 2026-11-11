// TIFF 6.0 baseline encoder. Pure function from a non-empty
// list of RGBA8 pages + Options to a single TIFF byte stream
// readable by `tiff_decoder`. Composes the frozen `flate`,
// `lzw`, and `runlength` primitives for the four lossless
// compressions in scope (None, LZW=5, Deflate=8, PackBits=
// 32773). std-only.
//
// Mirrors `tiff_decoder`'s read scope exactly: strips only,
// 1bpp/4bpp/8bpp samples, BlackIsZero / RGB / Palette
// photometrics, SamplesPerPixel ∈ {1, 3, 4}. The encode →
// decode round trip is byte-equality on the per-photometric
// inputs documented in the shared spec yaml.
//
// Output is always little-endian ('II'). Multi-page output
// chains IFDs via the next-IFD-offset slot.

#![allow(clippy::needless_range_loop)] // index-arithmetic numeric kernels

use std::fmt;

use super::ccitt;
use super::flate;
use super::lzw;
use super::runlength;

#[derive(Debug, Clone)]
pub struct Page {
    pub width: u32,
    pub height: u32,
    pub rgba8: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    Lzw,
    #[allow(dead_code)] // retained for a future-phase public-API binding
    Deflate,
    PackBits,
    /// ITU-T T.4 Group 3 1D facsimile (TIFF Compression=3,
    /// T4Options=0). Valid only when paired with `BlackIsZero`
    /// + `bits_per_sample = 1`.
    Ccitt3,
    /// ITU-T T.6 Group 4 facsimile (TIFF Compression=4,
    /// T6Options=0). Valid only when paired with `BlackIsZero`
    /// + `bits_per_sample = 1`.
    Ccitt4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Photometric {
    WhiteIsZero,
    BlackIsZero,
    Rgb,
    Palette,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub compression: Compression,
    pub photometric: Photometric,
    pub samples_per_pixel: u8,
    pub bits_per_sample: u8,
    pub dpi: u32,
    pub color_map: Option<Vec<u16>>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            compression: Compression::None,
            photometric: Photometric::Rgb,
            samples_per_pixel: 4,
            bits_per_sample: 8,
            dpi: 72,
            color_map: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    EmptyPages,
    PixelLengthMismatch { page_index: usize, expected: usize, got: usize },
    NonGrayscaleInput { page_index: usize, pixel_index: usize },
    DpiNotPositive,
    InvalidBitsPerSample(u8),
    InvalidSamplesPerPixel(u8),
    WhiteIsZeroOutOfScope,
    InvalidPhotometricSppBpsTuple,
    PaletteRequiresColorMap,
    ColorMapWrongLength { expected: usize, got: usize },
    /// CCITT3 / CCITT4 selected with a photometric + bps tuple
    /// other than `(BlackIsZero, 1, 1)`.
    CcittRequiresBlackIsZero1Bpp,
    /// Underlying CCITT encoder rejected the input strip.
    CcittEncode(ccitt::EncodeError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::EmptyPages =>
                write!(f, "tiff_encoder: pages list is empty"),
            Error::PixelLengthMismatch { page_index, expected, got } =>
                write!(f,
                    "tiff_encoder: page {page_index} rgba8 length {got} != width*height*4 = {expected}"),
            Error::NonGrayscaleInput { page_index, pixel_index } =>
                write!(f,
                    "tiff_encoder: page {page_index} pixel {pixel_index} is not grayscale (R != G or G != B)"),
            Error::DpiNotPositive =>
                write!(f, "tiff_encoder: dpi must be positive"),
            Error::InvalidBitsPerSample(b) =>
                write!(f, "tiff_encoder: bits_per_sample {b} not in {{1, 4, 8}}"),
            Error::InvalidSamplesPerPixel(s) =>
                write!(f, "tiff_encoder: samples_per_pixel {s} not in {{1, 3, 4}}"),
            Error::WhiteIsZeroOutOfScope =>
                write!(f, "tiff_encoder: WhiteIsZero photometric is out of v1 scope"),
            Error::InvalidPhotometricSppBpsTuple =>
                write!(f, "tiff_encoder: invalid (photometric, samples_per_pixel, bits_per_sample) tuple"),
            Error::PaletteRequiresColorMap =>
                write!(f, "tiff_encoder: Palette photometric requires a color_map"),
            Error::ColorMapWrongLength { expected, got } =>
                write!(f,
                    "tiff_encoder: color_map must be 3 * 2^bits_per_sample = {expected} entries (got {got})"),
            Error::CcittRequiresBlackIsZero1Bpp =>
                write!(f, "tiff_encoder: Ccitt3/Ccitt4 require (BlackIsZero, 1, 1)"),
            Error::CcittEncode(e) =>
                write!(f, "tiff_encoder: ccitt encode failed: {e}"),
        }
    }
}

impl std::error::Error for Error {}

const TYPE_SHORT: u16 = 3;
const TYPE_LONG: u16 = 4;
const TYPE_RATIONAL: u16 = 5;

/// Encode `pages` into a single multi-IFD TIFF byte stream.
pub fn encode(pages: &[Page], options: &Options) -> Result<Vec<u8>, Error> {
    if pages.is_empty() {
        return Err(Error::EmptyPages);
    }
    validate_options(options)?;

    let mut out: Vec<u8> = Vec::new();

    // Header: 'II' + magic 42 + first-IFD-offset placeholder.
    out.extend_from_slice(b"II");
    write_u16_le(&mut out, 42);
    let header_first_ifd_slot = out.len();
    write_u32_le(&mut out, 0);

    let mut previous_next_ifd_slot = header_first_ifd_slot;

    for (p, page) in pages.iter().enumerate() {
        validate_page(page, p)?;
        let (this_ifd_offset, this_next_ifd_slot) = emit_page(&mut out, page, options, p)?;
        // Back-patch the previous next-IFD-offset slot.
        let bytes = u32::try_from(this_ifd_offset)
            .expect("IFD offset fits in u32")
            .to_le_bytes();
        out[previous_next_ifd_slot..previous_next_ifd_slot + 4]
            .copy_from_slice(&bytes);
        previous_next_ifd_slot = this_next_ifd_slot;
    }

    Ok(out)
}

// ============================================================
// Per-page emission
// ============================================================

fn emit_page(
    out: &mut Vec<u8>,
    page: &Page,
    options: &Options,
    page_index: usize,
) -> Result<(usize, usize), Error> {
    // a. Sample preparation.
    let samples = prepare_samples(page, options, page_index)?;

    // c. Strip compression.
    let strip = compress(&samples, options.compression, page.width, page.height)?;

    // d. Strip data emit.
    let strip_offset = out.len();
    out.extend_from_slice(&strip);

    // e. Out-of-line tag values.
    let bits_per_sample_offset = if options.samples_per_pixel > 1 {
        align_word(out);
        let off = out.len();
        for _ in 0..options.samples_per_pixel {
            write_u16_le(out, options.bits_per_sample as u16);
        }
        off
    } else {
        0
    };

    align_word(out);
    let x_resolution_offset = out.len();
    write_u32_le(out, options.dpi);
    write_u32_le(out, 1);
    let y_resolution_offset = out.len();
    write_u32_le(out, options.dpi);
    write_u32_le(out, 1);

    let color_map_offset = if options.photometric == Photometric::Palette {
        align_word(out);
        let off = out.len();
        let cmap = options.color_map.as_ref().expect("validated");
        for &entry in cmap {
            write_u16_le(out, entry);
        }
        off
    } else {
        0
    };

    // f. IFD emit.
    align_word(out);
    let ifd_offset = out.len();

    let mut entries: Vec<IfdEntry> = Vec::with_capacity(14);
    entries.push(IfdEntry::new(256, TYPE_LONG, 1, page.width));
    entries.push(IfdEntry::new(257, TYPE_LONG, 1, page.height));
    entries.push(IfdEntry::new(
        258,
        TYPE_SHORT,
        u32::from(options.samples_per_pixel),
        if options.samples_per_pixel == 1 {
            inline_short(options.bits_per_sample as u16)
        } else {
            u32::try_from(bits_per_sample_offset).expect("offset fits")
        },
    ));
    entries.push(IfdEntry::new(
        259, TYPE_SHORT, 1,
        inline_short(compression_tag_value(options.compression)),
    ));
    entries.push(IfdEntry::new(
        262, TYPE_SHORT, 1,
        inline_short(photometric_tag_value(options.photometric)),
    ));
    entries.push(IfdEntry::new(
        273, TYPE_LONG, 1,
        u32::try_from(strip_offset).expect("offset fits"),
    ));
    entries.push(IfdEntry::new(
        277, TYPE_SHORT, 1,
        inline_short(options.samples_per_pixel as u16),
    ));
    entries.push(IfdEntry::new(278, TYPE_LONG, 1, page.height));
    entries.push(IfdEntry::new(
        279, TYPE_LONG, 1,
        u32::try_from(strip.len()).expect("strip length fits"),
    ));
    entries.push(IfdEntry::new(
        282, TYPE_RATIONAL, 1,
        u32::try_from(x_resolution_offset).expect("offset fits"),
    ));
    entries.push(IfdEntry::new(
        283, TYPE_RATIONAL, 1,
        u32::try_from(y_resolution_offset).expect("offset fits"),
    ));
    entries.push(IfdEntry::new(296, TYPE_SHORT, 1, inline_short(2)));
    if options.photometric == Photometric::Palette {
        entries.push(IfdEntry::new(
            320, TYPE_SHORT,
            3 * (1u32 << options.bits_per_sample),
            u32::try_from(color_map_offset).expect("offset fits"),
        ));
    }
    if options.photometric == Photometric::Rgb && options.samples_per_pixel == 4 {
        entries.push(IfdEntry::new(338, TYPE_SHORT, 1, inline_short(2)));
    }
    // CCITT-specific IFD tags. T4Options (292) / T6Options (293)
    // both default to 0 (1D coding, no byte-aligned EOLs, no
    // uncompressed mode) — emitted explicitly so canonical
    // decoders that key on tag presence pick up the right
    // mode regardless of their default-value handling.
    if options.compression == Compression::Ccitt3 {
        entries.push(IfdEntry::new(292, TYPE_LONG, 1, 0));
    }
    if options.compression == Compression::Ccitt4 {
        entries.push(IfdEntry::new(293, TYPE_LONG, 1, 0));
    }
    entries.sort_by_key(|e| e.tag);

    write_u16_le(out, entries.len() as u16);
    for e in &entries {
        write_u16_le(out, e.tag);
        write_u16_le(out, e.ty);
        write_u32_le(out, e.count);
        write_u32_le(out, e.value_or_offset);
    }
    let next_ifd_slot = out.len();
    write_u32_le(out, 0);

    Ok((ifd_offset, next_ifd_slot))
}

// ============================================================
// Sample preparation
// ============================================================

fn prepare_samples(
    page: &Page,
    options: &Options,
    page_index: usize,
) -> Result<Vec<u8>, Error> {
    let pixel_count = (page.width as usize) * (page.height as usize);
    let key = (
        options.photometric,
        options.samples_per_pixel,
        options.bits_per_sample,
    );
    match key {
        (Photometric::Rgb, 4, 8) => Ok(page.rgba8.clone()),
        (Photometric::Rgb, 3, 8) => {
            let mut samples = vec![0u8; pixel_count * 3];
            for i in 0..pixel_count {
                let base = i * 4;
                let dst = i * 3;
                samples[dst] = page.rgba8[base];
                samples[dst + 1] = page.rgba8[base + 1];
                samples[dst + 2] = page.rgba8[base + 2];
            }
            Ok(samples)
        }
        (Photometric::BlackIsZero, 1, 8) => {
            let mut samples = vec![0u8; pixel_count];
            for i in 0..pixel_count {
                let base = i * 4;
                let r = page.rgba8[base];
                let g = page.rgba8[base + 1];
                let b = page.rgba8[base + 2];
                if r != g || g != b {
                    return Err(Error::NonGrayscaleInput {
                        page_index,
                        pixel_index: i,
                    });
                }
                samples[i] = r;
            }
            Ok(samples)
        }
        (Photometric::BlackIsZero, 1, 1) => {
            let row_bytes = (page.width as usize).div_ceil(8);
            let mut samples = vec![0u8; row_bytes * (page.height as usize)];
            for y in 0..(page.height as usize) {
                let dst_row = y * row_bytes;
                let src_row = y * (page.width as usize) * 4;
                for x in 0..(page.width as usize) {
                    let base = src_row + x * 4;
                    let r = page.rgba8[base] as u32;
                    let g = page.rgba8[base + 1] as u32;
                    let b = page.rgba8[base + 2] as u32;
                    let luma = (r * 299 + g * 587 + b * 114) / 1000;
                    if luma >= 128 {
                        samples[dst_row + x / 8] |= 0x80 >> (x % 8);
                    }
                }
            }
            Ok(samples)
        }
        (Photometric::Palette, 1, 8) => {
            let mut samples = vec![0u8; pixel_count];
            for i in 0..pixel_count {
                samples[i] = page.rgba8[i * 4];
            }
            Ok(samples)
        }
        (Photometric::Palette, 1, 4) => {
            let row_bytes = (page.width as usize).div_ceil(2);
            let mut samples = vec![0u8; row_bytes * (page.height as usize)];
            for y in 0..(page.height as usize) {
                let dst_row = y * row_bytes;
                let src_row = y * (page.width as usize) * 4;
                for x in 0..(page.width as usize) {
                    let v = page.rgba8[src_row + x * 4] & 0x0F;
                    if x & 1 == 0 {
                        samples[dst_row + x / 2] = v << 4;
                    } else {
                        samples[dst_row + x / 2] |= v;
                    }
                }
            }
            Ok(samples)
        }
        _ => Err(Error::InvalidPhotometricSppBpsTuple),
    }
}

fn compress(
    strip: &[u8],
    scheme: Compression,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, Error> {
    match scheme {
        Compression::None => Ok(strip.to_vec()),
        // TIFF 6.0 §13 mandates EarlyChange=0 (decoder bumps
        // width when next_code first exceeds 2^N - 1, i.e.,
        // after slot 2^N - 1 is installed). The default
        // `lzw::encode` ships PDF's EarlyChange=1 dialect and
        // produces a stream libtiff rejects at the first width
        // transition.
        Compression::Lzw => Ok(lzw::encode_with(
            strip,
            lzw::EarlyChange::Disabled,
        )),
        Compression::Deflate => Ok(flate::encode(strip)),
        Compression::PackBits => Ok(runlength::encode(strip)),
        Compression::Ccitt3 => ccitt::encode_group3(strip, width, height)
            .map_err(Error::CcittEncode),
        Compression::Ccitt4 => ccitt::encode_group4(strip, width, height)
            .map_err(Error::CcittEncode),
    }
}

// ============================================================
// Validation
// ============================================================

fn validate_options(options: &Options) -> Result<(), Error> {
    if options.dpi == 0 {
        return Err(Error::DpiNotPositive);
    }
    if !matches!(options.bits_per_sample, 1 | 4 | 8) {
        return Err(Error::InvalidBitsPerSample(options.bits_per_sample));
    }
    if !matches!(options.samples_per_pixel, 1 | 3 | 4) {
        return Err(Error::InvalidSamplesPerPixel(options.samples_per_pixel));
    }
    let key = (
        options.photometric,
        options.samples_per_pixel,
        options.bits_per_sample,
    );
    let valid = matches!(
        key,
        (Photometric::BlackIsZero, 1, 1)
            | (Photometric::BlackIsZero, 1, 8)
            | (Photometric::Palette, 1, 4)
            | (Photometric::Palette, 1, 8)
            | (Photometric::Rgb, 3, 8)
            | (Photometric::Rgb, 4, 8)
    );
    if !valid {
        if options.photometric == Photometric::WhiteIsZero {
            return Err(Error::WhiteIsZeroOutOfScope);
        }
        return Err(Error::InvalidPhotometricSppBpsTuple);
    }
    if options.photometric == Photometric::Palette {
        let cmap = options.color_map.as_ref()
            .ok_or(Error::PaletteRequiresColorMap)?;
        let expected = 3 * (1usize << options.bits_per_sample);
        if cmap.len() != expected {
            return Err(Error::ColorMapWrongLength { expected, got: cmap.len() });
        }
    }
    if matches!(options.compression, Compression::Ccitt3 | Compression::Ccitt4)
        && key != (Photometric::BlackIsZero, 1, 1)
    {
        return Err(Error::CcittRequiresBlackIsZero1Bpp);
    }
    Ok(())
}

fn validate_page(page: &Page, page_index: usize) -> Result<(), Error> {
    let expected = (page.width as usize)
        .checked_mul(page.height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or(Error::PixelLengthMismatch {
            page_index,
            expected: usize::MAX,
            got: page.rgba8.len(),
        })?;
    if page.rgba8.len() != expected {
        return Err(Error::PixelLengthMismatch {
            page_index,
            expected,
            got: page.rgba8.len(),
        });
    }
    Ok(())
}

// ============================================================
// IFD entry helpers
// ============================================================

#[derive(Debug, Clone, Copy)]
struct IfdEntry {
    tag: u16,
    ty: u16,
    count: u32,
    value_or_offset: u32,
}

impl IfdEntry {
    fn new(tag: u16, ty: u16, count: u32, value_or_offset: u32) -> Self {
        Self { tag, ty, count, value_or_offset }
    }
}

fn inline_short(value: u16) -> u32 {
    // SHORT count=1: value occupies the low 2 bytes of the
    // 4-byte value/offset slot (left-justified per TIFF 6.0 §2;
    // little-endian places the low byte first).
    value as u32
}

fn compression_tag_value(c: Compression) -> u16 {
    match c {
        Compression::None => 1,
        Compression::Ccitt3 => 3,
        Compression::Ccitt4 => 4,
        Compression::Lzw => 5,
        Compression::Deflate => 8,
        Compression::PackBits => 32773,
    }
}

fn photometric_tag_value(p: Photometric) -> u16 {
    match p {
        Photometric::WhiteIsZero => 0,
        Photometric::BlackIsZero => 1,
        Photometric::Rgb => 2,
        Photometric::Palette => 3,
    }
}

fn write_u16_le(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn write_u32_le(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn align_word(out: &mut Vec<u8>) {
    if out.len() & 1 != 0 {
        out.push(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_rgba(w: u32, h: u32, color: [u8; 4]) -> Vec<u8> {
        let mut v = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..(w * h) {
            v.extend_from_slice(&color);
        }
        v
    }

    #[test]
    fn empty_pages_errors() {
        let err = encode(&[], &Options::default()).unwrap_err();
        assert_eq!(err, Error::EmptyPages);
    }

    #[test]
    fn pixel_length_mismatch_errors() {
        let page = Page { width: 2, height: 2, rgba8: vec![0; 4] };
        let err = encode(&[page], &Options::default()).unwrap_err();
        assert!(matches!(err, Error::PixelLengthMismatch { .. }));
    }

    #[test]
    fn rgba_passthrough_writes_header() {
        let pages = [Page {
            width: 2,
            height: 2,
            rgba8: solid_rgba(2, 2, [10, 20, 30, 40]),
        }];
        let out = encode(&pages, &Options::default()).unwrap();
        // 'II' + magic 42 LE + first-IFD-offset.
        assert_eq!(&out[0..2], b"II");
        assert_eq!(u16::from_le_bytes([out[2], out[3]]), 42);
        let first_ifd =
            u32::from_le_bytes([out[4], out[5], out[6], out[7]]) as usize;
        // IFD offset should be past the strip data (16 bytes) +
        // out-of-line blocks.
        assert!(first_ifd >= 8 + 16);
    }
}
