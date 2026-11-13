//! Öne getir and Arkaya gönder say their limits: the order is within a layer (an object of another layer where the
//! selection stands is said), and a mask never covers words (UR-059, UR-060).

use serde_json::json;

use crate::app::{App, Message};
use crate::tools_scenes::{Objects, open};

/// A framed mask (1) on Çizim, a line across it on Yol (2), a text inside it on Çizim (3), a line far away (4).
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let mut o = Objects::new();
    let m = o.path("cizim", &[[6.0, 4.0], [14.0, 4.0], [14.0, 12.0], [6.0, 12.0]], true);
    o.fields(m, json!({ "wipeout": { "frame": true } }));
    o.line("yol", [0.0, 8.0], [20.0, 8.0]);
    o.text("cizim", [8.0, 9.0], "TEST", 1.0, 0.0);
    o.line("cizim", [40.0, 40.0], [50.0, 40.0]);
    open(&mut app, o);
    app
}

fn said(app: &App, n: usize) -> String {
    app.log.lines().rev().take(n).map(|l| l.text.clone()).collect::<Vec<_>>().join(" | ")
}

/// UR-059: the mask brought to the front says the Yol line over it is drawn by the layers' order.
#[test]
fn bringing_a_mask_forward_says_the_other_layers_object() {
    let mut app = app();
    app.selection.set([kentos_domain::Slot(1)]);
    let _ = app.update(Message::Run("edit.bringToFront"));
    let words = said(&app, 3);
    assert!(words.contains("kendi katmanı içinde") && words.contains("1 nesne başka katmanda"), "{words}");
    // An object alone in its place says nothing more.
    app.selection.set([kentos_domain::Slot(4)]);
    let _ = app.update(Message::Run("edit.bringToFront"));
    assert!(!said(&app, 1).contains("katman"), "{}", said(&app, 1));
}

/// UR-060: the text sent behind the mask says the mask does not cover words.
#[test]
fn sending_a_text_back_says_the_mask_does_not_cover_it() {
    let mut app = app();
    app.selection.set([kentos_domain::Slot(3)]);
    let _ = app.update(Message::Run("edit.sendToBack"));
    let words = said(&app, 3);
    assert!(words.contains("Maske yazıları örtmez"), "{words}");
    app.selection.set([kentos_domain::Slot(4)]);
    let _ = app.update(Message::Run("edit.sendToBack"));
    assert!(!said(&app, 1).contains("Maske"), "{}", said(&app, 1));
}
