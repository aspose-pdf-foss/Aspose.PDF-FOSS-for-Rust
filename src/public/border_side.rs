//! `Aspose.Pdf.BorderSide` — which edges of a border are drawn, per
//! `capabilities/border_side.yaml`.
//!
//! A `[Flags]` enum. `All` and `Box` share the value 15, which a
//! plain Rust enum cannot express, so it is modelled as a bit-flag
//! newtype with associated constants and the bitwise operators.

use std::ops::{BitAnd, BitOr};

/// A set of border edges (bit flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BorderSide(i32);

impl BorderSide {
    /// No edges.
    pub const NONE: BorderSide = BorderSide(0);
    /// The left edge.
    pub const LEFT: BorderSide = BorderSide(1);
    /// The top edge.
    pub const TOP: BorderSide = BorderSide(2);
    /// The right edge.
    pub const RIGHT: BorderSide = BorderSide(4);
    /// The bottom edge.
    pub const BOTTOM: BorderSide = BorderSide(8);
    /// All four edges.
    pub const ALL: BorderSide = BorderSide(15);
    /// All four edges (alias of [`ALL`](BorderSide::ALL)).
    pub const BOX: BorderSide = BorderSide(15);

    /// The raw flag bits.
    pub fn bits(self) -> i32 {
        self.0
    }

    /// Whether every edge in `other` is set in `self`.
    pub fn contains(self, other: BorderSide) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl BitOr for BorderSide {
    type Output = BorderSide;
    fn bitor(self, rhs: BorderSide) -> BorderSide {
        BorderSide(self.0 | rhs.0)
    }
}

impl BitAnd for BorderSide {
    type Output = BorderSide;
    fn bitand(self, rhs: BorderSide) -> BorderSide {
        BorderSide(self.0 & rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_combine() {
        assert_eq!(BorderSide::ALL.bits(), 15);
        assert_eq!(BorderSide::BOX.bits(), 15);
        let lr = BorderSide::LEFT | BorderSide::RIGHT;
        assert!(lr.contains(BorderSide::LEFT));
        assert!(lr.contains(BorderSide::RIGHT));
        assert!(!lr.contains(BorderSide::TOP));
        assert_eq!(BorderSide::default(), BorderSide::NONE);
    }
}
