//! `Aspose.Pdf.Facades.ImageMergeMode` — image-merge layout, per
//! `capabilities/image_merge_mode.yaml`. Ordinals match canonical 26.4.0.

/// An image-merge layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ImageMergeMode {
    /// Stack images vertically.
    #[default]
    Vertical = 1,
    /// Lay images out horizontally.
    Horizontal = 2,
    /// Centre images.
    Center = 3,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordinals() {
        assert_eq!(ImageMergeMode::Vertical as i32, 1);
        assert_eq!(ImageMergeMode::Horizontal as i32, 2);
        assert_eq!(ImageMergeMode::Center as i32, 3);
    }
}
