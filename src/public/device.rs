//! `Aspose.Pdf.Devices.Device` — abstract root of the canonical
//! Aspose.Pdf.Devices class hierarchy.
//!
//! Canonical .NET surface declares `Device` as an abstract class
//! with no public members (`PageDevice`, `ImageDevice`,
//! `DocumentDevice` derive from it). Rust expresses that as a
//! marker trait — every concrete device type (currently just
//! [`PngDevice`](crate::PngDevice)) implements it. The trait
//! itself has no methods.
//!
//! Like its peers in C# / TypeScript, the marker carries no
//! state of its own; structural inheritance lands one tier
//! down on [`PageDevice`].

use super::document::Document;
use super::page::Page;

/// Marker trait for any Aspose.Pdf.Devices concrete device.
///
/// The canonical .NET `Device` abstract class has no public
/// members; this trait mirrors that — implementors gain no
/// methods from it directly, but downstream tiers
/// ([`PageDevice`]) require `Device` as a super-trait to make
/// the canonical inheritance chain enforceable at compile time.
pub trait Device {}

/// Page-rendering device — abstract base for any device that
/// processes a single PDF [`Page`] into an output sink. Mirrors
/// the canonical `Aspose.Pdf.Devices.PageDevice` abstract class.
///
/// Concrete implementors ([`PngDevice`](crate::PngDevice))
/// supply [`PageDevice::process`]; the
/// [`PageDevice::process_to_path`] file-path convenience opens
/// a [`std::fs::File`] and dispatches to `process`.
pub trait PageDevice: Device {
    /// Render `page` through this device into `output`.
    /// Format and per-device behaviour are determined by the
    /// concrete implementor.
    fn process(
        &self,
        page: &Page,
        output: &mut dyn std::io::Write,
    ) -> Result<(), super::error::Error>;

    /// File-path convenience that opens `output_file_name` for
    /// writing (creating or truncating) and dispatches to
    /// [`PageDevice::process`].
    fn process_to_path(
        &self,
        page: &Page,
        output_file_name: impl AsRef<std::path::Path>,
    ) -> Result<(), super::error::Error>
    where
        Self: Sized,
    {
        let mut f = std::fs::File::create(output_file_name.as_ref())?;
        self.process(page, &mut f)
    }
}

/// Document-rendering device — abstract intermediate base for any
/// device that processes an entire [`Document`] (or a page range)
/// into a single output sink. Mirrors the canonical
/// `Aspose.Pdf.Devices.DocumentDevice` abstract class.
///
/// Concrete implementors ([`TiffDevice`](crate::TiffDevice))
/// supply [`DocumentDevice::process_document_range`]; the
/// [`DocumentDevice::process_document`] and the file-path
/// convenience methods route through it.
pub trait DocumentDevice: PageDevice {
    /// Render the inclusive 1-based page range
    /// `[from_page, to_page]` of `document` through this device
    /// into `output`. Page numbers follow the PDF convention —
    /// the first page is `1`, the last is `document.pages().count()`.
    fn process_document_range(
        &self,
        document: &Document,
        from_page: usize,
        to_page: usize,
        output: &mut dyn std::io::Write,
    ) -> Result<(), super::error::Error>;

    /// Render the entire `document` — dispatches to
    /// [`DocumentDevice::process_document_range`] over
    /// `[1, document.pages().count()]`.
    fn process_document(
        &self,
        document: &Document,
        output: &mut dyn std::io::Write,
    ) -> Result<(), super::error::Error> {
        let count = document.pages().count();
        self.process_document_range(document, 1, count, output)
    }

    /// File-path convenience that opens `output_file_name` for
    /// writing (creating or truncating) and dispatches to
    /// [`DocumentDevice::process_document`].
    fn process_document_to_path(
        &self,
        document: &Document,
        output_file_name: impl AsRef<std::path::Path>,
    ) -> Result<(), super::error::Error>
    where
        Self: Sized,
    {
        let mut f = std::fs::File::create(output_file_name.as_ref())?;
        self.process_document(document, &mut f)
    }

    /// File-path convenience for the page-range overload.
    fn process_document_range_to_path(
        &self,
        document: &Document,
        from_page: usize,
        to_page: usize,
        output_file_name: impl AsRef<std::path::Path>,
    ) -> Result<(), super::error::Error>
    where
        Self: Sized,
    {
        let mut f = std::fs::File::create(output_file_name.as_ref())?;
        self.process_document_range(document, from_page, to_page, &mut f)
    }
}
