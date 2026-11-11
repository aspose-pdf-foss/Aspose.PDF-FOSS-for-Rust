//! `Aspose.Pdf.ImageDeleteAction` — how a deleted image's page
//! contents are treated, per `capabilities/image_delete_action.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// How a deleted image's page contents are treated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ImageDeleteAction {
    /// Keep the drawing operators that referenced the image.
    #[default]
    KeepContents = 0,
    /// Take no extra action on the page contents.
    None = 1,
    /// Force removal of the image and its references.
    ForceDelete = 2,
    /// Check whether the image is still referenced before deleting.
    Check = 3,
}
