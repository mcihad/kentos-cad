//! The windows' pictures (picture.rs, docs/adr/0937): a road window's drawing takes the room a full screen window has
//! over, zooms with the wheel, pans sideways and is saved as a PNG; and its pictures.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use iced::advanced::widget::{Id, Operation};
use iced::{Point, Rectangle, Size, Vector};
use kentos_ui::snapshot::{Input, Snapshot};
use kentos_ui::widget::picture::{Button, Saved, View, button_at};
use kentos_ui::widget::sizable::sizes;

use crate::app::{App, Message};

/// A road project from a command's shared cases (`road.<name>.query.json`).
fn opened(cases: &str) -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value = serde_json::from_str(cases).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

const SECTIONS: &str = include_str!("../../../fixtures/commands/v1/road.sections.query.json");
const PROFILES: &str = include_str!("../../../fixtures/commands/v1/road.profiles.query.json");
const SUPERELEVATIONS: &str = include_str!("../../../fixtures/commands/v1/road.superelevations.query.json");
const VOLUMES: &str = include_str!("../../../fixtures/commands/v1/road.volumes.query.json");
const HAULS: &str = include_str!("../../../fixtures/commands/v1/road.hauls.query.json");

/// Every picture on screen, in the tree's order: its view and its bounds.
#[derive(Clone, Default)]
struct Pictures(Arc<Mutex<Vec<(View, Rectangle)>>>);

impl Operation for Pictures {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
        operate(self);
    }

    fn custom(&mut self, _: Option<&Id>, bounds: Rectangle, state: &mut dyn std::any::Any) {
        if let Some(view) = state.downcast_ref::<View>() {
            self.0.lock().unwrap().push((*view, bounds));
        }
    }
}

fn pictures(snapshot: &mut Snapshot, app: &App) -> Vec<(View, Rectangle)> {
    let found = Pictures::default();
    snapshot.operate(app.view(), Box::new(found.clone()));
    found.0.lock().unwrap().clone()
}

/// The dialogs' kept sizes are the process's: these tests take turns (a full screen Enkesit in one made another's
/// Enkesit full screen half way).
static SIZES: Mutex<()> = Mutex::new(());

fn turn() -> std::sync::MutexGuard<'static, ()> {
    SIZES.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The window `run` opens on `cases`, full screen or at its own size, after `then`; the snapshot settled.
fn window(cases: &str, key: &str, run: &'static str, full: bool, then: impl FnOnce(&mut App)) -> (App, Snapshot) {
    if full {
        sizes::set(key, sizes::Kept { width: 800.0, height: 600.0, maximized: true });
    } else {
        sizes::forget(key);
    }
    let mut app = opened(cases);
    let _ = app.update(Message::RibbonTab("alignment"));
    let _ = app.update(Message::Run(run));
    then(&mut app);
    let mut snapshot = Snapshot::new(Size::new(1920.0, 1080.0)).expect("a renderer");
    snapshot.settle(&mut app, App::view, &mut |app: &mut App, m| {
        let _ = app.update(m);
    });
    (app, snapshot)
}

/// Full screen, a road window's picture takes the room the rest leaves: several times its own height.
#[test]
fn a_full_screen_road_window_gives_its_picture_the_room() {
    let _turn = turn();
    for (cases, key, run, least) in [
        (SUPERELEVATIONS, "Dever", "superelevation.editor", 210.0),
        (SECTIONS, "Enkesit", "section.editor", 230.0),
        (PROFILES, "Boykesit", "profile.editor", 330.0),
        (VOLUMES, "Kübaj", "volume.editor", 190.0),
    ] {
        let (app, mut snapshot) = window(cases, key, run, false, |_| {});
        let own = pictures(&mut snapshot, &app).first().map(|p| p.1.height).expect(key);
        // At least its least at the 12 px body text, grown with the text, less the box's 1 px frame; more where the
        // form is taller than the window and scrolls by itself (docs/adr/0938 §3): the picture takes what is over
        // (Kübaj with its Belirsizlik section: 455 against 206).
        let least = kentos_ui::theme::typography::scaled(least);
        assert!(own > least - 4.0, "{key}: at its own size the picture is at least its least ({least}): {own}");
        let (app, mut snapshot) = window(cases, key, run, true, |_| {});
        let full = pictures(&mut snapshot, &app).first().map(|p| p.1.height).expect(key);
        sizes::forget(key);
        // Full screen it takes the room: Boykesit keeps its PVI table under the chart (542 against 356 at
        // 1920 × 1080), Kübaj shares it with its sheet.
        assert!(full >= own - 1.0 && full > 1.4 * least && full > 440.0, "{key}: full screen {full} against {own}");
    }
}

