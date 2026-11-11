// Classical-trailer parser (PDF 32000-1:2008 §7.5.5). Pure
// function from raw PDF bytes to a Trailer struct exposing the
// four well-known top-level keys: /Size, /Root, /Info,
// /Encrypt.
//
// Strategy: locate `startxref` near the file tail (last 4 KiB
// — anything earlier might be an embedded `startxref` token in
// a stream / comment, not the file's real trailer landmark).
// Walk backward from startxref to the most recent `trailer`
// keyword; the bytes between are the trailer dict literal +
// trivia. Tokenise via the frozen `foundation::lexer` and pluck
// the four keys.
//
// v1 scope is classical-trailer-only — the /XRef-stream
// trailer form (§7.5.8, used by PDF 1.5+) carries the same
// keys in the xref stream's own dict; the trailer primitive
// errors out when there's no `trailer` keyword preceding
// `startxref`. Adding /XRef-stream support is a v2 superset.

use std::fmt;

use super::lexer::{lex, Token, TokenKind};

/// Errors returned by [`parse`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// No `startxref` keyword in the last 4 KiB of input.
    MissingStartxref,
    /// `startxref` was found but no preceding `trailer`
    /// keyword — likely an /XRef-stream trailer (out of v1
    /// scope) or a malformed file.
    MissingTrailerKeyword,
    /// The trailer dict literal is structurally malformed —
    /// unterminated, missing `<<` / `>>`, or a value position
    /// that doesn't lex as a value.
    MalformedDict,
    /// /Size or /Root is absent from the trailer dict.
    MissingRequiredKey(&'static str),
    /// A recognised key has the wrong value type — e.g. /Size
    /// is not an integer, /Root is not an indirect ref.
    WrongValueType(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MissingStartxref =>
                write!(f, "trailer: missing startxref in last 4 KiB"),
            Error::MissingTrailerKeyword =>
                write!(f, "trailer: missing trailer keyword (xref-stream trailers are out of v1 scope)"),
            Error::MalformedDict =>
                write!(f, "trailer: malformed trailer dict"),
            Error::MissingRequiredKey(k) =>
                write!(f, "trailer: missing required key /{k}"),
            Error::WrongValueType(k) =>
                write!(f, "trailer: wrong value type for /{k}"),
        }
    }
}

impl std::error::Error for Error {}

/// Parsed trailer dict. Ref-valued fields default to 0 when
/// absent. /Size and /Root are required by §7.5.5; absence
/// raises [`Error::MissingRequiredKey`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Trailer {
    pub size: i64,
    pub root_id: u32,
    pub info_id: u32,
    pub encrypt_id: u32,
    /// First element of the `/ID` array (permanent file identifier),
    /// decoded to raw bytes. `None` when `/ID` is absent.
    pub id_original: Option<Vec<u8>>,
    /// Second element of the `/ID` array (changing file identifier),
    /// decoded to raw bytes. `None` when `/ID` is absent.
    pub id_modified: Option<Vec<u8>>,
}

const TAIL_SCAN: usize = 4096;

