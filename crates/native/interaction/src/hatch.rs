//! Tarama: the web's `HatchTool` (`apps/web/src/tools/hatchTool.ts`) with
//! its `VisibleFaces` (`tools/visibleFaces.ts`), step for step
//! (docs/adr/0062, 0186). A click inside fills the region with the pattern.
//! Two ways to find the region:
//!
//! - kapalı nesne (the default, Netcad's): the smallest closed object around
//!   the click; closed objects inside it or across its edge (buildings in a
//!   parcel) are islands left unhatched; with İlişkili on, the hatch follows
//!   its objects (`assoc`, docs/adr/0186 §6);
//! - çizgiler (AutoCAD's): the face the visible line work closes around the
//!   click, closed groups inside it as islands; the boundary set can be one
//!   layer (Sınır katmanı, picked by an object of it). It follows nothing.
//!
//! Yazılar leaves the texts and inserts in the region open, each its box a
//! little wider. The pattern's options are `hatch_options`' (Desen with its
//! menu, Ölçek, Açı, İkinci renk, Ters); they, the way, the islands, the
//! tie and the texts stay for as long as the app lives (`Memory`); the
//! boundary layer is the run's. Each hatch is written through
//! `cad.entities.create`, one undo step named “Tarama”. The regions, the
//! faces, the cut and the hatch lines are the shared core's.

use kentos_contracts::{CreateOperation, EntityGeometry, EntityId, HatchAssoc, Vec2 as Point};
use kentos_domain::Slot;
use kentos_geometry_core::entity::{Shape, polygon_ring};
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::hatch::hatch_lines;
use kentos_geometry_core::geom::hatch_pattern::{pattern_pieces, too_dense};
use kentos_geometry_core::geom::region::{inside_area, net_area};
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::areas::areas_of_entity;
use kentos_geometry_core::ops::hatch_region::{Cut, cut_base, cutout};
use kentos_geometry_core::text::Font;
use kentos_geometry_core::tools::hatch::Kind;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::{core_pattern, drawing_font};

use crate::Vec2;
use crate::faces;
use crate::format::Format;
use crate::hatch_options::{self as options, Asking};
use crate::log::Level;
use crate::points::{self, wire_all};
use crate::prompt::{Prompt, upper_tr};
use crate::spatial::measures;
use crate::tool::{self, Context, Cursor, Flow, Memory, OptionChoice, Pointer, Preview, Tool};

/// The tool's id: its command is `tool.hatch`.
pub const ID: &str = "hatch";
pub const LABEL: &str = "Tarama";

/// The most lines and dots a preview draws (the web's 3000); the hatch itself draws them all.
const PREVIEW_PIECES: usize = 3000;

/// A ring's points, arcs tessellated (the web's `polygonRing`).
fn ring_points(r: &Ring) -> Vec<Vec2> {
    polygon_ring(&r.pts, r.bulges.as_deref())
}

/// The objects a region came from: the closed object, the islands and the
/// cutouts that reached in, by their store ids (kapalı nesne only).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Tie {
    pub outer: f64,
    pub islands: Vec<f64>,
    pub cutouts: Vec<f64>,
}

/// A base area cut once: what it was cut from (revision, object, part, the
/// islands and texts flags) and what reached in, by store id.
struct CutCache {
    key: (u64, f64, usize, bool, bool),
    cut: Cut,
    islands: Vec<f64>,
    cutouts: Vec<f64>,
}

/// The texts and inserts shown in `bounds` and their boxes left open (§4).
pub(crate) fn cutouts_in(
    bounds: &Bounds,
    font: Font,
    cx: &Context<'_>,
) -> (Vec<f64>, Vec<Vec<Vec2>>) {
    let store = cx.spatial.store();
    store
        .overlapping(bounds, None)
        .into_iter()
        .filter(|it| matches!(it.shape, Shape::Text { .. } | Shape::Insert { .. }))
        .filter_map(|it| cutout(&it.shape, store.blocks(), font).map(|b| (it.id, b)))
        .unzip()
}

/// The closed objects inside or across `base`'s object, smaller than it (a
/// block around a parcel is not an island; the web's `islandsOf`).
pub(crate) fn islands_in(
    id: f64,
    base: &Area,
    bounds: &Bounds,
    cx: &Context<'_>,
) -> (Vec<f64>, Vec<Vec<Area>>) {
    let size = net_area(base);
    cx.spatial
        .store()
        .overlapping(bounds, None)
        .into_iter()
        .filter(|it| it.id != id && !matches!(it.shape, Shape::Hatch { .. }))
        .filter_map(|it| {
            let areas: Vec<Area> = areas_of_entity(&it.shape)
                .into_iter()
                .filter(|a| net_area(a) < size * (1.0 - 1e-9))
                .collect();
            (!areas.is_empty()).then_some((it.id, areas))
        })
        .unzip()
}

