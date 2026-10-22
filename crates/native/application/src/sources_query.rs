//! `cad.sources.query` v1 (docs/adr/0581): a question asked of a data set where it is. A GeoPackage table or a PostGIS
//! table is read a part at a time in key order, with the box, the columns and the part of the filter the source can
//! check sent to it (`kentos_expression::pushdown`); a layer of the drawing is read from its objects in lasting-id
//! order. Every record that comes is made the drawing's object (in the project's system) and checked here with the
//! box, the place and the filter, so every source gives the answer the drawing's objects would. The page, the count
//! and the statistics are cut from what passes; the plan says what the source did. The shared cases are
//! `fixtures/commands/v1/cad.sources.query.json`.

use std::collections::BTreeMap;
use std::path::Path;

use kentos_commands::{
    CommandResult, CommandWarning, Entity, LayerFieldKind, LayerFilter, MOST_ORDERED_HERE,
    MOST_QUERIED, PlanBox, ProjectSettings, QUERIED, QueriedKind, QueriedRow, QueriedSource,
    QueriedStatistic, QueryOrder, QueryPlan, QueryScope, SourceKind, SourcesQueried, SourcesQuery,
};
use kentos_domain::{Document, Uuid};
use kentos_expression::pushdown::{self, ColumnSort, Dialect, Param, Pushed};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::spatial_query;
use kentos_geometry_core::ops::topology_rules::incremental::shape_bounds;
use kentos_geopackage::{self as gpkg, ColumnKind, GeoPackage};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::geopackage as gp;
use crate::layer_filter::{CompiledFilter, compile_filter};
use crate::provider::ProviderScan;
use crate::sources::{self, At, Mover, Origin};

/// How many records a source hands over at a time.
const CHUNK: usize = 1000;

fn fail(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.to_owned())))
}

/// Where a record stands: a source's key, or a drawing object's lasting id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Place {
    Key(i64),
    Uid(Uuid),
}

/// Where a page ends: a source's key, a drawing object's lasting id, or in an order by a field the last record's
/// place and its value there (docs/adr/0591 §3: `o:[17,"Ada"]`).
#[derive(Clone, Debug, PartialEq)]
enum Cursor {
    Key(i64),
    Uid(Uuid),
    After(Place, serde_json::Value),
}

impl Cursor {
    fn read(text: &str) -> Option<Cursor> {
        if let Some(k) = text.strip_prefix("k:") {
            let n: i64 = k.parse().ok()?;
            return (n.to_string() == k).then_some(Cursor::Key(n));
        }
        if let Some(rest) = text.strip_prefix("o:") {
            let v: serde_json::Value = serde_json::from_str(rest).ok()?;
            let [place, value] = v.as_array()?.as_slice() else {
                return None;
            };
            let place = match place {
                serde_json::Value::Number(n) => Place::Key(n.as_i64()?),
                serde_json::Value::String(u) => Place::Uid(Uuid::parse_str(u).ok()?),
                _ => return None,
            };
            if !matches!(
                value,
                serde_json::Value::Null
                    | serde_json::Value::Number(_)
                    | serde_json::Value::String(_)
            ) {
                return None;
            }
            let c = Cursor::After(place, value.clone());
            return (c.text() == text).then_some(c);
        }
        let u = Uuid::parse_str(text.strip_prefix("u:")?).ok()?;
        (format!("u:{u}") == text).then_some(Cursor::Uid(u))
    }

    fn text(&self) -> String {
        match self {
            Cursor::Key(k) => format!("k:{k}"),
            Cursor::Uid(u) => format!("u:{u}"),
            Cursor::After(place, value) => {
                let place = match place {
                    Place::Key(k) => serde_json::json!(k),
                    Place::Uid(u) => serde_json::json!(u.to_string()),
                };
                format!("o:{}", serde_json::json!([place, value]))
            }
        }
    }
}

/// A source's raw value as a cursor keeps it, and back.
fn json_of(v: &gpkg::Value) -> serde_json::Value {
    match v {
        gpkg::Value::Null | gpkg::Value::Blob(_) => serde_json::Value::Null,
        gpkg::Value::Integer(i) => serde_json::json!(i),
        gpkg::Value::Real(x) => serde_json::Number::from_f64(*x)
            .map_or(serde_json::Value::Null, serde_json::Value::Number),
        gpkg::Value::Text(t) => serde_json::json!(t),
    }
}

fn value_of(v: &serde_json::Value) -> gpkg::Value {
    match v {
        serde_json::Value::Number(n) => match n.as_i64() {
            Some(i) => gpkg::Value::Integer(i),
            None => gpkg::Value::Real(n.as_f64().unwrap_or(0.0)),
        },
        serde_json::Value::String(t) => gpkg::Value::Text(t.clone()),
        _ => gpkg::Value::Null,
    }
}

/// A record's value in a field, as the order here reads it: none when it has none (or an empty one).
fn here_value(e: &Entity, field: &str) -> Option<String> {
    e.base().attrs.get(field).filter(|v| !v.is_empty()).cloned()
}

