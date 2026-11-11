//! `Aspose.Pdf.Document` — PDF document loaded into memory.
//!
//! v1 surface:
//!
//! * [`Document::new`] — default-construct an empty PDF
//!   document (single-catalog, zero-page).
//! * [`Document::open`] / [`Document::from_bytes`] — parse a
//!   PDF from a file path or an in-memory byte buffer.
//! * [`Document::pages`] — borrowed [`PageCollection`].
//! * [`Document::info`] / [`Document::info_mut`] — typed
//!   `/Info` accessor (read-write in memory).
//! * [`Document::set_title`] — set `/Info /Title` and mark the
//!   document for incremental update on the next save.
//! * [`Document::save`] / [`Document::save_to`] /
//!   [`Document::save_to_writer`] — verbatim round-trip when
//!   clean; incremental update tail (§7.5.6) when dirty.
//! * [`Document::send_to`] / [`Document::send_to_range`] /
//!   [`Document::send_to_path`] /
//!   [`Document::send_to_range_to_path`] — route the document
//!   (or a 1-based page range) through a
//!   [`DocumentDevice`] into a
//!   [`std::io::Write`] sink or a file path; the concrete
//!   device determines the output format
//!   ([`TiffDevice`](crate::TiffDevice) for multi-page TIFF).
//! * [`Document::convert_page_to_png`] — convenience that
//!   renders a single [`Page`] to a PNG byte buffer through a
//!   default [`PngDevice`].
//!
//! Stream-based ctors (canonical `Document(Stream)`),
//! encryption, and format conversion
//! (`SaveFormat` / `SaveOptions`) are out of v1 scope and land
//! in later beats.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::internal::{
    agl,
    encoding::Encoding,
    encoding_tables::{self, Encoding as PdfEnc},
    flate,
    objects::{self, IndirectObject, Value},
    pages_tree, pdf_writer_incremental, trailer,
    xref::{self, Entry},
};

use super::device::{DocumentDevice, PageDevice};
use super::document_info::DocumentInfo;
use super::embedded_file_collection::EmbeddedFileCollection;
use super::error::Error;
use super::form::Form;
use super::metadata::Metadata;
use super::named_destination_collection::NamedDestinationCollection;
use super::outline_collection::OutlineCollection;
use super::page_label_collection::PageLabelCollection;
use super::page::Page;
use super::page_collection::PageCollection;
use super::png_device::PngDevice;

/// PDF document parsed from a file path or an in-memory byte
/// buffer.
///
/// `Document` retains the source bytes plus a small in-memory
/// model (page geometry + `/Info` accessor). It is `!Sync` /
/// `Send` per Rust's default ownership rules — the byte buffer
/// is owned, page references are borrowed off `&Document`.
#[derive(Debug)]
pub struct Document {
    bytes: Arc<Vec<u8>>,
    source_path: Option<PathBuf>,
    pages: PageCollection,
    info: DocumentInfo,
    metadata: Metadata,
    /// Indirect-object id of the source `/Info` dict. `0` when
    /// the source carries no `/Info` (trailer has no `/Info`
    /// key, or the key resolves to a null / missing entry).
    info_object_id: u32,
    /// Generation of the `/Info` indirect. Required to round-
    /// trip the dirty value through `pdf_writer_incremental`.
    info_object_gen: u16,
    /// Set by [`Document::set_title`]. Toggles the next
    /// [`Document::save`] from verbatim round-trip to an
    /// incremental update tail.
    dirty: bool,
    /// Outline (bookmark) tree, exposed via [`Document::outlines`].
    /// Parsed from the catalog's `/Outlines` on load, written back as
    /// an incremental update when mutated.
    outlines: OutlineCollection,
    /// Pending page content edits (shared with this document's
    /// pages). A [`TextBuilder`](crate::TextBuilder) pushes here;
    /// flushed as an incremental update on save.
    edits: crate::public::page_edits::EditBuffer,
    /// Named navigation targets — parsed from the catalog's
    /// `/Names /Dests` on load, written back as an incremental update
    /// when mutated. Exposed via [`Document::named_destinations`].
    named_destinations: NamedDestinationCollection,
    /// Page-label ranges — parsed from the catalog's `/PageLabels` on
    /// load, written back when mutated. Exposed via
    /// [`Document::page_labels`].
    page_labels: PageLabelCollection,
    /// Embedded files — parsed from the catalog's `/Names
    /// /EmbeddedFiles` name tree on load, written back (content bytes
    /// re-embedded) when mutated. Exposed via
    /// [`Document::embedded_files`].
    embedded_files: EmbeddedFileCollection,
    /// Interactive form — fields parsed from the catalog's `/AcroForm
    /// /Fields` on load, written back (as merged field / widget
    /// objects) when mutated. Exposed via [`Document::form`].
    form: Form,
    /// Whether the source document carries a `/Encrypt` dictionary.
    is_encrypted: bool,
    /// The document permission flags (`/Encrypt /P`), or all-allowed
    /// (`-1`) for a plain document.
    permissions: i32,
    /// Staged encryption captured by [`Document::encrypt`]; applied on
    /// the next save.
    staged_encryption: Option<StagedEncryption>,
    /// Catalog viewer preferences (`/PageLayout`, `/PageMode`,
    /// `/ViewerPreferences /Direction` + `/NonFullScreenPageMode`),
    /// parsed on load and written back on save when mutated.
    direction: crate::public::direction::Direction,
    page_mode: crate::public::page_mode::PageMode,
    non_full_screen_page_mode: crate::public::page_mode::PageMode,
    page_layout: crate::public::page_layout::PageLayout,
    duplex: crate::public::print_duplex::PrintDuplex,
    print_scaling: crate::public::print_scaling::PrintScaling,
    /// Set by any viewer-preference setter; triggers the catalog
    /// write-back on the next save.
    viewer_prefs_dirty: bool,
}

/// Encryption parameters staged by [`Document::encrypt`] until the next
/// save.
#[derive(Debug, Clone)]
struct StagedEncryption {
    user_password: String,
    owner_password: String,
    permissions: i32,
    algorithm: crate::public::crypto_algorithm::CryptoAlgorithm,
    encrypt_metadata: bool,
}

impl StagedEncryption {
    fn to_params(&self) -> crate::internal::document_encrypt::Params {
        use crate::internal::crypt_filter::Method;
        use crate::public::crypto_algorithm::CryptoAlgorithm;
        // `(v, r, length_bits, method)` per the chosen cipher.
        let variant = match self.algorithm {
            CryptoAlgorithm::Rc4x40 => (1, 2, 40, Method::V2),
            CryptoAlgorithm::Rc4x128 => (2, 3, 128, Method::V2),
            CryptoAlgorithm::Aesx128 => (4, 4, 128, Method::AesV2),
            CryptoAlgorithm::Aesx256 => (5, 6, 256, Method::AesV3),
        };
        crate::internal::document_encrypt::Params {
            user_password: self.user_password.clone(),
            owner_password: self.owner_password.clone(),
            permissions: self.permissions,
            variant,
            encrypt_metadata: self.encrypt_metadata,
        }
    }
}

/// All permission bits set — the value [`Document::permissions`]
/// reports for a plain (unencrypted) document.
const ALL_PERMISSIONS: i32 = -1;

/// A named destination loaded from a PDF's `/Names /Dests` tree. Holds
/// the reconstructed canonical destination string (e.g.
/// `"1 XYZ 100 700 2"`); its [`Display`] is that string, so it satisfies
/// [`IAppointment`](crate::IAppointment).
#[derive(Debug)]
struct LoadedDestination(String);

impl std::fmt::Display for LoadedDestination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl crate::public::i_appointment::IAppointment for LoadedDestination {}

/// Rebuild the public outline tree from parsed `/Outlines` data.
fn build_loaded_outlines(
    root: &OutlineCollection,
    items: &[crate::internal::outline_io::OutlineData],
) {
    for data in items {
        root.add(&loaded_outline_item(root, data));
    }
}

/// Materialise one loaded bookmark (recursing its children).
fn loaded_outline_item(
    root: &OutlineCollection,
    data: &crate::internal::outline_io::OutlineData,
) -> crate::public::outline_item_collection::OutlineItemCollection {
    let item = crate::public::outline_item_collection::OutlineItemCollection::new(root);
    item.set_title(data.title.clone());
    item.set_bold(data.bold);
    item.set_italic(data.italic);
    item.set_open(data.open);
    if let Some(dest) = &data.dest {
        item.set_destination(Box::new(LoadedDestination(dest.clone())));
    }
    if let Some(action) = &data.carried_action {
        item.set_carried_action(action.clone());
    }
    for child in &data.children {
        item.add(&loaded_outline_item(root, child));
    }
    item
}

/// Snapshot the public outline tree into the serialisable form the
/// `/Outlines` save-through writes.
fn outlines_for_save(
    root: &OutlineCollection,
) -> Vec<crate::internal::outline_io::OutlineData> {
    (1..=root.count() as i32).map(|i| snapshot_outline_item(&root.get(i))).collect()
}

fn snapshot_outline_item(
    item: &crate::public::outline_item_collection::OutlineItemCollection,
) -> crate::internal::outline_io::OutlineData {
    crate::internal::outline_io::OutlineData {
        title: item.title(),
        bold: item.bold(),
        italic: item.italic(),
        open: item.open(),
        dest: item.destination().map(|d| d.to_string()),
        action: item
            .action()
            .and_then(|a| crate::public::pdf_action::action_entry(a.as_ref())),
        carried_action: item.carried_action(),
        children: (1..=item.count() as i32)
            .map(|i| snapshot_outline_item(&item.get(i)))
            .collect(),
    }
}

/// A public rectangle as the four numbers the geometry writer takes.
/// Every dictionary entry a field contributes, in precedence order:
/// the widget's own (§12.5.6.19), §12.5.4's border — a field IS an
/// annotation, so the border is read off the same place the annotation
/// writer reads it — the field type's own, and last the quadding the
/// base carries.
///
/// Keys are de-duplicated keeping the FIRST, because a type-specific
/// entry is the more specific answer: a rich text box states its own
/// `/Q`, which must win over the alignment on the base. Two entries
/// with one key would be an invalid dictionary either way.
fn field_entries(
    field: &dyn crate::public::field::Field,
) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
    use crate::internal::annotation_io::Entry;
    use crate::public::horizontal_alignment::HorizontalAlignment;

    let quadding = match field.text_horizontal_alignment() {
        HorizontalAlignment::Left => Some(0),
        HorizontalAlignment::Center => Some(1),
        HorizontalAlignment::Right => Some(2),
        // §12.7.4.1 gives /Q three values; None means the field says
        // nothing, and justified has no /Q to be written as.
        _ => None,
    };
    let mut out: Vec<(&'static str, Entry)> =
        crate::public::widget_annotation::widget_entries(field.widget())
            .into_iter()
            .chain(crate::public::annotation_collection::border_entries(field.border()))
            .chain(field.pdf_field_entries())
            .chain(quadding.map(|q| ("Q", Entry::Int(q))))
            .collect();
    let mut seen: Vec<&'static str> = Vec::with_capacity(out.len());
    out.retain(|(key, _)| {
        if seen.contains(key) {
            false
        } else {
            seen.push(key);
            true
        }
    });
    out
}

fn rect_tuple(r: crate::public::rectangle::Rectangle) -> [f64; 4] {
    [r.llx(), r.lly(), r.urx(), r.ury()]
}

