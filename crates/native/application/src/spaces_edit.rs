//! `cad.spaces.edit` v1 (docs/adr/0342 §4, plan/cad.md `CAD-ARCH-25`, `27`): spaces named by their persistent ids
//! changed as one undo step “Mahali düzenle”, all or none. Değiştir sets their own fields (Öznitelikler writes through
//! it); Birleştir makes them one, the first's id kept; Böl cuts each along a line, its largest part keeping its id;
//! Duvarlardan bul finds a space's boundary again from the walls. The answer says the ids kept, added and removed.
//! The shared cases are `fixtures/commands/v1/cad.spaces.edit.json`.

use kentos_commands::{
    CommandResult, Entity, SpaceChange, SpaceEntity, SpaceFields, SpacesEdit, SpacesEdited,
    SpacesOperation,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::geom::arrangement::Source;
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::geom::region::{net_area, split_area, union_areas};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::spaces::{self, fail};

pub const STEP: &str = "Mahali düzenle";

#[derive(Default)]
struct Work {
    updates: Vec<(Slot, SpaceEntity)>,
    added: Vec<SpaceEntity>,
    removed: Vec<Slot>,
}

fn changed(s: &SpaceFields, c: &SpaceChange) -> SpaceFields {
    let mut f = s.clone();
    if let Some(v) = &c.sources { f.sources = v.clone().map(|mut ids| { ids.sort(); ids }); }
    if let Some(v) = c.scope { f.scope = v; }
    if let Some(v) = &c.number {
        f.number.clone_from(v);
    }
    if let Some(v) = &c.name {
        f.name.clone_from(v);
    }
    if let Some(v) = &c.usage {
        f.usage.clone_from(v);
    }
    if let Some(v) = c.columns { f.columns = v; }
    if let Some(v) = c.method {
        f.method = v;
    }
    if let Some(v) = &c.level {
        f.arch.level.clone_from(v);
    }
    if let Some(v) = c.phase {
        f.arch.phase = v;
    }
    if let Some(v) = c.height {
        f.height = v;
    }
    if let Some(v) = &c.floor {
        f.finishes.floor.clone_from(v);
    }
    if let Some(v) = &c.wall {
        f.finishes.wall.clone_from(v);
    }
    if let Some(v) = &c.ceiling {
        f.finishes.ceiling.clone_from(v);
    }
    f
}

fn check(doc: &Document, input: &SpacesEdit) -> Result<Work, Stop> {
    checks::uids(&input.uids, "Düzenlenecek mahal verilmedi.")?;
    let found = checks::named(doc, &input.uids)?;
    let mut list: Vec<(usize, Slot, &SpaceEntity)> = Vec::with_capacity(found.len());
    for (at, slot, e, _) in &found {
        match e {
            Entity::Space(s) => list.push((*at, *slot, s)),
            _ => {
                return Err(fail(
                    codes::NOT_A_SPACE,
                    "Nesne mahal değil; Mahali düzenle yalnız mahalleri değiştirir.".into(),
                    &format!("uids[{at}]"),
                ));
            }
        }
    }
    let mut work = Work::default();
    match &input.operation {
        SpacesOperation::Set(c) => {
            if c == &SpaceChange::default() {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Değiştirilecek bir alan verilmedi: numara, ad, kullanım, yöntem, kat, evre, yükseklik ya da kaplamalardan birini verin.".into(),
                    "operation",
                ));
            }
            if let Some(Some(h)) = c.height {
                checks::finite(h, "Yükseklik", "Sonlu bir sayı verin.", "operation.height")?;
            }
            for (at, slot, s) in &list {
                let mut f = changed(&s.space, c);
                spaces::scope_valid(&f,&format!("uids[{at}]"))?;
                if (f.scope != s.space.scope || f.sources != s.space.sources) && !kentos_domain::space_follow::SpaceSearch::new(doc).seed_is_room(&f) {
                    return Err(fail(codes::INVALID_SPACE,
                        "Mahalın iç noktası yeni kapsamda bir odanın içinde değil; duvarın üzerinde veya dışında kalıyor. İç nokta seç aracıyla geçerli bir iç nokta seçin, sonra kapsamı değiştirin.".into(),
                        if c.sources.is_some() { "operation.sources" } else { "operation.scope" }));
                }
                if (c.method.is_some() || c.level.is_some() || c.columns.is_some() || c.scope.is_some() || c.sources.is_some()) && f.seed.is_some() {
                    f = doc.space_found(&f);
                }
                if c.sources.is_some() && f.broken.is_some() { return Err(spaces::not_found(&f,"operation.sources")); }
                let path = format!("uids[{at}]");
                spaces::valid(&f, &path)?;
                spaces::level_known(doc, &f.arch, "operation.level")?;
                let mut e = (*s).clone();
                e.space = f;
                work.updates.push((*slot, e));
            }
        }
        SpacesOperation::Merge => {
            if list.len() < 2 {
                return Err(fail(
                    codes::TOO_FEW_OBJECTS,
                    "Birleştirmek için en az iki mahal verin.".into(),
                    "uids",
                ));
            }
            let all: Vec<_> = list
                .iter()
                .flat_map(|(_, _, s)| spaces::areas(&s.space.parts))
                .collect();
            let one = union_areas(&all);
            if one.len() != 1 {
                return Err(fail(
                    codes::SPACES_NOT_TOUCHING,
                    format!(
                        "Mahaller birleşince {} ayrı parça oluyor; birleştirilecek mahaller birbirine değmeli.",
                        one.len()
                    ),
                    "uids",
                ));
            }
            let (_, slot, first) = list[0];
            let mut e = first.clone();
            e.space.parts = one.iter().map(kentos_domain::space_follow::part).collect();
            e.space.sources = None;
            e.space.seed = None;
            e.space.scope = kentos_commands::SpaceScope::Project;
            e.space.columns = kentos_commands::SpaceColumns::Ignore;
            e.space.broken = None;
            spaces::valid(&e.space, "uids[0]")?;
            work.updates.push((slot, e));
            work.removed = list[1..].iter().map(|(_, s, _)| *s).collect();
        }
        SpacesOperation::Split { line } => {
            if line.len() < 2 {
                return Err(fail(
                    codes::SPLIT_MISSED,
                    "Bölme çizgisi en az iki noktadır.".into(),
                    "operation.line",
                ));
            }
            for (k, p) in line.iter().enumerate() {
                checks::point(
                    *p,
                    &format!("{}. noktanın", k + 1),
                    &format!("operation.line[{k}]"),
                )?;
            }
            let pts: Vec<CoreVec2> = line.iter().map(|p| CoreVec2::new(p.x, p.y)).collect();
            let cut = Source {
                edges: pts
                    .windows(2)
                    .map(|w| Edge::Seg { a: w[0], b: w[1] })
                    .collect(),
                points: Some(pts.clone()),
                cut: Some(true),
            };
            for (at, slot, s) in &list {
                let mut pieces: Vec<_> = spaces::areas(&s.space.parts)
                    .iter()
                    .flat_map(|a| split_area(a, &cut))
                    .collect();
                if pieces.len() < 2 {
                    return Err(fail(
                        codes::SPLIT_MISSED,
                        "Çizgi mahali bölmüyor: mahalin içinden geçip sınırını iki yerde kesmeli."
                            .into(),
                        &format!("uids[{at}]"),
                    ));
                }
                // The largest keeps the id; the others are new, in their order by size.
                pieces.sort_by(|a, b| net_area(b).total_cmp(&net_area(a)));
                let mut kept = (*s).clone();
                kept.space.parts = vec![kentos_domain::space_follow::part(&pieces[0])];
                kept.space.sources = None;
                kept.space.seed = None;
                kept.space.scope = kentos_commands::SpaceScope::Project;
                kept.space.columns = kentos_commands::SpaceColumns::Ignore;
                kept.space.broken = None;
                work.updates.push((*slot, kept.clone()));
                for p in &pieces[1..] {
                    let mut e = kept.clone();
                    e.base.id = 0;
                    e.base.label = None;
                    e.base.label_pins.clear();
                    e.base.attachments.clear();
                    e.space.parts = vec![kentos_domain::space_follow::part(p)];
                    work.added.push(e);
                }
            }
        }
        SpacesOperation::Rebound { seed } => {
            if let Some(p) = seed {
                checks::point(*p, "Mahalin iç noktasının", "operation.seed")?;
            }
            for (at, slot, s) in &list {
                let mut f = s.space.clone();
                f.seed = seed.or(f.seed).or_else(|| spaces::label_point(&f.parts));
                let f = doc.space_found(&f);
                if f.broken.is_some() {
                    return Err(spaces::not_found(&f, &format!("uids[{at}]")));
                }
                let mut e = (*s).clone();
                e.space = f;
                work.updates.push((*slot, e));
            }
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    for (at, _, e, _) in &found {
        let layer = &e.base().layer_id;
        if doc.layers().is_locked(layer) {
            let name = doc
                .layers()
                .get(layer)
                .map_or(layer.as_str(), |n| n.name.as_str());
            return Err(fail(
                codes::LAYER_LOCKED,
                format!(
                    "“{name}” katmanı kilitli; üzerindeki mahal düzenlenemez. Kilidi Katmanlar panelinden açın."
                ),
                &format!("uids[{at}]"),
            ));
        }
    }
    Ok(work)
}

fn kept_uids(doc: &Document, work: &Work) -> Vec<String> {
    work.updates
        .iter()
        .filter_map(|(s, _)| doc.uid(*s).map(|u| u.to_string()))
        .collect()
}

fn removed_uids(doc: &Document, work: &Work) -> Vec<String> {
    work.removed
        .iter()
        .filter_map(|s| doc.uid(*s).map(|u| u.to_string()))
        .collect()
}

pub fn validate(cx: &ExecutionContext<'_>, input: &SpacesEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &SpacesEdit) -> CommandResult<SpacesEdited> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: what the space tools draw as their preview.
pub fn plan_in(doc: &Document, input: &SpacesEdit) -> CommandResult<SpacesEdited> {
    match check(doc, input) {
        Ok(work) => CommandResult::Completed {
            output: SpacesEdited {
                spaces: work
                    .updates
                    .iter()
                    .map(|(_, e)| Entity::Space(e.clone()))
                    .collect(),
                added: work.added.iter().cloned().map(Entity::Space).collect(),
                kept: kept_uids(doc, &work),
                removed: removed_uids(doc, &work),
                revision: doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: SpacesEdit) -> CommandResult<SpacesEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_SPACES_EDIT,
        kentos_commands::CAD_SPACES_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: SpacesEdit) -> CommandResult<SpacesEdited> {
    let work = match check(cx.doc, &input) {
        Ok(work) => work,
        Err(stop) => return stop.into(),
    };
    let kept = kept_uids(cx.doc, &work);
    let removed = removed_uids(cx.doc, &work);
    let doc = &mut *cx.doc;
    let result = doc.transact(STEP, |d| {
        d.update_many(
            work.updates
                .iter()
                .map(|(s, e)| (*s, Entity::Space(e.clone())))
                .collect(),
            STEP,
        );
        let slots = d.add_many(
            work.added.iter().cloned().map(Entity::Space).collect(),
            STEP,
        )?;
        d.remove(&work.removed);
        Ok::<_, kentos_domain::SlotsExhausted>(slots)
    });
    let slots = match result {
        Ok(slots) => slots,
        Err(_) => {
            return CommandResult::Failed {
                error: error(
                    codes::SLOTS_EXHAUSTED,
                    "Çizimde yeni nesneye yer kalmadı; çizimi kaydedip yeniden açın.".into(),
                    None,
                ),
            };
        }
    };
    let spaces = work
        .updates
        .iter()
        .filter_map(|(s, _)| doc.get(*s).cloned())
        .collect();
    let added = slots.iter().filter_map(|s| doc.get(*s).cloned()).collect();
    CommandResult::Completed {
        output: SpacesEdited {
            spaces,
            added,
            kept,
            removed,
            revision: doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}
