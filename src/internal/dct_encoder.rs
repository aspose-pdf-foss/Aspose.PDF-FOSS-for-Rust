// foundation/dct_encoder — JPEG / ITU-T T.81 baseline sequential
// DCT encoder.
//
// One-shot lossy encode of a single RGBA8 pixel buffer into a
// JPEG baseline byte stream (T.81 Annex F sequential DCT, 8-bit
// samples, 4:4:4 chroma sampling, no APP markers). Mirrors the
// paired foundation::dctdecode primitive's read scope minus
// progressive — every JPEG this primitive emits is decodable by
// dctdecode within the freeze gate's PSNR floor (≥ 30 dB).
//
// Encode-only. The PDF /DCTDecode filter is read-side; encode is
// owned by callers (JpegDevice).
//
// References:
//   ITU-T T.81 (1992) §A / Annex F / Annex K / §B.2.4 / §F.1.4.
//   JFIF v1.02 Appendix A (BT.601 RGB↔YCbCr; APP0/JFIF marker
//                          not emitted in v1).
//
// See Feedstock/PDF/Spec/dct_encoder.yaml + targets/rust/dct_encoder.yaml.

#![allow(clippy::needless_range_loop)] // index-arithmetic numeric kernels

use std::fmt;

// ============================================================
// Public types
// ============================================================

/// Photometric interpretation for the on-wire pixel data and for
/// the JPEG SOF0 component count.
///
/// `Grayscale`: input must satisfy R=G=B at every pixel; the
///              encoder drops alpha and emits a 1-component
///              baseline carrying only the luma DC + AC tables.
/// `Rgb`     :  RGBA→RGB drop-alpha, then RGB→YCbCr per JFIF
///              Appendix A (BT.601); 3-component baseline at
///              4:4:4 sampling (Y, Cb, Cr each 1×1 per MCU).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Photometric {
    #[allow(dead_code)] // retained for a future-phase public-API binding
    Grayscale,
    Rgb,
}

/// Encoder-side knobs. Defaults are the production middle-of-
/// the-road JPEG: RGB at IJG quality 75, 72 DPI — matches
/// libjpeg-turbo's `cjpeg` defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// IJG quality scale, 0..100. Mapped to the Annex K reference
    /// quantisation tables via the IJG scaling formula. The
    /// encoder clamps to [1, 100] internally (q==0 would divide
    /// by zero in the q<50 branch).
    pub quality: u8,
    pub photometric: Photometric,
    /// Pixels-per-inch. Reserved on the option bag for parity
    /// with `tiff_encoder::dpi`; v1 doesn't emit JFIF APP0 so
    /// the value is currently a no-op (but must be positive).
    pub dpi: u32,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            quality: 75,
            photometric: Photometric::Rgb,
            dpi: 72,
        }
    }
}

/// Errors returned by [`encode`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    ZeroWidth,
    ZeroHeight,
    /// JPEG SOF marker fields encode dimensions as unsigned 16-bit.
    WidthOutOfRange { width: u32 },
    HeightOutOfRange { height: u32 },
    WrongRgbaLength { expected: usize, got: usize },
    /// `Photometric::Grayscale` with a pixel where R != G or G != B.
    NonGrayscaleInput { pixel_index: usize },
    DpiNotPositive,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ZeroWidth =>
                write!(f, "dct_encoder: width is zero"),
            Error::ZeroHeight =>
                write!(f, "dct_encoder: height is zero"),
            Error::WidthOutOfRange { width } =>
                write!(f, "dct_encoder: width {width} > 65535 (JPEG SOF marker is 16-bit)"),
            Error::HeightOutOfRange { height } =>
                write!(f, "dct_encoder: height {height} > 65535 (JPEG SOF marker is 16-bit)"),
            Error::WrongRgbaLength { expected, got } =>
                write!(f, "dct_encoder: rgba8 length {got} != width*height*4 = {expected}"),
            Error::NonGrayscaleInput { pixel_index } =>
                write!(f, "dct_encoder: pixel {pixel_index} not grayscale (R != G or G != B)"),
            Error::DpiNotPositive =>
                write!(f, "dct_encoder: dpi must be positive"),
        }
    }
}

impl std::error::Error for Error {}

// ============================================================
// Constants
// ============================================================

