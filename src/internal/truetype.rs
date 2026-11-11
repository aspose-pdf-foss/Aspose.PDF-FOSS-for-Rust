// TrueType (sfnt_version 0x00010000) parser. Pulls everything a
// PDF text+rendering pipeline needs out of a `.ttf` file:
// unitsPerEm + ascent/descent + per-glyph advance widths + BMP
// cmap (format 4) + per-glyph quadratic outlines (simple +
// composite-flattened) + the human font name (`name`, lenient).
//
// References:
//   * OpenType 1.9 / Apple TrueType Reference Manual — SFNT,
//     `head`, `hhea`, `maxp`, `hmtx`, `cmap` (format 4),
//     `loca`, `glyf` (simple + composite), `name`.
//   * PDF 32000-1:2008 §9.6.5 (TrueType fonts).
//
// Out of scope: OpenType CFF (`OTTO`), TrueType Collections
// (`ttcf`), cmap formats other than 4, kerning / GPOS, vertical
// metrics, variable-font tables, hinting (`prep`/`fpgm`/
// instructions are skipped in glyf bodies).

use std::fmt;

// =====================  Public surface  =====================

/// Parsed TrueType font — every field a PDF reader needs to
/// decode + rasterise text operators that reference the font.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrueType {
    pub units_per_em: u16,
    pub ascent: i16,
    pub descent: i16,
    pub num_glyphs: u16,
    /// One advance width per glyph; length == `num_glyphs`.
    pub widths: Vec<u16>,
    /// BMP cmap entries `(char_code, glyph_id)`, sorted by
    /// `char_code`. Entries with `glyph_id == 0` are dropped
    /// (PDF text decoders treat `.notdef` as "no mapping").
    pub cmap: Vec<(u32, u16)>,
    /// One glyph per gid; length == `num_glyphs`. Composite
    /// glyphs are flattened — only simple-glyph point streams
    /// reach the consumer.
    pub glyphs: Vec<Glyph>,
    /// The font's name from the `name` table — the PostScript
    /// name (nameID 6), falling back to the full font name
    /// (nameID 4) then the family name (nameID 1). Empty when no
    /// `name` table or no usable record is present. Read leniently
    /// (a malformed `name` table never fails the parse); not part
    /// of the `to_canonical` oracle projection.
    pub name: String,
}

/// A single glyph's flattened outline. Empty when the glyph has
/// no contours (`.notdef` placeholder space, blank glyphs, etc).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glyph {
    pub points: Vec<Point>,
    /// Inclusive last-point index for each contour, in
    /// declaration order.
    pub contour_ends: Vec<u32>,
}

