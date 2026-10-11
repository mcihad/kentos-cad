//! Olayları rotada göster and Olayları çakıştır (the web's
//! `builtin/lrs/events.ts`; docs/adr/0224 §5; ArcGIS Make Route Event Layer
//! and Overlay Route Events).

use std::collections::BTreeMap;

use kentos_contracts::{Entity, EntityBase, PointEntity, Vec2 as Point};
use kentos_geometry_core::ops::lrs::{Event, EventIn, Fault, events_run, overlay};
use serde_json::{Value, json};

use super::{
    ROUTE_KINDS, again_text, cell_reason, event_table, features, field, polyline_of, read_measure,
    routes_by_id, side_param, source_params, text_of,
};
use crate::builtin::geometry::layer_param;
use crate::builtin::join_source::missing;
use crate::builtin::proximity::length_text;
use crate::types::{
    ChangeSet, Defaults, EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind,
    Resolved, RunContext, RunResult, Target, Tool, Values,
};

/// Why an event gives no place, in the table.
pub fn fault_text(units: &Defaults, f: &Fault) -> String {
    match f {
        Fault::Outside { least, most } => format!(
            "Ölçü rotanın dışında ({} – {})",
            length_text(units, *least),
            length_text(units, *most)
        ),
        Fault::NoMeasures => "Rotanın ölçüsü yok".into(),
        Fault::Same => "Başlangıç ve bitiş ölçüsü aynı".into(),
        Fault::Offset => "Öteleme yapılamadı".into(),
        Fault::NoRoute => "Rota yok".into(),
        Fault::NoMeasure => "Ölçü yok".into(),
        Fault::FewPoints => "İkiden az nokta".into(),
    }
}

/// A column of an event table's source.
fn column(name: &str, label: &str, prefix: &str) -> ParamDef {
    match prefix {
        "a" => field(name, label, &["aSource", "aFile"]),
        "b" => field(name, label, &["bSource", "bFile"]),
        _ => field(name, label, &["source", "file"]),
    }
}

fn point_kind(v: &Values) -> bool {
    text_of(v, "eventKind") != "line"
}
fn line_kind(v: &Values) -> bool {
    text_of(v, "eventKind") == "line"
}

