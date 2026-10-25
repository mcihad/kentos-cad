//! Bounded free rooms from a joined wall network, with duplicate/conflict review.
//! Bounds choose representative points; they never cut sources or change the
//! persisted follower scope (explicit level, else existing all-wall semantics).
use crate::checks::Stop;
use crate::{ExecutionContext, checks, codes, spaces, spaces_create};
use kentos_commands::*;
use kentos_domain::Document;
use kentos_geometry_core::geom::{region, space, space_index::BoundaryIndex, wall_join::WallIn};

pub fn plan(cx: &ExecutionContext<'_>, input: &SpacesDiscover) -> CommandResult<SpacesDiscovered> {
    plan_in(cx.doc, input)
}
pub fn plan_in(doc: &Document, input: &SpacesDiscover) -> CommandResult<SpacesDiscovered> {
    match check(doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(e) => e.into(),
    }
}
pub fn validate(cx: &ExecutionContext<'_>, input: &SpacesDiscover) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(e) => e.into(),
    }
}
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: SpacesDiscover,
) -> CommandResult<SpacesCreated> {
    crate::call::recorded(
        cx,
        CAD_SPACES_DISCOVER,
        CAD_SPACES_DISCOVER_VERSION,
        input,
        write,
    )
}
fn write(cx: &mut ExecutionContext<'_>, input: SpacesDiscover) -> CommandResult<SpacesCreated> {
    let (plan, warnings) = match check(cx.doc, &input) {
        Ok(p) => p,
        Err(e) => return e.into(),
    };
    if plan.proposals.iter().any(|p| !p.conflicts.is_empty()) {
        return spaces::fail(codes::INVALID_SPACE,"Çizilmiş mahal sınırı önerilen odayla kısmen çakışıyor. Önizlemedeki çakışmaları çözün; hiçbir mahal yazılmadı.".into(),"proposals").into();
    }
    let new: Vec<_> = plan
        .proposals
        .into_iter()
        .filter(|p| p.existing.is_empty())
        .map(|p| {
            let s = p.space;
            NewSpace {
                seed: s.seed,
                method: Some(s.method),
                columns: Some(s.columns),
                arch: Some(s.arch),
                attrs: input.attrs.clone(),
                ..Default::default()
            }
        })
        .collect();
    if new.is_empty() {
        return CommandResult::Completed {
            output: SpacesCreated {
                uids: vec![],
                ids: vec![],
                revision: cx.doc.revision().to_string(),
            },
            warnings,
        };
    }
    spaces_create::write_execute(
        cx,
        SpacesCreate {
            layer_id: input.layer_id,
            spaces: new,
            color: None,
            line_weight: None,
            expected_revision: input.expected_revision,
        },
    )
}

