//! foundation/ccitt — PDF /CCITTFaxDecode filter (T.4 G3 1D + T.6 G4).
//!
//! Implements decode-only support for ITU-T T.4 (Group 3) and T.6
//! (Group 4) bilevel image compression as used by PDF's
//! /CCITTFaxDecode filter. Group 3 mixed (K > 0) is intentionally
//! out of scope for the v1 corpus — matches the peer CPP / C# /
//! TypeScript ports.
//!
//! References:
//!   ITU-T Recommendation T.4 (07/2003) — Group 3 facsimile.
//!   ITU-T Recommendation T.6 (11/1988) — Group 4 facsimile.
//!   PDF 32000-1:2008 §7.4.7 (CCITTFaxDecode filter, Table 11).
//!
//! See Feedstock/PDF/Spec/ccitt.yaml + targets/rust/ccitt.yaml.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub struct Params {
    pub k: i8,
    pub end_of_line: bool,
    pub encoded_byte_align: bool,
    pub end_of_block: bool,
    pub black_is_1: bool,
    pub columns: u32,
    pub rows: u32,
    pub damaged_rows_before_error: u32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            k: 0,
            end_of_line: false,
            encoded_byte_align: false,
            end_of_block: true,
            black_is_1: false,
            columns: 1728,
            rows: 0,
            damaged_rows_before_error: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub bits: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub enum Error {
    EmptyStream,
    InvalidColumns,
    InvalidRunLengthCode { is_white: bool },
    InvalidModeCode,
    UnsupportedExtension,
    RunExceedsColumns,
    VModeOutOfBounds,
    HModeExceedsColumns,
    RowCursorMismatch,
    MissingEol,
    UnsupportedMixedGroup3,
    CfaxMalformed(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::EmptyStream => f.write_str("CCITT: empty stream"),
            Error::InvalidColumns => f.write_str("CCITT: Columns must be > 0"),
            Error::InvalidRunLengthCode { is_white } => write!(
                f,
                "CCITT: invalid {} run-length code",
                if *is_white { "white" } else { "black" }
            ),
            Error::InvalidModeCode => f.write_str("CCITT: invalid 2D mode code"),
            Error::UnsupportedExtension => {
                f.write_str("CCITT: 2D extension code not supported")
            }
            Error::RunExceedsColumns => {
                f.write_str("CCITT: 1D run-length exceeds Columns")
            }
            Error::VModeOutOfBounds => {
                f.write_str("CCITT: V-mode position out of bounds")
            }
            Error::HModeExceedsColumns => {
                f.write_str("CCITT: H-mode run lengths exceed Columns")
            }
            Error::RowCursorMismatch => {
                f.write_str("CCITT: 2D row cursor != Columns at end")
            }
            Error::MissingEol => f.write_str("CCITT: missing EOL before row"),
            Error::UnsupportedMixedGroup3 => {
                f.write_str("CCITT: K > 0 (Group 3 mixed) not yet implemented")
            }
            Error::CfaxMalformed(msg) => {
                write!(f, "CCITT fixture: {}", msg)
            }
        }
    }
}

impl std::error::Error for Error {}

// ---------------------------------------------------------------------------
// Bit reader (MSB-first per byte).
// ---------------------------------------------------------------------------

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    buffer: u32,
    buffer_bits: i32,
}

impl<'a> BitReader<'a> {
    #[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            buffer: 0,
            buffer_bits: 0,
        }
    }

    #[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
    fn refill(&mut self, need: i32) {
        while self.buffer_bits < need && self.pos < self.data.len() {
            self.buffer = (self.buffer << 8) | self.data[self.pos] as u32;
            self.pos += 1;
            self.buffer_bits += 8;
        }
    }

    #[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
    fn peek(&mut self, n: i32) -> u32 {
        self.refill(n);
        if n <= 0 {
            return 0;
        }
        if n > self.buffer_bits {
            let shortfall = n - self.buffer_bits;
            let v = self.buffer;
            return (v << shortfall) & ((1u32 << n) - 1);
        }
        (self.buffer >> (self.buffer_bits - n)) & ((1u32 << n) - 1)
    }

    #[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
    fn consume(&mut self, n: i32) {
        if n <= 0 {
            return;
        }
        if n >= self.buffer_bits {
            self.buffer = 0;
            self.buffer_bits = 0;
            return;
        }
        self.buffer_bits -= n;
        self.buffer &= (1u32 << self.buffer_bits) - 1;
    }

    #[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
    fn align_to_byte(&mut self) {
        let extra = self.buffer_bits % 8;
        if extra != 0 {
            self.consume(extra);
        }
    }

    #[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
    fn bits_available(&self) -> i32 {
        self.buffer_bits + ((self.data.len() - self.pos) as i32) * 8
    }
}

// ---------------------------------------------------------------------------
// Huffman code tables.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct CodePattern {
    bits: u16,
    len: u8,
    value: i16,
}

