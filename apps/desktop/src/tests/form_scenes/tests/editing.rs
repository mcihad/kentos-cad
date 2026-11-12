//! Form tests: chips, what changed, kinds' tabs, related tables, value groups, contingents, recomputing,
//! scenarios, the form view, duplicates, renamed fields, records written and completed, the designer's history and
//! preview, relations, audits, photos, dates and records added in the table.

use super::*;

#[test]
fn a_searched_list_and_chips_write_codes() {
    let mut app = crate::files_testing::app_with_drawing();
    searched_list(&mut app);
    // A pick writes the code; the chips' add keeps the list's order.
    let _ = app.update(Message::FormEntry(entry::Event::Value(
        "İl".into(),
        "07".into(),
    )));
    let _ = app.update(Message::FormEntry(entry::Event::Value(
        "Hizmetler".into(),
        "su; elektrik; internet".into(),
    )));
    let _ = app.update(Message::FormEntry(entry::Event::Save));
    let doc = &app.document.as_ref().unwrap().model;
    let attrs = &doc.get(Slot(1)).unwrap().base().attrs;
    assert_eq!(attrs.get("İl").map(String::as_str), Some("07"));
    assert_eq!(
        attrs.get("Hizmetler").map(String::as_str),
        Some("su; elektrik; internet")
    );
    assert_eq!(
        super::super::super::forms::entry::ordered_for_tests(
            &["internet", "su"],
            &["su", "elektrik", "internet"]
        ),
        "su; internet"
    );
}

#[test]
fn the_designer_says_what_changed_since_the_release() {
    let mut app = crate::files_testing::app_with_drawing();
    form_changes(&mut app);
    let w = app.forms_window.as_ref().unwrap();
    assert!(w.changes);
    let form = app.chosen_form().unwrap();
    let changes = kentos_forms::compare::compare(
        &form.releases.last().unwrap().body,
        w.body.as_ref().unwrap(),
    );
    let kinds: Vec<_> = changes.iter().map(|c| c.kind).collect();
    use kentos_commands::FormChangeKind as K;
    assert!(
        kinds.contains(&K::Added) && kinds.contains(&K::Removed) && kinds.contains(&K::Changed),
        "{changes:?}"
    );
}

#[test]
fn a_tab_is_shown_in_the_kinds_chosen() {
    let mut app = crate::files_testing::app_with_drawing();
    kind_tabs(&mut app);
    let w = app.forms_window.as_ref().unwrap();
    let body = w.body.as_ref().unwrap();
    let rule = match &body.items[1] {
        kentos_commands::FormItem::Tab { visible_when, .. } => visible_when.clone(),
        _ => None,
    };
    assert_eq!(rule.as_deref(), Some("[Tür] içinde ('sanayi', 'depo')"));
    // Taking both away leaves no rule: the tab is shown for every kind.
    designer(&mut app, designer::Event::ShowInKind("sanayi".into()));
    designer(&mut app, designer::Event::ShowInKind("depo".into()));
    let body = app.forms_window.as_ref().unwrap().body.as_ref().unwrap();
    assert!(matches!(
        &body.items[1],
        kentos_commands::FormItem::Tab {
            visible_when: None,
            ..
        }
    ));
}

#[test]
fn a_related_table_lists_the_units() {
    let mut app = crate::files_testing::app_with_drawing();
    units_table(&mut app);
    let e = app.form_entry.as_ref().expect("the building's form");
    assert_eq!(e.related_of("bolumler").len(), 4);
}

#[test]
fn a_value_group_narrows_the_choices_and_refuses_a_combination() {
    let mut app = crate::files_testing::app_with_drawing();
    combination_refused(&mut app);
    let e = app.form_entry.as_ref().expect("the editor stays open");
    assert!(
        e.issues
            .iter()
            .any(|i| i.code == kentos_forms::codes::COMBINATION),
        "{:?}",
        e.issues
    );
    let roofs = e
        .state("cati")
        .and_then(|s| s.choices.clone())
        .unwrap_or_default();
    assert_eq!(roofs, ["duz", "kirma"], "steel's roof, and the one held");
    let doc = &app.document.as_ref().unwrap().model;
    let held = doc.by_layer_with_uids("bina").next().unwrap().1;
    assert_eq!(
        held.base().attrs.get("Malzeme").map(String::as_str),
        Some("ahsap"),
        "nothing written"
    );
}

#[test]
fn the_contingents_window_writes_through_its_command_and_closes() {
    use crate::forms::contingents::Event;
    let mut app = crate::files_testing::app_with_drawing();
    contingents_window(&mut app);
    let _ = app.update(Message::Contingents(Event::AddRow));
    let _ = app.update(Message::Contingents(Event::Cell(
        5,
        0,
        kentos_commands::Cell::Code {
            code: "ahsap".into(),
        },
    )));
    let _ = app.update(Message::Contingents(Event::Cell(
        5,
        1,
        kentos_commands::Cell::Code { code: "duz".into() },
    )));
    let _ = app.update(Message::Contingents(Event::Save));
    assert!(app.contingents.is_none(), "Kaydet closes the window");
    let doc = &app.document.as_ref().unwrap().model;
    assert_eq!(
        doc.layers().get("bina").unwrap().contingents[0].rows.len(),
        6
    );
}

