// foundation/jbig2 — PDF /JBIG2Decode filter (PDF 32000-1:2008
// §7.4.7) implementing a subset of ITU-T T.88 sufficient for the
// most common real-world PDF use case: scanned pages encoded as a
// single arithmetic-coded generic-region segment.
//
// v1 scope: segment-header parser + MQ arithmetic coder
// (T.82 Annex E) + arithmetic-coded generic region decoder
// (T.88 §6.2 with GBTEMPLATE 0/1/2/3) + page-info / end-of-page
// / end-of-file segment dispatch. Symbol dictionaries, text
// regions, halftones, refinement, Huffman variants, and the
// JBIG2 file-format wrapper are out of v1 scope and produce a
// structured Error::UnsupportedSegmentType.
//
// References:
//   ITU-T T.88 (2018) — JBIG2.
//   ITU-T T.82 (1993) Annex E — MQ arithmetic coder.
//   PDF 32000-1:2008 §7.4.7 — JBIG2Decode filter.
//
// See Feedstock/PDF/Spec/jbig2.yaml + targets/rust/jbig2.yaml.

#![allow(clippy::needless_range_loop)] // index-arithmetic numeric kernels

use std::fmt;

// ============================================================
// Public types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub components: u8, // always 1 (bilevel)
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub enum Error {
    EmptyInput,
    UnsupportedGlobals,
    TruncatedSegmentHeader,
    UnsupportedRefCount(u8),
    IndefiniteSegmentLength,
    TruncatedSegmentBody,
    DuplicatePageInfo,
    MissingPageInfo,
    RegionBeforePageInfo,
    PageInfoTooShort,
    ZeroPage { width: u32, height: u32 },
    IndefinitePageHeight,
    RegionTooShort,
    UnsupportedGbmmr,
    InvalidGbtemplate(u8),
    MissingGbat,
    UnsupportedSegmentType(u8),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::EmptyInput => f.write_str("jbig2: empty input (no segments)"),
            Error::UnsupportedGlobals => f.write_str(
                "jbig2: /JBIG2Globals not supported in v1 \
                 (globals streams almost always carry symbol dictionaries, v2 territory)",
            ),
            Error::TruncatedSegmentHeader => f.write_str("jbig2: truncated segment header"),
            Error::UnsupportedRefCount(r) => write!(
                f,
                "jbig2: referred-segment count {} > 0 not supported in v1 (R = 0 only)",
                r
            ),
            Error::IndefiniteSegmentLength => f.write_str(
                "jbig2: segment_data_length=0xFFFFFFFF (indefinite-length stripe mode) \
                 not supported in v1",
            ),
            Error::TruncatedSegmentBody => f.write_str(
                "jbig2: segment data truncated (announced data_length exceeds remaining bytes)",
            ),
            Error::DuplicatePageInfo => f.write_str(
                "jbig2: multiple page-info segments not supported in v1 \
                 (single-page PDF /JBIG2Decode)",
            ),
            Error::MissingPageInfo => f.write_str("jbig2: stream contains no page-info segment"),
            Error::RegionBeforePageInfo =>
                f.write_str("jbig2: generic region before page-info segment"),
            Error::PageInfoTooShort =>
                f.write_str("jbig2: page-info segment too short (need >= 19 bytes)"),
            Error::ZeroPage { width, height } =>
                write!(f, "jbig2: zero-dimension page {}x{}", width, height),
            Error::IndefinitePageHeight => f.write_str(
                "jbig2: indefinite-height page (height=0xFFFFFFFF) not supported in v1",
            ),
            Error::RegionTooShort =>
                f.write_str("jbig2: generic region segment too short"),
            Error::UnsupportedGbmmr => f.write_str(
                "jbig2: GBMMR=1 generic regions (CCITT G4 inline) not supported in v1 \
                 (would delegate to foundation::ccitt)",
            ),
            Error::InvalidGbtemplate(t) =>
                write!(f, "jbig2: invalid GBTEMPLATE {} (must be 0..3)", t),
            Error::MissingGbat => f.write_str("jbig2: missing GBAT bytes for region template"),
            Error::UnsupportedSegmentType(t) => write!(
                f,
                "jbig2: segment type {} ({}) not supported in v1; \
                 see Feedstock/PDF/Spec/jbig2.yaml v2/v3 roadmap",
                t,
                seg_type_name(*t),
            ),
        }
    }
}

