//! `Aspose.Pdf.Text.TabAlignmentType` — how text aligns against a tab
//! stop, per `capabilities/tab_alignment_type.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// How text is aligned relative to a tab stop position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum TabAlignmentType {
    /// Left-align text at the tab stop.
    #[default]
    Left = 0,
    /// Centre text on the tab stop.
    Center = 1,
    /// Right-align text at the tab stop.
    Right = 2,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals_and_default() {
        assert_eq!(TabAlignmentType::default(), TabAlignmentType::Left);
        assert_eq!(TabAlignmentType::Left as i32, 0);
        assert_eq!(TabAlignmentType::Center as i32, 1);
        assert_eq!(TabAlignmentType::Right as i32, 2);
    }
}
