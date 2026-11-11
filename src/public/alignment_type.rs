//! `Aspose.Pdf.Facades.AlignmentType` — a named horizontal-alignment
//! constant, per `capabilities/alignment_type.yaml`.
//!
//! A named-constant value type: construct via [`new`](AlignmentType::new)
//! or the [`left`](AlignmentType::left) / [`center`](AlignmentType::center)
//! / [`right`](AlignmentType::right) factories; [`Display`](std::fmt::Display) yields the
//! constant name.

use std::fmt;

/// A named horizontal-alignment constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlignmentType {
    name: String,
}

impl AlignmentType {
    /// Construct from a constant name.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Left alignment.
    pub fn left() -> Self {
        Self::new("Left")
    }
    /// Centre alignment.
    pub fn center() -> Self {
        Self::new("Center")
    }
    /// Right alignment.
    pub fn right() -> Self {
        Self::new("Right")
    }
}

impl fmt::Display for AlignmentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn factories_and_display() {
        assert_eq!(AlignmentType::left().to_string(), "Left");
        assert_eq!(AlignmentType::center().to_string(), "Center");
        assert_eq!(AlignmentType::right().to_string(), "Right");
        assert_eq!(AlignmentType::new("X").to_string(), "X");
        assert_eq!(AlignmentType::left(), AlignmentType::new("Left"));
    }
}
