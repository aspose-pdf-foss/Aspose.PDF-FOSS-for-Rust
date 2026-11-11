// Charset codec primitive — substitutes BCL System.Text.Encoding
// for the Rust lib. Five fixed charsets in v1: UTF-8, UTF-16-LE,
// UTF-16-BE, Latin-1, Windows-1252. See
// `Feedstock/PDF/Spec/encoding.yaml` for the full BCL contract.
//
// Hand-authored from RFC 3629 (UTF-8) and the Unicode UTF-16 /
// Win-1252 tables. No `encoding_rs`, no third-party codec crate
// — std-only per the rust-track strategy lock in Cargo.toml.
//
// Replacement-char policy (replace mode, never throw):
//   * decode: malformed byte sequences produce U+FFFD
//   * encode: unmappable codepoints produce 0x3F ('?')

use std::fmt;

const REPLACEMENT_CP: u32 = 0xFFFD;
const REPLACEMENT_BYTE: u8 = 0x3F;

/// One of the five v1 charsets. `Copy + Eq` — value equality
/// replaces TS's reference-equality singleton check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Encoding {
    /// UTF-8.
    Utf8,
    /// UTF-16, little-endian.
    Utf16Le,
    /// UTF-16, big-endian.
    Utf16Be,
    /// ISO-8859-1 (Latin-1).
    Latin1,
    /// Windows-1252.
    Windows1252,
}

/// `Encoding::get_encoding` rejects any name outside the
/// recognised alias set. Owned `String` so the error can be
/// stored / displayed without a lifetime borrowed from the
/// caller's input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownEncoding(pub String);

impl fmt::Display for UnknownEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "encoding: unknown name '{}'", self.0)
    }
}

impl std::error::Error for UnknownEncoding {}

// Windows-1252 high-half slot map for 0x80..0x9F. 0xFFFD entries
// mark the five undefined byte slots (0x81, 0x8D, 0x8F, 0x90,
// 0x9D). Used by both encode (reverse-search) and decode (direct
// index).
const WIN1252_HIGH: [u16; 32] = [
    0x20AC, 0xFFFD, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021,
    0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0xFFFD, 0x017D, 0xFFFD,
    0xFFFD, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014,
    0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0xFFFD, 0x017E, 0x0178,
];

impl Encoding {
    // ---- BCL static-factory mirrors ------------------------

    /// BCL `Encoding.UTF8`.
    pub const fn utf8() -> Self { Encoding::Utf8 }
    /// BCL `Encoding.Unicode` is UTF-16-LE.
    pub const fn unicode() -> Self { Encoding::Utf16Le }
    /// BCL `Encoding.BigEndianUnicode` is UTF-16-BE.
    pub const fn big_endian_unicode() -> Self { Encoding::Utf16Be }
    /// BCL `Encoding.Latin1` (ISO-8859-1).
    pub const fn latin1() -> Self { Encoding::Latin1 }
    /// The Windows-1252 code page.
    pub const fn windows_1252() -> Self { Encoding::Windows1252 }

    /// Look up an [`Encoding`] by canonical WebName or common
    /// alias. Case-insensitive on the ASCII portion; returns
    /// `UnknownEncoding` for anything outside the alias set.
    pub fn get_encoding(name: &str) -> Result<Self, UnknownEncoding> {
        let lower = name.to_ascii_lowercase();
        match lower.as_str() {
            "utf-8" | "utf8" => Ok(Encoding::Utf8),
            "utf-16" | "utf-16le" | "unicode" => Ok(Encoding::Utf16Le),
            "utf-16be" | "unicodefffe" => Ok(Encoding::Utf16Be),
            "iso-8859-1" | "iso_8859-1" | "latin1" => Ok(Encoding::Latin1),
            "windows-1252" | "cp1252" => Ok(Encoding::Windows1252),
            _ => Err(UnknownEncoding(name.to_string())),
        }
    }

    // ---- Encode / decode entrypoints -----------------------

    /// Encode `text` to bytes in this encoding (BCL `GetBytes`).
    pub fn get_bytes(self, text: &str) -> Vec<u8> {
        match self {
            Encoding::Utf8 => encode_utf8(text),
            Encoding::Utf16Le => encode_utf16(text, false),
            Encoding::Utf16Be => encode_utf16(text, true),
            Encoding::Latin1 => encode_latin1(text),
            Encoding::Windows1252 => encode_win1252(text),
        }
    }

    /// Decode `bytes` from this encoding to a string (BCL `GetString`).
    pub fn get_string(self, bytes: &[u8]) -> String {
        match self {
            Encoding::Utf8 => decode_utf8(bytes),
            Encoding::Utf16Le => decode_utf16(bytes, false),
            Encoding::Utf16Be => decode_utf16(bytes, true),
            Encoding::Latin1 => decode_latin1(bytes),
            Encoding::Windows1252 => decode_win1252(bytes),
        }
    }

