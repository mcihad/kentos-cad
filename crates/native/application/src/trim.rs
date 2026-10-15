//! `cad.entities.trim` v1 and `cad.entities.extend` v1 (docs/adr/0250):
//! objects cut back to, or grown up to, their boundaries, from what was
//! asked (the picks and the boundaries), as one undo step “Buda” (“Uzat”).
//! The desktop's handler over the native document, run by the Buda and Uzat
//! tools (their preview is [`plan_in`], their click and fence [`execute`]),
//! the headless server, Python and MCP. The shared cases are
//! `fixtures/commands/v1/cad.entities.trim.json` and `.extend.json`.
//!
//! Nothing is computed twice: the pieces are the shared geometry store's
//! (`trim_preview`, `extend_preview`: the boundaries near the target, then
//! the core's `trim_entity`, `extend_entity`), as the tools always took
//! them, and they are written through `cad.entities.edit`'s `trim` and
//! `extend` operations ([`crate::edit`]).
//!
//! The checks, in order (the first that fails answers):
//! 1. at least one pick; a boundary list not empty;
//! 2. every id lowercase UUID text with hyphens (the picks', then the boundaries');
//! 3. every point finite (the picks', then the rectangle's);
//! 4. the expected revision (every command's, `checks.rs`);
//! 5. every id names an object (the picks', then the boundaries');
//! 6. no picked object on a locked layer; for Buda no area with holes;
//! 7. at least one pick does something (each that does not, a warning).

use std::collections::BTreeMap;

use kentos_contracts::{
    CommandResult, CommandWarning, EditOperation, EntitiesEdit, EntitiesTrim, EntitiesTrimPlan,
    EntitiesTrimmed, Entity, EntityEdit, TrimBoundaries, TrimEdgeMode,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape, area_parts, is_multi_part};
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::curve_cuts::{Cut, Geometry};
use kentos_geometry_core::ops::offset::through_distance;
use kentos_geometry_core::ops::parts::polyline_of_pieces;
use kentos_geometry_core::ops::path::{nearest_s, path_of, point_at_s};
use kentos_geometry_core::ops::trim::{fence_crossings, trimmable, window_point, window_points};
use kentos_geometry_core::store::Store;
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_uid_text};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;
use crate::edit;
use crate::geometry::{edit_geometry, shape};
use crate::geometry_store::store_of;

/// What the command does to its picks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Trim,
    Extend,
}

impl Act {
    /// The undo step's name: the tool's (docs/adr/0047).
    pub fn label(self) -> &'static str {
        match self {
            Act::Trim => "Buda",
            Act::Extend => "Uzat",
        }
    }

    fn failed(self) -> &'static str {
        match self {
            Act::Trim => codes::TRIM_FAILED,
            Act::Extend => codes::EXTEND_FAILED,
        }
    }

    fn operation(self) -> EditOperation {
        match self {
            Act::Trim => EditOperation::Trim,
            Act::Extend => EditOperation::Extend,
        }
    }
}

/// How near a later pick must lie to a piece an earlier one left to be on it, metres (the fence's rule).
const ON_PIECE: f64 = 1e-6;