/// Parse `input` (raw PDF bytes) into a [`Trailer`].
pub fn parse(input: &[u8]) -> Result<Trailer, Error> {
    let startxref_at = find_keyword_in_tail(input, b"startxref", TAIL_SCAN)
        .ok_or(Error::MissingStartxref)?;
    let trailer_at = find_keyword_before(input, b"trailer", startxref_at)
        .ok_or(Error::MissingTrailerKeyword)?;

    // Span between the trailer keyword (exclusive) and
    // startxref (exclusive) is the dict literal + trivia.
    let body_start = trailer_at + b"trailer".len();
    let body = &input[body_start..startxref_at];

    let tokens = lex(body);
    let mut cursor = TokenCursor::new(&tokens);
    let dict_entries = parse_dict(&mut cursor)?;

    let mut t = Trailer::default();
    let mut have_size = false;
    let mut have_root = false;
    for (k, v) in &dict_entries {
        match k.as_slice() {
            b"Size" => {
                t.size = match v {
                    DictValue::Integer(n) => *n,
                    _ => return Err(Error::WrongValueType("Size")),
                };
                have_size = true;
            }
            b"Root" => {
                t.root_id = match v {
                    DictValue::Ref(id, _) => *id,
                    _ => return Err(Error::WrongValueType("Root")),
                };
                have_root = true;
            }
            b"Info" => {
                t.info_id = match v {
                    DictValue::Ref(id, _) => *id,
                    _ => return Err(Error::WrongValueType("Info")),
                };
            }
            b"Encrypt" => {
                t.encrypt_id = match v {
                    DictValue::Ref(id, _) => *id,
                    _ => return Err(Error::WrongValueType("Encrypt")),
                };
            }
            b"ID" => {
                if let DictValue::Array(items) = v {
                    let mut strs = items.iter().filter_map(|e| match e {
                        DictValue::StringBytes(b) => Some(b.clone()),
                        _ => None,
                    });
                    t.id_original = strs.next();
                    t.id_modified = strs.next();
                }
            }
            _ => {} // ignore /Prev, /XRefStm, custom keys
        }
    }
    if !have_size {
        return Err(Error::MissingRequiredKey("Size"));
    }
    if !have_root {
        return Err(Error::MissingRequiredKey("Root"));
    }
    Ok(t)
}

/// Emit the freeze-gate canonical form: one line per emitted
/// field, in fixed order (size, root, info, encrypt). Optional
/// fields (info / encrypt) are omitted when their id is 0.
#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn to_canonical(t: &Trailer) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(format!("size {}\n", t.size).as_bytes());
    out.extend_from_slice(format!("root {}\n", t.root_id).as_bytes());
    if t.info_id != 0 {
        out.extend_from_slice(format!("info {}\n", t.info_id).as_bytes());
    }
    if t.encrypt_id != 0 {
        out.extend_from_slice(
            format!("encrypt {}\n", t.encrypt_id).as_bytes());
    }
    out
}

// ============================================================
// Keyword-byte scan over raw input (Lexer-free)
// ============================================================

/// Find the rightmost occurrence of `kw` (as a delimited
/// keyword token) in the last `tail` bytes of `input`. We
/// require word boundaries on both sides — a match in the
/// middle of `Sstartxrefs` would not register.
fn find_keyword_in_tail(input: &[u8], kw: &[u8], tail: usize) -> Option<usize> {
    let lo = input.len().saturating_sub(tail);
    find_keyword_in_range(input, kw, lo, input.len())
        .map(|positions| *positions.last().unwrap())
}

fn find_keyword_before(input: &[u8], kw: &[u8], end: usize) -> Option<usize> {
    find_keyword_in_range(input, kw, 0, end)
        .map(|positions| *positions.last().unwrap())
}

fn find_keyword_in_range(
    input: &[u8],
    kw: &[u8],
    lo: usize,
    hi: usize,
) -> Option<Vec<usize>> {
    let mut hits = Vec::new();
    let span = &input[lo..hi];
    let kwlen = kw.len();
    if kwlen == 0 || span.len() < kwlen {
        return None;
    }
    for i in 0..=span.len() - kwlen {
        if &span[i..i + kwlen] != kw {
            continue;
        }
        let abs = lo + i;
        // Word-boundary check: the byte just before must be
        // whitespace / EOL / start-of-input; the byte just
        // after must be whitespace / EOL / end-of-input.
        let before_ok = abs == 0 || is_pdf_ws(input[abs - 1]);
        let after_ok = abs + kwlen == input.len()
            || is_pdf_ws(input[abs + kwlen]);
        if before_ok && after_ok {
            hits.push(abs);
        }
    }
    if hits.is_empty() { None } else { Some(hits) }
}

