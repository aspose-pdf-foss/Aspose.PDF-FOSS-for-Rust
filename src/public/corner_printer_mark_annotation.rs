//! `Aspose.Pdf.Annotations.CornerPrinterMarkAnnotation` — the
//! intermediate base for corner-anchored printer marks (trim /
//! bleed), per `capabilities/corner_printer_mark_annotation.yaml`.
//!
//! Modelled as the [`CornerPrinterMarkAnnotation`] trait (extending
//! [`PrinterMarkAnnotation`]) over a shared `CornerBase`.

use crate::public::printer_mark_annotation::PrinterMarkAnnotation;
use crate::public::printer_mark_corner_position::PrinterMarkCornerPosition;

/// Shared corner-position field. Crate-internal.
pub(crate) struct CornerBase {
    pub(crate) position: PrinterMarkCornerPosition,
}

impl CornerBase {
    pub(crate) fn new(position: PrinterMarkCornerPosition) -> Self {
        Self { position }
    }
}

pub(crate) mod sealed {
    use super::CornerBase;

    /// Corner-base access for concrete corner printer marks.
    pub(crate) trait HasCornerBase {
        /// The shared corner base.
        fn corner(&self) -> &CornerBase;
        /// The shared corner base, mutably.
        fn corner_mut(&mut self) -> &mut CornerBase;
    }
}

/// A corner-anchored printer mark.
#[allow(private_bounds)]
pub trait CornerPrinterMarkAnnotation: PrinterMarkAnnotation + sealed::HasCornerBase {
    /// The corner the mark is anchored to.
    fn position(&self) -> PrinterMarkCornerPosition {
        self.corner().position
    }
    /// Set the corner the mark is anchored to.
    fn set_position(&mut self, value: PrinterMarkCornerPosition) {
        self.corner_mut().position = value;
    }
}
