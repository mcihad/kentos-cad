//! Eklenti denetimi's rule picker (docs/adr/8057 §93): a utility extension's rules (Türkiye mevzuatı opens no tab) are
//! offered in the program they join, chosen and run from the tab; another program offers none of them.

use super::*;

/// catalog/standards/tr-mevzuat packed from its source.
fn mevzuat() -> Vec<u8> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../catalog/standards/tr-mevzuat");
    let files: std::collections::BTreeMap<String, Vec<u8>> =
        kentos_extension_host::store::read_files(&root)
            .unwrap()
            .into_iter()
            .filter(|(path, _)| !path.contains("__pycache__"))
            .collect();
    kentos_extensions::pack(&files, &kentos_extensions::Limits::default())
        .unwrap_or_else(|p| panic!("{p:#?}"))
        .bytes
}

#[test]
fn a_utility_extensions_rules_are_offered_where_they_join() {
    let root = temp("kural-secici");
    // A drawing open: a run needs one.
    let mut mimari = crate::processing::tests::app_with_parcels();
    mimari.program = Some(Module::Architecture);
    mimari.extensions = Extensions::open(&root, false).0;
    mimari.extensions_changed();
    install(&mut mimari, "mevzuat.kek", mevzuat());
    let words: Vec<String> = mimari
        .ext_audit_choices()
        .into_iter()
        .map(|r| r.words)
        .collect();
    assert!(
        words.contains(&"Türkiye mevzuatı › İmar durumu: emsal ve TAKS (PAİY 4/1, 5/6)".to_owned())
            && words.contains(&"Türkiye mevzuatı › Çıkmaz koridor (BYKHY 4/i)".to_owned()),
        "{words:#?}"
    );
    // The tab shows the picker before any run; choosing keeps the choice.
    let _ = mimari.ext_audit_tab();
    let at = words
        .iter()
        .position(|w| w.contains("İmar durumu"))
        .unwrap();
    let _ = mimari.extensions_event(Event::AuditPick(at));
    assert_eq!(
        mimari.extensions.audit_pick,
        Some(("tr.kentos.mevzuat".to_owned(), "imar-durumu".to_owned()))
    );
    let choices = mimari.ext_audit_choices();
    assert_eq!(mimari.ext_audit_picked(&choices), Some(at));
    // A run started elsewhere (the ribbon, an agent) puts the picker on the rule it shows; its answer is given here
    // in place of the extension's process (no Python in tests).
    mimari.extensions.test_audit_answer = Some(Ok(serde_json::json!({
        "ok": true, "result": { "kind": "findings", "findings": [] }
    })));
    let _ = mimari.ext_audit_run("tr.kentos.mevzuat", "cikmaz-koridor");
    let shown = mimari.extensions.audit.as_ref().expect("the run shown");
    assert_eq!(shown.local, "cikmaz-koridor");
    assert_eq!(mimari.extensions.audit_pick, None);
    let cikmaz = words.iter().position(|w| w.contains("Çıkmaz koridor"));
    assert_eq!(mimari.ext_audit_picked(&choices), cikmaz);
    let _ = mimari.ext_audit_tab();
    // In GIS the same registry offers none of them: they join Mimari's audit.
    let gis = app(&root, Some(Module::Gis));
    assert!(gis.ext_audit_choices().is_empty());
    let _ = gis.ext_audit_tab();
}