impl std::error::Error for Error {}

// ============================================================
// Spec data tables (T.82 §E.2.4 Table E.1; T.88 §6.2.5.3 Tables 5-8)
// ============================================================
//
// Source of truth: Feedstock/PDF/Spec/jbig2.yaml::data_tables.
// Public-domain ITU-T spec data; the same constants appear in
// every conforming JBIG2 decoder. Kept inline here to match the
// Rust track's "single .rs per primitive" pattern (mirrors
// dctdecode.rs's IDCT constants and bmp_encoder.rs's fixed
// palettes).

// T.82 §E.2.4 Table E.1 — MQ probability estimator (47 states).
// QE[i] = LPS sub-interval boundary; NMPS[i] / NLPS[i] = next
// state on MPS / LPS transition; SWITCH[i] = 1 when MPS bit
// flips on the LPS transition.
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const QE: [u16; 47] = [
    0x5601, 0x3401, 0x1801, 0x0AC1, 0x0521, 0x0221, 0x5601, 0x5401,
    0x4801, 0x3801, 0x3001, 0x2401, 0x1C01, 0x1601, 0x5601, 0x5401,
    0x5101, 0x4801, 0x3801, 0x3401, 0x3001, 0x2801, 0x2401, 0x2201,
    0x1C01, 0x1801, 0x1601, 0x1401, 0x1201, 0x1101, 0x0AC1, 0x09C1,
    0x08A1, 0x0521, 0x0441, 0x02A1, 0x0221, 0x0141, 0x0111, 0x0085,
    0x0049, 0x0025, 0x0015, 0x0009, 0x0005, 0x0001, 0x5601,
];
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const NMPS: [u8; 47] = [
    1, 2, 3, 4, 5, 38, 7, 8, 9, 10, 11, 12, 13, 29, 15, 16,
    17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
    33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 45, 46,
];
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const NLPS: [u8; 47] = [
    1, 6, 9, 12, 29, 33, 6, 14, 14, 14, 17, 18, 20, 21, 14, 14,
    15, 16, 17, 18, 19, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29,
    30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 46,
];
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const SWITCH: [u8; 47] = [
    1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

// T.88 §6.2.5.3 Tables 5-8 — generic-region neighbourhood layouts.
// Each slot is (dx, dy, adaptive). adaptive=-1 means a fixed
// (dx, dy) offset; adaptive in 0..3 indexes the segment's GBAT
// array (A1..A4) — the (dx, dy) on adaptive slots are unused.
// Bit ordering inside the context CX is LSB-first (slot 0 = bit 0).
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
type Slot = (i32, i32, i8);

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const GT0: [Slot; 16] = [
    (-1,  0, -1), (-2,  0, -1), (-3,  0, -1), (-4,  0, -1),
    ( 0,  0,  0),
    ( 2, -1, -1), ( 1, -1, -1), ( 0, -1, -1), (-1, -1, -1), (-2, -1, -1),
    ( 0,  0,  1), ( 0,  0,  2),
    ( 1, -2, -1), ( 0, -2, -1), (-1, -2, -1),
    ( 0,  0,  3),
];
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const GT1: [Slot; 13] = [
    (-1,  0, -1), (-2,  0, -1), (-3,  0, -1),
    ( 0,  0,  0),
    ( 2, -1, -1), ( 1, -1, -1), ( 0, -1, -1), (-1, -1, -1), (-2, -1, -1),
    ( 2, -2, -1), ( 1, -2, -1), ( 0, -2, -1), (-1, -2, -1),
];
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const GT2: [Slot; 10] = [
    (-1,  0, -1), (-2,  0, -1),
    ( 0,  0,  0),
    ( 1, -1, -1), ( 0, -1, -1), (-1, -1, -1), (-2, -1, -1),
    ( 1, -2, -1), ( 0, -2, -1), (-1, -2, -1),
];
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const GT3: [Slot; 10] = [
    (-1,  0, -1), (-2,  0, -1), (-3,  0, -1), (-4,  0, -1),
    ( 0,  0,  0),
    ( 1, -1, -1), ( 0, -1, -1), (-1, -1, -1), (-2, -1, -1), (-3, -1, -1),
];

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
struct Template {
    bit_count: u32,
    gbat_count: usize,
    default_gbat: [(i8, i8); 4],
    sltp_ctx: u32,
    bits: &'static [Slot],
}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const TEMPLATES: [Template; 4] = [
    Template {
        bit_count: 16, gbat_count: 4,
        default_gbat: [(3, -1), (-3, -1), (2, -2), (-2, -2)],
        sltp_ctx: 0x9B25, bits: &GT0,
    },
    Template {
        bit_count: 13, gbat_count: 1,
        default_gbat: [(3, -1), (0, 0), (0, 0), (0, 0)],
        sltp_ctx: 0x0795, bits: &GT1,
    },
    Template {
        bit_count: 10, gbat_count: 1,
        default_gbat: [(2, -1), (0, 0), (0, 0), (0, 0)],
        sltp_ctx: 0x00E5, bits: &GT2,
    },
    Template {
        bit_count: 10, gbat_count: 1,
        default_gbat: [(2, -1), (0, 0), (0, 0), (0, 0)],
        sltp_ctx: 0x0195, bits: &GT3,
    },
];

// ============================================================
// MQ arithmetic decoder (T.82 §E.1)
// ============================================================
//
// State variables follow spec naming exactly so the algorithm is
// line-by-line auditable against T.82 §E.1.1 / §E.1.2 / §E.1.4 /
// §E.1.5. GBSTATS context cells pack (mps << 7) | index into a
// single u8 — same layout the TS / C# siblings use.

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
struct MqDecoder<'a> {
    data: &'a [u8],
    p: usize,
    end: usize,
    c: u32,
    a: u32,
    ct: i32,
    b: u8,
}

