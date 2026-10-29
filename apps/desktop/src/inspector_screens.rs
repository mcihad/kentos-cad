//! Öznitelikler's look (docs/adr/0747, 0748): a dimension selected, a row under the pointer, its text being typed;
//! two dimensions whose rows say “Çeşitli”; a layer's fields, a long note cut and whole in its tip: the window and
//! the panel cut out of it, in `.run/shots/oznitelik-*`.
//!
//! ```text
//! cargo test -p kentos-desktop --bin kentos-cad inspector_screens -- --ignored --nocapture
//! ```

use std::path::PathBuf;

use iced::{Point, Rectangle, Size};
use kentos_commands::{DimensionEntity, Entity, EntityBase, Vec2};
use kentos_domain::Slot;
use kentos_ui::snapshot::{Image, Input, Snapshot};

use crate::app::{App, Message};
use crate::selecting::tests::objects;

const E: f64 = 487000.0;
const N: f64 = 4420000.0;

fn update(app: &mut App, message: Message) {
    let _ = app.update(message);
}

/// The part of `image` in `rect` (logical pixels at scale 1).
fn cut(image: &Image, rect: Rectangle) -> Image {
    let (x0, y0) = (rect.x.max(0.0) as u32, rect.y.max(0.0) as u32);
    let w = (rect.width as u32).min(image.width - x0);
    let h = (rect.height as u32).min(image.height - y0);
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in y0..y0 + h {
        let start = ((y * image.width + x0) * 4) as usize;
        rgba.extend_from_slice(&image.rgba[start..start + (w * 4) as usize]);
    }
    Image {
        width: w,
        height: h,
        rgba,
    }
}

/// `image` at twice its size, each pixel a square of four.
fn twice(image: &Image) -> Image {
    let (w, h) = (image.width * 2, image.height * 2);
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let i = (((y / 2) * image.width + x / 2) * 4) as usize;
            rgba.extend_from_slice(&image.rgba[i..i + 4]);
        }
    }
    Image {
        width: w,
        height: h,
        rgba,
    }
}

/// Where a text is in the window, the first found.
pub(crate) fn text_at(snapshot: &mut Snapshot, app: &App, words: &str) -> Option<Rectangle> {
    use iced::advanced::widget::{Id, Operation};
    use std::sync::{Arc, Mutex};
    struct Find(String, Arc<Mutex<Option<Rectangle>>>);
    impl Operation for Find {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn text(&mut self, _: Option<&Id>, bounds: Rectangle, text: &str) {
            let mut found = self.1.lock().expect("a lock");
            if found.is_none() && text == self.0 {
                *found = Some(bounds);
            }
        }
    }
    let found = Arc::new(Mutex::new(None));
    snapshot.operate(app.view(), Box::new(Find(words.to_owned(), found.clone())));
    found.lock().map_or(None, |at| *at)
}

fn dimension(layer: &str) -> Entity {
    dimension_at(layer, 0.0, 2.5, None)
}

