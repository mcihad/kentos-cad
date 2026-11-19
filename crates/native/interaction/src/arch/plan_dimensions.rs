//! Planı ölçülendir (docs/adr/8026; ADV-16): one command dimensions the plan of Etkin kat, as Mimarlar Odası İstanbul
//! Büyükkent Şubesi's "Mimari Proje Çizim ve Sunuş Standartları" (2015) asks.
//!
//! - **Outside:** the building's four sides, each with its chains: in an ön proje three (cephe hareketleri, taşıyıcı
//!   akslar, blok: Duvarlar, Akslar, Toplam), in an uygulama projesi a fourth inside them (doluluk ve boşluklar:
//!   Açıklıklar). The walls measured are the exterior ones, found by themselves: those whose body touches the outline
//!   of all the level's walls together; the walls meeting them come in as the façade recipe takes them (docs/adr/0729).
//!   A chain with nothing to measure is not made (the recipe's own rule).
//! - **Inside:** every room crossed by two chains each way ("her mahalde enine ve boyuna 2'şer iç ölçü"), each chain a
//!   cut through the whole building ("iç ölçü çizgileri birden çok mahalden kesintisiz geçer"):
//!   `cad.dimensions.interior`, the faces it crosses. The cuts are placed as few as can be: the rooms taken by their
//!   far side, a cut put as far along each room as it can go and still serve it.
//! - The command's plans are drawn before anything is written; Enter (or a click) writes them, one undo step, on the
//!   dimensions' layer (MM-XXXX-OLCU with the layer key, docs/adr/8008).

use kentos_commands::{
    CommandResult, DimensionEntity, DimensionSetFace, DimensionSetId, DimensionSetOpenings,
    DimensionSetReference, DimensionSetRow, DimensionSetTight, DimensionSetsCreateInput,
    DimensionsInterior, Entity, InteriorColumns, InteriorFaces, NewLook, uuid_v5,
};
use kentos_domain::{Document, Refusal, Slot};
use kentos_native_application::{ExecutionContext, dimension_sets_create, dimensions_interior};

use crate::Vec2;
use crate::arch_layer::Kind;
use crate::format::Format;
use crate::log::Level;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Pointer, Preview, Tag, Tone, Tool};
use crate::tool_settings::{Setting, choices};

pub const ID: &str = "planDimensions";
pub const LABEL: &str = "Planı ölçülendir";

/// How far apart two interior chains are at least, on paper (mm): their values must not touch.
const CUT_SPACING_MM: f64 = 6.0;
/// The rows' gap on paper (mm), Duvar ölçüleri's.
const GAP_MM: f64 = 7.5;
/// How far a cut reaches beyond the building on each end, metres: it starts and ends outside every wall.
const REACH: f64 = 0.5;
const PLAN_SCOPE: &str = "kentos.plan-dimensions.scope";
const PLAN_OWNER: &str = "kentos.plan-dimensions.member";

/// The project's stage: its outside chains (MO).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Stage {
    /// Ön proje: blok, cephe hareketleri, taşıyıcı akslar.
    #[default]
    Preliminary,
    /// Uygulama projesi: and doluluk ve boşluklar.
    Construction,
}

impl Stage {
    fn rows(self) -> Vec<DimensionSetRow> {
        match self {
            Stage::Preliminary => vec![
                DimensionSetRow::Walls,
                DimensionSetRow::Grids,
                DimensionSetRow::Overall,
            ],
            Stage::Construction => vec![
                DimensionSetRow::Openings,
                DimensionSetRow::Walls,
                DimensionSetRow::Grids,
                DimensionSetRow::Overall,
            ],
        }
    }

    fn key(self) -> &'static str {
        match self {
            Stage::Preliminary => "preliminary",
            Stage::Construction => "construction",
        }
    }

    fn words(self) -> &'static str {
        match self {
            Stage::Preliminary => "Ön proje",
            Stage::Construction => "Uygulama",
        }
    }
}

/// The plan's frame: along its longest exterior edge (`d`) and square to it (`n`), the walls' extent in both.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Frame {
    d: Vec2,
    n: Vec2,
    t: (f64, f64),
    o: (f64, f64),
}

