//! Tadilat lejantı (docs/adr/3039 §5): Mimarlar Odası's legend placed at a typed point, one block MM-XXXX-TADILATLEJANTI
//! on the legends' layer: its title, its three words, a plain box, a hatched one, a dashed one, at the plot scale's
//! paper sizes; one undo step.

use crate::app::{App, Message};

fn legend_entities(app: &App) -> Vec<kentos_commands::Entity> {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.block(kentos_domain::renovation_legend::LEGEND_ID)
        .map(|b| b.entities.clone())
        .unwrap_or_default()
}

#[test]
fn the_legend_is_placed_with_mimarlar_odasi_s_three_techniques() {
    let mut app = crate::files_testing::app_with_drawing();
    crate::files_testing::make_cad(&mut app);
    let _ = app.update(Message::Run("tool.renovationLegend"));
    assert_eq!(app.session.tool_id(), "renovationLegend");
    let _ = app.submit_line("10,0");
    assert_eq!(
        crate::files_testing::last_said(&app),
        "Tadilat lejantı: mevcut düz kontur, ilave taralı, iptal kesikli (Mimarlar Odası)."
    );
    let doc = &app.document.as_ref().expect("a drawing").model;
    let block = doc
        .block(kentos_domain::renovation_legend::LEGEND_ID)
        .expect("the legend's block");
    assert_eq!(block.name, "MM-XXXX-TADILATLEJANTI");
    assert_eq!(
        doc.block_uses(kentos_domain::renovation_legend::LEGEND_ID),
        1
    );
    let entities = legend_entities(&app);
    let words: Vec<String> = entities
        .iter()
        .filter_map(|e| match e {
            kentos_commands::Entity::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(words, ["Tadilat lejantı", "Mevcut", "İlave", "İptal"]);
    let hatches = entities
        .iter()
        .filter(|e| matches!(e, kentos_commands::Entity::Hatch(_)))
        .count();
    assert_eq!(hatches, 1, "ilave hatched");
    let dashed: Vec<bool> = entities
        .iter()
        .filter_map(|e| match e {
            kentos_commands::Entity::Polyline(p) => {
                Some(p.base.look.line_type == Some(kentos_commands::LineType::Dashed))
            }
            _ => None,
        })
        .collect();
    assert_eq!(dashed, [false, false, true], "iptal dashed");
    // Paper sizes at the plot scale: a box 15 mm wide.
    let scale = doc.settings().plot_scale;
    let kentos_commands::Entity::Polyline(p) = entities
        .iter()
        .find(|e| matches!(e, kentos_commands::Entity::Polyline(_)))
        .unwrap()
    else {
        unreachable!()
    };
    let width = p.pts.iter().map(|q| q.x).fold(f64::MIN, f64::max)
        - p.pts.iter().map(|q| q.x).fold(f64::MAX, f64::min);
    assert!((width - 15.0 * scale / 1000.0).abs() < 1e-9, "{width}");
    // One undo takes it away, block and all.
    let _ = app.update(Message::Run("edit.undo"));
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(
        doc.block_uses(kentos_domain::renovation_legend::LEGEND_ID),
        0
    );
}
