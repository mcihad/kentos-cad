//! The pictures and the acceptance of docs/adr/0773 (Bina programı): a 2+1 plan of two flats on two levels — flat 1 a
//! duplex (its bedroom upstairs) with an open kitchen by a room separator and a balcony, flat 2 a bedroom, a bathroom
//! and a living room — a shared corridor and a shaft, the levels side by side. The plan with its tags (group, target,
//! difference); Bina programı's three parts after Denetle and while it runs; the corridor's partition moved 10 cm (its
//! passage below the brief, flat 2's bedroom grown); Öznitelikler's Bölüm and target rows; Mahali böl saying its decision
//! before writing; Mahalleri birleştir asking with its decision. `tools_screens` takes them in both themes at 1440×900
//! and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=bina-programi-plan,bina-programi-bolumler,bina-programi-hedefler,bina-programi-durum,bina-programi-denetleniyor,bina-programi-kaydirildi,bina-programi-oznitelik,bina-programi-bol,bina-programi-birlestir cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! docs/adr/0795 (Bölüm çizelgesi): the shares' house with a units schedule beside it (number, name, kind, wing,
//! levels, net, share, gross, recorded area, spaces; the total), the tool placing one, Çizelge ayarları with the units'
//! filter, Tabloyu düzenle on a name saying it goes to the program:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=bolum-cizelgesi-plan,bolum-cizelgesi-arac,bolum-cizelgesi-ayarlar,bolum-cizelgesi-duzenle cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! docs/adr/8072 §1, §4 (two blocks over a shared ground floor): Binalar with each block's code, use and ±0.00 and the
//! shared level's building; Metraj on A Blok's storeys; Ruhsat takımı with B Blok chosen:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=bina-ortak-kat,bina-metraj,bina-ruhsat cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.
// The core's test kit (`testing`, docs/adr/9199 §4.11): a module package's tests use its helpers; what only the
// core's tests use is left unused there.
#![cfg_attr(not(test), allow(dead_code, unused_imports))]


use kentos_commands::Entity;
use kentos_domain::Slot;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::tools_scenes::{E, N, Objects, click, hover, run};
use crate::tools_screens::Scene;
use crate::wall_scenes::opened;

/// Where the upper floor is drawn: beside the ground floor.
pub const UP: f64 = 16.0;

pub(crate) use crate::files_testing::command;

fn p(x: f64, y: f64) -> Value {
    json!({"x": E + x, "y": N + y})
}

pub fn uids(answer: &Value) -> Vec<String> {
    answer["output"]["uids"].as_array().expect("ids").iter().map(|u| u.as_str().expect("an id").to_owned()).collect()
}

pub fn slot(app: &App, uid: &str) -> Slot {
    let model = &app.document.as_ref().expect("a drawing").model;
    model.slot_of(kentos_domain::Uuid::parse_str(uid).expect("an id")).expect("its slot")
}

/// The plan's ids: its walls (by name), its spaces (by number).
pub struct Plan {
    pub walls: Vec<String>,
    spaces: Vec<(String, String)>,
}

impl Plan {
    pub fn space(&self, number: &str) -> &str {
        &self.spaces.iter().find(|(n, _)| n == number).expect("the space").1
    }
}

