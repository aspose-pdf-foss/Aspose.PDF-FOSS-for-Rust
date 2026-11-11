//! `Aspose.Pdf.Annotations.Annotation` — the abstract base of every
//! PDF annotation, per `capabilities/annotation.yaml`.
//!
//! Modelled as the [`Annotation`] trait plus a shared
//! `AnnotationBase` field carrier. Each concrete annotation embeds
//! an `AnnotationBase`, implements the crate-private `sealed::HasBase`
//! accessors and [`Annotation::annotation_type`]; the property
//! accessors are provided as trait defaults that read / write the
//! shared base. The canonical `Actions`
//! (`PdfActionCollection`), `Modified` (`System.DateTime`), `States`
//! / `Appearance` (`AppearanceDictionary`), `ChangeAfterResize`
//! (`Matrix`), and `Accept` (`AnnotationSelector`, ships with the
//! selector) members are dropped (see `targets/rust/drops.yaml`).

use crate::public::annotation_flags::AnnotationFlags;
use crate::public::border::Border;
use crate::public::characteristics::Characteristics;
use crate::public::color::Color;
use crate::public::date_time::DateTime;
use crate::public::annotation_type::AnnotationType;
use crate::public::horizontal_alignment::HorizontalAlignment;
use crate::public::matrix::Matrix;
use crate::public::rectangle::Rectangle;
use crate::public::text_alignment::TextAlignment;

/// Shared field storage for every annotation. Crate-internal: it is
/// not a canonical type but the Rust mechanism for sharing the
/// `Annotation` base's properties across concrete annotations.
pub(crate) struct AnnotationBase {
    pub(crate) rect: Rectangle,
    pub(crate) color: Color,
    pub(crate) name: String,
    pub(crate) contents: String,
    pub(crate) active_state: String,
    pub(crate) full_name: String,
    pub(crate) flags: AnnotationFlags,
    pub(crate) border: Border,
    pub(crate) characteristics: Characteristics,
    pub(crate) alignment: TextAlignment,
    pub(crate) horizontal_alignment: HorizontalAlignment,
    pub(crate) text_horizontal_alignment: HorizontalAlignment,
    pub(crate) update_appearance_on_convert: bool,
    pub(crate) use_font_subset: bool,
    pub(crate) page_index: i32,
    pub(crate) modified: DateTime,
    /// Dictionary entries read off a loaded annotation that this model
    /// does not name, kept so a page rewrite does not strip them. Empty
    /// for an annotation built in memory — there is nothing to carry.
    pub(crate) carried: Vec<(Vec<u8>, crate::internal::objects::Value)>,
}

impl AnnotationBase {
    /// Construct a base with the given rectangle and defaults.
    pub(crate) fn new(rect: Rectangle) -> Self {
        Self {
            rect,
            color: Color::empty(),
            name: String::new(),
            contents: String::new(),
            active_state: String::new(),
            full_name: String::new(),
            flags: AnnotationFlags::Default,
            border: Border::default(),
            characteristics: Characteristics::new(),
            alignment: TextAlignment::Left,
            horizontal_alignment: HorizontalAlignment::None,
            text_horizontal_alignment: HorizontalAlignment::None,
            update_appearance_on_convert: false,
            use_font_subset: false,
            page_index: 0,
            modified: DateTime::default(),
            carried: Vec::new(),
        }
    }
}

impl Default for AnnotationBase {
    fn default() -> Self {
        Self::new(Rectangle::trivial())
    }
}

/// Crate-private supertrait carrying access to the shared
/// `AnnotationBase`. Sealing [`Annotation`] (external crates
/// cannot name this trait, so cannot implement `Annotation`) and
/// keeping the crate-private base off the public surface.
pub(crate) mod sealed {
    use super::AnnotationBase;

