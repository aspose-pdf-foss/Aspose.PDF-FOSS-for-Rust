// CFF (Compact Font Format) parser. Reads a raw CFF v1 byte
// stream — the contents of an OpenType /CFF table or a PDF
// /CIDFontType0C / Type1C font program — and exposes everything
// a PDF text + rendering pipeline needs:
//   * unitsPerEm (derived from Top DICT.FontMatrix[0]),
//   * postscript font name,
//   * per-glyph name (or `cid<N>` for CID-keyed fonts) + advance
//     width (resolved through Private DICT default/nominal),
//   * per-glyph cubic-Bezier outline produced by EXECUTING the
//     Type 2 CharString program — full interpreter scope, with
//     local + global subroutines, width handling, hint/cntr
//     masks, and the flex family.
//
// References:
//   * Adobe Technical Note #5176 — The Compact Font Format.
//   * Adobe Technical Note #5177 — The Type 2 Charstring Format.
//   * PDF 32000-1:2008 §9.6.2.2 + §9.7.4.2.
//
// Out of scope (per `cff.yaml`):
//   * CFF2 (variable fonts) — separate primitive.
//   * CFF Encoding (PDF supplies its own /Encoding).
//   * Type 1 CharStrings (CharstringType == 1).
//   * seac (12 6) compound-glyph accessor — throw.
//   * Random subroutine results — throw.

#![allow(clippy::needless_range_loop)] // index-arithmetic numeric kernels

use std::fmt;

use super::cff_data::STANDARD_STRINGS;

// =====================  Public surface  =====================

/// Parsed CFF font — the slice of CFF state a PDF reader needs
/// after Type 2 program execution. Outline coordinates live in
/// `glyphs[*].path`; advance widths in `glyphs[*].advance`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub struct Cff {
    /// `round(1.0 / FontMatrix[0])`. Default FontMatrix
    /// (`[0.001 0 0 0.001 0 0]`) gives 1000.
    pub units_per_em: u32,
    /// `true` iff Top DICT has a ROS operator (CID-keyed font).
    pub is_cid: bool,
    /// PostScript font name from Name INDEX entry 0 — raw bytes
    /// (CFF allows non-ASCII; the canonical form `%XX`-escapes
    /// bytes outside `0x21..0x7E`).
    pub font_name: Vec<u8>,
    /// One entry per glyph (indexed by gid).
    pub glyphs: Vec<CffGlyph>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub struct CffGlyph {
    /// `".notdef"` / `"space"` / … for non-CID fonts;
    /// `"cid<N>"` for CID-keyed fonts.
    pub name_or_cid: String,
    /// Advance width in font units, resolved through
    /// `defaultWidthX` / `nominalWidthX` (rounded to nearest
    /// integer).
    pub advance: i32,
    /// Path segments in execution order. Empty when the glyph's
    /// program emits nothing (whitespace / control glyphs).
    pub path: Vec<PathSeg>,
}