/// A tie in the contract's terms: the objects' persistent ids, the seed.
pub(crate) fn assoc_of(tie: &Tie, seed: Vec2, cx: &Context<'_>) -> Option<HatchAssoc> {
    let id = |s: f64| cx.doc.uid(Slot(s as u32)).map(|u| EntityId(*u.as_bytes()));
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

/// The hatch tool.
#[derive(Default)]
pub struct Hatch {
    /// Picking the boundary layer by an object of it.
    picking_layer: bool,
    /// Sınır katmanı: only this layer's line work bounds the faces; none, every visible layer's.
    boundary: Option<String>,
    /// The boundary layer's name, as of the last call.
    boundary_name: Option<String>,
    /// Kapalı nesne: the boundary object's part cut by what reaches in,
    /// kept while the drawing and the options stay as they were.
    cache: Option<CutCache>,
    faces: Option<faces::Faces>,
    /// The region under the cursor, and its lines and dots when the preview draws them.
    hover: Option<Area>,
    hover_lines: Vec<[Vec2; 2]>,
    /// A value the command line asks for (Ölçek, Açı, İkinci renk).
    asking: Option<Asking>,
    /// The project's plot scale and typeface, as of the last call.
    plot_scale: f64,
    font: Font,
    /// What the session remembered, as of the last call.
    memory: Memory,
}

impl Hatch {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.memory = *cx.memory;
        self.plot_scale = cx.doc.settings().plot_scale;
        self.font = drawing_font(cx.doc.settings().drawing_font);
        self.boundary_name = self.boundary.as_deref().map(|id| {
            cx.doc
                .layers()
                .get(id)
                .map_or_else(|| "—".to_owned(), |n| n.name.clone())
        });
    }

    /// The region to fill around `p` and, kapalı nesne, the objects it came from.
    fn region(&mut self, p: Vec2, cx: &Context<'_>) -> Option<(Area, Option<Tie>)> {
        let m = self.memory;
        if m.hatch_by_lines {
            let layer = self.boundary.clone();
            let face = faces::face_at(&mut self.faces, p, m.hatch_islands, layer.as_deref(), cx)?;
            if !m.hatch_texts {
                return Some((face, None));
            }
            let bounds = area_bounds(&face);
            let (_, boxes) = cutouts_in(&bounds, self.font, cx);
            let cut = cut_base(&face, &[], &boxes);
            return cut
                .parts
                .into_iter()
                .find(|a| inside_area(a, p))
                .map(|a| (a, None));
        }
        let store = cx.spatial.store();
        let (id, _) = store.enclosing(p)?;
        let item = store.get(id)?;
        // A multi-part area's part the point is in (docs/adr/0143); one area is itself.
        let mut areas = areas_of_entity(&item.shape);
        let part = if areas.len() == 1 {
            0
        } else {
            areas.iter().position(|a| inside_area(a, p))?
        };
        let base = areas.swap_remove(part);
        let key = (cx.doc.revision(), id, part, m.hatch_islands, m.hatch_texts);
        if self.cache.as_ref().is_none_or(|c| c.key != key) {
            let (islands, island_areas) = if m.hatch_islands {
                islands_in(id, &base, &item.bounds, cx)
            } else {
                (Vec::new(), Vec::new())
            };
            let (cutouts, boxes) = if m.hatch_texts {
                cutouts_in(&item.bounds, self.font, cx)
            } else {
                (Vec::new(), Vec::new())
            };
            let cut = cut_base(&base, &island_areas, &boxes);
            self.cache = Some(CutCache {
                key,
                cut,
                islands,
                cutouts,
            });
        }
        let c = self.cache.as_ref()?;
        let area = c.cut.parts.iter().find(|a| inside_area(a, p))?.clone();
        let tie = Tie {
            outer: id,
            islands: c
                .cut
                .islands
                .iter()
                .filter_map(|&k| c.islands.get(k).copied())
                .collect(),
            cutouts: c
                .cut
                .cutouts
                .iter()
                .filter_map(|&k| c.cutouts.get(k).copied())
                .collect(),
        };
        Some((area, Some(tie)))
    }

    /// The region and the lines the preview draws of it.
    fn hover_at(&mut self, p: Vec2, cx: &Context<'_>) {
        self.hover = self.region(p, cx).map(|(a, _)| a);
        self.hover_lines.clear();
        let (Some(area), Some(pattern)) =
            (&self.hover, options::pattern(&self.memory, self.plot_scale))
        else {
            return;
        };
        let ring = ring_points(&area.outer);
        let holes: Vec<Vec<Vec2>> = area.holes.iter().map(ring_points).collect();
        let pieces = pattern_pieces(&ring, &holes, &core_pattern(&pattern), PREVIEW_PIECES);
        if !pieces.capped {
            self.hover_lines = pieces.segments;
            self.hover_lines
                .extend(pieces.dots.into_iter().map(|q| [q, q]));
        }
    }

    /// Fills the region around `p` (the web's `pointerDown`).
    fn fill(&mut self, p: Vec2, cx: &mut Context<'_>) {
        let Some((area, tie)) = self.region(p, cx) else {
            cx.say(
                Level::Warn,
                if self.memory.hatch_by_lines {
                    "Tıklanan yer çizgilerle kapalı bir bölgenin içinde değil; görünüm dışındaki çizgiler sayılmaz."
                } else {
                    "Tıklanan noktayı çevreleyen kapalı bir alan, daire ya da kapalı eğri yok. Çizgilerle çevrili yerler için “Sınır: çizgiler” seçin."
                },
            );
            return;
        };
        let Some(pattern) = options::pattern(&self.memory, self.plot_scale) else {
            return;
        };
        let ring = ring_points(&area.outer);
        let holes: Vec<Vec<Vec2>> = area.holes.iter().map(ring_points).collect();
        if too_dense(&ring, &core_pattern(&pattern)) || dense_lines(&ring, &holes, &pattern) {
            cx.say(
                Level::Warn,
                "Desen bu alan için çok sık; ölçeği ya da çizim ölçeğini büyütün ya da başka bir desen seçin.",
            );
            return;
        }
        let assoc = match &tie {
            Some(tie) if self.memory.hatch_assoc => assoc_of(tie, p, cx),
            _ => None,
        };
        let name = options::choice(&self.memory).name;
        let geometry = EntityGeometry::Hatch {
            ring: wire_all(&ring),
            holes: (!holes.is_empty()).then(|| holes.iter().map(|h| wire_all(h)).collect()),
            pattern,
            assoc,
        };
        let Some(out) = points::write_objects(vec![geometry], Some(CreateOperation::Hatch), cx)
        else {
            return;
        };
        let size = out
            .ids
            .first()
            .and_then(|&id| cx.doc.get(Slot(id)))
            .and_then(|e| measures(e).0)
            .unwrap_or(0.0);
        let text = format!(
            "{name} tarama eklendi: {}{}",
            cx.format().area(size),
            left_out(&tie, holes.len())
        );
        cx.say(Level::Success, text);
        self.cache = None;
    }

    /// An option went in: the preview waits for the next move (the web's `hover = null`).
    fn changed(&mut self) -> bool {
        self.hover = None;
        self.hover_lines.clear();
        true
    }
}

