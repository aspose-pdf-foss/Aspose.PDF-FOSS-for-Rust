//! `Aspose.Pdf.Facades.PdfFileInfo` — a facade over a document's
//! `/Info` dictionary and basic file metadata, per
//! `capabilities/pdf_file_info.yaml`.
//!
//! Extends [`SaveableFacade`](crate::SaveableFacade). Reads / writes
//! the standard `/Info` fields, the page geometry, and the file
//! state (page count, version, encryption flags). The `Stream` /
//! `ICustomSecurityHandler` / password constructors, the `Header`
//! dictionary, and the `InputStream` property are dropped (those
//! types / the encrypted-open path are outside the shipped subset).
//!
//! **Carried, not consulted.**
//! [`use_strict_validation`](PdfFileInfo::use_strict_validation)
//! chooses between a strict check and a lenient one. This facade binds
//! by opening the document, which parses the trailer, the cross-reference
//! table and the page tree, so
//! [`is_pdf_file`](PdfFileInfo::is_pdf_file) is always answering the
//! strict question already. What the flag cannot select is the lenient
//! mode — a header sniff that accepts a file this crate would reject.

use crate::public::document::Document;
use crate::public::document_privilege::DocumentPrivilege;
use crate::public::error::Error;
use crate::public::i_facade::{impl_facade, FacadeBase};
use crate::public::i_saveable_facade::impl_saveable_facade;
use crate::public::password_type::PasswordType;
use std::path::Path;

/// A facade over document info + basic file metadata.
#[derive(Debug, Default)]
pub struct PdfFileInfo {
    base: FacadeBase,
    input_file: String,
    use_strict_validation: bool,
}

macro_rules! info_string_prop {
    ($getter:ident, $setter:ident, $inner_get:ident, $inner_set:ident) => {
        #[doc = concat!("The `/Info /", stringify!($getter), "` field.")]
        pub fn $getter(&self) -> String {
            self.base
                .document
                .as_ref()
                .map(|d| d.info().$inner_get().to_string())
                .unwrap_or_default()
        }
        #[doc = "Set the field on the bound document (no-op if unbound)."]
        pub fn $setter(&mut self, value: impl Into<String>) {
            if let Some(d) = self.base.document.as_mut() {
                d.info_mut().$inner_set(value);
            }
        }
    };
}

