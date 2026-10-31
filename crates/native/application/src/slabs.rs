//! What the slab commands share (docs/adr/0722 §4): the numbers checked, an outline drawn or found from the walls round a
//! point, moved by an edge offset, and the slab's fields checked.

use kentos_commands::{
    Entity, NewSlabEdge, SlabBound, SlabBoundary, SlabEdge, SlabFace, SlabFields, SlabSourceKind, SpacePart, Vec2,
};
use kentos_domain::Document;
use kentos_geometry_core::geom::arrangement::Area;

use crate::checks::{self, Stop};
use crate::codes;
use crate::spaces::fail;

/// `invalid_slab` for a slab whose fields make none.
pub(crate) fn valid(s: &SlabFields, path: &str) -> Result<(), Stop> {
    match s.problem() {
        Some((field, words)) => Err(fail(codes::INVALID_SLAB, words, &format!("{path}.{field}"))),
        None => Ok(()),
    }
}

/// `not_finite` for a NaN or infinite point of the parts drawn.
pub(crate) fn parts_finite(parts: &[SpacePart], path: &str) -> Result<(), Stop> {
    for (k, p) in parts.iter().enumerate() {
        for (h, ring) in std::iter::once(&p.outer).chain(&p.holes).enumerate() {
            let at = if h == 0 { format!("{path}[{k}].outer") } else { format!("{path}[{k}].holes[{}]", h - 1) };
            for (j, q) in ring.pts.iter().enumerate() {
                checks::point(*q, &format!("{}. köşenin", j + 1), &format!("{at}.pts[{j}]"))?;
            }
            for (j, b) in ring.bulges.iter().flatten().enumerate() {
                checks::finite(*b, "Büküm", "Sonlu bir sayı verin.", &format!("{at}.bulges[{j}]"))?;
            }
        }
    }
    Ok(())
}

/// The edge a slab is given, its width the kind's when none is given; its runs the document's.
pub(crate) fn edge_of(e: &NewSlabEdge) -> SlabEdge {
    SlabEdge {
        kind: e.kind,
        width: e.width.unwrap_or_else(|| e.kind.width()),
        runs: Vec::new(),
    }
}

/// `not_finite` for an edge's width that is NaN or infinite.
pub(crate) fn edge_finite(e: &NewSlabEdge, path: &str) -> Result<(), Stop> {
    if let Some(w) = e.width {
        checks::finite(w, &format!("{} eni", e.kind.whose()), "Sonlu bir sayı verin.", &format!("{path}.width"))?;
    }
    Ok(())
}

/// The outline drawn (`parts`) or found from the walls round `seed` on `level` (`slab_not_enclosed` when they enclose
/// nothing round it); `invalid_slab` for neither or both.
pub(crate) fn outline(
    doc: &Document,
    parts: Option<&Vec<SpacePart>>,
    seed: Option<Vec2>,
    boundary: Option<SlabBoundary>,
    level: Option<&str>,
    path: &str,
) -> Result<Vec<SpacePart>, Stop> {
    match (parts, seed) {
        (Some(p), None) => Ok(p.clone()),
        (None, Some(at)) => {
            let room = boundary.unwrap_or_default() == SlabBoundary::Room;
            doc.slab_from_walls(at, level, room).map(|p| vec![p]).ok_or_else(|| {
                fail(
                    codes::SLAB_NOT_ENCLOSED,
                    if room {
                        "Duvarlar noktanın çevresinde bir oda kapatmıyor; döşemenin sınırı bulunamadı. Noktayı duvarlarla çevrili bir odanın içine verin ya da sınırı çizin.".into()
                    } else {
                        "Nokta hiçbir duvar konturunun içinde değil; döşemenin sınırı bulunamadı. Noktayı yapının içine verin ya da sınırı çizin.".into()
                    },
                    &format!("{path}.seed"),
                )
            })
        }
        (Some(_), Some(_)) => Err(fail(
            codes::INVALID_SLAB,
            "Döşemenin sınırı ya çizilir (parts) ya da duvarlardan bulunur (seed); ikisi birden verilmez.".into(),
            path,
        )),
        (None, None) => Err(fail(
            codes::INVALID_SLAB,
            "Döşemenin yeri verilmedi: sınırını (parts) ya da duvarlarla çevrili bir iç noktasını (seed) verin.".into(),
            path,
        )),
    }
}

