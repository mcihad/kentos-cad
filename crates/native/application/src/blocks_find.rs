//! Candidate recognition over the shared analytic matcher. Read only, cancellable and revision guarded.
use crate::{
    ExecutionContext,
    checks::{self, Stop, error},
};
use kentos_commands::{
    BlockMatch, BlocksFind, BlocksFound, Bounds, CommandResult, CommandWarning, Entity,
    RECOGNITION_SCOPE_MAX, RECOGNITION_SOURCE_MAX, RECOGNITION_TOLERANCE, Vec2, codes,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::entity::{Shape, entity_bounds};
use kentos_geometry_core::ops::repeats::{self, Options};
use std::collections::{BTreeMap, HashMap, HashSet};

fn refuse(code: &str, message: &str, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

fn named(doc: &Document, list: &[String], path: &str) -> Result<Vec<Uuid>, Stop> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for (i, text) in list.iter().enumerate() {
        let uid = Uuid::parse_str(text)
            .ok()
            .filter(|u| u.to_string() == *text)
            .ok_or_else(|| {
                refuse(
                    codes::INVALID_UID,
                    "Nesne kimliği küçük harfli, tireli UUID olmalı.",
                    &format!("{path}[{i}]"),
                )
            })?;
        if doc.slot_of(uid).and_then(|s| doc.get(s)).is_none() {
            return Err(refuse(
                codes::ENTITY_NOT_FOUND,
                "Aranacak nesne çizimde yok; kaynağı ve kapsamı yeniden seçin.",
                &format!("{path}[{i}]"),
            ));
        }
        if seen.insert(uid) {
            out.push(uid);
        }
    }
    Ok(out)
}

struct Checked {
    source: Vec<Uuid>,
    search: HashSet<Uuid>,
    options: Options,
}

fn check(doc: &Document, input: &BlocksFind) -> Result<Checked, Stop> {
    if input.source.is_empty() {
        return Err(refuse(
            codes::NO_ENTITIES,
            "Tekrarı aranacak tek bir bileşenin çizgi, çember veya yaylarını seçin.",
            "source",
        ));
    }
    if input.source.len() > RECOGNITION_SOURCE_MAX {
        return Err(refuse(
            codes::RECOGNITION_SOURCE_LIMIT,
            "Kaynak en çok 128 nesne olabilir; tek bir bileşen seçin.",
            "source",
        ));
    }
    let tolerance = input.tolerance.unwrap_or(RECOGNITION_TOLERANCE);
    if !(tolerance.is_finite() && tolerance > 0.0 && tolerance <= 0.001) {
        return Err(refuse(
            codes::INVALID_TOLERANCE,
            "Tolerans sıfırdan büyük ve en çok 0,001 metre olmalı.",
            "tolerance",
        ));
    }
    let limit = input.limit.unwrap_or(1000);
    if !(1..=1000).contains(&limit) {
        return Err(refuse(
            codes::INVALID_LIMIT,
            "Eşleşme sınırı 1 ile 1000 arasında olmalı.",
            "limit",
        ));
    }
    if input
        .uids
        .as_ref()
        .is_some_and(|u| u.len() > RECOGNITION_SCOPE_MAX)
        || (input.uids.is_none()
            && doc.entities().take(RECOGNITION_SCOPE_MAX + 1).count() > RECOGNITION_SCOPE_MAX)
    {
        return Err(refuse(
            codes::RECOGNITION_SCOPE_LIMIT,
            "Arama kapsamı en çok 50 000 nesne olabilir; kapsamı daraltın.",
            "uids",
        ));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let source = named(doc, &input.source, "source")?;
    for (i, uid) in source.iter().enumerate() {
        let e = doc.slot_of(*uid).and_then(|s| doc.get(s)).ok_or_else(|| {
            refuse(
                codes::ENTITY_NOT_FOUND,
                "Kaynak çizimde yok; kaynağı yeniden seçin.",
                &format!("source[{i}]"),
            )
        })?;
        if !matches!(e, Entity::Line(_) | Entity::Circle(_) | Entity::Arc(_))
            || !repeats::supported(&crate::geometry::shape(e))
        {
            return Err(refuse(
                codes::RECOGNITION_UNSUPPORTED,
                "Kaynak yalnız sıfır uzunluklu olmayan çizgi, çember ve yay içerebilir. Diğer türleri seçimden çıkarın; hiçbir öğe sessizce atlanmaz.",
                &format!("source[{i}]"),
            ));
        }
    }
    let search = match &input.uids {
        Some(list) => named(doc, list, "uids")?.into_iter().collect(),
        None => doc.entities_with_uids().map(|(uid, _)| uid).collect(),
    };
    Ok(Checked {
        source,
        search,
        options: Options {
            rotations: input.rotations.unwrap_or(true),
            mirrors: input.mirrors.unwrap_or(false),
            scales: input.scales.unwrap_or(false),
            tolerance,
            limit: limit as usize,
            ..Default::default()
        },
    })
}

fn find_in(
    doc: &Document,
    input: &BlocksFind,
    mut stopped: impl FnMut() -> bool,
) -> Result<BlocksFound, Stop> {
    if stopped() {
        return Err(Stop::Cancelled);
    }
    let checked = check(doc, input)?;
    let originals: HashSet<_> = checked.source.iter().copied().collect();
    let mut records = Vec::new();
    let mut scene: Vec<Shape> = Vec::new();
    let mut properties = Vec::new();
    for (uid, e) in doc.entities_with_uids() {
        if stopped() {
            return Err(Stop::Cancelled);
        }
        if !checked.search.contains(&uid) && !originals.contains(&uid) {
            continue;
        }
        if !matches!(e, Entity::Line(_) | Entity::Circle(_) | Entity::Arc(_)) {
            continue;
        }
        records.push((uid, e));
        scene.push(crate::geometry::shape(e));
        let mut p = e.base().clone();
        p.id = 0;
        properties.push(p);
    }
    let by_uid: HashMap<_, _> = records
        .iter()
        .enumerate()
        .map(|(i, (u, _))| (*u, i))
        .collect();
    let source: Vec<_> = checked.source.iter().map(|u| by_uid[u]).collect();
    let geometry_only = input.geometry_only.unwrap_or(false);
    let found = repeats::find(
        &scene,
        &source,
        checked.options,
        |i, j| {
            checked.search.contains(&records[j].0)
                && (geometry_only || properties[i] == properties[j])
        },
        &mut stopped,
    )
    .map_err(|e| match e {
        repeats::Error::Cancelled => Stop::Cancelled,
        _ => refuse(
            codes::RECOGNITION_UNSUPPORTED,
            "Kaynağın analitik yönü veya boyutu bulunamadı; başka bir bileşen seçin.",
            "source",
        ),
    })?;
    let mut used = HashMap::new();
    for group in &found.matches {
        for &i in &group.members {
            *used.entry(i).or_insert(0usize) += 1;
        }
    }
    let mut links = kentos_domain::copy_links::Copies::new();
    let mut matches = Vec::new();
    for group in found.matches {
        if stopped() {
            return Err(Stop::Cancelled);
        }
        let mut bounds = Bounds {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        };
        for &i in &group.members {
            let b = entity_bounds(&scene[i]);
            bounds.min_x = bounds.min_x.min(b.min_x);
            bounds.min_y = bounds.min_y.min(b.min_y);
            bounds.max_x = bounds.max_x.max(b.max_x);
            bounds.max_y = bounds.max_y.max(b.max_y);
            links.insert(*records[i].0.as_bytes(), *records[i].0.as_bytes());
        }
        matches.push(BlockMatch {
            overlaps: group.members.iter().any(|i| used[i] > 1),
            members: group
                .members
                .iter()
                .map(|i| records[*i].0.to_string())
                .collect(),
            origin: Vec2 {
                x: group.origin.x,
                y: group.origin.y,
            },
            at: Vec2 {
                x: group.at.x,
                y: group.at.y,
            },
            rotation: group.rotation,
            scale: group.scale,
            mirror: group.mirror,
            bounds,
        });
    }
    let mut related = BTreeMap::new();
    for e in doc.entities() {
        if stopped() {
            return Err(Stop::Cancelled);
        }
        if kentos_domain::copy_links::links_to(e, &links) {
            *related.entry(e.kind().to_owned()).or_insert(0) += 1;
        }
    }
    Ok(BlocksFound {
        source: checked.source.iter().map(Uuid::to_string).collect(),
        matches,
        truncated: found.truncated,
        comparisons: found.comparisons,
        related,
        revision: doc.revision().to_string(),
    })
}

/// The same query for the desktop's cancellable worker and command hosts. Never writes to the document.
pub fn inspect(
    doc: &Document,
    input: &BlocksFind,
    stopped: impl FnMut() -> bool,
) -> CommandResult<BlocksFound> {
    match find_in(doc, input, stopped) {
        Ok(output) => {
            let mut warnings = Vec::new();
            if output.truncated {
                warnings.push(CommandWarning { code:codes::RECOGNITION_TRUNCATED.into(),message:"Arama sınıra ulaştı; bulunanlar kısmi sonuçtur. Kapsamı daraltıp yeniden arayın.".into(),path:Some("uids".into()) });
            }
            if input.geometry_only == Some(true) {
                warnings.push(CommandWarning { code:codes::RECOGNITION_GEOMETRY_ONLY.into(),message:"Yalnız geometri karşılaştırıldı; katman, görünüş ve öznitelik farkları eşleşmeye engel sayılmadı.".into(),path:Some("geometryOnly".into()) });
            }
            CommandResult::Completed { output, warnings }
        }
        Err(stop) => stop.into(),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &BlocksFind) -> CommandResult<()> {
    if cx.stopped() {
        return CommandResult::Cancelled;
    }
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}
pub fn plan(cx: &ExecutionContext<'_>, input: &BlocksFind) -> CommandResult<BlocksFound> {
    inspect(cx.doc, input, || cx.stopped())
}
pub fn execute(cx: &mut ExecutionContext<'_>, input: BlocksFind) -> CommandResult<BlocksFound> {
    plan(cx, &input)
}
