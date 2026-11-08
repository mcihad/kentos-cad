//! Doğrama kodları (docs/adr/3028; ADV-16, ADV-03): the joinery codes Mimarlar Odası's and İTÜ's drawing notes give
//! doors and windows, one for each type, the same on every opening of it (“aynı türde olup farklı boyut ve özelliklere
//! sahip yapı elemanlarına … ayırt edici referans numaraları verilir”).
//!
//! - **Kinds:** K a door, GK a door with a room on one side only (the building's outer wall: giriş kapısı), P a
//!   window, B a bare opening (KentOS's own letter, docs/adr/0714). Niches and flues take none.
//! - **Numbers:** two digits from 01 (K01, P01), the next free one of the kind, in the order the drawing first has the
//!   type. A code given before stays.
//! - **Types first:** an opening following no project type gets one of its own, one for each make (its form and every
//!   field a type holds), named by its form and size (Kapı 90/210); the openings of that make follow it. One on a
//!   locked layer is left out.
//!
//! `cad.openingtypes.edit`'s Codes writes this plan; Doğrama kodları' preview draws it.

use kentos_commands::{
    Entity, OpeningEntity, OpeningFields, OpeningForm, OpeningType, OpeningTypeField,
    builtin_opening_types, opening_type_id_of,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::Vec2 as V;

/// How far beyond a wall's face a door's side is looked at for a room, metres.
const SIDE: f64 = 0.3;

/// An opening the plan codes: its object, the code its type takes, its type's name and its middle.
#[derive(Clone, Debug, PartialEq)]
pub struct Coded {
    pub slot: Slot,
    pub code: String,
    pub type_name: String,
    pub at: V,
}

/// What Doğrama kodları writes: the project's types after it, the openings that start following a type, every coded
/// opening, and how many were left out on locked layers.
#[derive(Clone, Debug, Default)]
pub struct Plan {
    pub types: Vec<OpeningType>,
    pub updates: Vec<(Slot, OpeningEntity)>,
    pub coded: Vec<Coded>,
    pub locked: usize,
    /// The types it gave a code, in the order it gave them.
    pub given: Vec<String>,
}

/// The letters of a kind's code.
fn prefix(form: OpeningForm, outer: bool) -> Option<&'static str> {
    match form {
        OpeningForm::Door if outer => Some("GK"),
        OpeningForm::Door => Some("K"),
        OpeningForm::Window => Some("P"),
        OpeningForm::Opening => Some("B"),
        OpeningForm::Niche | OpeningForm::Flue => None,
    }
}

/// The next free code of `letters` among `types`: the highest number written after them, plus one.
fn next_code(types: &[OpeningType], letters: &str) -> String {
    let top = types
        .iter()
        .filter_map(|t| t.code.as_deref())
        .filter_map(|c| {
            let rest = c.get(letters.len()..)?;
            (c[..letters.len()].eq_ignore_ascii_case(letters)
                && !rest.is_empty()
                && rest.chars().all(|ch| ch.is_ascii_digit()))
            .then(|| rest.parse::<u32>().ok())
            .flatten()
        })
        .max()
        .unwrap_or(0);
    format!("{letters}{:02}", top + 1)
}

/// An opening's middle and its two sides' points a little beyond its wall's faces: left, right.
pub(crate) fn sides(f: &OpeningFields) -> Option<(V, V, V)> {
    let p: Vec<V> = f.place.iter().map(|q| V::new(q.x, q.y)).collect();
    if p.len() != 4 {
        return None;
    }
    let mid = |a: V, b: V| V::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    let (left, right) = (mid(p[0], p[2]), mid(p[1], p[3]));
    let c = mid(left, right);
    let (dx, dy) = (left.x - right.x, left.y - right.y);
    let len = dx.hypot(dy);
    if len < 1e-9 {
        return None;
    }
    let n = V::new(dx / len, dy / len);
    Some((
        c,
        V::new(left.x + n.x * SIDE, left.y + n.y * SIDE),
        V::new(right.x - n.x * SIDE, right.y - n.y * SIDE),
    ))
}

/// The rooms of each level, their areas as the geometry reads them.
pub(crate) fn rooms(doc: &Document) -> Rooms {
    doc.entities()
        .filter_map(|e| match e {
            Entity::Space(s) => Some((
                e.own_level().map(str::to_owned),
                kentos_domain::arch_values::core_areas(&s.space.parts),
            )),
            _ => None,
        })
        .collect()
}

/// The rooms of each level (`rooms`).
pub(crate) type Rooms = Vec<(
    Option<String>,
    Vec<kentos_geometry_core::geom::arrangement::Area>,
)>;

/// Whether `p` is in a room of `level`.
pub(crate) fn in_room(rooms: &Rooms, level: &Option<String>, p: V) -> bool {
    rooms.iter().any(|(l, areas)| {
        l == level
            && areas
                .iter()
                .any(|a| kentos_geometry_core::geom::region::inside_area(a, p))
    })
}

/// Whether a door has a room on one side only: it is in the building's outer wall.
fn outer(doc: &Document, f: &OpeningFields, rooms: &Rooms) -> bool {
    let Some((_, left, right)) = sides(f) else {
        return false;
    };
    let level = crate::opening_types::level_of(doc, f);
    in_room(rooms, &level, left) != in_room(rooms, &level, right)
}

