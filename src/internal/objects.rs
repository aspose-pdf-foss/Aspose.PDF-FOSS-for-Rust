// PDF indirect-object parser (PDF 32000-1:2008 §7.3 + §7.3.10).
// Pure function from raw PDF bytes + a byte offset (the start
// of an `N M obj` header) to a typed [`IndirectObject`].
// Composes the frozen lexer; consumers (xref + the freeze-gate
// driver) iterate in-use xref entries and call this once per
// entry.
//
// Scope is direct values only — `N M R` indirect references
// are opaque. Streams carry their body bytes inline as `Vec<u8>`
// — parser allocates from the source slice (consumers always
// copy into owned buffers for filter chains anyway). Matches
// the shared spec yaml's `stream {dict, body}` shape (v1.1).

use std::fmt;

use super::lexer::{lex, Token, TokenKind};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Integer(i64),
    /// Real-typed number — carries a "real" tag even when
    /// mathematically integer, so the canonical emitter
    /// preserves the type distinction (`5.0` vs `5`).
    Real(f64),
    Name(Vec<u8>),
    String(Vec<u8>),
    Array(Vec<Value>),
    Dict(Vec<(Vec<u8>, Value)>),
    Ref { id: u32, gen: u16 },
    Stream {
        dict: Vec<(Vec<u8>, Value)>,
        body: Vec<u8>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndirectObject {
    pub id: u32,
    pub generation: u16,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    MalformedHeader,
    MalformedValue,
    UnterminatedArray,
    UnterminatedDict,
    DuplicateDictKey,
    MissingEndobj,
    StreamWithoutDict,
    StreamLengthMissing,
    MalformedStreamFraming,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MalformedHeader =>
                write!(f, "objects: malformed `N M obj` header"),
            Error::MalformedValue =>
                write!(f, "objects: malformed value"),
            Error::UnterminatedArray =>
                write!(f, "objects: unterminated array"),
            Error::UnterminatedDict =>
                write!(f, "objects: unterminated dict"),
            Error::DuplicateDictKey =>
                write!(f, "objects: duplicate dict key"),
            Error::MissingEndobj =>
                write!(f, "objects: missing `endobj`"),
            Error::StreamWithoutDict =>
                write!(f, "objects: `stream` keyword without preceding dict"),
            Error::StreamLengthMissing =>
                write!(f, "objects: stream dict missing /Length integer"),
            Error::MalformedStreamFraming =>
                write!(f, "objects: malformed stream framing"),
        }
    }
}

impl std::error::Error for Error {}

// ============================================================
// Public API
// ============================================================

/// Parse the indirect object whose `N M obj` header starts at
/// byte `offset` in `input`.
pub fn parse_object(input: &[u8], offset: usize) -> Result<IndirectObject, Error> {
    if offset >= input.len() {
        return Err(Error::MalformedHeader);
    }
    let tokens = lex(&input[offset..]);
    let mut cursor = TokenCursor::new(&tokens, offset);

    // Header: <id> <gen> obj
    let id = cursor.read_uint().ok_or(Error::MalformedHeader)?;
    let gen = cursor.read_uint().ok_or(Error::MalformedHeader)?;
    if !cursor.eat_keyword(b"obj") {
        return Err(Error::MalformedHeader);
    }

    // Body: a single direct value.
    let value = parse_value(&mut cursor)?;

    // Optional stream framing: `stream\n<body>\nendstream`.
    let value = if cursor.peek_keyword_is(b"stream") {
        let dict = match value {
            Value::Dict(entries) => entries,
            _ => return Err(Error::StreamWithoutDict),
        };
        let length = find_length(&dict).ok_or(Error::StreamLengthMissing)?;
        cursor.advance(); // consume `stream`
        // PDF §7.3.8.1: `stream` is followed by EOL — LF or
        // CRLF, never bare CR. The lexer already emitted that
        // EOL as an Eol token (or the `stream` keyword would
        // have been Whitespace-glued). After the lexer the
        // body's byte offset is the absolute position right
        // after the `stream` keyword's EOL terminator.
        let body_offset = cursor.skip_one_eol_after_stream(input)?;
        let body_length = length;
        if body_offset + body_length > input.len() {
            return Err(Error::MalformedStreamFraming);
        }
        // Allocate body bytes from the source slice. Consumers
        // routinely call `.to_vec()` on the lazy view anyway —
        // moving the copy to parse time costs nothing.
        let body = input[body_offset..body_offset + body_length].to_vec();
        // Re-lex the tail (`endstream` / `endobj`) from the byte
        // position right after the length-delimited body. The
        // upfront token stream can be misaligned by binary body
        // bytes (an unbalanced `(` or `<` inside an encrypted /
        // compressed body glues tokens across the body's end), so
        // the tokens past the body are unreliable — re-tokenise
        // from this known-good byte boundary instead.
        let tail_start = body_offset + body_length;
        let tail_tokens = lex(&input[tail_start..]);
        let mut tail = TokenCursor::new(&tail_tokens, tail_start);
        if !tail.eat_keyword(b"endstream") {
            return Err(Error::MalformedStreamFraming);
        }
        if !tail.eat_keyword(b"endobj") {
            return Err(Error::MissingEndobj);
        }
        return Ok(IndirectObject {
            id: id as u32,
            generation: gen as u16,
            value: Value::Stream { dict, body },
        });
    } else {
        value
    };

    if !cursor.eat_keyword(b"endobj") {
        return Err(Error::MissingEndobj);
    }

    Ok(IndirectObject {
        id: id as u32,
        generation: gen as u16,
        value,
    })
}

