// Demonstrate the rendering-device surface.
//
// Snippet shows the parts callable today: ctors, property
// accessors, value round-trips. Driving each device end-to-end is
// shown by examples 07, 08, 09, 10, 11.
use aspose_pdf_foss::{
    BmpDevice, ImageDevice, JpegDevice, PageSize, PngDevice, Resolution,
    TextDevice,
};

fn main() {
    // PNG device, fixed-size output, transparent background.
    let mut png =
        PngDevice::with_dimensions_and_resolution(800, 600, Resolution::new(150).unwrap());
    png.set_transparent_background(true);
    println!(
        "PngDevice {}x{} @ {} DPI, transparent={}",
        png.width(),
        png.height(),
        png.resolution().unwrap().x(),
        png.transparent_background(),
    );

    // JPEG device with quality (ctor parameter, no public quality property).
    let jpeg = JpegDevice::with_page_size_resolution_and_quality(
        PageSize::a4(),
        Resolution::new(300).unwrap(),
        85,
    );
    println!(
        "JpegDevice from A4 @ {} DPI, {}x{}",
        jpeg.resolution().unwrap().x(),
        jpeg.width(),
        jpeg.height(),
    );

    // BMP device sized to PageLetter.
    let bmp = BmpDevice::with_page_size(PageSize::page_letter());
    println!("BmpDevice from PageLetter {}x{}", bmp.width(), bmp.height());

    // TextDevice — default-construct; UTF-8 output verbatim.
    let _ = TextDevice::new();
    println!("TextDevice default-constructed");
}