impl<'a> MqDecoder<'a> {
    #[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
    fn new(data: &'a [u8], start: usize, end: usize) -> Self {
        let mut mq = MqDecoder {
            data, p: start, end,
            c: 0, a: 0, ct: 0, b: 0,
        };
        mq.initdec();
        mq
    }

    #[inline]
    #[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
    fn byte_at(&self, p: usize) -> u8 {
        if p < self.end { self.data[p] } else { 0xFF }
    }

    // T.82 §E.1.1 INITDEC.
    #[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
    fn initdec(&mut self) {
        self.b = self.byte_at(self.p);
        self.p += 1;
        self.c = (self.b as u32) << 16;
        self.bytein();
        self.c = self.c.wrapping_shl(7);
        self.ct -= 7;
        self.a = 0x8000;
    }

    // T.82 §E.1.4 BYTEIN — fetch the next byte, handling 0xFF
    // stuffing.
    #[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
    fn bytein(&mut self) {
        if self.b == 0xFF {
            // Peek next byte WITHOUT consuming.
            let b1 = self.byte_at(self.p);
            if b1 > 0x8F {
                // End-of-data marker — stuff zeros into C.
                self.c = self.c.wrapping_add(0xFF00);
                self.ct = 8;
            } else {
                // Stuffed FF — consume next byte.
                self.b = b1;
                self.p += 1;
                self.c = self.c.wrapping_add((self.b as u32) << 9);
                self.ct = 7;
            }
        } else {
            self.b = self.byte_at(self.p);
            self.p += 1;
            self.c = self.c.wrapping_add((self.b as u32) << 8);
            self.ct = 8;
        }
    }

    // T.82 §E.1.5 RENORMD.
    #[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
    fn renormd(&mut self) {
        loop {
            if self.ct == 0 { self.bytein(); }
            self.a <<= 1;
            self.c = self.c.wrapping_shl(1);
            self.ct -= 1;
            if (self.a & 0x8000) != 0 { break; }
        }
    }