impl PdfFileInfo {
    /// Construct an unbound facade.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct and bind to a PDF file on disk.
    pub fn with_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path_ref = path.as_ref();
        let document = Document::open(path_ref)?;
        Ok(Self {
            base: FacadeBase { document: Some(document) },
            input_file: path_ref.to_string_lossy().into_owned(),
            use_strict_validation: false,
        })
    }

    /// Construct and bind to an in-memory [`Document`].
    pub fn with_document(document: Document) -> Self {
        Self {
            base: FacadeBase { document: Some(document) },
            input_file: String::new(),
            use_strict_validation: false,
        }
    }

    info_string_prop!(title, set_title, title, set_title);
    info_string_prop!(author, set_author, author, set_author);
    info_string_prop!(subject, set_subject, subject, set_subject);
    info_string_prop!(keywords, set_keywords, keywords, set_keywords);
    info_string_prop!(creator, set_creator, creator, set_creator);
    info_string_prop!(producer, set_producer, producer, set_producer);
    info_string_prop!(creation_date, set_creation_date, creation_date, set_creation_date);
    info_string_prop!(mod_date, set_mod_date, mod_date, set_mod_date);

    /// Whether a PDF is bound.
    pub fn is_pdf_file(&self) -> bool {
        self.base.document.is_some()
    }

    /// Whether the bound document is encrypted (always false in the
    /// shipped subset — encrypted open is a follow-on).
    pub fn is_encrypted(&self) -> bool {
        false
    }

    /// The number of pages (0 when unbound).
    pub fn number_of_pages(&self) -> i32 {
        self.base.document.as_ref().map_or(0, |d| d.pages().count() as i32)
    }

    /// Whether strict validation is enabled.
    pub fn use_strict_validation(&self) -> bool {
        self.use_strict_validation
    }
    /// Set whether strict validation is enabled.
    pub fn set_use_strict_validation(&mut self, value: bool) {
        self.use_strict_validation = value;
    }

    /// Whether the document is a portable collection (portfolio).
    pub fn has_collection(&self) -> bool {
        false
    }

    /// The bound file path (empty when bound by document / unbound).
    pub fn input_file(&self) -> &str {
        &self.input_file
    }

    /// The password kind the document was opened with.
    pub fn password_type(&self) -> PasswordType {
        PasswordType::None
    }

    /// Whether the document has an open (user) password.
    pub fn has_open_password(&self) -> bool {
        false
    }
    /// Whether the document has an edit (owner) password.
    pub fn has_edit_password(&self) -> bool {
        false
    }

    /// Clear the `/Info` dictionary.
    pub fn clear_info(&mut self) {
        if let Some(d) = self.base.document.as_mut() {
            d.info_mut().clear();
        }
    }

    /// The document's access privileges.
    pub fn get_document_privilege(&self) -> DocumentPrivilege {
        DocumentPrivilege::allow_all()
    }

    /// A custom `/Info` field value (empty if absent / unbound).
    pub fn get_meta_info(&self, key: &str) -> String {
        self.base
            .document
            .as_ref()
            .and_then(|d| {
                d.info().entries().iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
            })
            .unwrap_or_default()
    }

    /// Set a custom `/Info` field value (no-op if unbound).
    pub fn set_meta_info(&mut self, key: &str, value: impl Into<String>) {
        if let Some(d) = self.base.document.as_mut() {
            let info = d.info_mut();
            info.remove(key);
            let _ = info.add(key, value.into());
        }
    }

    /// The width of a 1-based page (0 if out of range).
    pub fn get_page_width(&self, page: i32) -> f64 {
        self.page(page).map_or(0.0, |p| p.width())
    }
    /// The height of a 1-based page (0 if out of range).
    pub fn get_page_height(&self, page: i32) -> f64 {
        self.page(page).map_or(0.0, |p| p.height())
    }
    /// The rotation of a 1-based page, in degrees (0 if out of range).
    pub fn get_page_rotation(&self, page: i32) -> f64 {
        self.page(page).map_or(0.0, |p| p.rotation() as f64)
    }
    /// The x offset of a 1-based page (always 0 in the shipped subset).
    pub fn get_page_x_offset(&self, _page: i32) -> f64 {
        0.0
    }
    /// The y offset of a 1-based page (always 0 in the shipped subset).
    pub fn get_page_y_offset(&self, _page: i32) -> f64 {
        0.0
    }

    fn page(&self, page: i32) -> Option<crate::public::page::Page> {
        if page < 1 {
            return None;
        }
        self.base.document.as_ref().and_then(|d| d.pages().get(page as usize).ok())
    }

    /// The PDF version string (e.g. `"1.4"`), empty when unbound.
    pub fn get_pdf_version(&self) -> String {
        let Some(doc) = self.base.document.as_ref() else {
            return String::new();
        };
        let head = &doc.internal_bytes()[..doc.internal_bytes().len().min(16)];
        let s = String::from_utf8_lossy(head);
        s.strip_prefix("%PDF-")
            .map(|rest| rest.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect())
            .unwrap_or_default()
    }

    /// Save the bound document (with any `/Info` edits) to a file.
    pub fn save_new_info(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        match self.base.document.as_ref() {
            Some(d) => d.save_to(path),
            None => Err(Error::InvalidPdf("no document bound".into())),
        }
    }

    /// Save the bound document with XMP metadata (delegates to the
    /// standard save in the shipped subset).
    pub fn save_new_info_with_xmp(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        self.save_new_info(path)
    }
}

impl_facade!(PdfFileInfo);
impl_saveable_facade!(PdfFileInfo);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_unbound() {
        let info = PdfFileInfo::new();
        assert!(!info.is_pdf_file());
        assert!(!info.is_encrypted());
        assert_eq!(info.number_of_pages(), 0);
        assert_eq!(info.password_type(), PasswordType::None);
        assert!(!info.has_open_password());
        assert!(!info.has_edit_password());
        assert!(!info.has_collection());
        assert!(!info.use_strict_validation());
    }

    #[test]
    fn info_roundtrip_on_bound_document() {
        let mut info = PdfFileInfo::with_document(Document::new());
        assert!(info.is_pdf_file());
        info.set_title("Hello PDF");
        info.set_author("Jane Doe");
        info.set_meta_info("Trapped", "False");
        assert_eq!(info.title(), "Hello PDF");
        assert_eq!(info.author(), "Jane Doe");
        assert_eq!(info.get_meta_info("Trapped"), "False");
        assert!(info.number_of_pages() >= 0);
    }
}
