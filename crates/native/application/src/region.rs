//! `cad.entities.boundary` v1 and `cad.entities.hatch` v1 (docs/adr/0254
//! §6, §7): the region around a point (or a closed object's) made an area
//! or hatched, as one undo step “Alan oluştur” (“Tarama”). The desktop's
//! handlers over the native document, run by İçine tıklayarak alan,
//! Tarama and Çoklu tara (their previews are the plans, their clicks and
//! Enter the executes), the headless server, Python and MCP. The shared
//! cases are `fixtures/commands/v1/cad.entities.boundary.json` and `.hatch.json`.
//!
//! The regions are found here, once, by the shared core (`FaceIndex` over
//! the boundaries' line work; the smallest closed object around a point,
//! its islands and the texts' boxes, `cut_base`): what the tools found for
//! themselves before (docs/adr/0062, 0065, 0186). They are written through
//! `cad.entities.create`'s `boundary` and `hatch` operations
//! ([`crate::creating`]).

use kentos_contracts::{
    CommandResult, CommandWarning, CreateOperation, EntitiesBoundary, EntitiesCreate,
    EntitiesCreatePlan, EntitiesCreated, EntitiesHatch, EntityGeometry, EntityId, HatchAssoc,
    HatchPattern as ContractPattern, HatchPatternSpec, HatchPatternType, HatchRegion, HatchWay,
    TrimBoundaries, Vec2 as Point,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape, polygon_ring};
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::hatch::hatch_lines;
use kentos_geometry_core::geom::hatch_pattern::too_dense;
use kentos_geometry_core::geom::region::{FaceIndex, inside_area, net_area, orient_ring};
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::areas::{areas_of_entity, line_source, polygon_of_area};
use kentos_geometry_core::ops::hatch_region::{cut_base, cutout, pick, seed_of};
use kentos_geometry_core::store::Store;
use kentos_geometry_core::text::Font;
use kentos_geometry_core::tools::hatch::{Options, choice_named, colour_of, pattern_of};
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_uid_text};
use crate::codes;
use crate::creating;
use crate::geometry::{contract_pattern, core_pattern, drawing_font, edit_geometry, shape};
use crate::geometry_store::store_of;
use crate::topology_follow::slot;

/// The largest Ölçek a pattern takes (the tool's).
pub const MOST_SCALE: f64 = 10_000.0;

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

fn find(doc: &Document, uid: &str) -> Option<Slot> {
    Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u))
}

fn everything() -> Bounds {
    Bounds {
        min_x: f64::MIN,
        min_y: f64::MIN,
        max_x: f64::MAX,
        max_y: f64::MAX,
    }
}

fn rect(min: Point, max: Point) -> Bounds {
    Bounds {
        min_x: min.x.min(max.x),
        min_y: min.y.min(max.y),
        max_x: min.x.max(max.x),
        max_y: min.y.max(max.y),
    }
}

/// A ring's points, arcs as chords (the hatch's and the preview's).
pub fn ring_points(r: &Ring) -> Vec<Vec2> {
    polygon_ring(&r.pts, r.bulges.as_deref())
}

/// `r` starting at its lowest vertex (the leftmost of the lowest): a ring
/// is written the same way whichever vertex a face's walk began at.
fn from_lowest(r: Ring) -> Ring {
    let Some(k) = r
        .pts
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)))
        .map(|(k, _)| k)
    else {
        return r;
    };
    let mut pts = r.pts;
    pts.rotate_left(k);
    let bulges = r.bulges.map(|mut b| {
        if b.len() == pts.len() {
            b.rotate_left(k);
        }
        b
    });
    Ring { pts, bulges }
}

