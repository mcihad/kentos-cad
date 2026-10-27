//! A walk through an apartment plan the way an architect draws it, one picture a step, to find what the tools do
//! wrong together (not one at a time): the shell with Dikdörtgen duvar (12 × 9 m clear, 25 cm), two Ölçülü bölme
//! partitions, doors and windows, Toplu mahal bul, Hepsini etiketle, Duvar ölçüleri round the shell, an İç ölçü
//! zinciri across the rooms, Mahalleri numarala, a kitchen run, a stair, the door and window schedule, the Metraj and
//! the Denetim; then a 1. kat over it and Kat görünümü (docs/adr/0715). `tools_screens` takes them in both themes at 1440×900 and
//! 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=daire-kabuk,daire-bolme-onizleme,daire-bolmeler,daire-aciklik,daire-mahal-onizleme,daire-mahaller,daire-etiket-onizleme,daire-etiketler,daire-olculer,daire-ic-olcu,daire-numara,daire-numaralandi,daire-mutfak,daire-merdiven,daire-cizelge,daire-cizelge-yazildi,daire-metraj,daire-denetim cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use crate::app::{App, Message};
use crate::tools_scenes::{E, N, Objects, click, hover, run, typed};
use crate::tools_screens::{Pointed, Scene, press_caption};
use crate::wall_scenes::opened;

/// The plan with room round it for the dimension rows.
fn view(app: &mut App) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 3.5,
            min_y: N - 3.5,
            max_x: E + 15.5,
            max_y: N + 12.5,
        },
        24.0,
    );
}

/// Dikdörtgen duvar, 25 cm, 12 × 9 m clear (İç net, its default).
fn shell(app: &mut App) {
    opened(app, Objects::new(), 100.0);
    crate::arch_scenes::on_level(app);
    view(app);
    app.tab = "arch";
    app.command_expanded = true;
    run(app, "tool.wallRectangle");
    typed(app, "E");
    typed(app, "0.25m");
    click(app, [0.0, 0.0]);
    click(app, [12.0, 9.0]);
    typed(app, "");
    app.selection.clear();
    hover(app, [14.0, 11.0]);
}

/// Ölçülü bölme: the west wall picked, 4 m clear typed; the preview before Enter.
fn partition_preview(app: &mut App) {
    shell(app);
    run(app, "tool.wallPartition");
    for (key, value) in [("E", "150mm"), ("U", "4m")] {
        let _ = app.tool_setting(crate::architecture_settings::Commit {
            tool: "wallPartition",
            key,
            value: value.into(),
        });
    }
    click(app, [0.0, 4.5]);
    hover(app, [3.0, 6.0]);
}

/// The first partition written; the second from the south wall, 5 m clear, between the first and the east wall.
fn partitions(app: &mut App) {
    partition_preview(app);
    typed(app, "");
    run(app, "tool.wallPartition");
    let _ = app.tool_setting(crate::architecture_settings::Commit {
        tool: "wallPartition",
        key: "E",
        value: "150mm".into(),
    });
    let _ = app.tool_setting(crate::architecture_settings::Commit {
        tool: "wallPartition",
        key: "U",
        value: "5m".into(),
    });
    let _ = app.tool_setting(crate::architecture_settings::Commit {
        tool: "wallPartition",
        key: "B",
        value: "between".into(),
    });
    click(app, [8.0, 0.0]);
    click(app, [4.075, 6.0]);
    click(app, [12.0, 6.0]);
    typed(app, "");
    app.selection.clear();
    hover(app, [14.0, 11.0]);
}

/// A door in each partition and on the south wall, windows on the north and east walls.
fn openings(app: &mut App) {
    partitions(app);
    run(app, "tool.door");
    for p in [[4.075, 2.0], [8.0, 5.075], [2.0, -0.125]] {
        click(app, p);
    }
    typed(app, "");
    run(app, "tool.window");
    for p in [[2.0, 9.125], [9.0, 9.125], [12.125, 2.5]] {
        click(app, p);
    }
    typed(app, "");
    app.selection.clear();
    hover(app, [14.0, 11.0]);
}

/// docs/adr/0714: Kapı again by the north wall, its settings' Numara the next one (K4 after K1–K3).
fn door_tool(app: &mut App) {
    openings(app);
    run(app, "tool.door");
    hover(app, [6.0, 8.8]);
}

/// Toplu mahal bul: the rooms it found, before Enter.
fn rooms_preview(app: &mut App) {
    openings(app);
    run(app, "tool.spaceDiscover");
    hover(app, [14.0, 11.0]);
}

