//! The drawing's kitchen runs (docs/adr/0706, 0727): how a run's recipe lays its objects on this drawing
//! (`lay_casework`: `cad.casework.run`'s, `cad.casework.edit`'s and the follower's one rule), the runs kept as undoable
//! state like the associative arrays (`CaseworkRecord`), the walls they follow (`follow_casework`) and the objects
//! they own (`casework_faults`).
//!
//! A run's path is `from`, its corners `through`, `to`; each piece a leg. Its frame on a leg: the unit direction along
//! it and the unit normal into the run's side. The rules (docs/adr/0727 §1–§4):
//! - a corner is square and turns toward the cabinets; blind: the leg before keeps the corner, the leg after starts a
//!   cabinet's depth and 5 cm from it (a 5 cm corner filler); open: both stop a cabinet's depth from it (its square a
//!   filler); a module of the leg before within a cabinet's depth of a blind corner is blind (the next leg's cabinets
//!   before its front): a sink, cooktop or dishwasher there is said (`casework_blind_corner`);
//! - modules are numbered from `from` and laid while they fit (a hair, 1e-9, of slack): the first leg of an L or U back
//!   from its corner, the others from their starts; a leg's leftover over 1 mm is a filler;
//! - widths: a module's own, else the sink's 0.90, an appliance's 0.60, the cooktop's at least 0.60, else the run's;
//! - the sink 5 cm in from its module's start and the line, the cooktop at its module's middle 4 cm from the line,
//!   each fitting its module and the countertop; the countertop's holes 2 cm in from their boxes;
//! - a leg is tied to the first wall face edge both its ends lie on (within 1 mm, between its ends).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use kentos_contracts::block_dynamic::{BlockParameter, ParamValue, same_param_value, value_problem};
use kentos_contracts::codes;
use kentos_contracts::{
    BlockDefinition, BlockId, BlockUnits, CASEWORK_APPLIANCE, CASEWORK_SINK_MODULE, CaseworkBreak,
    CaseworkCorner, CaseworkEnd, CaseworkFace, CaseworkFridge, CaseworkHand, CaseworkId, CaseworkLeg,
    CaseworkMember, CaseworkModule, CaseworkModuleKind, CaseworkPart, CaseworkRecord, CaseworkRun,
    CaseworkSide, CaseworkTie, DEFAULT_CASEWORK_DEPTH, DEFAULT_CASEWORK_MODULE, Entity, EntityBase,
    EntityId, InsertEntity, OpeningForm, PathEntity, RingGeometry, Vec2, casework_problem,
};
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::block::{Blocks, Definition};
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape};
use kentos_geometry_core::geom::casework::Frame;
use kentos_geometry_core::vec2::Vec2 as CoreVec2;

use crate::document::Document;
use crate::edit::Refusal;
use crate::history::Op;
use crate::identity::{Slot, Uuid, new_uid};
use crate::integrity::Fault;
use crate::store::Stored;

/// The undo step names of a run's own edits.
pub mod labels {
    pub const CASEWORK_RUN: &str = "Mutfak dizisi";
    pub const CASEWORK_EDIT: &str = "Mutfak dizisini düzenle";
    pub const CASEWORK_RELEASE: &str = "Mutfak dizisini bırak";
}

const SLACK: f64 = 1e-9;
/// The sink's back and start in from its module's; the cooktop's back from the line (docs/adr/0706 §1).
const SINK_INSET: f64 = 0.05;
const COOKTOP_INSET: f64 = 0.04;
/// The filler after a blind corner's cabinet (docs/adr/0727 §1).
const GAP: f64 = 0.05;
/// A filler narrower than this is none.
const NO_FILLER: f64 = 1e-3;
/// How near a leg's ends lie to a wall's face edge to stand on it.
const TIE: f64 = 1e-3;
/// A countertop's cut, in from its fixture's box.
const CUT_IN: f64 = 0.02;
/// A window whose sill is under the countertop clashes with it.
const COUNTER_HEIGHT: f64 = 0.90;
/// The countertop's depth's range, metres.
const DEPTHS: (f64, f64) = (0.30, 1.20);
const BASE: &str = "Alt dolap";
const SINK: &str = "Evye";
const COOKTOP: &str = "Ocak";
const UPPER: &str = "Üst dolap";
const WASHER: &str = "Bulaşık makinesi";
const OVEN: &str = "Fırın";
const FRIDGE: &str = "Buzdolabı";
/// The cabinets' width parameter.
const WIDTH: &str = "Genişlik";

/// Why a recipe lays no run: a command's refusal.
#[derive(Clone, Debug, PartialEq)]
pub struct CaseworkStop {
    pub code: &'static str,
    pub message: String,
    pub path: String,
}

/// A warning a laid run gives.
#[derive(Clone, Debug, PartialEq)]
pub struct CaseworkWarning {
    pub code: &'static str,
    pub message: String,
    pub path: String,
}

/// An object of a laid run with its part in it.
#[derive(Clone, Debug, PartialEq)]
pub struct LaidMember {
    pub part: CaseworkPart,
    pub index: Option<u32>,
    pub entity: Entity,
}

/// A run as its recipe lays it on the drawing (nothing written): its objects in order, the library blocks it brings,
/// its modules, legs, fridge and fillers, its legs' walls and its warnings.
#[derive(Clone, Debug)]
pub struct Laid {
    pub members: Vec<LaidMember>,
    pub added: Vec<BlockDefinition>,
    pub modules: Vec<CaseworkModule>,
    pub legs: Vec<CaseworkLeg>,
    pub fridge: Option<CaseworkFridge>,
    pub filler: f64,
    pub ties: Vec<Option<CaseworkTie>>,
    pub warnings: Vec<CaseworkWarning>,
}

fn invalid(words: String, path: impl Into<String>) -> CaseworkStop {
    CaseworkStop {
        code: codes::INVALID_CASEWORK,
        message: words,
        path: path.into(),
    }
}

fn fixed(v: f64, d: usize) -> String {
    kentos_geometry_core::display::fixed(v, d)
}

fn core_vec(p: Vec2) -> CoreVec2 {
    CoreVec2::new(p.x, p.y)
}

fn wire(p: CoreVec2) -> Vec2 {
    Vec2 { x: p.x, y: p.y }
}

/// The input's path of leg `j`'s end: a corner, or `to`.
fn leg_path(j: usize, n: usize) -> String {
    if j + 1 < n {
        format!("through[{j}]")
    } else {
        "to".into()
    }
}

/// The geometry core's reading of an object (the contract's own JSON).
fn core(e: &Entity) -> Option<CoreEntity> {
    let json = serde_json::to_string(e).ok()?;
    CoreEntity::from_json(&Json::parse(&json).ok()?).ok()
}

/// A definition as the core expands it: its pieces and its parameters' rules (no attributes: a box reads what it
/// draws).
fn core_definition(b: &BlockDefinition) -> Definition {
    let dynamic = b.dynamic.as_ref().and_then(|d| {
        let text = serde_json::to_string(d).ok()?;
        let json = Json::parse(&text).ok()?;
        Some(Arc::new(
            kentos_geometry_core::block_dynamic::Dynamic::from_json(&json),
        ))
    });
    Definition {
        id: b.id.to_text(),
        base: CoreVec2::new(b.base.x, b.base.y),
        entities: b.entities.iter().filter_map(core).collect(),
        attributes: Vec::new(),
        dynamic,
        underlay: None,
    }
}

/// The box of what `block` draws placed at the origin at scale 1, its parameters' defaults: `[x0, y0, x1, y1]`
/// relative to its base point; none for one that draws nothing.
fn block_box(blocks: &Blocks, block: BlockId, font: kentos_geometry_core::text::Font) -> Option<[f64; 4]> {
    let insert = Shape::Insert {
        params: None,
        block: block.to_text(),
        p: CoreVec2::new(0.0, 0.0),
        scale: 1.0,
        y_scale: None,
        rotation: 0.0,
        mirror: None,
        attrs: None,
        places: None,
        clip: None,
    };
    let mut out: Option<[f64; 4]> = None;
    for piece in blocks.expand(&insert) {
        if kentos_geometry_core::block::shows_nothing(&piece.shape) {
            continue;
        }
        let b = kentos_geometry_core::entity::entity_bounds_in(&piece.shape, font);
        if ![b.min_x, b.min_y, b.max_x, b.max_y].iter().all(|v| v.is_finite()) {
            continue;
        }
        out = Some(match out {
            None => [b.min_x, b.min_y, b.max_x, b.max_y],
            Some(a) => [a[0].min(b.min_x), a[1].min(b.min_y), a[2].max(b.max_x), a[3].max(b.max_y)],
        });
    }
    out
}

