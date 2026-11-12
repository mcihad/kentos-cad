//! Sheet layouts in the desktop (docs/sheet/design.md §11; the mode itself is
//! `kentos-sheet-ui`): the tabs under the drawing (“Model | Pafta 1 | +”),
//! the contextual Pafta tab, the mode in place of the drawing and its docks
//! while a sheet is in front, its status cells, its keys, the project's
//! values and the maps' content: the drawing's layers through the style
//! engine at the map's scale on the paper's palette, the same paths its PDF
//! writes (sheet_pdf.rs, map_vectors.rs; the web's map frames), and where a
//! layer has no vector form the drawing pipeline's plain scene. The books
//! are kept in the data folder by the project's key until `.kcad` carries
//! them (design §10).

use std::cell::{Cell, RefCell};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::rc::Rc;

use iced::Task;
use iced::widget::canvas::{self, Frame, Path};
use iced::{Point, Size, Vector};
use kentos_commands::DrawingFont;
use kentos_interaction::{Format, Level, Spatial};
use kentos_render_wgpu::precision::join;
use kentos_render_wgpu::scene::{self, ScenePart};
use kentos_render_wgpu::{Camera, Vec2};
use kentos_sheet::display::{CrsInfo, ProjectInfo};
use kentos_sheet::kinds::{GroundPoint, MapLayers};
use kentos_sheet::profile::Capabilities;
use kentos_sheet_ui::painter::{clip_polygon, clip_segment};
use kentos_sheet_ui::{Context, Effect, ExportKind, MapPainter, MapRequest, Painter, Say, Store};

use crate::app::{App, Message};
use crate::document::Document;
use crate::viewport::{Canvas, palette};

mod dashes;

/// The Pafta tab's id in the ribbon (the web's `SHEET_TAB_ID`).
pub const SHEET_TAB: &str = "sheet";
/// The curves of a map's content: chords within 5 cm on the ground (a quarter
/// pixel of a 1:1000 map at its real size), at most this many in all.
const CURVE_TOLERANCE_M: f64 = 0.05;
const CURVE_BUDGET: usize = 2_000_000;
/// The drawing's text in a map is laid out at least this many pixels a side, as the web draws a
/// map's picture (app/sheet/mapFrames.ts `MIN_SIDE`): a small map's labels are a larger one's, shrunk.
const MAP_LABEL_SIDE: f64 = 640.0;

/// Where the books, this device's templates and the pictures are kept: beside the recovery copies.
pub fn default_store() -> Option<Store> {
    let root = crate::recovery::default_root()?;
    Store::open(&root.with_file_name("pafta")).ok()
}

/// The drawing as its maps show it: its layers through the style engine at a
/// map's scale and local centre (a bounded cache for each drawing state), and
/// the drawing pipeline's plain scene of the open document on paper's palette
/// for a map whose layers have no vector form.
#[derive(Default)]
pub struct SheetMaps {
    annotation_scales: RefCell<Vec<f64>>,
    scene: RefCell<Option<MapScene>>,
    /// The styled layers at the scales the maps were last drawn at (a few kept).
    styled: RefCell<Vec<Rc<StyledAt>>>,
    /// The drawing as the maps at a few scales draw it, its annotative objects as each scale says
    /// (docs/adr/0323 §2); none at a scale where that is the drawing itself.
    scaled: RefCell<Vec<Rc<Scaled>>>,
    /// Counts the scenes built: the maps are painted again when it changes.
    built: Cell<u64>,
}

/// The shown layers through the style engine at one scale on the paper's palette.
struct StyledAt {
    /// The drawing (its session and state), the library's version, the scale and the map's own layers (its layer
    /// state and overrides, docs/adr/0314 §1; 0 for none), and the local viewport origin.
    key: (u64, u64, u64, u64, u64, u64, u64, u64),
    origin: Vec2,
    /// Each shown layer's id and name, and its build.
    layers: Vec<(String, String, kentos_native_style::batches::StyledLayer)>,
}

/// Styled layers kept for this many scales.
const STYLED_KEPT: usize = 4;

/// The drawing (its session and state), the scale and annotation scale, whether the screen's store draws otherwise
/// than paper, the bands' rows it took (docs/adr/0636 §2) and the level whose plan it is when that shows stairs from
/// above, and the wall detail its linked view asks for (docs/adr/0781 §1).
/// The design options it shows come last (docs/adr/0788 §3).
type ScaledKey = (
    u64,
    u64,
    u64,
    u64,
    bool,
    usize,
    Option<(String, bool)>,
    Option<kentos_commands::WallDetail>,
    Vec<kentos_commands::OptionChoice>,
    // The map's content turn (docs/adr/0982: a road's texts read left to right on it).
    kentos_sheet::units::Mdeg,
    // A road's map: its centre and size, which keep the curves' data inside it (docs/adr/0984).
    Option<(u64, u64, i32, i32)>,
    // A map showing a phase other than the drawing's (docs/adr/8039 §2).
    Option<kentos_commands::PhaseView>,
);

/// The drawing as the maps at one scale draw it (docs/adr/0323 §2): a reading copy with the annotative
/// texts, dimensions and leaders as that scale says, and its geometry store; none when that is the drawing. A level's
/// plan showing the stairs under it from above (docs/adr/0724 §4) has its own, its store's level view the map's.
struct Scaled {
    key: ScaledKey,
    view: Option<(Document, Spatial)>,
}

struct MapScene {
    session: u64,
    generation: u64,
    origin: Vec2,
    parts: [ScenePart; 2],
}

impl SheetMaps {
    /// Builds the scene again when the open drawing changed since the last one.
    pub fn sync(&self, doc: Option<&Document>) {
        let Some(doc) = doc else {
            self.annotation_scales.borrow_mut().clear();
            if self.scene.borrow_mut().take().is_some() {
                self.built.set(self.built.get() + 1);
            }
            return;
        };
        let generation = doc.model.generation();
        if self
            .scene
            .borrow()
            .as_ref()
            .is_some_and(|s| s.session == doc.session && s.generation == generation)
        {
            return;
        }
        self.styled.borrow_mut().clear();
        self.scaled.borrow_mut().clear();
        *self.annotation_scales.borrow_mut() = doc.model.annotation_scales();
        let paper = palette(Canvas::Paper);
        let origin = scene::scene_origin(doc);
        let fixed = scene::build_fixed(doc, &paper, origin);
        let curves = scene::build_curves(doc, &paper, origin, CURVE_TOLERANCE_M, CURVE_BUDGET);
        *self.scene.borrow_mut() = Some(MapScene {
            session: doc.session,
            generation,
            origin,
            parts: [fixed, curves],
        });
        self.built.set(self.built.get() + 1);
    }
}

