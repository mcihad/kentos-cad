//! The drawing's text over the scene (docs/adr/0055): text objects,
//! dimension values and object labels, as the web's overlay draws them
//! (`drawLabels`, apps/web/src/viewport/overlay.ts). Which of them a view
//! shows and where comes from the shared geometry store
//! ([`Spatial::labels`]: visible layer, box in view, size on screen, the
//! label style's scale range and smallest feature); strings, sizes and
//! colours from the object and its layer's label style, or the web's
//! default for its kind. Labels are thinned where they would cover one
//! another (8 px cells, as on the web); text objects and dimension values
//! always draw. Everything is in the project's typeface
//! (drawing_fonts.rs) with a halo of the drawing area's colour.
//!
//! The picture is kept while the view, the drawing and the colours stay;
//! upright text goes through Iced's glyph cache, turned text as outlines.

use std::cell::Cell;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Mutex;

use iced::advanced::graphics::text::Paragraph;
use iced::advanced::text::{self, Paragraph as _};
use iced::alignment::Vertical;
use iced::widget::canvas::{self, Frame, LineJoin, Path, Stroke};
use iced::{
    Color, Element, Fill, Font, Pixels, Point, Rectangle, Renderer, Size, Theme, Vector, mouse,
};
use kentos_contracts::{DrawingFont, Entity, LabelInk, LabelStyle};
use kentos_domain::Document;
use kentos_interaction::spatial::default_label;
use kentos_interaction::{Format, LabelSpot, Spatial, Vec2};
use kentos_render_wgpu::Camera;
use kentos_render_wgpu::color::{Palette, Rgba8};

use crate::app::Message;
use crate::drawing_fonts;
use crate::exchange::dimension_text;
use crate::viewport::Canvas;

/// The labels' layer over the drawing area.
pub fn layer<'a>(
    doc: &'a Document,
    spatial: &Spatial,
    camera: &Camera,
    canvas: Canvas,
    palette: &Palette,
    format: &Format,
) -> Element<'a, Message> {
    let view = camera.visible_bounds();
    let spots = spatial.labels(
        Vec2::new(view.min_x, view.min_y),
        Vec2::new(view.max_x, view.max_y),
        camera.scale,
    );
    let font = doc.settings().drawing_font.unwrap_or(DrawingFont::Barlow);
    let mut key = DefaultHasher::new();
    (
        camera.center.x.to_bits(),
        camera.center.y.to_bits(),
        camera.scale.to_bits(),
        camera.width.to_bits(),
        camera.height.to_bits(),
        doc.revision(),
        canvas as u8,
        font as u8,
    )
        .hash(&mut key);
    // The number formats decide a dimension's text.
    format!("{format:?}").hash(&mut key);
    canvas::Canvas::new(Labels {
        doc,
        spots,
        camera: *camera,
        colors: colors(canvas, palette),
        font,
        format: *format,
        key: key.finish(),
    })
    .width(Fill)
    .height(Fill)
    .into()
}

/// The colours of the drawing's text on a canvas: the web's `--canvas-label`
/// (slate and paper; night dims it, black brightens it) and the halo of the
/// area's own colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colors {
    pub label: Color,
    pub halo: Color,
    pub fg: Color,
    pub fg_dim: Color,
    palette: Palette,
}

fn color(c: Rgba8) -> Color {
    Color::from_rgba8(c.0[0], c.0[1], c.0[2], f32::from(c.0[3]) / 255.0)
}

pub fn colors(canvas: Canvas, palette: &Palette) -> Colors {
    let label = match canvas {
        Canvas::Slate => Color::from_rgb8(0xc4, 0xcd, 0xd7),
        Canvas::Paper => Color::from_rgb8(0x36, 0x41, 0x4d),
        Canvas::Night => Color::from_rgb8(0x9b, 0xa5, 0xb0),
        Canvas::Black => Color::from_rgb8(0xe8, 0xed, 0xf2),
    };
    Colors {
        label,
        halo: color(palette.background),
        fg: color(palette.fg),
        fg_dim: color(palette.fg_dim),
        palette: *palette,
    }
}

struct Labels<'a> {
    doc: &'a Document,
    spots: Vec<LabelSpot>,
    camera: Camera,
    colors: Colors,
    font: DrawingFont,
    format: Format,
    /// What the picture depends on: while it holds, the cached picture stays.
    key: u64,
}

#[derive(Default)]
struct State {
    cache: canvas::Cache,
    key: Cell<u64>,
}

