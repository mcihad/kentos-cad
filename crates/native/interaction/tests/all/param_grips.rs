//! A dynamic block's parameter grips under the select tool (docs/adr/0319 §1): a door 1 m wide at (E, N), its width
//! on 0.1 m steps from 0.6 to 1.5 m, a flip about its middle. Dragging the width's grip writes the nearest step
//! through `cad.blocks.parameters`, one undo step; clicking the flip's grip turns it. Expected values are worked out
//! by hand.

use crate::common;

use common::{Bench, E, N};
use kentos_commands::{Entity, ParamValue};
use kentos_domain::Slot;
use kentos_interaction::Format;

fn drawing() -> String {
    format!(
        r#"{{"format":"kentos.document","version":1,"name":"Parametre tutamaçları",
 "settings":{{"srid":5256,"lengthDecimals":2,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":500,"drawingFont":"barlow"}},
 "origin":{{"x":{E},"y":{N}}},
 "layers":[{{"id":"a","name":"Kapılar","type":"layer","visible":true,"locked":false,"expanded":true,"style":{{"color":"fg","lineType":"continuous","lineWeight":0.25}},"children":[]}}],
 "activeLayer":"a",
 "styles":{{"items":[],"categories":[]}},
 "blocks":[{{"id":"018f3a2b-0000-7000-8000-0000000000d1","name":"Kapı","base":{{"x":0,"y":0}},"entities":[
   {{"kind":"line","id":1,"layerId":"","attrs":{{}},"a":{{"x":0,"y":0}},"b":{{"x":1,"y":0}}}},
   {{"kind":"line","id":2,"layerId":"","attrs":{{}},"a":{{"x":1,"y":0}},"b":{{"x":1,"y":1}}}}],
   "dynamic":{{"parameters":[
     {{"kind":"linear","name":"Genişlik","from":{{"x":0,"y":0}},"to":{{"x":1,"y":0}},"rule":{{"min":0.6,"max":1.5,"step":0.1}}}},
     {{"kind":"flip","name":"Sol el","a":{{"x":0.5,"y":0}},"b":{{"x":0.5,"y":1}}}}],
    "actions":[{{"kind":"stretch","parameter":"Genişlik","min":{{"x":0.9,"y":-1}},"max":{{"x":2,"y":2}},"pieces":[1,2]}},
               {{"kind":"flip","parameter":"Sol el","pieces":[1,2]}}]}}}}],
 "entities":[{{"kind":"insert","id":1,"layerId":"a","attrs":{{}},"block":"018f3a2b-0000-7000-8000-0000000000d1","p":{{"x":{E},"y":{N}}},"scale":4,"rotation":0}}]}}"#
    )
}

fn bench() -> Bench {
    let mut b = Bench::on(&drawing());
    b.draft.snap = false;
    b.selection.set([Slot(1)]);
    b
}

fn values(b: &Bench) -> Vec<(String, ParamValue)> {
    let Some(Entity::Insert(i)) = b.doc.get(Slot(1)) else {
        panic!("the door");
    };
    i.parameters.clone().into_iter().collect()
}

#[test]
fn the_width_grip_dragged_writes_the_nearest_step() {
    let mut b = bench();
    // Four times the block: the width's grip 4 m east of the door's corner, the flip's in its axis's middle.
    let grips = b.spatial.param_grips(&[Slot(1)]);
    let at: Vec<[f64; 2]> = grips
        .iter()
        .map(|(_, g)| [g.at.x - E, g.at.y - N])
        .collect();
    assert_eq!(at, [[4.0, 0.0], [2.0, 2.0]]);
    // Dragged to 4.9 m: 1.225 of the block, the step 1.2.
    b.drag([4.0, 0.0], [4.9, 0.3]);
    let v = values(&b);
    assert_eq!(v.len(), 1);
    let ParamValue::Number(w) = &v[0].1 else {
        panic!("a number")
    };
    assert_eq!(v[0].0, "Genişlik");
    assert!((w - 1.2).abs() < 1e-12, "{w}");
    assert_eq!(b.doc.undo().as_deref(), Some("Blok parametreleri"));
    assert!(values(&b).is_empty());
}

#[test]
fn a_clicked_grip_is_hot_previews_and_takes_a_typed_length() {
    let mut b = bench();
    b.click(4.0, 0.0);
    assert!(b.session.grip_active());
    assert_eq!(
        b.session.prompt().text(),
        "Blok parametresi: “Genişlik” uzunluğu ya da noktası"
    );
    b.move_to(9.0, 0.0);
    let preview = b.session.preview(&Format::default()).expect("a preview");
    // Past the rule's largest the value stops at 1.5.
    assert_eq!(preview.tag.expect("a tag").lines, ["Genişlik: 1.500 m"]);
    assert!(!preview.strokes.is_empty());
    // Typed 0.8: the door 0.8 wide.
    assert!(b.type_text("0.8"));
    let v = values(&b);
    let ParamValue::Number(w) = &v[0].1 else {
        panic!("a number")
    };
    assert!((w - 0.8).abs() < 1e-12, "{w}");
    assert!(!b.session.grip_active());
}

#[test]
fn the_flip_grip_clicked_turns_and_esc_leaves_a_hot_grip() {
    let mut b = bench();
    b.click(2.0, 2.0);
    assert_eq!(values(&b), [("Sol el".to_owned(), ParamValue::Flag(true))]);
    assert_eq!(b.last_text(), Some("“Sol el” açıldı."));
    b.click(2.0, 2.0);
    assert!(values(&b).is_empty(), "off again: the default is not kept");
    // Esc forgets a hot grip; nothing written.
    b.click(4.0, 0.0);
    assert!(b.run(|s, cx| s.cancel(cx)));
    assert!(!b.session.grip_active());
    assert!(values(&b).is_empty());
}
