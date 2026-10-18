//! `cad.spaces.finish` v1 (docs/adr/0345 §4; plan/cad.md `CAD-ARCH-39`): a space's floor pattern or ceiling grid, a
//! hatch tied to the space (its `assoc.outer`) so that it follows the space as its walls change, one hatch for each of
//! the space's parts, its face in its `Kaplama` attribute (Döşeme, Tavan). A space's earlier hatch of the same face is
//! replaced. One undo step “Döşeme ve tavan”; the layer, when the call names none and the drawing has none of the
//! name, is opened in the same step. The Döşeme deseni and Tavan ızgarası tools preview the plan's hatches and write
//! through `execute`. The shared cases are `fixtures/commands/v1/cad.spaces.finish.json`.
//!
//! Checks, in order (the first that fails answers): 1. the spaces' ids (`no_entities`, `invalid_uid`,
//! `entity_not_found`, `not_a_space`); 2. the numbers finite (`not_finite`); 3. the finish (`invalid_finish`: a grid on
//! a floor, a pattern or no grid on a ceiling, spacings and scale over 0, too dense a pattern); 4. the pattern's name
//! (`unknown_pattern`); 5. the expected revision; 6. the layer (`layer_not_found`, `not_a_layer`, `layer_locked`), and an
//! earlier hatch on a locked layer (`layer_locked`).

use kentos_contracts::{
    CEILING_LAYER, CommandResult, CommandWarning, Entity, EntityBase, EntityId, FLOOR_LAYER,
    FinishGrid, FinishSurface, GridOrigin, HatchAssoc, HatchEntity, HatchPattern, HatchPatternSpec,
    HatchPatternType, PatternLine, SpaceEntity, SpacesFinish, SpacesFinished, Vec2,
};
use kentos_domain::{Document, NewLayer, Slot, Uuid};
use kentos_geometry_core::labels::geom::pole;
use kentos_geometry_core::tools::hatch::choice_named;
use kentos_geometry_core::vec2::Vec2 as CoreVec2;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::import_apply::layer_named;
use crate::region::{pattern_of_spec, pattern_too_dense};

pub const STEP: &str = "Döşeme ve tavan";
/// The attribute that names a finish hatch's face.
pub const FACE_ATTR: &str = "Kaplama";
/// The floor pattern when the call and the space name none.
pub const FLOOR_PATTERN: &str = "ANSI31";
/// The most a pattern is scaled.
const MOST_SCALE: f64 = 10_000.0;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

fn bad(words: &str, path: &str) -> Stop {
    fail(codes::INVALID_FINISH, words.to_owned(), path)
}

/// Where the hatches go: a layer the drawing has, or one to open of a name.
#[derive(Clone, Debug)]
enum Target {
    Layer(String),
    Open { id: String, name: String },
}

impl Target {
    fn id(&self) -> &str {
        match self {
            Target::Layer(id) | Target::Open { id, .. } => id,
        }
    }
}

struct Checked {
    hatches: Vec<Entity>,
    replaced: Vec<(Slot, String)>,
    target: Target,
    warnings: Vec<CommandWarning>,
}

/// Whether the project or the library has a pattern of the name.
fn pattern_known(doc: &Document, name: &str) -> bool {
    choice_named(name).is_some()
        || doc
            .settings()
            .hatch_patterns
            .iter()
            .any(|d| kentos_contracts::same_hatch_pattern_name(&d.name, name))
}

/// A ceiling grid's pattern: two families of lines, along x `dy` apart and along y `dx` apart, in metres, turned by its
/// angle, through `origin`.
pub fn grid_pattern(g: &FinishGrid, origin: Vec2) -> HatchPattern {
    let family = |angle: f64, gap: f64| PatternLine {
        angle,
        origin: [0.0, 0.0],
        offset: [0.0, gap],
        dashes: Vec::new(),
    };
    HatchPattern {
        kind: HatchPatternType::Pattern,
        angle: kentos_geometry_core::geom::hatch_pattern::turn_of(g.angle),
        spacing: 1.0,
        name: Some("TAVAN".into()),
        scale: Some(1.0),
        lines: Some(vec![family(0.0, g.dy), family(90.0, g.dx)]),
        gradient: None,
        origin: Some(origin),
    }
}

