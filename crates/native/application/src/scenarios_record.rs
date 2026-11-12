//! `cad.scenarios.record` v1 (plan/map-cad.md MAP-10, docs/adr/5060): Ölçüleri çizime yaz. `cad.scenarios.impact`'s
//! comparison written in the drawing as a table through `cad.entities.create`'s write, one undo step “Ölçüleri çizime
//! yaz”: a heading (the measure with the scenario's name, the unit, the field state, the scenario, the difference), a
//! row a measure in the comparison's order (areas to the project's area decimals, lengths to its length decimals, shares
//! to one decimal, counts whole; the difference signed), and the stamps' row; the scenario's id and both stamps are the
//! table's attributes. Every later comparison of the scenario reads its tables back (`records`): a table whose base or
//! scenario has changed since is stale and stays as it was. Not on a compared layer: the table would change the
//! measures. The shared cases are `fixtures/commands/v1/cad.scenarios.record.json`.

use std::collections::BTreeMap;

use kentos_commands::{
    AnnotationKind, CommandResult, CreateOperation, EntitiesCreate, Entity, EntityBase, EntityGeometry, ImpactRecord,
    LayerNodeType, NewObject, RECORD_SCENARIO, RECORD_SCENARIO_STAMP, RECORD_STAMP, ScenariosImpacted, ScenariosRecord,
    ScenariosRecorded, TableAlign, TableEntity, Vec2,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The heading's words after the first.
const HEADING: [&str; 4] = ["Birim", "Mevcut durum", "Senaryo", "Fark"];

/// The tables of scenario `id` written in the drawing, in its order, each with whether the base's or the scenario's
/// stamp has changed since it was written.
pub(crate) fn records(doc: &Document, id: &str, stamp: &str, scenario_stamp: &str) -> Vec<ImpactRecord> {
    doc.entities_with_uids()
        .filter(|(_, e)| e.base().attrs.get(RECORD_SCENARIO).is_some_and(|s| s == id))
        .filter_map(|(uid, e)| {
            let attr = |k: &str| e.base().attrs.get(k).cloned();
            let (b, s) = (attr(RECORD_STAMP)?, attr(RECORD_SCENARIO_STAMP)?);
            Some(ImpactRecord {
                table: uid.to_string(),
                base_changed: b != stamp,
                scenario_changed: s != scenario_stamp,
                stamp: b,
                scenario_stamp: s,
            })
        })
        .collect()
}

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// A number as the table writes it: `decimals` places, a minus zero as zero.
fn fixed(v: f64, decimals: usize) -> String {
    let text = format!("{v:.decimals$}");
    if text.trim_start_matches('-').chars().all(|c| c == '0' || c == '.') { text.trim_start_matches('-').to_owned() } else { text }
}

/// The table's cells: the heading, a row a measure, the stamps' row.
fn cells(doc: &Document, r: &ScenariosImpacted) -> Vec<Vec<String>> {
    let s = doc.settings();
    let (area, length) = (s.area_decimals as usize, s.length_decimals as usize);
    let value = |unit: &str, v: f64| match unit {
        "m2" => fixed(v, area),
        "m" => fixed(v, length),
        "%" => fixed(v, 1),
        _ => fixed(v, 0),
    };
    let unit = |u: &str| match u {
        "m2" => "m²",
        "m" => "m",
        "%" => "%",
        _ => "adet",
    };
    let mut rows = vec![std::iter::once(format!("Ölçü ({})", r.name)).chain(HEADING.iter().map(|h| (*h).to_owned())).collect()];
    for m in &r.measures {
        let d = value(&m.unit, m.difference);
        let signed = if m.difference > 0.0 && d.chars().any(|c| c.is_ascii_digit() && c != '0') { format!("+{d}") } else { d };
        rows.push(vec![m.label.clone(), unit(&m.unit).to_owned(), value(&m.unit, m.base), value(&m.unit, m.alternative), signed]);
    }
    rows.push(vec!["Damga".to_owned(), String::new(), r.stamp.clone(), r.scenario_stamp.clone(), String::new()]);
    rows
}

/// The compared layers' objects' box: their vertices, both sides.
fn compared_box(doc: &Document, r: &ScenariosImpacted) -> Option<[f64; 4]> {
    let layers: Vec<&str> = r.layers.iter().flat_map(|l| [l.base.as_str(), l.layer.as_str()]).collect();
    let mut b: Option<[f64; 4]> = None;
    for e in doc.entities().filter(|e| layers.contains(&e.base().layer_id.as_str())) {
        let pts: Vec<(f64, f64)> = match e {
            Entity::Line(l) => vec![(l.a.x, l.a.y), (l.b.x, l.b.y)],
            Entity::Polyline(p) | Entity::Polygon(p) => p.pts.iter().map(|v| (v.x, v.y)).collect(),
            Entity::Point(p) => vec![(p.p.x, p.p.y)],
            _ => Vec::new(),
        };
        for (x, y) in pts {
            b = Some(b.map_or([x, y, x, y], |b| [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)]));
        }
    }
    b
}

