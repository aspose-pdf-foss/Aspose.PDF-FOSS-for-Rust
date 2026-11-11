//! `Aspose.Pdf.Text.TextFragmentAbsorber` — a phrase-search visitor,
//! per `capabilities/text_fragment_absorber.yaml`.
//!
//! Walks a [`Document`] or [`Page`], finds positioned text runs, and
//! exposes the matches as [`TextFragment`]s carrying their text,
//! rectangle, position, and font size. The default constructor is
//! the canonical match-all mode (every positioned run becomes a
//! fragment); [`with_phrase`](TextFragmentAbsorber::with_phrase)
//! locates occurrences of a search phrase, and
//! [`with_phrase_and_options`](TextFragmentAbsorber::with_phrase_and_options)
//! scopes that search with a [`TextSearchOptions`].
//!
//! Canonically extends `TextAbsorber`. The edit-mode constructors take
//! a [`TextEditOptions`]; only the `Regex` constructors and the
//! `Visit(XForm)` overload are dropped (regex has no from-scratch
//! engine; XForm is not shipped). `Visit` returns `Result` (the
//! underlying parse is fallible) following the established convention
//! on [`TextAbsorber`](crate::TextAbsorber).

use crate::internal::text_extractor::{self, PositionedRun};
use crate::public::document::Document;
use crate::public::error::Error;
use crate::public::page::Page;
use crate::public::position::Position;
use crate::public::rectangle::Rectangle;
use crate::public::text_fragment::TextFragment;
use crate::public::text_edit_options::TextEditOptions;
use crate::public::text_fragment_collection::TextFragmentCollection;
use crate::public::text_search_options::TextSearchOptions;
use crate::internal::pending::PENDING;

/// Phrase-search visitor producing located [`TextFragment`]s.
#[derive(Debug, Default)]
pub struct TextFragmentAbsorber {
    /// The inherited absorber. Canonically this type derives from
    /// `TextAbsorber`; Rust has no inheritance, so the base is embedded
    /// and handed out through [`Deref`], which is what makes `text` and
    /// `has_errors` reachable here.
    base: crate::public::text_absorber::TextAbsorber,
    phrase: Option<String>,
    options: Option<TextSearchOptions>,
    // Edit-mode options retained from the editing constructors; acted on
    // by the write-back path rather than by the search itself.
    #[allow(dead_code)]
    edit_options: Option<TextEditOptions>,
    fragments: TextFragmentCollection,
}

impl TextFragmentAbsorber {
    /// Construct a match-all absorber: every positioned text run on
    /// the visited page(s) becomes a fragment.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct an absorber that locates occurrences of `phrase`.
    pub fn with_phrase(phrase: impl Into<String>) -> Self {
        Self { base: crate::public::text_absorber::TextAbsorber::new(), phrase: Some(phrase.into()), ..Self::default() }
    }

    /// Construct an absorber that locates occurrences of `phrase`,
    /// scoped by `options`. When the options carry a
    /// [`rectangle`](TextSearchOptions::rectangle), only matches whose
    /// area overlaps that region are collected.
    pub fn with_phrase_and_options(
        phrase: impl Into<String>,
        options: TextSearchOptions,
    ) -> Self {
        Self { base: crate::public::text_absorber::TextAbsorber::new(), phrase: Some(phrase.into()), options: Some(options), ..Self::default() }
    }

    /// Construct a match-all absorber operating in edit mode, governed
    /// by `edit_options`.
    pub fn with_edit_options(edit_options: TextEditOptions) -> Self {
        Self { edit_options: Some(edit_options), ..Self::default() }
    }

    /// Construct an absorber that locates occurrences of `phrase` in
    /// edit mode, governed by `edit_options`.
    pub fn with_phrase_and_edit_options(
        phrase: impl Into<String>,
        edit_options: TextEditOptions,
    ) -> Self {
        Self {
            base: crate::public::text_absorber::TextAbsorber::new(),
            phrase: Some(phrase.into()),
            edit_options: Some(edit_options),
            ..Self::default()
        }
    }

    /// Construct an absorber that locates occurrences of `phrase`,
    /// scoped by `options` and governed by `edit_options`.
    pub fn with_phrase_search_and_edit_options(
        phrase: impl Into<String>,
        options: TextSearchOptions,
        edit_options: TextEditOptions,
    ) -> Self {
        Self {
            base: crate::public::text_absorber::TextAbsorber::new(),
            phrase: Some(phrase.into()),
            options: Some(options),
            edit_options: Some(edit_options),
            ..Self::default()
        }
    }