// White run-length terminating codes (T.4 Table 2). Run lengths 0..63.
const WHITE_TERM: [CodePattern; 64] = [
    CodePattern { bits: 0b00110101, len: 8, value: 0 },
    CodePattern { bits: 0b000111, len: 6, value: 1 },
    CodePattern { bits: 0b0111, len: 4, value: 2 },
    CodePattern { bits: 0b1000, len: 4, value: 3 },
    CodePattern { bits: 0b1011, len: 4, value: 4 },
    CodePattern { bits: 0b1100, len: 4, value: 5 },
    CodePattern { bits: 0b1110, len: 4, value: 6 },
    CodePattern { bits: 0b1111, len: 4, value: 7 },
    CodePattern { bits: 0b10011, len: 5, value: 8 },
    CodePattern { bits: 0b10100, len: 5, value: 9 },
    CodePattern { bits: 0b00111, len: 5, value: 10 },
    CodePattern { bits: 0b01000, len: 5, value: 11 },
    CodePattern { bits: 0b001000, len: 6, value: 12 },
    CodePattern { bits: 0b000011, len: 6, value: 13 },
    CodePattern { bits: 0b110100, len: 6, value: 14 },
    CodePattern { bits: 0b110101, len: 6, value: 15 },
    CodePattern { bits: 0b101010, len: 6, value: 16 },
    CodePattern { bits: 0b101011, len: 6, value: 17 },
    CodePattern { bits: 0b0100111, len: 7, value: 18 },
    CodePattern { bits: 0b0001100, len: 7, value: 19 },
    CodePattern { bits: 0b0001000, len: 7, value: 20 },
    CodePattern { bits: 0b0010111, len: 7, value: 21 },
    CodePattern { bits: 0b0000011, len: 7, value: 22 },
    CodePattern { bits: 0b0000100, len: 7, value: 23 },
    CodePattern { bits: 0b0101000, len: 7, value: 24 },
    CodePattern { bits: 0b0101011, len: 7, value: 25 },
    CodePattern { bits: 0b0010011, len: 7, value: 26 },
    CodePattern { bits: 0b0100100, len: 7, value: 27 },
    CodePattern { bits: 0b0011000, len: 7, value: 28 },
    CodePattern { bits: 0b00000010, len: 8, value: 29 },
    CodePattern { bits: 0b00000011, len: 8, value: 30 },
    CodePattern { bits: 0b00011010, len: 8, value: 31 },
    CodePattern { bits: 0b00011011, len: 8, value: 32 },
    CodePattern { bits: 0b00010010, len: 8, value: 33 },
    CodePattern { bits: 0b00010011, len: 8, value: 34 },
    CodePattern { bits: 0b00010100, len: 8, value: 35 },
    CodePattern { bits: 0b00010101, len: 8, value: 36 },
    CodePattern { bits: 0b00010110, len: 8, value: 37 },
    CodePattern { bits: 0b00010111, len: 8, value: 38 },
    CodePattern { bits: 0b00101000, len: 8, value: 39 },
    CodePattern { bits: 0b00101001, len: 8, value: 40 },
    CodePattern { bits: 0b00101010, len: 8, value: 41 },
    CodePattern { bits: 0b00101011, len: 8, value: 42 },
    CodePattern { bits: 0b00101100, len: 8, value: 43 },
    CodePattern { bits: 0b00101101, len: 8, value: 44 },
    CodePattern { bits: 0b00000100, len: 8, value: 45 },
    CodePattern { bits: 0b00000101, len: 8, value: 46 },
    CodePattern { bits: 0b00001010, len: 8, value: 47 },
    CodePattern { bits: 0b00001011, len: 8, value: 48 },
    CodePattern { bits: 0b01010010, len: 8, value: 49 },
    CodePattern { bits: 0b01010011, len: 8, value: 50 },
    CodePattern { bits: 0b01010100, len: 8, value: 51 },
    CodePattern { bits: 0b01010101, len: 8, value: 52 },
    CodePattern { bits: 0b00100100, len: 8, value: 53 },
    CodePattern { bits: 0b00100101, len: 8, value: 54 },
    CodePattern { bits: 0b01011000, len: 8, value: 55 },
    CodePattern { bits: 0b01011001, len: 8, value: 56 },
    CodePattern { bits: 0b01011010, len: 8, value: 57 },
    CodePattern { bits: 0b01011011, len: 8, value: 58 },
    CodePattern { bits: 0b01001010, len: 8, value: 59 },
    CodePattern { bits: 0b01001011, len: 8, value: 60 },
    CodePattern { bits: 0b00110010, len: 8, value: 61 },
    CodePattern { bits: 0b00110011, len: 8, value: 62 },
    CodePattern { bits: 0b00110100, len: 8, value: 63 },
];

// White run-length make-up codes (T.4 Table 2). Runs 64..1728 in 64s.
const WHITE_MAKEUP: [CodePattern; 27] = [
    CodePattern { bits: 0b11011, len: 5, value: 64 },
    CodePattern { bits: 0b10010, len: 5, value: 128 },
    CodePattern { bits: 0b010111, len: 6, value: 192 },
    CodePattern { bits: 0b0110111, len: 7, value: 256 },
    CodePattern { bits: 0b00110110, len: 8, value: 320 },
    CodePattern { bits: 0b00110111, len: 8, value: 384 },
    CodePattern { bits: 0b01100100, len: 8, value: 448 },
    CodePattern { bits: 0b01100101, len: 8, value: 512 },
    CodePattern { bits: 0b01101000, len: 8, value: 576 },
    CodePattern { bits: 0b01100111, len: 8, value: 640 },
    CodePattern { bits: 0b011001100, len: 9, value: 704 },
    CodePattern { bits: 0b011001101, len: 9, value: 768 },
    CodePattern { bits: 0b011010010, len: 9, value: 832 },
    CodePattern { bits: 0b011010011, len: 9, value: 896 },
    CodePattern { bits: 0b011010100, len: 9, value: 960 },
    CodePattern { bits: 0b011010101, len: 9, value: 1024 },
    CodePattern { bits: 0b011010110, len: 9, value: 1088 },
    CodePattern { bits: 0b011010111, len: 9, value: 1152 },
    CodePattern { bits: 0b011011000, len: 9, value: 1216 },
    CodePattern { bits: 0b011011001, len: 9, value: 1280 },
    CodePattern { bits: 0b011011010, len: 9, value: 1344 },
    CodePattern { bits: 0b011011011, len: 9, value: 1408 },
    CodePattern { bits: 0b010011000, len: 9, value: 1472 },
    CodePattern { bits: 0b010011001, len: 9, value: 1536 },
    CodePattern { bits: 0b010011010, len: 9, value: 1600 },
    CodePattern { bits: 0b011000, len: 6, value: 1664 },
    CodePattern { bits: 0b010011011, len: 9, value: 1728 },
];