/// The width parameter of a cabinet block.
fn width_param(b: &BlockDefinition) -> Option<&BlockParameter> {
    b.dynamic.as_ref().and_then(|d| {
        d.parameters
            .iter()
            .find(|p| matches!(p, BlockParameter::Linear { name, .. } if name == WIDTH))
    })
}

/// A cabinet's parameters for `width` in its own numbers: none when it is the block's own.
fn width_values(p: &BlockParameter, width: f64) -> BTreeMap<String, ParamValue> {
    let v = ParamValue::Number(width);
    match p.default_value() {
        Some(d) if same_param_value(&d, &v) => BTreeMap::new(),
        _ => BTreeMap::from([(WIDTH.to_owned(), v)]),
    }
}

/// Why `width` is not one of a width parameter's values, in the ADR's words.
fn width_problem(p: &BlockParameter, width: f64) -> Option<String> {
    value_problem(p, &ParamValue::Number(width))
}

/// The point `a` along a frame's line and `b` into its side, as the contract's.
fn at(f: &Frame, a: f64, b: f64) -> Vec2 {
    wire(f.at(a, b))
}

/// The counter-clockwise rectangle `a`..`b` along a leg, `0`..`depth` into its side.
fn band(f: &Frame, a: f64, b: f64, depth: f64, left: bool) -> Vec<Vec2> {
    let mut out = vec![at(f, a, 0.0), at(f, b, 0.0), at(f, b, depth), at(f, a, depth)];
    if !left {
        out.reverse();
    }
    out
}

/// A hole in the countertop: `a`..`z` along a leg, `t0`..`t1` in from its line, turning against the countertop.
fn cut(f: &Frame, a: f64, z: f64, t0: f64, t1: f64, left: bool) -> Vec<Vec2> {
    let mut out = vec![at(f, a, t0), at(f, a, t1), at(f, z, t1), at(f, z, t0)];
    if !left {
        out.reverse();
    }
    out
}

fn path_entity(base: &EntityBase, pts: Vec<Vec2>, holes: Option<Vec<RingGeometry>>, area: bool) -> Entity {
    let path = PathEntity {
        base: base.clone(),
        pts,
        bulges: None,
        holes,
        zs: None,
        ms: None,
        parts: None,
        closed: !area,
        widths: None,
        space_separator: None,
    };
    if area {
        Entity::Polygon(path)
    } else {
        Entity::Polyline(path)
    }
}

fn ring(pts: Vec<Vec2>) -> RingGeometry {
    RingGeometry {
        pts,
        bulges: None,
        zs: None,
    }
}

/// A leg as its modules take it: the leg, its span's start and end, its modules (number, width) and their total.
type LegSpan = (usize, f64, f64, Vec<(u32, f64)>, f64);

/// What makes a laid run a record's objects: the ops, the members and their persistent ids and slots in plan order.
type MemberOps = (Vec<Op>, Vec<CaseworkMember>, Vec<(Uuid, Slot)>);

/// One module as laid: its number, leg, start along its leg and width.
#[derive(Clone, Copy, Debug)]
struct Placed {
    number: u32,
    leg: usize,
    start: f64,
    width: f64,
}

/// A wall's face lines, its left face first, its right face last: each the face's points (one per path vertex) and
/// whether each edge is straight; none for a wall whose faces are not one point a vertex.
fn wall_faces(e: &Entity) -> Option<[(Vec<CoreVec2>, Vec<bool>); 2]> {
    let shape = core(e)?.shape;
    let Shape::Wall { pts, bulges, wall } = &shape else {
        return None;
    };
    let lines = kentos_geometry_core::geom::wall::lines(pts, bulges.as_deref(), wall).ok()?;
    let n = pts.len();
    let straight: Vec<bool> = (0..n)
        .map(|i| bulges.as_ref().and_then(|b| b.get(i)).is_none_or(|b| *b == 0.0))
        .collect();
    let first = lines.first()?;
    let last = lines.last()?;
    if first.pts.len() != n || last.pts.len() != n {
        return None;
    }
    Some([
        (first.pts.clone(), straight.clone()),
        (last.pts.clone(), straight),
    ])
}

/// A wall's face edge `edge` on `face`, its two ends; none when it has no such straight edge.
fn face_edge(e: &Entity, face: CaseworkFace, edge: u32) -> Option<(CoreVec2, CoreVec2)> {
    let Entity::Wall(w) = e else { return None };
    let faces = wall_faces(e)?;
    let (pts, straight) = &faces[match face {
        CaseworkFace::Left => 0,
        CaseworkFace::Right => 1,
    }];
    let n = pts.len();
    let k = edge as usize;
    let edges = if w.wall.closed { n } else { n.saturating_sub(1) };
    if k >= edges || !straight[k] {
        return None;
    }
    Some((pts[k], pts[(k + 1) % n]))
}

/// `p`'s distance along a→b from a and its distance off the line; with the edge's length.
fn along(a: CoreVec2, b: CoreVec2, p: Vec2) -> Option<(f64, f64, f64)> {
    let f = Frame::of(a, b, true)?;
    let (dx, dy) = (p.x - a.x, p.y - a.y);
    Some((
        dx * f.along.x + dy * f.along.y,
        (dx * f.into.x + dy * f.into.y).abs(),
        f.length,
    ))
}

impl Document {
    /// The drawing's kitchen runs in the order they were made.
    pub fn casework(&self) -> &[Arc<CaseworkRecord>] {
        &self.casework
    }

    /// The run `key` names: its id, or the persistent id of one of its objects.
    pub fn casework_of(&self, key: &str) -> Option<&CaseworkRecord> {
        if let Some(id) = CaseworkId::parse(key)
            && let Some(r) = self.casework.iter().find(|r| r.id == id)
        {
            return Some(r);
        }
        let uid = EntityId::parse(key)?;
        self.casework
            .iter()
            .find(|r| r.members.iter().any(|m| m.uid == uid))
            .map(|r| &**r)
    }

    /// The run an object is part of, and its place in it.
    pub fn casework_member(&self, uid: Uuid) -> Option<(&CaseworkRecord, &CaseworkMember)> {
        let id = EntityId(uid.into_bytes());
        self.casework.iter().find_map(|r| {
            r.members
                .iter()
                .find(|m| m.uid == id)
                .map(|m| (&**r, m))
        })
    }

    /// Why `runs` cannot be the drawing's, or none: the contract's rules and every object an object of the drawing.
    pub(crate) fn casework_refused(&self, runs: &[&CaseworkRecord]) -> Option<String> {
        let owned: Vec<CaseworkRecord> = runs.iter().map(|r| (*r).clone()).collect();
        if let Some((_, why)) = casework_problem(&owned) {
            return Some(format!("Mutfak dizisi: {why}"));
        }
        None
    }

    /// The metres in one of a block's numbers (docs/adr/0315 §4): a unitless block's are the drawing's unit's.
    fn casework_factor(&self, b: &BlockDefinition) -> f64 {
        match b.units {
            Some(BlockUnits::Unitless) => {
                let (mul, div) = self.settings.drawing_unit.unwrap_or_default().metres();
                mul / div
            }
            _ => 1.0,
        }
    }

