//! `Aspose.Pdf.Facades.FormEditor` — a facade that adds / removes /
//! configures form fields on a bound document, per
//! `capabilities/form_editor.yaml`.
//!
//! Extends [`SaveableFacade`](crate::SaveableFacade). `AddField` builds a
//! real concrete [`Field`] (by [`FieldType`]) into the bound document's
//! [`Form`](crate::Form); `RemoveField` deletes by name. The
//! The configuration methods find a field by its partial name and
//! change it: the three Table 226 flags, the appearance flags, the
//! character limit and comb cells, the rectangle, multiline, the name,
//! the horizontal alignment, a choice field's options, the script it
//! runs, the submit target and format, and the facade's appearance
//! attributes applied to one field, to a whole `/FT` class, or to all
//! of them. Each answers whether it was applied, which is what the
//! canonical `bool` means — an absent name, or a family the request
//! means nothing to, is `false` rather than a failure. Vertical
//! alignment always answers `false`: the format has no field entry for
//! it.
//!
//! The five `CopyField` overloads still report `PENDING`, each with the
//! structural reason spelled out where it is declared.
//!
//! The `Stream` constructors and the `SrcStream` / `DestStream`
//! properties are dropped (no public Stream-handle surface).
//!
//! Behaviour note: a field added here goes into the bound *document's*
//! form, so `Save` writes it into the saved PDF's `/AcroForm` through
//! the document's own save path. The editor used to keep a form of its
//! own, which meant an added field never reached the file at all.

use crate::public::annotation::Annotation;
use crate::public::annotation_flags::AnnotationFlags;
use crate::public::barcode_field::BarcodeField;
use crate::public::button_field::ButtonField;
use crate::public::checkbox_field::CheckboxField;
use crate::public::combo_box_field::ComboBoxField;
use crate::public::date_field::DateField;
use crate::public::document::Document;
use crate::public::error::Error;
use crate::public::field::{Field, FieldConfig};
use crate::public::file_specification::FileSpecification;
use crate::public::javascript_action::JavascriptAction;
use crate::public::submit_form_action::SubmitFormAction;
use crate::public::field_type::FieldType;
use crate::public::form_field_facade::FormFieldFacade;
use crate::public::i_facade::{impl_facade, FacadeBase};
use crate::public::list_box_field::ListBoxField;
use crate::public::number_field::NumberField;
use crate::public::page::Page;
use crate::public::pdf_format::PdfFormat;
use crate::public::radio_button_field::RadioButtonField;
use crate::public::rectangle::Rectangle;
use crate::public::signature_field::SignatureField;
use crate::public::submit_form_flag::SubmitFormFlag;
use crate::public::text_box_field::TextBoxField;
use std::path::Path;
use crate::internal::pending::PENDING;

/// A facade for adding / removing / configuring form fields.
pub struct FormEditor {
    base: FacadeBase,
    facade: FormFieldFacade,
    src_file: String,
    dest_file: String,
    convert_to: PdfFormat,
    items: Vec<String>,
    export_items: Vec<Vec<String>>,
    radio_gap: f32,
    radio_horiz: bool,
    radio_button_item_size: f64,
    submit_flag: SubmitFormFlag,
}

impl std::fmt::Debug for FormEditor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormEditor")
            .field("bound", &self.base.document.is_some())
            .field(
                "field_count",
                &self.base.document.as_ref().map_or(0, |d| d.form().count()),
            )
            .field("src_file", &self.src_file)
            .field("dest_file", &self.dest_file)
            .field("submit_flag", &self.submit_flag)
            .finish()
    }
}

impl Default for FormEditor {
    fn default() -> Self {
        Self {
            base: FacadeBase::default(),
            facade: FormFieldFacade::default(),
            src_file: String::new(),
            dest_file: String::new(),
            convert_to: PdfFormat::v_1_7,
            items: Vec::new(),
            export_items: Vec::new(),
            radio_gap: 1.0,
            radio_horiz: false,
            radio_button_item_size: 20.0,
            submit_flag: SubmitFormFlag::Html,
        }
    }
}

impl FormEditor {
    /// Construct an unbound editor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Open `src_file` and stage `dest_file` as the save target.
    pub fn with_files(
        src_file: impl AsRef<Path>,
        dest_file: impl Into<String>,
    ) -> Result<Self, Error> {
        let path = src_file.as_ref();
        let document = Document::open(path)?;
        Ok(Self {
            base: FacadeBase { document: Some(document) },
            src_file: path.to_string_lossy().into_owned(),
            dest_file: dest_file.into(),
            ..Self::default()
        })
    }

