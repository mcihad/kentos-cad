//! GNSS arşivi's baselines, known points and adjustments (map_survey_network_rows.rs, plan/map-cad.md MAP-03,
//! docs/adr/2920) on the shared cases' drawing of `map.survey.network`: the network accepted from Ağ dengelemesi is a
//! row under Dengelemeler that selects the three points it made; a baseline left out from its row with the toolbar's
//! reason turns the adjustment out of date and Geri al takes it back; a known point leaves and comes back the same way;
//! the adjustment's button opens Ağ dengelemesi. And the pictures.

use kentos_launch::Module;
use kentos_ui::widget::Status;
use serde_json::Value;

use crate::app::{App, Dialog, Message, Panel};
use crate::map_survey_network_rows::{adjustment_state, made_by};
use crate::map_survey_panel::Event as P;
use crate::survey_network_window::{Class, Event as N};

fn opened() -> App {
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/map.survey.network.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The network adjusted in class C2 and accepted from Ağ dengelemesi, the window closed.
fn accepted() -> App {
    let mut app = opened();
    let _ = app.update(Message::Run("map.survey.adjustment"));
    let _ = app.update(Message::SurveyNetwork(N::Class(Class::C2)));
    let _ = app.update(Message::SurveyNetwork(N::Accept));
    app.dialog = None;
    app
}

fn send(app: &mut App, e: P) {
    let _ = app.update(Message::MapSurvey(e));
}

fn tree(app: &App) -> kentos_commands::MapSurveyTree {
    app.map_survey_read().expect("a drawing").expect("a map project")
}

#[test]
fn the_accepted_adjustment_selects_its_points_and_goes_out_of_date_with_a_baseline() {
    let mut app = accepted();
    let t = tree(&app);
    assert_eq!((t.baselines.len(), t.known.len(), t.adjustments.len()), (8, 2, 1));
    let a = &t.adjustments[0];
    let (tone, words, out) = adjustment_state(&t, a);
    assert_eq!((tone, words, out.len()), (Status::Info, "Kabul edildi", 0));
    // Its row selects the three points it made.
    let made = made_by(&t, a);
    assert_eq!(made.len(), 3);
    send(&mut app, P::Objects(made));
    assert_eq!(app.selection.len(), 3);
    // A baseline left out without a reason is not; with one it is, and the adjustment is out of date.
    send(&mut app, P::Exclude("bz-8".into()));
    assert!(tree(&app).baselines.iter().find(|b| b.id == "bz-8").is_some_and(|b| b.excluded.is_none()));
    send(&mut app, P::Text("Çok bağlantılı, şüpheli".into()));
    send(&mut app, P::Exclude("bz-8".into()));
    let t = tree(&app);
    assert_eq!(t.baselines.iter().find(|b| b.id == "bz-8").and_then(|b| b.excluded.clone()).as_deref(), Some("Çok bağlantılı, şüpheli"));
    let (tone, words, out) = adjustment_state(&t, &t.adjustments[0]);
    assert_eq!((tone, words, out), (Status::Warning, "Güncel değil", vec!["bz-8"]));
    // Geri al: accepted again.
    send(&mut app, P::Include("bz-8".into()));
    let t = tree(&app);
    assert_eq!(adjustment_state(&t, &t.adjustments[0]).1, "Kabul edildi");
    // A known point leaves and comes back the same way.
    let known = t.known[1].id.clone();
    send(&mut app, P::Text("Röper bozulmuş".into()));
    send(&mut app, P::Exclude(known.clone()));
    let t = tree(&app);
    assert!(t.known.iter().find(|k| k.id == known).is_some_and(|k| k.excluded.is_some()));
    assert_eq!(adjustment_state(&t, &t.adjustments[0]).2, vec![known.as_str()]);
    send(&mut app, P::Include(known.clone()));
    assert!(tree(&app).known.iter().all(|k| k.excluded.is_none()));
    // The adjustment's button opens Ağ dengelemesi.
    send(&mut app, P::Network);
    assert_eq!(app.dialog, Some(Dialog::SurveyNetwork));
}

/// The archive panel with Bazlar, Bilinen noktalar and Dengelemeler, a baseline left out (the adjustment out of
/// date); light wide and dark narrow, `.run/gui-evidence/arsivde-bazlar-2920/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib map_survey_network_rows_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/arsivde-bazlar-2920");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = accepted();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.show_map_survey();
        app.docks.update(kentos_ui::widget::docking::Event::Selected(Panel::MapSurvey));
        app.docks.update(kentos_ui::widget::docking::Event::Shared(kentos_ui::widget::docking::Side::Right, vec![0.72, 0.28]));
        send(&mut app, P::Text("Çok bağlantılı, şüpheli".into()));
        send(&mut app, P::Exclude("bz-8".into()));
        // The campaigns', the marks' and the uses' branches folded, the baselines' opened: the network's rows in sight.
        for key in ["campaigns", "marks", "uses", "baselines"] {
            send(&mut app, P::Fold(key.into()));
        }
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("arsiv-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// docs/adr/3320: a baseline's row draws it on the map between its ends' accepted points, with its id and length,
/// besides selecting them; the drawing's points are its ends (a point moved, the line follows); İşareti kaldır takes
/// it away, and is offered only while something is marked. Docs/adr/4120: an end without an accepted point is its
/// known point's own place in the project, a triangle, said; with that known point left out the baseline is not drawn.
#[test]
fn a_baseline_s_row_draws_it_between_its_ends() {
    let mut app = accepted();
    let t = tree(&app);
    let accepted_end = |m: &str| !crate::map_survey_network_rows::objects_of(&t, &[m]).is_empty();
    // A baseline from a known point without an accepted point of its own: drawn from the known point's own place.
    let lone = t.baselines.iter().find(|b| !(accepted_end(&b.from) && accepted_end(&b.to))).cloned().expect("one");
    let known_end = if accepted_end(&lone.from) { lone.to.clone() } else { lone.from.clone() };
    let own = t.known_places.iter().find(|p| p.mark == known_end).expect("its known point's place");
    send(&mut app, P::Baseline(lone.id.clone()));
    let marks = app.baseline_marks().expect("drawn");
    let line = &marks.lines[0];
    let (at, flag) = if lone.from == known_end { (line.from, line.known.clone()) } else { (line.to, line.known.clone()) };
    assert_eq!(at, [own.x.expect("placed"), own.y.expect("placed")]);
    assert_eq!(flag.iter().flatten().collect::<Vec<_>>(), [&known_end], "one end at the known point's place, named");
    let said = app.log.lines().last().map(|l| l.text.clone()).unwrap_or_default();
    assert_eq!(
        said,
        format!(
            "{}: {known_end} ucu kabul noktası olmadığından bilinen noktanın projedeki kendi yerinden çizildi (üçgen).",
            lone.id
        )
    );
    // Its known point left out: not drawn, and the window says why.
    let k = t.known.iter().find(|k| k.mark == known_end).expect("known").id.clone();
    send(&mut app, P::Text("Röper bozulmuş".into()));
    send(&mut app, P::Exclude(k.clone()));
    send(&mut app, P::Baseline(lone.id.clone()));
    assert!(app.baseline_marks().is_none());
    let said = app.log.lines().last().map(|l| l.text.clone()).unwrap_or_default();
    assert!(said.starts_with(&format!("{} çizilemedi", lone.id)), "{said}");
    send(&mut app, P::Include(k));
    let t = tree(&app);
    // One between two accepted points is.
    let b = t.baselines.iter().find(|b| accepted_end(&b.from) && accepted_end(&b.to)).cloned().expect("one");
    send(&mut app, P::Baseline(b.id.clone()));
    assert_eq!(app.selection.len(), 2, "its two ends selected");
    let marks = app.baseline_marks().expect("drawn");
    let mark = &marks.lines[0];
    assert_eq!((marks.lines.len(), mark.words.clone()), (1, format!("{} · {:.3} m", b.id, b.length)));
    // The ends are the accepted points' objects where the drawing holds them.
    let place = |mark_name: &str| {
        let uid = crate::map_survey_network_rows::objects_of(&t, &[mark_name]).into_iter().next().expect("accepted");
        let doc = app.document.as_ref().expect("open");
        let slot = doc.model.slot_of(kentos_domain::Uuid::parse_str(&uid).expect("a uid")).expect("drawn");
        match doc.model.get(slot) {
            Some(kentos_commands::Entity::Point(p)) => [p.p.x, p.p.y],
            _ => panic!("a point"),
        }
    };
    assert_eq!((mark.from, mark.to), (place(&b.from), place(&b.to)));
    assert!(app.available("data.unmark"), "something is marked");
    let _ = app.update(Message::Run("data.unmark"));
    assert!(app.baseline_marks().is_none());
}

/// The baseline drawn from its row, light wide and dark narrow, `.run/gui-evidence/baz-haritada-3320/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib map_survey_network_rows_tests::baseline_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn baseline_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/baz-haritada-3320");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = accepted();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.show_map_survey();
        app.docks.update(kentos_ui::widget::docking::Event::Selected(Panel::MapSurvey));
        app.docks.update(kentos_ui::widget::docking::Event::Shared(kentos_ui::widget::docking::Side::Right, vec![0.72, 0.28]));
        for key in ["campaigns", "marks", "uses", "baselines"] {
            send(&mut app, P::Fold(key.into()));
        }
        let _ = app.update(Message::Run("view.zoomExtents"));
        let t = tree(&app);
        let accepted_end = |m: &str| !crate::map_survey_network_rows::objects_of(&t, &[m]).is_empty();
        let b = t.baselines.iter().find(|b| accepted_end(&b.from) && accepted_end(&b.to)).expect("one").id.clone();
        send(&mut app, P::Baseline(b));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("baz-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// docs/adr/3720: Ağı göster draws every baseline whose two ends can be placed at once, their accepted ends selected;
/// a baseline left out is drawn as left out (its own colour); docs/adr/4120: an end without an accepted point at its
/// known point's own place, named; with BN2 left out its baselines are counted, not drawn; İşareti kaldır takes them all
/// away.
#[test]
fn ağı_göster_draws_every_baseline_at_once() {
    let mut app = accepted();
    // BN2 left out: its baselines have an end with neither an accepted point nor a known point's place.
    send(&mut app, P::Text("Röper bozulmuş".into()));
    let bn2 = tree(&app).known.iter().find(|k| k.mark == "BN2").expect("BN2").id.clone();
    send(&mut app, P::Exclude(bn2));
    let t = tree(&app);
    let accepted_end = |m: &str| !crate::map_survey_network_rows::objects_of(&t, &[m]).is_empty();
    let placed = |m: &str| accepted_end(m) || m == "BN1";
    let drawable: Vec<String> =
        t.baselines.iter().filter(|b| placed(&b.from) && placed(&b.to)).map(|b| b.id.clone()).collect();
    let missing = t.baselines.len() - drawable.len();
    assert!(drawable.len() > 2 && missing > 0, "{} drawable, {missing} not", drawable.len());
    // One left out first, with a reason.
    send(&mut app, P::Text("Çok bağlantılı, şüpheli".into()));
    let out = drawable[0].clone();
    send(&mut app, P::Exclude(out.clone()));
    send(&mut app, P::ShowNetwork);
    let marks = app.baseline_marks().expect("drawn");
    assert_eq!(marks.lines.len(), drawable.len());
    let words: Vec<&str> = marks.lines.iter().map(|l| l.words.as_str()).collect();
    assert!(drawable.iter().all(|id| words.iter().any(|w| w.starts_with(&format!("{id} ·")))), "{words:?}");
    let left_out: Vec<bool> = marks.lines.iter().map(|l| l.excluded).collect();
    assert_eq!(left_out.iter().filter(|e| **e).count(), 1, "{left_out:?}");
    assert!(marks.lines.iter().find(|l| l.words.starts_with(&format!("{out} ·"))).is_some_and(|l| l.excluded));
    let said = app.log.lines().last().map(|l| l.text.clone()).unwrap_or_default();
    assert_eq!(
        said,
        format!(
            "Ağ: {} baz çizildi; 1'i dışlandı, kırmızı noktalı. BN1 bilinen noktanın projedeki kendi yerinde (üçgen). {missing} baz çizilemedi: uçlarından birinin ne kabul noktası ne de projede yeri olan bilinen noktası var.",
            drawable.len()
        )
    );
    assert!(app.selection.len() > 2, "the ends selected");
    let _ = app.update(Message::Run("data.unmark"));
    assert!(app.baseline_marks().is_none());
}

/// The network drawn by Ağı göster, one baseline left out, light wide and dark narrow,
/// `.run/gui-evidence/ag-haritada-3720/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib map_survey_network_rows_tests::network_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn network_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/ag-haritada-3720");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = accepted();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.show_map_survey();
        app.docks.update(kentos_ui::widget::docking::Event::Selected(Panel::MapSurvey));
        app.docks.update(kentos_ui::widget::docking::Event::Shared(kentos_ui::widget::docking::Side::Right, vec![0.72, 0.28]));
        send(&mut app, P::Text("Çok bağlantılı, şüpheli".into()));
        send(&mut app, P::Exclude("bz-8".into()));
        for key in ["campaigns", "marks", "uses", "baselines"] {
            send(&mut app, P::Fold(key.into()));
        }
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        // Laid out first, so the network's fit knows the map's own size beside the dock.
        snapshot.settle(&mut app, App::view, &mut update);
        send(&mut app, P::ShowNetwork);
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("ag-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// docs/adr/4120: Bilinen noktalar' rows say each known point's own place in the project (the network's BN1 and BN2
/// at the campaign's 2005.0, by the chain alone), its epoch's move and its mark's accepted point's distance; one without
/// a place says why.
#[test]
fn a_known_point_s_row_says_its_own_place() {
    use crate::map_survey_network_rows::place_words;
    use kentos_commands::{KnownMoved, KnownMovedBy, SurveyKnownPlace};
    let app = accepted();
    let t = tree(&app);
    assert_eq!(t.known_places.len(), 2);
    for p in &t.known_places {
        let words = place_words(p);
        assert_eq!(words.len(), 1, "{words:?}");
        assert!(words[0].starts_with("Projede kendi yeri Y ") && words[0].ends_with("(2005,00 epokunda, zincir tam)."), "{words:?}");
    }
    let moved = SurveyKnownPlace {
        id: "bn-3".into(),
        mark: "N1".into(),
        x: Some(544_604.5248),
        y: Some(4_404_866.5063),
        z: Some(845.246),
        chain: Some("limited".into()),
        epoch: Some(2005.0),
        moved: Some(KnownMoved {
            id: "bn-3".into(),
            mark: "N1".into(),
            by: KnownMovedBy::Velocity,
            from: 2026.84,
            years: -21.84,
            shift: [0.5167, -0.1698, 0.0426],
        }),
        point: Some("kn-1".into()),
        dx: Some(0.006),
        dy: Some(-0.004),
        dz: Some(0.012),
        why: None,
    };
    assert_eq!(
        place_words(&moved),
        [
            "Projede kendi yeri Y 544604.525, X 4404866.506, H 845.246 (2005,00 epokunda, zincir sınırlı).",
            "Epok farkı 2026,84 → 2005,00, kendi hızıyla: doğu +0.517, kuzey -0.170, yukarı +0.043 m kaydı.",
            "kn-1 kabul noktası bu yerden 7.2 mm uzakta, kot farkı +12.0 mm.",
        ]
    );
    let lost = SurveyKnownPlace { x: None, y: None, z: None, moved: None, dx: None, dy: None, dz: None, why: Some("Bir neden.".into()), ..moved };
    assert_eq!(place_words(&lost), ["Projede yeri yok: Bir neden."]);
}

/// The network drawn by Ağı göster with BN1 and BN2 at their own places (triangles), light wide and dark narrow,
/// `.run/gui-evidence/bilinen-yeri-4120/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib map_survey_network_rows_tests::known_place_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn known_place_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/bilinen-yeri-4120");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = accepted();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.show_map_survey();
        app.docks.update(kentos_ui::widget::docking::Event::Selected(Panel::MapSurvey));
        app.docks.update(kentos_ui::widget::docking::Event::Shared(kentos_ui::widget::docking::Side::Right, vec![0.72, 0.28]));
        for key in ["campaigns", "marks", "uses", "baselines"] {
            send(&mut app, P::Fold(key.into()));
        }
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        // Ağı göster fits every end, the known points' own places too.
        send(&mut app, P::ShowNetwork);
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("ag-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
