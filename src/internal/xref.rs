// Cross-reference table parser (PDF 32000-1:2008 §7.5.4 +
// §7.5.7 + §7.5.8). Pure function from raw PDF bytes to a
// table enumerating every in-use object's location: file offset
// for uncompressed objects, container ObjStm id + index for
// compressed objects.
//
// Strategy:
//   1. Locate `startxref` near the file tail and read the
//      decimal offset on the line above it.
//   2. At that offset, branch:
//        - first non-trivia token is `xref` → classical table
//          (§7.5.4) followed by a `trailer << ... >>` dict.
//        - first non-trivia token is a Number (the start of
//          an indirect-object header `N M obj`) → /XRef stream
//          (§7.5.8) — read the stream, FlateDecode, unpack
//          binary entries per the dict's /W widths.
//   3. Follow the /Prev chain: read the older section, merge
//      its entries — newer revisions supersede older ones by
//      `(id, generation)`. Free entries are dropped.
//   4. Sort the merged entries by `(id, generation)` and
//      return.

use std::collections::BTreeMap;
use std::collections::HashSet;
use std::fmt;

use super::flate;
use super::lexer::{lex, Token, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    InUse { id: u32, gen: u16, offset: u64 },
    Compressed { id: u32, gen: u16, stream_id: u32, index: u32 },
}

impl Entry {
    fn id(&self) -> u32 {
        match self {
            Entry::InUse { id, .. } => *id,
            Entry::Compressed { id, .. } => *id,
        }
    }
    fn gen(&self) -> u16 {
        match self {
            Entry::InUse { gen, .. } => *gen,
            Entry::Compressed { gen, .. } => *gen,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Table {
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// No `startxref` keyword in the last 4 KiB of input.
    MissingStartxref,
    /// Classical-xref structure is broken.
    MalformedXref,
    /// /XRef stream dictionary or binary payload is malformed.
    MalformedXrefStream,
    /// /XRef stream is missing /Filter /FlateDecode (only filter
    /// supported in v1).
    MissingFlateFilter,
    /// FlateDecode failed on the /XRef stream payload.
    FlateDecodeFailed,
    /// /XRef stream entry carried an unsupported type field.
    UnsupportedXrefStreamType(u32),
    /// /Prev chain visited the same section offset twice.
    PrevChainCycle,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MissingStartxref =>
                write!(f, "xref: missing startxref"),
            Error::MalformedXref =>
                write!(f, "xref: malformed classical xref table"),
            Error::MalformedXrefStream =>
                write!(f, "xref: malformed /XRef stream"),
            Error::MissingFlateFilter =>
                write!(f, "xref: /XRef stream not FlateDecode-encoded"),
            Error::FlateDecodeFailed =>
                write!(f, "xref: FlateDecode failed on /XRef stream"),
            Error::UnsupportedXrefStreamType(t) =>
                write!(f, "xref: unsupported /XRef stream entry type {t}"),
            Error::PrevChainCycle =>
                write!(f, "xref: /Prev chain cycle detected"),
        }
    }
}

impl std::error::Error for Error {}

const TAIL_SCAN: usize = 4096;

// ============================================================
// Public API
// ============================================================

/// Parse the cross-reference table out of `input`.
pub fn parse(input: &[u8]) -> Result<Table, Error> {
    let startxref_at = find_keyword_in_tail(input, b"startxref", TAIL_SCAN)
        .ok_or(Error::MissingStartxref)?;
    // Read the decimal offset on the line(s) following startxref.
    let xref_offset = read_decimal_after(input, startxref_at + b"startxref".len())
        .ok_or(Error::MissingStartxref)?;

    // Walk the chain — newer first, older entries fill gaps.
    let mut by_id: BTreeMap<(u32, u16), Entry> = BTreeMap::new();
    let mut current = Some(xref_offset);
    let mut visited: HashSet<u64> = HashSet::new();
    while let Some(off) = current {
        if !visited.insert(off) {
            return Err(Error::PrevChainCycle);
        }
        let (entries, prev) = parse_section(input, off as usize)?;
        for e in entries {
            // newer-supersedes-older: only insert if not present.
            by_id.entry((e.id(), e.gen())).or_insert(e);
        }
        current = prev;
    }
    let entries: Vec<Entry> = by_id.into_values().collect();
    Ok(Table { entries })
}

/// Emit the freeze-gate canonical form: one line per entry,
/// sorted by `(id, gen)`, trailing newline after the last line.
#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn to_canonical(table: &Table) -> Vec<u8> {
    let mut out = Vec::new();
    for e in &table.entries {
        match *e {
            Entry::InUse { id, gen, offset } => {
                out.extend_from_slice(
                    format!("{id} {gen} inuse {offset}\n").as_bytes());
            }
            Entry::Compressed { id, gen, stream_id, index } => {
                out.extend_from_slice(
                    format!("{id} {gen} compressed {stream_id} {index}\n")
                        .as_bytes());
            }
        }
    }
    out
}