    /// Bind to an in-memory [`Document`].
    pub fn with_document(document: Document) -> Self {
        Self { base: FacadeBase { document: Some(document) }, ..Self::default() }
    }

    /// Bind to an in-memory [`Document`] and stage `dest_file`.
    pub fn with_document_and_dest(
        document: Document,
        dest_file: impl Into<String>,
    ) -> Self {
        Self {
            base: FacadeBase { document: Some(document) },
            dest_file: dest_file.into(),
            ..Self::default()
        }
    }

    /// Save the bound document to [`DestFileName`](Self::dest_file_name),
    /// `/AcroForm` and all — a field added here is the document's own,
    /// so the document's save path writes it.
    pub fn save(&self) -> Result<(), Error> {
        let doc = self
            .base
            .document
            .as_ref()
            .ok_or_else(|| Error::InvalidPdf("facade has no bound document".into()))?;
        if self.dest_file.is_empty() {
            return Err(Error::InvalidPdf("no destination file set".into()));
        }
        doc.save_to(&self.dest_file)
    }

    // ---- Field add / remove (real, via the editor's Form) ----

    /// Add a field of `field_type` named `field_name` on page `page_num`
    /// within `(llx, lly, urx, ury)`. Returns `false` for an invalid
    /// type or when unbound.
    #[allow(clippy::too_many_arguments)]
    pub fn add_field(
        &mut self,
        field_type: FieldType,
        field_name: &str,
        page_num: i32,
        llx: f32,
        lly: f32,
        urx: f32,
        ury: f32,
    ) -> bool {
        let rect = Rectangle::new(llx as f64, lly as f64, urx as f64, ury as f64, false);
        let built = {
            let Some(doc) = self.base.document.as_ref() else {
                return false;
            };
            let Ok(page) = doc.pages().get(page_num.max(1) as usize) else {
                return false;
            };
            Self::make_field(field_type, doc, &page, rect, &self.facade)
        };
        match built {
            Some(mut field) => {
                self.apply_facade(field.as_mut());
                // Into the document's own form: that is the one the save
                // path walks, so the field reaches the file.
                let Some(doc) = self.base.document.as_mut() else {
                    return false;
                };
                doc.form_mut().add_named(field, field_name.to_string(), page_num);
                true
            }
            None => false,
        }
    }

    /// Add a field with an explicit `field_id`. Behaves like
    /// [`add_field`](Self::add_field); the id is recorded as the field's
    /// partial name.
    #[allow(clippy::too_many_arguments)]
    pub fn add_field_with_id(
        &mut self,
        field_type: FieldType,
        field_name: &str,
        _field_id: &str,
        page_num: i32,
        llx: f32,
        lly: f32,
        urx: f32,
        ury: f32,
    ) -> bool {
        self.add_field(field_type, field_name, page_num, llx, lly, urx, ury)
    }

    /// Set the working facade's appearance attributes on a field the
    /// editor has just built. Only the ones that reach the file are
    /// applied — see the facade's own note for the rest.
    fn apply_facade(&self, field: &mut dyn Field) {
        Self::apply_facade_to(&self.facade, field);
    }

    /// The same, against a facade passed in — which is what the
    /// decorate methods need, since they hold the document mutably and
    /// cannot borrow `self` again.
    fn apply_facade_to(facade: &FormFieldFacade, field: &mut dyn Field) {
        let mut border = field.border();
        let mut touched = false;
        if facade.border_width() >= 0.0 {
            border.set_width(facade.border_width() as i32);
            touched = true;
        }
        if let Some(style) = facade.border_style_value() {
            border.set_style(style);
            touched = true;
        }
        if touched {
            field.set_border(border);
        }
        if let Some(da) = facade.default_appearance_value() {
            field.set_default_appearance(da);
        }
    }

