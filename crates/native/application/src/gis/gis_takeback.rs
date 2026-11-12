//! `cad.gis.takeback` v1 (docs/adr/0634 §1): what a GIS changed in a derived dataset written back to the drawing, the
//! source. Only a row `cad.gis.derivation` calls `edited` is taken: its source is as it was when exported (its digest
//! the row's) and the row is not what the derivation gives. Whatever else is refused: a row whose source changed too
//! (`row_stale`: both sides changed, the GIS's does not go over the drawing's), one without its source
//! (`row_source_gone`), one the GIS did not change (`row_not_edited`).
//!
//! - **Attributes** (the default): the columns that differ go to the source's attributes by the derivation's names
//!   (a renamed column to its source's), each read by the column's type and checked by the layer's field; a text's
//!   words, height and turn, a block insertion's scale and turn. The geometry stays; a row whose geometry changed
//!   says so (`geometry_left`).
//! - **Geometry**: the columns and the geometry. A source of straight geometry takes the GIS's exactly (a point, a
//!   line, a polyline without bulges, an area), its Z and M with it; one a curve stood for (a circle, an arc, a
//!   bulged edge, an ellipse, a curve, a NURBS) becomes the chords: its analytic definition is left and its link to it
//!   broken (`link_broken`, `broken` the kind it was). A text and a block insertion take the point. The GIS geometry
//!   must be of the source's dimension (`geometry_kind_mismatch`).
//!
//! Everything is one undo step “GIS'ten geri al”; afterwards the rows are `stale` (their sources changed): export again.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use kentos_commands::{
    CommandResult, CommandWarning, Entity, GisAnnotations, GisTableRole, GisTakeback, GisTaken,
    MOST_TAKEBACK_ROWS, TakebackPart, TakenRow,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geopackage::{self as gpkg, Coord, Curve, Geometry};
use serde_json::{Value as J, json};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::gis_derive::{self, Options, Outcome};

/// The undo step's name.
pub const LABEL: &str = "GIS'ten geri al";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// A source's dimension: what kind of GIS geometry stands for it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Dim {
    Point,
    Line,
    Area,
    None,
}

fn dim_of(e: &Entity) -> Dim {
    match e {
        Entity::Point(_) | Entity::Text(_) | Entity::Insert(_) => Dim::Point,
        Entity::Line(_) | Entity::Arc(_) => Dim::Line,
        Entity::Polyline(_) => Dim::Line,
        Entity::Polygon(_) | Entity::Circle(_) => Dim::Area,
        Entity::Ellipse(_) | Entity::Spline(_) | Entity::Nurbs(_) => Dim::Line,
        _ => Dim::None,
    }
}

fn dim_of_geometry(g: &Geometry, closed_curve: bool) -> Dim {
    match g {
        Geometry::Point(_) | Geometry::MultiPoint(_) => Dim::Point,
        Geometry::Curve(_) | Geometry::MultiCurve(_) => Dim::Line,
        Geometry::Polygon(_) | Geometry::MultiPolygon(_) => {
            if closed_curve {
                Dim::Line
            } else {
                Dim::Area
            }
        }
        Geometry::Collection(_) => Dim::None,
    }
}

/// Whether a source's geometry is a curve the derivation made chords of: taking the GIS's breaks its link.
fn curved(e: &Entity) -> bool {
    match e {
        Entity::Circle(_)
        | Entity::Arc(_)
        | Entity::Ellipse(_)
        | Entity::Spline(_)
        | Entity::Nurbs(_) => true,
        Entity::Polyline(p) => {
            p.bulges
                .as_ref()
                .is_some_and(|b| b.iter().any(|v| *v != 0.0))
                || p.parts.iter().flatten().any(|q| {
                    q.bulges
                        .as_ref()
                        .is_some_and(|b| b.iter().any(|v| *v != 0.0))
                })
        }
        Entity::Polygon(p) => {
            let bulged =
                |b: &Option<Vec<f64>>| b.as_ref().is_some_and(|b| b.iter().any(|v| *v != 0.0));
            bulged(&p.bulges)
                || p.holes.iter().flatten().any(|h| bulged(&h.bulges))
                || p.parts.iter().flatten().any(|q| bulged(&q.bulges))
        }
        _ => false,
    }
}

