//! `cad.zoning.check` v1 (plan/map-cad.md MAP-09, docs/adr/1420, 2020): İmar planı denetimi, read only. The layers and
//! the value lists come from the KentOS profile `imar-plani` (data, not code); the overlaps of decisions, a plan
//! boundary not covered by decisions, a decision outside the plan and a plan outside its upper plan come from the
//! topology core's rules (Çakışmamalı, … içinde kalmalı) over those layers; a road axis whose end meets neither another
//! road nor a plan boundary from its Sarkan uç olmamalı (docs/adr/2020; the road layer is optional: without it no road
//! is checked); a decision inside a road's planned width from … ile çakışmamalı over the road's corridor
//! (docs/adr/2240), and a parcel of the parcel layer the same (docs/adr/2640). Nothing is written; no legal or schema
//! conformity is claimed. The shared cases are
//! `fixtures/commands/v1/cad.zoning.check.json`.

use std::collections::{BTreeMap, HashSet};

use kentos_commands::{
    CommandResult, DataProfile, Entity, ZONING_PROFILE, ZoningCheck, ZoningChecked, ZoningFinding, builtin_profile,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::intersect::{Edge, point_at};
use kentos_geometry_core::geom::region::inside_area;
use kentos_geometry_core::ops::geoprocess::buffer::{Side, buffer};
use kentos_geometry_core::ops::geoprocess::{Class, class_of, shape_of, union_all};
use kentos_geometry_core::ops::topology_rules::{Kind, Objects, Rule, check};
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::geometry::shape;
use crate::profiles::{Place, find, path_words};

/// The kinds in the order the answer lists them.
pub const KINDS: [&str; 21] = [
    "missingLayer",
    "noLevel",
    "upperMissing",
    "levelOrder",
    "outsideUpper",
    "noPlan",
    "noUse",
    "unknownUse",
    "missingValue",
    "declaredValued",
    "kaksReview",
    "overlap",
    "uncovered",
    "outside",
    "roadGap",
    "roadWidth",
    "roadParcel",
    "noteMissing",
    "noteUnused",
    // Docs/adr/4640: last, so that the earlier kinds keep their order.
    "symbolMismatch",
    // Docs/adr/5040.
    "draftStale",
];

/// The uses on which nothing is built: they ask for no TAKS, KAKS or height.
pub const OPEN_USES: [&str; 4] = ["park", "yesil", "meydan", "yol"];

const LEAST_TOLERANCE: f64 = 1e-6;
const FIRST_TOLERANCE: f64 = 0.01;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// A value list's label of a code, or the code itself.
fn label<'a>(p: &'a DataProfile, domain: &str, code: &'a str) -> &'a str {
    p.domains
        .iter()
        .find(|d| d.id == domain)
        .and_then(|d| d.values.as_ref())
        .and_then(|v| v.iter().find(|c| c.code == code))
        .map_or(code, |c| c.label.as_str())
}

fn listed(p: &DataProfile, domain: &str, code: &str) -> bool {
    p.domains
        .iter()
        .find(|d| d.id == domain)
        .and_then(|d| d.values.as_ref())
        .is_some_and(|v| v.iter().any(|c| c.code == code))
}

/// An object of a class: its place in the drawing, its persistent id, its attributes trimmed.
struct Item<'a> {
    at: usize,
    uid: String,
    e: &'a Entity,
}

impl Item<'_> {
    fn get(&self, k: &str) -> String {
        self.e.base().attrs.get(k).map(|v| v.trim().to_owned()).unwrap_or_default()
    }
}

fn number(text: &str) -> Option<f64> {
    text.replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite())
}

/// A road's place for finding its plan (docs/adr/2240): the middle of its longest edge (by the chord of its ends, the
/// first of equals), on its curve.
fn middle_of(paths: &[Vec<Edge>]) -> Option<Vec2> {
    let mut best: Option<(f64, &Edge)> = None;
    for e in paths.iter().flatten() {
        let (a, b) = (point_at(e, 0.0), point_at(e, 1.0));
        let chord = (b.x - a.x).hypot(b.y - a.y);
        if best.is_none_or(|(c, _)| chord > c) {
            best = Some((chord, e));
        }
    }
    best.map(|(_, e)| point_at(e, 0.5))
}


