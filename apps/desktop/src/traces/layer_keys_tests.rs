//! Katman anahtarı on the 2+1 flat (docs/adr/3008). The mouse-and-keyboard trace (fixtures/interaction/v1/
//! acceptance-flat.json) played as KentOS Mimari starts (the key on, Dış duvar 25) puts its objects on their kinds' EK-4
//! layers, at least eight. A DXF written of it, read back as another program reads it, has the same layers and the
//! walls' hatches on their "-T" (EK-4 4.14.5). Before, all 54 objects were on Çizim.
//!
//! `KENTOS_EK4_DXF=path` keeps the DXF there (the evidence's external reading).

use std::collections::BTreeSet;

use super::player::AREA;
use super::{Player, Trace, VARIANTS, scratch_file};
use crate::app::App;

#[test]
fn the_2p1_flat_goes_on_its_kinds_ek4_layers_and_its_dxf_keeps_them() {
    let traces = Trace::all().expect("the traces read");
    let trace = traces
        .iter()
        .find(|t| t.id == "acceptance-flat")
        .expect("the 2+1 trace");
    let variant = VARIANTS[0];
    let (mut app, _) = App::boot(None);
    // As KentOS Mimari starts (arch_start.rs): the layer key on, the wall tools on Dış duvar 25.
    app.memory.layer_keys = true;
    let style = kentos_commands::builtin_wall_styles()
        .into_iter()
        .find(|s| s.id == crate::arch_start::FIRST_WALL_STYLE)
        .expect("KentOS's exterior wall");
    kentos_interaction::wall::take_style(&mut app.memory, &style);
    let problems = Player::new(&mut app, trace, variant, AREA, scratch_file(trace, variant))
        .expect("the trace starts")
        .play(None);
    assert!(problems.is_empty(), "{problems:#?}");
    let doc = &app.document.as_ref().expect("the flat").model;
    let name = |id: &str| {
        doc.layers()
            .get(id)
            .map_or_else(|| id.to_owned(), |l| l.name.clone())
    };
    let used: BTreeSet<String> = doc.entities().map(|e| name(&e.base().layer_id)).collect();
    for want in [
        "MM-DUVA-TUGLA",
        "MM-KAPI-GENEL",
        "MM-PENC-GENEL",
        "MM-XXXX-MAHAL",
        "MM-TEFR-MUTFAK",
        "MM-XXXX-OLCU",
        "MM-DOSM-GENEL",
        "MM-XXXX-ETIKET",
        "MM-XXXX-CIZELGE",
    ] {
        assert!(used.contains(want), "{want} missing: {used:?}");
    }
    let ek4: Vec<&String> = used
        .iter()
        .filter(|n| kentos_commands::is_ek4_layer(n))
        .collect();
    assert!(ek4.len() >= 8, "{used:?}");
    assert!(
        !used.contains("Çizim"),
        "nothing left on the active layer: {used:?}"
    );
    // The DXF as another program reads it: KentOS's data left out, the objects as their pieces.
    let objects: Vec<&kentos_commands::Entity> = doc.entities().collect();
    let chosen = kentos_headless::export::by_layer(doc, objects);
    let mut input = kentos_headless::export::dxf_input(doc, &chosen);
    input.annotations.metadata = false;
    let (bytes, _) = kentos_formats::dxf::write(&input);
    if let Ok(path) = std::env::var("KENTOS_EK4_DXF") {
        std::fs::write(&path, &bytes).expect("the DXF kept");
    }
    let read = kentos_formats::dxf::read(&bytes, &kentos_commands::DxfReadOptions::default())
        .expect("the DXF reads");
    let in_file: BTreeSet<String> = read
        .entities
        .iter()
        .map(|e| e.base().layer_id.clone())
        .collect();
    for layer in &ek4 {
        assert!(
            in_file.contains(*layer),
            "{layer} not in the DXF: {in_file:?}"
        );
    }
    assert!(
        in_file.contains("MM-DUVA-TUGLA-T"),
        "the walls' hatches: {in_file:?}"
    );
    let hatch_layers: BTreeSet<&str> = read
        .entities
        .iter()
        .filter(|e| matches!(e, kentos_commands::Entity::Hatch(_)))
        .map(|e| e.base().layer_id.as_str())
        .collect();
    assert!(
        hatch_layers.iter().all(|l| l.ends_with("-T")),
        "{hatch_layers:?}"
    );
}
