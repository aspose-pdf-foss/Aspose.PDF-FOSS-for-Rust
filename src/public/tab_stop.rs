//! `Aspose.Pdf.Text.TabStop` — one tab stop in a paragraph's tab ruler,
//! per `capabilities/tab_stop.yaml`.
//!
//! A value type describing where a tab advances to ([`position`], in
//! points), how following text aligns there ([`alignment_type`]), and
//! the leader drawn across the skipped gap ([`leader_type`]). Held in a
//! [`TabStops`](crate::TabStops) ruler.
//!
//! [`position`]: TabStop::position
//! [`alignment_type`]: TabStop::alignment_type
//! [`leader_type`]: TabStop::leader_type

use crate::public::tab_alignment_type::TabAlignmentType;
use crate::public::tab_leader_type::TabLeaderType;

/// A single tab stop.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TabStop {
    position: f32,
    leader_type: TabLeaderType,
    alignment_type: TabAlignmentType,
}

impl TabStop {
    /// A tab stop at position `0` with the zero-value leader / alignment
    /// (`Solid` / `Left`).
    pub fn new() -> Self {
        Self::default()
    }

    /// A tab stop at `position` points from the left margin.
    pub fn with_position(position: f32) -> Self {
        Self { position, ..Self::default() }
    }

    /// The tab stop position, in points.
    pub fn position(&self) -> f32 {
        self.position
    }
    /// Set the tab stop position, in points.
    pub fn set_position(&mut self, value: f32) {
        self.position = value;
    }

    /// The leader drawn across the skipped gap.
    pub fn leader_type(&self) -> TabLeaderType {
        self.leader_type
    }
    /// Set the leader drawn across the skipped gap.
    pub fn set_leader_type(&mut self, value: TabLeaderType) {
        self.leader_type = value;
    }

    /// How following text aligns at the stop.
    pub fn alignment_type(&self) -> TabAlignmentType {
        self.alignment_type
    }
    /// Set how following text aligns at the stop.
    pub fn set_alignment_type(&mut self, value: TabAlignmentType) {
        self.alignment_type = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_round_trip() {
        let d = TabStop::new();
        assert_eq!(d.position(), 0.0);
        assert_eq!(d.leader_type(), TabLeaderType::Solid);
        assert_eq!(d.alignment_type(), TabAlignmentType::Left);

        let mut t = TabStop::with_position(72.0);
        assert_eq!(t.position(), 72.0);
        t.set_position(144.0);
        t.set_leader_type(TabLeaderType::Dot);
        t.set_alignment_type(TabAlignmentType::Center);
        assert_eq!(t.position(), 144.0);
        assert_eq!(t.leader_type(), TabLeaderType::Dot);
        assert_eq!(t.alignment_type(), TabAlignmentType::Center);
    }
}