/// Emit the freeze-gate canonical form: one line.
#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn to_canonical(obj: &IndirectObject) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(format!("{} {} ", obj.id, obj.generation).as_bytes());
    canonicalise_value(&obj.value, &mut out);
    out.push(b'\n');
    out
}

fn find_length(dict: &[(Vec<u8>, Value)]) -> Option<usize> {
    for (k, v) in dict {
        if k == b"Length" {
            if let Value::Integer(n) = v {
                if *n >= 0 {
                    return Some(*n as usize);
                }
            }
        }
    }
    None
}

// ============================================================
// Token cursor
// ============================================================

struct TokenCursor<'a, 'b> {
    tokens: &'b [Token<'a>],
    pos: usize,
    /// Absolute byte offset where the lexer's slice started.
    /// We add this to per-token offsets to recover absolute
    /// file offsets (used by stream framing).
    base: usize,
}

impl<'a, 'b> TokenCursor<'a, 'b> {
    fn new(tokens: &'b [Token<'a>], base: usize) -> Self {
        Self { tokens, pos: 0, base }
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

    fn save(&self) -> usize { self.pos }
    fn restore(&mut self, p: usize) { self.pos = p; }

    fn read_uint(&mut self) -> Option<u64> {
        let t = self.peek()?;
        if !matches!(t.kind, TokenKind::Number) {
            return None;
        }
        let s = std::str::from_utf8(t.bytes).ok()?;
        let v = s.parse::<u64>().ok()?;
        self.advance();
        Some(v)
    }

    fn eat_keyword(&mut self, kw: &[u8]) -> bool {
        let t = match self.peek() {
            Some(t) => t,
            None => return false,
        };
        if matches!(t.kind, TokenKind::Keyword) && t.bytes == kw {
            self.advance();
            true
        } else {
            false
        }
    }

    fn peek_keyword_is(&mut self, kw: &[u8]) -> bool {
        match self.peek() {
            Some(t) => matches!(t.kind, TokenKind::Keyword) && t.bytes == kw,
            None => false,
        }
    }

    /// PDF §7.3.8.1: after the `stream` keyword the lexer has
    /// emitted one Eol or Whitespace+Eol — return the absolute
    /// offset that is the FIRST BYTE AFTER that EOL marker. We
    /// also tolerate a single LF or CRLF after `stream` even
    /// when the lexer's trivia bundling differs across input
    /// shapes.
    fn skip_one_eol_after_stream(&mut self, input: &[u8]) -> Result<usize, Error> {
        // Find the first non-trivia token after `stream` — we
        // need the absolute byte offset between `stream` and
        // that token. Trivia tokens carry contiguous offsets,
        // so we can advance past trivia and read the next
        // token's offset as the body start.
        // BUT — the spec mandates exactly one EOL between
        // `stream` and the body. If the lexer ate a
        // Whitespace + Eol pair as separate tokens, we treat
        // all of them as the EOL marker.
        // The body might actually start mid-trivia (a CRLF is
        // 2 bytes; the lexer emits one Eol token covering both).
        // To keep this simple, we find the absolute offset of
        // the next non-trivia token (or end-of-input) and use
        // that as the body's start.
        // Pre-stream-keyword cursor pos is already past the
        // `stream` keyword.
        let abs_offset = match self.tokens.get(self.pos) {
            Some(t) => {
                // Walk trivia tokens up to ONE EOL, then stop.
                let mut p = self.pos;
                let mut saw_eol = false;
                while let Some(t) = self.tokens.get(p) {
                    match t.kind {
                        TokenKind::Whitespace
                        | TokenKind::Comment => p += 1,
                        TokenKind::Eol => {
                            p += 1;
                            saw_eol = true;
                            break;
                        }
                        _ => break,
                    }
                }
                if !saw_eol {
                    return Err(Error::MalformedStreamFraming);
                }
                // The body offset is the absolute offset of
                // whatever sits at position `p` (next non-EOL
                // trivia or non-trivia). If the lexer is at
                // EOF we point past the end (caller will catch).
                self.pos = p;
                self.tokens
                    .get(p)
                    .map(|t| self.base + t.offset)
                    .unwrap_or(self.base + t.offset)
            }
            None => return Err(Error::MalformedStreamFraming),
        };
        // Confirm with the input bytes: the byte just before
        // body_offset must be LF (terminator of LF-only or
        // CRLF). Bare-CR is forbidden.
        if abs_offset == 0 {
            return Err(Error::MalformedStreamFraming);
        }
        if input[abs_offset - 1] != b'\n' {
            return Err(Error::MalformedStreamFraming);
        }
        Ok(abs_offset)
    }
}

