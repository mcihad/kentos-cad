//! The pictures of docs/adr/0401: Formlar with a form of three tabs' worth of items, an item's rules and
//! the preflight's issues, a new form being made; Formla düzenle on a building whose rules show the
//! note and ask for it, its issue after Kaydet, the second tab; Öznitelikler's Form row. `tools_screens`
//! takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=form-tasarim,form-tasarim-sorun,form-yeni,form-duzenle,form-duzenle-sorun,form-duzenle-sekme,form-oznitelikler cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! KENTOS_SHOTS_ONLY=form-hesap-tasarim,form-hesap-duzenle,form-hesap-sorun,form-onizleme,form-tasarim-tasima,form-toplu cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! docs/adr/0402's: the calculating form in the designer (the material's choice filter), Formla
//! düzenle on a new building (defaults, calculations, filtered choices, the note's text, a closed group
//! opened) and its issues (a unique code another building has, a soft check's warning).
//!
//! ```text
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

/// The calculating form's fields (as `scripts/fixtures/forms_cases.py`'s `CALC_FIELDS`).
fn calc_fields() -> Value {
    json!([
        { "name": "Durum", "kind": "text", "required": true,
          "values": [{ "code": "iyi", "label": "İyi" }, { "code": "inceleme", "label": "Ek inceleme gerekli" }] },
        { "id": "kat-sayisi", "name": "Kat", "kind": "integer", "min": "1", "max": "40" },
        { "name": "Yükseklik", "kind": "decimal", "scale": 2 },
        { "name": "Taban", "kind": "decimal", "scale": 2 },
        { "name": "Riskli", "kind": "boolean" },
        { "name": "Kod", "kind": "text", "length": 20 },
        { "name": "Tür", "kind": "text", "values": [{ "code": "konut", "label": "Konut" }, { "code": "sanayi", "label": "Sanayi" }] },
        { "name": "Malzeme", "kind": "text", "values": [
            { "code": "ahsap", "label": "Ahşap" }, { "code": "beton", "label": "Betonarme" },
            { "code": "celik", "label": "Çelik" }, { "code": "tas", "label": "Taş" }] },
        { "name": "Açıklama", "kind": "text", "length": 500 },
        { "name": "Tarih", "kind": "date" },
        { "name": "Kayıt", "kind": "text", "length": 64 },
        { "name": "Puan", "kind": "integer", "min": "0", "max": "10" }
    ])
}

/// The calculating form (docs/adr/0402; `CALC_BODY`): a code by default from the storeys and unique,
/// the material offered by the kind, a closed group of calculations and a note that writes them, a
/// soft check, the date and the person by default; storeys kept for the next building.
fn calc_form() -> Value {
    let body = json!({ "revision": 1, "items": [
        { "type": "group", "id": "genel", "label": "Genel", "columns": 2, "items": [
            { "type": "field", "id": "durum", "field": "Durum", "required": true },
            { "type": "field", "id": "kat", "field": "kat-sayisi", "reuseLast": true },
            { "type": "field", "id": "kod", "field": "Kod", "unique": true, "defaultExpr": "'B-' || Kat" },
            { "type": "field", "id": "tur", "field": "Tür" },
            { "type": "field", "id": "malzeme", "field": "Malzeme",
              "choiceFilter": "Tür <> 'sanayi' veya @secenek = 'beton' veya @secenek = 'celik'" }
        ]},
        { "type": "group", "id": "hesap", "label": "Hesaplanan", "collapsed": true, "items": [
            { "type": "field", "id": "yukseklik", "field": "Yükseklik", "calculate": "Kat * 3" },
            { "type": "field", "id": "taban", "field": "Taban", "calculate": "$alan" },
            { "type": "field", "id": "riskli", "field": "Riskli", "calculate": "Kat > 5" },
            { "type": "field", "id": "puan", "field": "Puan", "calculate": "Kat - 1" },
            { "type": "note", "id": "toplam", "text": "Toplam alan {Kat * Taban} m², {$köşe} köşe." }
        ]},
        { "type": "field", "id": "aciklama", "field": "Açıklama", "visibleWhen": "Durum = 'inceleme'",
          "checks": [{ "rule": "uzunluk(Açıklama) >= 5", "message": "Açıklama çok kısa." },
                     { "rule": "uzunluk(Açıklama) >= 20", "message": "Açıklama kısa görünüyor.", "warning": true }] },
        { "type": "field", "id": "tarih", "field": "Tarih", "defaultExpr": "@tarih" },
        { "type": "field", "id": "kayit", "field": "Kayıt", "defaultExpr": "@kullanici" }
    ]});
    let typed: kentos_contracts::FormBody = serde_json::from_value(body.clone()).expect("a body");
    json!({ "id": "bina-hesabi", "name": "Bina hesabı", "layer": "bina", "draft": body.clone(), "enforce": true,
            "releases": [{ "version": 1, "digest": kentos_contracts::body_digest(&typed), "body": body }] })
}