/// The box of a space's rings.
fn box_of(s: &SpaceEntity) -> Option<(Vec2, Vec2)> {
    let mut b: Option<(Vec2, Vec2)> = None;
    for p in s.space.parts.iter().flat_map(|p| &p.outer.pts) {
        b = Some(match b {
            None => (*p, *p),
            Some((lo, hi)) => (
                Vec2 {
                    x: kentos_geometry_core::jsmath::js_min(lo.x, p.x),
                    y: kentos_geometry_core::jsmath::js_min(lo.y, p.y),
                },
                Vec2 {
                    x: kentos_geometry_core::jsmath::js_max(hi.x, p.x),
                    y: kentos_geometry_core::jsmath::js_max(hi.y, p.y),
                },
            ),
        });
    }
    b
}

/// A point inside each of a space's parts, far from its edges.
fn seeds(s: &SpaceEntity) -> Vec<Vec2> {
    let core = |v: &Vec2| CoreVec2::new(v.x, v.y);
    s.space
        .parts
        .iter()
        .filter_map(|p| {
            let rings: Vec<Vec<CoreVec2>> = std::iter::once(&p.outer)
                .chain(&p.holes)
                .map(|r| r.pts.iter().map(core).collect())
                .collect();
            pole(&rings, 1e-3).map(|(c, _)| Vec2 { x: c.x, y: c.y })
        })
        .collect()
}

/// The hatches a space's finish is, cut from the space by the document's tie rule.
fn hatches_of(
    doc: &Document,
    space: &SpaceEntity,
    uid: Uuid,
    pattern: &HatchPattern,
    base: &EntityBase,
) -> Vec<HatchEntity> {
    seeds(space)
        .into_iter()
        .filter_map(|seed| {
            let first = space.space.parts.first()?;
            let draft = HatchEntity {
                base: base.clone(),
                ring: first.outer.pts.clone(),
                holes: None,
                pattern: pattern.clone(),
                assoc: Some(HatchAssoc {
                    outer: Some(EntityId(uid.into_bytes())),
                    gap: None,
                    bounds: Vec::new(),
                    islands: Vec::new(),
                    cutouts: Vec::new(),
                    seed,
                }),
                boundary: None,
                annotative: None,
            };
            doc.recut_hatch(&draft)
        })
        .collect()
}

/// The finish hatches of `face` tied to the space `uid`.
fn earlier(doc: &Document, uid: Uuid, face: FinishSurface) -> Vec<(Slot, String, String)> {
    let id = EntityId(uid.into_bytes());
    doc.entities()
        .filter_map(|e| match e {
            Entity::Hatch(h)
                if h.assoc.as_ref().and_then(|a| a.outer) == Some(id)
                    && h.base.attrs.get(FACE_ATTR).map(String::as_str) == Some(face.label()) =>
            {
                let slot = Slot(h.base.id);
                Some((
                    slot,
                    doc.uid(slot).map(|u| u.to_string()).unwrap_or_default(),
                    h.base.layer_id.clone(),
                ))
            }
            _ => None,
        })
        .collect()
}

/// An id no node of the tree has, from the layer's name (`a-zemn-dsn`).
fn fresh_id(doc: &Document, name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let mut id = slug.clone();
    let mut k = 2;
    while doc.layers().get(&id).is_some() {
        id = format!("{slug}-{k}");
        k += 1;
    }
    id
}

