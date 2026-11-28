//! Two revisions of a model set side by side (feature `geometry`; docs/adr/8103; plan/ifc-uyum.md IFC-10): objects
//! are the same object when their GlobalIds are the same, nothing else; an object only the newer file has is added,
//! one only the older has is removed, one both have is changed when any of these differ:
//!
//! - its class, its attributes (Name, Description, ObjectType, LongName, Tag, PredefinedType);
//! - the spatial structure that contains it and its type object (by their GlobalIds);
//! - its body: one side has one and the other not, its box's middle moved by more than the tolerance, or its box's size
//!   changed by more than the tolerance on an axis (a shape changed inside the same box is not seen here);
//! - a property or a quantity of its sets (by the set's name, whether it came from its type, the value's name): added,
//!   removed, or another value (a real equal to the rounding of a re-export, nine digits, is the same).
//!
//! The files' order is kept: the added and the changed in the newer file's, the removed in the older's. A GlobalId a
//! file holds twice says nothing of which object is which: such objects are left out and listed.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::read::{Model, PropertySet, ReadError, SetKind, ValueKind};

/// What changed on an object both files have.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Class {
        old: String,
        new: String,
    },
    /// An attribute by its schema name (`Name`, `Tag`), absent where unset.
    Attribute {
        name: &'static str,
        old: Option<String>,
        new: Option<String>,
    },
    /// The spatial structure containing it: its index in the older file's objects and in the newer's.
    Container {
        old: Option<usize>,
        new: Option<usize>,
    },
    /// Its type object: its index in each file's objects.
    Type {
        old: Option<usize>,
        new: Option<usize>,
    },
    /// A body on one side only.
    Body {
        old: bool,
        new: bool,
    },
    /// Its box's middle moved, metres (x, y, z of the newer less the older's).
    Moved {
        by: [f64; 3],
    },
    /// Its box's size, metres (along x, y, z), in each file.
    Resized {
        old: [f64; 3],
        new: [f64; 3],
    },
    /// A property or a quantity: its set's name, whether the set is its type's, its name, its value in each file
    /// (absent where it has none).
    Property {
        set: String,
        from_type: bool,
        name: String,
        old: Option<ValueKind>,
        new: Option<ValueKind>,
    },
}

/// An object both files have and what changed on it: its index in each file's objects.
#[derive(Debug, Clone, PartialEq)]
pub struct Changed {
    pub older: usize,
    pub newer: usize,
    pub changes: Vec<Change>,
}

/// The comparison: the added (newer's indices), the removed (older's), the changed, how many are the same, and the
/// GlobalIds a file holds more than once (left out), with which file (true: the newer).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Comparison {
    pub added: Vec<usize>,
    pub removed: Vec<usize>,
    pub changed: Vec<Changed>,
    pub unchanged: usize,
    pub repeated: Vec<(bool, String)>,
}

