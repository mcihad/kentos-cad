//! The pictures of docs/adr/0401: Formlar with a form of three tabs' worth of items, an item's rules and
//! the preflight's issues, a new form being made; Formla düzenle on a building whose rules show the
//! note and ask for it, its issue after Kaydet, the second tab; Öznitelikler's Form row. `tools_screens`
//! takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=form-tasarim,form-tasarim-sorun,form-yeni,form-duzenle,form-duzenle-sorun,form-duzenle-sekme,form-oznitelikler cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::forms::{designer, entry};
use crate::tools_scenes::{E, N, Objects, open_edited, run};
use crate::tools_screens::Scene;

/// The buildings' fields: a status from a list (required), the storeys by a stable id, a note, a date
/// and a yes/no.
fn fields() -> Value {
    json!([
        { "name": "Durum", "kind": "text", "required": true,
          "values": [{ "code": "iyi", "label": "İyi" }, { "code": "inceleme", "label": "Ek inceleme gerekli" }] },
        { "id": "kat-sayisi", "name": "Kat", "kind": "integer", "min": "1", "max": "40" },
        { "name": "Açıklama", "kind": "text", "length": 500 },
        { "name": "Tarih", "kind": "date" },
        { "name": "Onaylı", "kind": "boolean" }
    ])
}

/// The building form: Genel (status, storeys, the note shown and asked for a building to look at again)
/// and Ayrıntı (a group of two columns shown with storeys, the approval locked for a sound building, a note).
fn form(published: bool) -> Value {
    let body = json!({ "revision": 3, "items": [
        { "type": "tab", "id": "genel", "label": "Genel", "items": [
            { "type": "field", "id": "durum", "field": "Durum", "required": true },
            { "type": "field", "id": "kat", "field": "kat-sayisi", "label": "Kat sayısı", "help": "Zemin kat dahil." },
            { "type": "field", "id": "aciklama", "field": "Açıklama", "widget": "textArea",
              "visibleWhen": "Durum = 'inceleme'", "requiredWhen": "Durum = 'inceleme'",
              "placeholder": "Neye bakılmalı?",
              "checks": [{ "rule": "uzunluk(Açıklama) >= 10", "message": "Açıklama en az 10 karakter olmalı." }] }
        ]},
        { "type": "tab", "id": "ayrinti", "label": "Ayrıntı", "items": [
            { "type": "group", "id": "denetim", "label": "Denetim", "columns": 2, "visibleWhen": "Kat >= 1", "items": [
                { "type": "field", "id": "tarih", "field": "Tarih", "default": "2026-10-16" },
                { "type": "field", "id": "onay", "field": "Onaylı", "widget": "switch", "readOnlyWhen": "Durum = 'iyi'" }
            ]},
            { "type": "note", "id": "bilgi", "text": "Denetimi yapan kişi kaydeder; onay yalnız incelenen binada değişir." }
        ]}
    ]});
    let mut f = json!({ "id": "bina-denetimi", "name": "Bina denetimi", "layer": "bina", "draft": body.clone() });
    if published {
        let typed: kentos_contracts::FormBody =
            serde_json::from_value(body.clone()).expect("a body");
        f["releases"] = json!([{ "version": 1, "digest": kentos_contracts::body_digest(&typed), "body": body, "note": "İlk yayın" }]);
    }
    f
}

/// A street block of four buildings, two with records; the layer's form in the project.
fn buildings(app: &mut App, published: bool) {
    let mut o = Objects::new();
    let corners = [[0.0, 0.0], [24.0, 0.0], [0.0, 20.0], [24.0, 20.0]];
    let mut ids = Vec::new();
    for [x, y] in corners {
        ids.push(o.path(
            "bina",
            &[[x, y], [x + 18.0, y], [x + 18.0, y + 14.0], [x, y + 14.0]],
            true,
        ));
    }
    o.data(ids[0], &[("Durum", "inceleme"), ("Kat", "3")], Some("B-1"));
    o.data(
        ids[1],
        &[("Durum", "iyi"), ("Kat", "2"), ("Onaylı", "true")],
        Some("B-2"),
    );
    o.line("yol", [-8.0, -6.0], [50.0, -6.0]);
    let fields = fields();
    let form = form(published);
    open_edited(app, o, move |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut bina = crate::tools_scenes::layer("bina", "Bina", "#3E63DD", 0.35);
        bina["fields"] = fields;
        layers.push(bina);
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("bina");
        d["settings"]["forms"] = json!([form]);
    });
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 12.0,
            min_y: N - 12.0,
            max_x: E + 54.0,
            max_y: N + 40.0,
        },
        24.0,
    );
}

fn designer(app: &mut App, event: designer::Event) {
    let _ = app.update(Message::Forms(event));
}

fn entry(app: &mut App, event: entry::Event) {
    let _ = app.update(Message::FormEntry(event));
}

/// Formlar: the form chosen, its note item selected: the rules that show and ask for it, its check.
fn designing(app: &mut App) {
    buildings(app, true);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("aciklama".into()));
}

/// Formlar: a field moved into the group of a hidden-able box while the layer requires it, a rule that
/// does not read: the preflight's issues, Yayınla off.
fn designing_issues(app: &mut App) {
    buildings(app, true);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("denetim".into()));
    designer(
        app,
        designer::Event::RuleText(designer::Rule::Visible("denetim".into()), "Kat >= ".into()),
    );
    designer(app, designer::Event::SelectItem("onay".into()));
    designer(
        app,
        designer::Event::Widget(Some(kentos_contracts::FormWidget::Number)),
    );
}

/// Formlar: Yeni form for the road layer (its fields) being made.
fn new_form(app: &mut App) {
    buildings(app, false);
    run(app, "data.forms");
    designer(app, designer::Event::NewForm);
    designer(app, designer::Event::NewName("Bina hasar tespiti".into()));
}

/// Formla düzenle on the first building: under inspection, so the note shows and is required.
fn editing(app: &mut App) {
    buildings(app, true);
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
}

/// Formla düzenle: Kaydet with the note empty: the issue under its field, the count and İlk soruna git.
fn editing_issue(app: &mut App) {
    editing(app);
    entry(app, entry::Event::Value("kat-sayisi".into(), "4".into()));
    entry(app, entry::Event::Save);
}

/// Formla düzenle on the second building, Ayrıntı: the group in two columns, the approval locked.
fn editing_tab(app: &mut App) {
    buildings(app, true);
    app.selection.set([Slot(2)]);
    run(app, "data.formEdit");
    entry(app, entry::Event::Tab(1));
}

/// Öznitelikler on the first building: the Form row with Formla düzenle, then the fields (the general
/// and geometry sections closed so the fields show).
fn properties(app: &mut App) {
    buildings(app, true);
    app.selection.set([Slot(1)]);
    app.props_closed.insert("general");
    app.props_closed.insert("geometry");
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("form-tasarim", designing),
        ("form-tasarim-sorun", designing_issues),
        ("form-yeni", new_form),
        ("form-duzenle", editing),
        ("form-duzenle-sorun", editing_issue),
        ("form-duzenle-sekme", editing_tab),
        ("form-oznitelikler", properties),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;

    fn attrs(app: &App, slot: u32) -> std::collections::BTreeMap<String, String> {
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
}
