//! CAD inspection is a cancellable query. It never heals sources or detaches associations.
use crate::{
    ExecutionContext,
    checks::{self, Stop},
    codes,
    geometry::shape,
};
use kentos_commands::{
    CommandResult, CommandWarning, DrawingInspect, DrawingInspected, Entity, HatchInspection,
    HatchLinkState, InspectionFinding, InspectionGroup, InspectionSeverity, Vec2,
};
use kentos_domain::{Document, Uuid};
use std::collections::BTreeSet;

mod boundaries;
pub mod annotations;
const WORK_MAX: u64 = 2_000_000;
const EDGE_MAX: usize = 20_000;
const EPS: f64 = 1e-6;

struct Budget {
    work: u64,
    cut: bool,
}
impl Budget {
    fn tick(&mut self, stopped: &mut impl FnMut() -> bool) -> Result<bool, Stop> {
        if stopped() {
            return Err(Stop::Cancelled);
        }
        if self.work >= WORK_MAX {
            self.cut = true;
            return Ok(false);
        }
        self.work += 1;
        Ok(true)
    }
}
fn failed(code: &str, message: &str, path: &str) -> Stop {
    Stop::failed(checks::error(code, message.into(), Some(path.into())))
}
fn scope(doc: &Document, input: &DrawingInspect) -> Result<Vec<Uuid>, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let gap = input.max_gap.unwrap_or(0.01);
    if !gap.is_finite() || !(0.0..=1000.0).contains(&gap) {
        return Err(failed(
            codes::INVALID_GAP,
            "Boşluk eşiği 0 ile 1000 metre arasında sonlu bir sayı olmalı.",
            "maxGap",
        ));
    }
    if !(1..=1000).contains(&input.limit.unwrap_or(200)) {
        return Err(failed(
            codes::INVALID_LIMIT,
            "Bulgu sınırı 1 ile 1000 arasında olmalı.",
            "limit",
        ));
    }
    let mut ids = BTreeSet::new();
    if let Some(list) = &input.uids {
        if list.len() > kentos_commands::INSPECTION_SCOPE_MAX {
            return Err(scope_limit());
        }
        for (i, text) in list.iter().enumerate() {
            if !checks::is_uid_text(text) {
                return Err(failed(
                    codes::INVALID_UID,
                    "Nesne kimliği küçük harfli, tireli UUID olmalı.",
                    &format!("uids[{i}]"),
                ));
            }
            let uid = Uuid::parse_str(text)
                .map_err(|_| failed(codes::INVALID_UID, "Geçersiz nesne kimliği.", "uids"))?;
            if doc.slot_of(uid).is_none() {
                return Err(failed(
                    codes::ENTITY_NOT_FOUND,
                    "Kapsam nesnesi çizimde yok.",
                    &format!("uids[{i}]"),
                ));
            }
            ids.insert(uid);
        }
    } else {
        for (uid, _) in doc.entities_with_uids() {
            if ids.len() >= kentos_commands::INSPECTION_SCOPE_MAX {
                return Err(scope_limit());
            }
            ids.insert(uid);
        }
    }
    Ok(ids.into_iter().collect())
}
fn scope_limit() -> Stop {
    failed(
        "inspection_scope_limit",
        "Denetim kapsamı en çok 50 000 nesne olabilir; seçimi veya katmanı daraltın.",
        "uids",
    )
}
fn push(out: &mut DrawingInspected, finding: InspectionFinding, limit: usize) {
    if out.findings.len() < limit {
        out.findings.push(finding);
    } else {
        out.truncated = true;
    }
}
fn check_group(input: &DrawingInspect, group: InspectionGroup) -> bool {
    input.checks.is_empty() || input.checks.contains(&group)
}
fn known_open(e: &Entity) -> bool {
    match e {
        Entity::Line(_) | Entity::Point(_) | Entity::Arc(_) => true,
        Entity::Polyline(p) => {
            !p.closed
                && p.pts
                    .first()
                    .zip(p.pts.last())
                    .is_none_or(|(a, b)| (a.x - b.x).hypot(a.y - b.y) > EPS)
        }
        Entity::Ellipse(e) => {
            kentos_geometry_core::geom::arc::sweep(e.t0, e.t1) < std::f64::consts::TAU - 1e-12
        }
        _ => false,
    }
}
fn hatch(
    doc: &Document,
    uid: Uuid,
    h: &kentos_commands::HatchEntity,
    out: &mut DrawingInspected,
    limit: usize,
    budget: &mut Budget,
    stopped: &mut impl FnMut() -> bool,
) -> Result<(), Stop> {
    let mut sources = BTreeSet::new();
    for source in h
        .assoc
        .iter()
        .flat_map(|a| {
            a.outer
                .iter()
                .chain(a.bounds.iter())
                .chain(a.islands.iter())
                .chain(a.cutouts.iter())
        })
    {
        if !budget.tick(stopped)? {
            return Ok(());
        }
        sources.insert(Uuid::from_bytes(source.0));
    }
    let missing: Vec<_> = sources
        .iter()
        .filter(|u| doc.slot_of(**u).is_none())
        .map(ToString::to_string)
        .collect();
    let state = if h.assoc.is_none() {
        HatchLinkState::Unassociated
    } else if missing.is_empty()
        && h.assoc.as_ref().is_some_and(|a| a.outer.is_some() || !a.bounds.is_empty())
        && !h.assoc.as_ref().is_some_and(|a| {
            a.outer.is_some_and(|id| {
                doc.slot_of(Uuid::from_bytes(id.0))
                    .and_then(|s| doc.get(s))
                    .is_some_and(known_open)
            })
        })
    {
        HatchLinkState::Associated
    } else {
        HatchLinkState::Broken
    };
    let at = h.ring.first().copied().unwrap_or(Vec2 { x: 0., y: 0. });
    if state == HatchLinkState::Broken {
        push(out, InspectionFinding {group:InspectionGroup::Hatches,severity:InspectionSeverity::Warning,code:"hatch_missing_source".into(),uids:vec![uid.to_string()],points:vec![at],message:if missing.is_empty() {"Kaynak sınır eksik veya açık; taramanın kayıtlı geometrisi korunuyor.".into()} else {format!("{} sınır/ada/yazı kaynağı çizimde yok; taramanın kayıtlı geometrisi korunuyor.",missing.len())},repair:"Taramayı düzenle → Sınırı yeniden seç veya Sınırı yeniden oluştur; uygulanmadan önce sonucu inceleyin.".into(),distance:None},limit);
    }
    if state == HatchLinkState::Unassociated {
        push(out, InspectionFinding {
            group: InspectionGroup::Hatches,
            severity: InspectionSeverity::Info,
            code: "hatch_unassociated".into(),
            uids: vec![uid.to_string()],
            points: vec![at],
            message: "Taramanın sınır bağı yok; mevcut geometri sabit kalır.".into(),
            repair: "İstenirse Taramayı düzenle ile sınırı yeniden seçin veya oluşturun.".into(),
            distance: None,
        }, limit);
    }
    out.hatches.push(HatchInspection {
        uid: uid.to_string(),
        state,
        sources: sources.iter().map(ToString::to_string).collect(),
        missing,
        area: kentos_geometry_core::entity::entity_area(&shape(&Entity::Hatch(h.clone())))
            .unwrap_or(0.),
        pattern_origin: h.pattern.origin,
    });
    Ok(())
}
fn run(
    doc: &Document,
    input: &DrawingInspect,
    mut stopped: impl FnMut() -> bool,
) -> Result<DrawingInspected, Stop> {
    if stopped() {
        return Err(Stop::Cancelled);
    }
    let ids = scope(doc, input)?;
    let limit = input.limit.unwrap_or(200) as usize;
    let mut out = DrawingInspected {
        findings: Vec::new(),
        hatches: Vec::new(),
        dimensions: Vec::new(),
        unsupported: Vec::new(),
        examined: 0,
        comparisons: 0,
        truncated: false,
        revision: doc.revision().to_string(),
    };
    let mut budget = Budget {
        work: 0,
        cut: false,
    };
    let mut edges = Vec::new();
    for uid in ids {
        if !budget.tick(&mut stopped)? {
            break;
        }
        let Some(e) = doc.slot_of(uid).and_then(|s| doc.get(s)) else {
            continue;
        };
        if !input.include_hidden && !doc.layers().is_visible(&e.base().layer_id) {
            continue;
        }
        out.examined += 1;
        if check_group(input, InspectionGroup::Hatches)
            && let Entity::Hatch(h) = e
        {
            hatch(doc, uid, h, &mut out, limit, &mut budget, &mut stopped)?;
        }
        if check_group(input, InspectionGroup::Annotations) {
            for finding in annotations::findings(doc, uid, e) {
                if !budget.tick(&mut stopped)? { break; }
                push(&mut out, finding, limit);
            }
            if let Entity::Dimension(d) = e {
                let read = crate::dimensions_query::read(&kentos_project::format::Format::of(doc.settings()), uid, d);
                if read.measured.is_finite() { out.dimensions.push(read); }
            }
        }
        if check_group(input, InspectionGroup::Boundaries) {
            boundaries::collect(e, uid, &mut edges, &mut out);
        }
    }
    if check_group(input, InspectionGroup::Boundaries) {
        boundaries::inspect(
            &edges,
            input.max_gap.unwrap_or(0.01),
            limit,
            &mut budget,
            &mut stopped,
            &mut out,
        )?;
    }
    out.comparisons = budget.work;
    out.truncated |= budget.cut;
    Ok(out)
}
pub fn inspect(
    doc: &Document,
    input: &DrawingInspect,
    stopped: impl FnMut() -> bool,
) -> CommandResult<DrawingInspected> {
    match run(doc, input, stopped) {
        Ok(output) => {
            let warnings = if output.truncated {
                vec![CommandWarning {code:"inspection_truncated".into(),message:"Denetim sınırına ulaşıldı; sonuç kısmi. Kapsamı daraltıp yeniden denetleyin.".into(),path:Some("uids".into())}]
            } else {
                Vec::new()
            };
            CommandResult::Completed { output, warnings }
        }
        Err(e) => e.into(),
    }
}
pub fn validate(cx: &ExecutionContext<'_>, input: &DrawingInspect) -> CommandResult<()> {
    match scope(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(e) => e.into(),
    }
}
pub fn plan(cx: &ExecutionContext<'_>, input: &DrawingInspect) -> CommandResult<DrawingInspected> {
    inspect(cx.doc, input, || cx.stopped())
}
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: DrawingInspect,
) -> CommandResult<DrawingInspected> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_DRAWING_INSPECT,
        1,
        input,
        |cx, input| plan(cx, &input),
    )
}
