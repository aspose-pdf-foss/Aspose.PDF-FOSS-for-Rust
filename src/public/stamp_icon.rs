//! `Aspose.Pdf.Annotations.StampIcon` — annotation enum, per `capabilities/stamp_icon.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `StampIcon` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum StampIcon {
    /// `Draft`.
    #[default]
    Draft = 0,
    /// `Approved`.
    Approved = 1,
    /// `Experimental`.
    Experimental = 2,
    /// `NotApproved`.
    NotApproved = 3,
    /// `AsIs`.
    AsIs = 4,
    /// `Expired`.
    Expired = 5,
    /// `NotForPublicRelease`.
    NotForPublicRelease = 6,
    /// `Confidential`.
    Confidential = 7,
    /// `Final`.
    Final = 8,
    /// `Sold`.
    Sold = 9,
    /// `Departmental`.
    Departmental = 10,
    /// `ForComment`.
    ForComment = 11,
    /// `ForPublicRelease`.
    ForPublicRelease = 12,
    /// `TopSecret`.
    TopSecret = 13,
}
