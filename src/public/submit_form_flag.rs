//! `Aspose.Pdf.Facades.SubmitFormFlag` — the export format of a
//! submit-form action, per `capabilities/submit_form_flag.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0
//! `Aspose.Pdf.Facades.SubmitFormFlag` enum.

/// A submit-form export format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum SubmitFormFlag {
    /// Forms Data Format.
    #[default]
    Fdf = 0,
    /// HTML.
    Html = 1,
    /// XML Forms Data Format.
    Xfdf = 2,
    /// FDF including comments.
    FdfWithComments = 3,
    /// XFDF including comments.
    XfdfWithComments = 4,
    /// The full PDF.
    Pdf = 5,
}

impl SubmitFormFlag {
    /// The `/Flags` value a `SubmitForm` action carries for this
    /// submission format (§12.7.5.2 Table 237). Bit numbering there is
    /// 1-based, so bit *n* is `1 << (n - 1)`:
    ///
    /// - **ExportFormat**, bit 3 — submit as URL-encoded HTML rather
    ///   than as FDF.
    /// - **XFDF**, bit 6 — submit as XFDF.
    /// - **IncludeAnnotations**, bit 8 — carry the comments along.
    /// - **SubmitPDF**, bit 9 — submit the whole document.
    ///
    /// FDF is the format an all-zero `/Flags` already means, which is
    /// why it maps to nothing.
    pub(crate) fn pdf_flags(self) -> i64 {
        const EXPORT_FORMAT: i64 = 1 << 2;
        const XFDF: i64 = 1 << 5;
        const INCLUDE_ANNOTATIONS: i64 = 1 << 7;
        const SUBMIT_PDF: i64 = 1 << 8;
        match self {
            SubmitFormFlag::Fdf => 0,
            SubmitFormFlag::Html => EXPORT_FORMAT,
            SubmitFormFlag::Xfdf => XFDF,
            SubmitFormFlag::FdfWithComments => INCLUDE_ANNOTATIONS,
            SubmitFormFlag::XfdfWithComments => XFDF | INCLUDE_ANNOTATIONS,
            SubmitFormFlag::Pdf => SUBMIT_PDF,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_format_names_its_table_237_bits() {
        assert_eq!(SubmitFormFlag::Fdf.pdf_flags(), 0, "FDF is the default");
        assert_eq!(SubmitFormFlag::Html.pdf_flags(), 4, "ExportFormat, bit 3");
        assert_eq!(SubmitFormFlag::Xfdf.pdf_flags(), 32, "XFDF, bit 6");
        assert_eq!(
            SubmitFormFlag::FdfWithComments.pdf_flags(),
            128,
            "IncludeAnnotations, bit 8"
        );
        assert_eq!(
            SubmitFormFlag::XfdfWithComments.pdf_flags(),
            160,
            "XFDF and IncludeAnnotations together"
        );
        assert_eq!(SubmitFormFlag::Pdf.pdf_flags(), 256, "SubmitPDF, bit 9");
    }
}
