//! `Aspose.Pdf.Forms.Form.SignDependentElementsRenderingModes` — forms enum, per `capabilities/sign_dependent_elements_rendering_modes.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `SignDependentElementsRenderingModes` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum SignDependentElementsRenderingModes {
    /// `RenderFormAsUnsigned`.
    #[default]
    RenderFormAsUnsigned = 0,
    /// `RenderFormAsSigned`.
    RenderFormAsSigned = 1,
}