// Black run-length terminating codes (T.4 Table 3). Run lengths 0..63.
const BLACK_TERM: [CodePattern; 64] = [
    CodePattern { bits: 0b0000110111, len: 10, value: 0 },
    CodePattern { bits: 0b010, len: 3, value: 1 },
    CodePattern { bits: 0b11, len: 2, value: 2 },
    CodePattern { bits: 0b10, len: 2, value: 3 },
    CodePattern { bits: 0b011, len: 3, value: 4 },
    CodePattern { bits: 0b0011, len: 4, value: 5 },
    CodePattern { bits: 0b0010, len: 4, value: 6 },
    CodePattern { bits: 0b00011, len: 5, value: 7 },
    CodePattern { bits: 0b000101, len: 6, value: 8 },
    CodePattern { bits: 0b000100, len: 6, value: 9 },
    CodePattern { bits: 0b0000100, len: 7, value: 10 },
    CodePattern { bits: 0b0000101, len: 7, value: 11 },
    CodePattern { bits: 0b0000111, len: 7, value: 12 },
    CodePattern { bits: 0b00000100, len: 8, value: 13 },
    CodePattern { bits: 0b00000111, len: 8, value: 14 },
    CodePattern { bits: 0b000011000, len: 9, value: 15 },
    CodePattern { bits: 0b0000010111, len: 10, value: 16 },
    CodePattern { bits: 0b0000011000, len: 10, value: 17 },
    CodePattern { bits: 0b0000001000, len: 10, value: 18 },
    CodePattern { bits: 0b00001100111, len: 11, value: 19 },
    CodePattern { bits: 0b00001101000, len: 11, value: 20 },
    CodePattern { bits: 0b00001101100, len: 11, value: 21 },
    CodePattern { bits: 0b00000110111, len: 11, value: 22 },
    CodePattern { bits: 0b00000101000, len: 11, value: 23 },
    CodePattern { bits: 0b00000010111, len: 11, value: 24 },
    CodePattern { bits: 0b00000011000, len: 11, value: 25 },
    CodePattern { bits: 0b000011001010, len: 12, value: 26 },
    CodePattern { bits: 0b000011001011, len: 12, value: 27 },
    CodePattern { bits: 0b000011001100, len: 12, value: 28 },
    CodePattern { bits: 0b000011001101, len: 12, value: 29 },
    CodePattern { bits: 0b000001101000, len: 12, value: 30 },
    CodePattern { bits: 0b000001101001, len: 12, value: 31 },
    CodePattern { bits: 0b000001101010, len: 12, value: 32 },
    CodePattern { bits: 0b000001101011, len: 12, value: 33 },
    CodePattern { bits: 0b000011010010, len: 12, value: 34 },
    CodePattern { bits: 0b000011010011, len: 12, value: 35 },
    CodePattern { bits: 0b000011010100, len: 12, value: 36 },
    CodePattern { bits: 0b000011010101, len: 12, value: 37 },
    CodePattern { bits: 0b000011010110, len: 12, value: 38 },
    CodePattern { bits: 0b000011010111, len: 12, value: 39 },
    CodePattern { bits: 0b000001101100, len: 12, value: 40 },
    CodePattern { bits: 0b000001101101, len: 12, value: 41 },
    CodePattern { bits: 0b000011011010, len: 12, value: 42 },
    CodePattern { bits: 0b000011011011, len: 12, value: 43 },
    CodePattern { bits: 0b000001010100, len: 12, value: 44 },
    CodePattern { bits: 0b000001010101, len: 12, value: 45 },
    CodePattern { bits: 0b000001010110, len: 12, value: 46 },
    CodePattern { bits: 0b000001010111, len: 12, value: 47 },
    CodePattern { bits: 0b000001100100, len: 12, value: 48 },
    CodePattern { bits: 0b000001100101, len: 12, value: 49 },
    CodePattern { bits: 0b000001010010, len: 12, value: 50 },
    CodePattern { bits: 0b000001010011, len: 12, value: 51 },
    CodePattern { bits: 0b000000100100, len: 12, value: 52 },
    CodePattern { bits: 0b000000110111, len: 12, value: 53 },
    CodePattern { bits: 0b000000111000, len: 12, value: 54 },
    CodePattern { bits: 0b000000100111, len: 12, value: 55 },
    CodePattern { bits: 0b000000101000, len: 12, value: 56 },
    CodePattern { bits: 0b000001011000, len: 12, value: 57 },
    CodePattern { bits: 0b000001011001, len: 12, value: 58 },
    CodePattern { bits: 0b000000101011, len: 12, value: 59 },
    CodePattern { bits: 0b000000101100, len: 12, value: 60 },
    CodePattern { bits: 0b000001011010, len: 12, value: 61 },
    CodePattern { bits: 0b000001100110, len: 12, value: 62 },
    CodePattern { bits: 0b000001100111, len: 12, value: 63 },
];

