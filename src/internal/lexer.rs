// Trivia-preserving tokeniser for PDF 32000-1:2008 §7.2 lexical
// syntax. Every byte of the input ends up inside exactly one
// emitted Token — comments, whitespace, and end-of-line markers
// keep their own token kinds rather than being skipped, so the
// downstream parser can reconstitute the original byte stream
// verbatim. That property is what makes the byte_roundtrip
// freeze gate self-certifying: no oracle is needed because
// `emit(lex(b)) == b` for every input.
//
// Scope is lexical only — comments, whitespace, EOLs, numbers,
// names, strings (literal + hex), keywords, and the four
// bracket / brace / chevron delimiters. Cross-token meaning
// (xref tables, object structure, filter decoding, escape
// resolution) is the next layer up.
//
// Character classes (§7.2.3):
//
//   * Whitespace: NUL (0x00), TAB (0x09), LF (0x0A), FF (0x0C),
//     CR (0x0D), SP (0x20). LF and CR also count as EOL bytes
//     and are split into a separate token kind.
//   * Delimiters: ( ) < > [ ] { } / %
//   * Regular: any byte that is neither whitespace nor a
//     delimiter.
//
// EOL conventions (§7.2.3): LF alone, CR alone, or CRLF. The
// lexer emits a single Eol token per EOL marker — `\r\n` is one
// token, not two.

/// One PDF lexical token kind. The set is closed by the shared
/// spec; adding a kind here requires a matching update to the
/// fixture corpus + emitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    /// Name object — `/Foo`, `/With#20Space`. The leading `/`
    /// and any `#XX` hex escapes are preserved verbatim; escape
    /// resolution is the parser's job.
    Name,
    /// Number literal — integer or real. Sign characters and
    /// decimal-point location are preserved verbatim (no
    /// canonicalisation at the lexer layer).
    Number,
    /// `( ... )` literal string. The opening and closing parens
    /// plus every byte between them — including escape
    /// sequences and balanced inner parens — are preserved
    /// verbatim.
    StringLiteral,
    /// `< ... >` hex string. Embedded whitespace inside the
    /// braces is preserved verbatim (decoding-time stripping is
    /// the parser's job).
    StringHex,
    /// `[` array open delimiter.
    ArrayOpen,
    /// `]` array close delimiter.
    ArrayClose,
    /// `<<` dictionary open delimiter.
    DictOpen,
    /// `>>` dictionary close delimiter.
    DictClose,
    /// Bare regular-character run that didn't start with a
    /// digit / sign / dot — `obj`, `endobj`, `stream`,
    /// `endstream`, `R`, `xref`, `trailer`, `startxref`, `true`,
    /// `false`, `null`, `f`, `n`, etc. The lexer does not
    /// distinguish between language keywords and arbitrary
    /// regular runs.
    Keyword,
    /// `% ... <EOL>` comment, NOT including the trailing EOL
    /// byte(s) (those are emitted as a separate Eol token).
    Comment,
    /// Run of non-EOL whitespace bytes (NUL / TAB / FF / SP).
    /// LF and CR bytes are emitted as Eol tokens instead.
    Whitespace,
    /// One end-of-line marker — `\n`, `\r`, or `\r\n`.
    Eol,
}

/// One token. Carries its kind, a zero-copy byte view into the
/// input it was lexed from, and the absolute byte offset where
/// the token starts. The byte view IS the original input bytes
/// (no decoding / unescaping is applied), which is what makes
/// `emit` a pure inverse of `lex`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token<'a> {
    pub kind: TokenKind,
    pub bytes: &'a [u8],
    pub offset: usize,
}

