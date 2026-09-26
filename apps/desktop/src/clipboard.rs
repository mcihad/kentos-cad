//! The clipboard's commands on the desktop (docs/adr/0056): Kes (Ctrl+X),
//! Panoya kopyala (Ctrl+C), Yapıştır (Ctrl+V) and Özgün koordinatlara
//! yapıştır (Ctrl+Shift+V), the web's (`apps/web/src/app/commands.ts`) over
//! the session's clipboard (`kentos_interaction::clipboard`).
//!
//! The clipboard is the app's, not the drawing's: it outlives the drawing on
//! screen and never reaches the system clipboard, as on the web. A command
//! the web has switched off (copying with nothing selected, pasting with an
//! empty clipboard) does nothing here either.

use iced::Task;

use kentos_interaction::Level;
use kentos_interaction::clipboard;
use kentos_interaction::paste::{self, Paste};

use crate::app::{App, Message};

/// The web command ids this module runs.
pub const COMMANDS: &[&str] = &["edit.cut", "edit.copy", "edit.paste", "edit.pasteOriginal"];

impl App {
    /// Runs one of [`COMMANDS`].
    pub(crate) fn clipboard_command(&mut self, id: &str) -> Task<Message> {
        match id {
            "edit.cut" => self.cut(),
            "edit.copy" => self.copy(),
            "edit.paste" => return self.paste(),
            "edit.pasteOriginal" => self.paste_in_place(),
            _ => {}
        }
        Task::none()
    }

    /// Whether a clipboard command can run now (the web's `isEnabled`): Kes
    /// and Panoya kopyala with a selection, the pastes with something on the clipboard.
    pub(crate) fn clipboard_available(&self, id: &str) -> bool {
        match id {
            "edit.cut" | "edit.copy" => !self.selection.is_empty(),
            _ => !self.clipboard.is_empty(),
        }
    }

    /// Panoya kopyala: the selection goes to the clipboard.
    fn copy(&mut self) {
        if !self.clipboard_available("edit.copy") {
            return;
        }
        let mut board = std::mem::take(&mut self.clipboard);
        self.with_tool(|_, cx| clipboard::copy(&mut board, cx));
        self.clipboard = board;
    }

    /// Kes: the selection on unlocked layers goes to the clipboard and leaves the drawing.
    fn cut(&mut self) {
        if !self.clipboard_available("edit.cut") {
            return;
        }
        let mut board = std::mem::take(&mut self.clipboard);
        self.with_tool(|_, cx| clipboard::cut(&mut board, cx));
        self.clipboard = board;
    }

    /// Yapıştır: the paste tool with a copy of the clipboard, in place of
    /// whatever ran (the web's `tools.run(new PasteTool(…), 'Yapıştır')`).
    fn paste(&mut self) -> Task<Message> {
        if !self.clipboard_available("edit.paste") {
            return Task::none();
        }
        if self.document.is_none() {
            self.output("Açık çizim yok. Önce bir çizim açın (Ctrl+O).");
            return Task::none();
        }
        self.field = None;
        let tool = Paste::new(self.clipboard.items().to_vec(), self.clipboard.base());
        self.session.run(Box::new(tool));
        self.say(Level::Command, paste::LABEL);
        self.with_tool(|s, cx| s.activate(cx));
        Task::none()
    }

    /// Özgün koordinatlara yapıştır: the clipboard's objects where they were copied.
    fn paste_in_place(&mut self) {
        if !self.clipboard_available("edit.pasteOriginal") {
            return;
        }
        let board = std::mem::take(&mut self.clipboard);
        self.with_tool(|_, cx| clipboard::paste_in_place(&board, cx));
        self.clipboard = board;
    }
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::{Code, Physical};
    use iced::keyboard::{Key, Modifiers};
    use iced::{Point, Rectangle, Size};
    use kentos_contracts::DocumentSnapshotV1;
    use kentos_interaction::{Cursor, Vec2};
    use kentos_ui::widget::command_line::Entry;

    use super::COMMANDS;
    use crate::app::{App, Message};
    use crate::document::Document;
    use crate::keys::KeyPress;
    use crate::viewport::Event;

    /// The selection traces' drawing (fixtures/interaction/v1/objects.kcad).
    fn objects() -> Document {
        let snapshot = DocumentSnapshotV1::from_json(include_str!(
            "../../../fixtures/interaction/v1/objects.kcad"
        ))
        .expect("the drawing reads");
        Document::new(snapshot, None).expect("opens")
    }