/// An area as the commands write it (docs/adr/0254 §6): its outer ring
/// counter-clockwise, its holes clockwise, each from its lowest vertex, the
/// holes by their lowest vertices. The face's walk, the cut and the host
/// leave no mark on the rings.
pub fn canonical(a: Area) -> Area {
    let outer = from_lowest(orient_ring(&a.outer, true));
    let mut holes: Vec<Ring> = a
        .holes
        .iter()
        .map(|h| from_lowest(orient_ring(h, false)))
        .collect();
    holes.sort_by(|a, b| {
        let (p, q) = (a.pts.first(), b.pts.first());
        match (p, q) {
            (Some(p), Some(q)) => p.y.total_cmp(&q.y).then(p.x.total_cmp(&q.x)),
            _ => std::cmp::Ordering::Equal,
        }
    });
    Area { outer, holes }
}

/// An area's box.
pub fn area_bounds(a: &Area) -> Bounds {
    let mut b = Bounds {
        min_x: f64::INFINITY,
        min_y: f64::INFINITY,
        max_x: f64::NEG_INFINITY,
        max_y: f64::NEG_INFINITY,
    };
    for q in ring_points(&a.outer) {
        b.min_x = b.min_x.min(q.x);
        b.min_y = b.min_y.min(q.y);
        b.max_x = b.max_x.max(q.x);
        b.max_y = b.max_y.max(q.y);
    }
    b
}

// ── The boundaries' faces ──────────────────────────────────────────────

/// Kinds whose line work bounds faces: fills, texts and dimensions do not.
fn bounds_faces(shape: &Shape) -> bool {
    !matches!(
        shape,
        Shape::Point { .. } | Shape::Text { .. } | Shape::Dimension { .. } | Shape::Hatch { .. }
    )
}

/// The checks of a boundary set: its ids (`no_entities`, `invalid_uid`) and
/// its rectangle (`not_finite`), at `at` (`boundaries`).
fn check_set(set: Option<&TrimBoundaries>, at: &str) -> Result<(), Stop> {
    match set {
        Some(TrimBoundaries::Objects { uids }) => {
            if uids.is_empty() {
                return Err(refuse(
                    codes::NO_ENTITIES,
                    "Sınır nesnesi yok. Sınırları verin ya da bütün görünen nesneler için “all”u kullanın.".into(),
                    &format!("{at}.uids"),
                ));
            }
            for (i, uid) in uids.iter().enumerate() {
                if !is_uid_text(uid) {
                    return Err(refuse(
                        codes::INVALID_UID,
                        format!(
                            "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
                        ),
                        &format!("{at}.uids[{i}]"),
                    ));
                }
            }
            Ok(())
        }
        Some(TrimBoundaries::Area { min, max }) => {
            checks::point(*min, "Dikdörtgenin köşesinin", &format!("{at}.min"))?;
            checks::point(*max, "Dikdörtgenin köşesinin", &format!("{at}.max"))
        }
        Some(TrimBoundaries::All) | None => Ok(()),
    }
}

