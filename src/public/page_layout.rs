//! `Aspose.Pdf.PageLayout` — the page layout used when the document is
//! opened, per `capabilities/page_layout.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The page layout the viewer uses on open (the catalog's
/// `/PageLayout`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum PageLayout {
    /// One page at a time.
    SinglePage = 0,
    /// Pages in one continuous column.
    OneColumn = 1,
    /// Pages in two columns, odd-numbered on the left.
    TwoColumnLeft = 2,
    /// Pages in two columns, odd-numbered on the right.
    TwoColumnRight = 3,
    /// Two pages side by side, odd-numbered on the left.
    TwoPageLeft = 4,
    /// Two pages side by side, odd-numbered on the right.
    TwoPageRight = 5,
    /// Layout unset — the viewer's default is used.
    #[default]
    Default = 6,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals_match_canonical() {
        assert_eq!(PageLayout::SinglePage as i32, 0);
        assert_eq!(PageLayout::OneColumn as i32, 1);
        assert_eq!(PageLayout::TwoColumnLeft as i32, 2);
        assert_eq!(PageLayout::TwoColumnRight as i32, 3);
        assert_eq!(PageLayout::TwoPageLeft as i32, 4);
        assert_eq!(PageLayout::TwoPageRight as i32, 5);
        assert_eq!(PageLayout::Default as i32, 6);
    }

    #[test]
    fn default_is_default() {
        assert_eq!(PageLayout::default(), PageLayout::Default);
    }
}
