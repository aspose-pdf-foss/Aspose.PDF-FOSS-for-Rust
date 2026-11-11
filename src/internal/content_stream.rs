// Content-stream parser (PDF 32000-1:2008 §7.8 + §8 + §9 + §10).
// Pure function from raw content-stream bytes (already
// decompressed / filter-decoded by the caller) to an ordered
// Operations list, plus a canonical text serialisation used by
// the freeze gate.
//
// PDF content streams are RPN: every operator follows its
// operands. The parse loop maintains a scratch operand stack —
// non-keyword tokens push values onto it; keyword tokens pop
// the whole stack as their operand vector and emit one
// Operation. After each emission the stack is cleared. At
// end-of-input the stack must be empty; a non-empty stack means
// dangling operands without an operator (malformed input).
//
// Value grammar inlined here covers the direct-value subset
// content streams use (numbers, names, strings, arrays, dicts,
// booleans, null). Indirect-object framing and the Stream
// variant cannot appear inside a content stream per §7.8, so
// they are out of scope. When the objects primitive lands on
// the rust track this value parser moves out; the canonical
// rules are identical either way and the gate does not care.

use std::fmt;

use super::lexer::{lex, Token, TokenKind};

/// One direct PDF value as it appears in a content-stream
/// operand position.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Integer(i64),
    /// Real-typed number — flagged by the presence of a `.` in
    /// the source token. The boolean is true if the source had
    /// a fractional component (so `0.0` round-trips as `0.0`,
    /// not `0`).
    Real(f64),
    /// Name — the leading `/` is dropped, escapes resolved.
    Name(Vec<u8>),
    /// String — both `(literal)` and `<hex>` source kinds
    /// decode to bytes here; the canonical form re-emits as hex.
    String(Vec<u8>),
    Array(Vec<Value>),
    /// Dict entries in source order. Canonicalisation sorts by
    /// key bytes; insertion order is preserved otherwise.
    Dict(Vec<(Vec<u8>, Value)>),
    /// An inline image, carried whole: `header` is the parameter
    /// dictionary authored between `BI` and `ID` with its abbreviated
    /// keys verbatim (`W`, `H`, `BPC`, `CS`, `F`, …), `body` the
    /// sample bytes. Only the `INLINE_IMAGE` operation carries this.
    Stream { header: Vec<(Vec<u8>, Value)>, body: Vec<u8> },
}

/// The operator an inline image surfaces as. `BI` / `ID` / `EI` do not
/// reach the operation list separately — the whole image is one
/// operation whose single operand is a [`Value::Stream`].
pub const INLINE_IMAGE: &str = "INLINE_IMAGE";

/// One PDF content-stream operation: an operator string plus
/// its operand list in source order. `op` carries the operator's
/// raw PDF text (`m`, `Tj`, `Tf`, `cm`, `BDC`, `BMC`, …); no
/// operand-count or category validation is applied at parse
/// time per the anchor's intent.
#[derive(Debug, Clone, PartialEq)]
pub struct Operation {
    pub op: String,
    pub operands: Vec<Value>,
}

/// Errors returned by [`parse`].
///
/// Content-stream parsing is non-semantic and deliberately lenient:
/// structurally malformed input is repaired in place rather than
/// failing the stream, since failing would drop every operation a
/// renderer could still draw. Only a token whose own bytes cannot be
/// decoded is reported here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    MalformedNumber,
    MalformedHexString,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MalformedNumber => write!(
                f, "content_stream: malformed number"),
            Error::MalformedHexString => write!(
                f, "content_stream: malformed hex string"),
        }
    }
}

impl std::error::Error for Error {}

// ============================================================
// Public API
// ============================================================

/// Parse a content-stream byte buffer into an ordered list of
/// [`Operation`]s. The input must already be filter-decoded;
/// callers strip stream framing + run any FlateDecode /
/// ASCIIHexDecode / etc. chain before calling.
pub fn parse(input: &[u8]) -> Result<Vec<Operation>, Error> {
    // Inline-image samples are not lexable PDF syntax — an unbalanced
    // '(' in them would swallow the rest of the stream as a literal
    // string. Lift them out before lexing and splice them back onto
    // their operation afterwards.
    let (stripped, samples) = strip_inline_image_samples(input);
    let ops = parse_tokens(&stripped)?;
    Ok(fold_inline_images(ops, samples))
}

