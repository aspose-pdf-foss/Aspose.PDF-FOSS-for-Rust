//! `Aspose.Pdf.Annotations.Measure` — the measurement parameters of
//! a measure / dimension annotation, per `capabilities/measure.yaml`.
//!
//! A thin value view. The six per-axis `NumberFormatList` formats
//! (`XFormat` … `SlopeFormat`), `ScaleRatio`, `Origin`, and
//! `XYFactor` ship; the `Annotation`-taking constructor is dropped
//! (see `targets/rust/drops.yaml`) — a `Measure` is reached via
//! [`Annotation::measure`].
//!
//! [`Annotation::measure`]: crate::Annotation
//!
//! **Written as §12.9's measurement dictionary.** The scale ratio, the
//! origin, the x/y factor and each per-axis format list reach the file
//! as `/Measure`, on the annotations the format gives one — a line, a
//! polygon and a polyline. Every entry of it is a plain dictionary
//! value, so none of it needs an object of its own.
//!
//! Two things are left out on purpose, both because writing them would
//! say something untrue. `/O` and `/CYX` are omitted at their defaults:
//! an origin of (0, 0) is the page's own origin, which is what no
//! origin means, and Table 261 makes the x/y factor meaningful only
//! beside a `/Y` array. And a number format with no unit label or a
//! zero conversion factor is left out of its array entirely — Table 262
//! makes `/U` and `/C` required, so such an entry is a placeholder
//! rather than a format.

use crate::public::number_format_list::NumberFormatList;
use crate::public::point::Point;

/// The measurement parameters of a measure / dimension annotation.
#[derive(Debug, Clone, PartialEq)]
pub struct Measure {
    scale_ratio: String,
    origin: Point,
    xy_factor: f64,
    x_format: NumberFormatList,
    y_format: NumberFormatList,
    distance_format: NumberFormatList,
    area_format: NumberFormatList,
    angle_format: NumberFormatList,
    slope_format: NumberFormatList,
}

impl Default for Measure {
    fn default() -> Self {
        Measure {
            scale_ratio: String::new(),
            origin: Point::new(0.0, 0.0),
            xy_factor: 0.0,
            x_format: NumberFormatList::default(),
            y_format: NumberFormatList::default(),
            distance_format: NumberFormatList::default(),
            area_format: NumberFormatList::default(),
            angle_format: NumberFormatList::default(),
            slope_format: NumberFormatList::default(),
        }
    }
}

impl Measure {
    /// The scale-ratio string, e.g. `"1 in = 1 mi"` (canonical
    /// `ScaleRatio`).
    pub fn scale_ratio(&self) -> &str {
        &self.scale_ratio
    }
    /// Set the scale-ratio string.
    pub fn set_scale_ratio(&mut self, value: String) {
        self.scale_ratio = value;
    }

    /// The measurement origin in default user space (canonical
    /// `Origin`).
    pub fn origin(&self) -> Point {
        self.origin
    }
    /// Set the measurement origin.
    pub fn set_origin(&mut self, value: Point) {
        self.origin = value;
    }

    /// The ratio of x-axis to y-axis measurement units (canonical
    /// `XYFactor`).
    pub fn xy_factor(&self) -> f64 {
        self.xy_factor
    }
    /// Set the x-to-y unit ratio.
    pub fn set_xy_factor(&mut self, value: f64) {
        self.xy_factor = value;
    }

    /// The x-axis number format list (canonical `XFormat`).
    pub fn x_format(&self) -> &NumberFormatList {
        &self.x_format
    }
    /// Set the x-axis number format list.
    pub fn set_x_format(&mut self, value: NumberFormatList) {
        self.x_format = value;
    }

    /// The y-axis number format list (canonical `YFormat`).
    pub fn y_format(&self) -> &NumberFormatList {
        &self.y_format
    }
    /// Set the y-axis number format list.
    pub fn set_y_format(&mut self, value: NumberFormatList) {
        self.y_format = value;
    }

    /// The distance number format list (canonical `DistanceFormat`).
    pub fn distance_format(&self) -> &NumberFormatList {
        &self.distance_format
    }
    /// Set the distance number format list.
    pub fn set_distance_format(&mut self, value: NumberFormatList) {
        self.distance_format = value;
    }

    /// The area number format list (canonical `AreaFormat`).
    pub fn area_format(&self) -> &NumberFormatList {
        &self.area_format
    }
    /// Set the area number format list.
    pub fn set_area_format(&mut self, value: NumberFormatList) {
        self.area_format = value;
    }

    /// The angle number format list (canonical `AngleFormat`).
    pub fn angle_format(&self) -> &NumberFormatList {
        &self.angle_format
    }
    /// Set the angle number format list.
    pub fn set_angle_format(&mut self, value: NumberFormatList) {
        self.angle_format = value;
    }

    /// The slope number format list (canonical `SlopeFormat`).
    pub fn slope_format(&self) -> &NumberFormatList {
        &self.slope_format
    }
    /// Set the slope number format list.
    pub fn set_slope_format(&mut self, value: NumberFormatList) {
        self.slope_format = value;
    }
}

