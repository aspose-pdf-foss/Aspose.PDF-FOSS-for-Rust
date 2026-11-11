//! `Aspose.Pdf.AFRelationship` — the relationship an associated
//! (embedded) file bears to its containing document or annotation,
//! per `capabilities/af_relationship.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0
//! `Aspose.Pdf.AFRelationship` enum (PDF 2.0 `/AFRelationship`).

/// Associated-file relationship (`/AFRelationship` in PDF 2.0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum AFRelationship {
    /// The file is the source data for the visible representation.
    Source = 0,
    /// The file is alternative data (e.g. machine-readable form).
    Data = 1,
    /// The file is an alternative representation.
    Alternative = 2,
    /// The file is a supplemental representation.
    Supplement = 3,
    /// No defined relationship.
    #[default]
    Unspecified = 4,
    /// The file is an encrypted payload (PDF 2.0).
    EncryptedPayload = 5,
    /// Explicitly no relationship.
    None = 6,
}

impl AFRelationship {
    /// The `/AFRelationship` name this relationship is written as
    /// (PDF 2.0 §14.13.5), or `None` when there is nothing to record.
    ///
    /// Both [`Unspecified`](Self::Unspecified) and [`None`](Self::None)
    /// answer `None`: the first is the value an absent entry already
    /// means, and the second has no name in the format at all.
    pub(crate) fn pdf_name(self) -> Option<&'static str> {
        match self {
            Self::Source => Some("Source"),
            Self::Data => Some("Data"),
            Self::Alternative => Some("Alternative"),
            Self::Supplement => Some("Supplement"),
            Self::EncryptedPayload => Some("EncryptedPayload"),
            Self::Unspecified | Self::None => Option::None,
        }
    }

    /// The relationship an `/AFRelationship` name denotes.
    /// Unknown names — a later edition's additions among them — read
    /// back as [`Unspecified`](Self::Unspecified), per §14.13.5.
    pub(crate) fn from_pdf_name(name: &str) -> Self {
        match name {
            "Source" => Self::Source,
            "Data" => Self::Data,
            "Alternative" => Self::Alternative,
            "Supplement" => Self::Supplement,
            "EncryptedPayload" => Self::EncryptedPayload,
            _ => Self::Unspecified,
        }
    }
}