impl Document {
    /// Default-construct an empty PDF document — a `/Pages`
    /// tree with `/Count 0` and `/Kids []`. The synthesised
    /// source bytes are a minimal valid PDF (header, catalog,
    /// pages root, xref, and trailer); [`Document::save_to`]
    /// writes them to disk verbatim. [`Document::save`]
    /// (no-arg) returns [`Error::NoSourceFilename`] because no
    /// source path is associated.
    ///
    /// Same shape every peer track ships
    /// (C# `new Document()`, C++ `Document()`,
    /// TS `new Document()`); the synthesised byte sequence is
    /// not byte-identical across libs (offsets recompute) but
    /// semantically identical — every produced buffer parses
    /// back to a 0-page document via [`Document::from_bytes`].
    pub fn new() -> Self {
        // Build-and-parse cannot fail — the synthesised bytes
        // are a hand-rolled minimal PDF that the foundation
        // primitives accept by construction. The expect keeps
        // the public API panic-free in caller code paths.
        Self::build(build_empty_pdf(), None)
            .expect("synthesised empty PDF must parse")
    }

    /// Open the PDF stored at `path`. The file header,
    /// cross-reference table, trailer, and page tree are
    /// parsed at construction; subsequent operations work
    /// against the in-memory model. The source filename is
    /// retained so [`Document::save`] (no-arg) can write back
    /// to the same location.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let bytes = fs::read(path)?;
        Self::build(bytes, Some(path.to_path_buf()))
    }

    /// Open the PDF from an in-memory byte buffer. No source
    /// filename is retained; the document can only be saved
    /// via [`Document::save_to`] or
    /// [`Document::save_to_writer`].
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        Self::build(bytes, None)
    }

    /// Import an SVG file as a single-page PDF document sized to the
    /// SVG, rendering its basic vector primitives (rect / circle /
    /// line). Mirrors the canonical `Document(fileName, LoadOptions)`
    /// constructor for an [`SvgLoadOptions`](crate::SvgLoadOptions).
    pub fn open_svg(
        path: impl AsRef<Path>,
        _options: &crate::public::svg_load_options::SvgLoadOptions,
    ) -> Result<Self, Error> {
        let svg = fs::read(path.as_ref())?;
        let pdf = crate::internal::svg_import::to_pdf(&svg);
        Self::from_bytes(pdf)
    }

    fn build(bytes: Vec<u8>, source_path: Option<PathBuf>) -> Result<Self, Error> {
        let bytes = Arc::new(bytes);
        let tree = pages_tree::parse(&bytes)?;
        let edits = crate::public::page_edits::new_edit_buffer();
        let pages = build_page_collection(&tree, &bytes, &edits);

        let mut info = DocumentInfo::new();
        let (info_object_id, info_object_gen) = load_info(&bytes, &mut info)?;

        let metadata = load_metadata(&bytes)?;

        let is_encrypted = crate::internal::document_encrypt::is_encrypted(&bytes);
        let permissions = crate::internal::document_encrypt::permissions(&bytes)
            .unwrap_or(ALL_PERMISSIONS);

        let mut named_destinations = NamedDestinationCollection::new();
        for (name, dest) in crate::internal::named_dest_io::parse(&bytes) {
            named_destinations.add_loaded(name, Box::new(LoadedDestination(dest)));
        }

        let outlines = OutlineCollection::new_root();
        build_loaded_outlines(&outlines, &crate::internal::outline_io::parse(&bytes));
        outlines.clear_dirty();

        let mut page_labels = PageLabelCollection::new();
        for l in crate::internal::page_label_io::parse(&bytes) {
            let mut label = crate::public::page_label::PageLabel::new();
            label.set_numbering_style(
                crate::public::numbering_style::NumberingStyle::from_pdf_style_char(
                    l.style.as_deref(),
                ),
            );
            label.set_starting_value(l.starting);
            label.set_prefix(l.prefix);
            page_labels.update_loaded(l.index, label);
        }

        let mut embedded_files = EmbeddedFileCollection::new();
        for e in crate::internal::embedded_file_io::parse(&bytes) {
            let mut spec = crate::public::file_specification::FileSpecification::new();
            spec.set_name(e.name);
            spec.set_unicode_name(e.unicode_name);
            spec.set_description(e.description);
            spec.set_mime_type(e.mime_type);
            spec.set_af_relationship(
                crate::public::af_relationship::AFRelationship::from_pdf_name(&e.af_relationship),
            );
            spec.set_file_system(e.file_system);
            use crate::public::date_time::DateTime;
            spec.set_params(crate::public::file_params::FileParams::from_parts(
                if e.size < 0 { 0 } else { e.size as i32 },
                DateTime::from_pdf_date(&e.creation_date),
                DateTime::from_pdf_date(&e.mod_date),
                e.check_sum,
            ));
            if let Some(ep) = e.encrypted_payload {
                spec.set_encrypted_payload(
                    crate::public::encrypted_payload::EncryptedPayload::from_parts(
                        ep.type_, ep.subtype, ep.version,
                    ),
                );
            }
            embedded_files.add_loaded(e.key, spec);
        }

        let mut form = Form::default();
        for f in crate::internal::acroform_io::parse(&bytes) {
            let rect = crate::public::rectangle::Rectangle::new(
                f.rect[0], f.rect[1], f.rect[2], f.rect[3], false,
            );
            form.add_loaded(Box::new(crate::public::field::LoadedField::from_parsed(f, rect)));
        }

        let vp = crate::internal::viewer_prefs_io::parse(&bytes);

        Ok(Self {
            bytes,
            source_path,
            pages,
            info,
            metadata,
            info_object_id,
            info_object_gen,
            dirty: false,
            outlines,
            edits,
            named_destinations,
            page_labels,
            embedded_files,
            form,
            is_encrypted,
            permissions,
            staged_encryption: None,
            direction: vp.direction.unwrap_or_default(),
            page_mode: vp.page_mode.unwrap_or_default(),
            non_full_screen_page_mode: vp.non_full_screen_page_mode.unwrap_or_default(),
            page_layout: vp.page_layout.unwrap_or_default(),
            duplex: vp.duplex.unwrap_or_default(),
            print_scaling: vp.print_scaling.unwrap_or_default(),
            viewer_prefs_dirty: false,
        })
    }

    /// Open the encrypted PDF at `path`, authenticating `password`
    /// (user or owner). Returns [`Error::InvalidPdf`] when the
    /// password is incorrect. A plain (unencrypted) document opens
    /// regardless of the password. Mirrors the canonical
    /// `Document(fileName, password)` constructor.
    pub fn open_with_password(
        path: impl AsRef<Path>,
        password: &str,
    ) -> Result<Self, Error> {
        let path = path.as_ref();
        let bytes = fs::read(path)?;
        crate::internal::document_encrypt::authenticate(&bytes, password.as_bytes())
            .map_err(|_| Error::InvalidPdf("incorrect document password".into()))?;
        Self::build(bytes, Some(path.to_path_buf()))
    }

    /// Whether the document is encrypted (or staged for encryption).
    pub fn is_encrypted(&self) -> bool {
        self.is_encrypted || self.staged_encryption.is_some()
    }

    /// The document access-permission flags. Reflects the staged
    /// permissions after [`Document::encrypt`], the source `/Encrypt /P`
    /// for an encrypted document, or all-allowed (`-1`) for a plain
    /// document. Combine with [`Permissions`](crate::Permissions) bits
    /// to test a capability.
    pub fn permissions(&self) -> i32 {
        match &self.staged_encryption {
            Some(s) => s.permissions,
            None => self.permissions,
        }
    }

    /// Stage encryption with `user_password` / `owner_password`, the
    /// access `permissions`, and the cipher `algorithm`. The encryption
    /// is applied on the next [`Document::save_to`] /
    /// [`Document::save_to_writer`]. Mirrors canonical
    /// `Document.Encrypt(userPassword, ownerPassword, Permissions,
    /// CryptoAlgorithm)`.
    pub fn encrypt(
        &mut self,
        user_password: impl Into<String>,
        owner_password: impl Into<String>,
        permissions: crate::public::permissions::Permissions,
        algorithm: crate::public::crypto_algorithm::CryptoAlgorithm,
    ) {
        self.encrypt_with_metadata(user_password, owner_password, permissions, algorithm, true);
    }

    /// Stage encryption, explicitly choosing whether the document
    /// metadata is encrypted. Mirrors the canonical
    /// `Document.Encrypt(..., bool encryptMetadata)` overload.
    pub fn encrypt_with_metadata(
        &mut self,
        user_password: impl Into<String>,
        owner_password: impl Into<String>,
        permissions: crate::public::permissions::Permissions,
        algorithm: crate::public::crypto_algorithm::CryptoAlgorithm,
        encrypt_metadata: bool,
    ) {
        self.staged_encryption = Some(StagedEncryption {
            user_password: user_password.into(),
            owner_password: owner_password.into(),
            permissions: permissions.bits(),
            algorithm,
            encrypt_metadata,
        });
    }

    /// Stage encryption from a [`DocumentPrivilege`](crate::DocumentPrivilege)
    /// permission set. Mirrors canonical `Document.Encrypt(userPassword,
    /// ownerPassword, DocumentPrivilege, CryptoAlgorithm, bool)`.
    pub fn encrypt_with_privilege(
        &mut self,
        user_password: impl Into<String>,
        owner_password: impl Into<String>,
        privilege: &crate::public::document_privilege::DocumentPrivilege,
        algorithm: crate::public::crypto_algorithm::CryptoAlgorithm,
        encrypt_metadata: bool,
    ) {
        use crate::public::permissions::Permissions;
        let mut p = Permissions::NONE;
        if privilege.allow_print() {
            p = p | Permissions::PRINT_DOCUMENT;
        }
        if privilege.allow_modify_contents() {
            p = p | Permissions::MODIFY_CONTENT;
        }
        if privilege.allow_copy() {
            p = p | Permissions::EXTRACT_CONTENT;
        }
        if privilege.allow_modify_annotations() {
            p = p | Permissions::MODIFY_TEXT_ANNOTATIONS;
        }
        if privilege.allow_fill_in() {
            p = p | Permissions::FILL_FORM;
        }
        if privilege.allow_screen_readers() {
            p = p | Permissions::EXTRACT_CONTENT_WITH_DISABILITIES;
        }
        if privilege.allow_assembly() {
            p = p | Permissions::ASSEMBLE_DOCUMENT;
        }
        if privilege.allow_degraded_printing() {
            p = p | Permissions::PRINTING_QUALITY;
        }
        self.encrypt_with_metadata(user_password, owner_password, p, algorithm, encrypt_metadata);
    }

    /// Remove staged encryption (the next save writes the document
    /// unencrypted). Mirrors canonical `Document.Decrypt()`.
    pub fn decrypt(&mut self) {
        self.staged_encryption = None;
        self.is_encrypted = false;
        self.permissions = ALL_PERMISSIONS;
    }

    /// Re-stage encryption with new passwords, preserving the current
    /// permissions and cipher. Mirrors canonical
    /// `Document.ChangePasswords(ownerPassword, newUserPassword,
    /// newOwnerPassword)` — the first argument authenticates and is
    /// accepted but not retained.
    pub fn change_passwords(
        &mut self,
        _owner_password: &str,
        new_user_password: impl Into<String>,
        new_owner_password: impl Into<String>,
    ) {
        if let Some(s) = self.staged_encryption.as_mut() {
            s.user_password = new_user_password.into();
            s.owner_password = new_owner_password.into();
        }
    }

    /// Borrowed [`PageCollection`] over this document's pages.
    pub fn pages(&self) -> &PageCollection {
        &self.pages
    }

    /// The document's named navigation targets (read-only). Mirrors
    /// the canonical `Document.NamedDestinations` property; use
    /// [`named_destinations_mut`](Self::named_destinations_mut) to add
    /// or remove. Mutations are written to the `/Names /Dests` name
    /// tree on the next save.
    pub fn named_destinations(&self) -> &NamedDestinationCollection {
        &self.named_destinations
    }

    /// Mutable [`NamedDestinationCollection`] — adding or removing here
    /// is persisted to the catalog's `/Names /Dests` on save.
    pub fn named_destinations_mut(&mut self) -> &mut NamedDestinationCollection {
        &mut self.named_destinations
    }

    /// The document's page-label ranges (read-only). Mirrors the
    /// canonical `Document.PageLabels` property; use
    /// [`page_labels_mut`](Self::page_labels_mut) to update. Changes are
    /// written to the catalog's `/PageLabels` on the next save.
    pub fn page_labels(&self) -> &PageLabelCollection {
        &self.page_labels
    }

    /// Mutable [`PageLabelCollection`] — updates here are persisted to
    /// the catalog's `/PageLabels` number tree on save.
    pub fn page_labels_mut(&mut self) -> &mut PageLabelCollection {
        &mut self.page_labels
    }

    /// The document's embedded files — read-only view onto the
    /// `/Names /EmbeddedFiles` name tree. Mirrors the canonical
    /// `Document.EmbeddedFiles` property; use
    /// [`embedded_files_mut`](Self::embedded_files_mut) to attach or
    /// remove files.
    pub fn embedded_files(&self) -> &EmbeddedFileCollection {
        &self.embedded_files
    }

    /// Mutable [`EmbeddedFileCollection`] — adding or removing here is
    /// persisted (content bytes re-embedded) to the catalog's `/Names
    /// /EmbeddedFiles` name tree on save.
    pub fn embedded_files_mut(&mut self) -> &mut EmbeddedFileCollection {
        &mut self.embedded_files
    }

    /// The document's interactive form — read-only view onto the
    /// `/AcroForm` fields. Mirrors the canonical `Document.Form`
    /// property; use [`form_mut`](Self::form_mut) to add or remove
    /// fields.
    pub fn form(&self) -> &Form {
        &self.form
    }

    /// Mutable [`Form`] — adding or removing fields here is persisted
    /// to the catalog's `/AcroForm /Fields` (and each widget to its
    /// page's `/Annots`) on save.
    pub fn form_mut(&mut self) -> &mut Form {
        &mut self.form
    }

    /// The document's outline (bookmark) tree.
    ///
    /// The returned [`OutlineCollection`] is a live handle — items
    /// added through it mutate this document's tree (the collection
    /// uses interior mutability, so a shared `&` borrow suffices).
    pub fn outlines(&self) -> &OutlineCollection {
        &self.outlines
    }

    /// Borrowed [`DocumentInfo`] — read-only view onto the
    /// `/Info` dictionary.
    pub fn info(&self) -> &DocumentInfo {
        &self.info
    }

    /// Mutable [`DocumentInfo`]. Mutations through this borrow
    /// update the in-memory view but do NOT mark the document
    /// for incremental save — use [`Document::set_title`] (or
    /// the future typed `set_<key>` shortcuts) for persisted
    /// changes.
    pub fn info_mut(&mut self) -> &mut DocumentInfo {
        &mut self.info
    }

    /// Borrowed [`Metadata`] — read-only view onto the XMP
    /// `/Metadata` packet. An empty [`Metadata`] is returned
    /// when the source PDF carries no `/Metadata` in its
    /// catalog.
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Mutable [`Metadata`]. Mutations through this borrow
    /// update the in-memory view but do NOT mark the document
    /// for incremental save — persisted XMP updates are out of
    /// v1 scope (the canonical surface persists XMP through
    /// `Document.Save`, but the incremental-write path for
    /// streams lands in a later beat).
    pub fn metadata_mut(&mut self) -> &mut Metadata {
        &mut self.metadata
    }

    /// Borrowed slice over the document's source bytes — the
    /// buffer foundation primitives (`text_extractor`,
    /// `page_renderer`, …) consume when a public-API type needs
    /// to re-parse the original PDF. Mirrors
    /// `Document.InternalBytes` on the C# / TypeScript peer
    /// tracks; not part of the canonical .NET surface.
    pub(crate) fn internal_bytes(&self) -> &[u8] {
        self.bytes.as_ref()
    }

    /// Set `/Info /Title` and mark the document for incremental
    /// update on the next [`Document::save`] /
    /// [`Document::save_to`] / [`Document::save_to_writer`]
    /// call.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.info.set_title(title);
        self.dirty = true;
    }

    /// The predominant reading order (`/ViewerPreferences /Direction`).
    pub fn direction(&self) -> crate::public::direction::Direction {
        self.direction
    }
    /// Set the predominant reading order; persisted on the next save.
    pub fn set_direction(&mut self, value: crate::public::direction::Direction) {
        self.direction = value;
        self.viewer_prefs_dirty = true;
    }

    /// How the viewer displays the document on open (`/PageMode`).
    pub fn page_mode(&self) -> crate::public::page_mode::PageMode {
        self.page_mode
    }
    /// Set the page mode; persisted on the next save.
    pub fn set_page_mode(&mut self, value: crate::public::page_mode::PageMode) {
        self.page_mode = value;
        self.viewer_prefs_dirty = true;
    }

    /// The page mode used when leaving full-screen
    /// (`/ViewerPreferences /NonFullScreenPageMode`).
    pub fn non_full_screen_page_mode(&self) -> crate::public::page_mode::PageMode {
        self.non_full_screen_page_mode
    }
    /// Set the non-full-screen page mode; persisted on the next save.
    pub fn set_non_full_screen_page_mode(
        &mut self,
        value: crate::public::page_mode::PageMode,
    ) {
        self.non_full_screen_page_mode = value;
        self.viewer_prefs_dirty = true;
    }

    /// The page layout used when the document is opened (`/PageLayout`).
    pub fn page_layout(&self) -> crate::public::page_layout::PageLayout {
        self.page_layout
    }
    /// Set the page layout; persisted on the next save.
    pub fn set_page_layout(&mut self, value: crate::public::page_layout::PageLayout) {
        self.page_layout = value;
        self.viewer_prefs_dirty = true;
    }

    /// The duplex printing mode (`/ViewerPreferences /Duplex`).
    pub fn duplex(&self) -> crate::public::print_duplex::PrintDuplex {
        self.duplex
    }
    /// Set the duplex printing mode; persisted on the next save.
    pub fn set_duplex(&mut self, value: crate::public::print_duplex::PrintDuplex) {
        self.duplex = value;
        self.viewer_prefs_dirty = true;
    }

    /// The print-scaling mode (`/ViewerPreferences /PrintScaling`).
    pub fn print_scaling(&self) -> crate::public::print_scaling::PrintScaling {
        self.print_scaling
    }
    /// Set the print-scaling mode; persisted on the next save.
    pub fn set_print_scaling(&mut self, value: crate::public::print_scaling::PrintScaling) {
        self.print_scaling = value;
        self.viewer_prefs_dirty = true;
    }

    /// The document's `/ID` file identifier (the trailer `/ID` array).
    /// Returns empty identifiers when the document carries no `/ID`.
    pub fn id(&self) -> crate::public::id::Id {
        let t = crate::internal::trailer::parse(&self.bytes).unwrap_or_default();
        crate::public::id::Id::from_bytes(t.id_original.as_deref(), t.id_modified.as_deref())
    }

    /// The default page-setup used when generating pages (canonical A4
    /// defaults). Mirrors the get-only `Document.PageInfo` property.
    pub fn page_info(&self) -> crate::public::page_info::PageInfo {
        crate::public::page_info::PageInfo::default()
    }

    /// The document's named explicit destinations (`/Names /Dests`).
    /// Mirrors the get-only `Document.Destinations` property.
    pub fn destinations(&self) -> crate::public::destination_collection::DestinationCollection {
        crate::public::destination_collection::DestinationCollection::from_bytes(&self.bytes)
    }

    /// Save the document back to the source filename tracked
    /// by [`Document::open`]. Returns
    /// [`Error::NoSourceFilename`] when the document was
    /// constructed via [`Document::from_bytes`].
    pub fn save(&self) -> Result<(), Error> {
        let path = self.source_path.as_ref().ok_or(Error::NoSourceFilename)?;
        let bytes = self.build_save_bytes()?;
        fs::write(path, bytes)?;
        Ok(())
    }

    /// Serialise the document to `path`. Overwrites any
    /// existing file at that location. Round-trips the input
    /// bytes verbatim when clean; emits an incremental update
    /// tail (PDF 32000-1:2008 §7.5.6) when dirty.
    pub fn save_to(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let bytes = self.build_save_bytes()?;
        fs::write(path.as_ref(), bytes)?;
        Ok(())
    }

    /// Stream variant of [`Document::save_to`]. Writes to any
    /// [`std::io::Write`] sink.
    pub fn save_to_writer(&self, mut w: impl Write) -> Result<(), Error> {
        let bytes = self.build_save_bytes()?;
        w.write_all(&bytes)?;
        Ok(())
    }

    /// Render every page of the document through `device` into
    /// `output`. The concrete device determines the output
    /// format ([`TiffDevice`](crate::TiffDevice) for multi-page
    /// TIFF, etc.). Mirrors canonical
    /// `Document.SendTo(DocumentDevice, Stream)` — a pure
    /// delegation to
    /// [`DocumentDevice::process_document`].
    pub fn send_to(
        &self,
        device: &dyn DocumentDevice,
        output: &mut dyn Write,
    ) -> Result<(), Error> {
        device.process_document(self, output)
    }

    /// Render the inclusive 1-based page range
    /// `[from_page, to_page]` through `device` into `output`.
    /// Mirrors canonical
    /// `Document.SendTo(DocumentDevice, int, int, Stream)` —
    /// delegates to
    /// [`DocumentDevice::process_document_range`].
    pub fn send_to_range(
        &self,
        device: &dyn DocumentDevice,
        from_page: usize,
        to_page: usize,
        output: &mut dyn Write,
    ) -> Result<(), Error> {
        device.process_document_range(self, from_page, to_page, output)
    }

    /// File-path variant of [`Document::send_to`] — opens
    /// `output_file_name` for writing (creating or truncating)
    /// and dispatches to
    /// [`DocumentDevice::process_document`].
    pub fn send_to_path(
        &self,
        device: &dyn DocumentDevice,
        output_file_name: impl AsRef<Path>,
    ) -> Result<(), Error> {
        let mut f = fs::File::create(output_file_name.as_ref())?;
        device.process_document(self, &mut f)
    }

    /// File-path variant of [`Document::send_to_range`].
    pub fn send_to_range_to_path(
        &self,
        device: &dyn DocumentDevice,
        from_page: usize,
        to_page: usize,
        output_file_name: impl AsRef<Path>,
    ) -> Result<(), Error> {
        let mut f = fs::File::create(output_file_name.as_ref())?;
        device.process_document_range(self, from_page, to_page, &mut f)
    }

    /// Convenience that renders `page` to a PNG byte buffer
    /// through a default [`PngDevice`].
    /// Equivalent to constructing a default `PngDevice` and
    /// dispatching [`PageDevice::process`]
    /// into an in-memory sink. Mirrors canonical
    /// `Document.ConvertPageToPNGMemoryStream(Page)` — the
    /// `&self` receiver matches the canonical method shape;
    /// the page carries its own document back-channel
    /// (`Arc<Vec<u8>>`) and is sufficient to drive the
    /// rasteriser without further help from the document.
    pub fn convert_page_to_png(&self, page: &Page) -> Result<Vec<u8>, Error> {
        let mut buf: Vec<u8> = Vec::new();
        PngDevice::new().process(page, &mut buf)?;
        Ok(buf)
    }

    fn build_save_bytes(&self) -> Result<Vec<u8>, Error> {
        let plain = self.build_plain_save_bytes()?;
        match &self.staged_encryption {
            Some(s) => {
                let params = s.to_params();
                crate::internal::document_encrypt::encrypt(&plain, &params)
                    .ok_or_else(|| Error::InvalidPdf("encryption failed".into()))
            }
            None => Ok(plain),
        }
    }

    fn build_plain_save_bytes(&self) -> Result<Vec<u8>, Error> {
        // Render any added page paragraphs (drawing) into the content
        // edit buffer so the existing content-edit flush picks them up.
        self.render_dirty_paragraphs();
        // Render any added page artifacts (watermarks): text artifacts
        // into the content-edit buffer, image artifacts into the page's
        // image resources. Runs before the dirty checks below so the
        // content / images passes pick the results up.
        self.render_dirty_artifacts();

        let metadata_dirty = self.metadata.is_dirty();
        let content_dirty = !self.edits.borrow().content.is_empty();
        let redactions_dirty = !self.edits.borrow().redactions.is_empty();
        let new_pages_dirty = !self.edits.borrow().new_pages.is_empty();
        let edits_dirty = content_dirty || redactions_dirty || new_pages_dirty;
        let named_dests_dirty = self.named_destinations.is_dirty();
        let page_labels_dirty = self.page_labels.is_dirty();
        let outlines_dirty = self.outlines.is_dirty();
        let embedded_files_dirty = self.embedded_files.is_dirty();
        let annots_dirty = self.pages.iter().any(|p| p.annotations.is_dirty());
        let images_dirty = self.pages.iter().any(|p| p.resources.images().is_dirty());
        let form_dirty = self.form.is_dirty();
        let geometry_dirty = self.pages.iter().any(|p| p.geometry.borrow().dirty);
        let pages_dirty = self.pages.is_dirty();
        if !self.dirty
            && !metadata_dirty
            && !edits_dirty
            && !named_dests_dirty
            && !page_labels_dirty
            && !outlines_dirty
            && !embedded_files_dirty
            && !annots_dirty
            && !images_dirty
            && !form_dirty
            && !geometry_dirty
            && !pages_dirty
            && !self.viewer_prefs_dirty
        {
            return Ok((*self.bytes).clone());
        }

        let mut working: Vec<u8> = (*self.bytes).clone();

        if self.dirty {
            if self.info_object_id == 0 {
                return Err(Error::NoInfoForUpdate);
            }
            let dict = build_info_dict(&self.info);
            let info_dirty = [pdf_writer_incremental::DirtyObject {
                id: self.info_object_id,
                generation: self.info_object_gen,
                value: dict,
            }];
            working = pdf_writer_incremental::append_incremental(
                &working, &info_dirty)?;
        }

        if pages_dirty {
            let specs: Vec<crate::internal::page_tree_io::PageSpec> = self
                .pages
                .snapshot()
                .iter()
                .map(|p| {
                    if p.leaf_id() == 0 {
                        // A synthesised blank page — emit a fresh leaf.
                        let mb = p.media_box();
                        crate::internal::page_tree_io::PageSpec::Blank {
                            media_box: [mb.llx(), mb.lly(), mb.urx(), mb.ury()],
                        }
                    } else {
                        // A loaded or copied page — reference its existing
                        // leaf object (content / resources reused).
                        crate::internal::page_tree_io::PageSpec::Reference {
                            leaf_id: p.leaf_id(),
                        }
                    }
                })
                .collect();
            if let Some((updated, emitted)) =
                crate::internal::page_tree_io::append(&working, &specs)
            {
                working = updated;
                // A page created in-session now exists in the file. Record
                // the id it was written under: the content, annotation,
                // image and geometry passes below address pages by that id.
                for (page, id) in self.pages.snapshot().iter().zip(emitted) {
                    if page.leaf_id() == 0 && id != 0 {
                        page.assign_leaf_id(id);
                    }
                }
            }
        }

        if metadata_dirty {
            working = self.append_metadata_update(working)?;
        }

        if self.viewer_prefs_dirty {
            working = self.append_viewer_prefs_update(working)?;
        }

        if redactions_dirty {
            working = self.apply_redactions(working)?;
        }

        if content_dirty {
            working = self.append_content_edits(working)?;
        }

        if new_pages_dirty {
            working = self.append_continuation_pages(working)?;
        }

        if named_dests_dirty {
            let dests = self.named_destinations.entries_for_save();
            if let Some(updated) = crate::internal::named_dest_io::append(&working, &dests) {
                working = updated;
            }
        }

        if page_labels_dirty {
            let labels: Vec<crate::internal::page_label_io::LabelData> = self
                .page_labels
                .entries_for_save()
                .into_iter()
                .map(|(index, style, starting, prefix)| crate::internal::page_label_io::LabelData {
                    index,
                    style: style.pdf_style_char().map(str::to_string),
                    starting,
                    prefix,
                })
                .collect();
            if let Some(updated) = crate::internal::page_label_io::append(&working, &labels) {
                working = updated;
            }
        }

        if outlines_dirty {
            let items = outlines_for_save(&self.outlines);
            if let Some(updated) = crate::internal::outline_io::append(&working, &items) {
                working = updated;
            }
        }

        if embedded_files_dirty {
            let specs: Vec<crate::internal::embedded_file_io::EmbeddedFileSpec> = self
                .embedded_files
                .entries_for_save()
                .into_iter()
                .map(|(key, f)| {
                    // A freshly attached spec carries a source path; read
                    // its bytes to embed. A loaded entry has no re-readable
                    // path — its original stream is reused by key.
                    let content = if f.include_contents() && !f.file().is_empty() {
                        fs::read(f.file()).ok()
                    } else {
                        None
                    };
                    crate::internal::embedded_file_io::EmbeddedFileSpec {
                        key,
                        name: f.name().to_string(),
                        unicode_name: f.unicode_name().to_string(),
                        description: f.description().to_string(),
                        mime_type: f.mime_type().to_string(),
                        af_relationship: f.af_relationship().pdf_name(),
                        file_system: f.file_system().to_string(),
                        content,
                    }
                })
                .collect();
            if let Some(updated) = crate::internal::embedded_file_io::append(&working, &specs) {
                working = updated;
            }
        }

        if annots_dirty {
            let pages: Vec<(u32, Vec<crate::internal::annotation_io::AnnotData>)> = self
                .pages
                .iter()
                .filter(|p| p.annotations.is_dirty())
                .map(|p| {
                    let annots = p
                        .annotations
                        .for_save()
                        .into_iter()
                        .map(|s| crate::internal::annotation_io::AnnotData {
                            subtype: s.annotation_type.pdf_subtype().to_string(),
                            rect: s.rect,
                            contents: s.contents,
                            name: s.name,
                            // /MN travels with the other entries on the
                            // way out; this field is what the reader
                            // fills on the way in.
                            marks_name: String::new(),
                            entries: s.entries,
                            carried: s.carried,
                        })
                        .collect();
                    (p.leaf_id(), annots)
                })
                .collect();
            if let Some(updated) = crate::internal::annotation_io::append(&working, &pages) {
                working = updated;
            }
        }

        if images_dirty {
            let pages: Vec<(u32, Vec<crate::internal::ximage_io::ImageEntry>)> = self
                .pages
                .iter()
                .filter(|p| p.resources.images().is_dirty())
                .map(|p| {
                    let images = p
                        .resources
                        .images()
                        .for_save()
                        .into_iter()
                        .map(|img| crate::internal::ximage_io::ImageEntry {
                            name: img.name().to_string(),
                            kind: match img.kind() {
                                crate::public::x_image::ImageKind::Encoded => {
                                    crate::internal::ximage_io::Kind::Encoded
                                }
                                crate::public::x_image::ImageKind::RasterRgba8 => {
                                    crate::internal::ximage_io::Kind::RasterRgba8
                                }
                            },
                            raw: img.raw_bytes().to_vec(),
                            width: img.width(),
                            height: img.height(),
                        })
                        .collect();
                    (p.leaf_id(), images)
                })
                .collect();
            if let Some(updated) = crate::internal::ximage_io::append(&working, &pages) {
                working = updated;
            }
        }

        if geometry_dirty {
            let edits: Vec<crate::internal::page_geometry_io::GeometryEdit> = self
                .pages
                .iter()
                .filter(|p| p.geometry.borrow().dirty)
                .map(|p| {
                    let g = p.geometry.borrow();
                    crate::internal::page_geometry_io::GeometryEdit {
                        leaf_id: p.leaf_id(),
                        media_box: [
                            g.media_box.llx(),
                            g.media_box.lly(),
                            g.media_box.urx(),
                            g.media_box.ury(),
                        ],
                        crop_box: [
                            g.crop_box.llx(),
                            g.crop_box.lly(),
                            g.crop_box.urx(),
                            g.crop_box.ury(),
                        ],
                        art_box: g.art_box.map(rect_tuple),
                        bleed_box: g.bleed_box.map(rect_tuple),
                        trim_box: g.trim_box.map(rect_tuple),
                        rotate: g.rotate,
                    }
                })
                .collect();
            if let Some(updated) = crate::internal::page_geometry_io::append(&working, &edits) {
                working = updated;
            }
        }

        if form_dirty && self.form.is_flattened() {
            // §12.7.2: the form IS the catalog's /AcroForm, so a
            // flattened document has that entry removed rather than a
            // form written with an empty field list.
            if let Some(updated) = crate::internal::acroform_io::remove(&working) {
                working = updated;
            }
        } else if form_dirty {
            let specs: Vec<crate::internal::acroform_io::FieldSpec> = self
                .form
                .fields()
                .iter()
                .filter_map(|f| {
                    let page_number = f.page_index();
                    let page_leaf_id =
                        crate::internal::acroform_io::page_leaf_for_number(&working, page_number)?;
                    let r = f.rect();
                    Some(crate::internal::acroform_io::FieldSpec {
                        ft: String::from_utf8_lossy(f.pdf_field_type()).into_owned(),
                        partial_name: f.partial_name().to_string(),
                        alternate_name: f.alternate_name().to_string(),
                        mapping_name: f.mapping_name().to_string(),
                        value: f.value().to_string(),
                        rect: [r.llx(), r.lly(), r.urx(), r.ury()],
                        page_leaf_id,
                        read_only: f.read_only(),
                        required: f.required(),
                        exportable: f.exportable(),
                        // §12.5.3: a field with no flags of its own is
                        // written as Print, which is what a form field
                        // is for; anything the caller set is kept.
                        appearance_flags: match f.flags() {
                            crate::public::annotation_flags::AnnotationFlags::Default => 4,
                            other => other as i64,
                        },
                        flags: f.pdf_field_flags(),
                        // A field IS a widget here (§12.7.3.1 merges the
                        // two objects), so it carries the widget's own
                        // entries — the activation action above all —
                        // alongside the ones its field type contributes.
                        entries: field_entries(f.as_ref()),
                    })
                })
                .collect();
            if let Some(updated) = crate::internal::acroform_io::append(&working, &specs) {
                working = updated;
            }
        }

        Ok(working)
    }

    /// Render each page's added drawing paragraphs into the shared
    /// content-edit buffer (one chunk per page), then clear their dirty
    /// flag. The existing content-edit flush turns the buffer into a new
    /// content stream appended to the page.
    fn render_dirty_paragraphs(&self) {
        for page in self.pages.iter() {
            if !page.paragraphs.is_dirty() {
                continue;
            }
            for rendered in page.paragraphs.render_all(page.height()) {
                let mut edits = self.edits.borrow_mut();
                if !rendered.content.is_empty() {
                    edits.content.push(crate::public::page_edits::PageContentEdit {
                        leaf_id: page.leaf_ref(),
                        content: rendered.content,
                        fonts: rendered.fonts,
                    });
                }
                // A broken table's overflow flows onto whole new pages.
                for cont in rendered.continuations {
                    edits.new_pages.push(crate::public::page_edits::NewPage {
                        content: cont.content,
                        fonts: cont.fonts,
                        media_box: cont.media_box,
                    });
                }
            }
            page.paragraphs.clear_dirty();
        }
    }

