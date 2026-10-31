//! The pictures of docs/adr/0787 (the special doors' clashes, a pocket door in an arc wall, its void in the
//! quantities): the shared cases' drawings (`fixtures/commands/v1/cad.arch.audit.doors.special.json` and
//! `cad.openings.create.arcpocket.json` `setup`). Mimari denetim's Kapılar over the revolving door's drum on a column,
//! the folding door's panels on a column and a door, the sliding door's travel into a wall; the folding door's finding
//! chosen with its fix (Açılışı öbür yüze çevir), then fixed; the pocket door CG1 in the arc wall near, its curved leaf
//! and its pocket along the arc; Kapı, Cebe giren over the arc wall (the command's plan); Metraj's Duvarlar with the
//! pockets' volume. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=kapi-carpisma,kapi-carpisma-secili,kapi-carpisma-duzeltildi,yayli-cep,yayli-cep-arac,cep-metraj cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use iced::Point;
use kentos_render_wgpu::Vec2;
use kentos_ui::widget::docking;
use serde_json::Value;

use crate::app::{App, Message};
use crate::arch_audit::Event;
use crate::architecture_settings::Commit;
use crate::files_testing::make_cad;
use crate::tools_scenes::run;
use crate::tools_screens::Scene;

const CLASHES: &str = include_str!("../../../fixtures/commands/v1/cad.arch.audit.doors.special.json");
const ARC: &str = include_str!("../../../fixtures/commands/v1/cad.openings.create.arcpocket.json");

fn open(app: &mut App, cases: &str, bounds: [f64; 4]) {
    let file: Value = serde_json::from_str(cases).expect("the cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(file["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    make_cad(app);
    app.draft.snap = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: bounds[0],
            min_y: bounds[1],
            max_x: bounds[2],
            max_y: bounds[3],
        },
        24.0,
    );
}

/// The doors' audit: D1's drum on a column and on K4, KT1's panels on a column and K3, S1's travel into a wall, KT2's
/// on a fixture (its layer locked: no fix).
fn audited(app: &mut App) {
    open(app, CLASHES, [1.5, -2.6, 13.5, 2.6]);
    run(app, "arch.audit");
    let _ = app.update(Message::ArchAudit(Event::Group(Some(
        kentos_commands::ArchAuditCheck::Doors,
    ))));
}

/// The row of KT1's clash with the column, chosen: its fix is Açılışı öbür yüze çevir.
fn chosen(app: &mut App) {
    audited(app);
    let row = row_of(app, "KT1 katlanır");
    let _ = app.update(Message::ArchAudit(Event::Press(row)));
}

/// Düzelt: KT1 turned to the other face, its findings gone.
fn fixed(app: &mut App) {
    chosen(app);
    let _ = app.update(Message::ArchAudit(Event::Fix));
}

fn row_of(app: &App, words: &str) -> usize {
    let shown = app.arch_audit_shown();
    let findings = &app.arch_audit.audited.as_ref().expect("audited").findings;
    shown
        .iter()
        .position(|&i| findings[i].message.starts_with(words))
        .expect("a finding with those words")
}

/// The pocket door CG1 in the arc wall near: its leaf a curved band, its pocket along the arc.
fn arc(app: &mut App) {
    open(app, ARC, [-0.4, 2.6, 4.2, 5.4]);
}

/// Kapı, Açılış Cebe giren, the cursor by the arc wall: the pocket door the command would put there.
fn arc_tool(app: &mut App) {
    open(app, ARC, [-0.5, -0.5, 5.8, 5.8]);
    app.command_expanded = true;
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    run(app, "tool.door");
    let _ = app.update(Message::ToolSetting(Commit {
        tool: "door",
        key: "Ç",
        value: "pocket".into(),
    }));
    // A point beside the arc's middle part, inside it.
    let p = [4.85 * (0.7f64).cos(), 4.85 * (0.7f64).sin()];
    let [x, y] = app.viewport.camera.world_to_screen(Vec2::new(p[0], p[1]));
    let _ = app.update(Message::Viewport(crate::viewport::Event::Moved(Point::new(
        x as f32, y as f32,
    ))));
}

/// Metraj, Duvarlar: the arc wall's and the straight wall's pockets taken off their volume.
fn quantities(app: &mut App) {
    open(app, ARC, [-1.0, -11.0, 26.0, 6.0]);
    run(app, "arch.quantities");
    let _ = app.update(Message::Quantities(crate::quantities::Event::Tab(
        crate::quantities::Tab::Walls,
    )));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("kapi-carpisma", audited),
        ("kapi-carpisma-secili", chosen),
        ("kapi-carpisma-duzeltildi", fixed),
        ("yayli-cep", arc),
        ("yayli-cep-arac", arc_tool),
        ("cep-metraj", quantities),
    ]
}
