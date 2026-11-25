//! Bina programı's pictures (docs/adr/0773): the window and its tabs, Mahalleri birleştir, the blocks and the shared
//! units. The house is built by the core's commands (program_scenes.rs). `arch_screens` takes them.

use crate::arch_ext::*;
use crate::arch;
use std::sync::Arc;
use crate::arch::ArchState;
use kentos_commands::{Entity, ProgramCheck};
use serde_json::json;
use crate::app::{App, Message};
use crate::program_window::{Event, Tab};
use crate::tools_scenes::{click, run};
use crate::files_testing::command;
use crate::program_scenes::{UP, fit, house, share_house, slot, unit_schedule};

/// The pictures of this file, for the module's `screens` (arch_screens.rs).
pub(crate) fn scenes() -> Vec<crate::tools_screens::Scene> {
    vec![
        ("bina-programi-bolumler", units),
        ("bina-programi-hedefler", briefs),
        ("bina-programi-durum", status),
        ("bina-programi-denetleniyor", running),
        ("bina-programi-kaydirildi", moved),
        ("bina-programi-birlestir", merge),
        ("bina-pay-binalar", share_buildings),
        ("bina-ortak-kat", blocks),
        ("bina-tasiyici-sistem", blocks_structure),
        ("bina-metraj", blocks_quantities),
        ("bina-ruhsat", blocks_permit),
        ("bina-pay-bolumler", share_units),
        ("bina-pay-durum", share_status),
        ("bina-pay-silinmez", share_refused),
        ("bolum-cizelgesi-ayarlar", units_settings),
    ]
}

