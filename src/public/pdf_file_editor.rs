//! `Aspose.Pdf.Facades.PdfFileEditor` — file-level page operations
//! (concatenate / extract / delete / append / insert / split), per
//! `capabilities/pdf_file_editor.yaml`.
//!
//! The shipped subset is the file-path form of each core operation
//! (each returning `true` on success), backed by the foundation
//! page-tree assembler. The `Stream` / `Document[]` overloads, the
//! booklet / n-up / resize transforms, and the portfolio variants
//! are dropped. Page numbers are 1-based.

use crate::internal::page_tree_assembler::{self, ParsedDoc};
use std::path::Path;

/// File-level PDF page editor.
#[derive(Debug, Default)]
pub struct PdfFileEditor;

impl PdfFileEditor {
    /// Construct an editor.
    pub fn new() -> Self {
        Self
    }

    /// Concatenate `input1` and `input2` into `output`.
    pub fn concatenate(
        &self,
        input1: impl AsRef<Path>,
        input2: impl AsRef<Path>,
        output: impl AsRef<Path>,
    ) -> bool {
        run(&[input1.as_ref(), input2.as_ref()], output, |docs| {
            all_pages(docs)
        })
    }

    /// Concatenate every file in `inputs` into `output`.
    pub fn concatenate_many(
        &self,
        inputs: &[&Path],
        output: impl AsRef<Path>,
    ) -> bool {
        run(inputs, output, all_pages)
    }

    /// Extract the inclusive 1-based page range `[start, end]` of
    /// `input` into `output`.
    pub fn extract(
        &self,
        input: impl AsRef<Path>,
        start: i32,
        end: i32,
        output: impl AsRef<Path>,
    ) -> bool {
        run(&[input.as_ref()], output, |docs| {
            let n = page_tree_assembler::page_count(&docs[0]);
            (start.max(1)..=end)
                .filter_map(|p| to_idx(p, n).map(|i| (0, i)))
                .collect()
        })
    }

    /// Extract the given 1-based `pages` of `input` into `output`.
    pub fn extract_pages(
        &self,
        input: impl AsRef<Path>,
        pages: &[i32],
        output: impl AsRef<Path>,
    ) -> bool {
        run(&[input.as_ref()], output, |docs| {
            let n = page_tree_assembler::page_count(&docs[0]);
            pages.iter().filter_map(|&p| to_idx(p, n).map(|i| (0, i))).collect()
        })
    }

    /// Delete the given 1-based `pages` from `input`, writing the
    /// rest to `output`.
    pub fn delete(
        &self,
        input: impl AsRef<Path>,
        pages: &[i32],
        output: impl AsRef<Path>,
    ) -> bool {
        run(&[input.as_ref()], output, |docs| {
            let n = page_tree_assembler::page_count(&docs[0]);
            (0..n)
                .filter(|&i| !pages.contains(&((i + 1) as i32)))
                .map(|i| (0, i))
                .collect()
        })
    }

    /// Append pages `[start, end]` (1-based) of `concat` to the end
    /// of `input`, writing `output`.
    pub fn append(
        &self,
        input: impl AsRef<Path>,
        concat: impl AsRef<Path>,
        start: i32,
        end: i32,
        output: impl AsRef<Path>,
    ) -> bool {
        run(&[input.as_ref(), concat.as_ref()], output, |docs| {
            let mut sel: Vec<(usize, usize)> =
                (0..page_tree_assembler::page_count(&docs[0])).map(|i| (0, i)).collect();
            let nb = page_tree_assembler::page_count(&docs[1]);
            sel.extend(
                (start.max(1)..=end).filter_map(|p| to_idx(p, nb).map(|i| (1, i))),
            );
            sel
        })
    }

    /// Insert pages `[start, end]` (1-based) of `infile` into
    /// `input` after 1-based page `location`, writing `output`.
    pub fn insert(
        &self,
        input: impl AsRef<Path>,
        location: i32,
        infile: impl AsRef<Path>,
        start: i32,
        end: i32,
        output: impl AsRef<Path>,
    ) -> bool {
        run(&[input.as_ref(), infile.as_ref()], output, |docs| {
            let na = page_tree_assembler::page_count(&docs[0]);
            let nb = page_tree_assembler::page_count(&docs[1]);
            let at = (location.max(0) as usize).min(na);
            let mut sel: Vec<(usize, usize)> = (0..at).map(|i| (0, i)).collect();
            sel.extend(
                (start.max(1)..=end).filter_map(|p| to_idx(p, nb).map(|i| (1, i))),
            );
            sel.extend((at..na).map(|i| (0, i)));
            sel
        })
    }

    /// Write the first `location` pages of `input` to `output`.
    pub fn split_from_first(
        &self,
        input: impl AsRef<Path>,
        location: i32,
        output: impl AsRef<Path>,
    ) -> bool {
        run(&[input.as_ref()], output, |docs| {
            let n = page_tree_assembler::page_count(&docs[0]);
            let upto = (location.max(0) as usize).min(n);
            (0..upto).map(|i| (0, i)).collect()
        })
    }

