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
//! boundary layer is the run's. The product command `cad.entities.hatch`
//! does the work (docs/adr/0254 §7): the region under the cursor is its own
//! (`regions_in`, its faces and cut kept while the drawing stays), the click
//! its execute, one undo step named “Tarama”.

use kentos_contracts::{EntitiesHatch, HatchRegion, HatchWay, NewLook, TrimBoundaries};
use kentos_domain::Slot;
use kentos_geometry_core::entity::polygon_ring;
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::hatch_pattern::pattern_pieces;
use kentos_geometry_core::tools::hatch::Kind;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::core_pattern;
use kentos_native_application::region::{Planned, RegionCache};
use kentos_native_application::{ExecutionContext, entities_hatch};

use crate::Vec2;
use crate::format::Format;
use crate::hatch_options::{self as options, Asking};
use crate::log::Level;
use crate::points::{self, wire};
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

/// The hatch tool.
#[derive(Default)]
pub struct Hatch {
    /// Picking the boundary layer by an object of it.
    picking_layer: bool,
    /// Sınır katmanı: only this layer's line work bounds the faces; none, every visible layer's.
    boundary: Option<String>,
    /// The boundary layer's name, as of the last call.
    boundary_name: Option<String>,
    /// The command's faces and cut, kept while the drawing and the options stay.
    cache: RegionCache,
    /// The region under the cursor, and its lines and dots when the preview draws them.
    hover: Option<Area>,
    hover_lines: Vec<[Vec2; 2]>,
    /// A value the command line asks for (Ölçek, Açı, İkinci renk).
    asking: Option<Asking>,
    /// The project's plot scale, as of the last call.
    plot_scale: f64,
    /// What the session remembered, as of the last call.
    memory: Memory,
    /// The project's own patterns' names, as of the last call (docs/adr/0331 §1).
    user: Vec<String>,
    /// The UCS's turn, degrees (docs/adr/0261 §4).
    turn: f64,
}

impl Hatch {
    pub fn new() -> Self {
        Self::default()
    }

    /// The session's options with the pattern's angle from the UCS's x axis.
    fn turned(&self) -> Memory {
        let mut m = self.memory;
        m.hatch_angle += self.turn;
        m
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.memory = *cx.memory;
        self.user = options::user_names(cx);
        self.turn = cx.ucs_degrees();
        self.plot_scale = cx.doc.settings().plot_scale;
        self.boundary_name = self.boundary.as_deref().map(|id| {
            cx.doc
                .layers()
                .get(id)
                .map_or_else(|| "—".to_owned(), |n| n.name.clone())
        });
    }

    /// The command's input for a click at `p`: the way, the view's objects
    /// (or the boundary layer's), the options, the active layer and colour.
    fn request(&self, p: Vec2, cx: &Context<'_>) -> EntitiesHatch {
        let mut m = *cx.memory;
        // The pattern's angle is from the UCS's x axis (docs/adr/0261 §4).
        m.hatch_angle += cx.ucs_degrees();
        let view = cx.view.visible();
        EntitiesHatch {
            region: HatchRegion::Point {
                at: wire(p),
                way: Some(if m.hatch_by_lines {
                    HatchWay::Lines
                } else {
                    HatchWay::Object
                }),
                boundaries: m.hatch_by_lines.then(|| TrimBoundaries::Area {
                    min: wire(Vec2::new(view.min_x, view.min_y)),
                    max: wire(Vec2::new(view.max_x, view.max_y)),
                }),
                boundary_layer: if m.hatch_by_lines {
                    self.boundary.clone()
                } else {
                    None
                },
                islands: Some(m.hatch_islands),
                texts: Some(m.hatch_texts),
                associative: Some(m.hatch_assoc),
                also_at: Vec::new(),
                island_style: options::island_style(&m),
                gap: (m.hatch_by_lines && m.hatch_gap > 0.0).then_some(m.hatch_gap),
            },
            pattern: options::spec(&m, &self.user),
            layer_id: cx.doc.layers().active().to_owned(),
            look: NewLook {
                color: cx.draft.color_text(),
                line_weight: None,
                attrs: cx.template_attrs(None),
                label: cx.template_label(),
                symbol: cx.template_symbol(),
            },
            expected_revision: None,
        }
    }

