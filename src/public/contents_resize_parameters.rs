//! `Aspose.Pdf.Facades.PdfFileEditor.ContentsResizeParameters` — the
//! parameter bag for the file editor's contents-resize operation, per
//! `capabilities/contents_resize_parameters.yaml`.
//!
//! Holds a [`ContentsResizeValue`] for each of the four margins and the
//! contents width / height, plus a `ChangeMediaBox` flag. Built through
//! the default / six-value constructors or the static factories
//! ([`margins`](ContentsResizeParameters::margins),
//! [`content_size`](ContentsResizeParameters::content_size),
//! [`page_resize`](ContentsResizeParameters::page_resize), and their
//! percent variants). `PageResize` additionally sets
//! [`change_media_box`](ContentsResizeParameters::change_media_box),
//! resizing the page rather than only its contents.
//!
//! **Carried, not consulted.** Nothing reads these: the content-resize
//! facade operation they parameterise is not in the shipped subset.

use crate::public::contents_resize_value::ContentsResizeValue;

/// The parameters governing a contents-resize operation.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ContentsResizeParameters {
    change_media_box: bool,
    left_margin: ContentsResizeValue,
    right_margin: ContentsResizeValue,
    top_margin: ContentsResizeValue,
    bottom_margin: ContentsResizeValue,
    contents_width: ContentsResizeValue,
    contents_height: ContentsResizeValue,
}

impl ContentsResizeParameters {
    /// Construct with every measure auto and the media box unchanged.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct from the six resize measures, in the canonical
    /// parameter order (left margin, contents width, right margin, top
    /// margin, contents height, bottom margin).
    pub fn with_values(
        left_margin: ContentsResizeValue,
        contents_width: ContentsResizeValue,
        right_margin: ContentsResizeValue,
        top_margin: ContentsResizeValue,
        contents_height: ContentsResizeValue,
        bottom_margin: ContentsResizeValue,
    ) -> Self {
        Self {
            change_media_box: false,
            left_margin,
            right_margin,
            top_margin,
            bottom_margin,
            contents_width,
            contents_height,
        }
    }

    /// Parameters that set the four margins to absolute unit amounts.
    pub fn margins(left: f64, right: f64, top: f64, bottom: f64) -> Self {
        Self {
            left_margin: ContentsResizeValue::units(left),
            right_margin: ContentsResizeValue::units(right),
            top_margin: ContentsResizeValue::units(top),
            bottom_margin: ContentsResizeValue::units(bottom),
            ..Self::default()
        }
    }

    /// Parameters that set the four margins to percentages.
    pub fn margins_percent(left: f64, right: f64, top: f64, bottom: f64) -> Self {
        Self {
            left_margin: ContentsResizeValue::percents(left),
            right_margin: ContentsResizeValue::percents(right),
            top_margin: ContentsResizeValue::percents(top),
            bottom_margin: ContentsResizeValue::percents(bottom),
            ..Self::default()
        }
    }

    /// Parameters that set the contents size to absolute unit amounts.
    pub fn content_size(width: f64, height: f64) -> Self {
        Self {
            contents_width: ContentsResizeValue::units(width),
            contents_height: ContentsResizeValue::units(height),
            ..Self::default()
        }
    }

    /// Parameters that set the contents size to percentages.
    pub fn content_size_percent(width: f64, height: f64) -> Self {
        Self {
            contents_width: ContentsResizeValue::percents(width),
            contents_height: ContentsResizeValue::percents(height),
            ..Self::default()
        }
    }

    /// Parameters that resize the page to absolute unit dimensions
    /// (also changing the media box).
    pub fn page_resize(width: f64, height: f64) -> Self {
        Self {
            change_media_box: true,
            contents_width: ContentsResizeValue::units(width),
            contents_height: ContentsResizeValue::units(height),
            ..Self::default()
        }
    }

    /// Parameters that resize the page by percentages (also changing
    /// the media box).
    pub fn page_resize_pct(width_pct: f64, height_pct: f64) -> Self {
        Self {
            change_media_box: true,
            contents_width: ContentsResizeValue::percents(width_pct),
            contents_height: ContentsResizeValue::percents(height_pct),
            ..Self::default()
        }
    }

    /// Whether resizing also changes the page's media box.
    pub fn change_media_box(&self) -> bool {
        self.change_media_box
    }
    /// Set whether resizing also changes the page's media box.
    pub fn set_change_media_box(&mut self, value: bool) {
        self.change_media_box = value;
    }