/// The layers of the zoning plan's classes as the drawing has them (plan boundaries, decision areas, plan notes; the
/// profile `imar-plani`'s paths): what the selection's role `zoning` reads (docs/adr/2480). A class without its layer
/// is left out.
/// A finding's kind in words, as İmar denetimi's table and İşlem araç kutusu's tool say it (docs/adr/3840).
pub fn kind_word(kind: &str) -> &'static str {
    match kind {
        "missingLayer" => "Katman yok",
        "noLevel" => "Kademe yok",
        "upperMissing" => "Üst plan yok",
        "levelOrder" => "Kademe sırası",
        "outsideUpper" => "Üst plan dışına taşma",
        "noPlan" => "Planı yok",
        "noUse" => "Kullanım yok",
        "unknownUse" => "Bilinmeyen kullanım",
        "missingValue" => "Değer verilmemiş",
        "declaredValued" => "Değer çelişkisi",
        "kaksReview" => "KAKS gözden geçir",
        "overlap" => "Karar çelişkisi",
        "uncovered" => "Kararsız kısım",
        "outside" => "Plan dışına taşma",
        "roadGap" => "Kopuk yol",
        "roadWidth" => "Yola giren karar",
        // docs/adr/2640.
        "roadParcel" => "Yola giren parsel",
        "noteMissing" => "Plan notu yok",
        "noteUnused" => "Anılmayan not",
        "symbolMismatch" => "Sembol uyuşmazlığı",
        "draftStale" => "Taslak eskidi",
        _ => "Bulgu",
    }
}

pub fn plan_layers(doc: &Document) -> Vec<String> {
    let Some(p) = builtin_profile(ZONING_PROFILE) else { return Vec::new() };
    ["plan-siniri", "karar-alani", "plan-notu"]
        .iter()
        .filter_map(|class| p.classes.iter().find(|c| c.id == *class))
        .filter_map(|c| match find(doc.layers().nodes(), &c.layer) {
            Place::Layer(id) => Some(id),
            _ => None,
        })
        .collect()
}

fn run(doc: &Document, input: &ZoningCheck) -> Result<ZoningChecked, Stop> {
    run_with_regions(doc, input).map(|(checked, _)| checked)
}

/// A finding's regions as the answer gives them (docs/adr/3440): each ring from its lowest vertex (the westmost of the
/// lowest), the outer counter-clockwise and the holes clockwise, a vertex on a straight run (both its edges straight,
/// no turn at it) left out; the holes, and the regions, in order of their first vertices.
fn answered(areas: &[kentos_geometry_core::geom::arrangement::Area]) -> Vec<kentos_commands::ZoningRegion> {
    let first = |r: &kentos_commands::RingGeometry| r.pts.first().map_or((f64::MAX, f64::MAX), |p| (p.y, p.x));
    let by_first = |a: &kentos_commands::RingGeometry, b: &kentos_commands::RingGeometry| {
        let (fa, fb) = (first(a), first(b));
        fa.0.total_cmp(&fb.0).then(fa.1.total_cmp(&fb.1))
    };
    let mut out: Vec<kentos_commands::ZoningRegion> = areas
        .iter()
        .map(|a| {
            let mut holes: Vec<_> = a.holes.iter().map(|h| answered_ring(h, false)).collect();
            holes.sort_by(by_first);
            kentos_commands::ZoningRegion { outer: answered_ring(&a.outer, true), holes }
        })
        .collect();
    out.sort_by(|a, b| by_first(&a.outer, &b.outer));
    out
}