fn xy(c: &Coord) -> J {
    json!({ "x": c.x, "y": c.y })
}

/// The Z (or M) values of `coords`, none when none has one.
fn values(coords: &[Coord], m: bool) -> Option<J> {
    let vals: Vec<Option<f64>> = coords
        .iter()
        .map(|c| if m { c.m } else { c.z }.filter(|v| v.is_finite()))
        .collect();
    vals.iter().any(Option::is_some).then(|| json!(vals))
}

fn line_coords(c: &Curve) -> Vec<Coord> {
    match c {
        Curve::Line(p) | Curve::Arc(p) => p.clone(),
        Curve::Compound(parts) => parts.iter().flat_map(line_coords).collect(),
    }
}

/// A ring's coordinates without its closing point.
fn ring(c: &Curve) -> Vec<Coord> {
    let mut pts = line_coords(c);
    if pts.len() > 1 && pts.first().map(|p| (p.x, p.y)) == pts.last().map(|p| (p.x, p.y)) {
        pts.pop();
    }
    pts
}

fn area_part(rings: &[Curve]) -> J {
    let outer = rings.first().map(ring).unwrap_or_default();
    let mut part = json!({ "pts": outer.iter().map(xy).collect::<Vec<_>>() });
    if let Some(z) = values(&outer, false) {
        part["zs"] = z;
    }
    let holes: Vec<J> = rings[1.min(rings.len())..]
        .iter()
        .map(|h| {
            let pts = ring(h);
            let mut hole = json!({ "pts": pts.iter().map(xy).collect::<Vec<_>>() });
            if let Some(z) = values(&pts, false) {
                hole["zs"] = z;
            }
            hole
        })
        .collect();
    if !holes.is_empty() {
        part["holes"] = json!(holes);
    }
    part
}

fn line_part(c: &Curve) -> J {
    let pts = line_coords(c);
    let mut part = json!({ "pts": pts.iter().map(xy).collect::<Vec<_>>() });
    if let Some(z) = values(&pts, false) {
        part["zs"] = z;
    }
    if let Some(m) = values(&pts, true) {
        part["ms"] = m;
    }
    part
}

