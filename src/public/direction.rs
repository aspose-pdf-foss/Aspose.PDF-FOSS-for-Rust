//! `Aspose.Pdf.Direction` — the predominant reading order for text,
//! per `capabilities/direction.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The predominant reading order for text (the document catalog's
/// `/ViewerPreferences /Direction`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum Direction {
    /// Left-to-right reading order.
    #[default]
    L2R = 0,
    /// Right-to-left reading order.
    R2L = 1,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals_match_canonical() {
        assert_eq!(Direction::L2R as i32, 0);
        assert_eq!(Direction::R2L as i32, 1);
    }

    #[test]
    fn default_is_l2r() {
        assert_eq!(Direction::default(), Direction::L2R);
    }
}