    /// The region to fill around `p` and what it came from: the command's.
    fn region(&mut self, p: Vec2, cx: &Context<'_>) -> Option<Planned> {
        let input = self.request(p, cx);
        entities_hatch::regions_in(cx.doc, Some(cx.spatial.store()), &input, &mut self.cache)
            .into_iter()
            .next()
    }

    /// The region and the lines the preview draws of it.
    fn hover_at(&mut self, p: Vec2, cx: &Context<'_>) {
        self.hover = self.region(p, cx).map(|r| r.area);
        self.hover_lines.clear();
        // The pattern as the command makes it (docs/adr/0331): a project pattern by name, the basis, the origin.
        let spec = options::spec(&self.turned(), &self.user);
        let (Some(area), Some(mut pattern)) = (
            &self.hover,
            kentos_native_application::region::spec_pattern(&spec, cx.doc.settings()),
        ) else {
            return;
        };
        let ring = ring_points(&area.outer);
        let holes: Vec<Vec<Vec2>> = area.holes.iter().map(ring_points).collect();
        let own = kentos_native_application::region::box_of([ring.as_slice()]);
        pattern.origin = kentos_native_application::region::origin_of(spec.origin.as_ref(), own, own);
        let pieces = pattern_pieces(&ring, &holes, &core_pattern(&pattern), PREVIEW_PIECES);
        if !pieces.capped {
            self.hover_lines = pieces.segments;
            self.hover_lines
                .extend(pieces.dots.into_iter().map(|q| [q, q]));
        }
    }

    /// Fills the region around `p` (the web's `pointerDown`).
    fn fill(&mut self, p: Vec2, cx: &mut Context<'_>) {
        let Some(planned) = self.region(p, cx) else {
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
        let input = self.request(p, cx);
        let result = {
            let mut ecx = ExecutionContext::with_store(cx.doc, cx.spatial.store());
            entities_hatch::execute(&mut ecx, input)
        };
        let Some(out) = points::written(result, cx) else {
            return;
        };
        let size = out
            .ids
            .first()
            .and_then(|&id| cx.doc.get(Slot(id)))
            .and_then(|e| measures(e).0)
            .unwrap_or(0.0);
        let (islands, texts) = match &planned.tie {
            Some(t) => (t.islands.len(), t.cutouts.len()),
            None => (planned.area.holes.len(), 0),
        };
        let name = options::choice(&self.memory).name;
        let text = format!(
            "{name} tarama eklendi: {}{}",
            cx.format().area(size),
            left_out(islands, texts)
        );
        cx.say(Level::Success, text);
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
pub(crate) fn left_out(islands: usize, texts: usize) -> String {
    let mut out = String::new();
    if islands > 0 {
        out.push_str(&format!(", {islands} ada taranmadı"));
    }
    if texts > 0 {
        out.push_str(&format!(", {texts} yazı boş bırakıldı"));
    }
    out
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
            options::with_pattern_options(Prompt::new(LABEL, "taranacak yerin içine tıklayın"), &m, &self.user)
                .option_with(
                    "Sınır",
                    "B",
                    if m.hatch_by_lines {
                        "çizgiler"
                    } else {
                        "kapalı nesne"
                    },
                );
        let prompt = options::with_region_options(prompt, &m, true);
        if m.hatch_by_lines {
            let layer = self
                .boundary_name
                .clone()
                .unwrap_or_else(|| "tümü".to_owned());
            return prompt
                .option_with("Sınır katmanı", "K", layer)
                .option_with("Boşluk", "G", crate::format::js_number(m.hatch_gap));
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
        // Başlangıç's point or Özellik al's hatch (docs/adr/0331 §2).
        if options::clicked(&mut self.asking, p.world, p.raw, cx) {
            self.see(cx);
            self.changed();
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
            // Boşluk, in the line work (docs/adr/0329 §3).
            "G" if cx.memory.hatch_by_lines => {
                self.asking = Some(Asking::Gap);
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
            k if options::option(k, true, &mut self.asking, cx) => self.changed(),
            _ if options::typed_choice(text, cx) => self.changed(),
            _ => false,
        };
        self.see(cx);
        done
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        options::choices_of(key, &self.memory, &self.user)
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
