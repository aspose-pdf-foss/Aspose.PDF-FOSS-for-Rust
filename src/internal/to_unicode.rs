// ToUnicode CMap parser (PDF 32000-1:2008 §9.10.3 + Adobe Tech
// Note #5014). Pure function from raw CMap bytes to a populated
// ToUnicode struct, plus a canonical text serialisation used by
// the freeze gate.
//
// v1 scope: 1- or 2-byte codespaces; beginbfchar / beginbfrange
// blocks with single-codepoint or multi-codepoint BMP destinations
// (multi handles ligatures like "ffi"); both bfrange dst forms
// (single hex string with last-code-unit increment, or array of
// per-char hex strings). PostScript flow control outside these
// blocks is silently tolerated — we scan for the three keyword
// pairs and ignore everything else.

use std::fmt;

/// One codespace range from `begincodespacerange`. byte_count is
/// the half-width of the start/end hex strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodespaceEntry {
    pub byte_count: u8,
    pub start: u32,
    pub end: u32,
}

/// One char-code → codepoints mapping. byte_count carries the
/// codespace width that produced it (used for canonical
/// padding).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapping {
    pub byte_count: u8,
    pub char_code: u32,
    pub codepoints: Vec<u16>,
}

/// Parsed ToUnicode CMap.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ToUnicode {
    pub codespace: Vec<CodespaceEntry>,
    pub mappings: Vec<Mapping>,
}

/// Errors returned by [`parse`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A `<...>` token contained a non-hex, non-whitespace byte.
    MalformedHexString,
    /// `<...>` decoded to an odd number of nibbles, or the two
    /// tokens of a codespace / bfchar pair had different widths.
    OddHexWidth,
    /// Codespace start and end widths disagree.
    MismatchedCodespaceWidth,
    /// dst hex string length is not a multiple of 4 hex digits
    /// (each UTF-16 code unit is exactly 4).
    DstNotMultipleOf4,
    /// Decoded codepoint falls in [0xD800, 0xDFFF] (UTF-16
    /// surrogate) or > 0xFFFF (supplementary plane). v1 is
    /// BMP-only.
    SurrogateOrSupplementary,
    /// `bfrange` array-dst form contained != end-start+1 entries.
    RangeArrayLengthMismatch,
    /// `bfrange` single-dst form would increment the last code
    /// unit past 0xFFFF.
    RangeOverflow,
    /// `bfrange` end < start.
    RangeEndBeforeStart,
    /// A `begin*` block ran off the end of input without its
    /// matching `end*` keyword.
    UnclosedBlock(&'static str),
    /// Premature end of input mid-token.
    #[allow(dead_code)] // retained for a future-phase public-API binding
    UnexpectedEnd,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MalformedHexString =>
                write!(f, "to_unicode: malformed hex string"),
            Error::OddHexWidth =>
                write!(f, "to_unicode: odd or mismatched hex width"),
            Error::MismatchedCodespaceWidth =>
                write!(f, "to_unicode: codespace start/end widths disagree"),
            Error::DstNotMultipleOf4 =>
                write!(f, "to_unicode: dst hex length not a multiple of 4"),
            Error::SurrogateOrSupplementary => write!(
                f, "to_unicode: codepoint outside BMP / in surrogate range"),
            Error::RangeArrayLengthMismatch => write!(
                f, "to_unicode: bfrange array length != end - start + 1"),
            Error::RangeOverflow =>
                write!(f, "to_unicode: bfrange single-dst overflows past 0xFFFF"),
            Error::RangeEndBeforeStart =>
                write!(f, "to_unicode: bfrange end < start"),
            Error::UnclosedBlock(name) =>
                write!(f, "to_unicode: unclosed {name} block"),
            Error::UnexpectedEnd =>
                write!(f, "to_unicode: unexpected end of input"),
        }
    }
}

impl std::error::Error for Error {}

// ============================================================
// Public API
// ============================================================

