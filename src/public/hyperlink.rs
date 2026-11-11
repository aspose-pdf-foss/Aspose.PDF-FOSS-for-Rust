//! `Aspose.Pdf.Hyperlink` — the abstract base for a page-link
//! target, per `capabilities/hyperlink.yaml`.
//!
//! The canonical type is an abstract base with no members of its
//! own; its concrete subtypes (web / local link targets) carry the
//! destination and arrive with the annotations cluster. Modelled
//! here as an empty marker trait — the Rust analogue of the
//! canonical abstract base (mirroring the `Device` trait pattern).

/// Marker trait for a hyperlink target.
///
/// Concrete link targets implement this trait; it carries no
/// members of its own, matching the canonical abstract
/// `Aspose.Pdf.Hyperlink` base.
pub trait Hyperlink {}
