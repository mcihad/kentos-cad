//! Kayma sınırı and the suggested profile (map_work_limit.rs, map_work_panel.rs; plan/map-cad.md MAP-01,
//! docs/adr/5280): the design, without a profile, is offered the zoning profile and takes it from the panel; the design
//! approved and its line moved 0.5 m: the shift table sets it against the table's sayısal 1/1000 (beyond, both ends) and
//! the drawing's ±0.2 mm at 1/5000 (within); a cell the table leaves empty is said and the table goes on without a
//! limit. And the pictures.

use crate::app::App;
use crate::map_work_limit::LimitEvent as L;
use crate::map_work_panel::Event as P;
use crate::map_work_profile_tests::{approved_then_moved_and_removed, picture, staged};
use crate::map_work_stamps_tests::{opened, send, tree};

fn limit(app: &mut App, events: Vec<L>) {
    for e in events {
        send(app, P::Limit(e));
    }
}

#[test]
fn the_design_takes_its_suggested_profile() {
    let mut app = opened();
    let row = tree(&app).items[3].clone();
    let s = row.suggested.first().cloned().expect("a suggestion");
    assert_eq!(s.name, "imar-plani");
    send(&mut app, P::Suggest("is-4".into()));
    let row = tree(&app).items[3].clone();
    let profile = row.profile.expect("a profile");
    assert_eq!(profile.name, "imar-plani");
    assert_eq!(profile.classes.len(), s.classes.len());
    assert!(row.suggested.is_empty(), "an item with a profile is offered none");
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(said.iter().any(|l| l.contains("profilinin 4 sınıfı öğeye verildi")), "{said:?}");
}

#[test]
fn the_shift_table_sets_each_shift_against_the_limit() {
    let mut app = opened();
    let _ = approved_then_moved_and_removed(&mut app, 0.3, 0.4);
    send(&mut app, P::Changes("is-4".into()));
    assert_eq!(app.map_work.shifts[0].len(), 6, "no limit: no column");
    limit(&mut app, vec![L::Basis("method"), L::Method("sayisal".into()), L::Scale("1000".into())]);
    assert_eq!(app.map_work.shifts[0][1], "Dışında (2 köşe)");
    assert_eq!(tree(&app).limit.expect("the limit").metres, 0.09);
    assert_eq!(tree(&app).items[3].changes.clone().expect("changes").beyond, Some(1));
    limit(&mut app, vec![L::Basis("drawing"), L::Scale("5000".into())]);
    assert_eq!(app.map_work.shifts[0][1], "İçinde");
    // A cell the table leaves empty: said under the row, the query without a limit.
    limit(&mut app, vec![L::Basis("method"), L::Method("fotogrametrik".into()), L::Scale("1000".into())]);
    assert!(matches!(app.map_work.limit.used(), Some(Err(_))));
    assert!(tree(&app).limit.is_none());
    assert_eq!(app.map_work.shifts[0].len(), 6);
}

/// `.run/gui-evidence/kayma-siniri-5280/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib map_work_limit_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kayma-siniri-5280");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = staged(theme, size);
        send(&mut app, P::ProfileItem("is-4".into()));
        picture(&mut app, size, &out.join(format!("1-onerilen-profil-{name}.png")));
        let mut app = staged(theme, size);
        // The table under the tree wants the dock's height.
        app.docks.update(kentos_ui::widget::docking::Event::Shared(kentos_ui::widget::docking::Side::Right, vec![0.9, 0.1]));
        let _ = approved_then_moved_and_removed(&mut app, 0.06, 0.08);
        let _ = app.update(crate::app::Message::Run("view.zoomExtents"));
        send(&mut app, P::Changes("is-4".into()));
        limit(&mut app, vec![L::Basis("method"), L::Method("sayisal".into()), L::Scale("1000".into())]);
        picture(&mut app, size, &out.join(format!("2-kayma-siniri-{name}.png")));
        limit(&mut app, vec![L::Method("fotogrametrik".into())]);
        picture(&mut app, size, &out.join(format!("3-bos-hucre-{name}.png")));
    }
}
