//! `Aspose.Pdf.Facades.PdfFileEditor.ContentsResizeValue` — one
//! resize amount used by the file editor's contents-resize parameters,
//! per `capabilities/contents_resize_value.yaml`.
//!
//! A value type carrying a single measure that is either a percentage,
//! an absolute unit amount, or auto. Built through the static
//! [`percents`](ContentsResizeValue::percents) /
//! [`units`](ContentsResizeValue::units) /
//! [`auto`](ContentsResizeValue::auto) factories; the write-only
//! `PercentValue` / `UnitValue` setters re-assign the measure, and the
//! read-only [`value`](ContentsResizeValue::value) /
//! [`is_percent`](ContentsResizeValue::is_percent) accessors read it
//! back. `Auto` reads back as a non-percent zero — the only distinction
//! the shipped subset exposes.

/// A single contents-resize measure (percent / units / auto).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ContentsResizeValue {
    value: f64,
    is_percent: bool,
}

impl ContentsResizeValue {
    /// A percentage resize measure.
    pub fn percents(value: f64) -> Self {
        Self { value, is_percent: true }
    }

    /// An absolute unit resize measure.
    pub fn units(value: f64) -> Self {
        Self { value, is_percent: false }
    }

    /// An automatic resize measure.
    pub fn auto() -> Self {
        Self { value: 0.0, is_percent: false }
    }

    /// Set the measure as a percentage.
    pub fn set_percent_value(&mut self, value: f64) {
        self.value = value;
        self.is_percent = true;
    }

    /// Set the measure as an absolute unit amount.
    pub fn set_unit_value(&mut self, value: f64) {
        self.value = value;
        self.is_percent = false;
    }

    /// The numeric measure.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Whether the measure is a percentage.
    pub fn is_percent(&self) -> bool {
        self.is_percent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percents_units_auto() {
        let p = ContentsResizeValue::percents(80.0);
        assert_eq!(p.value(), 80.0);
        assert!(p.is_percent());

        let u = ContentsResizeValue::units(200.0);
        assert_eq!(u.value(), 200.0);
        assert!(!u.is_percent());

        let a = ContentsResizeValue::auto();
        assert_eq!(a.value(), 0.0);
        assert!(!a.is_percent());
    }

    #[test]
    fn setters_reassign_kind() {
        let mut v = ContentsResizeValue::units(10.0);
        v.set_percent_value(50.0);
        assert_eq!(v.value(), 50.0);
        assert!(v.is_percent());
        v.set_unit_value(12.0);
        assert_eq!(v.value(), 12.0);
        assert!(!v.is_percent());
    }

    #[test]
    fn default_is_auto_like() {
        let d = ContentsResizeValue::default();
        assert_eq!(d.value(), 0.0);
        assert!(!d.is_percent());
    }
}