/// The boundary set's objects are in the drawing (`entity_not_found`), the
/// boundary layer is a node of it (`layer_not_found`).
fn check_found(
    doc: &Document,
    set: Option<&TrimBoundaries>,
    at: &str,
    layer: Option<&str>,
    layer_at: &str,
) -> Result<(), Stop> {
    if let Some(TrimBoundaries::Objects { uids }) = set {
        for (i, uid) in uids.iter().enumerate() {
            if find(doc, uid).is_none() {
                return Err(refuse(
                    codes::ENTITY_NOT_FOUND,
                    format!(
                        "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                    ),
                    &format!("{at}.uids[{i}]"),
                ));
            }
        }
    }
    if let Some(id) = layer
        && doc.layers().get(id).is_none()
    {
        return Err(refuse(
            codes::LAYER_NOT_FOUND,
            format!("“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."),
            layer_at,
        ));
    }
    Ok(())
}

/// The faces the boundaries' line work closes: the shown objects of `set`
/// (all, or those meeting the rectangle) or the listed ones, of `layer`
/// alone when given.
pub fn faces(
    doc: &Document,
    store: &Store,
    set: Option<&TrimBoundaries>,
    layer: Option<&str>,
) -> FaceIndex {
    let on_layer = |e: &kentos_contracts::Entity| layer.is_none_or(|l| e.base().layer_id == l);
    let lines: Vec<CoreEntity> = match set {
        Some(TrimBoundaries::Objects { uids }) => uids
            .iter()
            .filter_map(|u| find(doc, u).and_then(|s| doc.get(s)))
            .filter(|e| on_layer(e))
            .map(|e| CoreEntity::new(shape(e)))
            .filter(|e| bounds_faces(&e.shape))
            .collect(),
        _ => {
            let view = match set {
                Some(TrimBoundaries::Area { min, max }) => rect(*min, *max),
                _ => everything(),
            };
            store
                .overlapping(&view, None)
                .into_iter()
                .filter(|it| bounds_faces(&it.shape))
                .filter(|it| slot(it.id).and_then(|s| doc.get(s)).is_some_and(&on_layer))
                .map(|it| CoreEntity::new(it.shape.clone()))
                .collect()
        }
    };
    FaceIndex::new(&[line_source(&lines)])
}

/// What finding regions keeps between calls on an unchanged drawing: the
/// faces of a boundary set, and a closed object's part cut by its islands
/// and texts. A tool keeps one across its pointer's moves (the preview);
/// a command makes its own. Keyed by the drawing's revision and what they
/// were found for.
#[derive(Default)]
pub struct RegionCache {
    faces: Option<(String, FaceIndex)>,
    cut: Option<(CutKey, Option<Whole>)>,
}

/// A closed object's cut: the revision, the object, its part, Adalar, Yazılar.
type CutKey = (u64, Slot, usize, bool, bool);

impl RegionCache {
    /// The faces of `set` (of `layer` alone when given), built once while the drawing stays.
    fn faces(
        &mut self,
        doc: &Document,
        store: &Store,
        set: Option<&TrimBoundaries>,
        layer: Option<&str>,
    ) -> &FaceIndex {
        let key = format!("{}|{set:?}|{layer:?}", doc.revision());
        if self.faces.as_ref().is_some_and(|(k, _)| *k != key) {
            self.faces = None;
        }
        &self
            .faces
            .get_or_insert_with(|| (key, faces(doc, store, set, layer)))
            .1
    }
}

// ── Alan oluştur ───────────────────────────────────────────────────────

/// The area `cad.entities.boundary` makes, found in `store` (the host's, or
/// one of the document): the region and the objects to create.
fn check_boundary(
    doc: &Document,
    store: Option<&Store>,
    input: &EntitiesBoundary,
    cache: &mut RegionCache,
) -> Result<(Area, EntitiesCreate), Stop> {
    checks::point(input.at, "Tıklanan noktanın", "at")?;
    check_set(input.boundaries.as_ref(), "boundaries")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    check_found(
        doc,
        input.boundaries.as_ref(),
        "boundaries",
        input.boundary_layer.as_deref(),
        "boundaryLayer",
    )?;
    let own;
    let store = match store {
        Some(s) => s,
        None => {
            own = store_of(doc);
            &own
        }
    };
    let index = cache.faces(
        doc,
        store,
        input.boundaries.as_ref(),
        input.boundary_layer.as_deref(),
    );
    let at = Vec2::new(input.at.x, input.at.y);
    let Some(area) = index.at(at, input.islands.unwrap_or(true)).map(canonical) else {
        return Err(refuse(
            codes::NOT_ENCLOSED,
            "Nokta kapalı bir bölgenin içinde değil. Bölgeyi saran çizgiler birleşmeli ya da kesişmeli.".into(),
            "at",
        ));
    };
    let Some(geometry) = edit_geometry(polygon_of_area(&area).shape) else {
        return Err(refuse(
            codes::NOT_ENCLOSED,
            "Bölgenin sınırı alana çevrilemedi.".into(),
            "at",
        ));
    };
    let objects = creating::input(
        &input.layer_id,
        vec![geometry],
        &input.look,
        CreateOperation::Boundary,
    );
    Ok((area, objects))
}

/// `cad.entities.boundary` v1: Alan oluştur.
pub mod entities_boundary {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesBoundary) -> CommandResult<()> {
        match check_boundary(cx.doc, cx.store, input, &mut RegionCache::default()) {
            Ok((_, objects)) => creating::validate(cx, &objects),
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &EntitiesBoundary,
    ) -> CommandResult<EntitiesCreatePlan> {
        plan_in(cx.doc, cx.store, input)
    }

    /// [`plan`] over the document and, when the host has one, its store (the tool's preview).
    pub fn plan_in(
        doc: &Document,
        store: Option<&Store>,
        input: &EntitiesBoundary,
    ) -> CommandResult<EntitiesCreatePlan> {
        match check_boundary(doc, store, input, &mut RegionCache::default()) {
            Ok((_, objects)) => creating::plan(doc, &objects),
            Err(stop) => stop.into(),
        }
    }

    /// The region around the point, for a tool's preview, its faces kept in
    /// `cache` while the drawing stays; none when no region closes there.
    pub fn region_in(
        doc: &Document,
        store: Option<&Store>,
        input: &EntitiesBoundary,
        cache: &mut RegionCache,
    ) -> Option<Area> {
        check_boundary(doc, store, input, cache)
            .ok()
            .map(|(area, _)| area)
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: EntitiesBoundary,
    ) -> CommandResult<EntitiesCreated> {
        crate::call::recorded(
            cx,
            kentos_contracts::CAD_ENTITIES_BOUNDARY,
            kentos_contracts::CAD_ENTITIES_BOUNDARY_VERSION,
            input,
            |cx, input| match check_boundary(cx.doc, cx.store, &input, &mut RegionCache::default())
            {
                Ok((_, objects)) => creating::execute(cx, objects),
                Err(stop) => stop.into(),
            },
        )
    }
}

// ── Tarama ─────────────────────────────────────────────────────────────

/// The objects a region came from: the closed object, the islands and the
/// cutouts that reached in, by their slots (kapalı nesne only).
#[derive(Clone, Debug, PartialEq)]
pub struct Tie {
    pub outer: Slot,
    pub islands: Vec<Slot>,
    pub cutouts: Vec<Slot>,
}

/// A region to hatch: its area, the point it was found by and, kapalı
/// nesne, what it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct Planned {
    pub area: Area,
    pub seed: Vec2,
    pub tie: Option<Tie>,
}

