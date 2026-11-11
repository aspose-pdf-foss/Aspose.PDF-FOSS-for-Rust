// foundation/dctdecode — PDF /DCTDecode filter (JPEG / JFIF, ITU-T T.81).
//
// One-shot lossy decode of a complete JPEG byte stream into a uniform
// RGBA8 pixel buffer. Implements baseline (SOF0) + progressive (SOF2)
// DCT-based decoding, JFIF wrapper parsing, YCbCr → RGB conversion
// (BT.601), and h2v2 fancy chroma upsampling.
//
// Decode-only — DCTDecode in PDF is read-side. Encoding is a separate
// primitive (dct_encoder).
//
// References:
//   ITU-T T.81 (1992) — Baseline + progressive DCT.
//   ISO/IEC 10918-1 — JPEG.
//   JFIF 1.02 — File interchange format.
//   PDF 32000-1:2008 §7.4.8 — DCTDecode filter.
//
// See Feedstock/PDF/Spec/dctdecode.yaml + targets/rust/dctdecode.yaml.

use std::fmt;

// ============================================================
// Public types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub components: u8, // always 4 (RGBA8)
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    EmptyInput,
    NotJpeg,
    UnexpectedEnd,
    UnsupportedProcess(u8),
    UnsupportedMarker(u8),
    BadSegmentLength,
    BadDqt,
    BadDht,
    BadSof,
    BadSos,
    BadDri,
    SosBeforeSof,
    NoFrame,
    NoSos,
    UnsupportedPrecision(u8),
    UnsupportedComponentCount(u8),
    UnsupportedSamplingFactor { h: u8, v: u8 },
    UndefinedQuantTable(u8),
    UndefinedHuffmanTable { class: u8, id: u8 },
    BadHuffmanCode,
    BadProgressiveAc,
    BadRestart,
    DcOutOfRange,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::EmptyInput => f.write_str("dctdecode: empty input"),
            Error::NotJpeg => f.write_str("dctdecode: missing SOI marker"),
            Error::UnexpectedEnd => f.write_str("dctdecode: unexpected end of stream"),
            Error::UnsupportedProcess(m) =>
                write!(f, "dctdecode: unsupported JPEG process (SOF marker 0xFF{:02X})", m),
            Error::UnsupportedMarker(m) =>
                write!(f, "dctdecode: unsupported marker 0xFF{:02X}", m),
            Error::BadSegmentLength => f.write_str("dctdecode: malformed segment length"),
            Error::BadDqt => f.write_str("dctdecode: malformed DQT segment"),
            Error::BadDht => f.write_str("dctdecode: malformed DHT segment"),
            Error::BadSof => f.write_str("dctdecode: malformed SOF segment"),
            Error::BadSos => f.write_str("dctdecode: malformed SOS segment"),
            Error::BadDri => f.write_str("dctdecode: malformed DRI segment"),
            Error::SosBeforeSof => f.write_str("dctdecode: SOS before SOF"),
            Error::NoFrame => f.write_str("dctdecode: no SOF before EOI"),
            Error::NoSos => f.write_str("dctdecode: no SOS scan in stream"),
            Error::UnsupportedPrecision(p) =>
                write!(f, "dctdecode: unsupported precision {} (8 only)", p),
            Error::UnsupportedComponentCount(n) =>
                write!(f, "dctdecode: unsupported component count {} (1 or 3 only)", n),
            Error::UnsupportedSamplingFactor { h, v } =>
                write!(f, "dctdecode: unsupported sampling factor H={} V={}", h, v),
            Error::UndefinedQuantTable(id) =>
                write!(f, "dctdecode: undefined quant table {}", id),
            Error::UndefinedHuffmanTable { class, id } =>
                write!(f, "dctdecode: undefined {} Huffman table {}",
                    if *class == 0 { "DC" } else { "AC" }, id),
            Error::BadHuffmanCode => f.write_str("dctdecode: bad Huffman code"),
            Error::BadProgressiveAc => f.write_str("dctdecode: malformed progressive AC scan"),
            Error::BadRestart => f.write_str("dctdecode: missing or wrong restart marker"),
            Error::DcOutOfRange => f.write_str("dctdecode: DC predictor out of range"),
        }
    }
}

impl std::error::Error for Error {}

// ============================================================
// Constants
// ============================================================

// JPEG zig-zag scan order (T.81 Figure A.6). zigzag[k] is the
// row-major 8x8 index of the k-th coefficient in zig-zag order.
const ZIGZAG: [usize; 64] = [
     0,  1,  8, 16,  9,  2,  3, 10,
    17, 24, 32, 25, 18, 11,  4,  5,
    12, 19, 26, 33, 40, 48, 41, 34,
    27, 20, 13,  6,  7, 14, 21, 28,
    35, 42, 49, 56, 57, 50, 43, 36,
    29, 22, 15, 23, 30, 37, 44, 51,
    58, 59, 52, 45, 38, 31, 39, 46,
    53, 60, 61, 54, 47, 55, 62, 63,
];

// Markers (low byte; high is always 0xFF).
const M_SOF0: u8 = 0xC0; // baseline
const M_SOF2: u8 = 0xC2; // progressive
const M_DHT:  u8 = 0xC4;
const M_SOI:  u8 = 0xD8;
const M_EOI:  u8 = 0xD9;
const M_SOS:  u8 = 0xDA;
const M_DQT:  u8 = 0xDB;
const M_DRI:  u8 = 0xDD;
const M_RST0: u8 = 0xD0;
const M_COM:  u8 = 0xFE;

// ============================================================
// Bit reader (MSB-first; transparent 0xFF00 byte-stuffing).
// ============================================================
//
// Once a 0xFF Mxx (Mxx != 0x00) is seen the reader holds it in
// `marker` and reports `UnexpectedEnd` for further bit reads —
// the caller (scan body / restart loop) detects the marker via
// `take_marker()` after byte-aligning.

struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    bit_buf: u32,
    n_bits: u8,
    marker: Option<u8>,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0, bit_buf: 0, n_bits: 0, marker: None }
    }

    fn fill(&mut self) {
        while self.n_bits <= 24 {
            if self.marker.is_some() { return; }
            if self.pos >= self.data.len() { return; }
            let b = self.data[self.pos];
            self.pos += 1;
            if b == 0xFF {
                // byte-stuffing or marker
                if self.pos >= self.data.len() {
                    self.marker = Some(0); // treat as end
                    return;
                }
                let b2 = self.data[self.pos];
                self.pos += 1;
                if b2 == 0x00 {
                    // stuffed 0xFF
                    self.bit_buf = (self.bit_buf << 8) | 0xFF;
                    self.n_bits += 8;
                } else if b2 == 0xFF {
                    // run of 0xFF fill bytes — backtrack and re-process
                    self.pos -= 1;
                } else {
                    self.marker = Some(b2);
                    return;
                }
            } else {
                self.bit_buf = (self.bit_buf << 8) | b as u32;
                self.n_bits += 8;
            }
        }
    }

    fn read_bits(&mut self, n: u8) -> Result<u32, Error> {
        if n == 0 { return Ok(0); }
        if self.n_bits < n { self.fill(); }
        if self.n_bits < n { return Err(Error::UnexpectedEnd); }
        self.n_bits -= n;
        let v = (self.bit_buf >> self.n_bits) & ((1u32 << n) - 1);
        Ok(v)
    }

    fn read_bit(&mut self) -> Result<u32, Error> {
        self.read_bits(1)
    }

    fn receive_extend(&mut self, ssss: u8) -> Result<i32, Error> {
        if ssss == 0 { return Ok(0); }
        if ssss > 11 { return Err(Error::DcOutOfRange); }
        let v = self.read_bits(ssss)? as i32;
        let vt = 1i32 << (ssss - 1);
        if v < vt {
            Ok(v - ((1i32 << ssss) - 1))
        } else {
            Ok(v)
        }
    }

    fn decode_huff(&mut self, table: &HuffmanTable) -> Result<u8, Error> {
        // Bit-at-a-time canonical Huffman decode (T.81 Figure F.16).
        // Could be sped up by a 9-bit lookahead table; clarity wins
        // here — every entropy-coded fixture is small.
        let mut code: i32 = 0;
        for len in 1..=16 {
            code = (code << 1) | (self.read_bit()? as i32);
            if code <= table.maxcode[len] {
                let j = (table.valptr[len] + (code - table.mincode[len])) as usize;
                if j >= table.huffval.len() {
                    return Err(Error::BadHuffmanCode);
                }
                return Ok(table.huffval[j]);
            }
        }
        Err(Error::BadHuffmanCode)
    }

    fn align_to_byte(&mut self) {
        let drop = self.n_bits & 7;
        self.n_bits -= drop;
    }

    fn take_marker(&mut self) -> Option<u8> {
        // Drain any buffered bits — once we look for a marker we no
        // longer trust the MSB-first stream window.
        self.align_to_byte();
        // If a marker was already latched by `fill`, just return it.
        if let Some(m) = self.marker.take() {
            // Drop any buffered bits — by spec the marker is byte-aligned.
            self.n_bits = 0;
            return Some(m);
        }
        // Otherwise we may need to consume any residual bits then look.
        if self.n_bits != 0 {
            // Discard residual buffered bits; markers are byte-aligned.
            self.n_bits = 0;
        }
        if self.pos + 1 < self.data.len() && self.data[self.pos] == 0xFF {
            let m = self.data[self.pos + 1];
            if m != 0x00 {
                self.pos += 2;
                return Some(m);
            }
        }
        None
    }

    fn position(&self) -> usize {
        // Caller wants where in `data` we are; after a marker latch
        // this is one past the marker's second byte.
        self.pos
    }
}

// ============================================================
// Huffman table (T.81 Annex C, canonical decoder).
// ============================================================

#[derive(Debug, Clone)]
struct HuffmanTable {
    // Indexed 1..=16. Slot 0 is unused (kept for natural indexing).
    mincode: [i32; 17],
    maxcode: [i32; 17],
    valptr: [i32; 17],
    huffval: Vec<u8>,
}

fn build_huffman(bits: &[u8; 16], huffval: Vec<u8>) -> HuffmanTable {
    // Generate HUFFSIZE / HUFFCODE per Annex C.
    let mut huffsize: Vec<u8> = Vec::with_capacity(huffval.len());
    for (i, &b) in bits.iter().enumerate() {
        for _ in 0..b { huffsize.push((i + 1) as u8); }
    }
    let mut huffcode: Vec<u32> = Vec::with_capacity(huffsize.len());
    if !huffsize.is_empty() {
        let mut code: u32 = 0;
        let mut si = huffsize[0];
        for &sz in &huffsize {
            while sz != si {
                code <<= 1;
                si += 1;
            }
            huffcode.push(code);
            code += 1;
        }
    }
    let mut mincode = [0i32; 17];
    let mut maxcode = [-1i32; 17]; // -1 means "no codes of this length"
    let mut valptr = [0i32; 17];
    let mut k = 0usize;
    for i in 1..=16 {
        if bits[i - 1] == 0 {
            maxcode[i] = -1;
        } else {
            valptr[i] = k as i32;
            mincode[i] = huffcode[k] as i32;
            k += bits[i - 1] as usize - 1;
            maxcode[i] = huffcode[k] as i32;
            k += 1;
        }
    }
    HuffmanTable { mincode, maxcode, valptr, huffval }
}

// ============================================================
// Frame / component / scan info.
// ============================================================

#[derive(Clone, Copy, Debug)]
struct ComponentInfo {
    id: u8,
    h_samp: u8,
    v_samp: u8,
    q_table_id: u8,
    // Block grid for this component (allocated to MCU boundary).
    blocks_w: u32,
    blocks_h: u32,
    // Sample plane dimensions in pixels: blocks * 8.
    plane_w: u32,
    plane_h: u32,
}

struct Frame {
    progressive: bool,
    width: u32,
    height: u32,
    max_h: u8,
    max_v: u8,
    components: Vec<ComponentInfo>,
}

struct ScanInfo {
    components: Vec<u8>,   // indices into Frame.components
    ss: u8,
    se: u8,
    ah: u8,
    al: u8,
}

// ============================================================
// Marker walk + segment parsers.
// ============================================================