// Marker bytes (T.81 Table B.1; the SOI/EOI/etc. low byte —
// every marker is prefixed with 0xFF on emit).
const M_SOI: u8 = 0xD8;
const M_EOI: u8 = 0xD9;
const M_SOF0: u8 = 0xC0;
const M_DHT: u8 = 0xC4;
const M_DQT: u8 = 0xDB;
const M_SOS: u8 = 0xDA;

// JPEG zig-zag scan order (T.81 Figure A.6): natural row-major
// index → zig-zag scan index. Used to permute the 64-coefficient
// block at quantisation time so AC coefficients are emitted in
// the order T.81 §F.1.4 expects, AND at DQT-emit time so the
// quant table is on-wire in zig-zag order while we keep it in
// natural order in memory.
const ZIGZAG: [usize; 64] = [
     0,  1,  8, 16,  9,  2,  3, 10,
    17, 24, 32, 25, 18, 11,  4,  5,
    12, 19, 26, 33, 40, 48, 41, 34,
    27, 20, 13,  6,  7, 14, 21, 28,
    35, 42, 49, 56, 57, 50, 43, 36,
    29, 22, 15, 23, 30, 37, 44, 51,
    58, 59, 52, 45, 38, 31, 39, 46,
    53, 60, 61, 54, 47, 55, 62, 63,
];

// T.81 Annex K.1 — luminance quantisation table (natural order).
const Q_LUM: [u8; 64] = [
    16, 11, 10, 16, 24, 40, 51, 61,
    12, 12, 14, 19, 26, 58, 60, 55,
    14, 13, 16, 24, 40, 57, 69, 56,
    14, 17, 22, 29, 51, 87, 80, 62,
    18, 22, 37, 56, 68, 109, 103, 77,
    24, 35, 55, 64, 81, 104, 113, 92,
    49, 64, 78, 87, 103, 121, 120, 101,
    72, 92, 95, 98, 112, 100, 103, 99,
];

// T.81 Annex K.2 — chrominance quantisation table (natural order).
const Q_CHROM: [u8; 64] = [
    17, 18, 24, 47, 99, 99, 99, 99,
    18, 21, 26, 66, 99, 99, 99, 99,
    24, 26, 56, 99, 99, 99, 99, 99,
    47, 66, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
];

// T.81 Annex K.3 — DC luminance Huffman BITS / HUFFVAL.
const DC_LUM_BITS: [u8; 16] =
    [0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0];
const DC_LUM_VALS: [u8; 12] =
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];

// T.81 Annex K.5 — DC chrominance Huffman BITS / HUFFVAL.
const DC_CHROM_BITS: [u8; 16] =
    [0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0];
const DC_CHROM_VALS: [u8; 12] =
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];

// T.81 Annex K.4 — AC luminance Huffman BITS / HUFFVAL.
const AC_LUM_BITS: [u8; 16] =
    [0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 0x7D];
const AC_LUM_VALS: [u8; 162] = [
    0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12,
    0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07,
    0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08,
    0x23, 0x42, 0xB1, 0xC1, 0x15, 0x52, 0xD1, 0xF0,
    0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0A, 0x16,
    0x17, 0x18, 0x19, 0x1A, 0x25, 0x26, 0x27, 0x28,
    0x29, 0x2A, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39,
    0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49,
    0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59,
    0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
    0x6A, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79,
    0x7A, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
    0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98,
    0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7,
    0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6,
    0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4, 0xC5,
    0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4,
    0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE1, 0xE2,
    0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA,
    0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8,
    0xF9, 0xFA,
];

// T.81 Annex K.6 — AC chrominance Huffman BITS / HUFFVAL.
const AC_CHROM_BITS: [u8; 16] =
    [0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 0x77];
const AC_CHROM_VALS: [u8; 162] = [
    0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21,
    0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71,
    0x13, 0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91,
    0xA1, 0xB1, 0xC1, 0x09, 0x23, 0x33, 0x52, 0xF0,
    0x15, 0x62, 0x72, 0xD1, 0x0A, 0x16, 0x24, 0x34,
    0xE1, 0x25, 0xF1, 0x17, 0x18, 0x19, 0x1A, 0x26,
    0x27, 0x28, 0x29, 0x2A, 0x35, 0x36, 0x37, 0x38,
    0x39, 0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48,
    0x49, 0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58,
    0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
    0x69, 0x6A, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78,
    0x79, 0x7A, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87,
    0x88, 0x89, 0x8A, 0x92, 0x93, 0x94, 0x95, 0x96,
    0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5,
    0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4,
    0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3,
    0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2,
    0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA,
    0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9,
    0xEA, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8,
    0xF9, 0xFA,
];