/// One absolute-FUnit point of the glyph outline. `on_curve`
/// distinguishes anchors from quadratic-Bézier control points;
/// consumers that need cubics lift quadratic→cubic at
/// consumption time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
    pub on_curve: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Buffer was too short to read the named structure.
    Truncated { what: &'static str },
    /// `sfnt_version` was not `0x00010000` (e.g. `OTTO`/`ttcf`).
    WrongSfntVersion { sfnt_version: u32 },
    /// One of the required tables was not in the directory.
    MissingTable { tag: [u8; 4] },
    /// `numberOfHMetrics > numGlyphs` (corrupt `hmtx` shape).
    BadHmtx { num_glyphs: u32, num_h_metrics: u32 },
    /// Neither (3,1) nor (0,3) format-4 cmap subtable present.
    NoBmpCmap,
    /// Format-4 subtable is malformed (length / segCount /
    /// reservedPad).
    BadCmapFormat4 { reason: &'static str },
    /// `head.indexToLocFormat` was outside `{0, 1}`.
    BadIndexToLocFormat { value: i16 },
    /// `loca` offset out of range or descending.
    BadLoca { reason: &'static str },
    /// `glyf` slot truncated relative to its own header.
    BadGlyf { reason: &'static str },
    /// Composite recursion exceeded the spec-mandated 10-level
    /// limit (cycle or pathologically deep nesting).
    CompositeRecursionTooDeep,
    /// Composite component referenced a glyph index outside
    /// `0..num_glyphs`.
    CompositeBadSubGlyph { glyph_index: u32, num_glyphs: u32 },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated { what } =>
                write!(f, "truetype: truncated input while reading {}",
                       what),
            Error::WrongSfntVersion { sfnt_version } =>
                write!(f, "truetype: sfnt_version 0x{:08x} \
                          unsupported (only 0x00010000)",
                       sfnt_version),
            Error::MissingTable { tag } =>
                write!(f, "truetype: required table `{}` missing",
                       fmt_tag(*tag)),
            Error::BadHmtx { num_glyphs, num_h_metrics } =>
                write!(f, "truetype: hmtx numberOfHMetrics={} \
                          exceeds numGlyphs={}",
                       num_h_metrics, num_glyphs),
            Error::NoBmpCmap =>
                write!(f, "truetype: no Unicode BMP cmap (3,1) \
                          / (0,3) format-4 subtable"),
            Error::BadCmapFormat4 { reason } =>
                write!(f, "truetype: malformed cmap format-4 ({})",
                       reason),
            Error::BadIndexToLocFormat { value } =>
                write!(f, "truetype: head.indexToLocFormat={} \
                          must be 0 or 1", value),
            Error::BadLoca { reason } =>
                write!(f, "truetype: bad loca ({})", reason),
            Error::BadGlyf { reason } =>
                write!(f, "truetype: bad glyf ({})", reason),
            Error::CompositeRecursionTooDeep =>
                write!(f, "truetype: composite glyph recursion \
                          depth exceeded 10"),
            Error::CompositeBadSubGlyph { glyph_index, num_glyphs } =>
                write!(f, "truetype: composite sub-glyph index {} \
                          out of range (numGlyphs={})",
                       glyph_index, num_glyphs),
        }
    }
}

impl std::error::Error for Error {}

/// Parse a TrueType font from raw bytes.
pub fn parse(input: &[u8]) -> Result<TrueType, Error> {
    // ---- Offset table (12 bytes) ---------------------------
    let off = Reader::new(input);
    let sfnt_version = off.u32_at(0, "offset table")?;
    if sfnt_version != 0x0001_0000 {
        return Err(Error::WrongSfntVersion { sfnt_version });
    }
    let num_tables = off.u16_at(4, "offset table")? as usize;

    // ---- Table directory (16 bytes per entry) --------------
    let dir_start: usize = 12;
    let dir_end = dir_start
        .checked_add(num_tables.checked_mul(16)
            .ok_or(Error::Truncated { what: "table directory" })?)
        .ok_or(Error::Truncated { what: "table directory" })?;
    if dir_end > input.len() {
        return Err(Error::Truncated { what: "table directory" });
    }

    let mut dir: Vec<([u8; 4], usize, usize)> =
        Vec::with_capacity(num_tables);
    for i in 0..num_tables {
        let base = dir_start + i * 16;
        let mut tag = [0u8; 4];
        tag.copy_from_slice(&input[base..base + 4]);
        let offset = u32_be(&input[base + 8..base + 12]) as usize;
        let length = u32_be(&input[base + 12..base + 16]) as usize;
        let end = offset.checked_add(length)
            .ok_or(Error::Truncated { what: "table directory" })?;
        if end > input.len() {
            return Err(Error::Truncated { what: "table directory" });
        }
        dir.push((tag, offset, length));
    }

    let table = |t: &[u8; 4]| -> Result<&[u8], Error> {
        for (tag, offset, length) in &dir {
            if tag == t {
                return Ok(&input[*offset..*offset + *length]);
            }
        }
        Err(Error::MissingTable { tag: *t })
    };

    // ---- head ----------------------------------------------
    let head = table(b"head")?;
    if head.len() < 54 {
        return Err(Error::Truncated { what: "head" });
    }
    let units_per_em = u16_be(&head[18..20]);
    let index_to_loc_format = i16_be(&head[50..52]);
    if index_to_loc_format != 0 && index_to_loc_format != 1 {
        return Err(Error::BadIndexToLocFormat {
            value: index_to_loc_format,
        });
    }

    // ---- hhea ----------------------------------------------
    let hhea = table(b"hhea")?;
    if hhea.len() < 36 {
        return Err(Error::Truncated { what: "hhea" });
    }
    let ascent = i16_be(&hhea[4..6]);
    let descent = i16_be(&hhea[6..8]);
    let num_h_metrics = u16_be(&hhea[34..36]) as u32;

    // ---- maxp (v0.5 + v1.0 carry numGlyphs at offset 4) ----
    let maxp = table(b"maxp")?;
    if maxp.len() < 6 {
        return Err(Error::Truncated { what: "maxp" });
    }
    let num_glyphs = u16_be(&maxp[4..6]);
    if num_h_metrics > num_glyphs as u32 {
        return Err(Error::BadHmtx {
            num_glyphs: num_glyphs as u32,
            num_h_metrics,
        });
    }

    // ---- hmtx ----------------------------------------------
    let widths =
        parse_hmtx(table(b"hmtx")?, num_glyphs, num_h_metrics)?;

    // ---- cmap (format-4 BMP) -------------------------------
    let cmap = parse_cmap(table(b"cmap")?)?;

    // ---- loca + glyf ---------------------------------------
    let loca_bytes = table(b"loca")?;
    let glyf = table(b"glyf")?;
    let loca = parse_loca(
        loca_bytes, num_glyphs, index_to_loc_format, glyf.len())?;

    let mut glyphs: Vec<Glyph> = Vec::with_capacity(num_glyphs as usize);
    for gid in 0..num_glyphs {
        glyphs.push(parse_glyph(glyf, &loca, num_glyphs, gid, 0)?);
    }

    // ---- name (optional, lenient) --------------------------
    let name = table(b"name").ok().map(parse_name).unwrap_or_default();

    Ok(TrueType {
        units_per_em,
        ascent,
        descent,
        num_glyphs,
        widths,
        cmap,
        glyphs,
        name,
    })
}

/// Extract the best human font name from a `name` table slice.
///
/// Prefers the PostScript name (nameID 6), then the full font
/// name (nameID 4), then the family name (nameID 1). Windows /
/// Unicode records (platformID 3 or 0) decode as UTF-16BE; Mac
/// records (platformID 1) decode as Latin-1. Read leniently:
/// any truncation or unknown encoding yields an empty string
/// rather than an error — the `name` table is informational.
fn parse_name(tbl: &[u8]) -> String {
    if tbl.len() < 6 {
        return String::new();
    }
    let count = u16_be(&tbl[2..4]) as usize;
    let storage = u16_be(&tbl[4..6]) as usize;
    let records_end = 6 + count * 12;
    if records_end > tbl.len() {
        return String::new();
    }

    // Lower nameID rank = more preferred (6 → 4 → 1).
    let rank = |name_id: u16| -> Option<u8> {
        match name_id {
            6 => Some(0),
            4 => Some(1),
            1 => Some(2),
            _ => None,
        }
    };

    let mut best: Option<(u8, String)> = None;
    for i in 0..count {
        let rec = 6 + i * 12;
        let platform_id = u16_be(&tbl[rec..rec + 2]);
        let name_id = u16_be(&tbl[rec + 6..rec + 8]);
        let length = u16_be(&tbl[rec + 8..rec + 10]) as usize;
        let offset = u16_be(&tbl[rec + 10..rec + 12]) as usize;

        let Some(r) = rank(name_id) else { continue };
        if best.as_ref().is_some_and(|(br, _)| *br <= r) {
            continue;
        }

        let start = storage + offset;
        let Some(bytes) = tbl.get(start..start + length) else {
            continue;
        };
        let decoded = match platform_id {
            1 => bytes.iter().map(|&b| b as char).collect::<String>(),
            _ => decode_utf16_be(bytes),
        };
        if !decoded.is_empty() {
            best = Some((r, decoded));
        }
    }
    best.map(|(_, s)| s).unwrap_or_default()
}

/// Decode a big-endian UTF-16 byte slice, replacing unpaired
/// surrogates / odd trailing bytes with the replacement char.
fn decode_utf16_be(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(u16_be)
        .collect();
    char::decode_utf16(units)
        .map(|r| r.unwrap_or('\u{FFFD}'))
        .collect()
}

/// Serialise a parsed font as the canonical text manifest the
/// two_oracle_agreement gate compares byte-for-byte against the
/// `.canonical` sidecar (see anchor `truetype.yaml` →
/// `ToCanonical`).
#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn to_canonical(font: &TrueType) -> String {
    use std::fmt::Write;

    // Two preliminary lower bounds: header (4 lines) + width
    // (num_glyphs lines) + cmap (variable) + glyph block
    // (variable). Most glyphs have ~10–30 points; allocate
    // generously to skip rehash.
    let mut out = String::with_capacity(
        128 + font.num_glyphs as usize * 32);

    let _ = writeln!(out, "unitsPerEm {}", font.units_per_em);
    let _ = writeln!(out, "ascent {}", font.ascent);
    let _ = writeln!(out, "descent {}", font.descent);
    let _ = writeln!(out, "numGlyphs {}", font.num_glyphs);

    for (gid, w) in font.widths.iter().enumerate() {
        let _ = writeln!(out, "width {} {}", gid, w);
    }

    for (code, gid) in &font.cmap {
        let _ = writeln!(out, "cmap {:04X} {}", code, gid);
    }

    for (gid, g) in font.glyphs.iter().enumerate() {
        if g.points.is_empty() {
            let _ = writeln!(out, "glyph {} empty", gid);
            continue;
        }
        let _ = writeln!(out, "glyph {} simple {} {}",
                         gid, g.contour_ends.len(), g.points.len());
        for p in &g.points {
            let tag = if p.on_curve { "on" } else { "off" };
            let _ = writeln!(out, "  {} {} {}", tag, p.x, p.y);
        }
        for e in &g.contour_ends {
            let _ = writeln!(out, "  contour_end {}", e);
        }
    }

    out
}