/// What a hatch left out, for its message: the islands and the texts that
/// reached in (kapalı nesne), else the region's holes as islands.
pub(crate) fn left_out(tie: &Option<Tie>, holes: usize) -> String {
    let (islands, texts) = match tie {
        Some(t) => (t.islands.len(), t.cutouts.len()),
        None => (holes, 0),
    };
    let mut out = String::new();
    if islands > 0 {
        out.push_str(&format!(", {islands} ada taranmadı"));
    }
    if texts > 0 {
        out.push_str(&format!(", {texts} yazı boş bırakıldı"));
    }
    out
}

/// A user-defined pattern's lines too many (the core's line count, ADR 0062's refusal).
fn dense_lines(ring: &[Vec2], holes: &[Vec<Vec2>], p: &kentos_contracts::HatchPattern) -> bool {
    use kentos_contracts::HatchPatternType as T;
    match p.kind {
        T::Lines => hatch_lines(ring, p.angle, p.spacing, holes).capped,
        T::Cross => {
            hatch_lines(ring, p.angle, p.spacing, holes).capped
                || hatch_lines(ring, p.angle + 90.0, p.spacing, holes).capped
        }
        _ => false,
    }
}

/// An area's box.
pub(crate) fn area_bounds(a: &Area) -> Bounds {
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

impl Tool for Hatch {
    /// An object is picked (the web's `cursor = 'pick'`).
    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        if self.picking_layer {
            return Prompt::new(LABEL, "sınır olacak katmandan bir nesneye tıklayın")
                .option("Tüm katmanlar", "K");
        }
        if let Some(a) = self.asking {
            return options::asking_prompt(LABEL, a);
        }
        let m = self.memory;
        let prompt =
            options::with_pattern_options(Prompt::new(LABEL, "taranacak yerin içine tıklayın"), &m)
                .option_with(
                    "Sınır",
                    "B",
                    if m.hatch_by_lines {
                        "çizgiler"
                    } else {
                        "kapalı nesne"
                    },
                );
        let prompt = options::with_region_options(prompt, &m, !m.hatch_by_lines);
        if m.hatch_by_lines {
            let layer = self
                .boundary_name
                .clone()
                .unwrap_or_else(|| "tümü".to_owned());
            return prompt.option_with("Sınır katmanı", "K", layer);
        }
        prompt
    }

    fn point_count(&self) -> usize {
        0
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    /// No snap at all: a click is a place inside, not a point.
    fn snaps(&self) -> bool {
        false
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        if self.picking_layer {
            let hit = cx.spatial.pick(p.raw, cx.pick_tolerance());
            cx.selection.set_hover(hit);
            return;
        }
        self.hover_at(p.raw, cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        if self.picking_layer {
            let Some(e) = cx
                .spatial
                .pick(p.raw, cx.pick_tolerance())
                .and_then(|s| cx.doc.get(s))
            else {
                cx.say(
                    Level::Warn,
                    "Sınır katmanını seçmek için bir nesneye tıklayın.",
                );
                return;
            };
            self.boundary = Some(e.base().layer_id.clone());
            self.picking_layer = false;
            cx.selection.set_hover(None);
            self.see(cx);
            return;
        }
        self.asking = None;
        self.fill(p.raw, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.asking.is_some() {
            let taken = options::answer(&mut self.asking, text, cx);
            self.see(cx);
            return taken && self.changed();
        }
        let key = upper_tr(js_trim(text));
        let done = match key.as_str() {
            "B" => {
                cx.memory.hatch_by_lines = !cx.memory.hatch_by_lines;
                self.picking_layer = false;
                self.changed()
            }
            "K" if cx.memory.hatch_by_lines || self.picking_layer => {
                if self.picking_layer || self.boundary.is_some() {
                    self.boundary = None;
                    self.picking_layer = false;
                } else {
                    self.picking_layer = true;
                }
                cx.selection.set_hover(None);
                self.changed()
            }
            k if options::option(k, !cx.memory.hatch_by_lines, &mut self.asking, cx) => {
                self.cache = None;
                self.changed()
            }
            _ if options::typed_choice(text, cx) => self.changed(),
            _ => false,
        };
        self.see(cx);
        done
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        options::choices_of(key, &self.memory)
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        let taken = options::choose(key, typed, &mut self.asking, cx);
        self.see(cx);
        taken && self.changed()
    }

    /// Enter, Space or a quick right click leave the tool; while a value is
    /// asked, Enter keeps the old one.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.asking.take().is_some() {
            self.see(cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    /// Esc leaves what is asked first, then the layer picking.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.asking.take().is_some() {
            self.see(cx);
            return true;
        }
        if !self.picking_layer {
            return false;
        }
        self.picking_layer = false;
        cx.selection.set_hover(None);
        true
    }

    /// The tool takes nothing back itself: Ctrl+Z is the drawing's (the last hatch).
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    /// The region under the cursor, outlined dashed; filled for a solid or a
    /// gradient, else its lines faint (the web's `draw`).
    fn preview(&self, _format: &Format) -> Preview {
        let Some(area) = self.hover.as_ref().filter(|_| !self.picking_layer) else {
            return Preview::default();
        };
        let filled = matches!(
            options::choice(&self.memory).kind,
            Kind::Solid | Kind::Gradient(_)
        );
        Preview {
            areas: vec![tool::Area {
                rings: std::iter::once(&area.outer)
                    .chain(&area.holes)
                    .map(ring_points)
                    .collect(),
                fill: if filled { 0.25 } else { 0.0 },
                width: 1.5,
                dash: Some([4.0, 3.0]),
                fill_tone: tool::Tone::Accent,
            }],
            hatch: self.hover_lines.clone(),
            ..Preview::default()
        }
    }
}
