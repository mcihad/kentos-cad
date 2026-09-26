//! What a tool is and what it gets: the web's `Tool` interface
//! (`apps/web/src/tools/Tool.ts`) without its DOM side. The host feeds
//! pointer events and typed text; the tool changes the drawing only through
//! the document and says what to draw as [`Preview`] data.

use kentos_domain::Document;
use kentos_geometry_core::store::snap::{SnapHit, SnapKind};
use kentos_geometry_core::tools::point_input::Tracking;

use crate::Vec2;
use crate::format::Format;
use crate::log::{Level, Line};
use crate::prompt::Prompt;
use crate::select::SelectBox;
use crate::selection::Selection;
use crate::spatial::Spatial;

/// Where the pointer is, as a tool sees it (the web's `ToolPointer`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pointer {
    /// The world point it stands for (x east, y north): the object snap's
    /// point when one applies, else `raw`.
    pub world: Vec2,
    /// The world point under it, through the view's float64 camera, before snapping.
    pub raw: Vec2,
    /// Logical pixels from the drawing area's top-left corner.
    pub screen: [f64; 2],
    /// Shift held: turns ortho over for this point, and adds to the selection (web).
    pub shift: bool,
    /// The object snap `world` came from. A snapped point is exact: ortho and
    /// polar tracking never move it (the web's `constrainPoint`).
    pub snap: Option<SnapHit>,
}

impl Pointer {
    /// The pointer at `raw`, on `snap`'s point when there is one.
    pub fn new(raw: Vec2, screen: [f64; 2], shift: bool, snap: Option<SnapHit>) -> Self {
        Self {
            world: snap.map_or(raw, |s| s.point),
            raw,
            screen,
            shift,
            snap,
        }
    }
}

/// The drawing area as a tool needs it: the camera's mapping, nothing else.
pub trait View {
    /// Where a world point is on the area, in logical pixels.
    fn to_screen(&self, p: Vec2) -> [f64; 2];
    /// How much world `px` logical pixels span at the current zoom.
    fn world_length(&self, px: f64) -> f64;
}

/// Drafting aids that change where a point goes, and what a click picks
/// (the typed settings' `drafting.*` and `snap.*`, docs/adr/0023, 0029).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Draft {
    pub ortho: bool,
    /// Polar tracking's step in degrees; `None` when it is off.
    pub polar: Option<f64>,
    /// How near a click must be to a point to be that point (kenet yarıçapı), logical pixels.
    pub snap_aperture: f64,
    /// Object snap is on (F3, `drafting.snap`).
    pub snap: bool,
    /// The snap kinds that apply, as `SnapKind::bit`s (`snap.*`, see [`snap_kinds`]).
    pub snap_kinds: u32,
    /// How near a click must be to an object to pick it (`drafting.pickAperture`), logical pixels.
    pub pick_aperture: f64,
}

impl Default for Draft {
    /// The web's defaults: ortho and polar off, an 11 px snap aperture,
    /// snapping on with its default kinds (all but nearest), a 5 px pick
    /// aperture (app/state.ts, the settings schema).
    fn default() -> Self {
        Self {
            ortho: false,
            polar: None,
            snap_aperture: 11.0,
            snap: true,
            snap_kinds: snap_kinds(|key| key != "snap.nearest"),
            pick_aperture: 5.0,
        }
    }
}

/// The snap kinds the settings turn on, as `SnapKind::bit`s: the web's
/// `snapKinds` (ViewportController), where the endpoint setting also brings
/// quadrants (its text says so: “dairelerin çeyrek noktaları”). `on` reads
/// a `snap.*` setting.
pub fn snap_kinds(on: impl Fn(&str) -> bool) -> u32 {
    let mut kinds = 0;
    for (key, kind) in [
        ("snap.endpoint", SnapKind::Endpoint),
        ("snap.midpoint", SnapKind::Midpoint),
        ("snap.center", SnapKind::Center),
        ("snap.node", SnapKind::Node),
        ("snap.endpoint", SnapKind::Quadrant),
        ("snap.intersection", SnapKind::Intersection),
        ("snap.perpendicular", SnapKind::Perpendicular),
        ("snap.nearest", SnapKind::Nearest),
        ("snap.tangent", SnapKind::Tangent),
    ] {
        if on(key) {
            kinds |= kind.bit();
        }
    }
    kinds
}

/// The rectangle tool's corners (the web's `CornerStyle`): sharp, rounded
/// by a radius (Köşe yuvarla) or cut by a distance (Pah).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Corners {
    Sharp,
    Fillet(f64),
    Chamfer(f64),
}