// =====================  hmtx  ===============================

fn parse_hmtx(
    bytes: &[u8],
    num_glyphs: u16,
    num_h_metrics: u32,
) -> Result<Vec<u16>, Error> {
    let n = num_glyphs as usize;
    let h = num_h_metrics as usize;
    // Trailing array of int16 lsb values for the gids past the
    // last full record. The advanceWidth array we expose just
    // repeats the LAST explicit width for those gids.
    let need = h * 4 + (n.saturating_sub(h)) * 2;
    if bytes.len() < need {
        return Err(Error::Truncated { what: "hmtx" });
    }
    let mut widths = Vec::with_capacity(n);
    let mut last_width = 0u16;
    for i in 0..h {
        let w = u16_be(&bytes[i * 4..i * 4 + 2]);
        last_width = w;
        widths.push(w);
    }
    for _ in h..n {
        widths.push(last_width);
    }
    Ok(widths)
}

// =====================  cmap (format 4)  ====================

fn parse_cmap(bytes: &[u8]) -> Result<Vec<(u32, u16)>, Error> {
    if bytes.len() < 4 {
        return Err(Error::Truncated { what: "cmap header" });
    }
    let num_subtables = u16_be(&bytes[2..4]) as usize;
    let recs_end = 4 + num_subtables * 8;
    if bytes.len() < recs_end {
        return Err(Error::Truncated { what: "cmap subtable records" });
    }

    // Two passes — prefer (3,1), fall back to (0,3).
    let pick = |want_p: u16, want_e: u16| -> Option<usize> {
        for i in 0..num_subtables {
            let base = 4 + i * 8;
            let p = u16_be(&bytes[base..base + 2]);
            let e = u16_be(&bytes[base + 2..base + 4]);
            if p == want_p && e == want_e {
                return Some(u32_be(&bytes[base + 4..base + 8])
                            as usize);
            }
        }
        None
    };
    let sub_off =
        pick(3, 1).or_else(|| pick(0, 3))
                  .ok_or(Error::NoBmpCmap)?;

    if sub_off + 14 > bytes.len() {
        return Err(Error::Truncated { what: "cmap subtable" });
    }
    let format = u16_be(&bytes[sub_off..sub_off + 2]);
    if format != 4 {
        return Err(Error::NoBmpCmap);
    }
    let length = u16_be(&bytes[sub_off + 2..sub_off + 4]) as usize;
    if sub_off + length > bytes.len() {
        return Err(Error::Truncated { what: "cmap format-4 length" });
    }
    let sub = &bytes[sub_off..sub_off + length];
    let seg_count_x2 = u16_be(&sub[6..8]) as usize;
    if seg_count_x2 % 2 != 0 || seg_count_x2 == 0 {
        return Err(Error::BadCmapFormat4 {
            reason: "segCountX2",
        });
    }
    let seg_count = seg_count_x2 / 2;

    let end_codes_off = 14_usize;
    let reserved_pad_off = end_codes_off + seg_count * 2;
    let start_codes_off = reserved_pad_off + 2;
    let id_delta_off = start_codes_off + seg_count * 2;
    let id_range_off = id_delta_off + seg_count * 2;
    let glyph_id_off = id_range_off + seg_count * 2;
    if glyph_id_off > sub.len() {
        return Err(Error::BadCmapFormat4 {
            reason: "subtable too short for segments",
        });
    }
    if u16_be(&sub[reserved_pad_off..reserved_pad_off + 2]) != 0 {
        return Err(Error::BadCmapFormat4 {
            reason: "reservedPad != 0",
        });
    }

    let glyph_id_words = (sub.len() - glyph_id_off) / 2;

    let mut entries: Vec<(u32, u16)> = Vec::new();
    for i in 0..seg_count {
        let end_code =
            u16_be(&sub[end_codes_off + i * 2..end_codes_off + i * 2 + 2]);
        let start_code =
            u16_be(&sub[start_codes_off + i * 2..start_codes_off + i * 2 + 2]);
        let id_delta =
            i16_be(&sub[id_delta_off + i * 2..id_delta_off + i * 2 + 2]);
        let id_range_offset =
            u16_be(&sub[id_range_off + i * 2..id_range_off + i * 2 + 2]);

        if end_code < start_code {
            return Err(Error::BadCmapFormat4 {
                reason: "segment endCode < startCode",
            });
        }

        for c in start_code..=end_code {
            let gid = if id_range_offset == 0 {
                ((c as i32 + id_delta as i32) & 0xFFFF) as u16
            } else {
                let idx = (id_range_offset as usize / 2)
                    .wrapping_add((c - start_code) as usize)
                    .wrapping_sub(seg_count - i);
                if idx >= glyph_id_words {
                    return Err(Error::BadCmapFormat4 {
                        reason: "idRangeOffset points past glyphIdArray",
                    });
                }
                let raw = u16_be(
                    &sub[glyph_id_off + idx * 2..glyph_id_off + idx * 2 + 2]);
                if raw == 0 {
                    0
                } else {
                    ((raw as i32 + id_delta as i32) & 0xFFFF) as u16
                }
            };
            if gid != 0 {
                entries.push((c as u32, gid));
            }
            // The terminator segment (start=end=0xFFFF) decodes
            // to gid 0 here and is dropped — matches the oracle
            // dict-keyed-by-non-notdef-name behaviour.
            if c == 0xFFFF { break; }
        }
    }
    Ok(entries)
}

