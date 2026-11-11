//! `Aspose.Pdf.Text.TextSearchOptions` — search options for
//! [`TextFragmentAbsorber`](crate::TextFragmentAbsorber), per
//! `capabilities/text_search_options.yaml`.
//!
//! Canonically extends the empty `TextOptions` base.
//!
//! **Honoured.** [`rectangle`](TextSearchOptions::rectangle) scopes the
//! search to a region, and
//! [`limit_to_page_bounds`](TextSearchOptions::limit_to_page_bounds)
//! drops matches outside the visible page — text can sit beyond the
//! crop box and is extracted all the same, so without the limit a
//! search answers with runs a reader never sees.
//!
//! **Refused, not ignored.**
//! [`is_regular_expression_used`](TextSearchOptions::is_regular_expression_used)
//! asks for a pattern search. There is no regular-expression engine
//! here, and answering a pattern with a literal search is a wrong
//! answer rather than a missing feature — `\d+` would find nothing and
//! report success — so a search with the flag set panics on the
//! capability `text_search_regular_expressions` instead. No peer FOSS
//! track ships a search-options surface at all, so this stays outside
//! the parity ceiling rather than becoming a regex engine of our own.
//!
//! **Carried, not consulted.** The remaining five are extraction-engine
//! knobs with no engine behind them: no shadow-text pass
//! ([`ignore_shadow_text`](TextSearchOptions::ignore_shadow_text)), no
//! graphics-element pass
//! ([`search_for_text_related_graphics`](TextSearchOptions::search_for_text_related_graphics)
//! and its element cap), no alternative font-engine encoding
//! ([`use_font_engine_encoding`](TextSearchOptions::use_font_engine_encoding)),
//! and no extraction log to write or to quieten
//! ([`log_text_extraction_errors`](TextSearchOptions::log_text_extraction_errors),
//! [`ignore_resource_font_errors`](TextSearchOptions::ignore_resource_font_errors)).

use crate::public::rectangle::Rectangle;

/// Options controlling a
/// [`TextFragmentAbsorber`](crate::TextFragmentAbsorber) search.
#[derive(Debug, Clone, Default)]
pub struct TextSearchOptions {
    is_regular_expression_used: bool,
    limit_to_page_bounds: bool,
    rectangle: Option<Rectangle>,
    use_font_engine_encoding: bool,
    ignore_shadow_text: bool,
    log_text_extraction_errors: bool,
    ignore_resource_font_errors: bool,
    search_for_text_related_graphics: bool,
    stored_graphic_elements_max_count: i32,
    search_in_annotations: bool,
}

impl TextSearchOptions {
    /// Create options, choosing whether the search phrase is a regular
    /// expression (regex evaluation is not in the shipped subset).
    pub fn new(is_regular_expression_used: bool) -> Self {
        Self { is_regular_expression_used, ..Self::default() }
    }

    /// Create options that scope the search to `rectangle`.
    pub fn with_rectangle(rectangle: Rectangle) -> Self {
        Self { rectangle: Some(rectangle), ..Self::default() }
    }

    /// Create options that scope the search to `rectangle`, choosing
    /// whether the phrase is a regular expression.
    pub fn with_rectangle_and_regex(
        rectangle: Rectangle,
        is_regular_expression_used: bool,
    ) -> Self {
        Self {
            rectangle: Some(rectangle),
            is_regular_expression_used,
            ..Self::default()
        }
    }

    /// Whether the search phrase is treated as a regular expression.
    pub fn is_regular_expression_used(&self) -> bool {
        self.is_regular_expression_used
    }

    /// Set whether the search phrase is treated as a regular expression.
    pub fn set_is_regular_expression_used(&mut self, value: bool) {
        self.is_regular_expression_used = value;
    }

    /// Whether the search is limited to the page bounds.
    pub fn limit_to_page_bounds(&self) -> bool {
        self.limit_to_page_bounds
    }

    /// Set whether the search is limited to the page bounds.
    pub fn set_limit_to_page_bounds(&mut self, value: bool) {
        self.limit_to_page_bounds = value;
    }

    /// The region the search is scoped to, if any.
    pub fn rectangle(&self) -> Option<Rectangle> {
        self.rectangle
    }

    /// Set the region the search is scoped to.
    pub fn set_rectangle(&mut self, value: Rectangle) {
        self.rectangle = Some(value);
    }

    /// Whether the font-engine encoding is used during extraction.
    pub fn use_font_engine_encoding(&self) -> bool {
        self.use_font_engine_encoding
    }

    /// Set whether the font-engine encoding is used during extraction.
    pub fn set_use_font_engine_encoding(&mut self, value: bool) {
        self.use_font_engine_encoding = value;
    }

    /// Whether shadow (overlapping) text is ignored.
    pub fn ignore_shadow_text(&self) -> bool {
        self.ignore_shadow_text
    }

    /// Set whether shadow (overlapping) text is ignored.
    pub fn set_ignore_shadow_text(&mut self, value: bool) {
        self.ignore_shadow_text = value;
    }

    /// Whether text-extraction errors are logged.
    pub fn log_text_extraction_errors(&self) -> bool {
        self.log_text_extraction_errors
    }

    /// Set whether text-extraction errors are logged.
    pub fn set_log_text_extraction_errors(&mut self, value: bool) {
        self.log_text_extraction_errors = value;
    }

    /// Whether resource-font errors are ignored.
    pub fn ignore_resource_font_errors(&self) -> bool {
        self.ignore_resource_font_errors
    }

    /// Set whether resource-font errors are ignored.
    pub fn set_ignore_resource_font_errors(&mut self, value: bool) {
        self.ignore_resource_font_errors = value;
    }

    /// Whether graphics related to matched text are also collected.
    pub fn search_for_text_related_graphics(&self) -> bool {
        self.search_for_text_related_graphics
    }

    /// Set whether graphics related to matched text are also collected.
    pub fn set_search_for_text_related_graphics(&mut self, value: bool) {
        self.search_for_text_related_graphics = value;
    }

    /// The maximum number of stored graphic elements.
    pub fn stored_graphic_elements_max_count(&self) -> i32 {
        self.stored_graphic_elements_max_count
    }

    /// Set the maximum number of stored graphic elements.
    pub fn set_stored_graphic_elements_max_count(&mut self, value: i32) {
        self.stored_graphic_elements_max_count = value;
    }

    /// Whether text inside annotations is searched.
    pub fn search_in_annotations(&self) -> bool {
        self.search_in_annotations
    }

    /// Set whether text inside annotations is searched.
    pub fn set_search_in_annotations(&mut self, value: bool) {
        self.search_in_annotations = value;
    }
}