fn answered_ring(r: &kentos_geometry_core::geom::arrangement::Ring, ccw: bool) -> kentos_commands::RingGeometry {
    let mut pts: Vec<Vec2> = r.pts.clone();
    let mut bulges: Vec<f64> = (0..pts.len()).map(|i| r.bulges.as_ref().and_then(|b| b.get(i).copied()).unwrap_or(0.0)).collect();
    // A vertex where the ring stays or runs straight on goes; its edges join.
    let mut k = 0;
    while pts.len() > 3 && k < pts.len() {
        let n = pts.len();
        let (prev, here, next) = (pts[(k + n - 1) % n], pts[k], pts[(k + 1) % n]);
        let (ax, ay, bx, by) = (here.x - prev.x, here.y - prev.y, next.x - here.x, next.y - here.y);
        let same = here == prev;
        let straight = bulges[(k + n - 1) % n] == 0.0 && bulges[k] == 0.0 && ax * by - ay * bx == 0.0 && ax * bx + ay * by > 0.0;
        if same || straight {
            let joined = if same { bulges[k] } else { 0.0 };
            pts.remove(k);
            bulges.remove(k);
            let n = pts.len();
            bulges[(k + n - 1) % n] = if same { joined } else { bulges[(k + n - 1) % n] };
            k = k.saturating_sub(1);
        } else {
            k += 1;
        }
    }
    let n = pts.len();
    if (kentos_geometry_core::geom::bulge::bulge_ring_area(&pts, Some(&bulges)) > 0.0) != ccw && n > 0 {
        // The other way round: the vertices reversed, each edge's bulge the reversed edge's, negated.
        let old_b = bulges.clone();
        pts.reverse();
        bulges = (0..n).map(|k| -old_b[(2 * n - 2 - k) % n]).collect();
    }
    let start = (0..n)
        .min_by(|&i, &j| pts[i].y.total_cmp(&pts[j].y).then(pts[i].x.total_cmp(&pts[j].x)))
        .unwrap_or(0);
    pts.rotate_left(start);
    bulges.rotate_left(start);
    kentos_commands::RingGeometry {
        pts: pts.iter().map(|p| kentos_commands::Vec2 { x: p.x, y: p.y }).collect(),
        bulges: bulges.iter().any(|b| *b != 0.0).then_some(bulges),
        zs: None,
    }
}

/// The answer and each finding's regions (docs/adr/3040): a road width's or a road parcel's overlap with the road's
/// corridor, as the core's rule finds it; none for the other kinds. Bulguları göster draws these; the answer carries
/// the same regions in its own order (docs/adr/3440, [`answered`]).
pub fn with_regions(
    doc: &Document,
    input: &ZoningCheck,
) -> Result<(ZoningChecked, Vec<Vec<kentos_geometry_core::geom::arrangement::Area>>), String> {
    run_with_regions(doc, input).map_err(|stop| stop.into_error().message)
}

