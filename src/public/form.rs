//! `Aspose.Pdf.Forms.Form` — the document's interactive form
//! (AcroForm), per `capabilities/form.yaml`.
//!
//! Owns the document's form fields. Reached canonically via
//! `Document.Form`; the type has no public constructor. The .NET
//! collection infrastructure, the JSON / XFA-XML import-export
//! helpers, the field-appearance helpers, `DefaultResources`, and
//! the object-identity field operations are dropped (see
//! `targets/rust/drops.yaml`).

use crate::public::default_appearance::DefaultAppearance;
use crate::public::field::Field;
use crate::public::form_type::FormType;
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;
use crate::public::xfa::XFA;
use crate::internal::pending::PENDING;

/// A document's interactive form (AcroForm).
pub struct Form {
    fields: Vec<Box<dyn Field>>,
    xfa: XFA,
    default_appearance: DefaultAppearance,
    form_type: FormType,
    auto_recalculate: bool,
    auto_restore_form: bool,
    ignore_needs_rendering: bool,
    remove_permission: bool,
    emulate_required_groups: bool,
    signatures_exist: bool,
    signatures_append_only: bool,
    /// Set when a field is added / removed, so `Document::save` knows
    /// to (re)write the catalog's `/AcroForm`.
    dirty: bool,
    /// Set by [`flatten`](Form::flatten): the save removes the catalog
    /// entry instead of writing a form with no fields.
    flattened: bool,
}

impl std::fmt::Debug for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Form")
            .field("fields", &self.fields.len())
            .field("form_type", &self.form_type)
            .finish()
    }
}

impl Default for Form {
    /// An empty form. The canonical type has no public constructor;
    /// the canonical entry point is the document's form accessor, and
    /// this `Default` is the idiomatic Rust stand-in (mirroring the
    /// other AcroForm collection types).
    fn default() -> Self {
        Self {
            fields: Vec::new(),
            xfa: XFA::new(),
            default_appearance: DefaultAppearance::new(),
            form_type: FormType::Standard,
            auto_recalculate: true,
            auto_restore_form: false,
            ignore_needs_rendering: false,
            remove_permission: false,
            emulate_required_groups: false,
            signatures_exist: false,
            signatures_append_only: false,
            dirty: false,
            flattened: false,
        }
    }
}

impl Form {
    /// Add a field to the form.
    pub fn add(&mut self, field: Box<dyn Field>) {
        self.fields.push(field);
        self.dirty = true;
    }

    /// Add a field, hinting the page index it belongs to.
    pub fn add_at(&mut self, field: Box<dyn Field>, _page_number: i32) {
        self.fields.push(field);
        self.dirty = true;
    }

    /// Add a field under an explicit partial name and page index.
    pub fn add_named(&mut self, mut field: Box<dyn Field>, name: String, _page_number: i32) {
        field.set_partial_name(name);
        self.fields.push(field);
        self.dirty = true;
    }

    /// Remove every field with the given partial name.
    pub fn delete_by_name(&mut self, name: &str) {
        self.fields.retain(|f| f.partial_name() != name);
        self.dirty = true;
    }

    /// Append a field parsed from the document (does not mark the form
    /// dirty, so a verbatim resave keeps the original `/AcroForm`).
    pub(crate) fn add_loaded(&mut self, field: Box<dyn Field>) {
        self.fields.push(field);
    }

    /// Whether the form has been mutated since load.
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Flatten the form: its fields stop being interactive.
    ///
    /// This is a **structural** flatten. Every field is dropped and the
    /// save removes the catalog's `/AcroForm`, which §12.7.2 makes the
    /// document's form — without it there is no form left. What it does
    /// NOT do is burn a field's value into the page content, because
    /// that means drawing an appearance and this crate generates none;
    /// a widget annotation already in the file therefore stays as a
    /// non-interactive annotation rather than becoming page marks.
    /// [`FlattenSettings`](crate::FlattenSettings) steers exactly the
    /// part that is missing, and says so.
    pub fn flatten(&mut self) {
        self.fields.clear();
        self.flattened = true;
        self.dirty = true;
    }

    /// Whether [`flatten`](Self::flatten) was called, which the save
    /// path reads to remove the catalog entry rather than write a form
    /// with no fields in it.
    pub(crate) fn is_flattened(&self) -> bool {
        self.flattened
    }

    /// Make the form annotations on a page independent.
    ///
    /// Not implemented — pending `form_field_page_binding`. Calling
    /// this panics.
    pub fn make_form_annotations_independent(&mut self, _page: &Page) {
        PENDING("form_field_page_binding")
    }

    /// Whether a field with the given partial name exists.
    pub fn has_field_by_name(&self, name: &str) -> bool {
        self.fields.iter().any(|f| f.partial_name() == name)
    }

    /// The number of fields.
    pub fn count(&self) -> usize {
        self.fields.len()
    }

    /// The field at a 0-based index.
    pub fn get(&self, index: usize) -> std::option::Option<&dyn Field> {
        self.fields.get(index).map(|b| b.as_ref())
    }

    /// The field with the given partial name.
    pub fn get_by_name(&self, name: &str) -> std::option::Option<&dyn Field> {
        self.fields
            .iter()
            .find(|f| f.partial_name() == name)
            .map(|b| b.as_ref())
    }

