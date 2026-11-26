//! The fonts folders' families as the core's typefaces (docs/adr/9313). They are numbered after the tables' faces:
//! first the families the host installed ([`font_data::install`]), then every other name in the order this program
//! met it. A family this program's folders do not hold keeps its name, so a text goes through the core unchanged; it
//! is drawn and measured in Arial.

use std::sync::{Mutex, OnceLock, PoisonError};

use super::font_data::installed;
use super::metrics::{FIRST, LAST};
use super::shaping;

/// The most letters a family's name may have (the contract's `MAX_FONT_FAMILY`).
pub const MAX_NAME: usize = 64;
/// The most family names one program numbers (the contract's `MAX_FONT_FAMILIES`, within a `u16` after the tables).
pub const MAX_FAMILIES: usize = 65_536 - 64;

/// The names met after the installed families, in the order this program met them.
static MET: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

/// The installed families' names, each once, in the order the host gave their faces.
pub fn installed_families() -> &'static [&'static str] {
    static NAMES: OnceLock<Vec<&'static str>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let mut out: Vec<&'static str> = Vec::new();
        for face in installed() {
            if !out.contains(&face.family) {
                out.push(face.family);
            }
        }
        out
    })
}

/// Whether `name` may be a family's name: trimmed, 1–[`MAX_NAME`] letters, no control character.
pub fn name_ok(name: &str) -> bool {
    !name.is_empty()
        && name.trim() == name
        && name.chars().count() <= MAX_NAME
        && !name.chars().any(char::is_control)
}

/// The slot of the family named `name`: an installed family's place, else a met name's after them (numbered now when
/// new). None for a name no family may have and past [`MAX_FAMILIES`].
pub(crate) fn slot(name: &str) -> Option<usize> {
    if !name_ok(name) {
        return None;
    }
    let installed = installed_families();
    if let Some(k) = installed.iter().position(|f| *f == name) {
        return Some(k);
    }
    let mut met = MET.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(k) = met.iter().position(|f| *f == name) {
        return Some(installed.len() + k);
    }
    if installed.len() + met.len() >= MAX_FAMILIES {
        return None;
    }
    met.push(Box::leak(name.to_owned().into_boxed_str()));
    Some(installed.len() + met.len() - 1)
}

/// The family's name in slot `k`.
pub(crate) fn name(k: usize) -> Option<&'static str> {
    let installed = installed_families();
    match k.checked_sub(installed.len()) {
        None => installed.get(k).copied(),
        Some(j) => MET.lock().unwrap_or_else(PoisonError::into_inner).get(j).copied(),
    }
}

/// How many slots are numbered.
pub(crate) fn count() -> usize {
    installed_families().len() + MET.lock().unwrap_or_else(PoisonError::into_inner).len()
}

/// Whether slot `k` is an installed family (else this program's folders do not hold it).
pub(crate) fn is_installed(k: usize) -> bool {
    k < installed_families().len()
}

/// An installed family's advances in thousandths of an em, as the tables' rows: not bold (500) and bold (600).
pub(crate) fn rows(k: usize) -> Option<&'static ([u16; 352], [u16; 352])> {
    static ROWS: OnceLock<Vec<([u16; 352], [u16; 352])>> = OnceLock::new();
    ROWS.get_or_init(|| installed_families().iter().map(|f| (row_of(f, 500), row_of(f, 600))).collect())
        .get(k)
}

/// The family's advances at `weight`, upright, from its nearest face; 0 for a letter it has no glyph of (the tables'
/// rule: such a letter counts as the face's average lowercase letter).
fn row_of(family: &str, weight: u16) -> [u16; 352] {
    let mut row = [0u16; 352];
    let Some(face) = shaping::select_font(family, weight, false).and_then(shaping::face_of) else {
        return row;
    };
    let em = f64::from(face.units_per_em().max(1));
    for (cell, code) in row.iter_mut().zip(FIRST..=LAST) {
        let advance = char::from_u32(code)
            .and_then(|c| face.glyph_index(c))
            .and_then(|g| face.glyph_hor_advance(g));
        if let Some(a) = advance {
            *cell = (f64::from(a) * 1000.0 / em).round().clamp(0.0, f64::from(u16::MAX)) as u16;
        }
    }
    row
}
