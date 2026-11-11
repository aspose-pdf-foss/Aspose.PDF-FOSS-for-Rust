//! `Aspose.Pdf.Color` — an RGB/ARGB colour value, per
//! `capabilities/color.yaml`.
//!
//! Components are held as normalised `0.0..=1.0` doubles (alpha,
//! red, green, blue). The public surface mirrors the C++ reference
//! member selection: the default (empty) constructor, the
//! `from_argb` integer factories, the normalised `from_rgb` double
//! factory, the `a()` alpha accessor, `empty()` / `transparent()`,
//! and the 140 W3C named presets. The canonical `Parse`,
//! `ToString`, `ToRgb`, the `System.Drawing.Color` factory
//! overload, `FromGray`, `FromCmyk`, `Equals`, `Data`,
//! `ColorSpace`, and `PatternColorSpace` members are dropped (see
//! `targets/rust/{translations,drops}.yaml`).
//!
//! Preset RGB values are the standard W3C / CSS3 extended named
//! colours (the same table `System.Drawing.Color` mirrors), each
//! fully opaque (`a() == 1.0`).

/// An RGB/ARGB colour with normalised `0.0..=1.0` components.
///
/// A default-constructed `Color` is the *empty* colour — every
/// component, including alpha, is `0.0`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Color {
    a: f64,
    r: f64,
    g: f64,
    b: f64,
}

impl Color {
    /// Construct the empty colour (all components `0.0`),
    /// equivalent to [`Color::empty`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Build an opaque colour from 8-bit `r` / `g` / `b`
    /// components (`0..=255`); alpha is fully opaque.
    pub fn from_argb_rgb(r: i32, g: i32, b: i32) -> Self {
        Self::from_argb(255, r, g, b)
    }

    /// Build a colour from 8-bit `a` / `r` / `g` / `b` components
    /// (`0..=255`).
    pub fn from_argb(a: i32, r: i32, g: i32, b: i32) -> Self {
        Self {
            a: a as f64 / 255.0,
            r: r as f64 / 255.0,
            g: g as f64 / 255.0,
            b: b as f64 / 255.0,
        }
    }

    /// Build an opaque colour from normalised `0.0..=1.0` `r` /
    /// `g` / `b` components.
    pub fn from_rgb(r: f64, g: f64, b: f64) -> Self {
        Self { a: 1.0, r, g, b }
    }

    /// The alpha component, normalised to `0.0..=1.0`.
    pub fn a(&self) -> f64 {
        self.a
    }

    /// The `(r, g, b)` components, normalised to `0.0..=1.0`. For the
    /// content-stream render pipeline.
    pub(crate) fn rgb(&self) -> (f64, f64, f64) {
        (self.r, self.g, self.b)
    }

    /// Whether this is the empty colour (every component `0.0`) — a
    /// "no colour set" sentinel for the render pipeline.
    pub(crate) fn is_empty(&self) -> bool {
        self.a == 0.0 && self.r == 0.0 && self.g == 0.0 && self.b == 0.0
    }

    /// The empty colour (all components `0.0`).
    pub fn empty() -> Color {
        Color::default()
    }

    /// Fully transparent white (`a() == 0.0`).
    pub fn transparent() -> Color {
        Color::from_argb(0, 255, 255, 255)
    }

    /// Build an opaque colour from a packed `0xRRGGBB` value.
    fn from_hex(rgb: u32) -> Color {
        Color::from_argb(
            255,
            ((rgb >> 16) & 0xFF) as i32,
            ((rgb >> 8) & 0xFF) as i32,
            (rgb & 0xFF) as i32,
        )
    }