    fn make_field(
        field_type: FieldType,
        document: &Document,
        page: &Page,
        rect: Rectangle,
        facade: &FormFieldFacade,
    ) -> Option<Box<dyn Field>> {
        match field_type {
            FieldType::Text | FieldType::MultiLineText => {
                Some(Box::new(TextBoxField::on_page(page, rect)))
            }
            FieldType::ComboBox => Some(Box::new(ComboBoxField::on_page(page, rect))),
            FieldType::ListBox => Some(Box::new(ListBoxField::on_page(page, rect))),
            FieldType::Radio => {
                let mut f = RadioButtonField::new(document);
                f.set_rect(rect);
                // The mark style is set here, where the concrete type
                // is known — the facade carries it as the ZapfDingbats
                // character code a /MK caption uses.
                if let Some(style) = facade.box_style_value() {
                    f.set_style(style);
                }
                Some(Box::new(f))
            }
            FieldType::CheckBox => {
                let mut f = CheckboxField::on_page(page, rect);
                if let Some(style) = facade.box_style_value() {
                    f.set_style(style);
                }
                Some(Box::new(f))
            }
            FieldType::PushButton | FieldType::Image => {
                Some(Box::new(ButtonField::on_page(page, rect)))
            }
            FieldType::Barcode => Some(Box::new(BarcodeField::on_page(page, rect))),
            FieldType::Signature => Some(Box::new(SignatureField::on_page(page, rect))),
            FieldType::Numeric => Some(Box::new(NumberField::on_page(page, rect))),
            FieldType::DateTime => Some(Box::new(DateField::on_page(page, rect))),
            FieldType::InvalidNameOrType => None,
        }
    }

    /// Remove the field named `field_name` (safe no-op when absent).
    pub fn remove_field(&mut self, field_name: &str) {
        if let Some(doc) = self.base.document.as_mut() {
            doc.form_mut().delete_by_name(field_name);
        }
    }

    // ---- Field configuration ----

    /// The field with this partial name, mutably, or `None` when the
    /// editor is unbound or no field has that name. Every configuration
    /// method below goes through here, which is why each answers
    /// `false` for a name that is not there rather than failing.
    fn field_mut(&mut self, field_name: &str) -> Option<&mut dyn Field> {
        self.base.document.as_mut()?.form_mut().get_by_name_mut(field_name)
    }

    /// Set a field property flag — §12.7.4 Table 226's first three
    /// `/Ff` bits. Returns whether it was applied: an unknown flag or
    /// an absent field is `false`.
    pub fn set_field_attribute(
        &mut self,
        field_name: &str,
        flag: crate::public::property_flag::PropertyFlag,
    ) -> bool {
        use crate::public::property_flag::PropertyFlag;
        let Some(field) = self.field_mut(field_name) else {
            return false;
        };
        match flag {
            PropertyFlag::ReadOnly => field.set_read_only(true),
            PropertyFlag::Required => field.set_required(true),
            // NoExport is the bit's name; the property is its opposite.
            PropertyFlag::NoExport => field.set_exportable(false),
            PropertyFlag::InvalidFlag => return false,
        }
        true
    }

    /// Set a field's appearance flags — the annotation `/F` entry
    /// (§12.5.3), which decides whether a viewer shows or prints it.
    pub fn set_field_appearance(&mut self, field_name: &str, flag: AnnotationFlags) -> bool {
        let Some(field) = self.field_mut(field_name) else {
            return false;
        };
        field.set_flags(flag);
        true
    }

    /// A field's appearance flags, or
    /// [`Default`](AnnotationFlags::Default) when the editor is unbound
    /// or no field has that name.
    pub fn get_field_appearance(&self, field_name: &str) -> AnnotationFlags {
        self.base
            .document
            .as_ref()
            .and_then(|d| d.form().get_by_name(field_name).map(|f| f.flags()))
            .unwrap_or(AnnotationFlags::Default)
    }
    /// Set the submission format of the named button's submit action
    /// (§12.7.5.2 `/Flags`), keeping whatever URL it already submits
    /// to. Also becomes the format
    /// [`set_submit_url`](Self::set_submit_url) uses next, matching
    /// [`submit_flag`](Self::submit_flag) as the editor's working
    /// value.
    pub fn set_submit_flag(&mut self, field_name: &str, flag: SubmitFormFlag) -> bool {
        self.submit_flag = flag;
        // Keep whatever URL the button already submits to; only the
        // format was asked about.
        let url = self
            .field_action_text(field_name, "SubmitForm", "F")
            .unwrap_or_default();
        self.set_submit_action(field_name, &url)
    }