impl canvas::Program<Message> for Labels<'_> {
    type State = State;

    fn draw(
        &self,
        state: &State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        if state.key.get() != self.key {
            state.cache.clear();
            state.key.set(self.key);
        }
        vec![
            state
                .cache
                .draw(renderer, bounds.size(), |frame| self.paint(frame)),
        ]
    }
}

/// Where a text's anchor sits on it: the web's `textAlign` and `textBaseline`.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Anchor {
    /// Left end, on the baseline (text objects).
    LeftBaseline,
    /// Middle, on the baseline (dimension values).
    CenterBaseline,
    /// Left end, halfway up (labels beside and at a corner).
    LeftMiddle,
    /// Middle, halfway up (labels centred and along).
    CenterMiddle,
}

/// One text to draw: at a point on screen, turned by `angle` radians
/// clockwise on screen, `size` pixels high.
struct Piece<'t> {
    text: &'t str,
    at: Point,
    angle: f32,
    size: f32,
    font: Font,
    anchor: Anchor,
    color: Color,
}

impl Labels<'_> {
    fn screen(&self, p: Vec2) -> Point {
        let [x, y] = self.camera.world_to_screen(p);
        Point::new(x as f32, y as f32)
    }

    /// A layer's (or the object's) colour as the dimension's text takes it:
    /// the label colour for none and the theme's ink, else the colour itself.
    fn ink_of(&self, name: Option<&str>) -> Color {
        match name {
            None | Some("fg" | "fg-dim") => self.colors.label,
            Some(c) => self
                .colors
                .palette
                .resolve(c)
                .map_or(self.colors.label, color),
        }
    }

    fn paint(&self, frame: &mut Frame) {
        let size = frame.size();
        let mut room = Room::new(size.width, size.height);
        let layers = self.doc.layers();
        for spot in &self.spots {
            let slot = match spot {
                LabelSpot::Dimension { slot, .. }
                | LabelSpot::Text { slot, .. }
                | LabelSpot::Center { slot, .. }
                | LabelSpot::Corner { slot, .. }
                | LabelSpot::Beside { slot, .. }
                | LabelSpot::Along { slot, .. } => *slot,
            };
            let Some(entity) = self.doc.get(slot) else {
                continue;
            };
            let base = entity.base();
            let layer = layers.get(&base.layer_id);
            match (spot, entity) {
                (
                    LabelSpot::Dimension {
                        at,
                        angle,
                        value,
                        angular,
                        prefix,
                        ..
                    },
                    Entity::Dimension(d),
                ) => {
                    let text = match d.text.as_deref().filter(|t| !t.is_empty()) {
                        Some(own) => own.to_owned(),
                        None => dimension_text(
                            &self.format,
                            prefix,
                            if *angular { "angle" } else { "length" },
                            *value,
                        ),
                    };
                    let color = self.ink_of(
                        base.color
                            .as_deref()
                            .or(layer.map(|l| l.style.color.as_str())),
                    );
                    draw(
                        frame,
                        &Piece {
                            text: &text,
                            at: self.screen(*at),
                            angle: (-angle.to_radians()) as f32,
                            size: (d.height * self.camera.scale) as f32,
                            font: drawing_fonts::font(self.font, 500, false),
                            anchor: Anchor::CenterBaseline,
                            color,
                        },
                        self.colors.halo,
                    );
                }
                (LabelSpot::Text { at, rotation, .. }, Entity::Text(t)) => draw(
                    frame,
                    &Piece {
                        text: &t.text,
                        at: self.screen(*at),
                        angle: (-rotation.to_radians()) as f32,
                        size: (t.height * self.camera.scale) as f32,
                        font: drawing_fonts::font(self.font, 400, true),
                        anchor: Anchor::LeftBaseline,
                        color: self.colors.label,
                    },
                    self.colors.halo,
                ),
                (LabelSpot::Dimension { .. } | LabelSpot::Text { .. }, _) => {}
                (spot, entity) => {
                    let own = layer.and_then(|l| l.style.label.clone());
                    let Some(style) = own.or_else(|| default_label(entity.kind())) else {
                        continue;
                    };
                    let Some(label) = base.label.as_deref().filter(|l| !l.is_empty()) else {
                        continue;
                    };
                    self.label(frame, &mut room, spot, &style, label);
                }
            }
        }
    }

    /// An object's label where its style places it, unless one is already there.
    fn label(
        &self,
        frame: &mut Frame,
        room: &mut Room,
        spot: &LabelSpot,
        style: &LabelStyle,
        label: &str,
    ) {
        let size = (style.max_size.unwrap_or(style.size))
            .min(style.size + style.grow.unwrap_or(0.0) * self.camera.scale)
            as f32;
        let text = match &style.template {
            Some(template) => template.replace("{label}", label),
            None => label.to_owned(),
        };
        let color = match style.ink {
            Some(LabelInk::Fg) => self.colors.fg,
            Some(LabelInk::FgDim) => self.colors.fg_dim,
            Some(LabelInk::Label) | None => self.colors.label,
        };
        let font = drawing_fonts::font(self.font, style.weight.unwrap_or(500), false);
        // Room is claimed with the letters' advances added up (no kerning):
        // shaping every label a view offers would cost more than drawing the
        // ones that fit; a drawn label is measured exactly where its place needs it.
        let width = quick_width(&text, font) * size / REFERENCE;
        let piece = |at: Point, angle: f32, anchor: Anchor| Piece {
            text: &text,
            at,
            angle,
            size,
            font,
            anchor,
            color,
        };
        let halo = self.colors.halo;
        match *spot {
            LabelSpot::Center { at, .. } => {
                let s = self.screen(at);
                if room.claim(
                    s.x - width / 2.0,
                    s.y - size / 2.0,
                    s.x + width / 2.0,
                    s.y + size / 2.0,
                ) {
                    draw(frame, &piece(s, 0.0, Anchor::CenterMiddle), halo);
                }
            }
            LabelSpot::Corner { at, .. } => {
                let tl = self.screen(at);
                let s = Point::new(tl.x + 8.0, tl.y + 14.0);
                if room.claim(s.x, s.y - size / 2.0, s.x + width, s.y + size / 2.0) {
                    draw(frame, &piece(s, 0.0, Anchor::LeftMiddle), halo);
                }
            }
            LabelSpot::Beside { at, .. } => {
                let p = self.screen(at);
                let s = Point::new(p.x + 7.0, p.y - 7.0);
                if room.claim(s.x, s.y - size / 2.0, s.x + width, s.y + size / 2.0) {
                    draw(frame, &piece(s, 0.0, Anchor::LeftMiddle), halo);
                }
            }
            LabelSpot::Along { a, b, .. } => {
                // Between the two vertices, kept upright.
                let (a, c) = (self.screen(a), self.screen(b));
                let mut angle = (c.y - a.y).atan2(c.x - a.x);
                if !(-std::f32::consts::FRAC_PI_2..=std::f32::consts::FRAC_PI_2).contains(&angle) {
                    angle += std::f32::consts::PI;
                }
                let mid = Point::new((a.x + c.x) / 2.0, (a.y + c.y) / 2.0);
                let hx = (angle.cos().abs() * width + angle.sin().abs() * size) / 2.0;
                let hy = (angle.sin().abs() * width + angle.cos().abs() * size) / 2.0;
                if room.claim(mid.x - hx, mid.y - hy, mid.x + hx, mid.y + hy) {
                    draw(frame, &piece(mid, angle, Anchor::CenterMiddle), halo);
                }
            }
            LabelSpot::Dimension { .. } | LabelSpot::Text { .. } => {}
        }
    }
}

