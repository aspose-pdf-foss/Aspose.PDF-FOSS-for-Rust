//! `Aspose.Pdf.Forms.Field` — the abstract base of every form
//! field, per `capabilities/field.yaml`.
//!
//! A field **is a** [`WidgetAnnotation`]
//! (it extends it) **and** owns a collection of widget kids.
//! Modelled as the [`Field`] trait (extending
//! [`Annotation`] plus the crate-private widget /
//! field base accessors); concrete fields embed an
//! `AnnotationBase`, a `WidgetBase`, and a `FieldBase` and
//! implement the trait chain. The abstract `.ctor`, the .NET
//! collection infrastructure (`CopyTo` / `GetEnumerator` /
//! `IsSynchronized` / `SyncRoot`), and the JSON import/export
//! helpers are dropped (see `targets/rust/drops.yaml`).
//!
//! **Carried, not written.** `/T`, `/TU`, `/TM`, `/V` and `/Ff` reach
//! the file; six of the base properties have no field-dictionary entry
//! to reach:
//!
//! - [`fit_into_rectangle`](Field::fit_into_rectangle),
//!   [`max_font_size`](Field::max_font_size) and
//!   [`min_font_size`](Field::min_font_size) steer the auto-sized
//!   appearance stream a viewer builds. This crate writes
//!   `/NeedAppearances true` and lets the viewer build it, so the three
//!   bounds have nothing to be written into.
//! - [`tab_order`](Field::tab_order) is a page-level ordering (the page
//!   `/Tabs` entry plus the order of `/Annots`), not a field entry.
//! - [`annotation_index`](Field::annotation_index) and
//!   [`is_shared_field`](Field::is_shared_field) describe a field split
//!   across several widget kids; a field is written as one merged
//!   field / widget object (§12.7.3.1), so neither has a place in it.

use crate::public::annotation::Annotation;
use crate::public::default_appearance::DefaultAppearance;
use crate::public::highlighting_mode::HighlightingMode;
use crate::public::javascript_action::JavascriptAction;
use crate::public::pdf_action::PdfAction;
use crate::public::point::Point;
use crate::public::widget_annotation::sealed::HasWidgetBase;
use crate::public::widget_annotation::WidgetAnnotation;

/// Shared form-field fields. Crate-internal — reused across the
/// `Field` hierarchy.
pub(crate) struct FieldBase {
    pub(crate) partial_name: String,
    pub(crate) alternate_name: String,
    pub(crate) mapping_name: String,
    pub(crate) value: String,
    pub(crate) annotation_index: i32,
    pub(crate) is_shared_field: bool,
    pub(crate) fit_into_rectangle: bool,
    pub(crate) max_font_size: f64,
    pub(crate) min_font_size: f64,
    pub(crate) tab_order: i32,
    pub(crate) kids: Vec<WidgetAnnotation>,
}

impl FieldBase {
    pub(crate) fn new() -> Self {
        Self {
            partial_name: String::new(),
            alternate_name: String::new(),
            mapping_name: String::new(),
            value: String::new(),
            annotation_index: 0,
            is_shared_field: false,
            fit_into_rectangle: false,
            max_font_size: 0.0,
            min_font_size: 0.0,
            tab_order: 0,
            kids: Vec::new(),
        }
    }
}

impl Default for FieldBase {
    fn default() -> Self {
        Self::new()
    }
}