/// The texts and inserts shown in `bounds` and their boxes left open.
fn cutouts_in(bounds: &Bounds, font: Font, store: &Store) -> (Vec<Slot>, Vec<Vec<Vec2>>) {
    store
        .overlapping(bounds, None)
        .into_iter()
        .filter(|it| matches!(it.shape, Shape::Text { .. } | Shape::Insert { .. }))
        .filter_map(|it| Some((slot(it.id)?, cutout(&it.shape, store.blocks(), font)?)))
        .unzip()
}

/// The closed objects inside or across `base`'s object, smaller than it (a
/// block around a parcel is not an island).
fn islands_in(
    own: Slot,
    base: &Area,
    bounds: &Bounds,
    store: &Store,
) -> (Vec<Slot>, Vec<Vec<Area>>) {
    let size = net_area(base);
    store
        .overlapping(bounds, None)
        .into_iter()
        .filter(|it| slot(it.id) != Some(own) && !matches!(it.shape, Shape::Hatch { .. }))
        .filter_map(|it| {
            let areas: Vec<Area> = areas_of_entity(&it.shape)
                .into_iter()
                .filter(|a| net_area(a) < size * (1.0 - 1e-9))
                .collect();
            (!areas.is_empty()).then_some((slot(it.id)?, areas))
        })
        .unzip()
}

/// What a closed object's part is cut by: Adalar, Yazılar, the typeface the texts' boxes take.
#[derive(Clone, Copy)]
struct Cutting {
    islands: bool,
    texts: bool,
    font: Font,
}

