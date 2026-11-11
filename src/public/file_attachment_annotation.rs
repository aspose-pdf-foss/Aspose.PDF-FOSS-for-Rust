//! `Aspose.Pdf.Annotations.FileAttachmentAnnotation` — an embedded-
//! file attachment annotation, per
//! `capabilities/file_attachment_annotation.yaml`.
//!
//! Extends [`MarkupAnnotation`]. `Opacity` is inherited from the
//! markup base.
//!
//! §12.5.6.15's required `/FS` is written as a file-specification
//! dictionary naming the file (`/F`, `/UF`, `/Desc`, `/AFRelationship`,
//! `/FS`), together with the `/Name` icon.
//!
//! **Carried, not written.** The file's *bytes* are not embedded here:
//! an embedded file lives in an `/EF` stream, a second object, and the
//! annotation writer emits one dictionary per annotation. The
//! attachment therefore names an external file. To embed bytes, add the
//! specification to [`Document::embedded_files`](crate::Document), which
//! writes the stream.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::file_icon::FileIcon;
use crate::public::file_specification::FileSpecification;
use crate::public::markup_annotation::sealed::HasMarkupBase;
use crate::public::markup_annotation::{MarkupAnnotation, MarkupBase};
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;

/// A file-attachment annotation referencing an embedded file.
pub struct FileAttachmentAnnotation {
    base: AnnotationBase,
    markup: MarkupBase,
    file: FileSpecification,
    icon: FileIcon,
}

impl FileAttachmentAnnotation {
    /// Construct on a [`Page`] at the given rectangle for the given
    /// file.
    pub fn on_page(page: &Page, rect: Rectangle, file: FileSpecification) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self {
            base,
            markup: MarkupBase::new(),
            file,
            icon: FileIcon::PushPin,
        }
    }

    /// The attached file specification.
    pub fn file(&self) -> &FileSpecification {
        &self.file
    }
    /// Set the attached file specification.
    pub fn set_file(&mut self, value: FileSpecification) {
        self.file = value;
    }

    /// The attachment icon.
    pub fn icon(&self) -> FileIcon {
        self.icon
    }
    /// Set the attachment icon.
    pub fn set_icon(&mut self, value: FileIcon) {
        self.icon = value;
    }
}

impl HasBase for FileAttachmentAnnotation {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
    fn pdf_entries(&self) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        // §12.5.6.15: /FS is the one entry the table marks REQUIRED —
        // the file specification the annotation attaches — and /Name is
        // the icon a viewer draws for it (/PushPin by default, which an
        // absent entry already means).
        let mut out = Vec::new();
        let file = &self.file;
        let mut fs: Vec<(&'static str, Entry)> =
            vec![("Type", Entry::Name("Filespec".to_string()))];
        if !file.name().is_empty() {
            fs.push(("F", Entry::Text(file.name().to_string())));
        }
        if !file.unicode_name().is_empty() {
            fs.push(("UF", Entry::Text(file.unicode_name().to_string())));
        }
        if !file.description().is_empty() {
            fs.push(("Desc", Entry::Text(file.description().to_string())));
        }
        if let Some(rel) = file.af_relationship().pdf_name() {
            fs.push(("AFRelationship", Entry::Name(rel.to_string())));
        }
        if !file.file_system().is_empty() {
            fs.push(("FS", Entry::Name(file.file_system().to_string())));
        }
        if fs.len() > 1 {
            out.push(("FS", Entry::Dict(fs)));
        }
        if self.icon != FileIcon::PushPin {
            out.push(("Name", Entry::Name(file_icon_name(self.icon).to_string())));
        }
        out
    }
    fn markup_base(&self) -> Option<&crate::public::markup_annotation::MarkupBase> {
        Some(&self.markup)
    }
}
impl HasMarkupBase for FileAttachmentAnnotation {
    fn markup(&self) -> &MarkupBase {
        &self.markup
    }
    fn markup_mut(&mut self) -> &mut MarkupBase {
        &mut self.markup
    }
}
impl Annotation for FileAttachmentAnnotation {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::FileAttachment
    }
}
impl MarkupAnnotation for FileAttachmentAnnotation {}

/// The name PDF gives a file-attachment icon (§12.5.6.15 Table 184).
fn file_icon_name(i: FileIcon) -> &'static str {
    match i {
        FileIcon::PushPin => "PushPin",
        FileIcon::Graph => "Graph",
        FileIcon::Paperclip => "Paperclip",
        FileIcon::Tag => "Tag",
    }
}
