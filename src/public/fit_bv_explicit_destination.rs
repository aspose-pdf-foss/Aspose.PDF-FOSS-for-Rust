//! `Aspose.Pdf.Annotations.FitBVExplicitDestination` — an explicit
//! destination using the `FitBV` fit, per
//! `capabilities/fit_bv_explicit_destination.yaml`.
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

/// An explicit destination using the `FitBV` fit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FitBVExplicitDestination {
    page_number: i32,
    left: f64,
}

impl FitBVExplicitDestination {
    /// Construct for a 1-based `page_number`.
    pub fn new(page_number: i32, left: f64) -> Self {
        Self {
            page_number,
            left,
        }
    }

    /// Construct for a [`Page`] (uses its 1-based page number).
    pub fn from_page(page: &Page, left: f64) -> Self {
        Self::new(page.number() as i32, left)
    }

    /// Construct for a `page_number` within a [`Document`].
    pub fn for_document(_document: &Document, page_number: i32, left: f64) -> Self {
        Self::new(page_number, left)
    }

    /// The left coordinate.
    pub fn left(&self) -> f64 {
        self.left
    }
}

impl fmt::Display for FitBVExplicitDestination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&dest_string(self.page_number, "FitBV", &[self.left]))
    }
}

impl IAppointment for FitBVExplicitDestination {}

impl ExplicitDestination for FitBVExplicitDestination {
    fn page_number(&self) -> i32 {
        self.page_number
    }
}