    /// Base-field access for concrete annotations.
    pub(crate) trait HasBase {
        /// The shared field base.
        fn base(&self) -> &AnnotationBase;
        /// The shared field base, mutably.
        fn base_mut(&mut self) -> &mut AnnotationBase;
        /// The shared markup base, for the subtypes that have one.
        /// Lets the common save path add the markup entries (§12.5.6.2
        /// — /T, /Subj, /RC, /RT, /CA) once instead of seventeen times.
        fn markup_base(&self) -> Option<&crate::public::markup_annotation::MarkupBase> {
            None
        }
        /// The shared text-markup base, for the four subtypes that
        /// have one. Lets the common save path add `/QuadPoints`
        /// (§12.5.6.10) once instead of four times.
        fn text_markup_base(
            &self,
        ) -> Option<&crate::public::text_markup_annotation::TextMarkupBase> {
            None
        }
        /// The shared figure base, for the two subtypes that have one.
        /// Lets the common save path add `/IC` and `/RD` (§12.5.6.8)
        /// once instead of twice.
        fn figure_base(
            &self,
        ) -> Option<&crate::public::common_figure_annotation::CommonFigureBase> {
            None
        }
        /// The dictionary entries this subtype contributes on save
        /// (§12.5.6) — `/InkList` for ink, `/L` for a line, and so on.
        /// The common entries come from the base and are added for
        /// every annotation, so a type that needs none says nothing.
        fn pdf_entries(&self) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
            Vec::new()
        }
    }
}

/// A PDF annotation.
///
/// Concrete annotations implement the crate-private base accessor
/// and [`annotation_type`](Self::annotation_type); every other
/// accessor is a default backed by the shared `AnnotationBase`.
///
/// The `sealed::HasBase` supertrait is crate-private — it seals the
/// trait (external crates cannot implement `Annotation`) and keeps
/// the crate-private `AnnotationBase` off the public surface. The
/// `private_bounds` lint flags exactly this intentional sealed-trait
/// shape, so it is allowed here.
#[allow(private_bounds)]
pub trait Annotation: sealed::HasBase {
    /// The annotation's type discriminator.
    fn annotation_type(&self) -> AnnotationType;

    /// The annotation rectangle.
    fn rect(&self) -> Rectangle {
        self.base().rect
    }
    /// Set the annotation rectangle.
    fn set_rect(&mut self, value: Rectangle) {
        self.base_mut().rect = value;
    }

    /// The annotation rectangle. `consider_rotation` is accepted for
    /// canonical parity; the in-memory rectangle is returned as-is.
    fn get_rectangle(&self, _consider_rotation: bool) -> Rectangle {
        self.base().rect
    }

    /// The rectangle width.
    fn width(&self) -> f64 {
        self.base().rect.width()
    }
    /// Set the rectangle width (adjusts the upper-right x).
    fn set_width(&mut self, value: f64) {
        let llx = self.base().rect.llx();
        self.base_mut().rect.set_urx(llx + value);
    }

    /// The rectangle height.
    fn height(&self) -> f64 {
        self.base().rect.height()
    }
    /// Set the rectangle height (adjusts the upper-right y).
    fn set_height(&mut self, value: f64) {
        let lly = self.base().rect.lly();
        self.base_mut().rect.set_ury(lly + value);
    }

    /// Update the annotation's parameters according to `transform`
    /// (canonical `ChangeAfterResize`) — transforms the annotation
    /// rectangle. Appearance is (re)generated at save time, so
    /// nothing is regenerated here.
    fn change_after_resize(&mut self, transform: &Matrix) {
        let rect = self.base().rect;
        self.base_mut().rect = transform.transform_rectangle(&rect);
    }

    /// The annotation colour.
    fn color(&self) -> Color {
        self.base().color
    }
    /// Set the annotation colour.
    fn set_color(&mut self, value: Color) {
        self.base_mut().color = value;
    }

    /// The last-modified date (canonical `Modified`).
    fn modified(&self) -> DateTime {
        self.base().modified
    }
    /// Set the last-modified date.
    fn set_modified(&mut self, value: DateTime) {
        self.base_mut().modified = value;
    }

    /// The annotation name (`/NM`).
    fn name(&self) -> &str {
        &self.base().name
    }
    /// Set the annotation name.
    fn set_name(&mut self, value: String) {
        self.base_mut().name = value;
    }

    /// The text contents (`/Contents`).
    fn contents(&self) -> &str {
        &self.base().contents
    }
    /// Set the text contents.
    fn set_contents(&mut self, value: String) {
        self.base_mut().contents = value;
    }