/// The source `e` with the GIS geometry `g`: its base (layer, attributes, look) kept, its geometry the GIS's.
fn with_geometry(e: &Entity, g: &Geometry) -> Option<Entity> {
    let mut v = serde_json::to_value(e.base()).ok()?;
    let J::Object(m) = &mut v else {
        return None;
    };
    let mut put = |key: &str, val: J| {
        m.insert(key.to_owned(), val);
    };
    match (e, g) {
        (Entity::Text(_) | Entity::Insert(_), Geometry::Point(Some(c))) => {
            let mut w = serde_json::to_value(e).ok()?;
            w["p"] = xy(c);
            return serde_json::from_value(w).ok();
        }
        (_, Geometry::Point(Some(c))) => {
            put("kind", json!("point"));
            put("p", xy(c));
            if let Some(z) = c.z.filter(|v| v.is_finite()) {
                put("z", json!(z));
            }
        }
        (_, Geometry::MultiPoint(all)) if !all.is_empty() => {
            put("kind", json!("point"));
            put("p", xy(&all[0]));
            if let Some(z) = all[0].z.filter(|v| v.is_finite()) {
                put("z", json!(z));
            }
            if all.len() > 1 {
                let parts: Vec<J> = all[1..]
                    .iter()
                    .map(|c| {
                        let mut p = json!({ "p": xy(c) });
                        if let Some(z) = c.z.filter(|v| v.is_finite()) {
                            p["z"] = json!(z);
                        }
                        p
                    })
                    .collect();
                put("parts", json!(parts));
            }
        }
        (Entity::Line(_), Geometry::Curve(c)) if line_coords(c).len() == 2 => {
            let pts = line_coords(c);
            put("kind", json!("line"));
            put("a", xy(&pts[0]));
            put("b", xy(&pts[1]));
            for (k, v) in [
                ("za", pts[0].z),
                ("zb", pts[1].z),
                ("ma", pts[0].m),
                ("mb", pts[1].m),
            ] {
                if let Some(v) = v.filter(|v| v.is_finite()) {
                    put(k, json!(v));
                }
            }
        }
        (_, Geometry::Curve(c)) => {
            let mut pts = line_coords(c);
            let closed =
                pts.len() > 2 && pts.first().map(|p| (p.x, p.y)) == pts.last().map(|p| (p.x, p.y));
            if closed {
                pts.pop();
            }
            put("kind", json!("polyline"));
            let part = line_part(&Curve::Line(pts));
            for (k, val) in part.as_object()?.clone() {
                put(&k, val);
            }
            if closed {
                put("closed", json!(true));
            }
        }
        (_, Geometry::MultiCurve(all)) if !all.is_empty() => {
            put("kind", json!("polyline"));
            for (k, val) in line_part(&all[0]).as_object()?.clone() {
                put(&k, val);
            }
            if all.len() > 1 {
                put(
                    "parts",
                    json!(all[1..].iter().map(line_part).collect::<Vec<_>>()),
                );
            }
        }
        (_, Geometry::Polygon(rings)) if !rings.is_empty() => {
            put("kind", json!("polygon"));
            for (k, val) in area_part(rings).as_object()?.clone() {
                put(&k, val);
            }
        }
        (_, Geometry::MultiPolygon(all)) if !all.is_empty() => {
            put("kind", json!("polygon"));
            for (k, val) in area_part(&all[0]).as_object()?.clone() {
                put(&k, val);
            }
            if all.len() > 1 {
                put(
                    "parts",
                    json!(all[1..].iter().map(|r| area_part(r)).collect::<Vec<_>>()),
                );
            }
        }
        _ => return None,
    }
    serde_json::from_value(v).ok()
}

/// A derived table read once: its layer, record, rows by lasting id.
struct Table<'a> {
    layer: &'a gpkg::Layer,
    record: &'a kentos_commands::DerivationRecord,
    rows: HashMap<String, &'a gpkg::Row>,
}

fn table_of(t: &crate::gis_set::SetTable) -> Table<'_> {
    let uid_col = gis_derive::column_of(&t.layer, "kentos_uid");
    let rows = t
        .rows
        .iter()
        .map(|row| {
            let uid = t
                .uids
                .get(&row.fid)
                .cloned()
                .or_else(|| gis_derive::text_at(row, uid_col))
                .unwrap_or_default();
            (uid, row)
        })
        .collect();
    Table {
        layer: &t.layer,
        record: &t.record,
        rows,
    }
}

/// What one row gives: the source's slot, the entity it becomes, what was taken, the warnings it raises.
struct Taking {
    slot: Slot,
    entity: Entity,
    row: TakenRow,
    geometry_left: bool,
    /// Computed columns (an architectural object's or a dimension's values) that changed and were not taken.
    fields_left: Vec<String>,
    /// Whether the source changes: a row whose geometry alone changed, its attributes taken, changes nothing.
    changes: bool,
}

