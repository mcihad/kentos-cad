//! The objects' values a layer's expressions read, as one table (the web's
//! `exprTable`, `model/expression/expression.ts`; the layout is the style
//! core's `expr::rows`): per object the fields in the program's order, then
//! the label, the layer name and the kind label, each only when an
//! expression reads it, their lengths in UTF-16 code units (−1: none); and
//! the id and the vertex count as numbers, each only when read.
//!
//! The processing tools build the same table (`kentos-processing`); the two
//! are to meet in one place once both are on main.

use kentos_contracts::Entity;
use kentos_style_core::expr::Needs;

/// A layer build's value table (`ExprTable`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExprTable {
    pub texts: String,
    pub lens: Vec<i32>,
    pub numbers: Vec<f64>,
}

/// The kind of object as the interface names it (`ENTITY_KIND_LABEL`).
pub fn kind_label(e: &Entity) -> &'static str {
    match e {
        Entity::Point(_) => "Nokta",
        Entity::Line(_) => "Çizgi",
        Entity::Polyline(_) => "Çoklu çizgi",
        Entity::Polygon(_) => "Kapalı alan",
        Entity::Circle(_) => "Daire",
        Entity::Arc(_) => "Yay",
        Entity::Ellipse(_) => "Elips",
        Entity::Spline(_) => "Eğri",
        Entity::Xline(_) => "Yardımcı çizgi",
        Entity::Ray(_) => "Işın",
        Entity::Text(_) => "Yazı",
        Entity::Dimension(_) => "Ölçü",
        Entity::Hatch(_) => "Tarama",
    }
}

/// Corners of a path or area, holes included, every part of a multi-part
/// area (docs/adr/0143) (`vertexCount`, `$köşe`).
pub fn vertex_count(e: &Entity) -> Option<f64> {
    let ring = |pts: usize, holes: &Option<Vec<kentos_contracts::RingGeometry>>| {
        pts + holes.iter().flatten().map(|h| h.pts.len()).sum::<usize>()
    };
    let n = match e {
        Entity::Polyline(p) => p.pts.len(),
        Entity::Polygon(p) => {
            ring(p.pts.len(), &p.holes)
                + p.parts
                    .iter()
                    .flatten()
                    .map(|q| ring(q.pts.len(), &q.holes))
                    .sum::<usize>()
        }
        Entity::Line(_) => 2,
        Entity::Spline(s) => s.pts.len(),
        Entity::Point(_) => 1,
        _ => return None,
    };
    Some(n as f64)
}

/// The table of `list` for expressions reading `fields` and `needs`.
pub fn expr_table(
    fields: &[String],
    needs: Needs,
    list: &[&Entity],
    layer_name: &dyn Fn(&str) -> String,
) -> ExprTable {
    let per_object = fields.len()
        + [needs.label, needs.layer, needs.kind]
            .iter()
            .filter(|n| **n)
            .count();
    let numbers_per_object = [needs.id, needs.vertices].iter().filter(|n| **n).count();
    let mut out = ExprTable {
        texts: String::new(),
        lens: Vec::with_capacity(list.len() * per_object),
        numbers: Vec::with_capacity(list.len() * numbers_per_object),
    };
    fn put(t: &mut ExprTable, v: Option<&str>) {
        match v {
            None => t.lens.push(-1),
            Some(v) => {
                t.texts.push_str(v);
                t.lens.push(v.encode_utf16().count() as i32);
            }
        }
    }
    for e in list {
        let base = e.base();
        for f in fields {
            put(&mut out, base.attrs.get(f).map(String::as_str));
        }
        if needs.label {
            put(&mut out, base.label.as_deref());
        }
        if needs.layer {
            let name = layer_name(&base.layer_id);
            put(&mut out, Some(&name));
        }
        if needs.kind {
            put(&mut out, Some(kind_label(e)));
        }
        if needs.id {
            out.numbers.push(f64::from(base.id));
        }
        if needs.vertices {
            out.numbers.push(vertex_count(e).unwrap_or(f64::NAN));
        }
    }
    out
}