/// The records ordered here (docs/adr/0591 §2), from where the cursor stands: by the attribute table's rule for the
/// field's kind (`number`, `date`, `boolean`, `text`), a record without a value after every one with, equal ones by
/// their place (the list comes in that order). The cursor's record is put back where it stood, so a page goes on
/// after it even when the record itself is gone.
fn order_here(
    list: Vec<(Uuid, Option<i64>, Entity)>,
    field: &str,
    kind: &str,
    descending: bool,
    cursor: Option<&Cursor>,
) -> Vec<(Uuid, Option<i64>, Entity)> {
    use kentos_geometry_core::ops::feature_table::{Cell, Column, Query, Row, feature_table};
    let row = |v: Option<String>| Row {
        cells: vec![Cell {
            shown: v.clone().unwrap_or_default(),
            key: v,
        }],
        selected: false,
        in_view: true,
        passes: true,
    };
    let mut rows: Vec<Row> = list
        .iter()
        .map(|(_, _, e)| row(here_value(e, field)))
        .collect();
    // The cursor's record goes after every record whose place is not past its own.
    let at = match cursor {
        Some(Cursor::After(place, value)) => {
            let at = list.partition_point(|(u, k, _)| match (place, k) {
                (Place::Key(c), Some(k)) => k <= c,
                (Place::Uid(c), None) => u <= c,
                _ => false,
            });
            let value = match value {
                serde_json::Value::Null => None,
                serde_json::Value::String(t) if t.is_empty() => None,
                serde_json::Value::String(t) => Some(t.clone()),
                other => Some(other.to_string()),
            };
            rows.insert(at, row(value));
            Some(at)
        }
        _ => None,
    };
    let order = feature_table(
        &[Column {
            order: kind.to_owned(),
            searched: false,
        }],
        &rows,
        &Query {
            search: String::new(),
            show: "all".into(),
            sort: Some(0),
            descending,
        },
    );
    let start = match at {
        None => 0,
        Some(at) => order
            .iter()
            .position(|&i| i as usize == at)
            .map_or(0, |p| p + 1),
    };
    let mut taken: Vec<Option<(Uuid, Option<i64>, Entity)>> = list.into_iter().map(Some).collect();
    order[start..]
        .iter()
        .filter_map(|&i| {
            let i = i as usize;
            let i = match at {
                Some(at) if i == at => return None,
                Some(at) if i > at => i - 1,
                _ => i,
            };
            taken.get_mut(i).and_then(Option::take)
        })
        .collect()
}

/// The kind of order a layer's field takes (its type), `text` without one.
fn field_order(doc: &Document, layer: &str, field: &str) -> &'static str {
    let kind = doc
        .layers()
        .get(layer)
        .and_then(|n| n.fields.iter().find(|f| f.name == field))
        .map(|f| f.kind);
    match kind {
        Some(LayerFieldKind::Integer | LayerFieldKind::Decimal) => "number",
        Some(LayerFieldKind::Date | LayerFieldKind::DateTime | LayerFieldKind::Time) => "date",
        Some(LayerFieldKind::Boolean) => "boolean",
        _ => "text",
    }
}

/// A question's input, checked.
struct Asked {
    limit: usize,
    bbox: Option<[f64; 4]>,
    near: Option<(Vec2, f64)>,
    filter: Option<CompiledFilter>,
    fields: Option<Vec<String>>,
    geometry: bool,
    count: bool,
    statistics: Vec<String>,
    cursor: Option<Cursor>,
    order: Option<QueryOrder>,
    expected_stamp: Option<String>,
}

impl Asked {
    /// Whether the filter reads the objects' geometry or kind (a source must hand over the geometry for it).
    fn filter_reads_shape(&self) -> bool {
        self.filter
            .as_ref()
            .is_some_and(|f| f.reads_geometry() || f.expr().is_some_and(|e| e.needs.kind))
    }

    /// Whether an object's shape is asked here.
    fn needs_shape(&self) -> bool {
        self.bbox.is_some() || self.near.is_some() || self.filter_reads_shape()
    }

    fn local(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.filter.is_some() {
            out.push("filter".to_owned());
        }
        if self.bbox.is_some() {
            out.push("box".to_owned());
        }
        if self.near.is_some() {
            out.push("near".to_owned());
        }
        if !self.statistics.is_empty() {
            out.push("statistics".to_owned());
        }
        out
    }
}

fn too_many_to_order() -> Stop {
    fail(
        codes::ORDER_TOO_LARGE,
        format!(
            "Bu alan burada sıralanır ve soru {MOST_ORDERED_HERE} kayıttan çoğunu tutuyor; süzgeçle, kutuyla ya da kaynağın sıraladığı bir alanla (sayı, tarih, evet/hayır) sorun."
        ),
        "order",
    )
}

fn finite_box(b: &[f64; 4]) -> bool {
    b.iter().all(|v| v.is_finite()) && b[0] <= b[2] && b[1] <= b[3]
}

