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

use iced::Task;
use iced::advanced::widget::operation;
use iced::keyboard::key::Named;
use iced::widget::operation as widget_operation;

use kentos_interaction::{
    Context, Draft, Level, Pointer, Session, SnapKind, Vec2, View, ViewChange, js_trim,
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
    /// The world point it sits beside: the pointer's, as it moves.
    pub at: Vec2,
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
}

impl App {
    /// Runs something on the session with the open drawing; `None` when no
    /// drawing is open. What the tool says goes to the command line; the
    /// view changes it asks for (Kaydır, Pencere yakınlaştır) go to the
    /// camera, in order (docs/adr/0056).
    pub(crate) fn with_tool<T>(
        &mut self,
        act: impl FnOnce(&mut Session, &mut Context<'_>) -> T,
    ) -> Option<T> {
        let doc = self.document.as_mut()?;
        // The store answers for the drawing as it is now (docs/adr/0029).
        self.spatial.sync(&doc.model);
        let mut log = Vec::new();
        let mut changes = Vec::new();
        let view = CameraView(&self.viewport.camera);
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
            },
        );
        for change in changes {
            match change {
                // Yazı's field opens over the drawing (text_field.rs).
                ViewChange::Text(field) => self.open_text_field(field),
                // Blok oluştur's base point: the window that names the block (blocks.rs).
                ViewChange::DefineBlock(base) => self.open_block_define(base),
                // Blok ekle's point: the block's attribute values are asked (attribute_values.rs).
                ViewChange::AttributeValues(block) => self.open_attribute_values(block),
                // Metin dosyası yerleştir's file: the dialog opens after the update (text_file.rs).
                ViewChange::OpenTextFile => self.text_file_wanted = true,
                // Çizimden: the point goes to the window that asked, which opens again (calc/).
                ViewChange::Picked(p) => {
                    if !self.processing_picked(p) && !self.blocks_picked(p) {
                        self.calc_picked(p);
                    }
                }
                // Sahneden seç's objects: the processing window opens again (processing/).
                ViewChange::PickedObjects(keep) => {
                    if !self.processing_picked_objects(keep) && !self.edgematch_picked_objects(keep)
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
        self.field = None;
        if self.session.start(id) {
            // The web logs the tool as a command, by its name (`ToolManager.activate`).
            let command = catalog().get(&format!("tool.{id}")).or_else(|| {
                // Koordinat oku is a menu command, not `tool.<id>`.
                (id == kentos_interaction::coordinate::ID)
                    .then(|| catalog().get("crs.query"))
                    .flatten()
            });
            let name = command.map_or(id, |command| command.title);
            self.say(Level::Command, name);
            // It may act at once: the erase tool deletes a selection and leaves.
            self.with_tool(|s, cx| s.activate(cx));
        }
        Task::none()
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
        if self.session.is_running() && self.with_tool(|s, cx| s.undo_step(cx)) == Some(true) {
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
        match event {
            viewport::Event::Moved(at) => {
                let world = self.viewport.world(at);
                if let Some(field) = &mut self.field {
                    field.at = world;
                }
                let p = self.pointer_at(at);
                self.with_tool(|s, cx| s.pointer_move(&p, cx));
            }
            viewport::Event::Pressed(at) => {
                // A new drag of Kaydır records its start again.
                self.pan_recorded = false;
                // A click on the drawing takes the keyboard from any text field
                // and the layer tree: the value field closes without applying
                // (web: it loses focus).
                self.field = None;
                self.line_focused = false;
                self.layers_keyboard = false;
                self.blocks_panel.keyboard = false;
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
            viewport::Event::RightClick(at) => self.right_clicked(at),
            _ => {}
        }
        Task::none()
    }

    /// The pointer at a place of the drawing area as the tools get it: the
    /// world point under it, on the object snap when the running tool snaps
    /// and snapping is on (the web's `updateSnap` and `pointer`). The snap is
    /// kept for its marker.
    fn pointer_at(&mut self, at: iced::Point) -> Pointer {
        let raw = self.viewport.world(at);
        // A one-shot snap works even with running snaps off (F3), and only
        // for its own kind (the web's `updateSnap`, drawing_menus.rs).
        let draft = match self.snap_once() {
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
        };
        self.snap = match &self.document {
            Some(doc) => {
                self.spatial.sync(&doc.model);
                let view = CameraView(&self.viewport.camera);
                self.session
                    .snap(&self.spatial, raw, &view, &draft, &self.tracking)
            }
            None => None,
        };
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
        .tracked(self.tracking.track().map(|t| t.point));
        self.cursor_point = Some((raw, pointer.world));
        pointer
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
        if self.points.editing() && press.named() == Some(Named::Tab) {
            let walk = if press.modifiers.shift() {
                crate::points::Walk::Left
            } else {
                crate::points::Walk::Right
            };
            return self.points_event(crate::points::Event::Finish(Some(walk)));
        }
        // Esc closes the folded ribbon open over the drawing first (the web's).
        if self.ribbon_peek && press.named() == Some(Named::Escape) {
            self.ribbon_peek = false;
            return Task::none();
        }
        // A sheet in front: its keys (docs/sheet/design.md §11, sheets.rs); what would type
        // into the hidden drawing's command line or reach its tools stays here.
        if self.sheets.is_active() {
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
            // Model tasarımcısı: Ctrl+S, its undo and Delete (processing/designer/).
            if self.dialog == Some(crate::app::Dialog::ModelDesigner)
                && press.named() != Some(Named::Escape)
                && let Some(task) = self.model_designer_key(&press)
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
                        && self.dialog == Some(crate::app::Dialog::Processing) =>
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
                    if self.dialog == Some(crate::app::Dialog::Calc) =>
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
        // 3. An option letter of the running command beats the shortcuts.
        if self.session.is_running()
            && let Some(letter) = keys::option_letter(&press)
            && let Some(option) = self.session.prompt().option_for_key(&letter)
        {
            return self.prompt_option(option.key);
        }
        // 4. Shortcuts.
        if let Some(chord) = keys::chord(&press)
            && let Some(id) = self.shortcut(&chord)
        {
            return if press.repeat {
                Task::none()
            } else {
                self.run(id)
            };
        }
        // 5. A character that starts a value.
        if let Some(first) = keys::value_start(&press) {
            if let Some(at) = self.field_place() {
                self.field = Some(Field {
                    text: first.to_string(),
                    at,
                });
                return Task::none();
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

    /// Where the value field opens: beside the pointer, when it is on the
    /// drawing, a command runs and the preference is on (the web's `CursorInput.accepts`).
    fn field_place(&self) -> Option<Vec2> {
        // A grip waiting for its new place takes one too (docs/adr/0068, 0069).
        let takes = self.session.is_running() || self.session.grip_active();
        (self.cursor_input && takes)
            .then_some(self.viewport.cursor)
            .flatten()
    }

    /// Focuses the command line with text typed elsewhere, after what it holds.
    fn type_into_line(&mut self, text: &str) -> Task<Message> {
        self.command_input.push_str(text);
        widget_operation::focus(COMMAND_INPUT)
    }

    /// A key while the value field is open (ADR 0018, “Tuş ve fare anlamları”).
    fn field_key(&mut self, press: KeyPress) -> Task<Message> {
        match press.named() {
            // Space is a second Enter, as in AutoCAD.
            Some(Named::Enter | Named::Space) => return self.submit_field(),
            Some(Named::Escape) => self.field = None,
            Some(Named::Backspace) => {
                if let Some(field) = &mut self.field {
                    field.text.pop();
                }
            }
            // One field for now: Tab keeps the typed value and moves nothing.
            Some(Named::Tab) => {}
            _ => match keys::typed(&press) {
                Some(text) => {
                    if let Some(field) = &mut self.field {
                        field.text.push_str(text);
                    }
                }
                None => return self.global_chord(&press),
            },
        }
        Task::none()
    }

    /// Runs a chord that works in text fields too (Ctrl+S, F-keys: the web's
    /// `allowInInput` bindings, from the inventory); nothing else.
    fn global_chord(&mut self, press: &KeyPress) -> Task<Message> {
        match keys::chord(press).and_then(|chord| shortcut_in_input(&chord)) {
            Some(id) if !press.repeat => self.run(id),
            _ => Task::none(),
        }
    }

    /// Enter or Space in the value field (the web's `CursorInput.submit`).
    fn submit_field(&mut self) -> Task<Message> {
        let Some(field) = self.field.take() else {
            return Task::none();
        };
        let text = js_trim(&field.text).to_owned();
        if text.is_empty() {
            return self.run("tool.confirm");
        }
        self.echo_value(text.clone());
        if self.with_tool(|s, cx| s.input(&text, cx)) != Some(true) {
            self.warn(format!(
                "“{text}” anlaşılamadı. Mesafe, Y,X, @dY,dX ya da @mesafe<açı yazın."
            ));
        }
        Task::none()
    }

    /// Enter or Space in the command line with no suggestion chosen (the web's
    /// `CommandLine.submit`): a value for the running command, else a command's name.
    pub(crate) fn submit_line(&mut self, text: &str) -> Task<Message> {
        if text.is_empty() {
            return self.run("tool.confirm");
        }
        // A value for the running command, or for a grip being moved (docs/adr/0068).
        if self.session.is_running() || self.session.grip_active() {
            self.echo_value(text);
            self.remember(text);
            if self.with_tool(|s, cx| s.input(text, cx)) != Some(true) {
                // Nokta hesapla by its alias while a point is expected (docs/adr/0083).
                if let Some(def) = kentos_interaction::point_calc::CalcDef::by_alias(text)
                    && self.session.can_calc_point()
                {
                    return self.start_point_calc(def.kind);
                }
                self.warn(format!(
                    "“{text}” anlaşılamadı. Koordinatı Y,X ya da @dY,dX biçiminde yazın."
                ));
            }
            return Task::none();
        }
        self.run_typed(text)
    }

    /// An option of the running command, from its letter or its button (the
    /// web's `runPromptOption`).
    pub(crate) fn prompt_option(&mut self, key: &str) -> Task<Message> {
        match key {
            "Enter" => self.run("tool.confirm"),
            "Esc" => self.run("tool.cancel"),
            _ => {
                self.echo_value(key);
                if self.with_tool(|s, cx| s.input(key, cx)) != Some(true) {
                    self.warn(format!("“{key}” seçeneği şu adımda kullanılamıyor."));
                }
                Task::none()
            }
        }
    }

    /// A value an option offers chosen from its menu, as if its key and the
    /// value had been typed (the web's `runPromptChoice`, docs/adr/0145 §6).
    pub(crate) fn prompt_choice(
        &mut self,
        key: &'static str,
        typed: &'static str,
    ) -> Task<Message> {
        self.echo_value(format!("{key} {typed}"));
        if self.with_tool(|s, cx| s.choose_option(key, typed, cx)) != Some(true) {
            self.warn(format!("“{key}” seçeneği şu adımda kullanılamıyor."));
        }
        Task::none()
    }

    /// A tool's method from its ribbon menu (docs/adr/0032), as the web's
    /// `runEntry`: the tool starts, then its option goes in as if typed. A
    /// method the tool refuses now says so; a tool that did not start has
    /// said why already.
    pub(crate) fn run_method(
        &mut self,
        id: &'static str,
        option: &str,
        label: &str,
    ) -> Task<Message> {
        let task = self.run(id);
        let started =
            self.session.is_running() && id.strip_prefix("tool.") == Some(self.session.tool_id());
        if started && self.with_tool(|s, cx| s.input(option, cx)) != Some(true) {
            let title = catalog().get(id).map_or(id, |command| command.title);
            self.warn(crate::ribbon_plan::texts::cannot_start(title, label));
        }
        task
    }

    /// The command a chord runs. Single keys reach only the commands the
    /// desktop runs (ADR 0017); while a command runs, + and − begin a value
    /// and zoom only when none runs (ADR 0018).
    pub(crate) fn shortcut(&self, chord: &str) -> Option<&'static str> {
        // Shift with Enter confirms as Enter does: what a selecting tool finds
        // then joins the selection (docs/adr/0141).
        let chord = if chord == "Shift+Enter" {
            "Enter"
        } else {
            chord
        };
        let command = catalog()
            .commands()
            .iter()
            .find(|c| c.shortcuts.contains(&chord))?;
        if !keys::is_chorded(chord) && command.standing != Standing::Ported {
            return None;
        }
        let takes = self.session.is_running() || self.session.grip_active();
        if takes && matches!(command.id, "view.zoomIn" | "view.zoomOut") {
            return None;
        }
        Some(command.id)
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