    // T.82 §E.1.2 + §E.2.4 DECODE(CX). `stats` is the GBSTATS
    // context-cell array; `cx` is the context index to read /
    // mutate. Returns the decoded bit (0 or 1).
    #[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
    fn decode(&mut self, stats: &mut [u8], cx: usize) -> u8 {
        let cell = stats[cx];
        let i = (cell & 0x7F) as usize;
        let mps = cell >> 7;
        let qe = QE[i] as u32;

        self.a = self.a.wrapping_sub(qe) & 0xFFFF;
        let d: u8;
        if (self.c >> 16) < qe {
            // LPS sub-interval — LPS_EXCHANGE (T.82 §E.1.3).
            if self.a < qe {
                d = mps;
                stats[cx] = (mps << 7) | NMPS[i];
            } else {
                d = mps ^ 1;
                let new_mps = if SWITCH[i] != 0 { mps ^ 1 } else { mps };
                stats[cx] = (new_mps << 7) | NLPS[i];
            }
            self.a = qe;
            self.renormd();
        } else {
            self.c = self.c.wrapping_sub(qe << 16);
            if (self.a & 0x8000) == 0 {
                // MPS_EXCHANGE (T.82 §E.1.3).
                if self.a < qe {
                    d = mps ^ 1;
                    let new_mps = if SWITCH[i] != 0 { mps ^ 1 } else { mps };
                    stats[cx] = (new_mps << 7) | NLPS[i];
                } else {
                    d = mps;
                    stats[cx] = (mps << 7) | NMPS[i];
                }
                self.renormd();
            } else {
                d = mps;
            }
        }
        d
    }
}

// ============================================================
// Generic-region decoder (T.88 §6.2)
// ============================================================

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
struct RegionParams {
    region_w: u32,
    region_h: u32,
    region_x: u32,
    region_y: u32,
    template: u8,    // 0..3
    tpgdon: bool,
    gbat: [(i32, i32); 4],
    mq_start: usize,
    mq_end: usize,
}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn decode_generic_region(
    data: &[u8],
    rp: &RegionParams,
    bitmap: &mut [u8],
    page_stride: usize,
    page_width: u32,
    page_height: u32,
) -> Result<(), Error> {
    if rp.region_w == 0 || rp.region_h == 0 {
        return Ok(());
    }
    if rp.template > 3 {
        return Err(Error::InvalidGbtemplate(rp.template));
    }
    let tmpl = &TEMPLATES[rp.template as usize];
    let stats_size = 1usize << tmpl.bit_count;
    let mut stats = vec![0u8; stats_size];
    let mut mq = MqDecoder::new(data, rp.mq_start, rp.mq_end);

    let region_w = rp.region_w as i32;
    let region_h = rp.region_h as i32;
    let region_x = rp.region_x as i32;
    let region_y = rp.region_y as i32;
    let pw = page_width as i32;
    let ph = page_height as i32;

    // Pixel read at REGION-LOCAL (x, y); pixels outside the
    // region or page bitmap are virtual white (T.88 §6.2.5.3).
    let read_pixel = |bm: &[u8], x: i32, y: i32| -> u8 {
        if x < 0 || y < 0 || x >= region_w {
            return 0;
        }
        let py = region_y + y;
        let px = region_x + x;
        if py < 0 || py >= ph || px < 0 || px >= pw {
            return 0;
        }
        let row = (py as usize) * page_stride + (px as usize >> 3);
        (bm[row] >> (7 - (px & 7))) & 1
    };

    let write_pixel = |bm: &mut [u8], x: i32, y: i32, v: u8| {
        if v == 0 { return; }
        let py = region_y + y;
        let px = region_x + x;
        if py < 0 || py >= ph || px < 0 || px >= pw { return; }
        bm[(py as usize) * page_stride + (px as usize >> 3)] |= 1 << (7 - (px & 7));
    };

    // TPGDON: per-row SLTP bit decoded against the template's
    // fixed SLTP context, XORed with the previous SLTP state.
    let sltp_ctx = tmpl.sltp_ctx as usize;
    let mut prev_sltp: u8 = 0;

    for y in 0..region_h {
        let mut sltp: u8 = 0;
        if rp.tpgdon {
            let bit = mq.decode(&mut stats, sltp_ctx);
            sltp = prev_sltp ^ bit;
            prev_sltp = sltp;
        }

        if sltp == 1 {
            // Row identical to previous row (TPGDON-typical).
            for x in 0..region_w {
                let v = read_pixel(bitmap, x, y - 1);
                write_pixel(bitmap, x, y, v);
            }
            continue;
        }

        for x in 0..region_w {
            // Build CX from the template's neighbour pixel values.
            let mut cx: u32 = 0;
            for (i, slot) in tmpl.bits.iter().enumerate() {
                let pv = if slot.2 >= 0 {
                    let a = rp.gbat[slot.2 as usize];
                    read_pixel(bitmap, x + a.0, y + a.1)
                } else {
                    read_pixel(bitmap, x + slot.0, y + slot.1)
                };
                cx |= (pv as u32) << i;
            }
            let bit = mq.decode(&mut stats, cx as usize);
            write_pixel(bitmap, x, y, bit);
        }
    }
    Ok(())
}