/// One type-specific change a [`FormEditor`](crate::FormEditor) can ask
/// of a field it has found by name. The editor holds a `&mut dyn Field`
/// and cannot know the concrete type, so each field family answers for
/// the requests it understands and declines the rest — which is exactly
/// the `bool` the canonical configuration methods return.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum FieldConfig<'a> {
    /// `/MaxLen` — the most characters the field accepts.
    MaxLen(i32),
    /// Comb formatting over `cells` evenly spaced cells, which is
    /// `/MaxLen` plus the Comb flag (§12.7.4.3).
    Comb(i32),
    /// Turn a single-line text field into a multiline one.
    Multiline,
    /// Append an option to a choice field's `/Opt`.
    AddOption(&'a str),
    /// Remove the option with this display name.
    RemoveOption(&'a str),
    /// The caption a push button shows (`/MK /CA`).
    Caption(&'a str),
}

pub(crate) mod sealed {
    use super::{FieldBase, FieldConfig};

    /// Field-base access for concrete form fields.
    pub(crate) trait HasFieldBase {
        /// The shared field base.
        fn field(&self) -> &FieldBase;
        /// The shared field base, mutably.
        fn field_mut(&mut self) -> &mut FieldBase;
        /// The PDF `/FT` field-type name written on save. Defaults to
        /// `Tx` (text); the button / choice / signature families
        /// override it.
        fn pdf_field_type(&self) -> &'static [u8] {
            b"Tx"
        }
        /// The `/Ff` bits this field adds to ReadOnly and Required
        /// (§12.7.4, Tables 226–228). Bit numbering is 1-based there,
        /// so bit 13 is `1 << 12`. Defaults to none.
        fn pdf_field_flags(&self) -> i64 {
            0
        }
        /// Apply a type-specific change, answering whether this field
        /// family understands the request. The default understands
        /// none, so a family that overrides nothing declines
        /// everything.
        fn configure(&mut self, _request: FieldConfig<'_>) -> bool {
            false
        }
        /// Field dictionary entries beyond the common ones — `/MaxLen`
        /// for a text field, and whatever a family needs.
        fn pdf_field_entries(
            &self,
        ) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
            Vec::new()
        }
    }
}

/// A form field.
///
/// Concrete fields implement the crate-private widget / field base
/// accessors and [`annotation_type`](Annotation::annotation_type);
/// every property below is a default backed by the shared bases. A
/// field re-exposes the widget properties it inherits from
/// `WidgetAnnotation` (read-only / required / … ).
#[allow(private_bounds)]
pub trait Field: Annotation + HasWidgetBase + sealed::HasFieldBase {
    // -- inherited widget properties ---------------------------------

    /// Whether the field is read-only.
    fn read_only(&self) -> bool {
        self.widget().read_only
    }
    /// Set whether the field is read-only.
    fn set_read_only(&mut self, value: bool) {
        self.widget_mut().read_only = value;
    }

    /// Whether the field is required.
    fn required(&self) -> bool {
        self.widget().required
    }
    /// Set whether the field is required.
    fn set_required(&mut self, value: bool) {
        self.widget_mut().required = value;
    }

    /// Whether the field value is exportable.
    fn exportable(&self) -> bool {
        self.widget().exportable
    }
    /// Set whether the field value is exportable.
    fn set_exportable(&mut self, value: bool) {
        self.widget_mut().exportable = value;
    }

    /// The field highlighting mode.
    fn highlighting(&self) -> HighlightingMode {
        self.widget().highlighting
    }
    /// Set the field highlighting mode.
    fn set_highlighting(&mut self, value: HighlightingMode) {
        self.widget_mut().highlighting = value;
    }

    /// The field's default text appearance.
    fn default_appearance(&self) -> &DefaultAppearance {
        &self.widget().default_appearance
    }
    /// Set the field's default text appearance.
    fn set_default_appearance(&mut self, value: DefaultAppearance) {
        self.widget_mut().default_appearance = value;
    }

    /// The action run when the field is activated, if any.
    fn on_activated(&self) -> Option<&dyn PdfAction> {
        self.widget().on_activated.as_deref()
    }
    /// Set the action run when the field is activated.
    fn set_on_activated(&mut self, value: Box<dyn PdfAction>) {
        self.widget_mut().on_activated = Some(value);
    }
    /// Remove the action run when the field is activated.
    fn clear_on_activated(&mut self) {
        self.widget_mut().on_activated = None;
    }

    // -- field properties --------------------------------------------

    /// The field's partial (local) name.
    fn partial_name(&self) -> &str {
        &self.field().partial_name
    }
    /// Set the field's partial name.
    fn set_partial_name(&mut self, value: String) {
        self.field_mut().partial_name = value;
    }

    /// The alternate (user-facing) name.
    fn alternate_name(&self) -> &str {
        &self.field().alternate_name
    }
    /// Set the alternate name.
    fn set_alternate_name(&mut self, value: String) {
        self.field_mut().alternate_name = value;
    }

    /// The mapping (export) name.
    fn mapping_name(&self) -> &str {
        &self.field().mapping_name
    }
    /// Set the mapping name.
    fn set_mapping_name(&mut self, value: String) {
        self.field_mut().mapping_name = value;
    }