/// The wheel zooms the section about the cursor, Shift and the wheel move it sideways, the bar saves what is shown
/// as a PNG at twice the pixels, and the window says where it went.
#[test]
fn a_section_zooms_pans_and_is_saved() {
    let _turn = turn();
    let (mut app, mut snapshot) = window(SECTIONS, "Enkesit", "section.editor", false, |_| {});
    let saved: Rc<RefCell<Vec<Saved>>> = Rc::default();
    let kept = saved.clone();
    let mut update = move |app: &mut App, m: Message| match m {
        Message::SavePicture(s) => kept.borrow_mut().push(s),
        other => {
            let _ = app.update(other);
        }
    };
    let (_, bounds) = pictures(&mut snapshot, &app)[0];
    let at = Point::new(bounds.x + bounds.width * 0.4, bounds.y + bounds.height * 0.5);
    snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, 4.0));
    let (view, _) = pictures(&mut snapshot, &app)[0];
    assert!((view.zoom - 1.2f32.powi(4)).abs() < 1e-3, "{view:?}");
    // The content's point under the cursor stayed under it.
    let k = view.zoom;
    let under = Vector::new(at.x - bounds.x, at.y - bounds.y);
    assert!((view.offset.x - under.x * (k - 1.0)).abs() < 1e-2 && (view.offset.y - under.y * (k - 1.0)).abs() < 1e-2);
    // Shift and the wheel down: to the right.
    let shift = iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(iced::keyboard::Modifiers::SHIFT));
    let _ = snapshot.deliver(app.view(), &shift);
    snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, -2.0));
    let (moved, _) = pictures(&mut snapshot, &app)[0];
    assert_eq!(moved.zoom, view.zoom);
    assert!((moved.offset.x - (view.offset.x + 96.0)).abs() < 1e-2, "{moved:?}");
    let plain = iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(iced::keyboard::Modifiers::default()));
    let _ = snapshot.deliver(app.view(), &plain);
    // Resim olarak kaydet: the view at twice the pixels, under the section's name.
    snapshot.input(&mut app, App::view, &mut update, Input::Click(button_at(bounds, Button::Save)));
    let saved = saved.borrow();
    assert_eq!(saved.len(), 1);
    let s = &saved[0];
    assert!(s.name.starts_with("Enkesit — km "), "{}", s.name);
    assert_eq!((s.width, s.height), ((bounds.width * 2.0).round() as u32, (bounds.height * 2.0).round() as u32));
    let file = std::env::temp_dir().join(format!("kentos-resim-{}.png", std::process::id()));
    crate::picture::write(s, &file).expect("written");
    assert_eq!(&std::fs::read(&file).expect("read")[1..4], b"PNG");
    let _ = app.update(Message::PictureWritten(Some(Ok((file.clone(), s.width, s.height)))));
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(said.iter().any(|l| l.starts_with("Resim kaydedildi")), "{said:?}");
    let _ = std::fs::remove_file(file);
    assert_eq!(crate::picture::file_name("Enkesit — km 1+120.000"), "Enkesit — km 1+120.000.png");
    assert_eq!(crate::picture::file_name("a/b:c"), "a-b-c.png");
}