    /// The run `recipe` lays on this drawing: its blocks the drawing's by name, else `library`'s (brought anew, a
    /// taken id given `fresh`); without the line weight's, the revision's and the layer's checks (the command's).
    pub fn lay_casework(
        &self,
        recipe: &CaseworkRun,
        library: &dyn Fn(&str) -> Option<BlockDefinition>,
        fresh: &mut dyn FnMut() -> BlockId,
    ) -> Result<Laid, CaseworkStop> {
        let through = recipe.through.clone().unwrap_or_default();
        let pts = recipe.path();
        let mut names: Vec<(String, String)> = vec![("from".into(), "Dizinin başının".into())];
        names.extend((0..through.len()).map(|i| (format!("through[{i}]"), format!("{}. köşenin", i + 1))));
        names.push(("to".into(), "Dizinin sonunun".into()));
        for (p, (path, whose)) in pts.iter().zip(&names) {
            for (axis, name, v) in [("x", "doğu (Y)", p.x), ("y", "kuzey (X)", p.y)] {
                if !v.is_finite() {
                    return Err(CaseworkStop {
                        code: codes::NOT_FINITE,
                        message: format!(
                            "{whose} {name} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin."
                        ),
                        path: format!("{path}.{axis}"),
                    });
                }
            }
        }
        for (v, what, path) in [
            (recipe.module, "Modül eni", "module"),
            (recipe.depth, "Tezgâh derinliği", "depth"),
        ] {
            if let Some(x) = v
                && !x.is_finite()
            {
                return Err(CaseworkStop {
                    code: codes::NOT_FINITE,
                    message: format!("{what} sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin."),
                    path: path.into(),
                });
            }
        }
        let own = recipe.per_module.clone().unwrap_or_default();
        for (i, m) in own.iter().enumerate() {
            if let Some(w) = m.width
                && !w.is_finite()
            {
                return Err(CaseworkStop {
                    code: codes::NOT_FINITE,
                    message: "Modülün eni sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.".into(),
                    path: format!("perModule[{i}].width"),
                });
            }
        }
        let left = recipe.side.unwrap_or_default() == CaseworkSide::Left;
        if through.len() > kentos_contracts::CASEWORK_CORNERS_MAX {
            return Err(invalid(
                "Dizi en çok iki köşelidir (U mutfak); köşeleri azaltın.".into(),
                "through",
            ));
        }
        let n = pts.len() - 1;
        let mut frames = Vec::with_capacity(n);
        for j in 0..n {
            match Frame::of(core_vec(pts[j]), core_vec(pts[j + 1]), left) {
                Some(f) => frames.push(f),
                None if n == 1 => {
                    return Err(invalid(
                        "Dizinin başı ile sonu aynı nokta; dolapların sırtının dayandığı çizgiyi verin.".into(),
                        "to",
                    ));
                }
                None => {
                    return Err(invalid(
                        format!("{}. kolun başı ile sonu aynı nokta; kolun çizgisini verin.", j + 1),
                        leg_path(j, n),
                    ));
                }
            }
        }
        let module = recipe.module.unwrap_or(DEFAULT_CASEWORK_MODULE);
        let depth = recipe.depth.unwrap_or(DEFAULT_CASEWORK_DEPTH);
        if !(DEPTHS.0 - 1e-9..=DEPTHS.1 + 1e-9).contains(&depth) {
            return Err(invalid(
                format!("Tezgâh derinliği {depth} m; 0,30 ile 1,20 m arasında olmalı."),
                "depth",
            ));
        }
        for k in 1..n {
            let (a, b) = (frames[k - 1].along, frames[k].along);
            let dot = a.x * b.x + a.y * b.y;
            let cross = a.x * b.y - a.y * b.x;
            if dot.abs() > 1e-6 {
                return Err(invalid(
                    format!("{k}. köşe dik değil; L ve U dizinin kolları dik birleşir."),
                    format!("through[{}]", k - 1),
                ));
            }
            if (cross > 0.0) != left {
                return Err(invalid(
                    format!("{k}. köşe dolapların öbür yanına dönüyor; köşe dolapların durduğu yana dönmeli."),
                    format!("through[{}]", k - 1),
                ));
            }
        }
        let mut seen = HashSet::new();
        for (i, m) in own.iter().enumerate() {
            if m.module < 1 {
                return Err(invalid(
                    "Modül numarası 1'den başlar.".into(),
                    format!("perModule[{i}].module"),
                ));
            }
            if !seen.insert(m.module) {
                return Err(invalid(
                    format!("{}. modülün ayarı iki kez verildi.", m.module),
                    format!("perModule[{i}].module"),
                ));
            }
        }
        let mut added: Vec<BlockDefinition> = Vec::new();
        let mut definition = |name: &str, added: &mut Vec<BlockDefinition>| -> Option<BlockDefinition> {
            if let Some(b) = self.block_named(name) {
                return Some(b.clone());
            }
            if let Some(b) = added.iter().find(|b| b.name == name) {
                return Some(b.clone());
            }
            let mut b = library(name)?;
            if self.block(b.id).is_some() || added.iter().any(|a| a.id == b.id) {
                b.id = fresh();
            }
            added.push(b.clone());
            Some(b)
        };
        let missing = |name: &str| invalid(format!("“{name}” bloğu ne çizimde ne KentOS kitaplığında var."), "layerId");
        let cabinet = definition(BASE, &mut added).ok_or_else(|| missing(BASE))?;
        let Some(width) = width_param(&cabinet).cloned() else {
            return Err(invalid(
                format!("“{BASE}” bloğunun “{WIDTH}” parametresi yok; dizi modülün enini bu parametreyle verir."),
                "module",
            ));
        };
        // The width parameter's rule is in the block's numbers (a unitless block's the drawing's unit's).
        let cab_factor = self.casework_factor(&cabinet);
        if let Some(why) = width_problem(&width, module / cab_factor) {
            return Err(invalid(
                format!("Modül eni {module} m; “{BASE}” bloğunun eni için {why}."),
                "module",
            ));
        }
        let font = crate::linked::font_of(self.settings.drawing_font);
        let blocks_now = |added: &[BlockDefinition]| {
            let mut defs: Vec<Definition> = self.blocks.iter().map(|b| core_definition(b)).collect();
            defs.extend(added.iter().map(core_definition));
            Blocks::new(defs)
        };
        let cab_depth = {
            let blocks = blocks_now(&added);
            block_box(&blocks, cabinet.id, font).map_or(DEFAULT_CASEWORK_DEPTH, |b| b[3] * cab_factor)
        };
        let (sink, cooktop, dishwasher) = (recipe.sink, recipe.cooktop, recipe.dishwasher);
        let oven = recipe.oven == Some(true);
        let fridge = recipe.fridge;
        let corner = recipe.corner.unwrap_or_default();
        let given: HashMap<u32, &kentos_contracts::CaseworkModuleSetting> =
            own.iter().map(|m| (m.module, m)).collect();
        for (i, m) in own.iter().enumerate() {
            if let Some(w) = m.width
                && let Some(why) = width_problem(&width, w / cab_factor)
            {
                return Err(invalid(
                    format!("{}. modülün eni {w} m; “{BASE}” bloğunun eni için {why}.", m.module),
                    format!("perModule[{i}].width"),
                ));
            }
        }
        let width_of = |i: u32| -> f64 {
            if let Some(w) = given.get(&i).and_then(|m| m.width) {
                return w;
            }
            if Some(i) == dishwasher || (oven && Some(i) == cooktop) {
                return CASEWORK_APPLIANCE;
            }
            if Some(i) == sink {
                return CASEWORK_SINK_MODULE;
            }
            if Some(i) == cooktop {
                return module.max(CASEWORK_APPLIANCE);
            }
            module
        };
        for (k, whose, w, path) in [
            (sink, "Evye", CASEWORK_SINK_MODULE, "sink"),
            (cooktop, "Ocak", module.max(CASEWORK_APPLIANCE), "cooktop"),
        ] {
            let Some(k) = k else { continue };
            let own_width = given.get(&k).and_then(|m| m.width).is_some();
            if own_width || (path == "cooktop" && oven) {
                continue;
            }
            if let Some(why) = width_problem(&width, w / cab_factor) {
                return Err(invalid(
                    format!("{whose} modülü {w} m; “{BASE}” bloğunun eni için {why}."),
                    path,
                ));
            }
        }
        let fridge_span = CASEWORK_APPLIANCE;
        // Each leg's span and the modules it holds, numbered on along the path.
        let mut spans: Vec<LegSpan> = Vec::with_capacity(n);
        let mut index = 1u32;
        for (j, f) in frames.iter().enumerate() {
            let s0 = if j == 0 {
                if fridge == Some(CaseworkEnd::Start) { fridge_span } else { 0.0 }
            } else if corner == CaseworkCorner::Blind {
                cab_depth + GAP
            } else {
                cab_depth
            };
            let s1 = if j + 1 == n {
                f.length - if fridge == Some(CaseworkEnd::End) { fridge_span } else { 0.0 }
            } else if corner == CaseworkCorner::Blind {
                f.length
            } else {
                f.length - cab_depth
            };
            let usable = s1 - s0;
            let mut laid = Vec::new();
            let mut total = 0.0;
            loop {
                let w = width_of(index);
                if total + w > usable + SLACK {
                    break;
                }
                laid.push((index, w));
                total += w;
                index += 1;
            }
            if laid.is_empty() {
                let need = width_of(index);
                if n == 1 {
                    if fridge.is_none() {
                        return Err(invalid(
                            format!(
                                "Dizinin çizgisi {} m; bir modüle ({} m) yetmiyor. Çizgiyi uzatın ya da modülü daraltın.",
                                fixed(f.length, 3),
                                fixed(need, 2)
                            ),
                            "to",
                        ));
                    }
                    return Err(invalid(
                        format!(
                            "Buzdolabından sonra kalan {} m bir modüle ({} m) yetmiyor.",
                            fixed(usable, 3),
                            fixed(need, 2)
                        ),
                        "to",
                    ));
                }
                return Err(invalid(
                    format!(
                        "{}. kolda köşeden ve uçtan sonra kalan {} m bir modüle ({} m) yetmiyor.",
                        j + 1,
                        fixed(usable, 3),
                        fixed(need, 2)
                    ),
                    leg_path(j, n),
                ));
            }
            spans.push((j, s0, s1, laid, total));
        }
        let count = index - 1;
        for (k, whose, path) in [
            (sink, "Evye", "sink"),
            (cooktop, "Ocak", "cooktop"),
            (dishwasher, "Bulaşık makinesi", "dishwasher"),
        ] {
            if let Some(k) = k
                && !(1..=count).contains(&k)
            {
                return Err(invalid(
                    format!("{whose} {k}. modülde; dizi {count} modül (1 ile {count} arası)."),
                    path,
                ));
            }
        }
        for (i, m) in own.iter().enumerate() {
            if m.module > count {
                return Err(invalid(
                    format!("{}. modülün ayarı verildi; dizi {count} modül.", m.module),
                    format!("perModule[{i}].module"),
                ));
            }
        }
        if sink.is_some() && sink == cooktop {
            return Err(invalid(
                "Evye ve ocak aynı modülde olamaz; ocağı başka bir modüle koyun.".into(),
                "cooktop",
            ));
        }
        if dishwasher.is_some() && (dishwasher == sink || dishwasher == cooktop) {
            return Err(invalid(
                "Bulaşık makinesi evyenin ya da ocağın modülüne konamaz; başka bir modül seçin.".into(),
                "dishwasher",
            ));
        }
        if oven && cooktop.is_none() {
            return Err(invalid(
                "Fırın ocağın altına konur; önce ocağın modülünü verin.".into(),
                "oven",
            ));
        }
        for (i, m) in own.iter().enumerate() {
            let k = m.module;
            let appliance = Some(k) == dishwasher || (oven && Some(k) == cooktop);
            if appliance
                && let Some(w) = m.width
                && (w - CASEWORK_APPLIANCE).abs() > SLACK
            {
                let whose = if Some(k) == dishwasher { WASHER } else { OVEN };
                return Err(invalid(
                    format!("{k}. modül {whose}; eni 0,60 m'dir."),
                    format!("perModule[{i}].width"),
                ));
            }
            if m.hand.is_some() {
                if appliance {
                    return Err(invalid(
                        format!("{k}. modül bir cihaz; açılması verilmez."),
                        format!("perModule[{i}].hand"),
                    ));
                }
                if width_of(k) > CASEWORK_APPLIANCE + SLACK {
                    return Err(invalid(
                        format!("{k}. modül çift kapılı ({} m); açılması verilmez.", width_of(k)),
                        format!("perModule[{i}].hand"),
                    ));
                }
            }
        }
        let mut placed: Vec<Placed> = Vec::with_capacity(count as usize);
        let mut fills: Vec<(usize, f64, f64, usize)> = Vec::new();
        let mut legs = Vec::with_capacity(n);
        for (j, s0, s1, laid, total) in &spans {
            let back = *j == 0 && n > 1;
            let mut x = if back { s1 - total } else { *s0 };
            let gap = if back { (*s0, s1 - total) } else { (s0 + total, *s1) };
            for &(number, w) in laid {
                placed.push(Placed {
                    number,
                    leg: *j,
                    start: x,
                    width: w,
                });
                x += w;
            }
            let leftover = gap.1 - gap.0;
            legs.push(CaseworkLeg {
                length: frames[*j].length,
                start: *s0,
                end: *s1,
                filler: if leftover > NO_FILLER { leftover } else { 0.0 },
                wall: None,
            });
            if leftover > NO_FILLER {
                fills.push((*j, gap.0, gap.1, laid.len()));
            }
        }
        let by_number: HashMap<u32, Placed> = placed.iter().map(|p| (p.number, *p)).collect();
        // The sink and the cooktop on their modules and the countertop (§2, §3).
        struct Fixture {
            part: CaseworkPart,
            block: BlockDefinition,
            factor: f64,
            leg: usize,
            at: (f64, f64),
            footprint: (f64, f64, f64, f64),
        }
        let mut fixtures: Vec<Fixture> = Vec::new();
        for (k, name, whose, part, path) in [
            (sink, SINK, "Evye", CaseworkPart::Sink, "sink"),
            (cooktop, COOKTOP, "Ocak", CaseworkPart::Cooktop, "cooktop"),
        ] {
            let Some(k) = k else { continue };
            let block = definition(name, &mut added).ok_or_else(|| missing(name))?;
            let factor = self.casework_factor(&block);
            let blocks = blocks_now(&added);
            let bx = block_box(&blocks, block.id, font).unwrap_or([0.0; 4]);
            let p = by_number[&k];
            let at = if part == CaseworkPart::Sink {
                (p.start + SINK_INSET, SINK_INSET)
            } else {
                (p.start + p.width / 2.0, COOKTOP_INSET)
            };
            let (a0, a1) = (at.0 + bx[0] * factor, at.0 + bx[2] * factor);
            let (t0, t1) = (at.1 + bx[1] * factor, at.1 + bx[3] * factor);
            if a0 < p.start - SLACK || a1 > p.start + p.width + SLACK {
                return Err(invalid(
                    format!(
                        "{whose} ({} m) {k}. modüle ({} m) sığmıyor; modülün enini büyütün.",
                        fixed(a1 - a0, 2),
                        fixed(p.width, 2)
                    ),
                    path,
                ));
            }
            if t0 < -SLACK || t1 > depth + SLACK {
                return Err(invalid(
                    format!(
                        "{whose} {} m derinlik ister; tezgâh {} m. Tezgâhı derinleştirin.",
                        fixed(t1, 2),
                        fixed(depth, 2)
                    ),
                    "depth",
                ));
            }
            fixtures.push(Fixture {
                part,
                block,
                factor,
                leg: p.leg,
                at,
                footprint: (a0, a1, t0, t1),
            });
        }
        let upper = match recipe.upper {
            Some(true) => {
                let b = definition(UPPER, &mut added).ok_or_else(|| missing(UPPER))?;
                let Some(p) = width_param(&b).cloned() else {
                    return Err(invalid(
                        format!("“{UPPER}” bloğunun “{WIDTH}” parametresi yok; dizi modülün enini bu parametreyle verir."),
                        "upper",
                    ));
                };
                let mut widths: Vec<f64> = Vec::new();
                for q in &placed {
                    if !widths.contains(&q.width) {
                        widths.push(q.width);
                    }
                }
                let factor = self.casework_factor(&b);
                for w in widths {
                    if let Some(why) = width_problem(&p, w / factor) {
                        return Err(invalid(
                            format!("Üst dolap {w} m; “{UPPER}” bloğunun eni için {why}."),
                            "upper",
                        ));
                    }
                }
                Some((b, p))
            }
            _ => None,
        };
        let washer = match dishwasher {
            Some(_) => Some(definition(WASHER, &mut added).ok_or_else(|| missing(WASHER))?),
            None => None,
        };
        let oven_block = if oven {
            Some(definition(OVEN, &mut added).ok_or_else(|| missing(OVEN))?)
        } else {
            None
        };
        let fridge_block = match fridge {
            Some(_) => Some(definition(FRIDGE, &mut added).ok_or_else(|| missing(FRIDGE))?),
            None => None,
        };

        let base = EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: recipe.layer_id.clone(),
            color: recipe.color.clone(),
            attrs: BTreeMap::new(),
            label: None,
            symbol: None,
            line_weight: recipe.line_weight,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        };
        let insert = |b: &BlockDefinition,
                      p: Vec2,
                      f: &Frame,
                      parameters: BTreeMap<String, ParamValue>,
                      attrs: BTreeMap<String, String>,
                      factor: f64| {
            let mut base = base.clone();
            base.attrs = attrs;
            Entity::Insert(InsertEntity {
                arch: Default::default(),
                base,
                block: b.id,
                p,
                scale: factor,
                y_scale: None,
                rotation: f.rotation(),
                mirror: !left,
                attribute_places: BTreeMap::new(),
                parameters,
                clipping: None,
            })
        };
        // A blind corner's leg runs into the corner: the next leg's cabinets stand before the last cabinet depth of it,
        // so much of a module's front (§1).
        let blind_by = |q: &Placed| -> f64 {
            if corner != CaseworkCorner::Blind || q.leg + 1 >= n {
                return 0.0;
            }
            let end = frames[q.leg].length;
            (q.start + q.width).min(end) - q.start.max(end - cab_depth)
        };
        let mut members: Vec<LaidMember> = Vec::new();
        let mut modules = Vec::with_capacity(placed.len());
        for q in &placed {
            let f = &frames[q.leg];
            let is_oven = oven && Some(q.number) == cooktop;
            let (kind, block, hand) = if let (true, Some(b)) = (Some(q.number) == dishwasher, &washer) {
                (CaseworkModuleKind::Dishwasher, b, None)
            } else if let (true, Some(b)) = (is_oven, &oven_block) {
                (CaseworkModuleKind::Cooktop, b, None)
            } else {
                let kind = if Some(q.number) == sink {
                    CaseworkModuleKind::Sink
                } else if Some(q.number) == cooktop {
                    CaseworkModuleKind::Cooktop
                } else {
                    CaseworkModuleKind::Base
                };
                let hand = if q.width > CASEWORK_APPLIANCE + SLACK {
                    CaseworkHand::Double
                } else {
                    match given.get(&q.number).and_then(|m| m.hand) {
                        Some(kentos_contracts::CaseworkDoor::Right) => CaseworkHand::Right,
                        _ => CaseworkHand::Left,
                    }
                };
                (kind, &cabinet, Some(hand))
            };
            let word = if is_oven {
                "Fırın"
            } else {
                match kind {
                    CaseworkModuleKind::Base => "Alt dolap",
                    CaseworkModuleKind::Sink => "Evye dolabı",
                    CaseworkModuleKind::Cooktop => "Ocak dolabı",
                    CaseworkModuleKind::Dishwasher => "Bulaşık makinesi",
                }
            };
            let mut attrs = BTreeMap::from([
                ("Modül".to_owned(), q.number.to_string()),
                ("Tür".to_owned(), word.to_owned()),
            ]);
            if let Some(h) = hand {
                attrs.insert(
                    "Açılma".to_owned(),
                    match h {
                        CaseworkHand::Left => "Sol",
                        CaseworkHand::Right => "Sağ",
                        CaseworkHand::Double => "Çift",
                    }
                    .to_owned(),
                );
            }
            let factor = self.casework_factor(block);
            // A cabinet stands from its module's start, an appliance on its middle (its base point its back's).
            let entity = if hand.is_some() {
                insert(block, at(f, q.start, 0.0), f, width_values(&width, q.width / factor), attrs, factor)
            } else {
                insert(block, at(f, q.start + q.width / 2.0, 0.0), f, BTreeMap::new(), attrs, factor)
            };
            members.push(LaidMember {
                part: CaseworkPart::Module,
                index: Some(q.number),
                entity,
            });
            modules.push(CaseworkModule {
                number: q.number,
                kind,
                start: q.start,
                width: q.width,
                leg: (q.leg > 0).then_some(q.leg as u32),
                hand,
                oven: is_oven.then_some(true),
                blind: (blind_by(q) > SLACK).then_some(true),
            });
        }
        let mut holes = Vec::new();
        for fx in &fixtures {
            let f = &frames[fx.leg];
            members.push(LaidMember {
                part: fx.part,
                index: None,
                entity: insert(&fx.block, at(f, fx.at.0, fx.at.1), f, BTreeMap::new(), BTreeMap::new(), fx.factor),
            });
            let (a0, a1, t0, t1) = fx.footprint;
            holes.push(ring(cut(f, a0 + CUT_IN, a1 - CUT_IN, t0 + CUT_IN, t1 - CUT_IN, left)));
        }
        let mut fridge_out = None;
        if let (Some(end), Some(b)) = (fridge, &fridge_block) {
            let j = if end == CaseworkEnd::Start { 0 } else { n - 1 };
            let f = &frames[j];
            let at_s = if end == CaseworkEnd::Start {
                fridge_span / 2.0
            } else {
                f.length - fridge_span / 2.0
            };
            members.push(LaidMember {
                part: CaseworkPart::Fridge,
                index: None,
                entity: insert(
                    b,
                    at(f, at_s, 0.0),
                    f,
                    BTreeMap::new(),
                    BTreeMap::from([("Tür".to_owned(), "Buzdolabı".to_owned())]),
                    self.casework_factor(b),
                ),
            });
            fridge_out = Some(CaseworkFridge {
                start: at_s - fridge_span / 2.0,
                width: fridge_span,
                leg: (j > 0).then_some(j as u32),
            });
        }
        if let Some((b, p)) = &upper {
            let factor = self.casework_factor(b);
            for q in &placed {
                let f = &frames[q.leg];
                members.push(LaidMember {
                    part: CaseworkPart::Upper,
                    index: Some(q.number),
                    entity: insert(
                        b,
                        at(f, q.start, 0.0),
                        f,
                        width_values(p, q.width / factor),
                        BTreeMap::from([
                            ("Modül".to_owned(), q.number.to_string()),
                            ("Tür".to_owned(), "Üst dolap".to_owned()),
                        ]),
                        factor,
                    ),
                });
            }
        }
        let mut warnings = Vec::new();
        let mut filler = 0.0;
        for &(j, a, z, laid) in &fills {
            members.push(LaidMember {
                part: CaseworkPart::Filler,
                index: Some(j as u32),
                entity: path_entity(&base, band(&frames[j], a, z, cab_depth, left), None, false),
            });
            let leftover = z - a;
            filler += leftover;
            let (message, path) = if n == 1 {
                (
                    format!("{laid} modülden sonra dizinin sonunda {} m dolgu kalıyor.", fixed(leftover, 3)),
                    "to".to_owned(),
                )
            } else {
                (
                    format!("{}. kolda {} m dolgu kalıyor.", j + 1, fixed(leftover, 3)),
                    if j == 0 { "from".to_owned() } else { leg_path(j, n) },
                )
            };
            warnings.push(CaseworkWarning {
                code: codes::CASEWORK_FILLER,
                message,
                path,
            });
        }
        // A sink, a cooktop or a dishwasher in a blind corner: what it needs before it is the next leg's (§1).
        for (k, what, path) in [
            (sink, "evyenin önünde durulamaz", "sink"),
            (
                cooktop,
                if oven {
                    "ocağın önünde durulamaz, fırının kapağı açılamaz"
                } else {
                    "ocağın önünde durulamaz"
                },
                "cooktop",
            ),
            (dishwasher, "bulaşık makinesinin kapağı açılamaz", "dishwasher"),
        ] {
            let Some(q) = k.and_then(|k| by_number.get(&k)) else { continue };
            let over = blind_by(q);
            if over > SLACK {
                warnings.push(CaseworkWarning {
                    code: codes::CASEWORK_BLIND_CORNER,
                    message: format!(
                        "{}. modül kör köşede: önünün {} m'sini {}. kolun dolapları kapatıyor; {what}. Başka bir modül seçin ya da köşeyi açık yapın.",
                        q.number,
                        fixed(over, 3),
                        q.leg + 2
                    ),
                    path: path.into(),
                });
            }
        }
        for (k, f) in frames.iter().enumerate().skip(1) {
            let pts = if corner == CaseworkCorner::Blind {
                band(f, cab_depth, cab_depth + GAP, cab_depth, left)
            } else {
                band(f, 0.0, cab_depth, cab_depth, left)
            };
            members.push(LaidMember {
                part: CaseworkPart::Corner,
                index: Some(k as u32),
                entity: path_entity(&base, pts, None, false),
            });
        }
        let cs = if fridge == Some(CaseworkEnd::Start) { fridge_span } else { 0.0 };
        let last = &frames[n - 1];
        let ce = last.length - if fridge == Some(CaseworkEnd::End) { fridge_span } else { 0.0 };
        let mut outer = vec![at(&frames[0], cs, 0.0)];
        outer.extend(pts[1..n].iter().copied());
        outer.push(at(last, ce, 0.0));
        let mut inner = vec![at(last, ce, depth)];
        for k in (1..n).rev() {
            let (na, nb) = (frames[k - 1].into, frames[k].into);
            let q = pts[k];
            inner.push(Vec2 {
                x: q.x + na.x * depth + nb.x * depth,
                y: q.y + na.y * depth + nb.y * depth,
            });
        }
        inner.push(at(&frames[0], cs, depth));
        let mut counter: Vec<Vec2> = outer;
        counter.extend(inner);
        if !left {
            counter.reverse();
        }
        members.push(LaidMember {
            part: CaseworkPart::Counter,
            index: None,
            entity: path_entity(&base, counter, (!holes.is_empty()).then_some(holes), true),
        });
        let ties: Vec<Option<CaseworkTie>> = (0..n).map(|j| self.casework_tie(pts[j], pts[j + 1])).collect();
        for (j, tie) in ties.iter().enumerate() {
            let Some(tie) = tie else { continue };
            legs[j].wall = Some(tie.wall.to_text());
            warnings.extend(self.casework_clashes(tie.wall, &frames[j], j, n));
        }
        // Its modules stand on their wall's level (docs/adr/0733 §1): the first wall's.
        let level = ties.iter().flatten().find_map(|t| {
            let slot = crate::identity::Uuid::parse_str(&t.wall.to_text())
                .ok()
                .and_then(|u| self.slot_of(u))?;
            self.get(slot)?.own_level().map(str::to_owned)
        });
        if let Some(level) = level {
            for m in &mut members {
                if let Entity::Insert(i) = &mut m.entity {
                    i.arch.level = Some(level.clone());
                }
            }
        }
        Ok(Laid {
            members,
            added,
            modules,
            legs,
            fridge: fridge_out,
            filler,
            ties,
            warnings,
        })
    }

    /// The wall face edge a leg from `a` to `b` stands on: the first wall's (document order), its left face before
    /// its right; none.
    fn casework_tie(&self, a: Vec2, b: Vec2) -> Option<CaseworkTie> {
        for s in self.store.iter() {
            let Entity::Wall(w) = &*s.entity else { continue };
            let Some(faces) = wall_faces(&s.entity) else { continue };
            for (face, (pts, straight)) in [CaseworkFace::Left, CaseworkFace::Right].into_iter().zip(&faces) {
                let n = pts.len();
                let edges = if w.wall.closed { n } else { n.saturating_sub(1) };
                for k in 0..edges {
                    if !straight[k] {
                        continue;
                    }
                    let (p, q) = (pts[k], pts[(k + 1) % n]);
                    let (Some((s0, d0, length)), Some((s1, d1, _))) = (along(p, q, a), along(p, q, b)) else {
                        continue;
                    };
                    let within = |s: f64| (-TIE..=length + TIE).contains(&s);
                    if d0 <= TIE && d1 <= TIE && within(s0) && within(s1) {
                        return Some(CaseworkTie {
                            wall: EntityId(s.uid.into_bytes()),
                            edge: k as u32,
                            face,
                        });
                    }
                }
            }
        }
        None
    }

    /// The leg `j` of `n` on `wall` in front of its doors, passages and low windows (docs/adr/0727 §2).
    fn casework_clashes(&self, wall: EntityId, f: &Frame, j: usize, n: usize) -> Vec<CaseworkWarning> {
        let host = wall.to_text();
        let mut out = Vec::new();
        for s in self.store.iter() {
            let Entity::Opening(o) = &*s.entity else { continue };
            let o = &o.opening;
            if o.host != host || o.place.len() != 4 {
                continue;
            }
            let low = o.form == OpeningForm::Window && o.sill.is_some_and(|h| h < COUNTER_HEIGHT - SLACK);
            if !matches!(o.form, OpeningForm::Door | OpeningForm::Opening) && !low {
                continue;
            }
            let along = |p: Vec2| (p.x - f.from.x) * f.along.x + (p.y - f.from.y) * f.along.y;
            let (x0, x1) = (along(o.place[0]), along(o.place[2]));
            let (lo, hi) = (x0.min(x1), x0.max(x1));
            let over = hi.min(f.length) - lo.max(0.0);
            if over <= NO_FILLER {
                continue;
            }
            let mark = o.number.clone().unwrap_or_else(|| "numarasız".into());
            let message = if o.form == OpeningForm::Window {
                format!(
                    "{}. kol {mark} penceresinin önünde ({} m); denizliği tezgâhtan alçak ({} m).",
                    j + 1,
                    fixed(over, 3),
                    fixed(o.sill.unwrap_or_default(), 2)
                )
            } else {
                let what = if o.form == OpeningForm::Door { "kapısının" } else { "boşluğunun" };
                format!("{}. kol {mark} {what} önüne geliyor ({} m).", j + 1, fixed(over, 3))
            };
            out.push(CaseworkWarning {
                code: codes::CASEWORK_CLASH,
                message,
                path: if n > 1 { leg_path(j, n) } else { "to".into() },
            });
        }
        out
    }

    /// The ops that make `laid` the objects of `record` (its old objects kept by their parts, the others removed, new
    /// ones added in plan order) and the record they leave; with the persistent ids in plan order.
    pub(crate) fn casework_member_ops(
        &mut self,
        record: Option<&CaseworkRecord>,
        laid: &Laid,
    ) -> Result<MemberOps, Refusal> {
        let old: HashMap<(CaseworkPart, Option<u32>), EntityId> = record
            .map(|r| r.members.iter().map(|m| ((m.part, m.index), m.uid)).collect())
            .unwrap_or_default();
        let keep: HashSet<(CaseworkPart, Option<u32>)> =
            laid.members.iter().map(|m| (m.part, m.index)).collect();
        let fresh = laid
            .members
            .iter()
            .filter(|m| {
                old.get(&(m.part, m.index))
                    .and_then(|u| self.slot_of(Uuid::from_bytes(u.0)))
                    .is_none()
            })
            .count();
        if self.next_slot + fresh as u64 > u64::from(u32::MAX) + 1 {
            return Err(Refusal(crate::edit::SlotsExhausted.to_string()));
        }
        let mut ops = Vec::new();
        if let Some(r) = record {
            for m in &r.members {
                if !keep.contains(&(m.part, m.index))
                    && let Some(stored) = self
                        .slot_of(Uuid::from_bytes(m.uid.0))
                        .and_then(|slot| self.store.get(slot))
                {
                    ops.push(Op::Remove(stored.clone()));
                }
            }
        }
        let mut members = Vec::with_capacity(laid.members.len());
        let mut written = Vec::with_capacity(laid.members.len());
        for m in &laid.members {
            let before = old
                .get(&(m.part, m.index))
                .and_then(|u| self.slot_of(Uuid::from_bytes(u.0)))
                .and_then(|slot| self.store.get(slot))
                .cloned();
            let (uid, slot) = match before {
                Some(before) => {
                    let mut entity = m.entity.clone();
                    // What the run does not own stays the object's own (docs/adr/0727 §5).
                    let was = before.entity.base();
                    let base = entity.base_mut();
                    base.id = before.slot().0;
                    base.label = was.label.clone();
                    base.symbol = was.symbol.clone();
                    base.look = was.look.clone();
                    base.label_pins = was.label_pins.clone();
                    base.attachments = was.attachments.clone();
                    for (k, v) in &was.attrs {
                        if !matches!(k.as_str(), "Modül" | "Tür" | "Açılma") {
                            base.attrs.entry(k.clone()).or_insert_with(|| v.clone());
                        }
                    }
                    let after = Stored {
                        uid: before.uid,
                        entity: Arc::new(entity),
                    };
                    let (uid, slot) = (before.uid, before.slot());
                    if before.entity != after.entity {
                        ops.push(Op::Update { before, after });
                    }
                    (uid, slot)
                }
                None => {
                    let slot = Slot(self.next_slot as u32);
                    self.next_slot += 1;
                    let uid = new_uid();
                    let mut entity = m.entity.clone();
                    entity.base_mut().id = slot.0;
                    ops.push(Op::Add(Stored {
                        uid,
                        entity: Arc::new(entity),
                    }));
                    (uid, slot)
                }
            };
            members.push(CaseworkMember {
                uid: EntityId(uid.into_bytes()),
                part: m.part,
                index: m.index,
            });
            written.push((uid, slot));
        }
        Ok((ops, members, written))
    }

    /// Writes a laid run: the blocks it brings, its objects and its record (a new one, or `record` laid anew), in the
    /// open transaction or as one undo step named `label`; its persistent ids and slots in plan order.
    pub fn write_casework(
        &mut self,
        record: Option<&CaseworkRecord>,
        recipe: CaseworkRun,
        laid: &Laid,
        label: &str,
    ) -> Result<(CaseworkId, Vec<(Uuid, Slot)>), Refusal> {
        for b in &laid.added {
            self.add_block(b.clone())?;
        }
        let (mut ops, members, written) = self.casework_member_ops(record, laid)?;
        let next = CaseworkRecord {
            id: record.map_or_else(|| CaseworkId(new_uid().into_bytes()), |r| r.id),
            recipe: CaseworkRun {
                expected_revision: None,
                ..recipe
            },
            ties: laid.ties.clone(),
            members,
            broken: None,
        };
        let mut list: Vec<&CaseworkRecord> = self.casework.iter().map(|r| &**r).collect();
        match record {
            Some(r) => {
                if let Some(at) = list.iter().position(|x| x.id == r.id) {
                    list[at] = &next;
                }
            }
            None => list.push(&next),
        }
        if let Some(why) = self.casework_refused(&list) {
            return Err(Refusal(why));
        }
        let id = next.id;
        ops.push(match record {
            Some(r) => Op::CaseworkUpdate {
                before: Arc::new(r.clone()),
                after: Arc::new(next),
            },
            None => Op::CaseworkAdd {
                index: self.casework.len(),
                run: Arc::new(next),
            },
        });
        self.record(ops, label);
        Ok((id, written))
    }

    /// Releases a run: its record goes, its objects stay as they are, as one undo step “Mutfak dizisini bırak”;
    /// `false` when no run has the id.
    pub fn release_casework(&mut self, id: CaseworkId) -> bool {
        let Some(index) = self.casework.iter().position(|r| r.id == id) else {
            return false;
        };
        let op = Op::CaseworkRemove {
            index,
            run: self.casework[index].clone(),
        };
        self.record(vec![op], labels::CASEWORK_RELEASE);
        true
    }

    /// Applies a run op (history.rs); the runs are found by id.
    pub(crate) fn apply_casework_op(&mut self, op: &Op) {
        match op {
            Op::CaseworkAdd { index, run } => {
                let at = (*index).min(self.casework.len());
                self.casework.insert(at, run.clone());
            }
            Op::CaseworkRemove { run, .. } => self.casework.retain(|r| r.id != run.id),
            Op::CaseworkUpdate { after, .. } => {
                if let Some(slot) = self.casework.iter_mut().find(|r| r.id == after.id) {
                    *slot = after.clone();
                }
            }
            _ => {}
        }
    }

    /// What keeps the runs with their walls and objects after `ops` (a step about to be recorded), applied: a run whose
    /// every object the step removed goes with them; a run whose tied wall the step changed is laid anew on the path
    /// its walls give, its objects kept by their parts; a run whose wall is gone, or whose new path lays no run,
    /// breaks and keeps its objects.
    pub(crate) fn follow_casework(&mut self, ops: &[Op]) -> Vec<Op> {
        if self.casework.is_empty() {
            return Vec::new();
        }
        let own: HashSet<CaseworkId> = ops
            .iter()
            .filter_map(|op| match op {
                Op::CaseworkAdd { run, .. } | Op::CaseworkRemove { run, .. } => Some(run.id),
                Op::CaseworkUpdate { after, .. } => Some(after.id),
                _ => None,
            })
            .collect();
        let mut removed: HashSet<EntityId> = HashSet::new();
        // Each changed object before the step's first write and after its last.
        let mut updated: HashMap<EntityId, (Arc<Entity>, Arc<Entity>)> = HashMap::new();
        // Each changed wall before the step's first write and after its last; none after: removed.
        let mut walls: HashMap<EntityId, (Arc<Entity>, Option<Arc<Entity>>)> = HashMap::new();
        for op in ops {
            match op {
                Op::Remove(s) => {
                    let id = EntityId(s.uid.into_bytes());
                    removed.insert(id);
                    if matches!(&*s.entity, Entity::Wall(_)) {
                        walls
                            .entry(id)
                            .and_modify(|(_, after)| *after = None)
                            .or_insert_with(|| (s.entity.clone(), None));
                    }
                }
                Op::Add(s) => {
                    let id = EntityId(s.uid.into_bytes());
                    removed.remove(&id);
                    if matches!(&*s.entity, Entity::Wall(_))
                        && let Some((_, after)) = walls.get_mut(&id)
                    {
                        *after = Some(s.entity.clone());
                    }
                }
                Op::Update { before, after } => {
                    updated
                        .entry(EntityId(after.uid.into_bytes()))
                        .and_modify(|(_, last)| *last = after.entity.clone())
                        .or_insert_with(|| (before.entity.clone(), after.entity.clone()));
                    if matches!(&*after.entity, Entity::Wall(_)) {
                        walls
                            .entry(EntityId(after.uid.into_bytes()))
                            .and_modify(|(_, last)| *last = Some(after.entity.clone()))
                            .or_insert_with(|| (before.entity.clone(), Some(after.entity.clone())));
                    }
                }
                _ => {}
            }
        }
        let mut out = Vec::new();
        let runs: Vec<Arc<CaseworkRecord>> = self.casework.clone();
        for (index, run) in runs.into_iter().enumerate() {
            if own.contains(&run.id) {
                continue;
            }
            // Every object gone with the step: the run goes too.
            if !run.members.is_empty()
                && run.members.iter().all(|m| {
                    removed.contains(&m.uid) && self.slot_of(Uuid::from_bytes(m.uid.0)).is_none()
                })
            {
                let op = Op::CaseworkRemove { index, run: run.clone() };
                self.apply_op(&op);
                out.push(op);
                continue;
            }
            let changed: HashMap<EntityId, (Arc<Entity>, Option<Arc<Entity>>)> = run
                .ties
                .iter()
                .flatten()
                .filter_map(|t| walls.get(&t.wall).map(|w| (t.wall, w.clone())))
                .collect();
            let path = if !changed.is_empty() {
                if changed.values().any(|(_, after)| after.is_none()) {
                    Err(CaseworkBreak::WallGone)
                } else {
                    followed_path(&run.recipe, &run.ties, &changed).ok_or(CaseworkBreak::NotBuilt)
                }
            } else if let Some(path) = moved_path(&run, &updated) {
                // Its every object moved or turned together (Taşı, Döndür): the run goes with them.
                Ok(path)
            } else {
                continue;
            };
            let mut next = (*run).clone();
            let laid = path.and_then(|p| {
                let recipe = run.recipe.along(&p);
                let library = |_: &str| None;
                let mut fresh = || BlockId(new_uid().into_bytes());
                self.lay_casework(&recipe, &library, &mut fresh)
                    .map(|laid| (recipe, laid))
                    .map_err(|_| CaseworkBreak::NotBuilt)
            });
            let derived = match laid {
                Ok((recipe, laid)) => match self.casework_member_ops(Some(&run), &laid) {
                    Ok((mut ops, members, _)) => {
                        next.recipe = recipe;
                        next.ties = laid.ties.clone();
                        next.members = members;
                        next.broken = None;
                        ops.push(Op::CaseworkUpdate {
                            before: run.clone(),
                            after: Arc::new(next),
                        });
                        ops
                    }
                    Err(_) => continue,
                },
                Err(why) => {
                    if next.broken == Some(why) {
                        continue;
                    }
                    next.broken = Some(why);
                    vec![Op::CaseworkUpdate {
                        before: run.clone(),
                        after: Arc::new(next),
                    }]
                }
            };
            for op in &derived {
                self.apply_op(op);
            }
            out.extend(derived);
        }
        out
    }

    /// The faults of a step that changed or removed a run's object apart from its run (docs/adr/0727 §5): a run's
    /// shape, layer, colour, line weight and data are its own; a part of a run is not removed alone.
    pub(crate) fn casework_faults(&self, report: &crate::step::StepReport) -> Vec<Fault> {
        if self.casework.is_empty() {
            return Vec::new();
        }
        let runs = &report.casework;
        let authorized =
            |id: &CaseworkId| runs.added.contains(id) || runs.changed.contains(id) || runs.removed.contains(id);
        // A removed part of a run the step kept: the run still names it.
        let owners: HashMap<Uuid, CaseworkId> = self
            .casework
            .iter()
            .flat_map(|r| r.members.iter().map(move |m| (Uuid::from_bytes(m.uid.0), r.id)))
            .collect();
        let message = "Mutfak dizisinin nesnesi diziden ayrı değiştirilemez ya da silinemez; diziyi düzenleyin (Öznitelikler › Mutfak dizisi), bırakın ya da dizinin tamamını silin.";
        let mut out = Vec::new();
        for change in &report.objects {
            let Some(owner) = owners.get(&change.uid).copied() else { continue };
            if authorized(&owner) {
                continue;
            }
            let legal = match (change.before.as_deref(), change.after.as_deref()) {
                (Some(before), Some(after)) => same_owned(before, after),
                _ => false,
            };
            if !legal {
                out.push(Fault {
                    uid: change.uid,
                    message: message.into(),
                    code: Some(codes::CASEWORK_OWNED),
                });
            }
        }
        out
    }
}

