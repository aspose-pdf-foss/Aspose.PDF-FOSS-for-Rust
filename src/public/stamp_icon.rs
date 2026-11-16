//! `Aspose.Pdf.Annotations.StampIcon` — annotation enum, per `capabilities/stamp_icon.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The named stamp a stamp annotation displays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum StampIcon {
    /// Marks the document as a draft.
    #[default]
    Draft = 0,
    /// Marks the document as approved.
    Approved = 1,
    /// Marks the document as experimental.
    Experimental = 2,
    /// Marks the document as not approved.
    NotApproved = 3,
    /// Marks the document as supplied as it stands.
    AsIs = 4,
    /// Marks the document as expired.
    Expired = 5,
    /// Marks the document as not for public release.
    NotForPublicRelease = 6,
    /// Marks the document as confidential.
    Confidential = 7,
    /// Marks the document as final.
    Final = 8,
    /// Marks the item as sold.
    Sold = 9,
    /// Marks the document as departmental.
    Departmental = 10,
    /// Marks the document as circulated for comment.
    ForComment = 11,
    /// Marks the document as cleared for public release.
    ForPublicRelease = 12,
    /// Marks the document as top secret.
    TopSecret = 13,
}