    /// The field value as a string.
    fn value(&self) -> &str {
        &self.field().value
    }
    /// Set the field value.
    fn set_value(&mut self, value: String) {
        self.field_mut().value = value;
    }

    /// The index of the widget within the field.
    fn annotation_index(&self) -> i32 {
        self.field().annotation_index
    }
    /// Set the widget index.
    fn set_annotation_index(&mut self, value: i32) {
        self.field_mut().annotation_index = value;
    }

    /// Whether the field is shared across pages.
    fn is_shared_field(&self) -> bool {
        self.field().is_shared_field
    }
    /// Set whether the field is shared across pages.
    fn set_is_shared_field(&mut self, value: bool) {
        self.field_mut().is_shared_field = value;
    }

    /// Whether text is scaled to fit the field rectangle.
    fn fit_into_rectangle(&self) -> bool {
        self.field().fit_into_rectangle
    }
    /// Set whether text is scaled to fit the field rectangle.
    fn set_fit_into_rectangle(&mut self, value: bool) {
        self.field_mut().fit_into_rectangle = value;
    }

    /// The maximum auto-size font size.
    fn max_font_size(&self) -> f64 {
        self.field().max_font_size
    }
    /// Set the maximum auto-size font size.
    fn set_max_font_size(&mut self, value: f64) {
        self.field_mut().max_font_size = value;
    }

    /// The minimum auto-size font size.
    fn min_font_size(&self) -> f64 {
        self.field().min_font_size
    }
    /// Set the minimum auto-size font size.
    fn set_min_font_size(&mut self, value: f64) {
        self.field_mut().min_font_size = value;
    }

    /// The field's tab order.
    fn tab_order(&self) -> i32 {
        self.field().tab_order
    }
    /// Set the field's tab order.
    fn set_tab_order(&mut self, value: i32) {
        self.field_mut().tab_order = value;
    }

    // -- methods + widget collection ---------------------------------

    /// Recalculate the field value. No-op (returns `true`) in the
    /// in-memory model.
    fn recalculate(&mut self) -> bool {
        true
    }

    /// Move the field so its lower-left corner is at `position`,
    /// preserving its size.
    fn set_position(&mut self, position: Point) {
        let (w, h) = {
            let r = self.base().rect;
            (r.width(), r.height())
        };
        let r = self.base_mut().rect;
        let mut moved = r;
        moved.set_llx(position.x());
        moved.set_lly(position.y());
        moved.set_urx(position.x() + w);
        moved.set_ury(position.y() + h);
        self.base_mut().rect = moved;
    }

    /// Run a JavaScript action against the field. No-op in the
    /// in-memory model.
    fn execute_field_javascript(&mut self, _action: JavascriptAction) {}

    /// The number of widget kids.
    fn count(&self) -> usize {
        self.field().kids.len()
    }

    /// Whether the field groups multiple widgets.
    fn is_group(&self) -> bool {
        self.field().kids.len() > 1
    }

    /// The widget kid at a 0-based index.
    fn widget_at(&self, index: usize) -> Option<&WidgetAnnotation> {
        self.field().kids.get(index)
    }

    /// The widget kid with the given name, if any.
    fn widget_by_name(&self, name: &str) -> Option<&WidgetAnnotation> {
        self.field().kids.iter().find(|w| w.name() == name)
    }
}

/// A form field parsed from a document's `/AcroForm /Fields`. One
/// concrete type carries every loaded field regardless of `/FT` (the
/// Rust analogue of how loaded annotations collapse to
/// `LoadedAnnotation`), so the load path needs no per-`/FT`
/// reconstruction. The original `/FT` is preserved for save-through.
pub(crate) struct LoadedField {
    base: crate::public::annotation::AnnotationBase,
    widget: crate::public::widget_annotation::WidgetBase,
    field: FieldBase,
    ft: &'static [u8],
    /// The `/Ff` bits the document carried beyond ReadOnly and
    /// Required. The model names a few of them per field type; a
    /// loaded field is not one of those types, so the bits are kept
    /// verbatim and written back rather than silently cleared.
    other_flags: i64,
}