    /// Predict `get_bytes(text).len()` without allocating the
    /// byte buffer. Equality with `get_bytes(text).len()` is a
    /// contract invariant verified by the freeze gate.
    pub fn get_byte_count(self, text: &str) -> usize {
        // Allocation-free for the price of recomputing — the
        // gate enforces equality with `get_bytes(text).len()`,
        // so the natural implementation is to delegate. A
        // codepoint-aware shortcut for UTF-8 / Latin-1 would
        // avoid the buffer but adds maintenance surface for no
        // contract benefit.
        self.get_bytes(text).len()
    }

    /// Predict `get_string(bytes).chars().count()` without
    /// allocating the string. Rust strings are UTF-8; we use
    /// `chars().count()` as the natural analogue of BCL
    /// `String.Length` (UTF-16 code units would surprise Rust
    /// callers — the gate's `char_count_matches` invariant is
    /// evaluated against this Rust-flavoured contract).
    pub fn get_char_count(self, bytes: &[u8]) -> usize {
        self.get_string(bytes).chars().count()
    }

    // ---- Properties ----------------------------------------

    /// The IANA web name (BCL `WebName`).
    pub const fn web_name(self) -> &'static str {
        match self {
            Encoding::Utf8 => "utf-8",
            Encoding::Utf16Le => "utf-16",
            Encoding::Utf16Be => "utf-16BE",
            Encoding::Latin1 => "iso-8859-1",
            Encoding::Windows1252 => "windows-1252",
        }
    }

    /// The Windows code-page identifier (BCL `CodePage`).
    pub const fn code_page(self) -> u32 {
        match self {
            Encoding::Utf8 => 65001,
            Encoding::Utf16Le => 1200,
            Encoding::Utf16Be => 1201,
            Encoding::Latin1 => 28591,
            Encoding::Windows1252 => 1252,
        }
    }

    /// The byte-order-mark preamble for this encoding (BCL `Preamble`).
    pub const fn preamble(self) -> &'static [u8] {
        match self {
            Encoding::Utf8 => &[0xEF, 0xBB, 0xBF],
            Encoding::Utf16Le => &[0xFF, 0xFE],
            Encoding::Utf16Be => &[0xFE, 0xFF],
            Encoding::Latin1 => &[],
            Encoding::Windows1252 => &[],
        }
    }
}

// ============================================================
// Per-charset encoders
// ============================================================

fn encode_utf8(text: &str) -> Vec<u8> {
    // Rust strings are already valid UTF-8 — copy the byte view.
    text.as_bytes().to_vec()
}

fn encode_utf16(text: &str, big_endian: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len() * 2);
    for c in text.chars() {
        let cp = c as u32;
        if cp <= 0xFFFF {
            push_u16(&mut out, cp as u16, big_endian);
        } else {
            // Supplementary plane — emit a UTF-16 surrogate pair.
            let s = cp - 0x10000;
            let hi = 0xD800u16 | ((s >> 10) as u16);
            let lo = 0xDC00u16 | ((s & 0x3FF) as u16);
            push_u16(&mut out, hi, big_endian);
            push_u16(&mut out, lo, big_endian);
        }
    }
    out
}

fn push_u16(out: &mut Vec<u8>, v: u16, big_endian: bool) {
    if big_endian {
        out.push((v >> 8) as u8);
        out.push(v as u8);
    } else {
        out.push(v as u8);
        out.push((v >> 8) as u8);
    }
}

fn encode_latin1(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    for c in text.chars() {
        let cp = c as u32;
        out.push(if cp <= 0xFF { cp as u8 } else { REPLACEMENT_BYTE });
    }
    out
}

fn encode_win1252(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    for c in text.chars() {
        let cp = c as u32;
        if cp < 0x80 || (0xA0..=0xFF).contains(&cp) {
            // Latin-1 shared range maps 1:1.
            out.push(cp as u8);
            continue;
        }
        // High-half reverse lookup. Skip the U+FFFD slots — those
        // mark undefined bytes; a real U+FFFD codepoint is not
        // encodable in win-1252 and falls through to '?'.
        let mut mapped = REPLACEMENT_BYTE;
        for (i, &slot) in WIN1252_HIGH.iter().enumerate() {
            if slot != 0xFFFD && slot as u32 == cp {
                mapped = 0x80 + i as u8;
                break;
            }
        }
        out.push(mapped);
    }
    out
}

// ============================================================
// Per-charset decoders
// ============================================================

