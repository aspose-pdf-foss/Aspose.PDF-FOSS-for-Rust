// Pure-math foundation for the three PDF Device colour spaces
// (PDF 32000-1:2008 §8.6.4). Three closed-form conversions —
// DeviceGray / DeviceRGB / DeviceCMYK → sRGB triple in [0, 1] —
// plus the round-half-away-from-zero `quantize` helper that turns
// a [0, 1] f64 into a [0, 255] u8.
//
// ICC profile application is the SIBLING `icc_profile` primitive
// (scheduled separately) — splitting the two keeps the spec-derived
// arithmetic here and isolates the colour-management binding behind
// its own freeze gate.

use std::fmt;

/// PDF Device colour-space families. Discriminants double as
/// component counts — `DeviceGray = 1`, `DeviceRGB = 3`,
/// `DeviceCMYK = 4` — so [`component_count`] is a one-line cast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(dead_code)] // retained for a future-phase public-API binding
pub enum Family {
    DeviceGray = 1,
    DeviceRGB = 3,
    DeviceCMYK = 4,
}

/// Equivalent sRGB triple in floating-point [0, 1]. f64 throughout
/// to match the transform / glyph_rasterizer pipeline; consumers
/// quantise to u8 only at the output-device boundary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorRgb {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

/// Errors returned by [`to_srgb`]. [`component_count`] and
/// [`quantize`] are total.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // retained for a future-phase public-API binding
pub enum Error {
    /// `components.len()` did not match the family's component
    /// count. Programmer error — out-of-range numerics are clamped,
    /// not rejected.
    WrongComponentCount {
        family: Family,
        expected: usize,
        got: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::WrongComponentCount { family, expected, got } => write!(
                f,
                "to_srgb: family {} expects {} component{}, got {}",
                family.name(),
                expected,
                if *expected == 1 { "" } else { "s" },
                got,
            ),
        }
    }
}

impl std::error::Error for Error {}

impl Family {
    #[allow(dead_code)] // retained for a future-phase public-API binding
    fn name(self) -> &'static str {
        match self {
            Family::DeviceGray => "DeviceGray",
            Family::DeviceRGB => "DeviceRGB",
            Family::DeviceCMYK => "DeviceCMYK",
        }
    }
}

/// Component count for a family. The `as usize` cast is a no-op
/// at the machine level thanks to `#[repr(u8)]` on `Family`.
#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn component_count(family: Family) -> usize {
    family as u8 as usize
}

/// Convert `components` from `family`'s device space to sRGB per
/// PDF 32000-1:2008 §8.6.4. Out-of-range values are CLAMPED to
/// [0, 1] before computation — §8.6.4 calls this
/// "implementation-defined", and clamping is the consensus
/// convention across qpdf, MuPDF, Poppler, ImageMagick. Wrong
/// `components.len()` is a programmer error and returns
/// [`Error::WrongComponentCount`].
#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn to_srgb(family: Family, components: &[f64]) -> Result<ColorRgb, Error> {
    let expected = component_count(family);
    if components.len() != expected {
        return Err(Error::WrongComponentCount {
            family,
            expected,
            got: components.len(),
        });
    }
    match family {
        Family::DeviceGray => {
            // §8.6.4.2: gray → R = G = B = clamp(gray).
            let v = clamp01(components[0]);
            Ok(ColorRgb { r: v, g: v, b: v })
        }
        Family::DeviceRGB => {
            // §8.6.4.3: per-channel identity. Family is required
            // here so callers don't have to special-case
            // "skip to_srgb on DeviceRGB" — uniform dispatch.
            Ok(ColorRgb {
                r: clamp01(components[0]),
                g: clamp01(components[1]),
                b: clamp01(components[2]),
            })
        }
        Family::DeviceCMYK => {
            // §8.6.4.4 / §10.3.5: subtractive, two-factor product.
            // Missing the `(1 - K)` factor is a documented trap
            // (see semantic_traps.cmyk_missing_k_factor) — the
            // all-K=0 fixtures pass without it but real K > 0 cases
            // wash out.
            let c = clamp01(components[0]);
            let m = clamp01(components[1]);
            let y = clamp01(components[2]);
            let k = clamp01(components[3]);
            let one_minus_k = 1.0 - k;
            Ok(ColorRgb {
                r: (1.0 - c) * one_minus_k,
                g: (1.0 - m) * one_minus_k,
                b: (1.0 - y) * one_minus_k,
            })
        }
    }
}

/// Map a f64 in [0, 1] to a u8 in [0, 255] using
/// **round-half-away-from-zero** — the dominant convention in
/// graphics code (libpng, libjpeg, Cairo, Skia, ImageMagick).
/// `clamp01` absorbs every numeric edge case (NaN, ±Inf), so this
/// function is total. NaN maps to 0 by convention; that's the
/// safest default for colour quantisation since NaN typically
/// signals "missing channel".
///
/// Banker's rounding (Rust's `f64::round_ties_even`) would map
/// 0.5/255 to 0 instead of 1 and is wrong for this use — see
/// semantic_traps.quantize_banker_rounding in the spec yaml.
pub fn quantize(value: f64) -> u8 {
    let c = clamp01(value);
    // `f64::round` is round-half-AWAY-from-zero. `c` is in [0, 1]
    // so the result is in [0, 255]; the saturating cast handles
    // the boundary defensively.
    let q = (c * 255.0).round();
    if q < 0.0 {
        0
    } else if q > 255.0 {
        255
    } else {
        q as u8
    }
}