/// A closed object's part cut by its islands and the texts that reach in:
/// every piece left, and what reached in.
#[derive(Clone, Debug, PartialEq)]
struct Whole {
    parts: Vec<Area>,
    tie: Option<Tie>,
}

fn cut_whole(own: Slot, part: &Area, bounds: &Bounds, by: Cutting, store: &Store) -> Option<Whole> {
    let (island_slots, island_areas) = if by.islands {
        islands_in(own, part, bounds, store)
    } else {
        (Vec::new(), Vec::new())
    };
    let (cutout_slots, boxes) = if by.texts {
        cutouts_in(bounds, by.font, store)
    } else {
        (Vec::new(), Vec::new())
    };
    let cut = cut_base(part, &island_areas, &boxes);
    Some(Whole {
        tie: Some(Tie {
            outer: own,
            islands: cut
                .islands
                .iter()
                .filter_map(|&k| island_slots.get(k).copied())
                .collect(),
            cutouts: cut
                .cutouts
                .iter()
                .filter_map(|&k| cutout_slots.get(k).copied())
                .collect(),
        }),
        parts: cut.parts,
    })
}

/// A closed object's part cut by its islands and the texts that reach in,
/// the piece around `seed`.
fn cut_part(
    own: Slot,
    part: &Area,
    bounds: &Bounds,
    seed: Vec2,
    by: Cutting,
    store: &Store,
) -> Option<Planned> {
    let whole = cut_whole(own, part, bounds, by, store)?;
    Some(Planned {
        area: canonical(pick(&whole.parts, seed)?.clone()),
        seed,
        tie: whole.tie,
    })
}

/// The pattern the spec names at the drawing's plot scale (1:N).
pub(crate) fn pattern_of_spec(
    spec: &HatchPatternSpec,
    plot_scale: f64,
) -> Result<ContractPattern, Stop> {
    let Some(choice) = choice_named(&spec.name) else {
        return Err(refuse(
            codes::UNKNOWN_PATTERN,
            format!(
                "“{}” adında desen yok. Desen kitaplığındaki bir adı verin (Dolu, Çizgili, ANSI31, ISO02W100 …).",
                spec.name
            ),
            "pattern.name",
        ));
    };
    let scale = spec.scale.unwrap_or(1.0);
    if !(scale.is_finite() && scale > 0.0 && scale <= MOST_SCALE) {
        return Err(refuse(
            codes::INVALID_SCALE,
            "Desenin ölçeği sıfırdan büyük, en çok 10 000 olan sonlu bir sayı olmalı.".into(),
            "pattern.scale",
        ));
    }
    let angle = spec.angle.unwrap_or(0.0);
    checks::finite(
        angle,
        "Desenin açısı",
        "Açıyı derece olarak sonlu bir sayıyla verin.",
        "pattern.angle",
    )?;
    let color2 = match &spec.second_color {
        None => "#FFFFFF".to_owned(),
        Some(text) => colour_of(text).ok_or_else(|| {
            refuse(
                codes::INVALID_COLOR,
                format!("“{text}” bir renk değil: #RRGGBB ya da bir renk adı (Beyaz, Siyah, Kırmızı …) verin."),
                "pattern.secondColor",
            )
        })?,
    };
    contract_pattern(pattern_of(&Options {
        choice,
        scale,
        angle,
        color2,
        inverted: spec.inverted.unwrap_or(false),
        plot_scale,
    }))
    .ok_or_else(|| {
        refuse(
            codes::UNKNOWN_PATTERN,
            format!("“{}” deseni yazılamıyor.", spec.name),
            "pattern.name",
        )
    })
}

/// A user-defined pattern's lines too many (the core's line count, ADR 0062's refusal).
fn dense_lines(ring: &[Vec2], holes: &[Vec<Vec2>], p: &ContractPattern) -> bool {
    match p.kind {
        HatchPatternType::Lines => hatch_lines(ring, p.angle, p.spacing, holes).capped,
        HatchPatternType::Cross => {
            hatch_lines(ring, p.angle, p.spacing, holes).capped
                || hatch_lines(ring, p.angle + 90.0, p.spacing, holes).capped
        }
        _ => false,
    }
}