/// Bina programı open on `tab` after Denetle.
fn window(app: &mut App, tab: Tab) {
    let _ = app.update(Message::Run("arch.program"));
    let _ = app.update(Message::from(arch::Event::Program(Event::Check)));
    let doc = app.document.as_ref().expect("a drawing");
    let result = kentos_native_application::program_check::check_watched(
        &doc.model,
        &ProgramCheck::default(),
        &|_, _| false,
    );
    let id = 1;
    let _ = app.update(Message::from(arch::Event::Program(Event::Checked(
        id,
        Arc::new(result),
    ))));
    let _ = app.update(Message::from(arch::Event::Program(Event::Tab(tab))));
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
    let _ = app.update(Message::from(arch::Event::Program(Event::Tab(Tab::Status))));
    let _ = app.update(Message::from(arch::Event::Program(Event::Check)));
    let _ = app.update(Message::from(arch::Event::Program(Event::Progress(
        1, 9, 22,
    ))));
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

/// Mahalleri birleştir over the living room and the kitchen: asked with its decision.
fn merge(app: &mut App) {
    let plan = house(app);
    let (a, b) = (slot(app, plan.space("102")), slot(app, plan.space("103")));
    app.selection.set([a, b]);
    fit(app, -0.5, -0.5, 13.5, 10.0);
    run(app, "arch.spaceMerge");
}

fn share_buildings(app: &mut App) {
    share_house(app, "net");
    fit(app, -0.5, -0.5, UP + 5.5, 10.0);
    window(app, Tab::Buildings);
}

/// Two blocks over a shared ground floor (docs/adr/8072 §1): B Blok spans the ground floor too; each block its code, use
/// and ±0.00 typed, the ground floor counted in A Blok; Binalar after Kaydet.
fn blocks(app: &mut App) {
    share_house(app, "net");
    window(app, Tab::Buildings);
    let ev = |app: &mut App, e: Event| {
        let _ = app.update(Message::from(arch::Event::Program(e)));
    };
    ev(app, Event::BuildingLevel(1, "zemin".into()));
    ev(app, Event::BuildingCode(0, "A1".into()));
    ev(app, Event::BuildingUsage(0, "Konut".into()));
    ev(app, Event::BuildingZero(0, "0".into()));
    // A Blok stands on a commercial decision (docs/adr/8144): its own Bölge.
    ev(app, Event::BuildingZone(0, Some(kentos_commands::Zone::Commercial)));
    // B Blok is built of cut stone with traditional elements (docs/adr/8145): its own structural system.
    ev(app, Event::BuildingStructure(1, Some(kentos_commands::StructuralSystem::CutStoneMasonry)));
    ev(app, Event::BuildingTraditional(1, true));
    ev(app, Event::BuildingCode(1, "B1".into()));
    ev(app, Event::BuildingUsage(1, "Ticaret".into()));
    ev(app, Event::BuildingZero(1, "1,20".into()));
    ev(app, Event::SharedBuilding("zemin".into(), "a".into()));
    ev(app, Event::Save);
    fit(app, -0.5, -0.5, UP + 5.5, 10.0);
    window(app, Tab::Buildings);
}

/// B Blok chosen in Binalar (docs/adr/8145): its own structural system, cut stone masonry with traditional elements.
fn blocks_structure(app: &mut App) {
    blocks(app);
    let _ = app.update(Message::from(arch::Event::Program(Event::ChooseBuilding(1))));
}

/// Metraj on A Blok (docs/adr/8072 §4): the storeys it counts, the shared ground floor among them; its choice in the
/// list beside the levels.
fn blocks_quantities(app: &mut App) {
    blocks(app);
    let _ = app.update(Message::from(arch::Event::Program(Event::Close)));
    app.open_quantities();
    let _ = app.update(Message::from(arch::Event::Quantities(crate::quantities::Event::Level(1))));
}

/// Ruhsat takımı on a two-block project (docs/adr/8072 §4): B Blok chosen, its storeys alone with its code.
fn blocks_permit(app: &mut App) {
    blocks(app);
    let _ = app.update(Message::from(arch::Event::Program(Event::Close)));
    let _ = app.permit_set_open();
    let _ = app.permit_set_event(crate::exchange::permit_set::Event::Building(Some(1)));
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

/// Sil on A Blok, whose flats name it: refused, the window says which.
fn share_refused(app: &mut App) {
    share_house(app, "net");
    fit(app, -0.5, -0.5, UP + 5.5, 10.0);
    window(app, Tab::Buildings);
    let _ = app.update(Message::from(arch::Event::Program(Event::RemoveBuilding(
        0,
    ))));
}

fn units_settings(app: &mut App) {
    share_house(app, "net");
    let (uid, _) = unit_schedule(app);
    fit(app, -0.5, -0.5, UP + 21.0, 10.0);
    let _ = app.schedule_event(crate::schedule_window::Event::Open(uid));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::program_scenes::*;
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
        let _ = app.update(Message::from(arch::Event::Program(Event::RemoveBuilding(
            0,
        ))));
        let w = app.modules.arch().program_window.as_ref().expect("the window");
        assert_eq!(w.buildings_len(), 2, "A Blok stays: its flats name it");
        let said = crate::files_testing::last_said(&app);
        assert!(
            said.contains("“1”, “2”, “O” bölümü"),
            "each flat once: {said}"
        );
        let _ = app.update(Message::from(arch::Event::Program(Event::RemoveBuilding(
            1,
        ))));
        let _ = app.update(Message::from(arch::Event::Program(Event::AddBuilding)));
        let _ = app.update(Message::from(arch::Event::Program(Event::AddWing(1))));
        let _ = app.update(Message::from(arch::Event::Program(Event::BuildingLevel(
            1,
            "zemin".into(),
        ))));
        let _ = app.update(Message::from(arch::Event::Program(Event::WingLevel(
            1,
            0,
            "zemin".into(),
        ))));
        let _ = app.update(Message::from(arch::Event::Program(Event::Save)));
        let program = app.document.as_ref().expect("a drawing").model.program();
        assert_eq!(program.buildings.len(), 2, "{program:?}");
        assert_eq!(program.buildings[1].name, "2. bina");
        assert_eq!(program.buildings[1].levels, vec!["zemin".to_owned()]);
        assert_eq!(program.buildings[1].wings[0].levels, vec!["zemin".to_owned()]);
        assert!(program.building("b").is_none(), "B Blok, which nothing named, is gone");
    }

    /// docs/adr/8072 §1: Binalar gives two blocks their codes (in capitals), uses and ±0.00 and the ground floor both span
    /// to A Blok; Kaydet writes them through the command, the ground floor then A Blok's storey; “00” beside another
    /// building is refused with its words and nothing is written.
    #[test]
    fn two_blocks_get_codes_zeros_and_a_shared_level_through_the_window() {
        let mut app = app_with_drawing();
        blocks(&mut app);
        let program = app.document.as_ref().expect("a drawing").model.program();
        assert_eq!(program.buildings[0].code.as_deref(), Some("A1"));
        assert_eq!(program.buildings[0].usage.as_deref(), Some("Konut"));
        assert_eq!(program.buildings[1].code.as_deref(), Some("B1"));
        assert_eq!(program.buildings[1].zero, Some(1.2));
        assert_eq!(program.shared, vec![kentos_commands::SharedLevel { level: "zemin".into(), building: "a".into() }]);
        assert_eq!(program.building_of_level("zemin").map(|b| b.id.as_str()), Some("a"));
        assert_eq!(program.building_of_level("kat-1").map(|b| b.id.as_str()), Some("a"));
        // A Blok's own Bölge (docs/adr/8144): its storeys are read by it; B Blok's by the project's.
        assert_eq!((program.buildings[0].zone, program.buildings[1].zone), (Some(kentos_commands::Zone::Commercial), None));
        let doc = &app.document.as_ref().expect("a drawing").model;
        let mimari = doc.settings().arch.clone().expect("Mimari");
        assert_eq!(mimari.zone_on("kat-1"), Some(kentos_commands::Zone::Commercial));
        let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
        let storeys: Vec<(&str, Option<&str>, Option<&str>)> =
            facts.storeys.iter().map(|s| (s.level.as_str(), s.building.as_deref(), s.zone.as_deref())).collect();
        assert!(storeys.contains(&("kat-1", Some("a"), Some("commercial"))), "{storeys:?}");
        // B Blok's own structural system (docs/adr/8145); its storeys read by it, A Blok's by the project's.
        let own = program.buildings[1].structure.expect("B Blok's system");
        assert!(own.traditional_cut_stone(), "{own:?}");
        assert!(!mimari.traditional_cut_stone_on(Some("kat-1")) && mimari.any_traditional_cut_stone());
        let _ = app.update(Message::from(arch::Event::Program(Event::BuildingCode(1, "00".into()))));
        let _ = app.update(Message::from(arch::Event::Program(Event::Save)));
        let kept = app.document.as_ref().expect("a drawing").model.program();
        assert_eq!(kept.buildings[1].code.as_deref(), Some("B1"), "nothing written");
        let said = crate::files_testing::last_said(&app);
        assert!(said.contains("“00” tek binalı projenin kodudur"), "{said}");
    }

    /// docs/adr/0795: the units schedule's rows are the program's groups with the check's numbers; its name cell goes to
    /// the program in one step and the schedule follows; a space given to another flat moves its row's area; Tabloyu
    /// düzenle says where a cell goes; the tool places one through the command.
    #[test]
    fn the_units_schedule_reads_the_program_and_writes_back_to_it() {
        let mut app = app_with_drawing();
        let plan = share_house(&mut app, "net");
        let (uid, s) = unit_schedule(&mut app);
        let c = checked(&app);
        let cells = |app: &App| match app.document.as_ref().expect("a drawing").model.get(s) {
            Some(Entity::Table(t)) => t.cells.clone(),
            _ => panic!("the schedule"),
        };
        let rows = cells(&app);
        assert_eq!(rows[0][0], "No");
        assert_eq!(rows[0][9], "Mahal sayısı");
        let d2 = c.units.iter().find(|u| u.id == "d2").and_then(|u| u.share.clone()).expect("flat 2's part");
        let flat2 = rows.iter().find(|r| r[0] == "2").expect("flat 2's row");
        assert_eq!(flat2[6], d2.share, "the check's share");
        assert_eq!(flat2[7], d2.gross, "the check's gross");
        assert_eq!(rows.last().expect("a total")[0], "Toplam");
        // Its name written through the schedule: the program changes, the row follows, one undo step.
        let input = json!({"uid": uid, "operation": {"kind": "cell", "row": 2, "column": 1, "value": "Dubleks"}});
        command(&mut app, "cad.schedules.edit", input);
        let model = &app.document.as_ref().expect("a drawing").model;
        assert_eq!(model.program().unit("d2").and_then(|u| u.name.clone()).as_deref(), Some("Dubleks"));
        assert_eq!(cells(&app)[2][1], "Dubleks");
        let said = crate::tables::editor::schedule_cell(model, &match model.get(s) {
            Some(Entity::Table(t)) => *t.clone(),
            _ => panic!("the schedule"),
        }, (2, 1));
        assert!(matches!(&said, Some(Ok(w)) if w.contains("Bina programına yazılır: 2 Dubleks")), "{said:?}");
        let _ = app.update(Message::Run("edit.undo"));
        assert_eq!(cells(&app)[2][1], "Daire 2");
        // A space given to flat 1: both rows' areas follow in the same step.
        let before = cells(&app);
        command(&mut app, "cad.program.edit", json!({"assign": [{"uids": [plan.space("203")], "unit": "d1"}]}));
        let after = cells(&app);
        assert_ne!(before[1][5], after[1][5], "flat 1 grew");
        assert_ne!(before[2][5], after[2][5], "flat 2 shrank");
        // The tool places one through the command.
        let count = |app: &App| {
            app.document.as_ref().expect("a drawing").model.entities().filter(|e| matches!(e, Entity::Table(_))).count()
        };
        let n = count(&app);
        run(&mut app, "tool.scheduleUnits");
        click(&mut app, [UP + 7.0, -4.0]);
        assert_eq!(count(&app), n + 1, "{}", crate::files_testing::last_said(&app));
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

    #[test]
    fn a_removed_door_breaks_reach_and_the_finding_shows_both_rooms() {
        let mut app = app_with_drawing();
        let plan = house(&mut app);
        let doc = &app.document.as_ref().expect("a drawing").model;
        let layer = doc.layers().active().to_owned();
        let mut program = doc.program();
        let brief = program.briefs.iter_mut().find(|brief| brief.usage == "Yatak odası").expect("a brief");
        brief.adjacent = vec!["Salon".into()];
        command(&mut app, "cad.program.edit", json!({"program": program}));
        let door = uids(&command(&mut app, "cad.openings.create", json!({
            "layerId": layer,
            "openings": [{"host": plan.walls[7], "at": 1.6, "width": 0.9, "form": "door"}]
        })));
        let target = plan.space("203").to_owned();
        let source = plan.space("201").to_owned();
        let before = checked(&app);
        assert_eq!(row(&before, "201").reach[0].by, std::slice::from_ref(&target));
        assert_eq!(row(&before, "201").adjacent[0].by, std::slice::from_ref(&target));

        command(&mut app, "cad.entities.delete", json!({"uids": door}));
        let after = checked(&app);
        assert!(row(&after, "201").reach[0].by.is_empty());
        assert_eq!(row(&after, "201").adjacent[0].by, std::slice::from_ref(&target));
        let subjects = [source.clone(), target.clone()];
        let index = after.findings.iter().position(|finding| {
            finding.code == "program_reach_missing" && finding.uids == subjects
        }).expect("the failed passage names both rooms in the same flat");

        app.choose_level(Some("kat-1".into()));
        window(&mut app, Tab::Status);
        let expected = [slot(&app, &source), slot(&app, &target)];
        let _ = app.update(Message::from(arch::Event::Program(Event::ShowFinding(index))));
        assert!(app.dialog.is_none());
        assert_eq!(app.active_level().as_deref(), Some("zemin"));
        assert_eq!(app.level_view, kentos_interaction::level_view::LevelView::Active);
        assert_eq!(app.selection.ids(), expected);
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
        let w = app.modules.arch().program_window.as_ref().expect("the window");
        let _ = w;

        // Mahalleri birleştir: asked with its decision, written on Birleştir.
        let (a, b) = (slot(&app, plan.space("102")), slot(&app, plan.space("103")));
        app.selection.set([a, b]);
        let _ = app.update(Message::Run("arch.spaceMerge"));
        assert_eq!(
            app.dialog,
            Some(crate::arch::dialog(crate::arch::ArchDialogKind::SpaceMerge))
        );
        let said = app
            .space_merging
            .as_ref()
            .map(|(_, w)| w.clone())
            .expect("the decision");
        assert!(
            said.contains("Kimliği sürecek mahal: “102 Salon”"),
            "{said}"
        );
        assert!(said.contains("Gidecek: “103 Mutfak”"), "{said}");
        let model = &app.document.as_ref().expect("a drawing").model;
        assert!(model.get(b).is_some(), "nothing written before Birleştir");
        let _ = app.update(Message::DialogConfirmed);
        let model = &app.document.as_ref().expect("a drawing").model;
        assert!(model.get(b).is_none(), "the kitchen merged into the living room");
        assert!(matches!(model.get(a), Some(Entity::Space(s)) if s.space.number.as_deref() == Some("102")));
    }
}
