//! `Aspose.Pdf.Annotations.AnnotationType` — annotation enum, per `capabilities/annotation_type.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The annotation subtype, one variant per kind this crate models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
#[allow(clippy::upper_case_acronyms)]
pub enum AnnotationType {
    /// A sticky note, shown as an icon until it is opened.
    #[default]
    Text = 0,
    /// An ellipse drawn within the annotation rectangle.
    Circle = 1,
    /// A closed shape through a list of vertices.
    Polygon = 2,
    /// An open path through a list of vertices.
    PolyLine = 3,
    /// A single straight line, optionally with end shapes and a caption.
    Line = 4,
    /// A rectangle drawn within the annotation rectangle.
    Square = 5,
    /// Text drawn directly on the page rather than in a pop-up.
    FreeText = 6,
    /// A highlight over a run of text.
    Highlight = 7,
    /// An underline beneath a run of text.
    Underline = 8,
    /// A wavy underline beneath a run of text.
    Squiggly = 9,
    /// A line struck through a run of text.
    StrikeOut = 10,
    /// A mark showing where text was inserted.
    Caret = 11,
    /// A freehand path, drawn from one or more lists of points.
    Ink = 12,
    /// A region that follows a destination or runs an action.
    Link = 13,
    /// The pop-up window belonging to another annotation.
    Popup = 14,
    /// An embedded file, shown as an icon.
    FileAttachment = 15,
    /// A sound to play, shown as an icon.
    Sound = 16,
    /// A movie to play.
    Movie = 17,
    /// A region that plays embedded media.
    Screen = 18,
    /// The on-page appearance of a form field.
    Widget = 19,
    /// Content drawn at a fixed size regardless of how the page is scaled.
    Watermark = 20,
    /// Trapping information for prepress.
    TrapNet = 21,
    /// A mark added for prepress, outside the trimmed page.
    PrinterMark = 22,
    /// A region marked for its content to be removed.
    Redaction = 23,
    /// A named or custom stamp.
    Stamp = 24,
    /// Embedded rich media.
    RichMedia = 25,
    /// A subtype this crate does not model.
    Unknown = 26,
    /// An embedded three-dimensional artwork.
    PDF3D = 27,
    /// A printer mark showing process-colour patches.
    ColorBar = 28,
    /// A printer mark showing where the page is trimmed.
    TrimMark = 29,
    /// A printer mark showing the bleed boundary.
    BleedMark = 30,
    /// A printer mark used to align the printing plates.
    RegistrationMark = 31,
    /// A printer mark carrying page identification text.
    PageInformation = 32,
}

impl AnnotationType {
    /// The PDF `/Subtype` name for this annotation type. Most match
    /// the variant name; `Redaction` → `Redact` and `PDF3D` → `3D`
    /// per the PDF specification.
    pub(crate) fn pdf_subtype(self) -> &'static str {
        use AnnotationType::*;
        match self {
            Text => "Text",
            Circle => "Circle",
            Polygon => "Polygon",
            PolyLine => "PolyLine",
            Line => "Line",
            Square => "Square",
            FreeText => "FreeText",
            Highlight => "Highlight",
            Underline => "Underline",
            Squiggly => "Squiggly",
            StrikeOut => "StrikeOut",
            Caret => "Caret",
            Ink => "Ink",
            Link => "Link",
            Popup => "Popup",
            FileAttachment => "FileAttachment",
            Sound => "Sound",
            Movie => "Movie",
            Screen => "Screen",
            Widget => "Widget",
            Watermark => "Watermark",
            TrapNet => "TrapNet",
            PrinterMark | ColorBar | TrimMark | BleedMark | RegistrationMark | PageInformation => {
                "PrinterMark"
            }
            Redaction => "Redact",
            Stamp => "Stamp",
            RichMedia => "RichMedia",
            PDF3D => "3D",
            Unknown => "Unknown",
        }
    }

    /// The `/MN` name §12.5.6.21 gives a printer mark. Five of this
    /// enum's variants share the `/PrinterMark` subtype, so without
    /// `/MN` the file cannot say which mark it is and a reader cannot
    /// tell them apart. `PrinterMark` itself is the unnamed general
    /// case; every other annotation type has no `/MN` at all.
    pub(crate) fn printer_mark_name(self) -> Option<&'static str> {
        use AnnotationType::*;
        match self {
            ColorBar => Some("ColorBar"),
            TrimMark => Some("TrimMark"),
            BleedMark => Some("BleedMark"),
            RegistrationMark => Some("RegistrationMark"),
            PageInformation => Some("PageInformation"),
            _ => None,
        }
    }

    /// The annotation type a `/PrinterMark`'s `/MN` name denotes, or
    /// the general `PrinterMark` when the name is absent or is one this
    /// crate does not model.
    pub(crate) fn from_printer_mark_name(name: &str) -> AnnotationType {
        use AnnotationType::*;
        match name {
            "ColorBar" => ColorBar,
            "TrimMark" => TrimMark,
            "BleedMark" => BleedMark,
            "RegistrationMark" => RegistrationMark,
            "PageInformation" => PageInformation,
            _ => PrinterMark,
        }
    }

    /// The annotation type for a PDF `/Subtype` name (inverse of
    /// [`pdf_subtype`](Self::pdf_subtype); unknown names → `Unknown`).
    pub(crate) fn from_pdf_subtype(name: &str) -> AnnotationType {
        use AnnotationType::*;
        match name {
            "Text" => Text,
            "Circle" => Circle,
            "Polygon" => Polygon,
            "PolyLine" => PolyLine,
            "Line" => Line,
            "Square" => Square,
            "FreeText" => FreeText,
            "Highlight" => Highlight,
            "Underline" => Underline,
            "Squiggly" => Squiggly,
            "StrikeOut" => StrikeOut,
            "Caret" => Caret,
            "Ink" => Ink,
            "Link" => Link,
            "Popup" => Popup,
            "FileAttachment" => FileAttachment,
            "Sound" => Sound,
            "Movie" => Movie,
            "Screen" => Screen,
            "Widget" => Widget,
            "Watermark" => Watermark,
            "TrapNet" => TrapNet,
            "PrinterMark" => PrinterMark,
            "Redact" => Redaction,
            "Stamp" => Stamp,
            "RichMedia" => RichMedia,
            "3D" => PDF3D,
            _ => Unknown,
        }
    }
}
