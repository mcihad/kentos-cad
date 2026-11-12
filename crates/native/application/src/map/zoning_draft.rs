//! `cad.zoning.draft` v1 (plan/map-cad.md MAP-09, docs/adr/5040): Plan değişikliği taslağı. A draft of a change to a
//! plan of the profile `imar-plani` is kept in the project (`settings.zoningPlan.drafts`) with the plan, the drawing's
//! revision, the reason (Mekânsal Planlar Yapım Yönetmeliği m. 9(4)) and each changed field's value before and after;
//! the decision areas in force are never written, so the plan in force stays as it is. The answer compares the plan's
//! decision areas by use before and after (the shared core's planar `entity_area`) and says what m. 26 asks: a social
//! or technical infrastructure use made smaller (m. 26(3)), a density raised (m. 26(5)(a)), a height raised (m. 26(4),
//! (5)(b)); it judges neither equivalence nor conformity. The zoning check says when a draft's area changed after it
//! (`draftStale`, [`stale`]). The shared cases are `fixtures/commands/v1/cad.zoning.draft.json`.

use std::collections::HashMap;

use kentos_commands::{
    CommandResult, DRAFT_FIELDS, DataProfile, Entity, INFRASTRUCTURE_USES, MAX_DRAFT_ID, MAX_DRAFT_REASON,
    MAX_ZONING_LABEL, PlanDraft, PlanDraftChange, ZONING_PROFILE, ZoningDraft, ZoningDraftFinding, ZoningDraftUse,
    ZoningDrafted, ZoningPlanData, builtin_profile,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::entity::entity_area;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::geometry::shape;
use crate::profiles::{Place, find, path_words};

/// The undo step's name.
pub const STEP: &str = "Plan değişikliği taslağı";
/// The undo step's name when a draft is taken out.
pub const REMOVE_STEP: &str = "Plan değişikliği taslağını sil";

/// The fields whose value is a number of 0 or more.
const NUMBERS: [&str; 7] = ["TAKS", "KAKS", "KatAdedi", "Hmax", "OnCekme", "YanCekme", "ArkaCekme"];

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn listed(p: &DataProfile, domain: &str, code: &str) -> bool {
    p.domains
        .iter()
        .find(|d| d.id == domain)
        .and_then(|d| d.values.as_ref())
        .is_some_and(|v| v.iter().any(|c| c.code == code))
}

fn label(p: &DataProfile, domain: &str, code: &str) -> String {
    p.domains
        .iter()
        .find(|d| d.id == domain)
        .and_then(|d| d.values.as_ref())
        .and_then(|v| v.iter().find(|c| c.code == code))
        .map_or_else(|| code.to_owned(), |c| c.label.clone())
}

fn attr(e: &Entity, key: &str) -> String {
    e.base().attrs.get(key).map(|v| v.trim().to_owned()).unwrap_or_default()
}

/// A value as a number: a dot or a comma as the decimal sign; none when it is not a finite number.
fn number(text: &str) -> Option<f64> {
    text.trim().replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Whether two values of a field are the same: as numbers for a measured field when both are numbers, else as text.
fn same_value(field: &str, a: &str, b: &str) -> bool {
    if NUMBERS.contains(&field)
        && let (Some(x), Some(y)) = (number(a), number(b))
    {
        return x == y;
    }
    a == b
}

fn text_ok(t: &str, most: usize) -> bool {
    !t.is_empty() && t.trim() == t && t.chars().count() <= most && !t.chars().any(char::is_control)
}

/// The drawing's zoning layers: the plan boundaries' and the decision areas'.
struct Layers {
    plans: String,
    decisions: String,
}

fn layers(doc: &Document, p: &DataProfile) -> Result<Layers, Stop> {
    let layer = |class: &str| -> Result<String, Stop> {
        let Some(c) = p.classes.iter().find(|c| c.id == class) else {
            return Err(fail(codes::INVALID_INPUT, "İmar planı profilinde plan sınırı ya da karar alanı sınıfı yok.".into(), "plan"));
        };
        match find(doc.layers().nodes(), &c.layer) {
            Place::Layer(id) => Ok(id),
            _ => Err(Stop::failed(error(
                codes::LAYER_NOT_FOUND,
                format!("“{}” katmanı yok; {ZONING_PROFILE} profilini uygulayın (cad.profiles.apply).", path_words(&c.layer)),
                None,
            ))),
        }
    };
    Ok(Layers { plans: layer("plan-siniri")?, decisions: layer("karar-alani")? })
}

fn is_area(e: &Entity, layer: &str) -> bool {
    e.base().layer_id == layer && matches!(e, Entity::Polygon(_))
}

/// The plan boundaries' ids and levels, in the drawing's order.
fn plans_of(doc: &Document, l: &Layers) -> Vec<(String, String)> {
    doc.entities().filter(|e| is_area(e, &l.plans)).map(|e| (attr(e, "PlanKimlik"), attr(e, "Seviye"))).collect()
}

/// The plan a decision area belongs to: the one its `Plan` names, or the only one when it names none.
fn plan_of(e: &Entity, plans: &[(String, String)]) -> Option<String> {
    let written = attr(e, "Plan");
    if written.is_empty() {
        (plans.len() == 1).then(|| plans[0].0.clone())
    } else {
        plans.iter().any(|(id, _)| *id == written).then_some(written)
    }
}

/// The draft input makes, or why not; the project's data after it.
fn run(doc: &Document, input: &ZoningDraft) -> Result<(ZoningDrafted, ZoningPlanData), Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let id = input.id.as_str();
    if !text_ok(id, MAX_DRAFT_ID) {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("Taslağın kimliği boşluksuz başlayıp biten 1–{MAX_DRAFT_ID} karakterdir (yazılan “{id}”)."),
            "id",
        ));
    }
    let mut data = doc.zoning_plan().cloned().unwrap_or_default();
    if input.remove {
        let before = data.drafts.len();
        data.drafts.retain(|d| d.id != id);
        if data.drafts.len() == before {
            return Err(fail(codes::UNKNOWN_DRAFT, format!("“{id}” kimlikli plan değişikliği taslağı projede yok."), "id"));
        }
        let out = ZoningDrafted { drafts: count(&data), revision: doc.revision().to_string(), ..ZoningDrafted::default() };
        return Ok((out, data));
    }
    let Some(p) = builtin_profile(ZONING_PROFILE) else {
        return Err(fail(codes::INVALID_INPUT, "İmar planı profili okunamadı.".into(), "plan"));
    };
    let l = layers(doc, &p)?;
    let plans = plans_of(doc, &l);
    let plan = input.plan.trim();
    if plan.is_empty() {
        return Err(fail(codes::INVALID_INPUT, "Değişecek planı yazın: plan sınırının PlanKimlik'i.".into(), "plan"));
    }
    let Some((_, level)) = plans.iter().find(|(pid, _)| pid == plan) else {
        return Err(fail(codes::UNKNOWN_PLAN, format!("“{plan}” kimlikli plan sınırı çizimde yok."), "plan"));
    };
    let reason = input.reason.trim();
    if reason.is_empty() {
        return Err(fail(
            codes::INVALID_INPUT,
            "Değişikliğin gerekçesini yazın: Mekânsal Planlar Yapım Yönetmeliği m. 9(4) plan değişikliğinde gerekçeyi ve gereklilik analizlerini açıklayan plan raporunu ister."
                .into(),
            "reason",
        ));
    }
    if reason.chars().count() > MAX_DRAFT_REASON {
        return Err(fail(codes::INVALID_INPUT, format!("Gerekçe en çok {MAX_DRAFT_REASON} karakterdir."), "reason"));
    }
    if input.changes.is_empty() {
        return Err(fail(
            codes::INVALID_INPUT,
            "Taslakta en az bir değişiklik olmalı: karar alanı, alanı ve yeni değeri.".into(),
            "changes",
        ));
    }
    if input.changes.len() > kentos_commands::MAX_DRAFT_CHANGES {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("Taslakta en çok {} değişiklik olur.", kentos_commands::MAX_DRAFT_CHANGES),
            "changes",
        ));
    }
    let mut changes: Vec<PlanDraftChange> = Vec::new();
    for (i, c) in input.changes.iter().enumerate() {
        let at = |f: &str| format!("changes[{i}]{f}");
        let (uid, field, value) = (c.uid.trim(), c.field.trim(), c.value.trim());
        if input.changes[..i].iter().any(|o| o.uid.trim() == uid && o.field.trim() == field) {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("“{uid}” karar alanının {field} alanı iki kez değişiyor."),
                &at(""),
            ));
        }
        if !checks::is_uid_text(uid) {
            return Err(fail(
                codes::INVALID_UID,
                format!("“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir."),
                &at(".uid"),
            ));
        }
        let Some(e) = Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s)) else {
            return Err(fail(codes::ENTITY_NOT_FOUND, format!("“{uid}” kimlikli nesne çizimde yok."), &at(".uid")));
        };
        if !is_area(e, &l.decisions) {
            return Err(fail(
                codes::NOT_A_DECISION,
                format!("“{uid}” imar planının karar katmanında bir alan değil; taslak karar alanlarını değiştirir."),
                &at(".uid"),
            ));
        }
        if plan_of(e, &plans).as_deref() != Some(plan) {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("“{uid}” karar alanı “{plan}” planının değil; taslak bir planın karar alanlarını değiştirir."),
                &at(".uid"),
            ));
        }
        if !DRAFT_FIELDS.contains(&field) {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("Taslak “{field}” alanını değiştirmez; değiştirdikleri: {}.", DRAFT_FIELDS.join(", ")),
                &at(".field"),
            ));
        }
        if value.is_empty() {
            return Err(fail(codes::INVALID_INPUT, format!("{field} için yeni değeri yazın; taslak bir değeri silmez."), &at(".value")));
        }
        if value.chars().count() > MAX_ZONING_LABEL || value.chars().any(char::is_control) {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("{field} değeri en çok {MAX_ZONING_LABEL} karakterlik tek satırdır."),
                &at(".value"),
            ));
        }
        let wrong = match field {
            "Kullanim" => (!listed(&p, "plan-kullanim", value)).then(|| format!("“{value}” kullanım listesinde yok (plan-kullanim).")),
            "Nizam" => (!listed(&p, "yapi-nizami", value)).then(|| format!("“{value}” yapı nizamı listesinde yok (yapi-nizami).")),
            _ => number(value).is_none_or(|v| v < 0.0).then(|| format!("{field} için “{value}” 0 ya da daha büyük bir sayı değil.")),
        };
        if let Some(why) = wrong {
            return Err(fail(codes::INVALID_INPUT, why, &at(".value")));
        }
        let before = attr(e, field);
        if same_value(field, &before, value) {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("“{uid}” karar alanının {field} değeri zaten “{before}”; değişiklik yok."),
                &at(".value"),
            ));
        }
        changes.push(PlanDraftChange { area: uid.to_owned(), field: field.to_owned(), before, after: value.to_owned() });
    }
    let draft = PlanDraft {
        id: id.to_owned(),
        plan: plan.to_owned(),
        revision: doc.revision().to_string(),
        reason: reason.to_owned(),
        changes,
    };
    let (uses, findings) = compare(doc, &p, &l, &plans, &draft);
    match data.drafts.iter_mut().find(|d| d.id == id) {
        Some(old) => *old = draft.clone(),
        None => data.drafts.push(draft.clone()),
    }
    if data.drafts.len() > kentos_commands::MAX_PLAN_DRAFTS {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("Projede en çok {} plan değişikliği taslağı saklanır; eskilerden birini silin.", kentos_commands::MAX_PLAN_DRAFTS),
            "id",
        ));
    }
    let out = ZoningDrafted {
        draft: Some(draft),
        level: level.clone(),
        uses,
        findings,
        drafts: count(&data),
        revision: doc.revision().to_string(),
    };
    Ok((out, data))
}