// =====================  loca  ===============================

fn parse_loca(
    bytes: &[u8],
    num_glyphs: u16,
    index_to_loc_format: i16,
    glyf_len: usize,
) -> Result<Vec<u32>, Error> {
    let n = num_glyphs as usize + 1;
    let entries: Vec<u32> = if index_to_loc_format == 0 {
        if bytes.len() < n * 2 {
            return Err(Error::Truncated { what: "loca (short)" });
        }
        (0..n).map(|i| u16_be(&bytes[i * 2..i * 2 + 2]) as u32 * 2)
              .collect()
    } else {
        if bytes.len() < n * 4 {
            return Err(Error::Truncated { what: "loca (long)" });
        }
        (0..n).map(|i| u32_be(&bytes[i * 4..i * 4 + 4]))
              .collect()
    };
    for w in entries.windows(2) {
        if w[1] < w[0] {
            return Err(Error::BadLoca { reason: "descending offsets" });
        }
        if w[1] as usize > glyf_len {
            return Err(Error::BadLoca { reason: "offset past glyf end" });
        }
    }
    Ok(entries)
}

// =====================  glyf (recursive)  ===================

const F_ON_CURVE: u8 = 0x01;
const F_X_SHORT: u8 = 0x02;
const F_Y_SHORT: u8 = 0x04;
const F_REPEAT: u8 = 0x08;
const F_X_SAME_OR_POS: u8 = 0x10;
const F_Y_SAME_OR_POS: u8 = 0x20;