impl SheetMaps {
    /// The drawing as the maps at `scale` draw it (docs/adr/0323 §2), from the copies kept; none when
    /// that is the drawing itself (no annotative object draws otherwise there).
    fn scaled(
        &self,
        drawing: &Document,
        view: &kentos_sheet::display::MapViewPrim,
        // The map's content rectangle (paper micrometres): a road's curve data stays inside it (docs/adr/0984).
        clip: Option<(i32, i32)>,
        level: Option<&kentos_sheet::kinds::MapLevel>,
        detail: Option<kentos_commands::WallDetail>,
        options: &[kentos_commands::OptionChoice],
        // The map's own phase view (docs/adr/0774 §3); none: the drawing's.
        phase: Option<kentos_commands::PhaseView>,
        main: &Spatial,
    ) -> Option<Rc<Scaled>> {
        // A model camera can draw more detail than this paper map, even at the same plot scale.
        // Such a map gets its own index so screen navigation cannot change paper/PDF output (ADR 0696).
        // The screen's stairs seen from above (docs/adr/0724 §4) are not the paper's either.
        let unscoped = main.draws_otherwise()
            || (drawing.model.has_scale_dependent_geometry()
                && main.store().wall_view()
                    != kentos_native_application::walls::view_with(
                        drawing.model.settings(),
                        view.scale,
                        detail,
                    ));
        let scale = view.scale;
        let annotation = view.annotation_denominator();
        // A map showing a phase other than the drawing's joins and cuts its walls as that phase stands
        // (docs/adr/8039 §2).
        let drawn = drawing
            .model
            .settings()
            .arch
            .as_ref()
            .and_then(|a| a.phase_view)
            .unwrap_or_default();
        let phased = phase.filter(|v| *v != drawn);
        let axes = kentos_native_application::road_plan_axes::has_axes(&drawing.model);
        let room = match (axes, view.center, clip) {
            (true, Some(c), Some((w, h))) => Some(
                kentos_native_application::road_plan_axes::Room::of_map([c.x, c.y], w, h, scale),
            ),
            _ => None,
        };
        let key = (
            drawing.session,
            drawing.model.generation(),
            scale.to_bits(),
            annotation.to_bits(),
            unscoped,
            main.ghost_total(),
            // A level's plan showing the stairs under it from above (docs/adr/0724 §4) or the slabs over it dashed
            // (docs/adr/8012, step 3).
            level
                .filter(|l| kentos_plot::layers::shows_from_above(&drawing.model, l))
                .map(|l| (l.id.clone(), l.below)),
            // A linked view's own wall detail (docs/adr/0781 §1).
            detail,
            // A linked view's design options (docs/adr/0788 §3): their walls join, the primaries' do not.
            options.to_vec(),
            // A road project's alignments, drawn for the map's paper turn (docs/adr/0982).
            view.rotation,
            room.map(|r| {
                (
                    r.center[0].to_bits(),
                    r.center[1].to_bits(),
                    clip.map_or(0, |c| c.0),
                    clip.map_or(0, |c| c.1),
                )
            }),
            phased,
        );
        let upper = key.6.is_some().then_some(level).flatten();
        let kept = self.scaled.borrow().iter().find(|s| s.key == key).cloned();
        let at = kept.unwrap_or_else(|| {
            let at = scale.max(1e-6);
            // Walls at a detail other than the drawing's (docs/adr/0339 §4) want a copy of their own too.
            let walls = Spatial::walls_differ_with(&drawing.model, at, detail);
            let view = drawing
                .model
                .has_annotative()
                .then(|| drawing.model.at_scale(annotation))
                .flatten()
                .or_else(|| {
                    (walls
                        || unscoped
                        || axes
                        || upper.is_some()
                        || !options.is_empty()
                        || phased.is_some())
                    .then(|| drawing.model.reading_copy())
                })
                .map(|model| match phased {
                    Some(v) => model.with_phase_view(v),
                    None => model,
                })
                // A road project's alignments on a layer of their own (docs/adr/0982): the model has no object for them.
                .map(|model| {
                    if !axes {
                        return model;
                    }
                    let turn = f64::from(view.rotation) / 1000.0;
                    let kept = model.reading_copy();
                    kentos_native_application::road_plan_axes::with_axes_on(model, annotation, turn, room)
                        .unwrap_or(kept)
                })
                .map(|model| {
                    let mut spatial = Spatial::new();
                    spatial.ignore_object_isolation();
                    spatial.set_wall_scale(Some(at));
                    spatial.set_wall_detail(detail);
                    spatial.set_option_view(options.to_vec());
                    spatial.reload(&model);
                    if let Some(l) = upper {
                        spatial.set_level_view(
                            &model,
                            kentos_plot::layers::level_view(l),
                            Some(l.id.clone()),
                        );
                    }
                    // The rows round a bound layer's part hold room in the copy too (docs/adr/0636 §2).
                    for link in model
                        .settings()
                        .sources
                        .iter()
                        .filter(|l| l.extent.is_some())
                    {
                        spatial.set_ghosts(&link.layer, main.ghosts_of(&link.layer));
                    }
                    let copy = Document {
                        model,
                        path: None,
                        source: crate::document::Source::Local,
                        session: drawing.session,
                        legacy: false,
                    };
                    (copy, spatial)
                });
            let at = Rc::new(Scaled { key, view });
            let mut kept = self.scaled.borrow_mut();
            kept.insert(0, Rc::clone(&at));
            kept.truncate(STYLED_KEPT);
            at
        });
        at.view.is_some().then_some(at)
    }

