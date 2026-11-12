//! The windows resized with the mouse, made full screen and opened again at their size (docs/adr/0746), on the
//! forms' windows: pictures in `.run/shots/pencere-*`.

use std::path::PathBuf;

use iced::{Event, Point, Size, mouse};
use kentos_domain::Slot;
use kentos_ui::snapshot::Snapshot;
use kentos_ui::widget::sizable::{self, sizes};

use crate::app::{App, Message};
use crate::form_scenes::buildings;
use crate::tools_scenes::run;

fn at(x: f32, y: f32) -> Event {
    Event::Mouse(mouse::Event::CursorMoved {
        position: Point::new(x, y),
    })
}

fn press() -> Event {
    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
}

fn release() -> Event {
    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
}

/// The window drawn last: its key and where it is.
fn last() -> (String, [f32; 4]) {
    sizes::shown().pop().expect("a window was drawn")
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn the_forms_windows_are_resized_made_full_screen_and_kept() {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder");
    let mut update = |app: &mut App, message: Message| {
        let _ = app.update(message);
    };
    for (name, open) in [
        (
            "formla-duzenle",
            (|app: &mut App| {
                buildings(app, true);
                app.selection.set([Slot(1)]);
                run(app, "data.formEdit");
            }) as fn(&mut App),
        ),
        ("formlar", |app: &mut App| {
            buildings(app, true);
            run(app, "data.forms");
        }),
    ] {
        sizes::load(Default::default());
        let (mut app, _) = App::boot(None);
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        open(&mut app);
        snapshot.settle(&mut app, App::view, &mut update);
        let shot = |snapshot: &mut Snapshot, app: &App, step: &str| {
            let file = out.join(format!("pencere-{name}-{step}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("a picture");
            println!("{}", file.display());
        };
        shot(&mut snapshot, &app, "1-acilis");
        let (key, [x, y, w, h]) = last();
        // The bottom right corner dragged out with the mouse: 120 right, 100 down (the box grows twice as much, within
        // the window less its margin).
        let corner = Point::new(x + w - 3.0, y + h - 3.0);
        snapshot.step(
            &mut app,
            App::view,
            &mut update,
            &[
                at(corner.x, corner.y),
                press(),
                at(corner.x + 120.0, corner.y + 100.0),
                release(),
            ],
        );
        snapshot.settle(&mut app, App::view, &mut update);
        shot(&mut snapshot, &app, "2-buyutuldu");
        let (_, [x2, y2, w2, h2]) = last();
        let room = (
            1440.0 - 2.0 * sizable::MARGIN,
            900.0 - 2.0 * sizable::MARGIN,
        );
        assert!(
            (w2 - (w + 240.0).min(room.0)).abs() < 1.0,
            "{key}: {w} → {w2}"
        );
        assert!(
            (h2 - (h + 200.0).min(room.1)).abs() < 1.0,
            "{key}: {h} → {h2}"
        );
        // Its size is written (pencereler.json's text) and opens again so.
        let kept = sizes::all();
        let text = crate::dialog_sizes::text(&kept);
        assert_eq!(crate::dialog_sizes::read(Some(&text)), kept);
        // Full screen from its button: the window behind, a margin left.
        let button = Point::new(x2 + w2 - sizable::PAD - 8.0, y2 + sizable::PAD + 8.0);
        snapshot.step(
            &mut app,
            App::view,
            &mut update,
            &[at(button.x, button.y), press(), release()],
        );
        snapshot.settle(&mut app, App::view, &mut update);
        shot(&mut snapshot, &app, "3-tam-ekran");
        let (_, full) = last();
        assert_eq!(
            full,
            [
                sizable::MARGIN,
                sizable::MARGIN,
                1440.0 - 2.0 * sizable::MARGIN,
                900.0 - 2.0 * sizable::MARGIN
            ]
        );
        // Closed and opened again in another run: the sizes read back, the window at its kept size.
        sizes::load(crate::dialog_sizes::read(Some(&crate::dialog_sizes::text(
            &sizes::all(),
        ))));
        if let Some(k) = sizes::get(&key) {
            sizes::set(
                &key,
                sizes::Kept {
                    maximized: false,
                    ..k
                },
            );
        }
        let (mut again, _) = App::boot(None);
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        snapshot.settle(&mut again, App::view, &mut update);
        open(&mut again);
        snapshot.settle(&mut again, App::view, &mut update);
        shot(&mut snapshot, &again, "4-yeniden-acilis");
        let (_, [_, _, w4, h4]) = last();
        assert!(
            (w4 - w2).abs() < 1.0 && (h4 - h2).abs() < 1.0,
            "{w2}×{h2} → {w4}×{h4}"
        );
    }
    sizes::load(Default::default());
}
