//! A retired extension (plan/eklenti.md ADIM 22A › Emeklilik ve devir; docs/adr/8057 §119): its publisher's words and
//! its replacement in the manager, by name when installed, else a press away in the catalogs' list.

use kentos_extensions::manifest::Deprecation;

use super::super::view::retired_words;
use super::*;

#[test]
fn a_retired_extension_says_so_and_leads_to_its_replacement() {
    let d = Deprecation {
        message: "Kent Rehberi'ne taşındı.".into(),
        replaced_by: Some("tr.ornek.kent-rehberi".into()),
    };
    let (words, find) = retired_words(&d, None);
    assert_eq!(
        words,
        "Bu eklenti yayımcısınca emekliye ayrıldı: Kent Rehberi'ne taşındı. Yerine geçen: tr.ornek.kent-rehberi."
    );
    assert_eq!(find.as_deref(), Some("tr.ornek.kent-rehberi"));
    let (words, find) = retired_words(&d, Some("Kent Rehberi"));
    assert!(
        words.ends_with("Yerine geçen “Kent Rehberi” kurulu."),
        "{words}"
    );
    assert_eq!(find, None, "installed: nothing to look for");
    let alone = Deprecation {
        message: "Bakımı bitti".into(),
        replaced_by: None,
    };
    assert_eq!(
        retired_words(&alone, None),
        (
            "Bu eklenti yayımcısınca emekliye ayrıldı: Bakımı bitti.".to_owned(),
            None
        )
    );
    // Installed retired, its replacement looked for: the catalogs' list on its id.
    let root = temp("emekli");
    let mut gis = app(&root, Some(Module::Gis));
    install(
        &mut gis,
        "ornek-altlik.kek",
        package("ornek-altlik", |m| {
            m["deprecation"] = serde_json::json!({
                "message": "Kent Rehberi'ne taşındı.", "replacedBy": "tr.ornek.kent-rehberi" });
        }),
    );
    let manifest = gis
        .extensions
        .registry
        .as_ref()
        .and_then(|r| r.manifest("tr.ornek.altlik"))
        .cloned()
        .expect("installed");
    assert_eq!(manifest.deprecation.as_ref(), Some(&d));
    let _ = gis.extensions_view();
    let _ = gis.extensions_event(Event::Find("tr.ornek.kent-rehberi".into()));
    assert_eq!(gis.extensions.part, Part::Available);
    assert_eq!(gis.extensions.query, "tr.ornek.kent-rehberi");
    let _ = std::fs::remove_dir_all(&root);
}
