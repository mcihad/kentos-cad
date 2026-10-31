//! `cad.options.query` v1 (docs/adr/0788 §2): the project's design option sets with each option's objects counted by
//! kind, its spaces' area and its walls' length; the set and option an object is in; and two options of a set compared,
//! their objects paired by place as Veri karşılaştır pairs them (`ops::compare`, docs/adr/0179). Read-only. The shared
//! cases are `fixtures/commands/v1/cad.options.query.json`.

use std::collections::BTreeMap;

use kentos_commands::{
    CommandResult, CommandWarning, DesignOption, Entity, OptionChange, OptionKindCount, OptionKindDifference,
    OptionRow, OptionSet, OptionSetRow, OptionsCompared, OptionsQueried, OptionsQuery, option_set,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::ops::compare::{Attrs, Matching, Member, Settings, Status, compare};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::openings::fail;

fn present(doc: &Document, uid: &str) -> Option<Slot> {
    Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u))
}

/// A path's length, its bulged edges as arcs.
fn path_length(pts: &[kentos_commands::Vec2], bulges: Option<&[f64]>, closed: bool) -> f64 {
    let n = pts.len();
    let edges = if closed { n } else { n.saturating_sub(1) };
    (0..edges)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            let chord = (b.x - a.x).hypot(b.y - a.y);
            let bulge = bulges.and_then(|bs| bs.get(i)).copied().unwrap_or(0.0);
            if bulge == 0.0 {
                chord
            } else {
                let theta = 4.0 * bulge.atan();
                chord * theta.abs() / (2.0 * (theta / 2.0).sin().abs())
            }
        })
        .sum()
}

/// The objects of an option in the drawing, in its order.
fn objects<'a>(doc: &'a Document, o: &DesignOption) -> Vec<(String, &'a Entity)> {
    o.members
        .iter()
        .filter_map(|m| present(doc, m).and_then(|s| doc.get(s)).map(|e| (m.clone(), e)))
        .collect()
}

fn row(doc: &Document, s: &OptionSet, o: &DesignOption) -> OptionRow {
    let objs = objects(doc, o);
    let mut kinds: BTreeMap<&'static str, u32> = BTreeMap::new();
    let (mut area, mut length) = (0.0, 0.0);
    for (_, e) in &objs {
        *kinds.entry(e.kind()).or_default() += 1;
        match e {
            Entity::Space(x) => area += crate::spaces::area_of(&x.space.parts),
            Entity::Wall(w) => {
                length += path_length(&w.wall.pts, w.wall.bulges.as_deref(), w.wall.closed);
            }
            _ => {}
        }
    }
    OptionRow {
        id: o.id.clone(),
        name: o.name.clone(),
        primary: o.id == s.primary,
        objects: u32::try_from(objs.len()).unwrap_or(u32::MAX),
        missing: u32::try_from(o.members.len() - objs.len()).unwrap_or(u32::MAX),
        kinds: kinds
            .into_iter()
            .map(|(kind, count)| OptionKindCount {
                kind: kind.to_owned(),
                count,
            })
            .collect(),
        space_area: area,
        wall_length: length,
    }
}

fn member(e: &Entity) -> Member {
    let base = e.base();
    let mut attrs: Vec<(String, String)> = base
        .attrs
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    attrs.push(("$katman".to_owned(), base.layer_id.clone()));
    Member {
        shape: crate::geometry::shape(e),
        attrs: Attrs(attrs),
    }
}

fn compared(
    doc: &Document,
    s: &OptionSet,
    a: &DesignOption,
    b: &DesignOption,
    search: f64,
    tolerance: f64,
) -> OptionsCompared {
    let (oa, ob) = (objects(doc, a), objects(doc, b));
    let ma: Vec<Member> = oa.iter().map(|(_, e)| member(e)).collect();
    let mb: Vec<Member> = ob.iter().map(|(_, e)| member(e)).collect();
    let settings = Settings {
        matching: Matching::Location,
        key: None,
        search,
        tolerance,
        ignore: None,
    };
    let mut out = OptionsCompared {
        set: s.id.clone(),
        a: a.id.clone(),
        b: b.id.clone(),
        same: 0,
        added: Vec::new(),
        removed: Vec::new(),
        changed: Vec::new(),
        space_area: 0.0,
        wall_length: 0.0,
        kinds: Vec::new(),
    };
    for r in compare(&ma, &mb, &settings) {
        match (r.status, r.old, r.new) {
            (Status::Same, ..) => out.same += 1,
            (Status::Added, _, Some(n)) => out.added.push(ob[n].0.clone()),
            (Status::Removed, Some(o), _) => out.removed.push(oa[o].0.clone()),
            (status, Some(o), Some(n)) => out.changed.push(OptionChange {
                a: oa[o].0.clone(),
                b: ob[n].0.clone(),
                what: match status {
                    Status::Geometry => "geometry",
                    Status::Attributes => "attributes",
                    _ => "both",
                }
                .into(),
            }),
            _ => {}
        }
    }
    let (ra, rb) = (row(doc, s, a), row(doc, s, b));
    out.space_area = rb.space_area - ra.space_area;
    out.wall_length = rb.wall_length - ra.wall_length;
    let mut kinds: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    for k in &ra.kinds {
        kinds.entry(k.kind.clone()).or_default().0 = k.count;
    }
    for k in &rb.kinds {
        kinds.entry(k.kind.clone()).or_default().1 = k.count;
    }
    out.kinds = kinds
        .into_iter()
        .filter(|(_, (x, y))| x != y)
        .map(|(kind, (a, b))| OptionKindDifference { kind, a, b })
        .collect();
    out
}