/// The input's own checks, before anything is read: what is asked of, the numbers, the cursor and the filter.
fn asked(input: &SourcesQuery, settings: &ProjectSettings) -> Result<Asked, Stop> {
    match (&input.layer, &input.path, &input.table) {
        (Some(_), None, None) | (None, Some(_), Some(_)) => {}
        (Some(_), Some(_), _) | (None, None, None) => {
            return Err(fail(
                codes::INVALID_INPUT,
                "Bir katman (layer) ya da bir dosya veya adres ve tablo (path, table) verin; ikisi birden ya da hiçbiri olmaz.",
                "layer",
            ));
        }
        (_, None, Some(_)) => {
            return Err(fail(
                codes::INVALID_INPUT,
                "Tablo (table) bir dosya ya da adresle (path) verilir.",
                "table",
            ));
        }
        (None, Some(_), None) => {
            return Err(fail(
                codes::INVALID_INPUT,
                "Dosyanın ya da veritabanının tablosunu (table) verin.",
                "table",
            ));
        }
    }
    if input.scope.is_some() && input.layer.is_none() {
        return Err(fail(
            codes::INVALID_INPUT,
            "Kapsam (scope) bir katmanla (layer) verilir.",
            "scope",
        ));
    }
    let limit = input.limit.unwrap_or(QUERIED);
    if limit > MOST_QUERIED {
        return Err(fail(
            codes::INVALID_COUNT,
            format!("Bir sayfada en çok {MOST_QUERIED} kayıt okunur; {limit} verildi."),
            "limit",
        ));
    }
    if let Some(b) = &input.bbox
        && !finite_box(b)
    {
        return Err(fail(
            codes::INVALID_EXTENT,
            "Kutu [batı, güney, doğu, kuzey] olmalı: sonlu sayılar, batı doğudan, güney kuzeyden büyük değil.",
            "bbox",
        ));
    }
    let near = match &input.near {
        None => None,
        Some(n) => {
            if !(n.tolerance.is_finite() && n.tolerance >= 0.0) {
                return Err(fail(
                    codes::INVALID_TOLERANCE,
                    "Tolerans sıfır ya da sıfırdan büyük, sonlu bir uzaklık olmalı.",
                    "near.tolerance",
                ));
            }
            if !n.at.iter().all(|v| v.is_finite()) {
                return Err(fail(
                    codes::INVALID_EXTENT,
                    "Yer (near.at) sonlu iki sayı olmalı: doğu, kuzey.",
                    "near.at",
                ));
            }
            Some((Vec2::new(n.at[0], n.at[1]), n.tolerance))
        }
    };
    let cursor = match &input.cursor {
        None => None,
        Some(c) => Some(Cursor::read(c).ok_or_else(|| {
            fail(
                codes::INVALID_CURSOR,
                format!("“{c}” bu sorunun sayfa imleci değil; bir önceki cevabın next'ini verin."),
                "cursor",
            )
        })?),
    };
    match (&cursor, &input.order) {
        (Some(Cursor::After(..)), None) => {
            return Err(fail(
                codes::INVALID_CURSOR,
                "Bu imleç bir sıralamanın; sırayı (order) ilk sayfadaki gibi verin.",
                "cursor",
            ));
        }
        (Some(Cursor::Key(_) | Cursor::Uid(_)), Some(_)) => {
            return Err(fail(
                codes::INVALID_CURSOR,
                "Bu imleç sırasız bir sorunun; sıralı sorunun sayfaları bir önceki sıralı cevabın next'iyle sürer.",
                "cursor",
            ));
        }
        _ => {}
    }
    let filter = match input.filter.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(text) => {
            let unit = crate::expression_unit::reading_unit(settings);
            let f = compile_filter(
                &LayerFilter {
                    expression: Some(text.to_owned()),
                    ..LayerFilter::default()
                },
                unit,
            )
            .map_err(|why| {
                fail(
                    codes::INVALID_EXPRESSION,
                    format!("Süzgeç: {why}"),
                    "filter",
                )
            })?;
            Some(f)
        }
    };
    Ok(Asked {
        limit: limit as usize,
        bbox: input.bbox,
        near,
        filter,
        fields: input.fields.clone(),
        geometry: input.geometry.unwrap_or(false),
        count: input.count.unwrap_or(false),
        statistics: input.statistics.clone().unwrap_or_default(),
        cursor,
        order: input.order.clone(),
        expected_stamp: input.expected_stamp.clone(),
    })
}

/// What passes is gathered here: the page, the count and the statistics' values.
struct Gather<'a> {
    asked: &'a Asked,
    rows: Vec<QueriedRow>,
    last: Option<Cursor>,
    next: Option<String>,
    total: u64,
    values: Vec<Vec<String>>,
    unreadable: u64,
}

impl<'a> Gather<'a> {
    fn new(asked: &'a Asked) -> Self {
        Gather {
            asked,
            rows: Vec::new(),
            last: None,
            next: None,
            total: 0,
            values: vec![Vec::new(); asked.statistics.len()],
            unreadable: 0,
        }
    }

    /// Whether every record still to come can be left: the page and its next are known, nothing is counted.
    fn done(&self) -> bool {
        self.next.is_some() && !self.asked.count && self.asked.statistics.is_empty()
            || (self.asked.limit == 0 && !self.asked.count && self.asked.statistics.is_empty())
    }

    /// One record that passes; `value`: its value in the order's field (an ordered question's cursor keeps it).
    fn take(&mut self, uid: Uuid, key: Option<i64>, e: &Entity, value: Option<&serde_json::Value>) {
        self.total += 1;
        for (k, f) in self.asked.statistics.iter().enumerate() {
            if let Some(v) = e.base().attrs.get(f) {
                self.values[k].push(v.clone());
            }
        }
        if self.rows.len() < self.asked.limit {
            let attrs: BTreeMap<String, String> = e
                .base()
                .attrs
                .iter()
                .filter(|(k, _)| self.asked.fields.as_ref().is_none_or(|f| f.contains(k)))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            let wkt = if self.asked.geometry {
                gp::geometry_of(e)
                    .ok()
                    .flatten()
                    .map(|(g, _)| gpkg::wkt::write_wkt(&g))
            } else {
                None
            };
            self.rows.push(QueriedRow {
                uid: uid.to_string(),
                key,
                attrs,
                wkt,
            });
            let place = match key {
                Some(k) => Place::Key(k),
                None => Place::Uid(uid),
            };
            self.last = Some(match (place, self.asked.order.is_some()) {
                (p, true) => Cursor::After(p, value.cloned().unwrap_or(serde_json::Value::Null)),
                (Place::Key(k), false) => Cursor::Key(k),
                (Place::Uid(u), false) => Cursor::Uid(u),
            });
        } else if self.next.is_none() && self.asked.limit > 0 {
            self.next = self.last.as_ref().map(Cursor::text);
        }
    }

    fn statistics(&self) -> Vec<QueriedStatistic> {
        self.asked
            .statistics
            .iter()
            .zip(&self.values)
            .map(|(field, values)| {
                let refs: Vec<Option<&str>> = values.iter().map(|v| Some(v.as_str())).collect();
                match kentos_geometry_core::ops::statistics::figures(&refs, None) {
                    Ok(f) => QueriedStatistic {
                        field: field.clone(),
                        read: f.read as u64,
                        skipped: f.skipped as u64,
                        sum: f.sum.map(|d| d.text()),
                        mean: f.mean.map(|d| d.text()),
                        min: f.min.map(|d| d.text()),
                        max: f.max.map(|d| d.text()),
                        std: f.std,
                        problem: None,
                    },
                    Err(why) => QueriedStatistic {
                        field: field.clone(),
                        read: 0,
                        skipped: 0,
                        sum: None,
                        mean: None,
                        min: None,
                        max: None,
                        std: None,
                        problem: Some(why),
                    },
                }
            })
            .collect()
    }
}

