//! A taken row someone else changed meanwhile, field by field (docs/adr/9148): the row as it was taken (`first`), as it
//! is here (`mine`) and in the table now (`theirs`). A field changed on one side only, or to the same value on both, is
//! no clash: the save writes the fields changed here and leaves the table's others as they are. A field changed on
//! both sides to different values is a clash, said with its three values; so is the geometry moved on both sides, a
//! row deleted in the table that was changed here, and a row deleted here that the table changed. Nothing of a clash is
//! written until the user chooses, field by field; [`merged`] makes the object a choice gives, against the table's row.

use std::collections::BTreeSet;

use kentos_commands::Entity;
use kentos_domain::Uuid;

use crate::geopackage::geometry_of;

/// A field both sides changed to different values: its name and its three values (none: empty).
#[derive(Clone, Debug, PartialEq)]
pub struct FieldClash {
    pub field: String,
    pub first: Option<String>,
    pub mine: Option<String>,
    pub theirs: Option<String>,
}

/// A taken row a save cannot write as it is: its layer, object and key; the row as taken and as it is here (none:
/// deleted here) and in the table (none: deleted there); the fields that clash and whether the geometry does.
#[derive(Clone, Debug, PartialEq)]
pub struct RowClash {
    pub layer: String,
    pub uid: Uuid,
    pub key: i64,
    pub first: Entity,
    pub mine: Option<Entity>,
    pub theirs: Option<Entity>,
    pub fields: Vec<FieldClash>,
    pub geometry: bool,
}

impl RowClash {
    /// The row was deleted in the table and changed here.
    pub fn gone(&self) -> bool {
        self.theirs.is_none()
    }

    /// The row was deleted here and changed in the table.
    pub fn deleted_here(&self) -> bool {
        self.mine.is_none() && self.theirs.is_some()
    }
}

/// A field's value; an empty text is none, as the table's NULL.
fn value(e: &Entity, field: &str) -> Option<String> {
    e.base().attrs.get(field).filter(|v| !v.is_empty()).cloned()
}

/// The fields any of the three holds, in order.
fn names<'a>(all: &[&'a Entity]) -> BTreeSet<&'a String> {
    all.iter().flat_map(|e| e.base().attrs.keys()).collect()
}

/// The clash of a taken row, or none when its sides can be written together. `mine` none: deleted here; `theirs` none:
/// deleted in the table.
pub fn clash_of(first: &Entity, mine: Option<&Entity>, theirs: Option<&Entity>) -> Option<(Vec<FieldClash>, bool)> {
    let Some(theirs) = theirs else {
        // Deleted in the table: a clash, deleted here too or not (a table refuses to delete a row it has not).
        return Some((Vec::new(), false));
    };
    let unchanged_there =
        first.base().attrs == theirs.base().attrs && geometry_of(first) == geometry_of(theirs);
    if unchanged_there {
        return None;
    }
    let Some(mine) = mine else {
        // Deleted here what the table changed.
        return Some((Vec::new(), false));
    };
    let mut fields = Vec::new();
    for name in names(&[first, mine, theirs]) {
        let (a, m, t) = (value(first, name), value(mine, name), value(theirs, name));
        if m != a && t != a && m != t {
            fields.push(FieldClash {
                field: name.clone(),
                first: a,
                mine: m,
                theirs: t,
            });
        }
    }
    let (ga, gm, gt) = (geometry_of(first), geometry_of(mine), geometry_of(theirs));
    let geometry = gm != ga && gt != ga && gm != gt;
    (geometry || !fields.is_empty()).then_some((fields, geometry))
}

/// The object a choice gives a changed row against the table's row `theirs`: each field changed here keeps its value
/// unless `take_theirs` says the table's, every other field takes the table's; the geometry is here's when it moved
/// here and `geometry_theirs` is false, else the table's. The save then writes only what stays the user's.
pub fn merged(
    first: &Entity,
    mine: &Entity,
    theirs: &Entity,
    take_theirs: &dyn Fn(&str) -> bool,
    geometry_theirs: bool,
) -> Entity {
    let moved_here = geometry_of(mine) != geometry_of(first);
    let mut out = if moved_here && !geometry_theirs {
        mine.clone()
    } else {
        let mut t = theirs.clone();
        t.base_mut().id = mine.base().id;
        mine.base().layer_id.clone_into(&mut t.base_mut().layer_id);
        t
    };
    let mut attrs = std::collections::BTreeMap::new();
    for name in names(&[first, mine, theirs]) {
        let changed_here = value(mine, name) != value(first, name);
        let keep = if changed_here && !take_theirs(name) { mine } else { theirs };
        if let Some(v) = keep.base().attrs.get(name) {
            attrs.insert(name.clone(), v.clone());
        }
    }
    out.base_mut().attrs = attrs;
    out
}