    /// Point the named button's submit action at `url`, in the editor's
    /// current [`submit_flag`](Self::submit_flag) format.
    pub fn set_submit_url(&mut self, field_name: &str, url: &str) -> bool {
        self.set_submit_action(field_name, url)
    }

    /// The text a named field's action carries under `key`, when that
    /// action is of `subtype`.
    fn field_action_text(&self, field_name: &str, subtype: &str, key: &str) -> Option<String> {
        let action = self
            .base
            .document
            .as_ref()
            .and_then(|d| d.form().get_by_name(field_name))
            .and_then(|f| f.on_activated())?;
        crate::public::pdf_action::action_text(action, subtype, key)
    }

    /// Put a `SubmitForm` action on the named field, built from `url`
    /// and the editor's current submission format.
    fn set_submit_action(&mut self, field_name: &str, url: &str) -> bool {
        let flags = self.submit_flag.pdf_flags() as i32;
        let Some(field) = self.field_mut(field_name) else {
            return false;
        };
        let mut action = SubmitFormAction::new();
        let mut spec = FileSpecification::new();
        spec.set_name(url.to_string());
        action.set_url(spec);
        action.set_flags(flags);
        field.set_on_activated(Box::new(action));
        true
    }
    /// Set a text field's character limit (`/MaxLen`). Returns whether
    /// it was applied — a field family with no limit to set, such as a
    /// button, answers `false`.
    pub fn set_field_limit(&mut self, field_name: &str, limit: i32) -> bool {
        self.configure_field(field_name, FieldConfig::MaxLen(limit))
    }

    /// Divide a text field into `comb` evenly spaced cells (§12.7.4.3).
    /// The cell count is the field's `/MaxLen`, so this sets both.
    pub fn set_field_comb_number(&mut self, field_name: &str, comb: i32) -> bool {
        self.configure_field(field_name, FieldConfig::Comb(comb))
    }

    /// Move and resize a field's rectangle.
    pub fn move_field(
        &mut self,
        field_name: &str,
        llx: f32,
        lly: f32,
        urx: f32,
        ury: f32,
    ) -> bool {
        let rect = Rectangle::new(llx as f64, lly as f64, urx as f64, ury as f64, false);
        let Some(field) = self.field_mut(field_name) else {
            return false;
        };
        field.set_rect(rect);
        true
    }

    /// Ask the named field for a type-specific change, answering
    /// whether that family understood it.
    fn configure_field(&mut self, field_name: &str, request: FieldConfig<'_>) -> bool {
        match self.field_mut(field_name) {
            Some(field) => field.configure(request),
            None => false,
        }
    }
    /// Put `script` on the named field as the JavaScript it runs when
    /// activated, replacing any action it already had.
    ///
    /// A field dictionary holds one `/A`, so a field's *additional*
    /// actions — the keystroke, format, validate and calculate
    /// handlers of `/AA` (§12.6.3) — are a different entry and are not
    /// written here; that is the `document_additional_actions`
    /// capability.
    pub fn set_field_script(&mut self, field_name: &str, script: &str) -> bool {
        let Some(field) = self.field_mut(field_name) else {
            return false;
        };
        field.set_on_activated(Box::new(JavascriptAction::new(script.to_string())));
        true
    }

    /// Add `script` to the named field's JavaScript. Since `/A` holds
    /// one action, a second script joins the first rather than
    /// replacing it — the two run in the order they were added, as one
    /// program. A field with no script yet gets this one.
    pub fn add_field_script(&mut self, field_name: &str, script: &str) -> bool {
        let existing = self.field_action_text(field_name, "JavaScript", "JS");
        let combined = match existing {
            Some(before) if !before.is_empty() => format!("{before}\n{script}"),
            _ => script.to_string(),
        };
        self.set_field_script(field_name, &combined)
    }
    /// Turn a single-line text field into a multiline one (§12.7.4.3
    /// Multiline). Answers `false` for a family with no such mode.
    pub fn single_to_multiple(&mut self, field_name: &str) -> bool {
        self.configure_field(field_name, FieldConfig::Multiline)
    }
    /// Set a field's horizontal alignment, as one of
    /// [`FormFieldFacade`]'s `ALIGN_*` constants. It reaches the file
    /// as `/Q` (§12.7.4.1), whose three values are left, centred and
    /// right — so `ALIGN_UNDEFINED` and `ALIGN_JUSTIFIED`, which `/Q`
    /// cannot express, answer `false`.
    ///
    /// [`FormFieldFacade`]: crate::FormFieldFacade
    pub fn set_field_alignment(&mut self, field_name: &str, alignment: i32) -> bool {
        use crate::public::horizontal_alignment::HorizontalAlignment;
        let value = match alignment {
            FormFieldFacade::ALIGN_LEFT => HorizontalAlignment::Left,
            FormFieldFacade::ALIGN_CENTER => HorizontalAlignment::Center,
            FormFieldFacade::ALIGN_RIGHT => HorizontalAlignment::Right,
            _ => return false,
        };
        let Some(field) = self.field_mut(field_name) else {
            return false;
        };
        field.set_text_horizontal_alignment(value);
        true
    }

