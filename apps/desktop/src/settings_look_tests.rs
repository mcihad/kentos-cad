//! Uygulama ayarları → Görünüm's pickers (settings_look.rs): they write the
//! shared schema's values, and a click on a theme card, an accent, a
//! typeface or a text size step puts its value in the window's draft.
//! `screens` draws the section for the owner, by hand (`.run/shots/gorunum-*`):
//!
//! `cargo test -p kentos-desktop settings_look_tests::screens -- --ignored --nocapture`

use std::path::{Path, PathBuf};

use iced::{Point, Size};
use kentos_ui::snapshot::{Input, Snapshot};
use kentos_ui::theme::Mode;
use kentos_ui::theme::typography::Family;
use serde_json::Value;

use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, find_texts};
use crate::settings::schema;
use crate::settings_look::{TEXT_SIZES, family_id, theme_id};
use crate::settings_view::range;

fn update(app: &mut App, message: Message) {
    let _ = app.update(message);
}

fn draft(app: &App, key: &str) -> Value {
    app.settings_draft
        .as_ref()
        .and_then(|d| d.values.get(key).cloned())
        .unwrap_or(Value::Null)
}

fn ids(key: &str) -> Vec<String> {
    schema().get(key).map_or_else(Vec::new, |d| {
        d.choices
            .iter()
            .filter_map(|c| c.value.as_str().map(str::to_owned))
            .collect()
    })
}

/// The window open on Görünüm with these settings saved, settled.
fn opened(size: Size, saved: &[(&str, Value)]) -> (App, Snapshot) {
    let mut app = app_with_drawing();
    let _ = app.settings.choose(saved);
    app.apply_settings();
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    snapshot.settle(&mut app, App::view, &mut update);
    let _ = app.run("tools.options");
    snapshot.settle(&mut app, App::view, &mut update);
    (app, snapshot)
}

/// Clicks the middle of a text on screen; the window's, over the rest.
fn click(app: &mut App, snapshot: &mut Snapshot, caption: &str) {
    let at = find_texts(snapshot, app, caption)
        .pop()
        .unwrap_or_else(|| panic!("“{caption}” on screen"))
        .center();
    snapshot.input(app, App::view, &mut update, Input::Click(at));
}

#[test]
fn the_pickers_write_the_schemas_values() {
    assert_eq!(
        Mode::ALL.map(theme_id).to_vec(),
        ids("appearance.theme"),
        "a card for each theme, in the schema's order"
    );
    assert_eq!(
        Family::ALL.map(family_id).to_vec(),
        ids("appearance.uiFont"),
        "a card for each typeface, in the schema's order"
    );
    let sizes = range("appearance.textSize");
    for (px, name) in TEXT_SIZES {
        assert!(
            sizes.contains(&f64::from(px)),
            "{name} ({px} px) in {sizes:?}"
        );
    }
}

#[test]
fn a_click_on_a_card_a_swatch_or_a_step_writes_the_draft() {
    let (mut app, mut snapshot) = opened(Size::new(1440.0, 900.0), &[]);
    assert_eq!(draft(&app, "appearance.theme"), Value::from("dark"));

    click(&mut app, &mut snapshot, "Açık pafta");
    assert_eq!(draft(&app, "appearance.theme"), Value::from("light"));

    click(&mut app, &mut snapshot, "Mavi");
    assert_eq!(draft(&app, "appearance.accent"), Value::from("blue"));

    click(&mut app, &mut snapshot, "Inter");
    assert_eq!(draft(&app, "appearance.uiFont"), Value::from("inter"));

    // The steps are under the cards: scrolled to, as by hand. Operations see
    // the content unscrolled, and iced scrolls 60 pixels a line.
    let step = find_texts(&mut snapshot, &app, "Büyük")
        .pop()
        .expect("the step");
    let lines = ((step.center_y() - 480.0) / 60.0).ceil().max(0.0);
    snapshot.input(
        &mut app,
        App::view,
        &mut update,
        Input::Scroll(Point::new(800.0, 500.0), -lines),
    );
    let at = Point::new(step.center_x(), step.center_y() - lines * 60.0);
    snapshot.input(&mut app, App::view, &mut update, Input::Click(at));
    assert_eq!(draft(&app, "appearance.textSize"), Value::from(14));

    // Nothing is taken before Kaydet: the window keeps a draft.
    assert_eq!(
        app.settings.requested("appearance.theme"),
        Value::from("dark")
    );
}

fn save(snapshot: &mut Snapshot, app: &App, out: &Path, name: &str) {
    let file = out.join(format!("{name}.png"));
    snapshot
        .render(app.view(), &app.theme())
        .save(&file)
        .expect("writes the picture");
    println!("{}", file.display());
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        let theme = ("appearance.theme", Value::from(mode));
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            let size = format!("{width}x{height}");
            let middle = Point::new(width * 0.6, height * 0.5);
            let (mut app, mut snapshot) =
                opened(Size::new(width, height), std::slice::from_ref(&theme));
            save(
                &mut snapshot,
                &app,
                &out,
                &format!("gorunum-kartlar-{size}{suffix}"),
            );
            // Down to the typefaces, then to the end.
            snapshot.input(
                &mut app,
                App::view,
                &mut update,
                Input::Scroll(middle, -8.0),
            );
            save(
                &mut snapshot,
                &app,
                &out,
                &format!("gorunum-yazi-{size}{suffix}"),
            );
            snapshot.input(
                &mut app,
                App::view,
                &mut update,
                Input::Scroll(middle, -40.0),
            );
            save(
                &mut snapshot,
                &app,
                &out,
                &format!("gorunum-alt-{size}{suffix}"),
            );

            // A colour of one's own, and one that does not read yet.
            for (accent, name) in [("#8b5a2b", "ozel-renk"), ("#8b5a", "okunamadi")] {
                let (mut app, mut snapshot) =
                    opened(Size::new(width, height), std::slice::from_ref(&theme));
                let _ = app.update(Message::Settings(crate::settings_view::Edit::Value(
                    "appearance.accent",
                    Value::from(accent),
                )));
                snapshot.settle(&mut app, App::view, &mut update);
                snapshot.input(
                    &mut app,
                    App::view,
                    &mut update,
                    Input::Scroll(middle, -4.0),
                );
                save(
                    &mut snapshot,
                    &app,
                    &out,
                    &format!("gorunum-{name}-{size}{suffix}"),
                );
            }
        }
        // The largest text in the smaller window: nothing cut, nothing over.
        let size = Size::new(1100.0, 650.0);
        let (mut app, mut snapshot) =
            opened(size, &[theme, ("appearance.textSize", Value::from(16))]);
        save(
            &mut snapshot,
            &app,
            &out,
            &format!("gorunum-16px-1100x650{suffix}"),
        );
        let middle = Point::new(660.0, 325.0);
        snapshot.input(
            &mut app,
            App::view,
            &mut update,
            Input::Scroll(middle, -10.0),
        );
        save(
            &mut snapshot,
            &app,
            &out,
            &format!("gorunum-16px-yazi-1100x650{suffix}"),
        );
    }
}
