//! Merdiven from its height (docs/adr/0704): one click and Enter draw a stair whose risers come from the height and
//! the highest riser, its last flight ending at its last riser however far the second point is; one undo takes it
//! back. Tıklanan yerde keeps the clicked point; the preview is the command's plan and writes nothing. Rampa ends where
//! its slope says; Korkuluk goes with both (docs/adr/0705).

use crate::common::{Bench, E, N};
use kentos_commands::{DrawingUnit, Entity, Workspace};

fn bench() -> Bench {
    let mut b = Bench::new("stair");
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b.start("stair");
    b
}

fn stair(b: &Bench) -> kentos_commands::StairFields {
    match b.newest() {
        Entity::Stair(s) => s.stair.clone(),
        other => panic!("a stair, not {other:?}"),
    }
}

#[test]
fn a_click_and_enter_draw_the_height_s_risers_ending_at_the_last() {
    let mut b = bench();
    b.click(0., 0.);
    b.move_to(0., 9.);
    let before = b.doc.to_snapshot_v2();
    let preview = b.session.preview(&Default::default()).expect("the plan drawn");
    assert!(preview.strokes.len() > 18, "the risers drawn: {}", preview.strokes.len());
    assert_eq!(b.doc.to_snapshot_v2(), before, "the preview writes nothing");
    b.click(0., 9.);
    b.confirm();
    let s = stair(&b);
    assert_eq!(s.risers, 18);
    assert!((s.riser_height - 3.0 / 18.0).abs() < 1e-12);
    assert_eq!(s.cut, Some(6), "1.10 m over 0.167 m risers: the sixth");
    let end = s.pts.last().copied().expect("its end");
    assert!((end.x - E).abs() < 1e-9 && (end.y - (N + 17.0 * 0.28)).abs() < 1e-9, "{end:?}");
    assert_eq!(b.doc.undo().as_deref(), Some("Merdiven"));
    assert!(!b.doc.entities().any(|e| matches!(e, Entity::Stair(_))));
}

#[test]
fn a_given_count_and_the_clicked_end_are_kept() {
    let mut b = bench();
    assert!(b.run(|session, cx| session.configure("S", "20", cx)));
    assert!(b.run(|session, cx| session.configure("U", "Tıklanan yerde biter", cx)));
    b.click(0., 0.);
    b.click(6., 0.);
    b.confirm();
    let s = stair(&b);
    assert_eq!(s.risers, 20);
    assert!((s.riser_height - 0.15).abs() < 1e-12);
    let end = s.pts.last().copied().expect("its end");
    assert!((end.x - (E + 6.0)).abs() < 1e-9, "the clicked end kept: {end:?}");
}

#[test]
fn risers_ending_before_the_last_flight_say_why_and_write_nothing() {
    let mut b = bench();
    b.click(0., 0.);
    b.click(8., 0.);
    b.move_to(8., 3.);
    let tag = b.session.preview(&Default::default()).and_then(|p| p.tag).expect("the tag");
    assert!(tag.lines.iter().any(|l| l.contains("son kola varmadan")), "{:?}", tag.lines);
    b.click(8., 3.);
    b.confirm();
    assert!(!b.doc.entities().any(|e| matches!(e, Entity::Stair(_))));
}

#[test]
fn a_ramp_ends_where_its_slope_says_and_railings_go_with_it() {
    let mut b = Bench::new("ramp");
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b.start("ramp");
    assert!(b.run(|session, cx| session.configure("O", "İki yan", cx)));
    b.click(0., 0.);
    b.click(2., 0.);
    b.confirm();
    let r = match b.newest() {
        Entity::Ramp(r) => r.ramp.clone(),
        other => panic!("a ramp, not {other:?}"),
    };
    let end = r.pts.last().copied().expect("its end");
    assert!((end.x - (E + 0.5 / 0.06)).abs() < 1e-9, "0.50 m at 6 %: 8.333 m, {end:?}");
    assert_eq!(r.railings, Some(kentos_commands::Railings::Both));
    assert_eq!(r.slope, Some(6.0));
}

#[test]
fn a_stair_s_railings_are_chosen_in_its_settings() {
    let mut b = bench();
    assert!(b.run(|session, cx| session.configure("O", "Sol", cx)));
    b.click(0., 0.);
    b.click(0., 6.);
    b.confirm();
    assert_eq!(stair(&b).railings, Some(kentos_commands::Railings::Left));
}