/// The objects of a part that pass the box, the place and the filter, in their order.
fn passing(
    asked: &Asked,
    list: &[(Uuid, Option<i64>, Entity)],
    layer_name: &dyn Fn(&str) -> String,
) -> Vec<bool> {
    let shapes: Vec<Shape> = if asked.needs_shape() {
        list.iter()
            .map(|(_, _, e)| crate::geometry::shape(e))
            .collect()
    } else {
        Vec::new()
    };
    let mut out = vec![true; list.len()];
    if let Some(b) = asked.bbox {
        for (k, s) in shapes.iter().enumerate() {
            let s = shape_bounds(s);
            out[k] = s.max_x >= b[0] && s.min_x <= b[2] && s.max_y >= b[1] && s.min_y <= b[3];
        }
    }
    if let Some((at, tolerance)) = asked.near {
        let point = Shape::Point {
            p: at,
            z: None,
            parts: None,
        };
        let point = spatial_query::geometry_of(&point);
        for (k, s) in shapes.iter().enumerate() {
            if !out[k] {
                continue;
            }
            out[k] = match (spatial_query::geometry_of(s), &point) {
                (Some(g), Some(p)) => spatial_query::distance(&g, p) <= tolerance,
                _ => false,
            };
        }
    }
    if let Some(f) = &asked.filter {
        let asked_ones: Vec<usize> = (0..list.len()).filter(|&k| out[k]).collect();
        let entities: Vec<&Entity> = asked_ones.iter().map(|&k| &list[k].2).collect();
        let met = f.condition(&entities, layer_name, |i| shapes.get(asked_ones[i]));
        for (i, &k) in asked_ones.iter().enumerate() {
            out[k] = met[i];
        }
    }
    out
}

// ── The drawing's objects ────────────────────────────────────────────────────

fn drawing(
    doc: &Document,
    layer: &str,
    asked: &Asked,
    stop: &dyn Fn() -> bool,
) -> Result<(SourcesQueried, Vec<CommandWarning>), Stop> {
    if let Some(Cursor::Key(_) | Cursor::After(Place::Key(_), _)) = asked.cursor {
        return Err(fail(
            codes::INVALID_CURSOR,
            "Bu imleç bir kaynağın; çizimin nesneleri kalıcı kimlikleriyle sayfalanır.",
            "cursor",
        ));
    }
    let node = doc.layers().get(layer);
    if let (Some(o), Some(n)) = (&asked.order, node)
        && !n.fields.is_empty()
        && !n.fields.iter().any(|lf| lf.name == o.field)
    {
        return Err(fail(
            codes::FIELD_NOT_FOUND,
            format!("“{}” alanı “{}” katmanında yok.", o.field, n.name),
            "order.field",
        ));
    }
    for (i, f) in asked.fields.iter().flatten().enumerate() {
        if let Some(n) = node
            && !n.fields.is_empty()
            && !n.fields.iter().any(|lf| lf.name == *f)
        {
            return Err(fail(
                codes::FIELD_NOT_FOUND,
                format!("“{f}” alanı “{}” katmanında yok.", n.name),
                &format!("fields.{i}"),
            ));
        }
    }
    for (i, f) in asked.statistics.iter().enumerate() {
        if let Some(n) = node
            && !n.fields.is_empty()
            && !n.fields.iter().any(|lf| lf.name == *f)
        {
            return Err(fail(
                codes::FIELD_NOT_FOUND,
                format!("“{f}” alanı “{}” katmanında yok.", n.name),
                &format!("statistics.{i}"),
            ));
        }
    }
    let out = crate::layer_filter::left_out(doc);
    let mut list: Vec<(Uuid, &Entity)> = doc
        .by_layer_with_uids(layer)
        .filter(|(u, _)| doc.slot_of(*u).is_some_and(|s| !out.contains(&s)))
        .collect();
    list.sort_by_key(|(u, _)| *u);
    let start = match asked.cursor {
        Some(Cursor::Uid(c)) => list.partition_point(|(u, _)| *u <= c),
        _ => 0,
    };
    let names = |id: &str| {
        doc.layers()
            .get(id)
            .map_or_else(|| id.to_owned(), |n| n.name.clone())
    };
    let mut g = Gather::new(asked);
    let mut read = 0u64;
    let mut held: Vec<(Uuid, Option<i64>, Entity)> = Vec::new();
    for part in list[start..].chunks(CHUNK) {
        if stop() {
            return Err(Stop::Cancelled);
        }
        read += part.len() as u64;
        let owned: Vec<(Uuid, Option<i64>, Entity)> =
            part.iter().map(|(u, e)| (*u, None, (*e).clone())).collect();
        let pass = passing(asked, &owned, &names);
        for (o, p) in owned.into_iter().zip(pass) {
            if !p {
                continue;
            }
            if asked.order.is_some() {
                held.push(o);
                if held.len() as u64 > MOST_ORDERED_HERE {
                    return Err(too_many_to_order());
                }
            } else {
                g.take(o.0, None, &o.2, None);
            }
        }
        if g.done() {
            break;
        }
    }
    if let Some(o) = &asked.order {
        let kind = field_order(doc, layer, &o.field);
        // As without an order, what is counted is what comes from the cursor on.
        for (u, k, e) in order_here(held, &o.field, kind, o.descending, asked.cursor.as_ref()) {
            let v = here_value(&e, &o.field).map(serde_json::Value::String);
            g.take(u, k, &e, v.as_ref());
        }
    }
    let link = doc.settings().sources.iter().find(|s| s.layer == layer);
    let partial = link.is_some_and(|l| l.extent.is_some());
    let mut warnings = Vec::new();
    if partial {
        warnings.push(CommandWarning {
            code: codes::PARTIAL_SCOPE.into(),
            message: format!(
                "“{}” katmanı kaynağının yalnız bir parçası; yanıt o parçanın. Bütün tablo için kapsamı kaynak yapın (scope: source).",
                names(layer)
            ),
            path: Some("scope".into()),
        });
    }
    Ok((
        SourcesQueried {
            source: QueriedSource {
                kind: QueriedKind::Drawing,
                layer: Some(layer.to_owned()),
                path: None,
                table: None,
                srid: (doc.settings().srid != 0).then_some(doc.settings().srid),
            },
            statistics: g.statistics(),
            total: (asked.count || !asked.statistics.is_empty()).then_some(g.total),
            plan: QueryPlan {
                bbox: asked.bbox.map(|_| PlanBox::Local),
                columns: Vec::new(),
                geometry: false,
                pushed: None,
                exact: asked.filter.is_none(),
                local: asked.local(),
                order: if asked.order.is_some() { "here" } else { "uid" }.into(),
            },
            rows: g.rows,
            next: g.next,
            partial,
            read,
            stamp: None,
            revision: doc.revision().to_string(),
        },
        warnings,
    ))
}

