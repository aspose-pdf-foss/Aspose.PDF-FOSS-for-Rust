//! `Aspose.Pdf.Forms.BoxStyle` — forms enum, per `capabilities/box_style.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The mark drawn inside a checked check box or a selected radio button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum BoxStyle {
    /// A filled circle.
    #[default]
    Circle = 0,
    /// A tick.
    Check = 1,
    /// A cross.
    Cross = 2,
    /// A filled diamond.
    Diamond = 3,
    /// A filled square.
    Square = 4,
    /// A five-pointed star.
    Star = 5,
}

impl BoxStyle {
    /// The ZapfDingbats character a viewer draws this mark with, as the
    /// `/MK /CA` caption of a check box or radio button (§12.5.6.19
    /// Table 189). The conventional codes: `4` a check, `8` a cross,
    /// `l` a circle, `u` a diamond, `H` a star, `n` a square — which is
    /// why [`FormFieldFacade`](crate::FormFieldFacade) spells its
    /// check-box constants as exactly those character codes.
    pub(crate) fn zapf_caption(self) -> &'static str {
        match self {
            BoxStyle::Circle => "l",
            BoxStyle::Check => "4",
            BoxStyle::Cross => "8",
            BoxStyle::Diamond => "u",
            BoxStyle::Square => "n",
            BoxStyle::Star => "H",
        }
    }
}
