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

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;
use std::sync::Mutex;

use iced::advanced::graphics::text::Paragraph;
use iced::advanced::text::{self, Paragraph as _};
use iced::alignment::Vertical;
use iced::widget::canvas::{self, Frame, LineJoin, Path, Stroke};
use iced::{
    Color, Element, Fill, Font, Pixels, Point, Rectangle, Renderer, Size, Theme, Vector, mouse,
};
use kentos_contracts::{DrawingFont, Entity, LabelInk, LabelStyle};
use kentos_domain::{Document, Slot};
use kentos_interaction::spatial::default_label;
use kentos_interaction::{Format, LabelSpot, Spatial, Vec2};
use kentos_render_wgpu::Camera;
use kentos_render_wgpu::color::{Palette, Rgba8};

use crate::app::Message;
use crate::drawing_fonts;
use crate::exchange::dimension_text;
use crate::viewport::Canvas;

/// The labels a view shows, as last asked of the store, and what they were
/// asked for: the drawing (`drawing`, its changes), the view and the text
/// edited in place. The store walks every object in view to answer (all of
/// them in an overview), so a frame whose view and drawing stay (a pointer
/// move, a button's hover) takes them from here.
#[derive(Default)]
pub struct Spots(RefCell<Option<(u64, Rc<Vec<LabelSpot>>)>>);

impl Spots {
    fn get(&self, key: u64, ask: impl FnOnce() -> Vec<LabelSpot>) -> Rc<Vec<LabelSpot>> {
        let mut kept = self.0.borrow_mut();
        match kept.as_ref() {
            Some((known, spots)) if *known == key => spots.clone(),
            _ => {
                let spots = Rc::new(ask());
                *kept = Some((key, spots.clone()));
                spots
            }
        }
    }
}

/// The labels' layer over the drawing area; `drawing` tells one open drawing from another.
#[allow(clippy::too_many_arguments)]
pub fn layer<'a>(
    doc: &'a Document,
    drawing: u64,
    spatial: &Spatial,
    kept: &Spots,
    camera: &Camera,
    canvas: Canvas,
    palette: &Palette,
    format: &Format,
    hidden: Option<Slot>,
) -> Element<'a, Message> {
    let mut key = DefaultHasher::new();
    (
        camera.center.x.to_bits(),
        camera.center.y.to_bits(),
        camera.scale.to_bits(),
        camera.width.to_bits(),
        camera.height.to_bits(),
        drawing,
        // Every change, other editors' too (a revision counts only this user's).
        doc.generation(),
        hidden.map(|s| s.0),
    )
        .hash(&mut key);
    let spots = kept.get(key.finish(), || {
        let view = camera.visible_bounds();
        let mut spots = spatial.labels(
            Vec2::new(view.min_x, view.min_y),
            Vec2::new(view.max_x, view.max_y),
            camera.scale,
        );
        // A text or a dimension's value being edited in place (the web's `setEditing`).
        if let Some(hidden) = hidden {
            spots.retain(|spot| match spot {
                LabelSpot::Text { slot, .. } | LabelSpot::Dimension { slot, .. } => *slot != hidden,
                _ => true,
            });
        }
        spots
    });
    let font = doc.settings().drawing_font.unwrap_or(DrawingFont::Barlow);
    (canvas as u8, font as u8).hash(&mut key);
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
        fence: None,
        current: Cell::new(None),
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

/// The paper's colours for the drawing's text on a sheet's map, on the screen and in its PDF:
/// the core's (`kentos_sheet::display::paper`, the web's `paperPalette`): #111111 letters on
/// white, whatever the theme; a dimension in a colour of its own keeps it.
pub fn paper_colors() -> Colors {
    use kentos_sheet::display::paper;
    let rgb = |hex: &str| {
        let v = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0);
        Rgba8::rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
    };
    let palette = Palette {
        background: rgb(paper::PAPER),
        fg: rgb(paper::FG),
        fg_dim: rgb(paper::FG_DIM),
        ink: rgb(paper::INK),
    };
    Colors {
        label: color(rgb(paper::LABEL)),
        halo: color(rgb(paper::LABEL_HALO)),
        fg: color(palette.fg),
        fg_dim: color(palette.fg_dim),
        palette,
    }
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
    spots: Rc<Vec<LabelSpot>>,
    camera: Camera,
    colors: Colors,
    font: DrawingFont,
    format: Format,
    /// What the picture depends on: while it holds, the cached picture stays.
    key: u64,
    /// A sheet's map frame: only what is anchored inside it is drawn (sheets.rs).
    fence: Option<Fence>,
    /// The object whose label is being drawn (a sheet PDF puts it in its layer).
    current: Cell<Option<Slot>>,
}

