//! `Aspose.Pdf.Annotations.FitHExplicitDestination` — an explicit
//! destination using the `FitH` fit, per
//! `capabilities/fit_h_explicit_destination.yaml`.
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

/// An explicit destination using the `FitH` fit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FitHExplicitDestination {
    page_number: i32,
    top: f64,
}

impl FitHExplicitDestination {
    /// Construct for a 1-based `page_number`.
    pub fn new(page_number: i32, top: f64) -> Self {
        Self {
            page_number,
            top,
        }
    }

    /// Construct for a [`Page`] (uses its 1-based page number).
    pub fn from_page(page: &Page, top: f64) -> Self {
        Self::new(page.number() as i32, top)
    }

    /// Construct for a `page_number` within a [`Document`].
    pub fn for_document(_document: &Document, page_number: i32, top: f64) -> Self {
        Self::new(page_number, top)
    }

    /// The top coordinate.
    pub fn top(&self) -> f64 {
        self.top
    }
}

impl fmt::Display for FitHExplicitDestination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&dest_string(self.page_number, "FitH", &[self.top]))
    }
}

impl IAppointment for FitHExplicitDestination {}

impl ExplicitDestination for FitHExplicitDestination {
    fn page_number(&self) -> i32 {
        self.page_number
    }
}
