//! Where a key, a click or a typed value goes (docs/adr/0018, 0021): the
//! desktop's side of the tool session (`kentos-interaction`).
//!
//! A key goes to the first of these that takes it:
//!
//! 1. an open text field: the value field beside the cursor, the command
//!    line (a focused text box captures what it types; only the web's global
//!    chords get past it), or a dialog;
//! 2. IME composition: not on the desktop yet;
//! 3. an option letter of the running command (`G` is Geri in the polygon
//!    tool, not the polygon shortcut);
//! 4. a shortcut from the web's inventory;
//! 5. a character that starts a value: a digit, `.`, `@`, and `-` or `+`
//!    while a command runs. It opens the value field beside the cursor when
//!    the pointer is on the drawing, else the command line; either way the
//!    character goes in exactly once;
//! 6. other text starts the command line (ADR 0017).
//!
//! Every change to the drawing is the session's, through the document.

use crate::gis;
use iced::Task;
use iced::advanced::widget::operation;
use iced::keyboard::key::Named;
use iced::widget::operation as widget_operation;

use kentos_domain::Slot;
use kentos_interaction::{
    Context, Draft, Level, LockAsk, Pointer, Session, SnapKind, Toward, Vec2, View, ViewChange,
    js_trim,
};
use kentos_render_wgpu::Camera;

use crate::app::{App, COMMAND_INPUT, Message};
use crate::catalog::{Standing, catalog};
use crate::keys::{self, KeyPress};
use crate::viewport;

/// Zoom step of the + and − keys and the ribbon's buttons (the web's `zoomBy(1.5)`).
const ZOOM_STEP: f64 = 1.5;

/// The value field beside the cursor while it is open (ADR 0018, `UX-04`).
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub text: String,
    /// Transient widget identity; late edits from a closed field cannot edit the next one.
    pub token: u64,
    /// Whether the opted-in tool's readonly plan accepts this editor value.
    pub preview: kentos_interaction::InputPreview,
    /// The world point where it opened; pinned so the pointer can reach it.
    pub at: Vec2,
    /// What the text is: a point, or a lock's value (docs/adr/0166 §6).
    pub mode: Option<LockAsk>,
    /// A setting of the running tool the text writes instead (docs/adr/8018): Duvar's Kalınlık or Hiza, after the
    /// locks in Tab's order.
    pub setting: Option<FieldSetting>,
}

/// The running tool's setting the value field writes (docs/adr/8018), as the tool's settings pane shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldSetting {
    pub key: &'static str,
    pub label: String,
    /// What it holds now, said in the hint.
    pub value: String,
    /// A choice's words (Sol, Orta, Sağ); none for a value typed.
    pub choices: Vec<String>,
    /// The choices offered, any text taken too (a room's name, docs/adr/8021).
    pub open: bool,
}

impl Field {
    /// The field for a point, holding what was typed.
    pub fn point(text: String, at: Vec2) -> Self {
        Self {
            text,
            at,
            mode: None,
            setting: None,
            token: next_field_token(),
            preview: kentos_interaction::InputPreview::Off,
        }
    }
}

/// Folded for a choice's start: Turkish lower case (İ → i, I → ı).
fn folded(text: &str) -> String {
    text.chars()
        .flat_map(|c| match c {
            'İ' => vec!['i'],
            'I' => vec!['ı'],
            c => c.to_lowercase().collect(),
        })
        .collect()
}

fn next_field_token() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

pub(crate) fn field_widget_id(token: u64) -> iced::widget::Id {
    iced::widget::Id::from(format!("cursor-value-{token}"))
}

/// A plain typed number: what Tab locks as a length (the web's `PLAIN_NUMBER`).
fn plain_number(text: &str) -> bool {
    let t = text.strip_prefix(['-', '+']).unwrap_or(text);
    let mut parts = t.splitn(2, ['.', ',']);
    let whole = parts.next().unwrap_or("");
    let fraction = parts.next();
    !whole.is_empty()
        && whole.bytes().all(|b| b.is_ascii_digit())
        && fraction.is_none_or(|f| !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()))
}

/// The drawing area's camera as the session sees it.
pub(crate) struct CameraView<'a>(pub(crate) &'a Camera);

impl View for CameraView<'_> {
    fn to_screen(&self, p: Vec2) -> [f64; 2] {
        self.0.world_to_screen(p)
    }

    fn world_length(&self, px: f64) -> f64 {
        px / self.0.scale
    }

    fn visible(&self) -> kentos_render_wgpu::Bounds {
        self.0.visible_bounds()
    }

    fn twist(&self) -> f64 {
        self.0.twist
    }

    fn to_world(&self, p: [f64; 2]) -> Option<Vec2> {
        Some(self.0.screen_to_world(p[0], p[1]))
    }
}

struct PaperCameraView<'a> {
    camera: CameraView<'a>,
    scale: Option<f64>,
}
impl View for PaperCameraView<'_> {
    fn to_screen(&self, p: Vec2) -> [f64; 2] {
        self.camera.to_screen(p)
    }
    fn world_length(&self, px: f64) -> f64 {
        self.camera.world_length(px)
    }
    fn visible(&self) -> kentos_render_wgpu::Bounds {
        self.camera.visible()
    }
    fn twist(&self) -> f64 {
        self.camera.twist()
    }
    fn to_world(&self, p: [f64; 2]) -> Option<Vec2> {
        self.camera.to_world(p)
    }
    fn paper_scale(&self) -> Option<f64> {
        self.scale
    }
}

