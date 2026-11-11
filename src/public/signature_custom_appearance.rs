//! `Aspose.Pdf.Forms.SignatureCustomAppearance` — the visual
//! appearance options for a signature, per
//! `capabilities/signature_custom_appearance.yaml`.
//!
//! A pure value type. `Culture` (`System.Globalization.CultureInfo`)
//! is dropped.

use crate::public::color::Color;
use crate::public::rotation::Rotation;
use crate::public::subject_name_elements::SubjectNameElements;

/// The visual appearance options for a digital signature.
#[derive(Debug, Clone, PartialEq)]
pub struct SignatureCustomAppearance {
    is_foreground_image: bool,
    font_family_name: String,
    font_size: f64,
    foreground_color: Color,
    background_color: Color,
    show_contact_info: bool,
    show_reason: bool,
    show_location: bool,
    contact_info_label: String,
    reason_label: String,
    location_label: String,
    digital_signed_label: String,
    use_digital_subject_format: bool,
    digital_subject_format: Vec<SubjectNameElements>,
    date_signed_at_label: String,
    date_time_local_format: String,
    date_time_format: String,
    rotation: Rotation,
}

impl Default for SignatureCustomAppearance {
    fn default() -> Self {
        Self {
            is_foreground_image: false,
            font_family_name: String::new(),
            font_size: 0.0,
            foreground_color: Color::black(),
            background_color: Color::empty(),
            show_contact_info: false,
            show_reason: false,
            show_location: false,
            contact_info_label: String::new(),
            reason_label: String::new(),
            location_label: String::new(),
            digital_signed_label: String::new(),
            use_digital_subject_format: false,
            digital_subject_format: Vec::new(),
            date_signed_at_label: String::new(),
            date_time_local_format: String::new(),
            date_time_format: String::new(),
            rotation: Rotation::None,
        }
    }
}

macro_rules! prop {
    ($get:ident, $set:ident, $ty:ty, $doc:literal) => {
        #[doc = $doc]
        pub fn $get(&self) -> $ty {
            self.$get
        }
        #[doc = concat!("Set the ", $doc)]
        pub fn $set(&mut self, value: $ty) {
            self.$get = value;
        }
    };
}

macro_rules! str_prop {
    ($get:ident, $set:ident, $doc:literal) => {
        #[doc = $doc]
        pub fn $get(&self) -> &str {
            &self.$get
        }
        #[doc = concat!("Set the ", $doc)]
        pub fn $set(&mut self, value: String) {
            self.$get = value;
        }
    };
}

impl SignatureCustomAppearance {
    /// Construct default appearance options.
    pub fn new() -> Self {
        Self::default()
    }

    prop!(is_foreground_image, set_is_foreground_image, bool, "whether the appearance image is drawn in the foreground.");
    str_prop!(font_family_name, set_font_family_name, "appearance font family name.");
    prop!(font_size, set_font_size, f64, "appearance font size.");
    prop!(foreground_color, set_foreground_color, Color, "foreground colour.");
    prop!(background_color, set_background_color, Color, "background colour.");
    prop!(show_contact_info, set_show_contact_info, bool, "whether the contact info is shown.");
    prop!(show_reason, set_show_reason, bool, "whether the reason is shown.");
    prop!(show_location, set_show_location, bool, "whether the location is shown.");
    str_prop!(contact_info_label, set_contact_info_label, "contact-info label.");
    str_prop!(reason_label, set_reason_label, "reason label.");
    str_prop!(location_label, set_location_label, "location label.");
    str_prop!(digital_signed_label, set_digital_signed_label, "digitally-signed label.");
    prop!(use_digital_subject_format, set_use_digital_subject_format, bool, "whether the digital-subject format is used.");
    str_prop!(date_signed_at_label, set_date_signed_at_label, "date-signed-at label.");
    str_prop!(date_time_local_format, set_date_time_local_format, "local date/time format.");
    str_prop!(date_time_format, set_date_time_format, "date/time format.");
    prop!(rotation, set_rotation, Rotation, "appearance rotation.");

    /// The subject-name elements shown in the digital-subject format.
    pub fn digital_subject_format(&self) -> &[SubjectNameElements] {
        &self.digital_subject_format
    }
    /// Set the subject-name elements shown in the digital-subject
    /// format.
    pub fn set_digital_subject_format(&mut self, value: Vec<SubjectNameElements>) {
        self.digital_subject_format = value;
    }
}
