//! Rota oluştur, Rotayı kalibre et and M değerlerini çıkar (the web's
//! `builtin/lrs/routes.ts`; docs/adr/0224 §5; ArcGIS Create Routes and
//! Calibrate Routes, QGIS Extract M values).

use std::collections::BTreeMap;

use kentos_contracts::{Entity, LineEntity, Vec2 as Point};
use kentos_domain::Slot;
use kentos_geometry_core::Vec2;
use kentos_geometry_core::jsmath::{TAU, cos, sin, tan};
use kentos_geometry_core::ops::elevation::{Carry, carry_elevations};
use kentos_geometry_core::ops::lrs::{
    CalibrateRules, CreateRules, Mark, Method, Piece, PieceFault, Priority, RoutePart, calibrate,
    create, summary,
};
use kentos_native_application::elevation;
use serde_json::{Value, json};

use super::{
    ROUTE_KINDS, again_text, cell_reason, features, field, number, polyline_of, read_measure,
    route_of, routes_by_id, text_of,
};
use crate::builtin::geometry::layer_param;
use crate::builtin::proximity::length_text;
use crate::builtin::queries::with_attr;
use crate::types::{
    ChangeSet, EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Patch, Resolved,
    RunContext, RunResult, Target, Tool, Values,
};

fn method_of(v: &Values) -> &str {
    text_of(v, "method")
}
fn by_fields(v: &Values) -> bool {
    method_of(v) != "twoFields"
}
fn one_field(v: &Values) -> bool {
    method_of(v) == "oneField"
}
fn two_fields(v: &Values) -> bool {
    method_of(v) == "twoFields"
}

pub fn create_routes() -> Tool {
    Tool {
        id: "linearRef.createRoutes".into(),
        label: "Rota oluştur".into(),
        category: "linearRef".into(),
        description: "Çizgileri kimliklerine göre uç uca birleştirip köşelerine ölçü (M) veren rotalar yazar.".into(),
        help: Some([
            "Örnekler: yol parçalarından karayolu rotaları, boru hattı parçalarından hat rotaları, kanal ve dere eksenlerinden ölçülü rotalar.",
            "Aynı kimlikli çizgiler bir rotadır. Uzunluk ve Tek alan yöntemlerinde parçalar başlangıç köşesine en yakın uçtan başlayarak uç uca eklenir; ters çizilen parça çevrilir; değmeyen parçalar rotanın ayrı parçalarıdır ve aralarındaki boşluk istenirse ölçüye katılır. Ölçü çizginin uzunluğundan (Uzunluk) ya da her parçanın kendi uzunluk alanından (Tek alan) gelir; çarpanla çarpılıp başlangıç ölçüsüne eklenir. İki alan yönteminde her parça başlangıç ve bitiş ölçüsü alanlarıyla, başlangıcına göre sırada yazılır.",
            "Yaylar yay olarak kalır; ölçü yay boyunca açıyla doğrusaldır. Kimliği boş, ölçü alanı sayı olmayan nesne atlanır ve tabloda nedeniyle söylenir. Rotalar yeni katmana çoklu çizgi olarak yazılır.",
        ]
        .join("\n\n")),
        keywords: [
            "rota", "route", "doğrusal referans", "linear referencing", "ölçü", "M değeri", "kilometre",
            "create routes",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["ROTAOLUSTUR", "CREATEROUTES"].map(String::from).to_vec(),
        icon: Some("routeCreate".into()),
        parameters: vec![
            features("input", "Girdi çizgileri", &["line", "polyline", "arc"], false)
                .describe("Rotaların parçaları: çizgiler, çoklu çizgiler ve yaylar."),
            field("idField", "Rota kimliği alanı", &["input"])
                .describe("Aynı değerli parçalar bir rotadır."),
            ParamDef::new(
                "method",
                "Ölçü kaynağı",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("length", "Uzunluk").hint("Çizginin uzunluğu"),
                        EnumOption::new("oneField", "Tek alan").hint("Her parçanın uzunluk alanı"),
                        EnumOption::new("twoFields", "İki alan")
                            .hint("Her parçanın başlangıç ve bitiş ölçüsü"),
                    ],
                },
            )
            .default_value(json!("length")),
            field("lengthField", "Uzunluk alanı", &["input"])
                .shown_when(one_field)
                .describe("Parçanın rotadaki uzunluğu."),
            field("fromField", "Başlangıç ölçüsü alanı", &["input"]).shown_when(two_fields),
            field("toField", "Bitiş ölçüsü alanı", &["input"]).shown_when(two_fields),
            ParamDef::new(
                "priority",
                "Başlangıç köşesi",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("upperLeft", "Sol üst"),
                        EnumOption::new("lowerLeft", "Sol alt"),
                        EnumOption::new("upperRight", "Sağ üst"),
                        EnumOption::new("lowerRight", "Sağ alt"),
                    ],
                },
            )
            .default_value(json!("upperLeft"))
            .shown_when(by_fields)
            .describe("Ölçü, rotanın kutusunun bu köşesine en yakın uçtan başlar."),
            number("factor", "Ölçü çarpanı", None, "")
                .default_value(json!(1))
                .shown_when(by_fields)
                .describe("Uzunluk bununla çarpılır: 0,001 ile metreden kilometreye."),
            number("start", "Başlangıç ölçüsü", None, "")
                .default_value(json!(0))
                .shown_when(by_fields),
            ParamDef::new("ignoreGaps", "Boşlukları yok say", ParamKind::Boolean)
                .default_value(json!(true))
                .shown_when(by_fields)
                .describe("Kapalıyken parçalar arası boşluk ölçüye katılır."),
            layer_param("Rotalar", "#E5484D"),
        ],
        outputs: vec![
            OutputDef::new("count", "Yazılan rota sayısı", OutputKind::Number),
            OutputDef::new("table", "Atlanan nesneler", OutputKind::Table),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_create),
    }
}