/// Text measured at this size; widths and baselines grow with the size.
const REFERENCE: f32 = 100.0;
/// The halo's width each side (the web strokes 3 px under the text).
const HALO: f32 = 1.5;
/// Text smaller than this on screen is not drawn (the store keeps dimensions to 5 … 240 px).
const SMALLEST: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Measure {
    width: f32,
    /// The baseline below the top of a line one size high.
    baseline: f32,
}

/// Text widths by face, kept between frames: a pan measures each text once
/// (the web's `widths`). Forgets everything when it grows large.
static MEASURES: Mutex<Option<HashMap<Font, HashMap<String, Measure>>>> = Mutex::new(None);
/// Measured texts kept at most (a dense view offers tens of thousands).
const MEASURES_KEPT: usize = 200_000;

/// Letters' advances by face at the reference size, kept for the program's life.
static ADVANCES: Mutex<Option<HashMap<(Font, char), f32>>> = Mutex::new(None);

/// A text's width at the reference size from its letters' advances (no
/// kerning): close enough to keep labels apart, and cheap for thousands.
fn quick_width(text: &str, font: Font) -> f32 {
    let Ok(mut all) = ADVANCES.lock() else {
        return measure(text, font).width;
    };
    let all = all.get_or_insert_with(HashMap::new);
    let mut width = 0.0;
    let mut buffer = [0u8; 4];
    for ch in text.chars() {
        width += *all
            .entry((font, ch))
            .or_insert_with(|| shaped(ch.encode_utf8(&mut buffer), font).width);
    }
    width
}

