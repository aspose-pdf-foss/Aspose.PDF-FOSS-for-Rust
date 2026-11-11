//! `Aspose.Pdf.Annotations.WidgetAnnotation` — a form-field widget
//! annotation, per `capabilities/widget_annotation.yaml`.
//!
//! Extends [`Annotation`]. It is the base of the
//! forms [`Field`](crate::Field) hierarchy; the shared widget fields
//! live in the crate-internal `WidgetBase` so form fields can
//! reuse them. The `ExportToJson` overloads
//! (`ExportFieldsToJsonOptions` / `FieldSerializationResult`),
//! `Actions` (`AnnotationActionCollection`), and `Parent`
//! (recursive `Field` reference) are dropped.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::default_appearance::DefaultAppearance;
use crate::public::document::Document;
use crate::public::highlighting_mode::HighlightingMode;
use crate::public::pdf_action::PdfAction;

/// Shared widget fields. Crate-internal — reused by the forms
/// [`Field`](crate::Field) hierarchy.
pub(crate) struct WidgetBase {
    pub(crate) on_activated: Option<Box<dyn PdfAction>>,
    pub(crate) highlighting: HighlightingMode,
    pub(crate) default_appearance: DefaultAppearance,
    pub(crate) read_only: bool,
    pub(crate) required: bool,
    pub(crate) exportable: bool,
}

impl WidgetBase {
    pub(crate) fn new() -> Self {
        Self {
            on_activated: None,
            highlighting: HighlightingMode::None,
            default_appearance: DefaultAppearance::new(),
            read_only: false,
            required: false,
            exportable: true,
        }
    }
}

impl Default for WidgetBase {
    fn default() -> Self {
        Self::new()
    }
}

/// The entries §12.5.6.19 gives a widget, read off the shared base:
/// the action activating it runs, the way a viewer highlights it under
/// the pointer, and the operator string its text is set with. Shared
/// with the forms [`Field`](crate::Field) hierarchy, whose fields are
/// written as merged field / widget objects (§12.7.3.1) and so carry
/// the same entries.
pub(crate) fn widget_entries(
    widget: &WidgetBase,
) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
    use crate::internal::annotation_io::Entry;
    let mut out = Vec::new();
    if let Some(action) = widget.on_activated.as_deref() {
        if let Some(entry) = crate::public::pdf_action::action_entry(action) {
            out.push(("A", entry));
        }
    }
    // Table 188: /H is the highlighting mode. The format's default is
    // /I (invert) while this carrier's is None, so None is written out
    // as /N rather than left to the default — the getter says the
    // widget does not highlight, and the file now says the same.
    // Invert is the one value an absent entry can stand for.
    if let Some(name) = highlighting_name(widget.highlighting) {
        out.push(("H", Entry::Name(name.to_string())));
    }
    let da = widget.default_appearance.text();
    if !da.is_empty() {
        out.push(("DA", Entry::Text(da)));
    }
    out
}

/// The name PDF gives a widget's highlighting mode (§12.5.6.19
/// Table 188). `Invert` is the format's default and needs no entry.
fn highlighting_name(mode: HighlightingMode) -> Option<&'static str> {
    match mode {
        HighlightingMode::Invert => None,
        HighlightingMode::None => Some("N"),
        HighlightingMode::Outline => Some("O"),
        HighlightingMode::Push => Some("P"),
        HighlightingMode::Toggle => Some("T"),
    }
}

/// The `/Ff` bits §12.7.4 Table 226 gives every field, as opposed to
/// the type-specific bits each field family adds: ReadOnly is bit 1,
/// Required bit 2, NoExport bit 3. A field that is exportable — the
/// default — leaves the third bit clear.
pub(crate) fn common_field_flags(widget: &WidgetBase) -> i64 {
    (widget.read_only as i64)
        | ((widget.required as i64) << 1)
        | ((!widget.exportable as i64) << 2)
}

pub(crate) mod sealed {
    use super::WidgetBase;

    /// Widget-base access — shared by [`WidgetAnnotation`](super::WidgetAnnotation)
    /// and the forms [`Field`](crate::Field) hierarchy.
    pub(crate) trait HasWidgetBase {
        /// The shared widget base.
        fn widget(&self) -> &WidgetBase;
        /// The shared widget base, mutably.
        fn widget_mut(&mut self) -> &mut WidgetBase;
    }
}

impl sealed::HasWidgetBase for WidgetAnnotation {
    fn widget(&self) -> &WidgetBase {
        &self.widget
    }
    fn widget_mut(&mut self) -> &mut WidgetBase {
        &mut self.widget
    }
}

/// A form-field widget annotation.
pub struct WidgetAnnotation {
    base: AnnotationBase,
    widget: WidgetBase,
}

impl WidgetAnnotation {
    /// Construct within a [`Document`].
    pub fn new(_document: &Document) -> Self {
        Self {
            base: AnnotationBase::default(),
            widget: WidgetBase::new(),
        }
    }

    /// The name of the widget's checked ("on") appearance state, or
    /// an empty string when the widget has none.
    pub fn get_checked_state_name(&self) -> String {
        String::new()
    }

    /// The action run when the widget is activated, if any.
    pub fn on_activated(&self) -> Option<&dyn PdfAction> {
        self.widget.on_activated.as_deref()
    }
    /// Set the action run when the widget is activated.
    pub fn set_on_activated(&mut self, value: Box<dyn PdfAction>) {
        self.widget.on_activated = Some(value);
    }

    /// The widget highlighting mode.
    pub fn highlighting(&self) -> HighlightingMode {
        self.widget.highlighting
    }
    /// Set the widget highlighting mode.
    pub fn set_highlighting(&mut self, value: HighlightingMode) {
        self.widget.highlighting = value;
    }

    /// The widget's default text appearance.
    pub fn default_appearance(&self) -> &DefaultAppearance {
        &self.widget.default_appearance
    }
    /// Set the widget's default text appearance.
    pub fn set_default_appearance(&mut self, value: DefaultAppearance) {
        self.widget.default_appearance = value;
    }

    /// Whether the field is read-only.
    pub fn read_only(&self) -> bool {
        self.widget.read_only
    }
    /// Set whether the field is read-only.
    pub fn set_read_only(&mut self, value: bool) {
        self.widget.read_only = value;
    }

    /// Whether the field is required.
    pub fn required(&self) -> bool {
        self.widget.required
    }
    /// Set whether the field is required.
    pub fn set_required(&mut self, value: bool) {
        self.widget.required = value;
    }

    /// Whether the field value is exportable.
    pub fn exportable(&self) -> bool {
        self.widget.exportable
    }
    /// Set whether the field value is exportable.
    pub fn set_exportable(&mut self, value: bool) {
        self.widget.exportable = value;
    }
}

impl HasBase for WidgetAnnotation {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
    fn pdf_entries(&self) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        // A widget standing on its own is written through the
        // annotation writer, so it carries its own /Ff: the three bits
        // of Table 226 are all a bare widget has.
        let mut out = widget_entries(&self.widget);
        let flags = common_field_flags(&self.widget);
        if flags != 0 {
            out.push(("Ff", Entry::Int(flags)));
        }
        out
    }
}
impl Annotation for WidgetAnnotation {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::Widget
    }
}
