//! `Aspose.Pdf.Annotations.FitExplicitDestination` — an explicit
//! destination using the `Fit` fit, per
//! `capabilities/fit_explicit_destination.yaml`.
//!
//! Mirrors the C++ reference: the page-number / page / document
//! constructors (collapsed to suffix-named factories), the
//! coordinate accessors, the [`ExplicitDestination`] page number,
//! and the [`IAppointment`] destination-array `to_string`.

use crate::public::document::Document;
use crate::public::explicit_destination::{dest_string, ExplicitDestination};
use crate::public::i_appointment::IAppointment;
use crate::public::page::Page;
use std::fmt;

/// An explicit destination using the `Fit` fit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FitExplicitDestination {
    page_number: i32,
}

impl FitExplicitDestination {
    /// Construct for a 1-based `page_number`.
    pub fn new(page_number: i32) -> Self {
        Self {
            page_number,
        }
    }

    /// Construct for a [`Page`] (uses its 1-based page number).
    pub fn from_page(page: &Page) -> Self {
        Self::new(page.number() as i32)
    }

    /// Construct for a `page_number` within a [`Document`].
    pub fn for_document(_document: &Document, page_number: i32) -> Self {
        Self::new(page_number)
    }

}

impl fmt::Display for FitExplicitDestination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&dest_string(self.page_number, "Fit", &[]))
    }
}

impl IAppointment for FitExplicitDestination {}

impl ExplicitDestination for FitExplicitDestination {
    fn page_number(&self) -> i32 {
        self.page_number
    }
}