/// A text shaped at the reference size: its width and where its baseline falls.
fn shaped(text: &str, font: Font) -> Measure {
    let p = Paragraph::with_text(text::Text {
        content: text,
        bounds: Size::INFINITE,
        size: Pixels(REFERENCE),
        line_height: text::LineHeight::Relative(1.0),
        font,
        align_x: text::Alignment::Left,
        align_y: Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: text::Wrapping::None,
    });
    let run = p.buffer().layout_runs().next();
    Measure {
        width: run.as_ref().map_or(0.0, |r| r.line_w),
        baseline: run.as_ref().map_or(REFERENCE * 0.8, |r| r.line_y),
    }
}

fn measure(text: &str, font: Font) -> Measure {
    let Ok(mut all) = MEASURES.lock() else {
        return shaped(text, font);
    };
    let all = all.get_or_insert_with(HashMap::new);
    if all.values().map(HashMap::len).sum::<usize>() > MEASURES_KEPT {
        all.clear();
    }
    let known = all.entry(font).or_default();
    if let Some(m) = known.get(text) {
        return *m;
    }
    let m = shaped(text, font);
    known.insert(text.to_owned(), m);
    m
}

/// Draws a text with its halo: upright through the glyph cache, turned as outlines.
fn draw(frame: &mut Frame, piece: &Piece<'_>, halo: Color) {
    if piece.size < SMALLEST || piece.text.is_empty() {
        return;
    }
    // Measured only where the anchor needs it (a label beside a point does not).
    let k = piece.size / REFERENCE;
    let m = || measure(piece.text, piece.font);
    let dx = match piece.anchor {
        Anchor::LeftBaseline | Anchor::LeftMiddle => 0.0,
        Anchor::CenterBaseline | Anchor::CenterMiddle => -m().width * k / 2.0,
    };
    let dy = match piece.anchor {
        Anchor::LeftBaseline | Anchor::CenterBaseline => -m().baseline * k,
        Anchor::LeftMiddle | Anchor::CenterMiddle => -piece.size / 2.0,
    };
    let text = |position: Point, color: Color| canvas::Text {
        content: piece.text.to_owned(),
        position,
        max_width: f32::INFINITY,
        color,
        size: Pixels(piece.size),
        line_height: text::LineHeight::Relative(1.0),
        font: piece.font,
        align_x: text::Alignment::Left,
        align_y: Vertical::Top,
        shaping: text::Shaping::Advanced,
    };
    if piece.angle.abs() < 1e-4 {
        let top_left = Point::new(piece.at.x + dx, piece.at.y + dy);
        // The halo as eight copies around the text, then the text.
        for i in 0..8 {
            let a = i as f32 * std::f32::consts::FRAC_PI_4;
            frame.fill_text(text(
                top_left + Vector::new(a.cos() * HALO, a.sin() * HALO),
                halo,
            ));
        }
        frame.fill_text(text(top_left, piece.color));
        return;
    }
    frame.with_save(|frame| {
        frame.translate(Vector::new(piece.at.x, piece.at.y));
        frame.rotate(piece.angle);
        let mut glyphs: Vec<Path> = Vec::new();
        text(Point::new(dx, dy), piece.color).draw_with(|path, _| glyphs.push(path));
        let stroke = Stroke {
            width: HALO * 2.0,
            line_join: LineJoin::Round,
            ..Stroke::default()
        }
        .with_color(halo);
        for glyph in &glyphs {
            frame.stroke(glyph, stroke);
        }
        for glyph in &glyphs {
            frame.fill(glyph, piece.color);
        }
    });
}

/// Where labels already are on screen, in 8 px cells: a label that would
/// cover one is not drawn (the web's `LabelRoom`). Coarse on purpose: a
/// cell or two of slack is spacing between labels.
struct Room {
    cols: usize,
    rows: usize,
    taken: Vec<bool>,
}

impl Room {
    const CELL: f32 = 8.0;

    fn new(width: f32, height: f32) -> Self {
        let cols = ((width / Self::CELL).ceil() as usize).max(1);
        let rows = ((height / Self::CELL).ceil() as usize).max(1);
        Self {
            cols,
            rows,
            taken: vec![false; cols * rows],
        }
    }

