//! What the space commands share (docs/adr/0342 §4): the project's method, a level named, the space's fields and its
//! area measured by the core.

use kentos_commands::{ArchInfo, SpaceFields, SpaceMethod, SpacePart};
use kentos_domain::Document;

use crate::checks::{Stop, error};
use crate::codes;

pub(crate) fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// The project's method for new spaces (`settings.arch.spaceMethod`, else net).
pub fn project_method(doc: &Document) -> SpaceMethod {
    doc.settings()
        .arch
        .as_ref()
        .and_then(|a| a.space_method)
        .unwrap_or_default()
}

/// `invalid_space` for a space whose fields make none.
pub(crate) fn valid(s: &SpaceFields, path: &str) -> Result<(), Stop> {
    match s.problem() {
        Some((field, words)) => Err(fail(
            codes::INVALID_SPACE,
            words,
            &format!("{path}.{field}"),
        )),
        None => Ok(()),
    }
}

/// `level_not_found` for a level the project does not have.
pub(crate) fn level_known(doc: &Document, arch: &ArchInfo, path: &str) -> Result<(), Stop> {
    crate::walls::level_known(doc, arch, path)
}

/// `space_not_enclosed` at `path`.
pub(crate) fn not_enclosed(path: &str) -> Stop {
    fail(
        codes::SPACE_NOT_ENCLOSED,
        "Duvarlar noktanın çevresini kapatmıyor; mahal bulunamadı. Duvarları kapatın ya da mahalin sınırını çizin.".into(),
        path,
    )
}

/// Preserve the legacy enclosure error, but say when a column source or a
/// complete physical deduction is what stopped a new/refound room.
pub(crate) fn not_found(s: &SpaceFields, path: &str) -> Stop {
    match s.broken {
        Some(kentos_commands::SpaceBreak::NoFreeArea) => fail(codes::INVALID_SPACE,
            "Kolonlar düşülünce mahalın kullanılabilir alanı kalmıyor; hiçbir mahal eklenmedi.".into(),path),
        Some(kentos_commands::SpaceBreak::InvalidBoundarySource) => fail(codes::INVALID_SPACE,
            "Kaynak kolonun geometrisi okunamıyor; kolonları düzeltin. Hiçbir mahal eklenmedi.".into(),path),
        _ => not_enclosed(path),
    }
}

/// The core's areas of the parts.
pub fn areas(parts: &[SpacePart]) -> Vec<kentos_geometry_core::geom::arrangement::Area> {
    parts.iter().map(kentos_formats::spaces::area).collect()
}

/// The parts' area, holes off, m².
pub fn area_of(parts: &[SpacePart]) -> f64 {
    kentos_geometry_core::geom::space::area(&areas(parts))
}

/// The parts' rings' length, metres.
pub fn perimeter_of(parts: &[SpacePart]) -> f64 {
    kentos_geometry_core::geom::space::perimeter(&areas(parts))
}

/// Where the space's label stands (its largest part's pole).
pub fn label_point(parts: &[SpacePart]) -> Option<kentos_commands::Vec2> {
    kentos_geometry_core::geom::space::label_at(&areas(parts))
        .map(|p| kentos_commands::Vec2 { x: p.x, y: p.y })
}