/// The program, the walls, the doors, the separator, the spaces and their tags, written through the commands.
fn build(app: &mut App) -> Plan {
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    let mut arch = settings.arch.clone().unwrap_or_default();
    arch.levels = vec![
        kentos_commands::ArchLevel { id: "zemin".into(), name: "Zemin kat".into(), elevation: 0.0, height: Some(3.0) },
        kentos_commands::ArchLevel { id: "kat-1".into(), name: "1. kat".into(), elevation: 3.0, height: Some(3.0) },
    ];
    settings.arch = Some(arch);
    doc.model.set_settings(settings);
    let layer = doc.model.layers().active().to_owned();
    command(
        app,
        "cad.program.edit",
        json!({"program": {
            "buildings": [{"id": "a", "name": "A Blok"}],
            "units": [
                {"id": "d1", "number": "1", "name": "Daire 1", "building": "a", "recordedArea": 60.0},
                {"id": "d2", "number": "2", "name": "Daire 2", "building": "a"},
                {"id": "ortak", "number": "O", "name": "Ortak alan", "kind": "common", "building": "a"}
            ],
            "briefs": [
                {"usage": "Yatak odası", "areaMin": 12.0, "areaMax": 15.0, "minWidth": 2.8, "reach": ["Salon"]},
                {"usage": "Salon", "areaMin": 18.0, "areaMax": 30.0, "adjacent": ["Mutfak"]},
                {"usage": "Mutfak", "areaMin": 6.0, "areaMax": 12.0, "reach": ["Salon"]},
                {"usage": "Koridor", "minWidth": 1.1, "minPassage": 1.2},
                {"usage": "Banyo", "areaMin": 4.0, "areaMax": 8.0},
                {"usage": "Balkon", "outdoor": true}
            ],
            "version": "Ofis konut hedefleri 2026-1"
        }}),
    );
    let wall = |a: (f64, f64), b: (f64, f64), level: &str| {
        json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": 0.2, "arch": {"level": level}})
    };
    let walls = command(
        app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [
            wall((0.0, 0.0), (13.0, 0.0), "zemin"), wall((13.0, 0.0), (13.0, 8.0), "zemin"),
            wall((13.0, 8.0), (0.0, 8.0), "zemin"), wall((0.0, 8.0), (0.0, 0.0), "zemin"),
            wall((5.0, 0.0), (5.0, 8.0), "zemin"), wall((6.4, 0.0), (6.4, 8.0), "zemin"),
            wall((0.0, 3.4), (5.0, 3.4), "zemin"), wall((6.4, 4.5), (13.0, 4.5), "zemin"),
            wall((10.0, 0.0), (10.0, 4.5), "zemin"), wall((5.0, 7.0), (6.4, 7.0), "zemin"),
            wall((UP, 0.0), (UP + 5.0, 0.0), "kat-1"), wall((UP + 5.0, 0.0), (UP + 5.0, 4.0), "kat-1"),
            wall((UP + 5.0, 4.0), (UP, 4.0), "kat-1"), wall((UP, 4.0), (UP, 0.0), "kat-1"),
            wall((UP + 3.0, 0.0), (UP + 3.0, 4.0), "kat-1")
        ]}),
    );
    let w = uids(&walls);
    let door = |k: usize, at: f64, width: f64| json!({"host": w[k], "at": at, "form": "door", "width": width});
    command(
        app,
        "cad.openings.create",
        json!({"layerId": layer, "openings": [
            door(4, 1.5, 0.9), door(4, 6.0, 0.9), door(5, 2.0, 0.9), door(5, 6.0, 0.9), door(8, 2.0, 0.9),
            door(0, 5.7, 1.0), door(2, 11.5, 0.9), door(6, 1.5, 0.9), door(14, 2.0, 0.9)
        ]}),
    );
    // The open kitchen: a room separator between the living room and the kitchen.
    command(
        app,
        "cad.spaces.separators",
        json!({"operation": {"kind": "create", "layerId": layer, "paths": [
            {"pts": [p(3.0, 3.5), p(3.0, 7.9)], "arch": {"level": "zemin"}}
        ]}}),
    );
    let seeded = |x: f64, y: f64, level: &str, unit: &str, usage: &str, number: &str| {
        json!({"seed": p(x, y), "arch": {"level": level}, "unit": unit, "usage": usage, "name": usage, "number": number})
    };
    let mut spaces = vec![
        seeded(2.0, 1.5, "zemin", "d1", "Yatak odası", "101"),
        seeded(1.5, 5.5, "zemin", "d1", "Salon", "102"),
        seeded(4.0, 5.5, "zemin", "d1", "Mutfak", "103"),
        seeded(5.7, 3.0, "zemin", "ortak", "Koridor", "001"),
        seeded(8.0, 2.0, "zemin", "d2", "Yatak odası", "201"),
        seeded(11.5, 2.0, "zemin", "d2", "Banyo", "202"),
        seeded(9.5, 6.0, "zemin", "d2", "Salon", "203"),
        seeded(UP + 1.5, 2.0, "kat-1", "d1", "Yatak odası", "105"),
        seeded(UP + 4.0, 2.0, "kat-1", "d1", "Hol", "106"),
    ];
    let mut shaft = seeded(5.7, 7.5, "zemin", "ortak", "Şaft", "002");
    shaft["void"] = json!(true);
    spaces.push(shaft);
    spaces.push(json!({"parts": [{"outer": {"pts": [p(0.1, 8.1), p(3.0, 8.1), p(3.0, 9.5), p(0.1, 9.5)]}}],
                       "arch": {"level": "zemin"}, "unit": "d1", "usage": "Balkon", "name": "Balkon", "number": "104"}));
    let numbers: Vec<String> =
        spaces.iter().map(|s| s["number"].as_str().expect("a number").to_owned()).collect();
    let made = command(app, "cad.spaces.create", json!({"layerId": layer, "spaces": spaces}));
    let ids = uids(&made);
    let tags: Vec<Value> = ids
        .iter()
        .zip(&numbers)
        .filter(|(_, n)| n.as_str() != "002")
        .map(|(u, _)| json!({"of": u, "content": "{number} · {unit}\n{area}\nHedef: {target}\nFark: {targetDiff}"}))
        .collect();
    command(app, "cad.tags.create", json!({"layerId": layer, "tags": tags}));
    app.tab = "arch";
    app.selection.clear();
    Plan { walls: w, spaces: numbers.into_iter().zip(ids).collect() }
}