fn count(d: &ZoningPlanData) -> u32 {
    u32::try_from(d.drafts.len()).unwrap_or(u32::MAX)
}

/// The plan's areas by use before and after the draft, and what m. 26 asks.
fn compare(
    doc: &Document,
    p: &DataProfile,
    l: &Layers,
    plans: &[(String, String)],
    draft: &PlanDraft,
) -> (Vec<ZoningDraftUse>, Vec<ZoningDraftFinding>) {
    let after_of = |uid: &str, field: &str| {
        draft.changes.iter().find(|c| c.area == uid && c.field == field).map(|c| c.after.clone())
    };
    let mut by_use: HashMap<String, (f64, f64)> = HashMap::new();
    let mut per_area: Vec<ZoningDraftFinding> = Vec::new();
    for e in doc.entities().filter(|e| is_area(e, &l.decisions)) {
        if plan_of(e, plans).as_deref() != Some(draft.plan.as_str()) {
            continue;
        }
        let uid = doc.uid(kentos_domain::Slot(e.base().id)).map(|u| u.to_string()).unwrap_or_default();
        let area = entity_area(&shape(e)).unwrap_or(0.0);
        let before = attr(e, "Kullanim");
        let after = after_of(&uid, "Kullanim").unwrap_or_else(|| before.clone());
        by_use.entry(before).or_default().0 += area;
        by_use.entry(after).or_default().1 += area;
        // What rises on this area (m. 26(4), (5)): a field written as a number before and after, larger after.
        let rise = |field: &str, word: &str| {
            let now = attr(e, field);
            let next = after_of(&uid, field)?;
            (number(&next)? > number(&now)?).then(|| format!("{word} {now} → {next}"))
        };
        let name = attr(e, "KararKimlik");
        let name = if name.is_empty() { uid.clone() } else { name };
        let density: Vec<String> = [rise("KAKS", "KAKS"), rise("KatAdedi", "kat adedi")].into_iter().flatten().collect();
        if !density.is_empty() {
            per_area.push(ZoningDraftFinding {
                kind: "densityUp".into(),
                uid: Some(uid.clone()),
                usage: None,
                area: None,
                message: format!(
                    "“{name}” karar alanında yoğunluk artıyor ({}): Mekânsal Planlar Yapım Yönetmeliği m. 26(5)(a) artan nüfusun ihtiyacı olan sosyal ve teknik altyapı alanlarının standartlara uygun ayrılmasını ister.",
                    density.join(", ")
                ),
            });
        }
        let height: Vec<String> = [rise("KatAdedi", "kat adedi"), rise("Hmax", "Hmax")].into_iter().flatten().collect();
        if !height.is_empty() {
            per_area.push(ZoningDraftFinding {
                kind: "heightUp".into(),
                uid: Some(uid.clone()),
                usage: None,
                area: None,
                message: format!(
                    "“{name}” karar alanında yükseklik artıyor ({}): Mekânsal Planlar Yapım Yönetmeliği m. 26(4) yörenin dokusunu ve siluetini, m. 26(5)(b) karşılıklı bina cepheleri arasında en az K = (Y1 + Y2)/2 + 7,00 m uzaklığı ister; KentOS bu uzaklığı ölçmez.",
                    height.join(", ")
                ),
            });
        }
    }
    let order: Vec<String> = p
        .domains
        .iter()
        .find(|d| d.id == "plan-kullanim")
        .and_then(|d| d.values.as_ref())
        .map(|v| v.iter().map(|c| c.code.clone()).collect())
        .unwrap_or_default();
    let mut codes_: Vec<String> = by_use.keys().cloned().collect();
    // The list's order, then uses off the list by their code, then areas without a use.
    codes_.sort_by_key(|c| (c.is_empty(), order.iter().position(|o| o == c).unwrap_or(order.len()), c.clone()));
    let uses: Vec<ZoningDraftUse> = codes_
        .iter()
        .map(|c| {
            let (before, after) = by_use[c];
            ZoningDraftUse { usage: c.clone(), before, after, difference: after - before }
        })
        .collect();
    let mut findings: Vec<ZoningDraftFinding> = uses
        .iter()
        .filter(|u| INFRASTRUCTURE_USES.contains(&u.usage.as_str()) && u.after < u.before)
        .map(|u| ZoningDraftFinding {
            kind: "infrastructureLess".into(),
            uid: None,
            usage: Some(u.usage.clone()),
            area: Some(u.before - u.after),
            message: format!(
                "“{}” küçülüyor: Mekânsal Planlar Yapım Yönetmeliği m. 26(3) sosyal ve teknik altyapı alanının kaldırılmasını ya da küçültülmesini zorunluluğa, ilgili kurumun görüşüne ve hizmet etki alanında eşdeğer yeni alana bağlar; KentOS eşdeğerliği değerlendirmez.",
                label(p, "plan-kullanim", &u.usage)
            ),
        })
        .collect();
    findings.extend(per_area);
    (uses, findings)
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ZoningDraft) -> CommandResult<()> {
    match run(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// What execute would keep; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ZoningDraft) -> CommandResult<ZoningDrafted> {
    match run(cx.doc, input) {
        Ok((out, _)) => CommandResult::Completed { output: out, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

fn write(cx: &mut ExecutionContext<'_>, input: ZoningDraft) -> CommandResult<ZoningDrafted> {
    let (mut out, data) = match run(cx.doc, &input) {
        Ok(done) => done,
        Err(stop) => return stop.into(),
    };
    cx.doc.set_zoning_plan(Some(data), if input.remove { REMOVE_STEP } else { STEP });
    out.revision = cx.doc.revision().to_string();
    CommandResult::Completed { output: out, warnings: Vec::new() }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: ZoningDraft) -> CommandResult<ZoningDrafted> {
    crate::call::recorded(cx, kentos_commands::CAD_ZONING_DRAFT, kentos_commands::CAD_ZONING_DRAFT_VERSION, input, write)
}

/// The project's drafts, for a window.
pub fn drafts(doc: &Document) -> Vec<PlanDraft> {
    doc.zoning_plan().map(|z| z.drafts.clone()).unwrap_or_default()
}

/// The drawing's plan boundaries (id, level) in its order, for a window; none without the zoning layers.
pub fn plans(doc: &Document) -> Vec<(String, String)> {
    builtin_profile(ZONING_PROFILE).and_then(|p| layers(doc, &p).ok()).map(|l| plans_of(doc, &l)).unwrap_or_default()
}

/// A plan's decision areas in the drawing's order, for a window: each one's persistent id, `KararKimlik` and use.
pub fn areas(doc: &Document, plan: &str) -> Vec<(String, String, String)> {
    let Some(l) = builtin_profile(ZONING_PROFILE).and_then(|p| layers(doc, &p).ok()) else { return Vec::new() };
    let plans = plans_of(doc, &l);
    doc.entities()
        .filter(|e| is_area(e, &l.decisions) && plan_of(e, &plans).as_deref() == Some(plan))
        .map(|e| {
            let uid = doc.uid(kentos_domain::Slot(e.base().id)).map(|u| u.to_string()).unwrap_or_default();
            (uid, attr(e, "KararKimlik"), attr(e, "Kullanim"))
        })
        .collect()
}

/// A decision area's value of a field as the draft reads it (trimmed; empty when it has none or is not there).
pub fn value(doc: &Document, uid: &str, field: &str) -> String {
    Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s)).map(|e| attr(e, field)).unwrap_or_default()
}

/// What changed after the drafts (docs/adr/5040, the zoning check's `draftStale`): for each draft in its order, each
/// change whose area is gone or whose field's value is no longer the one the draft read (by its area's persistent id,
/// none when it is gone), and a draft whose plan is gone (no area); with what to say.
pub(crate) fn stale(doc: &Document) -> Vec<(Option<String>, String)> {
    let mut out = Vec::new();
    let Some(data) = doc.zoning_plan() else { return out };
    let plans: Vec<String> = builtin_profile(ZONING_PROFILE)
        .and_then(|p| layers(doc, &p).ok())
        .map(|l| plans_of(doc, &l).into_iter().map(|(id, _)| id).collect())
        .unwrap_or_default();
    for d in &data.drafts {
        if !plans.contains(&d.plan) {
            out.push((
                None,
                format!("“{}” plan değişikliği taslağının planı “{}” çizimde yok; taslağı silin ya da planı geri getirin.", d.id, d.plan),
            ));
        }
        for c in &d.changes {
            let found = Uuid::parse_str(&c.area).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s));
            match found {
                None => out.push((
                    None,
                    format!(
                        "“{}” plan değişikliği taslağının değiştirdiği “{}” karar alanı çizimde yok; taslağı yeniden kaydedin ya da silin.",
                        d.id, c.area
                    ),
                )),
                Some(e) => {
                    let now = attr(e, &c.field);
                    if now != c.before {
                        out.push((
                            Some(c.area.clone()),
                            format!(
                                "“{}” plan değişikliği taslağı eskidi: karar alanının {} değeri taslak yazılırken “{}”, şimdi “{}”. Taslağı yeniden kaydedin.",
                                d.id, c.field, c.before, now
                            ),
                        ));
                    }
                }
            }
        }
    }
    out
}