    /// Search every page of `pdf`; matches accumulate in
    /// [`text_fragments`](TextFragmentAbsorber::text_fragments).
    pub fn visit_document(&mut self, pdf: &Document) -> Result<(), Error> {
        // The inherited absorber accumulates the page text, so the
        // inherited `text()` answers for real rather than staying empty.
        // It costs a second extraction pass over the same bytes.
        self.base.visit_document(pdf)?;
        let pages = text_extractor::parse_positioned(pdf.internal_bytes())
            .map_err(|e| Error::InvalidPdf(e.to_string()))?;
        // parse_positioned returns one run list per page, in page
        // order, so the page's own box is the one at the same index.
        for (index, page_runs) in pages.iter().enumerate() {
            let page_box = pdf.pages().get(index + 1).ok().map(|p| p.rect());
            self.absorb(page_runs, page_box);
        }
        Ok(())
    }

    /// Search a single `page`; matches accumulate in
    /// [`text_fragments`](TextFragmentAbsorber::text_fragments).
    pub fn visit_page(&mut self, page: &Page) -> Result<(), Error> {
        self.base.visit_page(page)?;
        let pages =
            text_extractor::parse_positioned(page.internal_doc_bytes())
                .map_err(|e| Error::InvalidPdf(e.to_string()))?;
        let idx = page.internal_leaf_index() as usize;
        if let Some(page_runs) = pages.get(idx) {
            self.absorb(page_runs, Some(page.rect()));
        }
        Ok(())
    }

    /// The fragments found across prior visits.
    pub fn text_fragments(&self) -> &TextFragmentCollection {
        &self.fragments
    }

    /// Whether `rect` falls within the active search scope: `true`
    /// when the options carry no rectangle, otherwise `true` only when
    /// `rect` overlaps that rectangle.
    fn in_scope(&self, rect: &Rectangle, page_box: Option<&Rectangle>) -> bool {
        if let Some(scope) = self.options.as_ref().and_then(|o| o.rectangle()) {
            if !scope.is_intersect(rect) {
                return false;
            }
        }
        // `limit_to_page_bounds` asks for matches outside the visible
        // page to be dropped. Text can sit outside the crop box —
        // trimmed-off marks, content left over from a resize — and is
        // extracted all the same, so without this the search answers
        // with runs a reader never sees.
        if self.options.as_ref().is_some_and(|o| o.limit_to_page_bounds()) {
            if let Some(bounds) = page_box {
                return bounds.is_intersect(rect);
            }
        }
        true
    }