/// One path-segment of a CFF glyph outline. Coordinates are
/// integers in font units (Type 2 stores glyph paths as
/// integers; we snap interpreter results to nearest integer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub enum PathSeg {
    Move { x: i32, y: i32 },
    Line { x: i32, y: i32 },
    Curve { x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32 },
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub enum Error {
    /// Buffer too short to read the named structure.
    Truncated { what: &'static str },
    /// CFF major version was not 1.
    BadVersion { major: u8 },
    /// `CharstringType` was not 2 (only Type 2 supported).
    UnsupportedCharstringType { value: i32 },
    /// DICT operand stream malformed (operator without operands,
    /// or trailing operands at end-of-DICT).
    MalformedDict { reason: &'static str },
    /// Required Top DICT operator missing.
    MissingTopDictKey { key: &'static str },
    /// INDEX `offSize` was outside `1..=4`, or offsets did not
    /// start at 1, or were non-monotonic, or out of buffer.
    BadIndex { reason: &'static str },
    /// Unknown / unsupported charset format byte.
    BadCharsetFormat { format: u8 },
    /// Predefined charset (Top DICT.charset = 1 or 2) referenced
    /// by a fixture. ISOAdobe (charset 0) IS supported via the
    /// default-Standard-Strings prefix; Expert / ExpertSubset
    /// SID arrays are not embedded in v1.
    UnsupportedPredefinedCharset { which: i32 },
    /// FDSelect format byte was not 0 or 3.
    BadFdSelectFormat { format: u8 },
    /// Type 2 program malformed (unknown opcode, stack
    /// over/underflow, mismatched return, depth > 10, etc).
    BadCharString { reason: &'static str },
    /// Encountered the deprecated `seac` form of `endchar`
    /// (compound glyph accessor) — out of v1 scope.
    SeacNotSupported,
    /// `random` (12 23) executed — non-deterministic; out of
    /// scope per the freeze-gate framing.
    RandomNotSupported,
    /// Subroutine call: biased number resolved outside the
    /// INDEX bounds.
    SubrOutOfRange { biased: i32 },
    /// Subroutine recursion exceeded the spec-mandated depth
    /// of 10 (cycle or pathologically deep nesting).
    SubrDepthExceeded,
    /// DICT real-number BCD stream malformed (unknown nibble
    /// or unterminated).
    BadReal { reason: &'static str },
    /// Operator with insufficient operands at execution time.
    StackUnderflow,
    /// Operand stack exceeded the 96-element limit.
    StackOverflow,
    /// Transient-array index outside `0..32`.
    TransientOutOfRange,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated { what } =>
                write!(f, "cff: truncated input while reading {}", what),
            Error::BadVersion { major } =>
                write!(f, "cff: major version {} unsupported \
                          (only CFF v1 = major 1)", major),
            Error::UnsupportedCharstringType { value } =>
                write!(f, "cff: CharstringType={} unsupported \
                          (only Type 2 = 2)", value),
            Error::MalformedDict { reason } =>
                write!(f, "cff: malformed DICT ({})", reason),
            Error::MissingTopDictKey { key } =>
                write!(f, "cff: required Top DICT key `{}` missing", key),
            Error::BadIndex { reason } =>
                write!(f, "cff: malformed INDEX ({})", reason),
            Error::BadCharsetFormat { format } =>
                write!(f, "cff: charset format byte {} unsupported \
                          (expected 0/1/2)", format),
            Error::UnsupportedPredefinedCharset { which } =>
                write!(f, "cff: predefined charset {} not embedded \
                          in v1 (Expert/ExpertSubset)", which),
            Error::BadFdSelectFormat { format } =>
                write!(f, "cff: FDSelect format byte {} unsupported \
                          (expected 0/3)", format),
            Error::BadCharString { reason } =>
                write!(f, "cff: malformed CharString ({})", reason),
            Error::SeacNotSupported =>
                write!(f, "cff: seac (deprecated compound-glyph \
                          endchar) not supported in v1"),
            Error::RandomNotSupported =>
                write!(f, "cff: `random` (12 23) operator not \
                          supported in v1 (non-deterministic)"),
            Error::SubrOutOfRange { biased } =>
                write!(f, "cff: subroutine biased number {} out of \
                          range", biased),
            Error::SubrDepthExceeded =>
                write!(f, "cff: subroutine recursion exceeded depth 10"),
            Error::BadReal { reason } =>
                write!(f, "cff: bad DICT real ({})", reason),
            Error::StackUnderflow =>
                write!(f, "cff: operator with insufficient operands"),
            Error::StackOverflow =>
                write!(f, "cff: operand stack exceeded 96 elements"),
            Error::TransientOutOfRange =>
                write!(f, "cff: transient-array index out of range \
                          (0..32)"),
        }
    }
}

impl std::error::Error for Error {}

// =====================  DICT operator codes  ===============

// Top DICT (single-byte).
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const OP_VERSION:        u16 = 0;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const OP_NOTICE:         u16 = 1;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const OP_FULLNAME:       u16 = 2;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const OP_FAMILYNAME:     u16 = 3;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const OP_WEIGHT:         u16 = 4;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const OP_FONTBBOX:       u16 = 5;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const OP_CHARSET:        u16 = 15;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const OP_ENCODING:       u16 = 16;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const OP_CHARSTRINGS:    u16 = 17;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const OP_PRIVATE:        u16 = 18;

// Private DICT (single-byte).
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const OP_SUBRS:          u16 = 19;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const OP_DEFAULT_WIDTH:  u16 = 20;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const OP_NOMINAL_WIDTH:  u16 = 21;

// Top DICT (escape, byte 12 + b2).
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const OP_COPYRIGHT:      u16 = 12 << 8;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const OP_CHARSTRING_TYPE:u16 = (12 << 8) | 6;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const OP_FONTMATRIX:     u16 = (12 << 8) | 7;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const OP_ROS:            u16 = (12 << 8) | 30;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const OP_CIDCOUNT:       u16 = (12 << 8) | 34;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const OP_FDARRAY:        u16 = (12 << 8) | 36;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const OP_FDSELECT:       u16 = (12 << 8) | 37;

// =====================  Internal types  ====================

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
enum DictValue {
    Int(i32),
    Real(f64),
}

impl DictValue {
    #[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
    fn as_f64(&self) -> f64 {
        match *self {
            DictValue::Int(i) => i as f64,
            DictValue::Real(r) => r,
        }
    }
    #[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
    fn as_i32(&self) -> i32 {
        match *self {
            DictValue::Int(i) => i,
            DictValue::Real(r) => r as i32,
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
struct Dict {
    entries: Vec<(u16, Vec<DictValue>)>,
}

impl Dict {
    #[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
    fn get(&self, op: u16) -> Option<&[DictValue]> {
        for (k, v) in &self.entries {
            if *k == op {
                return Some(v);
            }
        }
        None
    }
    #[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
    fn get_int(&self, op: u16) -> Option<i32> {
        self.get(op).and_then(|v| v.first()).map(|v| v.as_i32())
    }
    #[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
    fn get_f64(&self, op: u16) -> Option<f64> {
        self.get(op).and_then(|v| v.first()).map(|v| v.as_f64())
    }
    #[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
    fn has(&self, op: u16) -> bool {
        self.get(op).is_some()
    }
}

#[derive(Debug)]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
struct FontDictInfo {
    default_width_x: f64,
    nominal_width_x: f64,
    local_subrs: Vec<Vec<u8>>,
    local_bias: i32,
}

// =====================  Parse entry point  =================

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub fn parse(input: &[u8]) -> Result<Cff, Error> {
    // ---- Header (4 bytes) -----------------------------------
    if input.len() < 4 {
        return Err(Error::Truncated { what: "header" });
    }
    let major = input[0];
    if major != 1 {
        return Err(Error::BadVersion { major });
    }
    let hdr_size = input[2] as usize;
    if hdr_size < 4 || hdr_size > input.len() {
        return Err(Error::Truncated { what: "header" });
    }

    // ---- Top-level INDEXes ----------------------------------
    let (name_idx, after_name) = parse_index(input, hdr_size)?;
    if name_idx.count != 1 {
        return Err(Error::BadIndex {
            reason: "Name INDEX must contain exactly one entry",
        });
    }
    let font_name = name_idx.entry(input, 0)?.to_vec();

    let (top_idx, after_top) = parse_index(input, after_name)?;
    if top_idx.count != 1 {
        return Err(Error::BadIndex {
            reason: "Top DICT INDEX must contain exactly one entry",
        });
    }
    let top_dict = parse_dict(top_idx.entry(input, 0)?)?;

    let (string_idx, after_string) = parse_index(input, after_top)?;

    let (gsubrs_idx, _after_gsubrs) =
        parse_index(input, after_string)?;
    let global_subrs: Vec<Vec<u8>> =
        (0..gsubrs_idx.count)
            .map(|i| gsubrs_idx.entry(input, i).map(|s| s.to_vec()))
            .collect::<Result<Vec<_>, _>>()?;
    let global_bias = bias_for_count(gsubrs_idx.count);

    // ---- CharstringType -------------------------------------
    let cs_type = top_dict.get_int(OP_CHARSTRING_TYPE).unwrap_or(2);
    if cs_type != 2 {
        return Err(Error::UnsupportedCharstringType { value: cs_type });
    }

    // ---- CharStrings INDEX ----------------------------------
    let cs_offset = top_dict.get_int(OP_CHARSTRINGS)
        .ok_or(Error::MissingTopDictKey { key: "CharStrings" })?
        as usize;
    let (cs_idx, _) = parse_index(input, cs_offset)?;
    let num_glyphs = cs_idx.count;
    if num_glyphs == 0 {
        return Err(Error::BadIndex {
            reason: "CharStrings INDEX is empty (CFF must have ≥1 glyph)",
        });
    }

    // ---- FontMatrix → unitsPerEm ----------------------------
    let fm_a = top_dict
        .get(OP_FONTMATRIX)
        .and_then(|v| v.first())
        .map(|v| v.as_f64())
        .unwrap_or(0.001);
    let units_per_em = if fm_a > 0.0 {
        (1.0 / fm_a).round() as u32
    } else {
        1000
    };

    // ---- CID vs non-CID -------------------------------------
    let is_cid = top_dict.has(OP_ROS);

    // ---- FontDicts (Private DICT + Local Subrs) -------------
    let font_dicts: Vec<FontDictInfo> = if is_cid {
        let fd_array_off = top_dict.get_int(OP_FDARRAY)
            .ok_or(Error::MissingTopDictKey { key: "FDArray" })?
            as usize;
        let (fd_idx, _) = parse_index(input, fd_array_off)?;
        let mut dicts = Vec::with_capacity(fd_idx.count);
        for i in 0..fd_idx.count {
            let fd_dict = parse_dict(fd_idx.entry(input, i)?)?;
            dicts.push(load_font_dict(input, &fd_dict)?);
        }
        dicts
    } else {
        vec![load_font_dict(input, &top_dict)?]
    };

    // ---- FDSelect -------------------------------------------
    let fd_select: Vec<u8> = if is_cid {
        let fds_off = top_dict.get_int(OP_FDSELECT)
            .ok_or(Error::MissingTopDictKey { key: "FDSelect" })?
            as usize;
        parse_fd_select(input, fds_off, num_glyphs)?
    } else {
        vec![0u8; num_glyphs]
    };

    // ---- Charset (per-glyph SID/CID array) ------------------
    let charset_off = top_dict.get_int(OP_CHARSET).unwrap_or(0);
    let glyph_sids =
        parse_charset(input, charset_off, num_glyphs, is_cid)?;

    // ---- Resolve glyph names --------------------------------
    let glyph_names: Vec<String> =
        resolve_glyph_names(&glyph_sids, &string_idx, input, is_cid)?;

    // ---- Run interpreter on each glyph ----------------------
    let mut out_glyphs: Vec<CffGlyph> = Vec::with_capacity(num_glyphs);
    for gid in 0..num_glyphs {
        let fd_idx = *fd_select.get(gid).unwrap_or(&0) as usize;
        if fd_idx >= font_dicts.len() {
            return Err(Error::BadCharString {
                reason: "FDSelect references out-of-range FontDict",
            });
        }
        let fd = &font_dicts[fd_idx];
        let cs = cs_idx.entry(input, gid)?;
        let (path, advance) =
            run_charstring(cs, fd, &global_subrs, global_bias)?;
        out_glyphs.push(CffGlyph {
            name_or_cid: glyph_names[gid].clone(),
            advance: round_to_i32(advance),
            path,
        });
    }

    Ok(Cff {
        units_per_em,
        is_cid,
        font_name,
        glyphs: out_glyphs,
    })
}

// =====================  INDEX  =============================

#[derive(Debug)]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
struct IndexHandle {
    count: usize,
    off_size: u8,
    /// Absolute file offset of the offsets array (the byte after
    /// the (count, offSize) prefix).
    offsets_start: usize,
    /// Absolute file offset of the data area (the byte after
    /// the offsets array — `offsets_start + (count+1)*off_size`).
    data_start: usize,
}

impl IndexHandle {
    #[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
    fn entry<'a>(&self, input: &'a [u8], i: usize)
        -> Result<&'a [u8], Error>
    {
        if i >= self.count {
            return Err(Error::BadIndex {
                reason: "INDEX entry index out of range",
            });
        }
        let osz = self.off_size as usize;
        let off_a = read_offset(
            input, self.offsets_start + i * osz, osz)?;
        let off_b = read_offset(
            input, self.offsets_start + (i + 1) * osz, osz)?;
        if off_a == 0 || off_b < off_a {
            return Err(Error::BadIndex {
                reason: "non-monotonic INDEX offset",
            });
        }
        let start = self.data_start + (off_a - 1);
        let end   = self.data_start + (off_b - 1);
        if end > input.len() || start > end {
            return Err(Error::BadIndex {
                reason: "INDEX entry runs past end of buffer",
            });
        }
        Ok(&input[start..end])
    }
}

/// Parse an INDEX starting at `start` in `input`. Returns the
/// handle plus the absolute offset just past the INDEX (next
/// structure starts there).
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn parse_index(input: &[u8], start: usize)
    -> Result<(IndexHandle, usize), Error>
{
    if start.checked_add(2).is_none_or(|e| e > input.len()) {
        return Err(Error::Truncated { what: "INDEX header" });
    }
    let count =
        ((input[start] as usize) << 8) | (input[start + 1] as usize);
    if count == 0 {
        // Empty INDEX: 2 bytes total, no offSize / data.
        let handle = IndexHandle {
            count: 0,
            off_size: 0,
            offsets_start: start + 2,
            data_start: start + 2,
        };
        return Ok((handle, start + 2));
    }
    if start + 3 > input.len() {
        return Err(Error::Truncated { what: "INDEX offSize" });
    }
    let off_size = input[start + 2];
    if !(1..=4).contains(&off_size) {
        return Err(Error::BadIndex {
            reason: "offSize outside 1..=4",
        });
    }
    let osz = off_size as usize;
    let offsets_start = start + 3;
    let offsets_end = offsets_start
        .checked_add((count + 1) * osz)
        .ok_or(Error::Truncated { what: "INDEX offsets" })?;
    if offsets_end > input.len() {
        return Err(Error::Truncated { what: "INDEX offsets" });
    }

    // First offset must be 1.
    let off0 = read_offset(input, offsets_start, osz)?;
    if off0 != 1 {
        return Err(Error::BadIndex {
            reason: "INDEX offset[0] must be 1",
        });
    }
    let last = read_offset(input, offsets_start + count * osz, osz)?;
    if last == 0 {
        return Err(Error::BadIndex {
            reason: "INDEX last offset is 0",
        });
    }
    let data_start = offsets_end;
    let data_end = data_start
        .checked_add(last - 1)
        .ok_or(Error::Truncated { what: "INDEX data" })?;
    if data_end > input.len() {
        return Err(Error::Truncated { what: "INDEX data" });
    }

    let handle = IndexHandle {
        count,
        off_size,
        offsets_start,
        data_start,
    };
    Ok((handle, data_end))
}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn read_offset(input: &[u8], off: usize, off_size: usize)
    -> Result<usize, Error>
{
    if off.checked_add(off_size).is_none_or(|e| e > input.len()) {
        return Err(Error::Truncated { what: "INDEX offset" });
    }
    let mut v = 0usize;
    for i in 0..off_size {
        v = (v << 8) | (input[off + i] as usize);
    }
    Ok(v)
}

// =====================  DICT decoder  ======================

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn parse_dict(bytes: &[u8]) -> Result<Dict, Error> {
    let mut entries: Vec<(u16, Vec<DictValue>)> = Vec::new();
    let mut operands: Vec<DictValue> = Vec::new();
    let mut p = 0usize;
    while p < bytes.len() {
        let b0 = bytes[p];
        match b0 {
            0..=21 => {
                // Operator (single byte) or escape (byte 12).
                let op = if b0 == 12 {
                    if p + 1 >= bytes.len() {
                        return Err(Error::MalformedDict {
                            reason: "escape op truncated",
                        });
                    }
                    p += 2;
                    (12u16 << 8) | bytes[p - 1] as u16
                } else {
                    p += 1;
                    b0 as u16
                };
                let taken = std::mem::take(&mut operands);
                entries.push((op, taken));
            }
            28 => {
                if p + 3 > bytes.len() {
                    return Err(Error::MalformedDict {
                        reason: "int16 operand truncated",
                    });
                }
                let v = i16::from_be_bytes(
                    [bytes[p + 1], bytes[p + 2]]) as i32;
                operands.push(DictValue::Int(v));
                p += 3;
            }
            29 => {
                if p + 5 > bytes.len() {
                    return Err(Error::MalformedDict {
                        reason: "int32 operand truncated",
                    });
                }
                let v = i32::from_be_bytes([
                    bytes[p + 1], bytes[p + 2],
                    bytes[p + 3], bytes[p + 4],
                ]);
                operands.push(DictValue::Int(v));
                p += 5;
            }
            30 => {
                // BCD real.
                p += 1;
                let (r, end) = parse_bcd_real(bytes, p)?;
                operands.push(DictValue::Real(r));
                p = end;
            }
            32..=246 => {
                operands.push(DictValue::Int(b0 as i32 - 139));
                p += 1;
            }
            247..=250 => {
                if p + 2 > bytes.len() {
                    return Err(Error::MalformedDict {
                        reason: "2-byte int operand truncated",
                    });
                }
                let v = (b0 as i32 - 247) * 256
                      + bytes[p + 1] as i32 + 108;
                operands.push(DictValue::Int(v));
                p += 2;
            }
            251..=254 => {
                if p + 2 > bytes.len() {
                    return Err(Error::MalformedDict {
                        reason: "2-byte negative int truncated",
                    });
                }
                let v = -((b0 as i32 - 251) * 256)
                      - bytes[p + 1] as i32 - 108;
                operands.push(DictValue::Int(v));
                p += 2;
            }
            _ => {
                return Err(Error::MalformedDict {
                    reason: "reserved DICT byte",
                });
            }
        }
    }
    if !operands.is_empty() {
        return Err(Error::MalformedDict {
            reason: "trailing operands without operator",
        });
    }
    Ok(Dict { entries })
}

/// Decode a DICT real (BCD, prefix byte 30 already consumed).
/// The terminator nibble is `0xF`; landing on either nibble of
/// the last byte is allowed — we advance to the next byte
/// boundary before returning.
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn parse_bcd_real(bytes: &[u8], start: usize)
    -> Result<(f64, usize), Error>
{
    let mut p = start;
    let mut high = true;            // next read = high nibble?
    let mut buf = String::new();
    loop {
        if p >= bytes.len() {
            return Err(Error::BadReal {
                reason: "unterminated BCD stream",
            });
        }
        let nib = if high {
            (bytes[p] >> 4) & 0x0F
        } else {
            bytes[p] & 0x0F
        };
        // Advance nibble cursor.
        if high {
            high = false;
        } else {
            high = true;
            p += 1;
        }
        match nib {
            0..=9 => buf.push(char::from(b'0' + nib)),
            0xA   => buf.push('.'),
            0xB   => buf.push_str("E+"),
            0xC   => buf.push_str("E-"),
            0xD   => return Err(Error::BadReal {
                reason: "reserved BCD nibble 0xD",
            }),
            0xE   => buf.push('-'),
            0xF   => break,
            _     => unreachable!(),
        }
    }
    // Align to next byte boundary (skip trailing garbage nibble).
    if !high {
        p += 1;
    }
    let v = buf.parse::<f64>().map_err(|_| Error::BadReal {
        reason: "BCD digits did not parse as f64",
    })?;
    Ok((v, p))
}

// =====================  Charset  ===========================

/// Returns one SID (or CID) per non-`.notdef` glyph — i.e. a
/// Vec of length `num_glyphs - 1`. gid 0 is implicit (`.notdef`
/// for non-CID; CID 0 for CID-keyed).
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn parse_charset(
    input: &[u8],
    charset_off: i32,
    num_glyphs: usize,
    is_cid: bool,
) -> Result<Vec<u16>, Error> {
    if num_glyphs == 0 {
        return Ok(Vec::new());
    }
    let needed = num_glyphs - 1;

    if (0..=2).contains(&charset_off) {
        // Predefined charsets (non-CID only).
        if is_cid {
            return Err(Error::BadIndex {
                reason: "CID font referenced predefined charset",
            });
        }
        match charset_off {
            0 => {
                // ISOAdobe: SIDs 1..=needed (i.e. the prefix of
                // the Standard Strings starting at 1). The 229
                // Adobe Standard glyph set spans SID 1..=228.
                if needed > 228 {
                    return Err(Error::BadIndex {
                        reason: "ISOAdobe predefined charset \
                                 covers only 229 glyphs",
                    });
                }
                Ok((1..=needed as u16).collect())
            }
            1 | 2 => Err(Error::UnsupportedPredefinedCharset {
                which: charset_off,
            }),
            _ => unreachable!(),
        }
    } else {
        let off = charset_off as usize;
        if off >= input.len() {
            return Err(Error::Truncated { what: "charset" });
        }
        let format = input[off];
        match format {
            0 => {
                let bytes_needed = needed * 2;
                let end = off + 1 + bytes_needed;
                if end > input.len() {
                    return Err(Error::Truncated {
                        what: "charset format 0",
                    });
                }
                let mut out = Vec::with_capacity(needed);
                for i in 0..needed {
                    let p = off + 1 + i * 2;
                    out.push(((input[p] as u16) << 8)
                             | input[p + 1] as u16);
                }
                Ok(out)
            }
            1 | 2 => {
                let n_left_size = if format == 1 { 1 } else { 2 };
                let mut out: Vec<u16> = Vec::with_capacity(needed);
                let mut p = off + 1;
                while out.len() < needed {
                    if p + 2 + n_left_size > input.len() {
                        return Err(Error::Truncated {
                            what: "charset range",
                        });
                    }
                    let first =
                        ((input[p] as u16) << 8) | input[p + 1] as u16;
                    let n_left = if format == 1 {
                        input[p + 2] as u32
                    } else {
                        ((input[p + 2] as u32) << 8)
                            | input[p + 3] as u32
                    };
                    p += 2 + n_left_size;
                    let span = (n_left as usize) + 1;
                    for k in 0..span {
                        if out.len() >= needed { break; }
                        out.push(first + k as u16);
                    }
                }
                Ok(out)
            }
            _ => Err(Error::BadCharsetFormat { format }),
        }
    }
}

// =====================  FDSelect  ==========================

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn parse_fd_select(
    input: &[u8],
    off: usize,
    num_glyphs: usize,
) -> Result<Vec<u8>, Error> {
    if off >= input.len() {
        return Err(Error::Truncated { what: "FDSelect" });
    }
    let format = input[off];
    match format {
        0 => {
            let end = off + 1 + num_glyphs;
            if end > input.len() {
                return Err(Error::Truncated {
                    what: "FDSelect format 0",
                });
            }
            Ok(input[off + 1..end].to_vec())
        }
        3 => {
            if off + 3 > input.len() {
                return Err(Error::Truncated {
                    what: "FDSelect format 3 header",
                });
            }
            let n_ranges =
                ((input[off + 1] as usize) << 8)
                    | input[off + 2] as usize;
            let body_end = off + 3 + n_ranges * 3 + 2;
            if body_end > input.len() {
                return Err(Error::Truncated {
                    what: "FDSelect format 3 body",
                });
            }
            // Read ranges.
            let mut ranges: Vec<(u16, u8)> =
                Vec::with_capacity(n_ranges);
            for i in 0..n_ranges {
                let p = off + 3 + i * 3;
                let first =
                    ((input[p] as u16) << 8) | input[p + 1] as u16;
                let fd = input[p + 2];
                ranges.push((first, fd));
            }
            let sentinel_p = off + 3 + n_ranges * 3;
            let sentinel = ((input[sentinel_p] as usize) << 8)
                | input[sentinel_p + 1] as usize;
            // Build per-glyph fd array.
            if ranges.is_empty() || ranges[0].0 != 0 {
                return Err(Error::BadIndex {
                    reason: "FDSelect format 3 must start at glyph 0",
                });
            }
            if sentinel != num_glyphs {
                return Err(Error::BadIndex {
                    reason: "FDSelect format 3 sentinel \
                            ≠ numGlyphs",
                });
            }
            let mut out = vec![0u8; num_glyphs];
            for (i, &(first, fd)) in ranges.iter().enumerate() {
                let next = if i + 1 < ranges.len() {
                    ranges[i + 1].0 as usize
                } else {
                    sentinel
                };
                if (first as usize) > next {
                    return Err(Error::BadIndex {
                        reason: "FDSelect format 3 range descending",
                    });
                }
                for g in (first as usize)..next {
                    if g >= num_glyphs {
                        return Err(Error::BadIndex {
                            reason: "FDSelect format 3 range \
                                    overruns numGlyphs",
                        });
                    }
                    out[g] = fd;
                }
            }
            Ok(out)
        }
        _ => Err(Error::BadFdSelectFormat { format }),
    }
}

// =====================  FontDict / Private DICT  ===========

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn load_font_dict(input: &[u8], dict: &Dict) -> Result<FontDictInfo, Error> {
    let priv_arr = dict.get(OP_PRIVATE)
        .ok_or(Error::MissingTopDictKey { key: "Private" })?;
    if priv_arr.len() < 2 {
        return Err(Error::MalformedDict {
            reason: "Private operand must be [size, offset]",
        });
    }
    let priv_size = priv_arr[0].as_i32() as usize;
    let priv_off  = priv_arr[1].as_i32() as usize;
    let priv_end = priv_off
        .checked_add(priv_size)
        .ok_or(Error::Truncated { what: "Private DICT" })?;
    if priv_end > input.len() {
        return Err(Error::Truncated { what: "Private DICT" });
    }
    let priv_bytes = &input[priv_off..priv_end];
    let priv_dict = parse_dict(priv_bytes)?;

    let default_width_x = priv_dict.get_f64(OP_DEFAULT_WIDTH).unwrap_or(0.0);
    let nominal_width_x = priv_dict.get_f64(OP_NOMINAL_WIDTH).unwrap_or(0.0);

    let local_subrs: Vec<Vec<u8>> = if let Some(subrs_rel) =
        priv_dict.get_int(OP_SUBRS)
    {
        let subrs_off = priv_off
            .checked_add(subrs_rel as usize)
            .ok_or(Error::Truncated { what: "Local Subrs" })?;
        let (idx, _) = parse_index(input, subrs_off)?;
        (0..idx.count)
            .map(|i| idx.entry(input, i).map(|s| s.to_vec()))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    let local_bias = bias_for_count(local_subrs.len());

    Ok(FontDictInfo {
        default_width_x,
        nominal_width_x,
        local_subrs,
        local_bias,
    })
}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn bias_for_count(count: usize) -> i32 {
    if count < 1240 {
        107
    } else if count < 33900 {
        1131
    } else {
        32768
    }
}

// =====================  Glyph names  =======================

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn resolve_glyph_names(
    glyph_sids: &[u16],
    string_idx: &IndexHandle,
    input: &[u8],
    is_cid: bool,
) -> Result<Vec<String>, Error> {
    let num_glyphs = glyph_sids.len() + 1;
    let mut names: Vec<String> = Vec::with_capacity(num_glyphs);
    if is_cid {
        // For CID fonts the charset entries are CIDs; gid 0 = CID 0.
        names.push("cid0".to_string());
        for &cid in glyph_sids {
            names.push(format!("cid{}", cid));
        }
    } else {
        names.push(".notdef".to_string());
        for &sid in glyph_sids {
            let name = sid_to_name(sid as usize, string_idx, input)?;
            names.push(name);
        }
    }
    Ok(names)
}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn sid_to_name(
    sid: usize,
    string_idx: &IndexHandle,
    input: &[u8],
) -> Result<String, Error> {
    if sid < STANDARD_STRINGS.len() {
        Ok(STANDARD_STRINGS[sid].to_string())
    } else {
        let idx = sid - STANDARD_STRINGS.len();
        if idx >= string_idx.count {
            return Err(Error::BadIndex {
                reason: "SID references past String INDEX end",
            });
        }
        let bytes = string_idx.entry(input, idx)?;
        // CFF allows arbitrary bytes; we keep them as UTF-8 if
        // possible (typical) or lossy otherwise.
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }
}

// =====================  Type 2 interpreter  ================
//
// MARKER_RUN_CHARSTRING

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn run_charstring(
    cs: &[u8],
    fd: &FontDictInfo,
    global_subrs: &[Vec<u8>],
    global_bias: i32,
) -> Result<(Vec<PathSeg>, f64), Error> {
    // Per-call state.
    let mut stack: Vec<f64> = Vec::with_capacity(96);
    let mut transient: [f64; 32] = [0.0; 32];

    // Each frame is (program bytes, pc).
    struct Frame<'a> { code: &'a [u8], pc: usize }
    let mut frames: Vec<Frame> = Vec::with_capacity(11);
    frames.push(Frame { code: cs, pc: 0 });

    // Path / width / hint state.
    let mut x: f64 = 0.0;
    let mut y: f64 = 0.0;
    let mut path: Vec<PathSeg> = Vec::new();
    let mut subpath_open = false;
    let mut advance: f64 = fd.default_width_x;
    let mut width_set = false;
    let mut saw_stack_clearing = false;
    let mut stem_count: u32 = 0;
    let mut finished = false;

    macro_rules! push_op {
        ($v:expr) => {{
            if stack.len() >= 96 {
                return Err(Error::StackOverflow);
            }
            stack.push($v);
        }};
    }

    // Width-resolution helper. `min` is the operator's declared
    // minimum operand count; `pairs` indicates the operator
    // takes additional operands in pairs (stems).
    //
    // Width is consumed iff this is the FIRST stack-clearing
    // operator AND `(stack.len() - min)` is positive AND, for
    // pair-form ops, makes the rest a multiple of 2.
    let resolve_width =
        |stack: &mut Vec<f64>,
         saw_stack_clearing: &mut bool,
         width_set: &mut bool,
         advance: &mut f64,
         fd: &FontDictInfo,
         min: usize, pairs: bool| -> Result<(), Error>
    {
        if !*saw_stack_clearing {
            *saw_stack_clearing = true;
            let depth = stack.len();
            let extra = depth.saturating_sub(min);
            let take_width = if pairs {
                // Stem-style: extra operands beyond `min` come in
                // pairs; an odd remainder = width.
                extra % 2 == 1
            } else {
                extra >= 1
            };
            if take_width {
                let w = stack.remove(0);
                *advance = w + fd.nominal_width_x;
                *width_set = true;
            }
        }
        Ok(())
    };

    // Subpath emission helpers.
    let close_if_open = |subpath_open: &mut bool,
                         path: &mut Vec<PathSeg>| {
        if *subpath_open {
            path.push(PathSeg::Close);
            *subpath_open = false;
        }
    };

    while !finished {
        let frame_idx = frames.len() - 1;
        if frames[frame_idx].pc >= frames[frame_idx].code.len() {
            return Err(Error::BadCharString {
                reason: "fell off end of program (missing endchar/return)",
            });
        }
        let b = frames[frame_idx].code[frames[frame_idx].pc];
        frames[frame_idx].pc += 1;

        // ---- Operand bytes ---------------------------------
        if b >= 32 {
            // 1-byte int (32..=246), 2-byte int (247..=254),
            // 4-byte real (255).
            match b {
                32..=246 => {
                    push_op!((b as i32 - 139) as f64);
                }
                247..=250 => {
                    let f = &mut frames[frame_idx];
                    if f.pc >= f.code.len() {
                        return Err(Error::BadCharString {
                            reason: "2-byte int operand truncated",
                        });
                    }
                    let b1 = f.code[f.pc];
                    f.pc += 1;
                    let v = (b as i32 - 247) * 256
                          + b1 as i32 + 108;
                    push_op!(v as f64);
                }
                251..=254 => {
                    let f = &mut frames[frame_idx];
                    if f.pc >= f.code.len() {
                        return Err(Error::BadCharString {
                            reason: "negative 2-byte int truncated",
                        });
                    }
                    let b1 = f.code[f.pc];
                    f.pc += 1;
                    let v = -((b as i32 - 251) * 256)
                          - b1 as i32 - 108;
                    push_op!(v as f64);
                }
                255 => {
                    let f = &mut frames[frame_idx];
                    if f.pc + 4 > f.code.len() {
                        return Err(Error::BadCharString {
                            reason: "fixed real operand truncated",
                        });
                    }
                    let raw = i32::from_be_bytes([
                        f.code[f.pc],
                        f.code[f.pc + 1],
                        f.code[f.pc + 2],
                        f.code[f.pc + 3],
                    ]);
                    f.pc += 4;
                    push_op!(raw as f64 / 65536.0);
                }
                _ => unreachable!(),
            }
            continue;
        }

        if b == 28 {
            // 2-byte int (signed, big-endian).
            let f = &mut frames[frame_idx];
            if f.pc + 2 > f.code.len() {
                return Err(Error::BadCharString {
                    reason: "int16 operand truncated",
                });
            }
            let v = i16::from_be_bytes(
                [f.code[f.pc], f.code[f.pc + 1]]) as f64;
            f.pc += 2;
            push_op!(v);
            continue;
        }

        // ---- Operator -------------------------------------
        let op: u16 = if b == 12 {
            let f = &mut frames[frame_idx];
            if f.pc >= f.code.len() {
                return Err(Error::BadCharString {
                    reason: "escape op truncated",
                });
            }
            let b2 = f.code[f.pc];
            f.pc += 1;
            (12u16 << 8) | b2 as u16
        } else {
            b as u16
        };

        match op {
            // ---- 1  hstem -----------------------------------
            // ---- 3  vstem -----------------------------------
            // ---- 18 hstemhm ---------------------------------
            // ---- 23 vstemhm ---------------------------------
            1 | 3 | 18 | 23 => {
                resolve_width(
                    &mut stack, &mut saw_stack_clearing,
                    &mut width_set, &mut advance, fd,
                    2, true)?;
                if stack.len() % 2 != 0 || stack.is_empty() {
                    return Err(Error::BadCharString {
                        reason: "stem op needs even operand count",
                    });
                }
                stem_count += (stack.len() / 2) as u32;
                stack.clear();
            }

            // ---- 19 hintmask --------------------------------
            // ---- 20 cntrmask --------------------------------
            19 | 20 => {
                resolve_width(
                    &mut stack, &mut saw_stack_clearing,
                    &mut width_set, &mut advance, fd,
                    0, true)?;
                // Implicit vstem operands on stack at hintmask
                // time count toward stem_count.
                if !stack.is_empty() {
                    if stack.len() % 2 != 0 {
                        return Err(Error::BadCharString {
                            reason: "implicit vstem operands at \
                                    hintmask must be in pairs",
                        });
                    }
                    stem_count += (stack.len() / 2) as u32;
                    stack.clear();
                }
                // Consume mask bytes.
                let mask_len = stem_count.div_ceil(8) as usize;
                let f = &mut frames[frame_idx];
                if f.pc + mask_len > f.code.len() {
                    return Err(Error::BadCharString {
                        reason: "hintmask/cntrmask byte stream truncated",
                    });
                }
                f.pc += mask_len;
            }

            // ---- 4  vmoveto (dy) ----------------------------
            4 => {
                resolve_width(
                    &mut stack, &mut saw_stack_clearing,
                    &mut width_set, &mut advance, fd,
                    1, false)?;
                if stack.is_empty() {
                    return Err(Error::StackUnderflow);
                }
                let dy = stack[stack.len() - 1];
                close_if_open(&mut subpath_open, &mut path);
                y += dy;
                path.push(PathSeg::Move {
                    x: round_to_i32(x), y: round_to_i32(y),
                });
                subpath_open = true;
                stack.clear();
            }

            // ---- 21 rmoveto (dx dy) -------------------------
            21 => {
                resolve_width(
                    &mut stack, &mut saw_stack_clearing,
                    &mut width_set, &mut advance, fd,
                    2, false)?;
                if stack.len() < 2 {
                    return Err(Error::StackUnderflow);
                }
                let dx = stack[stack.len() - 2];
                let dy = stack[stack.len() - 1];
                close_if_open(&mut subpath_open, &mut path);
                x += dx;
                y += dy;
                path.push(PathSeg::Move {
                    x: round_to_i32(x), y: round_to_i32(y),
                });
                subpath_open = true;
                stack.clear();
            }

            // ---- 22 hmoveto (dx) ----------------------------
            22 => {
                resolve_width(
                    &mut stack, &mut saw_stack_clearing,
                    &mut width_set, &mut advance, fd,
                    1, false)?;
                if stack.is_empty() {
                    return Err(Error::StackUnderflow);
                }
                let dx = stack[stack.len() - 1];
                close_if_open(&mut subpath_open, &mut path);
                x += dx;
                path.push(PathSeg::Move {
                    x: round_to_i32(x), y: round_to_i32(y),
                });
                subpath_open = true;
                stack.clear();
            }

            // ---- 5  rlineto ({dx dy}+) ----------------------
            5 => {
                resolve_width(
                    &mut stack, &mut saw_stack_clearing,
                    &mut width_set, &mut advance, fd,
                    2, true)?;
                if stack.len() < 2 || stack.len() % 2 != 0 {
                    return Err(Error::BadCharString {
                        reason: "rlineto needs even operand count",
                    });
                }
                let mut i = 0usize;
                while i < stack.len() {
                    x += stack[i];
                    y += stack[i + 1];
                    path.push(PathSeg::Line {
                        x: round_to_i32(x),
                        y: round_to_i32(y),
                    });
                    i += 2;
                }
                stack.clear();
            }

            // ---- 6  hlineto ---------------------------------
            // ---- 7  vlineto ---------------------------------
            //
            // Alternating axes; first dx (op 6) or first dy
            // (op 7), then alternating until stack drains.
            6 | 7 => {
                resolve_width(
                    &mut stack, &mut saw_stack_clearing,
                    &mut width_set, &mut advance, fd,
                    1, false)?;
                if stack.is_empty() {
                    return Err(Error::StackUnderflow);
                }
                let mut horiz = op == 6;
                for &v in &stack {
                    if horiz { x += v; } else { y += v; }
                    path.push(PathSeg::Line {
                        x: round_to_i32(x),
                        y: round_to_i32(y),
                    });
                    horiz = !horiz;
                }
                stack.clear();
            }

            // ---- 8  rrcurveto ({dx1 dy1 dx2 dy2 dx3 dy3}+) --
            8 => {
                resolve_width(
                    &mut stack, &mut saw_stack_clearing,
                    &mut width_set, &mut advance, fd,
                    6, false)?;
                if stack.is_empty() || stack.len() % 6 != 0 {
                    return Err(Error::BadCharString {
                        reason: "rrcurveto needs multiple-of-6 \
                                operands",
                    });
                }
                let mut i = 0usize;
                while i < stack.len() {
                    let x1 = x + stack[i];
                    let y1 = y + stack[i + 1];
                    let x2 = x1 + stack[i + 2];
                    let y2 = y1 + stack[i + 3];
                    let x3 = x2 + stack[i + 4];
                    let y3 = y2 + stack[i + 5];
                    path.push(PathSeg::Curve {
                        x1: round_to_i32(x1), y1: round_to_i32(y1),
                        x2: round_to_i32(x2), y2: round_to_i32(y2),
                        x3: round_to_i32(x3), y3: round_to_i32(y3),
                    });
                    x = x3;
                    y = y3;
                    i += 6;
                }
                stack.clear();
            }

            // ---- 24 rcurveline ({c}+ dx dy) -----------------
            24 => {
                resolve_width(
                    &mut stack, &mut saw_stack_clearing,
                    &mut width_set, &mut advance, fd,
                    8, false)?;
                if stack.len() < 8 || (stack.len() - 2) % 6 != 0 {
                    return Err(Error::BadCharString {
                        reason: "rcurveline operand count",
                    });
                }
                let curve_end = stack.len() - 2;
                let mut i = 0usize;
                while i < curve_end {
                    let x1 = x + stack[i];
                    let y1 = y + stack[i + 1];
                    let x2 = x1 + stack[i + 2];
                    let y2 = y1 + stack[i + 3];
                    let x3 = x2 + stack[i + 4];
                    let y3 = y2 + stack[i + 5];
                    path.push(PathSeg::Curve {
                        x1: round_to_i32(x1), y1: round_to_i32(y1),
                        x2: round_to_i32(x2), y2: round_to_i32(y2),
                        x3: round_to_i32(x3), y3: round_to_i32(y3),
                    });
                    x = x3;
                    y = y3;
                    i += 6;
                }
                x += stack[curve_end];
                y += stack[curve_end + 1];
                path.push(PathSeg::Line {
                    x: round_to_i32(x), y: round_to_i32(y),
                });
                stack.clear();
            }

            // ---- 25 rlinecurve ({dx dy}+ c) -----------------
            25 => {
                resolve_width(
                    &mut stack, &mut saw_stack_clearing,
                    &mut width_set, &mut advance, fd,
                    8, false)?;
                if stack.len() < 8 || (stack.len() - 6) % 2 != 0 {
                    return Err(Error::BadCharString {
                        reason: "rlinecurve operand count",
                    });
                }
                let line_end = stack.len() - 6;
                let mut i = 0usize;
                while i < line_end {
                    x += stack[i];
                    y += stack[i + 1];
                    path.push(PathSeg::Line {
                        x: round_to_i32(x),
                        y: round_to_i32(y),
                    });
                    i += 2;
                }
                let x1 = x + stack[line_end];
                let y1 = y + stack[line_end + 1];
                let x2 = x1 + stack[line_end + 2];
                let y2 = y1 + stack[line_end + 3];
                let x3 = x2 + stack[line_end + 4];
                let y3 = y2 + stack[line_end + 5];
                path.push(PathSeg::Curve {
                    x1: round_to_i32(x1), y1: round_to_i32(y1),
                    x2: round_to_i32(x2), y2: round_to_i32(y2),
                    x3: round_to_i32(x3), y3: round_to_i32(y3),
                });
                x = x3;
                y = y3;
                stack.clear();
            }

            // ---- 26 vvcurveto -------------------------------
            //
            // optional dx0 followed by {dy1 dx2 dy2 dy3}+.
            26 => {
                resolve_width(
                    &mut stack, &mut saw_stack_clearing,
                    &mut width_set, &mut advance, fd,
                    4, false)?;
                let mut i = 0usize;
                let n = stack.len();
                let has_dx0 = (n % 4) == 1;
                if has_dx0 {
                    x += stack[0];
                    i = 1;
                }
                if (n - i) == 0 || (n - i) % 4 != 0 {
                    return Err(Error::BadCharString {
                        reason: "vvcurveto operand count",
                    });
                }
                while i < n {
                    let x1 = x;
                    let y1 = y + stack[i];
                    let x2 = x1 + stack[i + 1];
                    let y2 = y1 + stack[i + 2];
                    let x3 = x2;
                    let y3 = y2 + stack[i + 3];
                    path.push(PathSeg::Curve {
                        x1: round_to_i32(x1), y1: round_to_i32(y1),
                        x2: round_to_i32(x2), y2: round_to_i32(y2),
                        x3: round_to_i32(x3), y3: round_to_i32(y3),
                    });
                    x = x3;
                    y = y3;
                    i += 4;
                }
                stack.clear();
            }

            // ---- 27 hhcurveto -------------------------------
            //
            // optional dy0 followed by {dx1 dx2 dy2 dx3}+.
            27 => {
                resolve_width(
                    &mut stack, &mut saw_stack_clearing,
                    &mut width_set, &mut advance, fd,
                    4, false)?;
                let mut i = 0usize;
                let n = stack.len();
                let has_dy0 = (n % 4) == 1;
                if has_dy0 {
                    y += stack[0];
                    i = 1;
                }
                if (n - i) == 0 || (n - i) % 4 != 0 {
                    return Err(Error::BadCharString {
                        reason: "hhcurveto operand count",
                    });
                }
                while i < n {
                    let x1 = x + stack[i];
                    let y1 = y;
                    let x2 = x1 + stack[i + 1];
                    let y2 = y1 + stack[i + 2];
                    let x3 = x2 + stack[i + 3];
                    let y3 = y2;
                    path.push(PathSeg::Curve {
                        x1: round_to_i32(x1), y1: round_to_i32(y1),
                        x2: round_to_i32(x2), y2: round_to_i32(y2),
                        x3: round_to_i32(x3), y3: round_to_i32(y3),
                    });
                    x = x3;
                    y = y3;
                    i += 4;
                }
                stack.clear();
            }

            // ---- 30 vhcurveto -------------------------------
            // ---- 31 hvcurveto -------------------------------
            //
            // Alternating-axis curves; tail can be a 5-operand
            // curve carrying a final off-axis offset (dxf or dyf).
            //
            // Spec (§4.4 of TN #5177): valid operand counts are
            //   4N         (N curves, no tail)
            //   4N + 1     (N curves, last with dxf/dyf tail)
            //   4N + 4     (N + 1 curves, no tail) — covered by 4N
            //   4N + 5     (N + 1 curves, last with tail) — 4N+1
            // So: count `mod 4` is either 0 or 1 (with a tail).
            30 | 31 => {
                resolve_width(
                    &mut stack, &mut saw_stack_clearing,
                    &mut width_set, &mut advance, fd,
                    4, false)?;
                let n = stack.len();
                if n < 4 || (n % 4 != 0 && n % 4 != 1) {
                    return Err(Error::BadCharString {
                        reason: "v/hvcurveto operand count",
                    });
                }
                let mut vert_first = op == 30; // 30: vh starts vertical
                let mut i = 0usize;
                while i < n {
                    let remaining = n - i;
                    let with_tail = remaining == 5;
                    if remaining < 4 {
                        return Err(Error::BadCharString {
                            reason: "v/hvcurveto stub remainder",
                        });
                    }
                    if vert_first {
                        // vert_first curve: dy1 dx2 dy2 dx3 [dyf].
                        let x1 = x;
                        let y1 = y + stack[i];
                        let x2 = x1 + stack[i + 1];
                        let y2 = y1 + stack[i + 2];
                        let x3 = x2 + stack[i + 3];
                        let mut y3 = y2;
                        if with_tail {
                            y3 += stack[i + 4];
                        }
                        path.push(PathSeg::Curve {
                            x1: round_to_i32(x1), y1: round_to_i32(y1),
                            x2: round_to_i32(x2), y2: round_to_i32(y2),
                            x3: round_to_i32(x3), y3: round_to_i32(y3),
                        });
                        x = x3;
                        y = y3;
                    } else {
                        // horiz_first curve: dx1 dx2 dy2 dy3 [dxf].
                        let x1 = x + stack[i];
                        let y1 = y;
                        let x2 = x1 + stack[i + 1];
                        let y2 = y1 + stack[i + 2];
                        let mut x3 = x2;
                        let y3 = y2 + stack[i + 3];
                        if with_tail {
                            x3 += stack[i + 4];
                        }
                        path.push(PathSeg::Curve {
                            x1: round_to_i32(x1), y1: round_to_i32(y1),
                            x2: round_to_i32(x2), y2: round_to_i32(y2),
                            x3: round_to_i32(x3), y3: round_to_i32(y3),
                        });
                        x = x3;
                        y = y3;
                    }
                    vert_first = !vert_first;
                    i += if with_tail { 5 } else { 4 };
                }
                stack.clear();
            }

            // ---- 14 endchar ---------------------------------
            14 => {
                resolve_width(
                    &mut stack, &mut saw_stack_clearing,
                    &mut width_set, &mut advance, fd,
                    0, false)?;
                if !stack.is_empty() {
                    // Anything still on the stack at endchar
                    // beyond the optional width is the deprecated
                    // seac compound-glyph form (4 operands).
                    return Err(Error::SeacNotSupported);
                }
                close_if_open(&mut subpath_open, &mut path);
                finished = true;
            }

            // ---- 10 callsubr (local) ------------------------
            10 => {
                if stack.is_empty() {
                    return Err(Error::StackUnderflow);
                }
                let biased = stack.pop().unwrap() as i32;
                let idx = biased + fd.local_bias;
                if idx < 0 || (idx as usize) >= fd.local_subrs.len() {
                    return Err(Error::SubrOutOfRange { biased });
                }
                if frames.len() >= 11 {
                    return Err(Error::SubrDepthExceeded);
                }
                frames.push(Frame {
                    code: &fd.local_subrs[idx as usize],
                    pc: 0,
                });
            }

            // ---- 29 callgsubr (global) ----------------------
            29 => {
                if stack.is_empty() {
                    return Err(Error::StackUnderflow);
                }
                let biased = stack.pop().unwrap() as i32;
                let idx = biased + global_bias;
                if idx < 0 || (idx as usize) >= global_subrs.len() {
                    return Err(Error::SubrOutOfRange { biased });
                }
                if frames.len() >= 11 {
                    return Err(Error::SubrDepthExceeded);
                }
                frames.push(Frame {
                    code: &global_subrs[idx as usize],
                    pc: 0,
                });
            }

            // ---- 11 return ----------------------------------
            11 => {
                frames.pop();
                if frames.is_empty() {
                    return Err(Error::BadCharString {
                        reason: "return at top frame",
                    });
                }
            }

            // ---- 12 0 dotsection (deprecated; no-op) --------
            0x0C00 => {
                // Stack untouched per spec.
            }

            // ---- 12 3 and -----------------------------------
            0x0C03 => {
                let b = stack.pop().ok_or(Error::StackUnderflow)?;
                let a = stack.pop().ok_or(Error::StackUnderflow)?;
                push_op!(if a != 0.0 && b != 0.0 { 1.0 } else { 0.0 });
            }
            // ---- 12 4 or ------------------------------------
            0x0C04 => {
                let b = stack.pop().ok_or(Error::StackUnderflow)?;
                let a = stack.pop().ok_or(Error::StackUnderflow)?;
                push_op!(if a != 0.0 || b != 0.0 { 1.0 } else { 0.0 });
            }
            // ---- 12 5 not -----------------------------------
            0x0C05 => {
                let a = stack.pop().ok_or(Error::StackUnderflow)?;
                push_op!(if a == 0.0 { 1.0 } else { 0.0 });
            }
            // ---- 12 9 abs -----------------------------------
            0x0C09 => {
                let a = stack.pop().ok_or(Error::StackUnderflow)?;
                push_op!(a.abs());
            }
            // ---- 12 10 add ----------------------------------
            0x0C0A => {
                let b = stack.pop().ok_or(Error::StackUnderflow)?;
                let a = stack.pop().ok_or(Error::StackUnderflow)?;
                push_op!(a + b);
            }
            // ---- 12 11 sub ----------------------------------
            0x0C0B => {
                let b = stack.pop().ok_or(Error::StackUnderflow)?;
                let a = stack.pop().ok_or(Error::StackUnderflow)?;
                push_op!(a - b);
            }
            // ---- 12 12 div ----------------------------------
            0x0C0C => {
                let b = stack.pop().ok_or(Error::StackUnderflow)?;
                let a = stack.pop().ok_or(Error::StackUnderflow)?;
                if b == 0.0 {
                    return Err(Error::BadCharString {
                        reason: "div by zero",
                    });
                }
                push_op!(a / b);
            }
            // ---- 12 14 neg ----------------------------------
            0x0C0E => {
                let a = stack.pop().ok_or(Error::StackUnderflow)?;
                push_op!(-a);
            }
            // ---- 12 15 eq -----------------------------------
            0x0C0F => {
                let b = stack.pop().ok_or(Error::StackUnderflow)?;
                let a = stack.pop().ok_or(Error::StackUnderflow)?;
                push_op!(if a == b { 1.0 } else { 0.0 });
            }
            // ---- 12 18 drop ---------------------------------
            0x0C12 => {
                stack.pop().ok_or(Error::StackUnderflow)?;
            }
            // ---- 12 20 put ----------------------------------
            0x0C14 => {
                let idx = stack.pop().ok_or(Error::StackUnderflow)?
                    as i32;
                let val = stack.pop().ok_or(Error::StackUnderflow)?;
                if idx < 0 || (idx as usize) >= transient.len() {
                    return Err(Error::TransientOutOfRange);
                }
                transient[idx as usize] = val;
            }
            // ---- 12 21 get ----------------------------------
            0x0C15 => {
                let idx = stack.pop().ok_or(Error::StackUnderflow)?
                    as i32;
                if idx < 0 || (idx as usize) >= transient.len() {
                    return Err(Error::TransientOutOfRange);
                }
                push_op!(transient[idx as usize]);
            }
            // ---- 12 22 ifelse -------------------------------
            0x0C16 => {
                let v2 = stack.pop().ok_or(Error::StackUnderflow)?;
                let v1 = stack.pop().ok_or(Error::StackUnderflow)?;
                let s2 = stack.pop().ok_or(Error::StackUnderflow)?;
                let s1 = stack.pop().ok_or(Error::StackUnderflow)?;
                push_op!(if v1 <= v2 { s1 } else { s2 });
            }
            // ---- 12 23 random -------------------------------
            0x0C17 => return Err(Error::RandomNotSupported),
            // ---- 12 24 mul ----------------------------------
            0x0C18 => {
                let b = stack.pop().ok_or(Error::StackUnderflow)?;
                let a = stack.pop().ok_or(Error::StackUnderflow)?;
                push_op!(a * b);
            }
            // ---- 12 26 sqrt ---------------------------------
            0x0C1A => {
                let a = stack.pop().ok_or(Error::StackUnderflow)?;
                if a < 0.0 {
                    return Err(Error::BadCharString {
                        reason: "sqrt of negative",
                    });
                }
                push_op!(a.sqrt());
            }
            // ---- 12 27 dup ----------------------------------
            0x0C1B => {
                let a = *stack.last().ok_or(Error::StackUnderflow)?;
                push_op!(a);
            }
            // ---- 12 28 exch ---------------------------------
            0x0C1C => {
                let b = stack.pop().ok_or(Error::StackUnderflow)?;
                let a = stack.pop().ok_or(Error::StackUnderflow)?;
                push_op!(b);
                push_op!(a);
            }
            // ---- 12 29 index --------------------------------
            0x0C1D => {
                let mut i = stack.pop()
                    .ok_or(Error::StackUnderflow)? as i32;
                if i < 0 { i = 0; }
                let depth = stack.len();
                if (i as usize) >= depth {
                    return Err(Error::StackUnderflow);
                }
                let v = stack[depth - 1 - i as usize];
                push_op!(v);
            }
            // ---- 12 30 roll ---------------------------------
            0x0C1E => {
                let j = stack.pop()
                    .ok_or(Error::StackUnderflow)? as i32;
                let n = stack.pop()
                    .ok_or(Error::StackUnderflow)? as i32;
                if n < 0 || (n as usize) > stack.len() {
                    return Err(Error::BadCharString {
                        reason: "roll N out of range",
                    });
                }
                if n == 0 { continue; }
                let nn = n as usize;
                let base = stack.len() - nn;
                // Rotate top N items by J (positive J rotates
                // toward stack top).
                let jj = ((j % n) + n) % n;
                let slice = &mut stack[base..];
                slice.rotate_right(jj as usize);
            }

            // ---- 12 34 hflex --------------------------------
            //
            // dx1 dx2 dy2 dx3 dx4 dx5 dx6 (7 operands).
            // Both curves on a horizontal flex axis (start-y =
            // end-y = current-y; ditto for fd at 50/100 = 0.5
            // — we ignore fd because we emit cubics directly).
            0x0C22 => {
                if stack.len() < 7 {
                    return Err(Error::StackUnderflow);
                }
                let dx1 = stack[0];
                let dx2 = stack[1];
                let dy2 = stack[2];
                let dx3 = stack[3];
                let dx4 = stack[4];
                let dx5 = stack[5];
                let dx6 = stack[6];
                let x1 = x + dx1;
                let y1 = y;
                let x2 = x1 + dx2;
                let y2 = y1 + dy2;
                let x3 = x2 + dx3;
                let y3 = y2;
                path.push(PathSeg::Curve {
                    x1: round_to_i32(x1), y1: round_to_i32(y1),
                    x2: round_to_i32(x2), y2: round_to_i32(y2),
                    x3: round_to_i32(x3), y3: round_to_i32(y3),
                });
                let x4 = x3 + dx4;
                let y4 = y3;
                let x5 = x4 + dx5;
                let y5 = y4 - dy2;
                let x6 = x5 + dx6;
                let y6 = y; // back to original y
                path.push(PathSeg::Curve {
                    x1: round_to_i32(x4), y1: round_to_i32(y4),
                    x2: round_to_i32(x5), y2: round_to_i32(y5),
                    x3: round_to_i32(x6), y3: round_to_i32(y6),
                });
                x = x6;
                y = y6;
                stack.clear();
            }

            // ---- 12 35 flex ---------------------------------
            //
            // dx1 dy1 dx2 dy2 dx3 dy3 dx4 dy4 dx5 dy5 dx6 dy6 fd
            // (12 operands + flex-depth fd, 13 total).
            0x0C23 => {
                if stack.len() < 13 {
                    return Err(Error::StackUnderflow);
                }
                let s = &stack;
                let x1 = x + s[0];   let y1 = y + s[1];
                let x2 = x1 + s[2];  let y2 = y1 + s[3];
                let x3 = x2 + s[4];  let y3 = y2 + s[5];
                path.push(PathSeg::Curve {
                    x1: round_to_i32(x1), y1: round_to_i32(y1),
                    x2: round_to_i32(x2), y2: round_to_i32(y2),
                    x3: round_to_i32(x3), y3: round_to_i32(y3),
                });
                let x4 = x3 + s[6];  let y4 = y3 + s[7];
                let x5 = x4 + s[8];  let y5 = y4 + s[9];
                let x6 = x5 + s[10]; let y6 = y5 + s[11];
                path.push(PathSeg::Curve {
                    x1: round_to_i32(x4), y1: round_to_i32(y4),
                    x2: round_to_i32(x5), y2: round_to_i32(y5),
                    x3: round_to_i32(x6), y3: round_to_i32(y6),
                });
                x = x6;
                y = y6;
                stack.clear();
            }

            // ---- 12 36 hflex1 -------------------------------
            //
            // dx1 dy1 dx2 dy2 dx3 dx4 dx5 dy5 dx6 (9 operands).
            // Returns the y to its starting value.
            0x0C24 => {
                if stack.len() < 9 {
                    return Err(Error::StackUnderflow);
                }
                let s = &stack;
                let start_y = y;
                let x1 = x + s[0];   let y1 = y + s[1];
                let x2 = x1 + s[2];  let y2 = y1 + s[3];
                let x3 = x2 + s[4];  let y3 = y2;
                path.push(PathSeg::Curve {
                    x1: round_to_i32(x1), y1: round_to_i32(y1),
                    x2: round_to_i32(x2), y2: round_to_i32(y2),
                    x3: round_to_i32(x3), y3: round_to_i32(y3),
                });
                let x4 = x3 + s[5];  let y4 = y3;
                let x5 = x4 + s[6];  let y5 = y4 + s[7];
                let x6 = x5 + s[8];  let y6 = start_y;
                path.push(PathSeg::Curve {
                    x1: round_to_i32(x4), y1: round_to_i32(y4),
                    x2: round_to_i32(x5), y2: round_to_i32(y5),
                    x3: round_to_i32(x6), y3: round_to_i32(y6),
                });
                x = x6;
                y = y6;
                stack.clear();
            }

            // ---- 12 37 flex1 --------------------------------
            //
            // dx1 dy1 dx2 dy2 dx3 dy3 dx4 dy4 dx5 dy5 d6
            // (11 operands; d6 is dx6 if abs(Δx) > abs(Δy), else dy6.
            //  Final coordinate is constrained to start-axis.)
            0x0C25 => {
                if stack.len() < 11 {
                    return Err(Error::StackUnderflow);
                }
                let s = &stack;
                let start_x = x;
                let start_y = y;
                let x1 = x + s[0];   let y1 = y + s[1];
                let x2 = x1 + s[2];  let y2 = y1 + s[3];
                let x3 = x2 + s[4];  let y3 = y2 + s[5];
                path.push(PathSeg::Curve {
                    x1: round_to_i32(x1), y1: round_to_i32(y1),
                    x2: round_to_i32(x2), y2: round_to_i32(y2),
                    x3: round_to_i32(x3), y3: round_to_i32(y3),
                });
                let x4 = x3 + s[6];  let y4 = y3 + s[7];
                let x5 = x4 + s[8];  let y5 = y4 + s[9];
                let d6 = s[10];
                let dx_total = x5 - start_x;
                let dy_total = y5 - start_y;
                let (x6, y6) =
                    if dx_total.abs() > dy_total.abs() {
                        (x5 + d6, start_y)
                    } else {
                        (start_x, y5 + d6)
                    };
                path.push(PathSeg::Curve {
                    x1: round_to_i32(x4), y1: round_to_i32(y4),
                    x2: round_to_i32(x5), y2: round_to_i32(y5),
                    x3: round_to_i32(x6), y3: round_to_i32(y6),
                });
                x = x6;
                y = y6;
                stack.clear();
            }

            _ => {
                return Err(Error::BadCharString {
                    reason: "unknown Type 2 operator",
                });
            }
        }
    }

    let _ = width_set;
    Ok((path, advance))
}

// =====================  Canonical emitter  =================

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub fn to_canonical(cff: &Cff) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(
        128 + cff.glyphs.len() * 64);

    let _ = writeln!(out, "unitsPerEm {}", cff.units_per_em);
    let _ = writeln!(out, "isCID {}", if cff.is_cid { 1 } else { 0 });
    let _ = writeln!(out, "fontName {}", escape_font_name(&cff.font_name));
    let _ = writeln!(out, "numGlyphs {}", cff.glyphs.len());

    for (gid, g) in cff.glyphs.iter().enumerate() {
        let _ = writeln!(out, "glyph {} {} {}",
                         gid, g.name_or_cid, g.advance);
        for seg in &g.path {
            match *seg {
                PathSeg::Move { x, y } =>
                    { let _ = writeln!(out, "  M {} {}", x, y); }
                PathSeg::Line { x, y } =>
                    { let _ = writeln!(out, "  L {} {}", x, y); }
                PathSeg::Curve { x1, y1, x2, y2, x3, y3 } =>
                    { let _ = writeln!(out,
                        "  C {} {} {} {} {} {}",
                        x1, y1, x2, y2, x3, y3); }
                PathSeg::Close =>
                    { let _ = writeln!(out, "  Z"); }
            }
        }
    }
    out
}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn escape_font_name(name: &[u8]) -> String {
    let mut out = String::with_capacity(name.len());
    for &b in name {
        if (0x21..=0x7E).contains(&b) {
            out.push(b as char);
        } else {
            let _ = std::fmt::Write::write_fmt(
                &mut out, format_args!("%{:02X}", b));
        }
    }
    out
}

// =====================  Misc helpers  ======================

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn round_to_i32(v: f64) -> i32 {
    if !v.is_finite() {
        return 0;
    }
    v.round() as i32
}

// =====================  Tests  =============================

#[cfg(test)]
mod tests {
    use super::*;

    // INDEX with one 3-byte entry "abc".
    fn build_one_entry_index() -> Vec<u8> {
        // count=1 (u16), offSize=1, offsets=[1, 4] (1 byte each),
        // data="abc".
        vec![
            0x00, 0x01, // count
            0x01,       // offSize
            0x01, 0x04, // offsets
            b'a', b'b', b'c',
        ]
    }

    #[test]
    fn empty_index_two_bytes() {
        let bytes = vec![0x00, 0x00];
        let (idx, end) = parse_index(&bytes, 0).unwrap();
        assert_eq!(idx.count, 0);
        assert_eq!(end, 2);
    }

    #[test]
    fn single_entry_index_round_trip() {
        let bytes = build_one_entry_index();
        let (idx, end) = parse_index(&bytes, 0).unwrap();
        assert_eq!(idx.count, 1);
        assert_eq!(idx.entry(&bytes, 0).unwrap(), b"abc");
        assert_eq!(end, bytes.len());
    }

    #[test]
    fn dict_int_decoding_small_positive() {
        // bytes encode: 100 endchar  (operand 0xEF = 239 → 239-139=100)
        let dict = parse_dict(&[0xEF, 14]).unwrap();
        assert_eq!(dict.entries.len(), 1);
        assert_eq!(dict.entries[0].0, 14);
        assert_eq!(dict.entries[0].1[0].as_i32(), 100);
    }

    #[test]
    fn dict_int_decoding_small_negative() {
        // 0xFB 0x00 = -((251-251)*256) - 0 - 108 = -108.
        let dict = parse_dict(&[0xFB, 0x00, 14]).unwrap();
        assert_eq!(dict.entries[0].1[0].as_i32(), -108);
    }

    #[test]
    fn dict_int_decoding_int16() {
        // 28 0x10 0x00 = 4096.
        let dict = parse_dict(&[28, 0x10, 0x00, 14]).unwrap();
        assert_eq!(dict.entries[0].1[0].as_i32(), 4096);
    }

    #[test]
    fn dict_int_decoding_int32() {
        // 29 0x00 0x01 0x00 0x00 = 65536.
        let dict =
            parse_dict(&[29, 0x00, 0x01, 0x00, 0x00, 14]).unwrap();
        assert_eq!(dict.entries[0].1[0].as_i32(), 65536);
    }

    #[test]
    #[allow(clippy::approx_constant)] // 3.14 is the decoded literal, not π
    fn dict_real_decoding_pi_like() {
        // 30 + nibble stream encoding "3.14e0":
        //   '3' 'a' '1' '4' 'b' '0' 'f'
        // Pack: 0x3A 0x14 0xB0 0xFF (last F + pad 0xF).
        // BCD nibbles: 3 a 1 4 b 0 f (and one trailing pad).
        let dict = parse_dict(&[
            30, 0x3A, 0x14, 0xB0, 0xFF,
            14,
        ]).unwrap();
        let v = dict.entries[0].1[0].as_f64();
        assert!((v - 3.14).abs() < 1e-12);
    }

    #[test]
    fn dict_real_decoding_negative() {
        // "-2.5" = nibbles e 2 a 5 f (5 nibbles = 3 bytes with pad).
        // Pack: 0xE2 0xA5 0xFx → 0xE2 0xA5 0xF0.
        let dict = parse_dict(&[
            30, 0xE2, 0xA5, 0xF0,
            14,
        ]).unwrap();
        let v = dict.entries[0].1[0].as_f64();
        assert!((v - (-2.5)).abs() < 1e-12);
    }

    #[test]
    fn bias_formula() {
        assert_eq!(bias_for_count(0), 107);
        assert_eq!(bias_for_count(107), 107);
        assert_eq!(bias_for_count(1239), 107);
        assert_eq!(bias_for_count(1240), 1131);
        assert_eq!(bias_for_count(33899), 1131);
        assert_eq!(bias_for_count(33900), 32768);
    }

    #[test]
    fn font_name_escape_passthrough() {
        assert_eq!(escape_font_name(b"Foo-Bar"), "Foo-Bar");
    }

    #[test]
    fn font_name_escape_high_byte() {
        assert_eq!(escape_font_name(b"A\xFFB"), "A%FFB");
    }

    #[test]
    fn round_to_i32_half_away_from_zero() {
        assert_eq!(round_to_i32(0.5),  1);
        assert_eq!(round_to_i32(-0.5), -1);
        assert_eq!(round_to_i32(1.4),  1);
        assert_eq!(round_to_i32(-1.4), -1);
    }
}