// ============================================================
// Huffman encode table built from BITS + HUFFVAL (T.81 Annex C)
// ============================================================
//
// Indexed by symbol value (HUFFVAL byte). Symbols not present in
// the source HUFFVAL list have size==0 and must never be looked
// up; the encoder only emits symbols defined by the standard
// tables (AC RRRRSSSS and DC SSSS magnitudes are bounded by the
// magnitude-category derivation).

struct EncodeTable {
    code: [u16; 256],
    size: [u8; 256],
}

fn build_encode_table(bits: &[u8], huffval: &[u8]) -> EncodeTable {
    // Generate huffsize per Annex C Figure C.1. Sentinel 0 at
    // huffsize[k] terminates the iteration in huffcode build.
    let mut huffsize = [0u8; 257];
    let mut k = 0usize;
    for l in 1..=16 {
        for _ in 0..bits[l - 1] {
            huffsize[k] = l as u8;
            k += 1;
        }
    }
    huffsize[k] = 0;
    debug_assert_eq!(k, huffval.len(),
                     "Huffman: BITS sum != HUFFVAL length");

    // Generate huffcode per Annex C Figure C.2.
    let mut huffcode = [0u16; 257];
    if k > 0 {
        let mut code: u16 = 0;
        let mut si: u8 = huffsize[0];
        let mut p = 0usize;
        while huffsize[p] != 0 {
            while huffsize[p] == si {
                huffcode[p] = code;
                p += 1;
                code = code.wrapping_add(1);
            }
            if huffsize[p] == 0 {
                break;
            }
            // Bump code length until it matches huffsize[p].
            loop {
                code <<= 1;
                si += 1;
                if huffsize[p] == si {
                    break;
                }
            }
        }
    }

    let mut t = EncodeTable {
        code: [0u16; 256],
        size: [0u8; 256],
    };
    for i in 0..k {
        let v = huffval[i] as usize;
        t.code[v] = huffcode[i];
        t.size[v] = huffsize[i];
    }
    t
}

// ============================================================
// IJG quality scaling
// ============================================================
//
// T.81 doesn't standardise quality scaling — the IJG formula is
// the de-facto convention shipped by libjpeg / libjpeg-turbo and
// consumed by every major JPEG decoder.

fn scale_quant(reference: &[u8; 64], quality: u8) -> [u8; 64] {
    // Clamp to [1, 100]: q==0 would divide by zero in the q<50
    // branch; libjpeg clamps to 1. q==100 → scale=0, every entry
    // rounds to 1 (unit table — near-lossless).
    let q = (quality as i32).clamp(1, 100);
    let scale = if q < 50 { 5000 / q } else { 200 - q * 2 };

    let mut out = [0u8; 64];
    for i in 0..64 {
        let v = ((reference[i] as i32 * scale + 50) / 100).clamp(1, 255);
        out[i] = v as u8;
    }
    out
}

// ============================================================
// Forward DCT (T.81 §A.3.3)
// ============================================================
//
// Direct float-precision separable 2-D DCT. Mirrors the IDCT in
// foundation::dctdecode (row-column decomposition with Cu folded
// into the cosine coefficients). The encoder applies the level
// shift (subtract 128) before the DCT to bring 8-bit unsigned
// samples into the signed [-128, 127] range required by §A.3.1.

// cos_table[u][x] = C(u) * cos((2x+1)uπ/16); C(0)=1/√2 else 1.
// Computed once in `cos_table()`; the per-call cost is one match
// against a `OnceLock`-guarded init, but std-only and zero deps
// means we just reconstruct each call (8x8 = 64 cos calls; the
// cost is dwarfed by the per-block double DCT pass on real
// images and avoids any global mutability).
fn cos_table() -> [[f32; 8]; 8] {
    // PI from std::f32::consts to keep the value bit-equal to the
    // C++ port's literal (3.14159265358979323846f, which f32
    // rounds to PI anyway).
    let pi = std::f32::consts::PI;
    let invsqrt2 = 1.0_f32 / 2.0_f32.sqrt();
    let mut t = [[0.0_f32; 8]; 8];
    for u in 0..8 {
        let cu = if u == 0 { invsqrt2 } else { 1.0_f32 };
        for x in 0..8 {
            t[u][x] = cu
                * ((2.0_f32 * x as f32 + 1.0_f32)
                    * u as f32
                    * pi
                    / 16.0_f32)
                    .cos();
        }
    }
    t
}