// ============================================================
// Section parser — dispatches on classical vs stream form
// ============================================================

/// Parse the cross-reference section at byte `off`. Returns
/// (entries from this section, /Prev offset if any).
fn parse_section(
    input: &[u8],
    off: usize,
) -> Result<(Vec<Entry>, Option<u64>), Error> {
    if off >= input.len() {
        return Err(Error::MalformedXref);
    }
    // Skip leading whitespace; classical sections start with
    // `xref` and stream sections with a digit (the indirect-
    // object header).
    let mut p = off;
    while p < input.len() && is_pdf_ws(input[p]) {
        p += 1;
    }
    if p + 4 <= input.len() && &input[p..p + 4] == b"xref" {
        // Word boundary: byte after "xref" must be whitespace.
        if p + 4 == input.len() || is_pdf_ws(input[p + 4]) {
            return parse_classical(input, p + 4);
        }
    }
    // Otherwise treat as /XRef stream.
    parse_xref_stream(input, off)
}

// ------------------------------------------------------------
// Classical xref table (§7.5.4)
// ------------------------------------------------------------

fn parse_classical(
    input: &[u8],
    off: usize,
) -> Result<(Vec<Entry>, Option<u64>), Error> {
    let mut entries: Vec<Entry> = Vec::new();
    let mut p = off;
    // Sub-section loop: each subsection is `<first_id> <count>\n`
    // followed by `count` 20-byte entry records, until we hit the
    // `trailer` keyword.
    loop {
        skip_ws_and_eol(input, &mut p);
        // Detect end of table — `trailer` keyword.
        if p + 7 <= input.len() && &input[p..p + 7] == b"trailer" {
            p += 7;
            break;
        }
        // Subsection header: two non-negative integers separated
        // by whitespace, terminated by an EOL.
        let first = read_decimal(input, &mut p).ok_or(Error::MalformedXref)?;
        skip_inline_ws(input, &mut p);
        let count = read_decimal(input, &mut p).ok_or(Error::MalformedXref)?;
        // Step over the EOL separator — ONE EOL marker (LF, CR,
        // or CRLF).
        skip_one_eol(input, &mut p);
        // Read `count` 20-byte records.
        for i in 0..count {
            // Each entry is exactly 20 bytes per §7.5.4:
            // "nnnnnnnnnn ggggg n\n" or "nnnnnnnnnn ggggg f\n"
            // (ten-digit field, space, five-digit gen, space,
            // single-letter type, EOL pair).
            if p + 20 > input.len() {
                return Err(Error::MalformedXref);
            }
            let rec = &input[p..p + 20];
            p += 20;
            let field1 = parse_uint_str(&rec[0..10])?;
            let field2 = parse_uint_str(&rec[11..16])?;
            let kind = rec[17];
            let id = (first + i) as u32;
            match kind {
                b'n' => entries.push(Entry::InUse {
                    id,
                    gen: field2 as u16,
                    offset: field1,
                }),
                b'f' => {
                    // Free — drop.
                }
                _ => return Err(Error::MalformedXref),
            }
        }
    }
    // After `trailer` keyword: parse the dict literal looking
    // for /Prev. We tokenise the remainder via the lexer (the
    // trailer dict is small and lexer-clean).
    let prev = read_prev_from_classical_trailer(&input[p..])?;
    Ok((entries, prev))
}

