//! `Aspose.Pdf.Artifact` — a page artifact (watermark, background,
//! stamp), per `capabilities/artifact.yaml`.
//!
//! Carries the artifact's placement (rectangle / position /
//! rotation), its text + text state or image content, opacity, and
//! the background flag. The enum-typed constructor
//! (`ArtifactType` / `ArtifactSubtype` nested enums, not in the
//! shipped subset) is dropped; the string-typed constructor ships.

use crate::public::artifact_subtype::ArtifactSubtype;
use crate::public::artifact_type::ArtifactType;
use crate::public::point::Point;
use crate::public::rectangle::Rectangle;
use crate::public::text_state::TextState;

/// A page artifact.
#[derive(Debug, Clone)]
pub struct Artifact {
    // The type / subtype tokens are recorded at construction and
    // consumed by render-on-save (a follow-on); not yet read.
    #[allow(dead_code)]
    artifact_type: String,
    #[allow(dead_code)]
    subtype: String,
    rectangle: Rectangle,
    position: Point,
    rotation: f64,
    text: String,
    opacity: f64,
    text_state: TextState,
    is_background: bool,
    image: Option<Vec<u8>>,
}

impl Artifact {
    /// Construct an artifact of the given type and subtype.
    pub fn new(artifact_type: impl Into<String>, subtype: impl Into<String>) -> Self {
        Self {
            artifact_type: artifact_type.into(),
            subtype: subtype.into(),
            rectangle: Rectangle::trivial(),
            position: Point::new(0.0, 0.0),
            rotation: 0.0,
            text: String::new(),
            opacity: 1.0,
            text_state: TextState::default(),
            is_background: false,
            image: None,
        }
    }

    /// Construct an artifact from the typed [`ArtifactType`] /
    /// [`ArtifactSubtype`] tokens, mirroring the canonical enum-typed
    /// constructor. The tokens are recorded as their canonical names.
    pub fn from_types(artifact_type: ArtifactType, subtype: ArtifactSubtype) -> Self {
        Self::new(type_token(artifact_type), subtype_token(subtype))
    }

    /// Set the artifact text and its styling together.
    pub fn set_text_and_state(&mut self, text: impl Into<String>, text_state: TextState) {
        self.text = text.into();
        self.text_state = text_state;
    }

    /// Set the artifact image from encoded bytes.
    pub fn set_image_bytes(&mut self, bytes: &[u8]) {
        self.image = Some(bytes.to_vec());
    }

    /// Set the artifact image from a file path (best-effort: a
    /// missing / unreadable file leaves the image unset).
    pub fn set_image_file(&mut self, path: &str) {
        if let Ok(bytes) = std::fs::read(path) {
            self.image = Some(bytes);
        }
    }

    /// The artifact's bounding rectangle.
    pub fn rectangle(&self) -> Rectangle {
        self.rectangle
    }
    /// Set the artifact's bounding rectangle.
    pub fn set_rectangle(&mut self, value: Rectangle) {
        self.rectangle = value;
    }

    /// The artifact's placement position.
    pub fn position(&self) -> Point {
        self.position
    }
    /// Set the artifact's placement position.
    pub fn set_position(&mut self, value: Point) {
        self.position = value;
    }

    /// The artifact rotation, in degrees.
    pub fn rotation(&self) -> f64 {
        self.rotation
    }
    /// Set the artifact rotation, in degrees counter-clockwise about the
    /// artifact's [`position`](Self::position). Applied when the artifact
    /// is drawn.
    pub fn set_rotation(&mut self, value: f64) {
        self.rotation = value;
    }

    /// The artifact text.
    pub fn text(&self) -> &str {
        &self.text
    }
    /// Set the artifact text.
    pub fn set_text(&mut self, value: impl Into<String>) {
        self.text = value.into();
    }

    /// The artifact opacity (0..1).
    ///
    /// Carried, not applied: an opacity needs a `/ExtGState` graphics-state
    /// resource, and the content-edit flush that writes an artifact appends
    /// a content stream without one.
    pub fn opacity(&self) -> f64 {
        self.opacity
    }
    /// Set the artifact opacity (0..1).
    pub fn set_opacity(&mut self, value: f64) {
        self.opacity = value;
    }

    /// The artifact text state.
    pub fn text_state(&self) -> &TextState {
        &self.text_state
    }
    /// Set the artifact text state.
    pub fn set_text_state(&mut self, value: TextState) {
        self.text_state = value;
    }

    /// Whether the artifact is drawn behind the page content.
    ///
    /// Carried, not applied: an artifact is written as an appended content
    /// stream, which paints after everything already on the page, so it
    /// cannot take a place ahead of the existing content.
    pub fn is_background(&self) -> bool {
        self.is_background
    }
    /// Set whether the artifact is drawn behind the page content.
    pub fn set_is_background(&mut self, value: bool) {
        self.is_background = value;
    }

    /// The encoded image bytes set via [`set_image_bytes`](Self::set_image_bytes)
    /// / [`set_image_file`](Self::set_image_file), if any. Crate-internal —
    /// consumed by the document's artifact render-on-save.
    pub(crate) fn image(&self) -> Option<&[u8]> {
        self.image.as_deref()
    }
}

/// Canonical PDF token for an artifact type.
fn type_token(t: ArtifactType) -> &'static str {
    match t {
        ArtifactType::Pagination => "Pagination",
        ArtifactType::Layout => "Layout",
        ArtifactType::Page => "Page",
        ArtifactType::Background => "Background",
        ArtifactType::Undefined => "Undefined",
    }
}

/// Canonical PDF token for an artifact subtype.
fn subtype_token(s: ArtifactSubtype) -> &'static str {
    match s {
        ArtifactSubtype::Header => "Header",
        ArtifactSubtype::Footer => "Footer",
        ArtifactSubtype::Watermark => "Watermark",
        ArtifactSubtype::Background => "Background",
        ArtifactSubtype::Undefined => "Undefined",
        ArtifactSubtype::BatesN => "BatesN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construct_and_properties() {
        let mut a = Artifact::new("Watermark", "");
        assert_eq!(a.opacity(), 1.0);
        assert!(!a.is_background());
        a.set_rotation(45.0);
        a.set_opacity(0.4);
        a.set_is_background(true);
        a.set_text("CONFIDENTIAL");
        assert_eq!(a.rotation(), 45.0);
        assert_eq!(a.opacity(), 0.4);
        assert!(a.is_background());
        assert_eq!(a.text(), "CONFIDENTIAL");
    }

    #[test]
    fn from_types_maps_tokens() {
        assert_eq!(type_token(ArtifactType::Pagination), "Pagination");
        assert_eq!(type_token(ArtifactType::Background), "Background");
        assert_eq!(subtype_token(ArtifactSubtype::Watermark), "Watermark");
        assert_eq!(subtype_token(ArtifactSubtype::BatesN), "BatesN");
    }

    #[test]
    fn from_types_constructs_with_defaults() {
        let a = Artifact::from_types(ArtifactType::Page, ArtifactSubtype::Footer);
        assert_eq!(a.opacity(), 1.0);
        assert!(!a.is_background());
    }
}