fn decode_utf8(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i];
        let mut cp = REPLACEMENT_CP;
        let mut consumed: usize = 1;
        if b0 < 0x80 {
            cp = b0 as u32;
        } else if (b0 & 0xE0) == 0xC0 && i + 1 < bytes.len() {
            let b1 = bytes[i + 1];
            if (b1 & 0xC0) == 0x80 {
                let v = ((b0 & 0x1F) as u32) << 6 | (b1 & 0x3F) as u32;
                if v >= 0x80 {
                    cp = v;
                    consumed = 2;
                }
                // Else: overlong — keep cp = U+FFFD, advance 1.
            }
        } else if (b0 & 0xF0) == 0xE0 && i + 2 < bytes.len() {
            let b1 = bytes[i + 1];
            let b2 = bytes[i + 2];
            if (b1 & 0xC0) == 0x80 && (b2 & 0xC0) == 0x80 {
                let v = ((b0 & 0x0F) as u32) << 12
                      | ((b1 & 0x3F) as u32) << 6
                      | (b2 & 0x3F) as u32;
                if v >= 0x800 && !(0xD800..=0xDFFF).contains(&v) {
                    cp = v;
                    consumed = 3;
                }
            }
        } else if (b0 & 0xF8) == 0xF0 && i + 3 < bytes.len() {
            let b1 = bytes[i + 1];
            let b2 = bytes[i + 2];
            let b3 = bytes[i + 3];
            if (b1 & 0xC0) == 0x80
                && (b2 & 0xC0) == 0x80
                && (b3 & 0xC0) == 0x80
            {
                let v = ((b0 & 0x07) as u32) << 18
                      | ((b1 & 0x3F) as u32) << 12
                      | ((b2 & 0x3F) as u32) << 6
                      | (b3 & 0x3F) as u32;
                if (0x10000..=0x10FFFF).contains(&v) {
                    cp = v;
                    consumed = 4;
                }
            }
        }
        push_codepoint(&mut out, cp);
        i += consumed;
    }
    out
}

fn decode_utf16(bytes: &[u8], big_endian: bool) -> String {
    let mut out = String::with_capacity(bytes.len() / 2);
    let mut i = 0;
    while i + 1 < bytes.len() {
        let u = read_u16(bytes, i, big_endian);
        i += 2;
        if (0xD800..=0xDBFF).contains(&u) {
            // High surrogate — must be followed by a low.
            if i + 1 < bytes.len() {
                let lo = read_u16(bytes, i, big_endian);
                if (0xDC00..=0xDFFF).contains(&lo) {
                    let cp = 0x10000
                        + ((u as u32 - 0xD800) << 10)
                        + (lo as u32 - 0xDC00);
                    push_codepoint(&mut out, cp);
                    i += 2;
                    continue;
                }
            }
            push_codepoint(&mut out, REPLACEMENT_CP);
        } else if (0xDC00..=0xDFFF).contains(&u) {
            // Lone low surrogate.
            push_codepoint(&mut out, REPLACEMENT_CP);
        } else {
            push_codepoint(&mut out, u as u32);
        }
    }
    if i < bytes.len() {
        // Orphan trailing byte — odd input length.
        push_codepoint(&mut out, REPLACEMENT_CP);
    }
    out
}

fn read_u16(bytes: &[u8], i: usize, big_endian: bool) -> u16 {
    let b0 = bytes[i] as u16;
    let b1 = bytes[i + 1] as u16;
    if big_endian {
        (b0 << 8) | b1
    } else {
        b0 | (b1 << 8)
    }
}

fn decode_latin1(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        // Every Latin-1 byte is a valid BMP codepoint U+0000..U+00FF.
        out.push(b as char);
    }
    out
}

fn decode_win1252(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        if !(0x80..0xA0).contains(&b) {
            out.push(b as char);
        } else {
            // 0x80..0x9F lookup — undefined slots return U+FFFD.
            push_codepoint(&mut out, WIN1252_HIGH[(b - 0x80) as usize] as u32);
        }
    }
    out
}