/// The regions of `input`, found in `store`; the warnings of objects left out.
pub(crate) fn regions(
    doc: &Document,
    store: &Store,
    region: &HatchRegion,
    cache: &mut RegionCache,
) -> Result<(Vec<Planned>, Vec<CommandWarning>), Stop> {
    let font = drawing_font(doc.settings().drawing_font);
    match region {
        HatchRegion::Point {
            at,
            way,
            boundaries,
            boundary_layer,
            islands,
            texts,
            ..
        } => {
            let p = Vec2::new(at.x, at.y);
            let islands = islands.unwrap_or(true);
            let texts = texts.unwrap_or(false);
            let found = match way.unwrap_or_default() {
                HatchWay::Lines => {
                    let index =
                        cache.faces(doc, store, boundaries.as_ref(), boundary_layer.as_deref());
                    index.at(p, islands).and_then(|face| {
                        if !texts {
                            return Some(Planned {
                                area: canonical(face),
                                seed: p,
                                tie: None,
                            });
                        }
                        let (_, boxes) = cutouts_in(&area_bounds(&face), font, store);
                        cut_base(&face, &[], &boxes)
                            .parts
                            .into_iter()
                            .find(|a| inside_area(a, p))
                            .map(|area| Planned {
                                area: canonical(area),
                                seed: p,
                                tie: None,
                            })
                    })
                }
                HatchWay::Object => store.enclosing(p).and_then(|(id, _)| {
                    let own = slot(id)?;
                    let item = store.get(id)?;
                    // A multi-part area's part the point is in (docs/adr/0143); one area is itself.
                    let mut areas = areas_of_entity(&item.shape);
                    let part = if areas.len() == 1 {
                        0
                    } else {
                        areas.iter().position(|a| inside_area(a, p))?
                    };
                    // The part cut once while the drawing and the options stay; the piece
                    // around the point picked from it each time.
                    let key = (doc.revision(), own, part, islands, texts);
                    if cache.cut.as_ref().is_none_or(|(k, _)| *k != key) {
                        let base = areas.swap_remove(part);
                        let by = Cutting {
                            islands,
                            texts,
                            font,
                        };
                        let whole = cut_whole(own, &base, &item.bounds, by, store);
                        cache.cut = Some((key, whole));
                    }
                    let (_, whole) = cache.cut.as_ref()?;
                    let whole = whole.as_ref()?;
                    Some(Planned {
                        area: canonical(pick(&whole.parts, p)?.clone()),
                        seed: p,
                        tie: whole.tie.clone(),
                    })
                }),
            };
            let Some(planned) = found else {
                return Err(refuse(
                    codes::NOT_ENCLOSED,
                    if way.unwrap_or_default() == HatchWay::Lines {
                        "Nokta çizgilerle kapalı bir bölgenin içinde değil. Bölgeyi saran çizgiler birleşmeli ya da kesişmeli.".into()
                    } else {
                        "Noktayı çevreleyen kapalı bir alan, daire ya da kapalı eğri yok. Çizgilerle çevrili yerler için way: lines verin.".into()
                    },
                    "region.at",
                ));
            };
            Ok((vec![planned], Vec::new()))
        }
        HatchRegion::Objects {
            uids,
            islands,
            texts,
            ..
        } => {
            let islands = islands.unwrap_or(true);
            let texts = texts.unwrap_or(false);
            let mut planned = Vec::new();
            let mut warnings = Vec::new();
            for (i, uid) in uids.iter().enumerate() {
                let Some(s) = find(doc, uid) else { continue };
                let Some(e) = doc.get(s) else { continue };
                let sh = shape(e);
                let parts = areas_of_entity(&sh);
                if parts.is_empty() {
                    warnings.push(CommandWarning {
                        code: codes::NOT_CLOSED.into(),
                        message: "Nesne kapalı değil; taranmadı.".into(),
                        path: Some(format!("region.uids[{i}]")),
                    });
                    continue;
                }
                let bounds = store.get(f64::from(s.0)).map_or_else(
                    || {
                        let mut b = area_bounds(&parts[0]);
                        for p in &parts[1..] {
                            let q = area_bounds(p);
                            b.min_x = b.min_x.min(q.min_x);
                            b.min_y = b.min_y.min(q.min_y);
                            b.max_x = b.max_x.max(q.max_x);
                            b.max_y = b.max_y.max(q.max_y);
                        }
                        b
                    },
                    |it| it.bounds,
                );
                let by = Cutting {
                    islands,
                    texts,
                    font,
                };
                for part in &parts {
                    let Some(seed) = seed_of(part) else { continue };
                    if let Some(p) = cut_part(s, part, &bounds, seed, by, store) {
                        planned.push(p);
                    }
                }
            }
            if planned.is_empty() {
                return Err(if warnings.len() == uids.len() {
                    refuse(
                        codes::NOT_CLOSED,
                        "Nesnelerin hiçbiri kapalı değil: alan, daire, tam elips ya da kapalı eğri taranır.".into(),
                        "region.uids",
                    )
                } else {
                    refuse(
                        codes::NOT_ENCLOSED,
                        "Taranacak bölge kalmadı: adalar ya da yazılar alanların hepsini kaplıyor."
                            .into(),
                        "region.uids",
                    )
                });
            }
            Ok((planned, warnings))
        }
    }
}