pub fn fit(app: &mut App, x0: f64, y0: f64, x1: f64, y1: f64) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: E + x0, min_y: N + y0, max_x: E + x1, max_y: N + y1 },
        24.0,
    );
    hover(app, [x1 + 5.0, y0 - 5.0]);
}

pub fn house(app: &mut App) -> Plan {
    opened(app, Objects::new(), 50.0);
    build(app)
}

fn plan(app: &mut App) {
    house(app);
    fit(app, -0.5, -0.5, UP + 5.5, 10.0);
}







/// Katmanlar folded so Öznitelikler's rows show.
fn rows_tall(app: &mut App) {
    let upper = kentos_ui::widget::docking::Slot::Docked(kentos_ui::widget::docking::Side::Right, 0);
    let _ = app.update(Message::Dock(kentos_ui::widget::docking::Event::Collapsed(upper, true)));
}

fn rows(app: &mut App) {
    let plan = house(app);
    let s = slot(app, plan.space("201"));
    app.selection.set([s]);
    rows_tall(app);
    fit(app, -0.5, -0.5, 13.5, 10.0);
}

/// Mahali böl over flat 2's living room: the decision said before writing.
fn split(app: &mut App) {
    house(app);
    fit(app, 5.0, -0.5, 13.5, 9.0);
    run(app, "tool.spaceSplit");
    click(app, [9.5, 6.0]);
    click(app, [10.0, 4.4]);
    hover(app, [10.0, 8.1]);
}


/// The shares' program (docs/adr/0784): A Blok on both levels, its west wing both, its east wing the ground floor; flat 1
/// west, flat 2 east, the corridor A Blok's common area; `rule` and its weights.
fn shared(app: &mut App, rule: &str) {
    let weights = rule == "weight";
    command(
        app,
        "cad.program.edit",
        json!({"program": {
            "buildings": [{"id": "a", "name": "A Blok", "levels": ["zemin", "kat-1"], "wings": [
                {"id": "a-bati", "name": "Batı kanadı", "levels": ["zemin", "kat-1"]},
                {"id": "a-dogu", "name": "Doğu kanadı", "levels": ["zemin"]}
            ]}, {"id": "b", "name": "B Blok"}],
            "units": [
                {"id": "d1", "number": "1", "name": "Daire 1", "building": "a", "wing": "a-bati", "recordedArea": 60.0,
                 "shareWeight": if weights { json!("60") } else { Value::Null }},
                {"id": "d2", "number": "2", "name": "Daire 2", "building": "a", "wing": "a-dogu",
                 "shareWeight": if weights { json!("40") } else { Value::Null }},
                {"id": "ortak", "number": "O", "name": "Ortak alan", "kind": "common", "building": "a"}
            ],
            "briefs": [
                {"usage": "Yatak odası", "areaMin": 12.0, "areaMax": 15.0, "minWidth": 2.8, "reach": ["Salon"]},
                {"usage": "Salon", "areaMin": 18.0, "areaMax": 30.0, "adjacent": ["Mutfak"]},
                {"usage": "Mutfak", "areaMin": 6.0, "areaMax": 12.0, "reach": ["Salon"]},
                {"usage": "Koridor", "minWidth": 1.1, "minPassage": 1.2},
                {"usage": "Banyo", "areaMin": 4.0, "areaMax": 8.0},
                {"usage": "Balkon", "outdoor": true}
            ],
            "version": "Ofis konut hedefleri 2026-1",
            "shareRule": rule
        }}),
    );
}