/// The block of buildings under the calculating form: the first one's record already written (code
/// B-3); the moment and the person fixed.
fn calc_buildings(app: &mut App) {
    let mut o = Objects::new();
    let mut ids = Vec::new();
    for [x, y] in [[0.0, 0.0], [24.0, 0.0], [0.0, 20.0], [24.0, 20.0]] {
        ids.push(o.path(
            "bina",
            &[[x, y], [x + 18.0, y], [x + 18.0, y + 14.0], [x, y + 14.0]],
            true,
        ));
    }
    o.data(
        ids[0],
        &[
            ("Durum", "iyi"),
            ("Kat", "3"),
            ("Kod", "B-3"),
            ("Yükseklik", "9"),
            ("Taban", "252"),
            ("Riskli", "false"),
            ("Puan", "2"),
        ],
        Some("B-3"),
    );
    let fields = calc_fields();
    let form = calc_form();
    open_edited(app, o, move |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut bina = crate::tools_scenes::layer("bina", "Bina", "#3E63DD", 0.35);
        bina["fields"] = fields;
        layers.push(bina);
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("bina");
        d["settings"]["forms"] = json!([form]);
    });
    app.form_memory.clock = Some("2026-10-16T14:30:00".into());
    app.form_memory.person = Some("Ayşe Yılmaz".into());
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

/// Formlar: the calculating form, its height chosen: the calculation, the default expression, the
/// choices' filter and the unique and reuse settings.
fn calc_designing(app: &mut App) {
    calc_buildings(app);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("malzeme".into()));
}

/// Formla düzenle on a new building: storeys and kind typed; the code, the date and the person by
/// default, the material offered for an industrial building, the calculations' group opened, its note.
fn calc_editing(app: &mut App) {
    calc_buildings(app);
    app.selection.set([Slot(2)]);
    run(app, "data.formEdit");
    entry(app, entry::Event::Value("kat-sayisi".into(), "6".into()));
    entry(app, entry::Event::Value("Tür".into(), "sanayi".into()));
    entry(app, entry::Event::Group("hesap".into()));
}

/// Formla düzenle: the code another building has (an error), a short description (a warning, yellow).
fn calc_issues(app: &mut App) {
    calc_buildings(app);
    app.selection.set([Slot(2)]);
    run(app, "data.formEdit");
    entry(app, entry::Event::Value("Durum".into(), "inceleme".into()));
    entry(app, entry::Event::Value("kat-sayisi".into(), "3".into()));
    entry(
        app,
        entry::Event::Value("Açıklama".into(), "Çatı akıyor".into()),
    );
    entry(app, entry::Event::Save);
}

/// Formlar's preview (docs/adr/0403 §3) of the calculating form on a new building: storeys typed,
/// the calculations' group opened; the copy's change (a note's text) follows at once.
fn calc_preview(app: &mut App) {
    calc_buildings(app);
    run(app, "data.forms");
    designer(app, designer::Event::Preview(true));
    let _ = app.update(Message::FormPreview(entry::Event::Value(
        "kat-sayisi".into(),
        "7".into(),
    )));
    let _ = app.update(Message::FormPreview(entry::Event::Group("hesap".into())));
    designer(app, designer::Event::SelectItem("toplam".into()));
    designer(
        app,
        designer::Event::Text(
            designer::Text::Note,
            "Taban {Taban} m², toplam {Kat * Taban} m².".into(),
        ),
    );
}

/// Formlar: an item dragged in the tree (the description into the second tab), the history's buttons.
fn designing_moved(app: &mut App) {
    buildings(app, true);
    run(app, "data.forms");
    let order: Vec<String> = app
        .forms_window
        .as_ref()
        .and_then(|w| w.body.clone())
        .map(|b| (0..9).filter_map(|i| designer::id_at(&b, i)).collect())
        .unwrap_or_default();
    let at = |id: &str| order.iter().position(|x| x == id).unwrap_or(0);
    designer(
        app,
        designer::Event::Drop(
            at("aciklama"),
            at("ayrinti"),
            kentos_ui::widget::tree_view::Place::Into,
        ),
    );
}

/// Formla düzenle on three buildings at once (docs/adr/0403 §6): the status they differ in shows
/// “Çeşitli değerler”, the storeys typed go to all of them.
fn calc_many(app: &mut App) {
    calc_buildings(app);
    app.selection.set([Slot(1), Slot(2), Slot(3)]);
    run(app, "data.formEdit");
    entry(app, entry::Event::Value("kat-sayisi".into(), "5".into()));
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
        ("form-hesap-tasarim", calc_designing),
        ("form-hesap-duzenle", calc_editing),
        ("form-hesap-sorun", calc_issues),
        ("form-onizleme", calc_preview),
        ("form-tasarim-tasima", designing_moved),
        ("form-toplu", calc_many),
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
            Some(kentos_contracts::FormItem::Field(f)) => f.label.clone(),
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