// Forward 8x8 DCT. ``input`` is 64 signed level-shifted samples
// in row-major order; ``out`` receives 64 coefficients in
// natural row-major order.
fn forward_dct_8x8(input: &[f32; 64], out: &mut [f32; 64]) {
    let c = cos_table();
    let mut tmp = [0.0_f32; 64];
    // Row pass: tmp[y*8 + u] = sum_x C(u)cos((2x+1)uπ/16) * in[y*8 + x]
    for y in 0..8 {
        for u in 0..8 {
            let mut s = 0.0_f32;
            for x in 0..8 {
                s += c[u][x] * input[y * 8 + x];
            }
            tmp[y * 8 + u] = s;
        }
    }
    // Column pass: out[v*8 + u] = sum_y C(v)cos((2y+1)vπ/16) * tmp[y*8 + u]
    // Folded scaling: each pass contributes a 1/2 from the
    // C(0)=1/√2 row/col entry plus the implicit cos-sum
    // normalisation; the 0.25 factor below makes the overall
    // DCT scale match T.81 §A.3.3 (where Cu*Cv folded gives the
    // same 1/4).
    for v in 0..8 {
        for u in 0..8 {
            let mut s = 0.0_f32;
            for y in 0..8 {
                s += c[v][y] * tmp[y * 8 + u];
            }
            out[v * 8 + u] = s * 0.25_f32;
        }
    }
}

// ============================================================
// Bit writer with byte-stuffing (T.81 §F.1.2.3)
// ============================================================
//
// MSB-first bit accumulator. On each complete output byte equal
// to 0xFF, append a 0x00 stuffing byte to distinguish the data
// 0xFF from a marker prefix. `flush` emits the trailing partial
// byte (if any), padded with 1-bits per §F.1.2.3 — the 1-pad
// keeps the trailing partial from masquerading as an EOB-prefix
// half-byte.

struct BitWriter<'a> {
    out: &'a mut Vec<u8>,
    buf: u32,
    bits: i32,
}

impl<'a> BitWriter<'a> {
    fn new(out: &'a mut Vec<u8>) -> Self {
        Self {
            out,
            buf: 0,
            bits: 0,
        }
    }

    /// Append the low `n` bits of `v` to the stream, MSB-first.
    /// `n` is 0..=16 in baseline JPEG (DC magnitudes ≤ 11 bits,
    /// AC magnitudes ≤ 10 bits, plus Huffman codes ≤ 16 bits).
    fn put(&mut self, v: u32, n: i32) {
        if n == 0 {
            return;
        }
        let mask = if n >= 32 { u32::MAX } else { (1u32 << n) - 1 };
        self.buf = (self.buf << n) | (v & mask);
        self.bits += n;
        while self.bits >= 8 {
            let b = ((self.buf >> (self.bits - 8)) & 0xFF) as u8;
            self.bits -= 8;
            self.out.push(b);
            if b == 0xFF {
                self.out.push(0x00); // T.81 §F.1.2.3
            }
        }
    }

    /// Emit the trailing partial byte padded with 1-bits, then
    /// reset state.
    fn flush(&mut self) {
        if self.bits > 0 {
            let pad = 8 - self.bits;
            self.buf = (self.buf << pad) | ((1u32 << pad) - 1);
            self.bits = 8;
            let b = (self.buf & 0xFF) as u8;
            self.bits = 0;
            self.buf = 0;
            self.out.push(b);
            if b == 0xFF {
                self.out.push(0x00);
            }
        }
    }
}

// ============================================================
// Marker emit helpers
// ============================================================

fn push_byte(out: &mut Vec<u8>, b: u8) {
    out.push(b);
}

fn push_u16_be(out: &mut Vec<u8>, v: u16) {
    out.push((v >> 8) as u8);
    out.push((v & 0xFF) as u8);
}

fn emit_marker(out: &mut Vec<u8>, marker: u8) {
    push_byte(out, 0xFF);
    push_byte(out, marker);
}

// SOI / EOI are markers without a length-prefixed segment.

