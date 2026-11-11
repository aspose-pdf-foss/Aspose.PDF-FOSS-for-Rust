//! `Aspose.Pdf.Annotations.GoToURIAction` — an action that opens a
//! URI, per `capabilities/go_to_uri_action.yaml`.

use crate::public::i_appointment::IAppointment;
use crate::public::pdf_action::PdfAction;
use std::fmt;

/// A `/URI` action that navigates to an external address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoToURIAction {
    uri: String,
}

impl GoToURIAction {
    /// Construct from a URI string.
    pub fn new(uri: String) -> Self {
        Self { uri }
    }

    /// The target URI.
    pub fn uri(&self) -> &str {
        &self.uri
    }
    /// Set the target URI.
    pub fn set_uri(&mut self, value: String) {
        self.uri = value;
    }
}

impl fmt::Display for GoToURIAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.uri)
    }
}

impl IAppointment for GoToURIAction {}
impl crate::public::pdf_action::sealed::PdfActionEntries for GoToURIAction {
    fn action_subtype(&self) -> &'static str {
        "URI"
    }
    fn action_payload(
        &self,
    ) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        // §12.6.4.7: /URI is the target, as a plain string.
        if self.uri.is_empty() {
            Vec::new()
        } else {
            vec![("URI", Entry::Text(self.uri.clone()))]
        }
    }
}

impl PdfAction for GoToURIAction {}