/// Whether an object's change keeps what its run owns: its kind, shape, layer, colour, line weight and the run's
/// data (`Modül`, `Tür`, `Açılma`).
fn same_owned(before: &Entity, after: &Entity) -> bool {
    let strip = |e: &Entity| {
        let mut e = e.clone();
        let base = e.base_mut();
        base.label = None;
        base.symbol = None;
        base.look = Default::default();
        base.label_pins = Vec::new();
        base.attachments = Vec::new();
        base.attrs.retain(|k, _| matches!(k.as_str(), "Modül" | "Tür" | "Açılma"));
        e
    };
    strip(before) == strip(after)
}

/// The run's path moved and turned as its objects were when the step changed every one of them by the same move and
/// turn, read from its countertop's first two corners (a scale or a mirror is none: the run keeps its sizes and side);
/// none otherwise.
fn moved_path(run: &CaseworkRecord, updated: &HashMap<EntityId, (Arc<Entity>, Arc<Entity>)>) -> Option<Vec<Vec2>> {
    if run.members.is_empty() || !run.members.iter().all(|m| updated.contains_key(&m.uid)) {
        return None;
    }
    let counter = run.members.iter().find(|m| m.part == CaseworkPart::Counter)?;
    let (before, after) = updated.get(&counter.uid)?;
    let (Entity::Polygon(b), Entity::Polygon(a)) = (&**before, &**after) else {
        return None;
    };
    if b.pts.len() < 3 || a.pts.len() != b.pts.len() {
        return None;
    }
    let (p0, p1, q0, q1) = (b.pts[0], b.pts[1], a.pts[0], a.pts[1]);
    let (lb, la) = ((p1.x - p0.x).hypot(p1.y - p0.y), (q1.x - q0.x).hypot(q1.y - q0.y));
    if lb.is_nan() || lb <= SLACK || (la - lb).abs() > 1e-9 * lb.max(1.0) {
        return None;
    }
    let signed = |pts: &[Vec2]| {
        let n = pts.len();
        (0..n).map(|i| pts[i].x * pts[(i + 1) % n].y - pts[(i + 1) % n].x * pts[i].y).sum::<f64>()
    };
    if (signed(&b.pts) > 0.0) != (signed(&a.pts) > 0.0) {
        return None;
    }
    let turn = (q1.y - q0.y).atan2(q1.x - q0.x) - (p1.y - p0.y).atan2(p1.x - p0.x);
    let (sin, cos) = turn.sin_cos();
    if (q0.x - p0.x).abs() <= SLACK && (q0.y - p0.y).abs() <= SLACK && sin.abs() <= SLACK {
        return None;
    }
    Some(
        run.recipe
            .path()
            .into_iter()
            .map(|p| {
                let (dx, dy) = (p.x - p0.x, p.y - p0.y);
                Vec2 {
                    x: q0.x + dx * cos - dy * sin,
                    y: q0.y + dx * sin + dy * cos,
                }
            })
            .collect(),
    )
}

