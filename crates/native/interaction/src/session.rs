//! The tool session (ADR 0018, TODOS.md UX-01): the web's `ToolManager`
//! (`apps/web/src/tools/ToolManager.ts`) without the DOM.
//!
//! | State | Here |
//! |---|---|
//! | Boşta (idle) | no tool: the pointer selects ([`Select`], docs/adr/0029); the traces read the tool as `select` |
//! | Çalışıyor (running) | a tool waits for a point, a number or an option ([`Session::prompt`]) |
//! | Önizleme (preview) | pointer moves change only [`Session::preview`], never the drawing |
//! | Onay (confirm) | [`Session::confirm`]: the tool commits one undo step and stays for the next object |
//! | İptal (cancel) | [`Session::exit`]: the draft is dropped, nothing reaches the drawing or its history |
//! | Askıda (suspended) | [`Session::nest`]: the point calculator runs over a command waiting for a point (or a grip being moved); its point goes to the command as if clicked, Esc brings the command back as it was (docs/adr/0083) |
//!
//! “Repeat the last command” and what a key means are the host's: it asks
//! [`Session::last`] and routes keys by ADR 0018's order. So is the object
//! snap: the host asks [`Session::snap`] with the pointer and gives the tool
//! the snapped pointer, as the web's viewport does before its tools. Kaydır
//! is not remembered for repeat, nor is a tool that is not in the catalog
//! ([`Session::run`]: Yapıştır), as on the web (docs/adr/0056).

use kentos_geometry_core::store::snap::SnapHit;

use crate::Vec2;
use crate::align::{self, Align};
use crate::arc::{self, Arc};
use crate::array::{self, Array};
use crate::array_path;
use crate::breaking::{self, Break};
use crate::circle::{self, Circle};
use crate::corner::{self, CornerTool};
use crate::erase::{self, Erase};
use crate::format::Format;
use crate::lengthen::{self, Lengthen};
use crate::line::{self, Line};
use crate::log::Level;
use crate::mirror::{self, Mirror};
use crate::move_copy::{self, Move};
use crate::navigate::{self, Pan, ZoomWindow};
use crate::object::{self, ObjectAction};
use crate::offset::{self, Offset};
use crate::path::{self, Path};
use crate::perpendicular::{self, Perpendicular};
use crate::point::{self, Point};
use crate::polar::{self, Polar};
use crate::prompt::Prompt;
use crate::rectangle::{self, Rectangle};
use crate::regular::{self, RegularPolygon};
use crate::reshape::{self, Reshape};
use crate::rotate::{self, Rotate};
use crate::rotated::{self, RotatedRectangle};
use crate::scale::{self, Scale};
use crate::select::{Select, SelectBox};
use crate::spatial::Spatial;
use crate::stretch::{self, Stretch};
use crate::tool::{Context, Cursor, Draft, Flow, Pointer, Preview, Tool, View};
use crate::trim::{self, Boundary};
use crate::vertex::{self, Vertex};
use crate::{
    angle, area, between, block_insert, boundary, cleanup, construction, coordinate, dimension,
    dimension_chain,
    divide, donut, ellipse, hatch, match_properties, meeting, parallel, revcloud, sector,
    select_circle, select_containing, select_fence, set_elevation, spline, split, station_offset,
    text,
};

