//! `cad.zoning.symbolize` v1 (plan/map-cad.md MAP-09, docs/adr/4240): Sembolü karardan ver. Each decision area of the
//! profile `imar-plani` takes its MPYY symbol from its decision: its plan (`Plan`; the only plan boundary when it names
//! none) gives the level (`Seviye`), its use (`Kullanim`) the row of KentOS's table
//! (`catalog/v1/profiles/imar-plani-semboller.json`) or the user's pick, which goes first. Only the object's library
//! symbol is written, in one undo step; the attributes, which are the decision, are never touched, so a symbol does
//! not make a decision and a symbol changed later changes none. The shared cases are
//! `fixtures/commands/v1/cad.zoning.symbolize.json`.

use std::collections::HashMap;

use kentos_commands::{
    CommandResult, DataProfile, Entity, ZONING_PROFILE, ZoningSymbolRow, ZoningSymbolize, ZoningSymbolized,
    builtin_profile, builtin_zoning_symbols,
};
use kentos_domain::{Document, Slot, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::profiles::{Place, find, path_words};

/// The undo step's name.
pub const STEP: &str = "Sembolü karardan ver";

fn fail(code: &str, message: String, path: Option<&str>) -> Stop {
    Stop::failed(error(code, message, path.map(str::to_owned)))
}

fn listed(p: &DataProfile, domain: &str, code: &str) -> bool {
    p.domains
        .iter()
        .find(|d| d.id == domain)
        .and_then(|d| d.values.as_ref())
        .is_some_and(|v| v.iter().any(|c| c.code == code))
}

fn attr(e: &Entity, key: &str) -> String {
    e.base().attrs.get(key).map(|v| v.trim().to_owned()).unwrap_or_default()
}

/// The objects whose symbol changes and the answer, or why not.
/// `writes`: whether the symbols are to be written (a locked layer refuses then); the zoning check reads the rows
/// only (docs/adr/4640).
fn run(doc: &Document, input: &ZoningSymbolize, writes: bool) -> Result<(Vec<(Slot, Entity)>, ZoningSymbolized), Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let (Some(p), Some(table)) = (builtin_profile(ZONING_PROFILE), builtin_zoning_symbols()) else {
        return Err(fail(codes::INVALID_INPUT, "İmar planı profili ya da sembol tablosu okunamadı.".into(), None));
    };
    // The user's picks, before KentOS's table.
    let mut picks: HashMap<(String, String), String> = HashMap::new();
    for (i, pick) in input.symbols.iter().enumerate() {
        let at = format!("symbols[{i}]");
        let (level, usage, symbol) = (pick.level.trim(), pick.usage.trim(), pick.symbol.trim());
        let Some(lv) = table.level(level) else {
            return Err(fail(
                codes::INVALID_INPUT,
                format!(
                    "“{level}” kademesinin MPYY sembolleri yok; KentOS uygulama (uip) ve nazım (nip) imar planı \
                     kademelerinin sembollerini bilir."
                ),
                Some(&format!("{at}.level")),
            ));
        };
        if !listed(&p, "plan-kullanim", usage) {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("“{usage}” kullanım listesinde yok (plan-kullanim)."),
                Some(&format!("{at}.use")),
            ));
        }
        if picks.contains_key(&(level.to_owned(), usage.to_owned())) {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("“{level}” kademesinde “{usage}” kullanımının sembolü iki kez verildi."),
                Some(&at),
            ));
        }
        if !lv.choices.iter().any(|c| c.id == symbol) {
            return Err(fail(
                codes::UNKNOWN_PLAN_SYMBOL,
                format!("“{symbol}”, {} alan sembolleri arasında yok; kademenin sembollerinden birini seçin.", lv.library),
                Some(&format!("{at}.symbol")),
            ));
        }
        picks.insert((level.to_owned(), usage.to_owned()), symbol.to_owned());
    }
    let layer_of = |class: &str| {
        p.classes.iter().find(|c| c.id == class).map(|c| (find(doc.layers().nodes(), &c.layer), path_words(&c.layer)))
    };
    let decision_layer = match layer_of("karar-alani") {
        Some((Place::Layer(id), _)) => id,
        Some((_, words)) => {
            return Err(fail(
                codes::LAYER_NOT_FOUND,
                format!("“{words}” katmanı yok; {ZONING_PROFILE} profilini uygulayın (cad.profiles.apply)."),
                None,
            ));
        }
        None => return Err(fail(codes::INVALID_INPUT, "İmar planı profilinde karar alanı sınıfı yok.".into(), None)),
    };
    let order: HashMap<u32, usize> = doc.entities().enumerate().map(|(at, e)| (e.base().id, at)).collect();
    // The plans: their id and level, in the drawing's order.
    let plans: Vec<(String, String)> = match layer_of("plan-siniri") {
        Some((Place::Layer(id), _)) => doc
            .entities()
            .filter(|e| e.base().layer_id == id && matches!(e, Entity::Polygon(_)))
            .map(|e| (attr(e, "PlanKimlik"), attr(e, "Seviye")))
            .collect(),
        _ => Vec::new(),
    };
    let on_layer = |e: &Entity| e.base().layer_id == decision_layer && matches!(e, Entity::Polygon(_));
    // The decision areas, each with where a refusal about it points.
    let mut areas: Vec<(Slot, &Entity, String, Option<String>)> = Vec::new();
    match &input.uids {
        Some(uids) => {
            for (i, uid) in uids.iter().enumerate() {
                let at = format!("uids[{i}]");
                if uids[..i].contains(uid) {
                    return Err(fail(codes::INVALID_INPUT, format!("“{uid}” alanı iki kez verildi."), Some(&at)));
                }
                if !checks::is_uid_text(uid) {
                    return Err(fail(
                        codes::INVALID_UID,
                        format!("“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir."),
                        Some(&at),
                    ));
                }
                let Some((slot, e)) =
                    Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s).map(|e| (s, e)))
                else {
                    return Err(fail(codes::ENTITY_NOT_FOUND, format!("“{uid}” kimlikli nesne çizimde yok."), Some(&at)));
                };
                if !on_layer(e) {
                    return Err(fail(
                        codes::NOT_A_DECISION,
                        format!("“{uid}” imar planının karar katmanında bir alan değil; sembol karar alanlarına verilir."),
                        Some(&at),
                    ));
                }
                areas.push((slot, e, uid.clone(), Some(at)));
            }
            areas.sort_by_key(|(_, e, _, _)| order.get(&e.base().id).copied().unwrap_or(usize::MAX));
        }
        None => {
            for e in doc.entities().filter(|e| on_layer(e)) {
                let slot = Slot(e.base().id);
                let uid = doc.uid(slot).map(|u| u.to_string()).unwrap_or_default();
                areas.push((slot, e, uid, None));
            }
        }
    }
    let mut rows = Vec::new();
    let mut changed = Vec::new();
    for (slot, e, uid, at) in areas {
        let written = attr(e, "Plan");
        let plan = if written.is_empty() {
            (plans.len() == 1).then(|| &plans[0])
        } else {
            plans.iter().find(|(id, _)| *id == written)
        };
        let usage = attr(e, "Kullanim");
        let mut row = ZoningSymbolRow {
            uid,
            plan: plan.map_or(written, |(id, _)| id.clone()),
            level: plan.map(|(_, level)| level.clone()).unwrap_or_default(),
            usage: usage.clone(),
            symbol: None,
            from: None,
            was: e.base().symbol.clone(),
            state: String::new(),
        };
        row.state = if plan.is_none() {
            "noPlan"
        } else if row.level.is_empty() {
            "noLevel"
        } else if usage.is_empty() {
            "noUse"
        } else if !listed(&p, "plan-kullanim", &usage) {
            "unknownUse"
        } else {
            let user = picks.get(&(row.level.clone(), usage.clone())).cloned();
            let kentos = || {
                table.level(&row.level).and_then(|lv| lv.uses.iter().find(|u| u.code == usage)).map(|u| u.symbol.clone())
            };
            let (symbol, from) = match user {
                Some(s) => (Some(s), "user"),
                None => (kentos(), "kentos"),
            };
            match symbol {
                None => "unmapped",
                Some(s) => {
                    let same = e.base().symbol.as_deref() == Some(s.as_str());
                    row.symbol = Some(s.clone());
                    row.from = Some(from.to_owned());
                    if same {
                        "same"
                    } else {
                        if writes && doc.layers().is_locked(&e.base().layer_id) {
                            return Err(fail(
                                codes::LAYER_LOCKED,
                                format!(
                                    "“{}” katmanı kilitli: karar alanının sembolü yazılamaz. Kilidi açın.",
                                    doc.layers().path(&e.base().layer_id)
                                ),
                                at.as_deref(),
                            ));
                        }
                        let mut next = e.clone();
                        next.base_mut().symbol = Some(s);
                        changed.push((slot, next));
                        "set"
                    }
                }
            }
        }
        .to_owned();
        rows.push(row);
    }
    let count = |state: &[&str]| rows.iter().filter(|r| state.contains(&r.state.as_str())).count() as u32;
    let out = ZoningSymbolized {
        set: count(&["set"]),
        same: count(&["same"]),
        unmapped: count(&["unmapped"]),
        skipped: count(&["noPlan", "noLevel", "noUse", "unknownUse"]),
        areas: rows,
        revision: doc.revision().to_string(),
    };
    Ok((changed, out))
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ZoningSymbolize) -> CommandResult<()> {
    match run(cx.doc, input, false) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// What execute would write; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ZoningSymbolize) -> CommandResult<ZoningSymbolized> {
    match run(cx.doc, input, false) {
        Ok((_, out)) => CommandResult::Completed { output: out, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

fn write(cx: &mut ExecutionContext<'_>, input: ZoningSymbolize) -> CommandResult<ZoningSymbolized> {
    let (changed, mut out) = match run(cx.doc, &input, true) {
        Ok(done) => done,
        Err(stop) => return stop.into(),
    };
    cx.doc.update_many(changed, STEP);
    out.revision = cx.doc.revision().to_string();
    CommandResult::Completed { output: out, warnings: Vec::new() }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: ZoningSymbolize) -> CommandResult<ZoningSymbolized> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_ZONING_SYMBOLIZE,
        kentos_commands::CAD_ZONING_SYMBOLIZE_VERSION,
        input,
        write,
    )
}

/// The decision areas as `cad.zoning.symbolize` reads them without the user's picks, writing nothing (docs/adr/4640, the
/// zoning check's symbol rule): none when the table or the decision layer is not there.
pub(crate) fn rows(doc: &Document) -> Option<Vec<kentos_commands::ZoningSymbolRow>> {
    run(doc, &ZoningSymbolize::default(), false).ok().map(|(_, out)| out.areas)
}

/// A library symbol's name as KentOS's table lists it among a level's choices; the id itself when it is not there.
pub(crate) fn symbol_name(id: &str) -> String {
    builtin_zoning_symbols()
        .and_then(|t| t.levels.iter().flat_map(|l| &l.choices).find(|c| c.id == id).map(|c| c.name.clone()))
        .unwrap_or_else(|| id.to_owned())
}
