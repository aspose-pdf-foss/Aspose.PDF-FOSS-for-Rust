//! `Aspose.Pdf.Annotations.NamedDestination` — a navigation target
//! referenced by name (a `/Dest` name entry), per
//! `capabilities/named_destination.yaml`.
//!
//! Carries the destination name and renders to that name via
//! [`std::fmt::Display`] (the canonical `ToString`). It is an
//! [`IAppointment`], so it can be stored as an appointment in a
//! `NamedDestinationCollection`.

use crate::public::document::Document;
use crate::public::i_appointment::IAppointment;
use std::fmt;

/// A named navigation target (a `/Dest` name reference).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedDestination {
    name: String,
}

impl NamedDestination {
    /// Construct a named destination within a [`Document`].
    ///
    /// The document supplies the name resolution context; the
    /// destination itself is identified by `name`.
    pub fn from_document(_document: &Document, name: String) -> Self {
        Self { name }
    }

    /// The destination name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for NamedDestination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl IAppointment for NamedDestination {}
