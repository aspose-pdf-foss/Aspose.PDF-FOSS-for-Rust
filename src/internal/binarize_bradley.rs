//! Bradley adaptive thresholding (Bradley & Roth, 2007).
//!
//! 8-bit grayscale → 1-bit bilevel via integral-image-based
//! local mean + integer cross-multiplication. Used by
//! [`crate::Devices::TiffDevice`]'s `binarize_bradley`
//! method.
//!
//! Shared spec yaml pins the algorithm down to byte-exact
//! output across every target — Bradley is fully integer-
//! deterministic when the threshold check is rewritten as a
//! cross-multiplication
//! (`I * count * 100 <= sum * (100 - t)`) instead of the
//! textbook floating-point form. Every target's freeze-gate
//! test loads the same `.case` + `.expected` fixture pairs
//! and asserts byte-equal output.
//!
//! Rust track is `std`-only — no external crates.

use std::fmt;

/// Errors raised by [`apply`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// `grayscale.len() != width * height`.
    LengthMismatch {
        /// Actual length passed.
        got: usize,
        /// Expected `width * height`.
        expected: usize,
    },
    /// `width` not in `(0, 65535]`.
    WidthOutOfRange(u32),
    /// `height` not in `(0, 65535]`.
    HeightOutOfRange(u32),
    /// `window_size` not in `[2, max(width, height)]`.
    WindowSizeOutOfRange(u32),
    /// `threshold_percent` not in `[0, 100]`.
    ThresholdPercentOutOfRange(u32),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::LengthMismatch { got, expected } => write!(
                f,
                "binarize_bradley: grayscale length {got} != width*height {expected}"
            ),
            Error::WidthOutOfRange(w) =>
                write!(f, "binarize_bradley: width out of range: {w}"),
            Error::HeightOutOfRange(h) =>
                write!(f, "binarize_bradley: height out of range: {h}"),
            Error::WindowSizeOutOfRange(s) => write!(
                f,
                "binarize_bradley: window_size {s} not in [2, max(width, height)]"
            ),
            Error::ThresholdPercentOutOfRange(t) => write!(
                f,
                "binarize_bradley: threshold_percent {t} not in [0, 100]"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// Run Bradley adaptive thresholding over a row-major 8-bit
/// grayscale buffer. Returns the row-major bilevel output,
/// MSB-first per byte, right-padded with WHITE (1) bits when
/// `width` is not a multiple of 8.
///
/// `grayscale` must be row-major, top-left origin, 8 bits per
/// pixel; length must equal `width * height`.
/// `width`, `height` must be in `(0, 65535]`.
/// `window_size` must be in `[2, max(width, height)]`. Caller
/// passes `max(2, width / 8)` per Bradley's paper.
/// `threshold_percent` must be in `[0, 100]`. Default
/// caller-side is 15.
pub fn apply(
    grayscale: &[u8],
    width: u32,
    height: u32,
    window_size: u32,
    threshold_percent: u32,
) -> Result<Vec<u8>, Error> {
    if width == 0 || width > 65535 {
        return Err(Error::WidthOutOfRange(width));
    }
    if height == 0 || height > 65535 {
        return Err(Error::HeightOutOfRange(height));
    }
    let expected = (width as usize) * (height as usize);
    if grayscale.len() != expected {
        return Err(Error::LengthMismatch {
            got: grayscale.len(),
            expected,
        });
    }
    let max_dim = width.max(height);
    if window_size < 2 || window_size > max_dim {
        return Err(Error::WindowSizeOutOfRange(window_size));
    }
    if threshold_percent > 100 {
        return Err(Error::ThresholdPercentOutOfRange(threshold_percent));
    }

    let integral = build_integral_image(grayscale, width, height);
    Ok(threshold_via_integral(
        grayscale,
        width,
        height,
        window_size,
        threshold_percent,
        &integral,
    ))
}

/// Integral image S(x, y) = sum of I(i, j) for i<=x, j<=y.
/// Stored 1-indexed with a zero-padded row/column at index 0
/// so out-of-bounds lookups (y<0 / x<0) return 0 naturally.
/// `u64` element type covers the max sum
/// 255 × 65535 × 65535 ≈ 1.1e12 with room to spare.
fn build_integral_image(grayscale: &[u8], width: u32, height: u32) -> Vec<u64> {
    let stride = (width as usize) + 1;
    let mut integral = vec![0u64; stride * ((height as usize) + 1)];
    for y in 0..(height as usize) {
        let mut row_sum: u64 = 0;
        let base_in = y * (width as usize);
        let base_out_curr = (y + 1) * stride;
        let base_out_prev = y * stride;
        for x in 0..(width as usize) {
            row_sum += grayscale[base_in + x] as u64;
            integral[base_out_curr + (x + 1)] =
                integral[base_out_prev + (x + 1)] + row_sum;
        }
    }
    integral
}

fn threshold_via_integral(
    grayscale: &[u8],
    width: u32,
    height: u32,
    window_size: u32,
    threshold_percent: u32,
    integral: &[u64],
) -> Vec<u8> {
    let half = window_size / 2;
    let stride_out = width.div_ceil(8) as usize;
    let integral_stride = (width as usize) + 1;
    // Initialise to all-WHITE so right-padding bits past the
    // image width stay 1.
    let mut output = vec![0xFFu8; (height as usize) * stride_out];
    let factor: u64 = (100 - threshold_percent) as u64;

    for y in 0..height {
        let y1 = y.saturating_sub(half);
        let y2 = (y + half).min(height - 1);
        let i_y_lo = y1 as usize;
        let i_y_hi = (y2 + 1) as usize;
        let row_off_hi = i_y_hi * integral_stride;
        let row_off_lo = i_y_lo * integral_stride;
        for x in 0..width {
            let x1 = x.saturating_sub(half);
            let x2 = (x + half).min(width - 1);
            let count: u64 =
                ((x2 - x1 + 1) as u64) * ((y2 - y1 + 1) as u64);
            let i_x_lo = x1 as usize;
            let i_x_hi = (x2 + 1) as usize;
            // Integral image inclusion-exclusion. Sum the
            // unsigned values in a way that won't underflow
            // under debug-assertions: a + d >= b + c always
            // holds for a valid integral image.
            let a = integral[row_off_hi + i_x_hi];
            let b = integral[row_off_lo + i_x_hi];
            let c = integral[row_off_hi + i_x_lo];
            let d = integral[row_off_lo + i_x_lo];
            let window_sum = (a + d) - (b + c);
            let pixel = grayscale[(y as usize) * (width as usize) + x as usize] as u64;
            // Cross-multiplication: I * count * 100 <= sum * (100 - t).
            if pixel * count * 100 <= window_sum * factor {
                // BLACK — clear the MSB-first bit for x.
                let bit = 7 - (x % 8);
                let idx = (y as usize) * stride_out + (x / 8) as usize;
                output[idx] &= !(1u8 << bit);
            }
        }
    }
    output
}
