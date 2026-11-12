//! GNSS baz ağı (baselines_window.rs, docs/adr/1380): the sample network with a 6 cm blunder on B → C read from its
//! file; the window says the χ² test failed and names B → C the worst baseline; Minimal datum and the variance factor
//! adjust again; robust estimation (docs/adr/4960) weighs B → C down without removing it; nothing is written. Harita
//! and GIS offer it, Mimari and Yol do not. And its pictures.

use crate::map;
use kentos_launch::Module;

use crate::app::{App, Dialog, Message};
use crate::baselines_window::Event as B;
use crate::files_testing::app_with_drawing;

fn sample() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/gnss-network/v1/ankara-ag.txt")
}

fn opened(program: Option<Module>) -> App {
    let mut app = app_with_drawing();
    app.program = program;
    app
}

#[test]
fn the_blunder_is_named_and_nothing_is_written() {
    let mut app = opened(None);
    let revision = app.document.as_ref().expect("a drawing").model.revision();
    let _ = app.update(Message::Run("survey.baselines"));
    assert_eq!(app.dialog, Some(Dialog::Map(map::MapDialogKind::Baselines)));
    app.baselines_file(sample());
    let a = app
        .map
        .baselines
        .answer
        .clone()
        .expect("an answer")
        .expect("adjusted");
    assert_eq!(a.passed, Some(false));
    let worst = a.worst.expect("a worst baseline") as usize;
    assert_eq!(
        (
            a.baselines[worst].from.as_str(),
            a.baselines[worst].to.as_str()
        ),
        ("B", "C")
    );
    // The variance factor 4: m0 halves.
    let m0 = a.m0.expect("m0");
    let _ = app.update(Message::Map(map::Event::Baselines(B::Scale(4.0))));
    let b = app
        .map
        .baselines
        .answer
        .clone()
        .expect("an answer")
        .expect("adjusted");
    assert!((b.m0.expect("m0") - m0 / 2.0).abs() < 1e-12);
    assert_eq!(
        app.document.as_ref().expect("a drawing").model.revision(),
        revision,
        "read only"
    );
}

#[test]
fn robust_estimation_weighs_the_blunder_down() {
    use crate::baselines_window::Robust;
    let mut app = opened(Some(Module::Map));
    let revision = app.document.as_ref().expect("a drawing").model.revision();
    let _ = app.update(Message::Run("survey.baselines"));
    app.baselines_file(sample());
    let _ = app.update(Message::Map(map::Event::Baselines(B::Robust(
        Robust::Huber,
    ))));
    let a = app
        .map
        .baselines
        .answer
        .clone()
        .expect("an answer")
        .expect("adjusted");
    let r = a.robust.clone().expect("robust rounds");
    assert!(r.converged);
    assert_eq!((r.c, r.downweighted), (1.5, 1));
    for b in &a.baselines {
        let f = b.factor.expect("a factor");
        assert_eq!(f < 1.0, (b.from.as_str(), b.to.as_str()) == ("B", "C"), "{} → {}: {f}", b.from, b.to);
    }
    assert_eq!(a.baselines.len(), 6, "nothing removed");
    // The constant given: B → C's factor rises; Danish weighs it down further; off, plain least squares again.
    let huber = a.baselines[1].factor.expect("a factor");
    let _ = app.update(Message::Map(map::Event::Baselines(B::C(2.5))));
    let b = app
        .map
        .baselines
        .answer
        .clone()
        .expect("an answer")
        .expect("adjusted");
    assert_eq!(b.robust.as_ref().map(|r| r.c), Some(2.5));
    assert!(b.baselines[1].factor.expect("a factor") > huber);
    let _ = app.update(Message::Map(map::Event::Baselines(B::Robust(
        Robust::Danish,
    ))));
    let d = app
        .map
        .baselines
        .answer
        .clone()
        .expect("an answer")
        .expect("adjusted");
    assert_eq!(
        d.robust.as_ref().map(|r| r.c),
        Some(2.0),
        "the method's own constant"
    );
    assert!(d.baselines[1].factor.expect("a factor") < huber);
    let _ = app.update(Message::Map(map::Event::Baselines(B::Robust(Robust::Off))));
    let off = app
        .map
        .baselines
        .answer
        .clone()
        .expect("an answer")
        .expect("adjusted");
    assert!(off.robust.is_none() && off.baselines.iter().all(|b| b.factor.is_none()));
    assert_eq!(
        app.document.as_ref().expect("a drawing").model.revision(),
        revision,
        "read only"
    );
}

#[test]
fn harita_and_gis_offer_it_mimari_and_yol_do_not() {
    for (module, yes) in [(Module::Map, true), (Module::Gis, true), (Module::Architecture, false), (Module::Road, false)] {
        assert_eq!(opened(Some(module)).offers("survey.baselines"), yes, "{module:?}");
    }
}

/// `.run/gui-evidence/gnss-ag-1380/` (robust estimation: `dayanikli-*`, docs/adr/4960):
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib baselines_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/gnss-ag-1380");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("survey.baselines"));
        app.baselines_file(sample());
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("kaba-hata-{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes");
        println!("{}", file.display());
        let _ = app.update(Message::Map(map::Event::Baselines(B::Robust(
            crate::baselines_window::Robust::Huber,
        ))));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("dayanikli-{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes");
        println!("{}", file.display());
    }
}
