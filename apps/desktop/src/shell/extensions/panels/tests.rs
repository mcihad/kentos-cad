//! A panel's values and inputs (`panels.rs`): what each field gives as the command's parameter (a date, a time, a
//! choice, a number as typed), a required field left empty, its rules, its pickers, Enter, and a card's rows.

use super::*;

fn panel() -> OpenPanel {
    let value = json!({
        "format": "kentos.ui.panel", "schemaVersion": 1, "title": "Yapı ara",
        "body": [
            { "type": "textField", "id": "sorgu", "label": "Yapı no", "value": " Y-0 " },
            { "type": "numberField", "id": "en-az", "label": "En az alan", "min": 0 },
            { "type": "select", "id": "durum", "label": "Durum",
              "options": [{ "value": "hepsi", "label": "Hepsi" }, { "value": "numarasiz", "label": "Numarasız" }] },
            { "type": "checkbox", "id": "secili", "label": "Yalnız seçililer" },
            { "type": "button", "id": "ara", "label": "Ara", "command": "yapi-ara" },
            { "type": "table", "id": "sonuc" }
        ]
    });
    OpenPanel::new(
        "tr.ornek.vitrin",
        "yapi-ara",
        kentos_extensions::ui_panel::read(&value).unwrap(),
    )
}

/// A date field's value is the ISO date of what was typed, either way it was written; a text that is no day, and
/// a day out of its bounds, are said by its label; empty is none.
#[test]
fn a_components_rules_follow_the_panels_values() {
    // docs/adr/8057 §33: a rule is worked out from the panel's values as they change: a field shown and asked
    // while a check is on, a button usable once a number is above 0 (typed with a comma), no Python.
    let value = json!({ "format": kentos_extensions::ui_panel::FORMAT, "schemaVersion": kentos_extensions::ui_panel::VERSION,
        "title": "Kart",
        "body": [
            { "type": "checkbox", "id": "sureli", "label": "Süreli" },
            { "type": "dateTimeField", "id": "gecerli", "label": "Geçerlilik sonu",
              "visibleWhen": "sureli", "requiredWhen": "sureli" },
            { "type": "numberField", "id": "adet", "label": "Adet" },
            { "type": "button", "id": "kaydet", "label": "Kaydet", "command": "kaydet",
              "enabledWhen": "adet > 0" }
        ] });
    let mut p = OpenPanel::new(
        "tr.ornek.deneme",
        "kart",
        kentos_extensions::ui_panel::read(&value).expect("a panel"),
    );
    let body = p.definition.body.clone();
    assert!(!p.shown(1, &body[1]), "hidden while the check is off");
    assert!(p.check_params().is_ok(), "hidden: not asked");
    assert!(!p.usable(3, &body[3]), "no number yet");
    p.checks.insert("sureli".into(), true);
    assert!(p.shown(1, &body[1]) && p.asked(1, &body[1]));
    assert_eq!(
        p.check_params().unwrap_err().0,
        "gecerli",
        "shown and asked: left empty"
    );
    p.texts.insert("gecerli".into(), "20.11.2026 17:00".into());
    p.texts.insert("adet".into(), "2,5".into());
    assert!(p.check_params().is_ok());
    assert!(p.usable(3, &body[3]), "2,5 > 0");
    // The code's own switches still win over a rule that holds.
    p.disabled.insert("kaydet".into());
    assert!(!p.usable(3, &body[3]));
}

