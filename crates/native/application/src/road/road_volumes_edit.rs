//! `road.volumes.edit` v1 (plan/road.md Y8, docs/adr/0906 §4): an earthwork volume written (new, or the given fields
//! replacing its) or removed; one undo step “Kübaj”. A volume follows its corridor: it is not rebound. The shared cases
//! are `fixtures/commands/v1/road.volumes.edit.json`.

use kentos_commands::{CommandResult, RoadEditOperation, VolumeDef, VolumesEdit, VolumesWritten};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_corridors as corridors;
use crate::road_sections::judged;
use crate::road_terrains::fail;
use crate::road_volumes as common;

pub(crate) const STEP: &str = "Kübaj";

struct Checked {
    list: Vec<VolumeDef>,
    target: Option<String>,
}

fn check(doc: &kentos_domain::Document, input: &VolumesEdit) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    if input.operation == RoadEditOperation::Rebind {
        return Err(fail(
            codes::INVALID_VOLUME,
            "Kübaj hesabı koridorunu izler, yeniden bağlanmaz; koridoru aksa yeniden bağlayın.".into(),
            "operation",
        ));
    }
    let own = doc.road_volumes();
    let mut list = own.to_vec();
    let found = input.volume.as_deref().and_then(|t| common::find(own, t, "volume").ok());
    if input.operation == RoadEditOperation::Remove && found.is_none() {
        let name = input.volume.clone().unwrap_or_default();
        return Err(common::find(own, &name, "volume").err().unwrap_or(Stop::Cancelled));
    }
    if let Some((_, old)) = found.filter(|(_, v)| v.approved.is_some()) {
        return Err(fail(
            codes::VOLUME_APPROVED,
            format!("“{}” kübajı onaylı; değiştirmek ya da silmek için önce onayı kaldırın.", old.name),
            "volume",
        ));
    }
    let corridor = match input.corridor.as_deref() {
        Some(c) => Some(corridors::find(doc.road_corridors(), c, "corridor")?.1.id.clone()),
        None => None,
    };
    let target = match (input.operation, found) {
        (RoadEditOperation::Remove, Some((_, old))) if !old.history.is_empty() => {
            return Err(fail(
                codes::VOLUME_APPROVED,
                format!(
                    "“{}” kübajının {} onaylı hâli geçmişte; onay geçmişi silinmez (ADR 0950). Kübajı bırakıp yenisini yazın.",
                    old.name,
                    old.history.len()
                ),
                "volume",
            ));
        }
        (RoadEditOperation::Remove, Some((at, old))) => {
            let users: Vec<&str> = doc.road_hauls().iter().filter(|h| h.volume == old.id).map(|h| h.name.as_str()).collect();
            if !users.is_empty() {
                return Err(fail(
                    codes::VOLUME_IN_USE,
                    format!("“{}” kübajını şu Brüknerler kullanıyor: {}; önce onları silin.", old.name, users.join(", ")),
                    "volume",
                ));
            }
            list.remove(at);
            None
        }
        (RoadEditOperation::Set, Some((at, old))) => {
            if corridor.as_ref().is_some_and(|c| *c != old.corridor) {
                return Err(fail(
                    codes::INVALID_VOLUME,
                    format!("“{}” kübaj hesabının koridoru değiştirilmez; başka koridor için yeni bir kübaj hesabı yazın.", old.name),
                    "corridor",
                ));
            }
            if input.name.is_none()
                && input.method.is_none()
                && input.factors.is_none()
                && input.strip.is_none()
                && input.bench.is_none()
                && input.rule.is_none()
                && input.curve.is_none()
            {
                return Err(fail(
                    codes::INVALID_VOLUME,
                    "Değiştirilecek bir şey yok: name, method, factors, strip, bench, rule ya da curve verin.".into(),
                    "",
                ));
            }
            let mut d = old.clone();
            if let Some(n) = &input.name {
                d.name = n.clone();
            }
            if let Some(m) = input.method {
                d.method = m;
            }
            if let Some(f) = &input.factors {
                d.factors = f.clone();
            }
            if let Some(v) = &input.strip {
                d.strip = v.clone();
            }
            if let Some(r) = input.rule {
                d.rule = Some(r);
            }
            if let Some(b) = &input.bench {
                d.bench = b.clone();
            }
            if let Some(c) = input.curve {
                d.curve = c;
            }
            // Docs/adr/0974: the grid's cell; another method takes it off.
            if input.method.is_some_and(|m| m != kentos_commands::VolumeMethod::Grid) {
                d.cell = None;
            }
            if let Some(c) = input.cell {
                d.cell = Some(c);
            }
            if d != *old {
                d.revision = old.revision.saturating_add(1);
            }
            list[at] = d;
            Some(list[at].id.clone())
        }
        (RoadEditOperation::Set, None) => {
            let missing = [("name", input.name.is_none()), ("corridor", corridor.is_none()), ("method", input.method.is_none())]
                .into_iter()
                .find(|(_, m)| *m);
            let (Some(name), Some(c), Some(m), None) = (&input.name, corridor, input.method, missing) else {
                return Err(fail(
                    codes::INVALID_VOLUME,
                    "Yeni kübaj hesabı için name, corridor ve method verilmeli.".into(),
                    missing.map_or("", |(f, _)| f),
                ));
            };
            let id = input.volume.clone().unwrap_or_else(|| {
                (1..).map(|n| format!("v-{n}")).find(|id| own.iter().all(|v| &v.id != id)).unwrap_or_default()
            });
            list.push(VolumeDef {
                id: id.clone(),
                name: name.clone(),
                corridor: c,
                revision: 1,
                method: m,
                factors: input.factors.clone().unwrap_or_default(),
                strip: input.strip.clone().unwrap_or_default(),
                bench: input.bench.clone().flatten(),
                approved: None,
                history: Vec::new(),
                rule: input.rule,
                curve: input.curve.unwrap_or(false),
                cell: input.cell,
            });
            Some(id)
        }
        _ => return Err(Stop::Cancelled),
    };
    let mut road = doc.settings().road.clone().unwrap_or_default();
    road.volumes = list.clone();
    judged(&road, codes::INVALID_VOLUME, "Kübaj hesabı")?;
    if let Some(d) = target.as_ref().and_then(|id| list.iter().find(|v| &v.id == id)) {
        common::compute(doc, d)?;
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked { list, target })
}

fn write(cx: &mut ExecutionContext<'_>, c: Checked) -> CommandResult<VolumesWritten> {
    let doc = &mut *cx.doc;
    if let Err(r) = doc.set_road_volumes(c.list.clone(), STEP) {
        return fail(codes::INVALID_VOLUME, format!("Kübaj hesabı kurallarına uymuyor: {}.", r.0), "").into();
    }
    let volume = c.target.as_ref().and_then(|id| c.list.iter().find(|v| &v.id == id)).map(|d| common::report(doc, d));
    CommandResult::Completed {
        output: VolumesWritten { volume, volumes: common::listed(&c.list), revision: doc.revision().to_string() },
        warnings: vec![],
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &VolumesEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: vec![] },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &VolumesEdit) -> CommandResult<VolumesWritten> {
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

/// Writes the volume: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: VolumesEdit) -> CommandResult<VolumesWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_VOLUMES_EDIT,
        kentos_commands::ROAD_VOLUMES_EDIT_VERSION,
        input,
        |cx, input| match check(cx.doc, &input) {
            Ok(c) => write(cx, c),
            Err(stop) => stop.into(),
        },
    )
}