/// The pictures for the owner, `.run/gui-evidence/goruntuleyici/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib picture_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    let _turn = turn();
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/goruntuleyici");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let shot = |snapshot: &mut Snapshot, app: &mut App, file: &str| {
        snapshot.settle(app, App::view, &mut |app: &mut App, m| {
            let _ = app.update(m);
        });
        let path = out.join(format!("{file}.png"));
        snapshot.render(app.view(), &app.theme()).save(&path).expect("writes the picture");
        println!("{}", path.display());
    };
    for (theme, name) in [("dark", "koyu"), ("light", "acik")] {
        let themed = |app: &mut App| {
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
        };
        // Enkesit at its own size and full screen.
        let (mut app, mut snapshot) = window(SECTIONS, "Enkesit", "section.editor", false, themed);
        shot(&mut snapshot, &mut app, &format!("enkesit-kendi-boyunda-{name}"));
        // The cursor on the section: its bar opens.
        let (_, own) = pictures(&mut snapshot, &app)[0];
        snapshot.input(&mut app, App::view, &mut |app: &mut App, m| {
            let _ = app.update(m);
        }, Input::Move(Point::new(own.x + own.width * 0.3, own.y + own.height * 0.6)));
        shot(&mut snapshot, &mut app, &format!("enkesit-kendi-boyunda-imlecle-{name}"));
        let (mut app, mut snapshot) = window(SECTIONS, "Enkesit", "section.editor", true, themed);
        shot(&mut snapshot, &mut app, &format!("enkesit-tam-ekran-{name}"));
        // Zoomed with the wheel on the left ditch, the zoom's tip shown.
        let mut update = |app: &mut App, m: Message| {
            if !matches!(m, Message::SavePicture(_)) {
                let _ = app.update(m);
            }
        };
        let (_, bounds) = pictures(&mut snapshot, &app)[0];
        let at = Point::new(bounds.x + bounds.width * 0.405, bounds.y + bounds.height * 0.8);
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, 8.0));
        snapshot.input(&mut app, App::view, &mut update, Input::Move(button_at(bounds, Button::Zoom)));
        shot(&mut snapshot, &mut app, &format!("enkesit-yakin-{name}"));
        // Moved sideways with the bar's thumb.
        let thumb_y = bounds.y + bounds.height - 6.0;
        let from = Point::new(bounds.x + bounds.width * 0.3, thumb_y);
        snapshot.input(&mut app, App::view, &mut update, Input::Drag(from, Point::new(from.x + 260.0, thumb_y)));
        snapshot.input(&mut app, App::view, &mut update, Input::Move(Point::new(bounds.x + bounds.width * 0.5, thumb_y)));
        shot(&mut snapshot, &mut app, &format!("enkesit-kaydirilmis-{name}"));
        if theme == "dark" {
            // What Resim olarak kaydet writes: the view, in the light theme, at twice the pixels.
            let saved: Rc<RefCell<Option<Saved>>> = Rc::default();
            let kept = saved.clone();
            let mut keep = move |app: &mut App, m: Message| match m {
                Message::SavePicture(s) => *kept.borrow_mut() = Some(s),
                other => {
                    let _ = app.update(other);
                }
            };
            snapshot.input(&mut app, App::view, &mut keep, Input::Click(button_at(bounds, Button::Save)));
            let s = saved.borrow().clone().expect("saved");
            let path = out.join("kaydedilen-enkesit.png");
            crate::picture::write(&s, &path).expect("written");
            println!("{} ({}×{})", path.display(), s.width, s.height);
        }
        // Boykesit full screen, zoomed on its first vertical curve.
        let (mut app, mut snapshot) = window(PROFILES, "Boykesit", "profile.editor", true, themed);
        shot(&mut snapshot, &mut app, &format!("boykesit-tam-ekran-{name}"));
        let (_, bounds) = pictures(&mut snapshot, &app)[0];
        let at = Point::new(bounds.x + bounds.width * 0.3, bounds.y + bounds.height * 0.4);
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, 6.0));
        shot(&mut snapshot, &mut app, &format!("boykesit-yakin-{name}"));
        // The others full screen.
        for (cases, key, run, file) in [
            (SUPERELEVATIONS, "Dever", "superelevation.editor", "dever"),
            (VOLUMES, "Kübaj", "volume.editor", "kubaj"),
            (HAULS, "Brükner", "haul.editor", "bruckner"),
        ] {
            let (mut app, mut snapshot) = window(cases, key, run, true, themed);
            shot(&mut snapshot, &mut app, &format!("{file}-tam-ekran-{name}"));
        }
    }
    for key in ["Enkesit", "Boykesit", "Dever", "Kübaj", "Brükner"] {
        sizes::forget(key);
    }
}

/// Full screen, the point under the wheel stays: the dialog lays its body out twice (its least first), and the
/// offset clamped to the first pass had a zoom low in the window jump up (1600 → 815).
#[test]
fn the_point_under_the_wheel_stays_in_a_full_screen_window() {
    let _turn = turn();
    let (mut app, mut snapshot) = window(SECTIONS, "Enkesit", "section.editor", true, |_| {});
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    let (_, bounds) = pictures(&mut snapshot, &app)[0];
    let at = Point::new(bounds.x + bounds.width * 0.405, bounds.y + bounds.height * 0.8);
    snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, 8.0));
    let (view, after) = pictures(&mut snapshot, &app)[0];
    sizes::forget("Enkesit");
    assert_eq!(after, bounds);
    let k = view.zoom;
    let under = Vector::new(at.x - bounds.x, at.y - bounds.y);
    assert!((view.offset.x - under.x * (k - 1.0)).abs() < 0.5 && (view.offset.y - under.y * (k - 1.0)).abs() < 0.5, "{view:?}");
}