/// Enter: the rooms written.
fn rooms(app: &mut App) {
    rooms_preview(app);
    typed(app, "");
    app.selection.clear();
    hover(app, [14.0, 11.0]);
}

/// Hepsini etiketle before Enter: its preview and its panel's count (the doors and windows have no numbers yet).
fn tag_preview(app: &mut App) {
    rooms(app);
    run(app, "tool.tagAll");
    hover(app, [14.0, 11.0]);
}

/// Enter: every room and opening tagged; the rooms' tags their labels now.
fn tagged(app: &mut App) {
    tag_preview(app);
    typed(app, "");
    app.selection.clear();
    hover(app, [14.0, 11.0]);
}

/// Duvar ölçüleri round the shell (Bütün kenarlar), written.
fn dimensions(app: &mut App) {
    tagged(app);
    run(app, "tool.wallDimensions");
    click(app, [6.0, -0.25]);
    let _ = app.tool_setting(crate::architecture_settings::Commit {
        tool: "wallDimensions",
        key: "N",
        value: "all".into(),
    });
    hover(app, [6.0, -1.6]);
    typed(app, "");
    app.selection.clear();
    hover(app, [14.0, 11.0]);
}

/// An İç ölçü zinciri across the three rooms at y 7.5.
fn interior(app: &mut App) {
    dimensions(app);
    run(app, "tool.interiorDimensions");
    click(app, [-1.0, 3.5]);
    click(app, [13.0, 3.5]);
    click(app, [6.0, 3.5]);
    typed(app, "");
    app.selection.clear();
    hover(app, [14.0, 11.0]);
}

/// docs/adr/0713: a project of two levels (Zemin kat and 1. kat at 3 m), opened on Zemin kat.
fn two_levels(app: &mut App) {
    opened(app, Objects::new(), 100.0);
    if let Some(doc) = app.document.as_mut() {
        let mut settings = doc.model.settings().clone();
        settings.arch.get_or_insert_with(Default::default).levels = vec![
            kentos_commands::ArchLevel {
                id: "zemin".into(),
                name: "Zemin kat".into(),
                elevation: 0.0,
                height: Some(3.0),
            },
            kentos_commands::ArchLevel {
                id: "kat-1".into(),
                name: "1. kat".into(),
                elevation: 3.0,
                height: Some(3.0),
            },
        ];
        doc.model.set_settings(settings);
    }
    view(app);
    app.tab = "arch";
    app.command_expanded = true;
}

/// Duvar started: Etkin kat first in its settings (Zemin kat, the ground level the drawing opened on).
fn wall_tool_on_level(app: &mut App) {
    two_levels(app);
    run(app, "tool.wall");
    hover(app, [3.0, 2.0]);
}

/// 1. kat chosen on the Kat menu, a wall drawn: it is on 1. kat (its Öznitelikler say so).
fn on_first_floor(app: &mut App) {
    two_levels(app);
    let _ = app.update(Message::ActiveLevel(Some("kat-1".into())));
    run(app, "tool.wall");
    click(app, [0.0, 0.0]);
    click(app, [8.0, 0.0]);
    typed(app, "");
    typed(app, "");
    hover(app, [4.0, 0.0]);
    click(app, [4.0, 0.0]);
    hover(app, [14.0, 11.0]);
}

/// Mahalleri numarala: the rooms clicked in turn (west, south-east, north-east), the preview before Enter.
fn numbering(app: &mut App) {
    tagged(app);
    run(app, "tool.spaceNumber");
    for p in [[2.0, 4.5], [8.0, 2.5], [8.0, 7.0]] {
        click(app, p);
    }
    hover(app, [14.0, 11.0]);
}

/// Enter: the rooms numbered, their tags following.
fn numbered(app: &mut App) {
    numbering(app);
    typed(app, "");
    app.selection.clear();
    hover(app, [14.0, 11.0]);
}

/// Mutfak dizisi along the north-east room's north wall, the cabinets toward the room.
fn kitchen(app: &mut App) {
    numbered(app);
    run(app, "tool.casework");
    click(app, [4.6, 9.0]);
    click(app, [8.2, 9.0]);
    hover(app, [6.0, 8.0]);
}

/// The kitchen written; a straight stair in the west room, its preview.
fn stair(app: &mut App) {
    kitchen(app);
    click(app, [6.0, 8.0]);
    typed(app, "");
    run(app, "tool.stair");
    click(app, [0.6, 1.0]);
    hover(app, [0.6, 6.0]);
}

/// The stair written; Kapı ve pencere çizelgesi hanging from the pointer east of the plan.
fn schedule(app: &mut App) {
    stair(app);
    click(app, [0.6, 6.0]);
    typed(app, "");
    run(app, "tool.scheduleOpenings");
    hover(app, [13.5, 9.0]);
}