// DQT segment — Pq (precision) = 0 (8-bit). Tq (table id) selects
// destination 0 (luma) or 1 (chroma). Both tables in one DQT
// segment when RGB; just luma when Grayscale. T.81 §B.2.4.1.
fn emit_dqt(out: &mut Vec<u8>, luma: &[u8; 64], chroma: Option<&[u8; 64]>) {
    // Length = 2 (length field itself) + 65 per table.
    let tables = if chroma.is_some() { 2 } else { 1 };
    let len = (2 + tables * 65) as u16;
    emit_marker(out, M_DQT);
    push_u16_be(out, len);

    push_byte(out, 0x00); // Pq=0, Tq=0 (luma)
    for i in 0..64 {
        push_byte(out, luma[ZIGZAG[i]]);
    }

    if let Some(c) = chroma {
        push_byte(out, 0x01); // Pq=0, Tq=1 (chroma)
        for i in 0..64 {
            push_byte(out, c[ZIGZAG[i]]);
        }
    }
}

// SOF0 segment — baseline sequential, 8-bit precision. T.81
// §B.2.2. Components carry id / sampling factors / quant table
// selector. For 4:4:4 every component is 1×1; Y uses quant 0,
// Cb / Cr use quant 1.
fn emit_sof0(out: &mut Vec<u8>, width: u16, height: u16, grayscale: bool) {
    let nf = if grayscale { 1 } else { 3 };
    let len = (8 + 3 * nf) as u16;
    emit_marker(out, M_SOF0);
    push_u16_be(out, len);
    push_byte(out, 8); // sample precision
    push_u16_be(out, height);
    push_u16_be(out, width);
    push_byte(out, nf as u8);

    if grayscale {
        push_byte(out, 1); // component id
        push_byte(out, 0x11); // sampling factor 1×1
        push_byte(out, 0); // quant table id
    } else {
        push_byte(out, 1); push_byte(out, 0x11); push_byte(out, 0); // Y
        push_byte(out, 2); push_byte(out, 0x11); push_byte(out, 1); // Cb
        push_byte(out, 3); push_byte(out, 0x11); push_byte(out, 1); // Cr
    }
}

// DHT segment. One segment carries 4 tables for RGB (DC-Y, AC-Y,
// DC-C, AC-C) or 2 tables for Grayscale (DC-Y, AC-Y). Tc=0 is
// DC-class, Tc=1 is AC-class; Th picks the destination slot
// (0 = luma, 1 = chroma). T.81 §B.2.4.2.
fn emit_dht(out: &mut Vec<u8>, grayscale: bool) {
    let tables = if grayscale { 2 } else { 4 };
    let mut len: u16 = 2;
    for t in 0..tables {
        // 1 byte Tc/Th + 16 bytes BITS + N bytes HUFFVAL.
        let n = match t {
            0 => DC_LUM_VALS.len(),
            1 => AC_LUM_VALS.len(),
            2 => DC_CHROM_VALS.len(),
            3 => AC_CHROM_VALS.len(),
            _ => unreachable!(),
        };
        len += (1 + 16 + n) as u16;
    }
    emit_marker(out, M_DHT);
    push_u16_be(out, len);

    fn write_table(out: &mut Vec<u8>, tc_th: u8, bits: &[u8], vals: &[u8]) {
        push_byte(out, tc_th);
        for &b in bits {
            push_byte(out, b);
        }
        for &v in vals {
            push_byte(out, v);
        }
    }

    write_table(out, 0x00, &DC_LUM_BITS, &DC_LUM_VALS); // DC luma
    write_table(out, 0x10, &AC_LUM_BITS, &AC_LUM_VALS); // AC luma
    if !grayscale {
        write_table(out, 0x01, &DC_CHROM_BITS, &DC_CHROM_VALS); // DC chroma
        write_table(out, 0x11, &AC_CHROM_BITS, &AC_CHROM_VALS); // AC chroma
    }
}

