//! `cad.zoning.parcel` v1 (“Parselin plan kararları”, docs/adr/8072 §5), read only: the zoning plan's decisions on
//! the building's parcel, as the drawing's layers of the profile `imar-plani` give them (from the municipality's CBS).
//! Nothing is written; a drawn intersection is no legal ruling, so the decisions are shown with their source and the
//! project's İmar durumu is never set from them. The shared cases are `fixtures/commands/v1/cad.zoning.parcel.json`
//! (`scripts/fixtures/zoning_parcel_cases.py`).
//!
//! - The parcel: the one given, else the site plan's (docs/adr/8032); a closed line or a polygon, its arcs kept.
//! - A decision's share: its common part with the parcel, each piece counted when its mean width 2A/P passes the
//!   tolerance (İmar planı denetimi's rule, docs/adr/0202 §2: a sliver along a shared edge is no piece).
//! - Its plan: the plan boundary whose PlanKimlik its Plan names; its notes: the numbers its PlanNotlari names (a comma,
//!   a semicolon or a space between them) among its plan's notes, then its plan's general notes (Kapsam genel).
//! - A conflict: two decisions' shares overlapping in a counted piece, each field they write differently (numbers
//!   compared as numbers, "1,20" and "1.2" the same) — both are said, neither chosen.
//! - Against İmar durumu: the typed emsal and TAKS where a decision on the parcel writes another KAKS or TAKS.
//! - Notes: no decision layer; the parcel under several decisions (a kademe hattı, PAİY 4/1-mm, read from the plan);
//!   a plan not in force (Durum not onaylı or yürürlükte) or without its approval date; a note named but not drawn.

use std::collections::BTreeSet;

use kentos_commands::{
    ZoningParcelDecided, ZoningParcel, CommandResult, Entity, ParcelDecision, PlanAgainst, PlanConflict,
    PlanNoteText, PlanSource, PlanValue, ZONING_PROFILE, builtin_profile,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::region::{intersect_area_sets, net_area, subtract_areas, union_areas};
use kentos_geometry_core::geom::space::perimeter;
use kentos_geometry_core::ops::areas::areas_of_entity;

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::geometry::shape;
use crate::profiles::{Place, find};

/// İmar planı denetimi's tolerance, metres (docs/adr/1420).
const FIRST_TOLERANCE: f64 = 0.01;
const LEAST_TOLERANCE: f64 = 1e-6;

/// A decision's fields in the profile's order.
const FIELDS: [&str; 10] =
    ["Kullanim", "Nizam", "TAKS", "KAKS", "KatAdedi", "Hmax", "OnCekme", "YanCekme", "ArkaCekme", "Belirtilmeyen"];

/// The fields compared as numbers.
const NUMBERS: [&str; 7] = ["TAKS", "KAKS", "KatAdedi", "Hmax", "OnCekme", "YanCekme", "ArkaCekme"];

/// The statuses a plan is in force under (the profile's `plan-durum`).
const IN_FORCE: [&str; 2] = ["onayli", "yururlukte"];

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// An object of a profile's class: its persistent id and its entity.
struct Item<'a> {
    uid: String,
    e: &'a Entity,
}

impl Item<'_> {
    fn get(&self, k: &str) -> String {
        self.e.base().attrs.get(k).map(|v| v.trim().to_owned()).unwrap_or_default()
    }

    fn opt(&self, k: &str) -> Option<String> {
        Some(self.get(k)).filter(|v| !v.is_empty())
    }
}

