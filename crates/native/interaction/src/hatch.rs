//! Tarama: the web's `HatchTool` (`apps/web/src/tools/hatchTool.ts`) with
//! its `VisibleFaces` (`tools/visibleFaces.ts`), step for step
//! (docs/adr/0062). A click inside fills the region with the pattern; the
//! hatch is not associative. Two ways to find the region:
//!
//! - kapalı nesne (the default, Netcad's): the smallest closed object around
//!   the click; closed objects inside it or across its edge (buildings in a
//!   parcel) are islands left unhatched;
//! - çizgiler (AutoCAD's): the face the visible line work closes around the
//!   click, closed groups inside it as islands; the boundary set can be one
//!   layer (Sınır katmanı, picked by an object of it).
//!
//! Islands can be switched off (Adalar). The pattern, the way and the
//! islands stay for as long as the app lives (`Memory`); the boundary layer
//! is the run's. Each hatch is written through `cad.entities.create`, one
//! undo step named “Tarama”. The regions, the faces and the hatch lines are
//! the shared core's.

use kentos_contracts::{CreateOperation, EntityGeometry, HatchPattern, HatchPatternType};
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape, polygon_ring};
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::hatch::hatch_lines;
use kentos_geometry_core::geom::region::{FaceIndex, inside_area, net_area, subtract_areas};
use kentos_geometry_core::ops::areas::{area_of_entity, line_source};
use kentos_geometry_core::tools::point_text::js_trim;

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, wire_all};
use crate::prompt::{Prompt, upper_tr};
use crate::spatial::{measures, slot};
use crate::tool::{self, Context, Flow, Memory, Pointer, Preview, Tool};

/// The tool's id: its command is `tool.hatch`.
pub const ID: &str = "hatch";
pub const LABEL: &str = "Tarama";

/// A pattern the tool offers (the web's `PRESETS`): the spacing in paper millimetres.
struct Preset {
    name: &'static str,
    kind: HatchPatternType,
    angle: f64,
    mm: f64,
}

/// D's cycle, in the web's order.
const PRESETS: [Preset; 4] = [
    Preset {
        name: "Çizgili 45°",
        kind: HatchPatternType::Lines,
        angle: 45.0,
        mm: 3.0,
    },
    Preset {
        name: "Çapraz 45°",
        kind: HatchPatternType::Cross,
        angle: 45.0,
        mm: 3.0,
    },
    Preset {
        name: "Yatay çizgili",
        kind: HatchPatternType::Lines,
        angle: 0.0,
        mm: 2.0,
    },
    Preset {
        name: "Dolu",
        kind: HatchPatternType::Solid,
        angle: 0.0,
        mm: 3.0,
    },
];

/// The most lines a preview draws (the web's 3000); the hatch itself draws them all.
const PREVIEW_LINES: usize = 3000;

/// A pattern's name in the log (the web's `HATCH_PATTERN_LABEL`).
fn pattern_label(kind: HatchPatternType) -> &'static str {
    match kind {
        HatchPatternType::Solid => "Dolu",
        HatchPatternType::Lines => "Çizgili",
        HatchPatternType::Cross => "Çapraz",
    }
}

/// Kinds whose line work bounds faces: fills, texts and dimensions do not
/// (the web's `isBoundaryKind`).
fn bounds_faces(shape: &Shape) -> bool {
    !matches!(
        shape,
        Shape::Point { .. } | Shape::Text { .. } | Shape::Dimension { .. } | Shape::Hatch { .. }
    )
}

/// A ring's points, arcs tessellated (the web's `polygonRing`).
fn ring_points(r: &Ring) -> Vec<Vec2> {
    polygon_ring(&r.pts, r.bulges.as_deref())
}

