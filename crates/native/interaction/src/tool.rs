//! What a tool is and what it gets: the web's `Tool` interface
//! (`apps/web/src/tools/Tool.ts`) without its DOM side. The host feeds
//! pointer events and typed text; the tool changes the drawing only through
//! the document and says what to draw as [`Preview`] data.

use kentos_domain::Document;
use kentos_geometry_core::geometry::Bounds;
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
    /// The world box the area shows: the edges a trim or an extend meets by
    /// default (the web's `camera.visibleBounds()`).
    fn visible(&self) -> Bounds;
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

/// Uzat-kısalt's mode (the web's `LengthenTool.mode`): the moving end
/// follows the mouse, or every clicked end changes by a difference, a
/// percentage or to a total length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LengthenMode {
    Dynamic,
    Delta,
    Percent,
    Total,
}

/// What the web's drawing tools keep from one run to the next for as long as
/// the page lives (their static fields; docs/adr/0032, 0047): the host keeps
/// it for as long as the app lives. A new app starts with the web's values.
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
    /// Ötele's distance (`OffsetTool.distance`), metres.
    pub offset_distance: f64,
    /// Ötele's “Noktadan geç” (`OffsetTool.through`): the copy passes through the clicked point.
    pub offset_through: bool,
    /// Whether fillet and chamfer cut the corner's sides back (`CornerTool.trimSides`, Kırp).
    pub corner_trim: bool,
    /// The last fillet radius (`FilletTool.last`); none yet.
    pub fillet_radius: Option<f64>,
    /// The last chamfer distances (`ChamferTool.last`); none yet.
    pub chamfer: Option<(f64, f64)>,
    /// Birleştir's end gap tolerance (`JoinTool.tolerance`), metres.
    pub join_tolerance: f64,
    /// Uzat-kısalt's mode and values (`LengthenTool.mode`, `LengthenTool.values`).
    pub lengthen_mode: LengthenMode,
    pub lengthen_delta: f64,
    pub lengthen_percent: f64,
    pub lengthen_total: f64,
    /// Dizi's last rows, columns and spacing, east and north (`ArrayTool.last`).
    pub array_rows: u32,
    pub array_cols: u32,
    pub array_dx: f64,
    pub array_dy: f64,
    /// Kutupsal dizi's count, fill angle in degrees and whether the copies
    /// turn (`PolarArrayTool.last`).
    pub polar_count: u32,
    pub polar_fill: f64,
    pub polar_rotate: bool,
    /// Whether Hizala scales to fit its second pair (`AlignTool.scale`).
    pub align_scale: bool,
    /// Yardımcı çizgi's typed angle, degrees (`XlineTool.angle`; docs/adr/0057).
    pub xline_angle: f64,
    /// Paralel çizgi's left and right distances, metres, whether the axis
    /// is drawn and whether the corridor is one area (`ParallelLineTool`).
    pub parallel_left: f64,
    pub parallel_right: f64,
    pub parallel_axis: bool,
    pub parallel_area: bool,
    /// Halka's inner and outer diameters, metres (`DonutTool`).
    pub donut_inner: f64,
    pub donut_outer: f64,
    /// Revizyon bulutu: a rectangle (else a polygon), and its arc length in
    /// paper millimetres (`RevCloudTool`).
    pub cloud_rect: bool,
    pub cloud_arc_mm: f64,
    /// Böl: the parts, the step in metres, and whether the step applies (`DivideTool`).
    pub divide_parts: u32,
    pub divide_step: f64,
    pub divide_by_step: bool,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            circle_radius: 0.0,
            rect_rotation: 0.0,
            rect_corners: Corners::Sharp,
            polygon_sides: 6,
            polygon_inscribed: true,
            offset_distance: 1.0,
            offset_through: false,
            corner_trim: true,
            fillet_radius: None,
            chamfer: None,
            join_tolerance: 0.001,
            lengthen_mode: LengthenMode::Dynamic,
            lengthen_delta: 1.0,
            lengthen_percent: 100.0,
            lengthen_total: 10.0,
            array_rows: 2,
            array_cols: 3,
            array_dx: 10.0,
            array_dy: 10.0,
            polar_count: 6,
            polar_fill: 360.0,
            polar_rotate: true,
            align_scale: false,
            xline_angle: 0.0,
            parallel_left: 5.0,
            parallel_right: 5.0,
            parallel_axis: true,
            parallel_area: false,
            donut_inner: 0.5,
            donut_outer: 1.0,
            cloud_rect: true,
            cloud_arc_mm: 8.0,
            divide_parts: 4,
            divide_step: 10.0,
            divide_by_step: false,
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

/// The colour a preview part is drawn in, from the theme (the web's palette):
/// the accent, the danger colour (what a trim takes away, a vertex to go) or
/// the snap colour (a corner's reach).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Accent,
    Danger,
    Snap,
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
    pub tone: Tone,
}

impl Stroke {
    /// A solid 1 px line.
    pub fn solid(pts: Vec<Vec2>, closed: bool) -> Self {
        Self {
            pts,
            closed,
            dash: None,
            width: 1.0,
            tone: Tone::Accent,
        }
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
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
    /// The tag's colour: the danger colour where it names what goes (web `drawTag`).
    pub tag_tone: Tone,
    /// Marks at points, drawn 2 px wide: a corner found under the cursor, a
    /// vertex to add or remove (docs/adr/0047).
    pub markers: Vec<Marker>,
    /// A polar tracking ray the cursor is locked to.
    pub tracking: Option<Tracking>,
    /// Filled areas as the web's `drawArea` draws them: a corridor, a donut (docs/adr/0057).
    pub areas: Vec<Area>,
    /// Short texts beside points: the reference line's start “A” (docs/adr/0057).
    pub labels: Vec<Label>,
}

/// An area of a draft, filled in its tone and outlined solid: the outer ring,
/// then its holes (even-odd).
#[derive(Clone, Debug, PartialEq)]
pub struct Area {
    pub rings: Vec<Vec<Vec2>>,
    /// The fill's opacity over the tone's colour (the web's `tint(accent, 0.16)`).
    pub fill: f32,
    /// The outline, logical pixels.
    pub width: f32,
}

/// A text at a world point, moved by a logical pixel offset (right and down).
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    pub at: Vec2,
    pub text: String,
    pub offset: [f32; 2],
    pub tone: Tone,
}

/// A mark at a world point, sized in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Marker {
    pub at: Vec2,
    pub shape: MarkerShape,
    pub tone: Tone,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MarkerShape {
    /// A circle of this radius (the corner tools' 7 px ring).
    Ring(f32),
    /// An × this far from its centre (a vertex to remove).
    Cross(f32),
    /// A + this far from its centre (a vertex to add).
    Plus(f32),
    /// A 1 px circle of this radius (Böl's points to come; docs/adr/0057).
    Circle(f32),
    /// The right angle at a perpendicular's foot: an 8 px square corner, 1
    /// px, its sides towards these world points (docs/adr/0057).
    RightAngle { along: Vec2, up: Vec2 },
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
    /// Esc while the tool runs: it steps back (drops the object it picked,
    /// leaves a sub-step) and returns true, or false when it has nothing to
    /// drop and the session leaves it (the web's `Tool.cancel`).
    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }
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