// SOS segment — baseline scan over every component. Spectral
// selection 0..63, Ah=Al=0 (no successive approximation).
// T.81 §B.2.3.
fn emit_sos(out: &mut Vec<u8>, grayscale: bool) {
    let ns = if grayscale { 1 } else { 3 };
    let len = (6 + 2 * ns) as u16;
    emit_marker(out, M_SOS);
    push_u16_be(out, len);
    push_byte(out, ns as u8);
    if grayscale {
        push_byte(out, 1);    // Cs1 = Y component id
        push_byte(out, 0x00); // Td=0 / Ta=0 (luma DC + AC tables)
    } else {
        push_byte(out, 1); push_byte(out, 0x00); // Y:  DC=0, AC=0
        push_byte(out, 2); push_byte(out, 0x11); // Cb: DC=1, AC=1
        push_byte(out, 3); push_byte(out, 0x11); // Cr: DC=1, AC=1
    }
    push_byte(out, 0x00); // Ss=0
    push_byte(out, 0x3F); // Se=63
    push_byte(out, 0x00); // Ah=0, Al=0
}

// ============================================================
// DC / AC coefficient encoding (T.81 §F.1.2.1, §F.1.2.2)
// ============================================================

// SSSS magnitude category for a (signed) coefficient value.
// Category k spans [-(2^k - 1) .. -2^(k-1)] ∪ [2^(k-1) .. 2^k - 1].
// k=0 is the diff==0 case.
fn category(v: i32) -> i32 {
    if v == 0 {
        return 0;
    }
    let mut a = if v < 0 { -v } else { v };
    let mut k = 0;
    while a != 0 {
        k += 1;
        a >>= 1;
    }
    k
}

// Encoded magnitude bits per T.81 §F.1.2.1.3: for positive v the
// raw bits are simply the low `size` bits of v; for negative v
// the bias v + (1<<size) - 1 is used (one's-complement negative
// form).
fn mag_bits(v: i32, size: i32) -> u32 {
    if size == 0 {
        return 0;
    }
    if v >= 0 {
        v as u32
    } else {
        (v + ((1 << size) - 1)) as u32
    }
}

// Encode one DC coefficient (already differential-coded) as a
// (size Huffman code, raw magnitude bits) pair.
fn encode_dc(bw: &mut BitWriter<'_>, diff: i32, dc: &EncodeTable) {
    let s = category(diff);
    debug_assert!(s <= 11, "DC: magnitude bits > 11");
    let s_idx = s as usize;
    bw.put(dc.code[s_idx] as u32, dc.size[s_idx] as i32);
    if s != 0 {
        bw.put(mag_bits(diff, s), s);
    }
}

// Encode 63 AC coefficients in zig-zag order with run/size
// packing (T.81 §F.1.2.2). zz[1..63] are the AC band; zz[0] is
// DC and is encoded separately by encode_dc.
fn encode_ac(bw: &mut BitWriter<'_>, zz: &[i32; 64], ac: &EncodeTable) {
    let mut run = 0;
    for k in 1..64 {
        let v = zz[k];
        if v == 0 {
            run += 1;
            continue;
        }
        // ZRL for every full 16-zero run before a non-zero.
        while run >= 16 {
            // ZRL: RRRR=15, SSSS=0 → packed byte 0xF0.
            bw.put(ac.code[0xF0] as u32, ac.size[0xF0] as i32);
            run -= 16;
        }
        let s = category(v);
        debug_assert!(s <= 10, "AC: magnitude bits > 10");
        let rs = ((run << 4) | s) as usize;
        bw.put(ac.code[rs] as u32, ac.size[rs] as i32);
        bw.put(mag_bits(v, s), s);
        run = 0;
    }
    if run > 0 {
        // EOB: RRRR=0, SSSS=0 → packed byte 0x00.
        bw.put(ac.code[0x00] as u32, ac.size[0x00] as i32);
    }
}

// ============================================================
// Color conversion + level shift (per 8×8 block)
// ============================================================
//
// JFIF Appendix A / BT.601:
//   Y  =  0.299 R + 0.587 G + 0.114 B
//   Cb = -0.168736 R - 0.331264 G + 0.5 B + 128
//   Cr =  0.5 R - 0.418688 G - 0.081312 B + 128
// Round-to-nearest is the implicit f32 round when we later quantise
// the coefficient by `lroundf`. Subtract 128 to put samples into
// the signed [-128, 127] range required by T.81 §A.3.1.

fn y_from_rgb(r: f32, g: f32, b: f32) -> f32 {
    0.299_f32 * r + 0.587_f32 * g + 0.114_f32 * b
}

fn cb_from_rgb(r: f32, g: f32, b: f32) -> f32 {
    -0.168736_f32 * r - 0.331264_f32 * g + 0.5_f32 * b + 128.0_f32
}

