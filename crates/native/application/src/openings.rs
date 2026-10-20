//! What the opening commands share (docs/adr/0341 §4): a wall named as an opening's, an opening type named, where on
//! its wall an opening goes, its jambs and the checks that it lies on one edge of its wall and clear of the others.

use std::collections::BTreeMap;

use kentos_commands::{
    Entity, NewOpening, OpeningAnchor, OpeningEntity, OpeningFields, OpeningForm, OpeningFrom,
    OpeningSide, OpeningTypeDef, Vec2, WallEnd, WallEntity, builtin_opening_types,
};
use kentos_domain::opening_follow::{self, WallLine};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::geom::opening;
use kentos_geometry_core::ops::edges::edge_length;
use kentos_geometry_core::ops::path::{nearest_s, point_at_s, tangent_at_s};

use crate::checks::{self, Stop, error};
use crate::codes;

pub(crate) fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// The wall `uid` names at `path`: its slot, itself and its base line; `invalid_uid`, `entity_not_found`,
/// `not_a_wall`, `not_drawable` when its path makes no line.
pub(crate) fn wall_at<'a>(
    doc: &'a Document,
    uid: &str,
    path: &str,
) -> Result<(Slot, &'a WallEntity, WallLine), Stop> {
    if !checks::is_uid_text(uid) {
        return Err(fail(
            codes::INVALID_UID,
            format!("“{uid}” bir kalıcı kimlik (UUID) değil; duvarın kimliğini verin."),
            path,
        ));
    }
    let slot = Uuid::parse_str(uid)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .ok_or_else(|| {
            fail(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "Çizimde “{uid}” kimlikli nesne yok. Var olan bir duvarın kimliğini verin."
                ),
                path,
            )
        })?;
    let Some(e @ Entity::Wall(w)) = doc.get(slot) else {
        return Err(fail(
            codes::NOT_A_WALL,
            "Nesne duvar değil; kapı, pencere ve boşluk yalnız duvara konur.".into(),
            path,
        ));
    };
    let line = opening_follow::wall_line(e).ok_or_else(|| {
        fail(
            codes::NOT_A_WALL,
            "Duvarın yolu çizilemiyor; açıklık konamaz.".into(),
            path,
        )
    })?;
    Ok((slot, w, line))
}

/// An opening type the input names: KentOS's own by its id, then its name (case aside); `style_not_found` with the
/// names there are.
pub(crate) fn find_type(key: &str, path: &str) -> Result<OpeningTypeDef, Stop> {
    let folded = key.trim().to_lowercase();
    let all = builtin_opening_types();
    all.iter()
        .find(|t| t.id == key)
        .or_else(|| all.iter().find(|t| t.name.to_lowercase() == folded))
        .cloned()
        .ok_or_else(|| {
            let names: Vec<&str> = all.iter().map(|t| t.name.as_str()).collect();
            fail(
                codes::STYLE_NOT_FOUND,
                format!(
                    "“{key}” adında ya da kimliğinde açıklık türü yok. Var olanlar: {}.",
                    names.join(", ")
                ),
                path,
            )
        })
}

/// The first type of a form: what a door, window or bare opening is made with when no type is named.
pub(crate) fn first_type(form: OpeningForm) -> OpeningTypeDef {
    builtin_opening_types()
        .into_iter()
        .find(|t| t.form == form)
        .unwrap_or_else(|| builtin_opening_types().remove(0))
}

/// Whether an opening's style names `key` (its id, or a type whose name it is).
pub(crate) fn type_matches(o: &OpeningFields, key: &str) -> bool {
    let Some(id) = &o.style else { return false };
    id == key || find_type(key, "").is_ok_and(|t| &t.id == id)
}

/// The step's name for openings of `form`.
pub fn step_of(form: OpeningForm) -> &'static str {
    match form {
        OpeningForm::Door => "Kapı",
        OpeningForm::Window => "Pencere",
        OpeningForm::Opening => "Boşluk",
    }
}

/// `invalid_opening` for an opening whose fields make none.
pub(crate) fn valid(o: &OpeningFields, path: &str) -> Result<(), Stop> {
    match o.problem() {
        Some((field, words)) => Err(fail(
            codes::INVALID_OPENING,
            words,
            &if path.is_empty() {
                field
            } else {
                format!("{path}.{field}")
            },
        )),
        None => Ok(()),
    }
}

fn core(p: Vec2) -> CoreVec2 {
    CoreVec2::new(p.x, p.y)
}

/// `o` with its jambs written from `line`; `opening_off_wall` when it does not lie on one edge within its ends.
pub(crate) fn placed(
    o: &OpeningFields,
    line: &WallLine,
    path: &str,
) -> Result<OpeningFields, Stop> {
    let p = opening_follow::placed_on(o, Some(line), false);
    if p.broken.is_some() {
        let (s0, s1) = opening_follow::span(o);
        return Err(fail(
            codes::OPENING_OFF_WALL,
            format!(
                "Açıklık duvarın ucundan ya da köşesinden taşıyor: duvar boyunca {:.3}–{:.3} m'de, duvarın uzunluğu {:.3} m; açıklık duvarın tek bir kenarında, uçlarının arasında kalmalı. Açıklığı kaydırın ya da daraltın.",
                s0, s1, line.path.length
            ),
            path,
        ));
    }
    Ok(p)
}

