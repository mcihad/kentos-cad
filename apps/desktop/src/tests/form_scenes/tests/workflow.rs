//! Form tests: publishing, reviewers' notes, releases, workflow steps and their actions, drafts,
//! sections, rules and policies, retired values, lists and packages, links, child forms, snapshots and searched lists.

use super::*;

#[test]
fn a_publish_that_breaks_records_asks_first_and_publishes_when_asked_again() {
    let mut app = crate::files_testing::app_with_drawing();
    breaking_publish(&mut app);
    let w = app.forms_window.as_ref().expect("the window");
    let b = w.breaking.as_ref().expect("asked about the broken records");
    assert_eq!(
        b.records, 2,
        "the sent and the approved building lack a reason"
    );
    assert_eq!(
        app.project_forms()[0].releases.len(),
        1,
        "not published yet"
    );
    designer(&mut app, designer::Event::PublishAnyway);
    assert_eq!(app.project_forms()[0].releases.len(), 2);
}

#[test]
fn a_reviewer_s_notes_go_with_the_step_and_show_under_the_fields_after() {
    let mut app = crate::files_testing::app_with_drawing();
    reviewer_notes_shown(&mut app);
    let attrs = &app
        .document
        .as_ref()
        .unwrap()
        .model
        .get(Slot(2))
        .unwrap()
        .base()
        .attrs;
    assert_eq!(attrs.get("Durum").map(String::as_str), Some("iade"));
    assert_eq!(
        attrs.get("Notlar").map(String::as_str),
        Some(r#"{"Kat":"Çatı katı sayılmamış.","Kod":"Tabeladaki kod B-02."}"#)
    );
    assert!(
        attrs
            .get("Geçmiş")
            .is_some_and(|l| l.contains("2 alan notu"))
    );
}

#[test]
fn a_new_release_opens_the_record_again_keeping_what_was_typed() {
    let mut app = crate::files_testing::app_with_drawing();
    new_release(&mut app);
    assert!(app.form_entry.is_some(), "the window stays on the conflict");
    entry(&mut app, entry::Event::Reload);
    entry(&mut app, entry::Event::Save);
    assert!(app.form_entry.is_none(), "written by the new release");
    let kat = app
        .document
        .as_ref()
        .unwrap()
        .model
        .get(Slot(1))
        .unwrap()
        .base()
        .attrs
        .get("Kat")
        .cloned();
    assert_eq!(kat.as_deref(), Some("4"));
}

#[test]
fn a_step_action_takes_its_step_and_a_show_action_selects_the_record() {
    let mut app = crate::files_testing::app_with_drawing();
    workflow_buildings(&mut app);
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    let form = settings.forms.first_mut().unwrap();
    let mut body = form.draft.clone();
    body.items.push(
        serde_json::from_value(json!(
        { "type": "action", "id": "gonder", "label": "Gönder", "kind": "step", "step": "Gönder" }))
        .unwrap(),
    );
    body.items.push(
        serde_json::from_value(json!(
        { "type": "action", "id": "goster", "label": "Göster", "kind": "show" }))
        .unwrap(),
    );
    form.draft = body.clone();
    form.releases = vec![kentos_commands::FormRelease {
        fields: None,
        version: 1,
        digest: kentos_commands::body_digest(&body),
        body,
        note: None,
    }];
    doc.model.set_settings(settings);
    app.selection.set([Slot(1)]);
    run(&mut app, "data.formEdit");
    entry(&mut app, entry::Event::RunAction("goster".into()));
    assert_eq!(app.selection.ids(), [Slot(1)]);
    entry(&mut app, entry::Event::RunAction("gonder".into()));
    let state = app
        .document
        .as_ref()
        .unwrap()
        .model
        .get(Slot(1))
        .unwrap()
        .base()
        .attrs
        .get("Durum")
        .cloned();
    assert_eq!(state.as_deref(), Some("gonderildi"));
}

#[test]
fn the_attachments_facts_are_read_by_the_command() {
    let mut app = crate::files_testing::app_with_drawing();
    attachment_facts(&mut app);
    assert!(app.form_entry.is_some());
}

#[test]
fn a_divider_and_an_action_show_and_the_action_opens_its_tool_on_the_record() {
    let mut app = crate::files_testing::app_with_drawing();
    meters_chart(&mut app);
    assert!(app.form_entry.is_some(), "the form");
    entry(&mut app, entry::Event::RunAction("hesapla".into()));
    assert_eq!(
        app.selection.ids(),
        [Slot(1)],
        "the record is the tool's input"
    );
    assert!(app.processing.dialog.is_some(), "the tool's window opened");
}

#[test]
fn comments_go_through_cad_forms_edit_and_are_settled() {
    let mut app = crate::files_testing::app_with_drawing();
    meters_review(&mut app);
    let form = &app.project_forms()[0];
    assert_eq!(form.comments.len(), 2);
    assert_eq!(form.comments[0].author.as_deref(), Some("Ayşe Yılmaz"));
    assert!(!form.comments[0].resolved && form.comments[1].resolved);
    // An item's comment names the item.
    designer(&mut app, designer::Event::SelectItem("hesapla".into()));
    designer(
        &mut app,
        designer::Event::CommentText("Araç doğru mu?".into()),
    );
    designer(&mut app, designer::Event::AddComment);
    assert_eq!(
        app.project_forms()[0].comments[2].item.as_deref(),
        Some("hesapla")
    );
}

#[test]
fn a_draft_lost_with_the_window_is_offered_back_and_saving_forgets_it() {
    let mut app = crate::files_testing::app_with_drawing();
    meters_kept(&mut app);
    let w = app.forms_window.as_ref().expect("the window");
    assert!(w.kept.is_some(), "the kept draft is offered");
    designer(&mut app, designer::Event::RestoreDraft);
    let w = app.forms_window.as_ref().expect("the window");
    assert!(w.dirty);
    assert!(
        w.body
            .as_ref()
            .unwrap()
            .walk()
            .iter()
            .filter(|(_, i)| matches!(i, FormItem::Divider { .. }))
            .count()
            == 2
    );
    designer(&mut app, designer::Event::Save);
    let key = app.form_project_key();
    assert_eq!(
        app.form_files.kept_draft(&key, "bina"),
        None,
        "saved: nothing kept"
    );
}

#[test]
fn a_section_goes_into_the_library_and_comes_back_into_another_form_with_its_fields_bound() {
    let mut app = crate::files_testing::app_with_drawing();
    meters_sections(&mut app);
    let sections = app.form_files.sections();
    assert_eq!(
        sections
            .iter()
            .map(|s| (s.name.as_str(), s.version))
            .collect::<Vec<_>>(),
        [("Sayaç grafiği", 1), ("Sayaç grafiği", 2), ("Bina adı", 1)]
    );
    // Bina adı into the meter form: its Ad field is not on the meter layer, the item goes in, said.
    designer(&mut app, designer::Event::InsertSection(2));
    let w = app.forms_window.as_ref().expect("the window");
    assert!(w.dirty);
    assert!(
        app.log
            .lines()
            .any(|l| l.text.contains("katmanda olmayan alanlar: Ad"))
    );
}

#[test]
fn a_tables_rules_are_written_through_the_window_in_one_step() {
    use crate::forms::table_styles::Event;
    let mut app = crate::files_testing::app_with_drawing();
    styled_buildings(&mut app);
    let rows = app.feature_rows(app.document.as_ref().unwrap());
    assert_eq!(rows.styles.len(), 4, "the layer's rules reach the table");
    // B-2 is ruined: its row is painted, its floor count bold.
    let ruined = rows.look(1, Some("Kat"));
    assert_eq!(ruined.background.as_deref(), Some("#FDE2E1"));
    assert!(ruined.bold);
    assert!(rows.look(0, Some("Kod")).is_plain());
    run(&mut app, "data.tableStyles");
    let step = |app: &mut App, e: Event| {
        let _ = app.update(Message::TableStyles(e));
    };
    step(&mut app, Event::New);
    step(&mut app, Event::Condition("Kat < 3".into()));
    step(&mut app, Event::Bold(true));
    step(&mut app, Event::Up);
    // A condition that cannot be read keeps the window open and writes nothing.
    step(&mut app, Event::Condition("Kat <".into()));
    step(&mut app, Event::Save);
    assert!(
        app.table_styles.is_some(),
        "a broken condition is not written"
    );
    step(&mut app, Event::Condition("Kat < 3".into()));
    step(&mut app, Event::Save);
    assert!(
        app.table_styles.is_none(),
        "Kaydet closes the window: {:?}",
        app.log.lines().map(|l| &l.text).collect::<Vec<_>>()
    );
    let rules = |app: &App| {
        app.document
            .as_ref()
            .unwrap()
            .model
            .layers()
            .get("bina")
            .unwrap()
            .table_styles
            .clone()
    };
    assert_eq!(rules(&app).len(), 5);
    assert_eq!(
        rules(&app)[3].condition,
        "Kat < 3",
        "Yukarı moved it above the last rule"
    );
    let undone = app.document.as_mut().unwrap().model.undo();
    assert_eq!(undone.as_deref(), Some("Koşullu biçim"));
    assert_eq!(rules(&app).len(), 4, "one step undoes it");
}

#[test]
fn the_fields_window_writes_the_split_and_merge_policies() {
    let mut app = crate::files_testing::app_with_drawing();
    field_policies(&mut app);
    let _ = app.update(Message::LayerFields(crate::layer_fields::Event::Save));
    let doc = app.document.as_ref().unwrap();
    let kat = &doc.model.layers().get("bina").unwrap().fields[2];
    assert_eq!(kat.name, "Kat");
    assert_eq!(kat.split, Some(kentos_commands::SplitPolicy::Ratio));
    assert_eq!(kat.merge, Some(kentos_commands::MergePolicy::Sum));
}

#[test]
fn a_retired_value_stays_where_it_is_and_is_not_written_anew() {
    let mut app = crate::files_testing::app_with_drawing();
    retired_state(&mut app);
    let field = app
        .document
        .as_ref()
        .unwrap()
        .model
        .layers()
        .get("bina")
        .unwrap()
        .fields[1]
        .clone();
    let codes = |v: &str| {
        field
            .offered_values(v)
            .into_iter()
            .map(|c| c.code)
            .collect::<Vec<_>>()
    };
    assert_eq!(codes("iyi"), ["iyi", "kotu"], "not offered anew");
    assert_eq!(
        codes("orta"),
        ["iyi", "orta", "kotu"],
        "offered to the record holding it"
    );
    // B-1 is in a good state: the form does not write “orta” anew.
    app.selection.set([Slot(1)]);
    run(&mut app, "data.formEdit");
    let _ = app.update(Message::FormEntry(entry::Event::Value(
        "Durum".into(),
        "orta".into(),
    )));
    let _ = app.update(Message::FormEntry(entry::Event::Save));
    assert!(app.form_entry.is_some(), "refused: the window stays");
    assert_eq!(attrs(&app, 1).get("Durum").map(String::as_str), Some("iyi"));
    let _ = app.update(Message::FormEntry(entry::Event::Cancel));
    let _ = app.update(Message::FormEntry(entry::Event::Discard));
    // B-3 holds it: its floor count is written and its state kept.
    app.selection.set([Slot(3)]);
    run(&mut app, "data.formEdit");
    let _ = app.update(Message::FormEntry(entry::Event::Value(
        "Kat".into(),
        "3".into(),
    )));
    let _ = app.update(Message::FormEntry(entry::Event::Save));
    assert!(app.form_entry.is_none());
    assert_eq!(
        attrs(&app, 3).get("Durum").map(String::as_str),
        Some("orta")
    );
    assert_eq!(attrs(&app, 3).get("Kat").map(String::as_str), Some("3"));
}

#[test]
fn an_overridable_calculation_follows_until_typed_over() {
    let mut app = crate::files_testing::app_with_drawing();
    overridable_height(&mut app);
    let value = |app: &App, name: &str| app.form_entry.as_ref().and_then(|e| e.after_value(name));
    let typed = |app: &mut App, key: &str, text: &str| {
        let _ = app.update(Message::FormEntry(entry::Event::Value(
            key.into(),
            text.into(),
        )));
    };
    assert_eq!(
        value(&app, "Yükseklik").as_deref(),
        Some("9"),
        "empty: calculated"
    );
    typed(&mut app, "Kat", "4");
    assert_eq!(
        value(&app, "Yükseklik").as_deref(),
        Some("12"),
        "it follows its inputs"
    );
    typed(&mut app, "Yükseklik", "10.5");
    typed(&mut app, "Kat", "5");
    assert_eq!(
        value(&app, "Yükseklik").as_deref(),
        Some("10.5"),
        "typed over: kept"
    );
    let _ = app.update(Message::FormEntry(entry::Event::Save));
    assert_eq!(
        attrs(&app, 1).get("Yükseklik").map(String::as_str),
        Some("10.5")
    );
    // Opened again it does not follow; ↺ gives it back to its calculation.
    run(&mut app, "data.formEdit");
    typed(&mut app, "Kat", "6");
    assert_eq!(value(&app, "Yükseklik").as_deref(), Some("10.5"));
    typed(&mut app, "Yükseklik", "");
    assert_eq!(value(&app, "Yükseklik").as_deref(), Some("18"));
    let _ = app.update(Message::FormEntry(entry::Event::Save));
    assert_eq!(
        attrs(&app, 1).get("Yükseklik").map(String::as_str),
        Some("18")
    );
}

#[test]
fn the_subtypes_window_writes_through_its_command_and_closes() {
    use crate::forms::subtypes::Event;
    let mut app = crate::files_testing::app_with_drawing();
    subtypes_window(&mut app);
    let w = app.subtypes.as_ref().expect("the window");
    assert_eq!(w.types.len(), 3);
    // Depo is no longer a subtype; the industrial one offers concrete only.
    let _ = app.update(Message::Subtypes(Event::Toggle("depo".into())));
    let _ = app.update(Message::Subtypes(Event::Offer(
        "Malzeme".into(),
        "celik".into(),
    )));
    let _ = app.update(Message::Subtypes(Event::Save));
    assert!(app.subtypes.is_none(), "Kaydet closes the window");
    let doc = &app.document.as_ref().unwrap().model;
    let s = doc.layers().get("bina").unwrap().subtypes.clone().unwrap();
    assert_eq!(s.types.len(), 2);
    let rule = &s.types[1].fields[0];
    assert_eq!(rule.values.as_deref(), Some(&["beton".to_owned()][..]));
    assert_eq!(
        rule.default, None,
        "steel is no longer offered, nor its default"
    );
}

#[test]
fn a_step_of_the_workflow_writes_the_state_and_closes_the_editor() {
    let mut app = crate::files_testing::app_with_drawing();
    workflow_steps(&mut app);
    assert!(app.form_entry.is_some());
    let _ = app.update(Message::FormEntry(entry::Event::Transition(
        "Onayla".into(),
    )));
    assert!(
        app.form_entry.is_none(),
        "the editor closes when the step went in"
    );
    let doc = &app.document.as_ref().unwrap().model;
    let held = doc.get(Slot(2)).unwrap();
    assert_eq!(
        held.base().attrs.get("Durum").map(String::as_str),
        Some("onaylandi")
    );
    // The approved building is locked: Kaydet is off, Düzelt asks for a reason.
    workflow_locked(&mut app);
    let _ = app.update(Message::FormEntry(entry::Event::Transition(
        "Düzelt".into(),
    )));
    assert!(
        app.form_entry.is_some(),
        "no reason: the step is refused, the editor stays"
    );
    // Its reason stays open on the locked record; written, Düzelt opens it again.
    let _ = app.update(Message::FormEntry(entry::Event::Value(
        "Gerekçe".into(),
        "Kat yeniden ölçülecek.".into(),
    )));
    let _ = app.update(Message::FormEntry(entry::Event::Transition(
        "Düzelt".into(),
    )));
    assert!(app.form_entry.is_none());
    let doc = &app.document.as_ref().unwrap().model;
    let attrs = &doc.get(Slot(3)).unwrap().base().attrs;
    assert_eq!(attrs.get("Durum").map(String::as_str), Some("iade"));
    assert_eq!(
        attrs.get("Gerekçe").map(String::as_str),
        Some("Kat yeniden ölçülecek.")
    );
    // The step wrote its line to the log (docs/adr/0429).
    let log = attrs.get("Geçmiş").cloned().unwrap_or_default();
    assert!(
        log.contains("Düzelt (Onaylandı → İade edildi): Kat yeniden ölçülecek. · #"),
        "{log}"
    );
}

#[test]
fn a_list_dropping_a_code_moves_its_records_in_one_undo_step() {
    let mut app = crate::files_testing::app_with_drawing();
    domains_window(&mut app);
    let _ = app.update(Message::Domains(crate::forms::domains::Event::Save));
    assert!(
        app.domains.is_none(),
        "the window closes when the lists went in"
    );
    let doc = &app.document.as_ref().unwrap().model;
    let depot = doc.get(Slot(3)).unwrap();
    assert_eq!(
        depot.base().attrs.get("Malzeme").map(String::as_str),
        Some("bt")
    );
    let fields = &doc.layers().get("depo").unwrap().fields;
    assert_eq!(
        fields[1].values.as_ref().map(Vec::len),
        Some(3),
        "the depot's copy follows"
    );
    assert_eq!(
        doc.settings().domains[0].values.as_ref().map(Vec::len),
        Some(3)
    );
    // One step takes the list, the copies and the records back.
    let doc = &mut app.document.as_mut().unwrap().model;
    doc.undo();
    assert_eq!(
        doc.get(Slot(3))
            .unwrap()
            .base()
            .attrs
            .get("Malzeme")
            .map(String::as_str),
        Some("ym")
    );
    assert_eq!(
        doc.settings().domains[0].values.as_ref().map(Vec::len),
        Some(4)
    );
    assert_eq!(
        doc.layers().get("depo").unwrap().fields[1]
            .values
            .as_ref()
            .map(Vec::len),
        Some(4)
    );
}

#[test]
fn a_code_records_hold_keeps_the_window_open_until_it_is_moved() {
    let mut app = crate::files_testing::app_with_drawing();
    listed_buildings(&mut app);
    run(&mut app, "data.domains");
    let _ = app.update(Message::Domains(crate::forms::domains::Event::RemoveCode(
        2,
    )));
    let _ = app.update(Message::Domains(crate::forms::domains::Event::Save));
    assert!(app.domains.is_some(), "refused: Yığma is held by a depot");
    let doc = &app.document.as_ref().unwrap().model;
    assert!(!doc.can_undo() || doc.settings().domains[0].values.as_ref().map(Vec::len) == Some(4));
}

#[test]
fn a_draft_is_kept_incomplete_and_a_valid_step_refuses_it() {
    let mut app = crate::files_testing::app_with_drawing();
    workflow_draft(&mut app);
    let _ = app.update(Message::FormEntry(entry::Event::Save));
    assert!(app.form_entry.is_none(), "a draft saves with its warning");
    let doc = &app.document.as_ref().unwrap().model;
    assert_eq!(doc.get(Slot(1)).unwrap().base().attrs.get("Kat"), None);
    // Gönder asks for a valid record: refused, nothing written.
    app.selection.set([Slot(1)]);
    run(&mut app, "data.formEdit");
    let _ = app.update(Message::FormEntry(entry::Event::Transition(
        "Gönder".into(),
    )));
    assert!(app.form_entry.is_some());
    let doc = &app.document.as_ref().unwrap().model;
    assert_eq!(
        doc.get(Slot(1))
            .unwrap()
            .base()
            .attrs
            .get("Durum")
            .map(String::as_str),
        Some("taslak")
    );
}

#[test]
fn a_form_package_goes_to_another_layer_with_its_fields() {
    let mut app = crate::files_testing::app_with_drawing();
    imported_form(&mut app);
    let doc = &app.document.as_ref().unwrap().model;
    let forms = &doc.settings().forms;
    assert_eq!(forms.len(), 2, "the inspection form and its copy");
    let copy = &forms[1];
    assert_eq!(copy.layer, "parsel");
    let fields = &doc.layers().get("parsel").unwrap().fields;
    for name in ["Kod", "Durum", "Kat", "Gerekçe", "Geçmiş"] {
        assert!(fields.iter().any(|f| f.name == name), "{name} added");
    }
    assert_eq!(
        copy.draft.workflow.as_ref().map(|w| w.field.as_str()),
        Some("Durum")
    );
    assert_eq!(
        app.forms_window.as_ref().and_then(|w| w.chosen.clone()),
        Some(copy.id.clone())
    );
}

#[test]
fn a_rule_is_shown_on_its_field_and_a_check_kept_for_saving_waits_for_kaydet() {
    let mut app = crate::files_testing::app_with_drawing();
    permits(&mut app);
    app.selection.set([Slot(1)]);
    run(&mut app, "data.formEdit");
    entry(
        &mut app,
        entry::Event::Value("Bitiş".into(), "01.02.2026".into()),
    );
    let codes = |app: &App| -> Vec<String> {
        app.form_entry
            .as_ref()
            .map(|e| e.issues.iter().map(|i| i.code.clone()).collect())
            .unwrap_or_default()
    };
    assert!(codes(&app).contains(&"form_layer_rule".to_owned()));
    entry(
        &mut app,
        entry::Event::Value("Bitiş".into(), "30.09.2026".into()),
    );
    // Açıklama kısa: its check is kept for saving, not said while typing.
    entry(
        &mut app,
        entry::Event::Value("Açıklama".into(), "kısa".into()),
    );
    assert!(!codes(&app).contains(&"form_check_failed".to_owned()));
    entry(&mut app, entry::Event::Save);
    assert!(app.form_entry.is_some(), "the check refused Kaydet");
    assert!(codes(&app).contains(&"form_check_failed".to_owned()));
    entry(
        &mut app,
        entry::Event::Value("Açıklama".into(), "Kolonlarda çatlak.".into()),
    );
    entry(&mut app, entry::Event::Save);
    assert!(app.form_entry.is_none(), "written and closed");
}

/// docs/adr/0440 §2: a link made in the window waits for Kaydet and goes in with the record, one
/// step; Vazgeç drops it.
#[test]
fn a_link_waits_for_kaydet_and_goes_in_with_the_record() {
    let mut app = crate::files_testing::app_with_drawing();
    meters_pending(&mut app);
    let e = app.form_entry.as_ref().expect("open");
    assert_eq!(e.pending_for_tests(), 1);
    assert_eq!(
        e.related_of("sayaclar").len(),
        1,
        "the linked meter is listed"
    );
    let free = Slot(63);
    let link = |app: &App| {
        app.document
            .as_ref()
            .unwrap()
            .model
            .get(free)
            .unwrap()
            .base()
            .attrs
            .get("bina")
            .cloned()
    };
    assert_eq!(link(&app), None, "not written yet");
    entry(&mut app, entry::Event::Save);
    assert!(app.form_entry.is_none());
    let parent = app
        .document
        .as_ref()
        .unwrap()
        .model
        .uid(Slot(2))
        .unwrap()
        .to_string();
    assert_eq!(link(&app).as_deref(), Some(parent.as_str()));
    let doc = &mut app.document.as_mut().unwrap().model;
    doc.undo();
    assert_eq!(
        link(&app),
        None,
        "one step: the link goes back with the name"
    );
    assert_eq!(
        app.document
            .as_ref()
            .unwrap()
            .model
            .get(Slot(2))
            .unwrap()
            .base()
            .attrs
            .get("Ad")
            .map(String::as_str),
        Some("B blok")
    );
    // Vazgeç drops a waiting link.
    meters_pending(&mut app);
    entry(&mut app, entry::Event::Cancel);
    assert_eq!(link(&app), None);
}

/// docs/adr/0440 §7: a child's form opens over its parent's; the parent's draft waits and comes back;
/// at most four forms deep.
#[test]
fn a_child_form_opens_over_its_parent_and_the_parent_comes_back_with_its_draft() {
    let mut app = crate::files_testing::app_with_drawing();
    meters_nested(&mut app);
    assert_eq!(app.form_stack.len(), 1, "the building waits");
    assert_eq!(
        app.form_entry.as_ref().map(|e| e.name.clone()).as_deref(),
        Some("Sayaç")
    );
    entry(&mut app, entry::Event::Value("no".into(), "S-900".into()));
    entry(&mut app, entry::Event::Save);
    let e = app.form_entry.as_ref().expect("the building back");
    assert_eq!(e.name, "Bina");
    assert!(e.dirty_now(), "its typed name kept");
    assert!(app.form_stack.is_empty());
    let meter = &app
        .document
        .as_ref()
        .unwrap()
        .model
        .get(Slot(3))
        .unwrap()
        .base()
        .attrs;
    assert_eq!(meter.get("no").map(String::as_str), Some("S-900"));
}

/// docs/adr/0440 §4: the step's snapshot ends its log line; a meter changed after it is said.
#[test]
fn a_step_s_snapshot_is_logged_and_a_later_change_is_said() {
    let mut app = crate::files_testing::app_with_drawing();
    meters_snapshot(&mut app);
    let doc = &app.document.as_ref().unwrap().model;
    let log = doc
        .get(Slot(1))
        .unwrap()
        .base()
        .attrs
        .get("Geçmiş")
        .cloned()
        .unwrap_or_default();
    let then = kentos_native_application::forms::logged_snapshot(&log)
        .expect("a snapshot")
        .to_owned();
    let attrs = doc.get(Slot(1)).unwrap().base().attrs.clone();
    let uid = doc.uid(Slot(1)).unwrap().to_string();
    let now = kentos_native_application::forms::record_snapshot(doc, "bina", &uid, &attrs).unwrap();
    assert_ne!(then, now, "the meter changed after the step");
}

/// docs/adr/0439 §1–§5: only the parcels holding the building are offered, with their columns; the
/// chosen one's values are copied with a stamp and its columns shown; Kaydet writes them.
#[test]
fn a_chosen_parcel_is_copied_with_its_stamp_and_shown_with_its_columns() {
    let mut app = crate::files_testing::app_with_drawing();
    sources_entry(&mut app);
    let e = app.form_entry.as_ref().expect("open");
    let state = e.state_for_tests("parsel").expect("the parcel's state");
    let offered: Vec<&str> = state
        .options
        .as_ref()
        .map(|o| o.iter().map(|c| c.label.as_str()).collect())
        .unwrap_or_default();
    assert_eq!(offered, ["101/5"], "only the parcel holding the building");
    let preview = state.preview.as_ref().expect("the chosen parcel's columns");
    assert_eq!(preview[0].value, "450.50");
    entry(&mut app, entry::Event::Save);
    assert!(app.form_entry.is_none(), "written and closed");
    let doc = &app.document.as_ref().unwrap().model;
    let attrs = &doc.get(Slot(5)).unwrap().base().attrs;
    assert_eq!(attrs.get("Ada").map(String::as_str), Some("101"));
    assert_eq!(attrs.get("Alan").map(String::as_str), Some("450.50"));
    assert_eq!(attrs.get("Malik").map(String::as_str), Some("Ayşe Kaya"));
    assert!(
        attrs
            .get("Kaynak")
            .is_some_and(|s| s.starts_with("Parsel · ") && s.contains(" · #")),
        "{attrs:?}"
    );
}

/// docs/adr/0439 §6: a searched list searches once typing paused; an answer whose turn passed is
/// dropped, the newest is taken.
#[test]
fn a_searched_list_takes_the_newest_answer_and_drops_a_late_one() {
    let mut app = crate::files_testing::app_with_drawing();
    sources(&mut app);
    app.selection.set([Slot(6)]);
    run(&mut app, "data.formEdit");
    let _ = app.update(Message::FormEntry(entry::Event::Query(
        "Mahalle".into(),
        "k".into(),
    )));
    let first = app
        .form_entry
        .as_ref()
        .unwrap()
        .search_for_tests("Mahalle")
        .0
        .unwrap();
    let _ = app.update(Message::FormEntry(entry::Event::Query(
        "Mahalle".into(),
        "ka".into(),
    )));
    let second = app
        .form_entry
        .as_ref()
        .unwrap()
        .search_for_tests("Mahalle")
        .0
        .unwrap();
    assert!(second > first);
    let _ = app.update(Message::FormEntry(entry::Event::Search(
        "Mahalle".into(),
        first,
    )));
    assert_eq!(
        app.form_entry
            .as_ref()
            .unwrap()
            .search_for_tests("Mahalle")
            .1,
        None,
        "a late answer is dropped"
    );
    let _ = app.update(Message::FormEntry(entry::Event::Search(
        "Mahalle".into(),
        second,
    )));
    assert_eq!(
        app.form_entry
            .as_ref()
            .unwrap()
            .search_for_tests("Mahalle")
            .1
            .as_deref(),
        Some("ka")
    );
    // An answer for another field does not land here.
    let _ = app.update(Message::FormEntry(entry::Event::Search(
        "Kat".into(),
        second,
    )));
    assert_eq!(
        app.form_entry.as_ref().unwrap().search_for_tests("Kat").1,
        None
    );
}

/// docs/adr/0439 §7: the project's value lists go out as a package and come back in.
#[test]
fn the_value_lists_go_out_as_a_package_and_come_back() {
    let mut app = crate::files_testing::app_with_drawing();
    sources_domains(&mut app);
    let text = app.domains_package_text().expect("a package");
    let doc = app.document.as_mut().unwrap();
    let mut settings = doc.model.settings().clone();
    settings.domains.retain(|d| d.id != "cati");
    doc.model.set_settings(settings);
    app.domains_imported(Some(("deger-listeleri.kliste".into(), text)));
    let ids: Vec<String> = app
        .document
        .as_ref()
        .unwrap()
        .model
        .settings()
        .domains
        .iter()
        .map(|d| d.id.clone())
        .collect();
    assert_eq!(ids, ["kat", "cati"]);
}

/// docs/adr/0439 §8: Değerleri biçime çevir writes an import's values in their fields' canonical texts
/// and keeps what does not fit as written.
#[test]
fn values_written_before_the_fields_are_turned_into_their_canonical_texts() {
    let mut app = crate::files_testing::app_with_drawing();
    sources_migrated(&mut app);
    let doc = &app.document.as_ref().unwrap().model;
    let a = |slot: u32, k: &str| doc.get(Slot(slot)).unwrap().base().attrs.get(k).cloned();
    assert_eq!(a(1, "alan").as_deref(), Some("450.5"));
    assert_eq!(a(1, "tarih").as_deref(), Some("2024-03-01"));
    assert_eq!(a(3, "alan").as_deref(), Some("seksen"), "kept as written");
    assert!(doc.can_undo());
}

#[test]
fn a_record_changed_meanwhile_is_reloaded_with_what_was_typed_or_written_over() {
    let mut app = crate::files_testing::app_with_drawing();
    permit_conflict(&mut app);
    let e = app.form_entry.as_ref().expect("still open");
    assert!(e.conflict_for_tests(), "the bar says it changed");
    entry(&mut app, entry::Event::Reload);
    entry(&mut app, entry::Event::Save);
    assert!(app.form_entry.is_none());
    let doc = &app.document.as_ref().unwrap().model;
    let attrs = &doc.get(Slot(1)).unwrap().base().attrs;
    assert_eq!(
        attrs.get("Kat").map(String::as_str),
        Some("4"),
        "typed kept"
    );
    assert_eq!(
        attrs.get("Açıklama").map(String::as_str),
        Some("Başkası yazdı."),
        "theirs kept"
    );
}

#[test]
fn the_rules_window_writes_through_the_command_in_one_step() {
    let mut app = crate::files_testing::app_with_drawing();
    permits(&mut app);
    run(&mut app, "data.layerRules");
    use crate::forms::layer_rules::Event as R;
    let _ = app.update(Message::LayerRules(R::New));
    let _ = app.update(Message::LayerRules(R::Name("Kat sınırı".into())));
    let _ = app.update(Message::LayerRules(R::Rule(
        "Kat boş veya Kat <= 40".into(),
    )));
    let _ = app.update(Message::LayerRules(R::Message("En çok 40 kat.".into())));
    let _ = app.update(Message::LayerRules(R::Save));
    assert!(app.layer_rules.is_none(), "closed on Kaydet");
    let doc = &app.document.as_ref().unwrap().model;
    assert_eq!(doc.layers().get("ruhsat").unwrap().rules.len(), 4);
    // A write breaking it is refused, whatever writes it.
    let uid = doc.uid(Slot(1)).unwrap().to_string();
    let doc = &mut app.document.as_mut().unwrap().model;
    let refused = kentos_native_application::set::execute(
        &mut kentos_native_application::ExecutionContext::new(doc),
        serde_json::from_value(
            json!({ "uids": [uid], "attrs": { "Kat": "45" }, "operation": "attributes" }),
        )
        .unwrap(),
    );
    assert!(
        matches!(refused, kentos_commands::CommandResult::Failed { error } if error.code == "rule_broken")
    );
}

#[test]
fn a_step_form_keeps_a_step_with_an_error_and_its_mirror_writes_nothing() {
    let mut app = crate::files_testing::app_with_drawing();
    stepped_survey(&mut app);
    // Kat is required on step 2: İleri from step 2 is refused while it is empty.
    let _ = app.update(Message::FormEntry(entry::Event::Tab(1)));
    let _ = app.update(Message::FormEntry(entry::Event::Tab(2)));
    assert_eq!(
        app.form_entry.as_ref().map(|e| e.tab_for_tests()),
        Some(1),
        "held on Yapı"
    );
    let _ = app.update(Message::FormEntry(entry::Event::Value(
        "Kat".into(),
        "6".into(),
    )));
    let _ = app.update(Message::FormEntry(entry::Event::Tab(2)));
    assert_eq!(app.form_entry.as_ref().map(|e| e.tab_for_tests()), Some(2));
    // The tree picks a code; the mirror shows it, nothing more.
    let _ = app.update(Message::FormEntry(entry::Event::Value(
        "Kullanım".into(),
        "ofi".into(),
    )));
    let _ = app.update(Message::FormEntry(entry::Event::Save));
    let doc = &app.document.as_ref().unwrap().model;
    let attrs = &doc.get(Slot(1)).unwrap().base().attrs;
    assert_eq!(attrs.get("Kullanım").map(String::as_str), Some("ofi"));
    assert_eq!(attrs.get("Kat").map(String::as_str), Some("6"));
}

#[test]
fn a_copied_group_is_pasted_with_fresh_ids_and_its_fields_rebound() {
    let mut app = crate::files_testing::app_with_drawing();
    stepped_designer(&mut app);
    designer(&mut app, designer::Event::SelectItem("yapi".into()));
    designer(&mut app, designer::Event::CopyItem);
    designer(&mut app, designer::Event::PasteItem);
    let w = app.forms_window.as_ref().unwrap();
    let body = w.body.as_ref().unwrap();
    assert_eq!(body.items.len(), 4, "the tab pasted as a fourth step");
    let pasted = &body.items[3];
    assert_ne!(pasted.id(), "yapi");
    assert!(
        pasted.items().iter().all(|i| i.id() != "kat"),
        "its items' ids are fresh"
    );
    // Alt+↑ moves it back up one.
    designer(&mut app, designer::Event::Move(-1));
    let body = app.forms_window.as_ref().unwrap().body.as_ref().unwrap();
    assert_eq!(body.items[2].id(), pasted_id(&app));
}

pub(super) fn pasted_id(app: &App) -> String {
    app.forms_window.as_ref().unwrap().item.clone().unwrap()
}