/// The schedule placed; Esc leaves the tool (Enter would start the next one).
fn scheduled(app: &mut App) {
    schedule(app);
    click(app, [13.5, 9.0]);
    app.cancel();
    app.selection.clear();
    hover(app, [14.0, -2.0]);
}

/// Metraj over it all.
fn quantities(app: &mut App) {
    interior(app);
    run(app, "arch.quantities");
}

/// Denetim over it all.
fn audit(app: &mut App) {
    interior(app);
    run(app, "arch.audit");
}

/// docs/adr/0715: a 1. kat (3.06 m up) drawn over the finished ground floor: the same shell, a partition 6 m from the
/// west wall and one 4 m from the north wall east of it, two doors, three windows, its rooms found and everything
/// tagged on 1. kat. Kat görünümü still Bütün katlar: both floors over each other.
fn upper_floor(app: &mut App) {
    interior(app);
    if let Some(doc) = app.document.as_mut() {
        let mut settings = doc.model.settings().clone();
        let arch = settings.arch.get_or_insert_with(Default::default);
        arch.levels.push(kentos_commands::ArchLevel {
            id: "kat-1".into(),
            name: "1. kat".into(),
            elevation: 3.06,
            height: Some(3.06),
        });
        doc.model.set_settings(settings);
    }
    let _ = app.update(Message::ActiveLevel(Some("kat-1".into())));
    run(app, "tool.wallRectangle");
    typed(app, "E");
    typed(app, "0.25m");
    click(app, [0.0, 0.0]);
    click(app, [12.0, 9.0]);
    typed(app, "");
    run(app, "tool.wallPartition");
    for (key, value) in [("E", "150mm"), ("U", "6m")] {
        let _ = app.tool_setting(crate::architecture_settings::Commit {
            tool: "wallPartition",
            key,
            value: value.into(),
        });
    }
    click(app, [0.0, 4.5]);
    hover(app, [3.0, 6.0]);
    typed(app, "");
    run(app, "tool.wallPartition");
    for (key, value) in [("E", "150mm"), ("U", "4m"), ("B", "between")] {
        let _ = app.tool_setting(crate::architecture_settings::Commit {
            tool: "wallPartition",
            key,
            value: value.into(),
        });
    }
    click(app, [9.0, 9.0]);
    click(app, [6.075, 3.0]);
    click(app, [12.0, 3.0]);
    typed(app, "");
    run(app, "tool.door");
    for p in [[6.075, 2.0], [9.0, 4.925]] {
        click(app, p);
    }
    typed(app, "");
    run(app, "tool.window");
    for p in [[3.0, -0.125], [9.0, -0.125], [3.0, 9.125]] {
        click(app, p);
    }
    typed(app, "");
    run(app, "tool.spaceDiscover");
    typed(app, "");
    run(app, "tool.tagAll");
    typed(app, "");
    app.cancel();
    app.selection.clear();
    hover(app, [14.0, 11.0]);
}

/// Yalnız etkin kat: 1. kat alone (the ground floor's walls, rooms, tags and the dimensions bound to them hidden).
fn upper_floor_alone(app: &mut App) {
    upper_floor(app);
    let _ = app.update(Message::LevelView(kentos_interaction::level_view::LevelView::Active));
}

/// Etkin kat ve alt kat: 1. kat over the ground floor drawn faint.
fn upper_floor_over_ground(app: &mut App) {
    upper_floor(app);
    let _ = app.update(Message::LevelView(kentos_interaction::level_view::LevelView::Below));
}

/// Zemin kat active again with Yalnız etkin kat: the ground floor alone, its dimensions back.
fn ground_floor_alone(app: &mut App) {
    upper_floor_alone(app);
    let _ = app.update(Message::ActiveLevel(Some("zemin".into())));
}

/// Görünüm › Kat görünümü ▾ (CAD's Görünüm tab).
fn view_tab_levels(app: &mut App) {
    upper_floor_over_ground(app);
    app.tab = "view";
}

/// A designer's message to the sheets, and what it starts done.
fn sheet(app: &mut App, m: kentos_sheet_ui::Message) {
    let task = app.update(Message::Sheet(m));
    crate::files_testing::drive(app, task);
}