    /// The shown layers through the style engine at `scale` on the paper's palette, from the
    /// ones kept when nothing they depend on changed. Construction lines are cut to the
    /// drawing's box and a margin (the maps cut them again to their frames).
    #[allow(clippy::too_many_arguments)]
    fn styled_at(
        &self,
        doc: &Document,
        store: &kentos_geometry_core::store::Store,
        library: &kentos_native_style::library::StyleLibrary,
        view: &kentos_sheet::display::MapViewPrim,
        own: (&MapLayers, &[kentos_sheet::kinds::LayerOverride]),
        level: Option<&kentos_sheet::kinds::MapLevel>,
        recipe: (
            Option<kentos_commands::PhaseView>,
            &[kentos_commands::ViewCategory],
            &[kentos_commands::RecipePen],
            bool,
            &[kentos_commands::OptionChoice],
        ),
    ) -> Rc<StyledAt> {
        let scale = view.scale;
        let origin = view
            .center
            .map_or_else(|| scene::scene_origin(doc), |p| Vec2::new(p.x, p.y));
        let own_key = match (own, level) {
            _ if recipe.0.is_some() || !recipe.1.is_empty() || !recipe.2.is_empty() || recipe.3 || !recipe.4.is_empty() => {
                use std::hash::{Hash, Hasher};
                let mut h = std::collections::hash_map::DefaultHasher::new();
                serde_json::to_string(&own.0).unwrap_or_default().hash(&mut h);
                serde_json::to_string(own.1).unwrap_or_default().hash(&mut h);
                level.map(|l| (l.id.as_str(), l.below)).hash(&mut h);
                // A linked view's recipe (docs/adr/0774 §3).
                serde_json::to_string(&recipe).unwrap_or_default().hash(&mut h);
                h.finish() | 1
            }
            ((MapLayers::Theme(_), _), _) | ((_, [_, ..]), _) | (_, Some(_)) => {
                use std::hash::{Hash, Hasher};
                let mut h = std::collections::hash_map::DefaultHasher::new();
                serde_json::to_string(&own.0)
                    .unwrap_or_default()
                    .hash(&mut h);
                serde_json::to_string(own.1)
                    .unwrap_or_default()
                    .hash(&mut h);
                // The map's level (docs/adr/0716).
                level.map(|l| (l.id.as_str(), l.below)).hash(&mut h);
                h.finish() | 1
            }
            _ => 0,
        };
        // The walls' detail the store draws (a linked view's own, docs/adr/0781 §1): two maps at one scale may differ.
        let walls = {
            use kentos_geometry_core::geom::wall::Detail;
            match store.wall_view().detail {
                Detail::Schematic => 1,
                Detail::Standard => 2,
                Detail::Detailed => 3,
            }
        };
        let key = (
            doc.session,
            doc.model.generation(),
            library.version(),
            scale.to_bits(),
            view.annotation_denominator().to_bits(),
            own_key.wrapping_mul(4).wrapping_add(walls),
            origin.x.to_bits(),
            origin.y.to_bits(),
        );
        if let Some(at) = self.styled.borrow().iter().find(|a| a.key == key) {
            return Rc::clone(at);
        }
        // The map's level (docs/adr/0716): the other levels' objects left out, the one under it faint.
        let (left_out, faint) = level
            .map(|l| kentos_plot::layers::level_looks(&doc.model, l))
            .unwrap_or_default();
        let look = crate::style::scene::Look {
            line_types: std::sync::Arc::new(
                kentos_native_style::line_types::LineTypes::at_scale_with_resources(
                    doc.model.settings(),
                    doc.model.styles(),
                    scale,
                ),
            ),
            palette: crate::sheet_pdf::paper_style(),
            symbol_scale: scale.max(1e-6),
            screen: false,
            hairlines: false,
            hold_weights: false,
            origin,
            // CAD line types along the source curve, as the model and the PDF (docs/adr/1122).
            view_build: kentos_native_style::View {
                cad_phase: true,
                ..Default::default()
            },
            view_colors: Default::default(),
            reading_unit: kentos_native_style::reading_unit(doc.model.settings()),
            phasing: std::sync::Arc::new({
                // A level's plan (docs/adr/0716): the level under it faint; a linked view's phase and categories
                // (docs/adr/0774 §3).
                let mut p =
                    kentos_plot::layers::map_drawing_of(&doc.model, recipe.0, recipe.1, recipe.2, recipe.3, recipe.4);
                for &id in &faint {
                    p.fade(id);
                }
                p
            }),
        };
        let clip = scene::extents(doc).map_or(
            kentos_render_wgpu::Bounds {
                min_x: origin.x - 1.0e5,
                min_y: origin.y - 1.0e5,
                max_x: origin.x + 1.0e5,
                max_y: origin.y + 1.0e5,
            },
            |b| {
                let m = (b.max_x - b.min_x).max(b.max_y - b.min_y).max(1.0);
                kentos_render_wgpu::Bounds {
                    min_x: b.min_x - m,
                    min_y: b.min_y - m,
                    max_x: b.max_x + m,
                    max_y: b.max_y + m,
                }
            },
        );
        let mut names = std::collections::HashMap::new();
        crate::style::scene::names(doc.model.layers().nodes(), &mut names);
        let mut styles = std::collections::HashMap::new();
        kentos_plot::layers::styles(doc.model.layers().nodes(), &mut styles);
        let mut layers = Vec::new();
        // What a sheet prints: Yazdır off leaves a layer out; its objects in their drawing order (docs/adr/0260).
        // The map's layer state and overrides (docs/adr/0314 §1): its own layers, the drawing's untouched.
        for node in kentos_plot::layers::map_layers(
            doc.model.layers().nodes(),
            &doc.model.settings().layer_states,
            own.0,
            own.1,
        ) {
            let node: &kentos_commands::LayerNode = &node;
            let objects: Vec<&kentos_commands::Entity> = kentos_plot::layers::in_order(
                doc.model
                    .by_layer(&node.id)
                    .filter(|e| !left_out.contains(&e.base().id))
                    .collect(),
            );
            if objects.is_empty() {
                continue;
            }
            let styled = crate::style::scene::build_whole(
                store,
                library,
                node,
                &objects,
                &look,
                clip,
                kentos_plot::layers::LayerTables {
                    names: &names,
                    styles: &styles,
                },
            )
            .unwrap_or_default();
            layers.push((node.id.clone(), node.name.clone(), styled));
        }
        let at = Rc::new(StyledAt {
            key,
            origin,
            layers,
        });
        let mut kept = self.styled.borrow_mut();
        kept.insert(0, Rc::clone(&at));
        kept.truncate(STYLED_KEPT);
        at
    }
}

/// A colour of the core's (`#rrggbb`, `#rrggbbaa`).
fn hex_color(c: &str) -> Option<iced::Color> {
    let h = c.strip_prefix('#')?;
    let v = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    match h.len() {
        6 => Some(iced::Color::from_rgb8(v(0)?, v(2)?, v(4)?)),
        8 => Some(iced::Color::from_rgba8(
            v(0)?,
            v(2)?,
            v(4)?,
            f32::from(v(6)?) / 255.0,
        )),
        _ => None,
    }
}

/// A polyline cut to the box into the runs inside it: a run goes on while its segments do (a
/// dashed line keeps its dashes going), and starts again where it comes back in.
fn clip_runs(pts: &[Point], bounds: iced::Rectangle) -> Vec<Vec<Point>> {
    let mut runs: Vec<Vec<Point>> = Vec::new();
    let mut open = false;
    for w in pts.windows(2) {
        match clip_segment(w[0], w[1], bounds) {
            Some((a, z)) => {
                let goes_on = open && a == w[0] && runs.last().and_then(|r| r.last()) == Some(&a);
                if goes_on {
                    if let Some(r) = runs.last_mut() {
                        r.push(z);
                    }
                } else {
                    runs.push(vec![a, z]);
                }
                // The run goes on only where this segment ends inside the box.
                open = z == w[1];
            }
            None => open = false,
        }
    }
    runs
}

