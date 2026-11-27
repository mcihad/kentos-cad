//! The CBS review's ninth item and its icon table (docs/adr/9337): Kaydırmalı karşılaştır off with three views and why;
//! Görünüm › Paneller's buttons named for the groups they show (Sağ panel, Alt panel); Görünüş's launcher with the
//! settings' icon and Görünüm ayarları; Asistan's box as tall as its placeholder in a narrow panel.

use crate::app::{App, Message};
use crate::catalog::catalog;

#[test]
fn the_panel_buttons_are_named_for_their_groups() {
    for (id, name) in [("view.rightPanel", "Sağ panel"), ("view.bottomPanel", "Alt panel")] {
        let c = catalog().get(id).expect(id);
        assert_eq!((c.title, c.short), (name, name));
        assert!(c.description.contains("panel grubunu"), "{}", c.description);
    }
    assert!(catalog().get("view.swipe").is_some_and(|c| c.description.contains("Tek görünümdeyken basınca")));
}

/// A switch's row in a ▾ list has its icon beside its tick (Tam ekran, Çizgi kalınlığını göster, Mevcut durum); Tam
/// ekran on says how to leave it.
#[test]
fn a_switch_row_has_its_icon_beside_its_tick() {
    let ids = ["view.fullscreen", "view.lineWeights", "scenario.base"];
    let menu = format!("{:?}", crate::view::menu_of(&ids, &[Some(true), Some(false), Some(false)], &[None, None, None], None));
    assert_eq!(menu.matches("icon: Some(").count(), 3, "{menu}");
    assert!(menu.contains("Tam ekrandan çık"), "{menu}");
    assert_eq!(menu.matches("checked: Some(").count(), 3, "{menu}");
}

/// Three views: Karşılaştır's button off.
fn three_views() -> App {
    let mut app = crate::files_testing::app_with_drawing();
    let _ = app.update(Message::Run("view.split"));
    let _ = app.update(Message::Run("view.splitAdd"));
    let _ = app.update(Message::RibbonTab("view"));
    app
}

#[test]
fn karsilastir_is_off_with_three_views_and_on_with_one_or_two() {
    let mut app = crate::files_testing::app_with_drawing();
    assert!(app.available("view.swipe"), "one view: it splits in two first");
    let _ = app.update(Message::Run("view.split"));
    assert!(app.available("view.swipe"), "two views");
    let app = three_views();
    assert!(!app.available("view.swipe"));
    assert_eq!(
        app.why_disabled("view.swipe").as_deref(),
        Some("Kaydırmalı karşılaştırma için iki görünüm kullanın: fazla görünümleri üstlerindeki × ile kapatın.")
    );
}

/// The pictures (`.run/gui-evidence/kullanim-9337/`): Görünüm with three views (Karşılaştır off), Sağ panel and Alt
/// panel, Görünüş's launcher.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib review_panels_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    crate::files_testing::usage_shots("9337", "01-gorunum-paneller", three_views);
}
