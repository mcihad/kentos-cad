//! The forms' reach (docs/adr/0443 §5, §6): a form in other languages, the issues walked one by one, the
//! question when closing with what was typed, a two-column group filled a row at a time; and the
//! keyboard's real way through Formla düzenle (FORM-QA-04): Tab and Shift+Tab within the window,
//! typing into the focused field, a tab left and come back to, Kaydet sending the focus to the issue,
//! Esc asking before what was typed is dropped.

use kentos_domain::Slot;
use serde_json::{Value, json};

use crate::app::App;
use crate::form_scenes::{buildings, buildings_with, designer, entry, form};
use crate::forms::{designer as fd, entry as fe};
use crate::tools_scenes::run;

/// The building form with an English and an Arabic translation, its two-column group a row at a time.
fn translated_form() -> Value {
    let mut f = form(true);
    let mut body = f["draft"].clone();
    body["items"][1]["items"][0]["across"] = json!(true);
    body["translations"] = json!({
        "ar": { "name": "مسح المباني", "items": {
            "genel": { "label": "عام" }, "durum": { "label": "الحالة" }, "kat": { "label": "عدد الطوابق" } } },
        "en": { "name": "Building survey", "items": {
            "genel": { "label": "General" }, "ayrinti": { "label": "Details" },
            "durum": { "label": "Condition", "choices": { "iyi": "Sound", "inceleme": "Needs a closer look" } },
            "kat": { "label": "Storeys", "help": "Ground floor included." },
            "aciklama": { "label": "Note", "placeholder": "What should be looked at?" }, "denetim": { "label": "Inspection" },
            "tarih": { "label": "Date" }, "onay": { "label": "Approved" },
            "bilgi": { "label": "The inspector records it; approval changes only on a building under review." } } }
    });
    let typed: kentos_contracts::FormBody = serde_json::from_value(body.clone()).expect("a body");
    f["draft"] = body.clone();
    f["releases"] =
        json!([{ "version": 1, "digest": kentos_contracts::body_digest(&typed), "body": body }]);
    f
}

/// Formla düzenle on the first building in English: the labels, help and the window's title in it.
pub(crate) fn translated_entry(app: &mut App) {
    buildings_with(app, translated_form());
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
    entry(app, fe::Event::Language(Some("en".into())));
}

/// Formlar, its overview: Çeviriler with English chosen, the form's words beside its own.
pub(crate) fn translations_designer(app: &mut App) {
    buildings_with(app, translated_form());
    run(app, "data.forms");
    designer(app, fd::Event::LanguagePick("en".into()));
}

/// Two errors (storeys out of range, the note asked for): Sonraki sorun went to the first.
pub(crate) fn issues_walked(app: &mut App) {
    buildings(app, true);
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
    entry(app, fe::Event::Value("kat-sayisi".into(), "99".into()));
    entry(app, fe::Event::FirstIssue);
}

/// Vazgeç with a storey count typed: the window asks first.
pub(crate) fn closing_asked(app: &mut App) {
    buildings(app, true);
    app.selection.set([Slot(2)]);
    run(app, "data.formEdit");
    entry(app, fe::Event::Value("kat-sayisi".into(), "5".into()));
    entry(app, fe::Event::Cancel);
}

/// Ayrıntı's group of two columns filled a row at a time.
pub(crate) fn across_group(app: &mut App) {
    buildings_with(app, translated_form());
    app.selection.set([Slot(2)]);
    run(app, "data.formEdit");
    entry(app, fe::Event::Tab(1));
}

/// The first building's meters bounded one to one (docs/adr/0444 §1): it holds more, the form says so and
/// Kaydet stays.
pub(crate) fn related_bounded(app: &mut App) {
    let mut items = crate::form_scenes::meters_items();
    items[2]["max"] = json!(1);
    items[2]["view"] = json!("list");
    crate::form_scenes::meters_rebodied(app, items);
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
}

