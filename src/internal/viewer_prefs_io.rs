//! Reader for the document catalog's viewer-preference entries:
//! `/PageLayout`, `/PageMode`, and the `/ViewerPreferences` sub-dict's
//! `/Direction` and `/NonFullScreenPageMode`.
//!
//! Forgiving: any parse failure or absent key yields `None` for that
//! preference (a document without viewer preferences is valid).

use std::collections::BTreeMap;

use super::objects::{self, IndirectObject, Value};
use super::trailer;
use super::xref::{self, Entry};

use crate::public::direction::Direction;
use crate::public::page_layout::PageLayout;
use crate::public::page_mode::PageMode;
use crate::public::print_duplex::PrintDuplex;
use crate::public::print_scaling::PrintScaling;

type Dump = BTreeMap<(u32, u16), IndirectObject>;

/// The viewer-preference values found in a document's catalog. A `None`
/// field means the corresponding key was absent (the caller applies its
/// own default).
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ViewerPrefs {
    pub(crate) page_layout: Option<PageLayout>,
    pub(crate) page_mode: Option<PageMode>,
    pub(crate) direction: Option<Direction>,
    pub(crate) non_full_screen_page_mode: Option<PageMode>,
    pub(crate) duplex: Option<PrintDuplex>,
    pub(crate) print_scaling: Option<PrintScaling>,
}

/// Parse the catalog viewer preferences from `input`.
pub(crate) fn parse(input: &[u8]) -> ViewerPrefs {
    let mut prefs = ViewerPrefs::default();

    let trailer_bundle = match trailer::parse(input) {
        Ok(t) if t.root_id != 0 => t,
        _ => return prefs,
    };
    let dump = match build_dump(input) {
        Some(d) => d,
        None => return prefs,
    };
    let catalog = match dump.get(&(trailer_bundle.root_id, 0)) {
        Some(c) => c,
        None => return prefs,
    };
    let catalog_dict = match &catalog.value {
        Value::Dict(entries) => entries,
        _ => return prefs,
    };

    if let Some(Value::Name(n)) = lookup(catalog_dict, b"PageLayout") {
        prefs.page_layout = page_layout_from_name(n);
    }
    if let Some(Value::Name(n)) = lookup(catalog_dict, b"PageMode") {
        prefs.page_mode = page_mode_from_name(n);
    }

    // /ViewerPreferences is a sub-dict, possibly indirect.
    if let Some(vp) = lookup(catalog_dict, b"ViewerPreferences") {
        let vp_dict = match vp {
            Value::Dict(d) => Some(d.as_slice()),
            Value::Ref { id, gen } => match dump.get(&(*id, *gen)) {
                Some(obj) => match &obj.value {
                    Value::Dict(d) => Some(d.as_slice()),
                    _ => None,
                },
                None => None,
            },
            _ => None,
        };
        if let Some(vp_dict) = vp_dict {
            if let Some(Value::Name(n)) = lookup(vp_dict, b"Direction") {
                prefs.direction = direction_from_name(n);
            }
            if let Some(Value::Name(n)) = lookup(vp_dict, b"NonFullScreenPageMode") {
                prefs.non_full_screen_page_mode = page_mode_from_name(n);
            }
            if let Some(Value::Name(n)) = lookup(vp_dict, b"Duplex") {
                prefs.duplex = duplex_from_name(n);
            }
            if let Some(Value::Name(n)) = lookup(vp_dict, b"PrintScaling") {
                prefs.print_scaling = print_scaling_from_name(n);
            }
        }
    }

    prefs
}

fn page_layout_from_name(name: &[u8]) -> Option<PageLayout> {
    match name {
        b"SinglePage" => Some(PageLayout::SinglePage),
        b"OneColumn" => Some(PageLayout::OneColumn),
        b"TwoColumnLeft" => Some(PageLayout::TwoColumnLeft),
        b"TwoColumnRight" => Some(PageLayout::TwoColumnRight),
        b"TwoPageLeft" => Some(PageLayout::TwoPageLeft),
        b"TwoPageRight" => Some(PageLayout::TwoPageRight),
        _ => None,
    }
}

fn page_mode_from_name(name: &[u8]) -> Option<PageMode> {
    match name {
        b"UseNone" => Some(PageMode::UseNone),
        b"UseOutlines" => Some(PageMode::UseOutlines),
        b"UseThumbs" => Some(PageMode::UseThumbs),
        b"FullScreen" => Some(PageMode::FullScreen),
        b"UseOC" => Some(PageMode::UseOC),
        b"UseAttachments" => Some(PageMode::UseAttachments),
        _ => None,
    }
}

fn direction_from_name(name: &[u8]) -> Option<Direction> {
    match name {
        b"L2R" => Some(Direction::L2R),
        b"R2L" => Some(Direction::R2L),
        _ => None,
    }
}

fn duplex_from_name(name: &[u8]) -> Option<PrintDuplex> {
    match name {
        b"Simplex" => Some(PrintDuplex::Simplex),
        b"DuplexFlipShortEdge" => Some(PrintDuplex::DuplexFlipShortEdge),
        b"DuplexFlipLongEdge" => Some(PrintDuplex::DuplexFlipLongEdge),
        _ => None,
    }
}

fn print_scaling_from_name(name: &[u8]) -> Option<PrintScaling> {
    match name {
        b"AppDefault" => Some(PrintScaling::AppDefault),
        b"None" => Some(PrintScaling::None),
        _ => None,
    }
}

fn build_dump(input: &[u8]) -> Option<Dump> {
    let table = xref::parse(input).ok()?;
    let mut dump: Dump = BTreeMap::new();
    for entry in &table.entries {
        if let Entry::InUse { id, gen, offset } = *entry {
            if let Ok(obj) = objects::parse_object(input, offset as usize) {
                dump.insert((id, gen), obj);
            }
        }
    }
    Some(dump)
}

fn lookup<'a>(dict: &'a [(Vec<u8>, Value)], key: &[u8]) -> Option<&'a Value> {
    dict.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}