fn parse_tokens(input: &[u8]) -> Result<Vec<Operation>, Error> {
    let tokens = lex(input);
    let mut cursor = TokenCursor::new(&tokens);
    let mut ops: Vec<Operation> = Vec::new();
    let mut stack: Vec<Value> = Vec::new();

    while let Some(tok) = cursor.peek() {
        if matches!(tok.kind, TokenKind::Keyword) {
            // Booleans + null are literal value keywords; every
            // other keyword is an operator.
            match tok.bytes {
                b"true" => {
                    stack.push(Value::Bool(true));
                    cursor.advance();
                }
                b"false" => {
                    stack.push(Value::Bool(false));
                    cursor.advance();
                }
                b"null" => {
                    stack.push(Value::Null);
                    cursor.advance();
                }
                bytes if is_stray_delimiter(bytes) => {
                    // A run of delimiter bytes (a lone `>` / `]` /
                    // `>>`) cannot spell an operator — operators are
                    // made of regular characters. Drop it without
                    // disturbing the operands already on the stack.
                    cursor.advance();
                }
                _ => {
                    // Operator emit. A non-UTF-8 operator cannot name
                    // any operator, so it is dropped along with the
                    // operands it would have taken.
                    if let Ok(op_text) = std::str::from_utf8(tok.bytes) {
                        let operands = std::mem::take(&mut stack);
                        ops.push(Operation { op: op_text.to_string(), operands });
                    }
                    cursor.advance();
                }
            }
        } else {
            match parse_value(&mut cursor) {
                Some(Ok(v)) => stack.push(v),
                Some(Err(e)) => return Err(e),
                None => break,
            }
        }
    }

    // Operands left over at end of input carry no operation to emit,
    // so they are dropped rather than failing the stream.
    Ok(ops)
}

/// Serialise an [`Operation`] list to its canonical text form,
/// one line per operation, trailing newline after the last
/// operation. An empty list serialises to a zero-byte string.
#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn to_canonical(ops: &[Operation]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for op in ops {
        let mut first = true;
        for v in &op.operands {
            if !first {
                out.push(b' ');
            }
            first = false;
            canonicalise_value(v, &mut out);
        }
        if !op.operands.is_empty() {
            out.push(b' ');
        }
        out.extend_from_slice(op.op.as_bytes());
        out.push(b'\n');
    }
    out
}

// ============================================================
// Token cursor — trivia transparent
// ============================================================

struct TokenCursor<'a, 'b> {
    tokens: &'b [Token<'a>],
    pos: usize,
}