/// A sheet's map frame as the labels see it: the camera's picture turned by
/// the map's own turn about the frame's middle; a text is drawn when its
/// anchor is inside the frame, and cut at the frame's edge (design §9a, the
/// PDF's rule; a turned frame's text is cut to its upright box on the screen).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fence {
    /// The camera's middle, in its own pixels.
    pub from: Point,
    /// The frame's size; its middle is where the camera's middle goes.
    pub size: Size,
    /// The map's turn, radians clockwise on the screen.
    pub turn: f32,
    /// The camera's pixels to the frame's (the labels laid out finer than the screen shows them).
    pub scale: f32,
}

impl Fence {
    /// Whether a point of the camera's picture falls inside the frame.
    fn holds(&self, p: Point) -> bool {
        let (s, c) = self.turn.sin_cos();
        let (x, y) = (
            (p.x - self.from.x) * self.scale,
            (p.y - self.from.y) * self.scale,
        );
        let (u, v) = (
            x * c - y * s + self.size.width / 2.0,
            x * s + y * c + self.size.height / 2.0,
        );
        u >= -0.5 && v >= -0.5 && u <= self.size.width + 0.5 && v <= self.size.height + 0.5
    }
}

/// The drawing's text in a sheet's map frame (sheets.rs; the web draws the
/// drawing area's own labels into a map's picture): the same pieces the
/// drawing area draws, with `camera` on the map's view (north up), the
/// frame already turned by the map's turn about its middle, in the paper's
/// colours; a text anchored outside the frame is left out. No north arrow or
/// scale bar.
#[allow(clippy::too_many_arguments)]
pub fn paint_in_map(
    frame: &mut Frame,
    doc: &Document,
    spots: Vec<LabelSpot>,
    camera: Camera,
    colors: Colors,
    font: DrawingFont,
    format: Format,
    fence: Fence,
) {
    Labels {
        doc,
        spots: Rc::new(spots),
        camera,
        colors,
        font,
        format,
        key: 0,
        fence: Some(fence),
        current: Cell::new(None),
    }
    .paint(frame);
}

/// A label of a sheet's map as the PDF writes it (sheet_pdf.rs; the web's mapLabels.ts
/// `VecText`): its text anchored on the ground as the drawing area anchors it (the middle of a
/// centred label, a text's baseline start, a dimension value's baseline middle), its turn
/// (degrees, counter-clockwise on the paper), its size in paper mm (the screen's pixels at 100 %
/// are the paper's CSS pixels), its face, its colour and its halo's; the paper-coloured box
/// under a masked text (docs/adr/0145), on the ground.
#[derive(Clone, Debug, PartialEq)]
pub struct MapLabel {
    /// The layer of the object it is written for.
    pub layer: String,
    pub text: String,
    pub at: kentos_render_wgpu::Vec2,
    pub anchor: kentos_sheet::pdf::TextAnchor,
    pub rotation: f64,
    pub size: f64,
    pub weight: u16,
    pub italic: bool,
    pub color: Color,
    pub halo: Color,
    pub mask: Option<Vec<[f64; 2]>>,
}

/// The drawing's text in a sheet's map as [`paint_in_map`] lays it out (the same pieces, the
/// same room between them, the same fence), as a list rather than a picture.
#[allow(clippy::too_many_arguments)]
pub fn texts_in_map(
    doc: &Document,
    spots: Vec<LabelSpot>,
    camera: Camera,
    colors: Colors,
    font: DrawingFont,
    format: Format,
    fence: Fence,
) -> Vec<MapLabel> {
    let mut list = Collect {
        doc,
        camera,
        labels: Vec::new(),
    };
    Labels {
        doc,
        spots: Rc::new(spots),
        camera,
        colors,
        font,
        format,
        key: 0,
        fence: Some(fence),
        current: Cell::new(None),
    }
    .paint(&mut list);
    list.labels
}