fn check(
    doc: &Document,
    input: &SpacesDiscover,
) -> Result<(SpacesDiscovered, Vec<CommandWarning>), Stop> {
    let arch = ArchInfo {
        level: input.level.clone(),
        ..Default::default()
    };
    spaces::level_known(doc, &arch, "level")?;
    if let Some([a, b]) = input.bounds {
        checks::point(a, "Bölgenin", "bounds[0]")?;
        checks::point(b, "Bölgenin", "bounds[1]")?;
        if a.x == b.x || a.y == b.y {
            return Err(spaces::fail(
                codes::INVALID_SPACE,
                "Bölgenin iki karşı köşesi farklı olmalı.".into(),
                "bounds",
            ));
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let warnings = checks::layer(doc, &input.layer_id)?;
    checks::object_attrs(doc, &input.layer_id, input.attrs.as_ref(), "attrs")?;
    let eligible = |e: &Entity| {
        input.level.as_ref().is_none_or(|id| match e {
            Entity::Wall(w) => w.wall.arch.level.as_ref() == Some(id),
            Entity::Space(s) => s.space.arch.level.as_ref() == Some(id),
            _ => false,
        })
    };
    let mut walls = Vec::new();
    for e in doc.entities().filter(|e| eligible(e)) {
        let Entity::Wall(w) = e else { continue };
        let path = format!("walls[{}]", w.base.id);
        crate::walls::valid(&w.wall, &path)?;
        crate::walls::path_numbers(&w.wall.pts, w.wall.bulges.as_deref(), w.wall.closed, &path)?;
        walls.push(kentos_domain::space_follow::core_wall(e).ok_or_else(|| {
            spaces::fail(
                codes::INVALID_SPACE,
                "Kaynak duvar okunamadı; toplu mahal bulunamadı.".into(),
                &path,
            )
        })?);
    }
    if walls.len() > MAX_SPACES {
        return Err(spaces::fail(
            codes::INVALID_SPACE,
            format!(
                "Toplu mahal bulmada en çok {MAX_SPACES} kaynak duvar işlenir; {} duvar var. Kat seçerek kapsamı daraltın.",
                walls.len()
            ),
            "level",
        ));
    }
    let ins: Vec<_> = walls
        .iter()
        .map(|w| WallIn {
            pts: &w.pts,
            bulges: w.bulges.as_deref(),
            spec: &w.spec,
        })
        .collect();
    let index = BoundaryIndex::new(&ins);
    let method = input.method.unwrap_or_else(|| spaces::project_method(doc));
    let found = index
        .rooms(kentos_domain::space_follow::method(method))
        .map_err(|why| spaces::fail(codes::INVALID_SPACE, why.into(), "level"))?;
    if found.len() > MAX_SPACES {
        return Err(spaces::fail(
            codes::INVALID_SPACE,
            format!("Bir adımda en çok {MAX_SPACES} mahal bulunur."),
            "level",
        ));
    }
    let existing: Vec<_> = doc
        .entities_with_uids()
        .filter(|(_, e)| eligible(e))
        .filter_map(|(uid, e)| match e {
            Entity::Space(s) => Some((uid.to_string(), s)),
            _ => None,
        })
        .collect();
    let mut proposals = Vec::new();
    let mut search = kentos_domain::space_follow::SpaceSearch::new(doc);
    for room in found {
        // A comfortable representative point is used for the UI and region
        // filter, but exact winding verifies it; analytic probe is fallback.
        let seed = space::label_at(std::slice::from_ref(&room.net))
            .filter(|p| region::inside_area(&room.net, *p))
            .unwrap_or(room.seed);
        if let Some([a, b]) = input.bounds
            && !(seed.x >= a.x.min(b.x)
                && seed.x <= a.x.max(b.x)
                && seed.y >= a.y.min(b.y)
                && seed.y <= a.y.max(b.y))
        {
            continue;
        }
        let mut fields = SpaceFields {
            parts: vec![kentos_domain::space_follow::part(&room.boundary)],
            seed: Some(Vec2 {
                x: seed.x,
                y: seed.y,
            }),
            method,
            columns: input.columns.unwrap_or_default(),
            arch: arch.clone(),
            ..Default::default()
        };
        fields.parts = match search.deduct_columns(&fields) {
            Ok(parts) => parts,
            Err(SpaceBreak::NoFreeArea) => continue,
            Err(why) => return Err(spaces::fail(codes::INVALID_SPACE,why.words().into(),"columns")),
        };
        let comparison = if method == SpaceMethod::Net && fields.columns.applies(method) {
            spaces::areas(&fields.parts)
        } else { vec![room.net.clone()] };
        let mut seen = Vec::new();
        let mut conflicts = Vec::new();
        for (uid, old) in &existing {
            if old.space.seed.is_some_and(|p| {
                region::inside_area(&room.net, kentos_geometry_core::vec2::Vec2::new(p.x, p.y))
            }) {
                seen.push(uid.clone());
                continue;
            }
            // Seeded rooms in other free faces may legitimately overlap at
            // their gross wall offsets; compare drawn/stale geometry only.
            if old.space.seed.is_some() && old.space.broken.is_none() {
                continue;
            }
            let old_areas = spaces::areas(&old.space.parts);
            let overlap = region::intersect_area_sets(&[comparison.clone(), old_areas.clone()]);
            if overlap.is_empty() {
                continue;
            }
            let exactly_same = region::subtract_areas(&comparison, &old_areas)
                .is_empty()
                && region::subtract_areas(&old_areas, &comparison).is_empty();
            if exactly_same {
                seen.push(uid.clone());
            } else {
                conflicts.push(uid.clone());
            }
        }
        spaces::valid(&fields, "proposals")?;
        proposals.push(SpaceProposal {
            space: fields,
            existing: seen,
            conflicts,
        });
    }
    Ok((
        SpacesDiscovered {
            proposals,
            revision: doc.revision().to_string(),
        },
        warnings,
    ))
}
