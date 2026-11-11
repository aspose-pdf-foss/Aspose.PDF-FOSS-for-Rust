//! `Aspose.Pdf.Annotations.CommonFigureAnnotation` — the
//! intermediate base for figure annotations (square / circle), per
//! `capabilities/common_figure_annotation.yaml`.
//!
//! Modelled as the [`CommonFigureAnnotation`] trait (extending
//! [`MarkupAnnotation`]) over a shared `CommonFigureBase`, in the
//! annotation sealed-base style. The abstract `.ctor` is dropped
//! (the Rust trait has no constructor).

use crate::public::color::Color;
use crate::public::markup_annotation::MarkupAnnotation;
use crate::public::rectangle::Rectangle;

/// Shared figure fields (interior fill + frame). Crate-internal.
pub(crate) struct CommonFigureBase {
    pub(crate) interior_color: Color,
    pub(crate) frame: Rectangle,
}

impl CommonFigureBase {
    pub(crate) fn new() -> Self {
        Self {
            interior_color: Color::empty(),
            frame: Rectangle::trivial(),
        }
    }
}

impl Default for CommonFigureBase {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) mod sealed {
    use super::CommonFigureBase;

    /// Figure-base access for concrete figure annotations.
    pub(crate) trait HasFigureBase {
        /// The shared figure base.
        fn figure(&self) -> &CommonFigureBase;
        /// The shared figure base, mutably.
        fn figure_mut(&mut self) -> &mut CommonFigureBase;
    }
}

/// A figure annotation (square / circle) with an interior fill and
/// a padding frame.
#[allow(private_bounds)]
pub trait CommonFigureAnnotation: MarkupAnnotation + sealed::HasFigureBase {
    /// The interior fill colour.
    fn interior_color(&self) -> Color {
        self.figure().interior_color
    }
    /// Set the interior fill colour.
    fn set_interior_color(&mut self, value: Color) {
        self.figure_mut().interior_color = value;
    }

    /// The padding frame (the inset between `Rect` and the drawn
    /// figure).
    fn frame(&self) -> Rectangle {
        self.figure().frame
    }
    /// Set the padding frame.
    fn set_frame(&mut self, value: Rectangle) {
        self.figure_mut().frame = value;
    }
}