/// A type of `f`'s make: every field a type holds taken from it.
fn type_of(f: &OpeningFields, id: String, name: String) -> OpeningType {
    let base = builtin_opening_types()
        .into_iter()
        .find(|t| t.form == f.form)
        .unwrap_or_else(|| builtin_opening_types().remove(0));
    let mut t = crate::opening_types::of_builtin(&base, id, name);
    t.form = f.form;
    for k in OpeningTypeField::ALL {
        let _ = t.set_value(k, &f.value(k));
    }
    t
}

/// A make's key: its form and every field a type holds.
fn make_of(f: &OpeningFields) -> String {
    let values: Vec<serde_json::Value> =
        OpeningTypeField::ALL.iter().map(|k| f.value(*k)).collect();
    format!("{:?}{}", f.form, serde_json::Value::Array(values))
}

/// A free id for `name` among `types`: from its name, then with `-2`, `-3` ….
fn free_id(types: &[OpeningType], name: &str) -> String {
    let base = opening_type_id_of(name);
    let mut id = base.clone();
    let mut n = 2;
    while types.iter().any(|t| t.id == id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// A free name: `name`, then “ (2)”, “ (3)” ….
fn free_name(types: &[OpeningType], name: &str) -> String {
    let taken = |n: &str| {
        types
            .iter()
            .any(|t| t.name.to_lowercase() == n.to_lowercase())
    };
    let mut out = name.to_owned();
    let mut k = 2;
    while taken(&out) {
        out = format!("{name} ({k})");
        k += 1;
    }
    out
}

/// The plan of Doğrama kodları on `doc`.
pub fn plan(doc: &Document) -> Plan {
    let mut types = doc.opening_types().to_vec();
    let rooms = rooms(doc);
    let mut out = Plan::default();
    // Each type the drawing has, in the order it first has it, with whether all its doors are in an outer wall.
    let mut order: Vec<(String, bool)> = Vec::new();
    let mut made: Vec<(String, String)> = Vec::new();
    let mut members: Vec<(Slot, String, V)> = Vec::new();
    for e in doc.entities() {
        let Entity::Opening(o) = e else { continue };
        let f = &o.opening;
        if prefix(f.form, false).is_none() {
            continue;
        }
        if doc.layers().is_locked(&o.base.layer_id) {
            out.locked += 1;
            continue;
        }
        let slot = Slot(o.base.id);
        let follows = f.follows_type()
            && f.style
                .as_deref()
                .is_some_and(|s| types.iter().any(|t| t.id == s));
        let id = if follows {
            f.style.clone().unwrap_or_default()
        } else {
            let make = make_of(f);
            match made.iter().find(|(m, _)| *m == make) {
                Some((_, id)) => id.clone(),
                None => {
                    let name = free_name(&types, &format!("{} {}", f.form.label(), f.size_cm()));
                    let id = free_id(&types, &name);
                    types.push(type_of(f, id.clone(), name));
                    made.push((make, id.clone()));
                    id
                }
            }
        };
        if !follows {
            let mut next = o.clone();
            if let Some(t) = types.iter().find(|t| t.id == id) {
                next.opening.take_type(t);
                next.opening.overrides.clear();
            }
            out.updates.push((slot, next));
        }
        let door_outer = f.form == OpeningForm::Door && outer(doc, f, &rooms);
        match order.iter_mut().find(|(t, _)| *t == id) {
            Some((_, all_outer)) => *all_outer &= door_outer,
            None => order.push((id.clone(), door_outer)),
        }
        let at = sides(f).map_or(V::new(0.0, 0.0), |(c, _, _)| c);
        members.push((slot, id, at));
    }
    for (id, all_outer) in &order {
        let Some(i) = types.iter().position(|t| t.id == *id) else {
            continue;
        };
        if types[i].code.is_some() {
            continue;
        }
        let Some(letters) = prefix(types[i].form, *all_outer) else {
            continue;
        };
        let code = next_code(&types, letters);
        types[i].code = Some(code);
        out.given.push(types[i].id.clone());
    }
    out.coded = members
        .into_iter()
        .filter_map(|(slot, id, at)| {
            let t = types.iter().find(|t| t.id == id)?;
            Some(Coded {
                slot,
                code: t.code.clone()?,
                type_name: t.name.clone(),
                at,
            })
        })
        .collect();
    out.types = types;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(code: Option<&str>) -> OpeningType {
        let base = builtin_opening_types().remove(0);
        let mut t = crate::opening_types::of_builtin(&base, "t".into(), "T".into());
        t.code = code.map(str::to_owned);
        t
    }

    #[test]
    fn the_next_code_counts_on_from_the_kinds_highest() {
        let types = [
            typed(Some("K01")),
            typed(Some("k03")),
            typed(Some("GK01")),
            typed(Some("P1")),
        ];
        assert_eq!(next_code(&types, "K"), "K04");
        assert_eq!(next_code(&types, "GK"), "GK02");
        assert_eq!(next_code(&types, "P"), "P02");
        assert_eq!(next_code(&types, "B"), "B01");
    }

    #[test]
    fn a_size_is_written_in_centimetres_width_over_height() {
        let mut f = typed(None).fields();
        f.width = 0.9;
        f.sill = None;
        f.head = Some(2.1);
        assert_eq!(f.size_cm(), "90/210");
        f.width = 1.2;
        f.sill = Some(0.9);
        f.head = Some(2.2);
        assert_eq!(f.size_cm(), "120/130");
        f.width = 0.905;
        assert_eq!(f.size_cm(), "90.5/130");
        f.head = None;
        assert_eq!(f.size_cm(), "90.5");
    }
}