/// A decision's value read as a number (docs/adr/8072 §5, 8140): a comma or a point before its decimals; none for
/// words ("serbest") or a number with something after it ("3+Ç", "12.50 m").
pub fn plan_number(text: &str) -> Option<f64> {
    text.trim().replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Two values as written the same: numbers as numbers, text as text.
fn same(field: &str, a: &str, b: &str) -> bool {
    if NUMBERS.contains(&field)
        && let (Some(x), Some(y)) = (plan_number(a), plan_number(b))
    {
        return x == y;
    }
    a == b
}

/// The pieces of `parts` whose mean width 2A/P passes `t`, and their area.
fn counted(parts: Vec<Area>, t: f64) -> (Vec<Area>, f64) {
    let kept: Vec<Area> = parts
        .into_iter()
        .filter(|p| {
            let p_len = perimeter(std::slice::from_ref(p));
            p_len > 0.0 && 2.0 * net_area(p) / p_len > t
        })
        .collect();
    let a = kept.iter().map(net_area).sum();
    (kept, a)
}

fn check(doc: &Document, input: &ZoningParcel) -> Result<ZoningParcelDecided, Stop> {
    let t = input.tolerance.unwrap_or(FIRST_TOLERANCE);
    if !(t.is_finite() && t >= LEAST_TOLERANCE) {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("Tolerans sonlu ve en az {LEAST_TOLERANCE} m olmalı (yazılan {t})."),
            "tolerance",
        ));
    }
    let site = doc.settings().arch.as_ref().and_then(|a| a.site.as_ref()).and_then(|s| s.parcel.clone());
    let Some(uid) = input.parcel.clone().or(site) else {
        return Err(fail(
            codes::PARCEL_MISSING,
            "Parsel verilmedi ve vaziyet planında parsel yok. Parselin kalıcı kimliğini verin ya da parseli Mimari › \
             Proje ▾ › Parsel ve bahçe mesafeleri'nde seçin."
                .into(),
            "parcel",
        ));
    };
    let entity = kentos_domain::Uuid::parse_str(&uid)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|s| doc.get(s))
        .ok_or_else(|| {
            fail(
                codes::ENTITY_NOT_FOUND,
                format!("“{uid}” kimlikli nesne çizimde yok. Parselin kalıcı kimliğini verin."),
                "parcel",
            )
        })?;
    let parcel = areas_of_entity(&shape(entity));
    let area: f64 = parcel.iter().map(net_area).sum();
    if parcel.is_empty() || !(area > 0.0) {
        return Err(fail(
            codes::NOT_A_PARCEL,
            "Parsel kapalı bir çizgi ya da alan değil; parselin sınırını kapalı çizin.".into(),
            "parcel",
        ));
    }
    let mut notes: Vec<String> = Vec::new();
    let Some(profile) = builtin_profile(ZONING_PROFILE) else {
        return Err(fail(codes::INVALID_INPUT, "İmar planı profili okunamadı.".into(), "profile"));
    };
    // Each class's objects on its layer, in the drawing's order.
    let order: Vec<&Entity> = doc.entities().collect();
    let of = |class: &str, kind: &str| -> Option<Vec<Item<'_>>> {
        let c = profile.classes.iter().find(|c| c.id == class)?;
        let Place::Layer(layer) = find(doc.layers().nodes(), &c.layer) else {
            return None;
        };
        Some(
            order
                .iter()
                .filter(|e| e.base().layer_id == layer && e.kind() == kind)
                .map(|e| Item { uid: doc.uid(Slot(e.base().id)).map(|u| u.to_string()).unwrap_or_default(), e })
                .collect(),
        )
    };
    let Some(decisions) = of("karar-alani", "polygon") else {
        notes.push(
            "Çizimde imar planının karar alanı katmanı yok; plan kararları belediyenin CBS'inden imar-plani \
             profiliyle gelir (cad.profiles.apply)."
                .into(),
        );
        return Ok(ZoningParcelDecided {
            parcel: uid,
            area,
            decisions: Vec::new(),
            uncovered: area,
            conflicts: Vec::new(),
            against: Vec::new(),
            notes,
        });
    };
    let plans = of("plan-siniri", "polygon").unwrap_or_default();
    let plan_notes = of("plan-notu", "point").unwrap_or_default();
    // Each decision's share of the parcel.
    let mut found: Vec<(&Item<'_>, Vec<Area>, f64)> = Vec::new();
    for d in &decisions {
        let (pieces, share) = counted(intersect_area_sets(&[parcel.clone(), areas_of_entity(&shape(d.e))]), t);
        if !pieces.is_empty() {
            found.push((d, pieces, share));
        }
    }
    let covered: Vec<Area> = union_areas(&found.iter().flat_map(|(_, p, _)| p.iter().cloned()).collect::<Vec<_>>());
    let (_, uncovered) = counted(subtract_areas(&parcel, &covered), t);
    let mut said: BTreeSet<String> = BTreeSet::new();
    let mut out: Vec<ParcelDecision> = Vec::new();
    for (d, _, share) in &found {
        let values: Vec<PlanValue> = FIELDS
            .iter()
            .filter_map(|f| d.opt(f).map(|v| PlanValue { field: (*f).to_owned(), value: v }))
            .collect();
        let plan_id = d.get("Plan");
        let plan = plans.iter().find(|p| !plan_id.is_empty() && p.get("PlanKimlik") == plan_id);
        let source = plan.map(|p| PlanSource {
            uid: p.uid.clone(),
            id: p.get("PlanKimlik"),
            name: p.opt("PlanAdi"),
            level: p.opt("Seviye"),
            status: p.opt("Durum"),
            version: p.opt("Surum"),
            approved: p.opt("OnayTarihi"),
            basis: p.opt("Dayanak"),
        });
        if let Some(s) = &source {
            match s.status.as_deref() {
                Some(st) if IN_FORCE.contains(&st) => {}
                Some(st) => {
                    said.insert(format!("{} planının durumu “{st}”; yürürlükte değil.", s.id));
                }
                None => {
                    said.insert(format!("{} planının durumu yazılmamış.", s.id));
                }
            }
            if s.approved.is_none() {
                said.insert(format!("{} planının onay tarihi yok.", s.id));
            }
        } else if plan_id.is_empty() {
            said.insert(format!("“{}” kararının planı yazılmamış.", d.get("KararKimlik")));
        } else {
            said.insert(format!("“{}” kararının planı {plan_id} çizimde yok.", d.get("KararKimlik")));
        }
        // Its notes by number, then its plan's general ones.
        let of_plan = |n: &Item<'_>| {
            let p = n.get("Plan");
            p.is_empty() || p == plan_id
        };
        let mut texts: Vec<PlanNoteText> = Vec::new();
        for no in d.get("PlanNotlari").split([',', ';', ' ']).map(str::trim).filter(|n| !n.is_empty()) {
            match plan_notes.iter().find(|n| of_plan(n) && n.get("NotNo") == no) {
                Some(n) => texts.push(PlanNoteText { number: no.to_owned(), text: n.get("Metin"), scope: n.get("Kapsam") }),
                None => {
                    said.insert(format!(
                        "“{}” kararının andığı {no} numaralı plan notu çizimde yok.",
                        d.get("KararKimlik")
                    ));
                }
            }
        }
        for n in plan_notes.iter().filter(|n| of_plan(n) && n.get("Kapsam") == "genel") {
            if !texts.iter().any(|x| x.number == n.get("NotNo")) {
                texts.push(PlanNoteText { number: n.get("NotNo"), text: n.get("Metin"), scope: n.get("Kapsam") });
            }
        }
        out.push(ParcelDecision {
            uid: d.uid.clone(),
            id: d.get("KararKimlik"),
            share: *share,
            values,
            plan: source,
            notes: texts,
        });
    }
    // Two decisions over the same piece of the parcel.
    let mut conflicts: Vec<PlanConflict> = Vec::new();
    for i in 0..found.len() {
        for j in i + 1..found.len() {
            let (a, pa, _) = &found[i];
            let (b, pb, _) = &found[j];
            let (_, both) = counted(intersect_area_sets(&[pa.clone(), pb.clone()]), t);
            if both <= 0.0 {
                continue;
            }
            for f in FIELDS {
                let (va, vb) = (a.get(f), b.get(f));
                if !same(f, &va, &vb) {
                    conflicts.push(PlanConflict {
                        field: f.to_owned(),
                        area: both,
                        decisions: vec![a.get("KararKimlik"), b.get("KararKimlik")],
                        values: vec![va, vb],
                    });
                }
            }
        }
    }
    if out.len() > 1 {
        notes.push(format!(
            "Parsel {} karar alanına isabet ediyor ({}); kademe hattı ve hangi kararın nerede geçtiği plandan okunur \
             (PAİY 4/1-mm).",
            out.len(),
            out.iter().map(|d| d.id.as_str()).collect::<Vec<_>>().join(", ")
        ));
    }
    notes.extend(said);
    // The typed İmar durumu against the decisions' values.
    let mut against: Vec<PlanAgainst> = Vec::new();
    if let Some(z) = doc.settings().arch.as_ref().and_then(|a| a.zoning.as_ref()) {
        for (field, typed) in [("KAKS", z.far), ("TAKS", z.coverage)] {
            let Some(typed) = typed else { continue };
            let mut plan: Vec<String> = Vec::new();
            for d in &out {
                if let Some(v) = d.values.iter().find(|v| v.field == field)
                    && !plan.contains(&v.value)
                {
                    plan.push(v.value.clone());
                }
            }
            if plan.iter().any(|v| plan_number(v).is_none_or(|x| x != typed)) {
                against.push(PlanAgainst { field: field.to_owned(), typed, plan });
            }
        }
    }
    Ok(ZoningParcelDecided { parcel: uid, area, decisions: out, uncovered, conflicts, against, notes })
}

/// Reads the parcel's decisions; writes nothing.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ZoningParcel) -> CommandResult<ZoningParcelDecided> {
    plan_in(cx.doc, &input)
}

/// The same answer as `execute`.
pub fn plan(cx: &ExecutionContext<'_>, input: &ZoningParcel) -> CommandResult<ZoningParcelDecided> {
    plan_in(cx.doc, input)
}

/// The answer read from `doc`.
pub fn plan_in(doc: &Document, input: &ZoningParcel) -> CommandResult<ZoningParcelDecided> {
    match check(doc, input) {
        Ok(output) => CommandResult::Completed { output, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// Checks `input`; writes nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ZoningParcel) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}