pub fn route_events() -> Tool {
    Tool {
        id: "linearRef.routeEvents".into(),
        label: "Olayları rotada göster".into(),
        category: "linearRef".into(),
        description: "Bir tablonun rota ve ölçüyle verilen nokta ve çizgi olaylarını rotaların üstünde nesne olarak yazar.".into(),
        help: Some([
            "Örnekler: kaza kayıtlarının kilometreleri, yol kaplamasının başlangıç ve bitiş kilometreleri, trafik levhaları, bakım kesimleri.",
            "Nokta olayı rotanın o ölçüdeki yerine, çizgi olayı başlangıç ölçüsünden bitiş ölçüsüne rotanın parçası olarak yazılır (ters verilen olay ters çizilir; rotanın boşluğunda parçalara ayrılır). Öteleme sütunu verilirse olay rotaya dik o kadar kaydırılır: artı öteleme ölçü yönünün sağına (ya da seçilirse soluna).",
            "Kaynak bir dosya (ilk satırı sütun adları) ya da nesneler olabilir; nesnelerde öznitelikler satırdır. Olay satırın dolu hücrelerini öznitelik olarak alır. Rotası olmayan, ölçüsü sayı olmayan ya da rotanın ölçüleri dışında kalan satır atlanır ve tabloda nedeniyle söylenir.",
        ]
        .join("\n\n")),
        keywords: [
            "olay", "event", "route event", "rota", "ölçü", "kilometre", "doğrusal referans",
            "make route event layer",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["OLAYGOSTER", "ROUTEEVENTS"].map(String::from).to_vec(),
        icon: Some("routeEvents".into()),
        parameters: [
            vec![
                features("routes", "Rotalar", &ROUTE_KINDS, false),
                field("routeField", "Rota kimliği alanı", &["routes"]),
            ],
            source_params("", "Olay"),
            vec![
                column("eventRoute", "Olayın rota sütunu", ""),
                ParamDef::new(
                    "eventKind",
                    "Olay türü",
                    ParamKind::Choice {
                        options: vec![
                            EnumOption::new("point", "Nokta").hint("Bir ölçü"),
                            EnumOption::new("line", "Çizgi").hint("Başlangıç ve bitiş ölçüsü"),
                        ],
                    },
                )
                .default_value(json!("point")),
                column("measure", "Ölçü sütunu", "").shown_when(point_kind),
                column("from", "Başlangıç ölçüsü sütunu", "").shown_when(line_kind),
                column("to", "Bitiş ölçüsü sütunu", "").shown_when(line_kind),
                column("offset", "Öteleme sütunu", "")
                    .optional()
                    .describe("Rotaya dik uzaklık (m); boşsa olay rotanın üstündedir."),
                side_param("Artı öteleme"),
                layer_param("Olaylar", "#8E4EC6"),
            ],
        ]
        .concat(),
        outputs: vec![
            OutputDef::new("count", "Yazılan olay sayısı", OutputKind::Number),
            OutputDef::new("table", "Atlanan satırlar", OutputKind::Table),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_events),
    }
}

fn run_events(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let t = event_table(v, "sourceKind", "file", "source");
    let line = v.text("eventKind") == "line";
    let mut columns = vec![v.text("eventRoute").to_owned()];
    if line {
        columns.push(v.text("from").to_owned());
        columns.push(v.text("to").to_owned());
    } else {
        columns.push(v.text("measure").to_owned());
    }
    let offset_column = crate::text::js_trim(v.text("offset")).to_owned();
    for c in columns
        .iter()
        .chain((!offset_column.is_empty()).then_some(&offset_column))
    {
        if !t.header.contains(c) {
            return RunResult::refused(missing(c, &t.header));
        }
    }
    let at: Vec<usize> = columns
        .iter()
        .map(|c| t.header.iter().position(|h| h == c).unwrap_or(0))
        .collect();
    let off = (!offset_column.is_empty())
        .then(|| t.header.iter().position(|h| *h == offset_column))
        .flatten();
    let routes = routes_by_id(&v.features("routes").entities, v.text("routeField"));
    if routes.again > 0 {
        feedback.warn(again_text(routes.again));
    }
    let turn = if v.text("side") == "left" { -1.0 } else { 1.0 };
    // Why a row gave nothing, by its place: a cell's first, then the core's.
    let mut reasons: BTreeMap<usize, String> = BTreeMap::new();
    let mut events = Vec::new();
    let mut row_of = Vec::new();
    for (i, r) in t.rows.iter().enumerate() {
        let cell = |k: usize| r.get(k).map_or("", String::as_str);
        let id = crate::text::js_trim(cell(at[0]));
        let mut nums = Vec::new();
        let mut why: Option<String> = None;
        for k in 1..at.len() {
            match read_measure(cell(at[k])) {
                Some(n) => nums.push(n),
                None => {
                    why = Some(cell_reason(&columns[k], cell(at[k])));
                    break;
                }
            }
        }
        let mut offset = 0.0;
        if why.is_none()
            && let Some(o) = off
            && !crate::text::js_trim(cell(o)).is_empty()
        {
            match read_measure(cell(o)) {
                Some(n) => offset = n,
                None => why = Some(cell_reason(&offset_column, cell(o))),
            }
        }
        let route = routes.index.get(id).copied();
        if why.is_none() && route.is_none() {
            why = Some(format!("“{id}” rotası yok"));
        }
        if let Some(why) = why {
            reasons.insert(i, why);
            continue;
        }
        events.push(EventIn {
            route,
            m: (!line).then(|| nums[0]),
            from: line.then(|| nums[0]),
            to: line.then(|| nums[1]),
            offset: Some(offset * turn),
        });
        row_of.push(i);
    }
    let placed = events_run(&routes.routes, &events);
    let layer = &v.layer("layer").id;
    let mut add = Vec::new();
    for (k, out) in placed.iter().enumerate() {
        let i = row_of[k];
        if let Some(f) = &out.fault {
            reasons.insert(i, fault_text(ctx.units, f));
            continue;
        }
        let attrs: BTreeMap<String, String> = t
            .header
            .iter()
            .zip(&t.rows[i])
            .filter(|(_, value)| !crate::text::js_trim(value).is_empty())
            .map(|(h, value)| (h.clone(), value.clone()))
            .collect();
        match (&out.p, &out.parts) {
            (Some(p), _) => add.push(Entity::Point(PointEntity {
                base: EntityBase {
                    id: 0,
                    layer_id: layer.clone(),
                    color: None,
                    attrs,
                    label: None,
                    symbol: None,
                    line_weight: None,
                    label_pins: Vec::new(),
                },
                p: Point { x: p.x, y: p.y },
                z: None,
                parts: None,
            })),
            (None, Some(parts)) => add.push(polyline_of(parts, layer, attrs)),
            _ => {}
        }
    }
    let rows: Vec<Value> = reasons
        .iter()
        .map(|(i, why)| json!([t.labels[*i], why]))
        .collect();
    if !rows.is_empty() {
        feedback.warn(format!("{} satır atlandı; nedenleri tabloda.", rows.len()));
    }
    let count = add.len();
    let mut outputs: BTreeMap<String, Value> =
        [("count".to_owned(), json!(count))].into_iter().collect();
    if !rows.is_empty() {
        outputs.insert(
            "table".into(),
            json!({ "columns": ["Satır", "Neden"], "rows": rows }),
        );
    }
    RunResult {
        changes: Some(ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs: outputs.into_iter().collect(),
        summary: Some(format!("{} satırdan {count} olay yazıldı.", t.rows.len())),
        ..RunResult::default()
    }
}

pub fn overlay_events() -> Tool {
    Tool {
        id: "linearRef.overlayEvents".into(),
        label: "Olayları çakıştır".into(),
        category: "linearRef".into(),
        description: "İki çizgi olayı tablosunu aynı rotalarda keser; ortak (Kesişim) ya da bütün (Birleşim) parçaların tablosunu verir.".into(),
        help: Some([
            "Örnekler: kaplama türü ile hız sınırının birlikte geçerli olduğu kesimler, kaza kesimleriyle bakım kesimlerinin ortak parçaları, iki envanterin birleşimi.",
            "Olaylar bir olayın başladığı ya da bittiği her ölçüde bölünür. Kesişim iki tablonun da kapladığı, Birleşim herhangi birinin kapladığı parçaları verir; bir parçayı kaplayan her olay çifti bir satırdır, aynı çiftin komşu parçaları birleşir.",
            "Tablo Rota, Başlangıç ve Bitiş ile birinci tablonun, sonra ikinci tablonun öbür sütunlarını taşır (birincide de olan ad “ (2)” ekiyle). Rotası boş ya da ölçüsü sayı olmayan satır alınmaz. Tablo Kaydet ile XLSX, ODS ya da CSV olarak kaydedilir ve yeniden Olayları rotada göster'e verilebilir.",
        ]
        .join("\n\n")),
        keywords: [
            "olay", "çakıştır", "kesişim", "birleşim", "overlay route events", "doğrusal referans",
            "kesim",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["OLAYCAKISTIR", "OVERLAYEVENTS"].map(String::from).to_vec(),
        icon: Some("routeOverlay".into()),
        parameters: [
            source_params("a", "Birinci tablo"),
            vec![
                column("aRoute", "Birinci tablonun rota sütunu", "a"),
                column("aFrom", "Birinci tablonun başlangıç sütunu", "a"),
                column("aTo", "Birinci tablonun bitiş sütunu", "a"),
            ],
            source_params("b", "İkinci tablo"),
            vec![
                column("bRoute", "İkinci tablonun rota sütunu", "b"),
                column("bFrom", "İkinci tablonun başlangıç sütunu", "b"),
                column("bTo", "İkinci tablonun bitiş sütunu", "b"),
                ParamDef::new(
                    "how",
                    "İşlem",
                    ParamKind::Choice {
                        options: vec![
                            EnumOption::new("intersect", "Kesişim")
                                .hint("İki tablonun da kapladığı parçalar"),
                            EnumOption::new("union", "Birleşim")
                                .hint("Herhangi birinin kapladığı parçalar"),
                        ],
                    },
                )
                .default_value(json!("intersect")),
            ],
        ]
        .concat(),
        outputs: vec![
            OutputDef::new("count", "Parça sayısı", OutputKind::Number),
            OutputDef::new("table", "Parçalar", OutputKind::Table),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_overlay),
    }
}

/// One side of an overlay: its events, the rows they came from, its header and its other columns.
struct Side {
    events: Vec<Event>,
    kept: Vec<Vec<String>>,
    header: Vec<String>,
    others: Vec<String>,
}

fn run_overlay(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let mut sides = Vec::with_capacity(2);
    let mut dropped = 0usize;
    for prefix in ["a", "b"] {
        let t = event_table(
            v,
            &format!("{prefix}SourceKind"),
            &format!("{prefix}File"),
            &format!("{prefix}Source"),
        );
        let keys = [
            v.text(&format!("{prefix}Route")).to_owned(),
            v.text(&format!("{prefix}From")).to_owned(),
            v.text(&format!("{prefix}To")).to_owned(),
        ];
        for c in &keys {
            if !t.header.contains(c) {
                return RunResult::refused(missing(c, &t.header));
            }
        }
        let at: Vec<usize> = keys
            .iter()
            .map(|c| t.header.iter().position(|h| h == c).unwrap_or(0))
            .collect();
        let mut side = Side {
            events: Vec::new(),
            kept: Vec::new(),
            header: t.header.clone(),
            others: t
                .header
                .iter()
                .filter(|h| !keys.contains(h))
                .cloned()
                .collect(),
        };
        for r in t.rows {
            let cell = |k: usize| r.get(k).map_or("", String::as_str);
            let id = crate::text::js_trim(cell(at[0])).to_owned();
            let (from, to) = (read_measure(cell(at[1])), read_measure(cell(at[2])));
            let (Some(from), Some(to)) = (from, to) else {
                dropped += 1;
                continue;
            };
            if id.is_empty() {
                dropped += 1;
                continue;
            }
            side.events.push(Event {
                route: id,
                from,
                to,
            });
            side.kept.push(r);
        }
        sides.push(side);
    }
    if dropped > 0 {
        feedback.warn(format!(
            "{dropped} satırın rotası ya da ölçüsü yok; alınmadı."
        ));
    }
    let (a, b) = (&sides[0], &sides[1]);
    let pieces = overlay(&a.events, &b.events, v.text("how") == "union");
    let mut columns: Vec<String> = ["Rota", "Başlangıç", "Bitiş"].map(String::from).to_vec();
    columns.extend(a.others.iter().cloned());
    columns.extend(b.others.iter().map(|h| {
        if a.others.contains(h) {
            format!("{h} (2)")
        } else {
            h.clone()
        }
    }));
    let rows: Vec<Value> = pieces
        .iter()
        .map(|g| {
            let mut row = vec![
                g.route.clone(),
                length_text(ctx.units, g.from),
                length_text(ctx.units, g.to),
            ];
            for (idx, side) in [(g.a, a), (g.b, b)] {
                for h in &side.others {
                    let k = side.header.iter().position(|x| x == h).unwrap_or(0);
                    row.push(
                        idx.and_then(|i| side.kept.get(i))
                            .and_then(|r| r.get(k))
                            .cloned()
                            .unwrap_or_default(),
                    );
                }
            }
            json!(row)
        })
        .collect();
    let count = rows.len();
    RunResult {
        outputs: [
            ("count".to_owned(), json!(count)),
            (
                "table".to_owned(),
                json!({ "columns": columns, "rows": rows }),
            ),
        ]
        .into_iter()
        .collect(),
        summary: Some(format!(
            "{} ve {} olaydan {count} parça.",
            a.events.len(),
            b.events.len()
        )),
        ..RunResult::default()
    }
}