/// Render each page's added artifacts (watermarks). A text artifact
    /// is rendered as a positioned text run pushed into the shared
    /// content-edit buffer (the existing content-edit flush turns it into
    /// an appended content stream — so the text is extractable after
    /// save). An image artifact is embedded into the page's image
    /// resources (the existing image save-through flushes it as a
    /// `/Resources /XObject /Image`) and painted at its origin, at the
    /// image's natural size. The artifact's `Position` sets the run /
    /// image origin and `Rotation` turns the run or image about it.
    /// `Opacity` and `IsBackground` are carried but not applied — the
    /// first needs a graphics-state resource the content-edit flush does
    /// not yet write, the second a place in the page's content order
    /// that an appended stream cannot take. Clears each rendered
    /// collection's dirty flag.
    fn render_dirty_artifacts(&self) {
        use crate::public::page_edits::PageContentEdit;
        for page in self.pages.iter() {
            if !page.artifacts.is_dirty() {
                continue;
            }
            for artifact in page.artifacts.for_save() {
                if let Some(bytes) = artifact.image() {
                    // Skip unrecognised image bytes (forgiving posture
                    // matching the other save-through steps).
                    let probe = crate::public::x_image::XImage::from_bytes(
                        bytes.to_vec(),
                        "probe",
                    );
                    if probe.width() > 0 && probe.height() > 0 {
                        let name = page.resources.images().add_stream(bytes);
                        // An artifact carries an origin but no rectangle,
                        // so the image paints at its natural size, one
                        // pixel to the point.
                        let pos = artifact.position();
                        let rect = [
                            pos.x(),
                            pos.y(),
                            pos.x() + probe.width() as f64,
                            pos.y() + probe.height() as f64,
                        ];
                        let content = crate::public::page_edits::rotated_about(
                            crate::public::page_edits::image_draw_ops(&name, rect, None),
                            artifact.rotation(),
                            pos.x(),
                            pos.y(),
                        );
                        self.edits.borrow_mut().content.push(PageContentEdit {
                            leaf_id: page.leaf_ref(),
                            content,
                            fonts: Vec::new(),
                        });
                    }
                    continue;
                }
                if artifact.text().is_empty() {
                    continue;
                }
                let mut frag = crate::public::text_fragment::TextFragment::with_text(
                    artifact.text().to_string(),
                );
                frag.text_state_mut().state = artifact.text_state().clone();
                let pos = artifact.position();
                frag.set_position(crate::public::position::Position::new(pos.x(), pos.y()));
                if let Some((content, font)) =
                    crate::public::text_builder::render_text_fragment(&frag)
                {
                    let content = crate::public::page_edits::rotated_about(
                        content,
                        artifact.rotation(),
                        pos.x(),
                        pos.y(),
                    );
                    self.edits.borrow_mut().content.push(PageContentEdit {
                        leaf_id: page.leaf_ref(),
                        content,
                        fonts: vec![font],
                    });
                }
            }
            page.artifacts.clear_dirty();
        }
    }

    /// Flush pending page content edits (from `TextBuilder`) as an
    /// incremental update: a new content-stream object per edited
    /// page (the appended operators), a base font object per
    /// referenced font, and a rewritten page dict whose `/Contents`
    /// gains the new stream and whose `/Resources` `/Font` gains the
    /// font(s). Object ids are allocated from the trailer `/Size`.
    /// Burn pending redaction regions out of the existing page content
    /// as an incremental update: each affected content stream is decoded,
    /// every text-show whose origin falls inside a redaction rect is
    /// removed, and the stream is re-emitted (uncompressed) overriding its
    /// original object id. Runs before [`Self::append_content_edits`] so a
    /// page can both lose redacted text and gain appended content in one
    /// save. Returns `working` unchanged when nothing matched / parse
    /// failed — forgiving posture matching the other save-through steps.
    fn apply_redactions(&self, working: Vec<u8>) -> Result<Vec<u8>, Error> {
        let requests: Vec<crate::internal::redaction_io::RedactRequest> = self
            .edits
            .borrow()
            .redactions
            .iter()
            .map(|r| crate::internal::redaction_io::RedactRequest {
                leaf_id: r.leaf_id,
                rect: r.rect,
            })
            .collect();
        if requests.is_empty() {
            return Ok(working);
        }
        match crate::internal::redaction_io::apply(&working, &requests) {
            Some(updated) => Ok(updated),
            None => Ok(working),
        }
    }

    fn append_content_edits(&self, working: Vec<u8>) -> Result<Vec<u8>, Error> {
        use crate::internal::{objects, trailer, xref};
        use objects::Value;
        use pdf_writer_incremental::DirtyObject;
        use std::collections::BTreeMap;

        let edits = self.edits.borrow();
        if edits.content.is_empty() {
            return Ok(working);
        }

        let trailer_bundle = match trailer::parse(&working) {
            Ok(t) => t,
            Err(_) => return Ok(working),
        };
        let xref_bundle = match xref::parse(&working) {
            Ok(x) => x,
            Err(_) => return Ok(working),
        };

        // Object id → (offset, generation).
        let mut loc: BTreeMap<u32, (usize, u16)> = BTreeMap::new();
        for e in &xref_bundle.entries {
            if let xref::Entry::InUse { id, gen, offset } = e {
                loc.entry(*id).or_insert((*offset as usize, *gen));
            }
        }

        // Group edits by page (preserve first-seen order; merge
        // content; dedup fonts by resource name).
        let mut order: Vec<u32> = Vec::new();
        let mut by_page: BTreeMap<u32, (Vec<u8>, Vec<crate::public::page_edits::EditFont>)> =
            BTreeMap::new();
        for e in edits.content.iter() {
            let leaf_id = e.leaf_id.get();
            let slot = by_page.entry(leaf_id).or_insert_with(|| {
                order.push(leaf_id);
                (Vec::new(), Vec::new())
            });
            slot.0.extend_from_slice(&e.content);
            for f in &e.fonts {
                match slot.1.iter_mut().find(|x| x.resource_name == f.resource_name) {
                    Some(existing) => {
                        existing.used_codes.extend_from_slice(&f.used_codes);
                        existing.is_subset |= f.is_subset;
                    }
                    None => slot.1.push(f.clone()),
                }
            }
        }

        let mut next_id = trailer_bundle.size as u32;
        let mut dirty: Vec<DirtyObject> = Vec::new();

        for leaf_id in &order {
            let (content, fonts) = by_page.get(leaf_id).expect("grouped page");
            let (offset, gen) = match loc.get(leaf_id) {
                Some(v) => *v,
                None => continue,
            };
            let page_obj = match objects::parse_object(&working, offset) {
                Ok(o) => o,
                Err(_) => continue,
            };
            let mut page_dict = match page_obj.value {
                Value::Dict(d) => d,
                _ => continue,
            };

            // New content-stream object.
            let content_id = next_id;
            next_id += 1;
            dirty.push(DirtyObject {
                id: content_id,
                generation: 0,
                value: Value::Stream { dict: Vec::new(), body: content.clone() },
            });

            // Font objects: an embedded /TrueType (with /FontDescriptor
            // + /FontFile2) when the font carries an embeddable program,
            // otherwise a base /Type1 dict.
            let font_refs = emit_fonts(&mut next_id, &mut dirty, fonts);

            append_content_ref(&mut page_dict, content_id);
            add_fonts_to_resources(
                &mut page_dict,
                &font_refs,
                &working,
                &loc,
                &mut dirty,
            );

            dirty.push(DirtyObject {
                id: *leaf_id,
                generation: gen,
                value: Value::Dict(page_dict),
            });
        }
        drop(edits);

        if dirty.is_empty() {
            return Ok(working);
        }
        Ok(pdf_writer_incremental::append_incremental(&working, &dirty)?)
    }

    /// Append whole new pages (a broken table's overflow continuation
    /// pages) to the `/Pages` tree. Each new page gets a content-stream
    /// object (the baked-in operators), its font objects, and a leaf
    /// dict (`/Contents` + `/Resources /Font`); the new leaf refs are
    /// appended to the root `/Kids` and `/Count` bumped — all one
    /// incremental update. Returns `working` unchanged on parse failure.
    fn append_continuation_pages(&self, working: Vec<u8>) -> Result<Vec<u8>, Error> {
        use crate::internal::{objects, trailer, xref};
        use objects::Value;
        use pdf_writer_incremental::DirtyObject;

        let new_pages = self.edits.borrow().new_pages.clone();
        if new_pages.is_empty() {
            return Ok(working);
        }
        let tr = match trailer::parse(&working) {
            Ok(t) => t,
            Err(_) => return Ok(working),
        };
        let xb = match xref::parse(&working) {
            Ok(x) => x,
            Err(_) => return Ok(working),
        };
        // Latest object id → offset (incremental xref resolves to newest).
        let mut loc: std::collections::BTreeMap<u32, usize> = std::collections::BTreeMap::new();
        for e in &xb.entries {
            if let xref::Entry::InUse { id, offset, .. } = e {
                loc.insert(*id, *offset as usize);
            }
        }
        let resolve = |id: u32| -> Option<Value> {
            loc.get(&id).and_then(|o| objects::parse_object(&working, *o).ok().map(|x| x.value))
        };
        let dict_get = |v: &Value, key: &[u8]| -> Option<Value> {
            match v {
                Value::Dict(d) | Value::Stream { dict: d, .. } => {
                    d.iter().find(|(k, _)| k.as_slice() == key).map(|(_, val)| val.clone())
                }
                _ => None,
            }
        };

        let catalog = match resolve(tr.root_id) {
            Some(c) => c,
            None => return Ok(working),
        };
        let pages_root_id = match dict_get(&catalog, b"Pages") {
            Some(Value::Ref { id, .. }) => id,
            _ => return Ok(working),
        };
        let mut root_dict = match resolve(pages_root_id) {
            Some(Value::Dict(d)) => d,
            _ => return Ok(working),
        };
        let mut kids: Vec<Value> = match dict_get(&Value::Dict(root_dict.clone()), b"Kids") {
            Some(Value::Array(a)) => a,
            _ => return Ok(working),
        };

        let mut next_id = tr.size as u32;
        let mut dirty: Vec<DirtyObject> = Vec::new();

        for page in &new_pages {
            let content_id = next_id;
            next_id += 1;
            dirty.push(DirtyObject {
                id: content_id,
                generation: 0,
                value: Value::Stream { dict: Vec::new(), body: page.content.clone() },
            });

            let font_refs = emit_fonts(&mut next_id, &mut dirty, &page.fonts);
            let font_dict: Vec<(Vec<u8>, Value)> = font_refs;
            let resources = Value::Dict(vec![(b"Font".to_vec(), Value::Dict(font_dict))]);

            let page_id = next_id;
            next_id += 1;
            let leaf = vec![
                (b"Type".to_vec(), Value::Name(b"Page".to_vec())),
                (b"Parent".to_vec(), Value::Ref { id: pages_root_id, gen: 0 }),
                (
                    b"MediaBox".to_vec(),
                    Value::Array(page.media_box.iter().map(|c| Value::Real(*c)).collect()),
                ),
                (b"Contents".to_vec(), Value::Ref { id: content_id, gen: 0 }),
                (b"Resources".to_vec(), resources),
            ];
            dirty.push(DirtyObject { id: page_id, generation: 0, value: Value::Dict(leaf) });
            kids.push(Value::Ref { id: page_id, gen: 0 });
        }

        let count = kids.len() as i64;
        set_dict_entry(&mut root_dict, b"Kids", Value::Array(kids));
        set_dict_entry(&mut root_dict, b"Count", Value::Integer(count));
        dirty.push(DirtyObject {
            id: pages_root_id,
            generation: 0,
            value: Value::Dict(root_dict),
        });

        Ok(pdf_writer_incremental::append_incremental(&working, &dirty)?)
    }

    /// v1.1 metadata write-through. Builds the dirty list for an
    /// incremental `/Metadata` stream update over `working`.
    /// When the Catalog already references a `/Metadata`
    /// indirect, the stream is rewritten at that id. When
    /// `/Metadata` is absent, a fresh stream id is allocated
    /// (`trailer.size`) and the Catalog joins the dirty list
    /// with the new `/Metadata` Ref injected. Returns `working`
    /// unchanged on parse failure — forgiving posture matching
    /// the /Info path.
    fn append_metadata_update(
        &self,
        working: Vec<u8>,
    ) -> Result<Vec<u8>, Error> {
        use crate::internal::{objects, trailer};

        let trailer_bundle = match trailer::parse(&working) {
            Ok(t) => t,
            Err(_) => return Ok(working),
        };
        if trailer_bundle.root_id == 0 {
            return Ok(working);
        }

        // Walk objects to find Catalog.
        let xref_bundle = match crate::internal::xref::parse(&working) {
            Ok(x) => x,
            Err(_) => return Ok(working),
        };
        let mut catalog_obj: Option<objects::IndirectObject> = None;
        let mut catalog_gen: u16 = 0;
        for entry in &xref_bundle.entries {
            if let crate::internal::xref::Entry::InUse { id, gen, offset } = entry {
                if *id == trailer_bundle.root_id {
                    if let Ok(obj) = objects::parse_object(
                        &working, *offset as usize) {
                        catalog_obj = Some(obj);
                        catalog_gen = *gen;
                        break;
                    }
                }
            }
        }
        let catalog = match catalog_obj {
            Some(c) => c,
            None => return Ok(working),
        };
        let catalog_dict = match catalog.value {
            objects::Value::Dict(d) => d,
            _ => return Ok(working),
        };

        // Decide stream id: reuse existing /Metadata Ref or
        // allocate trailer.size.
        let mut stream_id: u32 = 0;
        let mut stream_gen: u16 = 0;
        let mut catalog_dirty = true;
        for (k, v) in &catalog_dict {
            if k == b"Metadata" {
                if let objects::Value::Ref { id, gen } = v {
                    stream_id = *id;
                    stream_gen = *gen;
                    catalog_dirty = false;
                }
                break;
            }
        }
        if catalog_dirty {
            stream_id = trailer_bundle.size as u32;
            stream_gen = 0;
        }

        let xmp_body = self.metadata.to_xmp_packet()?;
        let stream_value = objects::Value::Stream {
            dict: vec![
                (b"Type".to_vec(),
                    objects::Value::Name(b"Metadata".to_vec())),
                (b"Subtype".to_vec(),
                    objects::Value::Name(b"XML".to_vec())),
            ],
            body: xmp_body,
        };

        let mut dirty_list = vec![pdf_writer_incremental::DirtyObject {
            id: stream_id,
            generation: stream_gen,
            value: stream_value,
        }];
        if catalog_dirty {
            let mut updated_catalog = catalog_dict;
            updated_catalog.push((
                b"Metadata".to_vec(),
                objects::Value::Ref { id: stream_id, gen: stream_gen },
            ));
            dirty_list.push(pdf_writer_incremental::DirtyObject {
                id: catalog.id,
                generation: catalog_gen,
                value: objects::Value::Dict(updated_catalog),
            });
        }

        Ok(pdf_writer_incremental::append_incremental(
            &working, &dirty_list)?)
    }

    /// Incremental write-back for the catalog viewer preferences
    /// (`/PageLayout`, `/PageMode`, and the `/ViewerPreferences`
    /// sub-dict's `/Direction` + `/NonFullScreenPageMode`). Forgiving:
    /// returns `working` unchanged on parse failure.
    fn append_viewer_prefs_update(&self, working: Vec<u8>) -> Result<Vec<u8>, Error> {
        use crate::internal::{objects, trailer, xref};

        let trailer_bundle = match trailer::parse(&working) {
            Ok(t) if t.root_id != 0 => t,
            _ => return Ok(working),
        };
        let xref_bundle = match xref::parse(&working) {
            Ok(x) => x,
            Err(_) => return Ok(working),
        };
        let mut catalog_obj: Option<objects::IndirectObject> = None;
        let mut catalog_gen: u16 = 0;
        for entry in &xref_bundle.entries {
            if let xref::Entry::InUse { id, gen, offset } = *entry {
                if id == trailer_bundle.root_id {
                    if let Ok(obj) = objects::parse_object(&working, offset as usize) {
                        catalog_obj = Some(obj);
                        catalog_gen = gen;
                        break;
                    }
                }
            }
        }
        let catalog = match catalog_obj {
            Some(c) => c,
            None => return Ok(working),
        };
        let mut catalog_dict = match catalog.value {
            objects::Value::Dict(d) => d,
            _ => return Ok(working),
        };

        // Top-level /PageLayout (omitted when unset) + /PageMode.
        match page_layout_name(self.page_layout) {
            Some(name) => dict_set_name(&mut catalog_dict, b"PageLayout", name),
            None => dict_remove(&mut catalog_dict, b"PageLayout"),
        }
        dict_set_name(&mut catalog_dict, b"PageMode", page_mode_name(self.page_mode));

        // /ViewerPreferences sub-dict — merge /Direction +
        // /NonFullScreenPageMode, preserving any existing entries.
        let mut dirty_list: Vec<pdf_writer_incremental::DirtyObject> = Vec::new();
        let vp_existing = dict_lookup(&catalog_dict, b"ViewerPreferences").cloned();
        match vp_existing {
            Some(objects::Value::Ref { id, gen }) => {
                let mut vp_dict = resolve_indirect_dict(&working, &xref_bundle, id, gen);
                dict_set_name(&mut vp_dict, b"Direction", direction_name(self.direction));
                dict_set_name(&mut vp_dict, b"NonFullScreenPageMode",
                    page_mode_name(self.non_full_screen_page_mode));
                dict_set_name(&mut vp_dict, b"Duplex", duplex_name(self.duplex));
                dict_set_name(&mut vp_dict, b"PrintScaling",
                    print_scaling_name(self.print_scaling));
                dirty_list.push(pdf_writer_incremental::DirtyObject {
                    id,
                    generation: gen,
                    value: objects::Value::Dict(vp_dict),
                });
            }
            other => {
                let mut vp_dict = match other {
                    Some(objects::Value::Dict(d)) => d,
                    _ => Vec::new(),
                };
                dict_set_name(&mut vp_dict, b"Direction", direction_name(self.direction));
                dict_set_name(&mut vp_dict, b"NonFullScreenPageMode",
                    page_mode_name(self.non_full_screen_page_mode));
                dict_set_name(&mut vp_dict, b"Duplex", duplex_name(self.duplex));
                dict_set_name(&mut vp_dict, b"PrintScaling",
                    print_scaling_name(self.print_scaling));
                dict_set(&mut catalog_dict, b"ViewerPreferences",
                    objects::Value::Dict(vp_dict));
            }
        }

        dirty_list.push(pdf_writer_incremental::DirtyObject {
            id: catalog.id,
            generation: catalog_gen,
            value: objects::Value::Dict(catalog_dict),
        });

        Ok(pdf_writer_incremental::append_incremental(&working, &dirty_list)?)
    }
}

