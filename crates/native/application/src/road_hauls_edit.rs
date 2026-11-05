//! `road.hauls.edit` v1 (plan/road.md Y9, docs/adr/0907): a mass haul diagram written (new, or the given fields
//! replacing its) or removed; one undo step “Brükner”. A diagram follows its volume: it is not rebound. The shared cases
//! are `fixtures/commands/v1/road.hauls.edit.json`.

use kentos_commands::{CommandResult, HaulDef, HaulsEdit, HaulsWritten, RoadEditOperation};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_hauls as common;
use crate::road_sections::judged;
use crate::road_terrains::fail;
use crate::road_volumes as volumes;

pub(crate) const STEP: &str = "Brükner";

struct Checked {
    list: Vec<HaulDef>,
    target: Option<String>,
}

fn check(doc: &kentos_domain::Document, input: &HaulsEdit) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    if input.operation == RoadEditOperation::Rebind {
        return Err(fail(
            codes::INVALID_HAUL,
            "Brükner kübajını izler, yeniden bağlanmaz; kübajın koridorunu aksa yeniden bağlayın.".into(),
            "operation",
        ));
    }
    let own = doc.road_hauls();
    let mut list = own.to_vec();
    let found = input.haul.as_deref().and_then(|t| common::find(own, t, "haul").ok());
    if input.operation == RoadEditOperation::Remove && found.is_none() {
        let name = input.haul.clone().unwrap_or_default();
        return Err(common::find(own, &name, "haul").err().unwrap_or(Stop::Cancelled));
    }
    let volume = match input.volume.as_deref() {
        Some(v) => Some(volumes::find(doc.road_volumes(), v, "volume")?.1.id.clone()),
        None => None,
    };
    let target = match (input.operation, found) {
        (RoadEditOperation::Remove, Some((_, old))) if old.approved.is_some() || !old.history.is_empty() => {
            return Err(fail(
                codes::INVALID_HAUL,
                format!("“{}” Brüknerinin onaylı taşıma planı ya da onay geçmişi var; onay ve geçmişi silinmez (ADR 0954). Brükneri bırakıp yenisini yazın.", old.name),
                "haul",
            ));
        }
        (RoadEditOperation::Remove, Some((at, _))) => {
            list.remove(at);
            None
        }
        (RoadEditOperation::Set, Some((at, old))) => {
            if volume.as_ref().is_some_and(|v| *v != old.volume) {
                return Err(fail(
                    codes::INVALID_HAUL,
                    format!("“{}” Brüknerinin kübajı değiştirilmez; başka kübaj için yeni bir Brükner yazın.", old.name),
                    "volume",
                ));
            }
            if input.name.is_none()
                && input.balance.is_none()
                && input.free.is_none()
                && input.sites.is_none()
                && input.cost.is_none()
                && input.unsuitable.is_none()
                && input.barriers.is_none()
            {
                return Err(fail(
                    codes::INVALID_HAUL,
                    "Değiştirilecek bir şey yok: name, balance, free, sites, cost, unsuitable ya da barriers verin.".into(),
                    "",
                ));
            }
            let mut d = old.clone();
            if let Some(n) = &input.name {
                d.name = n.clone();
            }
            if let Some(b) = input.balance {
                d.balance = b;
            }
            if let Some(f) = input.free {
                d.free = f;
            }
            if let Some(s) = &input.sites {
                d.sites = s.clone();
            }
            if let Some(c) = &input.cost {
                d.cost = c.clone();
            }
            if let Some(u) = &input.unsuitable {
                d.unsuitable = u.clone();
            }
            if let Some(b) = &input.barriers {
                d.barriers = b.clone();
            }
            if d != *old {
                d.revision = old.revision.saturating_add(1);
            }
            list[at] = d;
            Some(list[at].id.clone())
        }
        (RoadEditOperation::Set, None) => {
            let missing = [("name", input.name.is_none()), ("volume", volume.is_none())].into_iter().find(|(_, m)| *m);
            let (Some(name), Some(v), None) = (&input.name, volume, missing) else {
                return Err(fail(
                    codes::INVALID_HAUL,
                    "Yeni Brükner için name ve volume verilmeli.".into(),
                    missing.map_or("", |(f, _)| f),
                ));
            };
            let id = input.haul.clone().unwrap_or_else(|| {
                (1..).map(|n| format!("h-{n}")).find(|id| own.iter().all(|h| &h.id != id)).unwrap_or_default()
            });
            list.push(HaulDef {
                id: id.clone(),
                name: name.clone(),
                volume: v,
                revision: 1,
                balance: input.balance.unwrap_or(0.0),
                free: input.free.flatten(),
                sites: input.sites.clone().unwrap_or_default(),
                cost: input.cost.clone().flatten(),
                unsuitable: input.unsuitable.clone().unwrap_or_default(),
                barriers: input.barriers.clone().unwrap_or_default(),
                approved: None,
                history: Vec::new(),
            });
            Some(id)
        }
        _ => return Err(Stop::Cancelled),
    };
    let mut road = doc.settings().road.clone().unwrap_or_default();
    road.hauls = list.clone();
    judged(&road, codes::INVALID_HAUL, "Brükner")?;
    if let Some(d) = target.as_ref().and_then(|id| list.iter().find(|h| &h.id == id)) {
        common::read(doc, d, false)?;
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked { list, target })
}

fn write(cx: &mut ExecutionContext<'_>, c: Checked) -> CommandResult<HaulsWritten> {
    let doc = &mut *cx.doc;
    if let Err(r) = doc.set_road_hauls(c.list.clone(), STEP) {
        return fail(codes::INVALID_HAUL, format!("Brükner kurallarına uymuyor: {}.", r.0), "").into();
    }
    let haul = c.target.as_ref().and_then(|id| c.list.iter().find(|h| &h.id == id)).map(|d| common::report(doc, d));
    CommandResult::Completed {
        output: HaulsWritten { haul, hauls: common::listed(&c.list), revision: doc.revision().to_string() },
        warnings: vec![],
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &HaulsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: vec![] },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &HaulsEdit) -> CommandResult<HaulsWritten> {
    let c = match check(cx.doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    let mut planned = write(&mut ExecutionContext::new(&mut copy), c);
    if let CommandResult::Completed { output, .. } = &mut planned {
        output.revision = cx.doc.revision().to_string();
    }
    planned
}

/// Writes the diagram: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: HaulsEdit) -> CommandResult<HaulsWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_HAULS_EDIT,
        kentos_commands::ROAD_HAULS_EDIT_VERSION,
        input,
        |cx, input| match check(cx.doc, &input) {
            Ok(c) => write(cx, c),
            Err(stop) => stop.into(),
        },
    )
}