    /// The active appearance state (`/AS`).
    /// **Carried, not applied.** An annotation is written as an
    /// interactive object with no appearance stream, so there are no
    /// appearance states for this to select between, and nothing this
    /// crate draws depends on it. The same holds for
    /// [`text_horizontal_alignment`](Self::text_horizontal_alignment),
    /// [`update_appearance_on_convert`](Self::update_appearance_on_convert)
    /// and [`use_font_subset`](Self::use_font_subset) — all three
    /// describe how an appearance would be generated, and none of them
    /// is written into the annotation dictionary either. A viewer draws
    /// the annotation from its own defaults instead.
    fn active_state(&self) -> &str {
        &self.base().active_state
    }
    /// Set the active appearance state.
    fn set_active_state(&mut self, value: String) {
        self.base_mut().active_state = value;
    }

    /// The annotation flags.
    fn flags(&self) -> AnnotationFlags {
        self.base().flags
    }
    /// Set the annotation flags.
    fn set_flags(&mut self, value: AnnotationFlags) {
        self.base_mut().flags = value;
    }

    /// The annotation border.
    fn border(&self) -> Border {
        self.base().border.clone()
    }
    /// Set the annotation border.
    fn set_border(&mut self, value: Border) {
        self.base_mut().border = value;
    }

    /// The appearance characteristics.
    fn characteristics(&self) -> Characteristics {
        self.base().characteristics
    }

    /// The content alignment.
    ///
    /// Carried, not written: it sits here because the canonical surface
    /// puts it on every annotation, while the format gives an alignment
    /// entry only to the subtypes that have text of their own —
    /// §12.5.6.6's `/Q` on a free text annotation, which
    /// [`FreeTextAnnotation`](crate::FreeTextAnnotation) writes from
    /// its own justification, and §12.7.4.1's `/Q` on a field, which a
    /// field writes from its text alignment. For every other subtype
    /// there is no entry for this to become.
    fn alignment(&self) -> TextAlignment {
        self.base().alignment
    }
    /// Set the content alignment.
    fn set_alignment(&mut self, value: TextAlignment) {
        self.base_mut().alignment = value;
    }

    /// The horizontal alignment.
    fn horizontal_alignment(&self) -> HorizontalAlignment {
        self.base().horizontal_alignment
    }
    /// Set the horizontal alignment.
    fn set_horizontal_alignment(&mut self, value: HorizontalAlignment) {
        self.base_mut().horizontal_alignment = value;
    }

    /// The text horizontal alignment.
    /// **Carried, not applied** — see [`active_state`](Self::active_state).
    fn text_horizontal_alignment(&self) -> HorizontalAlignment {
        self.base().text_horizontal_alignment
    }
    /// Set the text horizontal alignment.
    fn set_text_horizontal_alignment(&mut self, value: HorizontalAlignment) {
        self.base_mut().text_horizontal_alignment = value;
    }

    /// Whether the appearance is regenerated on convert.
    /// **Carried, not applied** — see [`active_state`](Self::active_state).
    fn update_appearance_on_convert(&self) -> bool {
        self.base().update_appearance_on_convert
    }
    /// Set whether the appearance is regenerated on convert.
    fn set_update_appearance_on_convert(&mut self, value: bool) {
        self.base_mut().update_appearance_on_convert = value;
    }

    /// Whether font subsetting is used.
    /// **Carried, not applied** — see [`active_state`](Self::active_state).
    fn use_font_subset(&self) -> bool {
        self.base().use_font_subset
    }
    /// Set whether font subsetting is used.
    fn set_use_font_subset(&mut self, value: bool) {
        self.base_mut().use_font_subset = value;
    }

    /// The fully-qualified name.
    fn full_name(&self) -> &str {
        &self.base().full_name
    }

    /// The 0-based index of the page this annotation belongs to.
    fn page_index(&self) -> i32 {
        self.base().page_index
    }

    /// Flatten the annotation into the page content. No-op in the
    /// in-memory model.
    fn flatten(&mut self) {}

