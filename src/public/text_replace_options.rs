//! `Aspose.Pdf.Text.TextReplaceOptions` — options controlling how a
//! text replacement is applied, per
//! `capabilities/text_replace_options.yaml`.
//!
//! Canonically extends the empty `TextOptions` base. The bag carries
//! the replacement scope, the layout/font adjustment behaviours, and
//! an optional bounding rectangle.
//!
//! **Carried, not consulted**, for the same reason as
//! [`TextEditOptions`](crate::TextEditOptions): the text-replacement
//! capability they parameterise is not wired.

use crate::public::font_size_adjustment::FontSizeAdjustment;
use crate::public::rectangle::Rectangle;
use crate::public::replace_adjustment::ReplaceAdjustment;
use crate::public::scope::Scope;

/// Options controlling how a text replacement is applied.
#[derive(Debug, Clone, Default)]
pub struct TextReplaceOptions {
    replace_scope: Scope,
    replace_adjustment_action: ReplaceAdjustment,
    adjustment_new_line_spacing: f64,
    right_adjustment: f64,
    left_adjustment: f64,
    ignore_paragraphs: bool,
    rectangle: Option<Rectangle>,
    font_size_adjustment_action: FontSizeAdjustment,
}

impl TextReplaceOptions {
    /// Create options with the given replacement [`Scope`].
    pub fn with_scope(scope: Scope) -> Self {
        Self { replace_scope: scope, ..Self::default() }
    }

    /// Create options with the given [`ReplaceAdjustment`] behaviour.
    pub fn with_adjustment(adjustment: ReplaceAdjustment) -> Self {
        Self { replace_adjustment_action: adjustment, ..Self::default() }
    }

    /// The replacement scope (first match vs all).
    pub fn replace_scope(&self) -> Scope {
        self.replace_scope
    }

    /// Set the replacement scope.
    pub fn set_replace_scope(&mut self, value: Scope) {
        self.replace_scope = value;
    }

    /// The layout-adjustment behaviour applied after replacement.
    pub fn replace_adjustment_action(&self) -> ReplaceAdjustment {
        self.replace_adjustment_action
    }

    /// Set the layout-adjustment behaviour applied after replacement.
    pub fn set_replace_adjustment_action(&mut self, value: ReplaceAdjustment) {
        self.replace_adjustment_action = value;
    }

    /// The line spacing used when the adjustment adds a new line.
    pub fn adjustment_new_line_spacing(&self) -> f64 {
        self.adjustment_new_line_spacing
    }

    /// Set the line spacing used when the adjustment adds a new line.
    pub fn set_adjustment_new_line_spacing(&mut self, value: f64) {
        self.adjustment_new_line_spacing = value;
    }

    /// The adjustment applied to the right of the replaced text.
    pub fn right_adjustment(&self) -> f64 {
        self.right_adjustment
    }

    /// Set the adjustment applied to the right of the replaced text.
    pub fn set_right_adjustment(&mut self, value: f64) {
        self.right_adjustment = value;
    }

    /// The adjustment applied to the left of the replaced text.
    pub fn left_adjustment(&self) -> f64 {
        self.left_adjustment
    }

    /// Set the adjustment applied to the left of the replaced text.
    pub fn set_left_adjustment(&mut self, value: f64) {
        self.left_adjustment = value;
    }

    /// Whether paragraph boundaries are ignored during replacement.
    pub fn ignore_paragraphs(&self) -> bool {
        self.ignore_paragraphs
    }

    /// Set whether paragraph boundaries are ignored during replacement.
    pub fn set_ignore_paragraphs(&mut self, value: bool) {
        self.ignore_paragraphs = value;
    }

    /// The bounding rectangle the replacement is limited to, if any.
    pub fn rectangle(&self) -> Option<Rectangle> {
        self.rectangle
    }

    /// Set the bounding rectangle the replacement is limited to.
    pub fn set_rectangle(&mut self, value: Rectangle) {
        self.rectangle = Some(value);
    }

    /// The font-size adjustment behaviour applied to the replacement.
    pub fn font_size_adjustment_action(&self) -> FontSizeAdjustment {
        self.font_size_adjustment_action
    }

    /// Set the font-size adjustment behaviour applied to the replacement.
    pub fn set_font_size_adjustment_action(&mut self, value: FontSizeAdjustment) {
        self.font_size_adjustment_action = value;
    }
}
