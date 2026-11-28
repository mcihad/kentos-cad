//! Renk planı lejandı (`tool.colorLegend`, docs/adr/8176 §3.3): a click places the colour plan's legend of the active
//! storey's rooms on the legends' layer, one undo step, and the tool leaves; without a colour plan it says where to
//! choose one and leaves at once.

use crate::common::{Bench, E, N};
use kentos_commands::{CadWork, ColorPlan, ColorScheme, Entity, Workspace};
use kentos_interaction::Level;
use serde_json::json;

/// Zemin kat with a living room and a bathroom, 4 × 3 m each; the colour plan by use when `plan`. The tool started.
fn bench(plan: bool) -> Bench {
    let mut b = Bench::new("line");
    b.session.exit();
    let mut s = b.doc.settings().clone();
    s.srid = 0;
    s.workspace = Some(Workspace::Cad);
    s.cad_work = Some(CadWork::Architecture);
    s.arch = Some(
        serde_json::from_value(json!({"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0}]}))
            .unwrap(),
    );
    if plan {
        s.arch.as_mut().unwrap().color_plan = Some(ColorPlan { scheme: ColorScheme::Usage });
    }
    b.doc.set_settings(s);
    for (x, name) in [(0.0, "Salon"), (4.0, "Banyo")] {
        let p = |x: f64, y: f64| json!({"x": E + x, "y": N + y});
        let e: Entity = serde_json::from_value(json!({"kind": "space", "id": 0, "layerId": "cizim", "attrs": {},
            "parts": [{"outer": {"pts": [p(x, 0.0), p(x + 4.0, 0.0), p(x + 4.0, 3.0), p(x, 3.0)]}}],
            "name": name, "arch": {"level": "zemin"}}))
        .unwrap();
        b.doc.add(e).expect("a slot");
    }
    b.level = Some("zemin".into());
    b.memory.layer_keys = true;
    b.start("colorLegend");
    b
}

fn legend_objects(b: &Bench) -> Vec<&Entity> {
    b.doc.entities().filter(|e| e.base().attrs.contains_key("Renk planı")).collect()
}

#[test]
fn a_click_places_the_legend_on_its_layer_in_one_step() {
    let mut b = bench(true);
    assert_eq!(b.session.tool_id(), "colorLegend");
    b.click(0.0, -2.0);
    assert_eq!(b.last_level(), Some(Level::Success));
    assert_eq!(b.last_text(), Some("Renk planı lejandı yerleşti: 2 satır."));
    let objects = legend_objects(&b);
    assert_eq!(objects.len(), 1 + 2 * 3 + 1, "the title, each row's swatch, outline and words, the total");
    for e in &objects {
        let name = b.doc.layers().get(&e.base().layer_id).map(|l| l.name.clone());
        assert_eq!(name.as_deref(), Some("MM-XXXX-LEJANT"));
    }
    assert_ne!(b.session.tool_id(), "colorLegend", "the tool leaves once it is placed");
    assert_eq!(b.doc.undo().as_deref(), Some("Renk planı lejandı"));
    assert!(legend_objects(&b).is_empty(), "one undo step");
}

#[test]
fn without_a_colour_plan_it_says_where_to_choose_one() {
    let b = bench(false);
    assert_eq!(b.last_text(), Some("Renk planı kapalı: önce Mahal ▸ Renk planı'ndan bir şema seçin."));
    assert_ne!(b.session.tool_id(), "colorLegend");
    assert!(legend_objects(&b).is_empty());
}