/// A dimension `dy` above the first, its text `height` high and `text`.
fn dimension_at(layer: &str, dy: f64, height: f64, text: Option<&str>) -> Entity {
    Entity::Dimension(DimensionEntity {
        annotative: None,
        text_shift: None,
        breaks: Vec::new(),
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: layer.to_owned(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        a: Vec2 { x: E, y: N + dy },
        b: Vec2 {
            x: E + 10.0,
            y: N + dy,
        },
        offset: 2.0,
        height,
        text: text.map(str::to_owned),
        style: None,
        angle: None,
        c: None,
        mask: false,
        za: None,
        zb: None,
        refs: None,
        look: Default::default(),
        level: None,
    })
}

/// Öznitelikler given three quarters of the dock, Genel closed: the rows to look at in view.
pub(crate) fn roomy(app: &mut App) {
    use kentos_ui::widget::docking::{Event, Side};
    app.docks
        .update(Event::Shared(Side::Right, vec![0.25, 0.75]));
    app.props_closed.insert("general");
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn inspector_screens() {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder");
    let save = |snapshot: &mut Snapshot, app: &App, name: &str| {
        let image = snapshot.render(app.view(), &app.theme());
        let file = out.join(format!("oznitelik-{name}.png"));
        image.save(&file).expect("a picture");
        // The right dock's lower half, Öznitelikler, at twice its size to look at.
        let panel = twice(&cut(
            &image,
            Rectangle::new(Point::new(1128.0, 330.0), Size::new(312.0, 540.0)),
        ));
        let file = out.join(format!("oznitelik-{name}-panel.png"));
        panel.save(&file).expect("a picture");
        println!("{}", file.display());
        println!("{}", file.display());
    };
    // A dimension: its rows; the pointer over Ötelenme; then its text typed.
    let mut app = objects();
    let layer = app
        .document
        .as_ref()
        .map(|d| d.model.layers().active().to_owned())
        .unwrap_or_default();
    let slot = app
        .document
        .as_mut()
        .and_then(|d| d.model.add(dimension(&layer)).ok())
        .expect("a slot");
    app.selection.set([slot]);
    roomy(&mut app);
    let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    snapshot.settle(&mut app, App::view, &mut update);
    save(&mut snapshot, &app, "olcu-1-duz");
    if let Some(row) = text_at(&mut snapshot, &app, "Ötelenme") {
        snapshot.input(
            &mut app,
            App::view,
            &mut update,
            Input::Move(Point::new(row.x + 20.0, row.y + row.height / 2.0)),
        );
        save(&mut snapshot, &app, "olcu-2-uzerine-gelince");
    }
    if let Some(row) = text_at(&mut snapshot, &app, "Yazı") {
        let cell = Point::new(row.x + 190.0, row.y + row.height / 2.0);
        snapshot.input(&mut app, App::view, &mut update, Input::Click(cell));
        snapshot.input(
            &mut app,
            App::view,
            &mut update,
            Input::Type("Ada 5".into()),
        );
        save(&mut snapshot, &app, "olcu-3-duzenlenirken");
        // The same in the light theme.
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from("light"))]);
        app.apply_settings();
        snapshot.settle(&mut app, App::view, &mut update);
        save(&mut snapshot, &app, "olcu-4-duzenlenirken-acik");
        // Enter writes it: the dimension shows the text, the row rests again.
        snapshot.input(
            &mut app,
            App::view,
            &mut update,
            Input::Key(iced::keyboard::key::Named::Enter),
        );
        let written = app
            .document
            .as_ref()
            .and_then(|d| d.model.get(slot))
            .and_then(|e| match e {
                Entity::Dimension(d) => d.text.clone(),
                _ => None,
            });
        assert_eq!(
            written.as_deref(),
            Some("Ada 5"),
            "Enter writes what was typed"
        );
    }
    // The narrow window (1100 × 650): long names end in “…”; the pointer resting on one shows the whole of it.
    let mut narrow = Snapshot::new(Size::new(1100.0, 650.0)).expect("a renderer");
    narrow.settle(&mut app, App::view, &mut update);
    let image = narrow.render(app.view(), &app.theme());
    image
        .save(out.join("oznitelik-dar.png"))
        .expect("a picture");
    if let Some(row) = text_at(&mut narrow, &app, "Ölçülen uzunluk") {
        let at = Point::new(row.x + 20.0, row.y + row.height / 2.0);
        narrow.input(&mut app, App::view, &mut update, Input::Move(at));
        narrow.wait(std::time::Duration::from_millis(500));
        narrow.input(
            &mut app,
            App::view,
            &mut update,
            Input::Move(Point::new(at.x + 1.0, at.y)),
        );
        let image = narrow.render(app.view(), &app.theme());
        image
            .save(out.join("oznitelik-dar-ipucu.png"))
            .expect("a picture");
    }
    // Two dimensions, one with its value's background and their notes apart: Zemin and Not say “Çeşitli”, faint;
    // pressed, the cell is empty.
    let mut app = objects();
    let slots: Vec<Slot> = [(0.0, true, "Kapı boşluğu"), (6.0, false, "Pencere boşluğu")]
        .into_iter()
        .filter_map(|(dy, mask, note)| {
            let mut e = dimension_at(&layer, dy, 2.5, None);
            if let Entity::Dimension(d) = &mut e {
                d.mask = mask;
                d.base.attrs.insert("Not".into(), note.into());
            }
            app.document.as_mut().and_then(|d| d.model.add(e).ok())
        })
        .collect();
    app.selection.set(slots);
    roomy(&mut app);
    app.props_closed.insert("scales");
    let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    snapshot.settle(&mut app, App::view, &mut update);
    save(&mut snapshot, &app, "olcu-5-cesitli");
    // Ölçü closed, Ortak öznitelikler comes up: its Not pressed.
    app.props_closed.insert("dimensions");
    snapshot.settle(&mut app, App::view, &mut update);
    if let Some(row) = text_at(&mut snapshot, &app, "Not") {
        let cell = Point::new(row.x + 190.0, row.y + row.height / 2.0);
        snapshot.input(&mut app, App::view, &mut update, Input::Click(cell));
        save(&mut snapshot, &app, "olcu-6-cesitli-duzenlenirken");
        snapshot.input(
            &mut app,
            App::view,
            &mut update,
            Input::Key(iced::keyboard::key::Named::Escape),
        );
    }
    // A layer's fields with a form (docs/adr/0199 §5): lists and cells.
    let (mut app, _) = App::boot(None);
    crate::form_scenes::buildings(&mut app, true);
    app.selection.set([Slot(1)]);
    app.props_closed.insert("general");
    app.props_closed.insert("geometry");
    roomy(&mut app);
    let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    snapshot.settle(&mut app, App::view, &mut update);
    save(&mut snapshot, &app, "alanlar");
    // A long note: it rests cut with “…”; the pointer resting on it shows the whole of it.
    if let Some(doc) = app.document.as_mut()
        && let Some(mut building) = doc.model.get(Slot(1)).cloned()
    {
        building.base_mut().attrs.insert(
            "Açıklama".into(),
            "Kuzey cephesinde sıva dökülmüş, çatı oluğu yer yer tıkalı; bodrumda nem izi var."
                .into(),
        );
        doc.model.update(Slot(1), building);
    }
    snapshot.settle(&mut app, App::view, &mut update);
    save(&mut snapshot, &app, "alanlar-2-uzun");
    if let Some(row) = text_at(&mut snapshot, &app, "Açıklama") {
        let cell = Point::new(row.x + 190.0, row.y + row.height / 2.0);
        snapshot.input(&mut app, App::view, &mut update, Input::Move(cell));
        snapshot.wait(std::time::Duration::from_millis(500));
        let rested = Point::new(cell.x + 1.0, cell.y);
        snapshot.input(&mut app, App::view, &mut update, Input::Move(rested));
        save(&mut snapshot, &app, "alanlar-3-ipucu");
    }
}