/// The first building's form opened with its release's fields changed since (docs/adr/0444 §2).
pub(crate) fn stale_release(app: &mut App) {
    // The release as a publish now writes it: its bound fields' digest.
    let mut f = form(true);
    let body: kentos_contracts::FormBody =
        serde_json::from_value(f["draft"].clone()).expect("a body");
    let fields: Vec<kentos_contracts::LayerField> =
        serde_json::from_value(crate::form_scenes::fields()).expect("fields");
    f["releases"][0]["fields"] = json!(kentos_contracts::bound_fields_digest(&body, &fields));
    buildings_with(app, f);
    let doc = app.document.as_mut().expect("a drawing");
    let mut fields = doc.model.layers().get("bina").expect("bina").fields.clone();
    if let Some(f) = fields.iter_mut().find(|f| f.name == "Kat") {
        f.max = Some("30".into());
    }
    let _ = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        kentos_contracts::CAD_LAYERS_EDIT,
        kentos_contracts::CallOp::Execute,
        json!({ "operation": "fields", "layer": "bina", "fields": fields }),
    );
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
}

/// Formlar with the building form withdrawn (docs/adr/0445 §1): its status and Yeniden yayına al.
pub(crate) fn withdrawn_designer(app: &mut App) {
    buildings(app, true);
    run(app, "data.forms");
    designer(app, fd::Event::Withdraw(true));
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use iced::advanced::widget::Operation;
    use iced::advanced::widget::operation::Focusable;
    use iced::{Rectangle, Size};
    use kentos_ui::snapshot::{Input, Snapshot};

    use super::*;
    use crate::app::Message;

    fn attrs(app: &App, slot: u32) -> std::collections::BTreeMap<String, String> {
        let doc = app.document.as_ref().expect("a drawing");
        doc.model
            .get(Slot(slot))
            .map(|e| e.base().attrs.clone())
            .unwrap_or_default()
    }

    /// Runs a message and the widget operations its task asks for (a focus) on the window.
    fn drive(s: &mut Snapshot, app: &mut App, message: Message) {
        use iced::futures::StreamExt as _;
        let task = app.update(message);
        if let Some(mut stream) = iced_runtime::task::into_stream(task) {
            while let Some(action) = iced::futures::executor::block_on(stream.next()) {
                match action {
                    iced_runtime::Action::Widget(operation) => s.operate(app.view(), operation),
                    iced_runtime::Action::Output(m) => drive(s, app, m),
                    _ => {}
                }
            }
        }
    }

    fn settle(s: &mut Snapshot, app: &mut App) {
        s.settle(app, |a| a.view(), &mut |a, m| {
            let _ = a.update(m);
        });
    }

    /// The focused field's key among `keys`; none when another widget, or none, has the focus.
    fn focused_key(s: &mut Snapshot, app: &App, keys: &[&str]) -> Option<String> {
        struct Find(Arc<Mutex<Option<iced::widget::Id>>>);
        impl Operation for Find {
            fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
                operate(self);
            }
            fn focusable(
                &mut self,
                id: Option<&iced::advanced::widget::Id>,
                _bounds: Rectangle,
                state: &mut dyn Focusable,
            ) {
                if state.is_focused() {
                    *self.0.lock().unwrap() = id.cloned();
                }
            }
        }
        let found = Arc::new(Mutex::new(None));
        s.operate(app.view(), Box::new(Find(found.clone())));
        let id = found.lock().unwrap().clone()?;
        keys.iter()
            .find(|k| fe::input_id(k) == id)
            .map(|k| (*k).to_owned())
    }

    fn tab(s: &mut Snapshot, app: &App, back: bool) {
        s.operate(
            app.view(),
            Box::new(kentos_ui::focus::step_operation(back, false)),
        );
    }

    #[test]
    fn the_keyboard_goes_through_the_form_keeps_what_is_typed_and_asks_before_dropping_it() {
        let mut app = crate::files_testing::app_with_drawing();
        buildings(&mut app, true);
        app.selection.set([Slot(1)]);
        run(&mut app, "data.formEdit");
        let mut s = Snapshot::software(Size::new(1440.0, 900.0)).expect("a renderer");
        settle(&mut s, &mut app);
        let keys = ["Durum", "kat-sayisi", "Açıklama"];
        // Tab goes round the window's fields in their order, never behind it; Shift+Tab back.
        let mut seen = Vec::new();
        for _ in 0..12 {
            tab(&mut s, &app, false);
            if let Some(k) = focused_key(&mut s, &app, &keys)
                && seen.last() != Some(&k)
            {
                seen.push(k);
            }
        }
        let storeys = seen
            .iter()
            .position(|k| k == "kat-sayisi")
            .expect("Tab reaches the storeys");
        let note = seen
            .iter()
            .position(|k| k == "Açıklama")
            .expect("Tab reaches the note");
        assert!(storeys < note, "in the form's order: {seen:?}");
        while focused_key(&mut s, &app, &keys).as_deref() != Some("kat-sayisi") {
            tab(&mut s, &app, false);
        }
        tab(&mut s, &app, false);
        tab(&mut s, &app, true);
        assert_eq!(
            focused_key(&mut s, &app, &keys).as_deref(),
            Some("kat-sayisi"),
            "Shift+Tab goes back"
        );
        // Typing goes into the focused field, through the form.
        s.input(
            &mut app,
            |a| a.view(),
            &mut |a, m| {
                let _ = a.update(m);
            },
            Input::Type("5".into()),
        );
        let typed = app.form_entry.as_ref().expect("the form").value_of("Kat");
        assert_eq!(typed.as_deref(), Some("35"), "typed after what it held");
        // Another tab and back: what was typed is kept.
        entry(&mut app, fe::Event::Tab(1));
        entry(&mut app, fe::Event::Tab(0));
        let kept = app.form_entry.as_ref().expect("the form").value_of("Kat");
        assert_eq!(kept.as_deref(), Some("35"));
        // Kaydet with the note missing: the window stays and the focus goes to the note.
        drive(&mut s, &mut app, Message::FormEntry(fe::Event::Save));
        settle(&mut s, &mut app);
        assert!(app.form_entry.is_some(), "refused: the window stays");
        assert_eq!(
            focused_key(&mut s, &app, &keys).as_deref(),
            Some("Açıklama"),
            "the issue's field takes the focus"
        );
        // Esc asks first; Esc on the question goes back to the form, nothing dropped.
        // Esc as the window's keys take it (input.rs: a dialog's Esc).
        app.close_dialog();
        let e = app.form_entry.as_ref().expect("asked, not closed");
        assert!(e.asking_close(), "the window asks");
        app.close_dialog();
        assert_eq!(
            app.dialog,
            Some(crate::app::Dialog::FormEntry),
            "still shown"
        );
        let e = app.form_entry.as_ref().expect("back to the form");
        assert!(!e.asking_close());
        assert_eq!(e.value_of("Kat").as_deref(), Some("35"));
        // Kaydetmeden kapat: closed, the record as it was.
        entry(&mut app, fe::Event::Cancel);
        entry(&mut app, fe::Event::Discard);
        assert!(app.form_entry.is_none());
        assert_eq!(attrs(&app, 1).get("Kat").map(String::as_str), Some("3"));
        // A form opened and left untouched closes without a word.
        run(&mut app, "data.formEdit");
        entry(&mut app, fe::Event::Cancel);
        assert!(app.form_entry.is_none(), "nothing typed: closed at once");
    }

    #[test]
    fn kaydet_ve_kapat_writes_and_closes_or_goes_back_to_the_issue() {
        let mut app = crate::files_testing::app_with_drawing();
        closing_asked(&mut app);
        assert!(app.form_entry.as_ref().is_some_and(|e| e.asking_close()));
        entry(&mut app, fe::Event::Save);
        assert!(app.form_entry.is_none(), "written and closed");
        assert_eq!(attrs(&app, 2).get("Kat").map(String::as_str), Some("5"));
        // The first building needs its note: Kaydet ve kapat goes back to the form, its issue shown.
        app.selection.set([Slot(1)]);
        run(&mut app, "data.formEdit");
        entry(&mut app, fe::Event::Value("kat-sayisi".into(), "4".into()));
        entry(&mut app, fe::Event::Cancel);
        entry(&mut app, fe::Event::Save);
        let e = app.form_entry.as_ref().expect("refused: the window stays");
        assert!(!e.asking_close(), "back to the form");
        assert_eq!(attrs(&app, 1).get("Kat").map(String::as_str), Some("3"));
    }

    #[test]
    fn the_next_issue_goes_round_the_errors_in_the_form_s_order() {
        let mut app = crate::files_testing::app_with_drawing();
        issues_walked(&mut app);
        let e = app.form_entry.as_ref().expect("the form");
        assert_eq!(e.issue_fields(), ["kat-sayisi", "Açıklama"]);
        assert_eq!(e.issue_at(), 1, "the first went to");
        entry(&mut app, fe::Event::FirstIssue);
        assert_eq!(app.form_entry.as_ref().unwrap().issue_at(), 2);
        entry(&mut app, fe::Event::FirstIssue);
        assert_eq!(
            app.form_entry.as_ref().unwrap().issue_at(),
            1,
            "round again after the last"
        );
    }

    #[test]
    fn a_translation_shows_the_form_in_its_language_and_writes_the_same_values() {
        let mut app = crate::files_testing::app_with_drawing();
        translated_entry(&mut app);
        let e = app.form_entry.as_ref().expect("the form");
        assert_eq!(e.name_in(), "Building survey");
        assert_eq!(e.words_of("durum", None).as_deref(), Some("Condition"));
        assert_eq!(e.words_of("kat", None).as_deref(), Some("Storeys"));
        assert!(
            e.translated_choice("durum", "inceleme")
                .is_some_and(|l| l == "Needs a closer look")
        );
        // A list's labels are the layer's: the codes and values are the form's own.
        entry(&mut app, fe::Event::Value("kat-sayisi".into(), "6".into()));
        entry(
            &mut app,
            fe::Event::Value("Açıklama".into(), "Çatı arası bakılmalı.".into()),
        );
        entry(&mut app, fe::Event::Save);
        assert!(app.form_entry.is_none(), "written");
        let a = attrs(&app, 1);
        assert_eq!(a.get("Kat").map(String::as_str), Some("6"));
        assert_eq!(a.get("Durum").map(String::as_str), Some("inceleme"));
        // A language the form has not is not chosen.
        run(&mut app, "data.formEdit");
        entry(&mut app, fe::Event::Language(Some("de".into())));
        assert_eq!(app.form_entry.as_ref().unwrap().language, None);
        assert_eq!(
            crate::forms::entry::language_name(Some("pt-BR")),
            "Português (pt-BR)"
        );
        assert_eq!(crate::forms::entry::language_name(Some("xx")), "xx");
    }

    #[test]
    fn the_designer_writes_a_translation_as_one_step_and_takes_it_back() {
        let mut app = crate::files_testing::app_with_drawing();
        buildings(&mut app, true);
        run(&mut app, "data.forms");
        // The form's own language and a malformed tag are not added.
        for tag in ["tr", "EN", "english"] {
            designer(&mut app, fd::Event::LanguageText(tag.into()));
            designer(&mut app, fd::Event::LanguageAdd);
        }
        assert!(app_body(&app).translations.is_empty());
        designer(&mut app, fd::Event::LanguageText("de".into()));
        designer(&mut app, fd::Event::LanguageAdd);
        assert!(app_body(&app).translations.contains_key("de"));
        let before = app.forms_window.as_ref().unwrap().undo.len();
        for typed in ["Z", "Zu", "Zustand"] {
            designer(
                &mut app,
                fd::Event::Translate(Some("durum".into()), fd::Said::Label, typed.into()),
            );
        }
        assert_eq!(
            app.forms_window.as_ref().unwrap().undo.len(),
            before + 1,
            "typing is one step"
        );
        assert_eq!(
            app_body(&app).translations["de"].items["durum"]
                .label
                .as_deref(),
            Some("Zustand")
        );
        designer(&mut app, fd::Event::Undo);
        assert!(
            !app_body(&app).translations["de"]
                .items
                .contains_key("durum")
        );
        // Kaydet keeps it with the draft; the preflight warns of words for an item the form has not.
        designer(
            &mut app,
            fd::Event::Translate(Some("durum".into()), fd::Said::Label, "Zustand".into()),
        );
        designer(&mut app, fd::Event::Save);
        let form = &app.project_forms()[0];
        assert_eq!(
            form.draft.translations["de"].items["durum"]
                .label
                .as_deref(),
            Some("Zustand")
        );
        let mut stale = form.draft.clone();
        stale.translations.get_mut("de").unwrap().items.insert(
            "yok".into(),
            kentos_contracts::ItemWords {
                label: Some("X".into()),
                ..Default::default()
            },
        );
        let issues = kentos_forms::preflight(&stale, &app.layer_fields_of("bina"), &[]);
        assert!(issues.iter().any(|i| i.code == "form_translation_item"));
    }

    /// The window and the command agree (docs/adr/0444 §4, FORM-STD-09): the same values typed into Formla
    /// düzenle and given to `cad.forms.validate` leave the same record, the same issues and the same items
    /// shown, required and read-only.
    #[test]
    fn the_window_and_the_command_judge_a_record_alike() {
        let inputs: [(u32, &[(&str, &str)]); 6] = [
            (1, &[]),
            (1, &[("kat-sayisi", "99")]),
            (1, &[("Açıklama", "kısa")]),
            (1, &[("Durum", "iyi"), ("kat-sayisi", " 07 ")]),
            (2, &[("Onaylı", "false")]),
            (
                2,
                &[
                    ("Durum", "inceleme"),
                    ("Açıklama", "Çatı arası ayrıca bakılmalı."),
                ],
            ),
        ];
        for (slot, values) in inputs {
            let mut app = crate::files_testing::app_with_drawing();
            buildings(&mut app, true);
            app.selection.set([Slot(slot)]);
            run(&mut app, "data.formEdit");
            for (k, v) in values {
                entry(&mut app, fe::Event::Value((*k).into(), (*v).into()));
            }
            let e = app.form_entry.as_ref().expect("the form");
            let doc = app.document.as_mut().expect("a drawing");
            let uid = doc.model.uid(Slot(slot)).expect("a uid").to_string();
            let given: std::collections::BTreeMap<String, String> = values
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect();
            let got = kentos_native_application::registry::run(
                &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
                kentos_contracts::CAD_FORMS_VALIDATE,
                kentos_contracts::CallOp::Execute,
                json!({ "form": "bina-denetimi", "uid": uid, "values": given }),
            )
            .expect("runs");
            let out = &got["output"];
            let at = format!("B-{slot} {values:?}");
            assert_eq!(
                serde_json::to_value(e.after_attrs()).unwrap(),
                out["attrs"],
                "{at}: the record"
            );
            let issues = |list: &Value| -> Vec<(String, String, String, String)> {
                let mut v: Vec<_> = list
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|i| {
                        let s = |k: &str| i[k].as_str().unwrap_or_default().to_owned();
                        (s("code"), s("severity"), s("item"), s("field"))
                    })
                    .collect();
                v.sort();
                v
            };
            assert_eq!(
                issues(&serde_json::to_value(&e.issues).unwrap()),
                issues(&out["issues"]),
                "{at}: the issues"
            );
            let states = |list: &Value| -> Vec<(String, bool, bool, bool)> {
                list.as_array()
                    .into_iter()
                    .flatten()
                    .map(|s| {
                        let b = |k: &str| s[k].as_bool().unwrap_or(false);
                        (
                            s["id"].as_str().unwrap_or_default().to_owned(),
                            b("shown"),
                            b("required"),
                            b("readOnly"),
                        )
                    })
                    .collect()
            };
            assert_eq!(
                states(&serde_json::to_value(e.item_states()).unwrap()),
                states(&out["items"]),
                "{at}: the items"
            );
        }
    }

    #[test]
    fn a_related_item_bounded_one_to_one_refuses_a_record_holding_more() {
        let mut app = crate::files_testing::app_with_drawing();
        related_bounded(&mut app);
        let e = app.form_entry.as_ref().expect("the form");
        assert!(
            e.issues.iter().any(|i| i.code == "form_related_many"),
            "{:?}",
            e.issues
        );
        entry(&mut app, fe::Event::Save);
        assert!(app.form_entry.is_some(), "refused: the window stays");
    }

    #[test]
    fn a_link_taken_off_in_the_window_counts_before_it_is_written() {
        let mut app = crate::files_testing::app_with_drawing();
        related_bounded(&mut app);
        let held = app.form_entry.as_ref().unwrap().related_now("sayaclar");
        let n = held.len();
        assert!(n >= 2, "{n}");
        // One fewer allowed than the building holds: one taken off in the window clears it.
        let mut items = crate::form_scenes::meters_items();
        items[2]["max"] = json!(n - 1);
        items[2]["view"] = json!("list");
        crate::form_scenes::meters_rebodied(&mut app, items);
        app.selection.set([Slot(1)]);
        run(&mut app, "data.formEdit");
        let many = |app: &App| {
            app.form_entry
                .as_ref()
                .unwrap()
                .issues
                .iter()
                .any(|i| i.code == "form_related_many")
        };
        assert!(many(&app));
        entry(
            &mut app,
            fe::Event::Unlink("sayaclar".into(), held[0].clone()),
        );
        assert!(!many(&app), "the window counts the link taken off");
        entry(&mut app, fe::Event::Save);
        assert!(
            app.form_entry.is_none(),
            "the command counts it alike and writes"
        );
    }

    #[test]
    fn a_release_whose_fields_changed_is_said_in_the_window() {
        let mut app = crate::files_testing::app_with_drawing();
        stale_release(&mut app);
        let r = app.project_forms()[0].releases[0].clone();
        let fields = app.layer_fields_of("bina");
        assert_ne!(
            r.fields.as_deref(),
            Some(kentos_contracts::bound_fields_digest(&r.body, &fields).as_str()),
            "the storeys' bound changed since"
        );
        assert!(app.form_entry.is_some());
    }

    #[test]
    fn a_withdrawn_form_opens_no_record_and_an_open_window_keeps_what_was_typed() {
        let mut app = crate::files_testing::app_with_drawing();
        buildings(&mut app, true);
        // A window open before the form is withdrawn.
        app.selection.set([Slot(2)]);
        run(&mut app, "data.formEdit");
        entry(&mut app, fe::Event::Value("kat-sayisi".into(), "6".into()));
        let open = app.form_entry.take();
        let dialog = app.dialog.take();
        run(&mut app, "data.forms");
        designer(&mut app, fd::Event::Withdraw(true));
        assert!(app.project_forms()[0].withdrawn);
        let _ = app.update(Message::Forms(fd::Event::Cancel));
        // Opening it again is refused and said.
        run(&mut app, "data.formEdit");
        assert!(app.form_entry.is_none(), "not opened");
        // The window that was open: Kaydet refused, what was typed kept.
        app.form_entry = open;
        app.dialog = dialog;
        entry(&mut app, fe::Event::Save);
        let e = app.form_entry.as_ref().expect("the window stays");
        assert_eq!(e.value_of("Kat").as_deref(), Some("6"));
        assert_eq!(attrs(&app, 2).get("Kat").map(String::as_str), Some("2"));
        // Put back: it writes again.
        let _ = app.update(Message::FormEntry(fe::Event::Discard));
        run(&mut app, "data.forms");
        designer(&mut app, fd::Event::Withdraw(false));
        assert!(!app.project_forms()[0].withdrawn);
    }

    fn app_body(app: &App) -> kentos_contracts::FormBody {
        app.forms_window
            .as_ref()
            .and_then(|w| w.body.clone())
            .expect("the copy")
    }
}