fn read_prev_from_classical_trailer(span: &[u8]) -> Result<Option<u64>, Error> {
    // The trailer dict is the next `<<...>>` literal in `span`.
    // We lex the whole span and pull /Prev out of the dict.
    let toks = lex(span);
    let mut cursor = TokenCursor::new(&toks);
    // Find the `<<` opening.
    while let Some(t) = cursor.peek() {
        if matches!(t.kind, TokenKind::DictOpen) {
            break;
        }
        cursor.advance();
    }
    if cursor.peek().is_none() {
        return Ok(None);
    }
    // Walk dict entries looking for /Prev. We don't need a
    // full value parser — /Prev's value is always an Integer.
    cursor.advance(); // consume `<<`
    let mut depth = 1i32;
    let mut prev: Option<u64> = None;
    while let Some(t) = cursor.peek() {
        match t.kind {
            TokenKind::DictClose => {
                depth -= 1;
                cursor.advance();
                if depth == 0 {
                    break;
                }
            }
            TokenKind::DictOpen => {
                depth += 1;
                cursor.advance();
            }
            TokenKind::Name if depth == 1 && t.bytes == b"/Prev" => {
                cursor.advance();
                if let Some(v) = cursor.peek() {
                    if matches!(v.kind, TokenKind::Number) {
                        if let Ok(s) = std::str::from_utf8(v.bytes) {
                            if let Ok(n) = s.parse::<u64>() {
                                prev = Some(n);
                            }
                        }
                    }
                    cursor.advance();
                }
            }
            _ => cursor.advance(),
        }
    }
    Ok(prev)
}

// ------------------------------------------------------------
// /XRef stream (§7.5.8)
// ------------------------------------------------------------

fn parse_xref_stream(
    input: &[u8],
    off: usize,
) -> Result<(Vec<Entry>, Option<u64>), Error> {
    // Indirect-object header: `<id> <gen> obj`. Skip past it.
    let mut p = off;
    skip_ws_and_eol(input, &mut p);
    let _id = read_decimal(input, &mut p).ok_or(Error::MalformedXrefStream)?;
    skip_inline_ws(input, &mut p);
    let _gen = read_decimal(input, &mut p).ok_or(Error::MalformedXrefStream)?;
    skip_inline_ws(input, &mut p);
    if p + 3 > input.len() || &input[p..p + 3] != b"obj" {
        return Err(Error::MalformedXrefStream);
    }
    p += 3;
    skip_ws_and_eol(input, &mut p);

    // Tokenise from here through the next `endobj` and pull the
    // dict + stream payload out.
    if p + 2 > input.len() || &input[p..p + 2] != b"<<" {
        return Err(Error::MalformedXrefStream);
    }
    // Find `>>` matching the opening `<<`.
    let dict_end = find_matching_dict_close(input, p)
        .ok_or(Error::MalformedXrefStream)?;
    let dict_bytes = &input[p..dict_end];
    let mut q = dict_end;
    // Skip whitespace, then `stream` keyword + EOL marker.
    skip_inline_ws(input, &mut q);
    skip_one_eol(input, &mut q);
    if q + 6 > input.len() || &input[q..q + 6] != b"stream" {
        return Err(Error::MalformedXrefStream);
    }
    q += 6;
    // PDF §7.3.8.1: the `stream` keyword is followed by EOL —
    // either CRLF or LF (NOT a bare CR).
    if q < input.len() && input[q] == b'\r' {
        q += 1;
    }
    if q < input.len() && input[q] == b'\n' {
        q += 1;
    }
    let stream_start = q;

    // Parse dict for /Length, /W, /Index, /Prev, /Size, /Filter.
    let dict = parse_xref_stream_dict(dict_bytes)?;
    let length = dict.length.ok_or(Error::MalformedXrefStream)? as usize;
    if !dict.flate {
        return Err(Error::MissingFlateFilter);
    }
    if stream_start + length > input.len() {
        return Err(Error::MalformedXrefStream);
    }
    let payload = &input[stream_start..stream_start + length];

    // FlateDecode the payload.
    let decoded = flate::decode(payload)
        .map_err(|_| Error::FlateDecodeFailed)?;

    // Unpack binary entries per /W and /Index.
    let entries = unpack_xref_stream(&decoded, &dict)?;

    Ok((entries, dict.prev))
}