#[test]
fn a_date_field_gives_the_iso_date_of_what_was_typed() {
    let value = json!({
        "format": "kentos.ui.panel", "schemaVersion": 1, "title": "Tarih",
        "body": [{ "type": "dateField", "id": "tarih", "label": "Kart tarihi", "value": "2026-11-15",
                   "min": "2026-01-01", "max": "2026-12-31" }]
    });
    let mut p = OpenPanel::new(
        "tr.ornek.vitrin",
        "tarih",
        kentos_extensions::ui_panel::read(&value).unwrap(),
    );
    assert_eq!(
        p.texts["tarih"], "15.11.2026",
        "its value as the field shows it"
    );
    assert_eq!(p.params().unwrap(), json!({ "tarih": "2026-11-15" }));
    p.texts.insert("tarih".into(), "3.2.2026".into());
    assert_eq!(p.params().unwrap(), json!({ "tarih": "2026-02-03" }));
    p.texts.insert("tarih".into(), "30.02.2026".into());
    assert_eq!(
        p.params().unwrap_err(),
        "“Kart tarihi”: “30.02.2026” bir tarih değil (GG.AA.YYYY ya da YYYY-AA-GG yazın)."
    );
    p.texts.insert("tarih".into(), "2025-12-31".into());
    assert_eq!(
        p.params().unwrap_err(),
        "“Kart tarihi” 01.01.2026 ile 31.12.2026 arasında olmalı."
    );
    p.texts.insert("tarih".into(), " ".into());
    assert_eq!(p.params().unwrap(), json!({ "tarih": null }));
}

/// A time field gives HH:MM and a date-time field the local ISO date and time, either way they were written; a
/// text that is neither, and one out of bounds, said by the label (the bounds in the field's own words).
#[test]
fn a_time_and_a_moment_field_give_their_values_however_written() {
    let value = json!({
        "format": "kentos.ui.panel", "schemaVersion": 1, "title": "Saat",
        "body": [
            { "type": "timeField", "id": "saat", "label": "Saat", "value": "09:00", "min": "08:00",
              "max": "17:30" },
            { "type": "dateTimeField", "id": "gecerli", "label": "Geçerlilik sonu",
              "value": "2026-11-20T17:00", "min": "2026-11-15T00:00" }
        ]
    });
    let mut p = OpenPanel::new(
        "tr.ornek.vitrin",
        "saat",
        kentos_extensions::ui_panel::read(&value).unwrap(),
    );
    assert_eq!(
        (p.texts["saat"].as_str(), p.texts["gecerli"].as_str()),
        ("09:00", "20.11.2026 17:00"),
        "their values as the fields show them"
    );
    assert_eq!(
        p.params().unwrap(),
        json!({ "saat": "09:00", "gecerli": "2026-11-20T17:00" })
    );
    p.texts.insert("saat".into(), "9.05".into());
    p.texts.insert("gecerli".into(), "2026-11-21 8:30".into());
    assert_eq!(
        p.params().unwrap(),
        json!({ "saat": "09:05", "gecerli": "2026-11-21T08:30" })
    );
    p.texts.insert("saat".into(), "25:00".into());
    assert_eq!(
        p.params().unwrap_err(),
        "“Saat”: “25:00” bir saat değil (SS:DD yazın, ör. 14:30)."
    );
    p.texts.insert("saat".into(), "18:00".into());
    assert_eq!(
        p.params().unwrap_err(),
        "“Saat” 08:00 ile 17:30 arasında olmalı."
    );
    p.texts.insert("saat".into(), String::new());
    p.texts.insert("gecerli".into(), "14.11.2026 23:59".into());
    assert_eq!(
        p.params().unwrap_err(),
        "“Geçerlilik sonu” en erken 15.11.2026 00:00 olabilir."
    );
    p.texts.insert("gecerli".into(), "15.11.2026".into());
    assert!(
        p.params().unwrap_err().contains("bir tarih ve saat değil"),
        "a moment needs its time"
    );
    p.texts.insert("gecerli".into(), String::new());
    assert_eq!(
        p.params().unwrap(),
        json!({ "saat": null, "gecerli": null })
    );
}