/// Tokenise `input` into a sequence of [`Token`]s. Total — never
/// fails. Every byte of `input` ends up inside exactly one
/// returned token, so `emit(lex(input)) == input` byte-for-byte.
pub fn lex(input: &[u8]) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < input.len() {
        let start = pos;
        let b = input[pos];

        // EOL — `\r\n` (count as one token), `\r`, or `\n`.
        if b == b'\r' {
            pos += 1;
            if pos < input.len() && input[pos] == b'\n' {
                pos += 1;
            }
            out.push(Token { kind: TokenKind::Eol, bytes: &input[start..pos], offset: start });
            continue;
        }
        if b == b'\n' {
            pos += 1;
            out.push(Token { kind: TokenKind::Eol, bytes: &input[start..pos], offset: start });
            continue;
        }

        // Non-EOL whitespace run — coalesce consecutive bytes.
        if is_ws_non_eol(b) {
            while pos < input.len() && is_ws_non_eol(input[pos]) {
                pos += 1;
            }
            out.push(Token { kind: TokenKind::Whitespace, bytes: &input[start..pos], offset: start });
            continue;
        }

        // Comment — `%` to (but not including) the next EOL.
        if b == b'%' {
            pos += 1;
            while pos < input.len() && input[pos] != b'\r' && input[pos] != b'\n' {
                pos += 1;
            }
            out.push(Token { kind: TokenKind::Comment, bytes: &input[start..pos], offset: start });
            continue;
        }

        // Bracket delimiters — single byte.
        if b == b'[' {
            pos += 1;
            out.push(Token { kind: TokenKind::ArrayOpen, bytes: &input[start..pos], offset: start });
            continue;
        }
        if b == b']' {
            pos += 1;
            out.push(Token { kind: TokenKind::ArrayClose, bytes: &input[start..pos], offset: start });
            continue;
        }

        // Chevron — `<<` is dict open, lone `<` opens a hex
        // string consumed up to and including the next `>`.
        if b == b'<' {
            if pos + 1 < input.len() && input[pos + 1] == b'<' {
                pos += 2;
                out.push(Token { kind: TokenKind::DictOpen, bytes: &input[start..pos], offset: start });
                continue;
            }
            pos += 1;
            while pos < input.len() && input[pos] != b'>' {
                pos += 1;
            }
            if pos < input.len() {
                pos += 1; // include closing '>'
            }
            out.push(Token { kind: TokenKind::StringHex, bytes: &input[start..pos], offset: start });
            continue;
        }

        // Chevron — `>>` is dict close. A lone `>` is malformed
        // per spec but for roundtrip safety emits as a one-byte
        // Keyword (the parser layer surfaces the structural
        // error with context).
        if b == b'>' {
            if pos + 1 < input.len() && input[pos + 1] == b'>' {
                pos += 2;
                out.push(Token { kind: TokenKind::DictClose, bytes: &input[start..pos], offset: start });
                continue;
            }
            pos += 1;
            out.push(Token { kind: TokenKind::Keyword, bytes: &input[start..pos], offset: start });
            continue;
        }

        // Literal string — `( ... )` with balanced parens and
        // backslash escapes. Any byte may appear after `\\`
        // (preserved verbatim — escape resolution is parser
        // business).
        if b == b'(' {
            pos += 1;
            let mut depth: usize = 1;
            while pos < input.len() && depth > 0 {
                let c = input[pos];
                if c == b'\\' {
                    pos += 1;
                    if pos < input.len() {
                        pos += 1;
                    }
                } else if c == b'(' {
                    depth += 1;
                    pos += 1;
                } else if c == b')' {
                    depth -= 1;
                    pos += 1;
                } else {
                    pos += 1;
                }
            }
            out.push(Token { kind: TokenKind::StringLiteral, bytes: &input[start..pos], offset: start });
            continue;
        }

        // Name — `/` followed by regular characters (with `#XX`
        // hex escapes preserved as-is).
        if b == b'/' {
            pos += 1;
            while pos < input.len() && is_regular(input[pos]) {
                pos += 1;
            }
            out.push(Token { kind: TokenKind::Name, bytes: &input[start..pos], offset: start });
            continue;
        }

        // Number literal — starts with a digit, or with a sign
        // (`+` / `-`) immediately followed by a digit-or-dot,
        // or with `.` immediately followed by a digit. Once
        // classified as a number, consume the full regular-
        // character run; structural validation (one decimal
        // point, signs only at the start, etc.) is the parser's
        // job.
        if starts_number(input, pos) {
            pos += 1;
            while pos < input.len() && is_regular(input[pos]) {
                pos += 1;
            }
            out.push(Token { kind: TokenKind::Number, bytes: &input[start..pos], offset: start });
            continue;
        }

        // Keyword — generic regular-character run that wasn't
        // classified above. Always consumes at least one byte
        // because `b` is a regular character (not whitespace,
        // not a delimiter, not a sign / digit / dot starting
        // a number).
        while pos < input.len() && is_regular(input[pos]) {
            pos += 1;
        }
        if pos == start {
            // Defensive guarantee: never emit a zero-byte token,
            // which would prevent loop progress. Unreachable in
            // well-formed PDF byte streams (every byte falls
            // into one of the categories above), but a guard
            // here prevents a hang on adversarial input.
            pos += 1;
        }
        out.push(Token { kind: TokenKind::Keyword, bytes: &input[start..pos], offset: start });
    }
    out
}

