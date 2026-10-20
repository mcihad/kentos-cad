//! Rota boyunca konumla (the web's `builtin/lrs/locate.ts`; docs/adr/0224
//! §5; ArcGIS Locate Features Along Routes): each object's places along the
//! routes within a radius, as a table and, when asked, the first place
//! written into the object's attributes. The search is the core's
//! (`ops::lrs::locate_run`, its routes' edges in a packed R-tree).

use kentos_commands::Entity;
use kentos_domain::Slot;
use kentos_geometry_core::ops::lrs::locate_run;
use kentos_native_application::geometry::shape;
use serde_json::{Value, json};

use super::{ROUTE_KINDS, again_text, features, field, number, routes_by_id, side_param};
use crate::builtin::proximity::length_text;
use crate::builtin::queries::with_attr;
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Patch, Resolved, RunContext,
    RunResult, Target, Tool,
};

/// What can be located: points, lines and areas.
const LOCATE_KINDS: [&str; 5] = ["point", "line", "polyline", "arc", "polygon"];

pub fn locate_along() -> Tool {
    Tool {
        id: "linearRef.locateAlong".into(),
        label: "Rota boyunca konumla".into(),
        category: "linearRef".into(),
        description: "Nesnelerin rotalardaki yerini ölçüleriyle bulur: noktanın ölçüsü ve uzaklığı, çizginin ve alanın başlangıç ve bitiş ölçüsü.".into(),
        help: Some([
            "Örnekler: kazaların hangi kilometrede olduğu, bir kaplama çizgisinin yolun hangi kesimine düştüğü, bir parselin yolun hangi kesimini kapladığı.",
            "Nokta rotadaki en yakın yerine konumlanır; arama yarıçapı içinde değilse konumlanmaz. Uzaklık ölçü yönünün sağına artıdır (ya da seçilirse soluna). Yarıçaptaki bütün rotalar istenirse her biri bir satırdır, istenmezse yalnız en yakını. Çizgi bütün köşeleri yarıçap içindeyse ilk ve son köşesinin ölçüleriyle, alan içinden geçen rotaların parçalarıyla konumlanır.",
            "Sonuç tablodadır. Özniteliklere yaz açıkken ilk satır nesnenin Rota, Ölçü ya da Başlangıç ve Bitiş, Uzaklık alanlarına yazılır.",
        ]
        .join("\n\n")),
        keywords: [
            "konumla", "locate features along routes", "rota", "ölçü", "kilometre",
            "doğrusal referans", "olay tablosu",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["ROTAKONUM", "LOCATEALONG"].map(String::from).to_vec(),
        icon: Some("routeLocate".into()),
        parameters: vec![
            features("input", "Konumlanacak nesneler", &LOCATE_KINDS, true),
            features("routes", "Rotalar", &ROUTE_KINDS, false),
            field("routeField", "Rota kimliği alanı", &["routes"]),
            number("radius", "Arama yarıçapı", Some(0.0), "m").default_value(json!(10)),
            ParamDef::new("all", "Yarıçaptaki bütün rotalar", ParamKind::Boolean)
                .default_value(json!(false))
                .describe("Kapalıyken yalnız en yakın rota."),
            side_param("Artı uzaklık"),
            ParamDef::new("write", "Özniteliklere yaz", ParamKind::Boolean)
                .default_value(json!(false))
                .describe("İlk satırı Rota, Ölçü (Başlangıç, Bitiş) ve Uzaklık alanlarına yazar."),
        ],
        outputs: vec![
            OutputDef::new("count", "Konumlanan nesne sayısı", OutputKind::Number),
            OutputDef::new("table", "Konumlar", OutputKind::Table),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let routes = routes_by_id(&v.features("routes").entities, v.text("routeField"));
    if routes.again > 0 {
        feedback.warn(again_text(routes.again));
    }
    let objects = &v.features("input").entities;
    feedback.progress(0.0, "Rotalar aranıyor");
    let shapes: Vec<_> = objects.iter().map(|e| shape(e)).collect();
    let found = locate_run(
        &routes.routes,
        &shapes,
        v.number("radius").unwrap_or(10.0),
        v.flag("all"),
    );
    let turn = if v.text("side") == "left" { -1.0 } else { 1.0 };
    let text = |n: Option<f64>| n.map(|n| length_text(ctx.units, n)).unwrap_or_default();
    let write = v.flag("write");
    let mut rows: Vec<Value> = Vec::new();
    let mut update = Vec::new();
    let mut located = 0usize;
    for (e, places) in objects.iter().zip(&found) {
        if places.is_empty() {
            continue;
        }
        located += 1;
        // A point's distance is signed (+ right of the measure direction); a line's farthest vertex's is not.
        let point = matches!(e, Entity::Point(_));
        let signed = |d: Option<f64>| d.map(|d| if point { d * turn } else { d });
        for q in places {
            rows.push(json!([
                format!("#{}", e.base().id),
                routes.ids[q.route],
                text(q.m),
                text(q.from),
                text(q.to),
                text(signed(q.distance)),
            ]));
        }
        if !write {
            continue;
        }
        let q = &places[0];
        let mut current: Entity = (*e).clone();
        let mut changed = false;
        let mut put = |name: &str, value: String| {
            if let Some(attrs) = with_attr(ctx, &current, name, Some(&value)) {
                current.base_mut().attrs = attrs;
                changed = true;
            }
        };
        put("Rota", routes.ids[q.route].clone());
        if q.m.is_some() {
            put("Ölçü", text(q.m));
        }
        if q.from.is_some() && q.to.is_some() {
            put("Başlangıç", text(q.from));
            put("Bitiş", text(q.to));
        }
        if q.distance.is_some() {
            put("Uzaklık", text(signed(q.distance)));
        }
        if changed {
            update.push(Patch {
                id: Slot(e.base().id),
                attrs: Some(current.base().attrs.clone()),
                label: None,
                zs: None,
                geometry: None,
                layer: None,
            });
        }
    }
    let count = rows.len();
    RunResult {
        changes: Some(ChangeSet {
            update,
            ..ChangeSet::default()
        }),
        outputs: [
            ("count".to_owned(), json!(located)),
            (
                "table".to_owned(),
                json!({ "columns": ["Nesne", "Rota", "Ölçü", "Başlangıç", "Bitiş", "Uzaklık"], "rows": rows }),
            ),
        ]
        .into_iter()
        .collect(),
        summary: Some(format!(
            "{} nesneden {located} nesne konumlandı; {count} satır.",
            objects.len()
        )),
        ..RunResult::default()
    }
}