/// The drawing's metres per unit (a length in metres made the drawing's).
fn metres_per_unit(doc: &Document) -> f64 {
    kentos_project::format::Format::of(doc.settings()).to_metres(1.0)
}

fn answer(doc: &Document, input: &OptionsQuery) -> Result<(OptionsQueried, Vec<CommandWarning>), Stop> {
    let all = doc.arch_options();
    let mut chosen: Vec<&OptionSet> = match &input.set {
        Some(key) => vec![option_set(all, key).ok_or_else(|| {
            fail(
                codes::OPTION_SET_NOT_FOUND,
                format!("Projede “{key}” adlı ya da kimlikli tasarım seçeneği kümesi yok."),
                "set",
            )
        })?],
        None => all.iter().collect(),
    };
    let (mut set, mut option) = (None, None);
    if let Some(uid) = &input.uid {
        checks::uids(std::slice::from_ref(uid), "")
            .and_then(|()| checks::named(doc, std::slice::from_ref(uid)).map(|_| ()))
            .map_err(|s| {
                let mut e = s.into_error();
                e.path = Some("uid".into());
                Stop::failed(e)
            })?;
        let Some((s, o)) = kentos_commands::option_of(all, uid) else {
            return Err(fail(
                codes::NOT_IN_OPTION,
                "Nesne bir tasarım seçeneğinde değil: ana modelin.".into(),
                "uid",
            ));
        };
        chosen.retain(|x| x.id == s.id);
        set = Some(s.id.clone());
        option = Some(o.id.clone());
    }
    let mut compared_out = None;
    if let Some(c) = &input.compare {
        let s = option_set(all, &c.set).ok_or_else(|| {
            fail(
                codes::OPTION_SET_NOT_FOUND,
                format!("Projede “{}” adlı ya da kimlikli tasarım seçeneği kümesi yok.", c.set),
                "compare.set",
            )
        })?;
        let a = s.option(&c.a).ok_or_else(|| {
            fail(
                codes::OPTION_NOT_FOUND,
                format!("“{}” kümesinde “{}” seçeneği yok.", s.name, c.a),
                "compare.a",
            )
        })?;
        let b = s.option(&c.b).ok_or_else(|| {
            fail(
                codes::OPTION_NOT_FOUND,
                format!("“{}” kümesinde “{}” seçeneği yok.", s.name, c.b),
                "compare.b",
            )
        })?;
        if a.id == b.id {
            return Err(fail(
                codes::INVALID_INPUT,
                "Bir seçenek kendisiyle karşılaştırılmaz; iki ayrı seçenek verin.".into(),
                "compare.b",
            ));
        }
        let unit = metres_per_unit(doc).max(f64::MIN_POSITIVE);
        for (v, path) in [(c.search, "compare.search"), (c.tolerance, "compare.tolerance")] {
            if v.is_some_and(|v| !(v.is_finite() && v >= 0.0)) {
                return Err(fail(
                    codes::INVALID_INPUT,
                    "Arama uzaklığı ve tolerans sonlu, sıfır ya da daha büyük olmalı.".into(),
                    path,
                ));
            }
        }
        let search = c.search.unwrap_or(0.5 / unit);
        let tolerance = c.tolerance.unwrap_or(0.001 / unit);
        compared_out = Some(compared(doc, s, a, b, search, tolerance));
    }
    let sets: Vec<OptionSetRow> = chosen
        .iter()
        .map(|s| OptionSetRow {
            id: s.id.clone(),
            name: s.name.clone(),
            primary: s.primary.clone(),
            options: s.options.iter().map(|o| row(doc, s, o)).collect(),
        })
        .collect();
    let missing: u32 = sets
        .iter()
        .flat_map(|s| s.options.iter())
        .map(|o| o.missing)
        .sum();
    let warnings = if missing > 0 {
        vec![CommandWarning {
            code: codes::OPTION_MEMBER_GONE.into(),
            message: format!(
                "Seçeneklerin {missing} nesnesi çizimde yok (silinmiş); bir sonraki seçenek yazmasında seçeneklerden düşer."
            ),
            path: None,
        }]
    } else {
        Vec::new()
    };
    Ok((
        OptionsQueried {
            sets,
            set,
            option,
            compared: compared_out,
            revision: doc.revision().to_string(),
        },
        warnings,
    ))
}

/// The answer over `doc`.
pub fn query(doc: &Document, input: &OptionsQuery) -> CommandResult<OptionsQueried> {
    match answer(doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &OptionsQuery) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &OptionsQuery) -> CommandResult<OptionsQueried> {
    query(cx.doc, input)
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: OptionsQuery) -> CommandResult<OptionsQueried> {
    query(cx.doc, &input)
}
