//! `Aspose.Pdf.Annotations.AnnotationCollection` — a page's
//! annotations, per `capabilities/annotation_collection.yaml`.
//!
//! A shared, interior-mutable handle (`Rc<RefCell<…>>`) so a page's
//! annotations can be mutated through a shared `&Page` and the change is
//! visible to the document's save path. Indexed 0-based; the
//! reference-returning accessors (`get` / `find_by_name`) yield a
//! [`Ref`] guard. Identity-based membership (`contains` / `remove` /
//! `delete`) is keyed on the annotation **name** (Rust ownership moves
//! the annotation in, so pointer identity is impossible). The .NET
//! collection infrastructure (`CopyTo` / `GetEnumerator` / …) is dropped.

use crate::public::annotation::Annotation;
use crate::public::annotation_type::AnnotationType;
use crate::public::error::Error;
use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;

/// A snapshot of one annotation, for save-through serialisation.
pub(crate) struct AnnotSave {
    pub annotation_type: AnnotationType,
    pub rect: [f64; 4],
    pub contents: String,
    pub name: String,
    /// The dictionary entries beyond the four above: the common ones
    /// every annotation carries, plus whatever the subtype contributes.
    pub entries: Vec<(&'static str, crate::internal::annotation_io::Entry)>,
    /// What a loaded annotation carried that this model does not name,
    /// passed through so a page rewrite keeps it.
    pub carried: Vec<(Vec<u8>, crate::internal::objects::Value)>,
}

/// A page's collection of annotations (0-based indexing).
#[derive(Default, Clone)]
pub struct AnnotationCollection {
    items: Rc<RefCell<Vec<Box<dyn Annotation>>>>,
    dirty: Rc<Cell<bool>>,
}

impl AnnotationCollection {
    /// Construct an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an annotation.
    pub fn add(&self, annotation: Box<dyn Annotation>) {
        self.items.borrow_mut().push(annotation);
        self.dirty.set(true);
    }

    /// Append an annotation, optionally regenerating its appearance.
    /// `consider_rotation` is accepted for canonical parity.
    pub fn add_with_appearance(&self, annotation: Box<dyn Annotation>, _consider_rotation: bool) {
        self.add(annotation);
    }

    /// The number of annotations.
    pub fn count(&self) -> usize {
        self.items.borrow().len()
    }

    /// The annotation at a 0-based index (a [`Ref`] guard).
    ///
    /// Returns [`Error`] when out of range (the Rust analogue of the
    /// canonical indexer's bounds throw).
    pub fn get(&self, index: usize) -> Result<Ref<'_, dyn Annotation>, Error> {
        if index >= self.items.borrow().len() {
            return Err(Error::CollectionItemNotFound(format!("annotation index {index}")));
        }
        Ok(Ref::map(self.items.borrow(), |v| v[index].as_ref()))
    }

    /// Remove the annotation at a 0-based index. Errors when out of
    /// range.
    pub fn delete_at(&self, index: usize) -> Result<(), Error> {
        let mut items = self.items.borrow_mut();
        if index < items.len() {
            items.remove(index);
            self.dirty.set(true);
            Ok(())
        } else {
            Err(Error::CollectionItemNotFound(format!("annotation index {index}")))
        }
    }

    /// Remove every annotation.
    pub fn delete_all(&self) {
        self.items.borrow_mut().clear();
        self.dirty.set(true);
    }

    /// Remove every annotation (alias of [`delete_all`](Self::delete_all)).
    pub fn clear(&self) {
        self.delete_all();
    }

    /// Remove every annotation whose type the predicate accepts.
    /// Crate-internal: the flattening editors select by type, and the
    /// canonical surface has no such member of its own.
    pub(crate) fn retain_types(&self, keep: impl Fn(AnnotationType) -> bool) {
        let before = self.items.borrow().len();
        self.items.borrow_mut().retain(|a| keep(a.annotation_type()));
        if self.items.borrow().len() != before {
            self.dirty.set(true);
        }
    }

    /// Remove the annotation matching `annotation`'s name.
    pub fn delete(&self, annotation: &dyn Annotation) {
        self.remove(annotation);
    }

    /// Whether an annotation with the same name is present.
    pub fn contains(&self, annotation: &dyn Annotation) -> bool {
        let name = annotation.name();
        self.items.borrow().iter().any(|a| a.name() == name)
    }

    /// Remove the first annotation matching `annotation`'s name;
    /// returns whether one was removed.
    pub fn remove(&self, annotation: &dyn Annotation) -> bool {
        let name = annotation.name().to_string();
        let mut items = self.items.borrow_mut();
        if let Some(pos) = items.iter().position(|a| a.name() == name) {
            items.remove(pos);
            self.dirty.set(true);
            true
        } else {
            false
        }
    }