impl Default for Document {
    /// Equivalent to [`Document::new`] — default-construct an
    /// empty PDF document.
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// Viewer-preference catalog helpers
// ============================================================

/// The PDF `/PageLayout` name for a layout, or `None` when the layout
/// is unset ([`PageLayout::Default`](crate::PageLayout::Default) — the
/// key is omitted so the viewer picks its own default).
fn page_layout_name(
    layout: crate::public::page_layout::PageLayout,
) -> Option<&'static [u8]> {
    use crate::public::page_layout::PageLayout;
    match layout {
        PageLayout::SinglePage => Some(b"SinglePage"),
        PageLayout::OneColumn => Some(b"OneColumn"),
        PageLayout::TwoColumnLeft => Some(b"TwoColumnLeft"),
        PageLayout::TwoColumnRight => Some(b"TwoColumnRight"),
        PageLayout::TwoPageLeft => Some(b"TwoPageLeft"),
        PageLayout::TwoPageRight => Some(b"TwoPageRight"),
        PageLayout::Default => None,
    }
}

/// The PDF name for a page mode.
fn page_mode_name(mode: crate::public::page_mode::PageMode) -> &'static [u8] {
    use crate::public::page_mode::PageMode;
    match mode {
        PageMode::UseNone => b"UseNone",
        PageMode::UseOutlines => b"UseOutlines",
        PageMode::UseThumbs => b"UseThumbs",
        PageMode::FullScreen => b"FullScreen",
        PageMode::UseOC => b"UseOC",
        PageMode::UseAttachments => b"UseAttachments",
    }
}