/// What the web's drawing tools keep from one run to the next for as long as
/// the page lives (their static fields; docs/adr/0032): the host keeps it for
/// as long as the app lives. A new app starts with the web's values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Memory {
    /// The last circle's radius, offered by Teğet-teğet-yarıçap (`CircleTool.lastRadius`); 0: none yet.
    pub circle_radius: f64,
    /// The rectangle tool's rotation, radians (`RectangleTool.rotation`).
    pub rect_rotation: f64,
    /// The rectangle tool's corners (`RectangleTool.corners`).
    pub rect_corners: Corners,
    /// The regular polygon's side count (`RegularPolygonTool.sides`).
    pub polygon_sides: u32,
    /// Whether the regular polygon's circle passes through its corners
    /// (`RegularPolygonTool.inscribed`), else it touches its edges.
    pub polygon_inscribed: bool,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            circle_radius: 0.0,
            rect_rotation: 0.0,
            rect_corners: Corners::Sharp,
            polygon_sides: 6,
            polygon_inscribed: true,
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
    /// The geometry store, in step with the document when the call began
    /// (docs/adr/0029): what a click picks.
    pub spatial: &'a Spatial,
    /// The selection: the select tool changes it, the erase tool deletes it.
    pub selection: &'a mut Selection,
    /// What the drawing tools remember between runs.
    pub memory: &'a mut Memory,
}

impl Context<'_> {
    pub fn format(&self) -> Format {
        Format::of(self.doc.settings())
    }

    pub(crate) fn say(&mut self, level: Level, text: impl Into<String>) {
        self.log.push(Line::new(level, text));
    }

    /// The world length the pick aperture spans at this zoom (the web's
    /// `pickAperture / camera.scale`).
    pub(crate) fn pick_tolerance(&self) -> f64 {
        self.view.world_length(self.draft.pick_aperture)
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

/// A line of a draft drawn as the web's `strokePath` draws it: the circle,
/// the arc or the rectangle that would be written, a dashed guide circle.
#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    /// World points, curves already tessellated by the shared core.
    pub pts: Vec<Vec2>,
    pub closed: bool,
    /// Dash and gap, logical pixels; solid when none.
    pub dash: Option<[f32; 2]>,
    /// Logical pixels.
    pub width: f32,
}

impl Stroke {
    /// A solid 1 px line.
    pub fn solid(pts: Vec<Vec2>, closed: bool) -> Self {
        Self {
            pts,
            closed,
            dash: None,
            width: 1.0,
        }
    }

    pub fn dashed(pts: Vec<Vec2>, closed: bool, dash: [f32; 2]) -> Self {
        Self {
            dash: Some(dash),
            ..Self::solid(pts, closed)
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
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
    /// Lines drawn as the web's `strokePath` draws them (docs/adr/0032).
    pub strokes: Vec<Stroke>,
    /// Points marked with a 9 px square: the objects picked for a tangent circle.
    pub squares: Vec<Vec2>,
    /// Points marked with a 7 px square, solid: points and texts among a
    /// modify tool's ghosts (the web's `strokePaths` markers, docs/adr/0037).
    pub marks: Vec<Vec2>,
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
    /// Right after it starts, with what the session knows (the web's
    /// `activate`): the erase tool deletes the selection and leaves.
    fn activate(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }
    /// Whether object snaps apply while it runs (the web's `Tool.snaps`).
    fn snaps(&self) -> bool {
        true
    }
    /// The point perpendicular and tangent snaps are taken from: the last
    /// point given (the web's `snapFrom`).
    fn snap_from(&self) -> Option<Vec2> {
        None
    }
    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>);
    /// The left button went down.
    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>);
    /// The left button came up.
    fn pointer_up(&mut self, _p: &Pointer, _cx: &mut Context<'_>) {}
    /// Typed text: a coordinate, a number or an option. False when not understood.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool;
    /// Enter, Space or a quick right click.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow;
    /// Ctrl+Z while the tool runs: takes back its newest step and returns
    /// true, or false when nothing is pending and the drawing is undone instead.
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool;
    fn preview(&self, format: &Format) -> Preview;
    /// The tool is done and leaves after the call that finished it (the web's
    /// `ctx.tools.exit()` from a click or typed text: a move written).
    fn finished(&self) -> bool {
        false
    }
    /// The selection box being drawn by a tool that picks objects itself
    /// (the modify tools before their points), for the host to show.
    fn select_box(&self) -> Option<SelectBox> {
        None
    }
}
