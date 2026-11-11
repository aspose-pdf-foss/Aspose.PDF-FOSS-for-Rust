//! `Aspose.Pdf.PrintDuplex` — the paper-handling (duplex) mode the
//! viewer should use when printing, per `capabilities/print_duplex.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The duplex printing mode (the catalog's `/ViewerPreferences
/// /Duplex`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum PrintDuplex {
    /// Single-sided printing.
    #[default]
    Simplex = 0,
    /// Duplex, flipping on the short edge.
    DuplexFlipShortEdge = 1,
    /// Duplex, flipping on the long edge.
    DuplexFlipLongEdge = 2,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals_match_canonical() {
        assert_eq!(PrintDuplex::Simplex as i32, 0);
        assert_eq!(PrintDuplex::DuplexFlipShortEdge as i32, 1);
        assert_eq!(PrintDuplex::DuplexFlipLongEdge as i32, 2);
    }

    #[test]
    fn default_is_simplex() {
        assert_eq!(PrintDuplex::default(), PrintDuplex::Simplex);
    }
}
