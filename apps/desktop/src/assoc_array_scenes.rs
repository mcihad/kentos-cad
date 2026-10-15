//! The pictures of docs/adr/0300 (associative arrays) in a CAD project: a
//! road's centreline and a street light drawn at its start (a pole, its arm
//! and lamp), made a path array of eight lights by `cad.entities.array` with
//! `associative`; one item selected, Öznitelikler naming its block “Dizi 1”.
//! Then one light removed and one moved by hand, and the road's end pulled
//! north-east: the other lights follow the road in the same step, the moved
//! one stays where it was put, the removed one stays removed. `tools_screens`
//! takes them in the dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=dizi-iliskili,dizi-iliskili-izler cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! And Diziyi düzenle (docs/adr/0301): the road's lights picked by one of them, the prompt with the array's options;
//! Adet 12 written; a grid of trees whose first place follows the mouse after Taban (the command's plan drawn):
//!
//! ```text
//! KENTOS_SHOTS_ONLY=dizi-duzenle,dizi-duzenle-adet,dizi-duzenle-taban cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! And docs/adr/0302: Yol boyunca dizi with İlişkili on, the light's three objects along the road; one light of the
//! array selected and Patlat: every light comes apart into its objects, the array is gone:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=dizi-iliskili-secenek,dizi-patlat cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_contracts::{ArrayLayout, CommandResult, EntitiesArray, Entity, Vec2};
use kentos_domain::Slot;
use kentos_native_application::{ExecutionContext, array};

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::Scene;

/// The road's centreline (1) and the light at its start: the pole (2), the arm (3), the lamp (4).
fn ground() -> Objects {
    let mut o = Objects::new();
    let quarter = (std::f64::consts::PI / 8.0).tan();
    o.bulged(
        "yol",
        &[[0.0, 0.0], [40.0, 0.0], [52.0, 12.0], [52.0, 40.0]],
        &[0.0, quarter, 0.0, 0.0],
    );
    o.circle("cizim", [0.0, 0.0], 0.4);
    o.line("cizim", [0.0, 0.4], [0.0, 3.0]);
    o.path(
        "cizim",
        &[[-0.9, 3.0], [0.9, 3.0], [0.6, 3.6], [-0.6, 3.6]],
        true,
    );
    o
}

/// The drawing in a CAD project, the light made an associative path array of eight along the road.
fn arrayed(app: &mut App) -> Vec<String> {
    open(app, ground());
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 4.0,
            min_y: N - 8.0,
            max_x: E + 74.0,
            max_y: N + 50.0,
        },
        24.0,
    );
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let uid = |s: u32| doc.uid(Slot(s)).expect("an object").to_string();
    let input = EntitiesArray {
        uids: vec![uid(2), uid(3), uid(4)],
        layout: ArrayLayout::Path {
            path: uid(1),
            count: 8,
            spacing: None,
            align: true,
        },
        associative: true,
        expected_revision: None,
    };
    match array::execute(&mut ExecutionContext::new(doc), input) {
        CommandResult::Completed { output, .. } => output.created,
        other => panic!("not written: {other:?}"),
    }
}

/// The array written, its fourth light selected.
fn made(app: &mut App) {
    let items = arrayed(app);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let fourth = doc
        .slot_of(items[3].parse().expect("an id"))
        .and_then(|s| doc.get(s).cloned());
    if let Some(Entity::Insert(i)) = fourth {
        click(app, [i.p.x - E, i.p.y - N]);
    }
    hover(app, [64.0, 4.0]);
}

/// The third light removed, the sixth moved 3 m east by hand, then the road's end pulled to (70, 46): the
/// others follow it.
fn followed(app: &mut App) {
    let items = arrayed(app);
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let slot = |doc: &kentos_domain::Document, u: &str| {
        doc.slot_of(u.parse().expect("an id")).expect("an item")
    };
    let third = slot(doc, &items[2]);
    doc.remove(&[third]);
    let sixth = slot(doc, &items[5]);
    if let Some(Entity::Insert(mut i)) = doc.get(sixth).cloned() {
        i.p = Vec2 {
            x: i.p.x + 3.0,
            y: i.p.y,
        };
        doc.update(sixth, Entity::Insert(i));
    }
    if let Some(Entity::Polyline(mut road)) = doc.get(Slot(1)).cloned() {
        if let Some(end) = road.pts.last_mut() {
            *end = Vec2 {
                x: E + 70.0,
                y: N + 46.0,
            };
        }
        doc.update(Slot(1), Entity::Polyline(road));
    }
    click(app, [20.0, 0.0]);
    hover(app, [64.0, 4.0]);
}