#[derive(Debug, Default)]
struct XrefStreamDict {
    length: Option<u64>,
    w: [u8; 3],
    /// Pairs of (first_id, count). When /Index is absent, the
    /// implied default is `[0, /Size]` per §7.5.8.3.
    index: Vec<(u32, u32)>,
    size: Option<u32>,
    prev: Option<u64>,
    flate: bool,
}

fn parse_xref_stream_dict(bytes: &[u8]) -> Result<XrefStreamDict, Error> {
    let toks = lex(bytes);
    let mut cursor = TokenCursor::new(&toks);
    // Skip past the leading `<<`.
    if !matches!(cursor.peek().map(|t| t.kind), Some(TokenKind::DictOpen)) {
        return Err(Error::MalformedXrefStream);
    }
    cursor.advance();
    let mut out = XrefStreamDict::default();
    let mut depth = 1i32;
    while let Some(t) = cursor.peek() {
        match t.kind {
            TokenKind::DictClose => {
                depth -= 1;
                cursor.advance();
                if depth == 0 {
                    break;
                }
            }
            TokenKind::DictOpen => {
                depth += 1;
                cursor.advance();
            }
            TokenKind::Name if depth == 1 => {
                let key = t.bytes;
                cursor.advance();
                match key {
                    b"/Length" => {
                        out.length = read_int_token(&mut cursor).map(|n| n as u64);
                    }
                    b"/Size" => {
                        out.size = read_int_token(&mut cursor).map(|n| n as u32);
                    }
                    b"/Prev" => {
                        out.prev = read_int_token(&mut cursor).map(|n| n as u64);
                    }
                    b"/W" => {
                        let nums = read_int_array(&mut cursor)?;
                        if nums.len() != 3 {
                            return Err(Error::MalformedXrefStream);
                        }
                        out.w = [
                            nums[0] as u8,
                            nums[1] as u8,
                            nums[2] as u8,
                        ];
                    }
                    b"/Index" => {
                        let nums = read_int_array(&mut cursor)?;
                        if nums.len() % 2 != 0 {
                            return Err(Error::MalformedXrefStream);
                        }
                        for pair in nums.chunks(2) {
                            out.index.push((pair[0] as u32, pair[1] as u32));
                        }
                    }
                    b"/Filter" => {
                        // Either a single name `/FlateDecode` or
                        // a 1-element array containing it.
                        if let Some(nx) = cursor.peek() {
                            match nx.kind {
                                TokenKind::Name => {
                                    if nx.bytes == b"/FlateDecode" {
                                        out.flate = true;
                                    }
                                    cursor.advance();
                                }
                                TokenKind::ArrayOpen => {
                                    cursor.advance();
                                    while let Some(it) = cursor.peek() {
                                        if matches!(it.kind, TokenKind::ArrayClose) {
                                            cursor.advance();
                                            break;
                                        }
                                        if matches!(it.kind, TokenKind::Name)
                                            && it.bytes == b"/FlateDecode"
                                        {
                                            out.flate = true;
                                        }
                                        cursor.advance();
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {} // ignore other keys
                }
            }
            _ => cursor.advance(),
        }
    }
    if out.index.is_empty() {
        if let Some(size) = out.size {
            out.index.push((0, size));
        } else {
            return Err(Error::MalformedXrefStream);
        }
    }
    Ok(out)
}

fn read_int_token(cursor: &mut TokenCursor<'_, '_>) -> Option<i64> {
    let t = cursor.peek()?;
    if matches!(t.kind, TokenKind::Number) {
        let s = std::str::from_utf8(t.bytes).ok()?;
        let v = s.parse::<i64>().ok();
        cursor.advance();
        v
    } else {
        None
    }
}

fn read_int_array(cursor: &mut TokenCursor<'_, '_>) -> Result<Vec<i64>, Error> {
    let t = cursor.peek().ok_or(Error::MalformedXrefStream)?;
    if !matches!(t.kind, TokenKind::ArrayOpen) {
        return Err(Error::MalformedXrefStream);
    }
    cursor.advance();
    let mut out = Vec::new();
    while let Some(t) = cursor.peek() {
        if matches!(t.kind, TokenKind::ArrayClose) {
            cursor.advance();
            return Ok(out);
        }
        if matches!(t.kind, TokenKind::Number) {
            let s = std::str::from_utf8(t.bytes)
                .map_err(|_| Error::MalformedXrefStream)?;
            let v = s.parse::<i64>().map_err(|_| Error::MalformedXrefStream)?;
            out.push(v);
            cursor.advance();
        } else {
            return Err(Error::MalformedXrefStream);
        }
    }
    Err(Error::MalformedXrefStream)
}

fn unpack_xref_stream(
    decoded: &[u8],
    dict: &XrefStreamDict,
) -> Result<Vec<Entry>, Error> {
    let w0 = dict.w[0] as usize;
    let w1 = dict.w[1] as usize;
    let w2 = dict.w[2] as usize;
    let row = w0 + w1 + w2;
    if row == 0 {
        return Err(Error::MalformedXrefStream);
    }
    let mut entries: Vec<Entry> = Vec::new();
    let mut p = 0usize;
    for &(first_id, count) in &dict.index {
        for k in 0..count {
            if p + row > decoded.len() {
                return Err(Error::MalformedXrefStream);
            }
            // Default type is 1 (uncompressed) when w0 == 0
            // per §7.5.8.3. Otherwise read the type field.
            let t = if w0 == 0 { 1 } else {
                read_be(&decoded[p..p + w0])
            };
            let f2 = read_be(&decoded[p + w0..p + w0 + w1]);
            let f3 = read_be(&decoded[p + w0 + w1..p + row]);
            p += row;
            let id = first_id + k;
            match t {
                0 => {
                    // Free entry — drop.
                }
                1 => {
                    entries.push(Entry::InUse {
                        id,
                        gen: f3 as u16,
                        offset: f2,
                    });
                }
                2 => {
                    entries.push(Entry::Compressed {
                        id,
                        gen: 0,
                        stream_id: f2 as u32,
                        index: f3 as u32,
                    });
                }
                other => {
                    return Err(Error::UnsupportedXrefStreamType(other as u32));
                }
            }
        }
    }
    Ok(entries)
}

fn read_be(bytes: &[u8]) -> u64 {
    let mut v: u64 = 0;
    for &b in bytes {
        v = (v << 8) | b as u64;
    }
    v
}

// ============================================================
// Byte-level helpers
// ============================================================

fn is_pdf_ws(b: u8) -> bool {
    matches!(b, 0x00 | 0x09 | 0x0A | 0x0C | 0x0D | b' ')
}

fn is_pdf_eol(b: u8) -> bool {
    matches!(b, 0x0A | 0x0D)
}

fn skip_inline_ws(input: &[u8], p: &mut usize) {
    while *p < input.len() && (input[*p] == b' ' || input[*p] == b'\t') {
        *p += 1;
    }
}

fn skip_ws_and_eol(input: &[u8], p: &mut usize) {
    while *p < input.len() && is_pdf_ws(input[*p]) {
        *p += 1;
    }
}

fn skip_one_eol(input: &[u8], p: &mut usize) {
    if *p < input.len() && input[*p] == b'\r' {
        *p += 1;
        if *p < input.len() && input[*p] == b'\n' {
            *p += 1;
        }
    } else if *p < input.len() && input[*p] == b'\n' {
        *p += 1;
    }
}

fn read_decimal(input: &[u8], p: &mut usize) -> Option<u64> {
    let start = *p;
    while *p < input.len() && input[*p].is_ascii_digit() {
        *p += 1;
    }
    if *p == start {
        return None;
    }
    std::str::from_utf8(&input[start..*p]).ok()?.parse().ok()
}

fn read_decimal_after(input: &[u8], from: usize) -> Option<u64> {
    let mut p = from;
    skip_ws_and_eol(input, &mut p);
    read_decimal(input, &mut p)
}

fn parse_uint_str(s: &[u8]) -> Result<u64, Error> {
    std::str::from_utf8(s)
        .map_err(|_| Error::MalformedXref)?
        .trim()
        .parse::<u64>()
        .map_err(|_| Error::MalformedXref)
}

fn find_matching_dict_close(input: &[u8], from: usize) -> Option<usize> {
    // Scan from `from` through the byte stream tracking PDF
    // dict / array / string nesting until the original `<<`
    // closes. Caller has already verified `input[from..from+2]
    // == b"<<"`. Returns the position just AFTER the matching
    // `>>`.
    let mut p = from + 2;
    let mut depth = 1i32;
    while p < input.len() && depth > 0 {
        let b = input[p];
        match b {
            b'(' => {
                // Skip a balanced literal string with backslash
                // escapes.
                p += 1;
                let mut sd = 1i32;
                while p < input.len() && sd > 0 {
                    match input[p] {
                        b'\\' => {
                            p = (p + 2).min(input.len());
                            continue;
                        }
                        b'(' => sd += 1,
                        b')' => sd -= 1,
                        _ => {}
                    }
                    p += 1;
                }
            }
            b'<' if p + 1 < input.len() && input[p + 1] == b'<' => {
                depth += 1;
                p += 2;
            }
            b'<' => {
                // Hex string — skip to next `>`.
                p += 1;
                while p < input.len() && input[p] != b'>' {
                    p += 1;
                }
                if p < input.len() {
                    p += 1;
                }
            }
            b'>' if p + 1 < input.len() && input[p + 1] == b'>' => {
                depth -= 1;
                p += 2;
                if depth == 0 {
                    return Some(p);
                }
            }
            _ => {
                p += 1;
            }
        }
    }
    None
}

/// Find the rightmost `kw` keyword token in the last `tail`
/// bytes of `input`. Word-bounded — the bytes adjacent to the
/// match must be whitespace / start-of-input / end-of-input.
fn find_keyword_in_tail(input: &[u8], kw: &[u8], tail: usize) -> Option<usize> {
    let lo = input.len().saturating_sub(tail);
    let span = &input[lo..];
    let kwlen = kw.len();
    if kwlen == 0 || span.len() < kwlen {
        return None;
    }
    let mut last = None;
    for i in 0..=span.len() - kwlen {
        if &span[i..i + kwlen] != kw {
            continue;
        }
        let abs = lo + i;
        let before_ok = abs == 0 || is_pdf_ws(input[abs - 1]);
        let after_ok = abs + kwlen == input.len()
            || is_pdf_ws(input[abs + kwlen]);
        if before_ok && after_ok {
            last = Some(abs);
        }
    }
    last
}

// ============================================================
// Trailer-dict TokenCursor (trivia-transparent)
// ============================================================

struct TokenCursor<'a, 'b> {
    tokens: &'b [Token<'a>],
    pos: usize,
}

impl<'a, 'b> TokenCursor<'a, 'b> {
    fn new(tokens: &'b [Token<'a>]) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&mut self) -> Option<&'b Token<'a>> {
        self.skip_trivia();
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) {
        self.skip_trivia();
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn skip_trivia(&mut self) {
        while let Some(t) = self.tokens.get(self.pos) {
            match t.kind {
                TokenKind::Whitespace
                | TokenKind::Eol
                | TokenKind::Comment => self.pos += 1,
                _ => break,
            }
        }
    }
}

// Suppress dead-code warnings for helpers reserved for clarity.
#[allow(dead_code)]
fn _retain_helpers() {
    let _ = is_pdf_eol(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_canonical(input: &[u8]) -> String {
        let t = parse(input).expect("parse xref");
        String::from_utf8(to_canonical(&t)).expect("utf8 canonical")
    }

    #[test]
    fn classical_minimal() {
        // Same skeleton as Feedstock/PDF/Fixtures/xref/minimal.pdf.
        let pdf = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>\nendobj\nxref\n0 4\n0000000000 65535 f \n0000000015 00000 n \n0000000064 00000 n \n0000000121 00000 n \ntrailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n209\n%%EOF\n";
        assert_eq!(
            parse_canonical(pdf),
            "1 0 inuse 15\n2 0 inuse 64\n3 0 inuse 121\n"
        );
    }

    #[test]
    fn missing_startxref_errors() {
        let pdf = b"junk";
        assert_eq!(parse(pdf).unwrap_err(), Error::MissingStartxref);
    }

    // The two_subsections / xref_stream / objstm / incremental
    // shapes are exercised by the corpus test against the real
    // fixtures under `Feedstock/PDF/Fixtures/xref/`. The byte
    // offsets in those PDFs are produced by the fixture
    // generator, so testing them here would either re-derive
    // those offsets or duplicate the corpus test.
}
