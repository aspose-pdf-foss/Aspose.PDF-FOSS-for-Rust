//! `Aspose.Pdf.PrintScaling` — the page-scaling mode the viewer should
//! use when printing, per `capabilities/print_scaling.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The print-scaling mode (the catalog's `/ViewerPreferences
/// /PrintScaling`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum PrintScaling {
    /// The viewer's default page-scaling behaviour.
    #[default]
    AppDefault = 0,
    /// No page scaling.
    None = 1,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals_match_canonical() {
        assert_eq!(PrintScaling::AppDefault as i32, 0);
        assert_eq!(PrintScaling::None as i32, 1);
    }

    #[test]
    fn default_is_app_default() {
        assert_eq!(PrintScaling::default(), PrintScaling::AppDefault);
    }
}