/// The tie in the contract's terms: the objects' persistent ids, the seed.
fn assoc_of(doc: &Document, tie: &Tie, seed: Vec2) -> Option<HatchAssoc> {
    let id = |s: Slot| doc.uid(s).map(|u| EntityId(*u.as_bytes()));
    Some(HatchAssoc {
        outer: id(tie.outer)?,
        islands: tie.islands.iter().filter_map(|s| id(*s)).collect(),
        cutouts: tie.cutouts.iter().filter_map(|s| id(*s)).collect(),
        seed: Point {
            x: seed.x,
            y: seed.y,
        },
    })
}

fn wire(pts: &[Vec2]) -> Vec<Point> {
    pts.iter().map(|p| Point { x: p.x, y: p.y }).collect()
}

/// The checks of `cad.entities.hatch`, in order; the regions and the objects to create.
fn check_hatch(
    doc: &Document,
    store: Option<&Store>,
    input: &EntitiesHatch,
) -> Result<(Vec<Planned>, EntitiesCreate, Vec<CommandWarning>), Stop> {
    let pattern = pattern_of_spec(&input.pattern, doc.settings().plot_scale)?;
    let (associative, way) = match &input.region {
        HatchRegion::Point {
            at,
            way,
            boundaries,
            associative,
            ..
        } => {
            checks::point(*at, "Tıklanan noktanın", "region.at")?;
            check_set(boundaries.as_ref(), "region.boundaries")?;
            (associative.unwrap_or(true), way.unwrap_or_default())
        }
        HatchRegion::Objects {
            uids, associative, ..
        } => {
            if uids.is_empty() {
                return Err(refuse(
                    codes::NO_ENTITIES,
                    "Taranacak nesne yok. En az bir kapalı nesnenin kalıcı kimliğini verin.".into(),
                    "region.uids",
                ));
            }
            for (i, uid) in uids.iter().enumerate() {
                if !is_uid_text(uid) {
                    return Err(refuse(
                        codes::INVALID_UID,
                        format!(
                            "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
                        ),
                        &format!("region.uids[{i}]"),
                    ));
                }
            }
            (associative.unwrap_or(true), HatchWay::Object)
        }
    };
    checks::revision(doc, input.expected_revision.as_deref())?;
    match &input.region {
        HatchRegion::Point {
            boundaries,
            boundary_layer,
            ..
        } => check_found(
            doc,
            boundaries.as_ref(),
            "region.boundaries",
            boundary_layer.as_deref(),
            "region.boundaryLayer",
        )?,
        HatchRegion::Objects { uids, .. } => {
            for (i, uid) in uids.iter().enumerate() {
                if find(doc, uid).is_none() {
                    return Err(refuse(
                        codes::ENTITY_NOT_FOUND,
                        format!(
                            "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                        ),
                        &format!("region.uids[{i}]"),
                    ));
                }
            }
        }
    }
    let own;
    let store = match store {
        Some(s) => s,
        None => {
            own = store_of(doc);
            &own
        }
    };
    let (planned, warnings) = regions(doc, store, &input.region, &mut RegionCache::default())?;
    let core = core_pattern(&pattern);
    let mut geometries = Vec::with_capacity(planned.len());
    for p in &planned {
        let ring = ring_points(&p.area.outer);
        let holes: Vec<Vec<Vec2>> = p.area.holes.iter().map(ring_points).collect();
        if too_dense(&ring, &core) || dense_lines(&ring, &holes, &pattern) {
            return Err(refuse(
                codes::PATTERN_TOO_DENSE,
                "Desen bu bölge için çok sık; ölçeği ya da çizim ölçeğini büyütün ya da başka bir desen seçin.".into(),
                "pattern",
            ));
        }
        let assoc = match &p.tie {
            Some(tie) if associative && way == HatchWay::Object => assoc_of(doc, tie, p.seed),
            _ => None,
        };
        geometries.push(EntityGeometry::Hatch {
            ring: wire(&ring),
            holes: (!holes.is_empty()).then(|| holes.iter().map(|h| wire(h)).collect()),
            pattern: pattern.clone(),
            assoc,
        });
    }
    let objects = creating::input(
        &input.layer_id,
        geometries,
        &input.look,
        CreateOperation::Hatch,
    );
    Ok((planned, objects, warnings))
}

/// `cad.entities.hatch` v1: Tarama (and Çoklu tara).
pub mod entities_hatch {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesHatch) -> CommandResult<()> {
        match check_hatch(cx.doc, cx.store, input) {
            Ok((_, objects, warnings)) => {
                creating::with_warnings(creating::validate(cx, &objects), warnings)
            }
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &EntitiesHatch,
    ) -> CommandResult<EntitiesCreatePlan> {
        plan_in(cx.doc, cx.store, input)
    }

    /// [`plan`] over the document and, when the host has one, its store (the tools' preview).
    pub fn plan_in(
        doc: &Document,
        store: Option<&Store>,
        input: &EntitiesHatch,
    ) -> CommandResult<EntitiesCreatePlan> {
        match check_hatch(doc, store, input) {
            Ok((_, objects, warnings)) => {
                creating::with_warnings(creating::plan(doc, &objects), warnings)
            }
            Err(stop) => stop.into(),
        }
    }

    /// The regions `input` would hatch, for a tool's preview (without the
    /// pattern's checks), what they were found from kept in `cache` while
    /// the drawing stays.
    pub fn regions_in(
        doc: &Document,
        store: Option<&Store>,
        input: &EntitiesHatch,
        cache: &mut RegionCache,
    ) -> Vec<Planned> {
        let own;
        let store = match store {
            Some(s) => s,
            None => {
                own = store_of(doc);
                &own
            }
        };
        regions(doc, store, &input.region, cache)
            .map(|(planned, _)| planned)
            .unwrap_or_default()
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: EntitiesHatch,
    ) -> CommandResult<EntitiesCreated> {
        crate::call::recorded(
            cx,
            kentos_contracts::CAD_ENTITIES_HATCH,
            kentos_contracts::CAD_ENTITIES_HATCH_VERSION,
            input,
            |cx, input| match check_hatch(cx.doc, cx.store, &input) {
                Ok((_, objects, warnings)) => {
                    creating::with_warnings(creating::execute(cx, objects), warnings)
                }
                Err(stop) => stop.into(),
            },
        )
    }
}
