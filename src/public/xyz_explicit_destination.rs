//! `Aspose.Pdf.Annotations.XYZExplicitDestination` — an explicit
//! destination using the `XYZ` fit, per
//! `capabilities/xyz_explicit_destination.yaml`.
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

/// An explicit destination using the `XYZ` fit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct XYZExplicitDestination {
    page_number: i32,
    left: f64,
    top: f64,
    zoom: f64,
}

impl XYZExplicitDestination {
    /// Construct for a 1-based `page_number`.
    pub fn new(page_number: i32, left: f64, top: f64, zoom: f64) -> Self {
        Self {
            page_number,
            left,
            top,
            zoom,
        }
    }

    /// Construct for a [`Page`] (uses its 1-based page number).
    pub fn from_page(page: &Page, left: f64, top: f64, zoom: f64) -> Self {
        Self::new(page.number() as i32, left, top, zoom)
    }

    /// Construct for a `page_number` within a [`Document`].
    pub fn for_document(_document: &Document, page_number: i32, left: f64, top: f64, zoom: f64) -> Self {
        Self::new(page_number, left, top, zoom)
    }

    /// The left coordinate.
    pub fn left(&self) -> f64 {
        self.left
    }
    /// The top coordinate.
    pub fn top(&self) -> f64 {
        self.top
    }
    /// The zoom factor.
    pub fn zoom(&self) -> f64 {
        self.zoom
    }
}

impl fmt::Display for XYZExplicitDestination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&dest_string(self.page_number, "XYZ", &[self.left, self.top, self.zoom]))
    }
}

impl IAppointment for XYZExplicitDestination {}

impl ExplicitDestination for XYZExplicitDestination {
    fn page_number(&self) -> i32 {
        self.page_number
    }
}