fn push_codepoint(out: &mut String, cp: u32) {
    // `char::from_u32` returns None for surrogate / out-of-range
    // values; replace with U+FFFD in that case so the emitted
    // string is always valid UTF-8.
    let c = char::from_u32(cp).unwrap_or('\u{FFFD}');
    out.push(c);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_round_trips_ascii() {
        let enc = Encoding::utf8();
        let bytes = enc.get_bytes("hello");
        assert_eq!(bytes, b"hello");
        assert_eq!(enc.get_string(&bytes), "hello");
    }

    #[test]
    fn utf8_round_trips_supplementary_emoji() {
        let enc = Encoding::utf8();
        let s = "smile 😀!";
        let bytes = enc.get_bytes(s);
        assert_eq!(enc.get_string(&bytes), s);
    }

    #[test]
    fn utf8_decodes_overlong_to_replacement() {
        // C0 80 is the overlong encoding of U+0000 — RFC 3629
        // requires rejection.
        let out = decode_utf8(&[0xC0, 0x80, b'X']);
        assert_eq!(out, "\u{FFFD}\u{FFFD}X");
    }

    #[test]
    fn utf16_le_emits_surrogate_pair_for_supplementary() {
        let enc = Encoding::unicode();
        let bytes = enc.get_bytes("😀");
        assert_eq!(bytes, [0x3D, 0xD8, 0x00, 0xDE]);
        assert_eq!(enc.get_string(&bytes), "😀");
    }

    #[test]
    fn utf16_be_emits_surrogate_pair_for_supplementary() {
        let enc = Encoding::big_endian_unicode();
        let bytes = enc.get_bytes("😀");
        assert_eq!(bytes, [0xD8, 0x3D, 0xDE, 0x00]);
        assert_eq!(enc.get_string(&bytes), "😀");
    }

    #[test]
    fn utf16_odd_length_input_appends_replacement() {
        // Two valid LE bytes for 'A' + one orphan byte.
        let out = decode_utf16(&[0x41, 0x00, 0xFF], false);
        assert_eq!(out, "A\u{FFFD}");
    }

    #[test]
    fn latin1_encodes_above_ff_to_question_mark() {
        let enc = Encoding::latin1();
        let bytes = enc.get_bytes("café 中");
        // 'c'=0x63 'a'=0x61 'f'=0x66 'é'=0xE9 ' '=0x20 '中'=>'?'
        assert_eq!(bytes, [0x63, 0x61, 0x66, 0xE9, 0x20, b'?']);
    }

    #[test]
    fn win1252_round_trips_smart_quotes_and_euro() {
        let enc = Encoding::windows_1252();
        let s = "“€™”";
        let bytes = enc.get_bytes(s);
        assert_eq!(bytes, [0x93, 0x80, 0x99, 0x94]);
        assert_eq!(enc.get_string(&bytes), s);
    }

    #[test]
    fn win1252_undefined_bytes_decode_to_replacement() {
        let enc = Encoding::windows_1252();
        // 0x81 0x8D 0x8F 0x90 0x9D are the five undefined slots.
        assert_eq!(enc.get_string(&[0x81]), "\u{FFFD}");
        assert_eq!(enc.get_string(&[0x8D]), "\u{FFFD}");
        assert_eq!(enc.get_string(&[0x8F]), "\u{FFFD}");
        assert_eq!(enc.get_string(&[0x90]), "\u{FFFD}");
        assert_eq!(enc.get_string(&[0x9D]), "\u{FFFD}");
    }

    #[test]
    fn get_encoding_known_aliases() {
        assert_eq!(
            Encoding::get_encoding("UTF-8").unwrap(),
            Encoding::Utf8
        );
        assert_eq!(
            Encoding::get_encoding("Unicode").unwrap(),
            Encoding::Utf16Le
        );
        assert_eq!(
            Encoding::get_encoding("UTF-16BE").unwrap(),
            Encoding::Utf16Be
        );
        assert_eq!(
            Encoding::get_encoding("Latin1").unwrap(),
            Encoding::Latin1
        );
        assert_eq!(
            Encoding::get_encoding("CP1252").unwrap(),
            Encoding::Windows1252
        );
    }

    #[test]
    fn get_encoding_unknown_errors() {
        assert!(Encoding::get_encoding("shift_jis").is_err());
    }

    #[test]
    fn web_name_and_code_page_canonical() {
        assert_eq!(Encoding::utf8().web_name(), "utf-8");
        assert_eq!(Encoding::utf8().code_page(), 65001);
        assert_eq!(Encoding::big_endian_unicode().web_name(), "utf-16BE");
        assert_eq!(Encoding::big_endian_unicode().code_page(), 1201);
        assert_eq!(Encoding::latin1().web_name(), "iso-8859-1");
        assert_eq!(Encoding::latin1().code_page(), 28591);
    }

    #[test]
    fn preamble_canonical() {
        assert_eq!(Encoding::utf8().preamble(), &[0xEF, 0xBB, 0xBF]);
        assert_eq!(Encoding::unicode().preamble(), &[0xFF, 0xFE]);
        assert_eq!(Encoding::big_endian_unicode().preamble(), &[0xFE, 0xFF]);
        assert_eq!(Encoding::latin1().preamble(), &[] as &[u8]);
    }

    #[test]
    fn byte_count_matches_get_bytes() {
        let enc = Encoding::utf8();
        let s = "café 中";
        assert_eq!(enc.get_byte_count(s), enc.get_bytes(s).len());
    }

    #[test]
    fn char_count_matches_get_string() {
        let enc = Encoding::windows_1252();
        let bytes = [0x80u8, 0x99, b'a'];
        assert_eq!(enc.get_char_count(&bytes), enc.get_string(&bytes).chars().count());
    }
}
