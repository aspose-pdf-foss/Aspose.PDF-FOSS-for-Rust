//! Shared test support.
//!
//! Included by integration tests via `#[path = "../common/mod.rs"] mod
//! common;`. Lives in a `tests/` subdirectory, so Cargo does not build
//! it as its own test binary.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static SEQ: AtomicU64 = AtomicU64::new(0);

/// A temp path unique to this process and call: the caller's `name` is
/// prefixed with the process id and a per-run counter, so two
/// overlapping `cargo test` invocations (e.g. parallel CI jobs sharing
/// a runner's temp dir) never collide on the same output filename.
pub fn unique_temp(name: impl std::fmt::Display) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("{}_{}_{}", std::process::id(), n, name))
}