/// Balansman (docs/adr/8045 §1): A then a count balances so many treads either side of the L's turn instead of a
/// landing; the design says the narrowest tread at the inner side against PAİY 31/2-c's 0.10 m. A two-point (straight)
/// stair gets none; A "yok" takes it away.
#[test]
fn balansman_balances_the_turn_and_says_its_narrowest_tread() {
    let mut b = bench();
    assert!(b.type_text("A"));
    assert!(b.type_text("5"));
    b.click(0., 0.);
    b.click(2.5, 0.);
    b.move_to(2.5, 3.0);
    let words = b
        .session
        .settings(&Default::default())
        .into_iter()
        .find(|s| s.key == "T")
        .map(|s| s.value)
        .unwrap_or_default();
    assert!(
        words.contains("en dar iç basamak") && words.contains("PAİY 31/2-c'nin 0,10 m'sinden dar"),
        "1.20 m with 5 winders is too narrow at the corner: {words}"
    );
    b.click(2.5, 3.0);
    b.confirm();
    let s = stair(&b);
    assert_eq!((s.winders, s.landing), (Some(5), None), "{s:?}");
    // Seven keep 0.10 m (the fewest for 1.20 m at 0.28 m treads).
    assert!(b.type_text("A"));
    assert!(b.type_text("7"));
    b.click(5., 0.);
    b.click(7.5, 0.);
    b.move_to(7.5, 3.0);
    let words = b
        .session
        .settings(&Default::default())
        .into_iter()
        .find(|s| s.key == "T")
        .map(|s| s.value)
        .unwrap_or_default();
    assert!(words.contains("(PAİY 31/2-c: en az 0,10 m)"), "{words}");
    b.click(7.5, 3.0);
    b.confirm();
    assert_eq!(stair(&b).winders, Some(7));
    // A straight stair has no turn: none written; A emptied: a landing again.
    b.click(10., 0.);
    b.click(10., 4.);
    b.confirm();
    assert_eq!(stair(&b).winders, None);
    assert!(b.type_text("A"));
    assert!(b.type_text("yok"));
    b.click(13., 0.);
    b.click(15.5, 0.);
    b.click(15.5, 3.0);
    b.confirm();
    let s = stair(&b);
    assert_eq!((s.winders, s.landing), (None, None), "{s:?}");
}

/// UR-018: the second point given, the pointer still on it: no error of a third point not given yet, the plan the
/// straight flight Enter draws; a ramp the same. Moved on to a turn, the L's plan.
#[test]
fn the_pointer_on_the_point_just_given_is_no_third_point() {
    for tool in ["stair", "ramp"] {
        let mut b = bench();
        b.start(tool);
        b.click(0., 0.);
        b.move_to(0., 9.);
        b.click(0., 9.);
        let preview = b.session.preview(&Default::default()).expect("the plan drawn");
        let said = preview.tag.as_ref().map(|t| t.lines.join(" ")).unwrap_or_default();
        assert!(!said.contains("noktası aynı"), "{tool}: {said}");
        assert_ne!(preview.tag_tone, kentos_interaction::Tone::Danger, "{tool}: {said}");
        b.confirm();
        assert!(
            matches!(b.newest(), Entity::Stair(_) | Entity::Ramp(_)),
            "{tool}: Enter draws it"
        );
    }
}

/// UR-057: a refused value is told by its own setting's rule (a ramp's slope never hears of risers or cuts) and the
/// value before stays.
#[test]
fn a_refused_setting_says_its_own_rule() {
    let mut b = Bench::new("ramp");
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b.start("ramp");
    assert!(!b.run(|session, cx| session.configure("M", "0", cx)));
    let said = b.last_text().unwrap_or_default().to_owned();
    assert!(said.contains("Eğim (%) sıfırdan büyük") && said.contains("Eğim %6 kaldı"), "{said}");
    assert!(!said.contains("Rıht") && !said.contains("Kesme"), "{said}");
    assert!(!b.run(|session, cx| session.configure("E", "-1", cx)));
    let said = b.last_text().unwrap_or_default().to_owned();
    assert!(said.starts_with("Değer uygulanmadı: Genişlik sıfırdan büyük"), "{said}");
    // The stair's riser count and cut height say theirs.
    let mut b = bench();
    assert!(!b.run(|session, cx| session.configure("S", "2.5", cx)));
    assert!(b.last_text().unwrap_or_default().starts_with("Değer uygulanmadı: Rıht sayısı 0 (hesaplanır) ile"));
    assert!(!b.run(|session, cx| session.configure("K", "abc", cx)));
    assert!(b.last_text().unwrap_or_default().starts_with("Değer uygulanmadı: Kesme yüksekliği"));
}

/// UR-030: a section mark's second point given, the pointer still on it: no error of two points the same; Enter writes
/// the mark, said as a section's with its letter.
#[test]
fn a_section_marks_resting_pointer_is_no_third_point() {
    let mut b = bench();
    b.start(kentos_interaction::viewmark_tools::SECTION_ID);
    b.click(0., 0.);
    b.move_to(10., 0.);
    b.click(10., 0.);
    let preview = b.session.preview(&Default::default()).expect("the mark drawn");
    let said = preview.tag.as_ref().map(|t| t.lines.join(" ")).unwrap_or_default();
    assert!(!said.contains("aynı olamaz"), "{said}");
    assert_ne!(preview.tag_tone, kentos_interaction::Tone::Danger, "{said}");
    b.confirm();
    assert!(matches!(b.newest(), Entity::ViewMark(_)));
    assert_eq!(
        b.last_text(),
        Some("Kesit işareti A yazıldı; hedefi yeniden adlandırılınca yazısı da değişir.")
    );
}
