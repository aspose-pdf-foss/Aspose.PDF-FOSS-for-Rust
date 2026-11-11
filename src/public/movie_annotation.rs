//! `Aspose.Pdf.Annotations.MovieAnnotation` — a movie annotation,
//! per `capabilities/movie_annotation.yaml`.
//!
//! Extends [`Annotation`] directly.
//!
//! §12.5.6.17's `/T` and the required `/Movie` dictionary (§13.4) are
//! written on save: the clip's file specification, its aspect, its
//! rotation and whether a poster is shown.
//!
//! **Carried, not written.** The clip's *bytes* are not embedded — a
//! media file lives in a stream, a second object, and the annotation
//! writer emits one dictionary per annotation — so `/Movie /F` names an
//! external file. `/A`, the activation dictionary, has no property on
//! this carrier to be written from.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::document::Document;
use crate::public::file_specification::FileSpecification;
use crate::public::page::Page;
use crate::public::point::Point;
use crate::public::rectangle::Rectangle;

/// A movie annotation referencing a media clip.
pub struct MovieAnnotation {
    base: AnnotationBase,
    title: String,
    file: FileSpecification,
    poster: bool,
    aspect: Point,
    rotate: i32,
}

impl MovieAnnotation {
    fn build(base: AnnotationBase, movie_file: String) -> Self {
        Self {
            base,
            title: String::new(),
            file: FileSpecification::with_file(movie_file),
            poster: false,
            aspect: Point::new(0.0, 0.0),
            rotate: 0,
        }
    }

    /// Construct within a [`Document`] for the given movie file.
    pub fn new(_document: &Document, movie_file: String) -> Self {
        Self::build(AnnotationBase::default(), movie_file)
    }

    /// Construct on a [`Page`] at the given rectangle for the movie
    /// file.
    pub fn on_page(page: &Page, rect: Rectangle, movie_file: String) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self::build(base, movie_file)
    }

    /// The movie title.
    pub fn title(&self) -> &str {
        &self.title
    }
    /// Set the movie title.
    pub fn set_title(&mut self, value: String) {
        self.title = value;
    }

    /// The media file specification.
    pub fn file(&self) -> &FileSpecification {
        &self.file
    }
    /// Set the media file specification.
    pub fn set_file(&mut self, value: FileSpecification) {
        self.file = value;
    }

    /// Whether the poster frame is shown.
    pub fn poster(&self) -> bool {
        self.poster
    }
    /// Set whether the poster frame is shown.
    pub fn set_poster(&mut self, value: bool) {
        self.poster = value;
    }

    /// The aspect ratio (width / height as a point).
    pub fn aspect(&self) -> Point {
        self.aspect
    }
    /// Set the aspect ratio.
    pub fn set_aspect(&mut self, value: Point) {
        self.aspect = value;
    }

    /// The playback rotation, in degrees.
    pub fn rotate(&self) -> i32 {
        self.rotate
    }
    /// Set the playback rotation.
    pub fn set_rotate(&mut self, value: i32) {
        self.rotate = value;
    }
}

impl HasBase for MovieAnnotation {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
    fn pdf_entries(&self) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        // §12.5.6.17 Table 186: /T titles the annotation and /Movie —
        // the required entry — is the clip itself. §13.4 Table 294
        // gives that dictionary /F (its own required entry, the file),
        // /Aspect as the clip's pixel dimensions, /Rotate in degrees
        // and /Poster, which `true` asks a viewer to draw from the
        // clip's first frame.
        let mut out = Vec::new();
        if !self.title.is_empty() {
            out.push(("T", Entry::Text(self.title.clone())));
        }
        let mut movie: Vec<(&'static str, Entry)> = Vec::new();
        if !self.file.name().is_empty() {
            movie.push((
                "F",
                Entry::Dict(vec![
                    ("Type", Entry::Name("Filespec".to_string())),
                    ("F", Entry::Text(self.file.name().to_string())),
                ]),
            ));
        }
        if self.aspect.x() != 0.0 || self.aspect.y() != 0.0 {
            movie.push((
                "Aspect",
                Entry::Array(vec![
                    Entry::Int(self.aspect.x() as i64),
                    Entry::Int(self.aspect.y() as i64),
                ]),
            ));
        }
        if self.rotate != 0 {
            movie.push(("Rotate", Entry::Int(self.rotate as i64)));
        }
        if self.poster {
            movie.push(("Poster", Entry::Bool(true)));
        }
        if !movie.is_empty() {
            out.push(("Movie", Entry::Dict(movie)));
        }
        out
    }
}
impl Annotation for MovieAnnotation {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::Movie
    }
}
