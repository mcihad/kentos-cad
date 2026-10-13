//! Nesneden nokta, Köşeleri noktaya çevir and Noktalardan çizgi (the web's
//! `toPoints.ts`, `vertices.ts`, `pointsToPath.ts`; docs/adr/0218 §2–§4).

use std::collections::BTreeMap;

use kentos_contracts::Entity;
use kentos_geometry_core::display::fixed;
use kentos_geometry_core::entity::{Shape, area_parts};
use kentos_geometry_core::ops::convert::points::{Which, centroid, interior_point, vertex_points};
use kentos_geometry_core::ops::order::{Key, order};
use kentos_geometry_core::vec2::Vec2;
use kentos_native_application::geometry::shape;
use kentos_style_core::expr::Value as ExprValue;
use kentos_style_core::expr::rows::As;
use serde_json::json;

use super::super::geometry::{has_elevation, layer_param, new_object};
use super::{point, scopes, zs_of};
use crate::expression::evaluate_in;
use crate::text::js_trim;
use crate::types::{
    ChangeSet, EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, Returns,
    RunContext, RunResult, Target, Tool,
};

fn features(kinds: &[&str], description: &str, label: &str) -> ParamDef {
    ParamDef::new(
        "input",
        label,
        ParamKind::Features {
            kinds: Some(kinds.iter().map(|k| (*k).to_owned()).collect()),
            scopes: scopes(),
            writes: false,
        },
    )
    .describe(description)
}

