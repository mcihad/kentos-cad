//! Tanımlı özniteliğin boşluklu etiketinin geri adı, docs/adr/4800 (plan/map-cad.md MAP-17), seen where the user meets
//! it: on fixtures/exchange/v1/devir-blok.json's Rögar given an attribute definition named “Kapak tipi” and its
//! insert a value, Dışa aktarma kaybı denetimi's DXF trip loses no attribute and no block: the ATTDEF and the ATTRIB go
//! out as “Kapak_tipi” and come back under the name. And its pictures.

use kentos_launch::Module;
use serde_json::json;

use crate::app::{App, Message};
use crate::exchange_check_window::Event as X;

const DRAWING: &str = include_str!("../../../fixtures/exchange/v1/devir-blok.json");
const ROGAR: &str = "0192f660-1660-7000-8000-00000000b001";

/// The block drawing with Rögar's “Kapak tipi” defined and its insert's value written, each by its command.
fn opened() -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_str(DRAWING).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    crate::program_scenes::command(&mut app, "cad.blocks.edit", json!({"operation": "attributes", "block": ROGAR, "attributes": [
        {"tag": "Kapak tipi", "p": {"x": 0.9, "y": 0.0}, "height": 0.3, "rotation": 0.0, "value": "D400"}
    ]}));
    let doc = &app.document.as_ref().expect("a drawing").model;
    let insert = doc
        .entities_with_uids()
        .find_map(|(u, e)| matches!(e, kentos_commands::Entity::Insert(_)).then(|| u.to_string()))
        .expect("the insert");
    crate::program_scenes::command(&mut app, "cad.entities.set", json!({"uids": [insert], "attrs": {"Kapak tipi": "C250"}, "operation": "attributes"}));
    app
}

#[test]
fn the_dxf_trip_keeps_a_defined_attribute_named_with_a_space() {
    let mut app = opened();
    let _ = app.update(Message::Run("file.exchangeCheck"));
    let _ = app.update(Message::ExchangeCheck(X::Format("dxf".into())));
    let _ = app.update(Message::ExchangeCheck(X::Run));
    let r = app.exchange_check.answer.clone().expect("an answer").expect("checked");
    let named = |l: &kentos_commands::ExchangeLoss| l.detail.as_deref().is_some_and(|d| d.contains("Kapak"));
    assert!(!r.losses.iter().any(named), "{:?}", r.losses);
    assert!(!r.losses.iter().any(|l| l.kind == "blockChanged"), "{:?}", r.losses);
    // The trip went through Rögar's definition and value: the reader's copy has the block back with the name.
    assert!(r.matched > 0 && r.losses.iter().all(|l| !matches!(l.kind.as_str(), "attrLost" | "attrAdded" | "attrChanged")), "{:?}", r.losses);
}

/// `.run/gui-evidence/oznitelik-adi-4800/`: the DXF trip of the drawing, no attribute or block lost.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib dxf_attribute_names_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/oznitelik-adi-4800");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("file.exchangeCheck"));
        let _ = app.update(Message::ExchangeCheck(X::Format("dxf".into())));
        let _ = app.update(Message::ExchangeCheck(X::Run));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-dxf-gidis-donus-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