    /// Find an annotation by name (a [`Ref`] guard), or `None`.
    pub fn find_by_name(&self, name: &str) -> Option<Ref<'_, dyn Annotation>> {
        let pos = self.items.borrow().iter().position(|a| a.name() == name)?;
        Some(Ref::map(self.items.borrow(), |v| v[pos].as_ref()))
    }

    /// Visit every annotation with the given selector.
    pub fn accept(&self, selector: &mut crate::public::annotation_selector::AnnotationSelector) {
        for annotation in self.items.borrow().iter() {
            annotation.accept(selector);
        }
    }

    // ---- save-through support (crate-internal) ----

    /// Whether the collection was mutated since it was built / loaded.
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    /// A snapshot of every annotation, for serialisation.
    pub(crate) fn for_save(&self) -> Vec<AnnotSave> {
        self.items
            .borrow()
            .iter()
            .map(|a| {
                let r = a.rect();
                AnnotSave {
                    annotation_type: a.annotation_type(),
                    rect: [r.llx(), r.lly(), r.urx(), r.ury()],
                    contents: a.contents().to_string(),
                    name: a.name().to_string(),
                    entries: printer_mark_entries(a.annotation_type())
                        .into_iter()
                        .chain(common_entries(a.as_ref()))
                        .chain(markup_entries(a.as_ref()))
                        .chain(text_markup_entries(a.as_ref()))
                        .chain(figure_entries(a.as_ref()))
                        .chain(a.pdf_entries())
                        .collect(),
                    carried: a.base().carried.clone(),
                }
            })
            .collect()
    }

    /// Append a loaded annotation without marking the collection dirty
    /// (a verbatim resave keeps the original `/Annots`).
    pub(crate) fn add_loaded(&self, annotation: Box<dyn Annotation>) {
        self.items.borrow_mut().push(annotation);
    }
}

impl std::fmt::Debug for AnnotationCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnnotationCollection").field("count", &self.count()).finish()
    }
}

/// The name PDF gives a border style (§12.5.4 Table 168). Beveled and
/// inset have no array form, so they map onto their dictionary names.
pub(crate) fn border_style_name(style: crate::public::border_style::BorderStyle) -> &'static str {
    use crate::public::border_style::BorderStyle;
    match style {
        BorderStyle::Solid => "S",
        BorderStyle::Dashed => "D",
        BorderStyle::Beveled => "B",
        BorderStyle::Inset => "I",
        BorderStyle::Underline => "U",
    }
}

/// The markup entries of §12.5.6.2, for the subtypes that carry a
/// markup base: the author, the subject, the rich-text contents, the
/// reply type and the constant opacity. Written only where they say
/// something — an empty string or a full opacity is the default and
/// needs no entry.
///
/// `/CreationDate` is written on the same terms as `/M`: only when the
/// caller gave a date, an unset one formatting to nothing.
fn markup_entries(
    annotation: &dyn Annotation,
) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
    use crate::internal::annotation_io::Entry;
    use crate::public::reply_type::ReplyType;
    let Some(markup) = annotation.markup_base() else { return Vec::new() };
    let mut out = Vec::new();
    if !markup.title.is_empty() {
        out.push(("T", Entry::Text(markup.title.clone())));
    }
    if !markup.subject.is_empty() {
        out.push(("Subj", Entry::Text(markup.subject.clone())));
    }
    if !markup.rich_text.is_empty() {
        out.push(("RC", Entry::Text(markup.rich_text.clone())));
    }
    // §12.5.6.2 Table 178 names the two reply kinds; Undefined means
    // the file says nothing.
    match markup.reply_type {
        ReplyType::Reply => out.push(("RT", Entry::Name("R".to_string()))),
        ReplyType::Group => out.push(("RT", Entry::Name("Group".to_string()))),
        ReplyType::Undefined => {}
    }
    if markup.opacity < 1.0 {
        out.push(("CA", Entry::Real(markup.opacity)));
    }
    // §12.5.6.2 Table 170: the date the annotation was created, as
    // distinct from `/M`, the date it was last changed.
    if let Some(created) = markup.creation_date.to_pdf_date() {
        out.push(("CreationDate", Entry::Text(created)));
    }
    out
}

