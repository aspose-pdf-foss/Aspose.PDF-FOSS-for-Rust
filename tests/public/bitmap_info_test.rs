// Public-API smoke driver — BitmapInfo.
//
// Coverage:
//   * Ctor wires every field; accessors round-trip.
//   * PixelFormat ordinals match the canonical
//     Aspose.Pdf.BitmapInfo.PixelFormat ordering (Rgb24=0,
//     Bgr24=1, Rgba32=2, Argb32=3, Bgra32=4) so the discriminant
//     round-trips through interop with the .NET, C#, C++ and
//     TypeScript peer libs.
//   * Clone + Eq semantics — BitmapInfo is a value carrier.

use aspose_pdf_foss::{BitmapInfo, PixelFormat};

#[test]
fn ctor_wires_every_field() {
    let pixels = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8];
    let info = BitmapInfo::new(pixels.clone(), 3, 1, PixelFormat::Rgb24);
    assert_eq!(info.pixel_bytes(), &pixels[..]);
    assert_eq!(info.width(), 3);
    assert_eq!(info.height(), 1);
    assert_eq!(info.format(), PixelFormat::Rgb24);
}

#[test]
fn pixel_format_ordinals_match_canonical() {
    assert_eq!(PixelFormat::Rgb24 as i32, 0);
    assert_eq!(PixelFormat::Bgr24 as i32, 1);
    assert_eq!(PixelFormat::Rgba32 as i32, 2);
    assert_eq!(PixelFormat::Argb32 as i32, 3);
    assert_eq!(PixelFormat::Bgra32 as i32, 4);
}

#[test]
fn empty_buffer_is_accepted() {
    let info = BitmapInfo::new(Vec::new(), 0, 0, PixelFormat::Rgba32);
    assert_eq!(info.pixel_bytes().len(), 0);
    assert_eq!(info.width(), 0);
    assert_eq!(info.height(), 0);
}

#[test]
fn clone_is_independent() {
    let info = BitmapInfo::new(vec![1, 2, 3, 4], 1, 1, PixelFormat::Rgba32);
    let cloned = info.clone();
    assert_eq!(info, cloned);
    assert_eq!(cloned.pixel_bytes(), &[1, 2, 3, 4]);
}

#[test]
fn equality_distinguishes_format() {
    let a = BitmapInfo::new(vec![0, 0, 0], 1, 1, PixelFormat::Rgb24);
    let b = BitmapInfo::new(vec![0, 0, 0], 1, 1, PixelFormat::Bgr24);
    assert_ne!(a, b);
}

#[test]
fn equality_distinguishes_pixels() {
    let a = BitmapInfo::new(vec![1, 2, 3, 4], 1, 1, PixelFormat::Rgba32);
    let b = BitmapInfo::new(vec![1, 2, 3, 0], 1, 1, PixelFormat::Rgba32);
    assert_ne!(a, b);
}
