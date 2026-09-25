//! What a tool is and what it gets: the web's `Tool` interface
//! (`apps/web/src/tools/Tool.ts`) without its DOM side. The host feeds
//! pointer events and typed text; the tool changes the drawing only through
//! the document and says what to draw as [`Preview`] data.

use kentos_domain::Document;
use kentos_geometry_core::tools::point_input::Tracking;

use crate::Vec2;
use crate::format::Format;
use crate::log::{Level, Line};
use crate::prompt::Prompt;

/// Where the pointer is, as a tool sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pointer {
    /// The world point under it (x east, y north), through the view's float64 camera.
    pub world: Vec2,
    /// Logical pixels from the drawing area's top-left corner.
    pub screen: [f64; 2],
    /// Shift held: turns ortho over for this point (web).
    pub shift: bool,
}

/// The drawing area as a tool needs it: the camera's mapping, nothing else.
pub trait View {
    /// Where a world point is on the area, in logical pixels.
    fn to_screen(&self, p: Vec2) -> [f64; 2];
    /// How much world `px` logical pixels span at the current zoom.
    fn world_length(&self, px: f64) -> f64;
}

/// Drafting aids that change where a point goes (the project's draft settings).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Draft {
    pub ortho: bool,
    /// Polar tracking's step in degrees; `None` when it is off.
    pub polar: Option<f64>,
    /// How near a click must be to a point to be that point (kenet yarıçapı), logical pixels.
    pub snap_aperture: f64,
}

impl Default for Draft {
    /// The web's defaults: ortho and polar off, an 11 px aperture (app/state.ts).
    fn default() -> Self {
        Self {
            ortho: false,
            polar: None,
            snap_aperture: 11.0,
        }
    }
}

/// What a tool works with during one call.
pub struct Context<'a> {
    /// The open drawing: the only way a tool changes anything.
    pub doc: &'a mut Document,
    pub view: &'a dyn View,
    pub draft: Draft,
    /// Messages for the user, newest last; the host shows them.
    pub log: &'a mut Vec<Line>,
}

impl Context<'_> {
    pub fn format(&self) -> Format {
        Format::of(self.doc.settings())
    }

    pub(crate) fn say(&mut self, level: Level, text: impl Into<String>) {
        self.log.push(Line::new(level, text));
    }
}

/// Whether the tool stays after a confirm, or the session goes back to idle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    Stay,
    Exit,
}

/// A measurement shown beside the cursor.
#[derive(Clone, Debug, PartialEq)]
pub struct Tag {
    /// The cursor's world point; the host places the tag below-right of it.
    pub at: Vec2,
    pub lines: Vec<String>,
}

/// What the running tool wants drawn over the drawing; the drawing itself
/// does not change until a confirm (ADR 0018, “Önizleme”). World
/// coordinates; arcs already tessellated by the shared core.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Preview {
    /// The shape so far with the cursor's segment, open.
    pub path: Vec<Vec2>,
    /// The closed shape (dashed, lightly filled) once it has three corners.
    pub ring: Option<Vec<Vec2>>,
    /// Helper lines, dashed: to a point given before an arc's end, and the like.
    pub guides: Vec<[Vec2; 2]>,
    pub tag: Option<Tag>,
    /// A polar tracking ray the cursor is locked to.
    pub tracking: Option<Tracking>,
}

/// An interactive tool.
pub trait Tool {
    /// The tool's id (`polygon`); its command is `tool.<id>`.
    fn id(&self) -> &'static str;
    /// The tool's name in the prompt (`Kapalı alan`).
    fn label(&self) -> &'static str;
    fn prompt(&self) -> Prompt;
    /// Points the running command has taken (the web's `Tool.pointCount`).
    fn point_count(&self) -> usize;
    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>);
    /// The left button went down.
    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>);
    /// Typed text: a coordinate, a number or an option. False when not understood.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool;
    /// Enter, Space or a quick right click.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow;
    /// Ctrl+Z while the tool runs: takes back its newest step and returns
    /// true, or false when nothing is pending and the drawing is undone instead.
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool;
    fn preview(&self, format: &Format) -> Preview;
}