/// The words of a cut or a growth that came to nothing, the core having said none.
const NOTHING: [&str; 2] = [
    "Budanacak bir parça yok; sınırlar bu noktanın iki yanında nesneyi kesmiyor.",
    "Uzatılacak bir uç yok; uç hiçbir sınıra ulaşmıyor.",
];

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesTrim, act: Act) -> CommandResult<()> {
    match check(cx.doc, cx.store, input, act) {
        Ok(checked) => CommandResult::Completed {
            output: (),
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &EntitiesTrim,
    act: Act,
) -> CommandResult<EntitiesTrimPlan> {
    plan_in(cx.doc, cx.store, input, act)
}

/// [`plan`] over the document and, when the host has one, its store (the tools' preview).
pub fn plan_in(
    doc: &Document,
    store: Option<&Store>,
    input: &EntitiesTrim,
    act: Act,
) -> CommandResult<EntitiesTrimPlan> {
    let checked = match check(doc, store, input, act) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    match edit::plan_in(doc, &checked.edit) {
        CommandResult::Completed { output, warnings } => CommandResult::Completed {
            output: EntitiesTrimPlan {
                changed: output.changed,
                created: output.created,
                removed: output.removed,
                revision: output.revision,
            },
            warnings: [checked.warnings, warnings].concat(),
        },
        other => relay(other),
    }
}

/// Checks `input` again and writes what the picks do as one undo step, into
/// the open transaction or group if one is.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesTrim,
    act: Act,
) -> CommandResult<EntitiesTrimmed> {
    let checked = match check(cx.doc, cx.store, &input, act) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    match edit::execute(cx, checked.edit) {
        CommandResult::Completed { output, warnings } => CommandResult::Completed {
            output: EntitiesTrimmed {
                changed: output.changed,
                created: output.created,
                removed: output.removed,
                revision: output.revision,
            },
            warnings: [checked.warnings, warnings].concat(),
        },
        other => relay(other),
    }
}

pub(crate) fn relay<T, U>(result: CommandResult<T>) -> CommandResult<U> {
    match result {
        CommandResult::Completed { .. } | CommandResult::Cancelled => CommandResult::Cancelled,
        CommandResult::Queued { job_id } => CommandResult::Queued { job_id },
        CommandResult::NeedsInput { error } => CommandResult::NeedsInput { error },
        CommandResult::Conflict { error } => CommandResult::Conflict { error },
        CommandResult::Failed { error } => CommandResult::Failed { error },
    }
}

struct Checked {
    edit: EntitiesEdit,
    warnings: Vec<CommandWarning>,
}

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// The changes that replace the object at `slot` by `pieces`: the first
/// piece is the object itself, the others new objects from it; attributes
/// and the label survive a single piece of anything but a closed area. No
/// piece deletes it (the tools' rule since docs/adr/0047).
pub fn replace_changes(doc: &Document, slot: Slot, pieces: &[Shape]) -> Vec<EntityEdit> {
    let polygon = matches!(doc.get(slot), Some(Entity::Polygon(_)));
    let keep = (pieces.len() == 1 && !polygon).then_some(true);
    let id = doc.uid(slot).map(|u| u.to_string()).unwrap_or_default();
    let mut changes = Vec::with_capacity(pieces.len().max(1));
    let mut geometries = pieces.iter().filter_map(|s| edit_geometry(s.clone()));
    match geometries.next() {
        Some(first) => changes.push(EntityEdit::Replace {
            uid: id.clone(),
            geometry: first,
            keep_data: keep,
        }),
        None => changes.push(EntityEdit::Remove { uid: id.clone() }),
    }
    for piece in geometries {
        changes.push(EntityEdit::add(id.clone(), piece, keep));
    }
    changes
}

/// The piece a later pick lies on, of those an earlier one left (the fence's rule).
fn piece_at(pieces: &[Shape], at: Vec2) -> Option<usize> {
    pieces
        .iter()
        .enumerate()
        .map(|(i, p)| (i, through_distance(p, at)))
        .filter(|(_, d)| *d < ON_PIECE)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

fn find(doc: &Document, uid: &str) -> Option<Slot> {
    Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u))
}

fn not_found(uid: &str, path: String) -> Stop {
    refuse(
        codes::ENTITY_NOT_FOUND,
        format!(
            "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
        ),
        &path,
    )
}

