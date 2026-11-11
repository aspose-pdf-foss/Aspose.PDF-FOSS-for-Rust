//! `Aspose.Pdf.Annotations.ExplicitDestination` — the abstract base
//! for the explicit-destination subtypes, per
//! `capabilities/explicit_destination.yaml`.
//!
//! The canonical base carries the target `PageNumber` and a
//! polymorphic `ToString` rendering the PDF destination array. It
//! is modelled here as a trait (extending [`IAppointment`]) that
//! the eight concrete fit subtypes implement; the shared
//! destination-array formatting lives in `dest_string`.

use crate::public::i_appointment::IAppointment;

/// An explicit (in-document) navigation destination.
///
/// Implemented by the eight fit subtypes (`XYZ`, `Fit`, `FitH`,
/// `FitV`, `FitR`, `FitB`, `FitBH`, `FitBV`). Each renders to its
/// PDF destination-array form via [`std::fmt::Display`] (inherited
/// through [`IAppointment`]).
pub trait ExplicitDestination: IAppointment {
    /// The 1-based number of the target page.
    fn page_number(&self) -> i32;
}

/// Format a PDF destination array: the 1-based page number, the
/// fit verb, then the fit parameters, space-separated. Whole
/// coordinates render without a fractional part (`100`, not
/// `100.0`), matching the canonical `ToString`.
pub(crate) fn dest_string(page_number: i32, verb: &str, params: &[f64]) -> String {
    let mut s = format!("{page_number} {verb}");
    for p in params {
        s.push(' ');
        s.push_str(&format!("{p}"));
    }
    s
}