// ── A source ─────────────────────────────────────────────────────────────────

/// What a question needs of a source, found before it is read.
struct Found {
    layer: gpkg::Layer,
    srid: Option<u32>,
    uids: std::collections::HashMap<i64, String>,
}

fn sort_of(kind: &ColumnKind) -> ColumnSort {
    match kind {
        ColumnKind::Integer | ColumnKind::Real => ColumnSort::Number,
        ColumnKind::Text(_) => ColumnSort::Text,
        _ => ColumnSort::Other,
    }
}

fn file_stamp(path: &Path) -> Option<String> {
    let m = std::fs::metadata(path).ok()?;
    let when = m
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();
    Some(format!("{}:{when}", m.len()))
}

fn found_in_file(path: &Path, table: &str, at: &str) -> Result<Found, Stop> {
    let unreadable = |why: String| {
        fail(
            codes::IMPORT_UNREADABLE,
            format!("{} GeoPackage olarak okunamadı: {why}", path.display()),
            at,
        )
    };
    let file = GeoPackage::open(path, false).map_err(|e| unreadable(e.0))?;
    let layers = file.layers().map_err(|e| unreadable(e.0))?;
    let Some(layer) = layers.into_iter().find(|l| l.name == table) else {
        return Err(fail(
            codes::TABLE_NOT_FOUND,
            format!(
                "“{table}” tablosu dosyada yok; dosyanın tablolarından birinin adını verin (cad.sources.describe listeler)."
            ),
            "table",
        ));
    };
    let srid = match layer.srs_id {
        Some(id) if id > 0 => file
            .srs(id)
            .map_err(|e| unreadable(e.0))?
            .filter(|s| s.definition != "undefined" && s.organization.eq_ignore_ascii_case("EPSG"))
            .and_then(|s| u32::try_from(s.organization_id).ok()),
        _ => None,
    };
    let uids = file.uids(table).map_err(|e| unreadable(e.0))?;
    Ok(Found { layer, srid, uids })
}