/// A piece object's paths: a line's ends, a polyline's own part and its
/// parts, an arc from its start to its end (counter-clockwise).
fn piece_paths(e: &Entity) -> Vec<(Vec<Vec2>, Option<Vec<f64>>)> {
    let v = |q: &Point| Vec2::new(q.x, q.y);
    match e {
        Entity::Line(l) => vec![(vec![v(&l.a), v(&l.b)], None)],
        Entity::Polyline(q) => std::iter::once((q.pts.iter().map(v).collect(), q.bulges.clone()))
            .chain(
                q.parts
                    .iter()
                    .flatten()
                    .map(|r| (r.pts.iter().map(v).collect(), r.bulges.clone())),
            )
            .collect(),
        Entity::Arc(a) => {
            let mut sweep = a.a1 - a.a0;
            if sweep <= 0.0 {
                sweep += TAU;
            }
            let at = |t: f64| Vec2::new(a.c.x + a.r * cos(t), a.c.y + a.r * sin(t));
            vec![(vec![at(a.a0), at(a.a1)], Some(vec![tan(sweep / 4.0)]))]
        }
        _ => Vec::new(),
    }
}

fn run_create(v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let objects = &v.features("input").entities;
    let id_field = v.text("idField");
    let method = match v.text("method") {
        "oneField" => Method::OneField,
        "twoFields" => Method::TwoFields,
        _ => Method::Length,
    };
    let cell = |e: &Entity, f: &str| -> String {
        if f.is_empty() {
            String::new()
        } else {
            e.base().attrs.get(f).cloned().unwrap_or_default()
        }
    };
    let mut pieces = Vec::new();
    let mut owner: Vec<&Entity> = Vec::new();
    for e in objects.iter().copied() {
        let id = crate::text::js_trim(&cell(e, id_field)).to_owned();
        let value = (method == Method::OneField)
            .then(|| read_measure(&cell(e, v.text("lengthField"))))
            .flatten();
        let from = (method == Method::TwoFields)
            .then(|| read_measure(&cell(e, v.text("fromField"))))
            .flatten();
        let to = (method == Method::TwoFields)
            .then(|| read_measure(&cell(e, v.text("toField"))))
            .flatten();
        for (pts, bulges) in piece_paths(e) {
            pieces.push(Piece {
                id: id.clone(),
                pts,
                bulges,
                value,
                from,
                to,
            });
            owner.push(e);
        }
    }
    let priority = match v.text("priority") {
        "lowerLeft" => Priority::LowerLeft,
        "upperRight" => Priority::UpperRight,
        "lowerRight" => Priority::LowerRight,
        _ => Priority::UpperLeft,
    };
    let made = create(
        &pieces,
        &CreateRules {
            method,
            priority: Some(priority),
            factor: Some(v.number("factor").unwrap_or(1.0)),
            start: Some(v.number("start").unwrap_or(0.0)),
            ignore_gaps: Some(v.flag("ignoreGaps")),
        },
    );
    let layer = &v.layer("layer").id;
    let add: Vec<Entity> = made
        .routes
        .iter()
        .map(|r| {
            polyline_of(
                &r.parts,
                layer,
                [(id_field.to_owned(), r.id.clone())].into_iter().collect(),
            )
        })
        .collect();
    let mut seen = std::collections::HashSet::new();
    let mut report: Vec<Value> = Vec::new();
    for f in &made.faults {
        let e = owner[f.piece];
        if !seen.insert(e.base().id) {
            continue;
        }
        let why = match f.why {
            PieceFault::NoId => format!("“{id_field}” boş"),
            PieceFault::NoValue => {
                cell_reason(v.text("lengthField"), &cell(e, v.text("lengthField")))
            }
            PieceFault::NoFromTo => {
                let bad = if read_measure(&cell(e, v.text("fromField"))).is_none() {
                    v.text("fromField")
                } else {
                    v.text("toField")
                };
                cell_reason(bad, &cell(e, bad))
            }
            PieceFault::NoPath => "İki köşesi yok".into(),
        };
        report.push(json!([format!("#{}", e.base().id), why]));
    }
    if !report.is_empty() {
        feedback.warn(format!(
            "{} nesne atlandı; nedenleri tabloda.",
            report.len()
        ));
    }
    let count = add.len();
    let mut outputs: BTreeMap<String, Value> =
        [("count".to_owned(), json!(count))].into_iter().collect();
    if !report.is_empty() {
        outputs.insert(
            "table".into(),
            json!({ "columns": ["Nesne", "Neden"], "rows": report }),
        );
    }
    RunResult {
        changes: Some(ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs: outputs.into_iter().collect(),
        summary: Some(format!("{} nesneden {count} rota yazıldı.", objects.len())),
        ..RunResult::default()
    }
}

pub fn calibrate_routes() -> Tool {
    Tool {
        id: "linearRef.calibrateRoutes".into(),
        label: "Rotayı kalibre et".into(),
        category: "linearRef".into(),
        description: "Rotaların ölçülerini (M) üzerlerindeki ölçüsü bilinen noktalardan yeniden verir.".into(),
        help: Some([
            "Örnekler: kilometre taşlarının ölçüleriyle karayolu rotası, vana ve muayene bacalarının bilinen kilometreleriyle boru hattı.",
            "Her rotanın noktaları, nokta rota alanı rotanın kimliğine eşit olanlardır; arama yarıçapındaki nokta rotadaki en yakın yerine konur. En az iki nokta gerekir. Çoklu çizgide noktaların yerine köşe eklenir; çizgi iki köşesini korur, uçlarının ölçüsü verilir. Her köşenin ölçüsü çevresindeki iki noktanın arasında rota uzunluğuyla doğrusaldır; ilk noktadan önce ve sonuncudan sonra istenirse uç aralığın oranıyla uzatılır, istenmezse eski ölçüsü kalır.",
            "Rotalar yerinde, tek adımda değişir; yeni köşenin kotu kenarı boyunca doğrusaldır. Tabloda her rotanın kullanılan nokta sayısı ve durumu söylenir.",
        ]
        .join("\n\n")),
        keywords: ["kalibrasyon", "calibrate routes", "rota", "ölçü", "M değeri", "kilometre taşı"]
            .map(String::from)
            .to_vec(),
        aliases: ["ROTAKALIBRE", "CALIBRATEROUTES"].map(String::from).to_vec(),
        icon: Some("routeCalibrate".into()),
        parameters: vec![
            features("routes", "Rotalar", &ROUTE_KINDS, true)
                .describe("Ölçüleri yeniden verilecek çizgi ve çoklu çizgiler."),
            field("routeField", "Rota kimliği alanı", &["routes"]),
            features("points", "Kalibrasyon noktaları", &["point"], false)
                .describe("Ölçüsü bilinen noktalar."),
            field("pointRoute", "Noktanın rota alanı", &["points"]),
            field("measure", "Ölçü alanı", &["points"]),
            number("radius", "Arama yarıçapı", Some(0.0), "m")
                .default_value(json!(1))
                .describe("Rotaya bundan uzak nokta alınmaz."),
            ParamDef::new("before", "İlk noktadan öncesini uzat", ParamKind::Boolean)
                .default_value(json!(true)),
            ParamDef::new("after", "Son noktadan sonrasını uzat", ParamKind::Boolean)
                .default_value(json!(true)),
            ParamDef::new("ignoreGaps", "Boşlukları yok say", ParamKind::Boolean)
                .default_value(json!(true))
                .describe("Kapalıyken rotanın parçaları arası boşluk uzunluğa katılır."),
        ],
        outputs: vec![
            OutputDef::new("count", "Kalibre edilen rota sayısı", OutputKind::Number),
            OutputDef::new("table", "Rotalar", OutputKind::Table),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_calibrate),
    }
}

/// A polyline's elevations carried to a calibrated route's vertices (a new
/// one linear along its edge), path by path; none when it has none.
fn carried(e: &Entity, parts: &[RoutePart]) -> Option<Vec<Vec<Option<f64>>>> {
    let before = elevation::paths(e);
    if !elevation::elevated(&before) {
        return None;
    }
    Some(
        parts
            .iter()
            .map(|q| carry_elevations(&q.pts, false, None, &before, Carry::Along))
            .collect(),
    )
}

fn run_calibrate(
    v: &Resolved<'_>,
    _ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
) -> RunResult {
    let routes = routes_by_id(&v.features("routes").entities, v.text("routeField"));
    if routes.again > 0 {
        feedback.warn(again_text(routes.again));
    }
    let (point_route, measure) = (v.text("pointRoute"), v.text("measure"));
    let mut bad = 0usize;
    let mut marks: Vec<Vec<Mark>> = vec![Vec::new(); routes.ids.len()];
    for s in &v.features("points").entities {
        let Entity::Point(pt) = s else {
            continue;
        };
        let id = crate::text::js_trim(s.base().attrs.get(point_route).map_or("", String::as_str));
        if id.is_empty() {
            continue;
        }
        let Some(m) = read_measure(s.base().attrs.get(measure).map_or("", String::as_str)) else {
            bad += 1;
            continue;
        };
        if let Some(&k) = routes.index.get(id) {
            marks[k].push(Mark {
                p: Vec2::new(pt.p.x, pt.p.y),
                m,
            });
        }
    }
    if bad > 0 {
        feedback.warn(format!("{bad} noktanın ölçüsü sayı değil; kullanılmadı."));
    }
    let mut update = Vec::new();
    let mut rows: Vec<Value> = Vec::new();
    let (mut done, mut used_all) = (0usize, 0usize);
    for (k, id) in routes.ids.iter().enumerate() {
        let e = routes.objects[k];
        // A line keeps its two vertices; a polyline takes one where each point falls.
        let rules = CalibrateRules {
            radius: v.number("radius").unwrap_or(1.0),
            before: Some(v.flag("before")),
            after: Some(v.flag("after")),
            ignore_gaps: Some(v.flag("ignoreGaps")),
            split: Some(!matches!(e, Entity::Line(_))),
        };
        let (parts, used) = match calibrate(&routes.routes[k], &marks[k], &rules) {
            Ok(c) => (Some(c.parts), c.used),
            Err((_, used)) => (None, used),
        };
        let n = used.iter().filter(|u| **u).count();
        let Some(parts) = parts else {
            rows.push(json!([id, n.to_string(), "İkiden az nokta"]));
            continue;
        };
        done += 1;
        used_all += n;
        rows.push(json!([id, n.to_string(), "Kalibre edildi"]));
        let geometry = match e {
            Entity::Line(l) => {
                let ms = parts.first().map(|q| q.ms.clone()).unwrap_or_default();
                Entity::Line(LineEntity {
                    ma: ms.first().copied().flatten(),
                    mb: ms.get(1).copied().flatten(),
                    ..l.clone()
                })
            }
            _ => polyline_of(&parts, &e.base().layer_id, e.base().attrs.clone()),
        };
        update.push(Patch {
            id: Slot(e.base().id),
            attrs: None,
            label: None,
            zs: carried(e, &parts),
            geometry: Some(Box::new(geometry)),
            layer: None,
        });
    }
    RunResult {
        changes: Some(ChangeSet {
            update,
            ..ChangeSet::default()
        }),
        outputs: [
            ("count".to_owned(), json!(done)),
            (
                "table".to_owned(),
                json!({ "columns": ["Rota", "Kullanılan nokta", "Durum"], "rows": rows }),
            ),
        ]
        .into_iter()
        .collect(),
        summary: Some(format!(
            "{} rotadan {done} rota kalibre edildi; {used_all} nokta kullanıldı.",
            routes.ids.len()
        )),
        ..RunResult::default()
    }
}

pub fn extract_measures() -> Tool {
    Tool {
        id: "linearRef.extractMeasures".into(),
        label: "M değerlerini çıkar".into(),
        category: "linearRef".into(),
        description: "Rotaların ilk, son, en küçük ve en büyük ölçüsünü (M) özniteliklerine yazar.".into(),
        help: Some([
            "Örnekler: her yol rotasının başlangıç ve bitiş kilometresi, ölçü aralıklarının bir tabloda listelenmesi, ölçüsüz rotaların bulunması.",
            "Yazılan alanlar önekle adlanır: “<önek>bas” ilk ölçülü köşenin, “<önek>son” sonuncunun ölçüsü, “<önek>min” en küçüğü, “<önek>max” en büyüğü; sayılar projenin uzunluk basamağıyla. Ölçüsü olmayan nesneye yazılmaz.",
        ]
        .join("\n\n")),
        keywords: ["M değeri", "ölçü", "rota", "extract m values", "kilometre"]
            .map(String::from)
            .to_vec(),
        aliases: ["MCIKAR", "EXTRACTM"].map(String::from).to_vec(),
        icon: Some("routeMeasures".into()),
        parameters: vec![
            features("input", "Rotalar", &ROUTE_KINDS, true),
            ParamDef::new(
                "prefix",
                "Önek",
                ParamKind::Text {
                    placeholder: None,
                    max_length: Some(24),
                    allow_empty: true,
                },
            )
            .default_value(json!("M_"))
            .describe("Yazılan alanların adlarının başı: “M_bas”, “M_son”."),
        ],
        outputs: vec![OutputDef::new("count", "Yazılan rota sayısı", OutputKind::Number)],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: Some(|v| {
            let p = text_of(v, "prefix");
            Some(format!("“{p}bas”, “{p}son”, “{p}min”, “{p}max”"))
        }),
        run: Some(run_extract),
    }
}

fn run_extract(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let prefix = v.text("prefix");
    let mut update = Vec::new();
    let (mut routes, mut without) = (0usize, 0usize);
    for e in &v.features("input").entities {
        let Some(route) = route_of(e) else {
            continue;
        };
        routes += 1;
        let s = summary(&route);
        let (Some(first), Some(last), Some(least), Some(most)) = (s.first, s.last, s.least, s.most)
        else {
            without += 1;
            continue;
        };
        let mut current: Entity = (*e).clone();
        let mut changed = false;
        for (key, value) in [("bas", first), ("son", last), ("min", least), ("max", most)] {
            let text = length_text(ctx.units, value);
            if let Some(attrs) = with_attr(ctx, &current, &format!("{prefix}{key}"), Some(&text)) {
                current.base_mut().attrs = attrs;
                changed = true;
            }
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
    if without > 0 {
        feedback.info(format!("{without} nesnenin ölçüsü yok; yazılmadı."));
    }
    let written = routes - without;
    RunResult {
        changes: Some(ChangeSet {
            update,
            ..ChangeSet::default()
        }),
        outputs: [("count".to_owned(), json!(written))].into_iter().collect(),
        summary: Some(format!(
            "{routes} rotadan {written} rotanın ölçüleri yazıldı."
        )),
        ..RunResult::default()
    }
}
