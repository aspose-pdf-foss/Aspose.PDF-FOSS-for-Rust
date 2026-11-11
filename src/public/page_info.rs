//! `Aspose.Pdf.PageInfo` — the canonical default page-setup carrier,
//! per `capabilities/page_info.yaml`.
//!
//! A value type carrying the page size ([`width`](PageInfo::width) /
//! [`height`](PageInfo::height)), orientation
//! ([`is_landscape`](PageInfo::is_landscape)), the page
//! [`margin`](PageInfo::margin) and an optional override
//! [`any_margin`](PageInfo::any_margin), and the
//! [`default_text_state`](PageInfo::default_text_state). Defaults match
//! the canonical A4 page setup. The canonical `ICloneable.Clone`
//! (returns `System.Object`) is dropped in favour of Rust's derived
//! `Clone`.
//!
//! **Carried, not consulted — the whole carrier, not only a property or
//! two.** Nothing in this crate reads a `PageInfo`: a page added through
//! [`PageCollection::add`](crate::PageCollection::add) is synthesised at
//! US Letter, and the generation path that would take a default page
//! setup — size, orientation, margins, the text state its text starts
//! in — does not exist. The values round-trip and decide nothing, which
//! is why no property here is singled out: they are all in the same
//! position.

use crate::public::margin_info::MarginInfo;
use crate::public::text_state::TextState;

/// Default page-setup parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct PageInfo {
    width: f64,
    height: f64,
    is_landscape: bool,
    margin: MarginInfo,
    any_margin: MarginInfo,
    default_text_state: TextState,
}

impl PageInfo {
    /// Construct the default page setup: A4 (595 × 842 pt), portrait,
    /// with the canonical `90 / 72` left-right / top-bottom margins.
    pub fn new() -> Self {
        Self {
            width: 595.0,
            height: 842.0,
            is_landscape: false,
            // MarginInfo order is (left, bottom, right, top).
            margin: MarginInfo::with_margins(90.0, 72.0, 90.0, 72.0),
            any_margin: MarginInfo::new(),
            default_text_state: TextState::default(),
        }
    }

    /// The page width, in points.
    pub fn width(&self) -> f64 {
        self.width
    }
    /// Set the page width, in points.
    pub fn set_width(&mut self, value: f64) {
        self.width = value;
    }

    /// The page height, in points.
    pub fn height(&self) -> f64 {
        self.height
    }
    /// Set the page height, in points.
    pub fn set_height(&mut self, value: f64) {
        self.height = value;
    }

    /// The content height: [`height`](PageInfo::height) minus the top
    /// and bottom margins.
    pub fn pure_height(&self) -> f64 {
        self.height - self.margin.top() - self.margin.bottom()
    }

    /// Whether the page is laid out in landscape orientation.
    pub fn is_landscape(&self) -> bool {
        self.is_landscape
    }
    /// Set the landscape-orientation flag.
    pub fn set_is_landscape(&mut self, value: bool) {
        self.is_landscape = value;
    }

    /// The page margins.
    pub fn margin(&self) -> MarginInfo {
        self.margin
    }
    /// Set the page margins.
    pub fn set_margin(&mut self, value: MarginInfo) {
        self.margin = value;
    }

    /// The override margins applied when set.
    pub fn any_margin(&self) -> MarginInfo {
        self.any_margin
    }
    /// Set the override margins.
    pub fn set_any_margin(&mut self, value: MarginInfo) {
        self.any_margin = value;
    }

    /// The default text state applied to generated text.
    pub fn default_text_state(&self) -> &TextState {
        &self.default_text_state
    }
    /// Set the default text state.
    pub fn set_default_text_state(&mut self, value: TextState) {
        self.default_text_state = value;
    }
}

impl Default for PageInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_a4_portrait() {
        let p = PageInfo::new();
        assert_eq!(p.width(), 595.0);
        assert_eq!(p.height(), 842.0);
        assert!(!p.is_landscape());
        assert_eq!(p.margin().top(), 72.0);
        assert_eq!(p.margin().left(), 90.0);
    }

    #[test]
    fn pure_height_subtracts_vertical_margins() {
        let p = PageInfo::new();
        // 842 - 72 (top) - 72 (bottom) = 698, matching canonical.
        assert_eq!(p.pure_height(), 698.0);
    }

    #[test]
    fn setters_round_trip() {
        let mut p = PageInfo::new();
        p.set_width(612.0);
        p.set_height(792.0);
        p.set_is_landscape(true);
        p.set_margin(MarginInfo::with_margins(10.0, 20.0, 30.0, 40.0));
        assert_eq!(p.width(), 612.0);
        assert_eq!(p.height(), 792.0);
        assert!(p.is_landscape());
        assert_eq!(p.pure_height(), 792.0 - 40.0 - 20.0);
    }
}
