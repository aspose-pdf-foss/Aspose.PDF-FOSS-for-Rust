//! `Aspose.Pdf.Annotations.PdfAction` — the abstract base for
//! document actions, per `capabilities/pdf_action.yaml`.
//!
//! The canonical base is an [`IAppointment`] (so it renders to a
//! string via [`std::fmt::Display`]) and adds `GetECMAScriptString`.
//! Modelled here as a trait the concrete action types implement.
//! The canonical `Next` property (an `ActionCollection`, not yet in
//! the shipped subset) is dropped (see `targets/rust/drops.yaml`).

use crate::public::i_appointment::IAppointment;

pub(crate) mod sealed {
    /// How an action writes itself into a PDF action dictionary
    /// (§12.6). Crate-private so the entry vocabulary stays on the
    /// writer's side of the boundary.
    pub(crate) trait PdfActionEntries {
        /// `/S` — the action's subtype name.
        fn action_subtype(&self) -> &'static str;
        /// The entries that carry the action's payload: `/D` for a
        /// destination, `/URI`, `/JS`, `/N`. An action whose payload is
        /// empty contributes nothing and is not written at all.
        fn action_payload(
            &self,
        ) -> Vec<(&'static str, crate::internal::annotation_io::Entry)>;
    }
}

/// The `/A` entry for `action`, or `None` when the action carries no
/// payload worth writing (§12.6.2: an action dictionary without its
/// defining entry tells a viewer nothing).
pub(crate) fn action_entry(
    action: &dyn PdfAction,
) -> Option<crate::internal::annotation_io::Entry> {
    use crate::internal::annotation_io::Entry;
    let payload = action.action_payload();
    if payload.is_empty() {
        return None;
    }
    let mut entries = vec![("S", Entry::Name(action.action_subtype().to_string()))];
    entries.extend(payload);
    Some(Entry::Dict(entries))
}

/// The text payload `action` carries under `key`, or `None` when the
/// action is of another subtype or has no such entry. Crate-internal:
/// it lets [`FormEditor`](crate::FormEditor) read back the script or
/// the submit URL already on a field, which it holds only as a
/// `&dyn PdfAction`.
pub(crate) fn action_text(action: &dyn PdfAction, subtype: &str, key: &str) -> Option<String> {
    use crate::internal::annotation_io::Entry;
    if action.action_subtype() != subtype {
        return None;
    }
    action.action_payload().into_iter().find_map(|(k, entry)| match entry {
        Entry::Text(text) if k == key => Some(text),
        _ => None,
    })
}

/// A document action (navigation, URI, named, JavaScript, submit).
///
/// Extends [`IAppointment`]: an action renders to its canonical
/// string form via [`std::fmt::Display`].
/// Sealing this trait the way [`Annotation`](crate::Annotation) and
/// [`Field`](crate::Field) are sealed: an external crate cannot name
/// the supertrait, so it cannot add an action type, and the writer's
/// entry vocabulary stays off the public surface. The `private_bounds`
/// lint flags exactly this intentional pattern.
#[allow(private_bounds)]
pub trait PdfAction: IAppointment + sealed::PdfActionEntries {
    /// The JavaScript / ECMAScript source this action runs, or an
    /// empty string for actions that carry no script.
    fn get_ecma_script_string(&self) -> String {
        String::new()
    }
}