    /// The drawing open in an area of 800 × 600 at 0.125 m per pixel around its centre.
    fn app() -> App {
        let (mut app, _) = App::boot(None);
        let _ = app.update(Message::Opened(Some(Ok(Box::new(objects())))));
        let _ = app.update(Message::Viewport(Event::Resized(Rectangle::new(
            Point::ORIGIN,
            Size::new(800.0, 600.0),
        ))));
        app.viewport.camera.center = Vec2::new(487000.0, 4420000.0);
        app.viewport.camera.scale = 8.0;
        app
    }

    /// The area's pixel of a point given east and north of the centre.
    fn at(de: f32, dn: f32) -> Point {
        Point::new(400.0 + de * 8.0, 300.0 - dn * 8.0)
    }

    fn click(app: &mut App, de: f32, dn: f32) {
        let p = at(de, dn);
        let _ = app.update(Message::Viewport(Event::Moved(p)));
        let _ = app.update(Message::Viewport(Event::Pressed(p)));
        let _ = app.update(Message::Viewport(Event::Released(p)));
    }

    /// A chord as the keyboard gives it: Ctrl+C types a control character, no text.
    fn chord(app: &mut App, modifiers: Modifiers, letter: &str, code: Code) {
        let _ = app.update(Message::Key(KeyPress {
            key: Key::Character(letter.into()),
            physical: Physical::Code(code),
            modifiers,
            text: None,
            repeat: false,
        }));
    }

    fn count(app: &App) -> usize {
        app.document.as_ref().map_or(0, Document::entity_count)
    }

    fn selected(app: &App) -> Vec<u32> {
        app.selection.ids().iter().map(|s| s.0).collect()
    }

    fn said(app: &App) -> Option<&str> {
        match app.history.last() {
            Some(Entry::Input(t) | Entry::Output(t) | Entry::Warning(t) | Entry::Error(t)) => {
                Some(t.as_str())
            }
            None => None,
        }
    }

    #[test]
    fn the_keys_cut_copy_and_paste_as_on_the_web() {
        let mut app = app();
        // Nothing selected, nothing put aside: every one of them is off and does nothing.
        assert!(COMMANDS.iter().all(|id| !app.available(id)));
        let before = app.history.len();
        chord(&mut app, Modifiers::CTRL, "c", Code::KeyC);
        chord(&mut app, Modifiers::CTRL, "v", Code::KeyV);
        assert_eq!(app.history.len(), before);
        assert!(!app.session.is_running());

        click(&mut app, -16.0, -12.0);
        assert!(app.available("edit.copy") && app.available("edit.cut"));
        assert!(!app.available("edit.paste") && !app.available("edit.pasteOriginal"));
        chord(&mut app, Modifiers::CTRL, "c", Code::KeyC);
        assert_eq!(said(&app), Some("1 nesne panoya kopyalandı."));
        assert!(app.available("edit.paste") && app.available("edit.pasteOriginal"));

        // Ctrl+V: the paste tool, named in the command line as the web names it.
        chord(&mut app, Modifiers::CTRL, "v", Code::KeyV);
        assert_eq!(app.session.tool_id(), "paste");
        assert_eq!(said(&app), Some("Yapıştır"));
        click(&mut app, -4.0, -18.0);
        assert_eq!(
            (app.session.tool_id(), count(&app), selected(&app)),
            ("select", 8, vec![8])
        );
        assert_eq!(app.session.last(), None, "Yapıştır is not remembered");

        // Ctrl+Shift+V: where the objects were copied.
        chord(
            &mut app,
            Modifiers::CTRL | Modifiers::SHIFT,
            "V",
            Code::KeyV,
        );
        assert_eq!((count(&app), selected(&app)), (9, vec![9]));

        // Ctrl+X: the locked line stays, selected; one step brings the other back.
        let _ = app.update(Message::Modifiers(Modifiers::SHIFT));
        click(&mut app, 10.0, -16.0);
        let _ = app.update(Message::Modifiers(Modifiers::empty()));
        assert_eq!(selected(&app), [9, 6]);
        chord(&mut app, Modifiers::CTRL, "x", Code::KeyX);
        assert_eq!((count(&app), selected(&app)), (8, vec![6]));
        assert_eq!(said(&app), Some("1 nesne panoya kesildi."));
        let _ = app.run("edit.undo");
        assert_eq!(said(&app), Some("Geri alındı: Kes"));
        assert_eq!(count(&app), 9);
    }