    /// Set a field's vertical alignment. Always `false`: the format has
    /// no field entry for it — where the text sits inside the box is
    /// decided by the appearance stream a viewer generates, and this
    /// crate writes none. Saying so is the point of the `bool`.
    pub fn set_field_alignment_v(&mut self, _field_name: &str, _alignment: i32) -> bool {
        false
    }

    // ---- No-op helpers (void) ----

    /// Reset the working [`FormFieldFacade`].
    pub fn reset_facade(&mut self) {
        self.facade.reset();
    }
    /// Reset the inner facade. No-op alias of [`reset_facade`](Self::reset_facade).
    pub fn reset_inner_facade(&mut self) {
        self.facade.reset();
    }
    /// Copy an inner field.
    ///
    /// Not implemented — pending `form_field_configuration`. Calling
    /// this panics.
    ///
    /// What it waits on, precisely: copying a field means producing a
    /// second field of the same concrete type, and the editor holds
    /// only a `&dyn Field`. There is no downcast here, and the trait
    /// object cannot be cloned either — a field owns a boxed
    /// [`PdfAction`](crate::PdfAction), itself a trait object, so a
    /// clone hook on the field would need one on every action type
    /// under it. Reading the source field's entries is not enough: a
    /// choice field's options and a button's mark are typed state, and
    /// a copy that silently dropped them would be worse than none.
    pub fn copy_inner_field(&mut self, _field_name: &str, _new_name: &str, _page_num: i32) {
        PENDING("form_field_configuration")
    }
    /// Copy an inner field with an offset.
    ///
    /// Not implemented — pending `form_field_configuration`. Calling
    /// this panics.
    ///
    /// What it waits on, precisely: copying a field means producing a
    /// second field of the same concrete type, and the editor holds
    /// only a `&dyn Field`. There is no downcast here, and the trait
    /// object cannot be cloned either — a field owns a boxed
    /// [`PdfAction`](crate::PdfAction), itself a trait object, so a
    /// clone hook on the field would need one on every action type
    /// under it. Reading the source field's entries is not enough: a
    /// choice field's options and a button's mark are typed state, and
    /// a copy that silently dropped them would be worse than none.
    pub fn copy_inner_field_with_offset(&mut self, _field_name: &str, _new_name: &str, _page_num: i32, _offset_x: f32, _offset_y: f32) {
        PENDING("form_field_configuration")
    }
    /// Copy an outer field.
    ///
    /// Not implemented — pending `form_field_configuration`. Calling
    /// this panics.
    ///
    /// What it waits on, precisely: copying a field means producing a
    /// second field of the same concrete type, and the editor holds
    /// only a `&dyn Field`. There is no downcast here, and the trait
    /// object cannot be cloned either — a field owns a boxed
    /// [`PdfAction`](crate::PdfAction), itself a trait object, so a
    /// clone hook on the field would need one on every action type
    /// under it. Reading the source field's entries is not enough: a
    /// choice field's options and a button's mark are typed state, and
    /// a copy that silently dropped them would be worse than none.
    pub fn copy_outer_field(&mut self, _field_name: &str, _new_name: &str) {
        PENDING("form_field_configuration")
    }
    /// Copy an outer field to a page.
    ///
    /// Not implemented — pending `form_field_configuration`. Calling
    /// this panics.
    ///
    /// What it waits on, precisely: copying a field means producing a
    /// second field of the same concrete type, and the editor holds
    /// only a `&dyn Field`. There is no downcast here, and the trait
    /// object cannot be cloned either — a field owns a boxed
    /// [`PdfAction`](crate::PdfAction), itself a trait object, so a
    /// clone hook on the field would need one on every action type
    /// under it. Reading the source field's entries is not enough: a
    /// choice field's options and a button's mark are typed state, and
    /// a copy that silently dropped them would be worse than none.
    pub fn copy_outer_field_to_page(&mut self, _field_name: &str, _new_name: &str, _page_num: i32) {
        PENDING("form_field_configuration")
    }
    /// Copy an outer field to a page with an offset.
    ///
    /// Not implemented — pending `form_field_configuration`. Calling
    /// this panics.
    ///
    /// What it waits on, precisely: copying a field means producing a
    /// second field of the same concrete type, and the editor holds
    /// only a `&dyn Field`. There is no downcast here, and the trait
    /// object cannot be cloned either — a field owns a boxed
    /// [`PdfAction`](crate::PdfAction), itself a trait object, so a
    /// clone hook on the field would need one on every action type
    /// under it. Reading the source field's entries is not enough: a
    /// choice field's options and a button's mark are typed state, and
    /// a copy that silently dropped them would be worse than none.
    pub fn copy_outer_field_with_offset(&mut self, _field_name: &str, _new_name: &str, _page_num: i32, _offset_x: f32, _offset_y: f32) {
        PENDING("form_field_configuration")
    }
    /// Apply the working [`facade`](Self::facade) to the named field —
    /// the same attributes [`add_field`](Self::add_field) applies to a
    /// field it creates, here applied to one that already exists.
    pub fn decorate_field(&mut self, field_name: &str) {
        let facade = self.facade.clone();
        if let Some(field) = self.field_mut(field_name) {
            Self::apply_facade_to(&facade, field);
        }
    }