    /// The `AliceBlue` named colour (`#F0F8FF`), fully opaque.
    pub fn alice_blue() -> Color {
        Color::from_hex(0xF0F8FF)
    }
    /// The `AntiqueWhite` named colour (`#FAEBD7`), fully opaque.
    pub fn antique_white() -> Color {
        Color::from_hex(0xFAEBD7)
    }
    /// The `Aqua` named colour (`#00FFFF`), fully opaque.
    pub fn aqua() -> Color {
        Color::from_hex(0x00FFFF)
    }
    /// The `Aquamarine` named colour (`#7FFFD4`), fully opaque.
    pub fn aquamarine() -> Color {
        Color::from_hex(0x7FFFD4)
    }
    /// The `Azure` named colour (`#F0FFFF`), fully opaque.
    pub fn azure() -> Color {
        Color::from_hex(0xF0FFFF)
    }
    /// The `Beige` named colour (`#F5F5DC`), fully opaque.
    pub fn beige() -> Color {
        Color::from_hex(0xF5F5DC)
    }
    /// The `Bisque` named colour (`#FFE4C4`), fully opaque.
    pub fn bisque() -> Color {
        Color::from_hex(0xFFE4C4)
    }
    /// The `Black` named colour (`#000000`), fully opaque.
    pub fn black() -> Color {
        Color::from_hex(0x000000)
    }
    /// The `BlanchedAlmond` named colour (`#FFEBCD`), fully opaque.
    pub fn blanched_almond() -> Color {
        Color::from_hex(0xFFEBCD)
    }
    /// The `Blue` named colour (`#0000FF`), fully opaque.
    pub fn blue() -> Color {
        Color::from_hex(0x0000FF)
    }
    /// The `BlueViolet` named colour (`#8A2BE2`), fully opaque.
    pub fn blue_violet() -> Color {
        Color::from_hex(0x8A2BE2)
    }
    /// The `Brown` named colour (`#A52A2A`), fully opaque.
    pub fn brown() -> Color {
        Color::from_hex(0xA52A2A)
    }
    /// The `BurlyWood` named colour (`#DEB887`), fully opaque.
    pub fn burly_wood() -> Color {
        Color::from_hex(0xDEB887)
    }
    /// The `CadetBlue` named colour (`#5F9EA0`), fully opaque.
    pub fn cadet_blue() -> Color {
        Color::from_hex(0x5F9EA0)
    }
    /// The `Chartreuse` named colour (`#7FFF00`), fully opaque.
    pub fn chartreuse() -> Color {
        Color::from_hex(0x7FFF00)
    }
    /// The `Chocolate` named colour (`#D2691E`), fully opaque.
    pub fn chocolate() -> Color {
        Color::from_hex(0xD2691E)
    }
    /// The `Coral` named colour (`#FF7F50`), fully opaque.
    pub fn coral() -> Color {
        Color::from_hex(0xFF7F50)
    }
    /// The `CornflowerBlue` named colour (`#6495ED`), fully opaque.
    pub fn cornflower_blue() -> Color {
        Color::from_hex(0x6495ED)
    }
    /// The `Cornsilk` named colour (`#FFF8DC`), fully opaque.
    pub fn cornsilk() -> Color {
        Color::from_hex(0xFFF8DC)
    }
    /// The `Crimson` named colour (`#DC143C`), fully opaque.
    pub fn crimson() -> Color {
        Color::from_hex(0xDC143C)
    }
    /// The `Cyan` named colour (`#00FFFF`), fully opaque.
    pub fn cyan() -> Color {
        Color::from_hex(0x00FFFF)
    }
    /// The `DarkBlue` named colour (`#00008B`), fully opaque.
    pub fn dark_blue() -> Color {
        Color::from_hex(0x00008B)
    }
    /// The `DarkCyan` named colour (`#008B8B`), fully opaque.
    pub fn dark_cyan() -> Color {
        Color::from_hex(0x008B8B)
    }
    /// The `DarkGoldenrod` named colour (`#B8860B`), fully opaque.
    pub fn dark_goldenrod() -> Color {
        Color::from_hex(0xB8860B)
    }
    /// The `DarkGray` named colour (`#A9A9A9`), fully opaque.
    pub fn dark_gray() -> Color {
        Color::from_hex(0xA9A9A9)
    }
    /// The `DarkGreen` named colour (`#006400`), fully opaque.
    pub fn dark_green() -> Color {
        Color::from_hex(0x006400)
    }
    /// The `DarkKhaki` named colour (`#BDB76B`), fully opaque.
    pub fn dark_khaki() -> Color {
        Color::from_hex(0xBDB76B)
    }
    /// The `DarkMagenta` named colour (`#8B008B`), fully opaque.
    pub fn dark_magenta() -> Color {
        Color::from_hex(0x8B008B)
    }
    /// The `DarkOliveGreen` named colour (`#556B2F`), fully opaque.
    pub fn dark_olive_green() -> Color {
        Color::from_hex(0x556B2F)
    }
    /// The `DarkOrange` named colour (`#FF8C00`), fully opaque.
    pub fn dark_orange() -> Color {
        Color::from_hex(0xFF8C00)
    }
    /// The `DarkOrchid` named colour (`#9932CC`), fully opaque.
    pub fn dark_orchid() -> Color {
        Color::from_hex(0x9932CC)
    }
    /// The `DarkRed` named colour (`#8B0000`), fully opaque.
    pub fn dark_red() -> Color {
        Color::from_hex(0x8B0000)
    }
    /// The `DarkSalmon` named colour (`#E9967A`), fully opaque.
    pub fn dark_salmon() -> Color {
        Color::from_hex(0xE9967A)
    }
    /// The `DarkSeaGreen` named colour (`#8FBC8F`), fully opaque.
    pub fn dark_sea_green() -> Color {
        Color::from_hex(0x8FBC8F)
    }
    /// The `DarkSlateBlue` named colour (`#483D8B`), fully opaque.
    pub fn dark_slate_blue() -> Color {
        Color::from_hex(0x483D8B)
    }
    /// The `DarkSlateGray` named colour (`#2F4F4F`), fully opaque.
    pub fn dark_slate_gray() -> Color {
        Color::from_hex(0x2F4F4F)
    }
    /// The `DarkTurquoise` named colour (`#00CED1`), fully opaque.
    pub fn dark_turquoise() -> Color {
        Color::from_hex(0x00CED1)
    }
    /// The `DarkViolet` named colour (`#9400D3`), fully opaque.
    pub fn dark_violet() -> Color {
        Color::from_hex(0x9400D3)
    }
    /// The `DeepPink` named colour (`#FF1493`), fully opaque.
    pub fn deep_pink() -> Color {
        Color::from_hex(0xFF1493)
    }
    /// The `DeepSkyBlue` named colour (`#00BFFF`), fully opaque.
    pub fn deep_sky_blue() -> Color {
        Color::from_hex(0x00BFFF)
    }
    /// The `DimGray` named colour (`#696969`), fully opaque.
    pub fn dim_gray() -> Color {
        Color::from_hex(0x696969)
    }
    /// The `DodgerBlue` named colour (`#1E90FF`), fully opaque.
    pub fn dodger_blue() -> Color {
        Color::from_hex(0x1E90FF)
    }
    /// The `Firebrick` named colour (`#B22222`), fully opaque.
    pub fn firebrick() -> Color {
        Color::from_hex(0xB22222)
    }
    /// The `FloralWhite` named colour (`#FFFAF0`), fully opaque.
    pub fn floral_white() -> Color {
        Color::from_hex(0xFFFAF0)
    }
    /// The `ForestGreen` named colour (`#228B22`), fully opaque.
    pub fn forest_green() -> Color {
        Color::from_hex(0x228B22)
    }
    /// The `Fuchsia` named colour (`#FF00FF`), fully opaque.
    pub fn fuchsia() -> Color {
        Color::from_hex(0xFF00FF)
    }
    /// The `Gainsboro` named colour (`#DCDCDC`), fully opaque.
    pub fn gainsboro() -> Color {
        Color::from_hex(0xDCDCDC)
    }
    /// The `GhostWhite` named colour (`#F8F8FF`), fully opaque.
    pub fn ghost_white() -> Color {
        Color::from_hex(0xF8F8FF)
    }
    /// The `Gold` named colour (`#FFD700`), fully opaque.
    pub fn gold() -> Color {
        Color::from_hex(0xFFD700)
    }
    /// The `Goldenrod` named colour (`#DAA520`), fully opaque.
    pub fn goldenrod() -> Color {
        Color::from_hex(0xDAA520)
    }
    /// The `Gray` named colour (`#808080`), fully opaque.
    pub fn gray() -> Color {
        Color::from_hex(0x808080)
    }
    /// The `Green` named colour (`#008000`), fully opaque.
    pub fn green() -> Color {
        Color::from_hex(0x008000)
    }
    /// The `GreenYellow` named colour (`#ADFF2F`), fully opaque.
    pub fn green_yellow() -> Color {
        Color::from_hex(0xADFF2F)
    }
    /// The `Honeydew` named colour (`#F0FFF0`), fully opaque.
    pub fn honeydew() -> Color {
        Color::from_hex(0xF0FFF0)
    }
    /// The `HotPink` named colour (`#FF69B4`), fully opaque.
    pub fn hot_pink() -> Color {
        Color::from_hex(0xFF69B4)
    }
    /// The `IndianRed` named colour (`#CD5C5C`), fully opaque.
    pub fn indian_red() -> Color {
        Color::from_hex(0xCD5C5C)
    }
    /// The `Indigo` named colour (`#4B0082`), fully opaque.
    pub fn indigo() -> Color {
        Color::from_hex(0x4B0082)
    }
    /// The `Ivory` named colour (`#FFFFF0`), fully opaque.
    pub fn ivory() -> Color {
        Color::from_hex(0xFFFFF0)
    }
    /// The `Khaki` named colour (`#F0E68C`), fully opaque.
    pub fn khaki() -> Color {
        Color::from_hex(0xF0E68C)
    }
    /// The `Lavender` named colour (`#E6E6FA`), fully opaque.
    pub fn lavender() -> Color {
        Color::from_hex(0xE6E6FA)
    }
    /// The `LavenderBlush` named colour (`#FFF0F5`), fully opaque.
    pub fn lavender_blush() -> Color {
        Color::from_hex(0xFFF0F5)
    }
    /// The `LawnGreen` named colour (`#7CFC00`), fully opaque.
    pub fn lawn_green() -> Color {
        Color::from_hex(0x7CFC00)
    }
    /// The `LemonChiffon` named colour (`#FFFACD`), fully opaque.
    pub fn lemon_chiffon() -> Color {
        Color::from_hex(0xFFFACD)
    }
    /// The `LightBlue` named colour (`#ADD8E6`), fully opaque.
    pub fn light_blue() -> Color {
        Color::from_hex(0xADD8E6)
    }
    /// The `LightCoral` named colour (`#F08080`), fully opaque.
    pub fn light_coral() -> Color {
        Color::from_hex(0xF08080)
    }
    /// The `LightCyan` named colour (`#E0FFFF`), fully opaque.
    pub fn light_cyan() -> Color {
        Color::from_hex(0xE0FFFF)
    }
    /// The `LightGoldenrodYellow` named colour (`#FAFAD2`), fully opaque.
    pub fn light_goldenrod_yellow() -> Color {
        Color::from_hex(0xFAFAD2)
    }
    /// The `LightGreen` named colour (`#90EE90`), fully opaque.
    pub fn light_green() -> Color {
        Color::from_hex(0x90EE90)
    }
    /// The `LightGray` named colour (`#D3D3D3`), fully opaque.
    pub fn light_gray() -> Color {
        Color::from_hex(0xD3D3D3)
    }
    /// The `LightPink` named colour (`#FFB6C1`), fully opaque.
    pub fn light_pink() -> Color {
        Color::from_hex(0xFFB6C1)
    }
    /// The `LightSalmon` named colour (`#FFA07A`), fully opaque.
    pub fn light_salmon() -> Color {
        Color::from_hex(0xFFA07A)
    }
    /// The `LightSeaGreen` named colour (`#20B2AA`), fully opaque.
    pub fn light_sea_green() -> Color {
        Color::from_hex(0x20B2AA)
    }
    /// The `LightSkyBlue` named colour (`#87CEFA`), fully opaque.
    pub fn light_sky_blue() -> Color {
        Color::from_hex(0x87CEFA)
    }
    /// The `LightSlateGray` named colour (`#778899`), fully opaque.
    pub fn light_slate_gray() -> Color {
        Color::from_hex(0x778899)
    }
    /// The `LightSteelBlue` named colour (`#B0C4DE`), fully opaque.
    pub fn light_steel_blue() -> Color {
        Color::from_hex(0xB0C4DE)
    }
    /// The `LightYellow` named colour (`#FFFFE0`), fully opaque.
    pub fn light_yellow() -> Color {
        Color::from_hex(0xFFFFE0)
    }
    /// The `Lime` named colour (`#00FF00`), fully opaque.
    pub fn lime() -> Color {
        Color::from_hex(0x00FF00)
    }
    /// The `LimeGreen` named colour (`#32CD32`), fully opaque.
    pub fn lime_green() -> Color {
        Color::from_hex(0x32CD32)
    }
    /// The `Linen` named colour (`#FAF0E6`), fully opaque.
    pub fn linen() -> Color {
        Color::from_hex(0xFAF0E6)
    }
    /// The `Magenta` named colour (`#FF00FF`), fully opaque.
    pub fn magenta() -> Color {
        Color::from_hex(0xFF00FF)
    }
    /// The `Maroon` named colour (`#800000`), fully opaque.
    pub fn maroon() -> Color {
        Color::from_hex(0x800000)
    }
    /// The `MediumAquamarine` named colour (`#66CDAA`), fully opaque.
    pub fn medium_aquamarine() -> Color {
        Color::from_hex(0x66CDAA)
    }
    /// The `MediumBlue` named colour (`#0000CD`), fully opaque.
    pub fn medium_blue() -> Color {
        Color::from_hex(0x0000CD)
    }
    /// The `MediumOrchid` named colour (`#BA55D3`), fully opaque.
    pub fn medium_orchid() -> Color {
        Color::from_hex(0xBA55D3)
    }
    /// The `MediumPurple` named colour (`#9370DB`), fully opaque.
    pub fn medium_purple() -> Color {
        Color::from_hex(0x9370DB)
    }
    /// The `MediumSeaGreen` named colour (`#3CB371`), fully opaque.
    pub fn medium_sea_green() -> Color {
        Color::from_hex(0x3CB371)
    }
    /// The `MediumSlateBlue` named colour (`#7B68EE`), fully opaque.
    pub fn medium_slate_blue() -> Color {
        Color::from_hex(0x7B68EE)
    }
    /// The `MediumSpringGreen` named colour (`#00FA9A`), fully opaque.
    pub fn medium_spring_green() -> Color {
        Color::from_hex(0x00FA9A)
    }
    /// The `MediumTurquoise` named colour (`#48D1CC`), fully opaque.
    pub fn medium_turquoise() -> Color {
        Color::from_hex(0x48D1CC)
    }
    /// The `MediumVioletRed` named colour (`#C71585`), fully opaque.
    pub fn medium_violet_red() -> Color {
        Color::from_hex(0xC71585)
    }
    /// The `MidnightBlue` named colour (`#191970`), fully opaque.
    pub fn midnight_blue() -> Color {
        Color::from_hex(0x191970)
    }
    /// The `MintCream` named colour (`#F5FFFA`), fully opaque.
    pub fn mint_cream() -> Color {
        Color::from_hex(0xF5FFFA)
    }
    /// The `MistyRose` named colour (`#FFE4E1`), fully opaque.
    pub fn misty_rose() -> Color {
        Color::from_hex(0xFFE4E1)
    }
    /// The `Moccasin` named colour (`#FFE4B5`), fully opaque.
    pub fn moccasin() -> Color {
        Color::from_hex(0xFFE4B5)
    }
    /// The `NavajoWhite` named colour (`#FFDEAD`), fully opaque.
    pub fn navajo_white() -> Color {
        Color::from_hex(0xFFDEAD)
    }
    /// The `Navy` named colour (`#000080`), fully opaque.
    pub fn navy() -> Color {
        Color::from_hex(0x000080)
    }
    /// The `OldLace` named colour (`#FDF5E6`), fully opaque.
    pub fn old_lace() -> Color {
        Color::from_hex(0xFDF5E6)
    }
    /// The `Olive` named colour (`#808000`), fully opaque.
    pub fn olive() -> Color {
        Color::from_hex(0x808000)
    }
    /// The `OliveDrab` named colour (`#6B8E23`), fully opaque.
    pub fn olive_drab() -> Color {
        Color::from_hex(0x6B8E23)
    }
    /// The `Orange` named colour (`#FFA500`), fully opaque.
    pub fn orange() -> Color {
        Color::from_hex(0xFFA500)
    }
    /// The `OrangeRed` named colour (`#FF4500`), fully opaque.
    pub fn orange_red() -> Color {
        Color::from_hex(0xFF4500)
    }
    /// The `Orchid` named colour (`#DA70D6`), fully opaque.
    pub fn orchid() -> Color {
        Color::from_hex(0xDA70D6)
    }
    /// The `PaleGoldenrod` named colour (`#EEE8AA`), fully opaque.
    pub fn pale_goldenrod() -> Color {
        Color::from_hex(0xEEE8AA)
    }
    /// The `PaleGreen` named colour (`#98FB98`), fully opaque.
    pub fn pale_green() -> Color {
        Color::from_hex(0x98FB98)
    }
    /// The `PaleTurquoise` named colour (`#AFEEEE`), fully opaque.
    pub fn pale_turquoise() -> Color {
        Color::from_hex(0xAFEEEE)
    }
    /// The `PaleVioletRed` named colour (`#DB7093`), fully opaque.
    pub fn pale_violet_red() -> Color {
        Color::from_hex(0xDB7093)
    }
    /// The `PapayaWhip` named colour (`#FFEFD5`), fully opaque.
    pub fn papaya_whip() -> Color {
        Color::from_hex(0xFFEFD5)
    }
    /// The `PeachPuff` named colour (`#FFDAB9`), fully opaque.
    pub fn peach_puff() -> Color {
        Color::from_hex(0xFFDAB9)
    }
    /// The `Peru` named colour (`#CD853F`), fully opaque.
    pub fn peru() -> Color {
        Color::from_hex(0xCD853F)
    }
    /// The `Pink` named colour (`#FFC0CB`), fully opaque.
    pub fn pink() -> Color {
        Color::from_hex(0xFFC0CB)
    }
    /// The `Plum` named colour (`#DDA0DD`), fully opaque.
    pub fn plum() -> Color {
        Color::from_hex(0xDDA0DD)
    }
    /// The `PowderBlue` named colour (`#B0E0E6`), fully opaque.
    pub fn powder_blue() -> Color {
        Color::from_hex(0xB0E0E6)
    }
    /// The `Purple` named colour (`#800080`), fully opaque.
    pub fn purple() -> Color {
        Color::from_hex(0x800080)
    }
    /// The `Red` named colour (`#FF0000`), fully opaque.
    pub fn red() -> Color {
        Color::from_hex(0xFF0000)
    }
    /// The `RosyBrown` named colour (`#BC8F8F`), fully opaque.
    pub fn rosy_brown() -> Color {
        Color::from_hex(0xBC8F8F)
    }
    /// The `RoyalBlue` named colour (`#4169E1`), fully opaque.
    pub fn royal_blue() -> Color {
        Color::from_hex(0x4169E1)
    }
    /// The `SaddleBrown` named colour (`#8B4513`), fully opaque.
    pub fn saddle_brown() -> Color {
        Color::from_hex(0x8B4513)
    }
    /// The `Salmon` named colour (`#FA8072`), fully opaque.
    pub fn salmon() -> Color {
        Color::from_hex(0xFA8072)
    }
    /// The `SandyBrown` named colour (`#F4A460`), fully opaque.
    pub fn sandy_brown() -> Color {
        Color::from_hex(0xF4A460)
    }
    /// The `SeaGreen` named colour (`#2E8B57`), fully opaque.
    pub fn sea_green() -> Color {
        Color::from_hex(0x2E8B57)
    }
    /// The `SeaShell` named colour (`#FFF5EE`), fully opaque.
    pub fn sea_shell() -> Color {
        Color::from_hex(0xFFF5EE)
    }
    /// The `Sienna` named colour (`#A0522D`), fully opaque.
    pub fn sienna() -> Color {
        Color::from_hex(0xA0522D)
    }
    /// The `Silver` named colour (`#C0C0C0`), fully opaque.
    pub fn silver() -> Color {
        Color::from_hex(0xC0C0C0)
    }
    /// The `SkyBlue` named colour (`#87CEEB`), fully opaque.
    pub fn sky_blue() -> Color {
        Color::from_hex(0x87CEEB)
    }
    /// The `SlateBlue` named colour (`#6A5ACD`), fully opaque.
    pub fn slate_blue() -> Color {
        Color::from_hex(0x6A5ACD)
    }
    /// The `SlateGray` named colour (`#708090`), fully opaque.
    pub fn slate_gray() -> Color {
        Color::from_hex(0x708090)
    }
    /// The `Snow` named colour (`#FFFAFA`), fully opaque.
    pub fn snow() -> Color {
        Color::from_hex(0xFFFAFA)
    }
    /// The `SpringGreen` named colour (`#00FF7F`), fully opaque.
    pub fn spring_green() -> Color {
        Color::from_hex(0x00FF7F)
    }
    /// The `SteelBlue` named colour (`#4682B4`), fully opaque.
    pub fn steel_blue() -> Color {
        Color::from_hex(0x4682B4)
    }
    /// The `Tan` named colour (`#D2B48C`), fully opaque.
    pub fn tan() -> Color {
        Color::from_hex(0xD2B48C)
    }
    /// The `Teal` named colour (`#008080`), fully opaque.
    pub fn teal() -> Color {
        Color::from_hex(0x008080)
    }
    /// The `Thistle` named colour (`#D8BFD8`), fully opaque.
    pub fn thistle() -> Color {
        Color::from_hex(0xD8BFD8)
    }
    /// The `Tomato` named colour (`#FF6347`), fully opaque.
    pub fn tomato() -> Color {
        Color::from_hex(0xFF6347)
    }
    /// The `Turquoise` named colour (`#40E0D0`), fully opaque.
    pub fn turquoise() -> Color {
        Color::from_hex(0x40E0D0)
    }
    /// The `Violet` named colour (`#EE82EE`), fully opaque.
    pub fn violet() -> Color {
        Color::from_hex(0xEE82EE)
    }
    /// The `Wheat` named colour (`#F5DEB3`), fully opaque.
    pub fn wheat() -> Color {
        Color::from_hex(0xF5DEB3)
    }
    /// The `White` named colour (`#FFFFFF`), fully opaque.
    pub fn white() -> Color {
        Color::from_hex(0xFFFFFF)
    }
    /// The `WhiteSmoke` named colour (`#F5F5F5`), fully opaque.
    pub fn white_smoke() -> Color {
        Color::from_hex(0xF5F5F5)
    }
    /// The `Yellow` named colour (`#FFFF00`), fully opaque.
    pub fn yellow() -> Color {
        Color::from_hex(0xFFFF00)
    }
    /// The `YellowGreen` named colour (`#9ACD32`), fully opaque.
    pub fn yellow_green() -> Color {
        Color::from_hex(0x9ACD32)
    }
}