const C_ARG_1_AND_2_ARE_WORDS: u16 = 0x0001;
const C_ARGS_ARE_XY_VALUES: u16 = 0x0002;
const C_WE_HAVE_A_SCALE: u16 = 0x0008;
const C_MORE_COMPONENTS: u16 = 0x0020;
const C_WE_HAVE_AN_X_AND_Y_SCALE: u16 = 0x0040;
const C_WE_HAVE_A_TWO_BY_TWO: u16 = 0x0080;

const MAX_COMPOSITE_DEPTH: u32 = 10;

fn parse_glyph(
    glyf: &[u8],
    loca: &[u32],
    num_glyphs: u16,
    gid: u16,
    depth: u32,
) -> Result<Glyph, Error> {
    if depth > MAX_COMPOSITE_DEPTH {
        return Err(Error::CompositeRecursionTooDeep);
    }
    let begin = loca[gid as usize] as usize;
    let end = loca[gid as usize + 1] as usize;
    if begin == end {
        return Ok(Glyph {
            points: Vec::new(),
            contour_ends: Vec::new(),
        });
    }
    if end > glyf.len() || begin > end {
        return Err(Error::BadGlyf { reason: "slot out of range" });
    }
    let body = &glyf[begin..end];
    if body.len() < 10 {
        return Err(Error::BadGlyf { reason: "header truncated" });
    }
    let num_contours = i16_be(&body[0..2]);
    // body[2..10] is xMin/yMin/xMax/yMax — informational; skip.
    let after_header = 10_usize;

    if num_contours >= 0 {
        parse_simple_glyph(body, after_header, num_contours as usize)
    } else if num_contours == -1 {
        parse_composite_glyph(
            body, after_header, glyf, loca, num_glyphs, depth)
    } else {
        Err(Error::BadGlyf { reason: "numberOfContours < -1" })
    }
}

fn parse_simple_glyph(
    body: &[u8],
    after_header: usize,
    num_contours: usize,
) -> Result<Glyph, Error> {
    // endPtsOfContours[numContours]
    let need_endpts = num_contours * 2;
    if body.len() < after_header + need_endpts + 2 {
        return Err(Error::BadGlyf {
            reason: "endPtsOfContours/instructionLength truncated",
        });
    }
    let mut contour_ends: Vec<u32> = Vec::with_capacity(num_contours);
    for i in 0..num_contours {
        let off = after_header + i * 2;
        contour_ends.push(u16_be(&body[off..off + 2]) as u32);
    }
    let num_points = if num_contours == 0 {
        0_usize
    } else {
        contour_ends[num_contours - 1] as usize + 1
    };

    // instructionLength + skip
    let inst_off = after_header + need_endpts;
    let instruction_length = u16_be(&body[inst_off..inst_off + 2]) as usize;
    let flags_off = inst_off + 2 + instruction_length;
    if flags_off > body.len() {
        return Err(Error::BadGlyf { reason: "instruction overflow" });
    }

    // Flag stream — variable length with REPEAT byte.
    let mut flags: Vec<u8> = Vec::with_capacity(num_points);
    let mut p = flags_off;
    while flags.len() < num_points {
        if p >= body.len() {
            return Err(Error::BadGlyf { reason: "flag stream truncated" });
        }
        let f = body[p];
        p += 1;
        flags.push(f);
        if f & F_REPEAT != 0 {
            if p >= body.len() {
                return Err(Error::BadGlyf {
                    reason: "REPEAT count truncated",
                });
            }
            let count = body[p] as usize;
            p += 1;
            for _ in 0..count {
                if flags.len() >= num_points {
                    return Err(Error::BadGlyf {
                        reason: "REPEAT overflows numPoints",
                    });
                }
                flags.push(f);
            }
        }
    }

    // X coordinates: int8/int16 deltas per flag bits.
    let (xs, p) = decode_coord_stream(
        body, p, &flags, F_X_SHORT, F_X_SAME_OR_POS)?;
    let (ys, _p) = decode_coord_stream(
        body, p, &flags, F_Y_SHORT, F_Y_SAME_OR_POS)?;

    let mut points: Vec<Point> = Vec::with_capacity(num_points);
    for i in 0..num_points {
        points.push(Point {
            x: xs[i],
            y: ys[i],
            on_curve: flags[i] & F_ON_CURVE != 0,
        });
    }
    Ok(Glyph { points, contour_ends })
}