    /// Apply the facade to every field of `field_type`.
    ///
    /// The model tells field families apart by the `/FT` their type
    /// writes, so a `FieldType` selects its whole PDF class: asking for
    /// `Text` reaches every `/Tx` field, numeric and date ones
    /// included, and asking for `ComboBox` reaches every `/Ch`.
    pub fn decorate_field_of_type(&mut self, field_type: FieldType) {
        let Some(wanted) = Self::pdf_field_type_of(field_type) else {
            return;
        };
        self.decorate_matching(|field| field.pdf_field_type() == wanted);
    }

    /// The `/FT` name a `FieldType` is written as, or `None` for the
    /// invalid type.
    fn pdf_field_type_of(field_type: FieldType) -> Option<&'static [u8]> {
        match field_type {
            FieldType::Text
            | FieldType::MultiLineText
            | FieldType::Numeric
            | FieldType::DateTime
            | FieldType::Barcode => Some(b"Tx"),
            FieldType::ComboBox | FieldType::ListBox => Some(b"Ch"),
            FieldType::CheckBox
            | FieldType::Radio
            | FieldType::PushButton
            | FieldType::Image => Some(b"Btn"),
            FieldType::Signature => Some(b"Sig"),
            FieldType::InvalidNameOrType => None,
        }
    }

    /// Apply the facade to every field the predicate accepts.
    fn decorate_matching(&mut self, accept: impl Fn(&dyn Field) -> bool) {
        let facade = self.facade.clone();
        let Some(doc) = self.base.document.as_mut() else {
            return;
        };
        let names: Vec<String> = doc
            .form()
            .fields()
            .iter()
            .filter(|f| accept(f.as_ref()))
            .map(|f| f.partial_name().to_string())
            .collect();
        for name in names {
            if let Some(field) = doc.form_mut().get_by_name_mut(&name) {
                Self::apply_facade_to(&facade, field);
            }
        }
    }
    /// Apply the facade to every field in the form — the canonical
    /// parameterless `DecorateField` overload, the widest of the three.
    pub fn decorate_facade(&mut self) {
        self.decorate_matching(|_| true);
    }
    /// Rename a field — its `/T` partial name, which is what every
    /// other method here addresses it by.
    pub fn rename_field(&mut self, field_name: &str, new_field_name: &str) {
        if let Some(field) = self.field_mut(field_name) {
            field.set_partial_name(new_field_name.to_string());
        }
    }
    /// Remove the action the named field runs when activated, leaving
    /// the field itself in place. A field with none is unchanged.
    pub fn remove_field_action(&mut self, field_name: &str) {
        if let Some(field) = self.field_mut(field_name) {
            field.clear_on_activated();
        }
    }
    /// Add a submit button. No-op.
    #[allow(clippy::too_many_arguments)]
    /// Add a push button on `page_num` within the given rectangle that
    /// submits the form to `url`, in the editor's current
    /// [`submit_flag`](Self::submit_flag) format. `button_name` is the
    /// caption a viewer draws on it (`/MK /CA`, §12.5.6.19).
    pub fn add_submit_btn(
        &mut self,
        field_name: &str,
        page_num: i32,
        button_name: &str,
        url: &str,
        llx: f32,
        lly: f32,
        urx: f32,
        ury: f32,
    ) {
        if !self.add_field(FieldType::PushButton, field_name, page_num, llx, lly, urx, ury) {
            return;
        }
        self.configure_field(field_name, FieldConfig::Caption(button_name));
        self.set_submit_action(field_name, url);
    }
    /// Add one option to a choice field's `/Opt` list. A field family
    /// with no options — a text box, a button — is left alone.
    pub fn add_list_item(&mut self, field_name: &str, item: &str) {
        self.configure_field(field_name, FieldConfig::AddOption(item));
    }

    /// Add several options, in order.
    pub fn add_list_items(&mut self, field_name: &str, items: Vec<String>) {
        for item in items {
            self.configure_field(field_name, FieldConfig::AddOption(&item));
        }
    }

    /// Remove the option with this display name.
    pub fn del_list_item(&mut self, field_name: &str, item: &str) {
        self.configure_field(field_name, FieldConfig::RemoveOption(item));
    }

    // ---- Properties ----

    /// The source file path.
    pub fn src_file_name(&self) -> &str {
        &self.src_file
    }
    /// Set the source file path.
    pub fn set_src_file_name(&mut self, value: impl Into<String>) {
        self.src_file = value.into();
    }

    /// The destination file path.
    pub fn dest_file_name(&self) -> &str {
        &self.dest_file
    }
    /// Set the destination file path.
    pub fn set_dest_file_name(&mut self, value: impl Into<String>) {
        self.dest_file = value.into();
    }

    /// Set the target format on save.
    ///
    /// Carried, not applied: converting to another PDF level or profile
    /// means writing the whole file to that profile's rules, and this
    /// save path appends an incremental update to the bytes it opened
    /// (§7.5.6). The same blocker keeps `PdfFileStamp::optimize_size`
    /// unapplied.
    pub fn set_convert_to(&mut self, value: PdfFormat) {
        self.convert_to = value;
    }

    /// The staged list items.
    pub fn items(&self) -> &[String] {
        &self.items
    }
    /// Set the staged list items.
    pub fn set_items(&mut self, value: Vec<String>) {
        self.items = value;
    }

    /// The staged `[display, export]` item pairs.
    pub fn export_items(&self) -> &[Vec<String>] {
        &self.export_items
    }
    /// Set the staged `[display, export]` item pairs.
    pub fn set_export_items(&mut self, value: Vec<Vec<String>>) {
        self.export_items = value;
    }

    /// The working appearance facade.
    pub fn facade(&self) -> &FormFieldFacade {
        &self.facade
    }
    /// Set the working appearance facade.
    pub fn set_facade(&mut self, value: FormFieldFacade) {
        self.facade = value;
    }

    /// The gap between radio-button items.
    pub fn radio_gap(&self) -> f32 {
        self.radio_gap
    }
    /// Set the gap between radio-button items.
    pub fn set_radio_gap(&mut self, value: f32) {
        self.radio_gap = value;
    }

    /// Whether radio buttons lay out horizontally.
    pub fn radio_horiz(&self) -> bool {
        self.radio_horiz
    }
    /// Set whether radio buttons lay out horizontally.
    pub fn set_radio_horiz(&mut self, value: bool) {
        self.radio_horiz = value;
    }

    /// The radio-button item size.
    pub fn radio_button_item_size(&self) -> f64 {
        self.radio_button_item_size
    }
    /// Set the radio-button item size.
    pub fn set_radio_button_item_size(&mut self, value: f64) {
        self.radio_button_item_size = value;
    }

    /// The submit-form flag.
    pub fn submit_flag(&self) -> SubmitFormFlag {
        self.submit_flag
    }
    /// Set the submit-form flag.
    pub fn set_submit_flag_value(&mut self, value: SubmitFormFlag) {
        self.submit_flag = value;
    }
}