/// The PDF name for a reading direction.
fn direction_name(dir: crate::public::direction::Direction) -> &'static [u8] {
    use crate::public::direction::Direction;
    match dir {
        Direction::L2R => b"L2R",
        Direction::R2L => b"R2L",
    }
}

/// The PDF name for a duplex mode.
fn duplex_name(d: crate::public::print_duplex::PrintDuplex) -> &'static [u8] {
    use crate::public::print_duplex::PrintDuplex;
    match d {
        PrintDuplex::Simplex => b"Simplex",
        PrintDuplex::DuplexFlipShortEdge => b"DuplexFlipShortEdge",
        PrintDuplex::DuplexFlipLongEdge => b"DuplexFlipLongEdge",
    }
}

/// The PDF name for a print-scaling mode.
fn print_scaling_name(s: crate::public::print_scaling::PrintScaling) -> &'static [u8] {
    use crate::public::print_scaling::PrintScaling;
    match s {
        PrintScaling::AppDefault => b"AppDefault",
        PrintScaling::None => b"None",
    }
}

/// Set `key` to a `/Name` value, replacing any existing entry.
fn dict_set_name(
    dict: &mut Vec<(Vec<u8>, crate::internal::objects::Value)>,
    key: &[u8],
    name: &[u8],
) {
    dict_set(dict, key, crate::internal::objects::Value::Name(name.to_vec()));
}

