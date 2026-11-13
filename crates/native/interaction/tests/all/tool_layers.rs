//! The layer an annotation tool draws on (usage review UR-003): a project that names its text layer has its texts
//! written there whatever the active layer; a locked text layer is said at the click; a named layer the drawing does
//! not have leaves the active one. Worked out by hand.

use crate::common;

use common::Bench;
use kentos_commands::{AnnotationTool, Entity, ToolLayer};
use kentos_interaction::Level;

const TOOLS: &str = include_str!("../../../../../fixtures/interaction/v1/tools.kcad");

fn named(b: &mut Bench, layer: &str) {
    let mut s = b.doc.settings().clone();
    s.tool_layers = vec![ToolLayer { tool: AnnotationTool::Text, layer: layer.to_owned() }];
    b.doc.set_settings(s);
}

#[test]
fn a_text_goes_to_the_project_s_text_layer() {
    let mut b = Bench::on(TOOLS);
    b.draft.snap = false;
    named(&mut b, "kot");
    assert_eq!(b.doc.layers().active(), "cizim");
    b.start("text");
    b.click(4.0, 2.0);
    b.run(|s, cx| s.text_typed(Some("Ada 101"), cx));
    let Entity::Text(t) = b.newest() else { panic!("a text") };
    assert_eq!(t.base.layer_id, "kot", "the text layer, not the active one");
    assert!(b.undo_step(), "one undo step");
}

#[test]
fn a_locked_text_layer_is_said_and_a_missing_one_leaves_the_active_layer() {
    let mut b = Bench::on(TOOLS);
    b.draft.snap = false;
    named(&mut b, "kilitli");
    b.start("text");
    b.click(0.0, 0.0);
    assert!(b.views.is_empty(), "no field: {:?}", b.views);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.last_text(),
        Some("Yazılar “Kilitli katman” katmanına çizilir ve o katman kilitli. Kilidi Katmanlar panelinden açın ya da Proje ayarları › Araç katmanları'nı değiştirin.")
    );
    let mut b = Bench::on(TOOLS);
    b.draft.snap = false;
    named(&mut b, "yok-boyle");
    b.start("text");
    b.click(4.0, 2.0);
    b.run(|s, cx| s.text_typed(Some("Ada 102"), cx));
    let Entity::Text(t) = b.newest() else { panic!("a text") };
    assert_eq!(t.base.layer_id, "cizim");
}
