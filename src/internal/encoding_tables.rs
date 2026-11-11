// PDF standard encodings (byte → PostScript glyph name) per
// PDF 32000-1:2008 §9.6.6.4 + Annex D Table D.2.
//
// The four 256-slot tables are generated from pdfminer.six's
// `latin_enc.ENCODING` (a republication of Annex D Table D.2)
// by `Feedstock/PDF/Spec/targets/rust/encoding_tables.py` and
// committed alongside in `encoding_tables_data.rs`.
//
// No corpus freeze gate: the data IS the canonical reference.
// Inline `#[cfg(test)] mod tests` spot-checks 33 representative
// slots across all four encodings + the from_name parser.

#[path = "encoding_tables_data.rs"]
mod data;

/// The four PDF standard encodings (PDF §9.6.6.4 + Annex D).
/// SymbolEncoding / ZapfDingbatsEncoding / MacExpertEncoding are
/// out of scope for v1 (their glyph names do not resolve through
/// the AGL).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// PDF default for Type1 fonts.
    Standard,
    /// MacRoman (PDF Table D.2 column 2).
    MacRoman,
    /// Windows ANSI (PDF default for TrueType fonts on most systems).
    WinAnsi,
    /// PDFDocEncoding — used by document-info / metadata strings.
    PdfDoc,
}

/// Return the glyph name at `code` in `encoding`. Returns `""`
/// for unmapped slots (e.g. WinAnsi 0x9D is officially undefined).
pub fn byte_to_glyph_name(encoding: Encoding, code: u8) -> &'static str {
    table(encoding)[code as usize]
}

/// Return the full 256-entry name table for `encoding`, indexed
/// by byte value. The returned slice references static storage
/// and is stable for the program's lifetime.
pub fn table(encoding: Encoding) -> &'static [&'static str; 256] {
    match encoding {
        Encoding::Standard => data::STANDARD,
        Encoding::MacRoman => data::MAC_ROMAN,
        Encoding::WinAnsi  => data::WIN_ANSI,
        Encoding::PdfDoc   => data::PDF_DOC,
    }
}

