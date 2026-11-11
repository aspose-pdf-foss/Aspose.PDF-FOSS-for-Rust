//! `Aspose.Pdf.Annotations.AnnotationType` — annotation enum, per `capabilities/annotation_type.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `AnnotationType` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
#[allow(clippy::upper_case_acronyms)]
pub enum AnnotationType {
    /// `Text`.
    #[default]
    Text = 0,
    /// `Circle`.
    Circle = 1,
    /// `Polygon`.
    Polygon = 2,
    /// `PolyLine`.
    PolyLine = 3,
    /// `Line`.
    Line = 4,
    /// `Square`.
    Square = 5,
    /// `FreeText`.
    FreeText = 6,
    /// `Highlight`.
    Highlight = 7,
    /// `Underline`.
    Underline = 8,
    /// `Squiggly`.
    Squiggly = 9,
    /// `StrikeOut`.
    StrikeOut = 10,
    /// `Caret`.
    Caret = 11,
    /// `Ink`.
    Ink = 12,
    /// `Link`.
    Link = 13,
    /// `Popup`.
    Popup = 14,
    /// `FileAttachment`.
    FileAttachment = 15,
    /// `Sound`.
    Sound = 16,
    /// `Movie`.
    Movie = 17,
    /// `Screen`.
    Screen = 18,
    /// `Widget`.
    Widget = 19,
    /// `Watermark`.
    Watermark = 20,
    /// `TrapNet`.
    TrapNet = 21,
    /// `PrinterMark`.
    PrinterMark = 22,
    /// `Redaction`.
    Redaction = 23,
    /// `Stamp`.
    Stamp = 24,
    /// `RichMedia`.
    RichMedia = 25,
    /// `Unknown`.
    Unknown = 26,
    /// `PDF3D`.
    PDF3D = 27,
    /// `ColorBar`.
    ColorBar = 28,
    /// `TrimMark`.
    TrimMark = 29,
    /// `BleedMark`.
    BleedMark = 30,
    /// `RegistrationMark`.
    RegistrationMark = 31,
    /// `PageInformation`.
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