#[test]
fn a_changed_kind_keeps_its_values_until_they_are_emptied() {
    let mut app = crate::files_testing::app_with_drawing();
    subtype_changed(&mut app);
    let e = app.form_entry.as_ref().expect("the editor");
    assert_eq!(e.misfits(), ["Malzeme"]);
    assert_eq!(
        e.value_of("Malzeme").as_deref(),
        Some("ahsap"),
        "kept, not emptied"
    );
    let choices = e
        .state("malzeme")
        .and_then(|s| s.choices.clone())
        .unwrap_or_default();
    // Industrial offers concrete and steel, the value group steel only with a flat roof (the
    // house's is hipped); the held timber stays offered.
    assert_eq!(choices, ["ahsap", "beton"], "the held value stays offered");
    let _ = app.update(Message::FormEntry(entry::Event::ClearMisfits));
    let e = app.form_entry.as_ref().unwrap();
    assert!(e.misfits().is_empty());
    assert_eq!(e.value_of("Malzeme"), None);
}

#[test]
fn recomputing_writes_the_calculation_on_every_record_as_one_step() {
    let mut app = crate::files_testing::app_with_drawing();
    recompute(&mut app);
    let doc = &mut app.document.as_mut().unwrap().model;
    let notes = |doc: &kentos_domain::Document| -> Vec<String> {
        doc.by_layer_with_uids("bina")
            .map(|(_, e)| e.base().attrs.get("Not").cloned().unwrap_or_default())
            .collect()
    };
    assert!(
        notes(doc).iter().all(|n| n.contains(" katlı, ")),
        "{:?}",
        notes(doc)
    );
    assert_eq!(
        doc.undo().as_deref(),
        Some("Hesapları yenile: Hasar tespiti")
    );
    assert!(
        !notes(doc).iter().any(|n| n.contains(" katlı, ")),
        "one step back"
    );
}

#[test]
fn a_scenario_is_kept_and_fails_when_the_form_changes() {
    let mut app = crate::files_testing::app_with_drawing();
    scenarios(&mut app);
    let w = app.forms_window.as_ref().unwrap();
    let body = w.body.as_ref().unwrap();
    assert_eq!(body.tests.len(), 2);
    assert_eq!(
        body.tests[0].values.get("Kat").map(String::as_str),
        Some("8")
    );
    let form = app.chosen_form().unwrap();
    let prepared = kentos_native_application::forms_test::prepared_for(
        &app.document.as_ref().unwrap().model,
        form,
        body,
    )
    .unwrap();
    let failed = kentos_forms::scenario::run(&prepared, &body.tests[0]);
    assert_eq!(
        failed,
        ["Sorunlar: beklenen hiçbiri, bulunan form_check_failed@kat."],
        "eight floors break the new check"
    );
    assert!(
        kentos_forms::scenario::run(&prepared, &body.tests[1]).is_empty(),
        "no floors: no check"
    );
}

#[test]
fn the_designer_lists_what_reads_a_field() {
    let mut app = crate::files_testing::app_with_drawing();
    designer_links(&mut app);
    let doc = app.document.as_ref().unwrap();
    let field = doc.model.layers().get("bina").unwrap().fields[2].clone();
    assert_eq!(field.name, "Kat");
    let read: Vec<_> = kentos_forms::field_refs::uses(
        doc.model.settings(),
        doc.model.layers().nodes(),
        "bina",
        &field,
    )
    .into_iter()
    .filter(|u| u.how != kentos_commands::FieldUseHow::Key && !u.kept)
    .map(|u| u.place)
    .collect();
    assert_eq!(read.len(), 5, "{read:#?}");
    assert!(read[0].contains("Görünme koşulu"));
}

#[test]
fn the_form_view_writes_a_record_and_stays_on_it() {
    use crate::features::{Event, Pane};
    let mut app = crate::files_testing::app_with_drawing();
    form_view(&mut app);
    let docked = |app: &App| app.form_entry.as_ref().is_some_and(|e| e.docked);
    assert!(docked(&app), "the form is in the table, not a window");
    assert!(app.dialog.is_none());
    let slot = app.selection.ids()[0];
    // Another record while the value is not saved: refused, the record stays.
    let _ = app.update(Message::Features(Event::FormRow(0)));
    assert_eq!(app.selection.ids()[0], slot);
    let _ = app.update(Message::FormEntry(entry::Event::Save));
    assert!(docked(&app), "Kaydet keeps the form on its record");
    let kat = app
        .document
        .as_ref()
        .unwrap()
        .model
        .get(slot)
        .unwrap()
        .base()
        .attrs
        .get("Kat")
        .cloned();
    assert_eq!(kat.as_deref(), Some("8"));
    let _ = app.update(Message::Features(Event::FormRow(0)));
    assert_ne!(
        app.selection.ids()[0],
        slot,
        "a saved record lets another one be chosen"
    );
    let _ = app.update(Message::Features(Event::Pane(Pane::Table)));
    assert!(app.form_entry.is_none());
}