/// Ids of the tools the session runs; each is the web command `tool.<id>`.
pub const TOOLS: &[&str] = &[
    path::POLYGON_ID,
    line::ID,
    path::POLYLINE_ID,
    // Mesafe ölç, Alan hesapla and Parsel oluştur: the path tool's other shapes (docs/adr/0067).
    path::MEASURE_ID,
    path::AREA_ID,
    path::PARCEL_ID,
    erase::ID,
    point::ID,
    circle::ID,
    arc::ID,
    rectangle::ID,
    rotated::ID,
    regular::ID,
    move_copy::MOVE_ID,
    move_copy::COPY_ID,
    rotate::ID,
    scale::ID,
    mirror::ID,
    stretch::ID,
    array::ID,
    polar::ID,
    align::ID,
    offset::ID,
    trim::TRIM_ID,
    trim::EXTEND_ID,
    corner::FILLET_ID,
    corner::CHAMFER_ID,
    breaking::ID,
    object::JOIN_ID,
    object::EXPLODE_ID,
    lengthen::ID,
    vertex::ID,
    navigate::PAN_ID,
    navigate::ZOOM_WINDOW_ID,
    // Drawing tools, round 3 (docs/adr/0057).
    ellipse::ID,
    spline::ID,
    construction::XLINE_ID,
    construction::RAY_ID,
    parallel::ID,
    perpendicular::IN_ID,
    perpendicular::OUT_ID,
    donut::ID,
    revcloud::ID,
    point::SPOT_ID,
    divide::ID,
    text::ID,
    dimension::ID,
    hatch::ID,
    // Blocks (docs/adr/0144).
    block_insert::ID,
    // Alan işlemleri (docs/adr/0065).
    area::UNION_ID,
    area::INTERSECT_ID,
    area::SUBTRACT_ID,
    area::SPLIT_ID,
    area::TO_AREA_ID,
    area::TO_POLYLINE_ID,
    area::PARTS_JOIN_ID,
    area::PARTS_SPLIT_ID,
    boundary::ID,
    // Drawing and editing tools of docs/adr/0140, phase 1.
    reshape::FILLET_ALL_ID,
    reshape::CHAMFER_ALL_ID,
    reshape::REVERSE_ID,
    reshape::SIMPLIFY_ID,
    split::ID,
    cleanup::ID,
    match_properties::ID,
    // Drawing tools of docs/adr/0140, phase 2.
    sector::ID,
    between::ID,
    meeting::ID,
    angle::ID,
    coordinate::ID,
    dimension_chain::CONTINUE_ID,
    dimension_chain::BASELINE_ID,
    // Phase 3: the array along a path (Çit, İki yana and Kaynağı sil are options of existing tools).
    array_path::ID,
    // docs/adr/0141: Dik ayak ölç, then the selecting tools that go back to Seç.
    station_offset::ID,
    select_fence::ID,
    select_circle::ID,
    select_containing::ID,
    // docs/adr/0142: Kot ver.
    set_elevation::ID,
];

/// What a tool running over another one suspended (docs/adr/0083).
enum Suspended {
    /// A command waiting for a point.
    Tool(Box<dyn Tool>),
    /// No command: the select tool with a grip being moved.
    Grip,
}

/// The running tool, if any, the last one started, and the select tool that
/// has the pointer while none runs.
#[derive(Default)]
pub struct Session {
    tool: Option<Box<dyn Tool>>,
    /// The command suspended under the running tool (the point calculator's).
    parent: Option<Suspended>,
    last: Option<&'static str>,
    select: Select,
}