/// Parse a ToUnicode CMap byte stream.
pub fn parse(input: &[u8]) -> Result<ToUnicode, Error> {
    // Strip `%`-to-EOL comments before any keyword scanning. A
    // comment may legitimately contain the words we scan for.
    let stripped = strip_comments(input);

    let mut codespace: Vec<CodespaceEntry> = Vec::new();
    // Use a `Vec<(u32, Mapping)>` and dedupe at the end — the
    // anchor's "last one wins" semantics for duplicate codes are
    // honoured by overwriting on insertion order.
    let mut mappings: Vec<Mapping> = Vec::new();

    let mut p = Parser::new(&stripped);
    while let Some(tok) = p.peek_keyword() {
        match tok {
            "begincodespacerange" => {
                p.advance_keyword();
                parse_codespace_block(&mut p, &mut codespace)?;
            }
            "beginbfchar" => {
                p.advance_keyword();
                parse_bfchar_block(&mut p, &mut mappings)?;
            }
            "beginbfrange" => {
                p.advance_keyword();
                parse_bfrange_block(&mut p, &mut mappings)?;
            }
            _ => {
                p.advance_keyword();
            }
        }
    }

    // Apply the widest-codespace byte-count to every mapping —
    // canonical-line byte_count is the widest codespace width
    // declared, NOT the per-mapping width. (PDF allows mixed
    // widths; the gate disambiguates by padding all bf lines
    // to the widest.)
    let widest = codespace
        .iter()
        .map(|c| c.byte_count)
        .max()
        .unwrap_or(1);
    for m in &mut mappings {
        m.byte_count = widest;
    }

    // Last-one-wins dedup, then sort by char_code for the
    // canonical-line ordering.
    let mut by_code: std::collections::BTreeMap<u32, Mapping> =
        std::collections::BTreeMap::new();
    for m in mappings {
        by_code.insert(m.char_code, m);
    }
    let mappings: Vec<Mapping> = by_code.into_values().collect();

    // Sort codespace by (byte_count, start) for stable canonical.
    codespace.sort_by(|a, b| {
        a.byte_count
            .cmp(&b.byte_count)
            .then_with(|| a.start.cmp(&b.start))
    });

    Ok(ToUnicode { codespace, mappings })
}

/// Serialise a [`ToUnicode`] to its canonical form. Codespace
/// lines first, then bf lines, both groups internally sorted.
#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn to_canonical(t: &ToUnicode) -> Vec<u8> {
    let mut out = Vec::new();
    for c in &t.codespace {
        out.extend_from_slice(b"codespace ");
        out.extend_from_slice(format!("{}", c.byte_count).as_bytes());
        out.push(b' ');
        write_hex_padded(&mut out, c.start, c.byte_count);
        out.push(b' ');
        write_hex_padded(&mut out, c.end, c.byte_count);
        out.push(b'\n');
    }
    for m in &t.mappings {
        out.extend_from_slice(b"bf ");
        out.extend_from_slice(format!("{}", m.byte_count).as_bytes());
        out.push(b' ');
        write_hex_padded(&mut out, m.char_code, m.byte_count);
        for cp in &m.codepoints {
            out.push(b' ');
            write_hex_padded(&mut out, *cp as u32, 2);
        }
        out.push(b'\n');
    }
    out
}

#[allow(dead_code)] // retained for a future-phase public-API binding
fn write_hex_padded(out: &mut Vec<u8>, value: u32, byte_count: u8) {
    let width = byte_count as usize * 2;
    let s = format!("{value:0width$X}");
    out.extend_from_slice(s.as_bytes());
}

// ============================================================
// Comment stripping
// ============================================================

fn strip_comments(input: &[u8]) -> Vec<u8> {
    // PostScript convention: `%` to end-of-line is a comment.
    // The escape-sensitive PostScript rule for `%` inside hex
    // strings is academic for ToUnicode — ToUnicode hex strings
    // do not contain `%` — so a flat scan is correct.
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if input[i] == b'%' {
            while i < input.len() && input[i] != b'\n' && input[i] != b'\r' {
                i += 1;
            }
            continue;
        }
        out.push(input[i]);
        i += 1;
    }
    out
}