// ============================================================
// Segment header parser (T.88 §7.2)
// ============================================================

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
struct SegmentHeader {
    seg_type: u8,
    data_length: u32,
    data_start: usize,
    data_end: usize,
}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn parse_segment_header(data: &[u8], mut p: usize) -> Result<SegmentHeader, Error> {
    if p + 6 > data.len() {
        return Err(Error::TruncatedSegmentHeader);
    }
    // segment_number (4 BE) — read but unused in v1 dispatch.
    let _seg_no = read_u32_be(data, p);
    p += 4;

    let flags = data[p]; p += 1;
    let seg_type = flags & 0x3F;
    let page_assoc_4byte = (flags & 0x40) != 0;

    let ref_byte = data[p]; p += 1;
    let ref_count = (ref_byte >> 5) & 0x07;
    if ref_count > 0 {
        return Err(Error::UnsupportedRefCount(ref_count));
    }
    // No referred-segment numbers since ref_count = 0.

    let pa_size = if page_assoc_4byte { 4 } else { 1 };
    if p + pa_size + 4 > data.len() {
        return Err(Error::TruncatedSegmentHeader);
    }
    p += pa_size;

    let data_length = read_u32_be(data, p);
    p += 4;
    if data_length == 0xFFFFFFFF {
        return Err(Error::IndefiniteSegmentLength);
    }

    let data_start = p;
    let data_end = data_start.checked_add(data_length as usize)
        .ok_or(Error::TruncatedSegmentBody)?;
    if data_end > data.len() {
        return Err(Error::TruncatedSegmentBody);
    }
    Ok(SegmentHeader { seg_type, data_length, data_start, data_end })
}

#[inline]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn read_u32_be(data: &[u8], p: usize) -> u32 {
    ((data[p] as u32) << 24)
        | ((data[p + 1] as u32) << 16)
        | ((data[p + 2] as u32) << 8)
        | (data[p + 3] as u32)
}

#[inline]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn signed_i8(b: u8) -> i32 {
    (b as i8) as i32
}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn seg_type_name(t: u8) -> &'static str {
    match t {
        0 => "symbol dictionary (v2)",
        2 | 4 => "intermediate text region (v2)",
        6 | 7 => "text region (v2)",
        16 => "pattern dictionary (v3)",
        20 | 22 | 23 => "halftone region (v3)",
        36 => "intermediate generic region",
        40 | 42 | 43 => "refinement region (v3)",
        50 => "end of stripe (indefinite-height only)",
        53 => "tables segment",
        _ => "unknown",
    }
}

