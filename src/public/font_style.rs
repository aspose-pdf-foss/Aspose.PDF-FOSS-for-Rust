//! `Aspose.Pdf.Facades.FontStyle` — a built-in form-field font, per
//! `capabilities/font_style.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0
//! `Aspose.Pdf.Facades.FontStyle` enum.

/// A built-in form-field font style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum FontStyle {
    /// Courier.
    #[default]
    Courier = 0,
    /// Courier Bold.
    CourierBold = 1,
    /// Courier Oblique.
    CourierOblique = 2,
    /// Courier Bold-Oblique.
    CourierBoldOblique = 3,
    /// Helvetica.
    Helvetica = 4,
    /// Helvetica Bold.
    HelveticaBold = 5,
    /// Helvetica Oblique.
    HelveticaOblique = 6,
    /// Helvetica Bold-Oblique.
    HelveticaBoldOblique = 7,
    /// Symbol.
    Symbol = 8,
    /// Times Roman.
    TimesRoman = 9,
    /// Times Bold.
    TimesBold = 10,
    /// Times Italic.
    TimesItalic = 11,
    /// Times Bold-Italic.
    TimesBoldItalic = 12,
    /// ZapfDingbats.
    ZapfDingbats = 13,
    /// Unknown.
    Unknown = 14,
    /// A CJK font.
    CjkFont = 15,
}

impl FontStyle {
    /// The Standard 14 `/BaseFont` name this style is (Annex D.1), or
    /// `None` for the two that name no such face.
    pub(crate) fn base_font_name(self) -> Option<&'static str> {
        match self {
            FontStyle::Courier => Some("Courier"),
            FontStyle::CourierBold => Some("Courier-Bold"),
            FontStyle::CourierOblique => Some("Courier-Oblique"),
            FontStyle::CourierBoldOblique => Some("Courier-BoldOblique"),
            FontStyle::Helvetica => Some("Helvetica"),
            FontStyle::HelveticaBold => Some("Helvetica-Bold"),
            FontStyle::HelveticaOblique => Some("Helvetica-Oblique"),
            FontStyle::HelveticaBoldOblique => Some("Helvetica-BoldOblique"),
            FontStyle::Symbol => Some("Symbol"),
            FontStyle::TimesRoman => Some("Times-Roman"),
            FontStyle::TimesBold => Some("Times-Bold"),
            FontStyle::TimesItalic => Some("Times-Italic"),
            FontStyle::TimesBoldItalic => Some("Times-BoldItalic"),
            FontStyle::ZapfDingbats => Some("ZapfDingbats"),
            // Neither names a Standard 14 face: Unknown names none at
            // all, and a CJK font is one this crate has no bundled
            // substitute for.
            FontStyle::Unknown | FontStyle::CjkFont => None,
        }
    }
}