/// Clamp to [0, 1]. NaN maps to 0 (the lower bound) by spec
/// convention. `f64::clamp` propagates NaN, so the explicit
/// `is_nan` check is required.
#[allow(clippy::manual_clamp, clippy::if_same_then_else)]
fn clamp01(v: f64) -> f64 {
    if v.is_nan() {
        0.0
    } else if v < 0.0 {
        0.0
    } else if v > 1.0 {
        1.0
    } else {
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_count_matches_discriminant() {
        assert_eq!(component_count(Family::DeviceGray), 1);
        assert_eq!(component_count(Family::DeviceRGB), 3);
        assert_eq!(component_count(Family::DeviceCMYK), 4);
    }

    #[test]
    fn gray_is_identity_on_each_channel() {
        let rgb = to_srgb(Family::DeviceGray, &[0.5]).unwrap();
        assert_eq!(rgb, ColorRgb { r: 0.5, g: 0.5, b: 0.5 });
    }

    #[test]
    fn rgb_is_per_channel_identity() {
        let rgb = to_srgb(Family::DeviceRGB,
                          &[0.25, 0.5, 0.75]).unwrap();
        assert_eq!(rgb, ColorRgb { r: 0.25, g: 0.5, b: 0.75 });
    }

    #[test]
    fn cmyk_pure_black_maps_to_black() {
        // K=1 forces every output channel to zero regardless of
        // C/M/Y. Catches the missing-(1-K)-factor trap.
        let rgb = to_srgb(Family::DeviceCMYK,
                          &[0.0, 0.0, 0.0, 1.0]).unwrap();
        assert_eq!(rgb, ColorRgb { r: 0.0, g: 0.0, b: 0.0 });
    }

    #[test]
    fn cmyk_pure_red_picks_correct_channels() {
        // C=0, M=1, Y=1, K=0 → red. Catches C↔Y / M↔Y swaps.
        let rgb = to_srgb(Family::DeviceCMYK,
                          &[0.0, 1.0, 1.0, 0.0]).unwrap();
        assert_eq!(rgb, ColorRgb { r: 1.0, g: 0.0, b: 0.0 });
    }

    #[test]
    fn out_of_range_inputs_are_clamped_on_input() {
        // §8.6.4 says out-of-range "produces implementation-defined
        // output"; we clamp on input, which keeps the (1-C)*(1-K)
        // product within [0, 1]. Catches "clamp on output".
        let rgb = to_srgb(Family::DeviceCMYK,
                          &[-0.5, 0.5, 0.5, 0.5]).unwrap();
        // After clamp: C=0, M=0.5, Y=0.5, K=0.5
        // R=(1-0)*(1-0.5)=0.5; G=(1-0.5)*(1-0.5)=0.25; B=0.25
        assert_eq!(rgb, ColorRgb { r: 0.5, g: 0.25, b: 0.25 });
    }

    #[test]
    fn nan_clamps_to_zero() {
        // K=NaN → clamped to 0, so output is just (1-C)·1 etc.
        let rgb = to_srgb(Family::DeviceCMYK,
                          &[0.5, 0.5, 0.5, f64::NAN]).unwrap();
        assert_eq!(rgb, ColorRgb { r: 0.5, g: 0.5, b: 0.5 });
    }

    #[test]
    fn wrong_component_count_errors() {
        let err = to_srgb(Family::DeviceCMYK, &[0.5, 0.5]).unwrap_err();
        assert_eq!(
            err,
            Error::WrongComponentCount {
                family: Family::DeviceCMYK,
                expected: 4,
                got: 2,
            });
    }

    #[test]
    fn quantize_half_rounds_up() {
        // Round-half-AWAY-from-zero: 0.5 * 255 = 127.5 → 128.
        // Banker's would give 128 here too (128 is even); the
        // half-LSB cases below distinguish the two.
        assert_eq!(quantize(0.5), 128);
    }

    #[test]
    fn quantize_half_lsb_rounds_up_not_to_even() {
        // 0.5/255 = 0.001960... → 0.5 → away-from-zero gives 1.
        // Banker's would give 0 (round to even).
        assert_eq!(quantize(0.5 / 255.0), 1);
    }

    #[test]
    fn quantize_extremes() {
        assert_eq!(quantize(0.0), 0);
        assert_eq!(quantize(1.0), 255);
        assert_eq!(quantize(-1.0), 0);
        assert_eq!(quantize(2.0), 255);
        assert_eq!(quantize(f64::NAN), 0);
        assert_eq!(quantize(f64::INFINITY), 255);
        assert_eq!(quantize(f64::NEG_INFINITY), 0);
    }
}