fn check(doc: &Document, input: &SpacesFinish) -> Result<Checked, Stop> {
    checks::uids(&input.uids, "Kaplanacak mahal verilmedi.")?;
    let found = checks::named(doc, &input.uids)?;
    let mut spaces = Vec::with_capacity(found.len());
    for (at, _, e, uid) in &found {
        let Entity::Space(s) = e else {
            return Err(fail(
                codes::NOT_A_SPACE,
                "Nesne mahal değil; döşeme deseni ve tavan ızgarası mahallere yazılır.".into(),
                &format!("uids[{at}]"),
            ));
        };
        let Ok(u) = Uuid::parse_str(uid) else { continue };
        spaces.push((*at, s, u));
    }
    if let Some(s) = input.scale {
        checks::finite(s, "Desenin ölçeği", "Sonlu bir sayı verin.", "scale")?;
    }
    if let Some(g) = &input.grid {
        for (v, f) in [(g.dx, "dx"), (g.dy, "dy"), (g.angle, "angle")] {
            checks::finite(v, "Izgaranın değeri", "Sonlu bir sayı verin.", &format!("grid.{f}"))?;
        }
    }
    match input.surface {
        FinishSurface::Floor if input.grid.is_some() => {
            return Err(bad("Döşemeye ızgara değil desen yazılır; tavan için surface: ceiling verin.", "grid"));
        }
        FinishSurface::Ceiling if input.pattern.is_some() || input.scale.is_some() => {
            let at = if input.pattern.is_some() { "pattern" } else { "scale" };
            return Err(bad("Tavana desen değil ızgara yazılır; aralıkları grid ile verin.", at));
        }
        FinishSurface::Ceiling if input.grid.is_none() => {
            return Err(bad("Tavan ızgarasının aralıkları verilmedi; grid ile dx ve dy verin.", "grid"));
        }
        _ => {}
    }
    if let Some(g) = &input.grid {
        for (v, f) in [(g.dx, "dx"), (g.dy, "dy")] {
            if !(v > 0.0 && v <= 1_000.0) {
                return Err(bad(
                    "Izgaranın aralığı sıfırdan büyük, en çok 1000 m olmalı.",
                    &format!("grid.{f}"),
                ));
            }
        }
    }
    if let Some(s) = input.scale
        && !(s > 0.0 && s <= MOST_SCALE)
    {
        return Err(bad("Desenin ölçeği sıfırdan büyük, en çok 10 000 olmalı.", "scale"));
    }
    if let Some(name) = &input.pattern
        && !pattern_known(doc, name)
    {
        return Err(fail(
            codes::UNKNOWN_PATTERN,
            format!(
                "“{name}” adında desen yok. Desen kitaplığındaki ya da projenin desenlerindeki bir adı verin (ANSI31, NET, BRICK …)."
            ),
            "pattern",
        ));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let (target, mut warnings) = match &input.layer_id {
        Some(id) => (Target::Layer(id.clone()), checks::layer(doc, id)?),
        None => {
            let name = match input.surface {
                FinishSurface::Floor => FLOOR_LAYER,
                FinishSurface::Ceiling => CEILING_LAYER,
            };
            match layer_named(doc, name) {
                Some(l) => (Target::Layer(l.id.clone()), checks::layer(doc, &l.id)?),
                None => (
                    Target::Open {
                        id: fresh_id(doc, name),
                        name: name.to_owned(),
                    },
                    Vec::new(),
                ),
            }
        }
    };
    let face = input.surface;
    let mut hatches = Vec::new();
    let mut replaced = Vec::new();
    for (at, space, uid) in spaces {
        for (slot, old, layer) in earlier(doc, uid, face) {
            if doc.layers().is_locked(&layer) {
                return Err(fail(
                    codes::LAYER_LOCKED,
                    "Mahalin eski kaplaması kilitli katmanda; yerine yenisi yazılamaz. Kilidi Katmanlar panelinden açın.".into(),
                    &format!("uids[{at}]"),
                ));
            }
            replaced.push((slot, old));
        }
        let pattern = match (&input.grid, face) {
            (Some(g), _) => {
                let o = match (g.origin, box_of(space)) {
                    (GridOrigin::Center, Some((lo, hi))) => Vec2 {
                        x: (lo.x + hi.x) / 2.0,
                        y: (lo.y + hi.y) / 2.0,
                    },
                    (GridOrigin::Corner, Some((lo, _))) => lo,
                    (_, None) => Vec2 { x: 0.0, y: 0.0 },
                };
                grid_pattern(g, o)
            }
            (None, _) => {
                let own = space
                    .space
                    .finishes
                    .floor
                    .as_deref()
                    .filter(|n| pattern_known(doc, n));
                let name = input.pattern.as_deref().or(own).unwrap_or(FLOOR_PATTERN);
                let spec = HatchPatternSpec {
                    name: name.to_owned(),
                    scale: input.scale,
                    angle: None,
                    second_color: None,
                    inverted: None,
                    basis: None,
                    origin: None,
                    annotative: None,
                };
                let mut p = pattern_of_spec(&spec, doc.settings())?;
                p.origin = box_of(space).map(|(lo, _)| lo);
                p
            }
        };
        let mut attrs = std::collections::BTreeMap::new();
        attrs.insert(FACE_ATTR.to_owned(), face.label().to_owned());
        let base = EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: target.id().to_owned(),
            color: None,
            attrs,
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        };
        for h in hatches_of(doc, space, uid, &pattern, &base) {
            let holes = h.holes.clone().unwrap_or_default();
            if pattern_too_dense(&h.ring, &holes, &h.pattern) {
                return Err(bad(
                    "Desen bu mahal için çok sık; ölçeği ya da ızgaranın aralığını büyütün.",
                    if input.grid.is_some() { "grid" } else { "scale" },
                ));
            }
            hatches.push(Entity::Hatch(h));
        }
    }
    if !replaced.is_empty() {
        warnings.push(CommandWarning {
            code: codes::FINISH_REPLACED.into(),
            message: format!(
                "{} mahalin eski {} kaplaması kaldırıldı.",
                replaced.len(),
                face.label().to_lowercase()
            ),
            path: Some("uids".into()),
        });
    }
    Ok(Checked {
        hatches,
        replaced,
        target,
        warnings,
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &SpacesFinish) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &SpacesFinish) -> CommandResult<SpacesFinished> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: what the Döşeme deseni and Tavan ızgarası tools draw as their preview.
pub fn plan_in(doc: &Document, input: &SpacesFinish) -> CommandResult<SpacesFinished> {
    match check(doc, input) {
        Ok(c) => CommandResult::Completed {
            output: SpacesFinished {
                hatches: c.hatches,
                replaced: c.replaced.into_iter().map(|(_, u)| u).collect(),
                revision: doc.revision().to_string(),
            },
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: SpacesFinish) -> CommandResult<SpacesFinished> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_SPACES_FINISH,
        kentos_contracts::CAD_SPACES_FINISH_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: SpacesFinish) -> CommandResult<SpacesFinished> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let group = doc.begin_group(STEP);
    if let Target::Open { id, name } = &c.target {
        let mut new = NewLayer::layer(name.clone());
        new.id = Some(id.clone());
        if let Err(r) = doc.add_layer(new, None, false) {
            doc.cancel_group(group);
            return fail(codes::LAYER_REFUSED, r.to_string(), "layerId").into();
        }
    }
    let gone: Vec<Slot> = c.replaced.iter().map(|(s, _)| *s).collect();
    if !gone.is_empty() {
        doc.remove(&gone);
    }
    let slots = match doc.add_many(c.hatches, STEP) {
        Ok(slots) => slots,
        Err(full) => {
            doc.cancel_group(group);
            return Stop::failed(error(codes::SLOTS_EXHAUSTED, full.to_string(), None)).into();
        }
    };
    doc.end_group(group);
    let hatches = slots
        .iter()
        .filter_map(|s| doc.get(*s).cloned())
        .collect();
    CommandResult::Completed {
        output: SpacesFinished {
            hatches,
            replaced: c.replaced.into_iter().map(|(_, u)| u).collect(),
            revision: doc.revision().to_string(),
        },
        warnings: c.warnings,
    }
}
