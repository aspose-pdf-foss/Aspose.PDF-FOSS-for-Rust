//! `Aspose.Pdf.Artifact.ArtifactSubtype` — the artifact's subtype, per
//! `capabilities/artifact_subtype.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The subtype of a page [`Artifact`](crate::Artifact).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ArtifactSubtype {
    /// Page header.
    #[default]
    Header = 0,
    /// Page footer.
    Footer = 1,
    /// Watermark.
    Watermark = 2,
    /// Background.
    Background = 3,
    /// Undefined subtype.
    Undefined = 4,
    /// Bates numbering.
    BatesN = 5,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals_match_canonical() {
        assert_eq!(ArtifactSubtype::Header as i32, 0);
        assert_eq!(ArtifactSubtype::Footer as i32, 1);
        assert_eq!(ArtifactSubtype::Watermark as i32, 2);
        assert_eq!(ArtifactSubtype::Background as i32, 3);
        assert_eq!(ArtifactSubtype::Undefined as i32, 4);
        assert_eq!(ArtifactSubtype::BatesN as i32, 5);
    }

    #[test]
    fn default_is_header() {
        assert_eq!(ArtifactSubtype::default(), ArtifactSubtype::Header);
    }
}