#[test]
fn a_building_is_duplicated_with_its_floors_in_one_step() {
    let mut app = crate::files_testing::app_with_drawing();
    floors(&mut app);
    let before = app.document.as_ref().unwrap().model.by_layer("kat").count();
    let _ = app.update(Message::Features(crate::features::Event::Duplicate));
    let doc = app.document.as_ref().unwrap();
    assert_eq!(
        doc.model.by_layer("kat").count(),
        before + 2,
        "its two floors"
    );
    assert_eq!(doc.model.by_layer("bina").count(), 3);
    let copy = app.selection.ids()[0];
    let copied = doc.model.get(copy).unwrap();
    assert!(
        !copied.base().attrs.contains_key("Kod"),
        "the unique code is left empty"
    );
    let copy_uid = doc.model.uid(copy).unwrap().to_string();
    let held = doc
        .model
        .by_layer("kat")
        .filter(|e| e.base().attrs.get("Bina") == Some(&copy_uid))
        .count();
    assert_eq!(held, 2, "the copied floors name the copy");
    assert_eq!(app.selection.ids().len(), 3);
    let undone = app.document.as_mut().unwrap().model.undo();
    assert_eq!(undone.as_deref(), Some("Kaydı çoğalt"));
}

#[test]
fn a_renamed_field_is_followed_by_the_rules_that_name_it() {
    use crate::layer_fields::Event;
    let mut app = crate::files_testing::app_with_drawing();
    field_usage(&mut app);
    let uses = app.saved_field_uses("bina", "Kat");
    assert_eq!(uses.len(), 10, "{uses:#?}");
    assert!(
        uses.iter().any(|u| u.kept),
        "the published release is listed"
    );
    let _ = app.update(Message::LayerFields(Event::Save));
    assert!(app.layer_fields.is_none(), "Kaydet closes the window");
    let doc = app.document.as_ref().unwrap();
    let bina = doc.model.layers().get("bina").unwrap();
    let kat = bina.fields.iter().find(|f| f.name == "Kat sayısı").unwrap();
    assert_eq!(
        kat.id.as_deref(),
        Some("Kat"),
        "the form names it by its key: it keeps it"
    );
    assert_eq!(bina.table_styles[2].field.as_deref(), Some("Kat sayısı"));
    assert_eq!(bina.table_styles[2].condition, "[Kat sayısı] >= 6");
    let form = &doc.model.settings().forms[0];
    let durum = serde_json::to_value(&form.draft.items[0]).unwrap();
    assert_eq!(durum["visibleWhen"], "[Kat sayısı] > 0");
    let kept = serde_json::to_value(&form.releases[0].body.items[0]).unwrap();
    assert_eq!(
        kept["visibleWhen"], "Kat > 0",
        "a published release is kept"
    );
    assert_eq!(
        doc.model
            .by_layer("bina")
            .next()
            .unwrap()
            .base()
            .attrs
            .get("Kat sayısı")
            .map(String::as_str),
        Some("3")
    );
}

pub(super) fn attrs(app: &App, slot: u32) -> std::collections::BTreeMap<String, String> {
    app.document
        .as_ref()
        .and_then(|d| d.model.get(Slot(slot)))
        .map(|e| e.base().attrs.clone())
        .unwrap_or_default()
}

/// docs/adr/0401 §9: Kaydet with an issue keeps the window on it and writes nothing; with the note
/// written it writes the record in one undo step and closes the window.
#[test]
fn a_record_is_written_through_the_form_only_when_it_keeps_the_rules() {
    let mut app = crate::files_testing::app_with_drawing();
    editing(&mut app);
    assert!(app.form_entry.is_some());
    let before = attrs(&app, 1);
    entry(&mut app, entry::Event::Save);
    assert!(
        app.form_entry.is_some(),
        "the note is required: the window stays"
    );
    assert_eq!(attrs(&app, 1), before);
    entry(
        &mut app,
        entry::Event::Value("Açıklama".into(), "Çatı arası ayrıca bakılmalı.".into()),
    );
    entry(
        &mut app,
        entry::Event::Value("kat-sayisi".into(), " 04 ".into()),
    );
    entry(&mut app, entry::Event::Save);
    assert!(app.form_entry.is_none(), "written: the window closes");
    let after = attrs(&app, 1);
    assert_eq!(after.get("Kat").map(String::as_str), Some("4"));
    assert_eq!(
        after.get("Açıklama").map(String::as_str),
        Some("Çatı arası ayrıca bakılmalı.")
    );
    // One undo step brings the record back.
    run(&mut app, "edit.undo");
    assert_eq!(attrs(&app, 1), before);
}