/// The program with its shares and the flats' tags saying their share and gross.
pub fn share_house(app: &mut App, rule: &str) -> Plan {
    let plan = house(app);
    shared(app, rule);
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let tags: Vec<Value> = ["102", "203"]
        .iter()
        .map(|n| json!({"of": plan.space(n), "content": "{unit}\nPay {share}\nBrüt {gross}"}))
        .collect();
    command(app, "cad.tags.create", json!({"layerId": layer, "tags": tags}));
    app.selection.clear();
    plan
}








fn share_plan(app: &mut App) {
    share_house(app, "net");
    fit(app, -0.5, -0.5, 13.5, 10.0);
}


/// A units schedule (docs/adr/0795) right of the plan: its uid and slot.
pub fn unit_schedule(app: &mut App) -> (String, Slot) {
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let columns: Vec<Value> = ["number", "name", "unitKind", "wing", "level", "area", "share", "gross", "recordedArea", "count"]
        .iter()
        .map(|f| json!({"field": f}))
        .collect();
    let made = command(
        app,
        "cad.schedules.create",
        json!({"layerId": layer, "at": p(UP + 7.0, 9.5), "of": "units", "columns": columns, "total": true}),
    );
    let uid = made["output"]["uid"].as_str().expect("its id").to_owned();
    let s = slot(app, &uid);
    app.selection.clear();
    (uid, s)
}

/// The schedule close enough to read, the upper floor beside it.
fn units_plan(app: &mut App) {
    share_house(app, "net");
    let (_, s) = unit_schedule(app);
    let table = match app.document.as_ref().expect("a drawing").model.get(s) {
        Some(Entity::Table(t)) => (t.p, t.columns.iter().sum::<f64>(), t.rows.iter().sum::<f64>()),
        _ => panic!("the schedule"),
    };
    let (q, w, h) = table;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: E + UP - 0.5, min_y: q.y - h - 2.0, max_x: q.x + w + 0.3, max_y: q.y + 0.4 },
        24.0,
    );
    hover(app, [UP + 1.0, -1.0]);
}

/// Bölüm çizelgesi: its ghost at the cursor, its settings visible.
fn units_tool(app: &mut App) {
    share_house(app, "net");
    fit(app, -0.5, -0.5, UP + 21.0, 10.0);
    app.command_expanded = true;
    run(app, "tool.scheduleUnits");
    hover(app, [UP + 7.0, 9.5]);
}


/// Tabloyu düzenle on flat 2's name: the status line says it goes to the program.
fn units_edit(app: &mut App) {
    share_house(app, "net");
    let (_, s) = unit_schedule(app);
    fit(app, -0.5, -0.5, UP + 21.0, 10.0);
    let _ = app.open_table_editor(s);
    let _ = app.table_editor_event(crate::tables::editor::Event::Cell(2, 1));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("bina-programi-plan", plan),
        ("bina-programi-oznitelik", rows),
        ("bina-programi-bol", split),
        ("bina-pay-plan", share_plan),
        ("bolum-cizelgesi-plan", units_plan),
        ("bolum-cizelgesi-arac", units_tool),
        ("bolum-cizelgesi-duzenle", units_edit),
    ]
}