/// The parts as the core reads them.
pub(crate) fn areas(parts: &[SpacePart]) -> Vec<Area> {
    parts.iter().map(kentos_formats::spaces::area).collect()
}

/// The core's areas as parts.
pub(crate) fn parts_of(areas: &[Area]) -> Vec<SpacePart> {
    areas.iter().map(kentos_domain::space_follow::part).collect()
}

/// `parts` with every edge moved by `d` (out when positive); `slab_offset_failed` when a ring turns over or crosses.
pub(crate) fn moved(parts: &[SpacePart], d: f64, path: &str) -> Result<Vec<SpacePart>, Stop> {
    if d == 0.0 {
        return Ok(parts.to_vec());
    }
    if d.is_nan() || d.abs() > kentos_commands::MAX_SLAB_EDGE_OFFSET {
        return Err(fail(
            codes::INVALID_SLAB,
            format!(
                "Kenar ötelemesi en çok ±{} m olur; {d} verildi.",
                kentos_commands::MAX_SLAB_EDGE_OFFSET
            ),
            path,
        ));
    }
    match kentos_geometry_core::geom::slab::offset(&areas(parts), d) {
        Ok(out) => Ok(parts_of(&out)),
        Err(f) => {
            let ring = match f.hole {
                Some(h) => format!("{}. parçanın {}. deliği", f.part + 1, h + 1),
                None => format!("{}. parçanın dış halkası", f.part + 1),
            };
            Err(fail(codes::SLAB_OFFSET_FAILED, format!("{} ({ring}).", f.words.trim_end_matches('.')), path))
        }
    }
}

/// A space a slab is bound to (docs/adr/0771 §4), by its persistent id: its rings; `invalid_uid`, `entity_not_found`,
/// `not_a_space` at `path` for one that is none.
pub(crate) fn space_parts(doc: &Document, uid: &str, path: &str) -> Result<Vec<SpacePart>, Stop> {
    let bad = |code: &str, words: String| Err(fail(code, words, path));
    if !checks::is_uid_text(uid) {
        return bad(
            codes::INVALID_UID,
            format!("“{uid}” geçerli bir nesne kimliği değil; mahalin kalıcı kimliğini verin."),
        );
    }
    let found = kentos_domain::Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s));
    match found {
        Some(Entity::Space(s)) => Ok(s.space.parts.clone()),
        Some(_) => bad(codes::NOT_A_SPACE, "Döşemenin bağlanacağı nesne mahal değil.".into()),
        None => bad(
            codes::ENTITY_NOT_FOUND,
            format!("“{uid}” kimlikli mahal çizimde yok; var olan bir mahalin kimliğini verin."),
        ),
    }
}

/// The bound a slab takes from walls round `seed` (by `boundary`) or from `space`, moved by `offset`.
pub(crate) fn bound_of(seed: Option<Vec2>, boundary: Option<SlabBoundary>, space: Option<&String>, offset: f64) -> SlabBound {
    SlabBound {
        source: if space.is_some() { SlabSourceKind::Space } else { SlabSourceKind::Walls },
        seed: if space.is_some() { None } else { seed },
        face: if boundary.unwrap_or_default() == SlabBoundary::Room { SlabFace::Room } else { SlabFace::Outline },
        space: space.cloned(),
        offset,
        uids: Vec::new(),
        broken: None,
    }
}

/// `s` with the walls that bound it written into its bound (docs/adr/0771 §4); the same when it has none.
pub(crate) fn with_bound_uids(doc: &Document, mut s: SlabFields) -> SlabFields {
    if let Some(b) = &s.bound
        && let Ok((_, uids)) = doc.bound_outline(b, s.arch.level.as_deref())
        && let Some(b) = &mut s.bound
    {
        b.uids = uids;
    }
    s
}
