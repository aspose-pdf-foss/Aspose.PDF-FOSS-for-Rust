//! `Aspose.Pdf.LoadOptions` — the base of the import-options types,
//! per `capabilities/load_options.yaml`.
//!
//! Canonically an abstract base (no public constructor). Rust models
//! it as a trait implemented by the concrete options types (e.g.
//! [`SvgLoadOptions`](crate::SvgLoadOptions)).

/// Common options for importing a non-PDF format.
pub trait LoadOptions {
    /// Whether font-license verification is disabled during import.
    fn disable_font_license_verifications(&self) -> bool;
    /// Set whether font-license verification is disabled.
    fn set_disable_font_license_verifications(&mut self, value: bool);
}