/// Two reals are the same when they differ by no more than a re-export's rounding: nine digits of the larger.
fn same_real(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

/// Whether two values are the same as the file writes them; a reference is the same as another reference (its
/// record's number is the file's own).
pub fn same_value(a: &ValueKind, b: &ValueKind) -> bool {
    match (a, b) {
        (
            ValueKind::Number {
                value: x,
                measure: m,
            },
            ValueKind::Number {
                value: y,
                measure: n,
            },
        ) => m == n && same_real(*x, *y),
        (ValueKind::List(x), ValueKind::List(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same_value(p, q))
        }
        (
            ValueKind::Bounded {
                lower: l1,
                upper: u1,
            },
            ValueKind::Bounded {
                lower: l2,
                upper: u2,
            },
        ) => same_value(l1, l2) && same_value(u1, u2),
        (ValueKind::Reference(_), ValueKind::Reference(_)) => true,
        _ => a == b,
    }
}

/// Each object's body box (x, y, z from, x, y, z to), metres in the file's world; none without a body.
fn boxes(
    model: &Model,
    bytes: &[u8],
    cancel: &dyn Fn() -> bool,
) -> Result<Vec<Option<[f64; 6]>>, ReadError> {
    let mut out = vec![None; model.objects.len()];
    for b in model.bodies(bytes, cancel)?.bodies {
        let Some(slot) = out.get_mut(b.object) else {
            continue;
        };
        for p in &b.positions {
            let k = slot.get_or_insert([p[0], p[1], p[2], p[0], p[1], p[2]]);
            for i in 0..3 {
                k[i] = k[i].min(p[i]);
                k[i + 3] = k[i + 3].max(p[i]);
            }
        }
    }
    Ok(out)
}

/// The GlobalIds of a file once each, by index; those it holds more than once apart.
fn by_global_id(model: &Model) -> (HashMap<&str, usize>, HashSet<&str>) {
    let mut seen: HashMap<&str, usize> = HashMap::new();
    let mut twice: HashSet<&str> = HashSet::new();
    for (i, o) in model.objects.iter().enumerate() {
        if o.global_id.is_empty() {
            continue;
        }
        if seen.insert(o.global_id.as_str(), i).is_some() {
            twice.insert(o.global_id.as_str());
        }
    }
    for g in &twice {
        seen.remove(g);
    }
    (seen, twice)
}

/// The values of an object's sets by (set, from its type, value), in the sets' order.
fn values(sets: &[PropertySet]) -> Vec<((String, bool, String), ValueKind)> {
    let mut out: Vec<((String, bool, String), ValueKind)> = Vec::new();
    let mut seen: HashSet<(String, bool, String)> = HashSet::new();
    for s in sets {
        if s.kind == SetKind::Other {
            // A definition with no values: its name stands for it.
            let key = (s.name.clone(), s.from_type, String::new());
            if seen.insert(key.clone()) {
                out.push((key, ValueKind::Empty));
            }
            continue;
        }
        for v in &s.values {
            let key = (s.name.clone(), s.from_type, v.name.clone());
            if seen.insert(key.clone()) {
                out.push((key, v.value.clone()));
            }
        }
    }
    out
}

fn attributes(o: &crate::read::Object) -> [(&'static str, Option<&str>); 6] {
    [
        ("Name", o.name.as_deref()),
        ("Description", o.description.as_deref()),
        ("ObjectType", o.object_type.as_deref()),
        ("LongName", o.long_name.as_deref()),
        ("Tag", o.tag.as_deref()),
        ("PredefinedType", o.predefined.as_deref()),
    ]
}

/// `older` and `newer` (each read from the bytes beside it) compared: moves and size changes past `tolerance` metres
/// count; `cancel` is asked now and then whether to stop.
pub fn compare(
    older: &Model,
    older_bytes: &[u8],
    newer: &Model,
    newer_bytes: &[u8],
    tolerance: f64,
    cancel: &dyn Fn() -> bool,
) -> Result<Comparison, ReadError> {
    let (old_ids, old_twice) = by_global_id(older);
    let (new_ids, new_twice) = by_global_id(newer);
    let old_boxes = boxes(older, older_bytes, cancel)?;
    let new_boxes = boxes(newer, newer_bytes, cancel)?;
    let mut out = Comparison::default();
    let mut repeated: Vec<(bool, String)> =
        old_twice.iter().map(|g| (false, (*g).to_owned())).collect();
    repeated.extend(new_twice.iter().map(|g| (true, (*g).to_owned())));
    repeated.sort();
    out.repeated = repeated;
    let gid_of = |m: &Model, i: Option<usize>| {
        i.and_then(|i| m.objects.get(i))
            .map(|o| o.global_id.clone())
    };
    for (j, n) in newer.objects.iter().enumerate() {
        if cancel() {
            return Err(ReadError::Cancelled);
        }
        if new_twice.contains(n.global_id.as_str()) || n.global_id.is_empty() {
            continue;
        }
        let Some(&i) = old_ids.get(n.global_id.as_str()) else {
            out.added.push(j);
            continue;
        };
        let o = &older.objects[i];
        let mut changes = Vec::new();
        if o.class != n.class {
            changes.push(Change::Class {
                old: o.class.clone(),
                new: n.class.clone(),
            });
        }
        for ((name, a), (_, b)) in attributes(o).into_iter().zip(attributes(n)) {
            if a != b {
                changes.push(Change::Attribute {
                    name,
                    old: a.map(str::to_owned),
                    new: b.map(str::to_owned),
                });
            }
        }
        if gid_of(older, o.container) != gid_of(newer, n.container) {
            changes.push(Change::Container {
                old: o.container,
                new: n.container,
            });
        }
        if gid_of(older, o.type_object) != gid_of(newer, n.type_object) {
            changes.push(Change::Type {
                old: o.type_object,
                new: n.type_object,
            });
        }
        match (old_boxes[i], new_boxes[j]) {
            (Some(a), Some(b)) => {
                let by = [
                    (b[0] + b[3] - a[0] - a[3]) / 2.0,
                    (b[1] + b[4] - a[1] - a[4]) / 2.0,
                    (b[2] + b[5] - a[2] - a[5]) / 2.0,
                ];
                if crate::math::hypot(crate::math::hypot(by[0], by[1]), by[2]) > tolerance {
                    changes.push(Change::Moved { by });
                }
                let (sa, sb) = (
                    [a[3] - a[0], a[4] - a[1], a[5] - a[2]],
                    [b[3] - b[0], b[4] - b[1], b[5] - b[2]],
                );
                if (0..3).any(|k| (sa[k] - sb[k]).abs() > tolerance) {
                    changes.push(Change::Resized { old: sa, new: sb });
                }
            }
            (None, None) => {}
            (a, b) => changes.push(Change::Body {
                old: a.is_some(),
                new: b.is_some(),
            }),
        }
        let old_values = values(&older.property_sets(older_bytes, i)?);
        let new_values = values(&newer.property_sets(newer_bytes, j)?);
        let old_map: BTreeMap<&(String, bool, String), &ValueKind> =
            old_values.iter().map(|(k, v)| (k, v)).collect();
        let new_keys: HashSet<&(String, bool, String)> =
            new_values.iter().map(|(k, _)| k).collect();
        for (key, value) in &new_values {
            match old_map.get(key) {
                Some(was) if same_value(was, value) => {}
                was => changes.push(Change::Property {
                    set: key.0.clone(),
                    from_type: key.1,
                    name: key.2.clone(),
                    old: was.map(|v| (*v).clone()),
                    new: Some(value.clone()),
                }),
            }
        }
        for (key, value) in &old_values {
            if !new_keys.contains(key) {
                changes.push(Change::Property {
                    set: key.0.clone(),
                    from_type: key.1,
                    name: key.2.clone(),
                    old: Some(value.clone()),
                    new: None,
                });
            }
        }
        if changes.is_empty() {
            out.unchanged += 1;
        } else {
            out.changed.push(Changed {
                older: i,
                newer: j,
                changes,
            });
        }
    }
    for (i, o) in older.objects.iter().enumerate() {
        if !o.global_id.is_empty()
            && !old_twice.contains(o.global_id.as_str())
            && !new_ids.contains_key(o.global_id.as_str())
            && !new_twice.contains(o.global_id.as_str())
        {
            out.removed.push(i);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read::{Limits, read};

    /// A wall and a slab in metres; `dx` moves the wall, `rating` its fire rating, `slab` keeps the slab.
    fn file(dx: f64, rating: &str, slab: bool, name: &str) -> Vec<u8> {
        let slab_lines = if slab {
            "#40=IFCSLAB('1kTvXnbbzCWw8lcMd1dR4o',$,'D-01',$,$,#41,#45,$,.FLOOR.);\n\
             #41=IFCLOCALPLACEMENT($,#42);\n#42=IFCAXIS2PLACEMENT3D(#43,$,$);\n#43=IFCCARTESIANPOINT((0.,0.,-0.2));\n\
             #45=IFCPRODUCTDEFINITIONSHAPE($,$,(#46));\n#46=IFCSHAPEREPRESENTATION(#11,'Body','SweptSolid',(#47));\n\
             #47=IFCEXTRUDEDAREASOLID(#48,#20,#25,0.2);\n#48=IFCRECTANGLEPROFILEDEF(.AREA.,$,#49,6.,4.);\n\
             #49=IFCAXIS2PLACEMENT2D(#50,$);\n#50=IFCCARTESIANPOINT((3.,2.));\n\
             #51=IFCRELCONTAINEDINSPATIALSTRUCTURE('2hYmZw4qr6Yx3rHlqXg$Vm',$,$,$,(#30,#40),#5);\n"
        } else {
            "#51=IFCRELCONTAINEDINSPATIALSTRUCTURE('2hYmZw4qr6Yx3rHlqXg$Vm',$,$,$,(#30),#5);\n"
        };
        format!(
            "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\nFILE_NAME('{name}','2026-11-17T12:00:00',(''),(''),'','','');\n\
             FILE_SCHEMA(('IFC4X3_ADD2'));\nENDSEC;\nDATA;\n\
             #1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,(#11),#2);\n#2=IFCUNITASSIGNMENT((#3));\n\
             #3=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n#5=IFCBUILDINGSTOREY('3pDT9cnlL0ZhgEdVxlbVpT',$,'Zemin kat',$,$,#6,$,$,.ELEMENT.,0.);\n\
             #6=IFCLOCALPLACEMENT($,#20);\n#7=IFCRELAGGREGATES('1rWSZOT4f7iAOb$rQ5V$4c',$,$,$,#1,(#5));\n\
             #10=IFCAXIS2PLACEMENT3D(#21,$,$);\n#11=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#10,$);\n\
             #20=IFCAXIS2PLACEMENT3D(#21,$,$);\n#21=IFCCARTESIANPOINT((0.,0.,0.));\n#25=IFCDIRECTION((0.,0.,1.));\n\
             #30=IFCWALL('0CmAx$J2b4n8dVdl_wLq1s',$,'W-01',$,$,#31,#35,$,.STANDARD.);\n#31=IFCLOCALPLACEMENT($,#32);\n\
             #32=IFCAXIS2PLACEMENT3D(#33,$,$);\n#33=IFCCARTESIANPOINT(({dx:.3},0.,0.));\n\
             #35=IFCPRODUCTDEFINITIONSHAPE($,$,(#36));\n#36=IFCSHAPEREPRESENTATION(#11,'Body','SweptSolid',(#37));\n\
             #37=IFCEXTRUDEDAREASOLID(#38,#20,#25,3.);\n#38=IFCRECTANGLEPROFILEDEF(.AREA.,$,#39,4.,0.2);\n\
             #39=IFCAXIS2PLACEMENT2D(#52,$);\n#52=IFCCARTESIANPOINT((2.,0.));\n\
             #60=IFCPROPERTYSET('2NjQa5Fwz9U8QpjP1VdUoS',$,'Pset_WallCommon',$,(#61,#62));\n\
             #61=IFCPROPERTYSINGLEVALUE('FireRating',$,IFCLABEL('{rating}'),$);\n\
             #62=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$);\n\
             #63=IFCRELDEFINESBYPROPERTIES('3wq9o1hBf5vRfYIVf8mN2c',$,$,$,(#30),#60);\n{slab_lines}ENDSEC;\nEND-ISO-10303-21;\n"
        )
        .into_bytes()
    }

    #[test]
    fn a_moved_wall_a_new_fire_rating_and_a_removed_slab_are_found() {
        let a = file(0.0, "REI 60", true, "a.ifc");
        let b = file(0.5, "REI 90", false, "b.ifc");
        let (ma, mb) = (
            read(&a, &Limits::default(), &|| false).unwrap(),
            read(&b, &Limits::default(), &|| false).unwrap(),
        );
        let c = compare(&ma, &a, &mb, &b, 0.001, &|| false).unwrap();
        let removed: Vec<&str> = c
            .removed
            .iter()
            .map(|&i| ma.objects[i].class.as_str())
            .collect();
        assert_eq!(removed, ["IfcSlab"]);
        assert!(c.added.is_empty());
        assert_eq!(c.changed.len(), 1, "{:?}", c.changed);
        let wall = &c.changed[0];
        assert_eq!(mb.objects[wall.newer].class, "IfcWall");
        let moved = wall.changes.iter().find_map(|x| match x {
            Change::Moved { by } => Some(*by),
            _ => None,
        });
        let by = moved.expect("moved");
        assert!(
            (by[0] - 0.5).abs() < 1e-6 && by[1].abs() < 1e-6 && by[2].abs() < 1e-6,
            "{by:?}"
        );
        let rating = wall.changes.iter().find_map(|x| match x {
            Change::Property {
                set,
                name,
                old,
                new,
                ..
            } if set == "Pset_WallCommon" && name == "FireRating" => {
                Some((old.clone(), new.clone()))
            }
            _ => None,
        });
        assert_eq!(
            rating,
            Some((
                Some(ValueKind::Text("REI 60".into())),
                Some(ValueKind::Text("REI 90".into()))
            ))
        );
        assert!(
            !wall
                .changes
                .iter()
                .any(|x| matches!(x, Change::Resized { .. })),
            "{:?}",
            wall.changes
        );
        // The project and the storey are the same.
        assert_eq!(c.unchanged, 2);
    }

    #[test]
    fn a_move_within_the_tolerance_is_no_change_and_the_same_file_is_the_same() {
        let a = file(0.0, "REI 60", true, "a.ifc");
        let b = file(0.0005, "REI 60", true, "b.ifc");
        let (ma, mb) = (
            read(&a, &Limits::default(), &|| false).unwrap(),
            read(&b, &Limits::default(), &|| false).unwrap(),
        );
        let c = compare(&ma, &a, &mb, &b, 0.001, &|| false).unwrap();
        assert!(
            c.changed.is_empty() && c.added.is_empty() && c.removed.is_empty(),
            "{c:?}"
        );
        assert_eq!(c.unchanged, 4);
        let c = compare(&ma, &a, &ma, &a, 0.001, &|| false).unwrap();
        assert_eq!((c.changed.len(), c.unchanged), (0, 4));
    }

    #[test]
    fn reals_equal_to_nine_digits_are_the_same() {
        let n = |v: f64| ValueKind::Number {
            value: v,
            measure: Some("IFCLENGTHMEASURE".into()),
        };
        assert!(same_value(&n(3.0), &n(3.000_000_000_1)));
        assert!(!same_value(&n(3.0), &n(3.000_01)));
        assert!(!same_value(
            &n(3.0),
            &ValueKind::Number {
                value: 3.0,
                measure: None
            }
        ));
    }
}