    fn absorb(&mut self, runs: &[PositionedRun], page_box: Option<Rectangle>) {
        // A regular-expression search is not part of the shipped
        // subset, and answering a pattern with a literal search is a
        // wrong answer rather than a missing feature: `\\d+` would
        // find nothing and report success.
        if self.options.as_ref().is_some_and(|o| o.is_regular_expression_used()) {
            PENDING("text_search_regular_expressions")
        }
        match self.phrase.clone() {
            None => {
                for run in runs {
                    let w = run.char_x.last().copied().unwrap_or(0.0);
                    let rect = Rectangle::new(
                        run.x,
                        run.y,
                        run.x + w,
                        run.y + run.font_size,
                        false,
                    );
                    if self.in_scope(&rect, page_box.as_ref()) {
                        self.fragments.push(TextFragment::located(
                            run.text.clone(),
                            Position::new(run.x, run.y),
                            rect,
                            run.font_size as f32,
                        ));
                    }
                }
            }
            Some(phrase) if !phrase.is_empty() => {
                let pchars: Vec<char> = phrase.chars().collect();
                let plen = pchars.len();
                for run in runs {
                    let chars: Vec<char> = run.text.chars().collect();
                    if plen > chars.len() {
                        continue;
                    }
                    let mut i = 0;
                    while i + plen <= chars.len() {
                        if chars[i..i + plen] == pchars[..] {
                            let x0 = run.x + run.char_x[i];
                            let x1 = run.x + run.char_x[i + plen];
                            let rect = Rectangle::new(
                                x0,
                                run.y,
                                x1,
                                run.y + run.font_size,
                                false,
                            );
                            if self.in_scope(&rect, page_box.as_ref()) {
                                self.fragments.push(TextFragment::located(
                                    phrase.clone(),
                                    Position::new(x0, run.y),
                                    rect,
                                    run.font_size as f32,
                                ));
                            }
                            i += plen;
                        } else {
                            i += 1;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}


impl std::ops::Deref for TextFragmentAbsorber {
    type Target = crate::public::text_absorber::TextAbsorber;
    fn deref(&self) -> &crate::public::text_absorber::TextAbsorber {
        &self.base
    }
}

impl std::ops::DerefMut for TextFragmentAbsorber {
    fn deref_mut(&mut self) -> &mut crate::public::text_absorber::TextAbsorber {
        &mut self.base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello_world_pdf() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/text_extractor/hello_world.pdf")
    }

    #[test]
    fn phrase_search_locates_fragment() {
        let doc = Document::open(hello_world_pdf()).unwrap();
        let mut a = TextFragmentAbsorber::with_phrase("World");
        a.visit_document(&doc).unwrap();
        assert_eq!(a.text_fragments().count(), 1);
        let f = a.text_fragments().get(1).unwrap();
        assert_eq!(f.text(), "World");
        let r = f.rectangle();
        assert!(r.width() > 0.0);
        assert!(r.height() > 0.0);
    }

    #[test]
    fn world_is_right_of_hello() {
        let doc = Document::open(hello_world_pdf()).unwrap();
        let mut h = TextFragmentAbsorber::with_phrase("Hello");
        h.visit_document(&doc).unwrap();
        let mut w = TextFragmentAbsorber::with_phrase("World");
        w.visit_document(&doc).unwrap();
        let hx = h.text_fragments().get(1).unwrap().rectangle().llx();
        let wx = w.text_fragments().get(1).unwrap().rectangle().llx();
        assert!(hx < wx, "Hello llx {hx} should be left of World llx {wx}");
    }

    #[test]
    fn match_all_carries_position_and_state() {
        let doc = Document::open(hello_world_pdf()).unwrap();
        let mut a = TextFragmentAbsorber::new();
        a.visit_document(&doc).unwrap();
        assert!(a.text_fragments().count() >= 1);
        let f = a.text_fragments().get(1).unwrap();
        let r = f.rectangle();
        assert!(r.width() > 0.0 && r.height() > 0.0);
        assert_eq!(f.position().x_indent(), r.llx());
        assert_eq!(f.position().y_indent(), r.lly());
        assert!(f.text_state().font_size() > 0.0);
        assert!(f.text().contains("Hello"));
    }

    #[test]
    fn absent_phrase_yields_none() {
        let doc = Document::open(hello_world_pdf()).unwrap();
        let mut a = TextFragmentAbsorber::with_phrase("NotInThisDocument");
        a.visit_document(&doc).unwrap();
        assert_eq!(a.text_fragments().count(), 0);
    }

    #[test]
    fn empty_phrase_yields_none() {
        let doc = Document::open(hello_world_pdf()).unwrap();
        let mut a = TextFragmentAbsorber::with_phrase("");
        a.visit_document(&doc).unwrap();
        assert_eq!(a.text_fragments().count(), 0);
    }

    #[test]
    fn indexer_out_of_range_is_none() {
        // Rust models the canonical out-of-range throw as `None`.
        let doc = Document::open(hello_world_pdf()).unwrap();
        let mut a = TextFragmentAbsorber::with_phrase("World");
        a.visit_document(&doc).unwrap();
        assert!(a.text_fragments().get(0).is_none());
        assert!(a.text_fragments().get(999).is_none());
    }

    #[test]
    fn visit_page_searches_one_page() {
        let doc = Document::open(hello_world_pdf()).unwrap();
        let page = doc.pages().get(1).unwrap();
        let mut a = TextFragmentAbsorber::with_phrase("World");
        a.visit_page(&page).unwrap();
        assert_eq!(a.text_fragments().count(), 1);
    }

    #[test]
    fn matches_accumulate_across_visits() {
        let doc = Document::open(hello_world_pdf()).unwrap();
        let mut a = TextFragmentAbsorber::with_phrase("Hello");
        a.visit_document(&doc).unwrap();
        a.visit_document(&doc).unwrap();
        assert_eq!(a.text_fragments().count(), 2);
    }
}