/// Concatenate every token's byte view to reconstruct the input
/// `lex` was called on. Pure inverse — `emit(lex(b)) == b` for
/// every byte slice `b`.
#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn emit(tokens: &[Token<'_>]) -> Vec<u8> {
    let total: usize = tokens.iter().map(|t| t.bytes.len()).sum();
    let mut out = Vec::with_capacity(total);
    for t in tokens {
        out.extend_from_slice(t.bytes);
    }
    out
}

#[inline]
fn is_ws_non_eol(b: u8) -> bool {
    matches!(b, 0x00 | 0x09 | 0x0C | 0x20)
}

#[inline]
fn is_delimiter(b: u8) -> bool {
    matches!(b, b'(' | b')' | b'<' | b'>' | b'[' | b']'
              | b'{' | b'}' | b'/' | b'%')
}

#[inline]
fn is_whitespace(b: u8) -> bool {
    is_ws_non_eol(b) || b == b'\n' || b == b'\r'
}

#[inline]
fn is_regular(b: u8) -> bool {
    !is_whitespace(b) && !is_delimiter(b)
}

#[inline]
fn starts_number(input: &[u8], pos: usize) -> bool {
    let b = input[pos];
    if b.is_ascii_digit() {
        return true;
    }
    let next = input.get(pos + 1).copied();
    match (b, next) {
        (b'+', Some(c)) | (b'-', Some(c)) if c.is_ascii_digit() || c == b'.' => true,
        (b'.', Some(c)) if c.is_ascii_digit() => true,
        _ => false,
    }
}

