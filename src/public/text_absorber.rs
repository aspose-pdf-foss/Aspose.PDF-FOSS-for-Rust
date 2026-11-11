//! `Aspose.Pdf.Text.TextAbsorber` — visitor that walks a
//! [`Document`] or [`Page`] and accumulates the text it encounters
//! into a single string.
//!
//! v1 surface (per `capabilities/text_absorber.yaml`): default
//! ctor, two visit entry points ([`TextAbsorber::visit_document`]
//! and [`TextAbsorber::visit_page`] — Rust mirrors of the canonical
//! `Visit(Document)` / `Visit(Page)` overloads), plus read-only
//! [`TextAbsorber::text`] and [`TextAbsorber::has_errors`] getters.
//! Bodies route through the foundation `text_extractor` primitive —
//! the same wiring the C# / TypeScript peers use.
//!
//! Visit methods return `Result<(), Error>` so structural failures
//! in the source PDF surface to the caller. The canonical surface
//! declares them `void`, but `text_extractor::parse` is fallible
//! and the established Rust convention across [`Document::save`],
//! [`PngDevice::process`](crate::PngDevice), etc. is "void →
//! `Result<(), Error>` when the body can fail". `has_errors`
//! remains in the surface for canonical fidelity but always
//! returns `false` — hard failures propagate via the returned
//! `Result`, so a returning `Ok` visit means a clean run.

use crate::internal::text_extractor;

use super::document::Document;
use super::error::Error;
use super::page::Page;

/// Visitor that walks a [`Document`] or single [`Page`] and
/// accumulates the extracted text. After a successful visit,
/// [`TextAbsorber::text`] returns the concatenated result.
#[derive(Debug, Default, Clone)]
pub struct TextAbsorber {
    text: String,
}

impl TextAbsorber {
    /// Default-construct an absorber with empty accumulated text.
    pub fn new() -> Self {
        Self { text: String::new() }
    }

    /// Walk every page of `pdf` in order and append its text
    /// content to the accumulated [`TextAbsorber::text`] buffer.
    /// Page texts concatenate without a separator — matches the
    /// C# / TypeScript peer bodies.
    pub fn visit_document(&mut self, pdf: &Document) -> Result<(), Error> {
        let pages = text_extractor::parse(pdf.internal_bytes())
            .map_err(|e| Error::InvalidPdf(e.to_string()))?;
        for page_text in &pages {
            self.text.push_str(page_text);
        }
        Ok(())
    }

    /// Walk a single `page` and append its text content to the
    /// accumulated [`TextAbsorber::text`] buffer.
    pub fn visit_page(&mut self, page: &Page) -> Result<(), Error> {
        let pages = text_extractor::parse(page.internal_doc_bytes())
            .map_err(|e| Error::InvalidPdf(e.to_string()))?;
        let idx = page.internal_leaf_index() as usize;
        if idx < pages.len() {
            self.text.push_str(&pages[idx]);
        }
        Ok(())
    }

    /// Concatenated text accumulated from prior visit calls.
    /// Empty string when no visit has been called or the input
    /// carried no extractable text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// True iff any extraction error was recorded during the
    /// last visit call. v1 visit paths propagate hard failures
    /// through `Result`, so a returning `Ok` visit means a clean
    /// run and `has_errors` stays `false`.
    pub fn has_errors(&self) -> bool {
        false
    }
}
