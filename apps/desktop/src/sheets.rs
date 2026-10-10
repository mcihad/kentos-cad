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
use kentos_contracts::DrawingFont;
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
/// map's scale (built once a scale and drawing state, every map at that scale
/// drawn from it), and the drawing pipeline's plain scene of the open
/// document on paper's palette for a map whose layers have no vector form.
#[derive(Default)]
pub struct SheetMaps {
    scene: RefCell<Option<MapScene>>,
    /// The styled layers at the scales the maps were last drawn at (a few kept).
    styled: RefCell<Vec<Rc<StyledAt>>>,
    /// Counts the scenes built: the maps are painted again when it changes.
    built: Cell<u64>,
}

/// The shown layers through the style engine at one scale on the paper's palette.
struct StyledAt {
    /// The drawing (its session and state), the library's version and the scale.
    key: (u64, u64, u64, u32),
    origin: Vec2,
    /// Each shown layer's id and name, and its build.
    layers: Vec<(String, String, kentos_native_style::batches::StyledLayer)>,
}

/// Styled layers kept for this many scales.
const STYLED_KEPT: usize = 4;

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
    /// The shown layers through the style engine at `scale` on the paper's palette, from the
    /// ones kept when nothing they depend on changed. Construction lines are cut to the
    /// drawing's box and a margin (the maps cut them again to their frames).
    fn styled_at(
        &self,
        doc: &Document,
        store: &kentos_geometry_core::store::Store,
        library: &kentos_native_style::library::StyleLibrary,
        scale: u32,
    ) -> Rc<StyledAt> {
        let key = (
            doc.session,
            doc.model.generation(),
            library.version(),
            scale,
        );
        if let Some(at) = self.styled.borrow().iter().find(|a| a.key == key) {
            return Rc::clone(at);
        }
        let origin = scene::scene_origin(doc);
        let look = crate::style::scene::Look {
            palette: crate::sheet_pdf::paper_style(),
            symbol_scale: f64::from(scale.max(1)),
            screen: false,
            hairlines: false,
            origin,
            view_build: Default::default(),
            view_colors: Default::default(),
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
        let mut layers = Vec::new();
        for node in crate::style::scene::shown_layers(doc.model.layers().nodes()) {
            let objects: Vec<&kentos_contracts::Entity> = doc.model.by_layer(&node.id).collect();
            if objects.is_empty() {
                continue;
            }
            let styled = crate::style::scene::build_whole(
                store, library, node, &objects, &look, clip, &names,
            );
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

/// The core's map paths on a canvas, as its PDF writer draws them and cut to the map's box as
/// the PDF's clip cuts them (a turned frame's canvas cannot cut to its turned box): areas filled
/// (even-odd with holes), lines at their paper width (`px_mm` pixels a paper mm, at least
/// `thinnest`), dashed, capped and joined as styled.
fn draw_paths(
    frame: &mut Frame,
    r: &MapRequest<'_>,
    paths: &[kentos_sheet::pdf::MapPath],
    px_mm: f32,
    thinnest: f32,
) {
    use iced::widget::canvas::{Fill, LineCap, LineDash, LineJoin, Stroke, Style, fill};
    let bounds = r.bounds();
    let px = |pts: &[[f64; 2]]| -> Vec<Point> { pts.iter().map(|q| r.to_px(q[0], q[1])).collect() };
    for p in paths {
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
            let mut lines: Vec<Vec<Point>> = Vec::new();
            let mut add = |ring: Vec<Point>, closed: bool| {
                let mut pts = ring;
                if closed && let Some(first) = pts.first().copied() {
                    pts.push(first);
                }
                lines.extend(clip_runs(&pts, bounds));
            };
            add(outer, p.closed);
            if p.closed {
                for h in &p.holes {
                    add(px(h), true);
                }
            }
            if lines.is_empty() {
                continue;
            }
            let path = Path::new(|b| {
                for run in &lines {
                    b.move_to(run[0]);
                    for q in &run[1..] {
                        b.line_to(*q);
                    }
                }
            });
            let dash: Vec<f32> = s
                .dash
                .iter()
                .map(|d| (*d as f32 * px_mm).max(0.5))
                .collect();
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
                    line_dash: LineDash {
                        segments: &dash,
                        offset: 0,
                    },
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
    fn paint(&self, r: &MapRequest<'_>, frame: &mut Frame) -> bool {
        // The styled vectors its PDF writes; where a layer has none, the plain scene.
        if !self.paint_styled(r, frame) && !self.maps.paint_scene(r, frame) {
            return false;
        }
        if let Some(doc) = self.doc {
            self.labels(doc, r, frame);
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
        h.finish()
    }

    /// The map as the drawing's vectors and text (sheet_pdf.rs): the PDF's map content; none
    /// where a layer draws something with no vector form (the map then goes as a picture).
    fn vector(&self, r: &MapRequest<'_>) -> Option<kentos_sheet::pdf::MapContent> {
        let (drawing, doc) = (self.drawing?, self.doc?);
        r.view.center?;
        let extent = r.extent?;
        let texts = self
            .label_layout(r)
            .map(|(camera, spots, fence)| {
                crate::labels::texts_in_map(
                    doc,
                    spots,
                    camera,
                    crate::labels::paper_colors(),
                    self.font,
                    self.format,
                    fence,
                )
            })
            .unwrap_or_default();
        let src = crate::sheet_pdf::MapSource {
            drawing,
            store: self.spatial.store(),
            library: self.library,
        };
        crate::sheet_pdf::vector_content(&src, r.layers, r.view.scale, extent, &texts).ok()
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

impl SheetPainter<'_> {
    /// The map's drawing as its PDF draws it (the styled layers at its scale, what reaches its
    /// box); where a shown layer has something no vector writes (a pattern or picture fill, a
    /// picture or text marker, a soft-edged line), the whole map as the styled shaders draw it,
    /// on the CPU ([`Self::paint_picture`]): what its PDF then embeds. False where it cannot (no
    /// drawing, no place).
    fn paint_styled(&self, r: &MapRequest<'_>, frame: &mut Frame) -> bool {
        let (Some(drawing), Some(extent)) = (self.drawing, r.extent) else {
            return false;
        };
        if r.view.center.is_none() {
            return false;
        }
        let scale = r.view.scale.max(1);
        let at = self
            .maps
            .styled_at(drawing, self.spatial.store(), self.library, scale);
        let s = f64::from(scale);
        let pad = 20.0 * s / 1000.0;
        let reach = [
            extent[0] - pad,
            extent[1] - pad,
            extent[2] + pad,
            extent[3] + pad,
        ];
        let o = crate::map_vectors::VectorScale {
            scale: s,
            origin: [at.origin.x, at.origin.y],
        };
        let only = match r.layers {
            MapLayers::List(l) => Some(&l.layers),
            _ => None,
        };
        // Pixels a paper mm; a line never thinner than the screen shows (a printed one as it is).
        let px_mm = (r.pixels_per_metre * s / 1000.0) as f32;
        let thinnest = if r.export { 0.25 } else { 0.6 };
        let mut drawn = Vec::new();
        for (id, _, layer) in &at.layers {
            if only.is_some_and(|l| !l.contains(id)) {
                continue;
            }
            let (paths, missing) = crate::map_vectors::layer_paths(layer, &o, Some(reach));
            // A layer with no vector form here: the map as a picture, styled all the same.
            if !missing.is_empty() {
                return self.paint_picture(r, frame, &at, only, s);
            }
            drawn.push(paths);
        }
        for paths in &drawn {
            draw_paths(frame, r, paths, px_mm, thinnest);
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
        let (sn, co) = kentos_sheet::units::sin_cos(r.view.rotation);
        let ppm = r.pixels_per_metre * f;
        let (a, b) = ((at.origin.x - c.x) * ppm, (at.origin.y - c.y) * ppm);
        let view = CpuView {
            m: [co * ppm, sn * ppm, sn * ppm, -co * ppm],
            t: [
                f64::from(pw) / 2.0 + a * co + b * sn,
                f64::from(ph) / 2.0 + a * sn - b * co,
            ],
            px_per_m: ppm,
            // A CSS pixel of the paper (25.4/96 mm) in the picture's pixels.
            dpr: ppm * scale / 1000.0 * 25.4 / 96.0,
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
        let drawing = self.drawing?;
        let extent = r.extent?;
        let scale = r.view.scale.max(1);
        let at = self
            .maps
            .styled_at(drawing, self.spatial.store(), self.library, scale);
        let s = f64::from(scale);
        let pad = 20.0 * s / 1000.0;
        let reach = [
            extent[0] - pad,
            extent[1] - pad,
            extent[2] + pad,
            extent[3] + pad,
        ];
        let o = crate::map_vectors::VectorScale {
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
                let what = crate::map_vectors::unsupported_in(layer, &o, Some(reach));
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
        r: &MapRequest<'_>,
    ) -> Option<(
        Camera,
        Vec<kentos_interaction::LabelSpot>,
        crate::labels::Fence,
    )> {
        let doc = self.doc?;
        let c = r.view.center?;
        let turn = kentos_sheet_ui::paint::rad(r.view.rotation) as f32;
        let scale = f64::from(r.view.scale.max(1));
        // The frame's pixels per paper millimetre, and per CSS pixel of the paper.
        let px_per_mm = r.pixels_per_metre * scale / 1000.0;
        let k = (px_per_mm * crate::sheet_pdf::PX_MM).max(1e-6);
        let (w, h) = (f64::from(r.size.width) / k, f64::from(r.size.height) / k);
        let (s, co) = (f64::from(turn).sin().abs(), f64::from(turn).cos().abs());
        let (bw, bh) = ((w * co + h * s).ceil(), (w * s + h * co).ceil());
        let camera = Camera {
            center: Vec2::new(c.x, c.y),
            // CSS pixels per ground metre on the paper.
            scale: 1000.0 / (crate::sheet_pdf::PX_MM * scale),
            width: bw,
            height: bh,
        };
        let view = camera.visible_bounds();
        let mut spots = self.spatial.labels(
            Vec2::new(view.min_x, view.min_y),
            Vec2::new(view.max_x, view.max_y),
            camera.scale,
            Default::default(),
        );
        // A map of some layers writes only their objects' text.
        if let MapLayers::List(layers) = r.layers {
            spots.retain(|spot| {
                doc.get(crate::labels::slot_of(spot))
                    .is_some_and(|e| layers.layers.contains(&e.base().layer_id))
            });
        }
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
    fn labels(&self, doc: &kentos_domain::Document, r: &MapRequest<'_>, frame: &mut Frame) {
        let Some((camera, spots, fence)) = self.label_layout(r) else {
            return;
        };
        if spots.is_empty() {
            return;
        }
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

impl App {
    /// The window while a sheet is in front: the ribbon with its Pafta tab,
    /// the sheet mode where the drawing and its docks were, the tabs under it,
    /// its cells in the status bar, its windows over everything.
    pub(crate) fn sheet_view(&self) -> iced::Element<'_, Message> {
        use iced::widget::{column, container};
        let painter: Painter<'_> = Rc::new(self.sheet_painter());
        let base = container(column![
            self.ribbon(false),
            self.sheets.body(painter.clone()).map(Message::Sheet),
            self.sheets.tabs().map(Message::Sheet),
            self.status_bar(),
        ])
        .width(iced::Fill)
        .height(iced::Fill)
        .style(kentos_ui::style::container::window);
        let mut layers: Vec<iced::Element<'_, Message>> = vec![base.into()];
        layers.extend(self.app_menu_view());
        layers.extend(self.sheets.window(painter).map(|w| w.map(Message::Sheet)));
        if let Some(dialog) = self.dialog {
            layers.push(self.dialog_view(dialog));
        }
        if layers.len() == 1 {
            return layers.remove(0);
        }
        iced::widget::Stack::with_children(layers).into()
    }

    /// The maps' painter of the open drawing (its scene and its text).
    pub(crate) fn sheet_painter(&self) -> SheetPainter<'_> {
        painter_of(
            &self.sheet_maps,
            self.document.as_ref(),
            &self.spatial,
            &self.styles.library,
            &self.styles.images,
        )
    }

    /// The project as the sheet mode sees it: its mode and type, whether it
    /// has a coordinate system and attributes, its plot scale, its name, the
    /// user and today, and where the drawing area looks.
    pub(crate) fn sheet_context(&self) -> Context {
        let doc = self.document.as_ref();
        let settings = doc.map(Document::settings);
        let srid = settings.map_or(0, |s| s.srid);
        // The registry's system or the project's own definition (docs/adr/0168 §1); the
        // latter without the registry's transverse Mercator values.
        let crs = settings
            .filter(|s| s.has_system())
            .map(crate::crs::project_name);
        let c = self.viewport.camera.center;
        Context {
            // The type the interface shows: a project not asked its type is CBS (docs/adr/0165).
            workspace: doc.map(|_| self.work_mode()),
            // The project's type is the catalog's; the opened drawing does not carry it yet.
            project_type: None,
            capabilities: Capabilities {
                georeferenced: crs.is_some(),
                attribute_layers: self.sheet_attributes.get(),
                plot_scale: settings
                    .map(|s| s.plot_scale.round())
                    .filter(|s| *s >= 1.0)
                    .map(|s| s as u32),
            },
            project: ProjectInfo {
                name: doc.map_or_else(String::new, |d| d.name().to_owned()),
                user: std::env::var("USER")
                    .or_else(|_| std::env::var("USERNAME"))
                    .unwrap_or_default(),
                date: kentos_sheet_ui::text::today(),
                crs_name: crs.clone().unwrap_or_default(),
                // The drawing's own variables, read after the book's (docs/adr/0214 §2.3).
                variables: settings.map_or_else(Vec::new, |s| {
                    s.variables
                        .iter()
                        .map(crate::sheet_inputs::sheet_variable)
                        .collect()
                }),
            },
            // With its transverse Mercator parameters: the core works out the meridian convergence.
            crs: crs.map(|name| {
                crate::sheet_inputs::crs_info(srid)
                    .filter(|_| srid != crate::crs::LOCAL_SRID)
                    .unwrap_or(CrsInfo { name, tm: None })
            }),
            center: doc.map(|_| GroundPoint { x: c.x, y: c.y }),
            // The layers a coordinate list may read (docs/adr/0206 §2).
            layers: doc.map_or_else(Vec::new, |d| {
                d.model
                    .layers()
                    .leaves()
                    .into_iter()
                    .map(|l| (l.id.clone(), l.name.clone()))
                    .collect()
            }),
            // How much ground the drawing area shows (Görünüme sığdır, docs/adr/0206 §1).
            view_size: doc
                .map(|_| &self.viewport.camera)
                .filter(|cam| cam.scale > 0.0 && cam.width > 0.0 && cam.height > 0.0)
                .map(|cam| [cam.width / cam.scale, cam.height / cam.scale]),
        }
    }

    /// The key the open project's book is kept under (the web's `projectKeyOf`).
    pub(crate) fn sheet_key(&self) -> kentos_sheet_ui::ProjectKey {
        let doc = self.document.as_ref();
        let cloud =
            doc.and_then(|d| d.cloud_source())
                .map(|c| kentos_sheet_ui::store::CloudProject {
                    tenant: c.tenant.to_string(),
                    project: c.project.to_string(),
                    name: c.info.name.clone(),
                });
        let project_id = doc
            .and_then(|d| d.model.project_id())
            .map(|id| id.to_string());
        let file = doc
            .and_then(|d| d.path.as_ref())
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned());
        let session = doc.map_or(0, |d| d.session);
        kentos_sheet_ui::project_key(
            cloud.as_ref(),
            project_id.as_deref(),
            file.as_deref(),
            &format!("{}-{session}", std::process::id()),
        )
    }

    /// After every message: the book follows the project (another drawing
    /// loads its own; a new one that got its name takes it along), the
    /// mode follows the project's values, the maps follow the drawing.
    pub(crate) fn follow_sheets(&mut self) {
        if let Some(store) = self.sheet_store.clone() {
            let key = self.sheet_key();
            let session = self.document.as_ref().map(|d| d.session);
            let replaced = session != self.sheet_session;
            match kentos_sheet_ui::store::key_change(self.sheet_project.as_ref(), &key, replaced) {
                kentos_sheet_ui::store::KeyChange::Keep => {}
                kentos_sheet_ui::store::KeyChange::Load => self.sheets.attach(store, &key.id),
                kentos_sheet_ui::store::KeyChange::Move
                | kentos_sheet_ui::store::KeyChange::Copy => self.sheets.rekey(&key.id),
            }
            self.sheet_project = Some(key);
            self.sheet_session = session;
        }
        // The gallery over the drawing reads the project too (what a template needs that it lacks).
        if !self.sheets.is_active() && !self.sheets.gallery_open() {
            return;
        }
        // What only a changed drawing changes: its objects' attributes, the maps' scene.
        let generation = self
            .document
            .as_ref()
            .map(|d| (d.session, d.model.generation()));
        if generation != self.sheet_generation {
            self.sheet_generation = generation;
            self.sheet_attributes.set(
                self.document
                    .as_ref()
                    .is_some_and(|d| d.model.entities().any(|e| !e.base().attrs.is_empty())),
            );
            self.sheet_maps.sync(self.document.as_ref());
        }
        let ctx = self.sheet_context();
        // The drawing area's centre changes as it is looked around; it matters only to a new map.
        let same = {
            let mut a = ctx.clone();
            let mut b = self.sheets.context().clone();
            (a.center, a.view_size) = (None, None);
            (b.center, b.view_size) = (None, None);
            a == b
        };
        if !same {
            self.sheets.set_context(ctx);
        }
        self.follow_sheet_data();
    }

    /// The sheet's tables, coordinate lists and legends follow the drawing,
    /// the chosen objects and the sheet's own items (what they name, where its
    /// maps look): worked out again only when one of these changed.
    fn follow_sheet_data(&mut self) {
        let (Some(doc), Some(sheet)) = (self.document.as_ref(), self.sheets.open_sheet()) else {
            return;
        };
        let book = self.sheets.book();
        let mut h = DefaultHasher::new();
        (doc.session, doc.model.generation()).hash(&mut h);
        self.selection
            .ids()
            .iter()
            .map(|s| s.0)
            .collect::<Vec<_>>()
            .hash(&mut h);
        // The items that read the drawing, as the sheet and its master page have them.
        let master = sheet
            .master
            .as_deref()
            .and_then(|m| book.masters.iter().find(|x| x.id == m));
        for it in master
            .into_iter()
            .flat_map(|m| m.items.iter())
            .chain(sheet.items.iter())
        {
            if matches!(
                it.kind,
                kentos_sheet::kinds::ItemKind::Table(_)
                    | kentos_sheet::kinds::ItemKind::CoordinateList(_)
                    | kentos_sheet::kinds::ItemKind::Legend(_)
                    | kentos_sheet::kinds::ItemKind::Map(_)
            ) {
                serde_json::to_string(&it.kind)
                    .unwrap_or_default()
                    .hash(&mut h);
                it.id.hash(&mut h);
            }
        }
        for (id, e) in crate::sheet_inputs::extents_of(self.sheets.display_list()) {
            (id, e.map(f64::to_bits)).hash(&mut h);
        }
        let key = h.finish();
        if self.sheet_data_key == Some(key) {
            return;
        }
        self.sheet_data_key = Some(key);
        let data = crate::sheet_inputs::sheet_data(
            &doc.model,
            &self.spatial,
            &self.styles.library,
            self.selection.ids(),
            book,
            sheet,
            self.sheets.display_list(),
        );
        let inputs = kentos_sheet::display::RenderInputs {
            tables: data.tables,
            coordinates: data.coordinates,
            legends: data.legends,
            ..kentos_sheet_ui::empty_inputs()
        };
        self.sheets.set_data(inputs);
    }

    /// A message of the sheet mode, and what it asks of the desktop.
    pub(crate) fn sheet_message(&mut self, m: kentos_sheet_ui::Message) -> Task<Message> {
        // A new map looks where the drawing area does now; a map takes its place and scale from
        // it (docs/adr/0206 §1).
        if matches!(
            &m,
            kentos_sheet_ui::Message::Stage(_)
                | kentos_sheet_ui::Message::NewSheet
                | kentos_sheet_ui::Message::Gallery(_)
                | kentos_sheet_ui::Message::MapFitView
                | kentos_sheet_ui::Message::MapCentreFromView
        ) {
            let ctx = self.sheet_context();
            let now = self.sheets.context();
            if ctx.center != now.center || ctx.view_size != now.view_size {
                self.sheets.set_context(ctx);
            }
        }
        // A coordinate list takes the drawing's choice by its objects' lasting ids, and shows its
        // objects in the drawing (docs/adr/0206 §2).
        if matches!(&m, kentos_sheet_ui::Message::CoordTakeSelection) {
            let uids: Vec<String> = self.document.as_ref().map_or_else(Vec::new, |d| {
                self.selection
                    .ids()
                    .iter()
                    .filter_map(|s| d.model.uid(*s))
                    .map(|u| u.to_string())
                    .collect()
            });
            if uids.is_empty() {
                self.warn("Çizimde seçili nesne yok: önce modelde nesneleri seçin, sonra “Seçimi al”a basın.");
                return Task::none();
            }
            return self.sheet_message(kentos_sheet_ui::Message::CoordObjects(uids));
        }
        if matches!(&m, kentos_sheet_ui::Message::CoordShow) {
            let (Some(doc), Some(source)) = (
                self.document.as_ref(),
                self.sheets.chosen_coordinate_source(),
            ) else {
                return Task::none();
            };
            let (slots, _) =
                crate::sheet_inputs::coordinate_objects(&doc.model, self.selection.ids(), &source);
            if slots.is_empty() {
                return Task::none();
            }
            self.selection.set(slots);
            self.sheet_tab = false;
            return self.update(Message::Run("view.zoomSelection"));
        }
        // The gallery's pictures show the drawing as it is now.
        if matches!(&m, kentos_sheet_ui::Message::Gallery(_)) {
            self.sheet_maps.sync(self.document.as_ref());
        }
        let effects = self.sheets.update(m);
        self.sheet_effects(effects)
    }

    /// What the sheet mode asks of the desktop.
    pub(crate) fn sheet_effects(&mut self, effects: Vec<Effect>) -> Task<Message> {
        let mut tasks = Vec::new();
        for effect in effects {
            let mut task = Task::none();
            match effect {
                Effect::ShowSheet => {
                    self.sheet_tab = true;
                    self.sheet_maps.sync(self.document.as_ref());
                }
                Effect::ShowModel => self.sheet_tab = false,
                Effect::AskExportPath { kind, suggested } => {
                    let (name, ext) = match kind {
                        ExportKind::Pdf => ("PDF belgesi", "pdf"),
                        ExportKind::Svg => ("SVG resmi", "svg"),
                        ExportKind::Png => ("PNG resmi", "png"),
                        ExportKind::Kpafta => ("KentOS pafta dosyası", "kpafta"),
                    };
                    task = Task::perform(
                        async move {
                            rfd::AsyncFileDialog::new()
                                .set_title("Dışa aktar")
                                .set_file_name(suggested)
                                .add_filter(name, &[ext])
                                .save_file()
                                .await
                                .map(|f| f.path().to_owned())
                        },
                        move |path| Message::SheetExportTo(kind, path),
                    );
                }
                Effect::AskImportPath => {
                    task = Task::perform(
                        async {
                            let file = rfd::AsyncFileDialog::new()
                                .set_title(".kpafta dosyasından")
                                .add_filter("KentOS pafta dosyası", &["kpafta"])
                                .pick_file()
                                .await?;
                            let name = file.file_name();
                            let bytes = file.read().await;
                            Some((name, String::from_utf8_lossy(&bytes).into_owned()))
                        },
                        |picked| match picked {
                            Some((name, text)) => {
                                Message::Sheet(kentos_sheet_ui::Message::ImportText(name, text))
                            }
                            None => Message::Sheet(kentos_sheet_ui::Message::CloseDialog),
                        },
                    );
                }
                // Resim seç…: a PNG or JPEG file, its bytes back to the sheet mode.
                Effect::AskPicturePath => {
                    task = Task::perform(
                        async {
                            let file = rfd::AsyncFileDialog::new()
                                .set_title("Resim seç")
                                .add_filter(
                                    "Resim (PNG, JPEG, SVG)",
                                    &["png", "jpg", "jpeg", "svg"],
                                )
                                .pick_file()
                                .await?;
                            Some((file.file_name(), file.read().await))
                        },
                        |picked| match picked {
                            Some((name, bytes)) => {
                                Message::Sheet(kentos_sheet_ui::Message::PictureFile(name, bytes))
                            }
                            None => Message::Sheet(kentos_sheet_ui::Message::CloseDialog),
                        },
                    );
                }
                // Yazdır: the PDF in the system's viewer (sheet_pdf.rs).
                Effect::Print => task = self.sheet_print(),
                // Koordinat sistemi seç: the project's settings at its coordinate system.
                Effect::HostAction(action) if action == "project.crs" => {
                    task = Task::done(Message::Run("crs.set"));
                }
                Effect::HostAction(action) => self.warn(format!(
                    "Bu düzeltme ({action}) masaüstünde bu sürümde yok."
                )),
                Effect::Notice(text) => self.output(text),
                Effect::Say(say, text) => self.say(
                    match say {
                        Say::Info => Level::Info,
                        Say::Success => Level::Success,
                        Say::Warn => Level::Warn,
                        Say::Error => Level::Error,
                    },
                    text,
                ),
                Effect::Library(request) => task = self.library_request(request),
            }
            tasks.push(task);
        }
        Task::batch(tasks)
    }

    /// Writes an export where the user chose (nothing when they did not).
    pub(crate) fn sheet_export_to(&mut self, kind: ExportKind, path: Option<PathBuf>) {
        let Some(path) = path else {
            return;
        };
        let host = (kind == ExportKind::Pdf).then(|| self.sheet_pdf_host());
        let painter = painter_of(
            &self.sheet_maps,
            self.document.as_ref(),
            &self.spatial,
            &self.styles.library,
            &self.styles.images,
        );
        let written = match host {
            Some(host) => self.sheets.export_pdf_to(&path, &painter, host),
            None => self
                .sheets
                .export_to(kind, &path, &painter)
                .map(|()| Vec::new()),
        };
        match written {
            Ok(findings) => {
                self.output(format!("{} yazıldı.", path.display()));
                self.say_pdf_findings(&findings);
            }
            Err(e) => self.warn(format!("Dışa aktarılamadı: {e}")),
        }
    }

    /// What a PDF writes differently from the screen (an SVG picture as a picture, or the missing
    /// picture's box), in the log: a note as information, the rest as a warning.
    pub(crate) fn say_pdf_findings(&mut self, findings: &[kentos_sheet::preflight::Finding]) {
        use kentos_sheet::preflight::Severity;
        for f in findings {
            match f.severity {
                Severity::Info => self.output(f.message.clone()),
                Severity::Warning | Severity::Error => {
                    self.warn(format!("{} {}", f.message, f.fix));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use iced::Size;
    use kentos_sheet_ui::questions::QuestionsMessage;
    use kentos_sheet_ui::{InspectorTab, Message as Sheet};

    /// Yeni pafta: the template's questions (design §12) answered with its own values.
    fn new_sheet(app: &mut App) {
        let _ = app.update(Message::Sheet(Sheet::NewSheet));
        if app.sheets.questions().is_some() {
            let _ = app.update(Message::Sheet(Sheet::Questions(QuestionsMessage::Make)));
        }
    }

    use super::*;
    use crate::files_testing::app_with_drawing;

    #[test]
    fn a_sheet_opens_over_the_drawing_and_the_model_comes_back() {
        let mut app = app_with_drawing();
        assert!(!app.sheets.is_active());
        new_sheet(&mut app);
        assert!(app.sheets.is_active());
        assert!(app.sheet_tab, "the contextual Pafta tab is open");
        // The new sheet's map looks where the drawing area does, and the project's values are the drawing's.
        let ctx = app.sheets.context().clone();
        assert_eq!(
            ctx.center.map(|c| (c.x, c.y)),
            Some((app.viewport.camera.center.x, app.viewport.camera.center.y))
        );
        assert_eq!(ctx.project.name, app.document.as_ref().unwrap().name());
        // The window draws with the mode in it (the stage, the inspector, the tabs, the cells).
        let mut snapshot =
            kentos_ui::snapshot::Snapshot::software(Size::new(1440.0, 900.0)).expect("a renderer");
        let image = snapshot.render(app.view(), &app.theme());
        assert_eq!((image.width, image.height), (1440, 900));
        // Keys no field took go to the sheet mode while a sheet is in front: Ctrl+Z takes back the new sheet.
        let _ = app.update(Message::Sheet(Sheet::InspectorTab(InspectorTab::Page)));
        let undo = crate::keys::KeyPress {
            key: iced::keyboard::Key::Character("z".into()),
            physical: iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::KeyZ),
            modifiers: iced::keyboard::Modifiers::CTRL,
            text: None,
            repeat: false,
        };
        // A letter the sheet mode does not take neither types into the hidden command line nor starts a drawing tool.
        let letter = crate::keys::KeyPress {
            key: iced::keyboard::Key::Character("c".into()),
            physical: iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::KeyC),
            modifiers: iced::keyboard::Modifiers::empty(),
            text: Some("c".into()),
            repeat: false,
        };
        let _ = app.update(Message::Key(letter));
        assert!(app.command_input.is_empty());
        assert_eq!(
            app.session.tool_id(),
            "select",
            "no drawing tool started behind the sheet"
        );
        let _ = app.update(Message::Key(undo));
        assert!(
            app.sheets.book().sheets.is_empty(),
            "the undo of the sheet mode, not the drawing's"
        );
        assert!(!app.sheets.is_active());
        // The model tab.
        new_sheet(&mut app);
        let _ = app.update(Message::Sheet(Sheet::Tab(0)));
        assert!(!app.sheets.is_active());
        assert!(!app.sheet_tab);
    }

    /// MODEL (`sheet.model`) returns from a sheet to the drawing, as the web's command does.
    #[test]
    fn the_model_command_returns_from_a_sheet() {
        let mut app = app_with_drawing();
        let _ = app.run("sheet.model");
        assert_eq!(crate::files_testing::last_said(&app), "Model zaten önde.");
        new_sheet(&mut app);
        assert!(app.sheets.is_active());
        let _ = app.run("sheet.model");
        assert!(!app.sheets.is_active());
    }

    /// Pictures of the sheet mode in the desktop for the owner: the demo
    /// drawing's sheet, its map painted from the drawing's own scene, in the
    /// Pafta (light) and Grafit (dark) themes, 1440 and 1100 pixels wide.
    /// `cargo test -p kentos-desktop sheet_screens -- --ignored --nocapture`
    /// writes `.run/shots/sheet-desktop/masaustu-*.png`.
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn sheet_screens() {
        let out =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots/sheet-desktop");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        crate::drawing_fonts::load();
        kentos_ui::theme::motion::set_reduced(true);
        for (theme, tag) in [("light", "pafta"), ("dark", "grafit")] {
            for (w, h) in [(1440.0, 900.0), (1100.0, 720.0)] {
                let mut app = app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
                app.apply_settings();
                // The map looks at the drawing: the sheet is made where the drawing area looks.
                if let Some(b) = scene::extents(app.document.as_ref().unwrap()) {
                    app.viewport.camera.center =
                        Vec2::new((b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0);
                }
                new_sheet(&mut app);
                let map = app
                    .sheets
                    .open_sheet()
                    .unwrap()
                    .items
                    .iter()
                    .find(|i| i.name == "Harita")
                    .map(|i| i.id.clone());
                let _ = app.update(Message::Sheet(Sheet::Select(map.into_iter().collect())));
                let mut snapshot =
                    kentos_ui::snapshot::Snapshot::software(Size::new(w, h)).expect("a renderer");
                let mut update = |app: &mut App, m: Message| {
                    let _ = app.update(m);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("masaustu-{tag}-{}.png", w as u32));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }

    /// Dışa aktar → PDF olarak…: the window asks which sheets and how, Kaydet asks where
    /// (the file window), and the file is the core's PDF with the drawing's faces, its map as
    /// the drawing's vectors in layers and its text.
    #[test]
    fn a_sheet_s_pdf_is_written_with_the_drawing_s_vectors_and_faces() {
        use kentos_sheet_ui::message::{PdfMessage, PdfScope};
        let mut app = app_with_drawing();
        // The sheet's map looks at the drawing (a parcel and a building).
        app.viewport.camera.center = Vec2::new(486_530.0, 4_420_200.0);
        new_sheet(&mut app);
        let _ = app.update(Message::Sheet(Sheet::Export(
            kentos_sheet_ui::ExportKind::Pdf,
        )));
        let _ = app.update(Message::Sheet(Sheet::Pdf(PdfMessage::Scope(PdfScope::All))));
        // Kaydet: the host's file window is asked for a .pdf.
        let effects = app.sheets.update(Sheet::Pdf(PdfMessage::Save));
        assert!(
            matches!(effects.as_slice(), [kentos_sheet_ui::Effect::AskExportPath { kind: kentos_sheet_ui::ExportKind::Pdf, suggested }] if suggested.ends_with(".pdf")),
            "{effects:?}"
        );
        let file = crate::files_testing::scratch("pafta-pdf").join("pafta.pdf");
        let _ = std::fs::create_dir_all(file.parent().unwrap());
        app.sheet_export_to(kentos_sheet_ui::ExportKind::Pdf, Some(file.clone()));
        let pdf = std::fs::read(&file).expect("the PDF is written");
        let text = String::from_utf8_lossy(&pdf);
        assert!(
            text.starts_with("%PDF-1.7"),
            "{}",
            &text[..16.min(text.len())]
        );
        assert!(text.contains("/FontFile2") && text.contains("/ToUnicode"));
        assert!(
            text.contains("/OCProperties"),
            "the drawing's layers as the viewer's"
        );
        // The map's layers are the drawing's shown ones (Çizim is hidden).
        assert!(
            text.contains("/Name (Parsel)") && text.contains("/Name (Bina)"),
            "the layers"
        );
        assert!(
            !text.contains("/Name <FEFF00C7"),
            "a hidden layer is not written"
        );
        assert!(app.log.lines().any(|l| l.text.contains("yazıldı")), "said");
    }

    /// A layer with no vector form here (a pattern fill, the web's own case): the PDF window
    /// names it and why, and its map goes to the PDF as a picture at the window's resolution.
    #[test]
    fn a_map_with_a_pattern_fill_goes_as_a_picture_and_the_window_says_why() {
        use kentos_sheet_ui::message::PdfMessage;
        let mut app = app_with_drawing();
        app.viewport.camera.center = Vec2::new(486_530.0, 4_420_200.0);
        // Bina's areas filled with a pattern of dots.
        let doc = app.document.as_mut().unwrap();
        let mut style = doc
            .model
            .layers()
            .get("bina")
            .map(|l| l.style.clone())
            .expect("the Bina layer");
        style.renderer = Some(
            serde_json::json!({ "type": "single", "symbols": { "fill": { "type": "fill", "layers": [{ "id": "p", "type": "patternFill", "marker": { "type": "marker", "layers": [{ "id": "m", "type": "shape", "shape": "circle", "size": 0.8, "fill": "#3366cc", "stroke": null }] }, "spacingX": 3, "spacingY": 3 }] } } }),
        );
        assert!(doc.model.set_layer_style("bina", style, "Desenli yapılar"));
        // A building's outline on it, as the web's demo has them.
        let building: kentos_contracts::Entity = serde_json::from_value(serde_json::json!({
            "kind": "polygon", "layerId": "bina", "attrs": {}, "id": 0,
            "pts": [{ "x": 486530.0, "y": 4420200.0 }, { "x": 486540.0, "y": 4420200.0 },
                    { "x": 486540.0, "y": 4420210.0 }, { "x": 486530.0, "y": 4420210.0 }]
        }))
        .expect("a polygon");
        doc.model.add(building).expect("added");
        app.spatial.reload(&app.document.as_ref().unwrap().model);
        new_sheet(&mut app);
        let ways = app.sheets.pdf_ways(&app.sheet_painter());
        // CBS's sheet has the map and its overview (a project not asked its type is CBS, docs/adr/0165).
        assert_eq!(ways.len(), 2, "{ways:?}");
        assert!(
            ways.iter()
                .all(|w| !w.vector && w.why == "Bina (desen dolgusu)"),
            "{ways:?}"
        );
        let _ = app.update(Message::Sheet(Sheet::Export(
            kentos_sheet_ui::ExportKind::Pdf,
        )));
        let file = crate::files_testing::scratch("pafta-pdf-desen").join("desen.pdf");
        let _ = std::fs::create_dir_all(file.parent().unwrap());
        app.sheet_export_to(kentos_sheet_ui::ExportKind::Pdf, Some(file.clone()));
        let pdf = std::fs::read(&file).expect("written");
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.contains("/Subtype /Image"), "the map as a picture");
        assert!(
            text.contains("/SMask"),
            "a clear picture over the paper, not the theme's background"
        );
        assert!(!text.contains("/OCProperties"), "a picture has no layers");
        let _ = app.update(Message::Sheet(Sheet::Pdf(PdfMessage::Save)));
        // The picture is the styled drawing (the styled shaders' CPU twin), not the plain scene:
        // the building's dots in the pattern's blue, the paper between them.
        let m = app
            .sheets
            .display_list()
            .and_then(|l| {
                l.prims.iter().find_map(|p| match p {
                    kentos_sheet::display::Prim::Map(m) => Some(m.clone()),
                    _ => None,
                })
            })
            .expect("the sheet's map");
        struct MapShot<'a> {
            painter: SheetPainter<'a>,
            m: &'a kentos_sheet::display::MapPrim,
            k: f64,
        }
        impl canvas::Program<Message> for MapShot<'_> {
            type State = ();
            fn draw(
                &self,
                _: &(),
                renderer: &iced::Renderer,
                _: &iced::Theme,
                bounds: iced::Rectangle,
                _: iced::mouse::Cursor,
            ) -> Vec<canvas::Geometry> {
                let mut f = Frame::new(renderer, bounds.size());
                f.fill_rectangle(iced::Point::ORIGIN, bounds.size(), iced::Color::WHITE);
                let r = MapRequest::of(self.m, bounds.size(), self.k, true);
                assert!(self.painter.paint(&r, &mut f), "the map is painted");
                vec![f.into_geometry()]
            }
        }
        // Four pixels a paper millimetre.
        let k = 4.0 / 1000.0;
        let size = Size::new(
            (f64::from(m.clip.width) * k) as f32,
            (f64::from(m.clip.height) * k) as f32,
        );
        let shot = MapShot {
            painter: app.sheet_painter(),
            m: &m,
            k,
        };
        let view: iced::Element<'_, Message> = canvas::Canvas::new(shot)
            .width(iced::Fill)
            .height(iced::Fill)
            .into();
        let image = kentos_ui::snapshot::Snapshot::software(size)
            .expect("a renderer")
            .render(view, &iced::Theme::Light);
        // The building is north-east of the map's middle: 10 m on the ground, 10 mm on the paper.
        let request = MapRequest::of(&m, size, k, true);
        let (w, h) = (image.width as usize, image.height as usize);
        let (mut blue, mut paper) = (0, 0);
        // Inside it, a metre from its outline, every quarter metre.
        for gy in 0..32 {
            for gx in 0..32 {
                let p = request.to_px(
                    486_531.0 + f64::from(gx) * 0.25,
                    4_420_201.0 + f64::from(gy) * 0.25,
                );
                let (x, y) = (p.x as usize, p.y as usize);
                if x >= w || y >= h {
                    continue;
                }
                let i = (y * w + x) * 4;
                let c = [image.rgba[i], image.rgba[i + 1], image.rgba[i + 2]];
                if c[2] > 150 && c[0] < 110 && c[1] < 150 {
                    blue += 1;
                } else if c.iter().all(|v| *v > 225) {
                    paper += 1;
                }
            }
        }
        // Dots of 0.8 mm every 3 mm ink about a twentieth of the area.
        assert!(
            blue >= 20 && paper >= 500,
            "dots and paper: {blue} blue, {paper} paper"
        );
    }

    /// An SVG picture (design §9a; the core draws no SVG): drawn here as a PNG at the export
    /// window's resolution, embedded in its place, and the log says so.
    #[test]
    fn an_svg_picture_goes_into_the_pdf_as_a_picture_and_the_log_says_so() {
        use kentos_sheet::kinds::{ItemKind, PictureFit, PictureItem};
        use kentos_sheet::ops::Op;
        let mut app = app_with_drawing();
        new_sheet(&mut app);
        let sheet = app
            .sheets
            .open_sheet()
            .map(|s| s.id.clone())
            .expect("a sheet");
        let item = kentos_sheet::model::Item::new(
            "logo",
            "Kurum logosu",
            kentos_sheet::units::RectUm::new(20_000, 20_000, 20_000, 20_000),
            ItemKind::Picture(PictureItem {
                asset: None,
                fit: PictureFit::Contain,
                clip: true,
            }),
        );
        assert!(app.sheets.apply(
            vec![Op::AddItems(kentos_sheet::ops::AddItems {
                to: kentos_sheet::model::Owner::sheet(&sheet),
                items: vec![item],
                index: None,
            })],
            "Resim çerçevesi",
        ));
        let _ = app.update(Message::Sheet(Sheet::Select(vec!["logo".into()])));
        let _ = app.update(Message::Sheet(Sheet::PictureFile(
            "logo.svg".into(),
            br##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><circle cx="12" cy="12" r="10" fill="#1f4a96"/></svg>"##.to_vec(),
        )));
        let _ = app.update(Message::Sheet(Sheet::Export(
            kentos_sheet_ui::ExportKind::Pdf,
        )));
        let file = crate::files_testing::scratch("pafta-pdf-svg").join("svg.pdf");
        let _ = std::fs::create_dir_all(file.parent().unwrap());
        app.sheet_export_to(kentos_sheet_ui::ExportKind::Pdf, Some(file.clone()));
        let pdf = std::fs::read(&file).expect("written");
        let text = String::from_utf8_lossy(&pdf);
        // 20 mm at the window's 150 dpi: 119 pixels, with its transparency as a soft mask.
        assert!(
            text.contains("/Width 119") && text.contains("/SMask"),
            "the SVG as a picture"
        );
        let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
        assert!(
            said.iter().any(|l| l.ends_with(
                "“Kurum logosu” (Resim): SVG resim PDF'e resim olarak gömülür (119 × 119 piksel)."
            )),
            "{said:?}"
        );
    }

    /// Yazdır: the PDF goes to a file of its own and is opened in the system's viewer.
    #[test]
    fn printing_opens_the_pdf_in_the_system_s_viewer() {
        use kentos_sheet_ui::message::PdfMessage;
        let mut app = app_with_drawing();
        new_sheet(&mut app);
        let _ = app.update(Message::Sheet(Sheet::Print));
        let _ = app.update(Message::Sheet(Sheet::Pdf(PdfMessage::Print)));
        let opened = crate::sheet_pdf::OPENED.with(|o| o.borrow().clone());
        let path = opened.last().expect("a file opened in the viewer");
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("pdf"));
        assert!(std::fs::read(path).expect("written").starts_with(b"%PDF-"));
        assert!(
            app.log
                .lines()
                .any(|l| l.text.contains("görüntüleyicide açıldı"))
        );
    }

    /// Step 5's windows on the desktop, for the owner beside the web's: the PDF export window
    /// over the ifraz sheet, Kullan's questions over the gallery, and a sheet whose template has
    /// a newer revision (the tab's dot, the inspector's “Yeni sürüm var”). Light theme first.
    ///
    /// ```text
    /// KENTOS_DEMO_DRAWING=$PWD/.run/demo/ornek-1244-1249-ada.json \
    ///   cargo test -p kentos-desktop sheet_step5_screens -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn sheet_step5_screens() {
        use kentos_sheet_ui::gallery::{GalleryMessage, Source};
        use kentos_sheet_ui::save_template::SaveMessage;
        let out =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots/sheet-desktop");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        let (mut app, ifraz) = demo_ifraz("pafta-adim5");
        let sheet = |app: &mut App, m: Sheet| {
            let task = app.update(Message::Sheet(m));
            crate::files_testing::drive(app, task);
        };
        let shoot = |app: &mut App, name: &str| {
            for (theme, tag) in [("light", "pafta"), ("dark", "grafit")] {
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
                app.apply_settings();
                let mut snapshot =
                    kentos_ui::snapshot::Snapshot::software(Size::new(1440.0, 900.0))
                        .expect("a renderer");
                let mut update = |app: &mut App, m: Message| {
                    let _ = app.update(m);
                };
                snapshot.settle(app, App::view, &mut update);
                let file = out.join(format!("masaustu-{name}-{tag}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        };
        sheet(&mut app, Sheet::ZoomPage);
        // The PDF window over the ifraz sheet.
        sheet(&mut app, Sheet::Export(kentos_sheet_ui::ExportKind::Pdf));
        shoot(&mut app, "c5-pdf-penceresi");
        sheet(&mut app, Sheet::CloseDialog);
        // Kullan's questions over the gallery (the imar plan asks fifteen).
        for m in [
            GalleryMessage::Open,
            GalleryMessage::AllModes(true),
            GalleryMessage::Pick("sys:imar-plani".into()),
            GalleryMessage::Use,
        ] {
            sheet(&mut app, Sheet::Gallery(m));
        }
        if app.sheets.gallery_asking().is_some() {
            sheet(&mut app, Sheet::Gallery(GalleryMessage::Answer(true)));
        }
        assert!(app.sheets.questions().is_some(), "the questions");
        sheet(
            &mut app,
            Sheet::Questions(QuestionsMessage::Text(0, "Suşehri Belediyesi".into())),
        );
        shoot(&mut app, "c5-sorular");
        sheet(&mut app, Sheet::CloseDialog);
        sheet(&mut app, Sheet::Gallery(GalleryMessage::Close));
        // The ifraz sheet kept as the user's template; a sheet made from it; the template saved
        // again from that sheet: revision 2, and the sheet says “Yeni sürüm var”.
        sheet(&mut app, Sheet::SaveTemplate(SaveMessage::Open));
        sheet(
            &mut app,
            Sheet::SaveTemplate(SaveMessage::Name("Belediye ifraz paftası".into())),
        );
        sheet(&mut app, Sheet::SaveTemplate(SaveMessage::Save));
        let mine = app
            .sheet_store
            .as_ref()
            .and_then(|s| s.templates().into_iter().next())
            .map(|r| r.id)
            .expect("the user's template");
        for m in [
            GalleryMessage::Open,
            GalleryMessage::Source(Source::Mine),
            GalleryMessage::Pick(mine.clone()),
            GalleryMessage::Use,
        ] {
            sheet(&mut app, Sheet::Gallery(m));
        }
        if app.sheets.gallery_asking().is_some() {
            sheet(&mut app, Sheet::Gallery(GalleryMessage::Answer(true)));
        }
        sheet(&mut app, Sheet::Questions(QuestionsMessage::Make));
        let made = app
            .sheets
            .open_sheet()
            .map(|s| s.id.clone())
            .expect("the sheet");
        assert_ne!(made, ifraz);
        sheet(&mut app, Sheet::SaveTemplate(SaveMessage::Open));
        sheet(&mut app, Sheet::SaveTemplate(SaveMessage::Save));
        let s = app.sheets.open_sheet().expect("the sheet").clone();
        assert!(
            app.sheets.template_newer(&s),
            "revision 2 kept, the sheet made from 1"
        );
        sheet(&mut app, Sheet::Select(Vec::new()));
        sheet(&mut app, Sheet::InspectorTab(InspectorTab::Page));
        // The page tab's first sections folded: its Şablon section in sight.
        for section in ["paper", "margins", "variant"] {
            sheet(&mut app, Sheet::Section(section));
        }
        sheet(&mut app, Sheet::ZoomPage);
        shoot(&mut app, "c5-yeni-surum");
    }

    /// Step 6's north diagram on the desktop (design §8a): the ifraz sheet's north arrow as the
    /// Turkish topographic sheets' diagram, the declination from WMM2025 at the map's centre on
    /// today's date; the whole sheet, then the diagram close up. Light theme first.
    ///
    /// ```text
    /// KENTOS_DEMO_DRAWING=$PWD/.run/demo/ornek-1244-1249-ada.json \
    ///   cargo test -p kentos-desktop sheet_step6_screens -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn sheet_step6_screens() {
        use kentos_sheet::ops::{Op, SetItemProps};
        let out =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots/sheet-desktop");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        let (mut app, ifraz) = demo_ifraz("pafta-adim6");
        let sheet = |app: &mut App, m: Sheet| {
            let task = app.update(Message::Sheet(m));
            crate::files_testing::drive(app, task);
        };
        let arrow = app
            .sheets
            .book()
            .sheet(&ifraz)
            .and_then(|s| {
                s.items
                    .iter()
                    .find(|i| matches!(i.kind, kentos_sheet::kinds::ItemKind::NorthArrow(_)))
            })
            .map(|i| i.id.clone())
            .expect("the ifraz sheet's north arrow");
        assert!(app.sheets.apply(
            vec![Op::SetItemProps(SetItemProps {
                id: arrow.clone(),
                patch: serde_json::json!({ "kind": { "type": "northArrow", "style": "diagram" } }),
            })],
            "Kuzey çizelgesi",
        ));
        let texts: Vec<String> = app
            .sheets
            .display_list()
            .map(|l| {
                l.prims
                    .iter()
                    .filter_map(|p| match p {
                        kentos_sheet::display::Prim::Text(t) if t.item == arrow => {
                            Some(t.text.clone())
                        }
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        println!("{texts:?}");
        assert!(texts.iter().any(|t| t.contains("(WMM2025, ")), "{texts:?}");
        let shoot = |app: &mut App, name: &str, zoom: Option<Sheet>| {
            for (theme, tag) in [("light", "pafta"), ("dark", "grafit")] {
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
                app.apply_settings();
                let mut snapshot =
                    kentos_ui::snapshot::Snapshot::software(Size::new(1440.0, 900.0))
                        .expect("a renderer");
                let mut update = |app: &mut App, m: Message| {
                    let _ = app.update(m);
                };
                // The window at this size first (the stage learns its size), then the zoom.
                snapshot.settle(app, App::view, &mut update);
                if let Some(z) = zoom.clone() {
                    let task = app.update(Message::Sheet(z));
                    crate::files_testing::drive(app, task);
                    snapshot.settle(app, App::view, &mut update);
                }
                let file = out.join(format!("masaustu-{name}-{tag}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        };
        sheet(&mut app, Sheet::Select(Vec::new()));
        shoot(&mut app, "c6-kuzey-cizelgesi", Some(Sheet::ZoomPage));
        sheet(&mut app, Sheet::Select(vec![arrow]));
        shoot(
            &mut app,
            "c6-kuzey-cizelgesi-yakin",
            Some(Sheet::ZoomSelection),
        );
    }

    /// The files the acceptance tools read (gdalinfo, pdffonts, pdftotext, pdfinfo, pdftoppm):
    /// the ifraz system template on the web's demo drawing, written by the desktop's own export.
    ///
    /// ```text
    /// KENTOS_DEMO_DRAWING=$PWD/.run/demo/ornek-1244-1249-ada.json \
    ///   cargo test -p kentos-desktop sheet_pdf_files -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "files for the acceptance tools, run by hand"]
    fn sheet_pdf_files() {
        let out =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots/sheet-pdf");
        std::fs::create_dir_all(&out).expect("a folder for the files");
        let (mut app, ifraz) = demo_ifraz("pafta-pdf-kabul");
        assert_eq!(
            app.sheets.open_sheet().map(|s| s.id.clone()),
            Some(ifraz.clone())
        );
        let _ = app.update(Message::Sheet(Sheet::Export(
            kentos_sheet_ui::ExportKind::Pdf,
        )));
        let file = out.join("masaustu-ifraz.pdf");
        app.sheet_export_to(kentos_sheet_ui::ExportKind::Pdf, Some(file.clone()));
        assert!(
            std::fs::read(&file)
                .expect("written")
                .starts_with(b"%PDF-1.7")
        );
        // The map frames as the sheet places them, for the corner check against `gdalinfo`.
        let list = kentos_sheet::display::display_list(
            app.sheets.book(),
            &ifraz,
            &app.sheets.export_inputs(),
        )
        .expect("the display list");
        let maps: Vec<&kentos_sheet::display::MapPrim> = list
            .prims
            .iter()
            .filter_map(|p| match p {
                kentos_sheet::display::Prim::Map(m) => Some(m),
                _ => None,
            })
            .collect();
        std::fs::write(
            out.join("masaustu-ifraz-haritalar.json"),
            serde_json::to_string_pretty(&maps).expect("the maps"),
        )
        .expect("written");
        println!("{}", file.display());
    }

    /// A preflight fix the desktop does: Koordinat sistemi seç opens the
    /// project's settings at its coordinate system, over the sheet.
    #[test]
    fn the_preflight_s_coordinate_system_fix_opens_the_project_s_settings() {
        let mut app = app_with_drawing();
        new_sheet(&mut app);
        let task = app.sheet_effects(vec![Effect::HostAction("project.crs".into())]);
        crate::files_testing::drive(&mut app, task);
        assert_eq!(app.dialog, Some(crate::app::Dialog::Project));
        assert!(matches!(
            &app.project,
            Some(crate::project::Window::Settings(_))
        ));
        // What the desktop does not have yet is said, not swallowed.
        let _ = app.sheet_effects(vec![Effect::HostAction("sheet.atlas".into())]);
        assert!(crate::files_testing::last_said(&app).contains("sheet.atlas"));
    }

    /// The map's content keeps inside its box, its view turned or not (a turned
    /// frame turns the box, so nothing spills past the frame), and the drawing's
    /// text is written in it as the drawing area writes it.
    #[test]
    fn a_map_keeps_inside_its_frame_and_writes_the_drawing_s_text() {
        let mut app = app_with_drawing();
        app.spatial.reload(&app.document.as_ref().unwrap().model);
        app.sheet_maps.sync(app.document.as_ref());
        let doc = app.document.as_ref().unwrap();
        let b = scene::extents(doc).expect("the drawing has objects");
        let span = (b.max_x - b.min_x).max(b.max_y - b.min_y).max(1.0);
        let picture = |rotation: i32, labels: bool| {
            let view = kentos_sheet::display::MapViewPrim {
                center: Some(GroundPoint {
                    x: (b.min_x + b.max_x) / 2.0,
                    y: (b.min_y + b.max_y) / 2.0,
                }),
                // Large enough for the drawing's labels at the paper's own size (the
                // parcel's name, the dimension's value: five pixels at least).
                scale: 250,
                rotation,
            };
            let layers = MapLayers::default();
            let r = MapRequest {
                item: "m",
                // As large as a map's picture is drawn (no shrinking: the text at its own size).
                size: Size::new(640.0, 480.0),
                view: &view,
                layers: &layers,
                crs: None,
                extent: Some([
                    b.min_x - span,
                    b.min_y - span,
                    b.max_x + span,
                    b.max_y + span,
                ]),
                pixels_per_metre: 960.0 / span,
                export: false,
            };
            struct One<'a>(SheetPainter<'a>, MapRequest<'a>, bool);
            impl canvas::Program<()> for One<'_> {
                type State = ();
                fn draw(
                    &self,
                    _: &(),
                    r: &iced::Renderer,
                    _: &iced::Theme,
                    b: iced::Rectangle,
                    _: iced::mouse::Cursor,
                ) -> Vec<canvas::Geometry> {
                    let mut f = Frame::new(r, b.size());
                    f.fill_rectangle(iced::Point::ORIGIN, b.size(), iced::Color::WHITE);
                    if self.2 {
                        assert!(self.0.paint(&self.1, &mut f));
                    } else {
                        assert!(self.0.maps.paint_scene(&self.1, &mut f));
                    }
                    vec![f.into_geometry()]
                }
            }
            let view: iced::Element<'_, ()> =
                iced::widget::canvas(One(app.sheet_painter(), r, labels))
                    .width(iced::Fill)
                    .height(iced::Fill)
                    .into();
            kentos_ui::snapshot::Snapshot::software(Size::new(800.0, 600.0))
                .expect("a renderer")
                .render(view, &app.theme())
        };
        let ink = |image: &kentos_ui::snapshot::Image| {
            let (mut inside, mut outside) = (0, 0);
            for (i, p) in image.rgba.chunks_exact(4).enumerate() {
                let (x, y) = ((i % 800) as f32, (i / 800) as f32);
                if p[0] < 245 || p[1] < 245 || p[2] < 245 {
                    if x <= 641.0 && y <= 481.0 {
                        inside += 1;
                    } else {
                        outside += 1;
                    }
                }
            }
            (inside, outside)
        };
        for rotation in [0, 30_000] {
            let (scene_only, out) = ink(&picture(rotation, false));
            assert_eq!(out, 0, "turned {rotation}: the scene keeps inside");
            let (with_text, out) = ink(&picture(rotation, true));
            assert_eq!(out, 0, "turned {rotation}: the text keeps inside");
            assert!(
                with_text > scene_only + 30,
                "turned {rotation}: the drawing's text is written ({scene_only} → {with_text})"
            );
        }
    }

    /// The web's demo drawing (`Ornek_1244-1249_Ada`, written by
    /// apps/web/scripts/style/demo-drawing.test.ts) open in the CAD mode, the drawing area on
    /// its home view, and the ifraz sheet from the system template with the web's values and
    /// parcel table columns (sheet-shots.mjs `VALUES`, `COLUMNS`).
    fn demo_ifraz(store: &str) -> (App, String) {
        use kentos_sheet::ops::Op;
        use kentos_sheet_ui::gallery::GalleryMessage;
        crate::drawing_fonts::load();
        kentos_ui::theme::motion::set_reduced(true);
        let path = std::env::var("KENTOS_DEMO_DRAWING").unwrap_or_else(|_| {
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../.run/demo/ornek-1244-1249-ada.json"
            )
            .to_owned()
        });
        let text =
            std::fs::read_to_string(&path).expect("the web's demo drawing (see the comment)");
        let snapshot = kentos_contracts::DocumentSnapshotV1::from_json(&text).expect("reads");
        let mut doc = Document::new(snapshot, None).expect("opens");
        doc.model.set_name("Ornek_1244-1249_Ada.kcad");
        let mut settings = doc.settings().clone();
        settings.workspace = Some(kentos_contracts::Workspace::Cad);
        doc.model.set_settings(settings);
        let home = doc.model.home_view();
        let (mut app, _) = App::boot(None);
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        app.sheet_store = Some(
            kentos_sheet_ui::Store::open(&crate::files_testing::scratch(store)).expect("a store"),
        );
        // The sheets look where the web's do: the drawing's home view.
        if let Some(h) = home {
            app.viewport.camera.center =
                Vec2::new((h.min_x + h.max_x) / 2.0, (h.min_y + h.max_y) / 2.0);
        }
        let sheet = |app: &mut App, m: Sheet| {
            let task = app.update(Message::Sheet(m));
            crate::files_testing::drive(app, task);
        };
        let gallery = |app: &mut App, m: GalleryMessage| sheet(app, Sheet::Gallery(m));
        let from = |app: &mut App, id: &str| {
            gallery(app, GalleryMessage::Open);
            gallery(app, GalleryMessage::Pick(id.into()));
            gallery(app, GalleryMessage::Use);
            if app.sheets.gallery_asking().is_some() {
                gallery(app, GalleryMessage::Answer(true));
            }
            // The template's questions, answered with its own values (set below as the web's).
            if app.sheets.questions().is_some() {
                sheet(app, Sheet::Questions(QuestionsMessage::Make));
            }
            app.sheets
                .open_sheet()
                .map(|s| s.id.clone())
                .expect("a sheet")
        };
        let ifraz = from(&mut app, "sys:ifraz-paftasi");
        // The web's values and the parcel table's columns (sheet-shots.mjs `VALUES`, `COLUMNS`).
        let values = serde_json::json!({ "il": "Sivas", "ilce": "Suşehri", "mahalle": "Kızılırmak", "ada": "1245", "pafta_no": "P-12" });
        let s = app.sheets.open_sheet().expect("the ifraz sheet").clone();
        let variables: Vec<serde_json::Value> = s
            .variables
            .iter()
            .map(|v| {
                let mut j = serde_json::to_value(v).expect("a variable");
                if let Some(x) = values.get(&v.name) {
                    j["value"] = x.clone();
                }
                j
            })
            .collect();
        let table = s
            .items
            .iter()
            .find(|i| matches!(i.kind, kentos_sheet::kinds::ItemKind::Table(_)))
            .map(|i| i.id.clone())
            .expect("the parcel table");
        let ops: Vec<Op> = serde_json::from_value(serde_json::json!([
            { "op": "saveVariables", "sheet": ifraz, "variables": variables },
            { "op": "setItemProps", "id": table, "patch": { "kind": {
                "columns": [
                    { "heading": "Parsel", "value": "Parsel", "width": 13000, "align": "center" },
                    { "heading": "Yüzölçümü (m²)", "value": "$alan", "width": 0, "align": "right", "decimals": 2 },
                    { "heading": "Tapu alanı (m²)", "value": "[Tapu alanı (m²)]", "width": 0, "align": "right", "decimals": 2 },
                    { "heading": "Nitelik", "value": "Nitelik", "width": 0, "align": "left" }
                ],
                "source": { "sort": [{ "expression": "to_int(Parsel)", "descending": false }] }
            } } }
        ]))
        .expect("the web's operations");
        assert!(
            app.sheets.apply(ops, "Değişkenler ve parsel tablosu"),
            "{:?}",
            app.sheets.error()
        );
        (app, ifraz)
    }

    /// Pictures of the desktop for setting beside the web's
    /// (apps/web/scripts/e2e/sheet-shots.mjs, `b1`, `b4`): the web's demo
    /// drawing (`Ornek_1244-1249_Ada`, written by
    /// apps/web/scripts/style/demo-drawing.test.ts), in the CAD mode, the ifraz
    /// sheet from the system template with the web's values and parcel table
    /// columns, an aplikasyon sketch beside it, a template of the user's own
    /// saved from it; the sheet, then the gallery's Sistem and Benim, light
    /// (Pafta) first, then Grafit, 1440 × 900 and 1100 × 650.
    ///
    /// ```text
    /// KENTOS_DEMO_DRAWING=$PWD/.run/demo/ornek-1244-1249-ada.json \
    ///   cargo test -p kentos-desktop sheet_comparison_screens -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn sheet_comparison_screens() {
        use kentos_sheet::ops::Op;
        use kentos_sheet_ui::gallery::{GalleryMessage, Source};
        use kentos_sheet_ui::save_template::SaveMessage;
        let out =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots/sheet-desktop");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        let (mut app, ifraz) = demo_ifraz("pafta-karsilastirma");
        let sheet = |app: &mut App, m: Sheet| {
            let task = app.update(Message::Sheet(m));
            crate::files_testing::drive(app, task);
        };
        let gallery = |app: &mut App, m: GalleryMessage| sheet(app, Sheet::Gallery(m));
        let from = |app: &mut App, id: &str| {
            gallery(app, GalleryMessage::Open);
            gallery(app, GalleryMessage::Pick(id.into()));
            gallery(app, GalleryMessage::Use);
            if app.sheets.gallery_asking().is_some() {
                gallery(app, GalleryMessage::Answer(true));
            }
            // The template's questions, answered with its own values (set below as the web's).
            if app.sheets.questions().is_some() {
                sheet(app, Sheet::Questions(QuestionsMessage::Make));
            }
            app.sheets
                .open_sheet()
                .map(|s| s.id.clone())
                .expect("a sheet")
        };
        // A template of the user's own, as the web's run saves one.
        sheet(&mut app, Sheet::SaveTemplate(SaveMessage::Open));
        sheet(
            &mut app,
            Sheet::SaveTemplate(SaveMessage::Name("Belediye ifraz paftası".into())),
        );
        sheet(&mut app, Sheet::SaveTemplate(SaveMessage::Description(
            "Belediyenin ifraz paftası: Kızılırmak mahallesi düzeni, parsel tablosu ve imzalar.".into(),
        )));
        sheet(
            &mut app,
            Sheet::SaveTemplate(SaveMessage::Category("kadastro".into())),
        );
        sheet(
            &mut app,
            Sheet::SaveTemplate(SaveMessage::Tags("ifraz, belediye".into())),
        );
        sheet(&mut app, Sheet::SaveTemplate(SaveMessage::Save));
        let _aplikasyon = from(&mut app, "sys:aplikasyon-krokisi");
        let at = app
            .sheets
            .book()
            .sheets
            .iter()
            .position(|s| s.id == ifraz)
            .expect("the ifraz sheet");
        sheet(&mut app, Sheet::Tab(at + 1));
        sheet(&mut app, Sheet::Select(Vec::new()));
        sheet(&mut app, Sheet::ZoomPage);
        // What the preflight finds on the ifraz sheet, for the comparison with the web's.
        for f in app.sheets.findings() {
            println!("Ön denetim: {:?} {} — {}", f.severity, f.code, f.message);
        }
        let mine = app
            .sheet_store
            .as_ref()
            .and_then(|s| s.templates().into_iter().next())
            .map(|r| r.id)
            .expect("the user's template");
        // A picture frame in the title row given an institution's logo, as Resim seç… gives
        // it; then Seçim (the frame fills the stage) so the picture is seen close.
        let logo = logo_png(512);
        let title = app
            .sheets
            .book()
            .sheet(&ifraz)
            .and_then(|s| s.items.iter().find(|i| i.name == "Başlık ve yer"))
            .map(|i| i.frame)
            .expect("the ifraz sheet's title block");
        let ifraz_sheet = ifraz.clone();
        let with_logo = move |app: &mut App| {
            use kentos_sheet::kinds::{ItemKind, PictureFit, PictureItem};
            let frame = kentos_sheet::units::RectUm {
                left: title.left + 2_000,
                top: title.top + 1_500,
                width: 10_000,
                height: 10_000,
            };
            let picture = kentos_sheet::model::Item::new(
                "logo",
                "Kurum logosu",
                frame,
                ItemKind::Picture(PictureItem {
                    asset: None,
                    fit: PictureFit::Contain,
                    clip: true,
                }),
            );
            assert!(app.sheets.apply(
                vec![Op::AddItems(kentos_sheet::ops::AddItems {
                    to: kentos_sheet::model::Owner::sheet(&ifraz_sheet),
                    items: vec![picture],
                    index: None,
                })],
                "Resim çerçevesi",
            ));
            sheet(app, Sheet::Select(vec!["logo".into()]));
            sheet(
                app,
                Sheet::PictureFile("kurum-logosu.png".into(), logo.clone()),
            );
            sheet(app, Sheet::ZoomSelection);
        };
        let without_logo = move |app: &mut App| {
            sheet(app, Sheet::Undo);
            sheet(app, Sheet::Undo);
            sheet(app, Sheet::Select(Vec::new()));
            sheet(app, Sheet::ZoomPage);
        };
        let close_gallery = |app: &mut App| {
            let t = app.update(Message::Sheet(Sheet::Gallery(GalleryMessage::Close)));
            crate::files_testing::drive(app, t);
        };
        type Scene = (&'static str, Box<dyn Fn(&mut App)>, Box<dyn Fn(&mut App)>);
        let scenes: Vec<Scene> = vec![
            (
                "b1-ifraz-sistem-sablonu",
                Box::new(|_: &mut App| {}),
                Box::new(close_gallery),
            ),
            (
                "b4-galeri-sistem",
                Box::new(move |app: &mut App| {
                    let t = app.update(Message::Sheet(Sheet::Gallery(GalleryMessage::Open)));
                    crate::files_testing::drive(app, t);
                    let t = app.update(Message::Sheet(Sheet::Gallery(GalleryMessage::Pick(
                        "sys:ifraz-paftasi".into(),
                    ))));
                    crate::files_testing::drive(app, t);
                }),
                Box::new(close_gallery),
            ),
            (
                "b4-galeri-benim",
                Box::new(move |app: &mut App| {
                    for m in [
                        GalleryMessage::Open,
                        GalleryMessage::Source(Source::Mine),
                        GalleryMessage::Pick(mine.clone()),
                    ] {
                        let t = app.update(Message::Sheet(Sheet::Gallery(m)));
                        crate::files_testing::drive(app, t);
                    }
                }),
                Box::new(close_gallery),
            ),
            ("resim-logo", Box::new(with_logo), Box::new(without_logo)),
        ];
        for (theme, tag) in [("light", "pafta"), ("dark", "grafit")] {
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            for (scene, open, close) in &scenes {
                for (w, h) in [(1440.0, 900.0), (1100.0, 650.0)] {
                    let mut snapshot = kentos_ui::snapshot::Snapshot::software(Size::new(w, h))
                        .expect("a renderer");
                    let mut update = |app: &mut App, m: Message| {
                        let _ = app.update(m);
                    };
                    // The window at this size first (the stage learns its size), then the scene.
                    snapshot.settle(&mut app, App::view, &mut update);
                    open(&mut app);
                    snapshot.settle(&mut app, App::view, &mut update);
                    let file = out.join(format!("masaustu-{scene}-{tag}-{}.png", w as u32));
                    snapshot
                        .render(app.view(), &app.theme())
                        .save(&file)
                        .expect("writes the picture");
                    println!("{}", file.display());
                    // Each scene from the sheet in front.
                    close(&mut app);
                }
            }
        }
    }

    /// Step 8's pictures (design, open question 6): in the ifraz sheet's title row a small
    /// logo (a 64-pixel PNG) and the KentOS sign as an SVG, then seen close (Seçim: the two
    /// frames fill the stage): smooth, not squares; the SVG drawn by resvg. The PNG export of the
    /// same sheet, cut round the two pictures, beside it.
    ///
    /// ```text
    /// cargo test -p kentos-desktop sheet_step8_screens -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn sheet_step8_screens() {
        use kentos_sheet::kinds::{ItemKind, PictureFit, PictureItem};
        use kentos_sheet::ops::Op;
        let out =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots/sheet-desktop");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        let (mut app, ifraz) = demo_ifraz("pafta-adim8");
        let sheet = |app: &mut App, m: Sheet| {
            let task = app.update(Message::Sheet(m));
            crate::files_testing::drive(app, task);
        };
        let title = app
            .sheets
            .book()
            .sheet(&ifraz)
            .and_then(|s| s.items.iter().find(|i| i.name == "Başlık ve yer"))
            .map(|i| i.frame)
            .expect("the ifraz sheet's title block");
        let sign = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/public/favicon.svg"),
        )
        .expect("the KentOS sign");
        let pictures = [
            (
                "logo",
                "Kurum logosu",
                2_000,
                "kurum-logosu.png",
                logo_png(64),
            ),
            ("isaret", "KentOS işareti", 14_000, "kentos.svg", sign),
        ];
        for (id, name, dx, file, bytes) in pictures {
            let frame = kentos_sheet::units::RectUm {
                left: title.left + dx,
                top: title.top + 1_500,
                width: 10_000,
                height: 10_000,
            };
            let item = kentos_sheet::model::Item::new(
                id,
                name,
                frame,
                ItemKind::Picture(PictureItem {
                    asset: None,
                    fit: PictureFit::Contain,
                    clip: true,
                }),
            );
            assert!(app.sheets.apply(
                vec![Op::AddItems(kentos_sheet::ops::AddItems {
                    to: kentos_sheet::model::Owner::sheet(&ifraz),
                    items: vec![item],
                    index: None,
                })],
                "Resim çerçevesi",
            ));
            sheet(&mut app, Sheet::Select(vec![id.into()]));
            sheet(&mut app, Sheet::PictureFile(file.into(), bytes));
        }
        let kinds: Vec<_> = app
            .sheets
            .book()
            .assets
            .iter()
            .map(|a| (a.kind, a.width, a.height))
            .collect();
        println!("{kinds:?}");
        assert!(kinds.contains(&(kentos_sheet::model::AssetKind::Png, 64, 64)));
        assert!(kinds.contains(&(kentos_sheet::model::AssetKind::Svg, 32, 32)));
        sheet(
            &mut app,
            Sheet::Select(vec!["logo".into(), "isaret".into()]),
        );
        for (theme, tag) in [("light", "pafta"), ("dark", "grafit")] {
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let mut snapshot = kentos_ui::snapshot::Snapshot::software(Size::new(1440.0, 900.0))
                .expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let task = app.update(Message::Sheet(Sheet::ZoomSelection));
            crate::files_testing::drive(&mut app, task);
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("masaustu-c8-resim-yakin-{tag}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
        // The PNG export at 400 dpi (the pictures magnified there too), cut round the two pictures.
        sheet(&mut app, Sheet::ExportDpi(400));
        let png = out.join("masaustu-c8-resim-disa-aktarim.png");
        app.sheet_export_to(ExportKind::Png, Some(png.clone()));
        let bytes = std::fs::read(&png).expect("the export");
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::EXPAND);
        let mut reader = decoder.read_info().expect("a PNG");
        let mut buf = vec![0; reader.output_buffer_size().expect("a size")];
        let info = reader.next_frame(&mut buf).expect("its pixels");
        let page = app
            .sheets
            .book()
            .sheet(&ifraz)
            .map(|s| s.page.size)
            .expect("a page");
        let k = f64::from(info.width) / f64::from(page.width);
        let px = |um: i32| (f64::from(um) * k).round() as u32;
        let (x0, y0) = (px(title.left + 1_000), px(title.top + 500));
        let (cw, ch) = (px(24_000), px(12_000));
        let channels = buf.len() / (info.width as usize * info.height as usize);
        let mut crop = Vec::with_capacity((cw * ch * 4) as usize);
        for y in y0..y0 + ch {
            for x in x0..x0 + cw {
                let i = (y as usize * info.width as usize + x as usize) * channels;
                let p = &buf[i..i + channels];
                crop.extend_from_slice(&[p[0], p[1], p[2], if channels == 4 { p[3] } else { 255 }]);
            }
        }
        let cut = out.join("masaustu-c8-resim-disa-aktarim-kesit.png");
        let mut file = std::io::BufWriter::new(std::fs::File::create(&cut).expect("a file"));
        let mut e = png::Encoder::new(&mut file, cw, ch);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        e.write_header()
            .and_then(|mut w| w.write_image_data(&crop))
            .expect("writes the cut");
        println!(
            "{} ({}×{} px, {} dpi)",
            cut.display(),
            info.width,
            info.height,
            (k * 25_400.0).round()
        );
    }

    /// Step 9's pictures: the north arrow's declination in the inspector (the ifraz sheet's arrow
    /// on magnetic north: what the paper writes and from what, the date and where it comes from);
    /// the ifraz map with the demo's buildings in a pattern fill (no vector form: the styled
    /// shaders' CPU twin draws the map on the screen) and the same map cut from its PDF; an SVG
    /// logo in the title row cut from the PDF (drawn here as a PNG, the core draws no SVG).
    ///
    /// ```text
    /// cargo test -p kentos-desktop sheet_step9_screens -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn sheet_step9_screens() {
        use kentos_sheet::kinds::{ItemKind, PictureFit, PictureItem};
        use kentos_sheet::ops::{Op, SetItemProps};
        let out =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots/sheet-desktop");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        let (mut app, ifraz) = demo_ifraz("pafta-adim9");
        let sheet = |app: &mut App, m: Sheet| {
            let task = app.update(Message::Sheet(m));
            crate::files_testing::drive(app, task);
        };
        let shoot = |app: &mut App, name: &str, zoom: Option<Sheet>| {
            for (theme, tag) in [("light", "pafta"), ("dark", "grafit")] {
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
                app.apply_settings();
                let mut snapshot =
                    kentos_ui::snapshot::Snapshot::software(Size::new(1440.0, 900.0))
                        .expect("a renderer");
                let mut update = |app: &mut App, m: Message| {
                    let _ = app.update(m);
                };
                snapshot.settle(app, App::view, &mut update);
                if let Some(z) = zoom.clone() {
                    let task = app.update(Message::Sheet(z));
                    crate::files_testing::drive(app, task);
                    snapshot.settle(app, App::view, &mut update);
                }
                let file = out.join(format!("masaustu-{name}-{tag}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from("light"))]);
            app.apply_settings();
        };
        let items = |app: &App| {
            app.sheets
                .book()
                .sheet(&ifraz)
                .map(|s| s.items.clone())
                .unwrap_or_default()
        };
        // 1. The north arrow on magnetic north, chosen: its declination in the inspector.
        let arrow = items(&app)
            .iter()
            .find(|i| matches!(i.kind, ItemKind::NorthArrow(_)))
            .map(|i| i.id.clone())
            .expect("the ifraz sheet's north arrow");
        assert!(app.sheets.apply(
            vec![Op::SetItemProps(SetItemProps {
                id: arrow.clone(),
                patch: serde_json::json!({ "kind": { "type": "northArrow", "north": "magnetic" } }),
            })],
            "Kuzey",
        ));
        sheet(&mut app, Sheet::Select(vec![arrow]));
        // The frame's and the constraints' sections closed: the arrow's own fields in view.
        sheet(&mut app, Sheet::Section("frame"));
        sheet(&mut app, Sheet::Section("constraints"));
        let info = app
            .sheets
            .north_info()
            .cloned()
            .expect("what the arrow shows");
        println!(
            "Kuzey: {:?} {:?} {:?} {:?}",
            info.declination, info.model, info.date, info.date_source
        );
        assert_eq!(
            info.source,
            Some(kentos_sheet::display::DeclinationSource::Model)
        );
        shoot(&mut app, "c9-kuzey-denetci", Some(Sheet::ZoomPage));
        sheet(&mut app, Sheet::Section("frame"));
        sheet(&mut app, Sheet::Section("constraints"));
        sheet(&mut app, Sheet::Select(Vec::new()));
        // 2. The demo's buildings in a pattern fill: a light fill, small crosses, the outline.
        {
            let doc = app.document.as_mut().expect("the demo drawing");
            let mut style = doc
                .model
                .layers()
                .get("yapi")
                .map(|l| l.style.clone())
                .expect("the Yapı layer");
            style.renderer = Some(
                serde_json::json!({ "type": "single", "symbols": { "fill": { "type": "fill", "layers": [
                { "id": "f", "type": "simpleFill", "color": "#DCE8F6" },
                { "id": "p", "type": "patternFill", "spacingX": 1.6, "spacingY": 1.6, "stagger": true,
                  "marker": { "type": "marker", "layers": [{ "id": "x", "type": "shape", "shape": "x", "size": 0.7, "fill": "#1F4A96", "stroke": null }] } },
                { "id": "o", "type": "simpleLine", "color": "#1F4A96", "width": 0.3 }
            ] } } }),
            );
            assert!(doc.model.set_layer_style("yapi", style, "Desenli yapılar"));
        }
        app.sheet_maps.sync(app.document.as_ref());
        let ways = app.sheets.pdf_ways(&app.sheet_painter());
        println!("{ways:?}");
        assert!(
            ways.iter()
                .any(|w| !w.vector && w.why.contains("desen dolgusu"))
        );
        let map = items(&app)
            .iter()
            .find(|i| matches!(i.kind, ItemKind::Map(_)))
            .map(|i| (i.id.clone(), i.frame))
            .expect("the ifraz sheet's map");
        sheet(&mut app, Sheet::Select(vec![map.0.clone()]));
        shoot(&mut app, "c9-desenli-harita", Some(Sheet::ZoomSelection));
        sheet(&mut app, Sheet::Select(Vec::new()));
        // 3. An SVG logo in the title row (the KentOS sign).
        let title = items(&app)
            .iter()
            .find(|i| i.name == "Başlık ve yer")
            .map(|i| i.frame)
            .expect("the ifraz sheet's title block");
        let sign = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/public/favicon.svg"),
        )
        .expect("the KentOS sign");
        let logo = kentos_sheet::units::RectUm {
            left: title.left + 2_000,
            top: title.top + 1_500,
            width: 10_000,
            height: 10_000,
        };
        assert!(app.sheets.apply(
            vec![Op::AddItems(kentos_sheet::ops::AddItems {
                to: kentos_sheet::model::Owner::sheet(&ifraz),
                items: vec![kentos_sheet::model::Item::new(
                    "isaret",
                    "KentOS işareti",
                    logo,
                    ItemKind::Picture(PictureItem {
                        asset: None,
                        fit: PictureFit::Contain,
                        clip: true,
                    }),
                )],
                index: None,
            })],
            "Resim çerçevesi",
        ));
        sheet(&mut app, Sheet::Select(vec!["isaret".into()]));
        sheet(&mut app, Sheet::PictureFile("kentos.svg".into(), sign));
        sheet(&mut app, Sheet::Select(Vec::new()));
        // The PDF at 200 dpi (the map's picture and the SVG's PNG at that resolution).
        sheet(&mut app, Sheet::ExportDpi(200));
        let _ = app.update(Message::Sheet(Sheet::Export(ExportKind::Pdf)));
        let pdf = out.join("masaustu-c9.pdf");
        app.sheet_export_to(ExportKind::Pdf, Some(pdf.clone()));
        let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
        println!("{:?}", &said[said.len().saturating_sub(4)..]);
        assert!(
            said.iter()
                .any(|l| l.contains("SVG resim PDF'e resim olarak gömülür")),
            "{said:?}"
        );
        // Cuts of the page at 200 dpi: the map's south-west corner with buildings, and the logo.
        let cut = |name: &str, r: kentos_sheet::units::RectUm| {
            let px = |um: i32| (f64::from(um) / 25_400.0 * 200.0).round() as i64;
            let file = out.join(name);
            let made = std::process::Command::new("pdftoppm")
                .args(["-r", "200", "-png", "-singlefile", "-f", "1", "-l", "1"])
                .args(["-x", &px(r.left).to_string(), "-y", &px(r.top).to_string()])
                .args([
                    "-W",
                    &px(r.width).to_string(),
                    "-H",
                    &px(r.height).to_string(),
                ])
                .arg(&pdf)
                .arg(file.with_extension(""))
                .status();
            if made.is_ok_and(|s| s.success()) {
                println!("{}", file.display());
            }
        };
        let (_, m) = map;
        cut(
            "masaustu-c9-desenli-harita-pdf.png",
            kentos_sheet::units::RectUm {
                left: m.left + m.width / 4,
                top: m.top + m.height / 4,
                width: m.width / 2,
                height: m.height / 2,
            },
        );
        cut(
            "masaustu-c9-svg-pdf.png",
            kentos_sheet::units::RectUm {
                left: logo.left - 2_000,
                top: logo.top - 1_500,
                width: logo.width + 4_000,
                height: logo.height + 3_000,
            },
        );
    }

    /// An institution's emblem for the picture scene: a disc shading from navy
    /// to teal, a white ring and a north arrow, the edges smoothed (16 samples
    /// a pixel), the corners transparent.
    fn logo_png(side: u32) -> Vec<u8> {
        let s = f64::from(side);
        let c = s / 2.0;
        // Inside triangle a-b-c (either winding).
        let tri = |p: [f64; 2], a: [f64; 2], b: [f64; 2], d: [f64; 2]| {
            let side = |u: [f64; 2], v: [f64; 2]| {
                (v[0] - u[0]) * (p[1] - u[1]) - (v[1] - u[1]) * (p[0] - u[0])
            };
            let (x, y, z) = (side(a, b), side(b, d), side(d, a));
            (x >= 0.0 && y >= 0.0 && z >= 0.0) || (x <= 0.0 && y <= 0.0 && z <= 0.0)
        };
        let (apex, left, notch, right) = (
            [c, 0.2 * s],
            [c - 0.15 * s, 0.72 * s],
            [c, 0.6 * s],
            [c + 0.15 * s, 0.72 * s],
        );
        let mut px = Vec::with_capacity((side * side * 4) as usize);
        for j in 0..side {
            for i in 0..side {
                let mut sum = [0.0f64; 4];
                for k in 0..16 {
                    let p = [
                        f64::from(i) + (f64::from(k % 4) + 0.5) / 4.0,
                        f64::from(j) + (f64::from(k / 4) + 0.5) / 4.0,
                    ];
                    let r = ((p[0] - c).powi(2) + (p[1] - c).powi(2)).sqrt();
                    if r > 0.47 * s {
                        continue;
                    }
                    let white = (0.38 * s..0.41 * s).contains(&r)
                        || tri(p, apex, left, notch)
                        || tri(p, apex, notch, right);
                    let t = p[1] / s;
                    let rgb = if white {
                        [255.0, 255.0, 255.0]
                    } else {
                        [29.0 - 3.0 * t, 59.0 + 68.0 * t, 114.0 + 24.0 * t]
                    };
                    sum = [
                        sum[0] + rgb[0],
                        sum[1] + rgb[1],
                        sum[2] + rgb[2],
                        sum[3] + 1.0,
                    ];
                }
                let n = sum[3].max(1.0);
                px.extend([
                    (sum[0] / n).round() as u8,
                    (sum[1] / n).round() as u8,
                    (sum[2] / n).round() as u8,
                    (sum[3] / 16.0 * 255.0).round() as u8,
                ]);
            }
        }
        let mut out = Vec::new();
        {
            let mut e = png::Encoder::new(&mut out, side, side);
            e.set_color(png::ColorType::Rgba);
            e.set_depth(png::BitDepth::Eight);
            let mut w = e.write_header().expect("a header");
            w.write_image_data(&px).expect("the pixels");
        }
        out
    }

    #[test]
    fn a_map_shows_the_drawing_from_the_drawing_pipeline_s_scene() {
        let mut app = app_with_drawing();
        new_sheet(&mut app);
        let list = app.sheets.display_list().expect("a list").clone();
        let map = list
            .prims
            .iter()
            .find_map(|p| match p {
                kentos_sheet::display::Prim::Map(m) => Some(m.clone()),
                _ => None,
            })
            .expect("the sheet has a map");
        // Fit the map on the drawing: its centre and a scale that shows it all.
        let doc = app.document.as_ref().unwrap();
        let b = scene::extents(doc).expect("the drawing has objects");
        let view = kentos_sheet::display::MapViewPrim {
            center: Some(GroundPoint {
                x: (b.min_x + b.max_x) / 2.0,
                y: (b.min_y + b.max_y) / 2.0,
            }),
            ..map.view.clone()
        };
        let span = (b.max_x - b.min_x).max(b.max_y - b.min_y).max(1.0);
        let size = Size::new(400.0, 300.0);
        let k = 300.0 / (span * 1.2) / 1_000_000.0 * f64::from(map.view.scale);
        let extent = Some([
            b.min_x - span,
            b.min_y - span,
            b.max_x + span,
            b.max_y + span,
        ]);
        let prim = kentos_sheet::display::MapPrim {
            view,
            extent,
            ..map
        };
        let request = MapRequest::of(&prim, size, k, false);
        app.sheet_maps.sync(app.document.as_ref());
        struct One<'a>(SheetPainter<'a>, MapRequest<'a>);
        impl canvas::Program<()> for One<'_> {
            type State = ();
            fn draw(
                &self,
                _: &(),
                r: &iced::Renderer,
                _: &iced::Theme,
                b: iced::Rectangle,
                _: iced::mouse::Cursor,
            ) -> Vec<canvas::Geometry> {
                let mut f = Frame::new(r, b.size());
                f.fill_rectangle(iced::Point::ORIGIN, b.size(), iced::Color::WHITE);
                assert!(self.0.paint(&self.1, &mut f));
                vec![f.into_geometry()]
            }
        }
        let view: iced::Element<'_, ()> = iced::widget::canvas(One(app.sheet_painter(), request))
            .width(iced::Fill)
            .height(iced::Fill)
            .into();
        let image = kentos_ui::snapshot::Snapshot::software(size)
            .expect("a renderer")
            .render(view, &app.theme());
        let inked = image
            .rgba
            .chunks_exact(4)
            .filter(|p| p[0] < 200 || p[1] < 200 || p[2] < 200)
            .count();
        assert!(
            inked > 50,
            "the drawing's lines are on the map: {inked} pixels"
        );
        // A changed drawing is a new revision: the stage paints its maps again.
        let before = app.sheet_maps.revision();
        app.sheet_maps.sync(app.document.as_ref());
        assert_eq!(
            app.sheet_maps.revision(),
            before,
            "nothing changed, nothing built"
        );
    }
}