    /// The left-margin measure.
    pub fn left_margin(&self) -> ContentsResizeValue {
        self.left_margin
    }
    /// Set the left-margin measure.
    pub fn set_left_margin(&mut self, value: ContentsResizeValue) {
        self.left_margin = value;
    }

    /// The right-margin measure.
    pub fn right_margin(&self) -> ContentsResizeValue {
        self.right_margin
    }
    /// Set the right-margin measure.
    pub fn set_right_margin(&mut self, value: ContentsResizeValue) {
        self.right_margin = value;
    }

    /// The top-margin measure.
    pub fn top_margin(&self) -> ContentsResizeValue {
        self.top_margin
    }
    /// Set the top-margin measure.
    pub fn set_top_margin(&mut self, value: ContentsResizeValue) {
        self.top_margin = value;
    }

    /// The bottom-margin measure.
    pub fn bottom_margin(&self) -> ContentsResizeValue {
        self.bottom_margin
    }
    /// Set the bottom-margin measure.
    pub fn set_bottom_margin(&mut self, value: ContentsResizeValue) {
        self.bottom_margin = value;
    }

    /// The contents-width measure.
    pub fn contents_width(&self) -> ContentsResizeValue {
        self.contents_width
    }
    /// Set the contents-width measure.
    pub fn set_contents_width(&mut self, value: ContentsResizeValue) {
        self.contents_width = value;
    }

    /// The contents-height measure.
    pub fn contents_height(&self) -> ContentsResizeValue {
        self.contents_height
    }
    /// Set the contents-height measure.
    pub fn set_contents_height(&mut self, value: ContentsResizeValue) {
        self.contents_height = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_auto_no_mediabox() {
        let p = ContentsResizeParameters::new();
        assert!(!p.change_media_box());
        assert!(!p.left_margin().is_percent());
        assert_eq!(p.contents_width().value(), 0.0);
    }

    #[test]
    fn margins_are_units() {
        let p = ContentsResizeParameters::margins(1.0, 2.0, 3.0, 4.0);
        assert_eq!(p.left_margin().value(), 1.0);
        assert_eq!(p.right_margin().value(), 2.0);
        assert_eq!(p.top_margin().value(), 3.0);
        assert_eq!(p.bottom_margin().value(), 4.0);
        assert!(!p.left_margin().is_percent());
        assert!(!p.change_media_box());
    }

    #[test]
    fn margins_percent_are_percent() {
        let p = ContentsResizeParameters::margins_percent(10.0, 20.0, 30.0, 40.0);
        assert!(p.left_margin().is_percent());
        assert_eq!(p.bottom_margin().value(), 40.0);
    }

    #[test]
    fn content_size_sets_dimensions() {
        let p = ContentsResizeParameters::content_size(400.0, 600.0);
        assert_eq!(p.contents_width().value(), 400.0);
        assert_eq!(p.contents_height().value(), 600.0);
        assert!(!p.contents_width().is_percent());
        assert!(!p.change_media_box());
    }

    #[test]
    fn page_resize_changes_media_box() {
        let p = ContentsResizeParameters::page_resize(500.0, 700.0);
        assert!(p.change_media_box());
        assert_eq!(p.contents_width().value(), 500.0);
        let pct = ContentsResizeParameters::page_resize_pct(90.0, 110.0);
        assert!(pct.change_media_box());
        assert!(pct.contents_height().is_percent());
    }

    #[test]
    fn six_value_ctor_maps_order() {
        let p = ContentsResizeParameters::with_values(
            ContentsResizeValue::units(1.0),
            ContentsResizeValue::units(2.0),
            ContentsResizeValue::units(3.0),
            ContentsResizeValue::units(4.0),
            ContentsResizeValue::units(5.0),
            ContentsResizeValue::units(6.0),
        );
        assert_eq!(p.left_margin().value(), 1.0);
        assert_eq!(p.contents_width().value(), 2.0);
        assert_eq!(p.right_margin().value(), 3.0);
        assert_eq!(p.top_margin().value(), 4.0);
        assert_eq!(p.contents_height().value(), 5.0);
        assert_eq!(p.bottom_margin().value(), 6.0);
    }

    #[test]
    fn setters_round_trip() {
        let mut p = ContentsResizeParameters::new();
        p.set_change_media_box(true);
        p.set_left_margin(ContentsResizeValue::percents(25.0));
        assert!(p.change_media_box());
        assert!(p.left_margin().is_percent());
        assert_eq!(p.left_margin().value(), 25.0);
    }
}
