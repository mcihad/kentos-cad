//! Olay kaymasını denetle (docs/adr/0526): event objects that Olayları rotada göster wrote keep their route's id
//! and their measures in their attributes (the row's cells); this tool places each again on its route as the route
//! is now (its geometry and its M), says how far the object lies from that place and, when asked, moves it there.
//! The bond is the route id: the routes' id field, as Olayları rotada göster's. The drift is the core's
//! (`ops::lrs::point_drift`, `line_drift`).

use std::collections::BTreeMap;

use kentos_commands::{Entity, LineEntity, PointEntity, Vec2 as Point};
use kentos_domain::Slot;
use kentos_geometry_core::Vec2;
use kentos_geometry_core::ops::lrs::{EventIn, RoutePart, events_run, line_drift, point_drift};
use serde_json::{Value, json};

use super::events::fault_text;
use super::{
    ROUTE_KINDS, again_text, cell_reason, features, field, number, polyline_of, read_measure,
    route_of, routes_by_id, side_param, text_of,
};
use crate::builtin::proximity::length_text;
use crate::types::{
    ChangeSet, EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Patch, Resolved,
    RunContext, RunResult, Target, Tool, Values,
};

/// What an event object is: a point, or a line or a polyline.
const EVENT_KINDS: [&str; 3] = ["point", "line", "polyline"];

fn point_kind(v: &Values) -> bool {
    text_of(v, "eventKind") != "line"
}
fn line_kind(v: &Values) -> bool {
    text_of(v, "eventKind") == "line"
}