/// A question asked of a source: a GeoPackage file or a database (docs/adr/0581 §2). `layer`: the bound layer the
/// question came through.
#[allow(clippy::too_many_arguments)]
fn source(
    settings: &ProjectSettings,
    origin: Origin<'_>,
    table: &str,
    at: At<'_>,
    layer: Option<&str>,
    asked: &Asked,
    revision: String,
    stop: &dyn Fn() -> bool,
    mut stage: Option<&mut Staging>,
) -> Result<(SourcesQueried, Vec<CommandWarning>), Stop> {
    if let Some(Cursor::Uid(_) | Cursor::After(Place::Uid(_), _)) = asked.cursor {
        return Err(fail(
            codes::INVALID_CURSOR,
            "Bu imleç çizimin; bir kaynağın kayıtları anahtarlarıyla sayfalanır.",
            "cursor",
        ));
    }
    let (found, path, address) = match origin {
        Origin::File(path) => (found_in_file(path, table, at.address)?, Some(path), None),
        Origin::Database(address) => {
            let p = sources::database_provider(at.address)?;
            let s = p
                .describe(address, table)
                .map_err(|e| sources::provider_stop(e, address, at))?;
            (
                Found {
                    layer: s.layer,
                    srid: s.srid,
                    uids: s.uids,
                },
                None,
                Some(address),
            )
        }
    };
    let column = |name: &str| found.layer.columns.iter().position(|c| c.name == name);
    for (i, f) in asked.fields.iter().flatten().enumerate() {
        if column(f).is_none() {
            return Err(fail(
                codes::FIELD_NOT_FOUND,
                format!("“{f}” sütunu “{table}” tablosunda yok."),
                &format!("fields.{i}"),
            ));
        }
    }
    for (i, f) in asked.statistics.iter().enumerate() {
        if column(f).is_none() {
            return Err(fail(
                codes::FIELD_NOT_FOUND,
                format!("“{f}” sütunu “{table}” tablosunda yok."),
                &format!("statistics.{i}"),
            ));
        }
    }
    // The order's column, and who orders by it (docs/adr/0591 §2): the source, unless it holds text.
    let order_column = match &asked.order {
        None => None,
        Some(o) => Some(column(&o.field).ok_or_else(|| {
            fail(
                codes::FIELD_NOT_FOUND,
                format!("“{}” sütunu “{table}” tablosunda yok.", o.field),
                "order.field",
            )
        })?),
    };
    let here = order_column.is_some_and(|i| {
        matches!(
            found.layer.columns[i].kind,
            ColumnKind::Text(_) | ColumnKind::Blob(_)
        )
    });
    // A later page is read only of the file the first one was (docs/adr/0591 §3).
    if let (Some(expected), Some(path)) = (&asked.expected_stamp, path)
        && file_stamp(path).as_deref() != Some(expected.as_str())
    {
        return Err(Stop::conflict(error(
            codes::SOURCE_CHANGED,
            format!(
                "{} ilk sayfa okunduğundan beri değişti; soruyu imleçsiz baştan sorun.",
                path.display()
            ),
            Some("expectedStamp".to_owned()),
        )));
    }
    // The geometry is in the project's system when anything here reads it.
    let geometry = asked.geometry || asked.needs_shape();
    let mover: Option<Mover> = if geometry && found.layer.geometry.is_some() {
        sources::mover_in(settings, found.srid).map_err(|why| {
            fail(
                codes::CRS_MISMATCH,
                format!("“{table}” tablosunun geometrisi projenin sistemine çevrilemiyor: {why}."),
                "table",
            )
        })?
    } else {
        None
    };
    // The box the source is sent: the place's, else the box's, in the table's system.
    let wanted = match (asked.near, asked.bbox) {
        (Some((p, t)), _) => Some([p.x - t, p.y - t, p.x + t, p.y + t]),
        (None, b) => b,
    };
    let sent = if found.layer.geometry.is_some() {
        sources::table_area(wanted, mover.as_ref()).map_err(|()| {
            fail(
                codes::CRS_MISMATCH,
                format!("Kutu “{table}” tablosunun sistemine çevrilemiyor."),
                "bbox",
            )
        })?
    } else {
        wanted
    };
    // The columns read: those returned, those the filter and the statistics read.
    let mut read_columns: Vec<usize> = Vec::new();
    let mut want = |name: &str| {
        if let Some(i) = column(name)
            && !read_columns.contains(&i)
        {
            read_columns.push(i);
        }
    };
    match &asked.fields {
        None => found.layer.columns.iter().for_each(|c| want(&c.name)),
        Some(fs) => fs.iter().for_each(|f| want(f)),
    }
    if let Some(e) = asked.filter.as_ref().and_then(CompiledFilter::expr) {
        e.fields.iter().for_each(|f| want(f));
    }
    asked.statistics.iter().for_each(|f| want(f));
    if let Some(o) = &asked.order {
        want(&o.field);
    }
    read_columns.sort_unstable();
    let order_at = order_column.and_then(|i| read_columns.iter().position(|&c| c == i));
    let mut projected = found.layer.clone();
    projected.columns = read_columns
        .iter()
        .map(|&i| found.layer.columns[i].clone())
        .collect();
    // The filter's part the source checks.
    let dialect = match origin {
        Origin::File(_) => Dialect::Sqlite,
        Origin::Database(_) => Dialect::Postgres,
    };
    let pushed: Pushed = match asked.filter.as_ref().and_then(CompiledFilter::expr) {
        None => Pushed::default(),
        Some(e) => {
            let columns = |name: &str| {
                let c = found.layer.columns.iter().find(|c| c.name == name)?;
                let q = gpkg::quote(&c.name);
                let named = match dialect {
                    Dialect::Sqlite => format!("t.{q}"),
                    Dialect::Postgres => q,
                };
                Some((named, sort_of(&c.kind)))
            };
            pushdown::push(e, &columns, dialect)
        }
    };
    let read_geometry = geometry || (sent.is_some() && !found.layer.indexed);
    let uid_of = |key: i64| match address {
        None => gp::row_uid(&found.layer.name, key),
        Some(a) => gp::database_row_uid(sources::database_name(a), &found.layer.name, key),
    };
    let after = match asked.cursor {
        Some(Cursor::Key(k)) => Some(k),
        _ => None,
    };
    let scan_order = match (&asked.order, order_column) {
        (Some(o), Some(column)) if !here => Some(gpkg::ScanOrder {
            column,
            descending: o.descending,
            after: match &asked.cursor {
                Some(Cursor::After(Place::Key(k), v)) => Some((value_of(v), *k)),
                _ => None,
            },
        }),
        _ => None,
    };
    let mut g = Gather::new(asked);
    let mut held: Vec<(Uuid, Option<i64>, Entity)> = Vec::new();
    let mut too_many = false;
    let mut problem: Option<String> = None;
    let mut cancelled = false;
    let names = |id: &str| id.to_owned();
    let mut take = |rows: Vec<gpkg::Row>| -> bool {
        if stop() {
            cancelled = true;
            return false;
        }
        // The order's raw values by key, for the cursor of a source's order.
        let raw: std::collections::HashMap<i64, serde_json::Value> = match (&scan_order, order_at) {
            (Some(_), Some(at)) => rows
                .iter()
                .map(|r| (r.fid, json_of(&r.values[at])))
                .collect(),
            _ => std::collections::HashMap::new(),
        };
        let t = match gp::table_of_rows(&projected, &rows, &found.uids, &uid_of) {
            Ok(t) => t,
            Err(why) => {
                problem = Some(why);
                return false;
            }
        };
        g.unreadable += t.skipped.len() as u64;
        if let Some(st) = stage.as_deref_mut()
            && st.fields.is_none()
        {
            st.fields = Some(t.fields.clone());
        }
        let mut list: Vec<(Uuid, Option<i64>, Entity)> = Vec::with_capacity(t.objects.len());
        for ((uid, mut e), key) in t.objects.into_iter().zip(t.fids) {
            if let Some(m) = &mover
                && kentos_formats::reproject::move_entity(&mut e, &m.source, &m.project, &m.choices)
                    .is_err()
            {
                g.unreadable += 1;
                continue;
            }
            list.push((uid, Some(key), e));
        }
        let pass = passing(asked, &list, &names);
        for (o, p) in list.into_iter().zip(pass) {
            if !p {
                continue;
            }
            if let Some(st) = stage.as_deref_mut() {
                // Counted as a page of none takes it, then held for the run.
                g.take(o.0, o.1, &o.2, o.1.and_then(|k| raw.get(&k)));
                st.objects.push((o.0, o.2));
                if st.objects.len() > st.most {
                    st.too_many = true;
                    return false;
                }
                continue;
            }
            if here {
                held.push(o);
                if held.len() as u64 > MOST_ORDERED_HERE {
                    too_many = true;
                    return false;
                }
            } else {
                let v = o.1.and_then(|k| raw.get(&k));
                g.take(o.0, o.1, &o.2, v);
            }
        }
        !g.done()
    };
    let read = match origin {
        Origin::File(path) => {
            let file = GeoPackage::open(path, false).map_err(|e| {
                fail(
                    codes::IMPORT_UNREADABLE,
                    format!("{} GeoPackage olarak okunamadı: {}", path.display(), e.0),
                    at.table,
                )
            })?;
            let values: Vec<gpkg::Value> = pushed
                .params
                .iter()
                .map(|p| match p {
                    Param::Number(x) => gpkg::Value::Real(*x),
                    Param::Text(t) => gpkg::Value::Text(t.clone()),
                })
                .collect();
            file.scan(
                &found.layer,
                &gpkg::Scan {
                    bbox: sent,
                    columns: Some(&read_columns),
                    geometry: read_geometry,
                    filter: (!pushed.sql.is_empty())
                        .then_some((pushed.sql.as_str(), values.as_slice())),
                    after,
                    chunk: CHUNK,
                    order: scan_order.clone(),
                },
                &mut take,
            )
            .map_err(|e| {
                fail(
                    codes::IMPORT_UNREADABLE,
                    format!("{} okunamadı: {}", path.display(), e.0),
                    at.table,
                )
            })?
        }
        Origin::Database(address) => {
            let p = sources::database_provider(at.address)?;
            p.scan(
                address,
                table,
                &ProviderScan {
                    bbox: sent,
                    columns: Some(&read_columns),
                    geometry: read_geometry,
                    filter: (!pushed.sql.is_empty())
                        .then_some((pushed.sql.as_str(), pushed.params.as_slice())),
                    after,
                    chunk: CHUNK,
                    order: scan_order.clone(),
                },
                &mut take,
            )
            .map_err(|e| sources::provider_stop(e, address, at))?
        }
    };
    if cancelled {
        return Err(Stop::Cancelled);
    }
    if let Some(why) = problem {
        return Err(fail(codes::IMPORT_UNREADABLE, why, at.table));
    }
    if too_many {
        return Err(too_many_to_order());
    }
    if let Some(most) = stage.as_ref().filter(|st| st.too_many).map(|st| st.most) {
        return Err(fail(
            codes::INPUT_TOO_LARGE,
            format!(
                "“{table}” tablosundan {most} kayıttan çoğu okunacaktı; bir çalıştırma bu kadarını alır. Süzgeçle (filter) ya da kutuyla (bbox) daraltın."
            ),
            at.table,
        ));
    }
    if let (true, Some(o)) = (here, &asked.order) {
        for (u, k, e) in order_here(held, &o.field, "text", o.descending, asked.cursor.as_ref()) {
            let v = here_value(&e, &o.field).map(serde_json::Value::String);
            g.take(u, k, &e, v.as_ref());
        }
    }
    let mut warnings = Vec::new();
    if g.unreadable > 0 {
        warnings.push(CommandWarning {
            code: codes::ROWS_UNREADABLE.into(),
            message: format!(
                "“{table}” tablosunun {} kaydının geometrisi okunamadı ya da projenin sistemine çevrilemedi; atlandı.",
                g.unreadable
            ),
            path: Some("table".into()),
        });
    }
    let shown = (!pushed.sql.is_empty()).then(|| pushed.shown().replace("t.\"", "\""));
    let plan = QueryPlan {
        bbox: sent.map(|_| {
            if found.layer.indexed {
                PlanBox::Index
            } else {
                PlanBox::Geometry
            }
        }),
        columns: projected.columns.iter().map(|c| c.name.clone()).collect(),
        geometry: read_geometry,
        pushed: shown,
        exact: asked.filter.is_none() || pushed.exact,
        local: asked.local(),
        order: match (&asked.order, here) {
            (None, _) => "key",
            (Some(_), false) => "column",
            (Some(_), true) => "here",
        }
        .into(),
    };
    Ok((
        SourcesQueried {
            source: QueriedSource {
                kind: match origin {
                    Origin::File(_) => QueriedKind::Geopackage,
                    Origin::Database(_) => QueriedKind::Postgis,
                },
                layer: layer.map(str::to_owned),
                path: Some(match origin {
                    Origin::File(p) => p.display().to_string(),
                    Origin::Database(a) => a.to_owned(),
                }),
                table: Some(found.layer.name.clone()),
                srid: found.srid,
            },
            statistics: g.statistics(),
            total: (asked.count || !asked.statistics.is_empty()).then_some(g.total),
            plan,
            rows: g.rows,
            next: g.next,
            partial: false,
            read,
            stamp: path.and_then(file_stamp),
            revision,
        },
        warnings,
    ))
}