/// Parse a PDF /Encoding name to an [`Encoding`] enumerant.
/// Returns `None` for any other input.
pub fn from_name(name: &str) -> Option<Encoding> {
    match name {
        "StandardEncoding" => Some(Encoding::Standard),
        "MacRomanEncoding" => Some(Encoding::MacRoman),
        "WinAnsiEncoding"  => Some(Encoding::WinAnsi),
        "PDFDocEncoding"   => Some(Encoding::PdfDoc),
        _                  => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------
    // StandardEncoding spot-checks (PDF Annex D Table D.2 col 1).
    // -----------------------------------------------------------

    #[test]
    fn standard_ascii_letters() {
        assert_eq!(byte_to_glyph_name(Encoding::Standard, b'A'), "A");
        assert_eq!(byte_to_glyph_name(Encoding::Standard, b'a'), "a");
        assert_eq!(byte_to_glyph_name(Encoding::Standard, b' '), "space");
    }

    #[test]
    fn standard_punctuation() {
        // 0x27 is quoteright (Annex D col 1) — NOT quotesingle.
        // The Type1 default uses curly variants for ASCII quotes.
        assert_eq!(byte_to_glyph_name(Encoding::Standard, 0x27), "quoteright");
        assert_eq!(byte_to_glyph_name(Encoding::Standard, 0x60), "quoteleft");
    }

    #[test]
    fn standard_sparse_high_byte_extras() {
        // StandardEncoding's high half is sparse — most slots
        // are unmapped, but a handful carry rare names per
        // Annex D (e.g. Oslash at 0xE9, OE at 0xEA).
        assert_eq!(byte_to_glyph_name(Encoding::Standard, 0xE0), "");
        assert_eq!(byte_to_glyph_name(Encoding::Standard, 0xE9), "Oslash");
        assert_eq!(byte_to_glyph_name(Encoding::Standard, 0xEA), "OE");
    }

    // -----------------------------------------------------------
    // MacRomanEncoding spot-checks.
    // -----------------------------------------------------------

    #[test]
    fn macroman_ascii_letters() {
        assert_eq!(byte_to_glyph_name(Encoding::MacRoman, b'A'), "A");
        assert_eq!(byte_to_glyph_name(Encoding::MacRoman, b'z'), "z");
    }

    #[test]
    fn macroman_extended_latin() {
        // MacRoman packs uppercase accented Latin in 0x80..=0x86,
        // lowercase in 0x87..=0x9F.
        assert_eq!(byte_to_glyph_name(Encoding::MacRoman, 0x80), "Adieresis");
        assert_eq!(byte_to_glyph_name(Encoding::MacRoman, 0x81), "Aring");
        assert_eq!(byte_to_glyph_name(Encoding::MacRoman, 0x88), "agrave");
        // MacRoman 0xCA is non-breaking space — pdfminer's source
        // lists both "nbspace" and "space" at 0xCA in turn; the
        // last-wins resolution picks one of the two.
        let cap_ca = byte_to_glyph_name(Encoding::MacRoman, 0xCA);
        assert!(cap_ca == "space" || cap_ca == "nbspace",
                "MacRoman 0xCA expected space/nbspace, got {cap_ca:?}");
    }

    #[test]
    fn macroman_apple_logo_unmapped() {
        // 0xF0 is the Apple logo; not part of PDF Annex D.
        assert_eq!(byte_to_glyph_name(Encoding::MacRoman, 0xF0), "");
    }

    // -----------------------------------------------------------
    // WinAnsiEncoding spot-checks.
    // -----------------------------------------------------------

    #[test]
    fn winansi_ascii_letters() {
        assert_eq!(byte_to_glyph_name(Encoding::WinAnsi, b'A'), "A");
        assert_eq!(byte_to_glyph_name(Encoding::WinAnsi, b'9'), "nine");
    }

    #[test]
    fn winansi_quotesingle_at_27() {
        // WinAnsi diverges from Standard at 0x27 — straight
        // quote, not curly.
        assert_eq!(byte_to_glyph_name(Encoding::WinAnsi, 0x27), "quotesingle");
    }

    #[test]
    fn winansi_smart_quotes() {
        assert_eq!(byte_to_glyph_name(Encoding::WinAnsi, 0x91), "quoteleft");
        assert_eq!(byte_to_glyph_name(Encoding::WinAnsi, 0x92), "quoteright");
    }

    #[test]
    fn winansi_extended_latin() {
        assert_eq!(byte_to_glyph_name(Encoding::WinAnsi, 0xC1), "Aacute");
        assert_eq!(byte_to_glyph_name(Encoding::WinAnsi, 0xE9), "eacute");
    }

    #[test]
    fn winansi_undefined_slot_9d() {
        // PDF 32000 Annex D explicitly leaves 0x9D unmapped.
        assert_eq!(byte_to_glyph_name(Encoding::WinAnsi, 0x9D), "");
    }

    // -----------------------------------------------------------
    // PDFDocEncoding spot-checks.
    // -----------------------------------------------------------

    #[test]
    fn pdfdoc_ascii_letters() {
        assert_eq!(byte_to_glyph_name(Encoding::PdfDoc, b'A'), "A");
    }

    #[test]
    fn pdfdoc_extended_latin() {
        assert_eq!(byte_to_glyph_name(Encoding::PdfDoc, 0xC1), "Aacute");
    }

    // -----------------------------------------------------------
    // table() shape + from_name() parse.
    // -----------------------------------------------------------

    #[test]
    fn table_returns_256_entries() {
        for enc in [Encoding::Standard, Encoding::MacRoman,
                    Encoding::WinAnsi, Encoding::PdfDoc] {
            let t = table(enc);
            assert_eq!(t.len(), 256, "{enc:?} table size != 256");
        }
    }

    #[test]
    fn table_indexed_matches_byte_to_glyph_name() {
        for enc in [Encoding::Standard, Encoding::MacRoman,
                    Encoding::WinAnsi, Encoding::PdfDoc] {
            let t = table(enc);
            for code in 0u8..=255 {
                assert_eq!(t[code as usize],
                           byte_to_glyph_name(enc, code),
                           "{enc:?} mismatch at 0x{code:02X}");
            }
        }
    }

    #[test]
    fn from_name_canonical_strings() {
        assert_eq!(from_name("StandardEncoding"),
                   Some(Encoding::Standard));
        assert_eq!(from_name("MacRomanEncoding"),
                   Some(Encoding::MacRoman));
        assert_eq!(from_name("WinAnsiEncoding"),
                   Some(Encoding::WinAnsi));
        assert_eq!(from_name("PDFDocEncoding"),
                   Some(Encoding::PdfDoc));
    }

    #[test]
    fn from_name_rejects_unknown() {
        assert_eq!(from_name("MacExpertEncoding"), None);
        assert_eq!(from_name("SymbolEncoding"), None);
        assert_eq!(from_name(""), None);
        assert_eq!(from_name("standardencoding"), None);  // case-sensitive
    }

    #[test]
    fn null_byte_is_unmapped_in_all_encodings() {
        for enc in [Encoding::Standard, Encoding::MacRoman,
                    Encoding::WinAnsi, Encoding::PdfDoc] {
            assert_eq!(byte_to_glyph_name(enc, 0x00), "",
                       "{enc:?} 0x00 should be unmapped");
        }
    }

    #[test]
    fn ascii_digit_glyph_names_uniform() {
        for enc in [Encoding::Standard, Encoding::MacRoman,
                    Encoding::WinAnsi, Encoding::PdfDoc] {
            assert_eq!(byte_to_glyph_name(enc, b'0'), "zero");
            assert_eq!(byte_to_glyph_name(enc, b'5'), "five");
            assert_eq!(byte_to_glyph_name(enc, b'9'), "nine");
        }
    }
}