/// Prepare the whole map before painting: an excessive/dense/dotted recipe
/// falls back as a whole to the existing CPU painter, without double compositing.
fn stroke_runs(
    r: &MapRequest<'_>, paths: &[kentos_sheet::pdf::MapPath], px_mm: f64,
) -> Option<Vec<Vec<Vec<Point>>>> {
    paths.iter().map(|p| {
        let Some(stroke) = &p.stroke else { return Some(Vec::new()); };
        let dash: Vec<f64> = stroke.dash.iter().map(|v| v * px_mm).collect();
        let phase = stroke.dash_offset.unwrap_or(0.) * px_mm;
        let mut out = Vec::new();
        for ring in std::iter::once(&p.points).chain(p.holes.iter().filter(|_| p.closed)) {
            let camera = r.transform();
            let mut points: Vec<Vec2> = ring.iter().map(|q| {
                let p = camera.point(*q); Vec2::new(p[0],p[1])
            }).collect();
            if p.closed && let Some(first) = points.first().copied() { points.push(first); }
            if dash.is_empty() {
                let points: Vec<Point> = points.iter().map(|p|Point::new(p.x as f32,p.y as f32)).collect();
                out.extend(clip_runs(&points,r.bounds()));
            } else {
                out.extend(dashes::runs(&points,r.bounds(),&dash,phase)?);
            }
        }
        Some(out)
    }).collect()
}

/// The core's map paths on a canvas, as its PDF writer draws them and cut to the map's box as
/// the PDF's clip cuts them (a turned frame's canvas cannot cut to its turned box): areas filled
/// (even-odd with holes), lines at their paper width (`px_mm` pixels a paper mm, at least
/// `thinnest`), dashed, capped and joined as styled.
fn draw_paths(
    frame: &mut Frame,
    r: &MapRequest<'_>,
    paths: &[kentos_sheet::pdf::MapPath],
    strokes: &[Vec<Vec<Point>>],
    px_mm: f32,
    thinnest: f32,
) {
    use iced::widget::canvas::{Fill, LineCap, LineJoin, Stroke, Style, fill};
    let bounds = r.bounds();
    let px = |pts: &[[f64; 2]]| -> Vec<Point> { pts.iter().map(|q| r.to_px(q[0], q[1])).collect() };
    for (p, lines) in paths.iter().zip(strokes) {
        if p.points.len() < 2 {
            continue;
        }
        let outer = px(&p.points);
        if p.closed
            && let Some(c) = p.fill.as_deref().and_then(hex_color)
        {
            let rings: Vec<Vec<Point>> = std::iter::once(outer.clone())
                .chain(p.holes.iter().map(|h| px(h)))
                .map(|ring| clip_polygon(&ring, bounds))
                .filter(|ring| ring.len() >= 3)
                .collect();
            if !rings.is_empty() {
                let area = Path::new(|b| {
                    for ring in &rings {
                        b.move_to(ring[0]);
                        for q in &ring[1..] {
                            b.line_to(*q);
                        }
                        b.close();
                    }
                });
                frame.fill(
                    &area,
                    Fill {
                        style: Style::Solid(c),
                        rule: if rings.len() > 1 {
                            fill::Rule::EvenOdd
                        } else {
                            fill::Rule::NonZero
                        },
                    },
                );
            }
        }
        if let Some(s) = &p.stroke
            && let Some(c) = hex_color(&s.color)
        {
            if lines.is_empty() {
                continue;
            }
            let path = Path::new(|b| {
                for run in lines {
                    b.move_to(run[0]);
                    for q in &run[1..] {
                        b.line_to(*q);
                    }
                }
            });
            frame.stroke(
                &path,
                Stroke {
                    style: Style::Solid(c),
                    width: (s.width as f32 * px_mm).max(thinnest),
                    line_cap: match s.cap {
                        kentos_sheet::style::LineCap::Butt => LineCap::Butt,
                        kentos_sheet::style::LineCap::Round => LineCap::Round,
                        kentos_sheet::style::LineCap::Square => LineCap::Square,
                    },
                    line_join: match s.join {
                        kentos_sheet::style::LineJoin::Round => LineJoin::Round,
                        kentos_sheet::style::LineJoin::Miter => LineJoin::Miter,
                        kentos_sheet::style::LineJoin::Bevel => LineJoin::Bevel,
                    },
                    line_dash: Default::default(),
                },
            );
        }
    }
}

fn color(c: [u8; 4]) -> iced::Color {
    iced::Color::from_rgba8(c[0], c[1], c[2], f32::from(c[3]) / 255.0)
}

/// The maps' content as the sheet mode paints it now: the drawing
/// pipeline's scene and the drawing's text (texts, dimension values, names:
/// what the drawing area writes, labels.rs), as the web's map frames draw
/// them. It borrows the open drawing for as long as the view lives.
pub(crate) struct SheetPainter<'a> {
    book: Option<&'a kentos_sheet::SheetBook>,
    sheet: Option<&'a str>,
    interaction: Option<&'a App>,
    maps: &'a SheetMaps,
    doc: Option<&'a kentos_domain::Document>,
    /// The open drawing as the drawing pipeline reads it (a PDF's vectors).
    drawing: Option<&'a Document>,
    spatial: &'a Spatial,
    /// The style library the drawing's layers are built with (a PDF's vectors).
    library: &'a kentos_native_style::library::StyleLibrary,
    /// The style atlas's pictures and glyphs (a map drawn as a picture: its picture and text
    /// markers, its picture fills).
    images: &'a crate::style::images::Images,
    format: Format,
    font: DrawingFont,
}

