//! `Aspose.Pdf.Annotations.RichMediaAnnotation.ContentType` — the kind
//! of media a rich-media annotation carries, per
//! `capabilities/content_type.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The kind of media a
/// [`RichMediaAnnotation`](crate::RichMediaAnnotation) carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ContentType {
    /// Audio content.
    #[default]
    Audio = 0,
    /// Video content.
    Video = 1,
    /// Unknown content.
    Unknown = 2,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals_match_canonical() {
        assert_eq!(ContentType::Audio as i32, 0);
        assert_eq!(ContentType::Video as i32, 1);
        assert_eq!(ContentType::Unknown as i32, 2);
    }

    #[test]
    fn default_is_audio() {
        assert_eq!(ContentType::default(), ContentType::Audio);
    }
}