fn cr_from_rgb(r: f32, g: f32, b: f32) -> f32 {
    0.5_f32 * r - 0.418688_f32 * g - 0.081312_f32 * b + 128.0_f32
}

// Encode one component block: forward DCT, quantise + zigzag,
// emit DC + AC. `dc_pred` is the previous block's DC for this
// same component (initialised to 0 at scan start).
fn encode_block(
    bw: &mut BitWriter<'_>,
    samples_signed: &[f32; 64],
    q_natural: &[u8; 64],
    dc: &EncodeTable,
    ac: &EncodeTable,
    dc_pred: &mut i32,
) {
    let mut coefs = [0.0_f32; 64];
    forward_dct_8x8(samples_signed, &mut coefs);

    // Quantise (round-to-nearest) into the zig-zag scan order.
    // `n = ZIGZAG[k]` is the natural-order index of the k-th
    // zig-zag coefficient; both the coef and the quant entry
    // sit at `n`.
    let mut zz = [0_i32; 64];
    for k in 0..64 {
        let n = ZIGZAG[k];
        let qv = q_natural[n] as f32;
        // round-to-nearest, ties-away-from-zero (matches lroundf).
        let r = coefs[n] / qv;
        let rounded = if r >= 0.0 {
            (r + 0.5).floor()
        } else {
            -((-r + 0.5).floor())
        };
        zz[k] = rounded as i32;
    }

    let dc_val = zz[0];
    let diff = dc_val - *dc_pred;
    *dc_pred = dc_val;
    encode_dc(bw, diff, dc);
    encode_ac(bw, &zz, ac);
}

// ============================================================
// Frame encode
// ============================================================

fn encode_grayscale(
    out: &mut Vec<u8>,
    width: u32,
    height: u32,
    rgba8: &[u8],
    q_lum: &[u8; 64],
    dc_l: &EncodeTable,
    ac_l: &EncodeTable,
) -> Result<(), Error> {
    // Validate Grayscale precondition: R == G == B at every pixel.
    let total = (width as usize) * (height as usize);
    for i in 0..total {
        let r = rgba8[i * 4];
        let g = rgba8[i * 4 + 1];
        let b = rgba8[i * 4 + 2];
        if r != g || g != b {
            return Err(Error::NonGrayscaleInput { pixel_index: i });
        }
    }

    let mut bw = BitWriter::new(out);
    let mut dc_pred: i32 = 0;

    let bx_count = width.div_ceil(8);
    let by_count = height.div_ceil(8);

    for by in 0..by_count {
        for bx in 0..bx_count {
            let mut blk = [0.0_f32; 64];
            for y in 0..8u32 {
                let mut src_y = by * 8 + y;
                if src_y >= height {
                    src_y = height - 1; // edge-clamp
                }
                for x in 0..8u32 {
                    let mut src_x = bx * 8 + x;
                    if src_x >= width {
                        src_x = width - 1;
                    }
                    let idx = (src_y as usize * width as usize + src_x as usize) * 4;
                    let v = rgba8[idx];
                    blk[(y as usize) * 8 + x as usize] = v as f32 - 128.0;
                }
            }
            encode_block(&mut bw, &blk, q_lum, dc_l, ac_l, &mut dc_pred);
        }
    }
    bw.flush();
    Ok(())
}