impl MapPainter for SheetPainter<'_> {
    fn interaction(&self, r: &MapRequest<'_>, frame: &mut Frame) {
        crate::sheet_model::paint(self.interaction, r, frame);
    }
    fn paint(&self, r: &MapRequest<'_>, frame: &mut Frame) -> bool {
        if let Some(primitive) = r.primitive
            && (r.export || !primitive.clip_shape.is_frame() || self.plot_profile(r).is_some())
        {
            let mut keep = None;
            let (drawing, spatial) = self.source(r, &mut keep);
            let Some(doc) = drawing.map(|d| &d.model).or(self.doc) else {
                return false;
            };
            let Some(center) = r.view.center else {
                return false;
            };
            let src = kentos_plot::maps::MapSource {
                map: None,
                level: primitive.level.as_ref(),
                phase: primitive.phase,
                hidden: &primitive.hidden,
                pens: &primitive.pens,
                mono: primitive.mono,
                options: &primitive.options,
                doc,
                store: spatial.store(),
                library: self.library,
                origin: Vec2::new(center.x, center.y),
            };
            let labels = kentos_plot::labels::map_labels(doc, spatial, primitive);
            let cap = if r.export { 8192. } else { 4096. };
            let factor = (cap / f64::from(r.size.width.max(r.size.height)).max(1.)).min(1.);
            let size = [
                (f64::from(r.size.width) * factor).ceil().max(1.) as u32,
                (f64::from(r.size.height) * factor).ceil().max(1.) as u32,
            ];
            let path = self
                .drawing
                .and_then(|d| d.path.as_ref())
                .and_then(|p| p.to_str());
            let Ok(image) = kentos_plot::render_map_image_profile(
                &src,
                primitive,
                &labels,
                size,
                path,
                self.plot_profile(r),
            ) else {
                return false;
            };
            kentos_sheet_ui::paint::draw_pixels(
                frame,
                r.bounds(),
                image.width,
                image.height,
                image.rgba,
            );
            return true;
        }
        // The styled vectors its PDF writes; where a layer has none, the plain scene.
        if !self.paint_styled(r, frame) && !self.maps.paint_scene(r, frame) {
            return false;
        }
        let mut keep = None;
        let (drawing, spatial) = self.source(r, &mut keep);
        if let Some(doc) = drawing.map(|d| &d.model).or(self.doc) {
            self.labels(doc, spatial, r, frame);
        }
        true
    }

    fn revision(&self) -> u64 {
        let mut h = DefaultHasher::new();
        (
            self.maps.revision(),
            self.library.version(),
            self.font as u8,
            format!("{:?}", self.format),
        )
            .hash(&mut h);
        // The drawing and the sources its bound layers last matched (docs/adr/0533 §4): the labels of a map are
        // placed against them, at the map's own scale (the request's).
        if let Some(d) = self.drawing {
            (d.session, d.model.generation()).hash(&mut h);
            crate::labels::sources_key(&d.model).hash(&mut h);
        }
        h.finish()
    }

    /// The map as the drawing's vectors and text (sheet_pdf.rs): the PDF's map content; none
    /// where a layer draws something with no vector form (the map then goes as a picture).
    fn vector(&self, r: &MapRequest<'_>) -> Option<kentos_sheet::pdf::MapContent> {
        self.export_content(r).ok().flatten()
    }

    fn export_content(
        &self,
        r: &MapRequest<'_>,
    ) -> Result<Option<kentos_sheet::pdf::MapContent>, String> {
        let mut keep = None;
        let (drawing, spatial) = self.source(r, &mut keep);
        let Some(drawing) = drawing else {
            return Ok(None);
        };
        let doc = &drawing.model;
        if r.view.center.is_none() {
            return Ok(None);
        }
        let Some(extent) = r.extent else {
            return Ok(None);
        };
        let level = r.primitive.and_then(|p| p.level.as_ref());
        let texts = kentos_plot::labels::labels_in_view(
            doc,
            spatial,
            r.view,
            r.layers,
            r.overrides,
            level,
            (
                r.primitive.and_then(|p| p.phase),
                r.primitive.map_or(&[][..], |p| p.hidden.as_slice()),
                r.primitive.map_or(&[][..], |p| p.pens.as_slice()),
                r.primitive.is_some_and(|p| p.mono),
                r.primitive.map_or(&[][..], |p| &p.options[..]),
            ),
            r.extent,
        );
        let src = kentos_plot::maps::MapSource {
            map: None,
            level,
            phase: r.primitive.and_then(|p| p.phase),
            hidden: r.primitive.map_or(&[][..], |p| p.hidden.as_slice()),
            pens: r.primitive.map_or(&[][..], |p| p.pens.as_slice()),
            mono: r.primitive.is_some_and(|p| p.mono),
            options: r.primitive.map_or(&[][..], |p| &p.options[..]),
            doc: &drawing.model,
            store: spatial.store(),
            library: self.library,
            origin: r.view.center.map_or_else(
                || kentos_render_wgpu::scene::scene_origin(drawing),
                |p| Vec2::new(p.x, p.y),
            ),
        };
        kentos_plot::maps::collect_vector_profile(
            &src,
            r.layers,
            r.overrides,
            r.view.scale,
            extent,
            &texts,
            self.plot_profile(r),
        )
        .map(Some)
        .or_else(|e| match e {
            kentos_plot::maps::VectorIssue::Unsupported(_) => Ok(None),
            kentos_plot::maps::VectorIssue::Invalid(message) => Err(message),
        })
    }

    fn vectors(&self) -> bool {
        self.drawing.is_some()
    }

    fn no_vectors(&self, r: &MapRequest<'_>) -> Option<String> {
        self.why_no_vectors(r)
    }
}

/// The maps' painter of a drawing, from the app's parts (an export borrows them beside the sheet mode).
pub(crate) fn painter_of<'a>(
    maps: &'a SheetMaps,
    doc: Option<&'a Document>,
    spatial: &'a Spatial,
    library: &'a kentos_native_style::library::StyleLibrary,
    images: &'a crate::style::images::Images,
) -> SheetPainter<'a> {
    SheetPainter {
        book: None,
        sheet: None,
        interaction: None,
        maps,
        doc: doc.map(|d| &d.model),
        drawing: doc,
        spatial,
        library,
        images,
        format: doc.map_or_else(Format::default, |d| Format::of(d.settings())),
        font: doc
            .and_then(|d| d.settings().drawing_font)
            .unwrap_or(DrawingFont::Barlow),
    }
}

/// What a map draws from: the drawing and its geometry store, or their copies at the map's scale.
type Source<'s> = (Option<&'s Document>, &'s Spatial);