fn read_u16(b: &[u8], i: usize) -> Result<u16, Error> {
    if i + 2 > b.len() { return Err(Error::UnexpectedEnd); }
    Ok(((b[i] as u16) << 8) | (b[i + 1] as u16))
}

// Read a length-prefixed segment body. Returns slice of segment payload
// (not including the 2-byte length field). Advances p past the segment.
fn read_segment<'a>(input: &'a [u8], p: &mut usize) -> Result<&'a [u8], Error> {
    let len = read_u16(input, *p)? as usize;
    if len < 2 { return Err(Error::BadSegmentLength); }
    if *p + len > input.len() { return Err(Error::UnexpectedEnd); }
    let body = &input[*p + 2..*p + len];
    *p += len;
    Ok(body)
}

fn parse_dqt(seg: &[u8], qt: &mut [Option<[u16; 64]>; 4]) -> Result<(), Error> {
    let mut i = 0;
    while i < seg.len() {
        let pq_tq = seg[i];
        i += 1;
        let pq = pq_tq >> 4;   // precision (0=8-bit, 1=16-bit)
        let tq = (pq_tq & 0x0F) as usize; // table id 0..3
        if tq > 3 { return Err(Error::BadDqt); }
        let mut t = [0u16; 64];
        if pq == 0 {
            if i + 64 > seg.len() { return Err(Error::BadDqt); }
            for k in 0..64 { t[k] = seg[i + k] as u16; }
            i += 64;
        } else if pq == 1 {
            if i + 128 > seg.len() { return Err(Error::BadDqt); }
            for k in 0..64 {
                t[k] = ((seg[i + 2 * k] as u16) << 8) | seg[i + 2 * k + 1] as u16;
            }
            i += 128;
        } else {
            return Err(Error::BadDqt);
        }
        qt[tq] = Some(t);
    }
    Ok(())
}

fn parse_dht(
    seg: &[u8],
    dc: &mut [Option<HuffmanTable>; 4],
    ac: &mut [Option<HuffmanTable>; 4],
) -> Result<(), Error> {
    let mut i = 0;
    while i < seg.len() {
        let tc_th = seg[i];
        i += 1;
        let tc = tc_th >> 4;            // 0=DC, 1=AC
        let th = (tc_th & 0x0F) as usize;
        if th > 3 || tc > 1 { return Err(Error::BadDht); }
        if i + 16 > seg.len() { return Err(Error::BadDht); }
        let mut bits = [0u8; 16];
        bits.copy_from_slice(&seg[i..i + 16]);
        i += 16;
        let total: usize = bits.iter().map(|&b| b as usize).sum();
        if i + total > seg.len() { return Err(Error::BadDht); }
        let huffval = seg[i..i + total].to_vec();
        i += total;
        let table = build_huffman(&bits, huffval);
        if tc == 0 { dc[th] = Some(table); } else { ac[th] = Some(table); }
    }
    Ok(())
}

fn parse_sof(seg: &[u8], progressive: bool) -> Result<Frame, Error> {
    if seg.len() < 6 { return Err(Error::BadSof); }
    let precision = seg[0];
    if precision != 8 { return Err(Error::UnsupportedPrecision(precision)); }
    let height = u16::from_be_bytes([seg[1], seg[2]]) as u32;
    let width = u16::from_be_bytes([seg[3], seg[4]]) as u32;
    let nf = seg[5] as usize;
    if !(nf == 1 || nf == 3) {
        return Err(Error::UnsupportedComponentCount(nf as u8));
    }
    if seg.len() != 6 + 3 * nf { return Err(Error::BadSof); }
    let mut comps: Vec<ComponentInfo> = Vec::with_capacity(nf);
    let mut max_h = 1u8;
    let mut max_v = 1u8;
    for i in 0..nf {
        let id = seg[6 + 3 * i];
        let hv = seg[6 + 3 * i + 1];
        let h = hv >> 4;
        let v = hv & 0x0F;
        let q = seg[6 + 3 * i + 2];
        if !(matches!(h, 1 | 2) && matches!(v, 1 | 2)) {
            return Err(Error::UnsupportedSamplingFactor { h, v });
        }
        if h > max_h { max_h = h; }
        if v > max_v { max_v = v; }
        comps.push(ComponentInfo {
            id, h_samp: h, v_samp: v, q_table_id: q,
            blocks_w: 0, blocks_h: 0, plane_w: 0, plane_h: 0,
        });
    }
    // Derive component block grid (MCU-aligned).
    let mcu_w_pixels = 8u32 * max_h as u32;
    let mcu_h_pixels = 8u32 * max_v as u32;
    let mcus_per_row = width.div_ceil(mcu_w_pixels);
    let mcus_per_col = height.div_ceil(mcu_h_pixels);
    for c in &mut comps {
        c.blocks_w = mcus_per_row * c.h_samp as u32;
        c.blocks_h = mcus_per_col * c.v_samp as u32;
        c.plane_w = c.blocks_w * 8;
        c.plane_h = c.blocks_h * 8;
    }
    Ok(Frame { progressive, width, height, max_h, max_v, components: comps })
}

fn parse_sos(seg: &[u8], frame: &Frame) -> Result<(ScanInfo, Vec<(u8, u8)>), Error> {
    if seg.is_empty() { return Err(Error::BadSos); }
    let ns = seg[0] as usize;
    if seg.len() != 1 + 2 * ns + 3 { return Err(Error::BadSos); }
    let mut comps_idx: Vec<u8> = Vec::with_capacity(ns);
    let mut bindings: Vec<(u8, u8)> = Vec::with_capacity(ns); // (dc_id, ac_id)
    for i in 0..ns {
        let cs = seg[1 + 2 * i];
        let td_ta = seg[1 + 2 * i + 1];
        let td = td_ta >> 4;
        let ta = td_ta & 0x0F;
        let idx = frame.components.iter().position(|c| c.id == cs)
            .ok_or(Error::BadSos)? as u8;
        comps_idx.push(idx);
        bindings.push((td, ta));
    }
    let off = 1 + 2 * ns;
    let ss = seg[off];
    let se = seg[off + 1];
    let ah_al = seg[off + 2];
    let ah = ah_al >> 4;
    let al = ah_al & 0x0F;
    Ok((ScanInfo { components: comps_idx, ss, se, ah, al }, bindings))
}