fn is_pdf_ws(b: u8) -> bool {
    // PDF §7.2.3 whitespace: NUL, TAB, LF, FF, CR, SP.
    matches!(b, 0x00 | 0x09 | 0x0A | 0x0C | 0x0D | b' ')
}

// ============================================================
// Tiny dict-literal parser (operating on lexer tokens)
// ============================================================

#[derive(Debug, Clone)]
enum DictValue {
    Null,
    #[allow(dead_code)] // retained for a future-phase public-API binding
    Bool(bool),
    Integer(i64),
    #[allow(dead_code)] // retained for a future-phase public-API binding
    Real(f64),
    #[allow(dead_code)] // retained for a future-phase public-API binding
    Name(Vec<u8>),
    #[allow(dead_code)] // retained for a future-phase public-API binding
    StringBytes(Vec<u8>),
    #[allow(dead_code)] // retained for a future-phase public-API binding
    Array(Vec<DictValue>),
    #[allow(dead_code)] // retained for a future-phase public-API binding
    Dict(Vec<(Vec<u8>, DictValue)>),
    /// (object_id, generation)
    #[allow(dead_code)] // retained for a future-phase public-API binding
    Ref(u32, u32),
}

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

    /// Save / restore for backtracking the indirect-reference
    /// detection (`N M R` triple — needs three-token lookahead
    /// where two of the three are bare numbers + a `R` keyword).
    fn save(&self) -> usize { self.pos }
    fn restore(&mut self, pos: usize) { self.pos = pos; }
}

fn parse_dict(cursor: &mut TokenCursor<'_, '_>) -> Result<Vec<(Vec<u8>, DictValue)>, Error> {
    let tok = cursor.peek().ok_or(Error::MalformedDict)?;
    if !matches!(tok.kind, TokenKind::DictOpen) {
        return Err(Error::MalformedDict);
    }
    cursor.advance();
    let mut entries: Vec<(Vec<u8>, DictValue)> = Vec::new();
    loop {
        let tok = cursor.peek().ok_or(Error::MalformedDict)?;
        if matches!(tok.kind, TokenKind::DictClose) {
            cursor.advance();
            return Ok(entries);
        }
        if !matches!(tok.kind, TokenKind::Name) {
            return Err(Error::MalformedDict);
        }
        let key = decode_name(tok.bytes);
        cursor.advance();
        let value = parse_value(cursor)?;
        entries.push((key, value));
    }
}

