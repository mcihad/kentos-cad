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
use crate::erase::{self, Erase};
use crate::format::Format;
use crate::line::{self, Line};
use crate::path::{self, Path};
use crate::prompt::Prompt;
use crate::select::{Select, SelectBox};
use crate::spatial::Spatial;
use crate::tool::{Context, Draft, Flow, Pointer, Preview, Tool, View};

/// Ids of the tools the session runs; each is the web command `tool.<id>`.
pub const TOOLS: &[&str] = &[path::POLYGON_ID, line::ID, path::POLYLINE_ID, erase::ID];

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

    /// Esc: leaves the running tool; its draft is dropped (ADR 0018, “İptal”).
    pub fn exit(&mut self) {
        self.tool = None;
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
    }

    /// The left button came up (on the drawing, or wherever a press on it ended).
    pub fn pointer_up(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        match &mut self.tool {
            Some(tool) => tool.pointer_up(p, cx),
            None => self.select.pointer_up(p, cx),
        }
    }

    /// The selection box being drawn while no command runs.
    pub fn select_box(&self) -> Option<SelectBox> {
        match self.tool {
            Some(_) => None,
            None => self.select.select_box(),
        }
    }

    /// Typed text for the running tool; false when it does not understand it
    /// or no tool runs.
    pub fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.tool.as_mut().is_some_and(|t| t.input(text, cx))
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
