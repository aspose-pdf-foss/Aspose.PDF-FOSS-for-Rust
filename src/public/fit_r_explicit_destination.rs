//! `Aspose.Pdf.Annotations.FitRExplicitDestination` — an explicit
//! destination using the `FitR` fit, per
//! `capabilities/fit_r_explicit_destination.yaml`.
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

/// An explicit destination using the `FitR` fit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FitRExplicitDestination {
    page_number: i32,
    left: f64,
    bottom: f64,
    right: f64,
    top: f64,
}

impl FitRExplicitDestination {
    /// Construct for a 1-based `page_number`.
    pub fn new(page_number: i32, left: f64, bottom: f64, right: f64, top: f64) -> Self {
        Self {
            page_number,
            left,
            bottom,
            right,
            top,
        }
    }

    /// Construct for a [`Page`] (uses its 1-based page number).
    pub fn from_page(page: &Page, left: f64, bottom: f64, right: f64, top: f64) -> Self {
        Self::new(page.number() as i32, left, bottom, right, top)
    }

    /// Construct for a `page_number` within a [`Document`].
    pub fn for_document(_document: &Document, page_number: i32, left: f64, bottom: f64, right: f64, top: f64) -> Self {
        Self::new(page_number, left, bottom, right, top)
    }

    /// The left coordinate.
    pub fn left(&self) -> f64 {
        self.left
    }
    /// The bottom coordinate.
    pub fn bottom(&self) -> f64 {
        self.bottom
    }
    /// The right coordinate.
    pub fn right(&self) -> f64 {
        self.right
    }
    /// The top coordinate.
    pub fn top(&self) -> f64 {
        self.top
    }
}

impl fmt::Display for FitRExplicitDestination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&dest_string(self.page_number, "FitR", &[self.left, self.bottom, self.right, self.top]))
    }
}

impl IAppointment for FitRExplicitDestination {}

impl ExplicitDestination for FitRExplicitDestination {
    fn page_number(&self) -> i32 {
        self.page_number
    }
}
