//! `Aspose.Pdf.NumberingStyle` — the numbering style of a page
//! label, per `capabilities/numbering_style.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0
//! `Aspose.Pdf.NumberingStyle` enum.

/// The numbering style applied to a page-label range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum NumberingStyle {
    /// Arabic numerals (`1`, `2`, `3`, …).
    #[default]
    NumeralsArabic = 0,
    /// Uppercase Roman numerals (`I`, `II`, `III`, …).
    NumeralsRomanUppercase = 1,
    /// Lowercase Roman numerals (`i`, `ii`, `iii`, …).
    NumeralsRomanLowercase = 2,
    /// Uppercase letters (`A`, `B`, …, `AA`, …).
    LettersUppercase = 3,
    /// Lowercase letters (`a`, `b`, …, `aa`, …).
    LettersLowercase = 4,
    /// No numbering (prefix only).
    None = 5,
}

impl NumberingStyle {
    /// The PDF `/S` page-label style name (`D`/`R`/`r`/`A`/`a`), or
    /// `None` for [`NumberingStyle::None`] (no `/S` key).
    pub(crate) fn pdf_style_char(self) -> Option<&'static str> {
        match self {
            NumberingStyle::NumeralsArabic => Some("D"),
            NumberingStyle::NumeralsRomanUppercase => Some("R"),
            NumberingStyle::NumeralsRomanLowercase => Some("r"),
            NumberingStyle::LettersUppercase => Some("A"),
            NumberingStyle::LettersLowercase => Some("a"),
            NumberingStyle::None => None,
        }
    }

    /// `n` written in this style, the way §12.4.2 defines the page
    /// label styles a viewer renders: decimal digits, upper- or
    /// lower-case roman, or letters `A`–`Z` then `AA`–`ZZ` and on.
    /// [`None`](Self::None) has no numbering at all, so it renders
    /// nothing — a label is then its prefix alone.
    ///
    /// A viewer does this itself for page *labels*, which are declared
    /// (`/S`) rather than drawn. A stamped number is drawn, so the
    /// stamp has to render it here.
    pub(crate) fn format_number(self, n: i32) -> String {
        match self {
            NumberingStyle::NumeralsArabic => n.to_string(),
            NumberingStyle::NumeralsRomanUppercase => roman(n),
            NumberingStyle::NumeralsRomanLowercase => roman(n).to_lowercase(),
            NumberingStyle::LettersUppercase => letters(n),
            NumberingStyle::LettersLowercase => letters(n).to_lowercase(),
            NumberingStyle::None => String::new(),
        }
    }

    /// The numbering style for a PDF `/S` name (inverse of
    /// [`pdf_style_char`](Self::pdf_style_char); absent / unknown →
    /// [`NumberingStyle::None`]).
    pub(crate) fn from_pdf_style_char(s: Option<&str>) -> NumberingStyle {
        match s {
            Some("D") => NumberingStyle::NumeralsArabic,
            Some("R") => NumberingStyle::NumeralsRomanUppercase,
            Some("r") => NumberingStyle::NumeralsRomanLowercase,
            Some("A") => NumberingStyle::LettersUppercase,
            Some("a") => NumberingStyle::LettersLowercase,
            _ => NumberingStyle::None,
        }
    }
}

/// `n` as an upper-case roman numeral. Zero and below have no roman
/// form, so they render as decimal rather than as nothing.
fn roman(n: i32) -> String {
    if n <= 0 {
        return n.to_string();
    }
    const VALUES: [(i32, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut left = n;
    let mut out = String::new();
    for (value, numeral) in VALUES {
        while left >= value {
            out.push_str(numeral);
            left -= value;
        }
    }
    out
}

/// `n` as the §12.4.2 letter sequence: `A`–`Z`, then `AA`–`ZZ`, then
/// `AAA`–`ZZZ`. Zero and below render as decimal, having no letter.
fn letters(n: i32) -> String {
    if n <= 0 {
        return n.to_string();
    }
    let index = (n - 1) % 26;
    let repeats = ((n - 1) / 26) as usize + 1;
    let letter = (b'A' + index as u8) as char;
    std::iter::repeat_n(letter, repeats).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roman_numerals_follow_the_subtractive_forms() {
        assert_eq!(roman(1), "I");
        assert_eq!(roman(4), "IV");
        assert_eq!(roman(9), "IX");
        assert_eq!(roman(14), "XIV");
        assert_eq!(roman(40), "XL");
        assert_eq!(roman(1990), "MCMXC");
        assert_eq!(roman(2026), "MMXXVI");
    }

    #[test]
    fn letters_repeat_once_per_full_alphabet() {
        assert_eq!(letters(1), "A");
        assert_eq!(letters(26), "Z");
        assert_eq!(letters(27), "AA");
        assert_eq!(letters(52), "ZZ");
        assert_eq!(letters(53), "AAA");
    }

    #[test]
    fn a_style_renders_a_number_its_own_way() {
        assert_eq!(NumberingStyle::NumeralsArabic.format_number(7), "7");
        assert_eq!(NumberingStyle::NumeralsRomanLowercase.format_number(7), "vii");
        assert_eq!(NumberingStyle::LettersUppercase.format_number(7), "G");
        assert_eq!(NumberingStyle::None.format_number(7), "");
    }
}