/// docs/adr/0402 §7: a new record through the calculating form takes its defaults (the moment, the
/// person, the code) and calculations ($alan of 18 × 14) in one step; a short description only
/// warns; the storeys start the next new building; the calculated height stays locked.
#[test]
fn a_new_record_is_completed_and_its_storeys_are_kept_for_the_next() {
    let mut app = crate::files_testing::app_with_drawing();
    calc_buildings(&mut app);
    app.selection.set([Slot(2)]);
    run(&mut app, "data.formEdit");
    entry(
        &mut app,
        entry::Event::Value("Durum".into(), "inceleme".into()),
    );
    entry(
        &mut app,
        entry::Event::Value("kat-sayisi".into(), "4".into()),
    );
    entry(
        &mut app,
        entry::Event::Value("Açıklama".into(), "Çatı akıyor".into()),
    );
    // The height is calculated: a value typed for it is not written.
    entry(
        &mut app,
        entry::Event::Value("Yükseklik".into(), "99".into()),
    );
    entry(&mut app, entry::Event::Save);
    assert!(
        app.form_entry.is_some(),
        "a calculated field typed: the window stays"
    );
    entry(
        &mut app,
        entry::Event::Value("Yükseklik".into(), "12".into()),
    );
    entry(&mut app, entry::Event::Save);
    assert!(app.form_entry.is_none(), "a warning does not stop Kaydet");
    let after = attrs(&app, 2);
    let want: std::collections::BTreeMap<String, String> = [
        ("Açıklama", "Çatı akıyor"),
        ("Durum", "inceleme"),
        ("Kat", "4"),
        ("Kayıt", "Ayşe Yılmaz"),
        ("Kod", "B-4"),
        ("Puan", "3"),
        ("Riskli", "false"),
        ("Taban", "252"),
        ("Tarih", "2026-10-16"),
        ("Yükseklik", "12"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v.to_owned()))
    .collect();
    assert_eq!(after, want);

    app.selection.set([Slot(3)]);
    run(&mut app, "data.formEdit");
    let e = app.form_entry.as_ref().expect("the window");
    assert_eq!(e.value_of("Kat").as_deref(), Some("4"), "reuse last");
    assert_eq!(e.value_of("Kod").as_deref(), Some("B-4"));
    // The other building's code: not unique.
    entry(&mut app, entry::Event::Value("Durum".into(), "iyi".into()));
    entry(&mut app, entry::Event::Save);
    assert!(app.form_entry.is_some(), "B-4 is the second building's");
    entry(&mut app, entry::Event::Value("Kod".into(), "B-4a".into()));
    entry(&mut app, entry::Event::Save);
    assert!(app.form_entry.is_none());
    assert_eq!(attrs(&app, 3).get("Kod").map(String::as_str), Some("B-4a"));
}

/// docs/adr/0402 §7: an industrial building is offered only concrete and steel; a closed group
/// opens by its title.
#[test]
fn the_choices_follow_the_filter_and_a_closed_group_opens() {
    let mut app = crate::files_testing::app_with_drawing();
    calc_buildings(&mut app);
    app.selection.set([Slot(2)]);
    run(&mut app, "data.formEdit");
    assert_eq!(
        app.form_entry.as_ref().and_then(|e| e.offered("malzeme")),
        Some(vec![
            "ahsap".to_owned(),
            "beton".to_owned(),
            "celik".to_owned(),
            "tas".to_owned()
        ])
    );
    entry(&mut app, entry::Event::Value("Tür".into(), "sanayi".into()));
    assert_eq!(
        app.form_entry.as_ref().and_then(|e| e.offered("malzeme")),
        Some(vec!["beton".to_owned(), "celik".to_owned()])
    );
    assert!(
        app.form_entry
            .as_ref()
            .is_some_and(|e| e.is_closed("hesap"))
    );
    entry(&mut app, entry::Event::Group("hesap".into()));
    assert!(
        app.form_entry
            .as_ref()
            .is_some_and(|e| !e.is_closed("hesap"))
    );
}

/// docs/adr/0403 §1–§2: typing into one label is one step; Geri al and Yinele walk the copy; an
/// item dragged into another tab moves, a drop the shape does not take is refused.
#[test]
fn the_designer_takes_steps_back_and_items_are_dragged() {
    use kentos_ui::widget::tree_view::Place;
    let mut app = crate::files_testing::app_with_drawing();
    buildings(&mut app, true);
    run(&mut app, "data.forms");
    designer(&mut app, designer::Event::SelectItem("kat".into()));
    for typed in ["K", "Ka", "Kat adedi"] {
        designer(
            &mut app,
            designer::Event::Text(designer::Text::Label, typed.into()),
        );
    }
    let label = |app: &App| match app
        .forms_window
        .as_ref()
        .and_then(|w| w.body.as_ref())
        .and_then(|b| designer::find(&b.items, "kat"))
    {
        Some(kentos_commands::FormItem::Field(f)) => f.label.clone(),
        _ => None,
    };
    assert_eq!(label(&app).as_deref(), Some("Kat adedi"));
    assert_eq!(app.forms_window.as_ref().map(|w| w.undo.len()), Some(1));
    designer(&mut app, designer::Event::Undo);
    assert_eq!(label(&app).as_deref(), Some("Kat sayısı"));
    assert!(
        !app.forms_window.as_ref().is_some_and(|w| w.dirty),
        "back to the saved draft"
    );
    designer(&mut app, designer::Event::Redo);
    assert_eq!(label(&app).as_deref(), Some("Kat adedi"));

    // genel[durum, kat, aciklama], ayrinti[denetim[tarih, onay], bilgi]: the note into Ayrıntı.
    let order = |app: &App| {
        let b = app
            .forms_window
            .as_ref()
            .and_then(|w| w.body.clone())
            .expect("a copy");
        (0..9)
            .filter_map(|i| designer::id_at(&b, i))
            .collect::<Vec<_>>()
    };
    let at = |app: &App, id: &str| order(app).iter().position(|x| x == id).expect(id);
    let (from, to) = (at(&app, "aciklama"), at(&app, "ayrinti"));
    designer(&mut app, designer::Event::Drop(from, to, Place::Into));
    assert_eq!(
        order(&app),
        [
            "genel", "durum", "kat", "ayrinti", "denetim", "tarih", "onay", "bilgi", "aciklama"
        ]
    );
    let (from, to) = (at(&app, "ayrinti"), at(&app, "denetim"));
    designer(&mut app, designer::Event::Drop(from, to, Place::Into));
    assert_eq!(at(&app, "ayrinti"), 3, "a tab does not go into a group");
    designer(&mut app, designer::Event::Undo);
    assert_eq!(at(&app, "aciklama"), 3, "the move is one step");
}