/// Decode one coordinate axis (X or Y) from the flag stream
/// into absolute font units. Returns `(values, new_pos)`.
fn decode_coord_stream(
    body: &[u8],
    mut p: usize,
    flags: &[u8],
    short_bit: u8,
    same_or_pos_bit: u8,
) -> Result<(Vec<i32>, usize), Error> {
    let mut out: Vec<i32> = Vec::with_capacity(flags.len());
    let mut acc: i32 = 0;
    for f in flags {
        let delta: i32 = if f & short_bit != 0 {
            // 1 unsigned byte; sign from same_or_pos bit
            // (set ⇒ positive).
            if p >= body.len() {
                return Err(Error::BadGlyf {
                    reason: "coord stream truncated (short)",
                });
            }
            let v = body[p] as i32;
            p += 1;
            if f & same_or_pos_bit != 0 { v } else { -v }
        } else if f & same_or_pos_bit != 0 {
            // X_SAME / Y_SAME: repeat previous (delta 0).
            0
        } else {
            // 2-byte signed delta.
            if p + 1 >= body.len() {
                return Err(Error::BadGlyf {
                    reason: "coord stream truncated (long)",
                });
            }
            let v = i16_be(&body[p..p + 2]) as i32;
            p += 2;
            v
        };
        acc += delta;
        out.push(acc);
    }
    Ok((out, p))
}

fn parse_composite_glyph(
    body: &[u8],
    mut p: usize,
    glyf: &[u8],
    loca: &[u32],
    num_glyphs: u16,
    depth: u32,
) -> Result<Glyph, Error> {
    let mut points: Vec<Point> = Vec::new();
    let mut contour_ends: Vec<u32> = Vec::new();

    loop {
        if p + 4 > body.len() {
            return Err(Error::BadGlyf {
                reason: "composite header truncated",
            });
        }
        let flags = u16_be(&body[p..p + 2]);
        let glyph_index = u16_be(&body[p + 2..p + 4]);
        p += 4;
        if glyph_index as u32 >= num_glyphs as u32 {
            return Err(Error::CompositeBadSubGlyph {
                glyph_index: glyph_index as u32,
                num_glyphs: num_glyphs as u32,
            });
        }

        // Args: dx/dy (or anchor indices, treated as 0/0).
        let words = flags & C_ARG_1_AND_2_ARE_WORDS != 0;
        let xy = flags & C_ARGS_ARE_XY_VALUES != 0;
        let (dx, dy) = if words {
            if p + 4 > body.len() {
                return Err(Error::BadGlyf {
                    reason: "composite args (words) truncated",
                });
            }
            let a = i16_be(&body[p..p + 2]);
            let b = i16_be(&body[p + 2..p + 4]);
            p += 4;
            if xy { (a as f64, b as f64) } else { (0.0, 0.0) }
        } else {
            if p + 2 > body.len() {
                return Err(Error::BadGlyf {
                    reason: "composite args (bytes) truncated",
                });
            }
            let a = body[p] as i8 as f64;
            let b = body[p + 1] as i8 as f64;
            p += 2;
            if xy { (a, b) } else { (0.0, 0.0) }
        };

        // Transform — see Apple TrueType ref. fontTools order:
        //   xscale, scale01, scale10, yscale →
        //   newX = xscale*x + scale01*y + dx
        //   newY = scale10*x + yscale*y + dy
        let (a, b, c, d) =
            if flags & C_WE_HAVE_A_TWO_BY_TWO != 0 {
                if p + 8 > body.len() {
                    return Err(Error::BadGlyf {
                        reason: "composite 2x2 transform truncated",
                    });
                }
                let a = f2dot14(&body[p..p + 2]);
                let b = f2dot14(&body[p + 2..p + 4]);
                let c = f2dot14(&body[p + 4..p + 6]);
                let d = f2dot14(&body[p + 6..p + 8]);
                p += 8;
                (a, b, c, d)
            } else if flags & C_WE_HAVE_AN_X_AND_Y_SCALE != 0 {
                if p + 4 > body.len() {
                    return Err(Error::BadGlyf {
                        reason: "composite x/y scale truncated",
                    });
                }
                let sx = f2dot14(&body[p..p + 2]);
                let sy = f2dot14(&body[p + 2..p + 4]);
                p += 4;
                (sx, 0.0, 0.0, sy)
            } else if flags & C_WE_HAVE_A_SCALE != 0 {
                if p + 2 > body.len() {
                    return Err(Error::BadGlyf {
                        reason: "composite scale truncated",
                    });
                }
                let s = f2dot14(&body[p..p + 2]);
                p += 2;
                (s, 0.0, 0.0, s)
            } else {
                (1.0, 0.0, 0.0, 1.0)
            };

        // Recurse + flatten.
        let sub = parse_glyph(
            glyf, loca, num_glyphs, glyph_index, depth + 1)?;
        let base = points.len() as u32;
        for pt in &sub.points {
            let xf = pt.x as f64;
            let yf = pt.y as f64;
            let nx = round_half_even(a * xf + b * yf + dx);
            let ny = round_half_even(c * xf + d * yf + dy);
            points.push(Point {
                x: nx as i32,
                y: ny as i32,
                on_curve: pt.on_curve,
            });
        }
        for e in &sub.contour_ends {
            contour_ends.push(e + base);
        }

        if flags & C_MORE_COMPONENTS == 0 {
            break;
        }
    }
    Ok(Glyph { points, contour_ends })
}