// Black run-length make-up codes (T.4 Table 3). Runs 64..1728 in 64s.
const BLACK_MAKEUP: [CodePattern; 27] = [
    CodePattern { bits: 0b0000001111, len: 10, value: 64 },
    CodePattern { bits: 0b000011001000, len: 12, value: 128 },
    CodePattern { bits: 0b000011001001, len: 12, value: 192 },
    CodePattern { bits: 0b000001011011, len: 12, value: 256 },
    CodePattern { bits: 0b000000110011, len: 12, value: 320 },
    CodePattern { bits: 0b000000110100, len: 12, value: 384 },
    CodePattern { bits: 0b000000110101, len: 12, value: 448 },
    CodePattern { bits: 0b0000001101100, len: 13, value: 512 },
    CodePattern { bits: 0b0000001101101, len: 13, value: 576 },
    CodePattern { bits: 0b0000001001010, len: 13, value: 640 },
    CodePattern { bits: 0b0000001001011, len: 13, value: 704 },
    CodePattern { bits: 0b0000001001100, len: 13, value: 768 },
    CodePattern { bits: 0b0000001001101, len: 13, value: 832 },
    CodePattern { bits: 0b0000001110010, len: 13, value: 896 },
    CodePattern { bits: 0b0000001110011, len: 13, value: 960 },
    CodePattern { bits: 0b0000001110100, len: 13, value: 1024 },
    CodePattern { bits: 0b0000001110101, len: 13, value: 1088 },
    CodePattern { bits: 0b0000001110110, len: 13, value: 1152 },
    CodePattern { bits: 0b0000001110111, len: 13, value: 1216 },
    CodePattern { bits: 0b0000001010010, len: 13, value: 1280 },
    CodePattern { bits: 0b0000001010011, len: 13, value: 1344 },
    CodePattern { bits: 0b0000001010100, len: 13, value: 1408 },
    CodePattern { bits: 0b0000001010101, len: 13, value: 1472 },
    CodePattern { bits: 0b0000001011010, len: 13, value: 1536 },
    CodePattern { bits: 0b0000001011011, len: 13, value: 1600 },
    CodePattern { bits: 0b0000001100100, len: 13, value: 1664 },
    CodePattern { bits: 0b0000001100101, len: 13, value: 1728 },
];

// Common make-up codes for runs 1792..2560 (T.4 Table 4).
const COMMON_MAKEUP: [CodePattern; 13] = [
    CodePattern { bits: 0b00000001000, len: 11, value: 1792 },
    CodePattern { bits: 0b00000001100, len: 11, value: 1856 },
    CodePattern { bits: 0b00000001101, len: 11, value: 1920 },
    CodePattern { bits: 0b000000010010, len: 12, value: 1984 },
    CodePattern { bits: 0b000000010011, len: 12, value: 2048 },
    CodePattern { bits: 0b000000010100, len: 12, value: 2112 },
    CodePattern { bits: 0b000000010101, len: 12, value: 2176 },
    CodePattern { bits: 0b000000010110, len: 12, value: 2240 },
    CodePattern { bits: 0b000000010111, len: 12, value: 2304 },
    CodePattern { bits: 0b000000011100, len: 12, value: 2368 },
    CodePattern { bits: 0b000000011101, len: 12, value: 2432 },
    CodePattern { bits: 0b000000011110, len: 12, value: 2496 },
    CodePattern { bits: 0b000000011111, len: 12, value: 2560 },
];

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
enum Mode {
    Pass,
    Horizontal,
    V0,
    VR1,
    VR2,
    VR3,
    VL1,
    VL2,
    VL3,
    Extension2D,
}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
struct ModePattern {
    bits: u16,
    len: u8,
    mode: Mode,
}

// Sorted by length ascending so shorter prefixes win.
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const MODE_CODES: [ModePattern; 10] = [
    ModePattern { bits: 0b1, len: 1, mode: Mode::V0 },
    ModePattern { bits: 0b011, len: 3, mode: Mode::VR1 },
    ModePattern { bits: 0b010, len: 3, mode: Mode::VL1 },
    ModePattern { bits: 0b001, len: 3, mode: Mode::Horizontal },
    ModePattern { bits: 0b0001, len: 4, mode: Mode::Pass },
    ModePattern { bits: 0b000011, len: 6, mode: Mode::VR2 },
    ModePattern { bits: 0b000010, len: 6, mode: Mode::VL2 },
    ModePattern { bits: 0b0000011, len: 7, mode: Mode::VR3 },
    ModePattern { bits: 0b0000010, len: 7, mode: Mode::VL3 },
    ModePattern { bits: 0b0000001, len: 7, mode: Mode::Extension2D },
];

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const MAX_RUN_CODE_BITS: i32 = 13;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const MAX_MODE_CODE_BITS: i32 = 7;
const EOL_PATTERN: u32 = 0x001;

// ---------------------------------------------------------------------------
// Code-table lookup helpers.
// ---------------------------------------------------------------------------

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn match_code(r: &mut BitReader, table: &[CodePattern], peek_max: i32) -> i32 {
    let bits = r.peek(peek_max);
    for c in table {
        let prefix = (bits >> (peek_max - c.len as i32)) & ((1u32 << c.len) - 1);
        if prefix == c.bits as u32 {
            r.consume(c.len as i32);
            return c.value as i32;
        }
    }
    -1
}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn decode_run_length(r: &mut BitReader, is_white: bool) -> Result<i32, Error> {
    let mut total: i32 = 0;
    loop {
        let term_table = if is_white { &WHITE_TERM[..] } else { &BLACK_TERM[..] };
        let term = match_code(r, term_table, MAX_RUN_CODE_BITS);
        if term >= 0 {
            return Ok(total + term);
        }
        let mk_table = if is_white { &WHITE_MAKEUP[..] } else { &BLACK_MAKEUP[..] };
        let mk = match_code(r, mk_table, MAX_RUN_CODE_BITS);
        if mk > 0 {
            total += mk;
            continue;
        }
        let cm = match_code(r, &COMMON_MAKEUP[..], MAX_RUN_CODE_BITS);
        if cm > 0 {
            total += cm;
            continue;
        }
        return Err(Error::InvalidRunLengthCode { is_white });
    }
}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn decode_mode(r: &mut BitReader) -> Result<Mode, Error> {
    let bits = r.peek(MAX_MODE_CODE_BITS);
    for c in &MODE_CODES {
        let prefix = (bits >> (MAX_MODE_CODE_BITS - c.len as i32)) & ((1u32 << c.len) - 1);
        if prefix == c.bits as u32 {
            r.consume(c.len as i32);
            return Ok(c.mode);
        }
    }
    Err(Error::InvalidModeCode)
}

