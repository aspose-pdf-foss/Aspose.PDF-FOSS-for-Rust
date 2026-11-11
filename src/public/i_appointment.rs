//! `Aspose.Pdf.Annotations.IAppointment` — the shared interface for
//! navigation targets (explicit / named destinations) and actions,
//! per `capabilities/i_appointment.yaml`.
//!
//! The canonical interface's single member is `ToString`, which
//! renders the target as its PDF destination-array / action string.
//! Modelled here as a marker trait requiring [`std::fmt::Display`]
//! — `Display` provides the `to_string` rendering, the Rust
//! analogue of the canonical `ToString`.

/// Marker trait for a navigation target or action.
///
/// Implementors render to their canonical string form via
/// [`std::fmt::Display`] (the Rust analogue of `IAppointment.ToString`).
pub trait IAppointment: std::fmt::Display {}