/// A field the user must give (`required`) left empty is said by its label, with its id for the form to say it
/// under the field; changing the field takes the words away.
#[test]
fn a_required_field_left_empty_is_said_under_it() {
    let value = json!({
        "format": "kentos.ui.panel", "schemaVersion": 1, "title": "Zorunlu",
        "body": [
            { "type": "textField", "id": "onek", "label": "Önek", "required": true },
            { "type": "numberField", "id": "ilk", "label": "İlk numara", "value": 1, "required": true },
            { "type": "layerPicker", "id": "katman", "label": "Katman", "required": true },
            { "type": "featurePicker", "id": "nesneler", "label": "Nesneler", "layer": "katman", "required": true },
            { "type": "multiSelect", "id": "bilgi", "label": "Bilgiler", "required": true,
              "options": [{ "value": "a", "label": "A" }] },
            { "type": "textField", "id": "not", "label": "Not" }
        ]
    });
    let mut p = OpenPanel::new(
        "tr.ornek.vitrin",
        "zorunlu",
        kentos_extensions::ui_panel::read(&value).unwrap(),
    );
    let wrong = |p: &OpenPanel| p.check_params().unwrap_err();
    assert_eq!(
        wrong(&p),
        ("onek".to_owned(), "“Önek” boş bırakılamaz.".to_owned())
    );
    p.texts.insert("onek".into(), "Y-".into());
    p.texts.insert("ilk".into(), "  ".into());
    assert_eq!(wrong(&p).1, "“İlk numara” boş bırakılamaz.");
    p.texts.insert("ilk".into(), "3".into());
    assert_eq!(
        wrong(&p),
        ("katman".to_owned(), "“Katman” seçilmedi.".to_owned())
    );
    p.choices.insert("katman".into(), "yapi".into());
    assert!(wrong(&p).1.contains("nesne alınmadı"));
    p.picks.insert("nesneler".into(), vec!["u-1".into()]);
    assert_eq!(wrong(&p).1, "“Bilgiler”: en az biri seçilmeli.");
    p.multi.insert("bilgi".into(), vec!["a".into()]);
    let values = p.check_params().expect("all given; Not need not be");
    assert_eq!(values["not"], "", "an optional field left empty");
    // The wrong field's words stay until it is changed.
    p.invalid = Some(("onek".into(), "“Önek” boş bırakılamaz.".into()));
    p.said = Some((words::Kind::Error, "“Önek” boş bırakılamaz.".into()));
    p.forget_invalid(&PanelEvent::Text("not".into(), "x".into()));
    assert!(
        p.invalid.is_some() && p.said.is_some(),
        "another field changed"
    );
    p.forget_invalid(&PanelEvent::Text("onek".into(), "B-".into()));
    assert!(p.invalid.is_none());
    assert!(
        p.said.is_none(),
        "the field's words under the form go with it"
    );
}

/// A multiple choice gives the chosen values in the options' order, an empty list when none is chosen.
#[test]
fn a_multiple_choice_gives_the_chosen_values_in_the_options_order() {
    let value = json!({
        "format": "kentos.ui.panel", "schemaVersion": 1, "title": "Bilgiler",
        "body": [{ "type": "multiSelect", "id": "bilgiler", "label": "Bilgiler", "value": ["taks", "alan"],
                   "options": [{ "value": "alan", "label": "Alan" }, { "value": "yapi", "label": "Yapı" },
                               { "value": "taks", "label": "TAKS" }] }]
    });
    let mut p = OpenPanel::new(
        "tr.ornek.vitrin",
        "bilgiler",
        kentos_extensions::ui_panel::read(&value).unwrap(),
    );
    assert_eq!(p.params().unwrap(), json!({ "bilgiler": ["alan", "taks"] }));
    p.multi.insert("bilgiler".into(), Vec::new());
    assert_eq!(p.params().unwrap(), json!({ "bilgiler": [] }));
}

#[test]
fn a_card_takes_a_commands_rows_where_a_table_would() {
    let value = json!({
        "format": "kentos.ui.panel", "schemaVersion": 1, "title": "Parsel kartı",
        "body": [
            { "type": "featurePicker", "id": "parseller", "label": "Parseller", "onChange": "parsel-karti" },
            { "type": "card", "id": "kartlar", "title": "parsel", "tag": "durum" }
        ]
    });
    let p = OpenPanel::new(
        "tr.ornek.vitrin",
        "parsel-paneli",
        kentos_extensions::ui_panel::read(&value).unwrap(),
    );
    assert_eq!(
        p.table_of(None).as_deref(),
        Some("kartlar"),
        "the panel's first output"
    );
    assert_eq!(panel().table_of(None).as_deref(), Some("sonuc"));
    // Before its first rows it says what fills it: no button here, a change.
    assert_eq!(
        p.waiting_words("kartlar", "kart kart listelenir"),
        "Değer değişince komutun satırları burada kart kart listelenir."
    );
    assert_eq!(
        panel().waiting_words("sonuc", "listelenir"),
        "Düğmeye basınca komutun satırları burada listelenir."
    );
}