// =====================  Tests  ==============================

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(tokens: &[Token<'_>]) -> Vec<TokenKind> {
        tokens.iter().map(|t| t.kind).collect()
    }

    fn views<'a>(tokens: &[Token<'a>]) -> Vec<&'a [u8]> {
        tokens.iter().map(|t| t.bytes).collect()
    }

    fn roundtrips(input: &[u8]) {
        let tokens = lex(input);
        // Concatenated bytes equal the input.
        assert_eq!(emit(&tokens), input,
                   "byte_roundtrip violated for input {:?}", input);
        // Offsets are monotonically non-decreasing and match
        // cumulative byte view length.
        let mut cursor = 0;
        for t in &tokens {
            assert_eq!(t.offset, cursor,
                       "token offset {} != cumulative {} for {:?}",
                       t.offset, cursor, t);
            cursor += t.bytes.len();
        }
        assert_eq!(cursor, input.len());
    }

    #[test]
    fn empty_input_yields_no_tokens() {
        let tokens = lex(b"");
        assert!(tokens.is_empty());
        assert_eq!(emit(&tokens), b"");
    }

    // ------ Trivia.

    #[test]
    fn whitespace_run_coalesced() {
        let tokens = lex(b"   \t\t ");
        assert_eq!(kinds(&tokens), vec![TokenKind::Whitespace]);
        assert_eq!(tokens[0].bytes, b"   \t\t ");
    }

    #[test]
    fn lf_eol_is_separate_token() {
        let tokens = lex(b" \n ");
        assert_eq!(kinds(&tokens), vec![
            TokenKind::Whitespace, TokenKind::Eol, TokenKind::Whitespace,
        ]);
        assert_eq!(views(&tokens), vec![&b" "[..], &b"\n"[..], &b" "[..]]);
    }

    #[test]
    fn cr_eol_alone() {
        let tokens = lex(b"a\rb");
        assert_eq!(kinds(&tokens), vec![
            TokenKind::Keyword, TokenKind::Eol, TokenKind::Keyword,
        ]);
        assert_eq!(tokens[1].bytes, b"\r");
    }

    #[test]
    fn crlf_eol_single_token() {
        let tokens = lex(b"\r\n");
        assert_eq!(kinds(&tokens), vec![TokenKind::Eol]);
        assert_eq!(tokens[0].bytes, b"\r\n");
    }

    #[test]
    fn comment_excludes_eol_byte() {
        let tokens = lex(b"% hi there\n");
        assert_eq!(kinds(&tokens), vec![TokenKind::Comment, TokenKind::Eol]);
        assert_eq!(tokens[0].bytes, b"% hi there");
        assert_eq!(tokens[1].bytes, b"\n");
    }

    #[test]
    fn comment_runs_to_eof_when_no_eol() {
        let tokens = lex(b"% trailing");
        assert_eq!(kinds(&tokens), vec![TokenKind::Comment]);
        assert_eq!(tokens[0].bytes, b"% trailing");
    }

    // ------ Names.

    #[test]
    fn simple_name() {
        let tokens = lex(b"/Type");
        assert_eq!(kinds(&tokens), vec![TokenKind::Name]);
        assert_eq!(tokens[0].bytes, b"/Type");
    }

    #[test]
    fn name_preserves_hex_escapes_verbatim() {
        let tokens = lex(b"/With#20Space");
        assert_eq!(kinds(&tokens), vec![TokenKind::Name]);
        assert_eq!(tokens[0].bytes, b"/With#20Space");
    }

    #[test]
    fn name_terminates_at_delimiter() {
        let tokens = lex(b"/A[/B");
        assert_eq!(kinds(&tokens), vec![
            TokenKind::Name, TokenKind::ArrayOpen, TokenKind::Name,
        ]);
        assert_eq!(tokens[0].bytes, b"/A");
        assert_eq!(tokens[2].bytes, b"/B");
    }

    // ------ Numbers.

    #[test]
    fn integer_positive() {
        let tokens = lex(b"42");
        assert_eq!(kinds(&tokens), vec![TokenKind::Number]);
        assert_eq!(tokens[0].bytes, b"42");
    }

    #[test]
    fn signed_integers() {
        let tokens = lex(b"-17 +8");
        assert_eq!(kinds(&tokens), vec![
            TokenKind::Number, TokenKind::Whitespace, TokenKind::Number,
        ]);
        assert_eq!(tokens[0].bytes, b"-17");
        assert_eq!(tokens[2].bytes, b"+8");
    }

    #[test]
    fn real_with_leading_dot() {
        let tokens = lex(b".5");
        assert_eq!(kinds(&tokens), vec![TokenKind::Number]);
        assert_eq!(tokens[0].bytes, b".5");
    }

    #[test]
    fn real_with_trailing_dot() {
        let tokens = lex(b"5.");
        assert_eq!(kinds(&tokens), vec![TokenKind::Number]);
        assert_eq!(tokens[0].bytes, b"5.");
    }

    #[test]
    fn negative_real() {
        let tokens = lex(b"-0.25");
        assert_eq!(kinds(&tokens), vec![TokenKind::Number]);
        assert_eq!(tokens[0].bytes, b"-0.25");
    }

    #[test]
    fn negative_zero_preserved() {
        let tokens = lex(b"-0");
        assert_eq!(kinds(&tokens), vec![TokenKind::Number]);
        assert_eq!(tokens[0].bytes, b"-0");
    }

    #[test]
    fn signed_dot_real() {
        // -.5 — sign followed by dot, then digit.
        let tokens = lex(b"-.5");
        assert_eq!(kinds(&tokens), vec![TokenKind::Number]);
        assert_eq!(tokens[0].bytes, b"-.5");
    }

    #[test]
    fn lone_sign_is_keyword() {
        let tokens = lex(b"+ ");
        assert_eq!(kinds(&tokens), vec![TokenKind::Keyword, TokenKind::Whitespace]);
        assert_eq!(tokens[0].bytes, b"+");
    }

    #[test]
    fn lone_dot_is_keyword() {
        let tokens = lex(b". ");
        assert_eq!(kinds(&tokens), vec![TokenKind::Keyword, TokenKind::Whitespace]);
        assert_eq!(tokens[0].bytes, b".");
    }

    // ------ Strings.

    #[test]
    fn literal_string_simple() {
        let tokens = lex(b"(hello)");
        assert_eq!(kinds(&tokens), vec![TokenKind::StringLiteral]);
        assert_eq!(tokens[0].bytes, b"(hello)");
    }

    #[test]
    fn literal_string_balanced_parens() {
        let tokens = lex(b"(outer (inner) text)");
        assert_eq!(kinds(&tokens), vec![TokenKind::StringLiteral]);
        assert_eq!(tokens[0].bytes, b"(outer (inner) text)");
    }

    #[test]
    fn literal_string_escaped_paren() {
        // Backslash-paren must not affect balance.
        let tokens = lex(b"(a\\)b)");
        assert_eq!(kinds(&tokens), vec![TokenKind::StringLiteral]);
        assert_eq!(tokens[0].bytes, b"(a\\)b)");
    }

    #[test]
    fn literal_string_octal_escape_preserved() {
        let tokens = lex(b"(\\053plus)");
        assert_eq!(kinds(&tokens), vec![TokenKind::StringLiteral]);
        assert_eq!(tokens[0].bytes, b"(\\053plus)");
    }

    #[test]
    fn literal_string_double_backslash_at_end() {
        let tokens = lex(b"(text\\\\)");
        assert_eq!(kinds(&tokens), vec![TokenKind::StringLiteral]);
        assert_eq!(tokens[0].bytes, b"(text\\\\)");
    }

    #[test]
    fn hex_string_simple() {
        let tokens = lex(b"<48656c6c6f>");
        assert_eq!(kinds(&tokens), vec![TokenKind::StringHex]);
        assert_eq!(tokens[0].bytes, b"<48656c6c6f>");
    }

    #[test]
    fn hex_string_with_embedded_whitespace_preserved() {
        let tokens = lex(b"<48 65 6C>");
        assert_eq!(kinds(&tokens), vec![TokenKind::StringHex]);
        assert_eq!(tokens[0].bytes, b"<48 65 6C>");
    }

    #[test]
    fn empty_hex_string() {
        let tokens = lex(b"<>");
        assert_eq!(kinds(&tokens), vec![TokenKind::StringHex]);
        assert_eq!(tokens[0].bytes, b"<>");
    }

    #[test]
    fn odd_length_hex_string_preserved() {
        let tokens = lex(b"<48F>");
        assert_eq!(kinds(&tokens), vec![TokenKind::StringHex]);
        assert_eq!(tokens[0].bytes, b"<48F>");
    }

    // ------ Delimiters.

    #[test]
    fn dict_open_close() {
        let tokens = lex(b"<<>>");
        assert_eq!(kinds(&tokens), vec![TokenKind::DictOpen, TokenKind::DictClose]);
    }

    #[test]
    fn dict_open_then_hex_then_close() {
        // `<<` then `<48>` then `>>`.
        let tokens = lex(b"<<<48>>>");
        assert_eq!(kinds(&tokens), vec![
            TokenKind::DictOpen, TokenKind::StringHex, TokenKind::DictClose,
        ]);
        assert_eq!(tokens[1].bytes, b"<48>");
    }

    #[test]
    fn array_brackets() {
        let tokens = lex(b"[]");
        assert_eq!(kinds(&tokens), vec![TokenKind::ArrayOpen, TokenKind::ArrayClose]);
    }

    // ------ Indirect reference shape.

    #[test]
    fn indirect_reference_tokens() {
        let tokens = lex(b"1 0 R");
        let ks = kinds(&tokens);
        assert_eq!(ks, vec![
            TokenKind::Number, TokenKind::Whitespace,
            TokenKind::Number, TokenKind::Whitespace,
            TokenKind::Keyword,
        ]);
        assert_eq!(tokens[4].bytes, b"R");
    }

    // ------ Roundtrip — the gate property.

    #[test]
    fn roundtrip_minimal_pdf_skeleton() {
        let input = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\n";
        roundtrips(input);
    }

    #[test]
    fn roundtrip_eol_variants() {
        roundtrips(b"a\nb\rc\r\nd");
    }

    #[test]
    fn roundtrip_binary_safe_magic_comment() {
        roundtrips(b"%\xe2\xe3\xcf\xd3\n");
    }

    #[test]
    fn roundtrip_unterminated_literal_string_at_eof() {
        // Lexer is total — unterminated strings just consume to
        // EOF; the parser layer surfaces the structural error.
        roundtrips(b"(unterminated");
    }

    #[test]
    fn roundtrip_unterminated_hex_string_at_eof() {
        roundtrips(b"<48656c");
    }

    #[test]
    fn roundtrip_lone_gt_emits_keyword() {
        let tokens = lex(b">a");
        assert_eq!(kinds(&tokens), vec![TokenKind::Keyword, TokenKind::Keyword]);
        assert_eq!(tokens[0].bytes, b">");
        assert_eq!(tokens[1].bytes, b"a");
        assert_eq!(emit(&tokens), b">a");
    }

    #[test]
    fn token_offsets_match_cumulative_position() {
        let tokens = lex(b"42 /A");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].offset, 0);
        assert_eq!(tokens[1].offset, 2);
        assert_eq!(tokens[2].offset, 3);
    }

    #[test]
    fn null_byte_is_whitespace() {
        // PDF whitespace set includes NUL (0x00).
        let tokens = lex(b"a\0b");
        assert_eq!(kinds(&tokens), vec![
            TokenKind::Keyword, TokenKind::Whitespace, TokenKind::Keyword,
        ]);
        assert_eq!(tokens[1].bytes, &[0x00][..]);
    }

    #[test]
    fn form_feed_is_whitespace() {
        let tokens = lex(b"a\x0Cb");
        assert_eq!(kinds(&tokens), vec![
            TokenKind::Keyword, TokenKind::Whitespace, TokenKind::Keyword,
        ]);
        assert_eq!(tokens[1].bytes, &[0x0C][..]);
    }
}