impl Frame {
    fn at(&self, t: f64, o: f64) -> Vec2 {
        Vec2::new(self.d.x * t + self.n.x * o, self.d.y * t + self.n.y * o)
    }
    fn along(&self, p: Vec2) -> f64 {
        p.x * self.d.x + p.y * self.d.y
    }
    fn across(&self, p: Vec2) -> f64 {
        p.x * self.n.x + p.y * self.n.y
    }
}

/// What the command would write: the façades' recipe, the interior cuts, and what is drawn of them.
#[derive(Clone, Debug, Default)]
struct Planned {
    facade: Option<DimensionSetsCreateInput>,
    cuts: Vec<DimensionsInterior>,
    existing: bool,
    preview: Preview,
    exterior: usize,
    rooms: usize,
    outside: usize,
    inside: usize,
    /// Where the card stands: the plan's corner.
    at: Option<Vec2>,
    error: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct PlanDimensions {
    stage: Stage,
    outside: bool,
    inside: bool,
    centers: bool,
    separate: bool,
    planned: Planned,
    seen: Option<u64>,
}

impl PlanDimensions {
    pub fn new() -> Self {
        Self {
            outside: true,
            inside: true,
            ..Self::default()
        }
    }
}

fn wire(p: Vec2) -> kentos_commands::Vec2 {
    kentos_commands::Vec2 { x: p.x, y: p.y }
}

/// The level's walls (Etkin kat's; without one, those without a level) and their bodies.
fn level_walls(cx: &Context<'_>) -> Vec<(kentos_domain::Uuid, Vec<Vec2>)> {
    let mut out = Vec::new();
    for e in cx.doc.entities() {
        let Entity::Wall(w) = e else { continue };
        if w.wall.arch.level.as_deref() != cx.level {
            continue;
        }
        let kentos_geometry_core::entity::Shape::Wall { pts, bulges, wall } =
            kentos_native_application::geometry::shape(e)
        else {
            continue;
        };
        let Ok((outer, _)) = kentos_geometry_core::geom::wall::body(&pts, bulges.as_deref(), &wall)
        else {
            continue;
        };
        let Some(uid) = cx.doc.uid(kentos_domain::Slot(e.base().id)) else {
            continue;
        };
        out.push((uid, outer));
    }
    out
}

/// The exterior walls among `walls`: a corner of their body on the outline of all their bodies together
/// (`geom::wall::exterior`).
fn exterior(walls: &[(kentos_domain::Uuid, Vec<Vec2>)]) -> Vec<usize> {
    let bodies: Vec<Vec<Vec2>> = walls.iter().map(|(_, ring)| ring.clone()).collect();
    kentos_geometry_core::geom::wall::exterior(&bodies)
}

/// The frame of `bodies`: along the longest straight edge among them.
fn frame(bodies: &[&Vec<Vec2>]) -> Option<Frame> {
    let mut best: Option<(f64, Vec2)> = None;
    for ring in bodies {
        let n = ring.len();
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            let l = (b.x - a.x).hypot(b.y - a.y);
            if best.is_none_or(|(bl, _)| l > bl + 1e-9) {
                best = Some((l, Vec2::new((b.x - a.x) / l, (b.y - a.y) / l)));
            }
        }
    }
    let (_, mut d) = best?;
    // A direction east (or north) first: the same plan in another drawing order frames alike.
    if d.x < -1e-9 || (d.x.abs() <= 1e-9 && d.y < 0.0) {
        d = Vec2::new(-d.x, -d.y);
    }
    let n = Vec2::new(-d.y, d.x);
    let mut f = Frame {
        d,
        n,
        t: (f64::INFINITY, f64::NEG_INFINITY),
        o: (f64::INFINITY, f64::NEG_INFINITY),
    };
    for p in bodies.iter().flat_map(|r| r.iter()) {
        let (t, o) = (f.along(*p), f.across(*p));
        f.t = (f.t.0.min(t), f.t.1.max(t));
        f.o = (f.o.0.min(o), f.o.1.max(o));
    }
    Some(f)
}

