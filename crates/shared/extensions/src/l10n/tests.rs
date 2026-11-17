//! An extension's translations (l10n.rs, docs/adr/8057 §106).

use serde_json::json;

use super::*;

fn translation(texts: serde_json::Value) -> Texts {
    Texts::read(texts.to_string().as_bytes()).expect("a translation")
}

/// A text is reached by its place: an object's key, a list's item by its id, else its name, else its number; an id, a
/// command, a value and an expression are no texts, though they read the same. An empty translation is none.
#[test]
fn a_translation_reaches_the_texts_by_their_places() {
    let texts = translation(json!({
        "name": "Mezar bilgi",
        "summary": "Mezarları gösterir.",
        "entrypoints.commands.init-schema.title": "Katmanları kur",
        "contributions.ribbonTabs.mebis.label": "Mezarlık BS",
        "contributions.ribbonTabs.mebis.panels.0.label": "Proje",
        "ui.search-dock.body.query.label": "Mezar no",
        "ui.search-dock.body.kind.options.grave.label": "Mezar",
        "schema.layers.mezar.fields.no.alias": "Mezar no",
        "entrypoints.commands.search.title": ""
    }));
    let mut manifest = json!({
        "id": "tr.ornek.mezar",
        "name": "Grave information",
        "summary": "Shows the graves.",
        "entrypoints": { "commands": [
            { "id": "init-schema", "title": "Set up the layers", "handler": "m.logic:init" },
            { "id": "search", "title": "Search" }
        ] },
        "contributions": { "ribbonTabs": [
            { "id": "mebis", "label": "Cemetery IS", "panels": [{ "label": "Project", "items": [] }] }
        ] }
    });
    texts.apply(&mut manifest, "");
    assert_eq!(manifest["name"], "Mezar bilgi");
    assert_eq!(manifest["summary"], "Mezarları gösterir.");
    assert_eq!(
        manifest["entrypoints"]["commands"][0]["title"],
        "Katmanları kur"
    );
    assert_eq!(
        manifest["entrypoints"]["commands"][0]["handler"],
        "m.logic:init"
    );
    assert_eq!(
        manifest["entrypoints"]["commands"][1]["title"], "Search",
        "an empty translation is none"
    );
    assert_eq!(
        manifest["contributions"]["ribbonTabs"][0]["label"],
        "Mezarlık BS"
    );
    assert_eq!(
        manifest["contributions"]["ribbonTabs"][0]["panels"][0]["label"],
        "Proje"
    );
    let mut panel = json!({ "title": "Search", "body": [
        { "type": "textField", "id": "query", "label": "Grave no", "help": "A grave's number." },
        { "type": "select", "id": "kind", "options": [{ "value": "grave", "label": "Grave" }] }
    ] });
    texts.apply(&mut panel, "ui.search-dock");
    assert_eq!(panel["body"][0]["label"], "Mezar no");
    assert_eq!(
        panel["body"][0]["help"], "A grave's number.",
        "untranslated: as written"
    );
    assert_eq!(panel["body"][1]["options"][0]["label"], "Mezar");
    assert_eq!(panel["body"][1]["options"][0]["value"], "grave");
    // A form package's own name at its root is a text; its fields' names are their ids.
    let mut form = json!({ "format": "kentos.form-package", "name": "Burial record",
                           "fields": [{ "name": "no", "alias": "Grave no" }] });
    translation(json!({ "forms.burial.name": "Defin kaydı" })).apply(&mut form, "forms.burial");
    assert_eq!(
        (&form["name"], &form["fields"][0]["name"]),
        (&json!("Defin kaydı"), &json!("no"))
    );
    let mut schema =
        json!({ "layers": [{ "id": "mezar", "fields": [{ "name": "no", "alias": "Grave no" }] }] });
    texts.apply(&mut schema, "schema");
    assert_eq!(schema["layers"][0]["fields"][0]["alias"], "Mezar no");
    assert_eq!(
        schema["layers"][0]["fields"][0]["name"], "no",
        "a field's name is its id"
    );
}

/// A file of another shape is said why.
#[test]
fn a_translation_file_is_a_flat_object_of_texts() {
    assert!(Texts::read(b"[]").unwrap_err().contains("nesnesi değil"));
    assert!(Texts::read(br#"{"a": 1}"#).unwrap_err().contains("“a”"));
    assert!(Texts::read(b"{").unwrap_err().contains("JSON"));
    assert!(Texts::read(br#"{"name": "Ad"}"#).is_ok());
}

/// Gate 1 says the places no text stands at, the first five named and the rest counted; a text left untranslated is
/// no mistake.
#[test]
fn gate_one_says_what_a_translation_left_behind() {
    let manifest =
        json!({ "name": "Find", "entrypoints": { "commands": [{ "id": "a", "title": "Go" }] } });
    let mut texts = texts_at(&manifest, "");
    texts.extend(texts_at(
        &json!({ "title": "Panel", "body": [{ "id": "x", "label": "One" }] }),
        "ui.p",
    ));
    assert_eq!(
        texts.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "entrypoints.commands.a.title",
            "name",
            "ui.p.body.x.label",
            "ui.p.title"
        ]
    );
    let said = check(
        "l10n/tr.json",
        &translation(json!({ "name": "Bul", "entrypoints.commands.old.title": "Eski" })),
        &texts,
    )
    .expect("said");
    assert_eq!(said.code, "l10n_stale");
    assert!(
        said.message.contains("“entrypoints.commands.old.title”"),
        "{}",
        said.message
    );
    assert_eq!(said.file.as_deref(), Some("l10n/tr.json"));
    assert!(
        check(
            "l10n/tr.json",
            &translation(json!({ "name": "Bul" })),
            &texts
        )
        .is_none()
    );
    let many: serde_json::Map<String, serde_json::Value> =
        (0..8).map(|i| (format!("t{i}"), json!("x"))).collect();
    let said = check(
        "l10n/tr.json",
        &translation(serde_json::Value::Object(many)),
        &BTreeMap::new(),
    )
    .expect("said");
    assert!(said.message.contains("ve 3 tane daha"), "{}", said.message);
}

/// MEBİS's English translation reaches its texts: none is left behind (docs/adr/8057 §106; its file came first).
#[test]
fn mebis_english_translation_has_no_place_without_a_text() {
    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/extensions/v1/mebis/manifest.json"
    ))
    .unwrap();
    let english = Texts::read(include_bytes!(
        "../../../../../fixtures/extensions/v1/mebis/l10n/en.json"
    ))
    .unwrap();
    let texts = texts_at(&manifest, "");
    let stale: Vec<&String> = english
        .places()
        .filter(|p| !texts.contains_key(*p))
        .collect();
    assert!(stale.is_empty(), "{stale:?}");
}