/// docs/adr/0403 §3: the preview shows the copy, typed values and all, and follows its changes;
/// nothing is written.
#[test]
fn the_preview_follows_the_copy_and_writes_nothing() {
    let mut app = crate::files_testing::app_with_drawing();
    calc_buildings(&mut app);
    run(&mut app, "data.forms");
    designer(&mut app, designer::Event::Preview(true));
    let before = attrs(&app, 2);
    let _ = app.update(Message::FormPreview(entry::Event::Value(
        "kat-sayisi".into(),
        "7".into(),
    )));
    let p = app.form_preview.as_ref().expect("the preview");
    assert_eq!(p.value_of("Yükseklik").as_deref(), Some("21"));
    assert_eq!(p.value_of("Kod").as_deref(), Some("B-7"));
    designer(&mut app, designer::Event::SelectItem("yukseklik".into()));
    designer(
        &mut app,
        designer::Event::RuleText(
            designer::Rule::Calculate("yukseklik".into()),
            "Kat * 3.5".into(),
        ),
    );
    let p = app.form_preview.as_ref().expect("the preview");
    assert_eq!(
        p.value_of("Yükseklik").as_deref(),
        Some("24.5"),
        "the typed storeys stay"
    );
    let _ = app.update(Message::FormPreview(entry::Event::Save));
    assert_eq!(attrs(&app, 2), before);
    // A rule that does not read: the preview says why instead.
    designer(
        &mut app,
        designer::Event::RuleText(
            designer::Rule::Calculate("yukseklik".into()),
            "Kat *".into(),
        ),
    );
    assert!(app.form_preview.is_none());
    assert!(
        app.forms_window
            .as_ref()
            .is_some_and(|w| w.preview_error.is_some())
    );
}

/// docs/adr/0403 §4: a rectangle drawn on the form's layer opens its form as a new record, its
/// calculations written by the drawing's step; Vazgeç leaves the drawn object; with the
/// preference off nothing opens.
#[test]
fn a_drawn_object_opens_its_form() {
    let mut app = crate::files_testing::app_with_drawing();
    calc_buildings(&mut app);
    let count = |app: &App| app.document.as_ref().map_or(0, |d| d.model.len());
    let before = count(&app);
    run(&mut app, "tool.rectangle");
    crate::tools_scenes::typed(&mut app, &format!("{},{}", E + 44.0, N));
    crate::tools_scenes::typed(&mut app, &format!("{},{}", E + 54.0, N + 8.0));
    // The command line's Enter is an update of its own: its tasks run after it.
    crate::tools_scenes::forget(&mut app);
    assert_eq!(count(&app), before + 1);
    let e = app.form_entry.as_ref().expect("the form opens");
    assert_eq!(e.value_of("Taban").as_deref(), Some("80"));
    entry(&mut app, entry::Event::Cancel);
    assert_eq!(count(&app), before + 1, "Vazgeç keeps the object");

    let _ = app
        .settings
        .choose(&[(crate::forms::ON_CREATE, serde_json::Value::Bool(false))]);
    run(&mut app, "tool.rectangle");
    crate::tools_scenes::typed(&mut app, &format!("{},{}", E + 44.0, N + 20.0));
    crate::tools_scenes::typed(&mut app, &format!("{},{}", E + 54.0, N + 28.0));
    crate::tools_scenes::forget(&mut app);
    assert_eq!(count(&app), before + 2);
    assert!(app.form_entry.is_none());
}

/// docs/adr/0403 §6: three buildings through one window: the values they differ in are mixed, the
/// values typed go to all of them in one step, each completed by itself; a unique code typed for
/// all of them is refused.
#[test]
fn many_records_are_written_through_the_form_at_once() {
    let mut app = crate::files_testing::app_with_drawing();
    calc_buildings(&mut app);
    app.selection.set([Slot(1), Slot(2), Slot(3)]);
    run(&mut app, "data.formEdit");
    let e = app.form_entry.as_ref().expect("the window");
    assert!(e.is_mixed("Durum"));
    entry(&mut app, entry::Event::Value("Kod".into(), "X-1".into()));
    entry(&mut app, entry::Event::Value("Durum".into(), "iyi".into()));
    entry(&mut app, entry::Event::Save);
    assert!(
        app.form_entry.is_some(),
        "one code for three buildings: refused"
    );
    assert_eq!(attrs(&app, 2), std::collections::BTreeMap::new());
    entry(&mut app, entry::Event::Value("Kod".into(), "".into()));
    entry(
        &mut app,
        entry::Event::Value("kat-sayisi".into(), "5".into()),
    );
    entry(&mut app, entry::Event::Save);
    assert!(app.form_entry.is_none());
    for slot in [1, 2, 3] {
        let a = attrs(&app, slot);
        assert_eq!(a.get("Kat").map(String::as_str), Some("5"), "{slot}");
        assert_eq!(a.get("Yükseklik").map(String::as_str), Some("15"), "{slot}");
        assert_eq!(a.get("Durum").map(String::as_str), Some("iyi"), "{slot}");
    }
    assert_eq!(attrs(&app, 1).get("Kod").map(String::as_str), Some("B-3"));
    run(&mut app, "edit.undo");
    assert_eq!(
        attrs(&app, 2),
        std::collections::BTreeMap::new(),
        "one undo step"
    );
}

