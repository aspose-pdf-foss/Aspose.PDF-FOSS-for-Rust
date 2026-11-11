//! Pending-capability marker.
//!
//! `PENDING(<slug>)` marks a public-API body whose real implementation is
//! blocked on a capability that has not shipped yet.
//!
//! Behaviour:
//!
//!   * Panics, naming the blocking capability, so a caller sees exactly
//!     what is missing. The alternative — returning `false`, an empty
//!     collection or `Ok(())` — reads as a successful call that found
//!     nothing or changed nothing, which is indistinguishable from the
//!     real answer and therefore a silent lie.
//!   * Returns the never type `!`, so a call site coerces to whatever the
//!     surrounding method returns and needs no placeholder value.
//!   * Greppable. The backlog walker scans the source tree for
//!     marker occurrences and reports them grouped by slug, so
//!     the marker is what makes the unimplemented surface countable.
//!
//! Convention: the slug names the capability that unblocks the body. When
//! that capability lands, the call site flips from the marker to a real
//! implementation. The name is deliberately uppercase and call-shaped so
//! it matches the neutral marker idiom used across the sibling tracks.

/// Mark this body as pending: panic, naming the blocking capability.
///
/// Never returns, so the call coerces to the enclosing method's return
/// type without a placeholder value.
///
/// # Panics
///
/// Always.
#[allow(non_snake_case)]
pub(crate) fn PENDING(slug: &str) -> ! {
    panic!("Aspose.Pdf: not implemented — blocked on capability '{slug}'")
}
