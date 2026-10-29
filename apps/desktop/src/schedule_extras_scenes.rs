//! The pictures of docs/adr/0733: a furnished flat of three rooms on the ground level, each room with its finishes
//! and its baseboard's; the Donatı çizelgesi beside it grouped by category, a heading and a subtotal for each; the
//! Çizelge ayarları window and Dosyaya yaz on it; the Kaplama çizelgesi with the baseboard's length; Tabloyu düzenle
//! saying what a computed cell is; Metraj's Tam sayılar; Blok ekle writing the active level:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=cizelge-donati,cizelge-ayarlari,cizelge-dosyaya-yaz,cizelge-kaplama,cizelge-hucre,metraj-tam,blok-ekle-kat cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use std::f64::consts::{FRAC_PI_2, PI};

use kentos_commands::{
    CommandResult, ScheduleColumn, ScheduleField, ScheduleOf, ScheduleSort, SchedulesCreate, Vec2,
};
use kentos_domain::Slot;
use serde_json::json;

use crate::app::App;
use crate::files_testing::make_cad;
use crate::furniture_scenes::{id_of, library, wall, xy};
use crate::tools_scenes::{E, N, Objects, hover, layer, open_edited, run};
use crate::tools_screens::Scene;

/// A room drawn on the wall faces, on the ground level, with its finishes.
fn room(o: &mut Objects, pts: &[(f64, f64)], number: &str, name: &str, finishes: [&str; 4]) {
    o.push(
        "mahal",
        json!({
            "kind": "space",
            "parts": [{ "outer": { "pts": pts.iter().map(|&(x, y)| xy(x, y)).collect::<Vec<_>>() } }],
            "number": number, "name": name, "arch": { "level": "zemin" }, "height": 2.8,
            "finishes": { "floor": finishes[0], "wall": finishes[1], "ceiling": finishes[2], "base": finishes[3] },
        }),
    );
}

/// The 9 × 6 m flat of docs/adr/0702 with its bedroom, kitchen and bathroom furnished on the ground level, and its three
/// rooms with their finishes.
fn rooms(app: &mut App) {
    let mut o = Objects::new();
    wall(
        &mut o,
        &[(-0.125, -0.125), (9.125, -0.125), (9.125, 6.125), (-0.125, 6.125)],
        0.25,
        true,
    );
    wall(&mut o, &[(0.0, 3.0), (4.0, 3.0), (4.0, 6.0)], 0.1, false);
    wall(&mut o, &[(6.0, 6.0), (6.0, 3.5), (9.0, 3.5)], 0.1, false);
    let mut put = |layer: &str, name: &str, at: [f64; 2], turn: f64| {
        let i = o.insert(layer, &id_of(name), at, 1.0, turn, false);
        o.fields(i, json!({ "arch": { "level": "zemin" } }));
    };
    put("mobilya", "Çift kişilik yatak", [2.8, 6.0], PI);
    put("mobilya", "Komodin", [1.1, 6.0], PI);
    put("mobilya", "Komodin", [3.4, 6.0], PI);
    put("mutfak", "Alt dolap", [6.0, 0.0], 0.0);
    put("mutfak", "Evye", [6.6, 0.0], 0.0);
    put("mutfak", "Ocak", [8.3, 0.0], 0.0);
    put("mutfak", "Buzdolabı", [9.0, 1.3], FRAC_PI_2);
    put("islak", "Köşe duş", [9.0, 6.0], PI);
    put("islak", "Asma klozet", [7.4, 6.0], PI);
    put("islak", "Dolaplı lavabo", [6.05, 4.6], -FRAC_PI_2);
    let dry = ["Laminat parke", "Saten boya", "Alçı", "Ahşap süpürgelik"];
    room(&mut o, &[(0.0, 3.05), (3.95, 3.05), (3.95, 6.0), (0.0, 6.0)], "Z-01", "Yatak odası", dry);
    room(
        &mut o,
        &[(0.0, 0.0), (9.0, 0.0), (9.0, 3.45), (5.95, 3.45), (5.95, 6.0), (4.05, 6.0), (4.05, 2.95), (0.0, 2.95)],
        "Z-02",
        "Salon ve mutfak",
        dry,
    );
    room(
        &mut o,
        &[(6.05, 3.55), (9.0, 3.55), (9.0, 6.0), (6.05, 6.0)],
        "Z-03",
        "Banyo",
        ["Seramik", "Seramik", "Alçıpan", "Seramik süpürgelik"],
    );
    open_edited(app, o, |d| {
        d["layers"] = json!([
            layer("cizim", "Çizim", "fg", 0.25),
            layer("duvar", "Duvar", "fg", 0.5),
            layer("mahal", "Mahal", "#8E6CDB", 0.18),
            layer("mobilya", "Mobilya", "#3BA55D", 0.25),
            layer("mutfak", "Mutfak", "#C8A165", 0.25),
            layer("islak", "Islak hacim", "#12A594", 0.25),
        ]);
        d["blocks"] = library();
        d["settings"]["plotScale"] = json!(50);
        d["settings"]["arch"] = json!({ "levels": [{ "id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0 }] });
    });
    make_cad(app);
    view(app, 21.5);
}

fn view(app: &mut App, right: f64) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 0.6,
            min_y: N - 1.6,
            max_x: E + right,
            max_y: N + 6.6,
        },
        24.0,
    );
}

/// A schedule of `of` placed right of the flat: its uid and slot.
fn schedule(app: &mut App, of: ScheduleOf, columns: Option<Vec<ScheduleColumn>>, sort: Vec<ScheduleSort>) -> Option<(String, Slot)> {
    let doc = app.document.as_mut()?;
    let made = kentos_native_application::schedules_create::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        SchedulesCreate {
            layer_id: "cizim".into(),
            at: Vec2 { x: E + 10.0, y: N + 6.0 },
            of,
            columns,
            filter: None,
            sort,
            group: Some(true),
            total: Some(true),
            height: None,
            color: None,
            line_weight: None,
            expected_revision: None,
        },
    );
    match made {
        CommandResult::Completed { output, .. } => Some((output.uid, Slot(output.id))),
        _ => None,
    }
}

