//! `Aspose.Pdf.DestinationCollection` — the document's named explicit
//! destinations (`/Names /Dests`), per
//! `capabilities/destination_collection.yaml`.
//!
//! A read view over the name → [`ExplicitDestination`] map. The .NET
//! `ICollection<KeyValuePair<…>>` surface (indexer, enumerator,
//! Add / Remove / Clear / Contains / CopyTo / IndexOf / IsReadOnly) is
//! dropped in favour of the named lookups
//! [`get_explicit_destination`](DestinationCollection::get_explicit_destination)
//! and [`get_page_number`](DestinationCollection::get_page_number).

use std::fmt;

use crate::public::explicit_destination::ExplicitDestination;
use crate::public::explicit_destination_type::ExplicitDestinationType;
use crate::public::fit_b_explicit_destination::FitBExplicitDestination;
use crate::public::fit_bh_explicit_destination::FitBHExplicitDestination;
use crate::public::fit_bv_explicit_destination::FitBVExplicitDestination;
use crate::public::fit_explicit_destination::FitExplicitDestination;
use crate::public::fit_h_explicit_destination::FitHExplicitDestination;
use crate::public::fit_r_explicit_destination::FitRExplicitDestination;
use crate::public::fit_v_explicit_destination::FitVExplicitDestination;
use crate::public::xyz_explicit_destination::XYZExplicitDestination;

struct Entry {
    name: String,
    page: i32,
    dest: Box<dyn ExplicitDestination>,
}

/// The document's named explicit destinations.
#[derive(Default)]
pub struct DestinationCollection {
    entries: Vec<Entry>,
}

impl DestinationCollection {
    /// Parse the `/Names /Dests` name tree from a document's bytes.
    pub(crate) fn from_bytes(bytes: &[u8]) -> Self {
        let mut entries = Vec::new();
        for (name, dest_str) in crate::internal::named_dest_io::parse(bytes) {
            if let Some((page, ty, values)) = parse_dest_string(&dest_str) {
                entries.push(Entry {
                    name,
                    page,
                    dest: build_destination(page, ty, &values),
                });
            }
        }
        Self { entries }
    }

    /// The number of named destinations.
    pub fn count(&self) -> i32 {
        self.entries.len() as i32
    }

    /// The 1-based target page number of the named destination, or `0`
    /// when the name is not present. `use_cache` is accepted for
    /// signature parity; results are computed from the parsed tree.
    pub fn get_page_number(&self, destination_name: &str, use_cache: bool) -> i32 {
        let _ = use_cache;
        self.entries
            .iter()
            .find(|e| e.name == destination_name)
            .map(|e| e.page)
            .unwrap_or(0)
    }

    /// The [`ExplicitDestination`] bound to `destination_name`, or
    /// `None` when the name is not present. `use_cache` is accepted for
    /// signature parity.
    pub fn get_explicit_destination(
        &self,
        destination_name: &str,
        use_cache: bool,
    ) -> Option<&dyn ExplicitDestination> {
        let _ = use_cache;
        self.entries
            .iter()
            .find(|e| e.name == destination_name)
            .map(|e| e.dest.as_ref())
    }
}

impl fmt::Debug for DestinationCollection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DestinationCollection")
            .field("count", &self.entries.len())
            .field(
                "names",
                &self.entries.iter().map(|e| &e.name).collect::<Vec<_>>(),
            )
            .finish()
    }
}

/// Parse a `"{page} {verb} {params…}"` destination string (as emitted by
/// `named_dest_io`) into its page, fit kind, and numeric parameters.
fn parse_dest_string(s: &str) -> Option<(i32, ExplicitDestinationType, Vec<f64>)> {
    let mut it = s.split_whitespace();
    let page: i32 = it.next()?.parse().ok()?;
    let ty = verb_to_type(it.next()?)?;
    let values = it.filter_map(|t| t.parse::<f64>().ok()).collect();
    Some((page, ty, values))
}

fn verb_to_type(verb: &str) -> Option<ExplicitDestinationType> {
    Some(match verb {
        "XYZ" => ExplicitDestinationType::XYZ,
        "Fit" => ExplicitDestinationType::Fit,
        "FitH" => ExplicitDestinationType::FitH,
        "FitV" => ExplicitDestinationType::FitV,
        "FitR" => ExplicitDestinationType::FitR,
        "FitB" => ExplicitDestinationType::FitB,
        "FitBH" => ExplicitDestinationType::FitBH,
        "FitBV" => ExplicitDestinationType::FitBV,
        _ => return None,
    })
}

/// Build the concrete explicit destination for a fit kind, reading
/// parameters from `values` (missing entries default to `0.0`).
fn build_destination(
    page: i32,
    ty: ExplicitDestinationType,
    values: &[f64],
) -> Box<dyn ExplicitDestination> {
    let v = |i: usize| values.get(i).copied().unwrap_or(0.0);
    match ty {
        ExplicitDestinationType::XYZ => {
            Box::new(XYZExplicitDestination::new(page, v(0), v(1), v(2)))
        }
        ExplicitDestinationType::Fit => Box::new(FitExplicitDestination::new(page)),
        ExplicitDestinationType::FitH => Box::new(FitHExplicitDestination::new(page, v(0))),
        ExplicitDestinationType::FitV => Box::new(FitVExplicitDestination::new(page, v(0))),
        ExplicitDestinationType::FitR => {
            Box::new(FitRExplicitDestination::new(page, v(0), v(1), v(2), v(3)))
        }
        ExplicitDestinationType::FitB => Box::new(FitBExplicitDestination::new(page)),
        ExplicitDestinationType::FitBH => Box::new(FitBHExplicitDestination::new(page, v(0))),
        ExplicitDestinationType::FitBV => Box::new(FitBVExplicitDestination::new(page, v(0))),
    }
}