#[allow(clippy::too_many_arguments)] // JPEG scan emitter needs the full context
fn encode_rgb(
    out: &mut Vec<u8>,
    width: u32,
    height: u32,
    rgba8: &[u8],
    q_lum: &[u8; 64],
    q_chrom: &[u8; 64],
    dc_l: &EncodeTable,
    ac_l: &EncodeTable,
    dc_c: &EncodeTable,
    ac_c: &EncodeTable,
) {
    let mut bw = BitWriter::new(out);
    let mut dc_y: i32 = 0;
    let mut dc_cb: i32 = 0;
    let mut dc_cr: i32 = 0;

    let bx_count = width.div_ceil(8);
    let by_count = height.div_ceil(8);

    // 4:4:4 sampling: every MCU contributes one block per
    // component, scanned in (Y, Cb, Cr) order. Block padding
    // replicates the last in-image column / row when the pixel
    // grid isn't a multiple of 8.
    for by in 0..by_count {
        for bx in 0..bx_count {
            let mut yblk = [0.0_f32; 64];
            let mut cbblk = [0.0_f32; 64];
            let mut crblk = [0.0_f32; 64];
            for yy in 0..8u32 {
                let mut src_y = by * 8 + yy;
                if src_y >= height {
                    src_y = height - 1;
                }
                for xx in 0..8u32 {
                    let mut src_x = bx * 8 + xx;
                    if src_x >= width {
                        src_x = width - 1;
                    }
                    let p = (src_y as usize * width as usize + src_x as usize) * 4;
                    let r = rgba8[p] as f32;
                    let g = rgba8[p + 1] as f32;
                    let b = rgba8[p + 2] as f32;
                    let yv = y_from_rgb(r, g, b);
                    let cb = cb_from_rgb(r, g, b);
                    let cr = cr_from_rgb(r, g, b);
                    let i = (yy as usize) * 8 + xx as usize;
                    yblk[i] = yv - 128.0;
                    cbblk[i] = cb - 128.0;
                    crblk[i] = cr - 128.0;
                }
            }
            encode_block(&mut bw, &yblk, q_lum, dc_l, ac_l, &mut dc_y);
            encode_block(&mut bw, &cbblk, q_chrom, dc_c, ac_c, &mut dc_cb);
            encode_block(&mut bw, &crblk, q_chrom, dc_c, ac_c, &mut dc_cr);
        }
    }
    bw.flush();
}

// ============================================================
// Public entry point
// ============================================================

/// Encode `rgba8` (row-major top-to-bottom, R G B A) into a
/// JPEG baseline byte stream (T.81 Annex F sequential DCT,
/// 8-bit samples, 4:4:4 chroma sampling, no APP markers).
///
/// Errors on out-of-range dimensions, wrong buffer length,
/// non-grayscale input under [`Photometric::Grayscale`], or
/// non-positive `dpi`. `quality` outside 0..=100 is clamped
/// internally per the IJG reference scaling formula.
pub fn encode(
    width: u32,
    height: u32,
    rgba8: &[u8],
    options: &Options,
) -> Result<Vec<u8>, Error> {
    if width == 0 {
        return Err(Error::ZeroWidth);
    }
    if height == 0 {
        return Err(Error::ZeroHeight);
    }
    if width > 65535 {
        return Err(Error::WidthOutOfRange { width });
    }
    if height > 65535 {
        return Err(Error::HeightOutOfRange { height });
    }
    let expected = (width as usize) * (height as usize) * 4;
    if rgba8.len() != expected {
        return Err(Error::WrongRgbaLength {
            expected,
            got: rgba8.len(),
        });
    }
    if options.dpi == 0 {
        return Err(Error::DpiNotPositive);
    }

    let grayscale = matches!(options.photometric, Photometric::Grayscale);

    let q_lum = scale_quant(&Q_LUM, options.quality);
    let q_chrom = scale_quant(&Q_CHROM, options.quality);

    let dc_l = build_encode_table(&DC_LUM_BITS, &DC_LUM_VALS);
    let ac_l = build_encode_table(&AC_LUM_BITS, &AC_LUM_VALS);
    let dc_c = build_encode_table(&DC_CHROM_BITS, &DC_CHROM_VALS);
    let ac_c = build_encode_table(&AC_CHROM_BITS, &AC_CHROM_VALS);

    // Reserve roughly 1 byte/pixel + header slack — typical Q75
    // RGB output is 0.2-0.5 byte/pixel; this avoids the first
    // few reallocs even on dense imagery.
    let mut out: Vec<u8> =
        Vec::with_capacity((width as usize) * (height as usize) + 1024);

    emit_marker(&mut out, M_SOI);
    emit_dqt(
        &mut out,
        &q_lum,
        if grayscale { None } else { Some(&q_chrom) },
    );
    emit_sof0(&mut out, width as u16, height as u16, grayscale);
    emit_dht(&mut out, grayscale);
    emit_sos(&mut out, grayscale);

    if grayscale {
        encode_grayscale(&mut out, width, height, rgba8, &q_lum, &dc_l, &ac_l)?;
    } else {
        encode_rgb(
            &mut out, width, height, rgba8, &q_lum, &q_chrom,
            &dc_l, &ac_l, &dc_c, &ac_c,
        );
    }

    emit_marker(&mut out, M_EOI);
    Ok(out)
}