// ============================================================
// Value parser
// ============================================================

fn parse_value(cursor: &mut TokenCursor<'_, '_>) -> Result<Value, Error> {
    let tok = cursor.peek().ok_or(Error::MalformedValue)?;
    match tok.kind {
        TokenKind::Number => {
            // Possible Ref `<int> <int> R`. Save and probe.
            let saved = cursor.save();
            let first_bytes = tok.bytes;
            let first_is_int = !contains_dot(first_bytes);
            cursor.advance();
            if first_is_int {
                if let Some(t2) = cursor.peek() {
                    if matches!(t2.kind, TokenKind::Number) && !contains_dot(t2.bytes) {
                        let second_bytes = t2.bytes;
                        cursor.advance();
                        if let Some(t3) = cursor.peek() {
                            if matches!(t3.kind, TokenKind::Keyword) && t3.bytes == b"R" {
                                let id = std::str::from_utf8(first_bytes)
                                    .map_err(|_| Error::MalformedValue)?
                                    .parse::<u64>()
                                    .map_err(|_| Error::MalformedValue)?;
                                let gen = std::str::from_utf8(second_bytes)
                                    .map_err(|_| Error::MalformedValue)?
                                    .parse::<u64>()
                                    .map_err(|_| Error::MalformedValue)?;
                                cursor.advance();
                                return Ok(Value::Ref {
                                    id: id as u32,
                                    gen: gen as u16,
                                });
                            }
                        }
                    }
                }
            }
            // Not a Ref — restore and parse the leading number.
            cursor.restore(saved);
            let tok = cursor.peek().ok_or(Error::MalformedValue)?;
            let v = parse_number(tok.bytes)?;
            cursor.advance();
            Ok(v)
        }
        TokenKind::Name => {
            let bytes = decode_name(tok.bytes);
            cursor.advance();
            Ok(Value::Name(bytes))
        }
        TokenKind::StringLiteral => {
            let bytes = decode_string_literal(tok.bytes);
            cursor.advance();
            Ok(Value::String(bytes))
        }
        TokenKind::StringHex => {
            let bytes = decode_string_hex(tok.bytes)?;
            cursor.advance();
            Ok(Value::String(bytes))
        }
        TokenKind::ArrayOpen => {
            cursor.advance();
            parse_array(cursor)
        }
        TokenKind::DictOpen => {
            cursor.advance();
            parse_dict(cursor)
        }
        TokenKind::Keyword => {
            match tok.bytes {
                b"true" => { cursor.advance(); Ok(Value::Bool(true)) }
                b"false" => { cursor.advance(); Ok(Value::Bool(false)) }
                b"null" => { cursor.advance(); Ok(Value::Null) }
                _ => Err(Error::MalformedValue),
            }
        }
        _ => Err(Error::MalformedValue),
    }
}

