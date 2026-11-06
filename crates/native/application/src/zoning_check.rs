//! `cad.zoning.check` v1 (plan/map-cad.md MAP-09, docs/adr/1420): İmar planı denetimi, read only. The layers and the
//! value lists come from the KentOS profile `imar-plani` (data, not code); the overlaps of decisions, a plan boundary
//! not covered by decisions and a decision outside the plan come from the topology core's rules (Çakışmamalı,
//! … tarafından kapsanmalı) over those two layers. Nothing is written; no legal or schema conformity is claimed. The
//! shared cases are `fixtures/commands/v1/cad.zoning.check.json`.

use std::collections::{BTreeMap, HashSet};

use kentos_commands::{
    CommandResult, DataProfile, Entity, ZONING_PROFILE, ZoningCheck, ZoningChecked, ZoningFinding, builtin_profile,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::topology_rules::{Kind, Objects, Rule, check};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::geometry::shape;
use crate::profiles::{Place, find, path_words};

/// The kinds in the order the answer lists them.
pub const KINDS: [&str; 15] = [
    "missingLayer",
    "noLevel",
    "upperMissing",
    "levelOrder",
    "noPlan",
    "noUse",
    "unknownUse",
    "missingValue",
    "declaredValued",
    "kaksReview",
    "overlap",
    "uncovered",
    "outside",
    "noteMissing",
    "noteUnused",
];

/// The uses on which nothing is built: they ask for no TAKS, KAKS or height.
const OPEN_USES: [&str; 4] = ["park", "yesil", "meydan", "yol"];

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

fn run(doc: &Document, input: &ZoningCheck) -> Result<ZoningChecked, Stop> {
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
    let mut findings: Vec<(usize, i64, ZoningFinding)> = Vec::new();
    let mut say = |kind: &str, items: &[&Item<'_>], area: Option<f64>, message: String| {
        let order = KINDS.iter().position(|k| *k == kind).unwrap_or(KINDS.len());
        let mut items = items.to_vec();
        items.sort_by_key(|i| i.at);
        let first = items.first().map_or(-1, |i| i.at as i64);
        findings.push((
            order,
            first,
            ZoningFinding { kind: kind.to_owned(), uids: items.iter().map(|i| i.uid.clone()).collect(), area, message },
        ));
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

    findings.sort_by_key(|(order, first, _)| (*order, *first));
    let findings: Vec<ZoningFinding> = findings.into_iter().map(|(_, _, f)| f).collect();
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    for f in &findings {
        *counts.entry(f.kind.clone()).or_default() += 1;
    }
    let n = |v: usize| v.min(u32::MAX as usize) as u32;
    Ok(ZoningChecked {
        profile: ZONING_PROFILE.to_owned(),
        tolerance,
        plans: n(plans.len()),
        decisions: n(decisions.len()),
        notes: n(notes.len()),
        findings,
        counts,
    })
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
