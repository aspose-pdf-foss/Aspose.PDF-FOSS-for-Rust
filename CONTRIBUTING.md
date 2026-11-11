# Contributing to Aspose.PDF FOSS for Rust

This document covers building, testing and submitting changes.

## Requirements

- Rust 1.86 or newer (edition 2021). The floor is declared as `rust-version` in
  `Cargo.toml` and proved by a CI job that builds and tests against it.
- Nothing else. There are no native dependencies and no crates to fetch.

## Build and test

```sh
cargo build
cargo build --release
cargo test
```

The suite spans public-API smoke tests, per-primitive freeze gates, the README snippets
as doctests, a round trip over every fixture PDF, and a sweep of deliberately corrupted
documents. All of it runs offline.

Before opening a pull request, run the same gates CI does:

```sh
cargo build --all-targets
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo test --no-fail-fast
```

Zero clippy findings and zero rustdoc warnings are the standing state, not a target —
both are enforced, so a new one fails the build.

## Layout

```text
src/public/      public-API types; re-exported at the crate root by src/lib.rs
src/internal/    foundation primitives; crate-private
tests/public/    public-API smoke tests
tests/internal/  freeze-gate drivers for the primitives
tests/fixtures/  the fixture corpus
examples/        runnable example programs
pdfs/            sample PDFs the examples take
```

Public-API bodies call into foundation primitives. Primitives never reach back into the
public API.

## Two constraints worth knowing before you write code

**The public API is a subset of a specific existing API — it is not designed here.** Every
public type, method and enum mirrors the canonical Aspose.PDF for .NET surface: same names,
same shapes, adapted to Rust idiom (`snake_case` methods, `PascalCase` types). A method
that would be convenient but has no counterpart there cannot be added, however reasonable
it looks. Implementing *less* of that surface is expected; inventing beyond it is not.
Part of the shipped surface is deliberately not wired to the engine — those methods panic
naming the capability that would unblock them. The README's scope note gives each one a
reason, and the reason matters before you reach for one: some are outside the parity
ceiling because no sibling track ships them, and wiring those is not a contribution this
project can take.

**No dependencies.** `[dependencies]` and `[dev-dependencies]` stay empty, deliberately.
The Rust ecosystem has excellent crates for most of what this library does from scratch,
and the point is that it does them from scratch. A patch that adds a dependency will be
declined regardless of quality.

## Pull requests

- Branch from `main`.
- Include tests. A new public member needs at least a smoke test; a parser or codec change
  needs a case that would have failed before it.
- Run the gates above and make sure they pass.
- Describe what changed and why. If behaviour changed, say so explicitly.

Contributions are accepted under the project's MIT licence.