/// docs/adr/0404: the address chains province, district and open neighbourhood; labels are written as
/// their codes, the parcel as its persistent id; a district outside the chain is refused; the meters
/// referring to the building are listed and open their own form.
#[test]
fn lists_from_other_layers_chain_and_related_records_are_listed() {
    let mut app = crate::files_testing::app_with_drawing();
    address_buildings(&mut app);
    app.selection.set([Slot(1)]);
    run(&mut app, "data.formEdit");
    let e = app.form_entry.as_ref().expect("the window");
    assert_eq!(e.offered("ilce"), Some(vec![]), "no province: no district");
    assert_eq!(e.related_of("sayaclar").len(), 2);
    entry(&mut app, entry::Event::Value("İl".into(), "Ankara".into()));
    entry(
        &mut app,
        entry::Event::Value("İlçe".into(), "Çankaya".into()),
    );
    let e = app.form_entry.as_ref().expect("the window");
    assert_eq!(
        e.offered("mahalle"),
        Some(vec!["M2".to_owned(), "M10".to_owned()])
    );
    entry(
        &mut app,
        entry::Event::Value("Mahalle".into(), "Kızılay".into()),
    );
    entry(
        &mut app,
        entry::Event::Value("Parsel".into(), "101/12".into()),
    );
    entry(&mut app, entry::Event::Save);
    assert!(app.form_entry.is_none());
    let a = attrs(&app, 1);
    assert_eq!(a.get("İl").map(String::as_str), Some("06"));
    assert_eq!(a.get("İlçe").map(String::as_str), Some("0601"));
    assert_eq!(a.get("Mahalle").map(String::as_str), Some("M10"));
    assert_eq!(
        a.get("Parsel").map(|p| p.len()),
        Some(36),
        "a persistent id"
    );

    run(&mut app, "data.formEdit");
    entry(&mut app, entry::Event::Value("İl".into(), "34".into()));
    entry(&mut app, entry::Event::Save);
    assert!(app.form_entry.is_some(), "Çankaya is not İstanbul's");

    // A meter opens its own layer's form only when it has one: said, the window stays.
    entry(&mut app, entry::Event::Value("İl".into(), "06".into()));
    let meter = app
        .form_entry
        .as_ref()
        .expect("the window")
        .related_of("sayaclar")[0]
        .clone();
    entry(&mut app, entry::Event::Cancel);
    run(&mut app, "data.formEdit");
    entry(
        &mut app,
        entry::Event::OpenRelated("sayaclar".into(), meter),
    );
    assert!(app.form_entry.is_some(), "the meters' layer has no form");
}

/// docs/adr/0405: a relation made in İlişkiler and saved; a building deleted takes its meters
/// (cascade); a meter unlinked from and linked back to its building through the form's list.
#[test]
fn a_relation_keeps_its_children_and_the_form_links_them() {
    let mut app = crate::files_testing::app_with_drawing();
    relations_window(&mut app);
    let _ = app.update(Message::Relations(relations::Event::Save));
    assert!(app.relations_window.is_none(), "Kaydet closes the window");
    let kept = app
        .document
        .as_ref()
        .map(|d| d.model.settings().relations.clone())
        .unwrap_or_default();
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].on_delete, kentos_commands::RelationDelete::Cascade);

    app.selection.set([Slot(1)]);
    run(&mut app, "data.formEdit");
    let meters = app
        .form_entry
        .as_ref()
        .expect("the window")
        .related_of("sayaclar");
    assert_eq!(meters.len(), 2);
    entry(
        &mut app,
        entry::Event::Unlink("sayaclar".into(), meters[0].clone()),
    );
    assert_eq!(
        app.form_entry
            .as_ref()
            .expect("the window")
            .related_of("sayaclar")
            .len(),
        1
    );
    entry(
        &mut app,
        entry::Event::Link("sayaclar".into(), meters[0].clone()),
    );
    assert_eq!(
        app.form_entry
            .as_ref()
            .expect("the window")
            .related_of("sayaclar")
            .len(),
        2
    );
    entry(&mut app, entry::Event::Cancel);

    let count = |app: &App| app.document.as_ref().map_or(0, |d| d.model.len());
    let before = count(&app);
    let building = app
        .document
        .as_ref()
        .and_then(|d| d.model.uid(Slot(1)))
        .map(|u| u.to_string())
        .expect("the building");
    if let Some(doc) = app.document.as_mut() {
        let _ = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            kentos_commands::CAD_ENTITIES_DELETE,
            kentos_commands::CallOp::Execute,
            json!({ "uids": [building] }),
        );
    }
    assert_eq!(count(&app), before - 3, "the building and its two meters");
}

/// docs/adr/0406: Kayıtları denetle lists the buildings breaking the form's rules and selects them;
/// a slider writes the storeys within the field's range.
#[test]
fn records_are_audited_and_a_slider_writes_within_the_range() {
    let mut app = crate::files_testing::app_with_drawing();
    records_audit(&mut app);
    assert!(app.form_audit.is_some());
    let _ = app.update(Message::FormAudit(audit::Event::SelectAll));
    // The first two share code B-3; the other two have no status.
    assert_eq!(app.selection.len(), 4);
    let _ = app.update(Message::FormAudit(audit::Event::Close));
    assert!(app.form_audit.is_none());

    let mut app = crate::files_testing::app_with_drawing();
    slider_editing(&mut app);
    entry(
        &mut app,
        entry::Event::Value("kat-sayisi".into(), "7".into()),
    );
    entry(&mut app, entry::Event::Save);
    assert_eq!(attrs(&app, 1).get("Kat").map(String::as_str), Some("7"));
    assert_eq!(
        attrs(&app, 1).get("Yükseklik").map(String::as_str),
        Some("21")
    );
}

