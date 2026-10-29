//! What a keystroke in Öznitelikler costs on a large drawing (the owner's 29 October report: typing a dimension's text
//! in the panel answered seconds late): the view built, the whole window laid out again (a cell that changes its
//! text asks for it) and drawn, with the first dimension selected. Run by hand on a file of one's own, in release:
//!
//! ```text
//! KENTOS_PROBE_FILE=$HOME/Downloads/deneme_suşehri.dxf cargo test --release -p kentos-desktop --bin kentos-cad inspector_probe -- --ignored --nocapture
//! ```

use std::time::{Duration, Instant};

use kentos_commands::Entity;
use kentos_ui::snapshot::{Input, Snapshot};

use crate::app::App;

fn median(mut ms: Vec<Duration>) -> f64 {
    ms.sort();
    ms[ms.len() / 2].as_secs_f64() * 1000.0
}

/// The view's build, the window's layout and its drawing, each the median of `n`.
fn times(snapshot: &mut Snapshot, app: &App, n: usize) -> (f64, f64, f64) {
    let (mut view, mut layout, mut draw) = (Vec::new(), Vec::new(), Vec::new());
    for _ in 0..n {
        let t = Instant::now();
        let v = app.view();
        view.push(t.elapsed());
        let t = Instant::now();
        let _ = snapshot.update(v, &[]);
        layout.push(t.elapsed());
        let t = Instant::now();
        let _ = snapshot.render(app.view(), &app.theme());
        draw.push(t.elapsed());
    }
    (median(view), median(layout), median(draw))
}

#[test]
#[ignore = "a probe on a file of one's own, run by hand"]
fn inspector_probe() {
    let Some((mut snapshot, mut app)) = crate::label_probe::opened() else {
        return;
    };
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    let count = app.document.as_ref().map_or(0, |d| d.model.len());
    let (v, l, d) = times(&mut snapshot, &app, 5);
    println!("{count} nesne, seçim yok: görünüm {v:.1} ms, yerleşim {l:.1} ms, çizim {d:.1} ms");
    let dimension = app.document.as_ref().and_then(|d| {
        d.model
            .entities_with_uids()
            .find(|(_, e)| matches!(e, Entity::Dimension(_)))
            .and_then(|(uid, _)| d.model.slot_of(uid))
    });
    // A drawing without a dimension gets one in the middle of the view.
    let slot = match dimension {
        Some(slot) => slot,
        None => {
            let b = app.viewport.camera.visible_bounds();
            let (x, y) = ((b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0);
            let doc = app.document.as_mut().expect("a drawing");
            let layer = doc.model.layers().active().to_owned();
            let dimension = Entity::Dimension(kentos_commands::DimensionEntity {
                annotative: None,
                text_shift: None,
                breaks: Vec::new(),
                base: kentos_commands::EntityBase {
                    look: Default::default(),
                    id: 0,
                    layer_id: layer,
                    color: None,
                    attrs: Default::default(),
                    label: None,
                    symbol: None,
                    line_weight: None,
                    label_pins: Vec::new(),
                    attachments: Vec::new(),
                },
                a: kentos_commands::Vec2 { x, y },
                b: kentos_commands::Vec2 { x: x + 10.0, y },
                offset: 2.0,
                height: 2.5,
                text: None,
                style: None,
                angle: None,
                c: None,
                mask: false,
                za: None,
                zb: None,
                refs: None,
                look: Default::default(),
                level: None,
            });
            doc.model.add(dimension).expect("a slot")
        }
    };
    app.selection.set([slot]);
    crate::inspector_screens::roomy(&mut app);
    snapshot.settle(&mut app, App::view, &mut update);
    let (v, l, d) = times(&mut snapshot, &app, 5);
    println!("bir ölçü seçili: görünüm {v:.1} ms, yerleşim {l:.1} ms, çizim {d:.1} ms");
    // The Yazı row's cell pressed, then letters typed one at a time as the window gets them.
    let Some(row) = crate::inspector_screens::text_at(&mut snapshot, &app, "Yazı") else {
        println!("Yazı satırı yok");
        return;
    };
    let cell = iced::Point::new(row.x + 190.0, row.y + row.height / 2.0);
    snapshot.input(&mut app, App::view, &mut update, Input::Click(cell));
    let mut keys = Vec::new();
    for c in "Ada 5 numaralı yapı".chars() {
        let t = Instant::now();
        snapshot.input(&mut app, App::view, &mut update, Input::Type(c.to_string()));
        let _ = snapshot.render(app.view(), &app.theme());
        keys.push(t.elapsed());
    }
    let longest = keys.iter().max().copied().unwrap_or_default();
    println!(
        "Yazı hücresine {} harf: harf başına ortanca {:.1} ms, en uzun {:.1} ms (olay, durulma ve çizim)",
        keys.len(),
        median(keys),
        longest.as_secs_f64() * 1000.0
    );
    // Enter writes it: the command, the drawing following it and the next two frames.
    let t = Instant::now();
    snapshot.input(
        &mut app,
        App::view,
        &mut update,
        Input::Key(iced::keyboard::key::Named::Enter),
    );
    let written = t.elapsed();
    let t = Instant::now();
    let _ = snapshot.render(app.view(), &app.theme());
    let first = t.elapsed();
    let t = Instant::now();
    let _ = snapshot.render(app.view(), &app.theme());
    let second = t.elapsed();
    let text = app
        .document
        .as_ref()
        .and_then(|d| d.model.get(slot))
        .and_then(|e| match e {
            Entity::Dimension(d) => d.text.clone(),
            _ => None,
        });
    println!(
        "Enter: yazma ve durulma {:.1} ms, sonraki iki kare {:.1} ms ve {:.1} ms; ölçünün yazısı {text:?}",
        written.as_secs_f64() * 1000.0,
        first.as_secs_f64() * 1000.0,
        second.as_secs_f64() * 1000.0
    );
}