fn parse_array(cursor: &mut TokenCursor<'_, '_>) -> Result<Value, Error> {
    let mut items = Vec::new();
    loop {
        let tok = cursor.peek().ok_or(Error::UnterminatedArray)?;
        if matches!(tok.kind, TokenKind::ArrayClose) {
            cursor.advance();
            return Ok(Value::Array(items));
        }
        items.push(parse_value(cursor)?);
    }
}

fn parse_dict(cursor: &mut TokenCursor<'_, '_>) -> Result<Value, Error> {
    let mut entries: Vec<(Vec<u8>, Value)> = Vec::new();
    loop {
        let tok = cursor.peek().ok_or(Error::UnterminatedDict)?;
        if matches!(tok.kind, TokenKind::DictClose) {
            cursor.advance();
            return Ok(Value::Dict(entries));
        }
        if !matches!(tok.kind, TokenKind::Name) {
            return Err(Error::MalformedValue);
        }
        let key = decode_name(tok.bytes);
        cursor.advance();
        let value = parse_value(cursor)?;
        // PDF §7.3.7: keys MUST be unique. Reject duplicates.
        if entries.iter().any(|(k, _)| k == &key) {
            return Err(Error::DuplicateDictKey);
        }
        entries.push((key, value));
    }
}

fn parse_number(bytes: &[u8]) -> Result<Value, Error> {
    let s = std::str::from_utf8(bytes).map_err(|_| Error::MalformedValue)?;
    if s.contains('.') {
        let v: f64 = s.parse().map_err(|_| Error::MalformedValue)?;
        Ok(Value::Real(v))
    } else {
        let v: i64 = s.parse().map_err(|_| Error::MalformedValue)?;
        Ok(Value::Integer(v))
    }
}

fn contains_dot(bytes: &[u8]) -> bool {
    bytes.contains(&b'.')
}

// ============================================================
// Token-bytes → typed-value decoders
// ============================================================