impl SheetPainter<'_> {
    fn plot_profile(&self, r: &MapRequest<'_>) -> Option<&kentos_sheet::plot_profile::PlotProfile> {
        if !r.export {
            return None;
        }
        kentos_sheet::plot_profile::for_sheet(self.book?, r.sheet.or(self.sheet)?)
            .ok()
            .flatten()
    }

    /// The drawing and store a map at `scale` draws from (docs/adr/0323 §2): the copy at that scale
    /// when an annotative object draws otherwise there; `keep` holds the copy while it is used.
    fn source<'s>(&'s self, r: &MapRequest<'_>, keep: &'s mut Option<Rc<Scaled>>) -> Source<'s> {
        let level = r.primitive.and_then(|m| m.level.as_ref());
        let detail = r.primitive.and_then(|m| m.wall_detail);
        let options = r.primitive.map_or(&[][..], |m| &m.options[..]);
        let phase = r.primitive.and_then(|m| m.phase);
        *keep = self.drawing.and_then(|d| {
            let clip = r.primitive.map(|m| (m.clip.width, m.clip.height));
            self.maps
                .scaled(d, r.view, clip, level, detail, options, phase, self.spatial)
        });
        match keep.as_ref().and_then(|s| s.view.as_ref()) {
            Some((d, sp)) => (Some(d), sp),
            None => (self.drawing, self.spatial),
        }
    }

    /// The map's drawing as its PDF draws it (the styled layers at its scale, what reaches its
    /// box); where a shown layer has something no vector writes (a pattern or picture fill, a
    /// picture or text marker, a soft-edged line), the whole map as the styled shaders draw it,
    /// on the CPU ([`Self::paint_picture`]): what its PDF then embeds. False where it cannot (no
    /// drawing, no place).
    fn paint_styled(&self, r: &MapRequest<'_>, frame: &mut Frame) -> bool {
        let mut keep = None;
        let (drawing, spatial) = self.source(r, &mut keep);
        let (Some(drawing), Some(extent)) = (drawing, r.extent) else {
            return false;
        };
        if r.view.center.is_none() {
            return false;
        }
        let scale = r.view.scale.max(1e-6);
        let at = self.maps.styled_at(
            drawing,
            spatial.store(),
            self.library,
            r.view,
            (r.layers, r.overrides),
            r.primitive.and_then(|p| p.level.as_ref()),
            (
                r.primitive.and_then(|p| p.phase),
                r.primitive.map_or(&[][..], |p| p.hidden.as_slice()),
                r.primitive.map_or(&[][..], |p| p.pens.as_slice()),
                r.primitive.is_some_and(|p| p.mono),
                r.primitive.map_or(&[][..], |p| &p.options[..]),
            ),
        );
        let s = scale;
        let pad = kentos_sheet::projection::Scale::for_rendering(s).model_metres_from_mm(20.);
        let reach = [
            extent[0] - pad,
            extent[1] - pad,
            extent[2] + pad,
            extent[3] + pad,
        ];
        let o = kentos_plot::vectors::VectorScale {
            scale: s,
            origin: [at.origin.x, at.origin.y],
        };
        let only = match r.layers {
            MapLayers::List(l) => Some(&l.layers),
            _ => None,
        };
        // Pixels a paper mm; a line never thinner than the screen shows (a printed one as it is).
        let px_mm = kentos_sheet::projection::Scale::for_rendering(s)
            .paper_pixels_per_mm(r.pixels_per_metre);
        let thinnest = if r.export { 0.25 } else { 0.6 };
        let mut drawn = Vec::new();
        for (id, _, layer) in &at.layers {
            if only.is_some_and(|l| !l.contains(id)) {
                continue;
            }
            let (paths, missing) = kentos_plot::vectors::layer_paths(layer, &o, Some(reach));
            // A layer with no vector form here: the map as a picture, styled all the same.
            if !missing.is_empty() {
                return self.paint_picture(r, frame, &at, only, s);
            }
            drawn.push(paths);
        }
        let strokes: Option<Vec<_>> = drawn.iter().map(|paths| stroke_runs(r, paths, px_mm)).collect();
        let Some(strokes) = strokes else {
            return self.paint_picture(r, frame, &at, only, s);
        };
        for (paths, strokes) in drawn.iter().zip(&strokes) {
            draw_paths(frame, r, paths, strokes, px_mm as f32, thinnest);
        }
        true
    }

    /// The map's shown layers as a picture the styled shaders' CPU twin draws (render-wgpu
    /// `styled::cpu`: the drawing area's look, the web's map pictures' look), at the frame's
    /// pixels (a screen's at most 4096 a side, an export's 8192), drawn over the frame.
    fn paint_picture(
        &self,
        r: &MapRequest<'_>,
        frame: &mut Frame,
        at: &StyledAt,
        only: Option<&Vec<String>>,
        scale: f64,
    ) -> bool {
        use kentos_render_wgpu::styled::cpu::{CpuView, paint};
        let Some(c) = r.view.center else {
            return false;
        };
        let (w, h) = (f64::from(r.size.width), f64::from(r.size.height));
        let cap = if r.export { 8192.0 } else { 4096.0 };
        let f = (cap / w.max(h).max(1.0)).min(1.0);
        let (pw, ph) = (
            (w * f).ceil().max(1.0) as u32,
            (h * f).ceil().max(1.0) as u32,
        );
        // `MapRequest::to_px`, for metres from the layers' origin, at the picture's pixels.
        let ppm = r.pixels_per_metre * f;
        let camera = kentos_sheet::projection::Transform::at_resolution(
            c,
            [f64::from(pw) / 2., f64::from(ph) / 2.],
            ppm,
            r.view.rotation,
        );
        let (m, t) = camera.relative_affine([at.origin.x, at.origin.y]);
        let view = CpuView {
            m,
            t,
            px_per_m: ppm,
            dpr: kentos_sheet::projection::Scale::for_rendering(scale).paper_pixels_per_mm(ppm)
                * 25.4
                / 96.,
            scale,
        };
        let layers: Vec<&kentos_native_style::batches::StyledLayer> = at
            .layers
            .iter()
            .filter(|(id, ..)| only.is_none_or(|l| l.contains(id)))
            .map(|(_, _, layer)| layer)
            .collect();
        // The pictures its builds made (heat maps, docs/adr/0213 §2.6).
        for layer in &layers {
            self.images.put_made(layer);
        }
        let Some(rgba) = paint(pw, ph, &view, &layers, self.images) else {
            return false;
        };
        kentos_sheet_ui::paint::draw_pixels(frame, r.bounds(), pw, ph, rgba);
        true
    }

    /// The layers (and what of them) that keep a map from going to a PDF as vectors, as the web's
    /// export window says it (“Yapı (desen dolgusu)”); none when it goes as vectors.
    fn why_no_vectors(&self, r: &MapRequest<'_>) -> Option<String> {
        let mut keep = None;
        let (drawing, spatial) = self.source(r, &mut keep);
        let drawing = drawing?;
        let extent = r.extent?;
        let scale = r.view.scale.max(1e-6);
        let at = self.maps.styled_at(
            drawing,
            spatial.store(),
            self.library,
            r.view,
            (r.layers, r.overrides),
            r.primitive.and_then(|p| p.level.as_ref()),
            (
                r.primitive.and_then(|p| p.phase),
                r.primitive.map_or(&[][..], |p| p.hidden.as_slice()),
                r.primitive.map_or(&[][..], |p| p.pens.as_slice()),
                r.primitive.is_some_and(|p| p.mono),
                r.primitive.map_or(&[][..], |p| &p.options[..]),
            ),
        );
        let s = scale;
        let pad = kentos_sheet::projection::Scale::for_rendering(s).model_metres_from_mm(20.);
        let reach = [
            extent[0] - pad,
            extent[1] - pad,
            extent[2] + pad,
            extent[3] + pad,
        ];
        let o = kentos_plot::vectors::VectorScale {
            scale: s,
            origin: [at.origin.x, at.origin.y],
        };
        let only = match r.layers {
            MapLayers::List(l) => Some(&l.layers),
            _ => None,
        };
        let why: Vec<String> = at
            .layers
            .iter()
            .filter(|(id, ..)| only.is_none_or(|l| l.contains(id)))
            .filter_map(|(_, name, layer)| {
                let what = kentos_plot::vectors::unsupported_in(layer, &o, Some(reach));
                (!what.is_empty()).then(|| format!("{name} ({})", what.join(", ")))
            })
            .collect();
        (!why.is_empty()).then(|| why.join("; "))
    }

    /// Where the drawing's text over a map is laid out: a camera on the map's view, north up,
    /// in the paper's CSS pixels (25.4/96 mm each: a label keeps its size on the paper whatever
    /// the zoom, as the web's map frames and the PDF write it), over the box the turned content
    /// covers; the frame as the labels see it (turned by the view's turn about the content's
    /// middle, the picture's pixels to the frame's); the spots of the layers the map shows.
    fn label_layout(
        &self,
        doc: &kentos_domain::Document,
        spatial: &Spatial,
        r: &MapRequest<'_>,
    ) -> Option<(
        Camera,
        Vec<kentos_interaction::LabelSpot>,
        crate::labels::Fence,
    )> {
        let c = r.view.center?;
        let turn = kentos_sheet_ui::paint::rad(r.view.rotation) as f32;
        let scale = r.view.scale.max(1e-6);
        // The frame's pixels per paper millimetre, and per CSS pixel of the paper.
        let px_per_mm = kentos_sheet::projection::Scale::for_rendering(scale)
            .paper_pixels_per_mm(r.pixels_per_metre);
        let k = (px_per_mm * crate::sheet_pdf::PX_MM).max(1e-6);
        let (w, h) = (f64::from(r.size.width) / k, f64::from(r.size.height) / k);
        let (s, co) = (f64::from(turn).sin().abs(), f64::from(turn).cos().abs());
        // Rounded to a thousandth of a pixel before the ceiling: the content's size is a float32 of the screen's
        // or the file's own zoom, and a 1e-7 of noise must not make the window a pixel larger in one and not the
        // other (docs/adr/0533 §5: the same map at the same scale is placed the same on both).
        let up = |v: f64| ((v * 1000.0).round() / 1000.0).ceil();
        let (bw, bh) = (up(w * co + h * s), up(w * s + h * co));
        let camera = Camera {
            map: None,
            map_ratio: 1.0,
            center: Vec2::new(c.x, c.y),
            // CSS pixels per ground metre on the paper.
            scale: kentos_sheet::projection::Scale::for_rendering(scale)
                .pixels_per_metre(1. / (crate::sheet_pdf::PX_MM * 1000.)),
            width: bw,
            height: bh,
            twist: 0.0,
        };
        let view = camera.visible_bounds();
        let drawn: std::collections::HashSet<String> = kentos_plot::layers::map_layers(
            doc.layers().nodes(),
            &doc.settings().layer_states,
            r.layers,
            r.overrides,
        )
        .into_iter()
        .map(|n| n.id.clone())
        .collect();
        let spots = spatial.labels_print(
            Vec2::new(view.min_x, view.min_y),
            Vec2::new(view.max_x, view.max_y),
            camera.scale,
            Default::default(),
            &|id| drawn.contains(id),
        );
        let fence = crate::labels::Fence {
            from: Point::new((bw / 2.0) as f32, (bh / 2.0) as f32),
            size: Size::new(r.size.width, r.size.height),
            turn,
            scale: k as f32,
        };
        Some((camera, spots, fence))
    }

    /// The drawing's text over the map, as [`label_layout`](Self::label_layout) lays it out; a
    /// text crossing the frame's edge left out.
    fn labels(
        &self,
        doc: &kentos_domain::Document,
        spatial: &Spatial,
        r: &MapRequest<'_>,
        frame: &mut Frame,
    ) {
        let Some((camera, spots, fence)) = self.label_layout(doc, spatial, r) else {
            return;
        };
        // The map's level (docs/adr/0716): the other levels' words out, the one under it faint.
        let (left_out, faint) = r
            .primitive
            .and_then(|p| p.level.as_ref())
            .map(|l| kentos_plot::layers::level_looks(doc, l))
            .unwrap_or_default();
        // A linked view's recipe (docs/adr/0774 §3): the words of what its phase or categories leave out.
        let recipe_out: std::collections::HashSet<u32> = match r.primitive {
            Some(p) if p.phase.is_some() || !p.hidden.is_empty() || !p.options.is_empty() => {
                kentos_plot::layers::map_phasing_of(doc, p.phase, &p.hidden, &p.options).hidden().collect()
            }
            _ => Default::default(),
        };
        let spots: Vec<_> = spots
            .into_iter()
            .filter(|s| {
                let id = kentos_plot::labels::spot_slot(s).0;
                !left_out.contains(&id) && !recipe_out.contains(&id)
            })
            .collect();
        if spots.is_empty() {
            return;
        }
        let layer_colors = kentos_plot::layers::map_layers(
            doc.layers().nodes(),
            &doc.settings().layer_states,
            r.layers,
            r.overrides,
        )
        .into_iter()
        .map(|n| (n.id.clone(), n.style.color.clone()))
        .collect();
        frame.with_save(|f| {
            f.translate(Vector::new(r.size.width / 2.0, r.size.height / 2.0));
            if fence.turn != 0.0 {
                f.rotate(fence.turn);
            }
            f.scale(fence.scale);
            f.translate(Vector::new(-fence.from.x, -fence.from.y));
            crate::labels::paint_in_map(
                f,
                doc,
                spots,
                camera,
                crate::labels::paper_colors(),
                self.font,
                self.format,
                fence,
                Some(&layer_colors),
                std::rc::Rc::new(faint),
            );
        });
    }
}

