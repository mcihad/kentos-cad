//! Geometri dönüştürme (docs/adr/0218; the web's `builtin/convert/`): points
//! from objects, lines from points and tables, a path's piece, an area's
//! rings and a line's area, edges divided. The shapes are the geometry
//! core's (`ops::convert`); what the tools share is here: an object's
//! elevations as the core takes them, a result as a new object with its
//! elevations, and the table tools' source, columns and report.

pub mod paths;
pub mod points;
pub mod tables;

use std::collections::BTreeMap;

use kentos_contracts::Entity;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::convert::table::RowFault;
use kentos_geometry_core::ops::convert::{Elevations, Zs};
use kentos_native_application::elevation;
use serde_json::{Value, json};

use super::geometry::new_object;
use super::join_source::{missing, source_table};
use crate::text::js_trim;
use crate::types::{EnumOption, ParamDef, ParamKind, ScopeKind, Values};

/// The scopes the tools offer, a layer first.
pub fn scopes() -> Option<Vec<ScopeKind>> {
    Some(vec![
        ScopeKind::Layer,
        ScopeKind::Selection,
        ScopeKind::Visible,
        ScopeKind::All,
    ])
}

/// An object's elevations path by path (`elevation::paths`' order), none when it has none.
pub fn zs_of(e: &Entity) -> Option<Zs> {
    let paths = elevation::paths(e);
    paths
        .iter()
        .any(|p| p.zs.iter().any(Option::is_some))
        .then(|| paths.into_iter().map(|p| p.zs).collect())
}

/// A result as a new object on the output layer, with its attributes and elevations.
pub fn placed(r: Elevations, layer_id: &str, attrs: BTreeMap<String, String>) -> Option<Entity> {
    let mut e = new_object(r.shape, layer_id, attrs)?;
    if let Some(zs) = &r.zs {
        elevation::assign(&mut e, zs);
    }
    Some(e)
}

/// A point on the output layer.
pub fn point(
    p: kentos_geometry_core::vec2::Vec2,
    z: Option<f64>,
    layer_id: &str,
    attrs: BTreeMap<String, String>,
) -> Option<Entity> {
    new_object(Shape::Point { p, z, parts: None }, layer_id, attrs)
}

// ── Tablolar ─────────────────────────────────────────────────────────

fn from_layer(v: &Values) -> bool {
    v.get("sourceKind").and_then(Value::as_str) == Some("layer")
}

