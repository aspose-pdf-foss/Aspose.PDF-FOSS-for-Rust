//! `Aspose.Pdf.Facades.VerticalAlignmentType` — a named vertical-alignment
//! constant, per `capabilities/vertical_alignment_type.yaml`.
//!
//! A named-constant value type: construct via
//! [`new`](VerticalAlignmentType::new) or the
//! [`top`](VerticalAlignmentType::top) / [`center`](VerticalAlignmentType::center)
//! / [`bottom`](VerticalAlignmentType::bottom) factories; [`Display`](std::fmt::Display)
//! yields the constant name.

use std::fmt;

/// A named vertical-alignment constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerticalAlignmentType {
    name: String,
}

impl VerticalAlignmentType {
    /// Construct from a constant name.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Top alignment.
    pub fn top() -> Self {
        Self::new("Top")
    }
    /// Centre alignment.
    pub fn center() -> Self {
        Self::new("Center")
    }
    /// Bottom alignment.
    pub fn bottom() -> Self {
        Self::new("Bottom")
    }
}

impl fmt::Display for VerticalAlignmentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn factories_and_display() {
        assert_eq!(VerticalAlignmentType::top().to_string(), "Top");
        assert_eq!(VerticalAlignmentType::center().to_string(), "Center");
        assert_eq!(VerticalAlignmentType::bottom().to_string(), "Bottom");
    }
}