/// Every dictionary entry `annotation` would be written with, as raw
/// values. Crate-internal: it lets an extracted annotation carry what
/// its source wrote even though the extracted object is not of the
/// source's concrete type, so writing it back reproduces the same
/// dictionary.
pub(crate) fn snapshot_entries(
    annotation: &dyn Annotation,
) -> Vec<(Vec<u8>, crate::internal::objects::Value)> {
    let mut out: Vec<(Vec<u8>, crate::internal::objects::Value)> =
        printer_mark_entries(annotation.annotation_type())
            .into_iter()
            .chain(common_entries(annotation))
            .chain(markup_entries(annotation))
            .chain(text_markup_entries(annotation))
            .chain(figure_entries(annotation))
            .chain(annotation.pdf_entries())
            .map(|(key, entry)| (key.as_bytes().to_vec(), entry.to_value()))
            .collect();
    // Then whatever the source itself carried and could not name.
    for (key, value) in &annotation.base().carried {
        if !out.iter().any(|(written, _)| written == key) {
            out.push((key.clone(), value.clone()));
        }
    }
    out
}

/// Remove every annotation whose `/NM` name equals `name`.
/// Crate-internal: the editors delete by name, and the canonical
/// collection has no such member.
pub(crate) fn retain_other_names(collection: &AnnotationCollection, name: &str) {
    let before = collection.items.borrow().len();
    collection.items.borrow_mut().retain(|a| a.name() != name);
    if collection.items.borrow().len() != before {
        collection.dirty.set(true);
    }
}

/// The entry §12.5.6.21 gives a printer mark: `/MN`, the name of the
/// mark. It is a function of the annotation's type alone — five types
/// share the `/PrinterMark` subtype — so it is written here rather than
/// five times over.
fn printer_mark_entries(
    annotation_type: AnnotationType,
) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
    use crate::internal::annotation_io::Entry;
    match annotation_type.printer_mark_name() {
        Some(name) => vec![("MN", Entry::Name(name.to_string()))],
        None => Vec::new(),
    }
}

/// The entry §12.5.6.10 gives a text-markup annotation: the
/// quadrilaterals it marks, four points each, flattened into one
/// number array. A highlight with no `/QuadPoints` marks nothing — the
/// rectangle alone does not say which runs of text are covered — so an
/// empty set writes no entry and a viewer draws nothing, which is what
/// the caller asked for.
fn text_markup_entries(
    annotation: &dyn Annotation,
) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
    use crate::internal::annotation_io::Entry;
    let Some(text_markup) = annotation.text_markup_base() else { return Vec::new() };
    if text_markup.quad_points.is_empty() {
        return Vec::new();
    }
    let flat = text_markup.quad_points.iter().flat_map(|p| [p.x(), p.y()]).collect();
    vec![("QuadPoints", Entry::Reals(flat))]
}

/// The entries §12.5.6.8 gives a figure annotation: the colour filling
/// its interior and, as `/RD`, the four differences between the
/// annotation rectangle and the figure drawn inside it. The carrier
/// holds those differences as a rectangle, whose four components are
/// the left, bottom, right and top insets in the order §12.5.6.8 gives
/// them; all-zero is the figure filling its rectangle, which is what
/// an absent `/RD` already means.
fn figure_entries(
    annotation: &dyn Annotation,
) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
    use crate::internal::annotation_io::Entry;
    let Some(figure) = annotation.figure_base() else { return Vec::new() };
    let mut out = Vec::new();
    if !figure.interior_color.is_empty() {
        let (r, g, b) = figure.interior_color.rgb();
        out.push(("IC", Entry::Reals(vec![r, g, b])));
    }
    let f = figure.frame;
    if f.llx() != 0.0 || f.lly() != 0.0 || f.urx() != 0.0 || f.ury() != 0.0 {
        out.push(("RD", Entry::Reals(vec![f.llx(), f.lly(), f.urx(), f.ury()])));
    }
    out
}

/// The entries §12.5.2 gives every annotation, read off the shared
/// base: the colour a viewer draws it in and the flags that decide
/// whether it is shown or printed at all. Each is written only when it
/// carries something — a transparent colour is the absence of a
/// colour, not black, and `Default` flags mean the entry is omitted.
///
/// `/M`, the modification date, is written only when the caller gave
/// one: [`DateTime`](crate::DateTime) formats a PDF date and answers
/// nothing for a date that was never set, so an annotation is never
/// stamped with a time nobody stated.
fn common_entries(
    annotation: &dyn Annotation,
) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
    use crate::internal::annotation_io::Entry;
    let mut out = Vec::new();
    let colour = annotation.color();
    if !colour.is_empty() {
        let (r, g, b) = colour.rgb();
        out.push(("C", Entry::Reals(vec![r, g, b])));
    }
    let flags = annotation.flags() as i64;
    if flags != 0 {
        out.push(("F", Entry::Int(flags)));
    }
    // §12.5.2 Table 164: a date or text string. An unset date formats
    // to nothing, which is what keeps the entry out of an annotation
    // the caller gave no date for.
    if let Some(modified) = annotation.modified().to_pdf_date() {
        out.push(("M", Entry::Text(modified)));
    }
    out.extend(border_entries(annotation.border()));
    out
}