    #[test]
    fn the_clipboard_outlives_the_drawing_on_screen() {
        let mut app = app();
        click(&mut app, -16.0, -12.0);
        let _ = app.run("edit.copy");
        // Another drawing on screen: the selection goes, the clipboard stays.
        let _ = app.update(Message::Opened(Some(Ok(Box::new(objects())))));
        assert!(app.selection.is_empty());
        assert!(app.available("edit.pasteOriginal"));
        let _ = app.run("edit.pasteOriginal");
        assert_eq!((count(&app), selected(&app)), (8, vec![8]));
        // With no drawing open the paste tool says so, as the tools do.
        app.document = None;
        let _ = app.run("edit.paste");
        assert!(!app.session.is_running());
        assert_eq!(
            said(&app),
            Some("Açık çizim yok. Önce bir çizim açın (Ctrl+O).")
        );
    }

    #[test]
    fn pan_and_zoom_window_change_the_view_and_repeat_what_came_before() {
        let mut app = app();
        let _ = app.run("tool.line");
        let _ = app.run("tool.cancel");
        let _ = app.run("tool.pan");
        assert_eq!(app.session.cursor(), Cursor::Grab);
        // A drag 64 px right and 32 up: the view follows, 8 m west and 4 m south.
        let _ = app.update(Message::Viewport(Event::Moved(at(0.0, 0.0))));
        let _ = app.update(Message::Viewport(Event::Pressed(at(0.0, 0.0))));
        let _ = app.update(Message::Viewport(Event::Moved(at(8.0, 4.0))));
        let _ = app.update(Message::Viewport(Event::Released(at(8.0, 4.0))));
        let c = app.viewport.camera.center;
        assert_eq!((c.x, c.y), (487000.0 - 8.0, 4420000.0 - 4.0));
        assert_eq!(app.viewport.camera.scale, 8.0);
        // The world point under the pointer followed the view.
        assert_eq!(app.viewport.cursor, Some(Vec2::new(487000.0, 4420000.0)));
        // A right click is no confirm here; Enter repeats the line tool.
        let _ = app.update(Message::Viewport(Event::RightClick(at(8.0, 4.0))));
        assert_eq!(app.session.tool_id(), "pan");
        let _ = app.run("tool.confirm");
        assert_eq!(app.session.tool_id(), "line");
        let _ = app.run("tool.cancel");
        // Pencere yakınlaştır, from the first view again: the box fills the
        // area's width (800 px for 20 m), centred on it.
        app.viewport.camera.center = Vec2::new(487000.0, 4420000.0);
        let _ = app.run("tool.zoomWindow");
        click(&mut app, -24.0, -12.0);
        click(&mut app, -4.0, -4.0);
        assert!(!app.session.is_running());
        let c = app.viewport.camera.center;
        assert_eq!((c.x, c.y), (487000.0 - 14.0, 4420000.0 - 8.0));
        assert_eq!(app.viewport.camera.scale, 40.0);
        // tool.repeat starts it again; with nothing started before, it does nothing.
        let _ = app.run("tool.repeat");
        assert_eq!(app.session.tool_id(), "zoomWindow");
        let (mut fresh, _) = App::boot(None);
        let _ = fresh.run("tool.repeat");
        assert!(!fresh.session.is_running());
    }

    #[test]
    fn zoom_to_the_selection_keeps_96_pixels_round_it() {
        let mut app = app();
        let before = app.viewport.camera;
        let _ = app.run("view.zoomSelection");
        assert_eq!(app.viewport.camera, before, "nothing selected: off");
        assert!(!app.available("view.zoomSelection"));
        click(&mut app, -18.0, 4.0);
        let _ = app.update(Message::Modifiers(Modifiers::SHIFT));
        click(&mut app, -16.0, -12.0);
        let _ = app.update(Message::Modifiers(Modifiers::empty()));
        assert!(app.available("view.zoomSelection"));
        chord(
            &mut app,
            Modifiers::CTRL | Modifiers::SHIFT,
            "F",
            Code::KeyF,
        );
        // The box −24…−8 by −12…14: 16 × 26 m in 608 × 408 px.
        let c = app.viewport.camera.center;
        assert_eq!((c.x, c.y), (487000.0 - 16.0, 4420000.0 + 1.0));
        assert_eq!(app.viewport.camera.scale, 408.0 / 26.0);
    }
}