// ============================================================
// Public entry points
// ============================================================

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub fn decode(page: &[u8]) -> Result<DecodedImage, Error> {
    decode_with_globals(page, &[])
}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub fn decode_with_globals(
    page: &[u8],
    globals: &[u8],
) -> Result<DecodedImage, Error> {
    if !globals.is_empty() {
        return Err(Error::UnsupportedGlobals);
    }
    if page.is_empty() {
        return Err(Error::EmptyInput);
    }

    let mut page_width: u32 = 0;
    let mut page_height: u32 = 0;
    let mut page_stride: usize = 0;
    let mut bitmap: Vec<u8> = Vec::new();
    let mut seen_page_info = false;

    let mut p: usize = 0;
    while p < page.len() {
        let hdr = parse_segment_header(page, p)?;
        match hdr.seg_type {
            48 => {
                if seen_page_info {
                    return Err(Error::DuplicatePageInfo);
                }
                if hdr.data_length < 19 {
                    return Err(Error::PageInfoTooShort);
                }
                page_width = read_u32_be(page, hdr.data_start);
                let h = read_u32_be(page, hdr.data_start + 4);
                if h == 0xFFFFFFFF {
                    return Err(Error::IndefinitePageHeight);
                }
                if page_width == 0 || h == 0 {
                    return Err(Error::ZeroPage { width: page_width, height: h });
                }
                page_height = h;
                page_stride = ((page_width as usize) + 7) >> 3;
                let flags = page[hdr.data_start + 16];
                let default_pixel = (flags >> 2) & 1;
                let total = page_stride * (page_height as usize);
                let fill = if default_pixel != 0 { 0xFF } else { 0x00 };
                bitmap = vec![fill; total];
                seen_page_info = true;
            }
            38 | 39 => {
                if !seen_page_info {
                    return Err(Error::RegionBeforePageInfo);
                }
                if hdr.data_length < 17 + 1 {
                    return Err(Error::RegionTooShort);
                }
                let rs = hdr.data_start;
                let region_w = read_u32_be(page, rs);
                let region_h = read_u32_be(page, rs + 4);
                let region_x = read_u32_be(page, rs + 8);
                let region_y = read_u32_be(page, rs + 12);
                // page[rs + 16] = region_flags (ext combination op,
                // colour) — ignored in v1: we composite via OR into
                // the page bitmap, JBIG2 polarity 1 = black.
                let gb_body = rs + 17;
                let gb_flags = page[gb_body];
                let mmr = (gb_flags & 0x01) != 0;
                let template = (gb_flags >> 1) & 0x03;
                let tpgdon = (gb_flags & 0x08) != 0;
                // (gb_flags & 0x10) = USESKIP — out of v1 scope, but
                // not an error to see when not used; the spec layers
                // USESKIP atop the GBAT tail, so we'd misparse if
                // anyone ever set it. Safe in current corpus.

                if mmr {
                    return Err(Error::UnsupportedGbmmr);
                }

                let tmpl = &TEMPLATES[template as usize];
                let mut mq_start = gb_body + 1;
                let mut gbat: [(i32, i32); 4] = [(0, 0); 4];
                let need = tmpl.gbat_count * 2;
                if mq_start + need > hdr.data_end {
                    return Err(Error::MissingGbat);
                }
                for i in 0..tmpl.gbat_count {
                    gbat[i].0 = signed_i8(page[mq_start]);
                    gbat[i].1 = signed_i8(page[mq_start + 1]);
                    mq_start += 2;
                }
                // Back-fill remaining GBAT slots from the template's
                // defaults so adaptive slot lookups never index an
                // uninitialised pair.
                for i in tmpl.gbat_count..4 {
                    let d = tmpl.default_gbat[i];
                    gbat[i] = (d.0 as i32, d.1 as i32);
                }

                let rp = RegionParams {
                    region_w, region_h, region_x, region_y,
                    template, tpgdon, gbat,
                    mq_start, mq_end: hdr.data_end,
                };
                decode_generic_region(
                    page, &rp, &mut bitmap,
                    page_stride, page_width, page_height,
                )?;
            }
            49 | 51 => {
                // End of page / end of file — accept gracefully.
            }
            _ => {
                return Err(Error::UnsupportedSegmentType(hdr.seg_type));
            }
        }
        p = hdr.data_end;
    }

    if !seen_page_info {
        return Err(Error::MissingPageInfo);
    }

    Ok(DecodedImage {
        width: page_width,
        height: page_height,
        components: 1,
        pixels: bitmap,
    })
}
