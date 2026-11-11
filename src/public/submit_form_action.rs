//! `Aspose.Pdf.Annotations.SubmitFormAction` — an action that
//! submits form data to a URL, per
//! `capabilities/submit_form_action.yaml`.
//!
//! The canonical submission-flag bit constants are exposed as
//! associated `i32` constants (Rust SCREAMING_SNAKE names mapping
//! to the canonical PascalCase fields).

use crate::public::file_specification::FileSpecification;
use crate::public::i_appointment::IAppointment;
use crate::public::pdf_action::PdfAction;
use std::fmt;

/// A `/SubmitForm` action submitting form fields to a URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitFormAction {
    flags: i32,
    url: FileSpecification,
}

impl SubmitFormAction {
    /// Submit field names listed in `Fields` are *excluded* rather
    /// than included (canonical `Exclude`).
    pub const EXCLUDE: i32 = 1;
    /// Include fields even when they have no value (canonical
    /// `IncludeNoValueFields`).
    pub const INCLUDE_NO_VALUE_FIELDS: i32 = 2;
    /// Submit as an HTML form (canonical `ExportFormat`).
    pub const EXPORT_FORMAT: i32 = 4;
    /// Use HTTP GET rather than POST (canonical `GetMethod`).
    pub const GET_METHOD: i32 = 8;
    /// Include the submit coordinates (canonical `SubmitCoordinates`).
    pub const SUBMIT_COORDINATES: i32 = 16;
    /// Submit as XFDF (canonical `Xfdf`).
    pub const XFDF: i32 = 32;
    /// Include incremental updates (canonical `IncludeAppendSaves`).
    pub const INCLUDE_APPEND_SAVES: i32 = 64;
    /// Include annotations (canonical `IncludeAnnotations`).
    pub const INCLUDE_ANNOTATIONS: i32 = 128;
    /// Submit the whole PDF (canonical `SubmitPdf`).
    pub const SUBMIT_PDF: i32 = 256;
    /// Use the canonical date format (canonical `CanonicalFormat`).
    pub const CANONICAL_FORMAT: i32 = 512;
    /// Exclude non-user annotations (canonical `ExclNonUserAnnots`).
    pub const EXCL_NON_USER_ANNOTS: i32 = 1024;
    /// Exclude the `F` key (canonical `ExclFKey`).
    pub const EXCL_F_KEY: i32 = 2048;
    /// Embed the form in the submission (canonical `EmbedForm`).
    pub const EMBED_FORM: i32 = 8192;

    /// Construct an empty submit-form action (no flags, empty URL).
    pub fn new() -> Self {
        Self {
            flags: 0,
            url: FileSpecification::new(),
        }
    }

    /// The submission flag bitmask.
    pub fn flags(&self) -> i32 {
        self.flags
    }
    /// Set the submission flag bitmask.
    pub fn set_flags(&mut self, value: i32) {
        self.flags = value;
    }

    /// The submission target URL (as a file specification).
    pub fn url(&self) -> &FileSpecification {
        &self.url
    }
    /// Set the submission target URL.
    pub fn set_url(&mut self, value: FileSpecification) {
        self.url = value;
    }
}

impl Default for SubmitFormAction {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SubmitFormAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.url.name())
    }
}

impl IAppointment for SubmitFormAction {}
impl crate::public::pdf_action::sealed::PdfActionEntries for SubmitFormAction {
    fn action_subtype(&self) -> &'static str {
        "SubmitForm"
    }
    fn action_payload(
        &self,
    ) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        // §12.6.4.5: /F is the submission target and /Flags the
        // submission options. The target is a file specification; only
        // its name is carried here, which is the URL form in practice.
        let mut out = Vec::new();
        let name = self.url.name();
        if !name.is_empty() {
            out.push(("F", Entry::Text(name.to_string())));
        }
        if self.flags != 0 {
            out.push(("Flags", Entry::Int(self.flags as i64)));
        }
        out
    }
}

impl PdfAction for SubmitFormAction {}
