//! `Aspose.Pdf.Annotations.PrinterMarkAnnotation` — the abstract
//! base for printer-mark annotations, per
//! `capabilities/printer_mark_annotation.yaml`.
//!
//! Modelled as the [`PrinterMarkAnnotation`] trait (extending
//! [`Annotation`]). `AddPrinterMarks` places a
//! set of marks on a document / page — a no-op placeholder in the
//! in-memory model.

use crate::public::annotation::Annotation;
use crate::public::document::Document;
use crate::public::page::Page;
use crate::public::printer_marks_kind::PrinterMarksKind;

/// A printer-mark annotation (color bar, trim / bleed / registration
/// mark, page information).
pub trait PrinterMarkAnnotation: Annotation {
    /// Add the given kinds of printer marks to a document. No-op in
    /// the in-memory model.
    fn add_printer_marks(&self, _document: &Document, _kind: PrinterMarksKind) {}

    /// Add the given kinds of printer marks to a page. No-op in the
    /// in-memory model.
    fn add_printer_marks_on_page(&self, _page: &Page, _kind: PrinterMarksKind) {}
}