/// The faces of the visible line work, and what they were built from: the
/// view, the boundary layer and the drawing's revision (the web's
/// `VisibleFaces`, rebuilt when any of them changes).
struct Faces {
    key: ([f64; 4], Option<String>, u64),
    index: FaceIndex,
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
    /// Kapalı nesne: the boundary object's region with its islands cut out,
    /// kept while the drawing stays as it was (revision, object).
    cache: Option<(u64, f64, Vec<Area>)>,
    faces: Option<Faces>,
    /// The region under the cursor, and its hatch lines when the preview draws them.
    hover: Option<Area>,
    hover_lines: Vec<[Vec2; 2]>,
    /// The project's plot scale, as of the last call.
    plot_scale: f64,
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
        self.boundary_name = self.boundary.as_deref().map(|id| {
            cx.doc
                .layers()
                .get(id)
                .map_or_else(|| "—".to_owned(), |n| n.name.clone())
        });
    }

    fn preset(&self) -> &'static Preset {
        &PRESETS[self.memory.hatch_preset % PRESETS.len()]
    }

    /// The pattern as it is written: the spacing in metres at the plot scale.
    fn pattern(&self) -> HatchPattern {
        let p = self.preset();
        HatchPattern {
            kind: p.kind,
            angle: p.angle,
            spacing: p.mm / 1000.0 * self.plot_scale,
        }
    }

    /// The region to fill around `p` (the web's `region`).
    fn region(&mut self, p: Vec2, cx: &Context<'_>) -> Option<Area> {
        if self.memory.hatch_by_lines {
            return self.face_at(p, cx);
        }
        let store = cx.spatial.store();
        let (id, _) = store.enclosing(p)?;
        let item = store.get(id)?;
        let base = area_of_entity(&item.shape)?;
        if !self.memory.hatch_islands {
            return Some(base);
        }
        let revision = cx.doc.revision();
        if self
            .cache
            .as_ref()
            .is_none_or(|(r, cached, _)| *r != revision || *cached != id)
        {
            // Closed objects inside or across it, smaller than it: a block
            // around a parcel is not an island (the web's `islandsOf`).
            let size = net_area(&base);
            let islands: Vec<Area> = store
                .overlapping(&item.bounds, None)
                .into_iter()
                .filter(|it| it.id != id && !matches!(it.shape, Shape::Hatch { .. }))
                .filter_map(|it| area_of_entity(&it.shape))
                .filter(|a| net_area(a) < size * (1.0 - 1e-9))
                .collect();
            let parts = subtract_areas(std::slice::from_ref(&base), &islands);
            self.cache = Some((revision, id, parts));
        }
        self.cache
            .as_ref()
            .and_then(|(_, _, parts)| parts.iter().find(|a| inside_area(a, p)).cloned())
    }

    /// Sınır: çizgiler: the face of the visible line work around `p`.
    fn face_at(&mut self, p: Vec2, cx: &Context<'_>) -> Option<Area> {
        let b = cx.view.visible();
        let key = (
            [b.min_x, b.min_y, b.max_x, b.max_y],
            self.boundary.clone(),
            cx.doc.revision(),
        );
        if self.faces.as_ref().is_none_or(|f| f.key != key) {
            let doc = &*cx.doc;
            let layer = self.boundary.as_deref();
            let lines: Vec<CoreEntity> = cx
                .spatial
                .store()
                .overlapping(&b, None)
                .into_iter()
                .filter(|it| bounds_faces(&it.shape))
                .filter(|it| {
                    layer.is_none_or(|layer| {
                        slot(it.id)
                            .and_then(|s| doc.get(s))
                            .is_some_and(|e| e.base().layer_id == layer)
                    })
                })
                .map(|it| CoreEntity::new(it.shape.clone()))
                .collect();
            self.faces = Some(Faces {
                key,
                index: FaceIndex::new(&[line_source(&lines)]),
            });
        }
        self.faces
            .as_ref()
            .and_then(|f| f.index.at(p, self.memory.hatch_islands))
    }

    /// The region and the lines the preview draws of it.
    fn hover_at(&mut self, p: Vec2, cx: &Context<'_>) {
        self.hover = self.region(p, cx);
        self.hover_lines.clear();
        let pattern = self.pattern();
        if let Some(area) = &self.hover
            && pattern.kind != HatchPatternType::Solid
        {
            let holes: Vec<Vec<Vec2>> = area.holes.iter().map(ring_points).collect();
            let lines = hatch_lines(
                &ring_points(&area.outer),
                pattern.angle,
                pattern.spacing,
                &holes,
            );
            if lines.segments.len() <= PREVIEW_LINES {
                self.hover_lines = lines.segments;
            }
        }
    }

    /// Fills the region around `p` (the web's `pointerDown`).
    fn fill(&mut self, p: Vec2, cx: &mut Context<'_>) {
        let Some(area) = self.region(p, cx) else {
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
        let ring = ring_points(&area.outer);
        let holes: Vec<Vec<Vec2>> = area.holes.iter().map(ring_points).collect();
        let pattern = self.pattern();
        if pattern.kind != HatchPatternType::Solid
            && hatch_lines(&ring, pattern.angle, pattern.spacing, &holes).capped
        {
            cx.say(
                Level::Warn,
                "Desen bu alan için çok sık; çizim ölçeğini büyütün ya da başka bir desen seçin.",
            );
            return;
        }
        let kind = pattern.kind;
        let geometry = EntityGeometry::Hatch {
            ring: wire_all(&ring),
            holes: (!holes.is_empty()).then(|| holes.iter().map(|h| wire_all(h)).collect()),
            pattern,
        };
        let Some(out) = points::write_objects(vec![geometry], Some(CreateOperation::Hatch), cx)
        else {
            return;
        };
        let area = out
            .ids
            .first()
            .and_then(|&id| cx.doc.get(kentos_domain::Slot(id)))
            .and_then(|e| measures(e).0)
            .unwrap_or(0.0);
        let islands = if holes.is_empty() {
            String::new()
        } else {
            format!(", {} ada taranmadı", holes.len())
        };
        let text = format!(
            "{} tarama eklendi: {}{islands}",
            pattern_label(kind),
            cx.format().area(area)
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

impl Tool for Hatch {
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
        let m = self.memory;
        let mut prompt = Prompt::new(LABEL, "taranacak yerin içine tıklayın")
            .option_with("Desen", "D", self.preset().name)
            .option_with(
                "Sınır",
                "B",
                if m.hatch_by_lines {
                    "çizgiler"
                } else {
                    "kapalı nesne"
                },
            )
            .option_with(
                "Adalar",
                "A",
                if m.hatch_islands {
                    "taranmaz"
                } else {
                    "taranır"
                },
            );
        if m.hatch_by_lines {
            let layer = self.boundary_name.clone().unwrap_or_else(|| "tümü".to_owned());
            prompt = prompt.option_with("Sınır katmanı", "K", layer);
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
        self.fill(p.raw, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let m = &mut *cx.memory;
        let done = match upper_tr(js_trim(text)).as_str() {
            "D" => {
                m.hatch_preset = (m.hatch_preset + 1) % PRESETS.len();
                self.changed()
            }
            "B" => {
                m.hatch_by_lines = !m.hatch_by_lines;
                self.picking_layer = false;
                self.changed()
            }
            "A" => {
                m.hatch_islands = !m.hatch_islands;
                self.cache = None;
                self.changed()
            }
            "K" if m.hatch_by_lines || self.picking_layer => {
                if self.picking_layer || self.boundary.is_some() {
                    self.boundary = None;
                    self.picking_layer = false;
                } else {
                    self.picking_layer = true;
                }
                cx.selection.set_hover(None);
                self.changed()
            }
            _ => false,
        };
        self.see(cx);
        done
    }

    /// Enter, Space or a quick right click leave the tool.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    /// Esc leaves the layer picking first.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
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

    /// The region under the cursor, outlined dashed; filled for a solid
    /// pattern, else its lines faint (the web's `draw`).
    fn preview(&self, _format: &Format) -> Preview {
        let Some(area) = self.hover.as_ref().filter(|_| !self.picking_layer) else {
            return Preview::default();
        };
        let solid = self.preset().kind == HatchPatternType::Solid;
        Preview {
            areas: vec![tool::Area {
                rings: std::iter::once(&area.outer)
                    .chain(&area.holes)
                    .map(ring_points)
                    .collect(),
                fill: if solid { 0.25 } else { 0.0 },
                width: 1.5,
                dash: Some([4.0, 3.0]),
            }],
            hatch: self.hover_lines.clone(),
            ..Preview::default()
        }
    }
}
