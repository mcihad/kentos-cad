//! The pictures of docs/adr/0318: KentOS's own block library placed, its dynamic blocks at several values (the door
//! at its defaults, wider and left-handed, opening outwards and without its swing; the window and the table stretched;
//! the wardrobe's doors arrayed along its width; the section mark turned), and the Bloklar panel's cards of the
//! library, all and by category. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=blok-dinamik,blok-kitaplik,blok-kitaplik-kategori cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! And of docs/adr/0319: the door's parameter grips and a width dragged (blok-tutamac), its values in Öznitelikler
//! (blok-parametreler) and two doors made plain blocks (blok-sabit):
//!
//! ```text
//! KENTOS_SHOTS_ONLY=blok-tutamac,blok-parametreler,blok-sabit cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, at, hover, layer, open_edited, run};
use crate::tools_screens::Scene;

const LIBRARY: &str = include_str!("../../../catalog/v1/blocks/kentos-temel.kblok");

fn library() -> Value {
    let file: Value = serde_json::from_str(LIBRARY).expect("the library");
    file["blocks"].clone()
}

/// A block of the library by its name.
fn id_of(name: &str) -> String {
    library()
        .as_array()
        .expect("blocks")
        .iter()
        .find(|b| b["name"] == name)
        .and_then(|b| b["id"].as_str())
        .expect("a block of that name")
        .to_owned()
}

fn drawn(app: &mut App) {
    let mut o = Objects::new();
    let put = |o: &mut Objects, layer: &str, name: &str, at: [f64; 2], params: Value| {
        let i = o.insert(layer, &id_of(name), at, 1.0, 0.0, false);
        if !params.is_null() {
            o.fields(i, json!({ "parameters": params }));
        }
    };
    // Doors along a wall: as drawn, wider and left-handed, opening outwards, without the swing.
    put(&mut o, "kapilar", "Kapı", [0.0, 0.0], Value::Null);
    put(
        &mut o,
        "kapilar",
        "Kapı",
        [3.2, 0.0],
        json!({ "Genişlik": 1.2, "Sol el": true }),
    );
    put(
        &mut o,
        "kapilar",
        "Kapı",
        [4.4, 0.0],
        json!({ "Dışa açılır": true }),
    );
    put(
        &mut o,
        "kapilar",
        "Kapı",
        [6.0, 0.0],
        json!({ "Görünüş": "Sade", "Genişlik": 0.7 }),
    );
    // Windows: as drawn and stretched.
    put(&mut o, "pencereler", "Pencere", [8.0, 0.0], Value::Null);
    put(
        &mut o,
        "pencereler",
        "Pencere",
        [9.8, 0.0],
        json!({ "Genişlik": 2.4 }),
    );
    // Furniture: the table and the wardrobe at two widths.
    put(&mut o, "mobilya", "Masa", [0.0, 2.0], Value::Null);
    put(
        &mut o,
        "mobilya",
        "Masa",
        [2.2, 2.0],
        json!({ "Uzunluk": 2.4 }),
    );
    put(&mut o, "mobilya", "Dolap", [5.4, 2.0], Value::Null);
    put(
        &mut o,
        "mobilya",
        "Dolap",
        [7.2, 2.0],
        json!({ "Genişlik": 2.4 }),
    );
    put(
        &mut o,
        "mobilya",
        "Çift kişilik yatak",
        [10.2, 2.0],
        Value::Null,
    );
    // The wet room, marks and a title block at 1:50.
    put(&mut o, "islak", "Lavabo", [0.5, 5.0], Value::Null);
    put(&mut o, "islak", "Klozet", [1.6, 5.0], Value::Null);
    put(&mut o, "islak", "Duş teknesi", [2.4, 5.0], Value::Null);
    put(&mut o, "islak", "Küvet", [3.6, 5.0], Value::Null);
    put(&mut o, "isaret", "Kot işareti", [6.0, 5.4], Value::Null);
    put(&mut o, "isaret", "Kuzey oku", [7.8, 5.6], Value::Null);
    put(&mut o, "isaret", "Kesit işareti", [9.0, 5.0], Value::Null);
    put(
        &mut o,
        "isaret",
        "Kesit işareti",
        [10.2, 5.0],
        json!({ "Ters yön": true }),
    );
    let title = o.insert("isaret", &id_of("Antet"), [1.5, -4.0], 50.0, 0.0, false);
    o.fields(
        title,
        json!({ "attrs": { "PROJE": "Örnek konut", "TARİH": "16.10.2026", "ÇİZEN": "KentOS" } }),
    );
    for (x, y, words) in [
        (0.0, -0.6, "Kapı"),
        (2.0, -0.6, "Genişlik 1.2, sol el"),
        (4.4, -0.6, "Dışa açılır"),
        (6.0, -0.6, "Sade, 0.7"),
        (9.8, -0.6, "Pencere 2.4"),
        (7.2, 1.6, "Dolap 2.4: kapılar dizilir"),
    ] {
        o.text("cizim", [x, y], words, 0.18, 0.0);
    }
    open_edited(app, o, |d| {
        d["layers"] = json!([
            layer("cizim", "Çizim", "fg", 0.25),
            layer("kapilar", "Kapılar", "#C8A165", 0.35),
            layer("pencereler", "Pencereler", "#3E63DD", 0.35),
            layer("mobilya", "Mobilya", "#3BA55D", 0.25),
            layer("islak", "Islak hacim", "#12A594", 0.25),
            layer("isaret", "İşaretler", "#E5484D", 0.25),
        ]);
        d["blocks"] = library();
        d["settings"]["plotScale"] = json!(50);
    });
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 0.5,
            min_y: N - 4.2,
            max_x: E + 12.8,
            max_y: N + 6.6,
        },
        24.0,
    );
}