/// Where a table tool's rows come from (the web's `sourceParams`).
pub fn source_params(what: &str) -> Vec<ParamDef> {
    vec![
        ParamDef::new(
            "sourceKind",
            "Kaynak",
            ParamKind::Choice {
                options: vec![
                    EnumOption::new("file", "Dosya").hint("CSV, TXT, Excel ya da ODS dosyası"),
                    EnumOption::new("layer", "Katman").hint("Nesnelerin öznitelikleri satırdır"),
                ],
            },
        )
        .default_value(json!("file")),
        ParamDef::new(
            "file",
            "Dosya",
            ParamKind::File {
                accept: [".csv", ".txt", ".xlsx", ".ods"].map(String::from).to_vec(),
            },
        )
        .shown_when(|v| !from_layer(v))
        .describe(&format!(
            "İlk satırı sütun adları olan CSV, TXT, Excel (.xlsx) ya da ODS dosyası; her satır bir {what}."
        )),
        ParamDef::new(
            "source",
            "Kaynak nesneler",
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
        .shown_when(from_layer)
        .describe(&format!("Öznitelikleri satır olan nesneler; her nesne bir {what}.")),
    ]
}

/// A column parameter.
pub fn column(name: &str, label: &str, description: &str, optional: bool) -> ParamDef {
    let p = ParamDef::new(
        name,
        label,
        ParamKind::Field {
            of: vec!["source".into(), "file".into()],
            allow_new: false,
            multiple: false,
        },
    );
    let p = if optional { p.optional() } else { p };
    p.describe(description)
}

/// An angle's unit, the project's by default.
pub fn unit_param() -> ParamDef {
    ParamDef::new(
        "unit",
        "Açı birimi",
        ParamKind::Choice {
            options: vec![
                EnumOption::new("grad", "Grad").hint("Tam dönüş 400"),
                EnumOption::new("deg", "Derece").hint("Tam dönüş 360"),
            ],
        },
    )
    .default_from(|d| {
        json!(if d.angle_unit == kentos_contracts::AngleUnit::Deg {
            "deg"
        } else {
            "grad"
        })
    })
}

/// A table tool's rows: the columns' cells row by row, each row's label in
/// the report (a file's line number, the names' line the first; an
/// object's number) and its attributes (its cells that are not blank).
pub struct Rows {
    pub cells: Vec<Vec<String>>,
    pub labels: Vec<String>,
    pub attrs: Vec<BTreeMap<String, String>>,
}

/// The rows of the run's source, or why not (a column it does not have).
pub fn read_rows(v: &crate::types::Resolved<'_>, columns: &[String]) -> Result<Rows, String> {
    let objects: Option<&[&Entity]> =
        from_layer(&v.values).then(|| v.features("source").entities.as_slice());
    let (header, rows) = match objects {
        Some(o) => source_table(Some(o), None),
        None => source_table(None, v.values.get("file")),
    };
    let mut at = Vec::with_capacity(columns.len());
    for c in columns {
        if c.is_empty() {
            at.push(None);
            continue;
        }
        match header.iter().position(|h| h == c) {
            Some(k) => at.push(Some(k)),
            None => return Err(missing(c, &header)),
        }
    }
    let cells = rows
        .iter()
        .map(|r| {
            at.iter()
                .map(|k| k.and_then(|k| r.get(k)).copied().unwrap_or("").to_owned())
                .collect()
        })
        .collect();
    let labels = match objects {
        Some(o) => o.iter().map(|e| format!("#{}", e.base().id)).collect(),
        None => (0..rows.len()).map(|i| (i + 2).to_string()).collect(),
    };
    let attrs = rows
        .iter()
        .map(|r| {
            header
                .iter()
                .zip(r)
                .filter(|(_, c)| !js_trim(c).is_empty())
                .map(|(h, c)| (h.clone(), (*c).to_owned()))
                .collect()
        })
        .collect();
    Ok(Rows {
        cells,
        labels,
        attrs,
    })
}

/// Why a point is not reached, as the notes say it (docs/adr/0201 §8).
pub fn unreached_text(u: kentos_geometry_core::crs::Unreached) -> &'static str {
    use kentos_geometry_core::crs::Unreached;
    match u {
        Unreached::Outside => "izdüşümün dışında",
        Unreached::NoLink => "datumlar arasında yol yok",
        Unreached::NoGrid => "datum seçiminin ızgarası bu cihazda yok",
        Unreached::OutsideGrid => "ızgaranın dışında",
    }
}

/// Why a row gave nothing, in the report (§5).
pub fn reason(fault: RowFault, names: &[String], cells: &[String]) -> String {
    let name = |k: usize| names.get(k).map_or("", String::as_str);
    match fault {
        RowFault::SamePlace => "Başlangıç ve bitiş aynı yerde".into(),
        RowFault::Empty(k) => format!("“{}” boş", name(k)),
        RowFault::NotNumber(k) => format!(
            "“{}” sayı değil: {}",
            name(k),
            js_trim(cells.get(k).map_or("", String::as_str))
        ),
        RowFault::NotPositive(k) => format!("“{}” sıfırdan büyük değil", name(k)),
        RowFault::OutOfRange(k) => format!(
            "“{}” coğrafi sınırın dışında (boylam ±180, enlem ±90): {}",
            name(k),
            js_trim(cells.get(k).map_or("", String::as_str))
        ),
        RowFault::Unreached(u) => format!("Projenin sistemine çevrilemedi ({})", unreached_text(u)),
    }
}
