//! Doğrusal referans (docs/adr/0224 §5; the web's `builtin/lrs/`): routes
//! made and calibrated, events shown and overlaid, objects located along
//! routes, measures extracted. The routes' rules are the geometry core's
//! (`ops::lrs`); what the tools share is here: a route of an object, the
//! routes of a layer by their id, the number rule of a table's cells
//! (docs/adr/0180, `ops::cogo::read_number`), how a cell that gives no
//! number is said, how numbers are written (the project's length decimals,
//! the display rule) and an event table's source.

pub mod events;
pub mod locate;
pub mod routes;

use std::collections::{BTreeMap, HashMap};

use kentos_contracts::{AreaPart, Entity, EntityBase, PathEntity, Vec2 as Point};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::ops::lrs::{Route, RoutePart};
use serde_json::{Value, json};

use super::join_source::source_table;
use crate::types::{EnumOption, ParamDef, ParamKind, ScopeKind, Values};

/// What a route is: a line or a polyline.
pub const ROUTE_KINDS: [&str; 2] = ["line", "polyline"];

/// The scopes the tools offer, a layer first.
pub fn scopes() -> Option<Vec<ScopeKind>> {
    Some(vec![
        ScopeKind::Layer,
        ScopeKind::Selection,
        ScopeKind::Visible,
        ScopeKind::All,
    ])
}

/// A features parameter of these kinds.
pub fn features(name: &str, label: &str, kinds: &[&str], writes: bool) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Features {
            kinds: Some(kinds.iter().map(|k| (*k).to_owned()).collect()),
            scopes: scopes(),
            writes,
        },
    )
}

/// A field parameter of `of`'s objects (or of a table's source: `source` and `file`).
pub fn field(name: &str, label: &str, of: &[&str]) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Field {
            of: of.iter().map(|s| (*s).to_owned()).collect(),
            allow_new: false,
            multiple: false,
        },
    )
}

/// A number parameter.
pub fn number(name: &str, label: &str, min: Option<f64>, unit: &str) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Number {
            min,
            max: None,
            integer: false,
            unit: unit.into(),
            placeholder: None,
        },
    )
}

/// The side a positive offset or distance is on.
pub fn side_param(label: &str) -> ParamDef {
    ParamDef::new(
        "side",
        label,
        ParamKind::Choice {
            options: vec![
                EnumOption::new("right", "Sağa").hint("Ölçü yönünün sağı"),
                EnumOption::new("left", "Sola").hint("Ölçü yönünün solu"),
            ],
        },
    )
    .default_value(json!("right"))
}

fn p(q: Vec2) -> Point {
    Point { x: q.x, y: q.y }
}

/// An object as a route (the application's, which Km yaz reads too).
pub use kentos_native_application::geometry::route as route_of;

/// The routes of objects by their id: the id field's value, trimmed (an
/// empty one no route); of several objects of an id the first.
pub struct Routes<'a> {
    pub ids: Vec<String>,
    pub routes: Vec<Route>,
    pub objects: Vec<&'a Entity>,
    pub index: HashMap<String, usize>,
    /// Objects of an id that came before.
    pub again: usize,
}

pub fn routes_by_id<'a>(objects: &[&'a Entity], field: &str) -> Routes<'a> {
    let mut out = Routes {
        ids: Vec::new(),
        routes: Vec::new(),
        objects: Vec::new(),
        index: HashMap::new(),
        again: 0,
    };
    for e in objects {
        let Some(route) = route_of(e) else {
            continue;
        };
        let id = crate::text::js_trim(e.base().attrs.get(field).map_or("", String::as_str));
        if id.is_empty() {
            continue;
        }
        if out.index.contains_key(id) {
            out.again += 1;
            continue;
        }
        out.index.insert(id.to_owned(), out.ids.len());
        out.ids.push(id.to_owned());
        out.routes.push(route);
        out.objects.push(e);
    }
    out
}

/// The warning of ids that came again.
pub fn again_text(again: usize) -> String {
    format!("{again} rota kimliği birden çok kez var; ilki kullanıldı.")
}

/// A cell's number (docs/adr/0180): a sign, digits and one decimal point or comma.
pub fn read_measure(text: &str) -> Option<f64> {
    kentos_geometry_core::ops::cogo::read_number(text)
}

/// Why a cell gives no number.
pub fn cell_reason(field: &str, cell: &str) -> String {
    let t = crate::text::js_trim(cell);
    if t.is_empty() {
        format!("“{field}” boş")
    } else {
        format!("“{field}” sayı değil: {t}")
    }
}