// ============================================================
// Block decode — baseline (Annex F.2).
// ============================================================

// Decode one 8x8 block of zig-zag coefficients into `coef`
// (row-major 8x8 in `coef`, indexed via ZIGZAG[k]).
// `dc_pred` is updated with the new DC predictor.
fn decode_block_baseline(
    reader: &mut BitReader<'_>,
    dc_table: &HuffmanTable,
    ac_table: &HuffmanTable,
    dc_pred: &mut i32,
    coef: &mut [i32; 64],
) -> Result<(), Error> {
    *coef = [0; 64];
    // DC.
    let s = reader.decode_huff(dc_table)?;
    let diff = reader.receive_extend(s)?;
    *dc_pred = dc_pred.wrapping_add(diff);
    coef[0] = *dc_pred;
    // AC (63 coefficients, zig-zag indices 1..=63).
    let mut k = 1;
    while k <= 63 {
        let rs = reader.decode_huff(ac_table)?;
        let r = (rs >> 4) as usize;
        let s = rs & 0x0F;
        if s == 0 {
            if r == 15 {
                // ZRL: skip 16 zeros.
                k += 16;
                continue;
            } else {
                // EOB: rest of block is zero.
                break;
            }
        }
        k += r;
        if k > 63 { return Err(Error::BadProgressiveAc); }
        let v = reader.receive_extend(s)?;
        coef[ZIGZAG[k]] = v;
        k += 1;
    }
    Ok(())
}

// ============================================================
// Block decode — progressive (T.81 §G.1.2).
//
// Coefficient state is stored externally (per-block buffers); each
// scan kind below mutates the buffer in place. EOBRUN persists
// across blocks within a scan; reset at restart and at scan start.
// ============================================================

fn decode_block_progressive_dc_first(
    reader: &mut BitReader<'_>,
    dc_table: &HuffmanTable,
    al: u8,
    dc_pred: &mut i32,
    coef: &mut [i32; 64],
) -> Result<(), Error> {
    let s = reader.decode_huff(dc_table)?;
    let diff = reader.receive_extend(s)?;
    *dc_pred = dc_pred.wrapping_add(diff);
    coef[0] = *dc_pred << al;
    Ok(())
}

fn decode_block_progressive_dc_refine(
    reader: &mut BitReader<'_>,
    al: u8,
    coef: &mut [i32; 64],
) -> Result<(), Error> {
    let bit = reader.read_bit()?;
    if bit == 1 { coef[0] |= 1i32 << al; }
    Ok(())
}

fn decode_block_progressive_ac_first(
    reader: &mut BitReader<'_>,
    ac_table: &HuffmanTable,
    ss: u8, se: u8, al: u8,
    eob_run: &mut u32,
    coef: &mut [i32; 64],
) -> Result<(), Error> {
    if *eob_run > 0 {
        *eob_run -= 1;
        return Ok(());
    }
    let mut k = ss as usize;
    let se = se as usize;
    while k <= se {
        let rs = reader.decode_huff(ac_table)?;
        let r = (rs >> 4) as usize;
        let s = rs & 0x0F;
        if s == 0 {
            if r != 15 {
                // EOBn: this block ends here, plus (1<<r)-1 more blocks
                // are zeroed (handled via *eob_run on subsequent calls).
                *eob_run = (1u32 << r) - 1;
                if r > 0 {
                    *eob_run += reader.read_bits(r as u8)?;
                }
                return Ok(());
            }
            // ZRL: skip 16 zeros.
            k += 16;
            if k > se + 1 { return Err(Error::BadProgressiveAc); }
        } else {
            k += r;
            if k > se { return Err(Error::BadProgressiveAc); }
            let v = reader.receive_extend(s)?;
            coef[ZIGZAG[k]] = v << al;
            k += 1;
        }
    }
    Ok(())
}

fn decode_block_progressive_ac_refine(
    reader: &mut BitReader<'_>,
    ac_table: &HuffmanTable,
    ss: u8, se: u8, al: u8,
    eob_run: &mut u32,
    coef: &mut [i32; 64],
) -> Result<(), Error> {
    let p1: i32 = 1i32 << al;
    let m1: i32 = -(1i32 << al);
    let se = se as usize;
    let mut k = ss as usize;
    if *eob_run == 0 {
        while k <= se {
            let rs = reader.decode_huff(ac_table)?;
            let mut r = (rs >> 4) as i32;
            let mut s = (rs & 0x0F) as i32;
            if s != 0 {
                if s != 1 { return Err(Error::BadProgressiveAc); }
                let bit = reader.read_bit()?;
                s = if bit == 1 { p1 } else { m1 };
            } else if r != 15 {
                // EOBn — set up run, then drop into refinement-only loop.
                *eob_run = 1u32 << r;
                if r > 0 {
                    *eob_run += reader.read_bits(r as u8)?;
                }
                break;
            }
            // Walk forward in zig-zag: refine existing non-zeros, count
            // down zero-run; place new value (s != 0) at the next zero
            // when run exhausts.
            loop {
                let pos = ZIGZAG[k];
                let coef_v = coef[pos];
                if coef_v != 0 {
                    let bit = reader.read_bit()?;
                    if bit == 1 {
                        // Only set the new low-order bit if not already present.
                        if (coef_v & p1) == 0 {
                            if coef_v >= 0 {
                                coef[pos] = coef_v + p1;
                            } else {
                                coef[pos] = coef_v + m1;
                            }
                        }
                    }
                } else {
                    r -= 1;
                    if r < 0 { break; } // landed at target zero
                }
                k += 1;
                if k > se { break; }
            }
            if s != 0 && k <= se {
                coef[ZIGZAG[k]] = s;
            }
            k += 1;
        }
    }
    if *eob_run > 0 {
        // Refine remaining non-zeros in band; consume one EOB count.
        while k <= se {
            let pos = ZIGZAG[k];
            let coef_v = coef[pos];
            if coef_v != 0 {
                let bit = reader.read_bit()?;
                if bit == 1 && (coef_v & p1) == 0 {
                    if coef_v >= 0 {
                        coef[pos] = coef_v + p1;
                    } else {
                        coef[pos] = coef_v + m1;
                    }
                }
            }
            k += 1;
        }
        *eob_run -= 1;
    }
    Ok(())
}

