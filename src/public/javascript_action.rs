//! `Aspose.Pdf.Annotations.JavascriptAction` — an action that runs
//! a JavaScript snippet, per `capabilities/javascript_action.yaml`.

use crate::public::i_appointment::IAppointment;
use crate::public::pdf_action::PdfAction;
use std::fmt;

/// A `/JavaScript` action carrying a script to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavascriptAction {
    script: String,
}

impl JavascriptAction {
    /// Construct from a JavaScript source string.
    pub fn new(script: String) -> Self {
        Self { script }
    }

    /// The JavaScript source.
    pub fn script(&self) -> &str {
        &self.script
    }
    /// Set the JavaScript source.
    pub fn set_script(&mut self, value: String) {
        self.script = value;
    }
}

impl fmt::Display for JavascriptAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.script)
    }
}

impl IAppointment for JavascriptAction {}

impl crate::public::pdf_action::sealed::PdfActionEntries for JavascriptAction {
    fn action_subtype(&self) -> &'static str {
        "JavaScript"
    }
    fn action_payload(
        &self,
    ) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        // §12.6.4.16: /JS may be a string or a stream; a string is
        // what a script of this size wants.
        if self.script.is_empty() {
            Vec::new()
        } else {
            vec![("JS", Entry::Text(self.script.clone()))]
        }
    }
}

impl PdfAction for JavascriptAction {
    fn get_ecma_script_string(&self) -> String {
        self.script.clone()
    }
}
