//! `Aspose.Pdf.Forms.Form.SignDependentElementsRenderingModes` — forms enum, per `capabilities/sign_dependent_elements_rendering_modes.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// Whether a form's signature-dependent elements are rendered as signed or unsigned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum SignDependentElementsRenderingModes {
    /// Draw the form as it appears before signing.
    #[default]
    RenderFormAsUnsigned = 0,
    /// Draw the form as it appears once signed.
    RenderFormAsSigned = 1,
}