impl LoadedField {
    /// Build a field from one parsed `/AcroForm /Fields` entry. The
    /// parsed data is taken whole rather than argument by argument, so
    /// an entry the parser learns to read reaches the field without
    /// another parameter.
    pub(crate) fn from_parsed(
        data: crate::internal::acroform_io::FieldData,
        rect: crate::public::rectangle::Rectangle,
    ) -> Self {
        let mut base = crate::public::annotation::AnnotationBase::new(rect);
        base.page_index = data.page_number;
        let mut widget = crate::public::widget_annotation::WidgetBase::new();
        widget.read_only = data.read_only;
        widget.required = data.required;
        widget.exportable = data.exportable;
        let mut field = FieldBase::new();
        field.partial_name = data.partial_name;
        field.alternate_name = data.alternate_name;
        field.mapping_name = data.mapping_name;
        field.value = data.value;
        Self {
            base,
            widget,
            field,
            ft: ft_literal(&data.ft),
            other_flags: data.other_flags,
        }
    }
}

/// Map a parsed `/FT` to one of the known `'static` names (defaults to
/// `Tx`), so `pdf_field_type` can keep returning `&'static [u8]`.
fn ft_literal(ft: &str) -> &'static [u8] {
    match ft {
        "Btn" => b"Btn",
        "Ch" => b"Ch",
        "Sig" => b"Sig",
        _ => b"Tx",
    }
}

impl crate::public::annotation::sealed::HasBase for LoadedField {
    fn base(&self) -> &crate::public::annotation::AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut crate::public::annotation::AnnotationBase {
        &mut self.base
    }
}
impl HasWidgetBase for LoadedField {
    fn widget(&self) -> &crate::public::widget_annotation::WidgetBase {
        &self.widget
    }
    fn widget_mut(&mut self) -> &mut crate::public::widget_annotation::WidgetBase {
        &mut self.widget
    }
}
impl sealed::HasFieldBase for LoadedField {
    fn field(&self) -> &FieldBase {
        &self.field
    }
    fn field_mut(&mut self) -> &mut FieldBase {
        &mut self.field
    }
    fn pdf_field_type(&self) -> &'static [u8] {
        self.ft
    }
    fn pdf_field_flags(&self) -> i64 {
        self.other_flags
    }
}
impl Annotation for LoadedField {
    fn annotation_type(&self) -> crate::public::annotation_type::AnnotationType {
        crate::public::annotation_type::AnnotationType::Widget
    }
}
impl Field for LoadedField {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::annotation::sealed::HasBase;
    use crate::public::annotation::AnnotationBase;
    use crate::public::annotation_type::AnnotationType;
    use crate::public::widget_annotation::WidgetBase;

    struct TestField {
        base: AnnotationBase,
        widget: WidgetBase,
        field: FieldBase,
    }

    impl HasBase for TestField {
        fn base(&self) -> &AnnotationBase {
            &self.base
        }
        fn base_mut(&mut self) -> &mut AnnotationBase {
            &mut self.base
        }
    }
    impl HasWidgetBase for TestField {
        fn widget(&self) -> &WidgetBase {
            &self.widget
        }
        fn widget_mut(&mut self) -> &mut WidgetBase {
            &mut self.widget
        }
    }
    impl sealed::HasFieldBase for TestField {
        fn field(&self) -> &FieldBase {
            &self.field
        }
        fn field_mut(&mut self) -> &mut FieldBase {
            &mut self.field
        }
    }
    impl Annotation for TestField {
        fn annotation_type(&self) -> AnnotationType {
            AnnotationType::Widget
        }
    }
    impl Field for TestField {}

    fn make() -> TestField {
        TestField {
            base: AnnotationBase::default(),
            widget: WidgetBase::new(),
            field: FieldBase::new(),
        }
    }

    #[test]
    fn field_property_round_trip() {
        let mut f = make();
        assert_eq!(f.partial_name(), "");
        assert!(!f.read_only());
        assert!(f.exportable());
        assert!(f.recalculate());

        f.set_partial_name("name".into());
        f.set_value("v".into());
        f.set_read_only(true);
        f.set_tab_order(3);
        assert_eq!(f.partial_name(), "name");
        assert_eq!(f.value(), "v");
        assert!(f.read_only());
        assert_eq!(f.tab_order(), 3);
    }

    #[test]
    fn field_position_and_group() {
        let mut f = make();
        assert_eq!(f.count(), 0);
        assert!(!f.is_group());
        f.set_position(Point::new(10.0, 20.0));
        assert_eq!(f.base().rect.llx(), 10.0);
        assert_eq!(f.base().rect.lly(), 20.0);
    }
}
