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
//! Test code only.

use std::sync::Arc;

use kentos_commands::{Entity, ProgramCheck};
use kentos_domain::Slot;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::program_window::{Event, Tab};
use crate::tools_scenes::{E, N, Objects, click, hover, run};
use crate::tools_screens::Scene;
use crate::wall_scenes::opened;

/// Where the upper floor is drawn: beside the ground floor.
const UP: f64 = 16.0;

pub(crate) fn command(app: &mut App, name: &str, input: Value) -> Value {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

fn p(x: f64, y: f64) -> Value {
    json!({"x": E + x, "y": N + y})
}

fn uids(answer: &Value) -> Vec<String> {
    answer["output"]["uids"].as_array().expect("ids").iter().map(|u| u.as_str().expect("an id").to_owned()).collect()
}

fn slot(app: &App, uid: &str) -> Slot {
    let model = &app.document.as_ref().expect("a drawing").model;
    model.slot_of(kentos_domain::Uuid::parse_str(uid).expect("an id")).expect("its slot")
}

/// The plan's ids: its walls (by name), its spaces (by number).
pub(crate) struct Plan {
    pub(crate) walls: Vec<String>,
    spaces: Vec<(String, String)>,
}

impl Plan {
    fn space(&self, number: &str) -> &str {
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

pub(crate) fn fit(app: &mut App, x0: f64, y0: f64, x1: f64, y1: f64) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: E + x0, min_y: N + y0, max_x: E + x1, max_y: N + y1 },
        24.0,
    );
    hover(app, [x1 + 5.0, y0 - 5.0]);
}

pub(crate) fn house(app: &mut App) -> Plan {
    opened(app, Objects::new(), 50.0);
    build(app)
}

fn plan(app: &mut App) {
    house(app);
    fit(app, -0.5, -0.5, UP + 5.5, 10.0);
}

/// Bina programı open on `tab` after Denetle.
fn window(app: &mut App, tab: Tab) {
    let _ = app.update(Message::Run("arch.program"));
    let _ = app.update(Message::Program(Event::Check));
    let doc = app.document.as_ref().expect("a drawing");
    let result = kentos_native_application::program_check::check_watched(&doc.model, &ProgramCheck::default(), &|_, _| false);
    let id = 1;
    let _ = app.update(Message::Program(Event::Checked(id, Arc::new(result))));
    let _ = app.update(Message::Program(Event::Tab(tab)));
}

fn units(app: &mut App) {
    house(app);
    fit(app, -0.5, -0.5, UP + 5.5, 10.0);
    window(app, Tab::Units);
}

fn briefs(app: &mut App) {
    house(app);
    fit(app, -0.5, -0.5, UP + 5.5, 10.0);
    window(app, Tab::Briefs);
}

fn status(app: &mut App) {
    house(app);
    fit(app, -0.5, -0.5, UP + 5.5, 10.0);
    window(app, Tab::Status);
}

/// Denetle running: its share said, Durdur beside it.
fn running(app: &mut App) {
    house(app);
    fit(app, -0.5, -0.5, UP + 5.5, 10.0);
    let _ = app.update(Message::Run("arch.program"));
    let _ = app.update(Message::Program(Event::Tab(Tab::Status)));
    let _ = app.update(Message::Program(Event::Check));
    let _ = app.update(Message::Program(Event::Progress(1, 9, 22)));
}

