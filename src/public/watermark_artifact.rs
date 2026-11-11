//! `Aspose.Pdf.WatermarkArtifact` — a watermark page artifact, per
//! `capabilities/watermark_artifact.yaml`.
//!
//! Canonically an [`Artifact`] specialised as a watermark. Rust has
//! no inheritance, so it wraps an `Artifact` (pre-typed
//! `"Watermark"`) and derefs to it, exposing every inherited
//! member; `Into<Artifact>` adds it to an
//! [`ArtifactCollection`](crate::ArtifactCollection).

use crate::public::artifact::Artifact;
use std::ops::{Deref, DerefMut};

/// A watermark artifact.
#[derive(Debug, Clone)]
pub struct WatermarkArtifact {
    inner: Artifact,
}

impl Default for WatermarkArtifact {
    fn default() -> Self {
        Self { inner: Artifact::new("Watermark", "") }
    }
}

impl WatermarkArtifact {
    /// Construct a watermark artifact.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Deref for WatermarkArtifact {
    type Target = Artifact;
    fn deref(&self) -> &Artifact {
        &self.inner
    }
}

impl DerefMut for WatermarkArtifact {
    fn deref_mut(&mut self) -> &mut Artifact {
        &mut self.inner
    }
}

impl From<WatermarkArtifact> for Artifact {
    fn from(w: WatermarkArtifact) -> Artifact {
        w.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watermark_is_an_artifact() {
        let mut wm = WatermarkArtifact::new();
        // Inherited Artifact members via Deref.
        wm.set_rotation(45.0);
        wm.set_opacity(0.4);
        wm.set_is_background(true);
        assert_eq!(wm.rotation(), 45.0);
        assert!(wm.is_background());

        let a: Artifact = wm.into();
        assert_eq!(a.opacity(), 0.4);
    }
}
