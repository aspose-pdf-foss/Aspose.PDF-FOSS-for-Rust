//! `Aspose.Pdf.PdfFormat` — the target format / version a document is
//! converted to, per `capabilities/pdf_format.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0
//! `Aspose.Pdf.PdfFormat` enum.

/// A PDF conformance format or version.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum PdfFormat {
    /// PDF/A-1a.
    #[default]
    PDF_A_1A = 0,
    /// PDF/A-1b.
    PDF_A_1B = 1,
    /// PDF/A-2a.
    PDF_A_2A = 2,
    /// PDF/A-3a.
    PDF_A_3A = 3,
    /// PDF/A-2b.
    PDF_A_2B = 4,
    /// PDF/A-2u.
    PDF_A_2U = 5,
    /// PDF/A-3b.
    PDF_A_3B = 6,
    /// PDF/A-3u.
    PDF_A_3U = 7,
    /// PDF 1.0.
    v_1_0 = 8,
    /// PDF 1.1.
    v_1_1 = 9,
    /// PDF 1.2.
    v_1_2 = 10,
    /// PDF 1.3.
    v_1_3 = 11,
    /// PDF 1.4.
    v_1_4 = 12,
    /// PDF 1.5.
    v_1_5 = 13,
    /// PDF 1.6.
    v_1_6 = 14,
    /// PDF 1.7.
    v_1_7 = 15,
    /// PDF 2.0.
    v_2_0 = 16,
    /// PDF/UA-1.
    PDF_UA_1 = 17,
    /// PDF/X-1a:2001.
    PDF_X_1A_2001 = 18,
    /// PDF/X-1a.
    PDF_X_1A = 19,
    /// PDF/X-3.
    PDF_X_3 = 20,
    /// ZUGFeRD.
    ZUGFeRD = 21,
    /// PDF/A-4.
    PDF_A_4 = 22,
    /// PDF/A-4e.
    PDF_A_4E = 23,
    /// PDF/A-4f.
    PDF_A_4F = 24,
    /// PDF/X-4.
    PDF_X_4 = 25,
    /// PDF/E-1.
    PDF_E_1 = 26,
}