fn run(
    doc: &Document,
    input: &SourcesQuery,
    stop: &dyn Fn() -> bool,
) -> Result<(SourcesQueried, Vec<CommandWarning>), Stop> {
    let asked = asked(input, doc.settings())?;
    if let Some(layer) = &input.layer {
        let node = doc
            .layers()
            .get(layer)
            .filter(|n| n.kind == kentos_commands::LayerNodeType::Layer)
            .ok_or_else(|| {
                fail(
                    codes::LAYER_NOT_FOUND,
                    format!("“{layer}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."),
                    "layer",
                )
            })?;
        return match input.scope.unwrap_or_default() {
            QueryScope::Loaded => drawing(doc, layer, &asked, stop),
            QueryScope::Source => {
                let Some(link) = doc.settings().sources.iter().find(|s| s.layer == *layer) else {
                    return Err(fail(
                        codes::INVALID_INPUT,
                        format!(
                            "“{}” katmanı bir kaynağa bağlı değil; kapsamı kaynak yapılamaz.",
                            node.name
                        ),
                        "scope",
                    ));
                };
                let origin = match link.kind {
                    SourceKind::Geopackage => sources::origin(&link.path, "layer")?,
                    SourceKind::Postgis => Origin::Database(&link.path),
                };
                source(
                    doc.settings(),
                    origin,
                    &link.table,
                    sources::at_one("layer"),
                    Some(layer),
                    &asked,
                    doc.revision().to_string(),
                    stop,
                    None,
                )
            }
        };
    }
    let path = input.path.as_deref().unwrap_or_default();
    let table = input.table.as_deref().unwrap_or_default();
    let origin = sources::origin(path, "path")?;
    source(
        doc.settings(),
        origin,
        table,
        At {
            address: "path",
            table: "table",
        },
        None,
        &asked,
        doc.revision().to_string(),
        stop,
        None,
    )
}

