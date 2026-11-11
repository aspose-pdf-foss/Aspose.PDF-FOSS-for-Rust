// Robustness gate over deliberately corrupted PDFs.
//
// The library parses untrusted input, so the property under test is that
// it always *answers* — `Ok` or `Err` — and never aborts the process. It
// is compiled with `#![forbid(unsafe_code)]`, so a memory-safety defect
// cannot become undefined behaviour; it surfaces as a panic, an
// arithmetic overflow or an unbounded allocation. All three fail here.
//
// **Why not `cargo-fuzz`.** A libFuzzer harness needs `libfuzzer-sys` in
// `[dev-dependencies]`, and the Rust track's strategy lock keeps both
// dependency tables empty. This harness trades libFuzzer's coverage
// guidance for determinism instead: a fixed seed, a fixed corpus and a
// fixed operator set, so a failure is reproducible from its descriptor
// alone and the gate runs inside the ordinary suite on every commit. It
// finds shallow defects, not deep ones — if the operator ever relaxes
// the lock, a coverage-guided fuzzer belongs here alongside it.

use aspose_pdf_foss::{Document, TextAbsorber};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

/// xorshift64*, so the mutation stream is reproducible without a crate.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// FNV-1a, so a fixture's name contributes to its seed and two
    /// same-sized fixtures do not share a mutation stream.
    fn hash(text: &str) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in text.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        h
    }

    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

fn seed_corpus() -> Vec<(String, Vec<u8>)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        paths.sort();
        for p in paths {
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|e| e == "pdf") {
                out.push(p);
            }
        }
    }
    let mut paths = Vec::new();
    walk(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
        &mut paths,
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    paths
        .into_iter()
        .filter_map(|p| {
            // Path relative to the corpus root, not the bare file name:
            // several fixtures share a basename across directories, and
            // a colliding name would collapse their mutation streams
            // onto one seed and make a failure report ambiguous.
            let name = p.strip_prefix(&root).ok()?.to_string_lossy().into_owned();
            Some((name, std::fs::read(&p).ok()?))
        })
        .filter(|(_, b)| !b.is_empty())
        .collect()
}

/// The corruption operators. Each is cheap, and between them they reach
/// the failure modes a PDF parser actually meets in the wild: a file cut
/// short mid-object, a byte rewritten inside a length or offset, a run
/// of nulls where structure was expected, and two documents spliced
/// together at an arbitrary point.
#[derive(Clone, Copy, Debug)]
enum Op {
    Truncate,
    FlipByte,
    ZeroRun,
    Splice,
    DamageHeader,
}

const OPS: [Op; 5] = [
    Op::Truncate,
    Op::FlipByte,
    Op::ZeroRun,
    Op::Splice,
    Op::DamageHeader,
];

fn mutate(op: Op, src: &[u8], other: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut bytes = src.to_vec();
    match op {
        Op::Truncate => {
            let cut = rng.below(bytes.len());
            bytes.truncate(cut);
        }
        Op::FlipByte => {
            if !bytes.is_empty() {
                let at = rng.below(bytes.len());
                bytes[at] ^= 1 << (rng.below(8) as u8);
            }
        }
        Op::ZeroRun => {
            if !bytes.is_empty() {
                let at = rng.below(bytes.len());
                let len = rng.below(64).min(bytes.len() - at);
                bytes[at..at + len].fill(0);
            }
        }
        Op::Splice => {
            let cut = rng.below(bytes.len());
            bytes.truncate(cut);
            let take = rng.below(other.len());
            bytes.extend_from_slice(&other[..take]);
        }
        Op::DamageHeader => {
            for b in bytes.iter_mut().take(8) {
                *b = b'?';
            }
        }
    }
    bytes
}

/// Open the bytes and walk the paths an untrusted document reaches:
/// the page tree, per-page geometry, and text extraction. Reports
/// whether the input parsed, so the sweep can prove it is reaching the
/// parser rather than bouncing off the header check.
fn exercise(bytes: Vec<u8>) -> bool {
    let Ok(doc) = Document::from_bytes(bytes) else { return false };
    let pages = doc.pages();
    let count = pages.count();
    for i in 1..=count.min(8) {
        if let Ok(page) = pages.get(i) {
            let _ = page.width();
            let _ = page.height();
            let _ = page.rotation();
        }
    }
    let mut absorber = TextAbsorber::new();
    let _ = absorber.visit_document(&doc);
    let _ = absorber.text().len();
    true
}

#[test]
fn corrupted_documents_are_answered_not_aborted() {
    let corpus = seed_corpus();
    assert!(!corpus.is_empty(), "no seed fixtures found");

    // Quiet the default hook: a caught panic is a reported failure here,
    // not something to spill across the test log.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let mut failures: Vec<String> = Vec::new();
    let mut cases = 0usize;
    let mut parsed = 0usize;

    for (name, bytes) in &corpus {
        for (op_index, op) in OPS.iter().enumerate() {
            for round in 0..12u64 {
                // The seed IS the hash of the descriptor, so a reported
                // failure replays by construction and no two cases can
                // silently share a mutation stream.
                let descriptor = format!("{name}/{op_index}/{round}");
                let seed = Rng::hash(&descriptor);
                let mut rng = Rng::new(seed);
                let other = &corpus[rng.below(corpus.len())].1;
                let mutant = mutate(*op, bytes, other, &mut rng);
                cases += 1;
                match catch_unwind(AssertUnwindSafe(|| exercise(mutant))) {
                    Ok(true) => parsed += 1,
                    Ok(false) => {}
                    Err(_) => failures.push(format!("{descriptor}  ({op:?}, seed {seed})")),
                }
            }
        }
    }

    std::panic::set_hook(previous);

    assert!(
        failures.is_empty(),
        "{} of {cases} corrupted inputs aborted instead of returning:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
    assert!(cases >= 1000, "expected a meaningful sweep, ran {cases}");
    // A gate whose every input is rejected at byte zero proves nothing.
    // Roughly a third of the sweep parses and reaches the deep paths;
    // the floor here is well under that, so it flags a real collapse
    // without tripping on ordinary drift.
    assert!(
        parsed * 10 >= cases,
        "only {parsed} of {cases} corrupted inputs reached the parser — \
         the sweep is no longer exercising anything"
    );
}
