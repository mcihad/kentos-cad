//! An extension written in another language and translated to the interface's is shown translated (docs/adr/8057
//! §106): its manifest as the registry gives it, a file of its package read through its translation; one written in
//! the interface's language, or without a translation to it, as written.

use super::*;

const ID: &str = "tr.ornek.ceviri";

/// A package written in English with a Turkish translation of its name, summary, basemap and a panel's title.
fn translated(default: &str) -> Vec<u8> {
    let mut f = files(ID, "1.0.0", &[], |m| {
        m["name"] = json!("Grave lookup");
        m["summary"] = json!("Finds a grave.");
        m["contributions"]["dataSources"][0]["title"] = json!("Basemap");
        m["l10n"] = json!({ "default": default, "files": { "tr": "l10n/tr.json" } });
    });
    if default == "tr"
        && let Some(m) = f.get_mut("manifest.json")
    {
        let mut v: Value = serde_json::from_slice(m).unwrap();
        v["l10n"] = json!({ "default": "tr", "files": { "en": "l10n/tr.json" } });
        *m = serde_json::to_vec_pretty(&v).unwrap();
    }
    f.insert(
        "l10n/tr.json".to_owned(),
        serde_json::to_vec(&json!({
            "name": "Mezar ara",
            "summary": "Mezar bulur.",
            "contributions.dataSources.altlik.title": "Altlık",
            "requestedCapabilities.dataSource.reason": "Altlığı ekler.",
            "ui.ara.body.no.label": "Mezar no"
        }))
        .unwrap(),
    );
    f.insert(
        "ui/ara.panel.json".to_owned(),
        br#"{"title": "Grave lookup", "body": [{"id": "no", "label": "Grave no"}]}"#.to_vec(),
    );
    kentos_extensions::pack(&f, &Limits::default())
        .unwrap_or_else(|p| panic!("{p:#?}"))
        .bytes
}

#[test]
fn an_extension_translated_to_the_interfaces_language_is_shown_in_it() {
    let temp = Temp::new("ceviri");
    let mut r = open(&temp.0);
    install(&mut r, &translated("en"));
    let m = r.manifest(ID).expect("installed");
    assert_eq!(
        (m.name.as_str(), m.summary.as_str()),
        ("Mezar ara", "Mezar bulur.")
    );
    let reasons: Vec<&str> = m
        .requested_capabilities
        .iter()
        .map(|c| c.reason.as_str())
        .collect();
    assert_eq!(
        reasons,
        ["Altlığı ekler.", "Karo sunucusunu okur."],
        "an untranslated text as written"
    );
    assert!(r.texts(ID).is_some());
    // A file of its package through the translation: a place not translated stays as written.
    let read = |r: &Registry, prefix: &str| -> Value {
        serde_json::from_slice(
            &r.localized_file(ID, "ui/ara.panel.json", prefix)
                .expect("the file"),
        )
        .unwrap()
    };
    let panel = read(&r, "ui.ara");
    assert_eq!(
        (&panel["title"], &panel["body"][0]["label"]),
        (&json!("Grave lookup"), &json!("Mezar no"))
    );
    assert_eq!(
        read(&r, "ui.baska")["body"][0]["label"],
        "Grave no",
        "another panel's places"
    );
    // A package written in the interface's language is shown as written: its translation is to another language.
    let temp = Temp::new("ceviri-tr");
    let mut r = open(&temp.0);
    install(&mut r, &translated("tr"));
    assert_eq!(r.manifest(ID).expect("installed").name, "Grave lookup");
    assert!(r.texts(ID).is_none());
}