/// A question asked of a bound layer's source away from the drawing (the desktop asks on a thread of its own, with a
/// copy of the project's settings and its own stop): as `scope: source`.
#[allow(clippy::too_many_arguments)]
pub fn ask_source(
    settings: &ProjectSettings,
    kind: SourceKind,
    address: &str,
    table: &str,
    layer: &str,
    input: &SourcesQuery,
    revision: String,
    stop: &dyn Fn() -> bool,
) -> CommandResult<SourcesQueried> {
    let answer = (|| {
        let asked = asked(input, settings)?;
        let origin = match kind {
            SourceKind::Geopackage => sources::origin(address, "layer")?,
            SourceKind::Postgis => Origin::Database(address),
        };
        source(
            settings,
            origin,
            table,
            sources::at_one("layer"),
            Some(layer),
            &asked,
            revision,
            stop,
            None,
        )
    })();
    match answer {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The most records a processing input read from its source holds (docs/adr/0592 §1).
pub const MOST_STAGED: usize = 1_000_000;

/// What a source's records read for a run come to (docs/adr/0592 §1).
pub struct Staging {
    pub objects: Vec<(Uuid, Entity)>,
    /// The table's columns as fields (from its first part).
    pub fields: Option<Vec<kentos_commands::LayerField>>,
    too_many: bool,
    most: usize,
}

/// Where a processing input is read from: a bound layer's source, or a table named by its file or address.
pub enum StageFrom<'a> {
    Layer(&'a kentos_commands::SourceLink),
    Table { address: &'a str, table: &'a str },
}

/// A source's records read whole for a processing run (docs/adr/0592 §1): its parts in key order, `filter`'s part the
/// source can check sent to it, `bbox` its box, every record checked here as `cad.sources.query` checks it, made the
/// drawing's objects in the project's system with their lasting ids; the table's columns as fields. `at`: the place a
/// refusal names (the input's parameter). At most [`MOST_STAGED`] records; a stop ends it with no error (`Err(None)`).
pub fn stage(
    settings: &ProjectSettings,
    from: &StageFrom<'_>,
    filter: Option<&str>,
    bbox: Option<[f64; 4]>,
    at: &str,
    stop: &dyn Fn() -> bool,
) -> Result<(Staging, SourcesQueried), Option<Box<kentos_commands::CommandError>>> {
    stage_at_most(settings, from, filter, bbox, at, stop, MOST_STAGED)
}

/// [`stage`] with another cap (a test's).
pub fn stage_at_most(
    settings: &ProjectSettings,
    from: &StageFrom<'_>,
    filter: Option<&str>,
    bbox: Option<[f64; 4]>,
    at: &str,
    stop: &dyn Fn() -> bool,
    most: usize,
) -> Result<(Staging, SourcesQueried), Option<Box<kentos_commands::CommandError>>> {
    staged(settings, from, filter, bbox, at, stop, most).map_err(|s| match s {
        Stop::Cancelled => None,
        other => Some(Box::new(other.into_error())),
    })
}

fn staged(
    settings: &ProjectSettings,
    from: &StageFrom<'_>,
    filter: Option<&str>,
    bbox: Option<[f64; 4]>,
    at: &str,
    stop: &dyn Fn() -> bool,
    most: usize,
) -> Result<(Staging, SourcesQueried), Stop> {
    let (path, name) = match from {
        StageFrom::Layer(link) => (link.path.as_str(), link.table.as_str()),
        StageFrom::Table { address, table } => (*address, *table),
    };
    let input = SourcesQuery {
        path: Some(path.to_owned()),
        table: Some(name.to_owned()),
        filter: filter.map(str::to_owned),
        bbox,
        limit: Some(0),
        count: Some(true),
        ..SourcesQuery::default()
    };
    let (at_path, at_table) = (format!("{at}.path"), format!("{at}.table"));
    let mut asked = asked(&input, settings).map_err(|s| at_place(s, at))?;
    asked.geometry = true;
    let (origin, table) = match from {
        StageFrom::Layer(link) => (
            match link.kind {
                SourceKind::Geopackage => sources::origin(&link.path, &at_path)?,
                SourceKind::Postgis => Origin::Database(&link.path),
            },
            link.table.as_str(),
        ),
        StageFrom::Table { address, table } => (sources::origin(address, &at_path)?, *table),
    };
    let mut staging = Staging {
        objects: Vec::new(),
        fields: None,
        too_many: false,
        most,
    };
    let (out, _) = source(
        settings,
        origin,
        table,
        At {
            address: &at_path,
            table: &at_table,
        },
        None,
        &asked,
        String::new(),
        stop,
        Some(&mut staging),
    )
    .map_err(|s| at_place(s, at))?;
    Ok((staging, out))
}

/// A refusal of a question's own input (`filter`, `bbox`) named at the run's parameter instead.
fn at_place(stop: Stop, at: &str) -> Stop {
    match stop {
        Stop::Failed(mut e) => {
            if let Some(p @ ("filter" | "bbox" | "table" | "path" | "layer")) = e.path.as_deref() {
                e.path = Some(format!("{at}.{p}"));
            }
            Stop::Failed(e)
        }
        other => other,
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &SourcesQuery) -> CommandResult<()> {
    match asked(input, cx.doc.settings()) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &SourcesQuery) -> CommandResult<SourcesQueried> {
    answer(cx, input)
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: SourcesQuery,
) -> CommandResult<SourcesQueried> {
    answer(cx, &input)
}

fn answer(cx: &ExecutionContext<'_>, input: &SourcesQuery) -> CommandResult<SourcesQueried> {
    match run(cx.doc, input, &|| cx.stopped()) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}