impl<'a, 'b> TokenCursor<'a, 'b> {
    fn new(tokens: &'b [Token<'a>]) -> Self {
        TokenCursor { tokens, pos: 0 }
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


// ============================================================
// Inline images (ISO 32000 §8.9.7)
// ============================================================

fn is_ws(b: u8) -> bool {
    matches!(b, b'\0' | b'\t' | b'\n' | 0x0C | b'\r' | b' ')
}

fn is_delim(b: u8) -> bool {
    matches!(b, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%')
}

fn is_boundary(b: u8) -> bool {
    is_ws(b) || is_delim(b)
}

/// Whether the two-byte keyword at `at` stands alone rather than being
/// part of a longer token.
fn keyword_at(input: &[u8], at: usize, word: &[u8; 2]) -> bool {
    if input.len() < at + 2 || &input[at..at + 2] != word {
        return false;
    }
    let before_ok = at == 0 || is_boundary(input[at - 1]);
    let after_ok = at + 2 == input.len() || is_boundary(input[at + 2]);
    before_ok && after_ok
}

/// Lift every inline image's sample run out of `input`, leaving
/// `ID EI` behind so the remainder lexes cleanly.
///
/// The scan is string- and comment-aware so an `ID` written inside a
/// text-show string is never taken for the operator. Sample bytes are
/// captured from just past the single mandatory whitespace after `ID`
/// up to the terminating `EI`; the byte immediately before `EI` is
/// kept, because §8.9.7 fixes the sample length by the image geometry
/// and a final sample byte may legitimately equal a whitespace code.
fn strip_inline_image_samples(input: &[u8]) -> (Vec<u8>, Vec<Vec<u8>>) {
    let mut out = Vec::with_capacity(input.len());
    let mut samples = Vec::new();
    let mut i = 0;

    while i < input.len() {
        let b = input[i];
        match b {
            b'%' => {
                // Comment runs to the end of the line.
                while i < input.len() && input[i] != b'\n' && input[i] != b'\r' {
                    out.push(input[i]);
                    i += 1;
                }
            }
            b'(' => {
                // Literal string: nested parens, backslash escapes.
                let mut depth = 0usize;
                while i < input.len() {
                    let c = input[i];
                    out.push(c);
                    i += 1;
                    match c {
                        b'\\' => {
                            if i < input.len() {
                                out.push(input[i]);
                                i += 1;
                            }
                        }
                        b'(' => depth += 1,
                        b')' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
            b'<' if input.get(i + 1) != Some(&b'<') => {
                // Hex string.
                while i < input.len() {
                    let c = input[i];
                    out.push(c);
                    i += 1;
                    if c == b'>' {
                        break;
                    }
                }
            }
            b'I' if keyword_at(input, i, b"ID") => {
                out.extend_from_slice(b"ID ");
                i += 2;
                // Trim the single mandatory whitespace after ID.
                if i < input.len() && is_ws(input[i]) {
                    i += 1;
                }
                let start = i;
                let end = find_inline_image_end(input, start);
                samples.push(input[start..end].to_vec());
                out.extend_from_slice(b"EI");
                i = end;
                // Step over the EI that terminated the run.
                while i < input.len() && !keyword_at(input, i, b"EI") {
                    i += 1;
                }
                i = (i + 2).min(input.len());
            }
            _ => {
                out.push(b);
                i += 1;
            }
        }
    }
    (out, samples)
}

/// The end of the sample run that starts at `start`: the offset of the
/// first delimiter-bracketed `EI` after it, or end of input.
fn find_inline_image_end(input: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < input.len() {
        if input[i] == b'E' && keyword_at(input, i, b"EI") {
            return i;
        }
        i += 1;
    }
    input.len()
}

/// Replace each `BI` / `ID` / `EI` run with the single `INLINE_IMAGE`
/// operation the spec calls for. A run that is not shaped that way is
/// left exactly as parsed.
fn fold_inline_images(ops: Vec<Operation>, samples: Vec<Vec<u8>>) -> Vec<Operation> {
    if samples.is_empty() {
        return ops;
    }
    let mut out: Vec<Operation> = Vec::with_capacity(ops.len());
    let mut sample = samples.into_iter();
    let mut i = 0;

    while i < ops.len() {
        let is_run = ops[i].op == "BI"
            && ops.get(i + 1).is_some_and(|o| o.op == "ID")
            && ops.get(i + 2).is_some_and(|o| o.op == "EI");
        if !is_run {
            out.push(ops[i].clone());
            i += 1;
            continue;
        }
        let Some(body) = sample.next() else {
            out.push(ops[i].clone());
            i += 1;
            continue;
        };
        let header = pair_up(&ops[i + 1].operands);
        let body = trim_to_geometry(&header, body);
        out.push(Operation {
            op: INLINE_IMAGE.to_string(),
            operands: vec![Value::Stream { header, body }],
        });
        i += 3;
    }
    out
}

/// The image parameter dictionary, read off the `/Key value` operand
/// run between `BI` and `ID`. An operand that is not a name in key
/// position ends the run.
fn pair_up(operands: &[Value]) -> Vec<(Vec<u8>, Value)> {
    let mut entries = Vec::new();
    let mut i = 0;
    while i + 1 < operands.len() {
        let Value::Name(key) = &operands[i] else { break };
        entries.push((key.clone(), operands[i + 1].clone()));
        i += 2;
    }
    entries
}

fn header_int(header: &[(Vec<u8>, Value)], keys: &[&[u8]]) -> Option<i64> {
    header.iter().find_map(|(k, v)| {
        if !keys.contains(&k.as_slice()) {
            return None;
        }
        match v {
            Value::Integer(n) => Some(*n),
            Value::Real(n) => Some(*n as i64),
            _ => None,
        }
    })
}

fn header_has(header: &[(Vec<u8>, Value)], keys: &[&[u8]]) -> bool {
    header.iter().any(|(k, _)| keys.contains(&k.as_slice()))
}

/// Trim an unfiltered sample run to the length §8.9.7 fixes by the
/// image geometry, `ceil(W * components * BPC / 8) * H`. A filtered
/// run's raw length is unknown, so it is left as captured — its
/// decoder self-terminates.
fn trim_to_geometry(header: &[(Vec<u8>, Value)], body: Vec<u8>) -> Vec<u8> {
    if header_has(header, &[b"F", b"Filter"]) {
        return body;
    }
    let Some(width) = header_int(header, &[b"W", b"Width"]) else { return body };
    let Some(height) = header_int(header, &[b"H", b"Height"]) else { return body };
    if width <= 0 || height <= 0 {
        return body;
    }

    let is_mask = matches!(
        header.iter().find(|(k, _)| k.as_slice() == b"IM" || k.as_slice() == b"ImageMask"),
        Some((_, Value::Bool(true)))
    );
    let bpc = if is_mask {
        1
    } else {
        header_int(header, &[b"BPC", b"BitsPerComponent"]).unwrap_or(8)
    };
    if bpc <= 0 {
        return body;
    }
    let components = if is_mask { 1 } else { color_components(header) };

    let row_bits = width * components * bpc;
    let row_bytes = row_bits.div_euclid(8) + i64::from(row_bits.rem_euclid(8) != 0);
    let Ok(len) = usize::try_from(row_bytes * height) else { return body };
    if body.len() > len {
        let mut trimmed = body;
        trimmed.truncate(len);
        trimmed
    } else {
        body
    }
}

/// Samples per pixel for the inline image's colour space, taking the
/// abbreviated names §8.9.7 allows alongside the full ones.
fn color_components(header: &[(Vec<u8>, Value)]) -> i64 {
    let space = header.iter().find_map(|(k, v)| {
        if k.as_slice() != b"CS" && k.as_slice() != b"ColorSpace" {
            return None;
        }
        match v {
            Value::Name(n) => Some(n.clone()),
            // An indexed space is written as an array; its samples are
            // single-component indices.
            Value::Array(_) => Some(b"I".to_vec()),
            _ => None,
        }
    });
    match space.as_deref() {
        Some(b"RGB") | Some(b"DeviceRGB") | Some(b"CalRGB") => 3,
        Some(b"CMYK") | Some(b"DeviceCMYK") => 4,
        // DeviceGray, an indexed palette, and an unnamed space all
        // carry one sample per pixel.
        _ => 1,
    }
}

// ============================================================
// Value parser
// ============================================================

/// Whether `bytes` is made only of PDF delimiter characters, which no
/// operator can be: operator names are regular characters. This is a
/// lexical test, not a check of operator legality — unknown operators
/// are still accepted unconditionally.
fn is_stray_delimiter(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes
            .iter()
            .all(|b| matches!(b, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'))
}

/// Decode the value at the cursor.
///
/// Returns `None` only at end of input — every other token makes
/// progress, so a value loop cannot spin. A token that cannot open a
/// value (a stray `]` / `>` / `>>`, or a keyword that is not `true` /
/// `false` / `null`, such as the `R` of a malformed indirect
/// reference) is consumed and stands in as a null, which keeps the
/// surrounding operand positions aligned.
fn parse_value(cursor: &mut TokenCursor<'_, '_>) -> Option<Result<Value, Error>> {
    let tok = cursor.peek()?;
    let value = match tok.kind {
        TokenKind::Number => {
            let bytes = tok.bytes;
            cursor.advance();
            match parse_number(bytes) {
                Ok(v) => Ok(v),
                Err(e) => Err(e),
            }
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
            let bytes = tok.bytes;
            cursor.advance();
            match decode_string_hex(bytes) {
                Ok(b) => Ok(Value::String(b)),
                Err(e) => Err(e),
            }
        }
        TokenKind::ArrayOpen => {
            cursor.advance();
            parse_array_body(cursor)
        }
        TokenKind::DictOpen => {
            cursor.advance();
            parse_dict_body(cursor)
        }
        TokenKind::Keyword => {
            // true / false / null are values; other keywords
            // shouldn't appear here (we only call parse_value
            // for non-keyword tokens at top level, but inside
            // arrays / dicts we re-enter and need the literal
            // value keywords to be acceptable).
            match tok.bytes {
                b"true" => {
                    cursor.advance();
                    Ok(Value::Bool(true))
                }
                b"false" => {
                    cursor.advance();
                    Ok(Value::Bool(false))
                }
                b"null" => {
                    cursor.advance();
                    Ok(Value::Null)
                }
                _ => {
                    cursor.advance();
                    Ok(Value::Null)
                }
            }
        }
        _ => {
            cursor.advance();
            Ok(Value::Null)
        }
    };
    Some(value)
}

fn parse_array_body(cursor: &mut TokenCursor<'_, '_>) -> Result<Value, Error> {
    let mut items = Vec::new();
    loop {
        // End of input closes the array over what was parsed.
        let Some(tok) = cursor.peek() else {
            return Ok(Value::Array(items));
        };
        if matches!(tok.kind, TokenKind::ArrayClose) {
            cursor.advance();
            return Ok(Value::Array(items));
        }
        match parse_value(cursor) {
            Some(Ok(v)) => items.push(v),
            Some(Err(e)) => return Err(e),
            None => return Ok(Value::Array(items)),
        }
    }
}

fn parse_dict_body(cursor: &mut TokenCursor<'_, '_>) -> Result<Value, Error> {
    let mut entries: Vec<(Vec<u8>, Value)> = Vec::new();
    loop {
        // End of input closes the dict over what was parsed.
        let Some(tok) = cursor.peek() else {
            return Ok(Value::Dict(entries));
        };
        if matches!(tok.kind, TokenKind::DictClose) {
            cursor.advance();
            return Ok(Value::Dict(entries));
        }
        // A key that is not a name cannot open an entry; drop it and
        // resynchronise on the next token.
        if !matches!(tok.kind, TokenKind::Name) {
            cursor.advance();
            continue;
        }
        let key = decode_name(tok.bytes);
        cursor.advance();
        match parse_value(cursor) {
            Some(Ok(value)) => entries.push((key, value)),
            Some(Err(e)) => return Err(e),
            None => return Ok(Value::Dict(entries)),
        }
    }
}

// ============================================================
// Token-bytes → typed-value decoders
// ============================================================

fn parse_number(bytes: &[u8]) -> Result<Value, Error> {
    // Real iff source contains a '.'. Sign characters and
    // leading dot ('.5') are valid per PDF.
    let s = std::str::from_utf8(bytes)
        .map_err(|_| Error::MalformedNumber)?;
    if s.contains('.') {
        let v: f64 = s.parse().map_err(|_| Error::MalformedNumber)?;
        Ok(Value::Real(v))
    } else {
        let v: i64 = s.parse().map_err(|_| Error::MalformedNumber)?;
        Ok(Value::Integer(v))
    }
}

fn decode_name(bytes: &[u8]) -> Vec<u8> {
    // bytes := "/foo[#XX]*". Drop the leading '/' and resolve
    // any "#XX" two-hex escape pairs.
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
    // bytes := "( ... )" — escape sequences resolved per
    // PDF §7.3.4.2: \n \r \t \b \f \( \) \\ \ddd line-cont.
    // Unbalanced inner parens are not legal; the lexer
    // delivered a balanced span.
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
                    // line continuation: \<EOL> drops both
                    // bytes from the decoded string.
                    i += 1;
                    if i < inner.len() && inner[i] == b'\n' {
                        i += 1;
                    }
                }
                b'\n' => { i += 1; }
                d if (b'0'..=b'7').contains(&d) => {
                    // Octal escape — up to 3 digits.
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
                other => {
                    // Unknown escape: drop the backslash, keep
                    // the byte verbatim per PDF §7.3.4.2.
                    out.push(other);
                    i += 1;
                }
            }
        } else {
            out.push(b);
            i += 1;
        }
    }
    out
}

fn decode_string_hex(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    // bytes := "< ... >". Whitespace inside is silently
    // ignored. An odd number of hex digits is padded with a
    // trailing 0 nibble per §7.3.4.3.
    let inner = &bytes[1..bytes.len() - 1];
    let mut nibbles: Vec<u8> = Vec::with_capacity(inner.len());
    for &b in inner {
        if let Some(v) = hex_val(b) {
            nibbles.push(v);
        } else if matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0C | 0x00) {
            continue;
        } else {
            return Err(Error::MalformedHexString);
        }
    }
    if nibbles.len() % 2 == 1 {
        nibbles.push(0);
    }
    Ok(nibbles
        .chunks(2)
        .map(|c| (c[0] << 4) | c[1])
        .collect())
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
        // The spec fixes a canonical form for every direct value but
        // not for the inline-image Stream, and the oracle corpus
        // avoids inline images, so this target picks one: the header
        // in dict form, then the samples as a hex string, which is
        // lossless and keeps the one-line-per-operation invariant that
        // raw sample bytes would break.
        Value::Stream { header, body } => {
            canonicalise_value(&Value::Dict(header.clone()), out);
            out.push(b' ');
            canonicalise_value(&Value::String(body.clone()), out);
        }
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(true) => out.extend_from_slice(b"true"),
        Value::Bool(false) => out.extend_from_slice(b"false"),
        Value::Integer(n) => {
            out.extend_from_slice(format!("{n}").as_bytes());
        }
        Value::Real(x) => {
            // Mathematically-integer reals emit as "X.0"; non-
            // integer reals use the shortest-round-trip form
            // Display gives by default. The "Real flag carries
            // through" rule (a real that happens to equal an
            // integer still serialises with a '.') is satisfied
            // by the tag — Integer never reaches this arm.
            if x.is_finite() && *x == x.trunc() {
                let abs = if *x < 0.0 { -*x } else { *x };
                if abs < (i64::MAX as f64) {
                    out.extend_from_slice(
                        format!("{}.0", *x as i64).as_bytes());
                    return;
                }
            }
            out.extend_from_slice(format!("{x}").as_bytes());
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
                    out.extend_from_slice(
                        format!("#{b:02X}").as_bytes());
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
                if i > 0 {
                    out.push(b' ');
                }
                canonicalise_value(it, out);
            }
            if !items.is_empty() {
                out.push(b' ');
            }
            out.push(b']');
        }
        Value::Dict(entries) => {
            // Sort entries by key bytes — hash-map iteration
            // order is undefined elsewhere, but this primitive
            // preserves insertion order via Vec; the canonical
            // sort happens here so the gate output is stable.
            let mut sorted: Vec<&(Vec<u8>, Value)> = entries.iter().collect();
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            out.extend_from_slice(b"<< ");
            for (i, (k, v)) in sorted.iter().enumerate() {
                if i > 0 {
                    out.push(b' ');
                }
                out.push(b'/');
                for &b in k.iter() {
                    if (0x21..=0x7E).contains(&b)
                        && !is_pdf_delim(b)
                        && b != b'#'
                    {
                        out.push(b);
                    } else {
                        out.extend_from_slice(
                            format!("#{b:02X}").as_bytes());
                    }
                }
                out.push(b' ');
                canonicalise_value(v, out);
            }
            if !sorted.is_empty() {
                out.push(b' ');
            }
            out.extend_from_slice(b">>");
        }
    }
}

#[allow(dead_code)] // retained for a future-phase public-API binding
fn is_pdf_delim(b: u8) -> bool {
    matches!(
        b,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical_str(input: &[u8]) -> String {
        let ops = parse(input).expect("parse");
        String::from_utf8(to_canonical(&ops)).expect("utf8 canonical")
    }

    #[test]
    fn empty_input_produces_empty_canonical() {
        assert_eq!(canonical_str(b""), "");
    }

    #[test]
    fn rpn_operands_then_operator() {
        // Simple path: two operations on one line.
        assert_eq!(canonical_str(b"100 200 m 150 250 l S"),
                   "100 200 m\n150 250 l\nS\n");
    }

    #[test]
    fn array_operand_emits_with_spaces() {
        assert_eq!(canonical_str(b"[5 3] 0 d"),
                   "[ 5 3 ] 0 d\n");
    }

    #[test]
    fn literal_string_decodes_to_hex_canonical() {
        assert_eq!(canonical_str(b"(Hi) Tj"),
                   "<4869> Tj\n");
    }

    #[test]
    fn hex_string_pads_odd_to_zero_low_nibble() {
        // <4> means one nibble; spec pads the missing low
        // nibble with 0, so the byte is 0x40.
        assert_eq!(canonical_str(b"<4> Tj"),
                   "<40> Tj\n");
    }

    #[test]
    fn dict_keys_sort_lexicographically() {
        assert_eq!(
            canonical_str(b"<< /MCID 1 /ActualText (Hi) >> BDC"),
            "<< /ActualText <4869> /MCID 1 >> BDC\n");
    }


    // ------------------------------------------------------------
    // Inline images (§8.9.7). The whole image is one operation whose
    // operand carries the parameter dictionary and the samples; the
    // sample bytes are not lexable PDF syntax, so they are lifted out
    // before lexing.
    // ------------------------------------------------------------

    fn inline_image(ops: &[Operation]) -> (&[(Vec<u8>, Value)], &[u8]) {
        assert_eq!(ops.len(), 1, "expected one operation, got {ops:?}");
        assert_eq!(ops[0].op, INLINE_IMAGE);
        match &ops[0].operands[..] {
            [Value::Stream { header, body }] => (header, body),
            other => panic!("expected a Stream operand, got {other:?}"),
        }
    }

    #[test]
    fn an_inline_image_surfaces_as_one_operation() {
        // 2x2, 1 component, 8 bpc -> 4 sample bytes.
        let src = b"BI /W 2 /H 2 /BPC 8 /CS /G ID \x01\x02\x03\x04 EI";
        let ops = parse(src).unwrap();
        let (header, body) = inline_image(&ops);
        assert_eq!(header[0], (b"W".to_vec(), Value::Integer(2)));
        assert_eq!(header[3], (b"CS".to_vec(), Value::Name(b"G".to_vec())));
        assert_eq!(body, b"\x01\x02\x03\x04");
    }

    #[test]
    fn samples_that_look_like_pdf_syntax_do_not_derail_the_lex() {
        // An unbalanced '(' in the samples would otherwise swallow the
        // rest of the stream as a literal string.
        let src = b"BI /W 4 /H 1 /BPC 8 /CS /G ID (\xff<<[ EI 10 20 m";
        let ops = parse(src).unwrap();
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].op, INLINE_IMAGE);
        assert_eq!(ops[1].op, "m");
        assert_eq!(ops[1].operands, vec![Value::Integer(10), Value::Integer(20)]);
    }

    #[test]
    fn an_id_inside_a_string_is_not_the_operator() {
        let ops = parse(b"(ID) Tj 10 20 m").unwrap();
        let rendered: Vec<&str> = ops.iter().map(|o| o.op.as_str()).collect();
        assert_eq!(rendered, ["Tj", "m"]);
    }

    #[test]
    fn the_sample_length_comes_from_geometry_not_whitespace() {
        // The last sample byte is 0x0A, which is also a whitespace
        // code. Trimming on whitespace would drop it and truncate the
        // image by a byte; the geometry says four bytes.
        let src = b"BI /W 2 /H 2 /BPC 8 /CS /G ID \x01\x02\x03\x0a EI";
        let ops = parse(src).unwrap();
        let (_, body) = inline_image(&ops);
        assert_eq!(body, b"\x01\x02\x03\x0a");
    }

    #[test]
    fn geometry_covers_component_count_and_sub_byte_depths() {
        // 2x2 RGB at 8 bpc = 12 bytes; the trailing " EI" bracket byte
        // captured with the run is trimmed away.
        let ops = parse(b"BI /W 2 /H 2 /BPC 8 /CS /RGB ID 0123456789ab EI").unwrap();
        let (_, body) = inline_image(&ops);
        assert_eq!(body, b"0123456789ab");

        // A 1-bit mask row rounds up to a whole byte: 2 rows, 1 byte.
        let ops = parse(b"BI /W 3 /H 2 /IM true ID \xf0\x0f EI").unwrap();
        let (_, body) = inline_image(&ops);
        assert_eq!(body, b"\xf0\x0f");
    }

    #[test]
    fn a_filtered_body_is_left_as_captured() {
        // A filtered run's raw length is unknown, so geometry cannot
        // trim it and it keeps the byte that brackets EI. That is safe
        // precisely because the filter's decoder self-terminates —
        // ASCIIHexDecode here stops at '>' and ignores whitespace.
        let ops = parse(b"BI /W 2 /H 2 /BPC 8 /CS /G /F /AHx ID 00112233445566 EI").unwrap();
        let (header, body) = inline_image(&ops);
        assert!(header.iter().any(|(k, _)| k == b"F"));
        assert_eq!(body, b"00112233445566 ");
    }

    #[test]
    fn an_unterminated_inline_image_takes_the_rest_of_the_stream() {
        // No EI: the samples run to end of input rather than failing.
        let ops = parse(b"BI /W 2 /H 2 /BPC 8 /CS /G ID \x01\x02").unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].op, INLINE_IMAGE);
    }

    #[test]
    fn an_inline_image_canonicalises_to_one_line() {
        let src = b"BI /W 1 /H 1 /BPC 8 /CS /G ID \x41 EI";
        let out = canonical_str(src);
        assert_eq!(out.lines().count(), 1);
        assert!(out.ends_with(" INLINE_IMAGE\n"), "{out:?}");
        assert!(out.contains("<41>"), "{out:?}");
    }

    // ------------------------------------------------------------
    // Leniency: structurally malformed input is repaired in place so
    // a renderer still receives every operation it can draw. Each
    // case below is a shape observed in the field.
    // ------------------------------------------------------------

    #[test]
    fn a_stray_delimiter_stands_in_as_a_null_operand() {
        // Between operations the stray token is dropped outright:
        // standing in as an operand there would shift every real
        // operand of the operation that follows.
        let ops = parse(b"10 20 m > 30 40 l").unwrap();
        let rendered: Vec<&str> = ops.iter().map(|o| o.op.as_str()).collect();
        assert_eq!(rendered, ["m", "l"]);
        assert_eq!(ops[1].operands, vec![Value::Integer(30), Value::Integer(40)]);
    }

    #[test]
    fn a_keyword_in_operand_position_stands_in_as_null() {
        // A malformed indirect reference: `R` cannot open a value.
        let ops = parse(b"[12 0 R] 0 d").unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(
            ops[0].operands[0],
            Value::Array(vec![Value::Integer(12), Value::Integer(0), Value::Null])
        );
    }

    #[test]
    fn an_unterminated_container_closes_over_what_was_parsed() {
        let ops = parse(b"[1 2 3").unwrap();
        assert_eq!(ops, Vec::new());
        let ops = parse(b"10 20 m [1 2").unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].op, "m");

        let ops = parse(b"<< /A 1").unwrap();
        assert_eq!(ops, Vec::new());
    }

    #[test]
    fn a_non_name_dict_key_is_skipped() {
        let ops = parse(b"<< 42 /A 1 >> BDC").unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(
            ops[0].operands[0],
            Value::Dict(vec![(b"A".to_vec(), Value::Integer(1))])
        );
    }

    #[test]
    fn malformed_input_never_costs_the_whole_stream() {
        // The shape that mattered: one bad byte used to fail the
        // parse, so a page yielded no operations at all.
        let ops = parse(b"q 1 0 0 1 0 0 cm > /Im0 Do Q").unwrap();
        let rendered: Vec<&str> = ops.iter().map(|o| o.op.as_str()).collect();
        assert_eq!(rendered, ["q", "cm", "Do", "Q"]);
        // The XObject name still reaches the Do that draws it.
        assert_eq!(ops[2].operands, vec![Value::Name(b"Im0".to_vec())]);
    }

    #[test]
    fn trailing_operands_are_dropped_not_failed() {
        // Operands with no operator to attach to carry no operation.
        assert_eq!(parse(b"100 200").unwrap(), Vec::new());
        // The operations before them still survive.
        let ops = parse(b"10 20 m 100 200").unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].op, "m");
    }

    #[test]
    fn whitespace_inside_array_is_skipped() {
        assert_eq!(
            canonical_str(b"[ 1   2\n3 ] g"),
            "[ 1 2 3 ] g\n");
    }

    #[test]
    fn comment_skipped_between_operands() {
        // "100 % comment\n 200 m" → 100 200 m
        assert_eq!(
            canonical_str(b"100 % comment\n 200 m"),
            "100 200 m\n");
    }

    #[test]
    fn real_with_integer_value_emits_with_decimal_zero() {
        // "1.0 g" should canonicalise to "1.0 g" (real flag
        // preserved through the round-trip).
        assert_eq!(canonical_str(b"1.0 g"), "1.0 g\n");
    }

    #[test]
    fn real_with_fraction_round_trips() {
        assert_eq!(canonical_str(b"0.8 0.2 0.1 rg"),
                   "0.8 0.2 0.1 rg\n");
    }
}
