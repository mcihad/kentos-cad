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
//! | Askıda (suspended) | not yet: the point calculator is not on the desktop (UX-07) |
//!
//! “Repeat the last command” and what a key means are the host's: it asks
//! [`Session::last`] and routes keys by ADR 0018's order. So is the object
//! snap: the host asks [`Session::snap`] with the pointer and gives the tool
//! the snapped pointer, as the web's viewport does before its tools.

use kentos_geometry_core::store::snap::SnapHit;

use crate::Vec2;
use crate::arc::{self, Arc};
use crate::breaking::{self, Break};
use crate::circle::{self, Circle};
use crate::corner::{self, CornerTool};
use crate::erase::{self, Erase};
use crate::format::Format;
use crate::lengthen::{self, Lengthen};
use crate::line::{self, Line};
use crate::mirror::{self, Mirror};
use crate::move_copy::{self, Move};
use crate::object::{self, ObjectAction};
use crate::offset::{self, Offset};
use crate::path::{self, Path};
use crate::point::{self, Point};
use crate::prompt::Prompt;
use crate::rectangle::{self, Rectangle};
use crate::regular::{self, RegularPolygon};
use crate::rotate::{self, Rotate};
use crate::rotated::{self, RotatedRectangle};
use crate::scale::{self, Scale};
use crate::select::{Select, SelectBox};
use crate::spatial::Spatial;
use crate::tool::{Context, Draft, Flow, Pointer, Preview, Tool, View};
use crate::trim::{self, Boundary};
use crate::vertex::{self, Vertex};

/// Ids of the tools the session runs; each is the web command `tool.<id>`.
pub const TOOLS: &[&str] = &[
    path::POLYGON_ID,
    line::ID,
    path::POLYLINE_ID,
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
];

/// The running tool, if any, the last one started, and the select tool that
/// has the pointer while none runs.
#[derive(Default)]
pub struct Session {
    tool: Option<Box<dyn Tool>>,
    last: Option<&'static str>,
    select: Select,
}

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
            _ => return false,
        };
        self.last = Some(tool.id());
        self.tool = Some(tool);
        self.select.reset();
        true
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

    /// Leaves the running tool; its draft is dropped (ADR 0018, “İptal”).
    pub fn exit(&mut self) {
        self.tool = None;
    }

    /// Esc: the running tool steps back when it can (the web's `cancel`: an
    /// edge tool drops the object it picked); otherwise it is left. Whether it stays.
    pub fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        let stays = self.tool.as_mut().is_some_and(|t| t.cancel(cx));
        if !stays {
            self.tool = None;
        }
        stays
    }

    pub fn is_running(&self) -> bool {
        self.tool.is_some()
    }

    /// The running tool's id; `select` when none runs, as the traces read it.
    pub fn tool_id(&self) -> &'static str {
        self.tool.as_ref().map_or("select", |t| t.id())
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

    pub fn prompt(&self) -> Prompt {
        self.tool.as_ref().map_or_else(Prompt::idle, |t| t.prompt())
    }

    /// The object snap for the pointer at `at` (the web's `updateSnap`):
    /// while a tool that snaps runs and snapping is on, the store's snap
    /// point among the drafting kinds within the aperture, perpendicular and
    /// tangent from the tool's last point. None otherwise: the select tool
    /// and the erase tool do not snap.
    pub fn snap(
        &self,
        spatial: &Spatial,
        at: Vec2,
        view: &dyn View,
        draft: &Draft,
    ) -> Option<SnapHit> {
        let tool = self.tool.as_ref().filter(|t| t.snaps())?;
        if !draft.snap {
            return None;
        }
        let tol = view.world_length(draft.snap_aperture);
        spatial.snap(at, tol, draft.snap_kinds, tool.snap_from())
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
            None => self.select.pointer_down(p),
        }
        self.settle();
    }

    /// The left button came up (on the drawing, or wherever a press on it ended).
    pub fn pointer_up(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        match &mut self.tool {
            Some(tool) => tool.pointer_up(p, cx),
            None => self.select.pointer_up(p, cx),
        }
        self.settle();
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
    pub fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let taken = self.tool.as_mut().is_some_and(|t| t.input(text, cx));
        self.settle();
        taken
    }

    /// A tool that finished with that call leaves: the web's modify tools
    /// call `ctx.tools.exit()` once their transform is written.
    fn settle(&mut self) {
        if self.tool.as_ref().is_some_and(|t| t.finished()) {
            self.tool = None;
        }
    }

    /// Enter, Space or a quick right click while a tool runs: it commits what
    /// it has, or leaves when it has nothing (the web's `PointInputTool.confirm`).
    pub fn confirm(&mut self, cx: &mut Context<'_>) {
        if let Some(tool) = &mut self.tool
            && tool.confirm(cx) == Flow::Exit
        {
            self.tool = None;
        }
    }

    /// Ctrl+Z while a tool runs: its newest step goes first. False when it
    /// had none (or no tool runs): the drawing is undone then (ADR 0018).
    pub fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.tool.as_mut().is_some_and(|t| t.undo_step(cx))
    }

    /// What to draw over the drawing while a tool runs.
    pub fn preview(&self, format: &Format) -> Option<Preview> {
        self.tool.as_ref().map(|t| t.preview(format))
    }
}