impl SheetMaps {
    /// The scene's fills, lines and points in the map's content, kept to its
    /// box (a turned frame turns the box: nothing crosses its edge).
    fn paint_scene(&self, r: &MapRequest<'_>, frame: &mut Frame) -> bool {
        let scene = self.scene.borrow();
        let (Some(s), Some(e)) = (scene.as_ref(), r.extent) else {
            return false;
        };
        let bounds = r.bounds();
        // A little beyond the frame: a line that crosses its edge is drawn to it.
        let pad = (e[2] - e[0]).max(e[3] - e[1]) * 0.02;
        let inside = |p: [f64; 2]| {
            p[0] >= e[0] - pad && p[0] <= e[2] + pad && p[1] >= e[1] - pad && p[1] <= e[3] + pad
        };
        let world = |hi: [f32; 2], lo: [f32; 2]| {
            let o = join(hi, lo);
            [s.origin.x + o[0], s.origin.y + o[1]]
        };
        let px = |p: [f64; 2]| r.to_px(p[0], p[1]);
        // Hairlines, as the drawing area draws them; a little heavier on a printed sheet. A small
        // map is drawn as the web draws it: as a picture of at least MAP_LABEL_SIDE pixels a side,
        // shrunk (its lines and marks thinner, a gallery's card not a dark blot).
        let fine =
            (MAP_LABEL_SIDE / f64::from(r.size.width.max(r.size.height)).max(1.0)).max(1.0) as f32;
        let line = if r.export { 0.75 } else { 1.0 } / fine;
        for part in &s.parts {
            for layer in &part.layers {
                // Fills: the layer's triangles, one path a colour.
                let mut by_color: Vec<([u8; 4], Vec<[iced::Point; 3]>)> = Vec::new();
                let fills = &part.fills[layer.fills.start as usize..layer.fills.end as usize];
                for t in fills.chunks_exact(3) {
                    let pts = [
                        world(t[0].hi, t[0].lo),
                        world(t[1].hi, t[1].lo),
                        world(t[2].hi, t[2].lo),
                    ];
                    // A triangle whose box misses the frame's is not drawn.
                    let lo = |i: usize| pts.iter().map(|p| p[i]).fold(f64::MAX, f64::min);
                    let hi = |i: usize| pts.iter().map(|p| p[i]).fold(f64::MIN, f64::max);
                    if hi(0) < e[0] - pad
                        || lo(0) > e[2] + pad
                        || hi(1) < e[1] - pad
                        || lo(1) > e[3] + pad
                    {
                        continue;
                    }
                    let tri = [px(pts[0]), px(pts[1]), px(pts[2])];
                    match by_color.iter_mut().find(|(c, _)| *c == t[0].color) {
                        Some((_, v)) => v.push(tri),
                        None => by_color.push((t[0].color, vec![tri])),
                    }
                }
                for (c, tris) in by_color {
                    let path = Path::new(|b| {
                        for t in &tris {
                            let cut = clip_polygon(t, bounds);
                            if let Some((first, rest)) = cut.split_first()
                                && rest.len() >= 2
                            {
                                b.move_to(*first);
                                for p in rest {
                                    b.line_to(*p);
                                }
                                b.close();
                            }
                        }
                    });
                    frame.fill(&path, color(c));
                }
                // Lines.
                let mut by_color: Vec<([u8; 4], Vec<[iced::Point; 2]>)> = Vec::new();
                let segments =
                    &part.segments[layer.segments.start as usize..layer.segments.end as usize];
                for seg in segments {
                    let (a, b) = (world(seg.a_hi, seg.a_lo), world(seg.b_hi, seg.b_lo));
                    if !inside(a) && !inside(b) && !crosses(a, b, e) {
                        continue;
                    }
                    let l = [px(a), px(b)];
                    match by_color.iter_mut().find(|(c, _)| *c == seg.color) {
                        Some((_, v)) => v.push(l),
                        None => by_color.push((seg.color, vec![l])),
                    }
                }
                for (c, lines) in by_color {
                    let path = Path::new(|b| {
                        for l in &lines {
                            if let Some((a, z)) = clip_segment(l[0], l[1], bounds) {
                                b.move_to(a);
                                b.line_to(z);
                            }
                        }
                    });
                    frame.stroke(
                        &path,
                        canvas::Stroke::default()
                            .with_color(color(c))
                            .with_width(line),
                    );
                }
                // Points: the drawing area's marks (shaders/wgsl/common/marks.wgsl): a ring, a cross
                // or a triangle of the point's size, rings and triangles with a dot on their spot;
                // their outlines cut to the box.
                let markers =
                    &part.markers[layer.markers.start as usize..layer.markers.end as usize];
                for m in markers {
                    let p = world(m.hi, m.lo);
                    if !inside(p) {
                        continue;
                    }
                    let at = px(p);
                    let r = ((m.size * 0.5 - 1.2) / fine).max(0.5);
                    let outline: Vec<(Point, Point)> = match m.shape {
                        kentos_render_wgpu::layout::marker_shape::CROSS => vec![
                            (Point::new(at.x - r, at.y), Point::new(at.x + r, at.y)),
                            (Point::new(at.x, at.y - r), Point::new(at.x, at.y + r)),
                        ],
                        kentos_render_wgpu::layout::marker_shape::TRIANGLE => {
                            let k = r * 0.95;
                            let c = Point::new(at.x, at.y - r * 0.18);
                            let v = [
                                Point::new(c.x, c.y - k),
                                Point::new(c.x + k * 0.866, c.y + k * 0.5),
                                Point::new(c.x - k * 0.866, c.y + k * 0.5),
                            ];
                            (0..3).map(|i| (v[i], v[(i + 1) % 3])).collect()
                        }
                        _ => (0..16)
                            .map(|i| {
                                let (a, b) = (
                                    i as f32 * std::f32::consts::TAU / 16.0,
                                    (i + 1) as f32 * std::f32::consts::TAU / 16.0,
                                );
                                (
                                    Point::new(at.x + r * a.cos(), at.y + r * a.sin()),
                                    Point::new(at.x + r * b.cos(), at.y + r * b.sin()),
                                )
                            })
                            .collect(),
                    };
                    let path = Path::new(|b| {
                        for (a, z) in &outline {
                            if let Some((a, z)) = clip_segment(*a, *z, bounds) {
                                b.move_to(a);
                                b.line_to(z);
                            }
                        }
                    });
                    frame.stroke(
                        &path,
                        canvas::Stroke::default()
                            .with_color(color(m.color))
                            .with_width(1.3 / fine),
                    );
                    if m.shape != kentos_render_wgpu::layout::marker_shape::CROSS
                        && bounds.contains(at)
                    {
                        frame.fill(&Path::circle(at, 1.0 / fine), color(m.color));
                    }
                }
            }
        }
        true
    }

    /// Counts the scenes built: the maps are painted again when it changes.
    pub fn revision(&self) -> u64 {
        self.built.get()
    }
}

/// Whether a segment crosses the box (both its ends outside it).
fn crosses(a: [f64; 2], b: [f64; 2], e: [f64; 4]) -> bool {
    let (x0, x1) = (a[0].min(b[0]), a[0].max(b[0]));
    let (y0, y1) = (a[1].min(b[1]), a[1].max(b[1]));
    x1 >= e[0] && x0 <= e[2] && y1 >= e[1] && y0 <= e[3]
}

#[cfg(test)]
mod tests;

/// Each map's own survey quality in the sheet mode (docs/adr/2800).
#[cfg(test)]
#[path = "../../tests/map_survey_scope_tests.rs"]
mod map_survey_scope_tests;

mod mode;
