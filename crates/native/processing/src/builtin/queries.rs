//! What the query tools share (docs/adr/0200; the web's `builtin/selectByLocation.ts`
//! and `builtin/attributeWrites.ts`): the kinds a query reads, the relations'
//! names, and how a value is written into an object's attribute: through the
//! object's layer's field of that name, when there is one, as the field's
//! canonical text (so a value already there is not written again); none takes
//! the attribute away. A value the field does not take is written as given,
//! and the runner's check refuses the run with the field's reason
//! (`crate::writes`).

use std::collections::BTreeMap;

use kentos_contracts::Entity;
use kentos_contracts::fields::{LayerFieldKind, check_value};
use kentos_geometry_core::ops::spatial_query::Relation;

use crate::types::RunContext;

/// Every kind a query reads: construction lines reach everywhere and stand in no relation.
pub const QUERY_KINDS: [&str; 15] = [
    "point",
    "line",
    "polyline",
    "polygon",
    "circle",
    "arc",
    "ellipse",
    "spline",
    "text",
    "dimension",
    "hatch",
    "insert",
    "leader",
    "table",
    "image",
];

/// The kinds that can enclose: a closed area, a circle, a whole ellipse, a closed curve, a hatch's region.
pub const AREA_KINDS: [&str; 5] = ["polygon", "circle", "ellipse", "spline", "hatch"];

pub fn kinds(list: &[&str]) -> Option<Vec<String>> {
    Some(list.iter().map(|k| (*k).to_owned()).collect())
}

/// A relation as the dialog and the summary name it.
pub fn relation_label(r: Relation) -> &'static str {
    match r {
        Relation::Intersects => "Kesişen",
        Relation::Contains => "İçeren",
        Relation::Within => "İçinde kalan",
        Relation::Disjoint => "Ayrık",
        Relation::Near => "Uzaklıkta",
        Relation::CenterIn => "Merkezi içinde",
    }
}

/// An attribute's value as the object holds it; none when it has none.
pub fn attr<'e>(e: &'e Entity, name: &str) -> Option<&'e str> {
    e.base().attrs.get(name).map(String::as_str)
}

/// The text a value becomes in this object's attribute.
pub fn canonical(ctx: &RunContext<'_>, e: &Entity, name: &str, value: &str) -> String {
    ctx.field(&e.base().layer_id, name)
        .and_then(|f| check_value(f, value).ok())
        .unwrap_or_else(|| value.to_owned())
}

/// The object's attributes with `name` set to `value` (none: taken away); none when nothing changes.
pub fn with_attr(
    ctx: &RunContext<'_>,
    e: &Entity,
    name: &str,
    value: Option<&str>,
) -> Option<BTreeMap<String, String>> {
    let attrs = &e.base().attrs;
    match value {
        None => {
            let mut next = attrs.clone();
            next.remove(name)?;
            Some(next)
        }
        Some(v) => {
            let text = canonical(ctx, e, name, v);
            if attrs.get(name) == Some(&text) {
                return None;
            }
            let mut next = attrs.clone();
            next.insert(name.to_owned(), text);
            Some(next)
        }
    }
}

/// The scale the mean is rounded to for this object's attribute: its field's
/// (whole numbers: 0), else the rule's own.
pub fn mean_scale(ctx: &RunContext<'_>, e: &Entity, name: &str) -> Option<u32> {
    let f = ctx.field(&e.base().layer_id, name)?;
    match f.kind {
        LayerFieldKind::Integer => Some(0),
        LayerFieldKind::Decimal => f.scale,
        _ => None,
    }
}