/// docs/adr/0407: a tree without a photo cannot be saved; a photo added in the form lets it; a
/// colour that is not one is refused; the name field offers the layer's names.
#[test]
fn photos_are_counted_and_colours_checked() {
    let mut app = crate::files_testing::app_with_drawing();
    trees(&mut app);
    app.selection.set([Slot(2)]);
    run(&mut app, "data.formEdit");
    entry(&mut app, entry::Event::Save);
    assert!(
        app.form_entry.is_some(),
        "a tree without a photo is not saved"
    );
    assert!(!attrs(&app, 2).contains_key("Renk"));
    let photo = fixture("fixtures/exif/v1/konum.jpg");
    let _ = app.update(Message::Attachment(crate::attachments::Event::Picked(
        Slot(2),
        vec![photo.into()],
    )));
    entry(&mut app, entry::Event::Value("Renk".into(), "yeşil".into()));
    entry(&mut app, entry::Event::Save);
    assert!(app.form_entry.is_some(), "yeşil is not a colour");
    entry(
        &mut app,
        entry::Event::Value("Renk".into(), "#2e7d32cc".into()),
    );
    entry(&mut app, entry::Event::Save);
    assert!(app.form_entry.is_none(), "saved with a photo and a colour");
    assert_eq!(
        attrs(&app, 2).get("Renk").map(String::as_str),
        Some("#2e7d32cc")
    );
}

/// docs/adr/0408: the care field takes several values in the list's order; the form steps to the
/// next tree, not while a change is unsaved.
#[test]
fn several_values_and_stepping_through_the_records() {
    let mut app = crate::files_testing::app_with_drawing();
    trees(&mut app);
    let uid = |app: &App, slot: u32| {
        app.document
            .as_ref()
            .and_then(|d| d.model.uid(Slot(slot)))
            .map(|u| u.to_string())
    };
    app.selection.set([Slot(1)]);
    run(&mut app, "data.formEdit");
    entry(
        &mut app,
        entry::Event::Value("Bakım".into(), "sulama; İlaçlama; budama".into()),
    );
    entry(&mut app, entry::Event::Step(1));
    assert_eq!(
        app.form_entry.as_ref().map(|e| e.uid_text()),
        uid(&app, 1),
        "an unsaved change keeps the record"
    );
    entry(&mut app, entry::Event::Save);
    assert_eq!(
        attrs(&app, 1).get("Bakım").map(String::as_str),
        Some("budama; ilaclama; sulama")
    );
    app.selection.set([Slot(1)]);
    run(&mut app, "data.formEdit");
    entry(&mut app, entry::Event::Step(1));
    assert_eq!(app.form_entry.as_ref().map(|e| e.uid_text()), uid(&app, 2));
}

/// docs/adr/0409: an owner holds several buildings; a building lists its owners, links one more
/// and unlinks it without touching the owner's other buildings; Yeni makes an owner linked to it.
#[test]
fn many_to_many_owners_are_linked_and_made() {
    let mut app = crate::files_testing::app_with_drawing();
    owners(&mut app);
    let uid = |app: &App, slot: u32| {
        app.document
            .as_ref()
            .and_then(|d| d.model.uid(Slot(slot)))
            .map(|u| u.to_string())
            .unwrap_or_default()
    };
    let (b1, b2, b3) = (uid(&app, 1), uid(&app, 2), uid(&app, 3));
    let (o1, o2) = (uid(&app, 4), uid(&app, 5));
    app.selection.set([Slot(2)]);
    run(&mut app, "data.formEdit");
    let listed = |app: &App| {
        app.form_entry
            .as_ref()
            .map(|e| e.related_of("malikler"))
            .unwrap_or_default()
    };
    assert_eq!(listed(&app), [o1.clone(), o2.clone()]);
    // Building 3 takes the second owner; its other building stays.
    app.selection.set([Slot(3)]);
    run(&mut app, "data.formEdit");
    // Through the project relation a link waits for Kaydet (docs/adr/0440 §2).
    entry(&mut app, entry::Event::Link("malikler".into(), o2.clone()));
    entry(&mut app, entry::Event::Save);
    assert_eq!(attrs(&app, 5).get("Binalar"), Some(&format!("{b2}; {b3}")));
    run(&mut app, "data.formEdit");
    entry(
        &mut app,
        entry::Event::Unlink("malikler".into(), o2.clone()),
    );
    entry(&mut app, entry::Event::Save);
    assert_eq!(attrs(&app, 5).get("Binalar"), Some(&b2));
    // Building 1's first owner is unlinked from it alone.
    app.selection.set([Slot(1)]);
    run(&mut app, "data.formEdit");
    entry(
        &mut app,
        entry::Event::Unlink("malikler".into(), o1.clone()),
    );
    entry(&mut app, entry::Event::Save);
    assert_eq!(attrs(&app, 4).get("Binalar"), Some(&b2));
    run(&mut app, "data.formEdit");
    // Yeni: an owner linked to the building, a record as the owners are (docs/adr/0410 §4),
    // opened by the owner's form.
    entry(&mut app, entry::Event::NewRelated("malikler".into()));
    assert_eq!(attrs(&app, 6).get("Binalar"), Some(&b1));
    let kind = |app: &App, slot: u32| {
        app.document
            .as_ref()
            .and_then(|d| d.model.get(Slot(slot)))
            .map(|e| e.kind())
    };
    assert_eq!(kind(&app, 6), Some("record"));
    assert_eq!(
        app.form_entry.as_ref().map(|e| e.uid_text()),
        Some(uid(&app, 6))
    );
}