impl App {
    /// Scope the reply before work starts, not when it finishes.
    pub(crate) fn tool_reply(&self, task: Task<Message>) -> Task<Message> {
        self.scoped_reply(task, Some(self.session.runs()))
    }

    pub(crate) fn document_reply(&self, task: Task<Message>) -> Task<Message> {
        self.scoped_reply(task, None)
    }

    fn scoped_reply(&self, task: Task<Message>, run: Option<u64>) -> Task<Message> {
        let Some(document) = self.document.as_ref().map(|d| d.session) else {
            return Task::none();
        };
        task.map(move |reply| Message::ScopedReply {
            document,
            run,
            reply: Box::new(reply),
        })
    }

    pub(crate) fn input_focus_lost(&mut self) {
        self.with_tool(|s, cx| s.pointer_cancel(cx));
        self.modifiers = Default::default();
        self.alt_armed = false;
        self.key_tips = None;
        self.drawing_menu = None;
        self.snap = None;
        self.snap_cycle = None;
        self.cursor_point = None;
        self.tracking.clear();
        self.viewport.cursor = None;
        self.panning = false;
        self.pan_recorded = false;
    }

    /// The drawing boundary is stronger than Esc: discard every unfinished
    /// command and drawing aid, without committing or resuming a parent.
    pub(crate) fn leave_document_interaction(&mut self) {
        self.session.leave_document();
        self.field = None;
        // These editors hold object slots from the drawing being left. Do
        // not blur/commit them against the next drawing, even if ids match.
        let inline = self.text_field.take().is_some();
        let paragraph = self.paragraph.take().is_some();
        self.text_field_release |= inline || paragraph;
        self.text_field_focus = false;
        self.text_field_select = false;
        self.paragraph_focus = false;
        self.last_click = None;
        self.snap = None;
        self.snap_cycle = None;
        self.cursor_point = None;
        self.tracking.clear();
        self.locks.reset();
        self.drawing_menu = None;
        self.snap_once = None;
        self.hover_card = None;
        self.bonds_refused = None;
        self.panning = false;
        self.pan_recorded = false;
        self.scale_field = None;
        self.command_input.clear();
        self.text_file_wanted = false;
        self.image_file_wanted = false;
        self.raster_values_wanted.clear();
        self.cloud_query_wanted.clear();
        self.network_wanted.clear();
        self.command_wanted.clear();
        self.form_on_create = None;
        self.gis.attribute_values = None;
        self.block_define_closed();
        if matches!(
            self.dialog,
            Some(
                crate::app::Dialog::Gis(gis::GisDialogKind::AttributeValues)
                    | crate::app::Dialog::BlockDefine
            )
        ) {
            self.dialog = None;
        }
    }