/// A leg's line now: a point on it and its unit direction.
type Line = (CoreVec2, CoreVec2);

fn unit(a: CoreVec2, b: CoreVec2) -> Option<CoreVec2> {
    Frame::of(a, b, true).map(|f| f.along)
}

/// The run's path after its walls changed (`changed`: a wall's before and after), or none when a tied face edge is
/// gone or two legs no longer meet (docs/adr/0727 §5): each tied leg's line its wall's face edge now, the others' as
/// they were; corners where the lines meet, the free ends their legs' old lengths from them; a one-leg run's ends
/// their old distances from the nearer end of its face edge.
fn followed_path(
    recipe: &CaseworkRun,
    ties: &[Option<CaseworkTie>],
    changed: &HashMap<EntityId, (Arc<Entity>, Option<Arc<Entity>>)>,
) -> Option<Vec<Vec2>> {
    let pts: Vec<CoreVec2> = recipe.path().into_iter().map(core_vec).collect();
    let n = pts.len() - 1;
    let mut lines: Vec<Line> = Vec::with_capacity(n);
    let mut edges: Vec<Option<(CoreVec2, CoreVec2, CoreVec2, CoreVec2)>> = Vec::with_capacity(n);
    for j in 0..n {
        let u = unit(pts[j], pts[j + 1])?;
        let tie = ties.get(j).cloned().flatten();
        let Some((tie, (before, Some(after)))) = tie.and_then(|t| changed.get(&t.wall).map(|w| (t, w.clone()))) else {
            lines.push((pts[j], u));
            edges.push(None);
            continue;
        };
        let (oa, ob) = face_edge(&before, tie.face, tie.edge)?;
        let (na, nb) = face_edge(&after, tie.face, tie.edge)?;
        let (ou, nu) = (unit(oa, ob)?, unit(na, nb)?);
        let sign = if u.x * ou.x + u.y * ou.y >= 0.0 { 1.0 } else { -1.0 };
        lines.push((na, CoreVec2::new(sign * nu.x, sign * nu.y)));
        edges.push(Some((oa, ob, na, nb)));
    }
    if n == 1 {
        let Some((oa, ob, na, nb)) = edges[0] else {
            return Some(pts.into_iter().map(wire).collect());
        };
        let old = Frame::of(oa, ob, true)?;
        let new = Frame::of(na, nb, true)?;
        let mut out = Vec::with_capacity(2);
        for q in &pts {
            let s = (q.x - oa.x) * old.along.x + (q.y - oa.y) * old.along.y;
            let at = if s <= old.length / 2.0 { s } else { new.length - (old.length - s) };
            out.push(wire(new.at(at, 0.0)));
        }
        return Some(out);
    }
    let mut corners = Vec::with_capacity(n - 1);
    for k in 1..n {
        let ((p1, u1), (p2, u2)) = (lines[k - 1], lines[k]);
        let den = u1.x * u2.y - u1.y * u2.x;
        if den.abs() < 1e-12 {
            return None;
        }
        let t = ((p2.x - p1.x) * u2.y - (p2.y - p1.y) * u2.x) / den;
        corners.push(CoreVec2::new(p1.x + u1.x * t, p1.y + u1.y * t));
    }
    let first = Frame::of(pts[0], pts[1], true)?.length;
    let last = Frame::of(pts[n - 1], pts[n], true)?.length;
    let (u0, ul) = (lines[0].1, lines[n - 1].1);
    let start = CoreVec2::new(corners[0].x - u0.x * first, corners[0].y - u0.y * first);
    let end = CoreVec2::new(
        corners[n - 2].x + ul.x * last,
        corners[n - 2].y + ul.y * last,
    );
    let mut out = vec![wire(start)];
    out.extend(corners.into_iter().map(wire));
    out.push(wire(end));
    Some(out)
}