fn counted(add: Vec<Entity>, summary: String) -> RunResult {
    let count = add.len();
    RunResult {
        changes: Some(ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs: [("count".to_owned(), json!(count))].into_iter().collect(),
        summary: Some(summary),
        ..RunResult::default()
    }
}

// ── Nesneden nokta ───────────────────────────────────────────────────

pub fn to_points() -> Tool {
    Tool {
        id: "convert.toPoints".into(),
        label: "Nesneden nokta".into(),
        category: "conversion".into(),
        description: "Her nesnenin ağırlık merkezini ya da üstündeki bir iç noktayı, öznitelikleriyle yeni katmana nokta olarak yazar.".into(),
        help: Some([
            "Örnekler: parsellerin merkezlerine numara noktaları koymak, yapıları nokta katmanına çevirmek, yolların orta noktalarını almak.",
            "Ağırlık merkezi alanda alanın merkezidir (delikler çıkar, yaylı kenarlar kesin), çizgide uzunlukla ağırlıklı merkezdir; L biçimli bir alanda ya da kıvrık bir çizgide nesnenin dışında kalabilir. İç nokta her zaman nesnenin üstündedir: alanda yatay bir doğrunun alanın içinde kalan en geniş aralığının ortası (GEOS’un kuralı), çizgide yolun yarısı.",
            "Her parça için açıkken çok parçalı nesnenin her parçası bir nokta olur, Parça alanı 1, 2 … yazılır. Noktaların kotu yoktur.",
        ]
        .join("\n\n")),
        keywords: [
            "nokta", "merkez", "ağırlık merkezi", "iç nokta", "centroid", "point on surface",
            "feature to point",
        ]
        .map(String::from)
        .to_vec(),
        aliases: vec!["NESNEDENNOKTA".into(), "MERKEZNOKTA".into()],
        icon: Some("featureToPoint".into()),
        parameters: vec![
            features(
                &["point", "line", "polyline", "polygon", "circle", "arc"],
                "Noktası alınacak nesneler.",
                "Nesneler",
            ),
            ParamDef::new(
                "location",
                "Konum",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("centroid", "Ağırlık merkezi").hint("Nesnenin dışında kalabilir"),
                        EnumOption::new("inside", "İç nokta").hint("Her zaman nesnenin üstünde"),
                    ],
                },
            )
            .default_value(json!("centroid")),
            ParamDef::new("parts", "Her parça için", ParamKind::Boolean)
                .default_value(json!(false))
                .describe("Çok parçalı nesnenin her parçası bir nokta olur."),
            layer_param("Merkezler", "#E54666"),
        ],
        outputs: vec![
            OutputDef::new("points", "Yazılan noktalar", OutputKind::Features),
            OutputDef::new("count", "Nokta sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_to_points),
    }
}

fn run_to_points(
    v: &Resolved<'_>,
    _ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
) -> RunResult {
    let inputs = &v.features("input").entities;
    let heights = inputs.iter().filter(|e| has_elevation(e)).count();
    if heights > 0 {
        feedback.warn(format!("{heights} nesnenin kotları sonuca taşınmadı."));
    }
    feedback.progress(0.0, "Noktalar hesaplanıyor");
    let inside = v.text("location") == "inside";
    let parts = v.flag("parts");
    let layer = &v.layer("layer").id;
    let at = |s: &Shape| {
        if inside {
            interior_point(s)
        } else {
            centroid(s)
        }
    };
    let mut add = Vec::new();
    for e in inputs {
        let s = shape(e);
        let found: Vec<Vec2> = if parts {
            area_parts(&s).iter().filter_map(at).collect()
        } else {
            at(&s).into_iter().collect()
        };
        for (k, p) in found.into_iter().enumerate() {
            let mut attrs = e.base().attrs.clone();
            if parts {
                attrs.insert("Parça".into(), (k + 1).to_string());
            }
            add.extend(point(p, None, layer, attrs));
        }
    }
    let summary = format!("{} nesneden {} nokta yazıldı.", inputs.len(), add.len());
    counted(add, summary)
}

// ── Köşeleri noktaya çevir ───────────────────────────────────────────

pub fn vertices() -> Tool {
    Tool {
        id: "convert.vertices".into(),
        label: "Köşeleri noktaya çevir".into(),
        category: "conversion".into(),
        description: "Çizgilerin ve alanların köşelerini (ya da yalnız uçlarını, ortalarını) öznitelikleriyle yeni katmana nokta olarak yazar.".into(),
        help: Some([
            "Örnekler: parsel köşelerini noktaya çevirip numaralamak, bir güzergâhın başlangıç ve bitiş noktalarını almak, hatların ortalarına etiket noktası koymak.",
            "Her nokta nesnenin özniteliklerine ek Köşe (nesnedeki sırası), Parça, Halka (0 dış halka ya da çizgi, 1 … delikler) ve Uzaklık (halkanın ya da yolun başından yol boyunca, m) alır; köşenin kotu noktanın kotudur.",
            "Kapalı halkanın ilk köşesi bir kez yazılır; başlangıcı ve bitişi ilk köşesidir. Orta, yolun (alanda dış halkanın) yarısındaki noktadır; köşe olmadığından Köşe yazılmaz.",
        ]
        .join("\n\n")),
        keywords: [
            "köşe", "nokta", "uç", "başlangıç", "bitiş", "orta", "vertices", "extract vertices",
            "feature vertices to points",
        ]
        .map(String::from)
        .to_vec(),
        aliases: vec!["KOSELERINOKTA".into(), "KOSENOKTA".into()],
        icon: Some("verticesToPoints".into()),
        parameters: vec![
            features(
                &["line", "polyline", "polygon", "arc"],
                "Köşeleri alınacak çizgiler ve alanlar.",
                "Nesneler",
            ),
            ParamDef::new(
                "which",
                "Köşeler",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("all", "Bütün köşeler"),
                        EnumOption::new("start", "Başlangıç"),
                        EnumOption::new("end", "Bitiş"),
                        EnumOption::new("ends", "İki uç"),
                        EnumOption::new("middle", "Orta").hint("Yolun yarısındaki nokta"),
                    ],
                },
            )
            .default_value(json!("all")),
            layer_param("Köşeler", "#0090FF"),
        ],
        outputs: vec![
            OutputDef::new("points", "Yazılan noktalar", OutputKind::Features),
            OutputDef::new("count", "Nokta sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_vertices),
    }
}

fn run_vertices(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    feedback.progress(0.0, "Köşeler okunuyor");
    let which = match v.text("which") {
        "start" => Which::Start,
        "end" => Which::End,
        "ends" => Which::Ends,
        "middle" => Which::Middle,
        _ => Which::All,
    };
    let ld = ctx.units.length_decimals as usize;
    let layer = &v.layer("layer").id;
    let mut add = Vec::new();
    for e in inputs {
        let zs = zs_of(e);
        for q in vertex_points(&shape(e), zs.as_ref(), which) {
            let mut attrs = e.base().attrs.clone();
            if let Some(index) = q.index {
                attrs.insert("Köşe".into(), index.to_string());
            }
            attrs.insert("Parça".into(), q.part.to_string());
            attrs.insert("Halka".into(), q.ring.to_string());
            attrs.insert("Uzaklık".into(), fixed(q.distance, ld));
            add.extend(point(q.p, q.z, layer, attrs));
        }
    }
    let summary = format!("{} nesneden {} nokta yazıldı.", inputs.len(), add.len());
    counted(add, summary)
}

// ── Noktalardan çizgi ────────────────────────────────────────────────

pub fn points_to_path() -> Tool {
    Tool {
        id: "convert.pointsToPath".into(),
        label: "Noktalardan çizgi".into(),
        category: "conversion".into(),
        description: "Noktaları bir ifadenin sırasıyla birleştirip çizgi (isterse kapalı alan) yapar; bir alanın her değeri ayrı bir çizgi olur.".into(),
        help: Some([
            "Örnekler: GNSS ile alınmış saha noktalarından zaman sırasıyla güzergâh çizmek, numaralı sınır noktalarından parsel sınırı kurmak, her hat kodunun noktalarını ayrı çizgi yapmak.",
            "Sıra boşsa noktalar girdinin sırasıyla birleşir; değeri olmayan noktalar sona kalır, eşitler girdinin sırasıyla. Grup boşsa bütün noktalar tek çizgidir. Çok noktalı nesnenin noktaları kendi sırasıyla aynı yerdedir.",
            "Bir öncekiyle aynı yerdeki nokta atlanır; iki noktadan (Kapat’ta üçten) az noktalı grup çizgi vermez; ikisi de söylenir. Çizgi grup alanını, Nokta sayısı’nı ve Sıra verilmişse ilk ve son noktanın sıra değerini (Başlangıç, Bitiş) alır; köşelerin kotları noktalarınkidir.",
        ]
        .join("\n\n")),
        keywords: [
            "nokta", "çizgi", "yol", "güzergâh", "birleştir", "sırala", "points to line",
            "points to path",
        ]
        .map(String::from)
        .to_vec(),
        aliases: vec!["NOKTADANCIZGI".into()],
        icon: Some("pointsToPath".into()),
        parameters: vec![
            features(&["point"], "Birleştirilecek noktalar.", "Noktalar"),
            ParamDef::new(
                "order",
                "Sıra",
                ParamKind::Expression {
                    returns: Returns::Value,
                    of: Some("input".into()),
                    placeholder: Some("No".into()),
                },
            )
            .optional()
            .describe("Noktaların sırası; boşsa girdinin sırası."),
            ParamDef::new(
                "group",
                "Grup",
                ParamKind::Field {
                    of: vec!["input".into()],
                    allow_new: false,
                    multiple: false,
                },
            )
            .optional()
            .describe("Her değeri ayrı bir çizgi; boşsa tek çizgi."),
            ParamDef::new("close", "Kapat", ParamKind::Boolean)
                .default_value(json!(false))
                .describe("Son nokta ilkine bağlanır; kapalı alan yazılır."),
            layer_param("Yollar", "#30A46C"),
        ],
        outputs: vec![
            OutputDef::new("paths", "Yazılan çizgiler", OutputKind::Features),
            OutputDef::new("count", "Çizgi sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_points_to_path),
    }
}

/// A group's line: its points, their elevations, the objects of its first and last.
#[derive(Default)]
struct Group {
    pts: Vec<Vec2>,
    zs: Vec<Option<f64>>,
    first: Option<usize>,
    last: usize,
}

fn run_points_to_path(
    v: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
) -> RunResult {
    let inputs = &v.features("input").entities;
    feedback.progress(0.0, "Sıralanıyor");
    let layer_name = |id: &str| ctx.layer_name(id).to_owned();
    let how = ctx.evaluation(&layer_name);
    let expr = v.expr("order");
    let (keys, texts) = match expr {
        Some(e) => (
            evaluate_in(e, inputs, &how, As::Value),
            evaluate_in(e, inputs, &how, As::Text),
        ),
        None => (Vec::new(), Vec::new()),
    };
    let at: Vec<usize> = if expr.is_some() {
        let keys: Vec<Option<Key>> = (0..inputs.len())
            .map(|i| match keys.get(i) {
                Some(ExprValue::Num(x)) => Some(Key::Num(*x)),
                Some(ExprValue::Text(t)) => Some(Key::Text(t.to_string())),
                Some(ExprValue::Bool(b)) => Some(Key::Bool(*b)),
                _ => None,
            })
            .collect();
        order(&keys, false, false)
            .into_iter()
            .map(|i| i as usize)
            .collect()
    } else {
        (0..inputs.len()).collect()
    };
    let field = js_trim(v.text("group")).to_owned();
    // The groups in the order their values first come in the input.
    let group_of: Vec<String> = inputs
        .iter()
        .map(|e| {
            if field.is_empty() {
                return String::new();
            }
            match js_trim(e.base().attrs.get(&field).map_or("", String::as_str)) {
                "" => "(boş)".to_owned(),
                g => g.to_owned(),
            }
        })
        .collect();
    let mut names: Vec<&str> = Vec::new();
    let mut groups: BTreeMap<&str, Group> = BTreeMap::new();
    for g in &group_of {
        if !groups.contains_key(g.as_str()) {
            names.push(g);
            groups.insert(g, Group::default());
        }
    }
    let mut repeats = 0;
    for &i in &at {
        // Every object's group is in (made above from the same values).
        let (Entity::Point(e), Some(g)) = (inputs[i], groups.get_mut(group_of[i].as_str())) else {
            continue;
        };
        let own = std::iter::once((&e.p, e.z));
        let more = e.parts.iter().flatten().map(|q| (&q.p, q.z));
        for (p, z) in own.chain(more) {
            let p = Vec2::new(p.x, p.y);
            if g.pts.last() == Some(&p) {
                repeats += 1;
                continue;
            }
            g.first.get_or_insert(i);
            g.last = i;
            g.pts.push(p);
            g.zs.push(z);
        }
    }
    let close = v.flag("close");
    let need = if close { 3 } else { 2 };
    let layer = &v.layer("layer").id;
    let text = |i: usize| match texts.get(i) {
        Some(ExprValue::Text(t)) => t.to_string(),
        _ => String::new(),
    };
    let mut add = Vec::new();
    let mut short = 0;
    for name in names {
        let g = &groups[name];
        if g.pts.len() < need {
            short += 1;
            continue;
        }
        let mut attrs = BTreeMap::new();
        if !field.is_empty() {
            attrs.insert(field.clone(), name.to_owned());
        }
        attrs.insert("Nokta sayısı".into(), g.pts.len().to_string());
        if expr.is_some() {
            attrs.insert("Başlangıç".into(), g.first.map(text).unwrap_or_default());
            attrs.insert("Bitiş".into(), text(g.last));
        }
        let pts = g.pts.clone();
        let s = if close {
            Shape::Polygon {
                pts,
                bulges: None,
                holes: None,
                parts: None,
            }
        } else {
            Shape::Polyline {
                pts,
                bulges: None,
                holes: None,
                parts: None,
                closed: false,
                widths: None,
            }
        };
        if let Some(mut e) = new_object(s, layer, attrs) {
            if g.zs.iter().any(Option::is_some) {
                kentos_native_application::elevation::assign(&mut e, std::slice::from_ref(&g.zs));
            }
            add.push(e);
        }
    }
    if repeats > 0 {
        feedback.warn(format!(
            "{repeats} nokta bir öncekiyle aynı yerde olduğu için atlandı."
        ));
    }
    if short > 0 {
        feedback.warn(format!(
            "{short} grupta çizgi için yeterli nokta yok (en az {need})."
        ));
    }
    let points: usize = inputs
        .iter()
        .map(|e| match e {
            Entity::Point(p) => 1 + p.parts.as_ref().map_or(0, Vec::len),
            _ => 0,
        })
        .sum();
    let summary = format!("{points} noktadan {} çizgi yazıldı.", add.len());
    counted(add, summary)
}