    /// Takes the box if nothing is there yet; false (and nothing taken) when a label is in the way.
    fn claim(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) -> bool {
        let cell = |v: f32| (v / Self::CELL).floor();
        let c0 = cell(x0).max(0.0);
        let c1 = cell(x1).min(self.cols as f32 - 1.0);
        let r0 = cell(y0).max(0.0);
        let r1 = cell(y1).min(self.rows as f32 - 1.0);
        // Wholly off screen: nothing to keep apart from.
        if c0 > c1 || r0 > r1 {
            return true;
        }
        let (c0, c1, r0, r1) = (c0 as usize, c1 as usize, r0 as usize, r1 as usize);
        for r in r0..=r1 {
            if self.taken[r * self.cols + c0..=r * self.cols + c1]
                .iter()
                .any(|t| *t)
            {
                return false;
            }
        }
        for r in r0..=r1 {
            self.taken[r * self.cols + c0..=r * self.cols + c1].fill(true);
        }
        true
    }
}

/// An object's slot in the spots (for tests).
#[cfg(test)]
fn slots(spots: &[LabelSpot]) -> Vec<kentos_domain::Slot> {
    spots
        .iter()
        .map(|s| match s {
            LabelSpot::Dimension { slot, .. }
            | LabelSpot::Text { slot, .. }
            | LabelSpot::Center { slot, .. }
            | LabelSpot::Corner { slot, .. }
            | LabelSpot::Beside { slot, .. }
            | LabelSpot::Along { slot, .. } => *slot,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_label_in_the_way_is_left_out_and_off_screen_ones_always_fit() {
        let mut room = Room::new(100.0, 100.0);
        assert!(room.claim(10.0, 10.0, 40.0, 20.0));
        assert!(!room.claim(30.0, 12.0, 60.0, 22.0), "overlaps the first");
        assert!(room.claim(50.0, 30.0, 90.0, 40.0));
        assert!(room.claim(-80.0, -80.0, -10.0, -10.0), "wholly off screen");
        assert!(room.claim(200.0, 10.0, 260.0, 20.0));
    }

    /// The desktop's default label styles are the web's `DEFAULT_LABELS`
    /// (apps/web/src/viewport/storeRecords.ts), read from its source.
    #[test]
    fn the_default_labels_are_the_webs() {
        let source = include_str!("../../web/src/viewport/storeRecords.ts");
        let start = source.find("DEFAULT_LABELS").expect("the web's defaults");
        let body = &source[start..];
        let body = &body[body.find('{').expect("an object") + 1..body.find("};").expect("its end")];
        for line in body.lines().map(str::trim).filter(|l| !l.is_empty()) {
            let (kind, style) = line.split_once(':').expect("kind: style");
            // The literal as JSON: quoted keys, double quotes.
            let mut json = String::new();
            for part in style
                .trim()
                .trim_end_matches(',')
                .replace('\'', "\"")
                .split(['{', ',', '}'])
            {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }
                let (k, v) = part.split_once(':').expect("key: value");
                json.push_str(&format!("\"{}\":{},", k.trim(), v.trim()));
            }
            let json = format!("{{{}}}", json.trim_end_matches(','));
            let web: LabelStyle = serde_json::from_str(&json).expect("a label style");
            assert_eq!(default_label(kind.trim()), Some(web), "{kind}");
        }
    }

    #[test]
    fn the_sample_drawing_shows_its_texts_labels_and_dimensions() {
        let app = crate::files_testing::app_with_drawing();
        let doc = &app.document.as_ref().expect("open").model;
        let mut spatial = Spatial::new();
        spatial.reload(doc);
        let extent = spatial.extent().expect("objects");
        let spots = spatial.labels(
            Vec2::new(extent.min_x, extent.min_y),
            Vec2::new(extent.max_x, extent.max_y),
            4.0,
        );
        assert!(!spots.is_empty());
        for slot in slots(&spots) {
            assert!(doc.get(slot).is_some());
        }
    }
}

/// Pictures of the drawing's text for the owner: the sample drawing with its
/// “Çizim” layer shown (a text, a dimension, a point's name, the parcel's
/// number), dark and light, at 1440×900 and 1100×650, and closer in;
/// `.run/shots/yazi-*`:
///
/// ```text
/// cargo test -p kentos-desktop labels::screens -- --ignored --nocapture
/// ```
#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use kentos_ui::snapshot::Snapshot;

    use crate::app::App;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height, zoom) in [
            (1440.0, 900.0, 1.0),
            (1100.0, 650.0, 1.0),
            (1440.0, 900.0, 3.0),
        ] {
            let mut app = crate::files_testing::app_with_drawing();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let _ = app.update(Message::LayerVisible("cizim".into()));
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let _ = app.update(Message::Run("view.zoomExtents"));
            if zoom > 1.0 {
                let camera = &mut app.viewport.camera;
                let (cx, cy) = (camera.width / 2.0, camera.height / 2.0);
                camera.zoom_at(zoom, cx, cy);
            }
            snapshot.settle(&mut app, App::view, &mut update);
            let name = if zoom > 1.0 {
                format!("yazi-yakin{suffix}")
            } else {
                format!("yazi-{width}x{height}{suffix}")
            };
            let file = out.join(format!("{name}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

/// Frame times with thousands of labels on screen, in release:
///
/// ```text
/// cargo test --release -p kentos-desktop labels::perf -- --ignored --nocapture
/// ```
///
/// 50 176 named points 1 m apart, viewed at 4 px/m (labels beside points
/// show from 2 px/m): the store offers the labels in view, the room thins
/// them. Each frame pans a little, so the labels are laid out again.
#[cfg(test)]
#[test]
#[ignore = "a measurement, run by hand in release"]
fn perf() {
    use std::time::Instant;

    use kentos_contracts::{DocumentSnapshotV1, EntityBase, PointEntity};
    use kentos_ui::snapshot::Snapshot;

    use crate::app::App;

    let mut snapshot: DocumentSnapshotV1 =
        serde_json::from_str(include_str!("../../../fixtures/document/v1/sample.json"))
            .expect("the sample");
    let first = snapshot
        .entities
        .iter()
        .map(|e| e.base().id)
        .max()
        .unwrap_or(0);
    let side = 224;
    for i in 0..side * side {
        snapshot.entities.push(Entity::Point(PointEntity {
            base: EntityBase {
                id: first + i as u32 + 1,
                layer_id: "parsel".into(),
                color: None,
                attrs: Default::default(),
                label: Some(format!("N{i}")),
                symbol: None,
            },
            p: kentos_contracts::Vec2 {
                x: 486_400.0 + (i % side) as f64,
                y: 4_420_000.0 + (i / side) as f64,
            },
            z: None,
        }));
    }
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let (mut app, _) = App::boot(None);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let mut shot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    shot.settle(&mut app, App::view, &mut update);
    app.viewport.camera.scale = 4.0;
    app.viewport.camera.center = Vec2::new(486_512.0, 4_420_112.0);
    let _ = shot.render(app.view(), &app.theme());
    let spots = {
        let v = app.viewport.camera.visible_bounds();
        app.spatial
            .labels(
                Vec2::new(v.min_x, v.min_y),
                Vec2::new(v.max_x, v.max_y),
                4.0,
            )
            .len()
    };
    let frames = 10;
    let started = Instant::now();
    for _ in 0..frames {
        app.viewport.camera.pan_by(3.0, 2.0);
        let _ = shot.render(app.view(), &app.theme());
    }
    let per_frame = started.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    println!("{spots} labels offered in view; {per_frame:.1} ms a frame (render included)");
    // The parts: the store's query, and laying the labels out (no drawing).
    let camera = app.viewport.camera;
    let v = camera.visible_bounds();
    let started = Instant::now();
    for _ in 0..frames {
        let _ = app.spatial.labels(
            Vec2::new(v.min_x, v.min_y),
            Vec2::new(v.max_x, v.max_y),
            4.0,
        );
    }
    let query = started.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    let doc = &app.document.as_ref().expect("open").model;
    let labels = Labels {
        doc,
        spots: app.spatial.labels(
            Vec2::new(v.min_x, v.min_y),
            Vec2::new(v.max_x, v.max_y),
            4.0,
        ),
        camera,
        colors: colors(Canvas::Slate, &crate::viewport::palette(Canvas::Slate)),
        font: DrawingFont::Barlow,
        format: Format::of(doc.settings()),
        key: 0,
    };
    let started = Instant::now();
    for _ in 0..frames {
        let mut frame = Frame::new(shot.renderer(), Size::new(1440.0, 900.0));
        labels.paint(&mut frame);
    }
    let paint = started.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    println!("query {query:.1} ms, laying out {paint:.1} ms a frame");
}