/// A route's parts as a polyline on `layer_id` with `attrs` (a part's
/// bulges only when one bends).
pub fn polyline_of(parts: &[RoutePart], layer_id: &str, attrs: BTreeMap<String, String>) -> Entity {
    let bends = |q: &RoutePart| {
        q.bulges
            .as_ref()
            .filter(|b| b.iter().any(|&x| x != 0.0))
            .cloned()
    };
    let (first, rest) = parts
        .split_first()
        .map_or((None, &[][..]), |(f, r)| (Some(f), r));
    let empty = RoutePart {
        pts: Vec::new(),
        bulges: None,
        ms: Vec::new(),
    };
    let first = first.unwrap_or(&empty);
    Entity::Polyline(PathEntity {
        base: EntityBase {
            id: 0,
            layer_id: layer_id.to_owned(),
            color: None,
            attrs,
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
        },
        pts: first.pts.iter().copied().map(p).collect(),
        bulges: bends(first),
        holes: None,
        zs: None,
        ms: Some(first.ms.clone()),
        parts: (!rest.is_empty()).then(|| {
            rest.iter()
                .map(|q| AreaPart {
                    pts: q.pts.iter().copied().map(p).collect(),
                    bulges: bends(q),
                    holes: None,
                    zs: None,
                    ms: Some(q.ms.clone()),
                })
                .collect()
        }),
    })
}

// ── Olay tabloları ────────────────────────────────────────────────────

/// Whether a source's rows come from a layer, by its kind parameter's name.
fn layer_kind(v: &Values, kind: &str) -> bool {
    v.get(kind).and_then(Value::as_str) == Some("layer")
}

fn is_file(v: &Values) -> bool {
    !layer_kind(v, "sourceKind")
}
fn is_layer(v: &Values) -> bool {
    layer_kind(v, "sourceKind")
}
fn is_a_file(v: &Values) -> bool {
    !layer_kind(v, "aSourceKind")
}
fn is_a_layer(v: &Values) -> bool {
    layer_kind(v, "aSourceKind")
}
fn is_b_file(v: &Values) -> bool {
    !layer_kind(v, "bSourceKind")
}
fn is_b_layer(v: &Values) -> bool {
    layer_kind(v, "bSourceKind")
}

/// When a parameter is shown, by the values.
type Shown = fn(&Values) -> bool;

/// Where an event table's rows come from: a file, or the objects'
/// attributes; `prefix` names the parameters (`sourceKind`, `file`,
/// `source`; `a`: `aSourceKind`, `aFile`, `aSource`; `b` alike).
pub fn source_params(prefix: &str, label: &str) -> Vec<ParamDef> {
    let (kind, file, source, file_shown, layer_shown): (&str, &str, &str, Shown, Shown) =
        match prefix {
            "a" => ("aSourceKind", "aFile", "aSource", is_a_file, is_a_layer),
            "b" => ("bSourceKind", "bFile", "bSource", is_b_file, is_b_layer),
            _ => ("sourceKind", "file", "source", is_file, is_layer),
        };
    vec![
        ParamDef::new(
            kind,
            &format!("{label} kaynağı"),
            ParamKind::Choice {
                options: vec![
                    EnumOption::new("file", "Dosya").hint("CSV, TXT, Excel ya da ODS dosyası"),
                    EnumOption::new("layer", "Katman").hint("Nesnelerin öznitelikleri satırdır"),
                ],
            },
        )
        .default_value(json!("file")),
        ParamDef::new(
            file,
            &format!("{label} dosyası"),
            ParamKind::File {
                accept: [".csv", ".txt", ".xlsx", ".ods"].map(String::from).to_vec(),
            },
        )
        .shown_when(file_shown)
        .describe("İlk satırı sütun adları olan CSV, TXT, Excel (.xlsx) ya da ODS dosyası; her satır bir olay."),
        ParamDef::new(
            source,
            &format!("{label} nesneleri"),
            ParamKind::Features {
                kinds: None,
                scopes: Some(vec![
                    ScopeKind::Layer,
                    ScopeKind::All,
                    ScopeKind::Visible,
                    ScopeKind::Selection,
                ]),
                writes: false,
            },
        )
        .shown_when(layer_shown)
        .describe("Öznitelikleri satır olan nesneler; her nesne bir olay."),
    ]
}

/// A table's rows: its column names (trimmed), its rows and each row's
/// label (a file's line, the names' line 1; an object's #id).
pub struct EventTable {
    pub header: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub labels: Vec<String>,
}

pub fn event_table(
    v: &crate::types::Resolved<'_>,
    kind: &str,
    file: &str,
    source: &str,
) -> EventTable {
    let from_layer = layer_kind(&v.values, kind);
    let objects: Option<&[&Entity]> = from_layer.then(|| v.features(source).entities.as_slice());
    let (header, rows) = match objects {
        Some(o) => source_table(Some(o), None),
        None => source_table(None, v.values.get(file)),
    };
    let labels = match objects {
        Some(o) => o.iter().map(|e| format!("#{}", e.base().id)).collect(),
        None => (0..rows.len()).map(|i| (i + 2).to_string()).collect(),
    };
    EventTable {
        header: header
            .iter()
            .map(|h| crate::text::js_trim(h).to_owned())
            .collect(),
        rows: rows
            .into_iter()
            .map(|r| r.into_iter().map(str::to_owned).collect())
            .collect(),
        labels,
    }
}

/// The text of a choice parameter, for a values map (a preview).
pub fn text_of<'v>(v: &'v Values, name: &str) -> &'v str {
    v.get(name).and_then(Value::as_str).unwrap_or("")
}