fn take_row(
    doc: &Document,
    table: &Table<'_>,
    asked: &kentos_commands::TakebackRow,
    at: &str,
) -> Result<Taking, Stop> {
    let missing = || {
        fail(
            codes::DERIVATION_ROW_NOT_FOUND,
            format!(
                "“{}” tablosunda “{}” kimlikli satır yok.",
                asked.table, asked.uid
            ),
            &format!("{at}.uid"),
        )
    };
    let row = *table.rows.get(&asked.uid).ok_or_else(missing)?;
    let (slot, e) = Uuid::parse_str(&asked.uid)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|s| doc.get(s).map(|e| (s, e)))
        .ok_or_else(|| {
            fail(
                codes::ROW_SOURCE_GONE,
                format!(
                    "“{}” satırının kaynak nesnesi çizimde yok; geri yazılacak nesne yok.",
                    asked.uid
                ),
                at,
            )
        })?;
    let digest_col = gis_derive::column_of(table.layer, "kentos_digest");
    if gis_derive::text_at(row, digest_col).as_deref() != Some(gis_derive::digest(doc, e).as_str())
    {
        return Err(fail(
            codes::ROW_STALE,
            format!(
                "“{}” satırının kaynağı dışa aktarmadan sonra çizimde değişti: GIS'teki değişiklik onun üstüne yazılmaz. Yeniden dışa aktarıp GIS'teki değişikliği yeni türevde yapın.",
                asked.uid
            ),
            at,
        ));
    }
    let rec = table.record;
    let opts = Options {
        tolerance: rec.tolerance,
        annotations: GisAnnotations::Table,
    };
    let expected = match gis_derive::derive_entity(doc, e, &opts) {
        Outcome::Row(x)
            if x.role == rec.role
                && (rec.role != GisTableRole::Layer
                    || rec.layer.as_ref().is_some_and(|l| l.id == x.key)) =>
        {
            x
        }
        _ => {
            return Err(fail(
                codes::ROW_STALE,
                format!(
                    "“{}” satırının kaynağı artık bu tablonun satırı olarak türetilemiyor.",
                    asked.uid
                ),
                at,
            ));
        }
    };
    let fields = gis_derive::differences(table.layer, row, &expected, e, &rec.renamed);
    if fields.is_empty() {
        return Err(fail(
            codes::ROW_NOT_EDITED,
            format!("“{}” satırı GIS'te değişmemiş; kaynakla aynı.", asked.uid),
            at,
        ));
    }
    let geometry_changed = fields.iter().any(|f| f == "geometry");
    let take_geometry = geometry_changed && asked.take == TakebackPart::Geometry;
    // An architectural object and a dimension are drawn from their values (docs/adr/0634 §4).
    let parametric = matches!(rec.role, GisTableRole::Arch | GisTableRole::Dimensions);
    if parametric && take_geometry {
        return Err(fail(
            codes::GEOMETRY_PARAMETRIC,
            format!(
                "“{}” satırının kaynağı bir {}: geometrisi kendi değerlerinden çizilir, GIS'teki geometrisi alınmaz. Yalnız öznitelikleri alın (take: attributes).",
                asked.uid,
                e.kind()
            ),
            at,
        ));
    }
    if doc.layers().is_locked(&e.base().layer_id) {
        return Err(fail(
            codes::LAYER_LOCKED,
            format!(
                "“{}” satırının kaynağı kilitli bir katmanda; kilidi açıp yeniden deneyin.",
                asked.uid
            ),
            at,
        ));
    }
    // The source with the GIS geometry, when asked.
    let mut entity = e.clone();
    let mut broken = None;
    if take_geometry {
        let g = match &row.geometry {
            Some(Ok(g)) => g.clone(),
            _ => {
                return Err(fail(
                    codes::GEOMETRY_KIND_MISMATCH,
                    format!("“{}” satırının geometrisi okunamadı ya da yok.", asked.uid),
                    at,
                ));
            }
        };
        let closed_curve = matches!(e, Entity::Polyline(p) if p.closed)
            || matches!(e, Entity::Spline(s) if s.closed)
            || matches!(e, Entity::Nurbs(n) if n.closed)
            || matches!(e, Entity::Ellipse(_)) && matches!(g, Geometry::Polygon(_));
        let want = dim_of(e);
        let have = dim_of_geometry(&g, closed_curve && want == Dim::Line);
        let fits = want == have
            || (want == Dim::Line && have == Dim::Area && closed_curve)
            || (matches!(e, Entity::Ellipse(_)) && have == Dim::Area);
        if !fits {
            return Err(fail(
                codes::GEOMETRY_KIND_MISMATCH,
                format!(
                    "“{}” satırının GIS'teki geometrisi kaynağın türüne ({}) uymuyor.",
                    asked.uid,
                    e.kind()
                ),
                at,
            ));
        }
        entity = with_geometry(e, &g).ok_or_else(|| {
            fail(
                codes::GEOMETRY_KIND_MISMATCH,
                format!(
                    "“{}” satırının GIS'teki geometrisi kaynağın türüne ({}) uymuyor.",
                    asked.uid,
                    e.kind()
                ),
                at,
            )
        })?;
        if curved(e) {
            broken = Some(e.kind().to_owned());
        }
    }
    // The columns that differ, to the source's attributes or parts.
    let mut taken: Vec<String> = Vec::new();
    let mut fields_left: Vec<String> = Vec::new();
    let mut attrs: BTreeMap<String, Option<String>> = BTreeMap::new();
    for name in fields.iter().filter(|f| *f != "geometry") {
        let Some(i) = gis_derive::column_of(table.layer, name) else {
            continue;
        };
        let kind = &table.layer.columns[i].kind;
        let value = row
            .values
            .get(i)
            .and_then(|v| crate::geopackage::cell_text(v, kind));
        let source = rec
            .renamed
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.clone());
        if parametric && expected.extras.iter().any(|(k, _)| *k == source) {
            fields_left.push(source);
            continue;
        }
        let number = || value.as_deref().and_then(|t| t.parse::<f64>().ok());
        match (&mut entity, source.as_str()) {
            (Entity::Text(t), "text") => t.text = value.clone().unwrap_or_default(),
            (Entity::Text(t), "height") => {
                if let Some(h) = number().filter(|h| *h > 0.0) {
                    t.height = h;
                }
            }
            (Entity::Text(t), "rotation_deg") => {
                if let Some(r) = number() {
                    t.rotation = r;
                }
            }
            (Entity::Insert(b), "scale") => {
                if let Some(s) = number().filter(|s| *s != 0.0) {
                    b.scale = s;
                }
            }
            (Entity::Insert(b), "rotation_deg") => {
                if let Some(r) = number() {
                    b.rotation = r.to_radians();
                }
            }
            _ => {
                attrs.insert(source.clone(), value);
            }
        }
        taken.push(source);
    }
    if !attrs.is_empty() {
        let checked =
            checks::field_values(doc, &e.base().layer_id, &attrs, &format!("{at}.attrs"))?;
        let base = entity.base_mut();
        for (k, v) in checked {
            match v {
                Some(v) => {
                    base.attrs.insert(k, v);
                }
                None => {
                    base.attrs.remove(&k);
                }
            }
        }
    }
    let changes = entity != *e;
    Ok(Taking {
        slot,
        entity,
        changes,
        fields_left,
        geometry_left: geometry_changed && !take_geometry,
        row: TakenRow {
            table: asked.table.clone(),
            uid: asked.uid.clone(),
            fields: taken,
            geometry: take_geometry,
            broken,
        },
    })
}