// ============================================================
// Scan driver (covers baseline + all 4 progressive scan kinds).
// ============================================================

#[derive(Clone, Copy)]
enum ScanKind {
    Baseline,
    ProgDcFirst,
    ProgDcRefine,
    ProgAcFirst,
    ProgAcRefine,
}

fn classify_scan(frame: &Frame, scan: &ScanInfo) -> ScanKind {
    if !frame.progressive { return ScanKind::Baseline; }
    if scan.ss == 0 {
        if scan.ah == 0 { ScanKind::ProgDcFirst } else { ScanKind::ProgDcRefine }
    } else {
        if scan.ah == 0 { ScanKind::ProgAcFirst } else { ScanKind::ProgAcRefine }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_scan(
    entropy: &[u8],
    frame: &Frame,
    scan: &ScanInfo,
    dc_tables: &[Option<HuffmanTable>; 4],
    ac_tables: &[Option<HuffmanTable>; 4],
    bindings: &[(u8, u8)],
    coef_blocks: &mut [Vec<[i32; 64]>], // one Vec per frame component
    restart_interval: u32,
) -> Result<usize, Error> {
    let kind = classify_scan(frame, scan);
    let mut reader = BitReader::new(entropy);

    // Per-component DC predictors (reset at scan start and at restart).
    let mut dc_pred = [0i32; 4]; // up to 4 components
    let mut eob_run: u32 = 0;
    let mut mcu_count: u32 = 0;
    let mut next_rst: u8 = 0;

    let interleaved = scan.components.len() > 1;

    if interleaved {
        // Interleaved scan — iterate MCU grid, consume H_i*V_i blocks per component per MCU.
        let mcus_per_row =
            frame.width.div_ceil(8 * frame.max_h as u32);
        let mcus_per_col =
            frame.height.div_ceil(8 * frame.max_v as u32);
        for my in 0..mcus_per_col {
            for mx in 0..mcus_per_row {
                for (i, &fc) in scan.components.iter().enumerate() {
                    let comp = frame.components[fc as usize];
                    let (td, ta) = bindings[i];
                    let dc_t = dc_tables[td as usize].as_ref()
                        .ok_or(Error::UndefinedHuffmanTable { class: 0, id: td })?;
                    let ac_t_opt = ac_tables[ta as usize].as_ref();
                    let blocks_w = comp.blocks_w;
                    for v in 0..comp.v_samp as u32 {
                        for h in 0..comp.h_samp as u32 {
                            let bx = mx * comp.h_samp as u32 + h;
                            let by = my * comp.v_samp as u32 + v;
                            let idx = (by * blocks_w + bx) as usize;
                            let coef = &mut coef_blocks[fc as usize][idx];
                            match kind {
                                ScanKind::Baseline => {
                                    let ac_t = ac_t_opt.ok_or(Error::UndefinedHuffmanTable { class: 1, id: ta })?;
                                    decode_block_baseline(
                                        &mut reader, dc_t, ac_t,
                                        &mut dc_pred[fc as usize], coef,
                                    )?;
                                }
                                ScanKind::ProgDcFirst => {
                                    decode_block_progressive_dc_first(
                                        &mut reader, dc_t, scan.al,
                                        &mut dc_pred[fc as usize], coef,
                                    )?;
                                }
                                ScanKind::ProgDcRefine => {
                                    decode_block_progressive_dc_refine(
                                        &mut reader, scan.al, coef,
                                    )?;
                                }
                                _ => return Err(Error::BadSos), // AC scans are non-interleaved
                            }
                        }
                    }
                }
                mcu_count += 1;
                if restart_interval > 0 && mcu_count % restart_interval == 0
                    && mcu_count < mcus_per_row * mcus_per_col {
                    handle_restart(&mut reader, &mut dc_pred, &mut eob_run, &mut next_rst,
                        mcu_count, mcus_per_row * mcus_per_col)?;
                }
            }
        }
    } else {
        // Non-interleaved — single component, raster of all its blocks.
        let fc = scan.components[0];
        let comp = frame.components[fc as usize];
        let (td, ta) = bindings[0];
        let dc_t_opt = dc_tables[td as usize].as_ref();
        let ac_t_opt = ac_tables[ta as usize].as_ref();
        // For non-interleaved, the spec's block count uses the
        // component's own dimensions (T.81 A.2.4). Our `blocks_w/h`
        // are already computed for the MCU grid; for non-interleaved
        // scans we recompute against the image proper.
        let nb_w = (frame.width * comp.h_samp as u32).div_ceil(8 * frame.max_h as u32);
        let nb_h = (frame.height * comp.v_samp as u32).div_ceil(8 * frame.max_v as u32);
        let total_blocks = nb_w * nb_h;
        for by in 0..nb_h {
            for bx in 0..nb_w {
                let idx = (by * comp.blocks_w + bx) as usize;
                if idx >= coef_blocks[fc as usize].len() {
                    return Err(Error::BadSof);
                }
                let coef = &mut coef_blocks[fc as usize][idx];
                match kind {
                    ScanKind::Baseline => {
                        let dc_t = dc_t_opt.ok_or(Error::UndefinedHuffmanTable { class: 0, id: td })?;
                        let ac_t = ac_t_opt.ok_or(Error::UndefinedHuffmanTable { class: 1, id: ta })?;
                        decode_block_baseline(
                            &mut reader, dc_t, ac_t,
                            &mut dc_pred[fc as usize], coef,
                        )?;
                    }
                    ScanKind::ProgDcFirst => {
                        let dc_t = dc_t_opt.ok_or(Error::UndefinedHuffmanTable { class: 0, id: td })?;
                        decode_block_progressive_dc_first(
                            &mut reader, dc_t, scan.al,
                            &mut dc_pred[fc as usize], coef,
                        )?;
                    }
                    ScanKind::ProgDcRefine => {
                        decode_block_progressive_dc_refine(&mut reader, scan.al, coef)?;
                    }
                    ScanKind::ProgAcFirst => {
                        let ac_t = ac_t_opt.ok_or(Error::UndefinedHuffmanTable { class: 1, id: ta })?;
                        decode_block_progressive_ac_first(
                            &mut reader, ac_t, scan.ss, scan.se, scan.al,
                            &mut eob_run, coef,
                        )?;
                    }
                    ScanKind::ProgAcRefine => {
                        let ac_t = ac_t_opt.ok_or(Error::UndefinedHuffmanTable { class: 1, id: ta })?;
                        decode_block_progressive_ac_refine(
                            &mut reader, ac_t, scan.ss, scan.se, scan.al,
                            &mut eob_run, coef,
                        )?;
                    }
                }
                mcu_count += 1;
                if restart_interval > 0 && mcu_count % restart_interval == 0
                    && mcu_count < total_blocks {
                    handle_restart(&mut reader, &mut dc_pred, &mut eob_run, &mut next_rst, mcu_count, total_blocks)?;
                }
            }
        }
    }

    // After the scan body the next byte should be a marker. The bit
    // reader may have already latched it during fill; align + return
    // its byte position. We DON'T consume the marker — the outer loop
    // will pick it up via read_marker on the rewound stream.
    reader.align_to_byte();
    let mut consumed = reader.position();
    if let Some(m) = reader.marker {
        // Marker bytes (0xFF + m) were consumed by the reader; rewind
        // to BEFORE them so the outer marker walker sees them.
        consumed = consumed.saturating_sub(2);
        let _ = m;
    } else {
        // No marker latched. If buffered bits remain or pos is at
        // end of stream, we leave consumed at current position.
    }
    Ok(consumed)
}

fn handle_restart(
    reader: &mut BitReader<'_>,
    dc_pred: &mut [i32; 4],
    eob_run: &mut u32,
    next_rst: &mut u8,
    _mcu_count: u32,
    _total_mcus: u32,
) -> Result<(), Error> {
    // Reset DC predictors + EOBRUN per T.81 §F.2.1.3.1.
    for v in dc_pred.iter_mut() { *v = 0; }
    *eob_run = 0;
    let m = reader.take_marker().ok_or(Error::BadRestart)?;
    let expected = M_RST0 + (*next_rst & 7);
    if m != expected { return Err(Error::BadRestart); }
    *next_rst = (*next_rst + 1) & 7;
    Ok(())
}

// ============================================================
// IDCT — direct definition, separable, f64.
//
// 50 dB PSNR floor swallows any drift between this reference IDCT
// and djpeg's integer IDCT.
// ============================================================

fn idct_8x8(coef: &[i32; 64], out: &mut [u8; 64]) {
    use std::f64::consts::PI;
    // Precomputed cosine table is overkill at our fixture sizes —
    // cosines computed inline keep the function self-contained.
    let c = |k: usize| if k == 0 { 1.0 / 2.0_f64.sqrt() } else { 1.0 };
    let mut temp = [0.0f64; 64];
    // Row pass: temp[v*8 + x] = (1/2) * sum_u c(u) * coef[v*8 + u] * cos(...)
    for v in 0..8 {
        for x in 0..8 {
            let mut sum = 0.0f64;
            for u in 0..8 {
                sum += c(u) * (coef[v * 8 + u] as f64)
                    * (((2 * x + 1) as f64) * (u as f64) * PI / 16.0).cos();
            }
            temp[v * 8 + x] = 0.5 * sum;
        }
    }
    // Column pass + level shift + clamp.
    for x in 0..8 {
        for y in 0..8 {
            let mut sum = 0.0f64;
            for v in 0..8 {
                sum += c(v) * temp[v * 8 + x]
                    * (((2 * y + 1) as f64) * (v as f64) * PI / 16.0).cos();
            }
            let val = 0.5 * sum + 128.0;
            let v = val.round();
            out[y * 8 + x] = if v < 0.0 { 0 } else if v > 255.0 { 255 } else { v as u8 };
        }
    }
}

// ============================================================
// Per-component plane assembly: dequantize, IDCT, level-shift.
// ============================================================

fn assemble_planes(
    frame: &Frame,
    coef_blocks: &[Vec<[i32; 64]>],
    quant: &[Option<[u16; 64]>; 4],
) -> Result<Vec<Vec<u8>>, Error> {
    let mut planes: Vec<Vec<u8>> = Vec::with_capacity(frame.components.len());
    for (ci, comp) in frame.components.iter().enumerate() {
        let qt = quant[comp.q_table_id as usize].as_ref()
            .ok_or(Error::UndefinedQuantTable(comp.q_table_id))?;
        let plane_w = comp.plane_w as usize;
        let plane_h = comp.plane_h as usize;
        let mut plane = vec![0u8; plane_w * plane_h];
        let blocks_w = comp.blocks_w as usize;
        let blocks_h = comp.blocks_h as usize;
        for by in 0..blocks_h {
            for bx in 0..blocks_w {
                let coef = &coef_blocks[ci][by * blocks_w + bx];
                // Dequantize. Quant tables are stored in zig-zag
                // order (qt[k]), our coefficients in row-major order
                // (coef[ZIGZAG[k]]) — apply qt[k] at position ZIGZAG[k].
                let mut deq = [0i32; 64];
                for k in 0..64 {
                    let pos = ZIGZAG[k];
                    deq[pos] = coef[pos] * qt[k] as i32;
                }
                let mut block = [0u8; 64];
                idct_8x8(&deq, &mut block);
                // Splat into plane at (bx*8, by*8).
                for y in 0..8 {
                    let dst_off = (by * 8 + y) * plane_w + bx * 8;
                    let src_off = y * 8;
                    plane[dst_off..dst_off + 8].copy_from_slice(&block[src_off..src_off + 8]);
                }
            }
        }
        planes.push(plane);
    }
    Ok(planes)
}

// ============================================================
// Chroma upsampling — h2v2 fancy (libjpeg-equivalent), h2v1, h1v2.
// Output planes match the luma plane's dimensions.
// ============================================================

fn upsample_to_luma(
    frame: &Frame,
    planes: &[Vec<u8>],
) -> Vec<Vec<u8>> {
    let max_h = frame.max_h as u32;
    let max_v = frame.max_v as u32;
    let target_w = frame.width as usize; // we'll crop at compose
    let _ = target_w;
    let mut out: Vec<Vec<u8>> = Vec::with_capacity(planes.len());
    for (ci, comp) in frame.components.iter().enumerate() {
        let h_scale = max_h / comp.h_samp as u32;
        let v_scale = max_v / comp.v_samp as u32;
        if h_scale == 1 && v_scale == 1 {
            out.push(planes[ci].clone());
        } else if h_scale == 2 && v_scale == 2 {
            out.push(h2v2_fancy(&planes[ci], comp.plane_w as usize, comp.plane_h as usize));
        } else if h_scale == 2 && v_scale == 1 {
            out.push(h2v1_fancy(&planes[ci], comp.plane_w as usize, comp.plane_h as usize));
        } else if h_scale == 1 && v_scale == 2 {
            out.push(h1v2_fancy(&planes[ci], comp.plane_w as usize, comp.plane_h as usize));
        } else {
            // Generic box upsampling for any other case (4x not in scope,
            // but kept structurally honest).
            out.push(box_upsample(
                &planes[ci],
                comp.plane_w as usize, comp.plane_h as usize,
                h_scale as usize, v_scale as usize,
            ));
        }
    }
    out
}

// Edge-clamping helper.
fn at(plane: &[u8], w: usize, h: usize, x: i32, y: i32) -> i32 {
    let cx = x.clamp(0, w as i32 - 1) as usize;
    let cy = y.clamp(0, h as i32 - 1) as usize;
    plane[cy * w + cx] as i32
}

// libjpeg-equivalent h2v2 fancy upsampling. Each chroma sample at
// (cx, cy) is bilinear-interpolated against three neighbours into a
// 2x2 block of output samples at (2cx..2cx+2, 2cy..2cy+2). Edge
// chroma samples replicate.
//
// The 4-tap weights (9, 3, 3, 1) come from JPEG centered-grid
// bilinear interpolation: at each output sub-pixel the 2x2 chroma
// neighbourhood collapses to 9c00 + 3c10 + 3c01 + c11 / 16, with
// the neighbour direction depending on the output sub-pixel quadrant.
fn h2v2_fancy(plane: &[u8], w: usize, h: usize) -> Vec<u8> {
    let ow = w * 2;
    let oh = h * 2;
    let mut out = vec![0u8; ow * oh];
    for cy in 0..h as i32 {
        for cx in 0..w as i32 {
            let c = at(plane, w, h, cx, cy);
            let cl = at(plane, w, h, cx - 1, cy);
            let cr = at(plane, w, h, cx + 1, cy);
            let cu = at(plane, w, h, cx, cy - 1);
            let cd = at(plane, w, h, cx, cy + 1);
            let cul = at(plane, w, h, cx - 1, cy - 1);
            let cur = at(plane, w, h, cx + 1, cy - 1);
            let cdl = at(plane, w, h, cx - 1, cy + 1);
            let cdr = at(plane, w, h, cx + 1, cy + 1);
            // Quadrants: TL, TR, BL, BR
            let tl = (9 * c + 3 * cl + 3 * cu + cul + 8) / 16;
            let tr = (9 * c + 3 * cr + 3 * cu + cur + 8) / 16;
            let bl = (9 * c + 3 * cl + 3 * cd + cdl + 8) / 16;
            let br = (9 * c + 3 * cr + 3 * cd + cdr + 8) / 16;
            let ox = (cx as usize) * 2;
            let oy = (cy as usize) * 2;
            out[oy * ow + ox]         = clamp_byte(tl);
            out[oy * ow + ox + 1]     = clamp_byte(tr);
            out[(oy + 1) * ow + ox]   = clamp_byte(bl);
            out[(oy + 1) * ow + ox + 1] = clamp_byte(br);
        }
    }
    out
}

fn h2v1_fancy(plane: &[u8], w: usize, h: usize) -> Vec<u8> {
    let ow = w * 2;
    let oh = h;
    let mut out = vec![0u8; ow * oh];
    for cy in 0..h as i32 {
        for cx in 0..w as i32 {
            let c = at(plane, w, h, cx, cy);
            let cl = at(plane, w, h, cx - 1, cy);
            let cr = at(plane, w, h, cx + 1, cy);
            let l = (3 * c + cl + 2) / 4;
            let r = (3 * c + cr + 2) / 4;
            let ox = (cx as usize) * 2;
            let oy = cy as usize;
            out[oy * ow + ox] = clamp_byte(l);
            out[oy * ow + ox + 1] = clamp_byte(r);
        }
    }
    out
}

fn h1v2_fancy(plane: &[u8], w: usize, h: usize) -> Vec<u8> {
    let ow = w;
    let oh = h * 2;
    let mut out = vec![0u8; ow * oh];
    for cy in 0..h as i32 {
        for cx in 0..w as i32 {
            let c = at(plane, w, h, cx, cy);
            let cu = at(plane, w, h, cx, cy - 1);
            let cd = at(plane, w, h, cx, cy + 1);
            let u = (3 * c + cu + 2) / 4;
            let d = (3 * c + cd + 2) / 4;
            let ox = cx as usize;
            let oy = (cy as usize) * 2;
            out[oy * ow + ox] = clamp_byte(u);
            out[(oy + 1) * ow + ox] = clamp_byte(d);
        }
    }
    out
}

fn box_upsample(plane: &[u8], w: usize, h: usize, hs: usize, vs: usize) -> Vec<u8> {
    let ow = w * hs;
    let oh = h * vs;
    let mut out = vec![0u8; ow * oh];
    for cy in 0..h {
        for cx in 0..w {
            let v = plane[cy * w + cx];
            for dy in 0..vs {
                for dx in 0..hs {
                    out[(cy * vs + dy) * ow + (cx * hs + dx)] = v;
                }
            }
        }
    }
    out
}

fn clamp_byte(v: i32) -> u8 {
    if v < 0 { 0 } else if v > 255 { 255 } else { v as u8 }
}

// ============================================================
// Color conversion + RGBA assembly (BT.601 per JFIF Appendix A).
// ============================================================

fn assemble_rgba(frame: &Frame, planes: &[Vec<u8>]) -> Vec<u8> {
    let w = frame.width as usize;
    let h = frame.height as usize;
    let mut pixels = vec![0u8; w * h * 4];
    let plane_w = frame.components[0].plane_w as usize;
    if frame.components.len() == 1 {
        let y_plane = &planes[0];
        for py in 0..h {
            for px in 0..w {
                let v = y_plane[py * plane_w + px];
                let off = (py * w + px) * 4;
                pixels[off] = v;
                pixels[off + 1] = v;
                pixels[off + 2] = v;
                pixels[off + 3] = 0xFF;
            }
        }
    } else {
        // Assume 3 components in (Y, Cb, Cr) order — JFIF default.
        let y_plane = &planes[0];
        let cb_plane = &planes[1];
        let cr_plane = &planes[2];
        // After upsampling all three planes share the luma plane width.
        for py in 0..h {
            for px in 0..w {
                let y = y_plane[py * plane_w + px] as i32;
                let cb = cb_plane[py * plane_w + px] as i32 - 128;
                let cr = cr_plane[py * plane_w + px] as i32 - 128;
                // BT.601, scaled by 1024 for integer math:
                //   R = Y + 1.402   * Cr      ≈ Y + (1436 * Cr) >> 10
                //   G = Y - 0.34414 * Cb - 0.71414 * Cr
                //                            ≈ Y - (352 * Cb) >> 10 - (731 * Cr) >> 10
                //   B = Y + 1.772   * Cb      ≈ Y + (1815 * Cb) >> 10
                let r = y + ((1436 * cr + 512) >> 10);
                let g = y - ((352 * cb + 512) >> 10) - ((731 * cr + 512) >> 10);
                let b = y + ((1815 * cb + 512) >> 10);
                let off = (py * w + px) * 4;
                pixels[off]     = clamp_byte(r);
                pixels[off + 1] = clamp_byte(g);
                pixels[off + 2] = clamp_byte(b);
                pixels[off + 3] = 0xFF;
            }
        }
    }
    pixels
}

// ============================================================
// Public API
// ============================================================

/// One-shot decode of a complete JPEG byte stream.
///
/// Returns RGBA8 pixels (`width * height * 4` bytes), regardless
/// of the source's component count. Grayscale source widens to
/// (Y, Y, Y, 0xFF); 3-component (YCbCr) source converts via
/// BT.601 to (R, G, B, 0xFF).
pub fn decode(input: &[u8]) -> Result<DecodedImage, Error> {
    if input.is_empty() { return Err(Error::EmptyInput); }
    if input.len() < 2 || input[0] != 0xFF || input[1] != M_SOI {
        return Err(Error::NotJpeg);
    }
    let mut p = 2;

    let mut quant_tables: [Option<[u16; 64]>; 4] = [None, None, None, None];
    let mut dc_tables: [Option<HuffmanTable>; 4] = Default::default();
    let mut ac_tables: [Option<HuffmanTable>; 4] = Default::default();
    let mut frame: Option<Frame> = None;
    let mut restart_interval: u32 = 0;
    let mut coef_blocks: Vec<Vec<[i32; 64]>> = Vec::new();
    let mut sos_seen = false;

    loop {
        // Find next 0xFF byte.
        while p < input.len() && input[p] != 0xFF { p += 1; }
        if p >= input.len() { return Err(Error::UnexpectedEnd); }
        // Skip 0xFF fill bytes; the marker byte is the last non-FF after.
        while p < input.len() && input[p] == 0xFF { p += 1; }
        if p >= input.len() { return Err(Error::UnexpectedEnd); }
        let m = input[p];
        p += 1;
        if m == 0x00 { continue; } // not a real marker (stray 0xFF00 outside scan — shouldn't happen)

        match m {
            M_SOI => return Err(Error::NotJpeg), // unexpected second SOI
            M_EOI => break,
            M_SOF0 | M_SOF2 => {
                let progressive = m == M_SOF2;
                let seg = read_segment(input, &mut p)?;
                let f = parse_sof(seg, progressive)?;
                // Allocate per-component coefficient buffers.
                coef_blocks = f.components.iter().map(|c| {
                    vec![[0i32; 64]; (c.blocks_w * c.blocks_h) as usize]
                }).collect();
                frame = Some(f);
            }
            M_DQT => {
                let seg = read_segment(input, &mut p)?;
                parse_dqt(seg, &mut quant_tables)?;
            }
            M_DHT => {
                let seg = read_segment(input, &mut p)?;
                parse_dht(seg, &mut dc_tables, &mut ac_tables)?;
            }
            M_DRI => {
                let seg = read_segment(input, &mut p)?;
                if seg.len() != 2 { return Err(Error::BadDri); }
                restart_interval = u16::from_be_bytes([seg[0], seg[1]]) as u32;
            }
            M_SOS => {
                let seg = read_segment(input, &mut p)?;
                let f = frame.as_ref().ok_or(Error::SosBeforeSof)?;
                let (scan, bindings) = parse_sos(seg, f)?;
                let entropy = &input[p..];
                let consumed = run_scan(
                    entropy, f, &scan,
                    &dc_tables, &ac_tables,
                    &bindings, &mut coef_blocks,
                    restart_interval,
                )?;
                p += consumed;
                sos_seen = true;
            }
            M_COM => {
                let _ = read_segment(input, &mut p)?;
            }
            // APPn (E0..EF), DAC, DNL, DHP, EXP, JPGn (F0..FD): all
            // are length-prefixed segments we don't need to interpret.
            // RSTn outside a scan body is a stream-level error; treat
            // as unsupported here.
            0xE0..=0xFD => {
                let _ = read_segment(input, &mut p)?;
            }
            // Unsupported SOF: extended sequential, lossless,
            // hierarchical, arithmetic-coding variants.
            0xC1 | 0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF => {
                return Err(Error::UnsupportedProcess(m));
            }
            other => return Err(Error::UnsupportedMarker(other)),
        }
    }

    if !sos_seen { return Err(Error::NoSos); }
    let frame = frame.ok_or(Error::NoFrame)?;

    // Dequantize + IDCT every block of every component.
    let planes = assemble_planes(&frame, &coef_blocks, &quant_tables)?;
    // Upsample chroma to luma resolution.
    let upsampled = upsample_to_luma(&frame, &planes);
    // Color convert + RGBA pack (cropped to frame.width x frame.height).
    let pixels = assemble_rgba(&frame, &upsampled);

    Ok(DecodedImage {
        width: frame.width,
        height: frame.height,
        components: 4,
        pixels,
    })
}
