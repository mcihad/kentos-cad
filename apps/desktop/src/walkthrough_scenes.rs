//! A walk through an apartment plan the way an architect draws it, one picture a step, to find what the tools do
//! wrong together (not one at a time): the shell with Dikdörtgen duvar (12 × 9 m clear, 25 cm), two Ölçülü bölme
//! partitions, doors and windows, Toplu mahal bul, Hepsini etiketle, Duvar ölçüleri round the shell, an İç ölçü
//! zinciri across the rooms, the Metraj and the Denetim. `tools_screens` takes them in both themes at 1440×900 and
//! 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=daire-kabuk,daire-bolme-onizleme,daire-bolmeler,daire-aciklik,daire-mahal-onizleme,daire-mahaller,daire-etiket-onizleme,daire-etiketler,daire-olculer,daire-ic-olcu,daire-metraj,daire-denetim cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use crate::app::App;
use crate::tools_scenes::{E, N, Objects, click, hover, run, typed};
use crate::tools_screens::Scene;
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

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("daire-kabuk", shell),
        ("daire-bolme-onizleme", partition_preview),
        ("daire-bolmeler", partitions),
        ("daire-aciklik", openings),
        ("daire-mahal-onizleme", rooms_preview),
        ("daire-mahaller", rooms),
        ("daire-etiket-onizleme", tag_preview),
        ("daire-etiketler", tagged),
        ("daire-olculer", dimensions),
        ("daire-ic-olcu", interior),
        ("daire-metraj", quantities),
        ("daire-denetim", audit),
    ]
}
