//! `Aspose.Pdf.Annotations.GoToAction` — an action that jumps to an
//! in-document destination, per `capabilities/go_to_action.yaml`.
//!
//! Carries an [`IAppointment`] destination. The canonical
//! constructor overloads collapse to suffix-named factories; the
//! page-number / page overloads default to a `Fit` destination,
//! matching the reference.

use crate::public::document::Document;
use crate::public::explicit_destination::ExplicitDestination;
use crate::public::explicit_destination_type::ExplicitDestinationType;
use crate::public::fit_b_explicit_destination::FitBExplicitDestination;
use crate::public::fit_bh_explicit_destination::FitBHExplicitDestination;
use crate::public::fit_bv_explicit_destination::FitBVExplicitDestination;
use crate::public::fit_explicit_destination::FitExplicitDestination;
use crate::public::fit_h_explicit_destination::FitHExplicitDestination;
use crate::public::fit_r_explicit_destination::FitRExplicitDestination;
use crate::public::fit_v_explicit_destination::FitVExplicitDestination;
use crate::public::i_appointment::IAppointment;
use crate::public::named_destination::NamedDestination;
use crate::public::page::Page;
use crate::public::pdf_action::PdfAction;
use crate::public::xyz_explicit_destination::XYZExplicitDestination;
use std::fmt;

/// A `/GoTo` action targeting an in-document destination.
#[derive(Default)]
pub struct GoToAction {
    destination: Option<Box<dyn IAppointment>>,
}

/// Build an explicit destination of the given fit kind for a page,
/// reading fit parameters from `values` (missing entries default to
/// `0.0`).
fn build_destination(
    page_number: i32,
    fit: ExplicitDestinationType,
    values: &[f64],
) -> Box<dyn IAppointment> {
    let v = |i: usize| values.get(i).copied().unwrap_or(0.0);
    match fit {
        ExplicitDestinationType::XYZ => {
            Box::new(XYZExplicitDestination::new(page_number, v(0), v(1), v(2)))
        }
        ExplicitDestinationType::Fit => Box::new(FitExplicitDestination::new(page_number)),
        ExplicitDestinationType::FitH => Box::new(FitHExplicitDestination::new(page_number, v(0))),
        ExplicitDestinationType::FitV => Box::new(FitVExplicitDestination::new(page_number, v(0))),
        ExplicitDestinationType::FitR => {
            Box::new(FitRExplicitDestination::new(page_number, v(0), v(1), v(2), v(3)))
        }
        ExplicitDestinationType::FitB => Box::new(FitBExplicitDestination::new(page_number)),
        ExplicitDestinationType::FitBH => Box::new(FitBHExplicitDestination::new(page_number, v(0))),
        ExplicitDestinationType::FitBV => Box::new(FitBVExplicitDestination::new(page_number, v(0))),
    }
}

impl GoToAction {
    /// Construct with no destination set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct targeting a 1-based page number (a `Fit`
    /// destination on that page).
    pub fn for_page_number(page_number: i32) -> Self {
        Self {
            destination: Some(Box::new(FitExplicitDestination::new(page_number))),
        }
    }

    /// Construct targeting a [`Page`] (a `Fit` destination).
    pub fn from_page(page: &Page) -> Self {
        Self::for_page_number(page.number() as i32)
    }

    /// Construct targeting a [`Page`] with an explicit fit kind and
    /// its parameters.
    pub fn from_page_with_fit(
        page: &Page,
        fit: ExplicitDestinationType,
        values: Vec<f64>,
    ) -> Self {
        Self {
            destination: Some(build_destination(page.number() as i32, fit, &values)),
        }
    }

    /// Construct from an explicit destination.
    pub fn for_destination(destination: Box<dyn ExplicitDestination>) -> Self {
        Self {
            destination: Some(destination),
        }
    }

    /// Construct targeting a named destination within a [`Document`].
    pub fn for_named_destination(document: &Document, name: String) -> Self {
        Self {
            destination: Some(Box::new(NamedDestination::from_document(document, name))),
        }
    }

    /// The target destination, or `None` when unset.
    pub fn destination(&self) -> Option<&dyn IAppointment> {
        self.destination.as_deref()
    }
    /// Set the target destination.
    pub fn set_destination(&mut self, destination: Box<dyn IAppointment>) {
        self.destination = Some(destination);
    }
}

impl fmt::Debug for GoToAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GoToAction")
            .field(
                "destination",
                &self.destination.as_ref().map(|d| d.to_string()),
            )
            .finish()
    }
}

impl fmt::Display for GoToAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.destination {
            Some(d) => write!(f, "{d}"),
            None => Ok(()),
        }
    }
}

impl IAppointment for GoToAction {}
impl crate::public::pdf_action::sealed::PdfActionEntries for GoToAction {
    fn action_subtype(&self) -> &'static str {
        "GoTo"
    }
    fn action_payload(
        &self,
    ) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        // §12.6.4.2: /D is the destination. A destination renders to
        // its canonical string form, which is what the named-
        // destination tree stores too.
        match &self.destination {
            Some(d) => vec![("D", Entry::Text(d.to_string()))],
            None => Vec::new(),
        }
    }
}

impl PdfAction for GoToAction {}