/// Positions of cuts across `ranges` (each a room's extent in the cut's across axis) so that every room is crossed
/// twice where it can be (a room too narrow for two, once in its middle): the rooms by their far side, each crossed
/// at its 70 % and 30 % (a cut already there shared), no two cuts closer than `spacing`. Before, the cuts went as far
/// along each room as they could and crowded at its far wall, on its doors.
pub fn cuts_for(ranges: &[(f64, f64)], spacing: f64) -> Vec<f64> {
    let mut order: Vec<(f64, f64)> = ranges.to_vec();
    order.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.total_cmp(&b.0)));
    let mut cuts: Vec<f64> = Vec::new();
    for (lo, hi) in order {
        let h = hi - lo;
        let margin = (spacing / 2.0).min(h / 10.0);
        let crossing = |cuts: &[f64]| {
            cuts.iter()
                .filter(|c| **c > lo + margin && **c < hi - margin)
                .count()
        };
        if h < 2.0 * spacing {
            // Room for one: its middle, unless a cut crosses it already.
            if crossing(&cuts) == 0 {
                cuts.push((lo + hi) / 2.0);
            }
            continue;
        }
        for share in [0.7, 0.3, 0.5, 0.85, 0.15] {
            if crossing(&cuts) >= 2 {
                break;
            }
            let p = lo + share * h;
            if cuts.iter().all(|c| (c - p).abs() >= spacing - 1e-9) {
                cuts.push(p);
            }
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts
}

/// Model-space dimensions are scoped to the active floor, including the unassigned floor.
fn scope(level: Option<&str>) -> String {
    level.map_or_else(|| "model".to_owned(), |id| format!("level:{id}"))
}

fn facade_ids(scope: &str) -> [DimensionSetId; 4] {
    std::array::from_fn(|side| {
        let key = format!("{PLAN_SCOPE}:{scope}:{side}");
        DimensionSetId(uuid_v5(b"kentos-plan-dims", key.as_bytes()))
    })
}

fn owned_interiors(doc: &Document, scope: &str) -> Vec<(Slot, DimensionEntity)> {
    doc.entities_with_uids()
        .filter_map(|(uid, entity)| {
            let Entity::Dimension(d) = entity else {
                return None;
            };
            (d.base
                .attrs
                .get(PLAN_SCOPE)
                .is_some_and(|value| value == scope)
                && d.base
                    .attrs
                    .get(PLAN_OWNER)
                    .is_some_and(|owner| *owner == uid.to_string()))
            .then(|| (Slot(d.base.id), d.clone()))
        })
        .collect()
}

fn refresh_interior(doc: &mut Document, scope: &str, fresh: Vec<Entity>) -> Result<(), Refusal> {
    let mut old = owned_interiors(doc, scope);
    if old
        .iter()
        .any(|(_, d)| doc.layers().is_locked(&d.base.layer_id))
    {
        return Err(Refusal("Plan ölçülerinin katmanı kilitli.".into()));
    }
    let same_sources = |a: &DimensionEntity, b: &DimensionEntity| {
        let ids = |d: &DimensionEntity| {
            let r = d.refs.as_ref()?;
            Some((r.a.as_ref()?.uid, r.b.as_ref()?.uid))
        };
        a.angle == b.angle && ids(a).is_some_and(|pair| ids(b) == Some(pair))
    };
    let mut changes = Vec::new();
    let mut additions = Vec::new();
    for e in fresh {
        let Entity::Dimension(next) = &e else {
            continue;
        };
        let distance = |dim: &DimensionEntity| {
            (dim.a.x - next.a.x).hypot(dim.a.y - next.a.y)
                + (dim.b.x - next.b.x).hypot(dim.b.y - next.b.y)
        };
        if let Some((index, _)) = old
            .iter()
            .enumerate()
            .filter(|(_, (_, dim))| {
                same_sources(dim, next) || (dim.c == next.c && distance(dim) <= 1e-9)
            })
            .min_by(|(_, (_, a)), (_, (_, b))| distance(a).total_cmp(&distance(b)))
        {
            let (slot, mut dim) = old.swap_remove(index);
            if distance(&dim) <= 1e-9 && dim.c == next.c && (dim.offset - next.offset).abs() <= 1e-9
            {
                continue;
            }
            dim.a = next.a;
            dim.b = next.b;
            dim.c = next.c;
            dim.offset = next.offset;
            dim.refs = next.refs.clone();
            changes.push((slot, Entity::Dimension(dim)));
        } else {
            additions.push(e);
        }
    }
    let mut removed = Vec::new();
    for (slot, mut dim) in old {
        // Leave a manually positioned value as an independent dimension if its
        // source pair disappeared from the refreshed cuts.
        if dim.text_shift.is_some() || dim.text.is_some() {
            dim.base.attrs.remove(PLAN_SCOPE);
            dim.base.attrs.remove(PLAN_OWNER);
            changes.push((slot, Entity::Dimension(dim)));
        } else {
            removed.push(slot);
        }
    }
    let slots = doc
        .add_many(additions, LABEL)
        .map_err(|e| Refusal(e.to_string()))?;
    for slot in slots {
        let mut entity = doc.get(slot).expect("new plan dimension").clone();
        let uid = doc.uid(slot).expect("new dimension identity");
        entity
            .base_mut()
            .attrs
            .insert(PLAN_OWNER.to_owned(), uid.to_string());
        entity
            .base_mut()
            .attrs
            .insert(PLAN_SCOPE.to_owned(), scope.to_owned());
        changes.push((slot, entity));
    }
    doc.update_many(changes, LABEL);
    doc.remove(&removed);
    Ok(())
}

impl PlanDimensions {
    /// The plans of what the command would write, and their preview.
    fn plan(&mut self, cx: &Context<'_>) {
        self.seen = Some(cx.doc.revision());
        let mut planned = Planned::default();
        let walls = level_walls(cx);
        let outside: Vec<usize> = exterior(&walls);
        let bodies: Vec<&Vec<Vec2>> = walls.iter().map(|(_, b)| b).collect();
        let Some(f) = frame(&bodies) else {
            planned.error = Some("Etkin katta ölçülecek duvar yok.".into());
            self.planned = planned;
            return;
        };
        planned.exterior = outside.len();
        let scale = cx.doc.settings().plot_scale.max(1.0);
        // The card above the top chains, clear of the plan: four rows' room and a gap.
        let height = crate::styles::standard_dimension_height(cx.doc.settings());
        planned.at = Some(f.at(f.t.0, f.o.1 + 3.0 * height + 5.0 * GAP_MM / 1000.0 * scale));
        let layer = cx
            .doc
            .layers()
            .get(&Kind::Dimension.layer_id())
            .map_or_else(|| cx.doc.layers().active().to_owned(), |l| l.id.clone());
        let level = cx.level.map(str::to_owned);
        if self.outside && !outside.is_empty() {
            let distance = 3.0 * crate::styles::standard_dimension_height(cx.doc.settings());
            let input = DimensionSetsCreateInput {
                layer_id: layer.clone(),
                wall: None,
                walls: outside.iter().map(|&i| walls[i].0.to_string()).collect(),
                space: None,
                direction: Some(wire(f.d)),
                reference: DimensionSetReference::Faces,
                at: wire(f.at((f.t.0 + f.t.1) / 2.0, f.o.0 - distance)),
                edge: None,
                around: true,
                face: DimensionSetFace::Left,
                rows: self.stage.rows(),
                openings: if self.centers {
                    DimensionSetOpenings::Centers
                } else {
                    DimensionSetOpenings::Ends
                },
                gap_mm: GAP_MM,
                dimension_style: None,
                scales: Vec::new(),
                tight: DimensionSetTight::Shift,
                expected_revision: None,
            };
            match dimension_sets_create::plan_in(cx.doc, &input) {
                CommandResult::Completed { output, .. } => {
                    planned.outside = output.entities.len();
                    for e in &output.entities {
                        let p = crate::wall_dimensions::dimension_preview(
                            e,
                            &cx.format(),
                            cx.doc.settings().drawing_font,
                        );
                        planned.preview.strokes.extend(p.strokes);
                        planned.preview.texts.extend(p.texts);
                    }
                    planned.facade = Some(input);
                }
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => {
                    planned.error = Some(error.message);
                }
                _ => {}
            }
        }
        if self.inside {
            let rooms: Vec<(Vec2, Vec2)> = cx
                .doc
                .entities()
                .filter_map(|e| match e {
                    Entity::Space(s)
                        if !s.space.void && s.space.arch.level.as_deref() == cx.level =>
                    {
                        let pts: Vec<Vec2> = s
                            .space
                            .parts
                            .iter()
                            .flat_map(|p| p.outer.pts.iter().map(|q| Vec2::new(q.x, q.y)))
                            .collect();
                        let ts = pts.iter().map(|p| f.along(*p));
                        let os = pts.iter().map(|p| f.across(*p));
                        let t = ts.fold((f64::INFINITY, f64::NEG_INFINITY), |m, v| {
                            (m.0.min(v), m.1.max(v))
                        });
                        let o = os.fold((f64::INFINITY, f64::NEG_INFINITY), |m, v| {
                            (m.0.min(v), m.1.max(v))
                        });
                        (t.0.is_finite() && o.0.is_finite())
                            .then_some((Vec2::new(t.0, t.1), Vec2::new(o.0, o.1)))
                    }
                    _ => None,
                })
                .collect();
            planned.rooms = rooms.len();
            let spacing = CUT_SPACING_MM / 1000.0 * scale;
            // Chains along the frame, at the rooms' places across it; then across it, at their places along.
            let along = cuts_for(
                &rooms.iter().map(|(_, o)| (o.x, o.y)).collect::<Vec<_>>(),
                spacing,
            );
            let across = cuts_for(
                &rooms.iter().map(|(t, _)| (t.x, t.y)).collect::<Vec<_>>(),
                spacing,
            );
            let cut = |from: Vec2, to: Vec2| DimensionsInterior {
                from: wire(from),
                to: wire(to),
                at: None,
                faces: Some(InteriorFaces::Outer),
                columns: Some(InteriorColumns::Ends),
                total: Some(false),
                level: level.clone(),
                layer_id: layer.clone(),
                dimension_style: None,
                height: None,
                mask: None,
                look: NewLook::default(),
                expected_revision: None,
            };
            let mut cuts: Vec<DimensionsInterior> = along
                .iter()
                .map(|&o| cut(f.at(f.t.0 - REACH, o), f.at(f.t.1 + REACH, o)))
                .collect();
            cuts.extend(
                across
                    .iter()
                    .map(|&t| cut(f.at(t, f.o.0 - REACH), f.at(t, f.o.1 + REACH))),
            );
            for c in cuts {
                if let CommandResult::Completed { output, .. } =
                    dimensions_interior::plan_in(cx.doc, &c)
                {
                    if output.entities.is_empty() {
                        continue;
                    }
                    planned.inside += output.entities.len();
                    for e in &output.entities {
                        let p = crate::wall_dimensions::dimension_preview(
                            e,
                            &cx.format(),
                            cx.doc.settings().drawing_font,
                        );
                        planned.preview.strokes.extend(p.strokes);
                        planned.preview.texts.extend(p.texts);
                    }
                    planned.cuts.push(c);
                }
            }
        }
        let scope = scope(cx.level);
        let owned = if self.inside {
            owned_interiors(cx.doc, &scope)
        } else {
            Vec::new()
        };
        let exterior = facade_ids(&scope);
        let sets = exterior
            .iter()
            .filter_map(|&id| self.outside.then(|| cx.doc.dimension_set(id)).flatten());
        let mut members: Vec<_> = owned.iter().map(|(slot, _)| *slot).collect();
        for set in sets {
            planned.existing = true;
            members.extend(
                set.members
                    .iter()
                    .filter_map(|m| cx.doc.slot_of(kentos_domain::Uuid::from_bytes(m.uid.0))),
            );
        }
        planned.existing |= !owned.is_empty();
        if !self.separate {
            for slot in members {
                if let Some(e) = cx.doc.get(slot) {
                    let mut outline = crate::edge::Outline::of(
                        &kentos_native_application::geometry::shape(e),
                        Some([5.0, 4.0]),
                        2.0,
                        Tone::Accent,
                    );
                    planned.preview.strokes.append(&mut outline.strokes);
                }
            }
        }
        self.planned = planned;
    }

    /// The card's lines.
    fn lines(&self) -> Vec<String> {
        let p = &self.planned;
        let mut lines = Vec::new();
        if let Some(why) = &p.error {
            lines.push(why.clone());
        }
        if self.outside {
            lines.push(format!(
                "{}: dört cephe, {} zincir · {} ölçü · {} dış duvar",
                self.stage.words(),
                self.stage.rows().len(),
                p.outside,
                p.exterior
            ));
        }
        if self.inside {
            lines.push(format!(
                "İç: {} zincir · {} ölçü · {} mahal, her biri 2+2",
                p.cuts.len(),
                p.inside,
                p.rooms
            ));
        }
        lines.push(format!("Enter: {}", self.action_label()));
        lines
    }

    fn action_label(&self) -> &'static str {
        if self.separate {
            "Ayrı takım oluştur"
        } else if self.planned.existing {
            "Güncelle"
        } else {
            "Oluştur"
        }
    }

    /// Writes the plans: one undo step on the dimensions' layer (opened with them).
    fn write(&mut self, cx: &mut Context<'_>) -> bool {
        if self.seen != Some(cx.doc.revision()) {
            self.plan(cx);
        }
        let planned = std::mem::take(&mut self.planned);
        if planned.facade.is_none() && planned.cuts.is_empty() {
            cx.say(
                Level::Warn,
                planned
                    .error
                    .unwrap_or_else(|| "Ölçülecek bir şey bulunmadı.".into()),
            );
            return false;
        }
        let group = cx.doc.begin_group(LABEL);
        let scope = scope(cx.level);
        let mut sets = 0;
        let chains = planned.cuts.len();
        let ok = crate::arch_layer::writing(&Kind::Dimension, cx, |id, cx| {
            if let Some(facade) = planned.facade {
                let input = DimensionSetsCreateInput {
                    layer_id: id.to_owned(),
                    ..facade
                };
                let count = if self.separate {
                    let result =
                        dimension_sets_create::execute(&mut ExecutionContext::new(cx.doc), input);
                    crate::points::written(result, cx).map(|d| d.sets.len())
                } else {
                    let result = dimension_sets_create::refresh_plan_facades(
                        cx.doc,
                        &input,
                        &facade_ids(&scope),
                    );
                    crate::points::written(result, cx)
                };
                let Some(count) = count else { return false };
                sets = count;
            }
            if self.separate {
                return planned.cuts.into_iter().all(|c| {
                    let input = DimensionsInterior {
                        layer_id: id.to_owned(),
                        ..c
                    };
                    let result =
                        dimensions_interior::execute(&mut ExecutionContext::new(cx.doc), input);
                    crate::points::written(result, cx).is_some()
                });
            }
            if self.inside {
                let mut fresh = Vec::new();
                for c in planned.cuts {
                    let input = DimensionsInterior {
                        layer_id: id.to_owned(),
                        ..c
                    };
                    let result = dimensions_interior::plan_in(cx.doc, &input);
                    let Some(output) = crate::points::written(result, cx) else {
                        return false;
                    };
                    fresh.extend(output.entities);
                }
                if let Err(error) = refresh_interior(cx.doc, &scope, fresh) {
                    cx.say(Level::Warn, error.to_string());
                    return false;
                }
            }
            true
        });
        if !ok {
            cx.doc.cancel_group(group);
            self.seen = None;
            return false;
        }
        cx.doc.end_group(group);
        let mut words = Vec::new();
        if sets > 0 {
            words.push(format!(
                "{} dış dizi ({}, dört cephe)",
                sets,
                self.stage.words().to_lowercase()
            ));
        }
        if chains > 0 {
            words.push(format!("{chains} iç zincir"));
        }
        let result = if planned.existing && !self.separate {
            "Plan ölçüleri güncellendi"
        } else {
            "Plan ölçülendi"
        };
        cx.say(Level::Success, format!("{result}: {}.", words.join(", ")));
        self.seen = None;
        true
    }
}

