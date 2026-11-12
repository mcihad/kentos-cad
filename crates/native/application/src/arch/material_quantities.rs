//! Metraj by material (docs/adr/0785 §4): `cad.spaces.quantities`' `materials`, given when the project keeps
//! materials. A row is a project material (by its id; every one of the table, in its order, even unused) or a text
//! material (a component's `material`, else its name; a slab's `material`), in the order the drawing first has it.
//!
//! - A wall component adds its wall once to `walls`; its area is the wall's net face area (none without a height: it adds
//!   nothing), its volume that area × the component's thickness (the components' thicknesses share the wall's width,
//!   `component_thicknesses`), so a wall's components' volumes add up to its volume.
//! - A slab adds itself to `slabs`; its net area and volume are summed unless it is left out of the quantities.
//! - `amount` is the area or the volume, by the material's way of counting (a text material: by volume).

use kentos_commands::{
    ArchMaterial, MaterialCategory, MaterialQuantity, MaterialRow, SlabFields, WallFields, component_thicknesses,
};

/// A wall as Metraj measured it: its fields and its net face area (none without a height).
pub(crate) type WallPart<'a> = (&'a WallFields, Option<f64>);
/// A slab as Metraj measured it: its fields, net area, volume and whether it is summed.
pub(crate) type SlabPart<'a> = (&'a SlabFields, f64, f64, bool);

struct Acc {
    row: MaterialRow,
}

fn key_of<'a>(table: &'a [ArchMaterial], id: Option<&str>, text: Option<&str>) -> Option<(Option<&'a ArchMaterial>, String)> {
    if let Some(m) = id.and_then(|id| table.iter().find(|m| m.id == id)) {
        return Some((Some(m), m.name.clone()));
    }
    let text = text.map(str::trim).filter(|t| !t.is_empty())?;
    Some((None, text.to_owned()))
}

fn row<'a>(rows: &'a mut Vec<Acc>, m: Option<&ArchMaterial>, name: &str) -> &'a mut MaterialRow {
    let at = rows.iter().position(|a| match m {
        Some(m) => a.row.id.as_deref() == Some(m.id.as_str()),
        None => a.row.id.is_none() && a.row.name == name,
    });
    let at = match at {
        Some(at) => at,
        None => {
            rows.push(Acc {
                row: MaterialRow {
                    id: m.map(|m| m.id.clone()),
                    name: name.to_owned(),
                    category: m.map(|m| m.category),
                    quantity: m.map_or(MaterialQuantity::Volume, |m| m.quantity),
                    walls: 0,
                    slabs: 0,
                    area: 0.0,
                    volume: 0.0,
                    amount: 0.0,
                },
            });
            rows.len() - 1
        }
    };
    &mut rows[at].row
}

/// The materials' rows; none when the project keeps no materials.
pub(crate) fn rows(table: &[ArchMaterial], walls: &[WallPart<'_>], slabs: &[SlabPart<'_>]) -> Vec<MaterialRow> {
    if table.is_empty() {
        return Vec::new();
    }
    let mut rows: Vec<Acc> = Vec::new();
    for m in table {
        row(&mut rows, Some(m), &m.name);
    }
    for (w, net) in walls {
        let Some(t) = component_thicknesses(w.width, &w.components) else {
            continue;
        };
        let mut counted: Vec<(Option<String>, String)> = Vec::new();
        for (c, t) in w.components.iter().zip(t) {
            let Some((m, name)) = key_of(table, c.material_id.as_deref(), c.material.as_deref().filter(|t| !t.trim().is_empty()).or(Some(c.name.as_str())))
            else {
                continue;
            };
            let r = row(&mut rows, m, &name);
            let key = (r.id.clone(), r.name.clone());
            if !counted.contains(&key) {
                counted.push(key);
                r.walls += 1;
            }
            if let Some(a) = net {
                r.area += a;
                r.volume += a * t;
            }
        }
    }
    for (s, net, volume, summed) in slabs {
        let Some((m, name)) = key_of(table, s.material_id.as_deref(), s.material.as_deref()) else {
            continue;
        };
        let r = row(&mut rows, m, &name);
        r.slabs += 1;
        if *summed {
            r.area += net;
            r.volume += volume;
        }
    }
    rows.into_iter()
        .map(|a| {
            let mut r = a.row;
            r.area += 0.0;
            r.volume += 0.0;
            r.amount = match r.quantity {
                MaterialQuantity::Area => r.area,
                MaterialQuantity::Volume => r.volume,
            };
            r
        })
        .collect()
}

/// A category's label, for the Metraj table.
pub fn category_label(c: Option<MaterialCategory>) -> &'static str {
    c.map_or("Metin", MaterialCategory::label)
}
