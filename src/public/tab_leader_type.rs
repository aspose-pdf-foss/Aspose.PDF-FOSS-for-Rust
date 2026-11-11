//! `Aspose.Pdf.Text.TabLeaderType` — the leader drawn in the gap a tab
//! stop skips over, per `capabilities/tab_leader_type.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The leader style drawn across a tab stop's gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum TabLeaderType {
    /// A solid line.
    #[default]
    Solid = 0,
    /// A dashed line.
    Dash = 1,
    /// A dotted line.
    Dot = 2,
    /// No leader.
    None = 3,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals_and_default() {
        assert_eq!(TabLeaderType::default(), TabLeaderType::Solid);
        assert_eq!(TabLeaderType::Solid as i32, 0);
        assert_eq!(TabLeaderType::Dash as i32, 1);
        assert_eq!(TabLeaderType::Dot as i32, 2);
        assert_eq!(TabLeaderType::None as i32, 3);
    }
}