/// The rows' takings, or the first refusal.
fn takings(doc: &Document, input: &GisTakeback) -> Result<(String, Vec<Taking>), Stop> {
    crate::gis_derivation::file_or_database(&input.path)?;
    let set = crate::gis_set::open_any(&input.path)
        .map_err(|(code, message)| fail(code, message, "path"))?;
    if input.rows.is_empty() || input.rows.len() > MOST_TAKEBACK_ROWS {
        return Err(fail(
            codes::INVALID_COUNT,
            format!(
                "Geri alınacak satırlar 1 ile {MOST_TAKEBACK_ROWS} arasında olur; {} verildi.",
                input.rows.len()
            ),
            "rows",
        ));
    }
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for (i, r) in input.rows.iter().enumerate() {
        if !seen.insert(r.uid.as_str()) {
            return Err(fail(
                codes::INVALID_COUNT,
                format!("“{}” satırı iki kez verildi.", r.uid),
                &format!("rows.{i}"),
            ));
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let tables: HashMap<&str, Table<'_>> = set
        .tables
        .iter()
        .map(|t| (t.layer.name.as_str(), table_of(t)))
        .collect();
    let mut out = Vec::with_capacity(input.rows.len());
    for (i, r) in input.rows.iter().enumerate() {
        let at = format!("rows.{i}");
        let Some(table) = tables.get(r.table.as_str()) else {
            return Err(fail(
                codes::DERIVATION_ROW_NOT_FOUND,
                format!("Dosyada “{}” adlı türev tablo yok.", r.table),
                &format!("{at}.table"),
            ));
        };
        out.push(take_row(doc, table, r, &at)?);
    }
    Ok((set.file.clone(), out))
}

fn warnings_of(takings: &[Taking]) -> Vec<CommandWarning> {
    let mut out = Vec::new();
    let broken: Vec<String> = takings
        .iter()
        .filter_map(|t| {
            t.row
                .broken
                .as_ref()
                .map(|k| format!("{} ({k})", t.row.uid))
        })
        .collect();
    if !broken.is_empty() {
        out.push(CommandWarning {
            code: codes::LINK_BROKEN.into(),
            message: format!(
                "{} nesnenin kaynağı bir eğriydi; GIS'teki kirişleri aldı ve analitik tanımı bırakıldı: {}.",
                broken.len(),
                broken.join(", ")
            ),
            path: Some("rows".into()),
        });
    }
    let computed: Vec<String> = takings
        .iter()
        .filter(|t| !t.fields_left.is_empty())
        .map(|t| format!("{} ({})", t.row.uid, t.fields_left.join(", ")))
        .collect();
    if !computed.is_empty() {
        out.push(CommandWarning {
            code: codes::FIELDS_LEFT.into(),
            message: format!(
                "{} satırın GIS'te değişen hesaplanmış sütunları kaynağa yazılmadı (mimari nesnenin ve ölçünün değerleri çizimde değiştirilir): {}.",
                computed.len(),
                computed.join("; ")
            ),
            path: Some("rows".into()),
        });
    }
    let left = takings.iter().filter(|t| t.geometry_left).count();
    if left > 0 {
        out.push(CommandWarning {
            code: codes::GEOMETRY_LEFT.into(),
            message: format!(
                "{left} satırın geometrisi GIS'te değişmiş; yalnız öznitelikleri alındı. Geometriyi almak için take: geometry verin."
            ),
            path: Some("rows".into()),
        });
    }
    out
}

fn answer(name: String, takings: &[Taking], revision: String) -> GisTaken {
    GisTaken {
        file: name,
        changed: takings.iter().filter(|t| t.changes).count() as u32,
        rows: takings.iter().map(|t| t.row.clone()).collect(),
        revision,
    }
}

/// Checks `input` against the document and the file, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &GisTakeback) -> CommandResult<()> {
    match takings(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would take; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &GisTakeback) -> CommandResult<GisTaken> {
    match takings(cx.doc, input) {
        Ok((name, t)) => CommandResult::Completed {
            warnings: warnings_of(&t),
            output: answer(name, &t, cx.doc.revision().to_string()),
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the rows back as one undo step “GIS'ten geri al”.
pub fn execute(cx: &mut ExecutionContext<'_>, input: GisTakeback) -> CommandResult<GisTaken> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_GIS_TAKEBACK,
        kentos_commands::CAD_GIS_TAKEBACK_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: GisTakeback) -> CommandResult<GisTaken> {
    let (name, t) = match takings(cx.doc, &input) {
        Ok(x) => x,
        Err(stop) => return stop.into(),
    };
    let warnings = warnings_of(&t);
    let changes: Vec<(Slot, Entity)> = t
        .iter()
        .filter(|x| x.changes)
        .map(|x| (x.slot, x.entity.clone()))
        .collect();
    if !changes.is_empty() {
        cx.doc.update_many(changes, LABEL);
    }
    CommandResult::Completed {
        output: answer(name, &t, cx.doc.revision().to_string()),
        warnings,
    }
}