#[test]
fn pickers_follow_their_layer_picker_and_give_names_and_ids() {
    let value = json!({
        "format": "kentos.ui.panel", "schemaVersion": 1, "title": "Değer say",
        "body": [
            { "type": "layerPicker", "id": "katman", "label": "Katman" },
            { "type": "fieldPicker", "id": "alan", "label": "Alan", "layer": "katman" },
            { "type": "featurePicker", "id": "nesneler", "label": "Nesneler", "layer": "katman" },
            { "type": "featurePicker", "id": "bir", "label": "Bir nesne", "multiple": false },
            { "type": "button", "id": "say", "label": "Say", "command": "deger-say" }
        ]
    });
    let mut p = OpenPanel::new(
        "tr.ornek.vitrin",
        "deger-paneli",
        kentos_extensions::ui_panel::read(&value).unwrap(),
    );
    assert_eq!(
        p.params().unwrap(),
        json!({ "katman": null, "alan": null, "nesneler": [], "bir": null }),
        "nothing chosen: none, no objects"
    );
    p.choose("katman".into(), "parsel".into());
    p.choose("alan".into(), "ada".into());
    p.picks
        .insert("nesneler".into(), vec!["a".into(), "b".into()]);
    p.picks.insert("bir".into(), vec!["c".into()]);
    assert_eq!(
        p.params().unwrap(),
        json!({ "katman": "parsel", "alan": "ada", "nesneler": ["a", "b"], "bir": "c" })
    );
    // The same layer again keeps them; another clears what was chosen from the one before, not the free picker.
    p.choose("katman".into(), "parsel".into());
    assert_eq!(p.params().unwrap()["alan"], json!("ada"));
    p.choose("katman".into(), "yapi".into());
    assert_eq!(
        p.params().unwrap(),
        json!({ "katman": "yapi", "alan": null, "nesneler": [], "bir": "c" })
    );
}

#[test]
fn enter_presses_the_primary_button_else_the_first() {
    let p = panel();
    assert_eq!(p.default_button(), Some("ara"), "the only button");
    let value = json!({
        "format": "kentos.ui.panel", "schemaVersion": 1, "title": "İki düğme",
        "body": [
            { "type": "textField", "id": "sorgu", "label": "Aranan" },
            { "type": "button", "id": "numarala", "label": "Numarala", "command": "a" },
            { "type": "button", "id": "ara", "label": "Ara", "command": "b", "primary": true }
        ]
    });
    let two = OpenPanel::new("x", "y", kentos_extensions::ui_panel::read(&value).unwrap());
    assert_eq!(
        two.default_button(),
        Some("ara"),
        "the primary one, wherever it is"
    );
}

#[test]
fn the_values_are_the_commands_parameters_a_number_read_as_typed() {
    let mut p = panel();
    assert_eq!(
        p.params().unwrap(),
        json!({ "sorgu": "Y-0", "en-az": null, "durum": "hepsi", "secili": false })
    );
    p.texts.insert("en-az".into(), "12,5".into());
    assert_eq!(p.params().unwrap()["en-az"], json!(12.5));
    p.texts.insert("en-az".into(), "on iki".into());
    assert_eq!(
        p.params().unwrap_err(),
        "“En az alan”: “on iki” bir sayı değil."
    );
    p.texts.insert("en-az".into(), "-3".into());
    assert!(p.params().unwrap_err().contains("arasında olmalı"));
    assert_eq!(
        p.table_of(None).as_deref(),
        Some("sonuc"),
        "the panel's first table"
    );
}
