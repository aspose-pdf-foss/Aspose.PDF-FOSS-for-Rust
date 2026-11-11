//! `Aspose.Pdf.FileSpecification` — a file-specification carrier
//! describing an embedded / associated file, per
//! `capabilities/file_specification.yaml`.
//!
//! Ships the C++ reference's data-carrier surface: the default and
//! file-path constructors, the case-insensitive `get_value` /
//! `set_value` collection-item accessors, and the `encoding` /
//! `include_contents` / `description` / `af_relationship` /
//! `mime_type` / `name` / `unicode_name` / `file_system`
//! properties, plus the `encrypted_payload` (`/EP`) descriptor. The
//! `System.IO.Stream` constructors, the `Annotation`-taking constructor,
//! and the `CollectionItem` / `StreamContents` / `Contents` properties
//! (each depending on types outside the shipped subset) are dropped (see
//! `targets/rust/drops.yaml`).

use crate::public::af_relationship::AFRelationship;
use crate::public::encrypted_payload::EncryptedPayload;
use crate::public::file_encoding::FileEncoding;
use std::collections::HashMap;

/// A file specification describing an embedded / associated file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSpecification {
    file: String,
    description: String,
    mime_type: String,
    name: String,
    unicode_name: String,
    file_system: String,
    encoding: FileEncoding,
    af_relationship: AFRelationship,
    include_contents: bool,
    // Case-insensitive collection-item store: keyed by the
    // lower-cased key, holding the original-cased value.
    values: HashMap<String, String>,
    // `/Params` metadata (size / dates / checksum), populated on load.
    params: crate::public::file_params::FileParams,
    // `/EP` encrypted-payload descriptor, populated on load. `None` when
    // the specification is not an encrypted-payload wrapper.
    encrypted_payload: Option<EncryptedPayload>,
}

impl Default for FileSpecification {
    fn default() -> Self {
        Self {
            file: String::new(),
            description: String::new(),
            mime_type: String::new(),
            name: String::new(),
            unicode_name: String::new(),
            file_system: String::new(),
            encoding: FileEncoding::None,
            af_relationship: AFRelationship::Unspecified,
            include_contents: true,
            values: HashMap::new(),
            params: crate::public::file_params::FileParams::default(),
            encrypted_payload: None,
        }
    }
}

impl FileSpecification {
    /// Construct an empty file specification.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct from a source file path. The `/F` name defaults to
    /// the same path.
    pub fn with_file(file: String) -> Self {
        Self {
            name: file.clone(),
            file,
            ..Self::default()
        }
    }

    /// The source file path this specification was constructed
    /// with. Crate-internal — the canonical surface exposes the
    /// path only indirectly (via the embedded-file collection key
    /// and the `/F` name); the embedding pipeline reads it here.
    pub(crate) fn file(&self) -> &str {
        &self.file
    }

    /// The embedded file's `/Params` metadata (size / dates / checksum).
    pub fn params(&self) -> &crate::public::file_params::FileParams {
        &self.params
    }

    /// Set the `/Params` metadata. Crate-internal — populated when the
    /// embedded file is loaded from the document.
    pub(crate) fn set_params(&mut self, params: crate::public::file_params::FileParams) {
        self.params = params;
    }

    /// The `/EP` encrypted-payload descriptor, or `None` when the
    /// specification is not an encrypted-payload wrapper.
    pub fn encrypted_payload(&self) -> Option<&EncryptedPayload> {
        self.encrypted_payload.as_ref()
    }

    /// Set the `/EP` descriptor. Crate-internal — populated when the
    /// embedded file is loaded from the document.
    pub(crate) fn set_encrypted_payload(&mut self, value: EncryptedPayload) {
        self.encrypted_payload = Some(value);
    }

    /// Construct from a source file path and a description. The
    /// `/F` name defaults to the path.
    pub fn with_file_and_description(file: String, description: String) -> Self {
        Self {
            name: file.clone(),
            file,
            description,
            ..Self::default()
        }
    }

    /// Read a collection-item value by (case-insensitive) key.
    /// Returns an empty string when the key is absent.
    pub fn get_value(&self, key: &str) -> String {
        self.values
            .get(&key.to_lowercase())
            .cloned()
            .unwrap_or_default()
    }

    /// Set a collection-item value by (case-insensitive) key.
    pub fn set_value(&mut self, key: &str, value: String) {
        self.values.insert(key.to_lowercase(), value);
    }

    /// Release the specification. The carrier owns only in-memory
    /// data; Rust frees it via `Drop`, so this is a no-op provided
    /// for canonical `IDisposable` parity.
    pub fn dispose(&self) {}

    /// The payload encoding.
    pub fn encoding(&self) -> FileEncoding {
        self.encoding
    }
    /// Set the payload encoding.
    pub fn set_encoding(&mut self, value: FileEncoding) {
        self.encoding = value;
    }

    /// Whether the file's contents are embedded in the document.
    pub fn include_contents(&self) -> bool {
        self.include_contents
    }
    /// Set whether the file's contents are embedded.
    pub fn set_include_contents(&mut self, value: bool) {
        self.include_contents = value;
    }

    /// The human-readable description.
    pub fn description(&self) -> &str {
        &self.description
    }
    /// Set the human-readable description.
    pub fn set_description(&mut self, value: String) {
        self.description = value;
    }

    /// The associated-file relationship.
    pub fn af_relationship(&self) -> AFRelationship {
        self.af_relationship
    }
    /// Set the associated-file relationship.
    pub fn set_af_relationship(&mut self, value: AFRelationship) {
        self.af_relationship = value;
    }

    /// The MIME type of the file.
    pub fn mime_type(&self) -> &str {
        &self.mime_type
    }
    /// Set the MIME type of the file.
    pub fn set_mime_type(&mut self, value: String) {
        self.mime_type = value;
    }

    /// The file's name (`/F` entry).
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Set the file's name.
    pub fn set_name(&mut self, value: String) {
        self.name = value;
    }

    /// The file's Unicode name (`/UF` entry).
    pub fn unicode_name(&self) -> &str {
        &self.unicode_name
    }
    /// Set the file's Unicode name.
    pub fn set_unicode_name(&mut self, value: String) {
        self.unicode_name = value;
    }

    /// The file system handler name (`/FS` entry).
    pub fn file_system(&self) -> &str {
        &self.file_system
    }
    /// Set the file system handler name.
    pub fn set_file_system(&mut self, value: String) {
        self.file_system = value;
    }
}