fn parse_value(cursor: &mut TokenCursor<'_, '_>) -> Result<DictValue, Error> {
    let tok = cursor.peek().ok_or(Error::MalformedDict)?;
    match tok.kind {
        TokenKind::Number => {
            // Indirect-reference detection: `N M R` triple.
            // The two leading numbers must be non-negative
            // integers; the trailing keyword must be exactly
            // the byte `R`.
            let saved = cursor.save();
            let first_bytes = tok.bytes;
            cursor.advance();
            let second = cursor.peek();
            let is_int_first = !contains_dot(first_bytes);
            let mut ref_pair: Option<(u32, u32)> = None;
            if is_int_first {
                if let Some(t2) = second {
                    if matches!(t2.kind, TokenKind::Number)
                        && !contains_dot(t2.bytes)
                    {
                        let second_bytes = t2.bytes;
                        cursor.advance();
                        if let Some(t3) = cursor.peek() {
                            if matches!(t3.kind, TokenKind::Keyword)
                                && t3.bytes == b"R"
                            {
                                let id_str =
                                    std::str::from_utf8(first_bytes)
                                        .map_err(|_| Error::MalformedDict)?;
                                let gen_str =
                                    std::str::from_utf8(second_bytes)
                                        .map_err(|_| Error::MalformedDict)?;
                                let id: u32 = id_str.parse()
                                    .map_err(|_| Error::MalformedDict)?;
                                let gen: u32 = gen_str.parse()
                                    .map_err(|_| Error::MalformedDict)?;
                                cursor.advance();
                                ref_pair = Some((id, gen));
                            }
                        }
                    }
                }
            }
            if let Some((id, gen)) = ref_pair {
                return Ok(DictValue::Ref(id, gen));
            }
            // Not an indirect ref — restore and parse the
            // single number atomically.
            cursor.restore(saved);
            let tok = cursor.peek().ok_or(Error::MalformedDict)?;
            cursor.advance();
            parse_number(tok.bytes)
        }
        TokenKind::Name => {
            let bytes = decode_name(tok.bytes);
            cursor.advance();
            Ok(DictValue::Name(bytes))
        }
        TokenKind::StringLiteral => {
            let bytes = decode_string_literal(tok.bytes);
            cursor.advance();
            Ok(DictValue::StringBytes(bytes))
        }
        TokenKind::StringHex => {
            let bytes = decode_string_hex(tok.bytes)?;
            cursor.advance();
            Ok(DictValue::StringBytes(bytes))
        }
        TokenKind::ArrayOpen => {
            cursor.advance();
            let mut items = Vec::new();
            loop {
                let t = cursor.peek().ok_or(Error::MalformedDict)?;
                if matches!(t.kind, TokenKind::ArrayClose) {
                    cursor.advance();
                    return Ok(DictValue::Array(items));
                }
                items.push(parse_value(cursor)?);
            }
        }
        TokenKind::DictOpen => Ok(DictValue::Dict(parse_dict(cursor)?)),
        TokenKind::Keyword => {
            match tok.bytes {
                b"true" => {
                    cursor.advance();
                    Ok(DictValue::Bool(true))
                }
                b"false" => {
                    cursor.advance();
                    Ok(DictValue::Bool(false))
                }
                b"null" => {
                    cursor.advance();
                    Ok(DictValue::Null)
                }
                _ => Err(Error::MalformedDict),
            }
        }
        _ => Err(Error::MalformedDict),
    }
}

fn parse_number(bytes: &[u8]) -> Result<DictValue, Error> {
    let s = std::str::from_utf8(bytes).map_err(|_| Error::MalformedDict)?;
    if s.contains('.') {
        let v: f64 = s.parse().map_err(|_| Error::MalformedDict)?;
        Ok(DictValue::Real(v))
    } else {
        let v: i64 = s.parse().map_err(|_| Error::MalformedDict)?;
        Ok(DictValue::Integer(v))
    }
}

fn contains_dot(bytes: &[u8]) -> bool {
    bytes.contains(&b'.')
}

fn decode_name(bytes: &[u8]) -> Vec<u8> {
    let inner = &bytes[1..]; // drop '/'
    let mut out = Vec::with_capacity(inner.len());
    let mut i = 0;
    while i < inner.len() {
        if inner[i] == b'#' && i + 2 < inner.len() {
            if let (Some(h), Some(l)) =
                (hex_val(inner[i + 1]), hex_val(inner[i + 2]))
            {
                out.push((h << 4) | l);
                i += 3;
                continue;
            }
        }
        out.push(inner[i]);
        i += 1;
    }
    out
}

fn decode_string_literal(bytes: &[u8]) -> Vec<u8> {
    let inner = &bytes[1..bytes.len() - 1];
    let mut out = Vec::with_capacity(inner.len());
    let mut i = 0;
    while i < inner.len() {
        let b = inner[i];
        if b == b'\\' && i + 1 < inner.len() {
            i += 1;
            match inner[i] {
                b'n' => { out.push(b'\n'); i += 1; }
                b'r' => { out.push(b'\r'); i += 1; }
                b't' => { out.push(b'\t'); i += 1; }
                b'b' => { out.push(0x08); i += 1; }
                b'f' => { out.push(0x0C); i += 1; }
                b'(' => { out.push(b'('); i += 1; }
                b')' => { out.push(b')'); i += 1; }
                b'\\' => { out.push(b'\\'); i += 1; }
                b'\r' => {
                    i += 1;
                    if i < inner.len() && inner[i] == b'\n' { i += 1; }
                }
                b'\n' => { i += 1; }
                d if (b'0'..=b'7').contains(&d) => {
                    let mut val: u32 = 0;
                    let mut k = 0;
                    while k < 3
                        && i < inner.len()
                        && (b'0'..=b'7').contains(&inner[i])
                    {
                        val = val * 8 + (inner[i] - b'0') as u32;
                        i += 1;
                        k += 1;
                    }
                    out.push(val as u8);
                }
                other => { out.push(other); i += 1; }
            }
        } else {
            out.push(b);
            i += 1;
        }
    }
    out
}

