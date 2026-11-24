//! Çizimden seç (docs/adr/8006): a window that works on the selection steps aside while objects are picked on the
//! drawing, then opens again as it was. Oda modülleri and Tasarım seçenekleri said "Seçimden tanımla" and "Seçimi
//! ekle" but, modal, let no selection change while open; now the window steps aside temporarily while the objects are
//! selected, and Enter returns to it. The pick is Sahneden seç's (`pick_objects`): a click turns an object over, a box adds; Enter, Space or
//! a short right click keep the selection, Esc gives back the one before.

use kentos_domain::Slot;
use kentos_interaction::Level;

use crate::app::{App, Dialog};

impl App {
    /// Steps the window `dialog` aside for a pick; the current selection is where it starts.
    pub fn pick_for_window(&mut self, dialog: Dialog, label: &str) {
        if self.document.is_none() {
            return;
        }
        let before: Vec<Slot> = self.selection.ids().to_vec();
        self.window_pick = Some((dialog, before));
        self.dialog = None;
        self.field = None;
        self.snap = None;
        self.say(Level::Command, format!("Çizimden seç: {label}. Enter pencereye döner, Esc seçimi geri alır."));
        self.session.run(Box::new(kentos_interaction::pick_objects::PickObjects::selection(label)));
        self.with_tool(|s, cx| s.activate(cx));
    }

    /// The pick ended: Enter kept what is selected, Esc gives back what was; the window opens again. False when no
    /// window was waiting.
    pub fn window_picked(&mut self, keep: bool) -> bool {
        let Some((dialog, before)) = self.window_pick.take() else {
            return false;
        };
        if !keep {
            self.selection.set(before);
        }
        let exchange = matches!(dialog, Dialog::Exchange);
        self.dialog = Some(dialog);
        // An exchange window's area or line is read from what was picked (GIS-036).
        if exchange {
            self.area_after_pick();
        }
        true
    }
}