/// Where the insert of `items[k]` is, from the drawing's origin.
fn item_at(app: &App, items: &[String], k: usize) -> [f64; 2] {
    let doc = &app.document.as_ref().expect("a drawing").model;
    match doc
        .slot_of(items[k].parse().expect("an id"))
        .and_then(|s| doc.get(s).cloned())
    {
        Some(Entity::Insert(i)) => [i.p.x - E, i.p.y - N],
        _ => panic!("an insert"),
    }
}

/// Diziyi düzenle on the road's lights, the fourth picked: its pole's circle is the base point.
fn edit_picked(app: &mut App) {
    let items = arrayed(app);
    run(app, "tool.arrayEdit");
    let at = item_at(app, &items, 3);
    click(app, [at[0] + 0.4, at[1]]);
    hover(app, [64.0, 4.0]);
}

/// Adet 12: the lights spread again along the road, four more.
fn edit_count(app: &mut App) {
    edit_picked(app);
    typed(app, "D");
    typed(app, "12");
    hover(app, [64.0, 4.0]);
}

/// A tree: a crown (a circle) over a trunk mark.
fn trees() -> Objects {
    let mut o = Objects::new();
    o.circle("cizim", [10.0, 10.0], 2.0);
    o.line("cizim", [9.0, 10.0], [11.0, 10.0]);
    o.line("cizim", [10.0, 9.0], [10.0, 11.0]);
    o
}

/// A 3 × 5 grid of trees, Taban asked: the first place at the mouse, the plan drawn.
fn edit_base(app: &mut App) {
    open(app, trees());
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 4.0,
            min_y: N - 6.0,
            max_x: E + 74.0,
            max_y: N + 44.0,
        },
        24.0,
    );
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let uid = |s: u32| doc.uid(Slot(s)).expect("an object").to_string();
    let input = EntitiesArray {
        uids: vec![uid(1), uid(2), uid(3)],
        layout: ArrayLayout::Grid {
            rows: 3,
            cols: 5,
            dx: 8.0,
            dy: 8.0,
        },
        associative: true,
        expected_revision: None,
    };
    let CommandResult::Completed { output, .. } =
        array::execute(&mut ExecutionContext::new(doc), input)
    else {
        panic!("not written")
    };
    run(app, "tool.arrayEdit");
    let at = item_at(app, &output.created, 7);
    click(app, [at[0] + 2.0, at[1]]);
    typed(app, "B");
    hover(app, [26.0, 4.0]);
}

/// Yol boyunca dizi on the light's objects, the road picked, İlişkili turned on: the copies shown along it.
fn path_associative(app: &mut App) {
    open(app, ground());
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 4.0,
            min_y: N - 8.0,
            max_x: E + 74.0,
            max_y: N + 50.0,
        },
        24.0,
    );
    app.selection.set([Slot(2), Slot(3), Slot(4)]);
    run(app, "tool.arrayPath");
    click(app, [20.0, 0.0]);
    typed(app, "İ");
    typed(app, "8");
    hover(app, [64.0, 4.0]);
}

/// The road's lights, one selected, exploded: the whole array comes apart.
fn exploded(app: &mut App) {
    let items = arrayed(app);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let fourth = doc
        .slot_of(items[3].parse().expect("an id"))
        .expect("an item");
    app.selection.set([fourth]);
    run(app, "tool.explode");
    hover(app, [64.0, 4.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("dizi-iliskili", made),
        ("dizi-iliskili-izler", followed),
        ("dizi-duzenle", edit_picked),
        ("dizi-duzenle-adet", edit_count),
        ("dizi-duzenle-taban", edit_base),
        ("dizi-iliskili-secenek", path_associative),
        ("dizi-patlat", exploded),
    ]
}