impl Tool for PlanDimensions {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(LABEL, "ölçü takımının önizlemesi")
            .option(self.action_label(), "Enter")
            .option_with(
                "Ölçü takımı",
                "T",
                if self.separate {
                    "Ayrı takım"
                } else {
                    "Mevcut takım"
                },
            )
            .option_with("Aşama", "A", self.stage.words())
            .option_with(
                "Dış zincirler",
                "D",
                if self.outside { "Açık" } else { "Kapalı" },
            )
            .option_with(
                "İç zincirler",
                "İ",
                if self.inside { "Açık" } else { "Kapalı" },
            )
    }

    /// It takes no points: the plan is the drawing's.
    fn point_count(&self) -> usize {
        0
    }

    /// Ctrl+Z: nothing of its own to take back; the drawing is undone.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        // An uygulama projesi is drawn at 1:50 (MO): its fourth chain from the start.
        if cx.doc.settings().plot_scale <= 50.0 {
            self.stage = Stage::Construction;
        }
        self.plan(cx);
        Flow::Stay
    }

    fn layer_kind(&self, _cx: &Context<'_>) -> Option<Kind> {
        Some(Kind::Dimension)
    }

    fn pointer_move(&mut self, _p: &Pointer, cx: &mut Context<'_>) {
        if self.seen != Some(cx.doc.revision()) {
            self.plan(cx);
        }
    }

    fn pointer_down(&mut self, _p: &Pointer, cx: &mut Context<'_>) {
        self.write(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(text.trim());
        let done = match key.as_str() {
            "A" => {
                self.stage = match self.stage {
                    Stage::Preliminary => Stage::Construction,
                    Stage::Construction => Stage::Preliminary,
                };
                true
            }
            "T" => {
                self.separate = !self.separate;
                true
            }
            "D" => {
                self.outside = !self.outside;
                true
            }
            "İ" | "I" => {
                self.inside = !self.inside;
                true
            }
            _ => false,
        };
        if done {
            self.plan(cx);
        }
        done
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.write(cx) {
            Flow::Exit
        } else {
            Flow::Stay
        }
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = self.planned.preview.clone();
        out.tag = self.planned.at.map(|at| Tag {
            at,
            lines: self.lines(),
        });
        out
    }

    fn settings(&self, _format: &Format) -> Vec<Setting> {
        vec![
            Setting::choice(
                "A",
                "Aşama",
                choices(
                    &[
                        ("preliminary", "Ön proje · 3 dış zincir"),
                        ("construction", "Uygulama · 4 dış zincir"),
                    ],
                    self.stage.key(),
                ),
            ),
            Setting::choice(
                "T",
                "Ölçü takımı",
                choices(
                    &[
                        ("current", "Varsayılan takım"),
                        ("separate", "Ayrı takım oluştur"),
                    ],
                    if self.separate { "separate" } else { "current" },
                ),
            ),
            Setting::toggle("D", "Dış zincirler", self.outside),
            Setting::toggle("İ", "İç zincirler (mahal başına 2+2)", self.inside),
            Setting::choice(
                "B",
                "Açıklıkları ölç",
                choices(
                    &[
                        ("ends", "Açıklık uçları"),
                        ("centers", "Açıklık merkezleri"),
                    ],
                    if self.centers { "centers" } else { "ends" },
                ),
            ),
        ]
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let on = |v: &str| {
            matches!(
                crate::prompt::fold_tr(v.trim()).as_str(),
                "YES" | "AÇIK" | "ACIK" | "EVET" | "1" | "TRUE"
            )
        };
        match key {
            "A" => {
                self.stage = if value == "construction" {
                    Stage::Construction
                } else {
                    Stage::Preliminary
                }
            }
            "T" => self.separate = value == "separate",
            "D" => self.outside = on(value),
            "İ" => self.inside = on(value),
            "B" => self.centers = value == "centers",
            _ => return false,
        }
        self.plan(cx);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::cuts_for;

    #[test]
    fn every_room_is_crossed_twice_by_as_few_cuts_as_can_be() {
        // Three rooms side by side across the cuts (0–4, 0–3, 3–6), cuts 0.6 apart.
        let cuts = cuts_for(&[(0.0, 4.0), (0.0, 3.0), (3.0, 6.0)], 0.6);
        let crossing = |lo: f64, hi: f64| cuts.iter().filter(|c| **c > lo && **c < hi).count();
        assert!(
            crossing(0.0, 4.0) >= 2 && crossing(0.0, 3.0) >= 2 && crossing(3.0, 6.0) >= 2,
            "{cuts:?}"
        );
        assert!(cuts.len() <= 4, "shared where they can be: {cuts:?}");
        for w in cuts.windows(2) {
            assert!(w[1] - w[0] >= 0.6 - 1e-9, "{cuts:?}");
        }
        // A corridor 0.9 wide: crossed once, in its middle.
        assert_eq!(cuts_for(&[(5.0, 5.9)], 0.6), vec![5.45]);
    }
}