/// What the web says when the command takes no point at its step.
pub const NO_POINT_NOW: &str =
    "Çalışan araç şu adımda nokta beklemiyor; hesaplanan nokta kullanılmadı.";

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    /// The ids of the tools this session can run.
    pub fn tools() -> &'static [&'static str] {
        TOOLS
    }

    /// Starts a tool by id (`polygon`), dropping whatever ran. False for an
    /// id the session does not know; nothing changes then. The host calls
    /// [`Session::activate`] right after.
    pub fn start(&mut self, id: &str) -> bool {
        let tool: Box<dyn Tool> = match id {
            path::POLYGON_ID => Box::new(Path::polygon()),
            path::POLYLINE_ID => Box::new(Path::polyline()),
            path::MEASURE_ID => Box::new(Path::new(path::Shape::MeasureLength)),
            path::AREA_ID => Box::new(Path::new(path::Shape::MeasureArea)),
            path::PARCEL_ID => Box::new(Path::new(path::Shape::Parcel)),
            line::ID => Box::new(Line::new()),
            erase::ID => Box::new(Erase::new()),
            point::ID => Box::new(Point::new()),
            circle::ID => Box::new(Circle::new()),
            arc::ID => Box::new(Arc::new()),
            rectangle::ID => Box::new(Rectangle::new()),
            rotated::ID => Box::new(RotatedRectangle::new()),
            regular::ID => Box::new(RegularPolygon::new()),
            move_copy::MOVE_ID => Box::new(Move::tool()),
            move_copy::COPY_ID => Box::new(Move::copy_tool()),
            rotate::ID => Box::new(Rotate::tool()),
            scale::ID => Box::new(Scale::tool()),
            mirror::ID => Box::new(Mirror::tool()),
            stretch::ID => Box::new(Stretch::new()),
            array::ID => Box::new(Array::tool()),
            polar::ID => Box::new(Polar::tool()),
            align::ID => Box::new(Align::tool()),
            offset::ID => Box::new(Offset::new()),
            trim::TRIM_ID => Box::new(Boundary::trim()),
            trim::EXTEND_ID => Box::new(Boundary::extend()),
            corner::FILLET_ID => Box::new(CornerTool::fillet()),
            corner::CHAMFER_ID => Box::new(CornerTool::chamfer()),
            breaking::ID => Box::new(Break::new()),
            object::JOIN_ID => Box::new(ObjectAction::join()),
            object::EXPLODE_ID => Box::new(ObjectAction::explode()),
            lengthen::ID => Box::new(Lengthen::new()),
            vertex::ID => Box::new(Vertex::new()),
            navigate::PAN_ID => Box::new(Pan::new()),
            navigate::ZOOM_WINDOW_ID => Box::new(ZoomWindow::new()),
            ellipse::ID => Box::new(crate::ellipse::Ellipse::new()),
            spline::ID => Box::new(crate::spline::Spline::new()),
            construction::XLINE_ID => Box::new(crate::construction::Xline::new()),
            construction::RAY_ID => Box::new(crate::construction::Ray::new()),
            parallel::ID => Box::new(crate::parallel::Parallel::new()),
            perpendicular::IN_ID => Box::new(Perpendicular::perpendicular_in()),
            perpendicular::OUT_ID => Box::new(Perpendicular::perpendicular_out()),
            donut::ID => Box::new(crate::donut::Donut::new()),
            revcloud::ID => Box::new(crate::revcloud::RevCloud::new()),
            point::SPOT_ID => Box::new(Point::spot()),
            divide::ID => Box::new(crate::divide::Divide::new()),
            text::ID => Box::new(crate::text::Text::new()),
            dimension::ID => Box::new(crate::dimension::Dimension::new()),
            hatch::ID => Box::new(crate::hatch::Hatch::new()),
            block_insert::ID => Box::new(crate::block_insert::BlockInsert::new()),
            area::UNION_ID => Box::new(crate::area::AreaAction::union()),
            area::INTERSECT_ID => Box::new(crate::area::AreaAction::intersect()),
            area::SUBTRACT_ID => Box::new(crate::area::AreaSubtract::tool()),
            area::SPLIT_ID => Box::new(crate::area::AreaSplit::tool()),
            area::TO_AREA_ID => Box::new(crate::area::AreaAction::to_area()),
            area::TO_POLYLINE_ID => Box::new(crate::area::AreaAction::to_polyline()),
            area::PARTS_JOIN_ID => Box::new(crate::area::AreaAction::parts_join()),
            area::PARTS_SPLIT_ID => Box::new(crate::area::AreaAction::parts_split()),
            boundary::ID => Box::new(crate::boundary::Boundary::new()),
            reshape::FILLET_ALL_ID => Box::new(Reshape::fillet_all()),
            reshape::CHAMFER_ALL_ID => Box::new(Reshape::chamfer_all()),
            reshape::REVERSE_ID => Box::new(Reshape::reverse()),
            reshape::SIMPLIFY_ID => Box::new(Reshape::simplify()),
            split::ID => Box::new(crate::split::Split::new()),
            cleanup::ID => Box::new(crate::cleanup::Cleanup::new()),
            match_properties::ID => Box::new(crate::match_properties::MatchProperties::new()),
            sector::ID => Box::new(crate::sector::Sector::new()),
            between::ID => Box::new(crate::between::PointsBetween::new()),
            meeting::ID => Box::new(crate::meeting::IntersectPoint::new()),
            angle::ID => Box::new(crate::angle::MeasureAngle::new()),
            coordinate::ID => Box::new(crate::coordinate::CrsQuery::new()),
            dimension_chain::CONTINUE_ID => {
                Box::new(crate::dimension_chain::DimensionChain::continued())
            }
            dimension_chain::BASELINE_ID => {
                Box::new(crate::dimension_chain::DimensionChain::baseline())
            }
            array_path::ID => Box::new(crate::array_path::ArrayPath::tool()),
            station_offset::ID => Box::new(station_offset::StationOffset::new()),
            select_fence::ID => Box::new(select_fence::SelectFence::new()),
            select_circle::ID => Box::new(select_circle::SelectCircle::new()),
            select_containing::ID => Box::new(select_containing::SelectContaining::new()),
            set_elevation::ID => Box::new(set_elevation::SetElevation::tool()),
            _ => return false,
        };
        // Kaydır is not repeated: Enter while panning repeats the command
        // before it (the web's `lastRepeatable`).
        if tool.id() != navigate::PAN_ID {
            self.last = Some(tool.id());
        }
        self.run(tool);
        true
    }

    /// Runs a tool that is not in the catalog (Yapıştır with the clipboard's
    /// objects), dropping whatever ran; it is not remembered for repeat (the
    /// web's `ToolManager.run`). The host calls [`Session::activate`] right after.
    pub fn run(&mut self, tool: Box<dyn Tool>) {
        self.tool = Some(tool);
        self.parent = None;
        self.select.reset();
    }

    /// Runs `child` over the running command without ending it (the point
    /// calculator; the web's `ToolManager.nest`, ADR 0018 “Askıda”). The
    /// command keeps its state. When the child finishes, its point goes to
    /// the command as if clicked; Esc brings the command back as it was.
    /// With no command, it runs over a grip being moved. `label` is what the
    /// command line says (“Nokta hesabı: Kenar kesişimi”).
    pub fn nest(&mut self, child: Box<dyn Tool>, label: String, cx: &mut Context<'_>) {
        if self.parent.is_some() {
            return;
        }
        let parent = match self.tool.take() {
            Some(tool) => Suspended::Tool(tool),
            None if self.select.grip_active() => Suspended::Grip,
            None => return,
        };
        self.parent = Some(parent);
        cx.say(Level::Command, label);
        self.tool = Some(child);
        if self
            .tool
            .as_mut()
            .is_some_and(|t| t.activate(cx) == Flow::Exit)
        {
            self.unnest(None, cx);
        }
    }

    /// Whether a tool runs over a suspended command.
    pub fn nested(&self) -> bool {
        self.parent.is_some()
    }

    /// Whether the point calculator may run now (the web's `canCalcPoint`):
    /// a command that takes points and snaps, or a grip being moved; not
    /// over another calculator.
    pub fn can_calc_point(&self) -> bool {
        if self.parent.is_some() {
            return false;
        }
        match &self.tool {
            Some(tool) => tool.accepts_points() && tool.snaps(),
            None => self.select.grip_active(),
        }
    }

    /// Ends the tool run over the suspended command, which goes on; `point`
    /// reaches it as if clicked (the web's `unnest`).
    fn unnest(&mut self, point: Option<Vec2>, cx: &mut Context<'_>) {
        let Some(parent) = self.parent.take() else {
            self.tool = None;
            return;
        };
        self.tool = match parent {
            Suspended::Tool(tool) => Some(tool),
            Suspended::Grip => None,
        };
        let Some(p) = point else {
            return;
        };
        let taken = match &mut self.tool {
            Some(tool) => tool.accept_point(p, cx),
            None => self.select.accept_point(p, cx),
        };
        if !taken {
            cx.say(Level::Warn, NO_POINT_NOW);
        }
    }

    /// Right after a start: the select tool lets go of the hovered object,
    /// and the new tool may act at once (the erase tool deletes a selection
    /// and leaves; the web's `activate`).
    pub fn activate(&mut self, cx: &mut Context<'_>) {
        cx.selection.set_hover(None);
        if let Some(tool) = &mut self.tool
            && tool.activate(cx) == Flow::Exit
        {
            self.tool = None;
        }
    }

    /// Leaves the running tool; its draft is dropped (ADR 0018, “İptal”),
    /// and so is a command suspended under it.
    pub fn exit(&mut self) {
        self.tool = None;
        self.parent = None;
    }

    /// Esc: the running tool steps back when it can (the web's `cancel`: an
    /// edge tool drops the object it picked); otherwise it is left. Whether it stays.
    pub fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        // No command: Esc leaves a grip being moved where it was.
        if self.tool.is_none() {
            return self.select.cancel();
        }
        let stays = self.tool.as_mut().is_some_and(|t| t.cancel(cx));
        if stays {
            return true;
        }
        // Over a suspended command: it comes back as it was.
        if self.parent.is_some() {
            self.unnest(None, cx);
            return true;
        }
        self.tool = None;
        false
    }

    pub fn is_running(&self) -> bool {
        self.tool.is_some()
    }

    /// The running command's id; `select` when none runs, as the traces
    /// read it. Under the point calculator it is the suspended command's.
    pub fn tool_id(&self) -> &'static str {
        match &self.parent {
            Some(Suspended::Tool(tool)) => tool.id(),
            Some(Suspended::Grip) => "select",
            None => self.tool.as_ref().map_or("select", |t| t.id()),
        }
    }

    /// The running tool's name (`Kapalı alan`).
    pub fn label(&self) -> Option<&'static str> {
        self.tool.as_ref().map(|t| t.label())
    }

    /// The last tool started, for Enter or Space with no command (repeat).
    pub fn last(&self) -> Option<&'static str> {
        self.last
    }

    /// Points the running command has taken; 0 when none runs.
    pub fn point_count(&self) -> usize {
        self.tool.as_ref().map_or(0, |t| t.point_count())
    }

    /// Whether a running tool takes Enter, Space and a quick right click as
    /// its confirm. Kaydır and Pencere yakınlaştır do not: Enter repeats the
    /// last command instead (the web's `tool.confirm`, docs/adr/0056). With no
    /// command, a grip being moved takes them (docs/adr/0068).
    pub fn confirms(&self) -> bool {
        self.tool
            .as_ref()
            .map_or(self.select.grip_active(), |t| t.confirms())
    }

    /// Whether no command runs and a grip of the selection is being moved
    /// (docs/adr/0068): typed points, Enter and Esc are the grip's then.
    pub fn grip_active(&self) -> bool {
        self.tool.is_none() && self.select.grip_active()
    }

    /// The grip being moved, drawn larger (the web's `activeGrip`).
    pub fn active_grip(&self) -> Option<(kentos_domain::Slot, usize)> {
        if self.tool.is_some() {
            return None;
        }
        self.select.active_grip()
    }

    /// The pointer's look over the drawing: the running tool's, else the
    /// select tool's pick (the web's `SelectTool.cursor`).
    pub fn cursor(&self) -> Cursor {
        self.tool.as_ref().map_or(Cursor::Pick, |t| t.cursor())
    }

    pub fn prompt(&self) -> Prompt {
        self.tool
            .as_ref()
            .map_or_else(|| self.select.prompt(), |t| t.prompt())
    }

    /// The object snap for the pointer at `at` (the web's `updateSnap`):
    /// while a tool that snaps runs and snapping is on, the store's snap
    /// point among the drafting kinds within the aperture, perpendicular and
    /// tangent from the tool's last point. None otherwise: the erase tool
    /// does not snap, the select tool only while a grip moves (from where it was).
    /// Whether object tracking follows the cursor: a command runs and snaps
    /// (the web's: not the select tool, not a tool that takes no snap).
    pub fn tracks(&self) -> bool {
        self.tool.as_ref().is_some_and(|t| t.snaps())
    }

    /// The running command's last point, for object tracking's crossings.
    pub fn snap_from(&self) -> Option<Vec2> {
        self.tool
            .as_ref()
            .filter(|t| t.snaps())
            .and_then(|t| t.snap_from())
    }

    pub fn snap(
        &self,
        spatial: &Spatial,
        at: Vec2,
        view: &dyn View,
        draft: &Draft,
    ) -> Option<SnapHit> {
        let from = match &self.tool {
            Some(tool) if tool.snaps() => tool.snap_from(),
            None if self.select.grip_active() => self.select.snap_from(),
            _ => return None,
        };
        if !draft.snap {
            return None;
        }
        let tol = view.world_length(draft.snap_aperture);
        spatial.snap(at, tol, draft.snap_kinds, from)
    }

    pub fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        match &mut self.tool {
            Some(tool) => tool.pointer_move(p, cx),
            None => self.select.pointer_move(p, cx),
        }
    }

    /// The left button went down on the drawing.
    pub fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        match &mut self.tool {
            Some(tool) => tool.pointer_down(p, cx),
            None => self.select.pointer_down(p, cx),
        }
        self.settle(cx);
    }

    /// The left button came up (on the drawing, or wherever a press on it ended).
    pub fn pointer_up(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        match &mut self.tool {
            Some(tool) => tool.pointer_up(p, cx),
            None => self.select.pointer_up(p, cx),
        }
        self.settle(cx);
    }

    /// The selection box being drawn: the select tool's while no command
    /// runs, a modify tool's while it picks its objects.
    pub fn select_box(&self) -> Option<SelectBox> {
        match &self.tool {
            Some(tool) => tool.select_box(),
            None => self.select.select_box(),
        }
    }

    /// Typed text for the running tool; false when it does not understand it
    /// or no tool runs.
    /// A text field's answer for the running tool (Yazı).
    pub fn text_typed(&mut self, text: Option<&str>, cx: &mut Context<'_>) {
        if let Some(tool) = self.tool.as_mut() {
            tool.text_typed(text, cx);
        }
        self.settle(cx);
    }

    pub fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let taken = match self.tool.as_mut() {
            Some(tool) => tool.input(text, cx),
            // A typed point places a grip being moved.
            None => self.select.input(text, cx),
        };
        self.settle(cx);
        taken
    }

    /// A tool that finished with that call leaves: the web's modify tools
    /// call `ctx.tools.exit()` once their transform is written. One run over
    /// a suspended command hands it its point, and the command may finish
    /// with that point in turn.
    fn settle(&mut self, cx: &mut Context<'_>) {
        while self.tool.as_ref().is_some_and(|t| t.finished()) {
            if self.parent.is_some() {
                let point = self.tool.as_ref().and_then(|t| t.computed());
                self.unnest(point, cx);
            } else {
                self.tool = None;
            }
        }
    }

    /// Enter, Space or a quick right click while a tool runs: it commits what
    /// it has, or leaves when it has nothing (the web's `PointInputTool.confirm`).
    pub fn confirm(&mut self, cx: &mut Context<'_>) {
        match &mut self.tool {
            Some(tool) => {
                if tool.confirm(cx) == Flow::Exit {
                    if self.parent.is_some() {
                        let point = self.tool.as_ref().and_then(|t| t.computed());
                        self.unnest(point, cx);
                        self.settle(cx);
                    } else {
                        self.tool = None;
                    }
                }
            }
            None => {
                self.select.confirm(cx);
            }
        }
    }

    /// Ctrl+Z while a tool runs: its newest step goes first. False when it
    /// had none (or no tool runs): the drawing is undone then (ADR 0018).
    pub fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.tool.as_mut().is_some_and(|t| t.undo_step(cx))
    }

    /// What to draw over the drawing while a tool runs, or a grip moves.
    pub fn preview(&self, format: &Format) -> Option<Preview> {
        match &self.tool {
            Some(tool) => Some(tool.preview(format)),
            None => self.select.preview(format),
        }
    }
}
