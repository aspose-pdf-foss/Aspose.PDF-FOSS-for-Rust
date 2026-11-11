//! `Aspose.Pdf.Devices.TextDevice` — concrete [`PageDevice`] that
//! extracts the text of a single PDF [`Page`] and writes it,
//! encoded via the configured [`Encoding`], to the caller's output
//! sink.
//!
//! Composes the same foundation primitive
//! (`internal::text_extractor`)
//! as [`TextAbsorber`](crate::TextAbsorber): `text_extractor::parse`
//! returns one pre-order entry per page-tree leaf and the device
//! picks the visited page's slot via `Page::internal_leaf_index()`.
//! Bytes round through `internal::encoding::Encoding::get_bytes`
//! (UTF-8 default) — mirrors the C# `_encoding.GetBytes(...)` body.
//!
//! Process returns `Result<(), Error>` so structural failures in
//! the source PDF surface to the caller. The canonical surface
//! declares `Process` as `void`, but `text_extractor::parse` is
//! fallible and the established convention across the device
//! hierarchy ([`PngDevice`](crate::PngDevice) etc.) is
//! "void → `Result<(), Error>` when the body can fail".

use super::device::{Device, PageDevice};
use super::error::Error;
use super::page::Page;

use crate::internal::encoding::Encoding;
use crate::internal::text_extractor;

/// Concrete [`PageDevice`] that writes extracted page text to an
/// output sink, encoded via the configured [`Encoding`].
#[derive(Debug, Clone)]
pub struct TextDevice {
    encoding: Encoding,
}

impl TextDevice {
    /// Default-construct with UTF-8 output encoding — matches the
    /// canonical .NET surface's documented default and the C# /
    /// TypeScript peer bodies.
    pub fn new() -> Self {
        Self { encoding: Encoding::utf8() }
    }

    /// Construct with an explicit output [`Encoding`].
    pub fn with_encoding(encoding: Encoding) -> Self {
        Self { encoding }
    }

    /// Output text encoding. Read-write; defaults to UTF-8.
    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    /// Replace the output text [`Encoding`]. See
    /// [`TextDevice::encoding`].
    pub fn set_encoding(&mut self, value: Encoding) {
        self.encoding = value;
    }
}

impl Default for TextDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl Device for TextDevice {}

impl PageDevice for TextDevice {
    fn process(
        &self,
        page: &Page,
        output: &mut dyn std::io::Write,
    ) -> Result<(), Error> {
        let pages = text_extractor::parse(page.internal_doc_bytes())
            .map_err(|e| Error::InvalidPdf(e.to_string()))?;
        let idx = page.internal_leaf_index() as usize;
        if idx < pages.len() {
            let bytes = self.encoding.get_bytes(&pages[idx]);
            output.write_all(&bytes)?;
        }
        Ok(())
    }
}