/// Where label pieces go: a canvas frame (drawn), or a list (a sheet PDF's map).
trait Ink {
    /// The picture's size (a frame's own; a fenced map's is the camera's).
    fn size(&self) -> Size;
    fn piece(&mut self, piece: &Piece<'_>, halo: Color, of: Option<Slot>);
    /// Grid north and the scale bar, or a CAD project's coordinate axes.
    fn marks(&mut self, camera: &Camera, colors: &Colors, axes: kentos_interaction::Axes);
}

impl Ink for Frame {
    fn size(&self) -> Size {
        Frame::size(self)
    }

    fn piece(&mut self, piece: &Piece<'_>, halo: Color, _of: Option<Slot>) {
        draw(self, piece, halo);
    }

    fn marks(&mut self, camera: &Camera, colors: &Colors, axes: kentos_interaction::Axes) {
        crate::map_marks::paint(self, camera, colors, axes);
    }
}

/// The pieces as the PDF takes them: each anchored on the ground as the drawing area anchors it.
struct Collect<'a> {
    doc: &'a Document,
    camera: Camera,
    labels: Vec<MapLabel>,
}

impl Ink for Collect<'_> {
    fn size(&self) -> Size {
        Size::ZERO
    }

    fn piece(&mut self, piece: &Piece<'_>, halo: Color, of: Option<Slot>) {
        if piece.size < SMALLEST || piece.text.is_empty() {
            return;
        }
        use kentos_sheet::pdf::TextAnchor;
        let layer = of
            .and_then(|slot| self.doc.get(slot))
            .map(|e| e.base().layer_id.clone())
            .unwrap_or_default();
        let at = self
            .camera
            .screen_to_world(f64::from(piece.at.x), f64::from(piece.at.y));
        // Clockwise on the screen (y down) is counter-clockwise on the paper.
        let rotation = -f64::from(piece.angle).to_degrees();
        let px = 1.0 / self.camera.scale.max(f64::MIN_POSITIVE);
        // A masked text's box (the web's `mask`): from below the baseline to above the letters,
        // a tenth of the height round it (a dimension's value along its line, no margin up and down).
        let mask = (piece.mask > 0.0).then(|| {
            let (w, h) = (f64::from(piece.mask) * px, f64::from(piece.size) * px);
            let along = matches!(piece.anchor, Anchor::CenterBaseline);
            let x0 = if along { -w / 2.0 } else { 0.0 };
            let m = h * 0.1;
            let v = if along { 0.0 } else { m };
            let (s, c) = rotation.to_radians().sin_cos();
            [
                [x0 - m, -0.23 * h - v],
                [x0 + w + m, -0.23 * h - v],
                [x0 + w + m, 1.15 * h + v],
                [x0 - m, 1.15 * h + v],
            ]
            .iter()
            .map(|[u, t]| [at.x + u * c - t * s, at.y + u * s + t * c])
            .collect()
        });
        self.labels.push(MapLabel {
            layer,
            text: piece.text.to_owned(),
            at,
            anchor: match piece.anchor {
                Anchor::LeftBaseline => TextAnchor::LeftBaseline,
                Anchor::CenterBaseline => TextAnchor::CenterBaseline,
                Anchor::LeftMiddle => TextAnchor::LeftMiddle,
                Anchor::CenterMiddle => TextAnchor::CenterMiddle,
            },
            rotation,
            size: f64::from(piece.size) * crate::sheet_pdf::PX_MM,
            weight: match piece.font.weight {
                iced::font::Weight::Thin
                | iced::font::Weight::ExtraLight
                | iced::font::Weight::Light
                | iced::font::Weight::Normal => 400,
                iced::font::Weight::Medium => 500,
                iced::font::Weight::Semibold => 600,
                _ => 700,
            },
            italic: piece.font.style == iced::font::Style::Italic,
            color: piece.color,
            halo,
            mask,
        });
    }

    fn marks(&mut self, _camera: &Camera, _colors: &Colors, _axes: kentos_interaction::Axes) {}
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
/// clockwise on screen, `size` pixels high, its letters `width_factor`
/// wide; `mask` pixels long, the box filled with the drawing area's colour
/// under it first (a text object's, docs/adr/0145; a dimension value's,
/// docs/adr/0147), 0 for none.
struct Piece<'t> {
    text: &'t str,
    at: Point,
    angle: f32,
    size: f32,
    font: Font,
    anchor: Anchor,
    color: Color,
    width_factor: f32,
    mask: f32,
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

    /// Draws a piece, unless a map frame's text is anchored outside the frame (design §9a: one
    /// anchored inside is drawn and cut at the frame's edge).
    fn draw(&self, frame: &mut impl Ink, piece: &Piece<'_>, halo: Color) {
        if let Some(fence) = &self.fence
            && !fence.holds(piece.at)
        {
            return;
        }
        frame.piece(piece, halo, self.current.get());
    }

    fn paint(&self, frame: &mut impl Ink) {
        // A map frame's labels keep apart on the camera's own picture.
        let size = match self.fence {
            Some(_) => Size::new(self.camera.width as f32, self.camera.height as f32),
            None => frame.size(),
        };
        let mut room = Room::new(size.width, size.height);
        let layers = self.doc.layers();
        for spot in self.spots.iter() {
            let slot = match spot {
                LabelSpot::Dimension { slot, .. }
                | LabelSpot::Text { slot, .. }
                | LabelSpot::Center { slot, .. }
                | LabelSpot::Corner { slot, .. }
                | LabelSpot::Beside { slot, .. }
                | LabelSpot::Along { slot, .. }
                | LabelSpot::PieceText { slot, .. }
                | LabelSpot::PieceDimension { slot, .. } => *slot,
            };
            self.current.set(Some(slot));
            // A label whose place is taken is not drawn: known before its
            // object, style and text are looked at (an overview offers
            // hundreds of thousands, a few hundred fit).
            if self.anchor(spot).is_some_and(|at| room.taken_at(at)) {
                continue;
            }
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
                        unit,
                        prefix,
                        mask,
                        ..
                    },
                    Entity::Dimension(d),
                ) => {
                    let text = match d.text.as_deref().filter(|t| !t.is_empty()) {
                        Some(own) => own.to_owned(),
                        None => dimension_text(&self.format, prefix, unit, *value),
                    };
                    let color = self.ink_of(
                        base.color
                            .as_deref()
                            .or(layer.map(|l| l.style.color.as_str())),
                    );
                    let size = (d.height * self.camera.scale) as f32;
                    let font = drawing_fonts::font(self.font, 500, false);
                    self.draw(
                        frame,
                        &Piece {
                            text: &text,
                            at: self.screen(*at),
                            angle: (-angle.to_radians()) as f32,
                            size,
                            font,
                            anchor: Anchor::CenterBaseline,
                            color,
                            width_factor: 1.0,
                            mask: value_mask(*mask, &text, font, size),
                        },
                        self.colors.halo,
                    );
                }
                (
                    LabelSpot::Text {
                        at,
                        rotation,
                        width_factor,
                        mask,
                        ..
                    },
                    Entity::Text(t),
                ) => self.draw(
                    frame,
                    &Piece {
                        text: &t.text,
                        at: self.screen(*at),
                        angle: (-rotation.to_radians()) as f32,
                        size: (t.height * self.camera.scale) as f32,
                        font: drawing_fonts::font(self.font, 400, true),
                        anchor: Anchor::LeftBaseline,
                        color: self.colors.label,
                        width_factor: *width_factor as f32,
                        mask: (mask * self.camera.scale) as f32,
                    },
                    self.colors.halo,
                ),
                // A leader's note, as a text (docs/adr/0146 §5).
                (
                    LabelSpot::Text {
                        at, rotation, mask, ..
                    },
                    Entity::Leader(l),
                ) => {
                    let Some(note) = l.text.as_deref() else {
                        continue;
                    };
                    self.draw(
                        frame,
                        &Piece {
                            text: note,
                            at: self.screen(*at),
                            angle: (-rotation.to_radians()) as f32,
                            size: (l.height * self.camera.scale) as f32,
                            font: drawing_fonts::font(self.font, 400, true),
                            anchor: Anchor::LeftBaseline,
                            color: self.colors.label,
                            width_factor: 1.0,
                            mask: (mask * self.camera.scale) as f32,
                        },
                        self.colors.halo,
                    );
                }
                // A block's texts and dimension values (docs/adr/0144), as its own objects draw theirs;
                // an attribute's text the insert's value, else its default (§7).
                (
                    LabelSpot::PieceText {
                        at,
                        rotation,
                        height,
                        text,
                        attribute,
                        width_factor,
                        mask,
                        ..
                    },
                    _,
                ) => {
                    let shown = kentos_interaction::spatial::shown_text(
                        text,
                        attribute.as_deref(),
                        &base.attrs,
                    );
                    if shown.is_empty() {
                        continue;
                    }
                    self.draw(
                        frame,
                        &Piece {
                            text: shown,
                            at: self.screen(*at),
                            angle: (-rotation.to_radians()) as f32,
                            size: (height * self.camera.scale) as f32,
                            font: drawing_fonts::font(self.font, 400, true),
                            anchor: Anchor::LeftBaseline,
                            color: self.colors.label,
                            width_factor: *width_factor as f32,
                            mask: (mask * self.camera.scale) as f32,
                        },
                        self.colors.halo,
                    );
                }
                (
                    LabelSpot::PieceDimension {
                        at,
                        angle,
                        value,
                        height,
                        text,
                        unit,
                        prefix,
                        mask,
                        ..
                    },
                    _,
                ) => {
                    let text = match text {
                        Some(own) => own.clone(),
                        None => dimension_text(&self.format, prefix, unit, *value),
                    };
                    let color = self.ink_of(
                        base.color
                            .as_deref()
                            .or(layer.map(|l| l.style.color.as_str())),
                    );
                    let size = (height * self.camera.scale) as f32;
                    let font = drawing_fonts::font(self.font, 500, false);
                    self.draw(
                        frame,
                        &Piece {
                            text: &text,
                            at: self.screen(*at),
                            angle: (-angle.to_radians()) as f32,
                            size,
                            font,
                            anchor: Anchor::CenterBaseline,
                            color,
                            width_factor: 1.0,
                            mask: value_mask(*mask, &text, font, size),
                        },
                        self.colors.halo,
                    );
                }
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
        // Grid north and the scale bar over the text, or a CAD project's coordinate axes,
        // as the web's overlay (map_marks.rs); a sheet's map has its own.
        if self.fence.is_none() {
            frame.marks(&self.camera, &self.colors, self.format.axes);
        }
    }

    /// Where an object's label is anchored on screen, inside the box it
    /// claims whatever its text and size; none for texts and dimensions.
    fn anchor(&self, spot: &LabelSpot) -> Option<Point> {
        match *spot {
            LabelSpot::Center { at, .. } => Some(self.screen(at)),
            LabelSpot::Corner { at, .. } => {
                let tl = self.screen(at);
                Some(Point::new(tl.x + 8.0, tl.y + 14.0))
            }
            LabelSpot::Beside { at, .. } => {
                let p = self.screen(at);
                Some(Point::new(p.x + 7.0, p.y - 7.0))
            }
            LabelSpot::Along { a, b, .. } => {
                let (a, c) = (self.screen(a), self.screen(b));
                Some(Point::new((a.x + c.x) / 2.0, (a.y + c.y) / 2.0))
            }
            LabelSpot::Dimension { .. }
            | LabelSpot::Text { .. }
            | LabelSpot::PieceText { .. }
            | LabelSpot::PieceDimension { .. } => None,
        }
    }

    /// An object's label where its style places it, unless one is already there.
    fn label(
        &self,
        frame: &mut impl Ink,
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
            width_factor: 1.0,
            mask: 0.0,
        };
        let halo = self.colors.halo;
        let Some(s) = self.anchor(spot) else {
            return;
        };
        match *spot {
            LabelSpot::Center { .. } => {
                if room.claim(
                    s.x - width / 2.0,
                    s.y - size / 2.0,
                    s.x + width / 2.0,
                    s.y + size / 2.0,
                ) {
                    self.draw(frame, &piece(s, 0.0, Anchor::CenterMiddle), halo);
                }
            }
            LabelSpot::Corner { .. } => {
                if room.claim(s.x, s.y - size / 2.0, s.x + width, s.y + size / 2.0) {
                    self.draw(frame, &piece(s, 0.0, Anchor::LeftMiddle), halo);
                }
            }
            LabelSpot::Beside { .. } => {
                if room.claim(s.x, s.y - size / 2.0, s.x + width, s.y + size / 2.0) {
                    self.draw(frame, &piece(s, 0.0, Anchor::LeftMiddle), halo);
                }
            }
            LabelSpot::Along { a, b, .. } => {
                // Between the two vertices, kept upright.
                let (a, c) = (self.screen(a), self.screen(b));
                let mut angle = (c.y - a.y).atan2(c.x - a.x);
                if !(-std::f32::consts::FRAC_PI_2..=std::f32::consts::FRAC_PI_2).contains(&angle) {
                    angle += std::f32::consts::PI;
                }
                let mid = s;
                let hx = (angle.cos().abs() * width + angle.sin().abs() * size) / 2.0;
                let hy = (angle.sin().abs() * width + angle.cos().abs() * size) / 2.0;
                if room.claim(mid.x - hx, mid.y - hy, mid.x + hx, mid.y + hy) {
                    self.draw(frame, &piece(mid, angle, Anchor::CenterMiddle), halo);
                }
            }
            LabelSpot::Dimension { .. }
            | LabelSpot::Text { .. }
            | LabelSpot::PieceText { .. }
            | LabelSpot::PieceDimension { .. } => {}
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
/// A dimension value's mask (docs/adr/0147): its measured width on screen when it has one, else 0.
fn value_mask(mask: bool, text: &str, font: Font, size: f32) -> f32 {
    if mask {
        measure(text, font).width * size / REFERENCE
    } else {
        0.0
    }
}

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
    // Upright, plain text through the glyph cache; a width factor or a mask needs the frame's transform.
    if piece.angle.abs() < 1e-4 && piece.width_factor == 1.0 && piece.mask <= 0.0 {
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
        // The mask (docs/adr/0145): the text's box a tenth of its height wider all round,
        // 1.15 of it over the baseline and 0.23 under (`TextPlace::mask`), in the area's colour;
        // from where the text starts. A dimension's value (centred) is wider on its sides only,
        // leaving its dimension line and an arc length's symbol in view (docs/adr/0147).
        if piece.mask > 0.0 {
            let m = piece.size * 0.1;
            let v = if piece.anchor == Anchor::CenterBaseline {
                0.0
            } else {
                m
            };
            frame.fill(
                &Path::rectangle(
                    Point::new(dx - m, -piece.size * 1.15 - v),
                    Size::new(piece.mask + 2.0 * m, piece.size * 1.38 + 2.0 * v),
                ),
                halo,
            );
        }
        if piece.width_factor != 1.0 {
            frame.scale_nonuniform(Vector::new(piece.width_factor, 1.0));
        }
        let glyphs = outlines(piece, dx, dy, || {
            let mut glyphs: Vec<Path> = Vec::new();
            text(Point::new(dx, dy), piece.color).draw_with(|path, _| glyphs.push(path));
            glyphs
        });
        let stroke = Stroke {
            width: HALO * 2.0,
            line_join: LineJoin::Round,
            ..Stroke::default()
        }
        .with_color(halo);
        for glyph in glyphs.iter() {
            frame.stroke(glyph, stroke);
        }
        for glyph in glyphs.iter() {
            frame.fill(glyph, piece.color);
        }
    });
}