/// Set `key` to `value`, replacing any existing entry.
fn dict_set(
    dict: &mut Vec<(Vec<u8>, crate::internal::objects::Value)>,
    key: &[u8],
    value: crate::internal::objects::Value,
) {
    if let Some(slot) = dict.iter_mut().find(|(k, _)| k == key) {
        slot.1 = value;
    } else {
        dict.push((key.to_vec(), value));
    }
}

/// Remove `key` if present.
fn dict_remove(dict: &mut Vec<(Vec<u8>, crate::internal::objects::Value)>, key: &[u8]) {
    dict.retain(|(k, _)| k != key);
}

/// Find `key` in a dict.
fn dict_lookup<'a>(
    dict: &'a [(Vec<u8>, crate::internal::objects::Value)],
    key: &[u8],
) -> Option<&'a crate::internal::objects::Value> {
    dict.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// Resolve an indirect dictionary object to its entries (empty when the
/// object is absent or not a dict).
fn resolve_indirect_dict(
    working: &[u8],
    xref_bundle: &crate::internal::xref::Table,
    id: u32,
    gen: u16,
) -> Vec<(Vec<u8>, crate::internal::objects::Value)> {
    use crate::internal::{objects, xref};
    for entry in &xref_bundle.entries {
        if let xref::Entry::InUse { id: eid, gen: egen, offset } = *entry {
            if eid == id && egen == gen {
                if let Ok(obj) = objects::parse_object(working, offset as usize) {
                    if let objects::Value::Dict(d) = obj.value {
                        return d;
                    }
                }
                break;
            }
        }
    }
    Vec::new()
}

