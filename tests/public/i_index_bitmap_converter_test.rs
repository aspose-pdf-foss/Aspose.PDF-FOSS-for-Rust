// Public-API smoke driver — IIndexBitmapConverter.
//
// Coverage:
//   * A stub impl satisfies the trait; the three methods are
//     reachable through dyn dispatch.
//   * Implementation routing — get_1bpp_image / get_4bpp_image /
//     get_8bpp_image dispatch to distinct method bodies and can
//     return arbitrary BitmapInfo shapes (the trait is a
//     pluggable strategy with no signature constraint beyond
//     the per-method palette ceiling).
//   * Arc<dyn IIndexBitmapConverter> is the storage shape
//     consumed by TiffDevice's converter-taking ctors — the
//     test confirms it round-trips through the dyn boundary.

use std::sync::Arc;

use aspose_pdf_foss::{BitmapInfo, IIndexBitmapConverter, PixelFormat};

// ============================================================
// Stub impl — returns a BitmapInfo whose first pixel byte
// encodes which method was invoked so the test can assert
// dispatch.
// ============================================================

#[derive(Debug, Default)]
struct StubConverter;

impl IIndexBitmapConverter for StubConverter {
    fn get_1bpp_image(&self, src: &BitmapInfo) -> BitmapInfo {
        BitmapInfo::new(vec![1], src.width(), src.height(), PixelFormat::Rgb24)
    }

    fn get_4bpp_image(&self, src: &BitmapInfo) -> BitmapInfo {
        BitmapInfo::new(vec![4], src.width(), src.height(), PixelFormat::Rgb24)
    }

    fn get_8bpp_image(&self, src: &BitmapInfo) -> BitmapInfo {
        BitmapInfo::new(vec![8], src.width(), src.height(), PixelFormat::Rgb24)
    }
}

#[test]
fn trait_dispatches_each_method_distinctly() {
    let converter: Arc<dyn IIndexBitmapConverter> = Arc::new(StubConverter);
    let src = BitmapInfo::new(vec![0; 16], 2, 2, PixelFormat::Rgba32);

    assert_eq!(converter.get_1bpp_image(&src).pixel_bytes(), &[1]);
    assert_eq!(converter.get_4bpp_image(&src).pixel_bytes(), &[4]);
    assert_eq!(converter.get_8bpp_image(&src).pixel_bytes(), &[8]);
}

#[test]
fn trait_preserves_input_geometry() {
    let converter: Arc<dyn IIndexBitmapConverter> = Arc::new(StubConverter);
    let src = BitmapInfo::new(vec![0; 16 * 4], 4, 4, PixelFormat::Rgba32);

    let one_bpp = converter.get_1bpp_image(&src);
    assert_eq!(one_bpp.width(), 4);
    assert_eq!(one_bpp.height(), 4);
}

#[test]
fn arc_dyn_round_trips_through_clone() {
    // Arc<dyn IIndexBitmapConverter> is Clone (Arc impls Clone
    // for ?Sized inner types). TiffDevice stores the converter
    // as Option<Arc<dyn IIndexBitmapConverter>> + derives Clone;
    // confirm the storage shape clones cleanly so a downstream
    // device.clone() can fan out to threads.
    let a: Arc<dyn IIndexBitmapConverter> = Arc::new(StubConverter);
    let b = Arc::clone(&a);
    assert_eq!(Arc::strong_count(&a), 2);
    let src = BitmapInfo::new(vec![0; 4], 1, 1, PixelFormat::Rgba32);
    assert_eq!(b.get_1bpp_image(&src).pixel_bytes(), &[1]);
}