/// The comparison and the table to write, or why not.
fn check(doc: &Document, input: &ScenariosRecord) -> Result<(ScenariosImpacted, NewObject), Stop> {
    let r = crate::scenarios_impact::run(doc, &input.impact).map_err(|stop| match stop {
        Stop::Failed(mut e) => {
            e.path = Some(e.path.map_or_else(|| "impact".to_owned(), |p| format!("impact.{p}")));
            Stop::Failed(e)
        }
        other => other,
    })?;
    let id = input.layer_id.trim();
    let Some(node) = doc.layers().get(id).filter(|n| n.kind != LayerNodeType::Group) else {
        return Err(refuse(
            codes::INVALID_INPUT,
            format!("“{id}” kimlikli katman çizimde yok; tablonun yazılacağı bir katmanın kimliğini verin."),
            "layerId",
        ));
    };
    if r.layers.iter().any(|l| l.base == id || l.layer == id) {
        return Err(refuse(
            codes::INVALID_INPUT,
            format!(
                "Ölçü tablosu karşılaştırılan “{}” katmanına yazılamaz: tablo o katmanın ölçülerini değiştirir. Başka bir katman seçin.",
                node.name
            ),
            "layerId",
        ));
    }
    if let Some(at) = input.at
        && !(at[0].is_finite() && at[1].is_finite())
    {
        return Err(refuse(codes::INVALID_INPUT, "Tablonun köşesi sonlu iki koordinat olmalı.".into(), "at"));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let h = doc.settings().annotation_height(AnnotationKind::Table);
    let p = match (input.at, compared_box(doc, &r)) {
        (Some(at), _) => Vec2 { x: at[0], y: at[1] },
        (None, Some(b)) => Vec2 { x: b[2] + 2.0 * h, y: b[3] },
        (None, None) => Vec2 { x: 0.0, y: 0.0 },
    };
    let rows = cells(doc, &r);
    let aligns: Vec<TableAlign> = (0..5).map(|j| if j < 2 { TableAlign::Left } else { TableAlign::Right }).collect();
    let mut t = TableEntity {
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: String::new(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        p,
        rotation: 0.0,
        height: h,
        rows: Vec::new(),
        columns: Vec::new(),
        cells: Vec::new(),
        merges: Vec::new(),
        aligns: Some(aligns),
        header: true,
        grid: None,
        frame: None,
        face: Default::default(),
        source: None,
        styles: Vec::new(),
        formulas: Vec::new(),
        fields: Vec::new(),
        breaks: None,
        table_style: None,
    };
    doc.take_rows(&mut t, rows);
    let attrs: BTreeMap<String, String> = [
        (RECORD_SCENARIO, r.scenario.clone()),
        (RECORD_STAMP, r.stamp.clone()),
        (RECORD_SCENARIO_STAMP, r.scenario_stamp.clone()),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect();
    let table = NewObject {
        geometry: EntityGeometry::Table {
            p: t.p,
            rotation: t.rotation,
            height: t.height,
            rows: t.rows,
            columns: t.columns,
            cells: t.cells,
            merges: Vec::new(),
            aligns: t.aligns,
            header: true,
            grid: None,
            frame: None,
            face: Default::default(),
            source: None,
            styles: None,
            formulas: None,
            fields: None,
            table_style: None,
        },
        color: None,
        line_weight: None,
        line_type: None,
        line_pattern: None,
        attrs: Some(attrs),
        label: None,
        label_of: None,
        label_scale: None,
        symbol: None,
        order: None,
        transparency: None,
    };
    Ok((r, table))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ScenariosRecord) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// What the write would be: the stamps and the cells, no table yet.
pub fn plan(cx: &ExecutionContext<'_>, input: &ScenariosRecord) -> CommandResult<ScenariosRecorded> {
    match check(cx.doc, input) {
        Ok((r, table)) => CommandResult::Completed {
            output: ScenariosRecorded {
                table: String::new(),
                stamp: r.stamp,
                scenario_stamp: r.scenario_stamp,
                cells: table_cells(&table),
                revision: cx.doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

fn table_cells(o: &NewObject) -> Vec<Vec<String>> {
    match &o.geometry {
        EntityGeometry::Table { cells, .. } => cells.clone(),
        _ => Vec::new(),
    }
}

/// Writes the table, one undo step “Ölçüleri çizime yaz”.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ScenariosRecord) -> CommandResult<ScenariosRecorded> {
    crate::call::recorded(cx, kentos_commands::CAD_SCENARIOS_RECORD, kentos_commands::CAD_SCENARIOS_RECORD_VERSION, input, write)
}

fn write(cx: &mut ExecutionContext<'_>, input: ScenariosRecord) -> CommandResult<ScenariosRecorded> {
    let (r, table) = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let cells = table_cells(&table);
    let create = EntitiesCreate {
        layer_id: input.layer_id.trim().to_owned(),
        objects: vec![table],
        operation: Some(CreateOperation::ScenarioRecord),
        expected_revision: None,
    };
    match crate::create::write_in(cx, create) {
        CommandResult::Completed { output, warnings } => CommandResult::Completed {
            output: ScenariosRecorded {
                table: output.created.first().cloned().unwrap_or_default(),
                stamp: r.stamp,
                scenario_stamp: r.scenario_stamp,
                cells,
                revision: output.revision,
            },
            warnings,
        },
        other => crate::trim::relay(other),
    }
}