/// The library's blocks drawn at several values.
fn dynamic(app: &mut App) {
    drawn(app);
}

/// The Bloklar panel's cards of the library.
fn cards(app: &mut App) {
    drawn(app);
    run(app, "block.panel");
    let _ = app.update(Message::BlocksPanel(crate::blocks_panel::Event::Cards(
        true,
    )));
}

/// Its cards of one category.
fn category(app: &mut App) {
    cards(app);
    let _ = app.update(Message::BlocksPanel(crate::blocks_panel::Event::Category(
        Some("Islak hacim".into()),
    )));
}

/// The view around the first door, its hinge at the origin.
fn near_the_door(app: &mut App) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 0.9,
            min_y: N - 0.9,
            max_x: E + 2.1,
            max_y: N + 1.5,
        },
        24.0,
    );
}

/// The first door selected: its parameter grips (width, the two flips); its width's grip clicked and taken to 1.13 m,
/// which its rule makes 1.15, the door drawn so dashed.
fn grips(app: &mut App) {
    drawn(app);
    near_the_door(app);
    app.selection.set([kentos_domain::Slot(1)]);
    let grip = at(app, [0.9, 0.0]);
    let _ = app.update(Message::Viewport(crate::viewport::Event::Moved(grip)));
    let _ = app.update(Message::Viewport(crate::viewport::Event::Pressed(grip)));
    let _ = app.update(Message::Viewport(crate::viewport::Event::Released(grip)));
    hover(app, [1.13, 0.12]);
}

/// The wider, left-handed door selected: Öznitelikler's Blok parametreleri.
fn values(app: &mut App) {
    drawn(app);
    app.selection.set([kentos_domain::Slot(2)]);
    // The layers folded and Genel folded: the geometry and the parameters in view.
    let upper =
        kentos_ui::widget::docking::Slot::Docked(kentos_ui::widget::docking::Side::Right, 0);
    let _ = app.update(Message::Dock(kentos_ui::widget::docking::Event::Collapsed(
        upper, true,
    )));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "general",
    )));
}

/// The second and third doors made plain blocks: the Bloklar panel lists “Kapı (sabit)” and “Kapı (sabit 2)”.
fn frozen(app: &mut App) {
    drawn(app);
    app.selection
        .set([kentos_domain::Slot(2), kentos_domain::Slot(3)]);
    run(app, "block.freeze");
    run(app, "block.panel");
    let _ = app.update(Message::BlocksPanel(crate::blocks_panel::Event::Search(
        "kapı".into(),
    )));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("blok-dinamik", dynamic),
        ("blok-kitaplik", cards),
        ("blok-kitaplik-kategori", category),
        ("blok-tutamac", grips),
        ("blok-parametreler", values),
        ("blok-sabit", frozen),
    ]
}