/// docs/adr/0716: Yeni pafta over the two floors (its template's questions answered), its map chosen, fitted to the
/// drawing and set to draw 1. kat; with `below`, Zemin kat faint under it.
fn sheet_of_level(app: &mut App, below: bool) {
    use kentos_sheet_ui::Message as Sheet;
    upper_floor(app);
    sheet(app, Sheet::NewSheet);
    if app.sheets.questions().is_some() {
        sheet(app, Sheet::Questions(kentos_sheet_ui::questions::QuestionsMessage::Make));
    }
    let map = app.sheets.open_sheet().and_then(|s| {
        s.items
            .iter()
            .find(|i| matches!(i.kind, kentos_sheet::kinds::ItemKind::Map(_)))
            .map(|i| i.id.clone())
    });
    if let Some(id) = map {
        sheet(app, Sheet::Select(vec![id]));
        sheet(app, Sheet::MapFitDrawing);
        sheet(app, Sheet::MapLevel(Some("kat-1".into())));
        if below {
            sheet(app, Sheet::MapLevelBelow(true));
        }
    }
}

fn sheet_upper(app: &mut App) {
    sheet_of_level(app, false);
}

fn sheet_upper_over_ground(app: &mut App) {
    sheet_of_level(app, true);
}

/// The status bar's Kat cell open (docs/adr/0713): the levels, Katsız, Katları düzenle.
pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        (
            "kat-menu",
            wall_tool_on_level,
            |s, app| press_caption(s, app, "Zemin kat", false, false),
        ),
        // The wall drawn on 1. kat, its Öznitelikler scrolled to its Mimari rows: Kat 1. kat.
        // docs/adr/0715: the Kat menu with Kat görünümü's three choices, and the ribbon's Kat görünümü ▾.
        ("kat-gorunumu-menu", upper_floor_over_ground, |s, app| {
            // The Kat cell right of the scale's (its words fold away first in a full bar): its icon.
            // The status bar's (Öznitelikler's Çizim ölçeği reads 1:100 too, below the window when it is low).
            let bar = crate::files_testing::find_text(s, app, "Yardımcılar").expect("the status bar");
            let scale = crate::files_testing::find_texts(s, app, "1:100")
                .into_iter()
                .find(|r| (r.center_y() - bar.center_y()).abs() < 6.0)
                .expect("the scale cell");
            let at = iced::Point::new(scale.x + scale.width + 50.0, scale.center_y());
            let mut update = |app: &mut App, message: Message| {
                let _ = app.update(message);
            };
            s.input(app, App::view, &mut update, kentos_ui::snapshot::Input::Click(at));
        }),
        // docs/adr/0716: the map's inspector scrolled to its Kat and Alt kat soluk.
        ("pafta-kat-denetci", sheet_upper_over_ground, |s, app| {
            let mut update = |app: &mut App, message: Message| {
                let _ = app.update(message);
            };
            let at = crate::tools_screens::caption_at(s, app, "Konum ve boyut", false);
            s.input(
                app,
                App::view,
                &mut update,
                kentos_ui::snapshot::Input::Scroll(at, -11.0),
            );
        }),
        ("kat-gorunumu-serit", view_tab_levels, |s, app| {
            crate::tools_screens::open_ribbon_menu(s, app, "Kat görünümü")
        }),
        ("kat-duvar-oznitelik", on_first_floor, |s, app| {
            let mut update = |app: &mut App, message: Message| {
                let _ = app.update(message);
            };
            let at = crate::tools_screens::caption_at(s, app, "Sembol", false);
            s.input(
                app,
                App::view,
                &mut update,
                kentos_ui::snapshot::Input::Scroll(at, -30.0),
            );
        }),
    ]
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("kat-arac", wall_tool_on_level),
        ("kat-duvar", on_first_floor),
        ("daire-kabuk", shell),
        ("daire-bolme-onizleme", partition_preview),
        ("daire-bolmeler", partitions),
        ("daire-aciklik", openings),
        ("daire-kapi-numara", door_tool),
        ("daire-mahal-onizleme", rooms_preview),
        ("daire-mahaller", rooms),
        ("daire-etiket-onizleme", tag_preview),
        ("daire-etiketler", tagged),
        ("daire-olculer", dimensions),
        ("daire-ic-olcu", interior),
        ("daire-numara", numbering),
        ("daire-numaralandi", numbered),
        ("daire-mutfak", kitchen),
        ("daire-merdiven", stair),
        ("daire-cizelge", schedule),
        ("daire-cizelge-yazildi", scheduled),
        ("daire-metraj", quantities),
        ("daire-denetim", audit),
        ("kat-gorunumu-butun", upper_floor),
        ("kat-gorunumu-yalniz", upper_floor_alone),
        ("kat-gorunumu-alt", upper_floor_over_ground),
        ("kat-gorunumu-zemin", ground_floor_alone),
        ("pafta-kat-1", sheet_upper),
        ("pafta-kat-1-alt", sheet_upper_over_ground),
    ]
}