    /// Write pages from 1-based `location` to the end of `input`
    /// into `output`.
    pub fn split_to_end(
        &self,
        input: impl AsRef<Path>,
        location: i32,
        output: impl AsRef<Path>,
    ) -> bool {
        run(&[input.as_ref()], output, |docs| {
            let n = page_tree_assembler::page_count(&docs[0]);
            let from = to_idx(location, n).unwrap_or(0);
            (from..n).map(|i| (0, i)).collect()
        })
    }

    // Try* aliases — same operations, named per the canonical
    // Try-prefixed overloads.

    /// See [`concatenate`](Self::concatenate).
    pub fn try_concatenate(
        &self,
        input1: impl AsRef<Path>,
        input2: impl AsRef<Path>,
        output: impl AsRef<Path>,
    ) -> bool {
        self.concatenate(input1, input2, output)
    }
    /// See [`concatenate_many`](Self::concatenate_many).
    pub fn try_concatenate_many(&self, inputs: &[&Path], output: impl AsRef<Path>) -> bool {
        self.concatenate_many(inputs, output)
    }
    /// See [`extract`](Self::extract).
    pub fn try_extract(
        &self,
        input: impl AsRef<Path>,
        start: i32,
        end: i32,
        output: impl AsRef<Path>,
    ) -> bool {
        self.extract(input, start, end, output)
    }
    /// See [`extract_pages`](Self::extract_pages).
    pub fn try_extract_pages(
        &self,
        input: impl AsRef<Path>,
        pages: &[i32],
        output: impl AsRef<Path>,
    ) -> bool {
        self.extract_pages(input, pages, output)
    }
    /// See [`delete`](Self::delete).
    pub fn try_delete(
        &self,
        input: impl AsRef<Path>,
        pages: &[i32],
        output: impl AsRef<Path>,
    ) -> bool {
        self.delete(input, pages, output)
    }
    /// See [`split_from_first`](Self::split_from_first).
    pub fn try_split_from_first(
        &self,
        input: impl AsRef<Path>,
        location: i32,
        output: impl AsRef<Path>,
    ) -> bool {
        self.split_from_first(input, location, output)
    }
    /// See [`split_to_end`](Self::split_to_end).
    pub fn try_split_to_end(
        &self,
        input: impl AsRef<Path>,
        location: i32,
        output: impl AsRef<Path>,
    ) -> bool {
        self.split_to_end(input, location, output)
    }
}

/// Convert a 1-based page number to a 0-based index within `n` pages.
fn to_idx(page: i32, n: usize) -> Option<usize> {
    if page >= 1 && (page as usize) <= n {
        Some((page - 1) as usize)
    } else {
        None
    }
}

fn all_pages(docs: &[ParsedDoc]) -> Vec<(usize, usize)> {
    let mut sel = Vec::new();
    for (di, d) in docs.iter().enumerate() {
        for i in 0..page_tree_assembler::page_count(d) {
            sel.push((di, i));
        }
    }
    sel
}

/// Read every `input`, build the page selection, assemble, and write
/// `output`; returns `false` on any read / parse / write failure.
fn run(
    inputs: &[&Path],
    output: impl AsRef<Path>,
    select: impl Fn(&[ParsedDoc]) -> Vec<(usize, usize)>,
) -> bool {
    let mut docs = Vec::with_capacity(inputs.len());
    for path in inputs {
        match std::fs::read(path).ok().and_then(|b| page_tree_assembler::parse(&b)) {
            Some(d) => docs.push(d),
            None => return false,
        }
    }
    let sel = select(&docs);
    let bytes = page_tree_assembler::assemble(&docs, &sel);
    std::fs::write(output, bytes).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::document::Document;

    fn fixture(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/text_extractor")
            .join(name)
    }
    fn tmp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(name)
    }
    fn page_count(p: &std::path::Path) -> usize {
        Document::open(p).unwrap().pages().count()
    }

    #[test]
    fn concatenate_extract_delete() {
        let ed = PdfFileEditor::new();
        let hello = fixture("hello_world.pdf");
        let two = fixture("two_pages.pdf");

        let cat = tmp("asp_rust_cat.pdf");
        assert!(ed.concatenate(&hello, &two, &cat));
        assert_eq!(page_count(&cat), 3); // 1 + 2

        let ext = tmp("asp_rust_ext.pdf");
        assert!(ed.extract(&two, 1, 1, &ext));
        assert_eq!(page_count(&ext), 1);

        let del = tmp("asp_rust_del.pdf");
        assert!(ed.delete(&two, &[1], &del));
        assert_eq!(page_count(&del), 1);

        let app = tmp("asp_rust_app.pdf");
        assert!(ed.append(&hello, &two, 1, 1, &app));
        assert_eq!(page_count(&app), 2); // 1 + 1

        for f in [cat, ext, del, app] {
            let _ = std::fs::remove_file(f);
        }
    }

    #[test]
    fn split_operations() {
        let ed = PdfFileEditor::new();
        let two = fixture("two_pages.pdf");
        let first = tmp("asp_rust_first.pdf");
        let rest = tmp("asp_rust_rest.pdf");
        assert!(ed.split_from_first(&two, 1, &first));
        assert_eq!(page_count(&first), 1);
        assert!(ed.split_to_end(&two, 2, &rest));
        assert_eq!(page_count(&rest), 1);
        for f in [first, rest] {
            let _ = std::fs::remove_file(f);
        }
    }
}
