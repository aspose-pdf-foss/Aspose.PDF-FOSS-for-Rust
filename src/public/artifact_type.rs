//! `Aspose.Pdf.Artifact.ArtifactType` — the artifact's type class, per
//! `capabilities/artifact_type.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The type class of a page [`Artifact`](crate::Artifact).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ArtifactType {
    /// Pagination artifact (running heads, page numbers).
    #[default]
    Pagination = 0,
    /// Layout artifact (typographic layout aids).
    Layout = 1,
    /// Page artifact (production aids: cut marks, colour bars).
    Page = 2,
    /// Background artifact (drawn behind the page content).
    Background = 3,
    /// Undefined artifact type.
    Undefined = 4,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals_match_canonical() {
        assert_eq!(ArtifactType::Pagination as i32, 0);
        assert_eq!(ArtifactType::Layout as i32, 1);
        assert_eq!(ArtifactType::Page as i32, 2);
        assert_eq!(ArtifactType::Background as i32, 3);
        assert_eq!(ArtifactType::Undefined as i32, 4);
    }

    #[test]
    fn default_is_pagination() {
        assert_eq!(ArtifactType::default(), ArtifactType::Pagination);
    }
}