// =====================  Helpers  ============================

fn u16_be(b: &[u8]) -> u16 {
    ((b[0] as u16) << 8) | (b[1] as u16)
}
fn i16_be(b: &[u8]) -> i16 {
    u16_be(b) as i16
}
fn u32_be(b: &[u8]) -> u32 {
    ((b[0] as u32) << 24)
        | ((b[1] as u32) << 16)
        | ((b[2] as u32) << 8)
        | (b[3] as u32)
}

fn f2dot14(b: &[u8]) -> f64 {
    i16_be(b) as f64 / 16384.0
}

/// Round half-to-even (banker's rounding) — matches Python 3
/// `round()`, which the fontTools oracle uses for composite
/// transforms. Rust's `f64::round()` rounds half-away-from-zero,
/// which would diverge on `.5` ties.
fn round_half_even(x: f64) -> i64 {
    if !x.is_finite() {
        return 0;
    }
    let lo = x.floor();
    let hi = x.ceil();
    if lo == hi {
        return lo as i64;
    }
    let dl = x - lo;
    let dh = hi - x;
    if dl < dh {
        lo as i64
    } else if dh < dl {
        hi as i64
    } else {
        // tie → pick the even neighbour
        let li = lo as i64;
        if li % 2 == 0 { li } else { hi as i64 }
    }
}

fn fmt_tag(tag: [u8; 4]) -> String {
    tag.iter().map(|&b|
        if (0x20..=0x7e).contains(&b) {
            b as char
        } else {
            '?'
        }).collect()
}

struct Reader<'a> {
    buf: &'a [u8],
}

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self { Self { buf } }
    fn u16_at(&self, off: usize, what: &'static str) -> Result<u16, Error> {
        if off + 2 > self.buf.len() {
            return Err(Error::Truncated { what });
        }
        Ok(u16_be(&self.buf[off..off + 2]))
    }
    fn u32_at(&self, off: usize, what: &'static str) -> Result<u32, Error> {
        if off + 4 > self.buf.len() {
            return Err(Error::Truncated { what });
        }
        Ok(u32_be(&self.buf[off..off + 4]))
    }
}