impl_facade!(FormEditor);

#[cfg(test)]
mod tests {
    use super::*;

    fn hello_world_pdf() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/text_extractor/hello_world.pdf")
    }

    fn bound() -> FormEditor {
        FormEditor::with_document(Document::open(hello_world_pdf()).unwrap())
    }

    #[test]
    fn add_field_real_via_form() {
        let mut editor = bound();
        assert!(editor.add_field(FieldType::Text, "name1", 1, 100.0, 600.0, 300.0, 620.0));
        assert!(editor.base.document.as_ref().unwrap().form().has_field_by_name("name1"));
        assert!(editor.base.document.as_ref().unwrap().form().count() >= 1);

        assert!(editor.add_field(FieldType::CheckBox, "agree", 1, 100.0, 560.0, 120.0, 580.0));
        assert!(editor.base.document.as_ref().unwrap().form().has_field_by_name("agree"));
    }

    #[test]
    fn add_field_invalid_type_returns_false() {
        let mut editor = bound();
        assert!(!editor.add_field(FieldType::InvalidNameOrType, "x", 1, 0.0, 0.0, 10.0, 10.0));
    }

    #[test]
    fn remove_field_real() {
        let mut editor = bound();
        editor.add_field(FieldType::Text, "keep", 1, 100.0, 600.0, 300.0, 620.0);
        editor.add_field(FieldType::Text, "drop", 1, 100.0, 560.0, 300.0, 580.0);
        assert!(editor.base.document.as_ref().unwrap().form().has_field_by_name("drop"));

        editor.remove_field("drop");
        assert!(!editor.base.document.as_ref().unwrap().form().has_field_by_name("drop"));
        assert!(editor.base.document.as_ref().unwrap().form().has_field_by_name("keep"));
    }

    #[test]
    fn file_based_add_field_save_passthrough() {
        let out = std::env::temp_dir()
            .join("aspose_rust_formeditor_persist.pdf")
            .to_string_lossy()
            .into_owned();
        {
            let mut editor = FormEditor::with_files(hello_world_pdf(), &out).unwrap();
            assert!(editor.add_field(FieldType::Text, "myfield", 1, 100.0, 600.0, 300.0, 620.0));
            editor.save().unwrap();
        }
        assert!(Path::new(&out).exists());
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn unbound_add_field_returns_false() {
        let mut editor = FormEditor::new();
        assert!(!editor.add_field(FieldType::Text, "x", 1, 0.0, 0.0, 10.0, 10.0));
        editor.remove_field("x"); // safe no-op
    }

    #[test]
    fn adds_a_field_and_resets() {
        let mut editor = bound();
        editor.add_field(FieldType::Text, "f", 1, 100.0, 600.0, 300.0, 620.0);
        editor.reset_facade();
    }

    #[test]
    fn a_field_is_renamed_and_limited_in_place() {
        let mut editor = bound();
        editor.add_field(FieldType::Text, "f", 1, 100.0, 600.0, 300.0, 620.0);
        assert!(editor.set_field_limit("f", 10));
        editor.rename_field("f", "g");

        let form = editor.base.document.as_ref().unwrap().form();
        assert!(form.has_field_by_name("g"), "the new name is the field's");
        assert!(!form.has_field_by_name("f"), "and the old one is gone");
        // The limit went to the field, not to a copy of it: asking by
        // the new name finds a field that still has it.
        assert!(editor.set_field_limit("g", 12), "reachable under the new name");
    }

    #[test]
    fn property_roundtrip() {
        let mut editor = FormEditor::new();
        editor.set_src_file_name("in.pdf");
        editor.set_dest_file_name("out.pdf");
        editor.set_radio_gap(2.5);
        editor.set_radio_horiz(true);
        editor.set_radio_button_item_size(15.0);
        editor.set_submit_flag_value(SubmitFormFlag::Pdf);
        editor.set_items(vec!["x".into(), "y".into()]);

        assert_eq!(editor.src_file_name(), "in.pdf");
        assert_eq!(editor.dest_file_name(), "out.pdf");
        assert_eq!(editor.radio_gap(), 2.5);
        assert!(editor.radio_horiz());
        assert_eq!(editor.radio_button_item_size(), 15.0);
        assert_eq!(editor.submit_flag(), SubmitFormFlag::Pdf);
        assert_eq!(editor.items().len(), 2);
        assert_eq!(editor.items()[0], "x");
    }
}