/// Where a new opening's centre goes along `line` and the side its point is on, as `n` says; `invalid_opening` when it
/// gives no place.
pub(crate) fn centre_of(
    n: &NewOpening,
    width: f64,
    line: &WallLine,
    path: &str,
) -> Result<(f64, Option<OpeningSide>), Stop> {
    let p = &line.path;
    if let Some(at) = n.at {
        return Ok((at, None));
    }
    let Some(q) = n.point else {
        return Err(fail(
            codes::INVALID_OPENING,
            "Açıklığın yeri verilmedi: duvar boyunca uzaklığı (at) ya da duvarın yanında bir nokta (point) verin."
                .into(),
            path,
        ));
    };
    let s = nearest_s(p, core(q));
    let base = point_at_s(p, s);
    let t = tangent_at_s(p, s);
    let cross = t.x * (q.y - base.y) - t.y * (q.x - base.x);
    let side = Some(if cross < 0.0 {
        OpeningSide::Right
    } else {
        OpeningSide::Left
    });
    if let Some(d) = n.distance {
        let at =
            match kentos_commands::wall_end_of(n.from.unwrap_or(OpeningFrom::Near), s, p.length) {
                WallEnd::Start => d + width / 2.0,
                WallEnd::End => p.length - d - width / 2.0,
            };
        return Ok((at, side));
    }
    if n.middle {
        let tol = opening::SPAN_TOLERANCE * p.length.max(1.0);
        let i = (0..p.edges.len())
            .find(|&i| s <= p.cum[i] + edge_length(&p.edges[i]) + tol)
            .unwrap_or(p.edges.len().saturating_sub(1));
        return Ok((p.cum[i] + edge_length(&p.edges[i]) / 2.0, side));
    }
    Ok((s, side))
}

/// A new opening's fields from `n`: its type's, then the values it gives over them, centred at `at` (its anchor the
/// centre).
pub(crate) fn fields_of(n: &NewOpening, kind: &OpeningTypeDef) -> OpeningFields {
    let mut f = OpeningFields {
        host: n.host.clone(),
        at: 0.0,
        anchor: OpeningAnchor::Center,
        width: kind.width,
        form: kind.form,
        leaves: 1,
        operation: Default::default(),
        hand: Default::default(),
        side: Default::default(),
        swing: None,
        show_swing: true,
        frame: None,
        sill: None,
        head: None,
        style: None,
        number: None,
        place: Vec::new(),
        broken: None,
    };
    kind.apply(&mut f);
    if let Some(v) = n.form {
        f.form = v;
    }
    if let Some(v) = n.width {
        f.width = v;
    }
    if let Some(v) = n.leaves {
        f.leaves = v;
    }
    if let Some(v) = n.operation {
        f.operation = v;
    }
    if let Some(v) = n.hand {
        f.hand = v;
    }
    if let Some(v) = n.swing {
        f.swing = Some(v);
    }
    f.number.clone_from(&n.number);
    f
}

/// An opening's interval along its wall and, for one a call gives, where the input names it.
type Standing<'a> = ((f64, f64), Option<&'a str>);

/// The openings standing on each wall, slot by slot, broken ones left out; `changed` the ones a call changes, as they
/// will be (their old selves left out), `new` the ones it adds (each its place in the input).
pub(crate) fn clear_of_others(
    doc: &Document,
    changed: &[(Slot, &OpeningFields, String)],
    new: &[(&OpeningFields, String)],
) -> Result<(), Stop> {
    let mut by_host: BTreeMap<&str, Vec<Standing<'_>>> = BTreeMap::new();
    let changed_slots: Vec<Slot> = changed.iter().map(|(s, ..)| *s).collect();
    for (_, f, at) in changed {
        if f.broken.is_none() {
            by_host
                .entry(&f.host)
                .or_default()
                .push((opening_follow::span(f), Some(at.as_str())));
        }
    }
    for (f, at) in new {
        if f.broken.is_none() {
            by_host
                .entry(&f.host)
                .or_default()
                .push((opening_follow::span(f), Some(at.as_str())));
        }
    }
    for (host, list) in by_host.iter_mut() {
        for (slot, o) in doc.openings_of(host) {
            if !changed_slots.contains(&slot) && o.broken.is_none() {
                list.push((opening_follow::span(&o), None));
            }
        }
        for i in 0..list.len() {
            let Some(at) = list[i].1 else { continue };
            if let Some((j, _)) = list
                .iter()
                .enumerate()
                .find(|(j, (b, _))| *j != i && opening_follow::overlap(list[i].0, *b))
            {
                let (a, b) = (list[i].0, list[j].0);
                return Err(fail(
                    codes::OPENINGS_OVERLAP,
                    format!(
                        "Açıklık ({:.3}–{:.3} m) aynı duvardaki başka bir açıklığa ({:.3}–{:.3} m) biniyor. Açıklığı kaydırın ya da daraltın.",
                        a.0, a.1, b.0, b.1
                    ),
                    at,
                ));
            }
        }
    }
    Ok(())
}

/// An opening as the document stores it, given its base.
pub(crate) fn entity(base: kentos_commands::EntityBase, opening: OpeningFields) -> Entity {
    Entity::Opening(OpeningEntity { base, opening })
}