// =====================  Tests  ==============================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_otto_sfnt_version() {
        // OTTO header (OpenType CFF) — out of v1 scope.
        let mut buf = vec![0x4F, 0x54, 0x54, 0x4F]; // "OTTO"
        buf.extend_from_slice(&[0; 8]);
        let err = parse(&buf).unwrap_err();
        assert!(matches!(err, Error::WrongSfntVersion { .. }),
                "got {:?}", err);
    }

    #[test]
    fn rejects_truncated_offset_table() {
        let buf = vec![0x00, 0x01, 0x00, 0x00]; // sfnt only
        let err = parse(&buf).unwrap_err();
        assert!(matches!(err, Error::Truncated { .. }),
                "got {:?}", err);
    }

    #[test]
    fn round_half_even_matches_python() {
        assert_eq!(round_half_even(0.5), 0);
        assert_eq!(round_half_even(1.5), 2);
        assert_eq!(round_half_even(2.5), 2);
        assert_eq!(round_half_even(3.5), 4);
        assert_eq!(round_half_even(-0.5), 0);
        assert_eq!(round_half_even(-1.5), -2);
        assert_eq!(round_half_even(-2.5), -2);
        assert_eq!(round_half_even(0.4), 0);
        assert_eq!(round_half_even(0.6), 1);
        assert_eq!(round_half_even(-0.4), 0);
        assert_eq!(round_half_even(-0.6), -1);
        assert_eq!(round_half_even(2.0), 2);
    }

    #[test]
    fn f2dot14_basic() {
        assert!((f2dot14(&[0x40, 0x00]) - 1.0).abs() < 1e-9);
        assert!((f2dot14(&[0xC0, 0x00]) - (-1.0)).abs() < 1e-9);
        assert!((f2dot14(&[0x00, 0x00])).abs() < 1e-9);
        // 0x7FFF = 1.99993896484375
        assert!((f2dot14(&[0x7F, 0xFF]) - 1.999_938_964_843_75).abs() < 1e-9);
    }

    #[test]
    fn u16_u32_be_endianness() {
        assert_eq!(u16_be(&[0x12, 0x34]), 0x1234);
        assert_eq!(i16_be(&[0xFF, 0xFE]), -2);
        assert_eq!(u32_be(&[0x01, 0x02, 0x03, 0x04]), 0x01020304);
    }

    #[test]
    fn cmap_format4_minimal_rejects_non_format_4() {
        // Header (numSubtables=1) + one (3,1) subtable header at
        // offset 12, but the subtable starts with format=0.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[0, 0, 0, 1]); // version + numSubtables
        bytes.extend_from_slice(&[0, 3]); // platformID
        bytes.extend_from_slice(&[0, 1]); // platEncID
        bytes.extend_from_slice(&[0, 0, 0, 12]); // subtableOffset = 12
        bytes.extend_from_slice(&[0, 0]); // format=0
        bytes.extend_from_slice(&[0, 14]); // length=14
        bytes.extend_from_slice(&[0; 10]); // padding
        let err = parse_cmap(&bytes).unwrap_err();
        assert!(matches!(err, Error::NoBmpCmap), "got {:?}", err);
    }

    #[test]
    fn loca_short_format_doubles_offsets() {
        // numGlyphs=2 ⇒ loca has 3 entries (uint16 each).
        // Offsets/2 stored: 0, 4, 8 ⇒ resolve to 0, 8, 16.
        let bytes: Vec<u8> = vec![0, 0, 0, 4, 0, 8];
        let glyf_len = 16;
        let loca = parse_loca(&bytes, 2, 0, glyf_len).unwrap();
        assert_eq!(loca, vec![0, 8, 16]);
    }

    #[test]
    fn loca_long_format_passes_through() {
        // numGlyphs=1, indexToLocFormat=1 ⇒ two uint32 entries.
        let bytes: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0x10];
        let loca = parse_loca(&bytes, 1, 1, 16).unwrap();
        assert_eq!(loca, vec![0, 0x10]);
    }

    #[test]
    fn loca_descending_rejected() {
        let bytes: Vec<u8> = vec![0, 4, 0, 0]; // 8, 0
        let err = parse_loca(&bytes, 1, 0, 8).unwrap_err();
        assert!(matches!(err, Error::BadLoca { .. }), "got {:?}", err);
    }

    #[test]
    fn hmtx_repeats_last_width_for_trailing_glyphs() {
        // num_h_metrics=1, num_glyphs=3 ⇒ widths [W, W, W].
        // record: width=500, lsb=0; trailing lsb int16 × 2.
        let bytes: Vec<u8> = vec![
            0x01, 0xF4, 0x00, 0x00, // (500, 0)
            0x00, 0x00,             // lsb for gid 1
            0x00, 0x00,             // lsb for gid 2
        ];
        let widths = parse_hmtx(&bytes, 3, 1).unwrap();
        assert_eq!(widths, vec![500, 500, 500]);
    }

    #[test]
    fn fmt_tag_renders_ascii() {
        assert_eq!(fmt_tag(*b"head"), "head");
        assert_eq!(fmt_tag([0xFF, b'b', b'c', b'd']), "?bcd");
    }

    #[test]
    fn extracts_name_from_real_font() {
        let bytes = include_bytes!(
            "standard14_outlines.fonts/LiberationSans-Regular.ttf");
        let font = parse(bytes).expect("Liberation Sans parses");
        // The PostScript / full name carries the family.
        assert!(font.name.contains("Liberation"),
                "unexpected font name: {:?}", font.name);
    }

    #[test]
    fn missing_name_table_yields_empty_name() {
        // Reuse the synthetic font from the offset-table tests: a
        // font without a `name` table parses with an empty name.
        // (A truncated name table likewise yields "".)
        assert_eq!(parse_name(&[]), "");
        assert_eq!(parse_name(&[0, 0, 0, 1]), ""); // header too short
    }

    #[test]
    fn parse_name_prefers_postscript_record() {
        // Two Windows UTF-16BE records: nameID 1 "Fam", nameID 6
        // "PS". The PostScript name (6) wins.
        let mut tbl = Vec::new();
        tbl.extend_from_slice(&[0, 0]); // format 0
        tbl.extend_from_slice(&[0, 2]); // count 2
        let storage_off: u16 = 6 + 2 * 12;
        tbl.extend_from_slice(&storage_off.to_be_bytes());
        let push_rec = |t: &mut Vec<u8>, name_id: u16, len: u16, off: u16| {
            t.extend_from_slice(&3u16.to_be_bytes()); // platform 3
            t.extend_from_slice(&1u16.to_be_bytes()); // encoding 1
            t.extend_from_slice(&0x409u16.to_be_bytes()); // lang en-US
            t.extend_from_slice(&name_id.to_be_bytes());
            t.extend_from_slice(&len.to_be_bytes());
            t.extend_from_slice(&off.to_be_bytes());
        };
        let fam: Vec<u8> = "Fam".encode_utf16().flat_map(|u| u.to_be_bytes()).collect();
        let ps: Vec<u8> = "PS".encode_utf16().flat_map(|u| u.to_be_bytes()).collect();
        push_rec(&mut tbl, 1, fam.len() as u16, 0);
        push_rec(&mut tbl, 6, ps.len() as u16, fam.len() as u16);
        tbl.extend_from_slice(&fam);
        tbl.extend_from_slice(&ps);
        assert_eq!(parse_name(&tbl), "PS");
    }
}