fn decode_string_hex(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    let inner = &bytes[1..bytes.len() - 1];
    let mut nibbles: Vec<u8> = Vec::with_capacity(inner.len());
    for &b in inner {
        if let Some(v) = hex_val(b) {
            nibbles.push(v);
        } else if matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0C | 0x00) {
            continue;
        } else {
            return Err(Error::MalformedDict);
        }
    }
    if nibbles.len() % 2 == 1 {
        nibbles.push(0);
    }
    Ok(nibbles.chunks(2).map(|c| (c[0] << 4) | c[1]).collect())
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical_for(input: &[u8]) -> Vec<u8> {
        let t = parse(input).expect("parse trailer");
        to_canonical(&t)
    }

    #[test]
    fn minimal_size_root_only() {
        let pdf = b"%PDF-1.4\n%fake body\ntrailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n123\n%%EOF\n";
        assert_eq!(canonical_for(pdf), b"size 4\nroot 1\n");
    }

    #[test]
    fn with_info_and_encrypt() {
        let pdf = b"%PDF-1.4\nbody\ntrailer\n<< /Size 9 /Root 1 0 R /Info 4 0 R /Encrypt 7 0 R >>\nstartxref\n0\n%%EOF\n";
        assert_eq!(
            canonical_for(pdf),
            b"size 9\nroot 1\ninfo 4\nencrypt 7\n"
        );
    }

    #[test]
    fn missing_size_errors() {
        let pdf = b"trailer\n<< /Root 1 0 R >>\nstartxref\n0\n%%EOF\n";
        assert_eq!(parse(pdf).unwrap_err(),
                   Error::MissingRequiredKey("Size"));
    }

    #[test]
    fn missing_root_errors() {
        let pdf = b"trailer\n<< /Size 4 >>\nstartxref\n0\n%%EOF\n";
        assert_eq!(parse(pdf).unwrap_err(),
                   Error::MissingRequiredKey("Root"));
    }

    #[test]
    fn missing_startxref_errors() {
        let pdf = b"trailer\n<< /Size 4 /Root 1 0 R >>\n%%EOF\n";
        assert_eq!(parse(pdf).unwrap_err(), Error::MissingStartxref);
    }

    #[test]
    fn missing_trailer_keyword_errors() {
        // xref-stream-style file with no `trailer` keyword.
        let pdf = b"%PDF-1.5\nbody\nstartxref\n0\n%%EOF\n";
        assert_eq!(parse(pdf).unwrap_err(),
                   Error::MissingTrailerKeyword);
    }

    #[test]
    fn extra_keys_ignored() {
        let pdf = b"trailer\n<< /Size 4 /Root 1 0 R /ID [<a><b>] /Prev 100 >>\nstartxref\n0\n%%EOF\n";
        assert_eq!(canonical_for(pdf), b"size 4\nroot 1\n");
    }

    #[test]
    fn most_recent_trailer_wins() {
        // Linearised / incremental file with two trailers — the
        // primitive returns the LAST one (closest to file tail).
        let pdf = b"trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n0\n%%EOF\nbody\ntrailer\n<< /Size 9 /Root 2 0 R >>\nstartxref\n100\n%%EOF\n";
        assert_eq!(canonical_for(pdf), b"size 9\nroot 2\n");
    }
}