/// The corridor's partition 10 cm west: its passage 1.10 m, below the brief; flat 2's bedroom and living room grow.
fn moved(app: &mut App) {
    let plan = house(app);
    command(
        app,
        "cad.entities.transform",
        json!({"uids": [plan.walls[5]], "transform": {"kind": "move", "dx": -0.1, "dy": 0.0}}),
    );
    fit(app, -0.5, -0.5, UP + 5.5, 10.0);
    window(app, Tab::Status);
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

/// Mahalleri birleştir over the living room and the kitchen: asked with its decision.
fn merge(app: &mut App) {
    let plan = house(app);
    let (a, b) = (slot(app, plan.space("102")), slot(app, plan.space("103")));
    app.selection.set([a, b]);
    fit(app, -0.5, -0.5, 13.5, 10.0);
    run(app, "arch.spaceMerge");
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
fn share_house(app: &mut App, rule: &str) -> Plan {
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

fn share_buildings(app: &mut App) {
    share_house(app, "net");
    fit(app, -0.5, -0.5, UP + 5.5, 10.0);
    window(app, Tab::Buildings);
}

fn share_units(app: &mut App) {
    share_house(app, "weight");
    fit(app, -0.5, -0.5, UP + 5.5, 10.0);
    window(app, Tab::Units);
}

fn share_status(app: &mut App) {
    share_house(app, "net");
    fit(app, -0.5, -0.5, UP + 5.5, 10.0);
    window(app, Tab::Status);
}

fn share_plan(app: &mut App) {
    share_house(app, "net");
    fit(app, -0.5, -0.5, 13.5, 10.0);
}

/// Sil on A Blok, whose flats name it: refused, the window says which.
fn share_refused(app: &mut App) {
    share_house(app, "net");
    fit(app, -0.5, -0.5, UP + 5.5, 10.0);
    window(app, Tab::Buildings);
    let _ = app.update(Message::Program(Event::RemoveBuilding(0)));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("bina-programi-plan", plan),
        ("bina-programi-bolumler", units),
        ("bina-programi-hedefler", briefs),
        ("bina-programi-durum", status),
        ("bina-programi-denetleniyor", running),
        ("bina-programi-kaydirildi", moved),
        ("bina-programi-oznitelik", rows),
        ("bina-programi-bol", split),
        ("bina-programi-birlestir", merge),
        ("bina-pay-binalar", share_buildings),
        ("bina-pay-bolumler", share_units),
        ("bina-pay-durum", share_status),
        ("bina-pay-plan", share_plan),
        ("bina-pay-silinmez", share_refused),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::app_with_drawing;

    /// Steps of a decimal “12.35”.
    fn steps(t: &str) -> i64 {
        let (w, f) = t.split_once('.').expect("two decimals");
        w.parse::<i64>().expect("whole") * 100 + f.parse::<i64>().expect("hundredths")
    }

    /// docs/adr/0784: the common area shared out adds up exactly; the tags say the flats' shares; Binalar refuses to
    /// remove a building its flats name and writes a new building, wing and levels through the command.
    #[test]
    fn the_shares_add_up_and_the_buildings_window_writes_through_the_command() {
        let mut app = app_with_drawing();
        let plan = share_house(&mut app, "net");
        let c = checked(&app);
        let sh = c.shares.as_ref().expect("shared");
        let pool = &sh.pools[0];
        assert_eq!(pool.unit, "ortak");
        let given: i64 = pool.shares.iter().map(|s| steps(&s.share)).sum();
        assert_eq!(given, steps(&pool.area), "the shares are the pool, exactly");
        let gross: i64 = c.units.iter().filter_map(|u| u.share.as_ref()).map(|p| steps(&p.gross)).sum();
        assert_eq!(gross, steps(&sh.net) + steps(&sh.shared));
        let d2 = c.units.iter().find(|u| u.id == "d2").and_then(|u| u.share.clone()).expect("flat 2's part");
        let model = &app.document.as_ref().expect("a drawing").model;
        // The house has its own room tags; the share tag is the one written with {share}.
        let tags: Vec<String> = model
            .entities()
            .filter_map(|e| match e {
                Entity::Tag(t) if t.tag.of == plan.space("203") => Some(t.tag.text.clone()),
                _ => None,
            })
            .collect();
        let want = format!("Pay {} m²", d2.share);
        assert!(tags.iter().any(|t| t.contains(&want)), "{want} in {tags:?}");
        // The ground floor is the east wing's only level: flat 2 has no space upstairs, flat 1 is in the west wing.
        assert!(c.findings.iter().all(|f| f.code != "program_level_outside"), "{:?}", c.findings);

        window(&mut app, Tab::Buildings);
        let _ = app.update(Message::Program(Event::RemoveBuilding(0)));
        let w = app.program_window.as_ref().expect("the window");
        assert_eq!(w.buildings_len(), 2, "A Blok stays: its flats name it");
        let said = crate::files_testing::last_said(&app);
        assert!(said.contains("“1”, “2”, “O” bölümü"), "each flat once: {said}");
        let _ = app.update(Message::Program(Event::RemoveBuilding(1)));
        let _ = app.update(Message::Program(Event::AddBuilding));
        let _ = app.update(Message::Program(Event::AddWing(1)));
        let _ = app.update(Message::Program(Event::BuildingLevel(1, "zemin".into())));
        let _ = app.update(Message::Program(Event::WingLevel(1, 0, "zemin".into())));
        let _ = app.update(Message::Program(Event::Save));
        let program = app.document.as_ref().expect("a drawing").model.program();
        assert_eq!(program.buildings.len(), 2, "{program:?}");
        assert_eq!(program.buildings[1].name, "2. bina");
        assert_eq!(program.buildings[1].levels, vec!["zemin".to_owned()]);
        assert_eq!(program.buildings[1].wings[0].levels, vec!["zemin".to_owned()]);
        assert!(program.building("b").is_none(), "B Blok, which nothing named, is gone");
    }

    fn checked(app: &App) -> kentos_commands::ProgramChecked {
        let doc = app.document.as_ref().expect("a drawing");
        match kentos_native_application::program_check::check_in(&doc.model, &ProgramCheck::default()) {
            kentos_commands::CommandResult::Completed { output, .. } => output,
            other => panic!("{other:?}"),
        }
    }

    fn row<'a>(c: &'a kentos_commands::ProgramChecked, number: &str) -> &'a kentos_commands::ProgramSpaceRow {
        c.spaces.iter().find(|s| s.number.as_deref() == Some(number)).expect("the row")
    }

    /// The acceptance of docs/adr/0773: two flats on a 2+1 plan; a 10 cm move of the corridor's partition updates the
    /// targets' and the passage's differences in the same step; the units' totals count no common area twice; the
    /// merge's decision is said before it is written; the window's numbers are the command's.
    #[test]
    fn the_program_follows_the_partition_and_counts_each_space_once() {
        let mut app = app_with_drawing();
        let plan = house(&mut app);
        let before = checked(&app);
        // Asked to stop, the check answers cancelled and says nothing more (docs/adr/0773 §5).
        let doc = app.document.as_ref().expect("a drawing");
        let stopped = kentos_native_application::program_check::check_watched(&doc.model, &ProgramCheck::default(), &|done, _| done >= 3);
        assert!(matches!(stopped, kentos_commands::CommandResult::Cancelled), "{stopped:?}");
        // The open kitchen opens into the living room: the living room's neighbour, the kitchen's reach met.
        let kitchen = row(&before, "103");
        assert!(kitchen.reach.iter().all(|l| !l.by.is_empty()), "{:?}", kitchen.reach);
        let corridor = row(&before, "001");
        assert!((corridor.passage.expect("measured") - 1.2).abs() < 1e-3, "{corridor:?}");
        assert!((corridor.area - 1.2 * 6.8).abs() < 1e-6);
        let bedroom = row(&before, "201");
        assert!((bedroom.area - 3.4 * 4.3).abs() < 1e-6, "{bedroom:?}");
        assert_eq!(bedroom.target_difference, Some(0.0));
        // Each space once: the units, the common area and the voids add up to all but the voids.
        let t = &before.totals;
        assert!((t.units + t.unit_outdoor + t.common + t.outdoor + t.unassigned - t.all).abs() < 1e-9, "{t:?}");
        let d1 = before.units.iter().find(|u| u.id == "d1").expect("flat 1");
        assert_eq!(d1.levels, vec!["zemin".to_owned(), "kat-1".to_owned()], "a duplex");
        let d1_rooms: f64 = before
            .spaces
            .iter()
            .filter(|s| s.unit.as_deref() == Some("d1") && !s.void && !s.outdoor)
            .map(|s| s.area)
            .sum();
        assert!((d1.rooms - d1_rooms).abs() < 1e-9);
        assert!((t.common - corridor.area).abs() < 1e-9, "the corridor alone is common, the shaft a void");
        assert_eq!(d1.recorded_area, Some(60.0), "the recorded area is read, never written");

        command(
            &mut app,
            "cad.entities.transform",
            json!({"uids": [plan.walls[5]], "transform": {"kind": "move", "dx": -0.1, "dy": 0.0}}),
        );
        let after = checked(&app);
        let corridor = row(&after, "001");
        assert!((corridor.passage.expect("measured") - 1.1).abs() < 1e-3, "{corridor:?}");
        assert!((corridor.passage_difference.expect("against the brief") + 0.1).abs() < 1e-3);
        assert!(after.findings.iter().any(|f| f.code == "program_passage_narrow"));
        let bedroom = row(&after, "201");
        assert!((bedroom.area - 3.5 * 4.3).abs() < 1e-6, "{bedroom:?}");
        assert!((bedroom.target_difference.expect("a range") - (3.5 * 4.3 - 15.0)).abs() < 1e-6, "{bedroom:?}");
        assert!(after.findings.iter().any(|f| f.code == "program_area_above" && f.uids == vec![plan.space("201").to_owned()]));

        // Bina programı's numbers are the command's.
        window(&mut app, Tab::Status);
        let w = app.program_window.as_ref().expect("the window");
        let _ = w;

        // Mahalleri birleştir: asked with its decision, written on Birleştir.
        let (a, b) = (slot(&app, plan.space("102")), slot(&app, plan.space("103")));
        app.selection.set([a, b]);
        let _ = app.update(Message::Run("arch.spaceMerge"));
        assert_eq!(app.dialog, Some(crate::app::Dialog::SpaceMerge));
        let said = app.space_merging.as_ref().map(|(_, w)| w.clone()).expect("the decision");
        assert!(said.contains("Kimliği sürecek mahal: “102 Salon”"), "{said}");
        assert!(said.contains("Gidecek: “103 Mutfak”"), "{said}");
        let model = &app.document.as_ref().expect("a drawing").model;
        assert!(model.get(b).is_some(), "nothing written before Birleştir");
        let _ = app.update(Message::DialogConfirmed);
        let model = &app.document.as_ref().expect("a drawing").model;
        assert!(model.get(b).is_none(), "the kitchen merged into the living room");
        assert!(matches!(model.get(a), Some(Entity::Space(s)) if s.space.number.as_deref() == Some("102")));
    }
}
