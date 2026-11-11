//! `Aspose.Pdf.Annotations.Characteristics` — the appearance
//! characteristics of a widget-style annotation, per
//! `capabilities/characteristics.yaml`.
//!
//! The canonical `Background` / `Border` colours are typed
//! `System.Drawing.Color` and drop on the Rust surface (matching
//! the C++ reference subset); `Rotate` ships.

use crate::public::rotation::Rotation;

/// Appearance characteristics (the annotation `/MK` dictionary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Characteristics {
    rotate: Rotation,
}

impl Characteristics {
    /// Construct default characteristics (no rotation).
    pub fn new() -> Self {
        Self::default()
    }

    /// The rotation applied to the annotation's appearance.
    pub fn rotate(&self) -> Rotation {
        self.rotate
    }
    /// Set the rotation applied to the annotation's appearance.
    pub fn set_rotate(&mut self, value: Rotation) {
        self.rotate = value;
    }
}