// ============================================================
// Parser
// ============================================================

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Parser { bytes, pos: 0 }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.bytes.len()
            && is_ps_ws(self.bytes[self.pos])
        {
            self.pos += 1;
        }
    }

    fn peek_keyword(&mut self) -> Option<&'a str> {
        let saved = self.pos;
        self.skip_whitespace();
        // Find a "keyword"-shaped run — either a bare regular
        // run of non-delimiter / non-whitespace bytes, OR `<` /
        // `[` (which start hex strings / arrays — caller still
        // sees those by peeking the actual token).
        if self.pos >= self.bytes.len() {
            self.pos = saved;
            return None;
        }
        let b = self.bytes[self.pos];
        if !is_keyword_start(b) {
            // Not a keyword — caller should advance via a token
            // reader, but for the top-level scanner we
            // skip non-keyword tokens (numbers, hex, arrays) via
            // `advance_keyword`.
            return Some("");
        }
        let start = self.pos;
        let mut end = start;
        while end < self.bytes.len()
            && is_keyword_byte(self.bytes[end])
        {
            end += 1;
        }
        // Don't advance — peek only.
        self.pos = saved;
        Some(std::str::from_utf8(&self.bytes[start..end]).unwrap_or(""))
    }

    /// Skip the next token (keyword, number, hex string, or
    /// array — whatever it is) without parsing it. Used by the
    /// top-level scanner to ignore non-block-keyword content.
    fn advance_keyword(&mut self) {
        self.skip_whitespace();
        if self.pos >= self.bytes.len() {
            return;
        }
        let b = self.bytes[self.pos];
        if b == b'<' {
            // skip a hex string
            while self.pos < self.bytes.len() && self.bytes[self.pos] != b'>' {
                self.pos += 1;
            }
            if self.pos < self.bytes.len() {
                self.pos += 1;
            }
            return;
        }
        if b == b'[' {
            // skip a balanced array
            let mut depth = 0;
            while self.pos < self.bytes.len() {
                match self.bytes[self.pos] {
                    b'[' => depth += 1,
                    b']' => {
                        depth -= 1;
                        if depth == 0 {
                            self.pos += 1;
                            return;
                        }
                    }
                    _ => {}
                }
                self.pos += 1;
            }
            return;
        }
        if b == b'(' {
            // skip a balanced literal string
            let mut depth = 0;
            while self.pos < self.bytes.len() {
                match self.bytes[self.pos] {
                    b'\\' => {
                        self.pos = (self.pos + 2).min(self.bytes.len());
                        continue;
                    }
                    b'(' => depth += 1,
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            self.pos += 1;
                            return;
                        }
                    }
                    _ => {}
                }
                self.pos += 1;
            }
            return;
        }
        // Bare token — number / name / keyword / operator. Eat
        // non-whitespace, non-delimiter bytes. If we entered on
        // an unhandled delimiter (e.g. `/` introducing a Name,
        // or `>` / `}` after comment-strip), advance past it —
        // otherwise the top-level scanner stalls forever on the
        // first PostScript name in the surrounding boilerplate.
        let start = self.pos;
        while self.pos < self.bytes.len()
            && !is_ps_ws(self.bytes[self.pos])
            && !is_ps_delim(self.bytes[self.pos])
        {
            self.pos += 1;
        }
        if self.pos == start && self.pos < self.bytes.len() {
            if self.bytes[self.pos] == b'/' {
                // Skip a Name (`/foo`) entirely so we land
                // cleanly on the next token boundary.
                self.pos += 1;
                while self.pos < self.bytes.len()
                    && !is_ps_ws(self.bytes[self.pos])
                    && !is_ps_delim(self.bytes[self.pos])
                {
                    self.pos += 1;
                }
            } else {
                // Lone unhandled delim — bump past it.
                self.pos += 1;
            }
        }
    }

    /// Read the next token, classifying it. Returns None at EOF.
    fn next_token(&mut self) -> Option<Token<'a>> {
        self.skip_whitespace();
        if self.pos >= self.bytes.len() {
            return None;
        }
        let b = self.bytes[self.pos];
        if b == b'<' {
            let start = self.pos + 1;
            self.pos += 1;
            while self.pos < self.bytes.len() && self.bytes[self.pos] != b'>' {
                self.pos += 1;
            }
            let end = self.pos;
            if self.pos < self.bytes.len() {
                self.pos += 1;
            }
            return Some(Token::Hex(&self.bytes[start..end]));
        }
        if b == b'[' {
            self.pos += 1;
            return Some(Token::ArrayOpen);
        }
        if b == b']' {
            self.pos += 1;
            return Some(Token::ArrayClose);
        }
        if b == b'(' {
            // Skip literal-string content; ToUnicode never
            // requires us to read one in v1, but the surrounding
            // /CIDSystemInfo dict carries `(Adobe)` etc.
            let _ = self.skip_literal_string();
            return Some(Token::Other);
        }
        // Bare token. Same stall guard as advance_keyword: if
        // we land on an unhandled delim (`/`, `>`, `}`, `%`
        // after comment-strip), make at least one byte of
        // progress so block-scanners don't loop forever
        // ignoring stray tokens.
        let start = self.pos;
        while self.pos < self.bytes.len()
            && !is_ps_ws(self.bytes[self.pos])
            && !is_ps_delim(self.bytes[self.pos])
        {
            self.pos += 1;
        }
        if self.pos == start && self.pos < self.bytes.len() {
            if self.bytes[self.pos] == b'/' {
                self.pos += 1;
                while self.pos < self.bytes.len()
                    && !is_ps_ws(self.bytes[self.pos])
                    && !is_ps_delim(self.bytes[self.pos])
                {
                    self.pos += 1;
                }
            } else {
                self.pos += 1;
            }
        }
        Some(Token::Bare(&self.bytes[start..self.pos]))
    }

    fn skip_literal_string(&mut self) -> Option<()> {
        if self.bytes.get(self.pos)? != &b'(' {
            return None;
        }
        self.pos += 1;
        let mut depth = 1;
        while self.pos < self.bytes.len() && depth > 0 {
            match self.bytes[self.pos] {
                b'\\' => self.pos = (self.pos + 2).min(self.bytes.len()),
                b'(' => {
                    depth += 1;
                    self.pos += 1;
                }
                b')' => {
                    depth -= 1;
                    self.pos += 1;
                }
                _ => self.pos += 1,
            }
        }
        Some(())
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Token<'a> {
    Hex(&'a [u8]),
    Bare(&'a [u8]),
    ArrayOpen,
    ArrayClose,
    Other,
}

fn is_ps_ws(b: u8) -> bool {
    matches!(b, 0x00 | 0x09 | 0x0A | 0x0C | 0x0D | b' ')
}

fn is_ps_delim(b: u8) -> bool {
    matches!(b, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%')
}

fn is_keyword_start(b: u8) -> bool {
    !is_ps_ws(b) && !is_ps_delim(b)
}

fn is_keyword_byte(b: u8) -> bool {
    !is_ps_ws(b) && !is_ps_delim(b)
}

// ============================================================
// Block parsers
// ============================================================

fn parse_codespace_block(
    p: &mut Parser<'_>,
    out: &mut Vec<CodespaceEntry>,
) -> Result<(), Error> {
    loop {
        match p.next_token().ok_or(Error::UnclosedBlock("codespacerange"))? {
            Token::Bare(b) if b == b"endcodespacerange" => return Ok(()),
            Token::Hex(start_hex) => {
                let end_tok = p.next_token()
                    .ok_or(Error::UnclosedBlock("codespacerange"))?;
                let end_hex = match end_tok {
                    Token::Hex(h) => h,
                    _ => return Err(Error::OddHexWidth),
                };
                let (start_bytes, sw) = decode_hex(start_hex)?;
                let (end_bytes, ew) = decode_hex(end_hex)?;
                if sw != ew {
                    return Err(Error::MismatchedCodespaceWidth);
                }
                out.push(CodespaceEntry {
                    byte_count: sw,
                    start: bytes_to_u32(&start_bytes),
                    end: bytes_to_u32(&end_bytes),
                });
            }
            _ => {
                // Numbers / arrays / etc. are tolerated and ignored.
            }
        }
    }
}

fn parse_bfchar_block(
    p: &mut Parser<'_>,
    out: &mut Vec<Mapping>,
) -> Result<(), Error> {
    loop {
        match p.next_token().ok_or(Error::UnclosedBlock("bfchar"))? {
            Token::Bare(b) if b == b"endbfchar" => return Ok(()),
            Token::Hex(src_hex) => {
                let dst_tok = p.next_token()
                    .ok_or(Error::UnclosedBlock("bfchar"))?;
                let dst_hex = match dst_tok {
                    Token::Hex(h) => h,
                    _ => return Err(Error::OddHexWidth),
                };
                let (src_bytes, sw) = decode_hex(src_hex)?;
                let (dst_bytes, _) = decode_hex(dst_hex)?;
                let cps = decode_utf16be(&dst_bytes)?;
                out.push(Mapping {
                    byte_count: sw,
                    char_code: bytes_to_u32(&src_bytes),
                    codepoints: cps,
                });
            }
            _ => {
                // Tolerate stray tokens (rare in real fixtures).
            }
        }
    }
}

fn parse_bfrange_block(
    p: &mut Parser<'_>,
    out: &mut Vec<Mapping>,
) -> Result<(), Error> {
    loop {
        match p.next_token().ok_or(Error::UnclosedBlock("bfrange"))? {
            Token::Bare(b) if b == b"endbfrange" => return Ok(()),
            Token::Hex(start_hex) => {
                // Consume <endHex>, then either <hex> or [<hex> ...].
                let end_tok = p.next_token()
                    .ok_or(Error::UnclosedBlock("bfrange"))?;
                let end_hex = match end_tok {
                    Token::Hex(h) => h,
                    _ => return Err(Error::OddHexWidth),
                };
                let (start_bytes, sw) = decode_hex(start_hex)?;
                let (end_bytes, ew) = decode_hex(end_hex)?;
                if sw != ew {
                    return Err(Error::MismatchedCodespaceWidth);
                }
                let start = bytes_to_u32(&start_bytes);
                let end = bytes_to_u32(&end_bytes);
                if end < start {
                    return Err(Error::RangeEndBeforeStart);
                }

                let dst_tok = p.next_token()
                    .ok_or(Error::UnclosedBlock("bfrange"))?;
                match dst_tok {
                    Token::Hex(dst_hex) => {
                        // Single-form: increment last code unit.
                        let (dst_bytes, _) = decode_hex(dst_hex)?;
                        let mut cps = decode_utf16be(&dst_bytes)?;
                        if cps.is_empty() {
                            return Err(Error::DstNotMultipleOf4);
                        }
                        let count = end - start + 1;
                        for offset in 0..count {
                            let last = cps.last_mut().unwrap();
                            // Overflow check on the last code unit
                            // (BMP-only; v1 rejects supplementary
                            // results).
                            let bumped = (*last as u32).checked_add(offset);
                            let bumped = match bumped {
                                Some(v) if v <= 0xFFFF => v as u16,
                                _ => return Err(Error::RangeOverflow),
                            };
                            *last = bumped;
                            check_bmp_only(&cps)?;
                            out.push(Mapping {
                                byte_count: sw,
                                char_code: start + offset,
                                codepoints: cps.clone(),
                            });
                            // Restore for next iteration so
                            // increments compose correctly.
                            *cps.last_mut().unwrap() = (bumped as u32 - offset) as u16;
                        }
                    }
                    Token::ArrayOpen => {
                        // Array-form: collect inner hex tokens.
                        let mut entries: Vec<Vec<u16>> = Vec::new();
                        loop {
                            match p
                                .next_token()
                                .ok_or(Error::UnclosedBlock("bfrange"))?
                            {
                                Token::ArrayClose => break,
                                Token::Hex(h) => {
                                    let (b, _) = decode_hex(h)?;
                                    entries.push(decode_utf16be(&b)?);
                                }
                                _ => {}
                            }
                        }
                        let expected = (end - start + 1) as usize;
                        if entries.len() != expected {
                            return Err(Error::RangeArrayLengthMismatch);
                        }
                        for (i, cps) in entries.into_iter().enumerate() {
                            check_bmp_only(&cps)?;
                            out.push(Mapping {
                                byte_count: sw,
                                char_code: start + i as u32,
                                codepoints: cps,
                            });
                        }
                    }
                    _ => return Err(Error::OddHexWidth),
                }
            }
            _ => {}
        }
    }
}

// ============================================================
// Hex / UTF-16BE helpers
// ============================================================

fn decode_hex(input: &[u8]) -> Result<(Vec<u8>, u8), Error> {
    // Whitespace bytes inside `<...>` are stripped per
    // PDF §7.3.4.3. Odd nibble counts are an error.
    let mut nibbles: Vec<u8> = Vec::with_capacity(input.len());
    for &b in input {
        if is_ps_ws(b) {
            continue;
        }
        match b {
            b'0'..=b'9' => nibbles.push(b - b'0'),
            b'a'..=b'f' => nibbles.push(b - b'a' + 10),
            b'A'..=b'F' => nibbles.push(b - b'A' + 10),
            _ => return Err(Error::MalformedHexString),
        }
    }
    if nibbles.len() % 2 != 0 {
        return Err(Error::OddHexWidth);
    }
    let bytes: Vec<u8> = nibbles.chunks(2).map(|c| (c[0] << 4) | c[1]).collect();
    Ok((bytes.clone(), bytes.len() as u8))
}

fn bytes_to_u32(bytes: &[u8]) -> u32 {
    // Big-endian; up to 4 bytes (v1 caps at 2 in practice but
    // the helper handles wider too).
    let mut v: u32 = 0;
    for &b in bytes {
        v = (v << 8) | b as u32;
    }
    v
}

fn decode_utf16be(bytes: &[u8]) -> Result<Vec<u16>, Error> {
    if bytes.len() % 2 != 0 {
        return Err(Error::DstNotMultipleOf4);
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks(2) {
        let cu = ((chunk[0] as u16) << 8) | chunk[1] as u16;
        out.push(cu);
    }
    check_bmp_only(&out)?;
    Ok(out)
}

fn check_bmp_only(units: &[u16]) -> Result<(), Error> {
    // v1 rejects surrogate halves (a multi-codepoint dst that
    // includes a surrogate would represent a > 0xFFFF code
    // point; the anchor explicitly rejects supplementary plane).
    for &u in units {
        if (0xD800..=0xDFFF).contains(&u) {
            return Err(Error::SurrogateOrSupplementary);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical_str(input: &[u8]) -> String {
        let t = parse(input).expect("parse");
        String::from_utf8(to_canonical(&t)).expect("utf8 canonical")
    }

    #[test]
    fn simple_bfchar_round_trip() {
        let input = b"\
            1 begincodespacerange\n\
            <00> <FF>\n\
            endcodespacerange\n\
            2 beginbfchar\n\
            <41> <0041>\n\
            <42> <0042>\n\
            endbfchar\n";
        assert_eq!(
            canonical_str(input),
            "codespace 1 00 FF\nbf 1 41 0041\nbf 1 42 0042\n"
        );
    }

    #[test]
    fn bfrange_single_dst_increments_last_code_unit() {
        let input = b"\
            1 begincodespacerange\n\
            <00> <FF>\n\
            endcodespacerange\n\
            1 beginbfrange\n\
            <41> <43> <0041>\n\
            endbfrange\n";
        assert_eq!(
            canonical_str(input),
            "codespace 1 00 FF\nbf 1 41 0041\nbf 1 42 0042\nbf 1 43 0043\n"
        );
    }

    #[test]
    fn bfrange_array_dst_explicit_per_char() {
        let input = b"\
            1 begincodespacerange\n\
            <00> <FF>\n\
            endcodespacerange\n\
            1 beginbfrange\n\
            <10> <12> [<0041> <0042> <0043>]\n\
            endbfrange\n";
        assert_eq!(
            canonical_str(input),
            "codespace 1 00 FF\nbf 1 10 0041\nbf 1 11 0042\nbf 1 12 0043\n"
        );
    }

    #[test]
    fn comments_stripped_before_block_scan() {
        let input = b"\
            % a fake beginbfchar in a comment\n\
            1 begincodespacerange\n\
            <00> <FF>\n\
            endcodespacerange\n\
            1 beginbfchar\n\
            <41> <0041>\n\
            endbfchar\n";
        assert_eq!(
            canonical_str(input),
            "codespace 1 00 FF\nbf 1 41 0041\n"
        );
    }

    #[test]
    fn multi_codepoint_dst_for_ligature() {
        let input = b"\
            1 begincodespacerange\n\
            <0000> <FFFF>\n\
            endcodespacerange\n\
            1 beginbfchar\n\
            <0100> <0066 0066 0069>\n\
            endbfchar\n";
        assert_eq!(
            canonical_str(input),
            "codespace 2 0000 FFFF\nbf 2 0100 0066 0066 0069\n"
        );
    }

    #[test]
    fn bfrange_array_length_mismatch_errors() {
        let input = b"\
            1 begincodespacerange\n\
            <00> <FF>\n\
            endcodespacerange\n\
            1 beginbfrange\n\
            <10> <12> [<0041> <0042>]\n\
            endbfrange\n";
        assert_eq!(
            parse(input).unwrap_err(),
            Error::RangeArrayLengthMismatch
        );
    }

    #[test]
    fn bfrange_overflow_past_ffff_errors() {
        let input = b"\
            1 begincodespacerange\n\
            <00> <FF>\n\
            endcodespacerange\n\
            1 beginbfrange\n\
            <00> <05> <FFFE>\n\
            endbfrange\n";
        assert_eq!(parse(input).unwrap_err(), Error::RangeOverflow);
    }

    #[test]
    fn surrogate_dst_rejected() {
        let input = b"\
            1 begincodespacerange\n\
            <00> <FF>\n\
            endcodespacerange\n\
            1 beginbfchar\n\
            <41> <D800>\n\
            endbfchar\n";
        assert_eq!(parse(input).unwrap_err(), Error::SurrogateOrSupplementary);
    }
}