// ============================================================
// Construction helpers
// ============================================================

/// Build the minimal valid PDF byte sequence parsed by
/// [`Document::new`]. Header + catalog + 0-page pages tree +
/// xref + trailer + EOF marker. Same algorithm as the C# / C++
/// / TypeScript peer `BuildEmptyPdf` helpers.
fn build_empty_pdf() -> Vec<u8> {
    let header = b"%PDF-1.7\n";
    let obj1 = b"1 0 obj\n<</Type/Catalog/Pages 2 0 R>>\nendobj\n";
    let obj2 = b"2 0 obj\n<</Type/Pages/Count 0/Kids[]>>\nendobj\n";
    let obj1_off = header.len();
    let obj2_off = obj1_off + obj1.len();
    let xref_off = obj2_off + obj2.len();
    let xref = format!(
        "xref\n0 3\n\
         0000000000 65535 f \n\
         {obj1_off:010} 00000 n \n\
         {obj2_off:010} 00000 n \n\
         trailer\n<</Size 3/Root 1 0 R>>\nstartxref\n\
         {xref_off}\n\
         %%EOF\n",
    );

    let mut out = Vec::with_capacity(
        header.len() + obj1.len() + obj2.len() + xref.len(),
    );
    out.extend_from_slice(header);
    out.extend_from_slice(obj1);
    out.extend_from_slice(obj2);
    out.extend_from_slice(xref.as_bytes());
    out
}

/// Append a content-stream reference to a page dict's `/Contents`
/// (turning a single ref into an array, extending an array, or
/// creating one).
fn append_content_ref(
    dict: &mut Vec<(Vec<u8>, crate::internal::objects::Value)>,
    content_id: u32,
) {
    use crate::internal::objects::Value;
    let new_ref = Value::Ref { id: content_id, gen: 0 };
    if let Some(pos) = dict.iter().position(|(k, _)| k == b"Contents") {
        let arr = match dict[pos].1.clone() {
            Value::Array(mut items) => {
                items.push(new_ref);
                items
            }
            other => vec![other, new_ref],
        };
        dict[pos].1 = Value::Array(arr);
    } else {
        dict.push((b"Contents".to_vec(), Value::Array(vec![new_ref])));
    }
}

/// A deterministic 6-uppercase-letter subset tag derived from the used
/// char-code set (PDF §9.6.4 subset-font naming: `XXXXXX+BaseFont`).
/// Reproducible — the same glyph set always yields the same tag.
/// Set (or insert) a dict entry by key.
fn set_dict_entry(entries: &mut Vec<(Vec<u8>, Value)>, key: &[u8], value: Value) {
    if let Some(slot) = entries.iter_mut().find(|(k, _)| k.as_slice() == key) {
        slot.1 = value;
    } else {
        entries.push((key.to_vec(), value));
    }
}