/// The fixtures by category: a heading and a subtotal for each, by type within.
fn fixtures(app: &mut App) -> Option<(String, Slot)> {
    rooms(app);
    let by = |column, ends| ScheduleSort { column, descending: false, header: ends, footer: ends };
    let made = schedule(app, ScheduleOf::Fixtures, None, vec![by(1, true), by(0, false)]);
    hover(app, [9.6, -1.2]);
    made
}

fn fixtures_shown(app: &mut App) {
    fixtures(app);
}

fn settings(app: &mut App) {
    if let Some((uid, _)) = fixtures(app) {
        let _ = app.schedule_event(crate::schedule_window::Event::Open(uid));
    }
}

fn export(app: &mut App) {
    if let Some((uid, _)) = fixtures(app) {
        let _ = app.schedule_event(crate::schedule_window::Event::Export(uid));
    }
}

/// The rooms' finishes and their baseboards' lengths.
fn finishes(app: &mut App) {
    rooms(app);
    let column = |field| ScheduleColumn { field, title: None, attr: None };
    let columns = [
        ScheduleField::Number,
        ScheduleField::Name,
        ScheduleField::FloorFinish,
        ScheduleField::BaseFinish,
        ScheduleField::WallFinish,
        ScheduleField::CeilingFinish,
        ScheduleField::Area,
        ScheduleField::Baseboard,
    ]
    .map(column)
    .to_vec();
    let made = schedule(app, ScheduleOf::Spaces, Some(columns), Vec::new());
    // The table close enough to read, the bathroom's corner beside it.
    let table = made.and_then(|(_, slot)| {
        let doc = app.document.as_ref()?;
        match doc.model.entities().find(|e| e.base().id == slot.0)? {
            kentos_commands::Entity::Table(t) => Some((t.p, t.columns.iter().sum::<f64>(), t.rows.iter().sum::<f64>())),
            _ => None,
        }
    });
    if let Some((p, w, h)) = table {
        app.viewport.camera.fit(
            &kentos_render_wgpu::Bounds { min_x: p.x - 2.2, min_y: p.y - h - 0.6, max_x: p.x + w + 0.3, max_y: p.y + 0.4 },
            24.0,
        );
    }
    hover(app, [9.6, -1.2]);
}

/// Tabloyu düzenle on the fixtures schedule, its count of a data row active: the status line says it is computed.
fn cell(app: &mut App) {
    if let Some((_, slot)) = fixtures(app) {
        let _ = app.open_table_editor(slot);
        let _ = app.table_editor_event(crate::tables::editor::Event::Cell(2, 4));
    }
}

/// Metraj with Tam sayılar on.
fn exact(app: &mut App) {
    rooms(app);
    app.open_quantities();
    let _ = app.quantities_event(crate::quantities::Event::Exact(true));
}

/// Blok ekle on the ground level: its Kat row says where the fixture goes.
fn insert_level(app: &mut App) {
    rooms(app);
    view(app, 9.6);
    app.command_expanded = true;
    run(app, "tool.blockInsert");
    hover(app, [2.0, 1.5]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("cizelge-donati", fixtures_shown),
        ("cizelge-ayarlari", settings),
        ("cizelge-dosyaya-yaz", export),
        ("cizelge-kaplama", finishes),
        ("cizelge-hucre", cell),
        ("metraj-tam", exact),
        ("blok-ekle-kat", insert_level),
    ]
}
