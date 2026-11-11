//! `Aspose.Pdf.Annotations.NamedAction` — a built-in viewer action
//! invoked by name, per `capabilities/named_action.yaml`.

use crate::public::i_appointment::IAppointment;
use crate::public::pdf_action::PdfAction;
use crate::public::predefined_action::PredefinedAction;
use std::fmt;

/// A `/Named` action — runs a built-in viewer command identified by
/// its name (e.g. `NextPage`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedAction {
    name: String,
}

impl NamedAction {
    /// Construct from a [`PredefinedAction`]; the name is the
    /// action's canonical identifier.
    pub fn from_predefined(action: PredefinedAction) -> Self {
        Self {
            name: action.name().to_string(),
        }
    }

    /// The action name.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Set the action name.
    pub fn set_name(&mut self, value: String) {
        self.name = value;
    }
}

impl fmt::Display for NamedAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl IAppointment for NamedAction {}
impl crate::public::pdf_action::sealed::PdfActionEntries for NamedAction {
    fn action_subtype(&self) -> &'static str {
        "Named"
    }
    fn action_payload(
        &self,
    ) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        // §12.6.4.11: /N names one of the predefined viewer actions.
        if self.name.is_empty() {
            Vec::new()
        } else {
            vec![("N", Entry::Name(self.name.clone()))]
        }
    }
}

impl PdfAction for NamedAction {}