fn subset_tag(used_codes: &[u8]) -> String {
    // A small FNV-1a hash over the sorted, deduped code set keeps the
    // tag stable regardless of draw order or repeats.
    let mut codes: Vec<u8> = used_codes.to_vec();
    codes.sort_unstable();
    codes.dedup();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &c in &codes {
        hash ^= c as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let mut tag = String::with_capacity(6);
    for _ in 0..6 {
        tag.push((b'A' + (hash % 26) as u8) as char);
        hash /= 26;
    }
    tag
}

/// Emit a font object per [`EditFont`] (an embedded `/TrueType` with
/// `/FontDescriptor` + `/FontFile2` — subset + tagged when `is_subset`
/// — when the font carries a program, else a base `/Type1` dict), and
/// return the `(resource_name, Ref)` pairs for the page `/Resources`.
fn emit_fonts(
    next_id: &mut u32,
    dirty: &mut Vec<pdf_writer_incremental::DirtyObject>,
    fonts: &[crate::public::page_edits::EditFont],
) -> Vec<(Vec<u8>, Value)> {
    use pdf_writer_incremental::DirtyObject;
    let mut font_refs: Vec<(Vec<u8>, Value)> = Vec::new();
    for f in fonts {
        // When subsetting, replace the program with a glyph-stripped
        // subset and prefix the BaseFont with a canonical 6-letter tag.
        // Falls back to the full program if subsetting fails.
        let embed_program: Option<Vec<u8>> = f.program.as_ref().map(|p| {
            if f.is_subset {
                crate::internal::font_subset::subset(p, &f.used_codes).unwrap_or_else(|| p.clone())
            } else {
                p.clone()
            }
        });
        let embed_base_font = if f.is_subset && f.program.is_some() {
            format!("{}+{}", subset_tag(&f.used_codes), f.base_font)
        } else {
            f.base_font.clone()
        };
        let metrics = embed_program
            .as_ref()
            .and_then(|p| crate::internal::font_embed::metrics(p).map(|m| (p.clone(), m)));
        let fid = match metrics {
            Some((program, m)) => {
                emit_embedded_truetype(next_id, dirty, &embed_base_font, &program, &m)
            }
            None => {
                let fid = *next_id;
                *next_id += 1;
                dirty.push(DirtyObject {
                    id: fid,
                    generation: 0,
                    value: Value::Dict(vec![
                        (b"Type".to_vec(), Value::Name(b"Font".to_vec())),
                        (b"Subtype".to_vec(), Value::Name(b"Type1".to_vec())),
                        (b"BaseFont".to_vec(), Value::Name(f.base_font.clone().into_bytes())),
                    ]),
                });
                fid
            }
        };
        font_refs.push((f.resource_name.clone(), Value::Ref { id: fid, gen: 0 }));
    }
    font_refs
}

/// Emit the three objects of an embedded TrueType font — the
/// `/FontFile2` program stream, the `/FontDescriptor`, and the
/// `/TrueType` font dict — and return the font dict's object id.
fn emit_embedded_truetype(
    next_id: &mut u32,
    dirty: &mut Vec<pdf_writer_incremental::DirtyObject>,
    base_font: &str,
    program: &[u8],
    m: &crate::internal::font_embed::FontMetrics,
) -> u32 {
    use crate::internal::objects::Value;
    use pdf_writer_incremental::DirtyObject;

    let name = base_font.as_bytes().to_vec();

    let ff_id = *next_id;
    *next_id += 1;
    dirty.push(DirtyObject {
        id: ff_id,
        generation: 0,
        value: Value::Stream {
            dict: vec![(b"Length1".to_vec(), Value::Integer(program.len() as i64))],
            body: program.to_vec(),
        },
    });

    let fd_id = *next_id;
    *next_id += 1;
    dirty.push(DirtyObject {
        id: fd_id,
        generation: 0,
        value: Value::Dict(vec![
            (b"Type".to_vec(), Value::Name(b"FontDescriptor".to_vec())),
            (b"FontName".to_vec(), Value::Name(name.clone())),
            (b"Flags".to_vec(), Value::Integer(32)), // Nonsymbolic
            (
                b"FontBBox".to_vec(),
                Value::Array(m.bbox.iter().map(|v| Value::Integer(*v)).collect()),
            ),
            (b"ItalicAngle".to_vec(), Value::Integer(0)),
            (b"Ascent".to_vec(), Value::Integer(m.ascent)),
            (b"Descent".to_vec(), Value::Integer(m.descent)),
            (b"CapHeight".to_vec(), Value::Integer(m.cap_height)),
            (b"StemV".to_vec(), Value::Integer(80)),
            (b"FontFile2".to_vec(), Value::Ref { id: ff_id, gen: 0 }),
        ]),
    });

    let font_id = *next_id;
    *next_id += 1;
    dirty.push(DirtyObject {
        id: font_id,
        generation: 0,
        value: Value::Dict(vec![
            (b"Type".to_vec(), Value::Name(b"Font".to_vec())),
            (b"Subtype".to_vec(), Value::Name(b"TrueType".to_vec())),
            (b"BaseFont".to_vec(), Value::Name(name)),
            (b"FirstChar".to_vec(), Value::Integer(m.first_char as i64)),
            (b"LastChar".to_vec(), Value::Integer(m.last_char as i64)),
            (
                b"Widths".to_vec(),
                Value::Array(m.widths.iter().map(|w| Value::Integer(*w)).collect()),
            ),
            (b"FontDescriptor".to_vec(), Value::Ref { id: fd_id, gen: 0 }),
            (b"Encoding".to_vec(), Value::Name(b"WinAnsiEncoding".to_vec())),
        ]),
    });

    font_id
}

/// Add font resource entries to a page dict's `/Resources` `/Font`.
/// Handles an inline `/Resources` dict (the common case) and an
/// indirect `/Resources` / `/Font` (parsed and rewritten as an
/// extra dirty object). Adds an inline `/Resources` when absent.
fn add_fonts_to_resources(
    page_dict: &mut Vec<(Vec<u8>, crate::internal::objects::Value)>,
    font_refs: &[(Vec<u8>, crate::internal::objects::Value)],
    working: &[u8],
    loc: &std::collections::BTreeMap<u32, (usize, u16)>,
    dirty: &mut Vec<pdf_writer_incremental::DirtyObject>,
) {
    use crate::internal::objects::Value;
    if font_refs.is_empty() {
        return;
    }
    match page_dict.iter().position(|(k, _)| k == b"Resources") {
        Some(pos) => match page_dict[pos].1.clone() {
            Value::Dict(res) => {
                page_dict[pos].1 =
                    Value::Dict(add_fonts_to_resdict(res, font_refs, working, loc, dirty));
            }
            Value::Ref { id, gen } => {
                if let Some(res) = resolve_dict(id, working, loc) {
                    let new_res =
                        add_fonts_to_resdict(res, font_refs, working, loc, dirty);
                    push_unique_dirty(
                        dirty,
                        id,
                        gen,
                        Value::Dict(new_res),
                    );
                }
            }
            _ => {}
        },
        None => {
            let font_dict =
                Value::Dict(font_refs.to_vec());
            page_dict.push((
                b"Resources".to_vec(),
                Value::Dict(vec![(b"Font".to_vec(), font_dict)]),
            ));
        }
    }
}

/// Add fonts to a `/Resources` dict's `/Font`, returning the
/// updated dict. An indirect `/Font` is parsed and rewritten as an
/// extra dirty object.
fn add_fonts_to_resdict(
    mut res: Vec<(Vec<u8>, crate::internal::objects::Value)>,
    font_refs: &[(Vec<u8>, crate::internal::objects::Value)],
    working: &[u8],
    loc: &std::collections::BTreeMap<u32, (usize, u16)>,
    dirty: &mut Vec<pdf_writer_incremental::DirtyObject>,
) -> Vec<(Vec<u8>, crate::internal::objects::Value)> {
    use crate::internal::objects::Value;
    match res.iter().position(|(k, _)| k == b"Font") {
        Some(p) => match res[p].1.clone() {
            Value::Dict(fonts) => {
                res[p].1 = Value::Dict(merge_font_entries(fonts, font_refs));
            }
            Value::Ref { id, gen } => {
                if let Some(fonts) = resolve_dict(id, working, loc) {
                    let merged = merge_font_entries(fonts, font_refs);
                    push_unique_dirty(dirty, id, gen, Value::Dict(merged));
                }
            }
            _ => {}
        },
        None => {
            res.push((
                b"Font".to_vec(),
                Value::Dict(font_refs.to_vec()),
            ));
        }
    }
    res
}

/// Merge new font entries into an existing `/Font` dict, skipping
/// names already present.
fn merge_font_entries(
    mut fonts: Vec<(Vec<u8>, crate::internal::objects::Value)>,
    font_refs: &[(Vec<u8>, crate::internal::objects::Value)],
) -> Vec<(Vec<u8>, crate::internal::objects::Value)> {
    for (name, r) in font_refs {
        if !fonts.iter().any(|(k, _)| k == name) {
            fonts.push((name.clone(), r.clone()));
        }
    }
    fonts
}

/// Parse the dict body of an indirect object by id.
fn resolve_dict(
    id: u32,
    working: &[u8],
    loc: &std::collections::BTreeMap<u32, (usize, u16)>,
) -> Option<Vec<(Vec<u8>, crate::internal::objects::Value)>> {
    use crate::internal::objects::{self, Value};
    let (offset, _) = loc.get(&id)?;
    match objects::parse_object(working, *offset).ok()?.value {
        Value::Dict(d) => Some(d),
        _ => None,
    }
}

/// Push a dirty object only if its id is not already queued (avoids
/// double-rewriting a shared resources / font object).
fn push_unique_dirty(
    dirty: &mut Vec<pdf_writer_incremental::DirtyObject>,
    id: u32,
    generation: u16,
    value: crate::internal::objects::Value,
) {
    if dirty.iter().any(|d| d.id == id) {
        return;
    }
    dirty.push(pdf_writer_incremental::DirtyObject { id, generation, value });
}

fn build_page_collection(
    tree: &pages_tree::Tree,
    bytes: &Arc<Vec<u8>>,
    edits: &crate::public::page_edits::EditBuffer,
) -> PageCollection {
    let pages = tree
        .leaves
        .iter()
        .enumerate()
        .map(|(idx, leaf)| {
            let annotations =
                crate::public::annotation_collection::AnnotationCollection::new();
            for a in crate::internal::annotation_io::parse_page_annots(bytes, leaf.id) {
                let rect = crate::public::rectangle::Rectangle::new(
                    a.rect[0], a.rect[1], a.rect[2], a.rect[3], false,
                );
                let mut ty =
                    crate::public::annotation_type::AnnotationType::from_pdf_subtype(&a.subtype);
                if !a.marks_name.is_empty() {
                    ty = crate::public::annotation_type::AnnotationType::from_printer_mark_name(
                        &a.marks_name,
                    );
                }
                annotations.add_loaded(Box::new(
                    crate::public::annotation::LoadedAnnotation::from_parsed(ty, rect, a),
                ));
            }

            let resources = crate::public::resources::Resources::new();
            for img in crate::internal::ximage_io::parse_page_images(bytes, leaf.id) {
                let color_type = crate::public::x_image::XImage::color_type_from_pdf(
                    img.color_space.as_deref(),
                    img.bits_per_component,
                    img.image_mask,
                );
                let ximage = if img.encoded {
                    crate::public::x_image::XImage::from_bytes(img.raw, img.name)
                } else {
                    crate::public::x_image::XImage::from_raster(
                        img.raw, img.width, img.height, img.name, img.has_alpha,
                    )
                }
                .with_color_type(color_type);
                resources.images().add_loaded(ximage);
            }
            resources.images().clear_dirty();

            // Resolve the page geometry boxes (/MediaBox, /CropBox,
            // /Rotate) into the shared, interior-mutable handle. Falls
            // back to the pages_tree-resolved width/height/rotation when
            // the per-leaf parse can't find a /MediaBox (e.g. a
            // synthesised document the objects parser can't index).
            let geom = crate::internal::page_geometry_io::parse_page_geometry(bytes, leaf.id);
            let to_rect = |b: [f64; 4]| {
                crate::public::rectangle::Rectangle::new(b[0], b[1], b[2], b[3], false)
            };
            let (art, bleed, trim) = match &geom {
                Some(g) => (
                    g.art_box.map(to_rect),
                    g.bleed_box.map(to_rect),
                    g.trim_box.map(to_rect),
                ),
                None => (None, None, None),
            };
            let (media_box, crop_box, rotate) = match geom {
                Some(g) => (to_rect(g.media_box), to_rect(g.crop_box), g.rotate),
                None => {
                    let r = crate::public::rectangle::Rectangle::new(
                        0.0, 0.0, leaf.width, leaf.height, false,
                    );
                    (r, r, leaf.rotation)
                }
            };
            let geometry = std::rc::Rc::new(std::cell::RefCell::new(
                crate::public::page::PageGeometry {
                    media_box,
                    crop_box,
                    art_box: art,
                    bleed_box: bleed,
                    trim_box: trim,
                    rotate,
                    dirty: false,
                },
            ));

            Page {
                number: leaf.pagenum,
                width: leaf.width,
                height: leaf.height,
                rotation: leaf.rotation,
                leaf_id: std::rc::Rc::new(std::cell::Cell::new(leaf.id)),
                leaf_index: idx as u64,
                doc_bytes: Arc::clone(bytes),
                edits: std::rc::Rc::clone(edits),
                annotations,
                resources,
                paragraphs: crate::public::paragraphs::Paragraphs::new(),
                geometry,
                artifacts: crate::public::artifact_collection::ArtifactCollection::new(),
            }
        })
        .collect();
    PageCollection::new(pages, Arc::clone(bytes), std::rc::Rc::clone(edits))
}

/// Resolve the source `/Info` indirect and populate every text-
/// string entry into `info`. Returns the `(id, generation)` of
/// the `/Info` indirect; `(0, 0)` when the document has no
/// `/Info` (trailer absent, indirect resolves to a non-dict,
/// or xref entry is missing / compressed).
fn load_info(input: &[u8], info: &mut DocumentInfo) -> Result<(u32, u16), Error> {
    let t = trailer::parse(input)?;
    if t.info_id == 0 {
        return Ok((0, 0));
    }

    let table = xref::parse(input)?;
    let entry = match table.entries.iter().find(|e| match e {
        Entry::InUse { id, .. } | Entry::Compressed { id, .. } => *id == t.info_id,
    }) {
        Some(e) => e,
        None => return Ok((0, 0)),
    };

    let (gen, offset) = match entry {
        Entry::InUse { gen, offset, .. } => (*gen, *offset),
        // Compressed /Info dicts are out of v1 scope — the
        // dict body lives inside an object stream which needs
        // a separate decoder. Return id=0 so save() rejects a
        // dirty mutation with NoInfoForUpdate.
        Entry::Compressed { .. } => return Ok((0, 0)),
    };

    let obj = match objects::parse_object(input, offset as usize) {
        Ok(o) => o,
        // Malformed /Info — treat as missing so reads succeed
        // (the rest of the doc may be fine) but writes are
        // rejected.
        Err(_) => return Ok((0, 0)),
    };

    let IndirectObject { value, .. } = obj;
    let entries = match value {
        Value::Dict(d) => d,
        _ => return Ok((t.info_id, gen)),
    };

    for (key, val) in entries {
        let key = match std::str::from_utf8(&key) {
            Ok(s) => s.to_string(),
            // Non-UTF8 name objects shouldn't occur in /Info —
            // skip them rather than panicking.
            Err(_) => continue,
        };
        if let Value::String(bytes) = val {
            info.set_entry(key, decode_text_string(&bytes));
        }
    }

    Ok((t.info_id, gen))
}

/// Resolve the document's catalog `/Metadata` indirect (if any),
/// extract the XMP packet body, decode `/FlateDecode` if
/// declared, and parse into a [`Metadata`]. Returns an empty
/// [`Metadata`] when no `/Metadata` is reachable — mirrors the
/// canonical behaviour where `Document.Metadata` always returns
/// a usable object.
fn load_metadata(input: &[u8]) -> Result<Metadata, Error> {
    let t = trailer::parse(input)?;
    if t.root_id == 0 {
        return Ok(Metadata::new());
    }
    let table = xref::parse(input)?;

    let catalog = match resolve_indirect(input, &table, t.root_id) {
        Some(o) => o,
        None => return Ok(Metadata::new()),
    };
    let cat_dict = match catalog.value {
        Value::Dict(d) => d,
        _ => return Ok(Metadata::new()),
    };
    let (meta_id, _meta_gen) = match find_indirect_ref(&cat_dict, b"Metadata") {
        Some(r) => r,
        None => return Ok(Metadata::new()),
    };

    let meta_obj = match resolve_indirect(input, &table, meta_id) {
        Some(o) => o,
        None => return Ok(Metadata::new()),
    };
    let (dict, body) = match meta_obj.value {
        Value::Stream { dict, body } => (dict, body),
        _ => return Ok(Metadata::new()),
    };

    let _ = input;
    let packet: Vec<u8> = if dict_has_flate_filter(&dict) {
        flate::decode(&body).map_err(|e| Error::InvalidPdf(e.to_string()))?
    } else {
        body
    };

    Metadata::from_xmp_packet(&packet)
}

fn resolve_indirect(
    input: &[u8],
    table: &xref::Table,
    id: u32,
) -> Option<IndirectObject> {
    let entry = table.entries.iter().find(|e| match e {
        Entry::InUse { id: eid, .. } | Entry::Compressed { id: eid, .. } => {
            *eid == id
        }
    })?;
    let offset = match entry {
        Entry::InUse { offset, .. } => *offset,
        // Compressed indirects live inside an object stream which
        // needs a separate decoder — out of v1 scope.
        Entry::Compressed { .. } => return None,
    };
    objects::parse_object(input, offset as usize).ok()
}

fn find_indirect_ref(
    dict: &[(Vec<u8>, Value)],
    key: &[u8],
) -> Option<(u32, u16)> {
    for (k, v) in dict {
        if k.as_slice() != key {
            continue;
        }
        if let Value::Ref { id, gen } = v {
            return Some((*id, *gen));
        }
    }
    None
}

fn dict_has_flate_filter(dict: &[(Vec<u8>, Value)]) -> bool {
    for (k, v) in dict {
        if k.as_slice() != b"Filter" {
            continue;
        }
        match v {
            Value::Name(n) if n.as_slice() == b"FlateDecode" => return true,
            Value::Array(items) => {
                for item in items {
                    if let Value::Name(n) = item {
                        if n.as_slice() == b"FlateDecode" {
                            return true;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    false
}

// ============================================================
// PDF text-string codec (PDF 32000-1:2008 §7.9.2.2)
// ============================================================

fn decode_text_string(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        return Encoding::Utf16Be.get_string(&bytes[2..]);
    }
    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        return String::from_utf8_lossy(&bytes[3..]).into_owned();
    }
    decode_pdf_doc_encoding(bytes)
}

fn decode_pdf_doc_encoding(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        let glyph = encoding_tables::byte_to_glyph_name(PdfEnc::PdfDoc, b);
        if glyph.is_empty() {
            // Unmapped slot — Latin-1 byte-as-codepoint fallback
            // (matches the C# / cpp peer convention).
            out.push(b as char);
            continue;
        }
        let cps = agl::lookup(glyph);
        if cps.is_empty() {
            out.push(b as char);
            continue;
        }
        for cp in cps {
            if let Some(c) = char::from_u32(cp) {
                out.push(c);
            }
        }
    }
    out
}

fn encode_text_string_utf16be(text: &str) -> Vec<u8> {
    let utf16 = Encoding::Utf16Be.get_bytes(text);
    let mut out = Vec::with_capacity(utf16.len() + 2);
    out.push(0xFE);
    out.push(0xFF);
    out.extend_from_slice(&utf16);
    out
}

/// Rebuild a `/Info` dict value from the current `DocumentInfo`
/// state. Empty-valued entries are dropped (matches the
/// canonical "unset" semantics — an empty string would inflate
/// the persisted dict on every save).
fn build_info_dict(info: &DocumentInfo) -> Value {
    let mut entries: Vec<(Vec<u8>, Value)> = Vec::with_capacity(info.entries().len());
    for (k, v) in info.entries() {
        if v.is_empty() {
            continue;
        }
        entries.push((k.as_bytes().to_vec(), Value::String(encode_text_string_utf16be(v))));
    }
    Value::Dict(entries)
}