fn decode_name(bytes: &[u8]) -> Vec<u8> {
    let inner = &bytes[1..];
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
            return Err(Error::MalformedValue);
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

// ============================================================
// Canonical serialisation
// ============================================================

#[allow(dead_code)] // retained for a future-phase public-API binding
fn canonicalise_value(v: &Value, out: &mut Vec<u8>) {
    match v {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(true) => out.extend_from_slice(b"true"),
        Value::Bool(false) => out.extend_from_slice(b"false"),
        Value::Integer(n) => {
            out.extend_from_slice(format!("{n}").as_bytes());
        }
        Value::Real(x) => {
            // Mathematically-integer reals emit "X.0"; non-
            // integer reals use the shortest-round-trip Display
            // form. Real-tagged values are always formatted
            // with a dot — never as plain integers — so the
            // "real type distinction" survives the round-trip.
            if x.is_finite() && *x == x.trunc() && x.abs() < (i64::MAX as f64) {
                out.extend_from_slice(
                    format!("{}.0", *x as i64).as_bytes());
            } else {
                out.extend_from_slice(format!("{x}").as_bytes());
            }
        }
        Value::Name(bytes) => {
            out.push(b'/');
            for &b in bytes {
                if (0x21..=0x7E).contains(&b)
                    && !is_pdf_delim(b)
                    && b != b'#'
                {
                    out.push(b);
                } else {
                    out.extend_from_slice(format!("#{b:02X}").as_bytes());
                }
            }
        }
        Value::String(bytes) => {
            out.push(b'<');
            for &b in bytes {
                out.extend_from_slice(format!("{b:02X}").as_bytes());
            }
            out.push(b'>');
        }
        Value::Array(items) => {
            out.extend_from_slice(b"[ ");
            for (i, it) in items.iter().enumerate() {
                if i > 0 { out.push(b' '); }
                canonicalise_value(it, out);
            }
            if !items.is_empty() { out.push(b' '); }
            out.push(b']');
        }
        Value::Dict(entries) => {
            emit_dict(entries, out);
        }
        Value::Ref { id, gen } => {
            out.extend_from_slice(format!("{id} {gen} R").as_bytes());
        }
        Value::Stream { dict, .. } => {
            out.extend_from_slice(b"stream ");
            emit_dict(dict, out);
        }
    }
}

#[allow(dead_code)] // retained for a future-phase public-API binding
fn emit_dict(entries: &[(Vec<u8>, Value)], out: &mut Vec<u8>) {
    let mut sorted: Vec<&(Vec<u8>, Value)> = entries.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    out.extend_from_slice(b"<< ");
    for (i, (k, v)) in sorted.iter().enumerate() {
        if i > 0 { out.push(b' '); }
        out.push(b'/');
        for &b in k.iter() {
            if (0x21..=0x7E).contains(&b) && !is_pdf_delim(b) && b != b'#' {
                out.push(b);
            } else {
                out.extend_from_slice(format!("#{b:02X}").as_bytes());
            }
        }
        out.push(b' ');
        canonicalise_value(v, out);
    }
    if !sorted.is_empty() { out.push(b' '); }
    out.extend_from_slice(b">>");
}

#[allow(dead_code)] // retained for a future-phase public-API binding
fn is_pdf_delim(b: u8) -> bool {
    matches!(b, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical(input: &[u8], offset: usize) -> String {
        let obj = parse_object(input, offset).expect("parse");
        String::from_utf8(to_canonical(&obj)).expect("utf8")
    }

    #[test]
    fn integer_object() {
        let pdf = b"7 0 obj 42 endobj";
        assert_eq!(canonical(pdf, 0), "7 0 42\n");
    }

    #[test]
    fn real_with_integer_value_keeps_decimal() {
        let pdf = b"1 0 obj 5.0 endobj";
        assert_eq!(canonical(pdf, 0), "1 0 5.0\n");
    }

    #[test]
    fn negative_integer() {
        let pdf = b"2 0 obj -17 endobj";
        assert_eq!(canonical(pdf, 0), "2 0 -17\n");
    }

    #[test]
    fn name_with_hex_escape() {
        let pdf = b"3 0 obj /A#20B endobj";
        // /A B — space is decoded but emitted as #20 in canonical
        // (space is outside 0x21..0x7E inside a name — the
        // canonical re-escapes it).
        assert_eq!(canonical(pdf, 0), "3 0 /A#20B\n");
    }

    #[test]
    fn literal_string_decodes_to_hex() {
        let pdf = b"4 0 obj (Hello) endobj";
        assert_eq!(canonical(pdf, 0), "4 0 <48656C6C6F>\n");
    }

    #[test]
    fn hex_string_round_trips() {
        let pdf = b"5 0 obj <48656C6C6F> endobj";
        assert_eq!(canonical(pdf, 0), "5 0 <48656C6C6F>\n");
    }

    #[test]
    fn array_with_ref() {
        let pdf = b"6 0 obj [1 2 0 R 3] endobj";
        assert_eq!(canonical(pdf, 0), "6 0 [ 1 2 0 R 3 ]\n");
    }

    #[test]
    fn dict_keys_sorted() {
        let pdf = b"7 0 obj << /B 2 /A 1 >> endobj";
        assert_eq!(canonical(pdf, 0), "7 0 << /A 1 /B 2 >>\n");
    }

    #[test]
    fn duplicate_key_rejected() {
        let pdf = b"8 0 obj << /A 1 /A 2 >> endobj";
        assert_eq!(parse_object(pdf, 0).unwrap_err(), Error::DuplicateDictKey);
    }

    // ---- /Length stream-framing discrimination (mirrors the C++
    //      objects_length_discrimination regression test). ----

    fn stream_body(pdf: &[u8]) -> Vec<u8> {
        match parse_object(pdf, 0).unwrap().value {
            Value::Stream { body, .. } => body,
            other => panic!("expected a stream, got {other:?}"),
        }
    }

    #[test]
    fn length_zero_returns_empty_stream() {
        let pdf = b"1 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n";
        assert_eq!(stream_body(pdf).len(), 0);
    }

    #[test]
    fn length_positive_unchanged() {
        let pdf = b"1 0 obj\n<< /Length 5 >>\nstream\nABCDE\nendstream\nendobj\n";
        assert_eq!(stream_body(pdf), b"ABCDE");
    }

    #[test]
    fn missing_length_errors() {
        let pdf = b"1 0 obj\n<< /Type /Foo >>\nstream\n\nendstream\nendobj\n";
        assert_eq!(parse_object(pdf, 0).unwrap_err(), Error::StreamLengthMissing);
    }

    #[test]
    fn negative_length_errors() {
        // A negative /Length is rejected (find_length only accepts n >= 0).
        let pdf = b"1 0 obj\n<< /Length -1 >>\nstream\n\nendstream\nendobj\n";
        assert_eq!(parse_object(pdf, 0).unwrap_err(), Error::StreamLengthMissing);
    }
}