/// §12.9 Table 261: the `/Measure` dictionary of a measurement
/// annotation, or `None` when the parameters say nothing a viewer could
/// measure with.
///
/// Every entry of it is a plain dictionary value — the per-axis format
/// lists are arrays of dictionaries rather than indirect objects — so
/// the whole dictionary is writable inside the annotation that carries
/// it, with no second object needed.
///
/// `/Type` and `/Subtype` are both optional, and both are written: `RL`
/// is the one measurement subtype the format defines, and naming it
/// costs nothing beside the axes.
pub(crate) fn measure_entries(
    measure: &Measure,
) -> Option<crate::internal::annotation_io::Entry> {
    use crate::internal::annotation_io::Entry;
    let mut out: Vec<(&'static str, Entry)> = Vec::new();

    if !measure.scale_ratio.is_empty() {
        out.push(("R", Entry::Text(measure.scale_ratio.clone())));
    }
    // Table 261 names one format array per axis: x and y change, then
    // distance, area, angle and slope.
    for (key, list) in [
        ("X", &measure.x_format),
        ("Y", &measure.y_format),
        ("D", &measure.distance_format),
        ("A", &measure.area_format),
        ("T", &measure.angle_format),
        ("S", &measure.slope_format),
    ] {
        if let Some(array) = number_format_array(list) {
            out.push((key, array));
        }
    }
    // The origin is in default user space, so the default (0, 0) is the
    // page's own origin — which is what "no origin given" means, and
    // writing it would say nothing.
    if measure.origin.x() != 0.0 || measure.origin.y() != 0.0 {
        out.push(("O", Entry::Reals(vec![measure.origin.x(), measure.origin.y()])));
    }
    // Table 261: /CYX is meaningful only when /Y is present, being the
    // factor that converts the y axis's largest unit to the x axis's.
    if measure.xy_factor != 0.0 && number_format_array(&measure.y_format).is_some() {
        out.push(("CYX", Entry::Real(measure.xy_factor)));
    }

    if out.is_empty() {
        return None;
    }
    let mut dict = vec![
        ("Type", Entry::Name("Measure".to_string())),
        ("Subtype", Entry::Name("RL".to_string())),
    ];
    dict.extend(out);
    Some(Entry::Dict(dict))
}

/// One axis's format list as an array of §12.9 Table 262 number-format
/// dictionaries, or `None` when none of its entries is complete.
///
/// Table 262 makes `/U` and `/C` required, so a default-constructed
/// format — no unit label and a zero conversion factor — is a
/// placeholder rather than a format, and writing it would produce a
/// dictionary the format calls invalid. Such an entry is left out; a
/// list of nothing but those produces no array at all.
fn number_format_array(
    list: &NumberFormatList,
) -> Option<crate::internal::annotation_io::Entry> {
    use crate::internal::annotation_io::Entry;
    let formats: Vec<Entry> = (0..list.count())
        .filter_map(|i| list.at(i))
        .filter_map(number_format_dict)
        .collect();
    (!formats.is_empty()).then_some(Entry::Array(formats))
}

/// One §12.9 Table 262 number-format dictionary.
fn number_format_dict(
    format: &crate::public::number_format::NumberFormat,
) -> Option<crate::internal::annotation_io::Entry> {
    use crate::internal::annotation_io::Entry;
    use crate::public::fraction_style::FractionStyle;

    if format.unit_label().is_empty() || format.convresion_factor() == 0.0 {
        return None;
    }
    let style = format.fraction_displayment();
    let mut dict = vec![
        ("Type", Entry::Name("NumberFormat".to_string())),
        ("U", Entry::Text(format.unit_label().to_string())),
        ("C", Entry::Real(format.convresion_factor())),
        (
            "F",
            Entry::Name(
                match style {
                    FractionStyle::ShowAsDecimal => "D",
                    FractionStyle::ShowAsFraction => "F",
                    FractionStyle::Round => "R",
                    FractionStyle::Truncate => "T",
                }
                .to_string(),
            ),
        ),
    ];
    // Table 262 gives the two fractional carriers one key: /D is "a
    // positive integer denominator, constrained to be a power of 10
    // when the value of F is D". So a decimal format's /D is its
    // precision and a fractional format's is its denominator, which is
    // what the two properties are; rounding and truncating have no
    // fractional part for either to describe.
    let fractional = match style {
        FractionStyle::ShowAsDecimal => format.precision(),
        FractionStyle::ShowAsFraction => format.denominator(),
        FractionStyle::Round | FractionStyle::Truncate => 0,
    };
    if fractional > 0 {
        dict.push(("D", Entry::Int(fractional as i64)));
    }
    if format.force_denominator() {
        dict.push(("FD", Entry::Bool(true)));
    }
    // The separators and the surrounding text each have a Table 262
    // default (a comma, a period, a space either side), so an empty
    // string is the caller saying nothing rather than asking for one.
    for (key, text) in [
        ("RT", format.thousands_separator()),
        ("RD", format.fraction_separator()),
        ("PS", format.before_text()),
        ("SS", format.after_text()),
    ] {
        if !text.is_empty() {
            dict.push((key, Entry::Text(text.to_string())));
        }
    }
    Some(Entry::Dict(dict))
}