/// docs/adr/0412: a date and time and a time of day are written in their canonical texts through
/// the form, typed as people write them; a time out of the day is refused.
#[test]
fn a_date_and_time_and_a_time_are_written_through_the_form() {
    let mut app = crate::files_testing::app_with_drawing();
    timed(&mut app);
    assert!(app.form_entry.is_some());
    entry(
        &mut app,
        entry::Event::Value("Tespit".into(), "17.10.2026 9:05".into()),
    );
    entry(
        &mut app,
        entry::Event::Value("Vardiya".into(), "17:45".into()),
    );
    entry(&mut app, entry::Event::Save);
    let got = attrs(&app, 1);
    assert_eq!(
        got.get("Tespit").map(String::as_str),
        Some("2026-10-17T09:05:00")
    );
    assert_eq!(got.get("Vardiya").map(String::as_str), Some("17:45:00"));
    run(&mut app, "data.formEdit");
    entry(
        &mut app,
        entry::Event::Value("Vardiya".into(), "24:10".into()),
    );
    entry(&mut app, entry::Event::Save);
    assert!(
        app.form_entry.is_some(),
        "the refused time keeps the window"
    );
    assert_eq!(
        attrs(&app, 1).get("Vardiya").map(String::as_str),
        Some("17:45:00")
    );
}

/// docs/adr/0410 §4: the owners are records; the Tablo tab lists them, Kayıt ekle writes one through
/// `cad.records.create` in one undo step and opens it by the owners' form; nothing of them is drawn,
/// picked or zoomed to.
#[test]
fn a_record_is_added_in_the_table_and_opened_by_its_form() {
    let mut app = crate::files_testing::app_with_drawing();
    owner_table(&mut app);
    let (count, _, rows) = app.features_seen().expect("the tab is open");
    assert_eq!(count, "2 / 2", "{rows:?}");
    // The buildings by their codes, not their ids: the owners' form lists them from the building layer.
    assert_eq!(
        rows,
        [
            "Kayıt | Ayşe Yılmaz | B-1; B-2",
            "Kayıt | Mehmet Kaya | B-2"
        ]
    );
    let _ = app.update(Message::Features(crate::features::Event::AddRecord));
    let made = app
        .document
        .as_ref()
        .and_then(|d| d.model.get(Slot(6)))
        .map(|e| (e.kind(), e.base().layer_id.clone()));
    assert_eq!(made, Some(("record", "malik".to_owned())));
    assert_eq!(
        app.form_entry.as_ref().map(|e| e.uid_text()),
        app.document
            .as_ref()
            .and_then(|d| d.model.uid(Slot(6)))
            .map(|u| u.to_string())
    );
    // In no place: the store keeps it, no query by place finds it, the drawing's extent leaves it out.
    if let Some(doc) = app.document.as_ref() {
        app.spatial.sync(&doc.model);
    }
    let store = app.spatial.store();
    assert!(store.get(6.0).is_some());
    assert_eq!(store.extent(Some(&[4.0, 5.0, 6.0])), None);
    let all = kentos_geometry_core::geometry::Bounds {
        min_x: -1e12,
        min_y: -1e12,
        max_x: 1e12,
        max_y: 1e12,
    };
    assert!(!store.in_rect(&all, true).iter().any(|id| *id >= 4.0));
    let _ = app.update(Message::Run("edit.undo"));
    assert!(
        app.document
            .as_ref()
            .and_then(|d| d.model.get(Slot(6)))
            .is_none()
    );
}

/// docs/adr/0401 §9: a form made of the layer's fields, an item added, Kaydet writes the draft and
/// closes; Yayınla publishes it and the window stays.
#[test]
fn a_form_is_made_saved_and_published_in_the_window() {
    let mut app = crate::files_testing::app_with_drawing();
    buildings(&mut app, false);
    run(&mut app, "data.forms");
    designer(&mut app, designer::Event::NewForm);
    designer(&mut app, designer::Event::NewName("Hasar tespiti".into()));
    designer(&mut app, designer::Event::Create);
    let made = app
        .project_forms()
        .iter()
        .find(|f| f.name == "Hasar tespiti")
        .cloned()
        .expect("the new form");
    assert_eq!(made.draft.items.len(), 5);
    designer(&mut app, designer::Event::AddNote);
    designer(&mut app, designer::Event::Publish);
    assert!(app.forms_window.is_some(), "Yayınla keeps the window");
    let kept = app
        .project_forms()
        .iter()
        .find(|f| f.id == made.id)
        .cloned()
        .expect("the form");
    assert_eq!(kept.draft.items.len(), 6);
    assert_eq!(kept.releases.len(), 1);
    designer(&mut app, designer::Event::Name("Hasar tespiti 2026".into()));
    designer(&mut app, designer::Event::Save);
    assert!(app.forms_window.is_none(), "Kaydet closes the window");
    assert!(
        app.project_forms()
            .iter()
            .any(|f| f.name == "Hasar tespiti 2026")
    );
}