/// §12.5.4's border style dictionary, read off any annotation — a form
/// field included, since a field is written as a merged field / widget
/// object (§12.7.3.1) and carries the same entry.
///
/// The width, the style name and — for a dashed style — the pattern,
/// plus `/BE` for a cloudy border. Only the corner radii stay unwritten:
/// they live in the older `/Border` array, which §12.5.2 makes `/BS`
/// supersede, so an array written beside the dictionary would be inert
/// (see `Border::h_corner_radius`).
pub(crate) fn border_entries(
    border: crate::public::border::Border,
) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
    use crate::internal::annotation_io::Entry;
    let mut out: Vec<(&'static str, Entry)> = Vec::new();
    if border.width() != 0 {
        let mut bs = vec![
            ("W", Entry::Int(border.width() as i64)),
            ("S", Entry::Name(border_style_name(border.style()).to_string())),
        ];
        // Table 168: /D is the dash array a dashed style is drawn with.
        // An all-zero pattern strokes nothing, so it is the unset state
        // rather than an instruction.
        let dash: Vec<i64> = border.dash().pattern().iter().map(|n| *n as i64).collect();
        if dash.iter().any(|n| *n > 0) {
            bs.push(("D", Entry::Array(dash.into_iter().map(Entry::Int).collect())));
        }
        out.push(("BS", Entry::Dict(bs)));
    }
    // §12.5.4 Table 167: /BE is the border effect — a cloudy border and
    // how strongly it waves. It is a sibling of /BS, not part of an
    // appearance stream, so a viewer honours it whether or not this
    // crate draws anything itself.
    if border.effect() == crate::public::border_effect::BorderEffect::Cloudy {
        let mut be = vec![("S", Entry::Name("C".to_string()))];
        // Intensity 0 is the default and says nothing.
        if border.effect_intensity() != 0 {
            be.push(("I", Entry::Int(border.effect_intensity() as i64)));
        }
        out.push(("BE", Entry::Dict(be)));
    }
    out
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::public::annotation::{sealed::HasBase, AnnotationBase};

    struct Named(AnnotationBase);

    impl Named {
        fn make(name: &str) -> Box<dyn Annotation> {
            let mut base = AnnotationBase::default();
            base.name = name.to_string();
            Box::new(Named(base))
        }
    }
    impl HasBase for Named {
        fn base(&self) -> &AnnotationBase {
            &self.0
        }
        fn base_mut(&mut self) -> &mut AnnotationBase {
            &mut self.0
        }
    }
    impl Annotation for Named {
        fn annotation_type(&self) -> AnnotationType {
            AnnotationType::Text
        }
    }

    #[test]
    fn add_count_index_and_lookup() {
        let c = AnnotationCollection::new();
        assert_eq!(c.count(), 0);
        c.add(Named::make("a1"));
        c.add(Named::make("a2"));
        c.add_with_appearance(Named::make("a3"), true);
        assert_eq!(c.count(), 3);
        assert_eq!(c.get(0).unwrap().name(), "a1");
        assert_eq!(c.get(2).unwrap().name(), "a3");
        assert!(c.get(3).is_err());
        assert!(c.find_by_name("a2").is_some());
        assert!(c.find_by_name("missing").is_none());
    }

    #[test]
    fn contains_remove_and_delete() {
        let c = AnnotationCollection::new();
        c.add(Named::make("a1"));
        c.add(Named::make("a2"));
        let probe = Named::make("a1");
        assert!(c.contains(probe.as_ref()));
        assert!(c.remove(probe.as_ref()));
        assert!(!c.contains(probe.as_ref()));
        assert!(!c.remove(probe.as_ref())); // already gone
        assert_eq!(c.count(), 1);

        c.delete_at(0).unwrap();
        assert!(c.delete_at(0).is_err());
        assert_eq!(c.count(), 0);
    }

    #[test]
    fn shared_handle_sees_mutations() {
        // A clone shares the same storage (the page/document semantics).
        let a = AnnotationCollection::new();
        let b = a.clone();
        a.add(Named::make("x"));
        assert_eq!(b.count(), 1);
    }
}