/// Consume zero-bit "fill" followed by the EOL's terminating 1 bit (T.4 §3.3).
/// Returns `false` if the stream ended before the EOL completed.
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn consume_eol(r: &mut BitReader) -> bool {
    let mut zeros = 0i32;
    while r.bits_available() > 0 {
        let bit = r.peek(1);
        r.consume(1);
        if bit == 0 {
            zeros += 1;
        } else {
            return zeros >= 11;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// 2D row decoding helpers.
// ---------------------------------------------------------------------------

fn find_b1(ref_changes: &[i32], a0: i32, a0_color: i32, columns: i32) -> i32 {
    let target_parity = a0_color;
    for (i, &x) in ref_changes.iter().enumerate() {
        if x <= a0 {
            continue;
        }
        if (i as i32 & 1) == target_parity {
            return x;
        }
    }
    columns
}

fn find_b2(ref_changes: &[i32], b1: i32, columns: i32) -> i32 {
    for &x in ref_changes {
        if x > b1 {
            return x;
        }
    }
    columns
}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn emit_range(row: &mut [u8], from: i32, to: i32, columns: i32, color: u8) {
    let lo = if from < 0 { 0 } else { from };
    let hi = if to > columns { columns } else { to };
    if lo >= hi {
        return;
    }
    for x in lo..hi {
        row[x as usize] = color;
    }
}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn decode_row_2d(
    r: &mut BitReader,
    columns: i32,
    ref_changes: &[i32],
    cur_changes: &mut Vec<i32>,
    row: &mut Vec<u8>,
) -> Result<(), Error> {
    row.clear();
    row.resize(columns as usize, 0u8);
    cur_changes.clear();

    let mut a0: i32 = -1;
    let mut a0_color: i32 = 0; // 0 = white, 1 = black

    while a0 < columns {
        let b1 = find_b1(ref_changes, a0, a0_color, columns);
        let b2 = find_b2(ref_changes, b1, columns);

        let m = decode_mode(r)?;
        match m {
            Mode::Pass => {
                emit_range(row, a0, b2, columns, a0_color as u8);
                a0 = b2;
            }
            Mode::V0 | Mode::VR1 | Mode::VR2 | Mode::VR3
            | Mode::VL1 | Mode::VL2 | Mode::VL3 => {
                let offset = match m {
                    Mode::V0 => 0,
                    Mode::VR1 => 1,
                    Mode::VR2 => 2,
                    Mode::VR3 => 3,
                    Mode::VL1 => -1,
                    Mode::VL2 => -2,
                    Mode::VL3 => -3,
                    _ => 0,
                };
                let new_a0 = b1 + offset;
                if new_a0 < 0 || new_a0 > columns {
                    return Err(Error::VModeOutOfBounds);
                }
                emit_range(row, a0, new_a0, columns, a0_color as u8);
                if new_a0 < columns {
                    cur_changes.push(new_a0);
                }
                a0 = new_a0;
                a0_color = 1 - a0_color;
            }
            Mode::Horizontal => {
                let first_color = a0_color as u8;
                let run1 = decode_run_length(r, first_color == 0)?;
                let run2 = decode_run_length(r, first_color != 0)?;
                let from_x = if a0 < 0 { 0 } else { a0 };
                let after_first = from_x + run1;
                let after_second = after_first + run2;
                if after_second > columns {
                    return Err(Error::HModeExceedsColumns);
                }
                emit_range(row, a0, after_first, columns, first_color);
                emit_range(
                    row,
                    after_first,
                    after_second,
                    columns,
                    1 - first_color,
                );
                a0 = after_second;
                // a0_color toggles twice through H — unchanged.
            }
            Mode::Extension2D => return Err(Error::UnsupportedExtension),
        }
    }

    if a0 != columns {
        return Err(Error::RowCursorMismatch);
    }

    // Re-derive cur_changes from the filled row to handle H-mode edges
    // cleanly (matches the peer C++ port's strategy).
    cur_changes.clear();
    let mut prev_color: u8 = 0;
    for (x, &v) in row.iter().enumerate() {
        if v != prev_color {
            cur_changes.push(x as i32);
            prev_color = v;
        }
    }
    Ok(())
}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn decode_row_1d(
    r: &mut BitReader,
    columns: i32,
    row: &mut Vec<u8>,
) -> Result<(), Error> {
    row.clear();
    row.resize(columns as usize, 0u8);
    let mut x = 0i32;
    let mut is_white = true;
    while x < columns {
        let run = decode_run_length(r, is_white)?;
        if run < 0 || x + run > columns {
            return Err(Error::RunExceedsColumns);
        }
        let color: u8 = if is_white { 0 } else { 1 };
        emit_range(row, x, x + run, columns, color);
        x += run;
        is_white = !is_white;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Output bit packing.
// ---------------------------------------------------------------------------

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn pack_bits(
    rows: &[Vec<u8>],
    columns: i32,
    black_is_1: bool,
    out: &mut Vec<u8>,
) {
    let bytes_per_row = ((columns + 7) / 8) as usize;
    out.clear();
    out.resize(bytes_per_row * rows.len(), 0u8);
    for (r_idx, row) in rows.iter().enumerate() {
        for c in 0..columns as usize {
            let v = row[c];
            let out_bit = if black_is_1 { 1 - v } else { v };
            if out_bit != 0 {
                out[r_idx * bytes_per_row + (c / 8)] |= 1u8 << (7 - (c % 8));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// CFAX framing reader (test-fixture entry point).
// ---------------------------------------------------------------------------

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const CFAX_MAGIC: &[u8; 4] = b"CFAX";

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
struct ParsedFixture {
    params: Params,
    body: Vec<u8>,
}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn parse_cfax(file: &[u8]) -> Result<ParsedFixture, Error> {
    if file.len() < 16 {
        return Err(Error::CfaxMalformed("too short for CFAX header"));
    }
    if &file[0..4] != CFAX_MAGIC {
        return Err(Error::CfaxMalformed("missing CFAX magic"));
    }
    if file[4] != 1 {
        return Err(Error::CfaxMalformed("unsupported version"));
    }
    let k = file[5] as i8;
    let flags = file[6];
    let damaged_rows_before_error = file[7] as u32;
    let columns = ((file[8] as u32) << 8) | (file[9] as u32);
    let rows = ((file[10] as u32) << 24)
        | ((file[11] as u32) << 16)
        | ((file[12] as u32) << 8)
        | (file[13] as u32);
    let body_len = ((file[14] as u32) << 8) | (file[15] as u32);
    if file.len() != 16 + body_len as usize {
        return Err(Error::CfaxMalformed(
            "declared body length does not match file size",
        ));
    }
    let params = Params {
        k,
        end_of_line: (flags & 0x01) != 0,
        encoded_byte_align: (flags & 0x02) != 0,
        end_of_block: (flags & 0x04) != 0,
        black_is_1: (flags & 0x08) != 0,
        columns,
        rows,
        damaged_rows_before_error,
    };
    Ok(ParsedFixture {
        params,
        body: file[16..].to_vec(),
    })
}

// ---------------------------------------------------------------------------
// Public API.
// ---------------------------------------------------------------------------

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub fn decode(stream: &[u8], params: &Params) -> Result<DecodedImage, Error> {
    if params.columns == 0 {
        return Err(Error::InvalidColumns);
    }
    if stream.is_empty() {
        return Err(Error::EmptyStream);
    }

    let mut r = BitReader::new(stream);
    let mut rows: Vec<Vec<u8>> = Vec::new();
    let mut ref_changes: Vec<i32> = Vec::new();
    let mut cur_changes: Vec<i32> = Vec::new();
    let mut row: Vec<u8> = Vec::new();

    let columns = params.columns as i32;
    let target_rows = params.rows as i32;

    if params.k < 0 {
        // Group 4 (T.6) — pure 2D, no per-row EOL. EOFB terminates.
        let mut row_index = 0i32;
        loop {
            if params.end_of_block {
                let peek = r.peek(24);
                let eofb = (EOL_PATTERN << 12) | EOL_PATTERN;
                if peek == eofb {
                    r.consume(24);
                    break;
                }
            }
            if target_rows > 0 && row_index >= target_rows {
                break;
            }
            if r.bits_available() <= 0 {
                break;
            }
            decode_row_2d(&mut r, columns, &ref_changes, &mut cur_changes, &mut row)?;
            rows.push(row.clone());
            ref_changes = cur_changes.clone();
            row_index += 1;
        }
    } else if params.k == 0 {
        // Group 3 1D.
        let mut row_index = 0i32;
        loop {
            if target_rows > 0 && row_index >= target_rows {
                break;
            }
            if r.bits_available() <= 0 {
                break;
            }
            if params.end_of_line
                && !consume_eol(&mut r) {
                    return Err(Error::MissingEol);
                }
            if params.encoded_byte_align {
                r.align_to_byte();
            }
            decode_row_1d(&mut r, columns, &mut row)?;
            rows.push(row.clone());
            row_index += 1;
        }
    } else {
        return Err(Error::UnsupportedMixedGroup3);
    }

    let mut bits = Vec::new();
    pack_bits(&rows, columns, params.black_is_1, &mut bits);

    Ok(DecodedImage {
        width: params.columns,
        height: rows.len() as u32,
        bits,
    })
}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub fn parse(file: &[u8]) -> Result<DecodedImage, Error> {
    let pf = parse_cfax(file)?;
    decode(&pf.body, &pf.params)
}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub fn to_canonical(image: &DecodedImage) -> String {
    let bytes_per_row = (image.width as usize).div_ceil(8);
    let mut out = String::with_capacity(image.bits.len() * 3 + 32);
    out.push_str(&format!("width={}\n", image.width));
    out.push_str(&format!("height={}\n", image.height));
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for r in 0..image.height as usize {
        out.push_str(&format!("row {} ", r));
        for c in 0..bytes_per_row {
            let b = image.bits[r * bytes_per_row + c];
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 0x0f) as usize] as char);
        }
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// Encoder (Group 3 1D + Group 4) — TIFF strip payload producer.
// ---------------------------------------------------------------------------
//
// Encode input convention mirrors the BlackIsZero 1bpp strip the
// foundation tiff_encoder produces: MSB-first bit packing,
// scanlines padded to whole bytes, **bit = 1 means white**
// (matches PDF /BlackIs1 = true). Group 3 emits one 12-bit EOL
// before every scanline followed by alternating white/black
// run-length codes (T.4 Tables 2-4) — `Params { k: 0,
// end_of_line: true, black_is_1: true, columns, rows, .. }`
// round-trips through [`decode`]. Group 4 emits a 2D-coded
// stream terminated by EOFB (two consecutive EOLs) — `Params
// { k: -1, end_of_block: true, black_is_1: true, columns,
// rows, .. }` round-trips.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    InvalidColumns,
    InvalidRows,
    BitsLengthMismatch { expected: usize, got: usize },
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncodeError::InvalidColumns => f.write_str("CCITT encode: columns must be > 0"),
            EncodeError::InvalidRows => f.write_str("CCITT encode: rows must be > 0"),
            EncodeError::BitsLengthMismatch { expected, got } => {
                write!(f, "CCITT encode: bits length {got} != ((columns+7)/8)*rows = {expected}")
            }
        }
    }
}

impl std::error::Error for EncodeError {}

struct BitWriter {
    out: Vec<u8>,
    cur: u8,
    used: u8, // bits already filled in `cur`, 0..=7
}

impl BitWriter {
    fn new() -> Self {
        Self { out: Vec::new(), cur: 0, used: 0 }
    }

    fn write_bits(&mut self, value: u32, len: u8) {
        debug_assert!(len <= 16);
        for i in (0..len).rev() {
            let bit = ((value >> i) & 1) as u8;
            self.cur |= bit << (7 - self.used);
            self.used += 1;
            if self.used == 8 {
                self.out.push(self.cur);
                self.cur = 0;
                self.used = 0;
            }
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.used > 0 {
            self.out.push(self.cur);
        }
        self.out
    }
}

// Largest color-specific make-up code <= run (run in [64, 1791]).
// Returns (bits, len, value).
fn largest_makeup_le(run: i32, is_white: bool) -> (u16, u8, i32) {
    let table = if is_white { &WHITE_MAKEUP[..] } else { &BLACK_MAKEUP[..] };
    let m = ((run / 64) * 64).min(1728) as i16;
    let c = table.iter().find(|c| c.value == m).expect("make-up code present");
    (c.bits, c.len, c.value as i32)
}

// Largest common (color-shared) make-up code <= run (run in [1792, ..]).
fn largest_common_makeup_le(run: i32) -> (u16, u8, i32) {
    let m = (1792 + ((run - 1792) / 64) * 64).min(2560) as i16;
    let c = COMMON_MAKEUP
        .iter()
        .find(|c| c.value == m)
        .expect("common make-up code present");
    (c.bits, c.len, c.value as i32)
}

fn emit_run(w: &mut BitWriter, mut run: i32, is_white: bool) {
    while run >= 1792 {
        let (bits, len, value) = largest_common_makeup_le(run);
        w.write_bits(bits as u32, len);
        run -= value;
    }
    while run >= 64 {
        let (bits, len, value) = largest_makeup_le(run, is_white);
        w.write_bits(bits as u32, len);
        run -= value;
    }
    // Terminator (0..=63).
    let table = if is_white { &WHITE_TERM[..] } else { &BLACK_TERM[..] };
    let t = &table[run as usize];
    w.write_bits(t.bits as u32, t.len);
}

fn row_changes(bits: &[u8], columns: u32, row_idx: u32) -> Vec<i32> {
    let stride = columns.div_ceil(8) as usize;
    let row_off = row_idx as usize * stride;
    let row_slice = &bits[row_off..row_off + stride];
    let mut changes: Vec<i32> = Vec::new();
    // Internal convention: 0 = white, 1 = black (mirrors the
    // decoder's row buffer). Input has bit=1 == white; invert to
    // align with the decoder's run-table parity.
    let mut prev_color: u8 = 0;
    for x in 0..columns as usize {
        let byte = row_slice[x / 8];
        let bit = (byte >> (7 - (x % 8))) & 1;
        let color: u8 = if bit != 0 { 0 } else { 1 };
        if color != prev_color {
            changes.push(x as i32);
            prev_color = color;
        }
    }
    changes
}

fn encode_row_1d(w: &mut BitWriter, changes: &[i32], columns: i32) {
    let mut prev: i32 = 0;
    let mut is_white = true;
    for &c in changes {
        emit_run(w, c - prev, is_white);
        prev = c;
        is_white = !is_white;
    }
    emit_run(w, columns - prev, is_white);
}

fn write_vertical(w: &mut BitWriter, delta: i32) {
    let (bits, len): (u32, u8) = match delta {
        0 => (0b1, 1),
        1 => (0b011, 3),
        -1 => (0b010, 3),
        2 => (0b000011, 6),
        -2 => (0b000010, 6),
        3 => (0b0000011, 7),
        -3 => (0b0000010, 7),
        _ => unreachable!("vertical delta out of range"),
    };
    w.write_bits(bits, len);
}

fn encode_row_2d(
    w: &mut BitWriter,
    columns: i32,
    ref_changes: &[i32],
    cur_changes: &[i32],
) {
    let mut a0: i32 = -1;
    let mut a0_color: i32 = 0;
    while a0 < columns {
        let a1 = cur_changes
            .iter()
            .copied()
            .find(|&x| x > a0)
            .unwrap_or(columns);
        let b1 = find_b1(ref_changes, a0, a0_color, columns);
        let b2 = find_b2(ref_changes, b1, columns);

        if b2 < a1 {
            // Pass: 0001
            w.write_bits(0b0001, 4);
            a0 = b2;
        } else if (a1 - b1).abs() <= 3 {
            write_vertical(w, a1 - b1);
            a0 = a1;
            a0_color = 1 - a0_color;
        } else {
            // Horizontal: 001, then two runs.
            let a2 = cur_changes
                .iter()
                .copied()
                .find(|&x| x > a1)
                .unwrap_or(columns);
            w.write_bits(0b001, 3);
            let from_x = if a0 < 0 { 0 } else { a0 };
            let run1 = a1 - from_x;
            let run2 = a2 - a1;
            emit_run(w, run1, a0_color == 0);
            emit_run(w, run2, a0_color != 0);
            a0 = a2;
        }
    }
}

fn validate_encode_inputs(
    bits: &[u8],
    columns: u32,
    rows: u32,
) -> Result<(), EncodeError> {
    if columns == 0 {
        return Err(EncodeError::InvalidColumns);
    }
    if rows == 0 {
        return Err(EncodeError::InvalidRows);
    }
    let expected = (columns as usize).div_ceil(8) * rows as usize;
    if bits.len() != expected {
        return Err(EncodeError::BitsLengthMismatch {
            expected,
            got: bits.len(),
        });
    }
    Ok(())
}

/// Encode `bits` (packed BlackIsZero 1bpp, MSB-first, bit=1=white)
/// as a CCITT Group 3 1D byte stream. The output starts each
/// scanline with an EOL code and contains no byte alignment
/// or end-of-block sentinel — sufficient for TIFF
/// Compression=3 with T4Options=0.
pub fn encode_group3(
    bits: &[u8],
    columns: u32,
    rows: u32,
) -> Result<Vec<u8>, EncodeError> {
    validate_encode_inputs(bits, columns, rows)?;
    let mut w = BitWriter::new();
    for r in 0..rows {
        w.write_bits(EOL_PATTERN, 12);
        let changes = row_changes(bits, columns, r);
        encode_row_1d(&mut w, &changes, columns as i32);
    }
    Ok(w.finish())
}

/// Encode `bits` (packed BlackIsZero 1bpp) as a CCITT Group 4
/// byte stream (T.6, k = -1). The output is the concatenation
/// of 2D-coded rows followed by EOFB (two 12-bit EOL patterns)
/// — sufficient for TIFF Compression=4 with T6Options=0.
pub fn encode_group4(
    bits: &[u8],
    columns: u32,
    rows: u32,
) -> Result<Vec<u8>, EncodeError> {
    validate_encode_inputs(bits, columns, rows)?;
    let mut w = BitWriter::new();
    let mut ref_changes: Vec<i32> = Vec::new();
    for r in 0..rows {
        let cur = row_changes(bits, columns, r);
        encode_row_2d(&mut w, columns as i32, &ref_changes, &cur);
        ref_changes = cur;
    }
    // EOFB: two consecutive 12-bit EOL codes.
    w.write_bits(EOL_PATTERN, 12);
    w.write_bits(EOL_PATTERN, 12);
    Ok(w.finish())
}

#[cfg(test)]
mod encode_tests {
    use super::*;

    fn make_bits(columns: u32, rows: u32, mut pixel: impl FnMut(u32, u32) -> bool) -> Vec<u8> {
        let stride = columns.div_ceil(8) as usize;
        let mut bits = vec![0u8; stride * rows as usize];
        for y in 0..rows {
            for x in 0..columns {
                if pixel(x, y) {
                    bits[y as usize * stride + (x / 8) as usize] |= 0x80 >> (x % 8);
                }
            }
        }
        bits
    }

    fn round_trip(bits: &[u8], columns: u32, rows: u32, k: i8) {
        let encoded = match k {
            0 => encode_group3(bits, columns, rows).expect("encode_group3"),
            -1 => encode_group4(bits, columns, rows).expect("encode_group4"),
            _ => panic!("test only supports k in {{0,-1}}"),
        };
        let params = Params {
            k,
            end_of_line: k == 0,
            encoded_byte_align: false,
            end_of_block: k == -1,
            black_is_1: true,
            columns,
            rows,
            damaged_rows_before_error: 0,
        };
        let decoded = decode(&encoded, &params).expect("decode encoded bytes");
        assert_eq!(decoded.width, columns);
        assert_eq!(decoded.height, rows);
        assert_eq!(decoded.bits, bits, "round-trip mismatch (k={k})");
    }

    #[test]
    fn g3_all_white() {
        let bits = make_bits(64, 4, |_x, _y| true);
        round_trip(&bits, 64, 4, 0);
    }

    #[test]
    fn g3_all_black() {
        let bits = make_bits(64, 4, |_x, _y| false);
        round_trip(&bits, 64, 4, 0);
    }

    #[test]
    fn g3_vertical_stripes() {
        let bits = make_bits(128, 8, |x, _y| (x / 8) % 2 == 0);
        round_trip(&bits, 128, 8, 0);
    }

    #[test]
    fn g3_diagonal() {
        let bits = make_bits(32, 32, |x, y| x != y);
        round_trip(&bits, 32, 32, 0);
    }

    #[test]
    fn g3_long_run() {
        let bits = make_bits(2000, 2, |_x, _y| true);
        round_trip(&bits, 2000, 2, 0);
    }

    #[test]
    fn g3_make_up_chain_above_1792() {
        let bits = make_bits(2400, 1, |_x, _y| false);
        round_trip(&bits, 2400, 1, 0);
    }

    #[test]
    fn g3_make_up_chain_above_2560() {
        let bits = make_bits(5000, 1, |_x, _y| false);
        round_trip(&bits, 5000, 1, 0);
    }

    #[test]
    fn g4_all_white() {
        let bits = make_bits(64, 4, |_x, _y| true);
        round_trip(&bits, 64, 4, -1);
    }

    #[test]
    fn g4_all_black() {
        let bits = make_bits(64, 4, |_x, _y| false);
        round_trip(&bits, 64, 4, -1);
    }

    #[test]
    fn g4_horizontal_stripes() {
        let bits = make_bits(64, 16, |_x, y| (y / 4) % 2 == 0);
        round_trip(&bits, 64, 16, -1);
    }

    #[test]
    fn g4_vertical_stripes() {
        let bits = make_bits(128, 8, |x, _y| (x / 8) % 2 == 0);
        round_trip(&bits, 128, 8, -1);
    }

    #[test]
    fn g4_diagonal() {
        let bits = make_bits(32, 32, |x, y| x != y);
        round_trip(&bits, 32, 32, -1);
    }

    #[test]
    fn g4_isolated_pixels() {
        let bits = make_bits(32, 32, |x, y| !((x % 5 == 0) && (y % 7 == 0)));
        round_trip(&bits, 32, 32, -1);
    }

    #[test]
    fn g4_long_run() {
        let bits = make_bits(2400, 2, |x, y| (x + y) % 2 == 0);
        round_trip(&bits, 2400, 2, -1);
    }

    #[test]
    fn rejects_zero_columns() {
        let err = encode_group3(&[], 0, 1).unwrap_err();
        assert_eq!(err, EncodeError::InvalidColumns);
    }

    #[test]
    fn rejects_zero_rows() {
        let err = encode_group3(&[], 8, 0).unwrap_err();
        assert_eq!(err, EncodeError::InvalidRows);
    }

    #[test]
    fn rejects_bits_length_mismatch() {
        let err = encode_group3(&[0u8; 3], 16, 1).unwrap_err();
        assert!(matches!(err, EncodeError::BitsLengthMismatch { expected: 2, got: 3 }));
    }
}