    /// Runs something on the session with the open drawing; `None` when no
    /// drawing is open. What the tool says goes to the command line; the
    /// view changes it asks for (Kaydır, Pencere yakınlaştır) go to the
    /// camera, in order (docs/adr/0056).
    pub(crate) fn with_tool<T>(
        &mut self,
        act: impl FnOnce(&mut Session, &mut Context<'_>) -> T,
    ) -> Option<T> {
        self.sync_sheet_model_index();
        let paper_scale = self.sheets.active_viewport().map(|v| v.denominator);
        // Etkin kat (docs/adr/0713), owned: the context borrows the drawing mutably.
        let level = self.active_level();
        let doc = self.document.as_mut()?;
        // The store answers for the drawing as it is now (docs/adr/0029).
        self.spatial.sync(&doc.model);
        // A group template's members are written with every object the tool
        // writes, in its undo step (docs/adr/0176 §5, template_members.rs).
        let group = self
            .arch
            .template
            .as_ref()
            .is_some_and(|run| !run.members.is_empty())
            .then(|| doc.model.begin_group(kentos_domain::labels::ADD));
        let from = doc.model.next_slot();
        let tool = self.session.tool_id();
        let mut log = Vec::new();
        let mut changes = Vec::new();
        let view = PaperCameraView { camera: CameraView(&self.viewport.camera), scale: paper_scale };
        let out = act(
            &mut self.session,
            &mut Context {
                doc: &mut doc.model,
                view: &view,
                draft: self.draft,
                log: &mut log,
                spatial: &self.spatial,
                selection: &mut self.selection,
                memory: &mut self.memory,
                view_changes: &mut changes,
                tracking: &self.tracking,
                shift: self.modifiers.shift(),
                overlap_layers: &self.overlap_layers,
                locks: &mut self.locks,
                template: self.arch.template.as_ref().map(|run| &run.stamp),
                level: level.as_deref(),
                refused: &mut self.bonds_refused,
            },
        );
        // A drawing tool's one new object on a layer with a form: its form opens (docs/adr/0403 §4).
        if crate::forms::DRAWING_TOOLS.contains(&tool) {
            let made: Vec<Slot> = (from..doc.model.next_slot())
                .filter_map(|s| u32::try_from(s).ok().map(Slot))
                .filter(|s| doc.model.get(*s).is_some())
                .collect();
            if let [one] = made[..] {
                self.form_on_create = Some(one);
            }
        }
        // Geçici izleme noktası (docs/adr/0270 §3): the point given is a tracking point now.
        if let Some(p) = self.session.take_track_point() {
            self.tracking.acquire_point(p);
        }
        if let Some(group) = group {
            // The objects the tool wrote: the slots given out meanwhile.
            let made: Vec<Slot> = (from..doc.model.next_slot())
                .filter_map(|s| u32::try_from(s).ok().map(Slot))
                .filter(|s| doc.model.get(*s).is_some())
                .collect();
            if !made.is_empty() {
                self.write_members(&made);
            }
            if let Some(doc) = self.document.as_mut() {
                doc.model.end_group(group);
            }
        }
        for change in changes {
            match change {
                // Yazı's field opens over the drawing (text_field.rs).
                ViewChange::Text(field) => self.open_text_field(field),
                // Çok satırlı yazı's editor opens over the drawing (paragraph_editor.rs).
                ViewChange::Paragraph(field) => self.open_paragraph(field),
                // Blok oluştur's base point: the window that names the block (blocks.rs).
                ViewChange::DefineBlock(base) => self.open_block_define(base),
                // Taban noktasıyla kopyala's copy: the app's clipboard (clipboard.rs, docs/adr/0309).
                ViewChange::Copied(clip) => self.clipboard.set(*clip),
                // Blok ekle's point: the block's attribute values are asked (attribute_values.rs).
                ViewChange::AttributeValues(block) => self.open_attribute_values(block),
                // Metin dosyası yerleştir's file: the dialog opens after the update (text_file.rs).
                ViewChange::OpenTextFile => self.text_file_wanted = true,
                ViewChange::OpenImageFile => self.image_file_wanted = true,
                // Koordinat oku: the rasters' values under the point, read off the thread (rasters/).
                ViewChange::RasterValues(p) => self.raster_values_wanted.push(p),
                // XYZ sor: the clouds' nearest point, read off the thread (pointclouds/query.rs).
                ViewChange::CloudQuery { at, reach } => self.cloud_query_wanted.push((at, reach)),
                // Ağ analizi: a question to a network, asked on the network thread (networks/).
                ViewChange::Network(ask) => self.network_wanted.push(*ask),
                // A command the tool asks for (Ağlar from the network tools).
                ViewChange::Command(id) => self.command_wanted.push(id),
                // Köşelere koordinat yaz's schedule hangs from the cursor (docs/adr/0185 §1).
                ViewChange::PlaceTable(table, label) => self.place_table(*table, label),
                // Bilgi's card moves to the object clicked, or closes (info_card.rs, docs/adr/0225 §5).
                ViewChange::InfoCard(card) => self.info_card = card,
                // Konumdan adres's place: its address looked for after the update (address/).
                ViewChange::AddressAt(p) => self.gis.address.wanted.push(p),
                // Çizimden: the point goes to the window that asked, which opens again (calc/).
                ViewChange::Picked(p) => {
                    if !self.processing_picked(p)
                        && !self.leader_tools_picked(p)
                        && !self.blocks_picked(p)
                        && !self.xrefs_picked(p)
                        && !self.terrain_picked(p)
                        && !self.grade_picked(p)
                        && !self.pdf_import_picked(p)
                        && !self.service_info_picked(p)
                    {
                        self.calc_picked(p);
                    }
                }
                // Sahneden seç's objects: the processing window opens again (processing/).
                ViewChange::PickedObjects(keep) => {
                    if !self.window_picked(keep)
                        && !self.processing_picked_objects(keep)
                        && !self.edgematch_picked_objects(keep)
                        && !self.table_insert_picked(keep)
                        && !self.text_tools_picked(keep)
                        && !self.form_entry_picked(keep)
                    {
                        let _ = self.builder_picked(keep);
                    }
                }
                // Kaydır: the first step of a drag records the view it leaves; Pencere
                // yakınlaştır's box is a navigation (navigation.rs, docs/adr/0141).
                ViewChange::Pan { dx, dy } => {
                    if !std::mem::replace(&mut self.pan_recorded, true) {
                        self.view_history.record(self.viewpoint());
                    }
                    self.viewport.change(ViewChange::Pan { dx, dy });
                }
                change => self.navigating(|app| app.viewport.change(change)),
            }
        }
        for line in log {
            self.say(line.level, line.text);
        }
        Some(out)
    }

    /// Starts a tool by its id (`polygon`): the ribbon's button, its shortcut,
    /// its name in the command line or Enter repeating it.
    pub(crate) fn start_tool(&mut self, id: &str) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok. Önce bir çizim açın (Ctrl+O).");
            return Task::none();
        }
        // The typed active layer does not hold what the tool draws (layer_types.rs, docs/adr/0622 §5).
        if let Some(why) = self.layer_misfit(&format!("tool.{id}")) {
            self.warn(why);
            return Task::none();
        }
        self.field = None;
        // Kaydır, Pencere yakınlaştır and the measures run over a running command
        // and give it back as it was (docs/adr/0283): its template's run goes on.
        if self.session.can_start_over(id) {
            if self.with_tool(|s, cx| s.start_over(id, cx)) == Some(true) {
                let name = self.tool_name(id);
                self.say(Level::Command, name);
            }
            return Task::none();
        }
        // Another command ends an object template's run (docs/adr/0176 §3).
        self.release_template();
        if self.session.start(id) {
            // A command starting closes Sıradakini seç's chip (docs/adr/0187 §1).
            self.selection.end_cycle();
            // Kaydır is not repeated; any other command is, not the template before it.
            if id != kentos_interaction::navigate::PAN_ID {
                self.last_template = None;
            }
            // The web logs the tool as a command, by its name (`ToolManager.activate`).
            let name = self.tool_name(id);
            self.say(Level::Command, name);
            // It may act at once: the erase tool deletes a selection and leaves.
            self.with_tool(|s, cx| s.activate(cx));
            if !self.session.settings(&self.format()).is_empty() {
                self.docks.show(crate::app::Panel::Properties, kentos_ui::widget::docking::Side::Right);
                return crate::architecture_settings::reveal();
            }
        }
        Task::none()
    }

    /// A started tool's name as the log says it: its command's title; a tool
    /// with no command of its own says its own name, never its id.
    fn tool_name(&self, id: &str) -> String {
        let command = catalog().get(&format!("tool.{id}")).or_else(|| {
            // Koordinat oku and XYZ sor are menu commands, not `tool.<id>`.
            match id {
                kentos_interaction::coordinate::ID => catalog().get("crs.query"),
                kentos_interaction::cloud_query::ID => catalog().get("pointcloud.query"),
                _ => None,
            }
        });
        command
            .map(|command| command.title)
            .or_else(|| self.session.label())
            .unwrap_or(id)
            .to_owned()
    }

    /// `tool.select`: back to selecting; the running command's draft is
    /// dropped, the selection stays (the web activates its select tool).
    pub(crate) fn leave_tool(&mut self) {
        self.field = None;
        self.snap = None;
        self.session.exit();
    }

    /// `tool.confirm`: Enter, Space or the Onayla button. The running tool
    /// commits (or leaves when it has nothing); with no command, or one that
    /// takes no confirm (Kaydır, Pencere yakınlaştır), the last one starts
    /// again (the web's `t.confirm ? t.confirm() : tools.repeatLast()`).
    pub(crate) fn confirm(&mut self) -> Task<Message> {
        if self.field.as_ref().is_some_and(|f| f.preview != kentos_interaction::InputPreview::Off) {
            return self.submit_field();
        }
        if self.session.confirms() {
            self.with_tool(|s, cx| s.confirm(cx));
            if !self.session.is_running() {
                self.field = None;
            }
            return Task::none();
        }
        self.repeat_last()
    }

    /// `tool.repeat`: the last command started again, if there was one.
    /// Kaydır and Yapıştır are not remembered (docs/adr/0056).
    pub(crate) fn repeat_last(&mut self) -> Task<Message> {
        // An object template is started again as itself (docs/adr/0176 §3).
        if let Some(id) = self.last_template.clone() {
            return self.draw_template(&id);
        }
        match self.session.last() {
            Some(last) => self.start_tool(last),
            None => Task::none(),
        }
    }

    /// Whether Esc has something to cancel: a value being typed, a running
    /// command, a grip being moved, a selection.
    pub(crate) fn cancellable(&self) -> bool {
        self.field.is_some()
            || self.session.is_running()
            || self.session.grip_active()
            || !self.selection.is_empty()
    }

    /// `tool.cancel`: Esc. The running tool steps back when it can (an edge
    /// tool drops the object it picked, docs/adr/0047); otherwise it leaves and
    /// its draft is dropped: nothing reaches the drawing. With no command
    /// running, it clears the selection (the web's `ToolManager.exit`).
    pub(crate) fn cancel(&mut self) {
        self.field = None;
        // The snap marker belongs to the command (the web drops it when the tool changes).
        self.snap = None;
        if self.session.is_running() {
            if self.with_tool(|s, cx| s.cancel(cx)) != Some(true) {
                self.session.exit();
            }
        } else if self.session.grip_active() {
            // A grip being moved stays where it was; the selection stays (docs/adr/0068).
            self.with_tool(|s, cx| s.cancel(cx));
        } else {
            self.selection.clear();
        }
    }

    /// Ctrl+Z: the running command's newest step first, then the drawing (ADR 0018).
    pub(crate) fn undo(&mut self) {
        let before = self.document.as_ref().map(|d| d.model.revision());
        if self.session.is_running() && self.with_tool(|s, cx| s.undo_step(cx)) == Some(true) {
            if before != self.document.as_ref().map(|d| d.model.revision()) {
                self.echo_history(true);
            }
            return;
        }
        self.step_history(true);
    }

    pub(crate) fn zoom(&mut self, factor: f64) {
        let camera = &self.viewport.camera;
        let middle = iced::Point::new((camera.width / 2.0) as f32, (camera.height / 2.0) as f32);
        self.viewport.update(
            viewport::Event::Zoomed { factor, at: middle },
            self.document.as_ref(),
        );
    }

    pub(crate) fn zoom_in(&mut self) {
        self.zoom(ZOOM_STEP);
    }

    pub(crate) fn zoom_out(&mut self) {
        self.zoom(1.0 / ZOOM_STEP);
    }

    /// What the drawing area reports: the view changes, and the pointer goes
    /// to the running tool, or to the select tool while none runs (docs/adr/0029).
    pub(crate) fn pointer(&mut self, event: viewport::Event) -> Task<Message> {
        // A press on the drawing, or a pan with the middle button, keeps the text
        // field's text first (the web's blur); the wheel does not (text_field.rs).
        if matches!(
            event,
            viewport::Event::Pressed(_)
                | viewport::Event::RightPressed(_)
                | viewport::Event::Panned { .. }
        ) {
            self.close_text_field(true);
            self.close_paragraph(true);
            // The status bar's scale field closes as a blurred field would (screen_scale.rs).
            self.scale_field = None;
        }
        // The wheel, the middle button and its double click move the view: the
        // history keeps what they leave (navigation.rs, docs/adr/0141).
        let (before, was_panning) = (self.viewpoint(), self.panning);
        if matches!(event, viewport::Event::Extents) {
            self.zoom_extents();
        } else {
            self.viewport.update(event.clone(), self.document.as_ref());
        }
        self.keep_view(&event, before, was_panning);
        // No crosshair while the middle button pans (the web's `panFrom`): a
        // pan reports itself, and the first plain move after it ends it.
        match event {
            viewport::Event::Panned { .. } => self.panning = true,
            viewport::Event::Moved(_) | viewport::Event::Left => self.panning = false,
            _ => {}
        }
        // Where the pointer is, for the constraints' marks (constraint_view.rs).
        self.constraint_view.pointer = match event {
            viewport::Event::Moved(at) => Some(at),
            viewport::Event::Left => None,
            _ => self.constraint_view.pointer,
        };
        match event {
            viewport::Event::Moved(at) => {
                // Keep an open editor where it opened so the pointer can reach
                // its text, selection and paste controls without chasing it.
                let p = self.pointer_at(at);
                self.with_tool(|s, cx| s.pointer_move(&p, cx));
            }
            viewport::Event::Pressed(at) => {
                if self.field.as_ref().is_some_and(|f| f.preview != kentos_interaction::InputPreview::Off) {
                    return self.submit_field();
                }
                // A new drag of Kaydır records its start again.
                self.pan_recorded = false;
                // A click on the drawing takes the keyboard from any text field
                // and the layer tree: the value field closes without applying
                // (web: it loses focus).
                self.field = None;
                self.line_focused = false;
                self.layers_keyboard = false;
                self.blocks_panel.keyboard = false;
                self.arch.templates_panel.keyboard = false;
                self.vertices.keyboard = false;
                // The snap is taken again here, never from the last move (CLAUDE.md §4.7).
                let p = self.pointer_at(at);
                let running = self.session.is_running();
                let gripping = self.session.grip_active();
                self.with_tool(|s, cx| s.pointer_down(&p, cx));
                // A one-shot snap was for this press (the web drops it on a left press).
                self.snap_once = None;
                // No command: a second press on a text or a dimension edits it
                // (text_field.rs), unless the press took or placed a grip.
                if !running && !gripping && !self.session.grip_active() {
                    self.maybe_edit_text(self.viewport.world(at));
                }
            }
            viewport::Event::Released(at) => {
                self.pan_recorded = false;
                let p = self.pointer_at(at);
                self.with_tool(|s, cx| s.pointer_up(&p, cx));
                // The candidate Tab chose was for this click (docs/adr/0271).
                self.snap_cycle = None;
            }
            viewport::Event::Left => {
                self.snap = None;
                if !self.session.is_running() {
                    self.selection.set_hover(None);
                }
            }
            // Held, a menu; a quick click, Enter or the idle menu (drawing_menus.rs).
            viewport::Event::RightPressed(at) => self.right_pressed(at),
            viewport::Event::RightHeld(at) => self.right_held(at),
            viewport::Event::RightClick(at) => return self.right_clicked(at),
            _ => {}
        }
        Task::none()
    }

    /// The pointer at a place of the drawing area as the tools get it: the
    /// world point under it, on the object snap when the running tool snaps
    /// and snapping is on (the web's `updateSnap` and `pointer`). The snap is
    /// kept for its marker.
    pub(crate) fn pointer_at(&mut self, at: iced::Point) -> Pointer {
        self.sync_sheet_model_index();
        let raw = self.viewport.world(at);
        let draft = self.snap_draft();
        self.snap = match &self.document {
            Some(doc) => {
                self.spatial.sync(&doc.model);
                let view = CameraView(&self.viewport.camera);
                self.session
                    .snap(&self.spatial, raw, &view, &draft, &self.tracking)
            }
            None => None,
        };
        // Tab's pick among the candidates while the pointer stays (docs/adr/0271).
        if let Some(hit) = self.cycled_snap(at) {
            self.snap = Some(hit);
        }
        // Nesne izleme and Uzantı's and Paralel's rests (tracking.rs): a rest
        // on a snap or an edge, and with no snap the lock.
        let view = CameraView(&self.viewport.camera);
        self.session.follow(
            &mut self.tracking,
            &self.spatial,
            raw,
            &view,
            &draft,
            self.snap.as_ref(),
        );
        let pointer = Pointer::new(
            raw,
            [f64::from(at.x), f64::from(at.y)],
            self.modifiers.shift(),
            self.snap,
        )
        .with_ctrl(self.modifiers.command())
        .tracked(self.tracking.track().map(|t| t.point));
        self.cursor_point = Some((raw, pointer.world));
        pointer
    }

    /// The drafting aids the snap is asked with: a one-shot snap works even with running snaps off (F3), and
    /// only for its own kind (the web's `updateSnap`, drawing_menus.rs).
    pub(crate) fn snap_draft(&self) -> Draft {
        match self.snap_once() {
            // A one-shot Uzantı snaps to ends too: an end is rested on to
            // acquire its extension (docs/adr/0163 §2).
            Some(kind) => Draft {
                snap: true,
                snap_kinds: if kind == SnapKind::Extension {
                    kind.bit() | SnapKind::Endpoint.bit()
                } else {
                    kind.bit()
                },
                ..self.draft
            },
            None => self.draft,
        }
    }

    /// A key no text box captured, by ADR 0018's order (see the module).
    pub(crate) fn key(&mut self, press: KeyPress) -> Task<Message> {
        // The ribbon's key tips take every key while they show; Alt pressed
        // alone arms them, shown when it is let go (ribbon_keys.rs).
        if let Some(task) = self.key_tips_key(&press) {
            return task;
        }
        if self.alt_tap(&press) {
            return Task::none();
        }
        // Noktalar's cell editor: Tab and Shift+Tab write the cell and go right or left (points/).
        if self.gis.points.editing() && press.named() == Some(Named::Tab) {
            let walk = if press.modifiers.shift() {
                crate::points::Walk::Left
            } else {
                crate::points::Walk::Right
            };
            return self.points_event(crate::points::Event::Finish(Some(walk)));
        }
        // Öznitelik tablosu's cell editor, the same way (features/).
        if self.gis.features.editing() && press.named() == Some(Named::Tab) {
            let walk = crate::features::walk(press.modifiers.shift());
            return self.features_event(crate::features::Event::Finish(Some(walk)));
        }
        // Köşe tablosu's cell editor, the same way (vertices/).
        if self.vertices.editing() && press.named() == Some(Named::Tab) {
            let walk = if press.modifiers.shift() {
                crate::vertices::Walk::Left
            } else {
                crate::vertices::Walk::Right
            };
            return self.vertices_event(crate::vertices::Event::Finish(Some(walk)));
        }
        // Esc closes the status bar's scale field first (screen_scale.rs).
        if self.scale_field.is_some() && press.named() == Some(Named::Escape) {
            self.scale_field = None;
            return Task::none();
        }
        // Esc closes the folded ribbon open over the drawing first (the web's).
        if self.ribbon_peek && press.named() == Some(Named::Escape) {
            self.ribbon_peek = false;
            return Task::none();
        }
        // A sheet in front: its keys (docs/sheet/design.md §11, sheets.rs); what would type
        // into the hidden drawing's command line or reach its tools stays here.
        if self.sheets.is_active() && self.sheets.model_scope().is_none() {
            if let Some(m) = self.sheets.key(&press.key, press.modifiers) {
                return self.sheet_message(m);
            }
            let typing = press.character().is_some()
                || matches!(
                    press.named(),
                    Some(Named::Enter | Named::Space | Named::Tab | Named::Escape)
                );
            if typing && !press.modifiers.command() && !press.modifiers.alt() {
                return Task::none();
            }
        }
        // 0. İfade oluşturucu takes the keys its editor leaves (expression/).
        if let Some(task) = self.builder_key(&press) {
            return task;
        }
        // 1. The application menu takes every key while it is open (app_menu.rs).
        if self.app_menu.is_some() {
            return self.app_menu_key(&press);
        }
        // 1. A dialog: Esc closes it; its own buttons do the rest.
        if self.dialog.is_some() {
            // Tabloyu düzenle's grid takes its keys, Esc too: the bar's words back first (tables/editor.rs).
            if self.dialog == Some(crate::app::Dialog::TableEditor)
                && let Some(task) = self.table_editor_key(&press)
            {
                return task;
            }
            // Model tasarımcısı: Ctrl+S, its undo and Delete (processing/designer/).
            if self.dialog == Some(crate::app::Dialog::Gis(gis::GisDialogKind::ModelDesigner))
                && press.named() != Some(Named::Escape)
                && let Some(task) = self.model_designer_key(&press)
            {
                return task;
            }
            // Formlar: the designer's own undo and Kaydet (forms/designer.rs, docs/adr/0403 §1).
            if self.dialog == Some(crate::app::Dialog::Gis(gis::GisDialogKind::Forms))
                && press.named() != Some(Named::Escape)
                && let Some(task) = self.forms_key(&press)
            {
                return task;
            }
            // Sembol tasarımcısı's undo and ↑ ↓ (style/designer/).
            // SVG çizim düzenleyicisi takes every key, Esc too: it gives up what is half done first (style/svgedit/).
            if self.dialog == Some(crate::app::Dialog::SvgEditor)
                && let Some(task) = self.svgedit_key(&press)
            {
                return task;
            }
            if self.dialog == Some(crate::app::Dialog::SymbolDesigner)
                && press.named() != Some(Named::Escape)
                && let Some(task) = self.designer_key(&press)
            {
                return task;
            }
            // Yeni proje's wizard: Enter goes on, ← → choose the type (project/wizard/).
            if self.dialog == Some(crate::app::Dialog::Project)
                && let Some(key) = press.named()
                && let Some(task) = self.new_project_key(key)
            {
                return task;
            }
            match press.named() {
                // Tab and Shift+Tab go round the window's fields, its choosers and boxes too; never out to
                // the window behind it (kentos_ui::focus, docs/adr/0400).
                Some(Named::Tab) if !press.modifiers.control() && !press.modifiers.alt() => {
                    return kentos_ui::focus::next(press.modifiers.shift());
                }
                // Projeyi paylaş: its question first, then the window; over
                // the catalog, the catalog stays and reads its list again.
                Some(Named::Escape) if self.cloud.share.is_some() => return self.share_escape(),
                // What the window held goes with it (a password, a request).
                Some(Named::Escape) => self.close_dialog(),
                // A question about the file project's revisions: Enter is its
                // amber answer, or, without one, the answer that changes nothing.
                Some(Named::Enter) if self.dialog == Some(crate::app::Dialog::Revision) => {
                    let answer = self.cloud.question.as_ref().map(|q| {
                        q.answers
                            .iter()
                            .find(|a| a.kind == Some(crate::cloud::revisions::AnswerKind::Primary))
                            .map_or(q.cancel, |a| a.value)
                    });
                    if let Some(answer) = answer {
                        return self.cloud_event(crate::cloud::Event::RevisionAnswer(answer));
                    }
                }
                // Ctrl+Enter runs a processing tool from anywhere in its window (the web's).
                Some(Named::Enter)
                    if press.modifiers.control()
                        && self.dialog
                            == Some(crate::app::Dialog::Gis(gis::GisDialogKind::Processing)) =>
                {
                    return self.processing_event(crate::processing::Event::Run);
                }
                // The catalog's list keys (the web's): ↑ ↓ Home End move the
                // selection, Enter does the main action; not under its question.
                Some(key @ (Named::ArrowUp | Named::ArrowDown | Named::Home | Named::End))
                    if self.dialog == Some(crate::app::Dialog::Catalog)
                        && self.cloud.share.is_none() =>
                {
                    use crate::cloud::catalog::Step;
                    let step = match key {
                        Named::ArrowUp => Step::Up,
                        Named::ArrowDown => Step::Down,
                        Named::Home => Step::Home,
                        _ => Step::End,
                    };
                    return self.cloud_event(crate::cloud::Event::CatalogStep(step));
                }
                Some(Named::Enter)
                    if self.dialog == Some(crate::app::Dialog::Catalog)
                        && self.cloud.share.is_none()
                        && !self.cloud.catalog.as_ref().is_some_and(|c| c.busy()) =>
                {
                    return self.cloud_event(crate::cloud::Event::CatalogOpen);
                }
                // ← → change Katman stili's kind when no field has the keyboard (style/layer_style/).
                Some(key @ (Named::ArrowLeft | Named::ArrowRight))
                    if self.dialog == Some(crate::app::Dialog::LayerStyle) =>
                {
                    return self.layer_style_event(crate::style::layer_style::Event::Step(
                        key == Named::ArrowRight,
                    ));
                }
                // Stil yöneticisi: F2 renames the category, Delete asks to delete
                // the item, Enter answers the question or picks (style/manager/).
                Some(key @ (Named::F2 | Named::Delete | Named::Enter))
                    if self.dialog == Some(crate::app::Dialog::StyleManager) =>
                {
                    return self.style_manager_key(key);
                }
                // ↑ ↓ in a Hesap window's table go to the row above or below (calc/grid.rs).
                Some(key @ (Named::ArrowUp | Named::ArrowDown))
                    if self.dialog == Some(crate::app::Dialog::Gis(gis::GisDialogKind::Calc)) =>
                {
                    return crate::calc::arrow(key == Named::ArrowUp);
                }
                // So in Blok öznitelikleri's table (block_attributes.rs).
                Some(key @ (Named::ArrowUp | Named::ArrowDown))
                    if self.dialog == Some(crate::app::Dialog::BlockAttributes) =>
                {
                    return crate::block_attributes::arrow(key == Named::ArrowUp);
                }
                _ => {}
            }
            return Task::none();
        }
        // 1. The paragraph editor has its own keys (Esc, Ctrl+Enter); nothing reaches the app.
        if self.paragraph.is_some() {
            return Task::none();
        }
        // 1. The text field over the drawing takes the keys: Esc drops what
        // was typed, nothing reaches the app (the web's InlineTextEditor).
        if self.text_field.is_some() {
            if press.named() == Some(Named::Escape) {
                self.close_text_field(false);
            }
            return Task::none();
        }
        // 1. The value field takes every key but the global chords.
        if self.field.is_some() {
            return self.field_key(press);
        }
        // 1. The command line has the keyboard: what reaches here passed its
        // text box (Tab, chords); only the global chords work there.
        if self.line_focused {
            return self.global_chord(&press);
        }
        // 1. The layer tree has the keyboard: its keys are its own (layer_tree.rs).
        if self.layers_keyboard
            && let Some(task) = self.layer_key(&press)
        {
            return task;
        }
        // 1. So has the Bloklar list, once a row was pressed (blocks_panel.rs).
        if self.blocks_panel.keyboard
            && let Some(task) = self.blocks_key(&press)
        {
            return task;
        }
        // 1. So has the Şablonlar list, after ↓ in its search (templates_panel.rs).
        if self.arch.templates_panel.keyboard
            && let Some(task) = self.templates_key(&press)
        {
            return task;
        }
        // 1. So has Köşe tablosu, once a row was pressed: Delete removes the
        // selected rows' vertices, not the object (vertices/).
        if self.vertices.keyboard && press.named() == Some(Named::Delete) {
            return self.vertices_event(crate::vertices::Event::Remove);
        }
        // 3. Tab over the drawing while a point is taken: the next snap candidate (docs/adr/0271).
        if press.named() == Some(Named::Tab)
            && (self.session.takes_points() || self.session.grip_active())
            && self.cycle_snap(press.modifiers.shift())
        {
            return Task::none();
        }
        // 3. ↓ over the drawing: the value field opens on the running tool's first setting beside the cursor
        // (docs/adr/8018), as AutoCAD's dynamic input shows a command's options; a room's sizes or Duvar's Hiza are
        // typed without a click. Tab stays the snap candidates' (there is nearly always one: the grid's).
        if press.named() == Some(Named::ArrowDown)
            && !press.modifiers.control()
            && !press.modifiers.alt()
            && self.session.is_running()
            && self.field.is_none()
            && let Some(first) = self.field_settings().into_iter().next()
            && let Some(at) = self.field_place()
        {
            let mut field = Field::point(String::new(), at);
            field.setting = Some(first);
            self.field = Some(field);
            return self.focus_value_field();
        }
        // 3. Tab in a panel's field (Öznitelikler, a dock's form): the next field; with none focused it
        // does nothing (docs/adr/0400).
        if press.named() == Some(Named::Tab) && !press.modifiers.control() && !press.modifiers.alt()
        {
            return kentos_ui::focus::next_if_focused(press.modifiers.shift());
        }
        // 3. An option letter of the running command beats the shortcuts.
        // A hot grip's modes are letters too (docs/adr/0280).
        if (self.session.is_running() || self.session.grip_active())
            && let Some(letter) = keys::option_letter(&press)
            && let Some(option) = self.session.prompt().option_for_key(&letter)
        {
            // The drawing keeps the keyboard: the next letter is an option too (Blok ekle's B, B, A), a digit starts
            // the value field; only an option chosen from the command bar hands it to the command line.
            let _ = self.prompt_option(option.key);
            return Task::none();
        }
        // 4. Shortcuts.
        if let Some(chord) = keys::chord(&press)
            && let Some(id) = self.shortcut(&chord)
        {
            if press.repeat {
                return Task::none();
            }
            // A letter that is not the running command's option does not end it (docs/adr/8003).
            if let Some(why) = self.letter_keeps_command(&press, &chord, id) {
                self.warn(why);
                return Task::none();
            }
            return self.run(id);
        }
        // 5. A character that starts a value.
        if let Some(first) = keys::value_start(&press) {
            if let Some(at) = self.field_place() {
                self.field = Some(Field::point(first.to_string(), at));
                self.refresh_value_preview();
                return self.focus_value_field();
            }
            return self.type_into_line(&first.to_string());
        }
        // 6. Other text starts the command line (CAD habit, ADR 0017).
        match keys::typed(&press) {
            Some(text) => {
                let text = text.to_owned();
                self.type_into_line(&text)
            }
            None => Task::none(),
        }
    }

    /// Why a letter's shortcut does not start its tool now (docs/adr/8003): a command runs, the letter is not one
    /// of its options (those come first, step 3) and the tool would end it, losing its points, the wall chain or the
    /// stair taken so far. AutoCAD reads letters typed during a command as the command's input and never starts
    /// another one by them; a KentOS letter did, so E pressed for a stair's width (G then) started Esnet. The
    /// command stays and is told how to leave; tools that run over a command (Kaydır, Pencere yakınlaştır, the
    /// measures) and commands that start no tool still run.
    fn letter_keeps_command(&self, press: &KeyPress, chord: &str, id: &'static str) -> Option<String> {
        if !self.session.is_running() || keys::option_letter(press).is_none() {
            return None;
        }
        let tool = id.strip_prefix("tool.")?;
        if !Session::tools().contains(&tool) || self.session.can_start_over(tool) {
            return None;
        }
        let running = self.session.prompt().tool.unwrap_or("Komut");
        let target = catalog().get(id).map_or(tool, |c| c.title);
        Some(format!(
            "{running} sürüyor: “{chord}” bu adımın seçeneği değil. {target} için önce Esc'ye basın."
        ))
    }

}

/// The command a chord runs while a text field has the keyboard: one the web
/// binds with `allowInInput` (the inventory's `shortcutsInInput`). They are
/// all chords (Ctrl, Alt, F-keys), so they reach every command, as the
/// drawing's shortcuts do (ADR 0017).
fn shortcut_in_input(chord: &str) -> Option<&'static str> {
    catalog()
        .commands()
        .iter()
        .find(|c| c.shortcuts_in_input.contains(&chord))
        .map(|c| c.id)
}

/// A task that takes the keyboard from every text box: the drawing has it
/// after a tool starts from the command line (the web's `view.focus()`).
pub fn release_keyboard() -> Task<Message> {
    iced::advanced::widget::operate(operation::focusable::unfocus::<()>()).discard()
}

#[cfg(test)]
#[path = "input/value_field_tests.rs"]
mod value_field_tests;

/// The running tool's settings in the value field after its locks (docs/adr/8018): Duvar's Kalınlık and Hiza, Tab
/// from one to the next, Enter writing and closing, a choice by the start of its word or the arrows.
#[cfg(test)]
#[path = "input/cursor_settings_tests.rs"]
mod cursor_settings_tests;

#[path = "input/fields.rs"]
mod fields;