pub fn event_drift() -> Tool {
    Tool {
        id: "linearRef.eventDrift".into(),
        label: "Olay kaymasını denetle".into(),
        category: "linearRef".into(),
        description: "Rotaların geometrisi ya da ölçüleri değişince olay nesnelerinin rotanın verdiği yerden ne kadar kaydığını raporlar; istenirse yerlerine taşır.".into(),
        help: Some([
            "Örnekler: yol ekseni düzeltildikten sonra kaza noktalarının ve kaplama kesimlerinin yeni eksene oturup oturmadığı, rota yeniden kalibre edilince levhaların kilometrelerinin yerinde kalıp kalmadığı.",
            "Olay nesnesi rotasını ve ölçüsünü kendi özniteliklerinde taşır (Olayları rotada göster satırın dolu hücrelerini öyle yazar). Araç her olayı rotanın bugünkü hâliyle yeniden yerleştirir: nokta olayının kayması iki noktanın uzaklığı, çizgi olayınınki birinin köşelerinin ve yay ortalarının öbürünün yoluna en büyük uzaklığıdır. Toleranstan çok kayanlar tabloda.",
            "Rotası olmayan, ölçüsü sayı olmayan ya da ölçüsü rotanın dışında kalan olay gösterilemez; tabloda nedeniyle söylenir. Yerine taşı açıkken kayan olaylar rotanın verdiği yere taşınır (tek adımda); çizgi nesnesi kırıklı yola taşınamaz, söylenir. Olay türünde olmayan nesneler sayılır, denetlenmez.",
        ]
        .join("\n\n")),
        keywords: [
            "olay", "kayma", "drift", "rota", "ölçü", "kilometre", "doğrusal referans",
            "event behavior", "denetle",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["OLAYKAYMA", "EVENTDRIFT"].map(String::from).to_vec(),
        icon: Some("routeDrift".into()),
        parameters: vec![
            features("events", "Olaylar", &EVENT_KINDS, true),
            features("routes", "Rotalar", &ROUTE_KINDS, false),
            field("routeField", "Rota kimliği alanı", &["routes"]),
            field("eventRoute", "Olayın rota alanı", &["events"]),
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
            field("measure", "Ölçü alanı", &["events"]).shown_when(point_kind),
            field("from", "Başlangıç ölçüsü alanı", &["events"]).shown_when(line_kind),
            field("to", "Bitiş ölçüsü alanı", &["events"]).shown_when(line_kind),
            field("offset", "Öteleme alanı", &["events"])
                .optional()
                .describe("Rotaya dik uzaklık (m); boşsa olay rotanın üstündedir."),
            side_param("Artı öteleme"),
            number("tolerance", "Tolerans", Some(0.0), "m")
                .default_value(json!(0.001))
                .describe("Bundan az kayan olay yerinde sayılır."),
            ParamDef::new("fix", "Yerine taşı", ParamKind::Boolean)
                .default_value(json!(false))
                .describe("Kayan olayları rotanın verdiği yere taşır."),
        ],
        outputs: vec![
            OutputDef::new("count", "Kayan olay sayısı", OutputKind::Number),
            OutputDef::new("table", "Kayan ve gösterilemeyen olaylar", OutputKind::Table),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn p(q: Vec2) -> Point {
    Point { x: q.x, y: q.y }
}

/// An event object's shape as a route's parts (a point none).
fn parts_of(e: &Entity) -> Option<Vec<RoutePart>> {
    route_of(e).map(|r| r.parts)
}

/// The object moved to its place, of its own kind; none when a line cannot take a bent path.
fn moved(e: &Entity, at: Option<Vec2>, parts: Option<&[RoutePart]>) -> Option<Entity> {
    match (e, at, parts) {
        (Entity::Point(q), Some(at), _) => Some(Entity::Point(PointEntity {
            p: p(at),
            ..q.clone()
        })),
        (Entity::Line(l), _, Some([one])) => {
            let straight = one
                .bulges
                .as_ref()
                .is_none_or(|b| b.iter().all(|&x| x == 0.0));
            if one.pts.len() != 2 || !straight {
                return None;
            }
            Some(Entity::Line(LineEntity {
                a: p(one.pts[0]),
                b: p(one.pts[1]),
                za: None,
                zb: None,
                ma: one.ms.first().copied().flatten(),
                mb: one.ms.get(1).copied().flatten(),
                ..l.clone()
            }))
        }
        (Entity::Polyline(_), _, Some(parts)) => {
            Some(polyline_of(parts, &e.base().layer_id, BTreeMap::new()))
        }
        _ => None,
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let routes = routes_by_id(&v.features("routes").entities, v.text("routeField"));
    if routes.again > 0 {
        feedback.warn(again_text(routes.again));
    }
    let line = v.text("eventKind") == "line";
    let mut fields = vec![v.text("eventRoute").to_owned()];
    if line {
        fields.push(v.text("from").to_owned());
        fields.push(v.text("to").to_owned());
    } else {
        fields.push(v.text("measure").to_owned());
    }
    let offset_field = crate::text::js_trim(v.text("offset")).to_owned();
    let turn = if v.text("side") == "left" { -1.0 } else { 1.0 };
    let tolerance = v.number("tolerance").unwrap_or(0.001);
    let fix = v.flag("fix");
    let objects = &v.features("events").entities;
    // The objects of the event kind, and why each gives no event (in their order).
    let mut checked: Vec<(usize, &Entity)> = Vec::new();
    let mut events = Vec::new();
    // The table's rows by the object's place in the input.
    let mut rows: Vec<(usize, Value)> = Vec::new();
    let mut other = 0usize;
    let mut unplaced = 0usize;
    // Of them, those whose own attributes give no event.
    let mut unread = 0usize;
    for (at, e) in objects.iter().enumerate() {
        let point = matches!(e, Entity::Point(_));
        if point == line {
            other += 1;
            continue;
        }
        let attrs = &e.base().attrs;
        let cell = |name: &str| attrs.get(name).map_or("", String::as_str);
        let id = crate::text::js_trim(cell(&fields[0])).to_owned();
        let mut nums = Vec::new();
        let mut why: Option<String> = None;
        for f in &fields[1..] {
            match read_measure(cell(f)) {
                Some(n) => nums.push(n),
                None => {
                    why = Some(cell_reason(f, cell(f)));
                    break;
                }
            }
        }
        let mut offset = 0.0;
        if why.is_none()
            && !offset_field.is_empty()
            && !crate::text::js_trim(cell(&offset_field)).is_empty()
        {
            match read_measure(cell(&offset_field)) {
                Some(n) => offset = n,
                None => why = Some(cell_reason(&offset_field, cell(&offset_field))),
            }
        }
        let route = routes.index.get(id.as_str()).copied();
        if why.is_none() && route.is_none() {
            why = Some(format!("“{id}” rotası yok"));
        }
        if let Some(why) = why {
            unread += 1;
            unplaced += 1;
            rows.push((at, json!([format!("#{}", e.base().id), id, "", why])));
            continue;
        }
        checked.push((at, e));
        events.push(EventIn {
            route,
            m: (!line).then(|| nums[0]),
            from: line.then(|| nums[0]),
            to: line.then(|| nums[1]),
            offset: Some(offset * turn),
        });
    }
    if other > 0 {
        feedback.info(format!("{other} nesne olay türünde değil; denetlenmedi."));
    }
    feedback.progress(0.0, "Olaylar rotalara yerleştiriliyor");
    let placed = events_run(&routes.routes, &events);
    let mut drifted = 0usize;
    let mut update = Vec::new();
    for (((at, e), event), out) in checked.iter().zip(&events).zip(&placed) {
        let (at, e) = (*at, *e);
        let label = format!("#{}", e.base().id);
        let rid = event
            .route
            .map(|r| routes.ids[r].clone())
            .unwrap_or_default();
        if let Some(f) = &out.fault {
            unplaced += 1;
            rows.push((at, json!([label, rid, "", fault_text(ctx.units, f)])));
            continue;
        }
        let drift = match (e, out.p, &out.parts) {
            (Entity::Point(q), Some(at), _) => point_drift(Vec2 { x: q.p.x, y: q.p.y }, at),
            (_, _, Some(parts)) => parts_of(e).map_or(f64::INFINITY, |now| line_drift(&now, parts)),
            _ => continue,
        };
        if drift <= tolerance {
            continue;
        }
        drifted += 1;
        let mut said = "Kaydı".to_owned();
        if fix {
            match moved(e, out.p, out.parts.as_deref()) {
                Some(shape) => {
                    said = "Yerine taşındı".into();
                    update.push(Patch {
                        id: Slot(e.base().id),
                        attrs: None,
                        label: None,
                        zs: None,
                        geometry: Some(Box::new(shape)),
                        layer: None,
                    });
                }
                None => said = "Kaydı; çizgi nesnesi kırıklı yola taşınamaz".into(),
            }
        }
        rows.push((at, json!([label, rid, length_text(ctx.units, drift), said])));
    }
    let total = checked.len() + unread;
    let mut summary = format!("{total} olaydan {drifted} olay kaymış");
    if unplaced > 0 {
        summary.push_str(&format!(", {unplaced} olay rotasında gösterilemiyor"));
    }
    if fix && !update.is_empty() {
        summary.push_str(&format!("; {} olay yerine taşındı", update.len()));
    }
    summary.push('.');
    if drifted > 0 && !fix {
        feedback.warn(format!(
            "{drifted} olay rotanın verdiği yerden kaymış; Yerine taşı ile düzeltilir."
        ));
    }
    let mut outputs: BTreeMap<String, Value> =
        [("count".to_owned(), json!(drifted))].into_iter().collect();
    rows.sort_by_key(|(at, _)| *at);
    let rows: Vec<Value> = rows.into_iter().map(|(_, r)| r).collect();
    if !rows.is_empty() {
        outputs.insert(
            "table".into(),
            json!({ "columns": ["Nesne", "Rota", "Kayma", "Durum"], "rows": rows }),
        );
    }
    RunResult {
        changes: Some(ChangeSet {
            update,
            ..ChangeSet::default()
        }),
        outputs: outputs.into_iter().collect(),
        summary: Some(summary),
        ..RunResult::default()
    }
}
