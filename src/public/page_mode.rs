//! `Aspose.Pdf.PageMode` — how the document is displayed when opened,
//! per `capabilities/page_mode.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// How the viewer displays the document on open (the catalog's
/// `/PageMode`, and `/ViewerPreferences /NonFullScreenPageMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum PageMode {
    /// Neither outlines nor thumbnails visible.
    #[default]
    UseNone = 0,
    /// Outline (bookmark) panel visible.
    UseOutlines = 1,
    /// Thumbnail panel visible.
    UseThumbs = 2,
    /// Full-screen mode, no menu / panels.
    FullScreen = 3,
    /// Optional-content (layers) panel visible.
    UseOC = 4,
    /// Attachments panel visible.
    UseAttachments = 5,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals_match_canonical() {
        assert_eq!(PageMode::UseNone as i32, 0);
        assert_eq!(PageMode::UseOutlines as i32, 1);
        assert_eq!(PageMode::UseThumbs as i32, 2);
        assert_eq!(PageMode::FullScreen as i32, 3);
        assert_eq!(PageMode::UseOC as i32, 4);
        assert_eq!(PageMode::UseAttachments as i32, 5);
    }

    #[test]
    fn default_is_use_none() {
        assert_eq!(PageMode::default(), PageMode::UseNone);
    }
}