#[allow(clippy::type_complexity)]
fn run_with_regions(
    doc: &Document,
    input: &ZoningCheck,
) -> Result<(ZoningChecked, Vec<Vec<kentos_geometry_core::geom::arrangement::Area>>), Stop> {
    let tolerance = input.tolerance.unwrap_or(FIRST_TOLERANCE);
    if !tolerance.is_finite() || tolerance < LEAST_TOLERANCE {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("Tolerans sonlu ve en az {LEAST_TOLERANCE} m olmalı (yazılan {tolerance})."),
            "tolerance",
        ));
    }
    let Some(p) = builtin_profile(ZONING_PROFILE) else {
        return Err(fail(codes::INVALID_INPUT, "İmar planı profili okunamadı.".into(), "profile"));
    };
    // The regions the next finding is shown by (a road's corridor's overlap, docs/adr/3040), taken by `say`.
    let pending: std::cell::RefCell<Vec<kentos_geometry_core::geom::arrangement::Area>> = std::cell::RefCell::new(Vec::new());
    let mut findings: Vec<(usize, i64, ZoningFinding, Vec<kentos_geometry_core::geom::arrangement::Area>)> = Vec::new();
    let mut say = |kind: &str, items: &[&Item<'_>], area: Option<f64>, message: String| {
        let order = KINDS.iter().position(|k| *k == kind).unwrap_or(KINDS.len());
        let mut items = items.to_vec();
        items.sort_by_key(|i| i.at);
        let first = items.first().map_or(-1, |i| i.at as i64);
        let regions = pending.take();
        let finding = ZoningFinding {
            kind: kind.to_owned(),
            uids: items.iter().map(|i| i.uid.clone()).collect(),
            area,
            message,
            regions: answered(&regions),
        };
        findings.push((order, first, finding, regions));
    };
    // Each class's layer and objects, in the drawing's order.
    let order: Vec<&Entity> = doc.entities().collect();
    let mut layers: BTreeMap<&str, String> = BTreeMap::new();
    for class in ["plan-siniri", "karar-alani", "plan-notu"] {
        let Some(c) = p.classes.iter().find(|c| c.id == class) else { continue };
        match find(doc.layers().nodes(), &c.layer) {
            Place::Layer(id) => {
                layers.insert(class, id);
            }
            _ => say(
                "missingLayer",
                &[],
                None,
                format!("“{}” katmanı yok; {ZONING_PROFILE} profilini uygulayın (cad.profiles.apply).", path_words(&c.layer)),
            ),
        }
    }
    let of = |class: &str, kind: &str| -> Vec<Item<'_>> {
        let Some(layer) = layers.get(class) else { return Vec::new() };
        order
            .iter()
            .enumerate()
            .filter(|(_, e)| e.base().layer_id == *layer && e.kind() == kind)
            .map(|(at, e)| Item {
                at,
                uid: doc.uid(Slot(e.base().id)).map(|u| u.to_string()).unwrap_or_default(),
                e,
            })
            .collect()
    };
    let plans = of("plan-siniri", "polygon");
    let decisions = of("karar-alani", "polygon");
    let notes = of("plan-notu", "point");
    // The road axes (docs/adr/2020): their layer if the drawing has it; none is no finding.
    let road_layer = p.classes.iter().find(|c| c.id == "yol-aksi").and_then(|c| match find(doc.layers().nodes(), &c.layer) {
        Place::Layer(id) => Some(id),
        _ => None,
    });
    // The parcels (docs/adr/2640): the areas of the parcel layer, as Parsel alan denetimi reads them.
    let parcels: Vec<Item<'_>> = order
        .iter()
        .enumerate()
        .filter(|(_, e)| e.base().layer_id == kentos_commands::PARCEL_LAYER && e.kind() == "polygon")
        .map(|(at, e)| Item { at, uid: doc.uid(Slot(e.base().id)).map(|u| u.to_string()).unwrap_or_default(), e })
        .collect();
    let roads: Vec<Item<'_>> = road_layer
        .map(|layer| {
            order
                .iter()
                .enumerate()
                .filter(|(_, e)| e.base().layer_id == layer && matches!(e.kind(), "line" | "polyline"))
                .map(|(at, e)| Item { at, uid: doc.uid(Slot(e.base().id)).map(|u| u.to_string()).unwrap_or_default(), e })
                .collect()
        })
        .unwrap_or_default();

    // The plans: their level, their upper plan.
    let rank = |code: &str| ["msp", "cdp", "nip", "uip"].iter().position(|c| *c == code);
    let plan_ids: Vec<String> = plans.iter().map(|i| i.get("PlanKimlik")).collect();
    for item in &plans {
        let level = item.get("Seviye");
        if level.is_empty() || !listed(&p, "plan-seviye", &level) {
            say("noLevel", &[item], None, "Plan sınırının kademesi yok ya da listede değil (plan-seviye).".into());
        }
        let upper = item.get("UstPlan");
        if upper.is_empty() {
            continue;
        }
        match plans.iter().find(|q| q.get("PlanKimlik") == upper) {
            None => say("upperMissing", &[item], None, format!("Üst plan “{upper}” plan sınırlarında yok.")),
            Some(u) => {
                let ul = u.get("Seviye");
                if let (Some(a), Some(b)) = (rank(&ul), rank(&level))
                    && a >= b
                {
                    say(
                        "levelOrder",
                        &[item, u],
                        None,
                        format!(
                            "Üst plan “{upper}” ({}) bu planın ({}) üstünde bir kademe değil.",
                            label(&p, "plan-seviye", &ul),
                            label(&p, "plan-seviye", &level)
                        ),
                    );
                }
            }
        }
    }

    // The decisions: their plan, their use, their values.
    let note_numbers: HashSet<String> = notes.iter().map(|n| n.get("NotNo")).filter(|n| !n.is_empty()).collect();
    let mut named: HashSet<String> = HashSet::new();
    for item in &decisions {
        let plan = item.get("Plan");
        if plan.is_empty() && plans.len() != 1 {
            say("noPlan", &[item], None, format!("Karar alanının planı yazılmamış; çizimde {} plan sınırı var.", plans.len()));
        } else if !plan.is_empty() && !plan_ids.contains(&plan) {
            say("noPlan", &[item], None, format!("Karar alanının planı “{plan}” plan sınırlarında yok."));
        }
        let usage = item.get("Kullanim");
        let use_label = label(&p, "plan-kullanim", &usage).to_owned();
        if usage.is_empty() {
            say("noUse", &[item], None, "Karar alanının kullanımı yok; plan kararı yazılmadan alan bir karar değildir.".into());
        } else if !listed(&p, "plan-kullanim", &usage) {
            say("unknownUse", &[item], None, format!("“{usage}” kullanım listesinde yok (plan-kullanim)."));
        }
        let declared: Vec<String> = item
            .get("Belirtilmeyen")
            .split(';')
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect();
        let has = |k: &str| !item.get(k).is_empty();
        let said = |k: &str| declared.iter().any(|d| d == k);
        if !usage.is_empty() && !OPEN_USES.contains(&usage.as_str()) {
            let mut missing = Vec::new();
            for k in ["TAKS", "KAKS"] {
                if !has(k) && !said(k) {
                    missing.push(label(&p, "plan-deger", k).to_owned());
                }
            }
            if !has("KatAdedi") && !has("Hmax") && !said("KatAdedi") && !said("Hmax") {
                missing.push("Kat adedi / Hmax".to_owned());
            }
            if !missing.is_empty() {
                say(
                    "missingValue",
                    &[item],
                    None,
                    format!("{use_label}: {} verilmemiş; değeri yazın ya da Planda verilmeyen'e ekleyin.", missing.join(", ")),
                );
            }
        }
        let both: Vec<String> = declared
            .iter()
            .filter(|d| match d.as_str() {
                "Cekme" => has("OnCekme") || has("YanCekme") || has("ArkaCekme"),
                k => has(k),
            })
            .map(|d| label(&p, "plan-deger", d).to_owned())
            .collect();
        if !both.is_empty() {
            say("declaredValued", &[item], None, format!("{} hem verilmiş hem Planda verilmeyen'de.", both.join(", ")));
        }
        let (taks, kaks, floors) = (item.get("TAKS"), item.get("KAKS"), item.get("KatAdedi"));
        if let (Some(t), Some(k), Some(n)) = (number(&taks), number(&kaks), number(&floors)) {
            let product = t * n;
            if k > product + 1e-9 {
                say(
                    "kaksReview",
                    &[item],
                    None,
                    format!(
                        "KAKS {kaks} > TAKS × kat adedi ({taks} × {floors} = {product:.2}); kapalı çıkmalar emsale girebilir, gözden geçirin."
                    ),
                );
            }
        }
        let wanted: Vec<String> =
            item.get("PlanNotlari").split([';', ',', ' ']).map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect();
        let absent: Vec<&str> = wanted.iter().filter(|n| !note_numbers.contains(*n)).map(String::as_str).collect();
        if !absent.is_empty() {
            say("noteMissing", &[item], None, format!("Plan notu {} yok.", absent.join(", ")));
        }
        named.extend(wanted);
    }
    for n in &notes {
        let no = n.get("NotNo");
        if n.get("Kapsam") == "alan" && !no.is_empty() && !named.contains(&no) {
            say("noteUnused", &[n], None, format!("Plan notu {no} karar alanları içindir ama hiçbir karar alanı onu anmıyor."));
        }
    }

    // Two decisions on one place, a plan not covered, a decision outside: the topology core, plan by plan, over the
    // plan's boundary and the decisions that name it (a nazım plan's own decisions are not an uygulama plan's gap). A
    // plan without decisions is drawn as an upper plan only: it has none to cover it.
    let resolved = |item: &Item<'_>| {
        let plan = item.get("Plan");
        if plan.is_empty() && plans.len() == 1 { plan_ids.first().cloned() } else { plan_ids.contains(&plan).then_some(plan) }
    };
    let mut all: Vec<&Item<'_>> = Vec::new();
    let mut on: Vec<String> = Vec::new();
    let mut rules: Vec<Rule> = Vec::new();
    let mut kinds: Vec<&str> = Vec::new();
    for (plan, id) in plans.iter().zip(&plan_ids) {
        let own: Vec<&Item<'_>> = decisions.iter().filter(|d| resolved(d).as_ref() == Some(id)).collect();
        if own.is_empty() || id.is_empty() {
            continue;
        }
        let (bound, decided) = (format!("plan:{}", plan.at), format!("karar:{}", plan.at));
        all.push(plan);
        on.push(bound.clone());
        for d in own {
            all.push(d);
            on.push(decided.clone());
        }
        let rule = |kind, layer: &str, other: Option<&str>| Rule {
            id: format!("{}", rules.len()),
            kind,
            layer: layer.to_owned(),
            other: other.map(str::to_owned),
            value: None,
        };
        let these = [
            ("overlap", rule(Kind::MustNotOverlap, &decided, None)),
            ("uncovered", rule(Kind::MustBeCoveredBy, &bound, Some(&decided))),
            ("outside", rule(Kind::MustBeCoveredBy, &decided, Some(&bound))),
        ];
        for (k, r) in these {
            kinds.push(k);
            rules.push(r);
        }
    }
    if !rules.is_empty() {
        let shapes: Vec<Shape> = all.iter().map(|i| shape(i.e)).collect();
        let uids: Vec<String> = all.iter().map(|i| i.uid.clone()).collect();
        let checked = check(&Objects { shapes: &shapes, layers: &on, uids: &uids }, &rules, tolerance, &[]);
        for f in checked.findings {
            let mut items: Vec<&Item<'_>> = f.objects.iter().map(|&k| all[k]).collect();
            items.sort_by_key(|i| i.at);
            let area = f.measure;
            match kinds[f.rule] {
                "overlap" => {
                    let words: Vec<String> =
                        items.iter().map(|i| label(&p, "plan-kullanim", &i.get("Kullanim")).to_owned()).collect();
                    say("overlap", &items, area, format!("İki karar alanı aynı yerde: {} (karar çelişkisi).", words.join(" ve ")));
                }
                "uncovered" => say("uncovered", &items, area, "Plan sınırının bir kısmında karar yok.".into()),
                _ => say("outside", &items, area, "Karar alanı plan sınırının dışına taşıyor.".into()),
            }
        }
    }

    // A plan outside its upper plan (docs/adr/2020): the topology core's … içinde kalmalı, pair by pair (a plan may be
    // one pair's lower and another's upper, so each pair takes its own copies).
    let pairs: Vec<(&Item<'_>, &Item<'_>)> = plans
        .iter()
        .filter_map(|lower| {
            let up = lower.get("UstPlan");
            (!up.is_empty()).then(|| plans.iter().find(|q| q.get("PlanKimlik") == up).map(|upper| (lower, upper))).flatten()
        })
        .collect();
    if !pairs.is_empty() {
        let mut shapes: Vec<Shape> = Vec::new();
        let (mut on, mut uids, mut rules) = (Vec::new(), Vec::new(), Vec::new());
        for (k, (lower, upper)) in pairs.iter().enumerate() {
            for (item, name) in [(lower, format!("alt:{k}")), (upper, format!("ust:{k}"))] {
                shapes.push(shape(item.e));
                on.push(name);
                uids.push(item.uid.clone());
            }
            rules.push(Rule {
                id: format!("{k}"),
                kind: Kind::MustBeCoveredBy,
                layer: format!("alt:{k}"),
                other: Some(format!("ust:{k}")),
                value: None,
            });
        }
        let checked = check(&Objects { shapes: &shapes, layers: &on, uids: &uids }, &rules, tolerance, &[]);
        for f in checked.findings {
            let (lower, upper) = pairs[f.rule];
            say(
                "outsideUpper",
                &[lower, upper],
                f.measure,
                format!("Plan sınırı, üst plan “{}” sınırının dışına taşıyor.", upper.get("PlanKimlik")),
            );
        }
    }
    // A road axis's end that meets neither another road nor a plan boundary (docs/adr/2020): Sarkan uç olmamalı over
    // the roads, the plan boundaries its connectors (the rule takes its own layer's areas as connectors, so the plan
    // boundaries go on the roads' layer here).
    if !roads.is_empty() {
        let mut shapes: Vec<Shape> = roads.iter().map(|r| shape(r.e)).collect();
        let mut on: Vec<String> = vec!["yol".to_owned(); roads.len()];
        let mut uids: Vec<String> = roads.iter().map(|r| r.uid.clone()).collect();
        for plan in &plans {
            shapes.push(shape(plan.e));
            on.push("yol".to_owned());
            uids.push(plan.uid.clone());
        }
        let rule = Rule { id: "0".into(), kind: Kind::MustNotHaveDangles, layer: "yol".into(), other: None, value: None };
        let checked = check(&Objects { shapes: &shapes, layers: &on, uids: &uids }, &[rule], tolerance, &[]);
        for f in checked.findings {
            let Some(&k) = f.objects.first().filter(|&&k| k < roads.len()) else { continue };
            let road = &roads[k];
            say(
                "roadGap",
                &[road],
                None,
                format!(
                    "“{}” yol aksının ucu (Y {:.3}, X {:.3}) başka bir yola ya da plan sınırına bağlanmıyor (kopuk yol).",
                    road.get("YolKimlik"),
                    f.at.x,
                    f.at.y
                ),
            );
        }
    }

    // A decision inside a road's planned width (docs/adr/2240; Planlı Alanlar İmar Yönetmeliği m. 9(1)(b): the road's
    // width is the one the plan states). The road's corridor is Genişlik / 2 each side of its axis, the core's two
    // one-sided buffers joined (flat ends; the outer side of a turn round). It must not overlap a decision of the
    // road's plan whose use is not a road (… ile çakışmamalı, its pieces narrower than the tolerance not counted). The
    // road's plan is the lowest-level plan whose boundary holds the middle of the road's longest edge; a road without
    // a width, or in no plan with decisions, is not checked.
    if !roads.is_empty() {
        let plan_areas: Vec<Vec<kentos_geometry_core::geom::arrangement::Area>> = plans
            .iter()
            .map(|p| match class_of(&shape(p.e)) {
                Class::Areas(a) => a,
                _ => Vec::new(),
            })
            .collect();
        let home_of = |at: Vec2| {
            let mut best: Option<(i32, usize)> = None;
            for (k, areas) in plan_areas.iter().enumerate() {
                if !areas.iter().any(|a| inside_area(a, at)) {
                    continue;
                }
                let r = rank(&plans[k].get("Seviye")).map_or(-1, |r| r as i32);
                if best.is_none_or(|(b, _)| r > b) {
                    best = Some((r, k));
                }
            }
            best.map(|(_, k)| k)
        };
        let mut shapes: Vec<Shape> = Vec::new();
        let (mut on, mut uids) = (Vec::new(), Vec::new());
        // Each object's road (and its width as written), decision or parcel.
        enum Whose<'i, 'a> {
            Road(&'i Item<'a>, String),
            Decision(&'i Item<'a>),
            Parcel(&'i Item<'a>),
        }
        let mut whose: Vec<Whose<'_, '_>> = Vec::new();
        let mut homes: Vec<usize> = Vec::new();
        for road in &roads {
            let written = road.get("Genislik");
            let Some(width) = number(&written).filter(|w| *w > 0.0) else { continue };
            let Class::Paths(paths) = class_of(&shape(road.e)) else { continue };
            let Some(home) = middle_of(&paths).and_then(home_of) else { continue };
            let class = Class::Paths(paths);
            let mut pieces = buffer(&class, width / 2.0, Side::Left);
            pieces.extend(buffer(&class, width / 2.0, Side::Right));
            let Some(corridor) = shape_of(&Class::Areas(union_all(&pieces))) else { continue };
            shapes.push(corridor);
            on.push(format!("genislik:{home}"));
            uids.push(road.uid.clone());
            whose.push(Whose::Road(road, written));
            if !homes.contains(&home) {
                homes.push(home);
            }
        }
        let mut rules = Vec::new();
        for &home in &homes {
            let id = &plan_ids[home];
            for d in decisions.iter().filter(|d| resolved(d).as_ref() == Some(id) && d.get("Kullanim") != "yol") {
                shapes.push(shape(d.e));
                on.push(format!("karar:{home}"));
                uids.push(d.uid.clone());
                whose.push(Whose::Decision(d));
            }
            rules.push(Rule {
                id: format!("{}", rules.len()),
                kind: Kind::MustNotOverlapWith,
                layer: format!("genislik:{home}"),
                other: Some(format!("karar:{home}")),
                value: None,
            });
        }
        // The parcels of the parcel layer, whatever their plan, against every road's corridor (docs/adr/2640).
        if !homes.is_empty() {
            for parcel in &parcels {
                shapes.push(shape(parcel.e));
                on.push("parsel".to_owned());
                uids.push(parcel.uid.clone());
                whose.push(Whose::Parcel(parcel));
            }
            for &home in &homes {
                rules.push(Rule {
                    id: format!("{}", rules.len()),
                    kind: Kind::MustNotOverlapWith,
                    layer: format!("genislik:{home}"),
                    other: Some("parsel".to_owned()),
                    value: None,
                });
            }
        }
        if !rules.is_empty() {
            let checked = check(&Objects { shapes: &shapes, layers: &on, uids: &uids }, &rules, tolerance, &[]);
            for f in checked.findings {
                let road = f.objects.iter().find_map(|&k| match &whose[k] {
                    Whose::Road(r, w) => Some((*r, w)),
                    _ => None,
                });
                let other = f.objects.iter().find_map(|&k| match &whose[k] {
                    Whose::Road(..) => None,
                    w => Some(w),
                });
                let (Some((road, written)), Some(other)) = (road, other) else { continue };
                *pending.borrow_mut() = f.regions.clone();
                match other {
                    Whose::Decision(decision) => say(
                        "roadWidth",
                        &[road, *decision],
                        f.measure,
                        format!(
                            "“{}” yolunun {written} m genişliği içine {} kararı “{}” giriyor.",
                            road.get("YolKimlik"),
                            label(&p, "plan-kullanim", &decision.get("Kullanim")),
                            decision.get("KararKimlik")
                        ),
                    ),
                    Whose::Parcel(parcel) => {
                        let name: Vec<String> =
                            ["Ada", "Parsel"].iter().map(|k| parcel.get(k)).filter(|v| !v.is_empty()).collect();
                        let name = if name.is_empty() { "kimliksiz".to_owned() } else { name.join("/") };
                        say(
                            "roadParcel",
                            &[road, *parcel],
                            f.measure,
                            format!(
                                "“{}” yolunun {written} m genişliği içine “{name}” parseli giriyor.",
                                road.get("YolKimlik")
                            ),
                        );
                    }
                    Whose::Road(..) => {}
                }
            }
        }
    }

    // Docs/adr/4640: a decision area whose library symbol is not the one its decision gives (the project's kept choice,
    // docs/adr/5040, else KentOS's table, ADR 4240, read as Sembolleri karardan ver reads it): judged only when the area
    // has a symbol and its use has one; the symbol shows the decision, never the other way round.
    if let Some(rows) = crate::zoning_symbolize::rows(doc) {
        for row in rows.iter().filter(|r| r.state == "set") {
            let (Some(was), Some(symbol)) = (row.was.as_deref(), row.symbol.as_deref()) else { continue };
            let Some(item) = decisions.iter().find(|d| d.uid == row.uid) else { continue };
            say(
                "symbolMismatch",
                &[item],
                None,
                format!(
                    "Karar alanının sembolü “{}”, kararı “{}” ({}) “{}” sembolünü verir: sembol kararı göstermiyor. Sembolleri karardan ver ile düzeltin.",
                    crate::zoning_symbolize::symbol_name(was),
                    label(&p, "plan-kullanim", &row.usage),
                    label(&p, "plan-seviye", &row.level),
                    crate::zoning_symbolize::symbol_name(symbol)
                ),
            );
        }
    }
    // Docs/adr/5040: a plan change draft whose decision area changed, or is gone, after it was written.
    for (uid, message) in crate::zoning_draft::stale(doc) {
        let item = uid.as_deref().and_then(|u| decisions.iter().find(|d| d.uid == u));
        match item {
            Some(item) => say("draftStale", &[item], None, message),
            None => say("draftStale", &[], None, message),
        }
    }
    findings.sort_by_key(|(order, first, _, _)| (*order, *first));
    let (findings, regions): (Vec<ZoningFinding>, Vec<Vec<_>>) = findings.into_iter().map(|(_, _, f, r)| (f, r)).unzip();
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    for f in &findings {
        *counts.entry(f.kind.clone()).or_default() += 1;
    }
    let n = |v: usize| v.min(u32::MAX as usize) as u32;
    Ok((ZoningChecked {
        profile: ZONING_PROFILE.to_owned(),
        tolerance,
        plans: n(plans.len()),
        decisions: n(decisions.len()),
        notes: n(notes.len()),
        findings,
        counts,
    }, regions))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ZoningCheck) -> CommandResult<()> {
    match run(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &ZoningCheck) -> CommandResult<ZoningChecked> {
    match run(cx.doc, input) {
        Ok(output) => CommandResult::Completed { output, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ZoningCheck) -> CommandResult<ZoningChecked> {
    plan(cx, &input)
}