/// A value as the words show it: quoted, or “boş”.
fn shown(v: &Option<String>) -> String {
    v.as_ref().map_or_else(|| "boş".to_owned(), |s| format!("“{s}”"))
}

/// What a save that met clashes says: each row by its key and what clashes in it.
pub fn clash_words(clashes: &[RowClash]) -> String {
    let rows: Vec<String> = clashes
        .iter()
        .map(|c| {
            if c.gone() {
                return match c.mine {
                    Some(_) => format!("satır {}: tabloda silinmiş, sizde değişmiş", c.key),
                    None => format!("satır {}: tabloda da silinmiş", c.key),
                };
            }
            if c.deleted_here() {
                return format!("satır {}: sizde silinmiş, tabloda değişmiş", c.key);
            }
            let mut parts: Vec<String> = c
                .fields
                .iter()
                .map(|f| {
                    format!("{} (ilk {}, sizinki {}, tablodaki {})", f.field, shown(&f.first), shown(&f.mine), shown(&f.theirs))
                })
                .collect();
            if c.geometry {
                parts.push("şekil iki yanda da değişmiş".to_owned());
            }
            format!("satır {}: {}", c.key, parts.join(", "))
        })
        .collect();
    format!(
        "Başkası aynı satırların aynı alanlarını siz aldıktan sonra değiştirmiş ya da satırı silmiş ({}); hiçbir şey yazılmadı.",
        rows.join("; ")
    )
}

/// The first value of a clashing field as the words show it (the window's columns).
pub fn value_words(v: &Option<String>) -> String {
    v.clone().unwrap_or_else(|| "(boş)".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(durum: &str, not: Option<&str>, x: f64) -> Entity {
        let mut attrs = serde_json::json!({ "durum": durum });
        if let Some(n) = not {
            attrs["not"] = serde_json::Value::from(n);
        }
        serde_json::from_value(serde_json::json!({
            "kind": "point", "id": 7, "layerId": "saha", "p": { "x": x, "y": 0.0 }, "attrs": attrs
        }))
        .expect("a point")
    }

    #[test]
    fn different_fields_are_no_clash_and_the_same_field_to_two_values_is_one() {
        let first = row("acik", None, 0.0);
        // A changed durum, B wrote a note: written together.
        assert_eq!(clash_of(&first, Some(&row("A-ikinci", None, 0.0)), Some(&row("acik", Some("B-sadece-not"), 0.0))), None);
        // Both changed durum to the same value: no clash.
        assert_eq!(clash_of(&first, Some(&row("iyi", None, 0.0)), Some(&row("iyi", None, 0.0))), None);
        // To two values: a clash with the three values.
        let (fields, geometry) =
            clash_of(&first, Some(&row("benim", None, 0.0)), Some(&row("onun", None, 0.0))).expect("a clash");
        assert_eq!(
            fields,
            [FieldClash {
                field: "durum".into(),
                first: Some("acik".into()),
                mine: Some("benim".into()),
                theirs: Some("onun".into())
            }]
        );
        assert!(!geometry);
        // Moved on both sides to two places: the geometry clashes.
        assert_eq!(clash_of(&first, Some(&row("acik", None, 1.0)), Some(&row("acik", None, 2.0))), Some((Vec::new(), true)));
        // The table unchanged: nothing to check, whatever is here.
        assert_eq!(clash_of(&first, Some(&row("benim", None, 5.0)), Some(&first)), None);
        // Deleted in the table, or here while the table changed it: clashes with no field.
        assert_eq!(clash_of(&first, Some(&row("benim", None, 0.0)), None), Some((Vec::new(), false)));
        assert_eq!(clash_of(&first, None, Some(&row("onun", None, 0.0))), Some((Vec::new(), false)));
    }

    #[test]
    fn the_merge_keeps_what_is_chosen_and_the_tables_other_changes() {
        let first = row("acik", None, 0.0);
        let mine = row("benim", None, 1.0);
        let theirs = row("onun", Some("B-notu"), 0.0);
        // The table's durum chosen: the note and durum are the table's, the move here stays.
        let m = merged(&first, &mine, &theirs, &|f| f == "durum", false);
        assert_eq!(m.base().attrs.get("durum").map(String::as_str), Some("onun"));
        assert_eq!(m.base().attrs.get("not").map(String::as_str), Some("B-notu"), "the table's own change is kept");
        assert_eq!(geometry_of(&m), geometry_of(&mine));
        // Mine chosen: durum is mine, the note still the table's.
        let m = merged(&first, &mine, &theirs, &|_| false, false);
        assert_eq!(m.base().attrs.get("durum").map(String::as_str), Some("benim"));
        assert_eq!(m.base().attrs.get("not").map(String::as_str), Some("B-notu"));
        // The table's geometry chosen.
        let m = merged(&first, &mine, &theirs, &|_| false, true);
        assert_eq!(geometry_of(&m), geometry_of(&theirs));
        assert_eq!(m.base().id, mine.base().id);
    }
}
