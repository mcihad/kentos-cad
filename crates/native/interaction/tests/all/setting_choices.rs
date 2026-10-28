//! Every choice the settings pane offers applies (docs/adr/0709): the pane sends a choice's typed value (`right`,
//! `clicked`, `fit`), which the tools once read in Turkish capitals (RİGHT, CLİCKED, FİT) and refused. Each tool with
//! settings is started in a metric CAD drawing and every choice of every editable setting is configured back.

use crate::common::Bench;
use kentos_commands::{DrawingUnit, Workspace};
use kentos_interaction::Format;

const TOOLS: &[&str] = &[
    "wall",
    "wallRectangle",
    "wallPartition",
    "wallFromLines",
    "wallDimensions",
    "door",
    "window",
    "opening",
    "openingArray",
    "space",
    "spaceDiscover",
    "spaceNumber",
    "tag",
    "tagAll",
    "grid",
    "column",
    "stair",
    "stairCurved",
    "ramp",
    "gridSystem",
    "casework",
];

/// Choices that need what the drawing does not hold here, and say so: Sabit uzaklık measures from a wall picked first.
const NEEDS_MORE: &[(&str, &str, &str)] = &[("wallDimensions", "P", "fixed")];

#[test]
fn every_offered_choice_applies() {
    let mut refused = Vec::new();
    for tool in TOOLS {
        let mut b = Bench::new(tool);
        let mut settings = b.doc.settings().clone();
        settings.workspace = Some(Workspace::Cad);
        settings.srid = 0;
        settings.drawing_unit = Some(DrawingUnit::M);
        b.doc.set_settings(settings);
        b.start(tool);
        let asked: Vec<(&'static str, String)> = b
            .session
            .settings(&Format::default())
            .iter()
            .filter(|r| r.editable && !r.note)
            .flat_map(|r| {
                r.choices
                    .iter()
                    .filter(|c| c.command.is_none())
                    .map(|c| (r.key, c.typed.clone()))
            })
            .collect();
        for (key, typed) in asked {
            // A choice changes what the others offer (Yöntem: Kontur gives Hiza a Referans's): only what is offered
            // now is chosen, as the pane shows it after each change.
            let now = b.session.settings(&Format::default());
            let Some(row) = now
                .iter()
                .find(|r| r.key == key && r.choices.iter().any(|c| c.typed == typed))
            else {
                continue;
            };
            let label = row.label.clone();
            if NEEDS_MORE.contains(&(*tool, key, typed.as_str())) {
                continue;
            }
            if !b.run(|s, cx| s.configure(key, &typed, cx)) {
                refused.push(format!("{tool}: {label} ({key}) = {typed}"));
            }
        }
    }
    assert!(
        refused.is_empty(),
        "uygulanmayan seçimler:\n{}",
        refused.join("\n")
    );
}
