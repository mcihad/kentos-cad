//! Atlası PDF yap, docs/adr/4820 (plan/map-cad.md MAP-18): the atlas written, parcel 2's owner changed, the atlas
//! written again against the first one's record: the message says how the changed pages look against the previous
//! atlas: none other, every one the same (the owner is never drawn). And its pictures.

use super::atlas_export::Event as A;
use super::atlas_tests::{app_with_atlas_of, previewed_against_the_record, window};
use super::tests::send;
use super::Event;
use crate::app::{App, Message};
use crate::files_testing::scratch;
use kentos_commands::AtlasPageState;
use serde_json::Value;

#[test]
fn the_message_says_the_changed_pages_look_the_same() {
    let mut app = app_with_atlas_of("plain");
    let dir = scratch("atlas-gorunus");
    previewed_against_the_record(&mut app, &dir);
    let changed = window(&app).changes.as_ref().expect("against the record").pages.iter().filter(|p| p.state == AtlasPageState::Changed).count();
    assert!(changed > 1, "{changed}");
    send(&mut app, Event::Atlas(A::SavePicked(Some(dir.join("atlas-2.pdf")))));
    assert!(app.exchange.is_none(), "written and closed");
    let said = crate::files_testing::last_said(&app);
    assert!(said.contains(&format!("görünüşü değişen 0, görünüşü aynı kalan {changed} (yalnız çizmedikleri değişti)")), "{said}");
    let record: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("atlas-2.kaynaklar.json")).expect("its record")).expect("json");
    assert!(record["pages"].as_array().expect("pages").iter().all(|p| p["look"].as_str().is_some_and(|l| l.len() == 64)));
}

/// `.run/gui-evidence/atlas-gorunus-4820/`: the preview against the previous record, then the message once written.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib exchange::atlas_look_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/atlas-gorunus-4820");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = app_with_atlas_of("plain");
        let _ = app.settings.choose(&[("appearance.theme", Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let dir = scratch(&format!("atlas-gorunus-resim-{name}"));
        previewed_against_the_record(&mut app, &dir);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-onizleme-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        send(&mut app, Event::Atlas(A::SavePicked(Some(dir.join("atlas-2.pdf")))));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-yazildi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