    /// Accept an [`AnnotationSelector`](crate::AnnotationSelector),
    /// recording this annotation's type. The canonical double
    /// dispatch (each concrete calling the matching `Visit` overload)
    /// is modelled here by recording the dynamic
    /// [`annotation_type`](Self::annotation_type).
    fn accept(&self, selector: &mut crate::public::annotation_selector::AnnotationSelector) {
        selector.record_visit(self.annotation_type());
    }
}

/// An annotation reconstructed from a PDF's `/Annots` on load. Carries
/// the `/Subtype`-derived type plus the rect / contents / name read from
/// the page; reports that type via [`Annotation::annotation_type`].
pub(crate) struct LoadedAnnotation {
    base: AnnotationBase,
    annotation_type: AnnotationType,
}

impl LoadedAnnotation {
    /// Build an annotation from one parsed `/Annots` entry. The parsed
    /// data is taken whole so an entry the parser learns to read
    /// reaches the annotation without another parameter — and so the
    /// entries this model does not name travel with it.
    pub(crate) fn from_parsed(
        annotation_type: AnnotationType,
        rect: Rectangle,
        data: crate::internal::annotation_io::AnnotData,
    ) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.contents = data.contents;
        base.name = data.name;
        base.carried = data.carried;
        Self { base, annotation_type }
    }
}

impl LoadedAnnotation {
    /// A standalone copy of `source`: the same type and base
    /// properties, carrying every entry `source` would be written with.
    ///
    /// The copy is not of `source`'s concrete type — nothing here can
    /// construct one from a `&dyn Annotation` — so the subtype's typed
    /// accessors are gone. What survives is the dictionary: writing
    /// this copy back produces the same entries, which is what an
    /// extracted annotation is for.
    pub(crate) fn from_model(source: &dyn Annotation) -> Self {
        let mut base = AnnotationBase::new(source.rect());
        base.contents = source.contents().to_string();
        base.name = source.name().to_string();
        base.page_index = source.page_index();
        base.carried = crate::public::annotation_collection::snapshot_entries(source);
        Self { base, annotation_type: source.annotation_type() }
    }
}

impl sealed::HasBase for LoadedAnnotation {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
}

impl Annotation for LoadedAnnotation {
    fn annotation_type(&self) -> AnnotationType {
        self.annotation_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal concrete annotation for exercising the base.
    struct TestAnnotation {
        base: AnnotationBase,
    }

    impl sealed::HasBase for TestAnnotation {
        fn base(&self) -> &AnnotationBase {
            &self.base
        }
        fn base_mut(&mut self) -> &mut AnnotationBase {
            &mut self.base
        }
    }

    impl Annotation for TestAnnotation {
        fn annotation_type(&self) -> AnnotationType {
            AnnotationType::Text
        }
    }

    #[test]
    fn base_defaults_and_round_trip() {
        let mut a = TestAnnotation {
            base: AnnotationBase::new(Rectangle::new(0.0, 0.0, 100.0, 50.0, false)),
        };
        assert_eq!(a.annotation_type(), AnnotationType::Text);
        assert_eq!(a.width(), 100.0);
        assert_eq!(a.height(), 50.0);
        assert_eq!(a.flags(), AnnotationFlags::Default);

        a.set_name("note1".into());
        a.set_contents("hello".into());
        a.set_flags(AnnotationFlags::Print);
        a.set_width(200.0);
        assert_eq!(a.name(), "note1");
        assert_eq!(a.contents(), "hello");
        assert_eq!(a.flags(), AnnotationFlags::Print);
        assert_eq!(a.width(), 200.0);
        assert_eq!(a.rect().urx(), 200.0);
    }

    #[test]
    fn border_round_trip() {
        let mut a = TestAnnotation {
            base: AnnotationBase::default(),
        };
        let mut b = Border::new(&a);
        b.set_width(3);
        b.set_style(BorderStyle::Dashed);
        b.set_dash(crate::public::dash::Dash::new(4, 2));
        a.set_border(b);
        assert_eq!(a.border().width(), 3);
        assert_eq!(a.border().style(), BorderStyle::Dashed);
        assert_eq!(a.border().dash().on(), 4);
        assert_eq!(a.border().dash().off(), 2);
    }

    use crate::public::border_style::BorderStyle;
}