/// Turned texts' glyph outlines kept at most (a view at 1:1000 over a
/// parcel sheet turns about 1 300 edge lengths).
const OUTLINES_KEPT: usize = 8_192;

/// What a turned text's outlines depend on: the text, its typeface, its
/// size and its offset from the anchor. The turn and the place are the
/// frame's transform, so a pan finds every text of the last frame here.
type OutlineKey = (String, Font, u32, u32, u32);

thread_local! {
    /// Shaping a text and taking its glyphs' outlines is most of a turned
    /// text's cost; the drawing's canvas paints on this thread only.
    static OUTLINES: RefCell<HashMap<OutlineKey, Rc<Vec<Path>>>> = RefCell::new(HashMap::new());
}

/// A turned text's glyph outlines in its own frame, from the kept ones or
/// made by `make`; the kept ones are let go all at once when too many.
fn outlines(
    piece: &Piece<'_>,
    dx: f32,
    dy: f32,
    make: impl FnOnce() -> Vec<Path>,
) -> Rc<Vec<Path>> {
    let key = (
        piece.text.to_owned(),
        piece.font,
        piece.size.to_bits(),
        dx.to_bits(),
        dy.to_bits(),
    );
    OUTLINES.with(|kept| {
        if let Some(glyphs) = kept.borrow().get(&key) {
            return glyphs.clone();
        }
        let glyphs = Rc::new(make());
        let mut kept = kept.borrow_mut();
        if kept.len() >= OUTLINES_KEPT {
            kept.clear();
        }
        kept.insert(key, glyphs.clone());
        glyphs
    })
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

    /// Whether the cell under a point on screen is taken: a label whose box
    /// holds the point cannot claim it then (`claim` refuses it).
    fn taken_at(&self, p: Point) -> bool {
        if !(p.x >= 0.0 && p.y >= 0.0) {
            return false;
        }
        let (c, r) = ((p.x / Self::CELL) as usize, (p.y / Self::CELL) as usize);
        c < self.cols && r < self.rows && self.taken[r * self.cols + c]
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

/// The object a spot writes for.
pub fn slot_of(spot: &LabelSpot) -> Slot {
    match spot {
        LabelSpot::Dimension { slot, .. }
        | LabelSpot::Text { slot, .. }
        | LabelSpot::Center { slot, .. }
        | LabelSpot::Corner { slot, .. }
        | LabelSpot::Beside { slot, .. }
        | LabelSpot::Along { slot, .. }
        | LabelSpot::PieceText { slot, .. }
        | LabelSpot::PieceDimension { slot, .. } => *slot,
    }
}

/// An object's slot in the spots (for tests).
#[cfg(test)]
fn slots(spots: &[LabelSpot]) -> Vec<kentos_domain::Slot> {
    spots.iter().map(slot_of).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A label whose box holds a taken cell's point cannot claim it: what
    /// lets the paint pass over it before looking at its object.
    #[test]
    fn a_taken_anchor_means_the_claim_fails() {
        let mut room = Room::new(200.0, 120.0);
        assert!(room.claim(40.0, 30.0, 90.0, 45.0));
        for (x, y) in [(40.0, 30.0), (60.0, 40.0), (95.0, 47.9)] {
            assert!(room.taken_at(Point::new(x, y)), "{x}, {y}");
            // Every box around the point, wide or narrow.
            for (w, h) in [(0.0, 0.0), (30.0, 10.0), (4.0, 60.0)] {
                assert!(!room.claim(x - w / 2.0, y - h / 2.0, x + w / 2.0, y + h / 2.0));
            }
        }
        // Off the screen, or not a number: never taken (the claim decides then).
        for (x, y) in [(-1.0, 40.0), (60.0, 500.0), (f32::NAN, 40.0)] {
            assert!(!room.taken_at(Point::new(x, y)));
        }
        assert!(!room.taken_at(Point::new(150.0, 100.0)));
    }

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

/// Text extras (docs/adr/0145) as the drawing shows them: the twelve
/// alignments at their marked points, a turned centred text, width factors
/// 0.6, 1 and 1.5, and a masked text next to one without over a hatch and a
/// line (fixtures/interaction/v1/text-extras.kcad, the web's
/// `shots.mjs texts`), dark and light, at 1440×900 and 1100×650, and
/// closer in; `.run/shots/yazi-ekleri-*`:
///
/// ```text
/// cargo test -p kentos-desktop labels::text_extras_screens -- --ignored --nocapture
/// ```
#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn text_extras_screens() {
    use kentos_ui::snapshot::Snapshot;

    use crate::app::App;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let drawing = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/interaction/v1/text-extras.kcad"
    );
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height, zoom, name) in [
            (1440.0, 900.0, 1.0, ""),
            (1100.0, 650.0, 1.0, ""),
            (1440.0, 900.0, 2.5, "zemin"),
            (1440.0, 900.0, 5.0, "donuk"),
        ] {
            let (mut app, _) = App::boot(None);
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let doc =
                crate::document::Document::read(std::path::Path::new(drawing)).expect("opens");
            let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let _ = app.update(Message::Run("view.zoomExtents"));
            if zoom > 1.0 {
                // Closer in: on the masked text, or on the turned one and its point.
                let camera = &mut app.viewport.camera;
                camera.center = if name == "zemin" {
                    kentos_interaction::Vec2::new(487108.0, 4419985.0)
                } else {
                    kentos_interaction::Vec2::new(487118.0, 4420025.0)
                };
                camera.scale *= zoom;
            }
            snapshot.settle(&mut app, App::view, &mut update);
            let name = if zoom > 1.0 {
                format!("yazi-ekleri-{name}{suffix}")
            } else {
                format!("yazi-ekleri-{width}x{height}{suffix}")
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

/// Leaders (docs/adr/0146) as the drawing shows them: the four arrowheads,
/// one without a note, one masked, one turned and one in a block
/// (fixtures/interaction/v1/leaders.kcad, the web's `shots.mjs leaders`),
/// the second selected so that Öznitelikler shows it, dark and light, at
/// 1440×900 and 1100×650; `.run/shots/kilavuz-*`:
///
/// ```text
/// cargo test -p kentos-desktop labels::leader_screens -- --ignored --nocapture
/// ```
#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn leader_screens() {
    use kentos_ui::snapshot::Snapshot;

    use crate::app::App;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let drawing = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/interaction/v1/leaders.kcad"
    );
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            let (mut app, _) = App::boot(None);
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let doc =
                crate::document::Document::read(std::path::Path::new(drawing)).expect("opens");
            let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let _ = app.update(Message::Run("view.zoomExtents"));
            app.selection.set([kentos_domain::Slot(2)]);
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("kilavuz-{width}x{height}{suffix}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

/// The dimensions of docs/adr/0147 as the drawing shows them, over a parcel
/// and a curved road (fixtures/interaction/v1/dimensions.kcad, the web's
/// `shots.mjs dimensions`): the north-west corner's Y and X, the road
/// edge's arc length and its jogged radius, the west edge's azimuth, the
/// east edge's slope between its corners' elevations, the north edge's
/// value masked over the hatch and a slope in a block; the arc length
/// selected so that Öznitelikler shows it, dark and light, at 1440×900 and
/// 1100×650; `.run/shots/olcu-turleri-*`:
///
/// ```text
/// cargo test -p kentos-desktop labels::dimension_screens -- --ignored --nocapture
/// ```
#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn dimension_screens() {
    use kentos_ui::snapshot::Snapshot;

    use crate::app::App;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let drawing = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/interaction/v1/dimensions.kcad"
    );
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            let (mut app, _) = App::boot(None);
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let doc =
                crate::document::Document::read(std::path::Path::new(drawing)).expect("opens");
            let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let _ = app.update(Message::Run("view.zoomExtents"));
            app.selection.set([kentos_domain::Slot(9)]);
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("olcu-turleri-{width}x{height}{suffix}.png"));
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
                line_weight: None,
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
        spots: Rc::new(app.spatial.labels(
            Vec2::new(v.min_x, v.min_y),
            Vec2::new(v.max_x, v.max_y),
            4.0,
        )),
        camera,
        colors: colors(Canvas::Slate, &crate::viewport::palette(Canvas::Slate)),
        font: DrawingFont::Barlow,
        format: Format::of(doc.settings()),
        key: 0,
        fence: None,
        current: Cell::new(None),
    };
    let started = Instant::now();
    for _ in 0..frames {
        let mut frame = Frame::new(shot.renderer(), Size::new(1440.0, 900.0));
        labels.paint(&mut frame);
    }
    let paint = started.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    println!("query {query:.1} ms, laying out {paint:.1} ms a frame");
}