fn check(
    doc: &Document,
    store: Option<&Store>,
    input: &EntitiesTrim,
    act: Act,
) -> Result<Checked, Stop> {
    let fence = input.fence.as_deref();
    if input.picks.is_empty() && fence.is_none() && input.crossing.is_none() {
        let what = match act {
            Act::Trim => "Budanacak",
            Act::Extend => "Uzatılacak",
        };
        return Err(refuse(
            codes::NO_ENTITIES,
            format!("{what} nesne yok. En az bir nesneyi ve tıklandığı yeri verin."),
            "picks",
        ));
    }
    let listed = match &input.boundaries {
        Some(TrimBoundaries::Objects { uids }) => Some(uids.as_slice()),
        _ => None,
    };
    if listed.is_some_and(<[String]>::is_empty) {
        return Err(refuse(
            codes::NO_ENTITIES,
            "Sınır nesnesi yok. Sınırları verin ya da bütün görünen nesneler için “all”u kullanın."
                .into(),
            "boundaries.uids",
        ));
    }
    if fence.is_some_and(|f| f.len() < 2) {
        return Err(refuse(
            codes::TOO_FEW_POINTS,
            "Çit için en az iki nokta gerekir; çitin noktalarını sırayla verin.".into(),
            "fence",
        ));
    }
    let uids = input
        .picks
        .iter()
        .enumerate()
        .map(|(i, p)| (p.uid.as_str(), format!("picks[{i}].uid")))
        .chain(
            listed
                .unwrap_or_default()
                .iter()
                .enumerate()
                .map(|(i, u)| (u.as_str(), format!("boundaries.uids[{i}]"))),
        );
    for (uid, path) in uids.clone() {
        if !is_uid_text(uid) {
            return Err(refuse(
                codes::INVALID_UID,
                format!(
                    "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
                ),
                &path,
            ));
        }
    }
    for (i, p) in input.picks.iter().enumerate() {
        checks::point(p.at, "Tıklanan noktanın", &format!("picks[{i}].at"))?;
    }
    for (i, p) in fence.unwrap_or_default().iter().enumerate() {
        checks::point(*p, "Çitin noktasının", &format!("fence[{i}]"))?;
    }
    if let Some(w) = &input.crossing {
        checks::point(w.min, "Pencerenin köşesinin", "crossing.min")?;
        checks::point(w.max, "Pencerenin köşesinin", "crossing.max")?;
    }
    if let Some(TrimBoundaries::Area { min, max }) = &input.boundaries {
        checks::point(*min, "Dikdörtgenin köşesinin", "boundaries.min")?;
        checks::point(*max, "Dikdörtgenin köşesinin", "boundaries.max")?;
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let mut slots = Vec::with_capacity(input.picks.len());
    for (i, p) in input.picks.iter().enumerate() {
        slots.push(find(doc, &p.uid).ok_or_else(|| not_found(&p.uid, format!("picks[{i}].uid")))?);
    }
    let mut chosen: Option<Vec<f64>> = None;
    if let Some(list) = listed {
        let mut ids = Vec::with_capacity(list.len());
        for (i, u) in list.iter().enumerate() {
            let slot = find(doc, u).ok_or_else(|| not_found(u, format!("boundaries.uids[{i}]")))?;
            ids.push(f64::from(slot.0));
        }
        chosen = Some(ids);
    }
    let layers = doc.layers();
    for (i, slot) in slots.iter().enumerate() {
        let Some(e) = doc.get(*slot) else { continue };
        let layer = &e.base().layer_id;
        if layers.is_locked(layer) {
            let name = layers
                .get(layer)
                .map_or(layer.as_str(), |n| n.name.as_str());
            return Err(refuse(
                codes::LAYER_LOCKED,
                format!(
                    "“{name}” katmanı kilitli; üzerindeki nesne değiştirilemez. Kilidi Katmanlar panelinden açın."
                ),
                &format!("picks[{i}]"),
            ));
        }
        if act == Act::Trim && holed(e) {
            return Err(refuse(
                codes::NOT_TRIMMABLE,
                HOLES.into(),
                &format!("picks[{i}]"),
            ));
        }
    }
    // The boundaries: the host's store when it keeps one, else one of the document.
    let own;
    let store = match store {
        Some(s) => s,
        None => {
            own = store_of(doc);
            &own
        }
    };
    let view = match &input.boundaries {
        Some(TrimBoundaries::Area { min, max }) => rect(*min, *max),
        _ => Bounds {
            min_x: f64::MIN,
            min_y: f64::MIN,
            max_x: f64::MAX,
            max_y: f64::MAX,
        },
    };
    let mut run = Run {
        doc,
        store,
        chosen,
        view,
        extended: input.edge_mode == Some(TrimEdgeMode::Extend),
        erase: act == Act::Trim && input.erase_uncut == Some(true),
        act,
        warnings: Vec::new(),
        first_why: None,
        order: Vec::new(),
        work: BTreeMap::new(),
    };
    for (i, (p, slot)) in input.picks.iter().zip(&slots).enumerate() {
        run.pick(*slot, Vec2::new(p.at.x, p.at.y), &format!("picks[{i}]"));
    }
    let mut locked = 0;
    if let Some(points) = fence {
        let pts: Vec<Vec2> = points.iter().map(|p| Vec2::new(p.x, p.y)).collect();
        let mut crossed = false;
        for (slot, shut) in run.shown_in(&rect_of(&pts)) {
            let Some(e) = doc.get(slot) else { continue };
            // A multi-part polyline is crossed part by part (docs/adr/0307).
            let at: Vec<Vec2> = area_parts(&shape(e))
                .iter()
                .flat_map(|part| fence_crossings(&CoreEntity::new(part.clone()), &pts))
                .collect();
            if at.is_empty() {
                continue;
            }
            if shut {
                locked += 1;
                continue;
            }
            crossed = true;
            if act == Act::Trim && holed(e) {
                run.warn("fence", HOLES.into());
                continue;
            }
            for p in at {
                run.pick(slot, p, "fence");
            }
        }
        // Only locked objects crossed: the count says it.
        if !crossed && locked == 0 {
            let verb = match act {
                Act::Trim => "budanacak",
                Act::Extend => "uzatılacak",
            };
            run.warn(
                "fence",
                format!("Çit hiçbir nesneyi kesmiyor; çiti {verb} nesnelerin üstünden geçirin."),
            );
        }
    }
    if let Some(w) = &input.crossing {
        let window = rect(w.min, w.max);
        let mut met = false;
        let before = locked;
        for (slot, shut) in run.shown_in(&window) {
            match run.window(slot, &window, shut) {
                Met::No => {}
                Met::Locked => locked += 1,
                Met::Yes => met = true,
            }
        }
        if !met && locked == before {
            let what = match act {
                Act::Trim => "budanacak bir parçaya",
                Act::Extend => "uzatılacak bir uca",
            };
            run.warn(
                "crossing",
                format!("Pencere {what} değmiyor; pencereyi nesnelerin üstüne çizin."),
            );
        }
    }
    let skipped = (locked > 0).then(|| CommandWarning {
        code: codes::LAYER_LOCKED.into(),
        message: format!(
            "{locked} nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın."
        ),
        path: Some(if fence.is_some() { "fence" } else { "crossing" }.into()),
    });
    let mut changes = Vec::new();
    for slot in &run.order {
        let w = &run.work[slot];
        if !w.done {
            continue;
        }
        match act {
            // A multi-part polyline's pieces stay parts of the one object (docs/adr/0307).
            Act::Trim if w.parted && !w.pieces.is_empty() => {
                if let Some(joined) = polyline_of_pieces(&w.pieces) {
                    changes.extend(replace_changes(doc, *slot, &[joined]));
                }
            }
            Act::Trim => changes.extend(replace_changes(doc, *slot, &w.pieces)),
            Act::Extend => {
                if let Some(geometry) = edit_geometry(w.current.clone()) {
                    changes.push(EntityEdit::Update {
                        uid: doc.uid(*slot).map(|u| u.to_string()).unwrap_or_default(),
                        geometry,
                    });
                }
            }
        }
    }
    if changes.is_empty() {
        let path = if !input.picks.is_empty() {
            "picks"
        } else if fence.is_some() {
            "fence"
        } else {
            "crossing"
        };
        return Err(match (run.first_why, skipped) {
            (None, Some(w)) => refuse(codes::LAYER_LOCKED, w.message, path),
            (why, _) => refuse(
                act.failed(),
                why.unwrap_or_else(|| NOTHING[act as usize].into()),
                path,
            ),
        });
    }
    let mut warnings = run.warnings;
    warnings.extend(skipped);
    Ok(Checked {
        edit: EntitiesEdit {
            operation: act.operation(),
            changes,
            expected_revision: None,
        },
        warnings,
    })
}

/// The words of an area with holes Buda refuses.
const HOLES: &str = "Adalı alanda budama yapılamaz; iç halkalar kaybolurdu. Önce Patlat ile halkalarına ayırın ya da Alan böl kullanın.";

/// Whether `e` is an area with holes (Buda would lose them).
fn holed(e: &Entity) -> bool {
    matches!(e, Entity::Polygon(p) if p.holes.as_ref().is_some_and(|h| !h.is_empty()))
}

fn rect(a: kentos_contracts::Vec2, b: kentos_contracts::Vec2) -> Bounds {
    Bounds {
        min_x: a.x.min(b.x),
        min_y: a.y.min(b.y),
        max_x: a.x.max(b.x),
        max_y: a.y.max(b.y),
    }
}

fn rect_of(pts: &[Vec2]) -> Bounds {
    pts.iter().fold(
        Bounds {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        },
        |b, p| Bounds {
            min_x: b.min_x.min(p.x),
            min_y: b.min_y.min(p.y),
            max_x: b.max_x.max(p.x),
            max_y: b.max_y.max(p.y),
        },
    )
}

/// What a window did with one object.
enum Met {
    /// Nothing it acts on: a kind it does not cut or grow, or only touched.
    No,
    /// Something it would act on, on a locked layer: left alone, counted.
    Locked,
    Yes,
}

/// Steps a window takes on one object at most: each takes a piece away.
const WINDOW_STEPS: usize = 10_000;

/// One object as the picks leave it.
struct Work {
    /// Buda: the pieces left, in order along the object; none: it went whole. A multi-part polyline's
    /// pieces start as its parts (docs/adr/0307).
    pieces: Vec<Shape>,
    /// Buda: whether each piece is as the object had it, no pick having cut it. A piece an earlier cut
    /// left that no boundary cuts goes whole (its ends are cuts or the object's).
    fresh: Vec<bool>,
    /// A multi-part polyline: a pick acts on the part it is near, the parts written back as one object.
    parted: bool,
    /// Uzat: the object as grown so far, and which ends of each part grew.
    current: Shape,
    used: Vec<[bool; 2]>,
    done: bool,
}

/// Whether Buda and Uzat take `s` part by part: a multi-part polyline (docs/adr/0307).
fn parted(s: &Shape) -> bool {
    matches!(s, Shape::Polyline { .. }) && is_multi_part(s)
}

/// The piece of `pieces` nearest `at`, with its distance.
fn nearest_piece(pieces: &[Shape], at: Vec2) -> Option<(usize, f64)> {
    pieces
        .iter()
        .enumerate()
        .map(|(i, p)| (i, through_distance(p, at)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

/// The command's walk over its picks.
struct Run<'a> {
    doc: &'a Document,
    store: &'a Store,
    chosen: Option<Vec<f64>>,
    view: Bounds,
    extended: bool,
    erase: bool,
    act: Act,
    warnings: Vec<CommandWarning>,
    first_why: Option<String>,
    order: Vec<Slot>,
    work: BTreeMap<Slot, Work>,
}

impl Run<'_> {
    fn warn(&mut self, path: &str, why: String) {
        if self.first_why.is_none() {
            self.first_why = Some(why.clone());
        }
        self.warnings.push(CommandWarning {
            code: self.act.failed().into(),
            message: why,
            path: Some(path.into()),
        });
    }

    /// The shown objects meeting `r` in the drawing's order, each with whether its layer is locked.
    fn shown_in(&self, r: &Bounds) -> Vec<(Slot, bool)> {
        let mut ids = self.store.in_rect(r, true);
        ids.sort_by(f64::total_cmp);
        let layers = self.doc.layers();
        ids.into_iter()
            .filter_map(|id| {
                let slot = Slot(id as u32);
                let e = self.doc.get(slot)?;
                Some((slot, layers.is_locked(&e.base().layer_id)))
            })
            .collect()
    }

    fn work(&mut self, slot: Slot) -> Option<&mut Work> {
        if !self.work.contains_key(&slot) {
            let e = self.doc.get(slot)?;
            let s = shape(e);
            self.order.push(slot);
            let parted = parted(&s);
            let pieces = if parted {
                area_parts(&s).into_owned()
            } else {
                vec![s.clone()]
            };
            let n = pieces.len();
            self.work.insert(
                slot,
                Work {
                    pieces,
                    fresh: vec![true; n],
                    parted,
                    current: s,
                    used: vec![[false; 2]; n],
                    done: false,
                },
            );
        }
        self.work.get_mut(&slot)
    }

    fn cut(&self, slot: Slot, piece: &Shape, at: Vec2) -> Cut {
        let target = CoreEntity::new(piece.clone());
        self.store.trim_preview_in(
            &target,
            at,
            Some(f64::from(slot.0)),
            self.chosen.as_deref(),
            &self.view,
            self.extended,
        )
    }

    /// Whether a pick at `at` cuts the object as it is (a boundary is around it).
    fn cuts_at(&self, slot: Slot, original: &Shape, at: Vec2) -> bool {
        matches!(self.cut(slot, original, at), Cut::Pieces(_))
    }

    fn grow(&self, slot: Slot, current: &Shape, at: Vec2) -> Geometry {
        let target = CoreEntity::new(current.clone());
        self.store.extend_preview_in(
            &target,
            at,
            Some(f64::from(slot.0)),
            self.chosen.as_deref(),
            &self.view,
            self.extended,
        )
    }

    /// A pick on the object at `slot`: Buda cuts the piece it lies on, Uzat grows the end nearest it.
    fn pick(&mut self, slot: Slot, at: Vec2, path: &str) {
        match self.act {
            Act::Trim => {
                self.trim_at(slot, at, path);
            }
            Act::Extend => self.extend_at(slot, at, path),
        };
    }

    /// Buda at `at`: whether something went. A first pick is on the object as it is (wherever it lies, as a
    /// click near it); a later one on the piece an earlier left it lies on, or nothing.
    fn trim_at(&mut self, slot: Slot, at: Vec2, path: &str) {
        let Some(w) = self.work(slot) else { return };
        let k = if w.parted {
            // The part the pick is near, as on an object as it is; a piece an earlier cut left, only on it.
            nearest_piece(&w.pieces, at)
                .and_then(|(k, d)| (w.fresh[k] || d < ON_PIECE).then_some(k))
        } else if w.fresh.first() == Some(&true) && w.pieces.len() == 1 {
            Some(0)
        } else {
            piece_at(&w.pieces, at)
        };
        if let Some(k) = k {
            self.trim_piece(slot, k, at, path);
        }
    }

    /// Buda of the `k`th piece at `at`: whether something went.
    fn trim_piece(&mut self, slot: Slot, k: usize, at: Vec2, path: &str) -> bool {
        let Some(w) = self.work(slot) else {
            return false;
        };
        let piece = w.pieces[k].clone();
        let was_cut = !w.fresh[k];
        let result = self.cut(slot, &piece, at);
        let erase = self.erase && trimmable(&piece);
        let now = match result {
            Cut::Pieces(now) => Some(now.into_iter().map(|p| p.shape).collect()),
            // A piece an earlier cut left that no boundary cuts goes whole: its ends are cuts.
            Cut::Error(_) if was_cut => Some(Vec::new()),
            // Quick mode: what no boundary cuts goes whole.
            Cut::Error(_) if erase => None,
            Cut::Error(why) => {
                let why = if why.is_empty() {
                    NOTHING[0].into()
                } else {
                    why
                };
                self.warn(path, why);
                return false;
            }
        };
        let Some(w) = self.work.get_mut(&slot) else {
            return false;
        };
        // What no boundary cuts in quick mode goes whole: the piece, a part of a multi-part line alone.
        let now = now.unwrap_or_default();
        w.fresh.splice(k..=k, vec![false; now.len()]);
        w.pieces.splice(k..=k, now);
        w.done = true;
        true
    }

    /// Uzat at `at`: the end of the object nearest it grows, once.
    fn extend_at(&mut self, slot: Slot, at: Vec2, path: &str) {
        let Some(w) = self.work(slot) else { return };
        // A multi-part polyline grows the part the pick is near (docs/adr/0307).
        let parts = area_parts(&w.current).into_owned();
        let k = if w.parted {
            nearest_piece(&parts, at).map_or(0, |(k, _)| k)
        } else {
            0
        };
        let target = if w.parted {
            parts[k].clone()
        } else {
            w.current.clone()
        };
        let Some(line) = path_of(&target).filter(|p| !p.closed) else {
            self.warn(
                path,
                "Yalnız açık çizgiler, çoklu çizgiler ve yaylar uzatılabilir.".into(),
            );
            return;
        };
        let end = usize::from(nearest_s(&line, at) > line.length / 2.0);
        if std::mem::replace(&mut w.used[k][end], true) {
            return;
        }
        match self.grow(slot, &target, at) {
            Geometry::Ok(g) => {
                if let Some(w) = self.work.get_mut(&slot) {
                    let grown = if w.parted {
                        let mut parts = parts;
                        parts[k] = g.shape;
                        polyline_of_pieces(&parts)
                    } else {
                        Some(g.shape)
                    };
                    if let Some(grown) = grown {
                        w.current = grown;
                        w.done = true;
                    }
                }
            }
            Geometry::Error(why) => {
                let why = if why.is_empty() {
                    NOTHING[1].into()
                } else {
                    why
                };
                self.warn(path, why);
            }
        }
    }

    /// Kesişen on one object: Buda takes away every piece between the boundaries that meets the window
    /// (each in turn, from the object's start), Uzat grows every end inside it. Whether the window met
    /// something it could act on (on a locked layer: left alone); kinds it cannot are passed over without
    /// a word.
    fn window(&mut self, slot: Slot, window: &Bounds, locked: bool) -> Met {
        let Some(e) = self.doc.get(slot) else {
            return Met::No;
        };
        let s = shape(e);
        match self.act {
            Act::Trim => {
                let pieces = match self.work.get(&slot) {
                    Some(w) => w.pieces.clone(),
                    None if parted(&s) => area_parts(&s).into_owned(),
                    None => vec![s.clone()],
                };
                if !(trimmable(&s) || parted(&s))
                    || pieces.iter().all(|p| window_point(p, window).is_none())
                {
                    return Met::No;
                }
                if locked {
                    return Met::Locked;
                }
                if holed(e) {
                    self.warn("crossing", HOLES.into());
                    return Met::Yes;
                }
                for _ in 0..WINDOW_STEPS {
                    let pieces = match self.work.get(&slot) {
                        Some(w) => w.pieces.clone(),
                        None if parted(&s) => area_parts(&s).into_owned(),
                        None => vec![s.clone()],
                    };
                    let Some((k, at)) = pieces.iter().enumerate().find_map(|(k, p)| {
                        let at = window_points(p, window);
                        (!at.is_empty()).then_some((k, at))
                    }) else {
                        break;
                    };
                    // On the object as it is, a pick that lands on its only crossing finds none around it:
                    // the part's other points are tried first.
                    let fresh = self.work.get(&slot).is_none_or(|w| w.fresh[k]);
                    let q = if fresh && !self.cuts_at(slot, &pieces[k], at[0]) {
                        at[1..]
                            .iter()
                            .copied()
                            .find(|q| self.cuts_at(slot, &pieces[k], *q))
                            .unwrap_or(at[0])
                    } else {
                        at[0]
                    };
                    if !self.trim_piece(slot, k, q, "crossing") {
                        break;
                    }
                }
                Met::Yes
            }
            Act::Extend => {
                let current = match self.work.get(&slot) {
                    Some(w) => w.current.clone(),
                    None => s,
                };
                let lines: Vec<_> = area_parts(&current)
                    .iter()
                    .filter_map(path_of)
                    .filter(|p| !p.closed)
                    .collect();
                if lines.is_empty() {
                    return Met::No;
                }
                let ends: Vec<Vec2> = lines
                    .iter()
                    .flat_map(|line| [point_at_s(line, 0.0), point_at_s(line, line.length)])
                    .filter(|p| {
                        p.x >= window.min_x
                            && p.x <= window.max_x
                            && p.y >= window.min_y
                            && p.y <= window.max_y
                    })
                    .collect();
                if ends.is_empty() {
                    return Met::No;
                }
                if locked {
                    return Met::Locked;
                }
                for p in ends {
                    self.extend_at(slot, p, "crossing");
                }
                Met::Yes
            }
        }
    }
}