    /// The field with the given partial name, mutably.
    /// Crate-internal: the canonical surface configures a field through
    /// [`FormEditor`](crate::FormEditor), which needs to reach it.
    /// Marks the form dirty, since the caller is about to change the
    /// field and the change has to reach the file.
    pub(crate) fn get_by_name_mut(&mut self, name: &str) -> std::option::Option<&mut dyn Field> {
        let found = self.fields.iter_mut().find(|f| f.partial_name() == name);
        match found {
            std::option::Option::Some(field) => {
                self.dirty = true;
                std::option::Option::Some(field.as_mut())
            }
            std::option::Option::None => std::option::Option::None,
        }
    }

    /// All fields.
    pub fn fields(&self) -> &[Box<dyn Field>] {
        &self.fields
    }

    /// The fields whose rectangle intersects `rect`.
    pub fn get_fields_in_rect(&self, rect: Rectangle) -> Vec<&dyn Field> {
        self.fields
            .iter()
            .filter(|f| f.rect().is_intersect(&rect))
            .map(|b| b.as_ref())
            .collect()
    }

    /// The XFA form data.
    pub fn xfa(&self) -> &XFA {
        &self.xfa
    }

    /// Whether the document has XFA data.
    pub fn has_xfa(&self) -> bool {
        !self.xfa.field_names().is_empty()
    }

    /// Whether any field needs its appearance regenerated.
    pub fn needs_rendering(&self) -> bool {
        false
    }

    /// The default text appearance for the form.
    pub fn default_appearance(&self) -> &DefaultAppearance {
        &self.default_appearance
    }
    /// Set the default text appearance.
    pub fn set_default_appearance(&mut self, value: DefaultAppearance) {
        self.default_appearance = value;
    }

    /// The form type (standard / static / dynamic XFA).
    pub fn form_type(&self) -> FormType {
        self.form_type
    }
    /// Set the form type.
    pub fn set_form_type(&mut self, value: FormType) {
        self.form_type = value;
    }

    /// Whether calculated fields auto-recalculate.
    pub fn auto_recalculate(&self) -> bool {
        self.auto_recalculate
    }
    /// Set whether calculated fields auto-recalculate.
    pub fn set_auto_recalculate(&mut self, value: bool) {
        self.auto_recalculate = value;
    }

    /// Whether the form is auto-restored on load.
    pub fn auto_restore_form(&self) -> bool {
        self.auto_restore_form
    }
    /// Set whether the form is auto-restored on load.
    pub fn set_auto_restore_form(&mut self, value: bool) {
        self.auto_restore_form = value;
    }

    /// Whether the needs-rendering flag is ignored.
    pub fn ignore_needs_rendering(&self) -> bool {
        self.ignore_needs_rendering
    }
    /// Set whether the needs-rendering flag is ignored.
    pub fn set_ignore_needs_rendering(&mut self, value: bool) {
        self.ignore_needs_rendering = value;
    }

    /// Whether the form removal permission is granted.
    pub fn remove_permission(&self) -> bool {
        self.remove_permission
    }
    /// Set the form removal permission.
    pub fn set_remove_permission(&mut self, value: bool) {
        self.remove_permission = value;
    }

    /// Whether required radio groups are emulated.
    pub fn emulate_requierd_groups(&self) -> bool {
        self.emulate_required_groups
    }
    /// Set whether required radio groups are emulated.
    pub fn set_emulate_requierd_groups(&mut self, value: bool) {
        self.emulate_required_groups = value;
    }

    /// Whether the form contains signatures.
    pub fn signatures_exist(&self) -> bool {
        self.signatures_exist
    }
    /// Set whether the form contains signatures.
    pub fn set_signatures_exist(&mut self, value: bool) {
        self.signatures_exist = value;
    }

    /// Whether signatures are append-only.
    pub fn signatures_append_only(&self) -> bool {
        self.signatures_append_only
    }
    /// Set whether signatures are append-only.
    pub fn set_signatures_append_only(&mut self, value: bool) {
        self.signatures_append_only = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::annotation::Annotation;
    use crate::public::document::Document;
    use crate::public::text_box_field::TextBoxField;

    #[test]
    fn add_query_and_delete_fields() {
        let doc = Document::new();
        let mut form = Form::default();
        assert_eq!(form.count(), 0);
        assert_eq!(form.form_type(), FormType::Standard);
        assert!(form.auto_recalculate());

        let mut a = TextBoxField::new(&doc);
        a.set_partial_name("name".into());
        form.add(Box::new(a));
        form.add_named(Box::new(TextBoxField::new(&doc)), "addr".into(), 1);

        assert_eq!(form.count(), 2);
        assert!(form.has_field_by_name("name"));
        assert!(form.has_field_by_name("addr"));
        assert!(!form.has_field_by_name("missing"));
        assert_eq!(form.get(0).unwrap().partial_name(), "name");
        assert!(form.get_by_name("addr").is_some());
        assert_eq!(form.fields().len(), 2);

        form.delete_by_name("name");
        assert_eq!(form.count(), 1);
        assert_eq!(form.get(0).unwrap().partial_name(), "addr");
    }

    #[test]
    fn fields_in_rect_and_props() {
        let doc = Document::new();
        let mut form = Form::default();
        form.set_form_type(FormType::Dynamic);
        form.set_signatures_exist(true);
        assert_eq!(form.form_type(), FormType::Dynamic);
        assert!(form.signatures_exist());
        assert!(!form.has_xfa());
        assert!(!form.needs_rendering());

        let mut t = TextBoxField::new(&doc);
        t.set_rect(Rectangle::new(0.0, 0.0, 100.0, 100.0, false));
        form.add(Box::new(t));
        let hits = form.get_fields_in_rect(Rectangle::new(50.0, 50.0, 150.0, 150.0, false));
        assert_eq!(hits.len(), 1);
    }
}
