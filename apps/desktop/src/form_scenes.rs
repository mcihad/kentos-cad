//! The pictures of docs/adr/0401: Formlar with a form of three tabs' worth of items, an item's rules and
//! the preflight's issues, a new form being made; Formla düzenle on a building whose rules show the
//! note and ask for it, its issue after Kaydet, the second tab; Öznitelikler's Form row. `tools_screens`
//! takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=form-tasarim,form-tasarim-sorun,form-yeni,form-duzenle,form-duzenle-sorun,form-duzenle-sekme,form-oznitelikler cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! KENTOS_SHOTS_ONLY=form-hesap-tasarim,form-hesap-duzenle,form-hesap-sorun,form-onizleme,form-tasarim-tasima,form-toplu,form-adres,form-adres-tasarim,form-iliskiler,form-kayit-denetle,form-kaydirici,form-agac,form-agac-eksik,form-agac-tasarim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
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

use kentos_contracts::FormItem;
use kentos_domain::Slot;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::forms::{audit, designer, entry, relations};
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
pub(crate) fn form(published: bool) -> Value {
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

/// A file of the shared fixtures, by its path from the repository's root.
fn fixture(path: &str) -> String {
    format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR"))
}

/// The tree inventory (docs/adr/0407; `EXTRA_BODY` of `scripts/fixtures/forms_cases.py`): the name
/// offered from the layer, the colour picked, the kind listed, the height, the photos counted (one to
/// three) and the documents shown for a tall tree.
fn tree_form() -> Value {
    let body = json!({ "revision": 1, "items": [
        { "type": "field", "id": "ad", "field": "Ad", "suggest": true },
        { "type": "field", "id": "renk", "field": "Renk", "widget": "color", "default": "#2E7D32" },
        { "type": "field", "id": "cins", "field": "Cins" },
        { "type": "field", "id": "boy", "field": "Boy" },
        { "type": "field", "id": "bakim", "field": "Bakım" },
        { "type": "attachments", "id": "fotograflar", "label": "Fotoğraflar", "accept": "photo", "min": 1, "max": 3 },
        { "type": "attachments", "id": "belgeler", "label": "Belgeler", "accept": "document", "visibleWhen": "Boy > 10" }
    ]});
    let typed: kentos_contracts::FormBody = serde_json::from_value(body.clone()).expect("a body");
    json!({ "id": "agac", "name": "Ağaç envanteri", "layer": "agac", "draft": body.clone(),
            "releases": [{ "version": 1, "digest": kentos_contracts::body_digest(&typed), "body": body }] })
}

/// A park's trees under the inventory form: the first with two photos and a report, the second
/// without a photo, the others named for the suggestions.
fn trees(app: &mut App) {
    let mut o = Objects::new();
    let mut ids = Vec::new();
    for [x, y] in [
        [0.0, 0.0],
        [20.0, 4.0],
        [6.0, 22.0],
        [28.0, 24.0],
        [42.0, 8.0],
    ] {
        let ring: Vec<[f64; 2]> = (0..16)
            .map(|k| {
                let a = f64::from(k) * std::f64::consts::TAU / 16.0;
                [x + 6.0 + 5.0 * a.cos(), y + 6.0 + 5.0 * a.sin()]
            })
            .collect();
        ids.push(o.path("agac", &ring, true));
    }
    o.data(
        ids[0],
        &[
            ("Ad", "Çınar"),
            ("Renk", "#2E7D32"),
            ("Cins", "agac"),
            ("Boy", "14"),
            ("Bakım", "budama; sulama"),
        ],
        Some("Çınar"),
    );
    o.attach(
        ids[0],
        json!([
            { "name": "Gövde", "file": fixture("fixtures/exif/v1/konum.jpg") },
            { "name": "Taç", "file": fixture("fixtures/exif/v1/yarim-gps.jpg") },
            { "name": "Bakım raporu", "file": fixture("fixtures/interaction/v1/belgeler/tapu-101-5.pdf") }
        ]),
    );
    o.data(
        ids[1],
        &[("Ad", "Ihlamur"), ("Cins", "agac"), ("Boy", "6")],
        None,
    );
    o.data(ids[2], &[("Ad", "Çınar"), ("Boy", "11")], None);
    o.data(ids[3], &[("Ad", "Ihlamur")], None);
    o.data(ids[4], &[("Ad", "Kuşburnu"), ("Cins", "cali")], None);
    let fields = json!([
        { "name": "Ad", "kind": "text", "required": true },
        { "name": "Renk", "kind": "text" },
        { "name": "Cins", "kind": "text",
          "values": [{ "code": "agac", "label": "Ağaç" }, { "code": "cali", "label": "Çalı" }] },
        { "name": "Boy", "kind": "decimal", "scale": 1, "min": "0", "max": "40" },
        { "name": "Bakım", "kind": "text", "multiple": true,
          "values": [{ "code": "budama", "label": "Budama" }, { "code": "ilaclama", "label": "İlaçlama" },
                     { "code": "sulama", "label": "Sulama" }] }
    ]);
    let form = tree_form();
    open_edited(app, o, move |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut agac = crate::tools_scenes::layer("agac", "Ağaç", "#2F9E44", 0.35);
        agac["fields"] = fields;
        layers.push(agac);
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("agac");
        d["settings"]["forms"] = json!([form]);
    });
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 8.0,
            min_y: N - 8.0,
            max_x: E + 62.0,
            max_y: N + 42.0,
        },
        24.0,
    );
}

/// Formla düzenle on the first tree: its colour, its photos and report.
fn tree_editing(app: &mut App) {
    trees(app);
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
}

/// The second tree: no photo yet, and a name being typed: the layer's names offered.
fn tree_missing(app: &mut App) {
    trees(app);
    app.selection.set([Slot(2)]);
    run(app, "data.formEdit");
    entry(app, entry::Event::Value("Ad".into(), "ı".into()));
}

/// The name field focused: the layer's names holding what is typed are offered (docs/adr/0407 §3).
fn name_focused(s: &mut kentos_ui::snapshot::Snapshot, app: &mut App) {
    focused(s, app, "Ad");
}

/// The province field focused: the provinces holding “an” are offered (docs/adr/0431).
fn province_focused(s: &mut kentos_ui::snapshot::Snapshot, app: &mut App) {
    focused(s, app, "İl");
}

pub(crate) fn focused(s: &mut kentos_ui::snapshot::Snapshot, app: &mut App, key: &str) {
    use iced::futures::StreamExt as _;
    let task: iced::Task<Message> = iced::widget::operation::focus(entry::input_id(key));
    if let Some(mut stream) = iced_runtime::task::into_stream(task) {
        while let Some(action) = iced::futures::executor::block_on(stream.next()) {
            if let iced_runtime::Action::Widget(operation) = action {
                s.operate(app.view(), operation);
            }
        }
    }
}

/// The pictures that need the window: the suggestions open under a focused field.
pub(crate) fn pointed() -> Vec<crate::tools_screens::Pointed> {
    vec![
        ("form-agac-oneri", tree_missing, name_focused),
        ("form-aranir-liste", searched_list, province_focused),
        ("form-kaynak-arama", sources_search, neighbourhood_focused),
        ("form-kaynak-tasarim", sources_designer, properties_down),
    ]
}

fn neighbourhood_focused(s: &mut kentos_ui::snapshot::Snapshot, app: &mut App) {
    focused(s, app, "Mahalle");
}

/// The designer's properties scrolled to the list's extras (docs/adr/0439).
fn properties_down(s: &mut kentos_ui::snapshot::Snapshot, app: &mut App) {
    use iced::futures::StreamExt as _;
    let task: iced::Task<Message> = iced::widget::operation::snap_to(
        crate::forms::designer_view::PROPERTIES_SCROLL,
        iced::widget::scrollable::RelativeOffset { x: 0.0, y: 0.55 },
    );
    if let Some(mut stream) = iced_runtime::task::into_stream(task) {
        while let Some(action) = iced::futures::executor::block_on(stream.next()) {
            if let iced_runtime::Action::Widget(operation) = action {
                s.operate(app.view(), operation);
            }
        }
    }
}

/// A published form of `layer` with `items`.
fn published(id: &str, name: &str, layer: &str, items: Value) -> Value {
    let body = json!({ "revision": 1, "items": items });
    let typed: kentos_contracts::FormBody = serde_json::from_value(body.clone()).expect("a body");
    json!({ "id": id, "name": name, "layer": layer, "draft": body.clone(),
            "releases": [{ "version": 1, "digest": kentos_contracts::body_digest(&typed), "body": body }] })
}

/// Three buildings and two owners (docs/adr/0409): an owner holds several buildings by their ids
/// (a many-to-many field); a building's form lists its owners; the relation unlinks. The owners are
/// records without a geometry, a table's rows (docs/adr/0410).
fn owners(app: &mut App) {
    let mut o = Objects::new();
    let mut ids = Vec::new();
    for (x, code) in [(0.0, "B-1"), (24.0, "B-2"), (48.0, "B-3")] {
        let id = o.path(
            "bina",
            &[[x, 0.0], [x + 18.0, 0.0], [x + 18.0, 14.0], [x, 14.0]],
            true,
        );
        o.data(id, &[("Kod", code)], Some(code));
        ids.push(id);
    }
    let first = o.record("malik");
    o.data(first, &[("Ad", "Ayşe Yılmaz")], Some("Ayşe Yılmaz"));
    let second = o.record("malik");
    o.data(second, &[("Ad", "Mehmet Kaya")], Some("Mehmet Kaya"));
    let owner_form = published(
        "malik-formu",
        "Malik",
        "malik",
        json!([
            { "type": "field", "id": "ad", "field": "Ad", "required": true },
            { "type": "field", "id": "binalar", "field": "Binalar",
              "lookup": { "layer": "bina", "key": "$uid", "label": "Kod" } }
        ]),
    );
    let building_form = published(
        "bina-formu",
        "Bina",
        "bina",
        json!([
            { "type": "field", "id": "kod", "field": "Kod" },
            { "type": "related", "id": "malikler", "label": "Malikler", "layer": "malik", "field": "Binalar" }
        ]),
    );
    open_edited(app, o, move |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut bina = crate::tools_scenes::layer("bina", "Bina", "#3E63DD", 0.35);
        bina["fields"] = json!([{ "name": "Kod", "kind": "text" }]);
        let mut malik = crate::tools_scenes::layer("malik", "Malik", "#E5484D", 0.35);
        malik["fields"] = json!([{ "name": "Ad", "kind": "text" },
                                 { "name": "Binalar", "kind": "text", "multiple": true }]);
        layers.push(bina);
        layers.push(malik);
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("bina");
        d["settings"]["forms"] = json!([owner_form, building_form]);
        d["settings"]["relations"] = json!([{ "id": "bina-malik", "name": "Binanın malikleri",
                                              "parent": "bina", "child": "malik", "field": "Binalar" }]);
    });
    // The owners' buildings by their persistent ids, once the drawing has them.
    let uid = |app: &App, slot: u32| {
        app.document
            .as_ref()
            .and_then(|d| d.model.uid(Slot(slot)))
            .map(|u| u.to_string())
            .unwrap_or_default()
    };
    let (b1, b2) = (uid(app, ids[0]), uid(app, ids[1]));
    for (slot, held) in [(first, format!("{b1}; {b2}")), (second, b2.clone())] {
        let owner = uid(app, slot);
        if let Some(doc) = app.document.as_mut() {
            let _ = kentos_native_application::registry::run(
                &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
                kentos_contracts::CAD_ENTITIES_SET,
                kentos_contracts::CallOp::Execute,
                json!({ "uids": [owner], "attrs": { "Binalar": held }, "operation": "attributes" }),
            );
        }
    }
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 10.0,
            min_y: N - 10.0,
            max_x: E + 76.0,
            max_y: N + 34.0,
        },
        24.0,
    );
}

/// The first owner by its form: the buildings checked from the building layer.
fn owner_editing(app: &mut App) {
    owners(app);
    app.selection.set([Slot(4)]);
    run(app, "data.formEdit");
}

/// The second building by its form: its two owners, Yeni and Bağla.
fn building_owners(app: &mut App) {
    owners(app);
    app.selection.set([Slot(2)]);
    run(app, "data.formEdit");
}

/// A damage survey's moment and shift (docs/adr/0412): a date and time field and a time field, by
/// their pickers in Formla düzenle.
fn timed(app: &mut App) {
    let mut o = Objects::new();
    let id = o.path(
        "bina",
        &[[0.0, 0.0], [18.0, 0.0], [18.0, 14.0], [0.0, 14.0]],
        true,
    );
    o.data(
        id,
        &[
            ("Kod", "B-1"),
            ("Tespit", "2026-10-16T14:30:00"),
            ("Vardiya", "08:00:00"),
        ],
        Some("B-1"),
    );
    let form = published(
        "tespit",
        "Hasar tespiti",
        "bina",
        json!([
            { "type": "field", "id": "kod", "field": "Kod" },
            { "type": "field", "id": "tespit", "field": "Tespit", "required": true },
            { "type": "field", "id": "vardiya", "field": "Vardiya" }
        ]),
    );
    open_edited(app, o, move |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut bina = crate::tools_scenes::layer("bina", "Bina", "#3E63DD", 0.35);
        bina["fields"] = json!([{ "name": "Kod", "kind": "text" },
                                { "name": "Tespit", "kind": "datetime" },
                                { "name": "Vardiya", "kind": "time" }]);
        layers.push(bina);
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("bina");
        d["settings"]["forms"] = json!([form]);
    });
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
}

/// The buildings of the damage survey, their table painted by Koşullu biçim (docs/adr/0413): a ruined
/// building's row, a tall building's floor count bold, a note in blue.
fn styled_buildings(app: &mut App) {
    let mut o = Objects::new();
    let rows = [
        ("B-1", "iyi", "3", ""),
        ("B-2", "kotu", "7", "Taşıyıcı sistemde çatlak."),
        ("B-3", "orta", "2", ""),
        ("B-4", "iyi", "9", "Asansör denetlenmeli."),
        ("B-5", "kotu", "4", ""),
        ("B-6", "orta", "6", ""),
    ];
    for (i, (code, state, floors, note)) in rows.iter().enumerate() {
        let x = (i % 3) as f64 * 24.0;
        let y = (i / 3) as f64 * 20.0;
        let id = o.path(
            "bina",
            &[[x, y], [x + 18.0, y], [x + 18.0, y + 14.0], [x, y + 14.0]],
            true,
        );
        let mut attrs = vec![("Kod", *code), ("Durum", *state), ("Kat", *floors)];
        if !note.is_empty() {
            attrs.push(("Not", *note));
        }
        o.data(id, &attrs, Some(code));
    }
    open_edited(app, o, |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut bina = crate::tools_scenes::layer("bina", "Bina", "#3E63DD", 0.35);
        bina["fields"] = json!([{ "name": "Kod", "kind": "text" },
                                { "name": "Durum", "kind": "text", "values": [
                                    { "code": "iyi", "label": "İyi" }, { "code": "orta", "label": "Orta hasarlı" },
                                    { "code": "kotu", "label": "Ağır hasarlı" }] },
                                { "name": "Kat", "kind": "integer" },
                                { "name": "Not", "kind": "text" }]);
        bina["tableStyles"] = json!([
            { "condition": "Durum = 'kotu'", "background": "#FDE2E1", "color": "#9F1239" },
            { "condition": "Durum = 'orta'", "background": "#FEF3C7" },
            { "field": "Kat", "condition": "Kat >= 6", "bold": true },
            { "field": "Not", "condition": "[Not] <> ''", "color": "#1D4ED8" }
        ]);
        layers.push(bina);
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("bina");
    });
    app.features.layer = Some("bina".into());
    let _ = app.update(Message::Run("data.featureTable"));
}

/// The buildings with a published survey form that names the floor count in its rules (docs/adr/0414).
fn buildings_with_a_form(app: &mut App) {
    styled_buildings(app);
    let form = published(
        "hasar",
        "Hasar tespiti",
        "bina",
        json!([
            { "type": "field", "id": "durum", "field": "Durum", "visibleWhen": "Kat > 0" },
            { "type": "field", "id": "kat", "field": "Kat", "checks": [{ "rule": "Kat < 40", "message": "Kat 40'tan az olmalı." }] },
            { "type": "note", "id": "bilgi", "text": "Bu bina {Kat} katlı." }
        ]),
    );
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms = vec![serde_json::from_value(form).expect("a form")];
    doc.model.set_settings(settings);
}

/// Alanlar on the buildings: the floor count chosen and being renamed, its places listed.
fn field_usage(app: &mut App) {
    use crate::layer_fields::{Cell, Event};
    buildings_with_a_form(app);
    let _ = app.open_layer_fields("bina");
    let _ = app.update(Message::LayerFields(Event::Choose(2)));
    let _ = app.update(Message::LayerFields(Event::Type(
        2,
        Cell::Name,
        "Kat sayısı".into(),
    )));
}

/// The attribute table's Form view (docs/adr/0416): the buildings listed, the second one's form
/// beside them, its floor count typed and not yet saved.
fn form_view(app: &mut App) {
    use crate::features::{Event, Pane};
    buildings_with_a_form(app);
    let _ = app.update(Message::Features(Event::Pane(Pane::Form)));
    let _ = app.update(Message::Features(Event::FormRow(1)));
    let _ = app.update(Message::FormEntry(entry::Event::Value(
        "Kat".into(),
        "8".into(),
    )));
}

/// The buildings with “Orta hasarlı” retired from the state's list (docs/adr/0418): B-3 and B-6 keep it.
fn retired_state(app: &mut App) {
    buildings_with_a_form(app);
    let doc = app.document.as_mut().expect("a drawing");
    let mut fields = doc.model.layers().get("bina").expect("bina").fields.clone();
    for c in fields[1].values.iter_mut().flatten() {
        c.retired = c.code == "orta";
    }
    let _ = doc.model.set_layer_fields("bina", fields, &[]);
}

/// Alanlar's value list of the state: Orta hasarlı retired.
fn retired_list(app: &mut App) {
    retired_state(app);
    let _ = app.open_layer_fields("bina");
    let _ = app.update(Message::LayerFields(crate::layer_fields::Event::OpenList(
        1,
    )));
}

/// Buildings sorted by their kind (docs/adr/0422): Tür is the subtype field; an industrial building
/// is concrete or steel (steel by default) and has a note; a depot is concrete or stone. A published
/// form shows the four fields.
fn typed_buildings(app: &mut App) {
    let mut o = Objects::new();
    let rows = [
        ("B-1", "konut", "ahsap", "3", "", "kirma"),
        ("B-2", "sanayi", "beton", "2", "Kalıp atölyesi", "duz"),
        ("B-3", "depo", "tas", "1", "", "kirma"),
        ("B-4", "konut", "beton", "5", "", "teras"),
    ];
    for (i, (code, kind, material, floors, note, roof)) in rows.iter().enumerate() {
        let x = (i % 2) as f64 * 24.0;
        let y = (i / 2) as f64 * 20.0;
        let id = o.path(
            "bina",
            &[[x, y], [x + 18.0, y], [x + 18.0, y + 14.0], [x, y + 14.0]],
            true,
        );
        let mut attrs = vec![
            ("Kod", *code),
            ("Tür", *kind),
            ("Malzeme", *material),
            ("Kat", *floors),
            ("Çatı", *roof),
        ];
        if !note.is_empty() {
            attrs.push(("Not", *note));
        }
        o.data(id, &attrs, Some(code));
    }
    open_edited(app, o, |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut bina = crate::tools_scenes::layer("bina", "Bina", "#3E63DD", 0.35);
        bina["fields"] = json!([{ "name": "Kod", "kind": "text" },
                                { "name": "Tür", "kind": "text", "values": [
                                    { "code": "konut", "label": "Konut" }, { "code": "sanayi", "label": "Sanayi" },
                                    { "code": "depo", "label": "Depo" }] },
                                { "name": "Malzeme", "kind": "text", "default": "ahsap", "values": [
                                    { "code": "ahsap", "label": "Ahşap" }, { "code": "beton", "label": "Betonarme" },
                                    { "code": "celik", "label": "Çelik" }, { "code": "tas", "label": "Taş" }] },
                                { "name": "Kat", "kind": "integer" },
                                { "name": "Not", "kind": "text" },
                                { "name": "Çatı", "kind": "text", "values": [
                                    { "code": "duz", "label": "Düz" }, { "code": "kirma", "label": "Kırma" },
                                    { "code": "teras", "label": "Teras" }] }]);
        bina["contingents"] = json!([{ "name": "Malzeme ve çatı", "fields": ["Malzeme", "Çatı"], "restrictive": true, "rows": [
            { "cells": [{ "kind": "code", "code": "ahsap" }, { "kind": "code", "code": "kirma" }] },
            { "cells": [{ "kind": "code", "code": "beton" }, { "kind": "any" }] },
            { "cells": [{ "kind": "code", "code": "celik" }, { "kind": "code", "code": "duz" }] },
            { "cells": [{ "kind": "code", "code": "celik" }, { "kind": "code", "code": "teras" }], "subtype": "sanayi" },
            { "cells": [{ "kind": "code", "code": "tas" }, { "kind": "code", "code": "kirma" }] }
        ] }]);
        bina["subtypes"] = json!({ "field": "Tür", "types": [
            { "code": "konut" },
            { "code": "sanayi", "label": "Sanayi binası", "fields": [
                { "field": "Malzeme", "values": ["beton", "celik"], "default": "celik", "required": true },
                { "field": "Not", "required": true }] },
            { "code": "depo", "fields": [{ "field": "Malzeme", "values": ["beton", "tas"] }] }
        ] });
        layers.push(bina);
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("bina");
    });
    let form = published(
        "yapi",
        "Yapı kaydı",
        "bina",
        json!([
            { "type": "field", "id": "tur", "field": "Tür" },
            { "type": "field", "id": "malzeme", "field": "Malzeme" },
            { "type": "field", "id": "kat", "field": "Kat" },
            { "type": "field", "id": "cati", "field": "Çatı" },
            { "type": "field", "id": "not", "field": "Not" }
        ]),
    );
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms = vec![serde_json::from_value(form).expect("a form")];
    doc.model.set_settings(settings);
    app.features.layer = Some("bina".into());
}

/// A building's units as a table (docs/adr/0425): number, floor and area of each, the areas totalled;
/// the units are records without a geometry linked to the building by its persistent id.
fn units_table(app: &mut App) {
    let mut o = Objects::new();
    let building = o.path(
        "bina",
        &[[0.0, 0.0], [24.0, 0.0], [24.0, 16.0], [0.0, 16.0]],
        true,
    );
    o.data(building, &[("Kod", "B-1")], Some("B-1"));
    let rows = [
        ("1", "0", "92.40", "Ayşe Yılmaz"),
        ("2", "1", "88.15", "Mehmet Kaya"),
        ("3", "1", "88.15", "Zeynep Arslan"),
        ("4", "2", "120.75", "Ali Demir"),
    ];
    let mut units = Vec::new();
    for (no, floor, area, owner) in rows {
        let id = o.record("bolum");
        o.data(
            id,
            &[("No", no), ("Kat", floor), ("Alan", area), ("Malik", owner)],
            Some(no),
        );
        units.push(id);
    }
    let form = published(
        "bina-formu",
        "Bina",
        "bina",
        json!([
            { "type": "field", "id": "kod", "field": "Kod" },
            { "type": "related", "id": "bolumler", "label": "Bağımsız bölümler", "layer": "bolum", "field": "Bina",
              "view": "table", "columns": ["No", "Kat", "Alan", "Malik"] }
        ]),
    );
    open_edited(app, o, move |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut bina = crate::tools_scenes::layer("bina", "Bina", "#3E63DD", 0.35);
        bina["fields"] = json!([{ "name": "Kod", "kind": "text" }]);
        let mut bolum = crate::tools_scenes::layer("bolum", "Bağımsız bölüm", "#E5484D", 0.35);
        bolum["fields"] = json!([{ "name": "No", "kind": "text" }, { "name": "Kat", "kind": "integer" },
                                 { "name": "Alan", "kind": "decimal", "scale": 2 }, { "name": "Malik", "kind": "text" },
                                 { "name": "Bina", "kind": "text" }]);
        layers.push(bina);
        layers.push(bolum);
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("bina");
        d["settings"]["forms"] = json!([form]);
        d["settings"]["relations"] = json!([{ "id": "bina-bolum", "name": "Bağımsız bölümler",
                                              "parent": "bina", "child": "bolum", "field": "Bina" }]);
    });
    let uid = |app: &App, slot: u32| {
        app.document
            .as_ref()
            .and_then(|d| d.model.uid(Slot(slot)))
            .map(|u| u.to_string())
            .unwrap_or_default()
    };
    let parent = uid(app, building);
    let children: Vec<String> = units.iter().map(|s| uid(app, *s)).collect();
    if let Some(doc) = app.document.as_mut() {
        let _ = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            kentos_contracts::CAD_ENTITIES_SET,
            kentos_contracts::CallOp::Execute,
            json!({ "uids": children, "attrs": { "Bina": parent }, "operation": "attributes" }),
        );
    }
    app.selection.set([Slot(building)]);
    run(app, "data.formEdit");
}

/// Formlar on the building form, its units item chosen: Görünüm Tablo and its columns (docs/adr/0425).
fn units_designer(app: &mut App) {
    units_table(app);
    let _ = app.update(Message::FormEntry(entry::Event::Cancel));
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("bolumler".into()));
}

/// Formlar on the building form in tabs, its Sanayi tab chosen and shown only for industrial and
/// depot buildings by Alt türlerde göster (docs/adr/0426).
fn kind_tabs(app: &mut App) {
    typed_buildings(app);
    let form = published(
        "yapi",
        "Yapı kaydı",
        "bina",
        json!([
            { "type": "tab", "id": "genel", "label": "Genel", "items": [
                { "type": "field", "id": "tur", "field": "Tür" },
                { "type": "field", "id": "malzeme", "field": "Malzeme" },
                { "type": "field", "id": "cati", "field": "Çatı" } ] },
            { "type": "tab", "id": "sanayi", "label": "Sanayi", "items": [
                { "type": "field", "id": "kat", "field": "Kat" },
                { "type": "field", "id": "not", "field": "Not" } ] }
        ]),
    );
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms = vec![serde_json::from_value(form).expect("a form")];
    doc.model.set_settings(settings);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("sanayi".into()));
    designer(app, designer::Event::ShowInKind("sanayi".into()));
    designer(app, designer::Event::ShowInKind("depo".into()));
}

/// Formlar's Değişiklikler (docs/adr/0427): the survey form's draft against its release after the
/// floor count became required, a group was added and the note removed.
fn form_changes(app: &mut App) {
    buildings_with_a_form(app);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("kat".into()));
    designer(app, designer::Event::Required(true));
    designer(app, designer::Event::SelectItem("bilgi".into()));
    designer(app, designer::Event::RemoveItem);
    designer(app, designer::Event::AddGroup);
    designer(app, designer::Event::Changes);
}

/// Buildings through an inspection workflow (docs/adr/0428): drafted, sent, approved (locked) and
/// returned with a reason; the published form's steps Gönder, Onayla, Geri gönder, Düzelt.
fn workflow_buildings(app: &mut App) {
    let mut o = Objects::new();
    let rows = [
        ("B-1", "taslak", "3"),
        ("B-2", "gonderildi", "5"),
        ("B-3", "onaylandi", "2"),
        ("B-4", "iade", "4"),
    ];
    for (i, (code, state, floors)) in rows.iter().enumerate() {
        let x = (i % 2) as f64 * 24.0;
        let y = (i / 2) as f64 * 20.0;
        let id = o.path(
            "bina",
            &[[x, y], [x + 18.0, y], [x + 18.0, y + 14.0], [x, y + 14.0]],
            true,
        );
        let mut attrs = vec![("Kod", *code), ("Durum", *state), ("Kat", *floors)];
        if *state == "iade" {
            attrs.push((
                "Geçmiş",
                "2026-10-15T09:20:00 · Ayşe Yılmaz · Gönder (Taslak → Gönderildi)\n\
                 2026-10-16T16:05:00 · Mehmet Kaya · Geri gönder (Gönderildi → İade edildi): Kat sayısı yanlış.",
            ));
            attrs.push(("Gerekçe", "Kat sayısı yanlış."));
        }
        o.data(id, &attrs, Some(code));
    }
    open_edited(app, o, |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut bina = crate::tools_scenes::layer("bina", "Bina", "#3E63DD", 0.35);
        bina["fields"] = json!([{ "name": "Kod", "kind": "text" },
                                { "name": "Durum", "kind": "text", "values": [
                                    { "code": "taslak", "label": "Taslak" }, { "code": "gonderildi", "label": "Gönderildi" },
                                    { "code": "onaylandi", "label": "Onaylandı" }, { "code": "iade", "label": "İade edildi" }] },
                                { "name": "Kat", "kind": "integer" },
                                { "name": "Gerekçe", "kind": "text" },
                                { "name": "Geçmiş", "kind": "text" }]);
        layers.push(bina);
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("bina");
    });
    let body = json!({ "revision": 1, "items": [
            { "type": "field", "id": "kod", "field": "Kod" },
            { "type": "field", "id": "durum", "field": "Durum" },
            { "type": "field", "id": "kat", "field": "Kat", "required": true },
            { "type": "field", "id": "gerekce", "field": "Gerekçe" },
            { "type": "field", "id": "gecmis", "field": "Geçmiş", "widget": "textArea" }
        ],
        "workflow": { "field": "Durum", "initial": "taslak", "locked": ["onaylandi"], "drafts": ["taslak", "iade"], "log": "Geçmiş", "steps": [
            { "label": "Gönder", "from": ["taslak", "iade"], "to": "gonderildi", "valid": true },
            { "label": "Onayla", "from": ["gonderildi"], "to": "onaylandi", "valid": true },
            { "label": "Geri gönder", "from": ["gonderildi"], "to": "iade", "reason": "Gerekçe" },
            { "label": "Düzelt", "from": ["onaylandi"], "to": "iade", "reason": "Gerekçe" }
        ] } });
    let typed: kentos_contracts::FormBody = serde_json::from_value(body.clone()).expect("a body");
    let form = json!({ "id": "denetim", "name": "Bina denetimi", "layer": "bina", "draft": body.clone(),
        "releases": [{ "version": 1, "digest": kentos_contracts::body_digest(&typed), "body": body }] });
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms = vec![serde_json::from_value(form).expect("a form")];
    doc.model.set_settings(settings);
    app.features.layer = Some("bina".into());
}

/// Formlar's İş akışı on the inspection form (docs/adr/0428 §4).
fn workflow_designer(app: &mut App) {
    workflow_buildings(app);
    run(app, "data.forms");
    designer(app, designer::Event::Flow);
}

/// A sent building's form: its steps Onayla and Geri gönder beside Kaydet.
fn workflow_steps(app: &mut App) {
    workflow_buildings(app);
    app.selection.set([Slot(2)]);
    run(app, "data.formEdit");
}

/// An approved building's form: read-only, Düzelt its only step (docs/adr/0428 §2).
fn workflow_locked(app: &mut App) {
    workflow_buildings(app);
    app.selection.set([Slot(3)]);
    run(app, "data.formEdit");
}

/// A returned building's form: its log of steps, read-only, and Gönder again (docs/adr/0429).
fn workflow_log(app: &mut App) {
    workflow_buildings(app);
    app.selection.set([Slot(4)]);
    run(app, "data.formEdit");
}

/// Buildings and depots whose material comes from the project's list “Yapı malzemesi” and whose floor
/// count from the range “Kat sayısı” (docs/adr/0430); a depot is of masonry (ym).
fn listed_buildings(app: &mut App) {
    let mut o = Objects::new();
    let rows = [
        ("bina", "B-1", "ah", "3"),
        ("bina", "B-2", "bt", "7"),
        ("depo", "D-1", "ym", ""),
        ("depo", "D-2", "bt", ""),
    ];
    for (i, (layer, code, material, floors)) in rows.iter().enumerate() {
        let x = (i % 2) as f64 * 24.0;
        let y = (i / 2) as f64 * 20.0;
        let id = o.path(
            layer,
            &[[x, y], [x + 18.0, y], [x + 18.0, y + 14.0], [x, y + 14.0]],
            true,
        );
        let mut attrs = vec![("Kod", *code), ("Malzeme", *material)];
        if !floors.is_empty() {
            attrs.push(("Kat", *floors));
        }
        o.data(id, &attrs, Some(code));
    }
    let materials = json!([{ "code": "ah", "label": "Ahşap" }, { "code": "bt", "label": "Betonarme" },
                           { "code": "ym", "label": "Yığma" }, { "code": "ce", "label": "Çelik" }]);
    let fields_of = |floors: bool| {
        let mut f = vec![
            json!({ "name": "Kod", "kind": "text" }),
            json!({ "name": "Malzeme", "kind": "text", "values": materials.clone(), "domain": "malzeme" }),
        ];
        if floors {
            f.push(json!({ "name": "Kat", "kind": "integer", "min": "1", "max": "40", "domain": "kat" }));
        }
        Value::Array(f)
    };
    open_edited(app, o, |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut bina = crate::tools_scenes::layer("bina", "Bina", "#3E63DD", 0.35);
        bina["fields"] = fields_of(true);
        let mut depo = crate::tools_scenes::layer("depo", "Depo", "#E54D2E", 0.35);
        depo["fields"] = fields_of(false);
        layers.push(bina);
        layers.push(depo);
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("bina");
        d["settings"]["domains"] = json!([
            { "id": "malzeme", "name": "Yapı malzemesi", "description": "Bütün yapı katmanlarının malzemesi.", "kind": "text",
              "values": materials.clone() },
            { "id": "kat", "name": "Kat sayısı", "kind": "integer", "min": "1", "max": "40" },
            { "id": "cati", "name": "Çatı türü", "kind": "text",
              "values": [{ "code": "dz", "label": "Düz" }, { "code": "bs", "label": "Beşik" }, { "code": "kr", "label": "Kırma" }] }
        ]);
    });
    app.features.layer = Some("bina".into());
}

/// Değer listeleri on the material list (docs/adr/0430 §4): Yığma dropped, its records moved to Betonarme.
fn domains_window(app: &mut App) {
    use crate::forms::domains::Event;
    listed_buildings(app);
    run(app, "data.domains");
    let _ = app.update(Message::Domains(Event::RemoveCode(2)));
    let _ = app.update(Message::Domains(Event::Recode(
        "ym".into(),
        Some("bt".into()),
    )));
}

/// Alanlar's value list of the building's material: linked to the project's list, its codes read-only.
fn domains_field(app: &mut App) {
    listed_buildings(app);
    let _ = app.open_layer_fields("bina");
    let _ = app.update(Message::LayerFields(crate::layer_fields::Event::OpenList(
        1,
    )));
}

/// Facilities with a province from a long list and the services they have (docs/adr/0431): the province
/// searched, the services as chips; a published form.
fn chosen_facilities(app: &mut App) {
    let mut o = Objects::new();
    let id = o.path(
        "tesis",
        &[[0.0, 0.0], [18.0, 0.0], [18.0, 14.0], [0.0, 14.0]],
        true,
    );
    o.data(
        id,
        &[
            ("Ad", "Merkez arıtma"),
            ("İl", "06"),
            ("Hizmetler", "su; elektrik"),
        ],
        Some("Merkez arıtma"),
    );
    let provinces = [
        ("01", "Adana"),
        ("06", "Ankara"),
        ("07", "Antalya"),
        ("75", "Ardahan"),
        ("08", "Artvin"),
        ("09", "Aydın"),
        ("10", "Balıkesir"),
        ("16", "Bursa"),
        ("20", "Denizli"),
        ("21", "Diyarbakır"),
        ("25", "Erzurum"),
        ("26", "Eskişehir"),
        ("27", "Gaziantep"),
        ("31", "Hatay"),
        ("34", "İstanbul"),
        ("35", "İzmir"),
        ("38", "Kayseri"),
        ("41", "Kocaeli"),
        ("42", "Konya"),
        ("55", "Samsun"),
        ("58", "Sivas"),
        ("61", "Trabzon"),
        ("65", "Van"),
        ("66", "Yozgat"),
    ];
    let values: Vec<Value> = provinces
        .iter()
        .map(|(c, l)| json!({ "code": c, "label": l }))
        .collect();
    open_edited(app, o, |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut tesis = crate::tools_scenes::layer("tesis", "Tesis", "#30A46C", 0.35);
        tesis["fields"] = json!([{ "name": "Ad", "kind": "text" },
                                 { "name": "İl", "kind": "text", "values": values },
                                 { "name": "Hizmetler", "kind": "text", "multiple": true, "values": [
                                     { "code": "su", "label": "Su" }, { "code": "elektrik", "label": "Elektrik" },
                                     { "code": "dogalgaz", "label": "Doğal gaz" }, { "code": "internet", "label": "İnternet" },
                                     { "code": "kanal", "label": "Kanalizasyon" }] }]);
        layers.push(tesis);
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("tesis");
    });
    let body = json!({ "revision": 1, "items": [
        { "type": "field", "id": "ad", "field": "Ad" },
        { "type": "field", "id": "il", "field": "İl", "widget": "search", "placeholder": "İl adı yazın" },
        { "type": "field", "id": "hizmet", "field": "Hizmetler", "widget": "chips" }
    ] });
    let typed: kentos_contracts::FormBody = serde_json::from_value(body.clone()).expect("a body");
    let form = json!({ "id": "tesis", "name": "Tesis kaydı", "layer": "tesis", "draft": body.clone(),
        "releases": [{ "version": 1, "digest": kentos_contracts::body_digest(&typed), "body": body }] });
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms = vec![serde_json::from_value(form).expect("a form")];
    doc.model.set_settings(settings);
    app.features.layer = Some("tesis".into());
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
}

/// Parcels and buildings (docs/adr/0439): a building's parcel chosen from the parcels holding it, their
/// area, owner and day shown beside it and under it, copied into the building with a stamp; the
/// neighbourhoods searched a page at a time; the project's value lists.
fn sources(app: &mut App) {
    let mut o = Objects::new();
    // Ada, parsel, the square, area, owner and day.
    type Parcel<'a> = (&'a str, &'a str, [f64; 4], &'a str, &'a str, &'a str);
    let parcels: [Parcel<'_>; 4] = [
        (
            "101",
            "5",
            [0.0, 0.0, 30.0, 24.0],
            "450.50",
            "Ayşe Kaya",
            "2024-03-01",
        ),
        (
            "101",
            "12",
            [30.0, 0.0, 60.0, 24.0],
            "1200.00",
            "Ali Can",
            "2023-11-20",
        ),
        ("102", "1", [0.0, 24.0, 30.0, 48.0], "80.25", "", ""),
        (
            "102",
            "4",
            [30.0, 24.0, 60.0, 48.0],
            "640.00",
            "Veli Can",
            "2025-06-12",
        ),
    ];
    for (ada, parsel, [x0, y0, x1, y1], alan, malik, tarih) in parcels {
        let id = o.path("parsel", &[[x0, y0], [x1, y0], [x1, y1], [x0, y1]], true);
        let no = format!("{ada}/{parsel}");
        let mut attrs = vec![
            ("ada", ada),
            ("parsel", parsel),
            ("no", no.as_str()),
            ("alan", alan),
        ];
        if !malik.is_empty() {
            attrs.push(("malik", malik));
        }
        if !tarih.is_empty() {
            attrs.push(("tarih", tarih));
        }
        o.data(id, &attrs, Some(no.as_str()));
    }
    let b1 = o.path(
        "bina",
        &[[6.0, 5.0], [20.0, 5.0], [20.0, 17.0], [6.0, 17.0]],
        true,
    );
    o.data(b1, &[("Kat", "3")], None);
    let b2 = o.path(
        "bina",
        &[[38.0, 30.0], [52.0, 30.0], [52.0, 42.0], [38.0, 42.0]],
        true,
    );
    o.data(b2, &[("Kat", "5"), ("Mahalle", "M07")], None);
    // Forty neighbourhoods, records without geometry: a list searched a page at a time.
    let names = [
        "Akpınar",
        "Aşağı Ayrancı",
        "Bahçelievler",
        "Balgat",
        "Beşevler",
        "Birlik",
        "Cebeci",
        "Çankaya",
        "Çayyolu",
        "Demirtepe",
        "Dikmen",
        "Emek",
        "Esat",
        "Etlik",
        "Gaziosmanpaşa",
        "Göztepe",
        "Hilal",
        "İncesu",
        "İlkadım",
        "Kavaklıdere",
        "Keçiören",
        "Kızılay",
        "Kocatepe",
        "Kurtuluş",
        "Maltepe",
        "Mebusevleri",
        "Mustafa Kemal",
        "Oran",
        "Ostim",
        "Öveçler",
        "Remzi Oğuz Arık",
        "Sağlık",
        "Sıhhiye",
        "Söğütözü",
        "Tandoğan",
        "Ümitköy",
        "Yenimahalle",
        "Yukarı Ayrancı",
        "Yüzüncüyıl",
        "Zafertepe",
    ];
    for (i, name) in names.iter().enumerate() {
        let id = o.record("mahalle");
        let code = format!("M{:02}", i + 1);
        let ilce = if i % 3 == 0 {
            "Yenimahalle"
        } else {
            "Çankaya"
        };
        o.data(
            id,
            &[("kod", code.as_str()), ("ad", name), ("ilce", ilce)],
            None,
        );
    }
    open_edited(app, o, |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut parsel = crate::tools_scenes::layer("parsel", "Parsel", "#B45309", 0.35);
        parsel["fields"] = json!([
            { "name": "ada", "alias": "Ada", "kind": "integer" },
            { "name": "parsel", "alias": "Parsel", "kind": "integer" },
            { "name": "no", "alias": "Ada/Parsel", "kind": "text" },
            { "name": "alan", "alias": "Yüzölçümü", "kind": "decimal", "scale": 2, "unit": "m²" },
            { "name": "malik", "alias": "Malik", "kind": "text" },
            { "name": "tarih", "alias": "Tescil", "kind": "date" }]);
        let mut bina = crate::tools_scenes::layer("bina", "Bina", "#0E7490", 0.35);
        bina["fields"] = json!([
            { "name": "Parsel", "kind": "text" },
            { "name": "Ada", "kind": "integer" },
            { "name": "Alan", "alias": "Parsel alanı", "kind": "decimal", "scale": 2, "unit": "m²" },
            { "name": "Malik", "kind": "text" },
            { "name": "Kaynak", "kind": "text" },
            { "name": "Kat", "kind": "integer" },
            { "name": "Mahalle", "kind": "text" }]);
        let mut mahalle = crate::tools_scenes::layer("mahalle", "Mahalle", "#6D28D9", 0.25);
        mahalle["fields"] = json!([
            { "name": "kod", "kind": "text" }, { "name": "ad", "kind": "text" }, { "name": "ilce", "alias": "İlçe", "kind": "text" }]);
        layers.push(parsel);
        layers.push(bina);
        layers.push(mahalle);
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("bina");
    });
    let body = json!({ "revision": 1, "items": [
        { "type": "field", "id": "parsel", "field": "Parsel", "lookup": {
            "layer": "parsel", "key": "$uid", "label": "no", "columns": ["alan", "malik", "tarih"],
            "sort": { "field": "alan", "descending": true },
            "copy": [{ "from": "ada", "to": "Ada" }, { "from": "alan", "to": "Alan" }, { "from": "malik", "to": "Malik" }],
            "stamp": "Kaynak", "duplicates": "refuse", "spatial": { "relation": "contains" } } },
        { "type": "field", "id": "kat", "field": "Kat" },
        { "type": "field", "id": "mahalle", "field": "Mahalle", "widget": "search", "minSearch": 2, "lookup": {
            "layer": "mahalle", "key": "kod", "label": "ad", "columns": ["ilce"], "limit": 12 } },
        { "type": "field", "id": "ada", "field": "Ada", "readOnly": true },
        { "type": "field", "id": "alan", "field": "Alan", "readOnly": true },
        { "type": "field", "id": "malik", "field": "Malik", "readOnly": true },
        { "type": "field", "id": "kaynak", "field": "Kaynak", "readOnly": true }
    ] });
    let typed: kentos_contracts::FormBody = serde_json::from_value(body.clone()).expect("a body");
    let form = json!({ "id": "bina", "name": "Bina", "layer": "bina", "draft": body.clone(),
        "releases": [{ "version": 1, "digest": kentos_contracts::body_digest(&typed), "body": body }] });
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms = vec![serde_json::from_value(form).expect("a form")];
    settings.domains = serde_json::from_value(json!([
        { "id": "cati", "name": "Çatı türü", "kind": "text",
          "values": [{ "code": "dz", "label": "Düz" }, { "code": "bh", "label": "Beşik" }, { "code": "kr", "label": "Kırma" }] },
        { "id": "kat", "name": "Kat sayısı", "kind": "integer", "min": "1", "max": "40" }]))
    .expect("lists");
    settings.variables = serde_json::from_value(json!([])).expect("variables");
    doc.model.set_settings(settings);
    app.features.layer = Some("bina".into());
}

/// Formlar on the building form, its parcel item chosen: the list's order, columns, page, duplicates,
/// geometry, copies and stamp.
fn sources_designer(app: &mut App) {
    sources(app);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("parsel".into()));
}

/// Formla düzenle on the first building: the parcels holding it offered with their columns, 101/5
/// chosen, its area, owner and day under it, copied into the building with the stamp.
fn sources_entry(app: &mut App) {
    sources(app);
    app.selection.set([Slot(5)]);
    run(app, "data.formEdit");
    entry(app, entry::Event::Value("Parsel".into(), "101/5".into()));
}

/// The neighbourhoods searched: “ka” typed, a page of suggestions with their district, Daha fazla.
fn sources_search(app: &mut App) {
    sources(app);
    app.selection.set([Slot(6)]);
    run(app, "data.formEdit");
    searched(app, "Mahalle", "a");
    let _ = app.update(Message::FormEntry(entry::Event::More("Mahalle".into())));
    searched(app, "Mahalle", "ka");
}

/// Değer listeleri with İçe aktar…, Dışa aktar… and Aynı kimlikli listeleri değiştir.
fn sources_domains(app: &mut App) {
    sources(app);
    run(app, "data.domains");
}

/// Alanlar on the parcels after Değerleri biçime çevir: values typed before the fields were (an
/// import's 450,5, 01.03.2024) in their canonical texts, what did not fit said.
fn sources_migrated(app: &mut App) {
    sources(app);
    let doc = app.document.as_mut().expect("a drawing");
    for (slot, attrs) in [
        (1, [("alan", "450,5"), ("tarih", "01.03.2024")]),
        (2, [("alan", "1.200"), ("tarih", "20.11.2023")]),
        (3, [("alan", "seksen"), ("tarih", "2024-02-30")]),
    ] {
        if let Some(mut e) = doc.model.get(Slot(slot)).cloned() {
            for (k, v) in attrs {
                e.base_mut().attrs.insert(k.into(), v.into());
            }
            doc.model
                .update_many(vec![(Slot(slot), e)], "Eski değerler");
        }
    }
    let _ = app.open_layer_fields("parsel");
    let _ = app.update(Message::LayerFields(crate::layer_fields::Event::Migrate));
}

/// Buildings and their meters (docs/adr/0440): a project relation, sixty meters of the first building
/// shown as cards a page at a time, the meters' own form, the building's workflow logging its snapshot.
fn meters(app: &mut App) {
    let mut o = Objects::new();
    let a = o.path(
        "bina",
        &[[0.0, 0.0], [20.0, 0.0], [20.0, 14.0], [0.0, 14.0]],
        true,
    );
    o.data(a, &[("Ad", "A blok"), ("Durum", "taslak")], Some("A blok"));
    let b = o.path(
        "bina",
        &[[30.0, 0.0], [44.0, 0.0], [44.0, 12.0], [30.0, 12.0]],
        true,
    );
    o.data(b, &[("Ad", "B blok"), ("Durum", "taslak")], Some("B blok"));
    for i in 1..=62 {
        let id = o.record("sayac");
        let no = format!("S-{i:03}");
        let kind = if i % 3 == 0 { "elektrik" } else { "su" };
        o.data(id, &[("no", no.as_str()), ("tur", kind)], None);
    }
    open_edited(app, o, |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut bina = crate::tools_scenes::layer("bina", "Bina", "#0E7490", 0.35);
        bina["fields"] = json!([
            { "name": "Ad", "kind": "text" },
            { "name": "Durum", "kind": "text", "values": [
                { "code": "taslak", "label": "Taslak" }, { "code": "onay", "label": "Onaylandı" }] },
            { "name": "Geçmiş", "kind": "text", "length": 2000 }]);
        let mut sayac = crate::tools_scenes::layer("sayac", "Sayaç", "#B45309", 0.25);
        sayac["fields"] = json!([
            { "name": "no", "alias": "Numara", "kind": "text" },
            { "name": "tur", "alias": "Tür", "kind": "text" },
            { "name": "bina", "kind": "text" }]);
        layers.push(bina);
        layers.push(sayac);
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("bina");
    });
    let bina = json!({ "revision": 1, "items": [
        { "type": "field", "id": "ad", "field": "Ad" },
        { "type": "related", "id": "sayaclar", "label": "Sayaçlar", "layer": "sayac", "field": "bina",
          "view": "cards", "columns": ["no", "tur"] }
    ], "workflow": { "field": "Durum", "initial": "taslak", "log": "Geçmiş",
        "steps": [{ "label": "Onayla", "from": ["taslak"], "to": "onay" },
                  { "label": "Geri al", "from": ["onay"], "to": "taslak" }] } });
    let sayac = json!({ "revision": 1, "items": [
        { "type": "field", "id": "no", "field": "no", "required": true },
        { "type": "field", "id": "tur", "field": "tur" } ] });
    let form = |id: &str, name: &str, layer: &str, body: &Value| {
        let typed: kentos_contracts::FormBody =
            serde_json::from_value(body.clone()).expect("a body");
        json!({ "id": id, "name": name, "layer": layer, "draft": body.clone(),
            "releases": [{ "version": 1, "digest": kentos_contracts::body_digest(&typed), "body": body.clone() }] })
    };
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms = vec![
        serde_json::from_value(form("bina", "Bina", "bina", &bina)).expect("a form"),
        serde_json::from_value(form("sayac", "Sayaç", "sayac", &sayac)).expect("a form"),
    ];
    settings.relations = vec![serde_json::from_value(json!({
        "id": "bina-sayac", "name": "Binanın sayaçları", "parent": "bina", "child": "sayac", "field": "bina",
        "onDelete": "cascade" })).expect("a relation")];
    doc.model.set_settings(settings);
    // The first sixty meters are the first building's.
    let uid = doc
        .model
        .uid(Slot(1))
        .map(|u| u.to_string())
        .unwrap_or_default();
    let uids: Vec<String> = (3..=62)
        .filter_map(|s| doc.model.uid(Slot(s)).map(|u| u.to_string()))
        .collect();
    let _ = kentos_native_application::set::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        serde_json::from_value(
            json!({ "uids": uids, "attrs": { "bina": uid }, "operation": "attributes" }),
        )
        .expect("an input"),
    );
    app.features.layer = Some("bina".into());
}

/// Formla düzenle on the first building: its meters as cards, 1–50 of 60.
fn meters_cards(app: &mut App) {
    meters(app);
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
}

/// The second building: a free meter linked and the name typed, both waiting for Kaydet.
fn meters_pending(app: &mut App) {
    meters(app);
    app.selection.set([Slot(2)]);
    run(app, "data.formEdit");
    let free = app
        .document
        .as_ref()
        .and_then(|d| d.model.uid(Slot(63)))
        .map(|u| u.to_string())
        .unwrap_or_default();
    entry(
        app,
        entry::Event::Value("Ad".into(), "B blok (yeni)".into()),
    );
    entry(app, entry::Event::Link("sayaclar".into(), free));
}

/// A meter's form opened over its building's, the building's typed name waiting under it.
fn meters_nested(app: &mut App) {
    meters(app);
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
    entry(
        app,
        entry::Event::Value("Ad".into(), "A blok (yeni)".into()),
    );
    let child = app
        .document
        .as_ref()
        .and_then(|d| d.model.uid(Slot(3)))
        .map(|u| u.to_string())
        .unwrap_or_default();
    entry(app, entry::Event::OpenRelated(child));
}

/// Formlar on the building form, its meters item chosen: İlişki, Görünüm Kartlar, the columns; Başvuru.
fn meters_designer(app: &mut App) {
    meters(app);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("sayaclar".into()));
}

/// The first building approved, then one of its meters changed: the step's snapshot is not the record's.
fn meters_snapshot(app: &mut App) {
    meters(app);
    let doc = app.document.as_mut().expect("a drawing");
    let uid = doc
        .model
        .uid(Slot(1))
        .map(|u| u.to_string())
        .unwrap_or_default();
    let meter = doc
        .model
        .uid(Slot(3))
        .map(|u| u.to_string())
        .unwrap_or_default();
    let _ = kentos_native_application::forms_transition::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        serde_json::from_value(json!({ "form": "bina", "uid": uid, "step": "Onayla",
            "moment": "2026-10-17T10:00:00", "user": "Ayşe Yılmaz" }))
        .expect("an input"),
    );
    let _ = kentos_native_application::set::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        serde_json::from_value(
            json!({ "uids": [meter], "attrs": { "tur": "gaz" }, "operation": "attributes" }),
        )
        .expect("an input"),
    );
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
}

/// The meters' building form with new items (docs/adr/0441): its draft and release replaced by `items`.
fn meters_rebodied(app: &mut App, items: Value) {
    meters(app);
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    let form = settings
        .forms
        .iter_mut()
        .find(|f| f.id == "bina")
        .expect("the form");
    let mut body = form.draft.clone();
    body.items = serde_json::from_value(items).expect("items");
    form.draft = body.clone();
    form.releases = vec![kentos_contracts::FormRelease {
        version: 1,
        digest: kentos_contracts::body_digest(&body),
        body,
        note: None,
    }];
    doc.model.set_settings(settings);
    app.form_memory.clock = Some("2026-10-17T10:00:00".into());
    app.form_memory.person = Some("Ayşe Yılmaz".into());
}

/// The building items of ADR 0441's pictures: a divider, the meters as a chart by kind, an action.
fn meters_items() -> Value {
    json!([
        { "type": "field", "id": "ad", "field": "Ad" },
        { "type": "divider", "id": "sayac-bolumu", "label": "Sayaçlar" },
        { "type": "related", "id": "sayaclar", "label": "Türlerine göre sayaçlar", "layer": "sayac", "field": "bina",
          "view": "chart", "columns": ["tur"] },
        { "type": "action", "id": "hesapla", "label": "Öznitelik hesapla", "tool": "attributes.calculate" }
    ])
}

/// Formla düzenle on the first building: a divider, its meters as bars by kind, an action's button
/// (docs/adr/0441 §2, §3, §6).
fn meters_chart(app: &mut App) {
    meters_rebodied(app, meters_items());
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
}

/// Formlar on the building form, İlişkiler's palette shown (docs/adr/0441 §1).
fn meters_palette(app: &mut App) {
    meters_rebodied(app, meters_items());
    run(app, "data.forms");
    designer(app, designer::Event::Palette(designer::Palette::Relations));
}

/// The action chosen: its button's words and the processing tool picked from the registered ones.
fn meters_action(app: &mut App) {
    meters_rebodied(app, meters_items());
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("hesapla".into()));
}

/// A section library in a folder of its own for the pictures and tests.
pub(crate) fn scene_library(app: &mut App, name: &str) {
    let dir = std::env::temp_dir().join(format!("kentos-form-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    app.form_files = crate::forms::library::FormFiles::at(&dir);
}

/// The chart kept twice in the section library, the meter form's palette showing how it binds there.
fn meters_sections(app: &mut App) {
    meters_rebodied(app, meters_items());
    scene_library(app, "bolumler");
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("sayaclar".into()));
    designer(app, designer::Event::SectionName("Sayaç grafiği".into()));
    designer(app, designer::Event::SaveSection);
    designer(app, designer::Event::SectionName("Sayaç grafiği".into()));
    designer(app, designer::Event::SaveSection);
    designer(app, designer::Event::SelectItem("ad".into()));
    designer(app, designer::Event::SectionName("Bina adı".into()));
    designer(app, designer::Event::SaveSection);
    designer(app, designer::Event::Choose("sayac".into()));
    designer(app, designer::Event::Palette(designer::Palette::Sections));
}

/// No item chosen: what the form needs of its host and the reviewers' comments, one settled.
fn meters_review(app: &mut App) {
    meters_rebodied(app, meters_items());
    run(app, "data.forms");
    designer(
        app,
        designer::Event::CommentText("Sayaçların türleri değer listesine bağlansın.".into()),
    );
    designer(app, designer::Event::AddComment);
    designer(
        app,
        designer::Event::CommentText("Bölücünün başlığı uygun.".into()),
    );
    designer(app, designer::Event::AddComment);
    designer(app, designer::Event::ResolveComment(2, true));
}

/// The designer lost with a change in it (the program closed), opened again: the draft offered back.
fn meters_kept(app: &mut App) {
    meters_rebodied(app, meters_items());
    scene_library(app, "taslak");
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("ad".into()));
    designer(app, designer::Event::AddDivider);
    app.forms_window = None;
    app.dialog = None;
    run(app, "data.forms");
}

/// The inspection buildings with a notes field the workflow's steps write to (docs/adr/0442 §4).
fn noted_buildings(app: &mut App) {
    workflow_buildings(app);
    let doc = app.document.as_mut().expect("a drawing");
    let mut fields = doc.model.layers().get("bina").expect("bina").fields.clone();
    fields.push(kentos_contracts::LayerField::new(
        "Notlar",
        kentos_contracts::LayerFieldKind::Text,
    ));
    let _ = doc.model.set_layer_fields("bina", fields, &[]);
    let mut settings = doc.model.settings().clone();
    let form = settings.forms.first_mut().expect("the form");
    if let Some(w) = form.draft.workflow.as_mut() {
        w.notes = Some("Notlar".into());
    }
    let body = form.draft.clone();
    form.releases = vec![kentos_contracts::FormRelease {
        version: 1,
        digest: kentos_contracts::body_digest(&body),
        body,
        note: None,
    }];
    doc.model.set_settings(settings);
    app.form_memory.clock = Some("2026-10-17T10:00:00".into());
    app.form_memory.person = Some("Ayşe Yılmaz".into());
}

/// The sent building under review: Alanlara not on, notes on two fields, the reason typed (docs/adr/0442 §4).
fn reviewer_notes(app: &mut App) {
    noted_buildings(app);
    app.selection.set([Slot(2)]);
    run(app, "data.formEdit");
    entry(app, entry::Event::Reviewing(true));
    entry(
        app,
        entry::Event::FieldNote("Kat".into(), "Çatı katı sayılmamış.".into()),
    );
    entry(
        app,
        entry::Event::FieldNote("Kod".into(), "Tabeladaki kod B-02.".into()),
    );
    entry(
        app,
        entry::Event::Value("Gerekçe".into(), "Ölçüler eksik.".into()),
    );
}

/// Returned with the notes, opened again: each note under its field.
fn reviewer_notes_shown(app: &mut App) {
    reviewer_notes(app);
    entry(app, entry::Event::Transition("Geri gönder".into()));
    app.selection.set([Slot(2)]);
    run(app, "data.formEdit");
}

/// The form published again while a record was being typed into: Yeni sürümle aç (docs/adr/0442 §2).
fn new_release(app: &mut App) {
    workflow_buildings(app);
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
    entry(app, entry::Event::Value("Kat".into(), "4".into()));
    let doc = app.document.as_mut().expect("a drawing");
    let mut body = doc.model.settings().forms[0].draft.clone();
    body.items.retain(|i| i.id() != "gerekce");
    let _ = kentos_native_application::forms_edit::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        kentos_contracts::FormsEdit {
            edit: kentos_contracts::FormsOp::Update {
                id: "denetim".into(),
                body,
            },
            expected_revision: None,
        },
    );
    let _ = kentos_native_application::forms_edit::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        kentos_contracts::FormsEdit {
            edit: kentos_contracts::FormsOp::Publish {
                id: "denetim".into(),
                note: None,
            },
            expected_revision: None,
        },
    );
    entry(app, entry::Event::Save);
}

/// Yayınla over records the new release breaks: asked first (docs/adr/0442 §2).
fn breaking_publish(app: &mut App) {
    workflow_buildings(app);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("gerekce".into()));
    designer(app, designer::Event::Required(true));
    designer(app, designer::Event::Publish);
}

/// An action that takes a workflow step, its step picked (docs/adr/0442 §5).
fn step_action(app: &mut App) {
    workflow_buildings(app);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("kat".into()));
    designer(app, designer::Event::AddAction);
    designer(
        app,
        designer::Event::ActionKind(kentos_contracts::ActionKind::Step),
    );
    designer(
        app,
        designer::Event::Text(designer::Text::Label, "Gönder".into()),
    );
}

/// Değişiklikler with the areas each change touches and what the draft newly needs (docs/adr/0442 §3).
fn change_areas(app: &mut App) {
    workflow_buildings(app);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("kat".into()));
    designer(app, designer::Event::Required(false));
    designer(app, designer::Event::AddLocation);
    designer(app, designer::Event::AddAction);
    designer(app, designer::Event::Changes);
}

/// The first building's attachments read by `cad.attachments.inspect`: Bilgi (docs/adr/0442 §7).
fn attachment_facts(app: &mut App) {
    let mut items = meters_items();
    if let Some(xs) = items.as_array_mut() {
        xs.insert(
            1,
            json!({ "type": "attachments", "id": "ekler", "label": "Ekler" }),
        );
    }
    meters_rebodied(app, items);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    let photo = root.join("exif/v1/konum.jpg").display().to_string();
    let plan = root.join("attachments/v1/plan.pdf").display().to_string();
    let doc = app.document.as_mut().expect("a drawing");
    let uid = doc
        .model
        .uid(Slot(1))
        .map(|u| u.to_string())
        .unwrap_or_default();
    let _ = kentos_native_application::set::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        serde_json::from_value(
            json!({ "uids": [uid], "operation": "attachments", "attachments": [
            { "name": "Ön cephe", "file": photo },
            { "name": "Yapı ruhsatı", "file": plan },
            { "name": "Tapu kaydı", "url": "https://ornek.org/tapu/101-5.pdf" }
        ] }),
        )
        .expect("an input"),
    );
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
    entry(app, entry::Event::InspectAttachments);
}

/// The province being searched: “an” typed, typing paused.
fn searched_list(app: &mut App) {
    chosen_facilities(app);
    searched(app, "İl", "an");
}

/// `text` typed into the searched list of `key` and its turn come (docs/adr/0439 §6): what the pause
/// after typing delivers.
fn searched(app: &mut App, key: &str, text: &str) {
    let _ = app.update(Message::FormEntry(entry::Event::Query(
        key.into(),
        text.into(),
    )));
    let turn = app
        .form_entry
        .as_ref()
        .and_then(|e| e.search_for_tests(key).0);
    if let Some(turn) = turn {
        let _ = app.update(Message::FormEntry(entry::Event::Search(key.into(), turn)));
    }
}

/// A drafted building with its floor count emptied: kept incomplete, Gönder asks for it (docs/adr/0432).
fn workflow_draft(app: &mut App) {
    workflow_buildings(app);
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
    let _ = app.update(Message::FormEntry(entry::Event::Value(
        "Kat".into(),
        String::new(),
    )));
}

/// A parcel's form with its place (docs/adr/0433): an L-shaped parcel on the small map, its centre's
/// coordinates, area and perimeter.
fn located_parcel(app: &mut App) {
    let mut o = Objects::new();
    let id = o.path(
        "kadastro",
        &[
            [0.0, 0.0],
            [32.0, 0.0],
            [32.0, 14.0],
            [18.0, 14.0],
            [18.0, 26.0],
            [0.0, 26.0],
        ],
        true,
    );
    o.data(
        id,
        &[("Ada", "112"), ("Parsel", "7"), ("Nitelik", "Arsa")],
        Some("112/7"),
    );
    open_edited(app, o, |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut kadastro = crate::tools_scenes::layer("kadastro", "Kadastro", "#E5484D", 0.35);
        kadastro["fields"] = json!([{ "name": "Ada", "kind": "text" }, { "name": "Parsel", "kind": "text" },
                                  { "name": "Nitelik", "kind": "text" }]);
        layers.push(kadastro);
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("kadastro");
    });
    let body = json!({ "revision": 1, "items": [
        { "type": "group", "id": "kimlik", "label": "Parsel", "columns": 2, "items": [
            { "type": "field", "id": "ada", "field": "Ada" }, { "type": "field", "id": "parsel", "field": "Parsel" },
            { "type": "field", "id": "nitelik", "field": "Nitelik" }] },
        { "type": "location", "id": "konum", "label": "Konum" }
    ] });
    let typed: kentos_contracts::FormBody = serde_json::from_value(body.clone()).expect("a body");
    let form = json!({ "id": "parsel", "name": "Parsel kaydı", "layer": "kadastro", "draft": body.clone(),
        "releases": [{ "version": 1, "digest": kentos_contracts::body_digest(&typed), "body": body }] });
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms = vec![serde_json::from_value(form).expect("a form")];
    doc.model.set_settings(settings);
    app.features.layer = Some("kadastro".into());
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
}

/// The inspection form taken as a package onto the parcels' layer (docs/adr/0435): its fields added there,
/// the new form chosen in Formlar.
fn imported_form(app: &mut App) {
    workflow_buildings(app);
    run(app, "data.forms");
    let text = app.form_package_text().expect("a package");
    if let Some(doc) = app.document.as_mut() {
        doc.model.set_active_layer("parsel");
    }
    if let Some(w) = app.forms_window.as_mut() {
        w.chosen = None;
    }
    app.form_imported(Some(("bina-denetimi.kform".into(), text)));
}

/// A parcel's registration date out of its range (docs/adr/0436): the date field takes 2000-01-01 to
/// 2026-12-31, and 1.1.2027 is typed.
fn dated_parcel(app: &mut App) {
    located_parcel(app);
    let doc = app.document.as_mut().expect("a drawing");
    let mut fields = doc
        .model
        .layers()
        .get("kadastro")
        .expect("kadastro")
        .fields
        .clone();
    fields.push(kentos_contracts::LayerField {
        min: Some("2000-01-01".into()),
        max: Some("2026-12-31".into()),
        ..kentos_contracts::LayerField::new("Tescil", kentos_contracts::LayerFieldKind::Date)
    });
    let _ = doc.model.set_layer_fields("kadastro", fields, &[]);
    let mut settings = doc.model.settings().clone();
    if let Some(f) = settings.forms.first_mut()
        && let Some(kentos_contracts::FormItem::Group { items, .. }) = f.draft.items.first_mut()
    {
        items.push(
            serde_json::from_value(json!({ "type": "field", "id": "tescil", "field": "Tescil" }))
                .expect("an item"),
        );
        let body = f.draft.clone();
        f.releases = vec![kentos_contracts::FormRelease {
            version: 1,
            digest: kentos_contracts::body_digest(&body),
            body,
            note: None,
        }];
    }
    doc.model.set_settings(settings);
    app.form_entry = None;
    app.dialog = None;
    run(app, "data.formEdit");
    let _ = app.update(Message::FormEntry(entry::Event::Value(
        "Tescil".into(),
        "1.1.2027".into(),
    )));
}

/// A survey form of three steps (docs/adr/0437): Kimlik with the use as a tree, Yapı, and Özet showing the
/// name and the use again as mirrors.
fn stepped_survey(app: &mut App) {
    let mut o = Objects::new();
    let id = o.path(
        "yapi",
        &[[0.0, 0.0], [16.0, 0.0], [16.0, 12.0], [0.0, 12.0]],
        true,
    );
    o.data(
        id,
        &[("Ad", "Gül Apartmanı"), ("Kullanım", "apt")],
        Some("Gül Apartmanı"),
    );
    open_edited(app, o, |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut yapi = crate::tools_scenes::layer("yapi", "Yapı", "#8E4EC6", 0.35);
        yapi["fields"] = json!([{ "name": "Ad", "kind": "text" },
                                { "name": "Kullanım", "kind": "text", "values": [
                                    { "code": "mus", "label": "Konut / Müstakil" }, { "code": "apt", "label": "Konut / Apartman" },
                                    { "code": "vil", "label": "Konut / Villa" }, { "code": "duk", "label": "Ticaret / Dükkân" },
                                    { "code": "ofi", "label": "Ticaret / Ofis" }, { "code": "dep", "label": "Sanayi / Depo" },
                                    { "code": "ata", "label": "Sanayi / Atölye" }] },
                                { "name": "Kat", "kind": "integer", "min": "1" },
                                { "name": "Not", "kind": "text" }]);
        layers.push(yapi);
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("yapi");
    });
    let body = json!({ "revision": 1, "steps": true, "items": [
        { "type": "tab", "id": "kimlik", "label": "Kimlik", "items": [
            { "type": "field", "id": "ad", "field": "Ad", "required": true },
            { "type": "field", "id": "kullanim", "field": "Kullanım", "widget": "tree" }] },
        { "type": "tab", "id": "yapi", "label": "Yapı", "items": [
            { "type": "field", "id": "kat", "field": "Kat", "required": true }] },
        { "type": "tab", "id": "ozet", "label": "Özet", "items": [
            { "type": "field", "id": "ad-ozet", "field": "Ad", "mirror": true, "label": "Yapının adı" },
            { "type": "field", "id": "kullanim-ozet", "field": "Kullanım", "mirror": true },
            { "type": "field", "id": "not", "field": "Not", "widget": "textArea" }] }
    ] });
    let typed: kentos_contracts::FormBody = serde_json::from_value(body.clone()).expect("a body");
    let form = json!({ "id": "yapi", "name": "Yapı tespiti", "layer": "yapi", "draft": body.clone(),
        "releases": [{ "version": 1, "digest": kentos_contracts::body_digest(&typed), "body": body }] });
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms = vec![serde_json::from_value(form).expect("a form")];
    doc.model.set_settings(settings);
    app.features.layer = Some("yapi".into());
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
}

/// Ruhsatlı yapılar (docs/adr/0438): a layer with record rules (dates in order, a note for an
/// inspection, floors and area), a unique parcel, a floor count whose 0 is left out, an area in m², a
/// status list with a value's description and days; a form whose note check is asked on saving.
fn permits(app: &mut App) {
    let mut o = Objects::new();
    let first = o.path(
        "ruhsat",
        &[[0.0, 0.0], [18.0, 0.0], [18.0, 14.0], [0.0, 14.0]],
        true,
    );
    o.data(
        first,
        &[
            ("Durum", "iyi"),
            ("Ada/Parsel", "101/5"),
            ("Başlangıç", "2026-03-01"),
            ("Bitiş", "2026-09-30"),
            ("Kat", "3"),
            ("Alan", "900.00"),
        ],
        Some("101/5"),
    );
    let second = o.path(
        "ruhsat",
        &[[26.0, 0.0], [40.0, 0.0], [40.0, 12.0], [26.0, 12.0]],
        true,
    );
    o.data(
        second,
        &[
            ("Durum", "inceleme"),
            ("Ada/Parsel", "101/6"),
            ("Kat", "1"),
            ("Alan", "700.00"),
        ],
        Some("101/6"),
    );
    open_edited(app, o, |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut ruhsat = crate::tools_scenes::layer("ruhsat", "Ruhsatlı yapı", "#0E7490", 0.35);
        ruhsat["fields"] = json!([
            { "name": "Durum", "kind": "text", "values": [
                { "code": "iyi", "label": "İyi", "description": "Taşıyıcı sistemde kusur yok." },
                { "code": "inceleme", "label": "Ek inceleme gerekli" },
                { "code": "eski", "label": "Eski sınıf", "until": "2025-12-31" },
                { "code": "yeni", "label": "Yeni sınıf", "from": "2027-01-01" }] },
            { "id": "ada-parsel", "name": "Ada/Parsel", "kind": "text", "unique": true },
            { "name": "Başlangıç", "kind": "date" },
            { "name": "Bitiş", "kind": "date" },
            { "name": "Kat", "kind": "integer", "min": "0", "max": "60", "minExclusive": true },
            { "name": "Alan", "kind": "decimal", "scale": 2, "unit": "m²" },
            { "name": "Açıklama", "kind": "text" }]);
        ruhsat["rules"] = json!([
            { "id": "tarih-sirasi", "name": "Tarih sırası",
              "rule": "Bitiş boş veya Başlangıç boş veya Bitiş >= Başlangıç",
              "message": "Bitiş tarihi başlangıçtan önce olamaz.", "fields": ["Bitiş"] },
            { "id": "inceleme-notu", "name": "İnceleme notu",
              "rule": "Durum <> 'inceleme' veya Açıklama boş değil",
              "message": "Ek inceleme gereken yapıya açıklama yazın.", "warning": true },
            { "id": "kat-alan", "name": "Kat ve alan", "rule": "Alan boş veya Kat boş veya Alan <= Kat * 500",
              "message": "Kat başına en çok 500 m² olur." }]);
        layers.push(ruhsat);
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("ruhsat");
    });
    let body = json!({ "revision": 1, "items": [
        { "type": "field", "id": "durum", "field": "Durum" },
        { "type": "field", "id": "ada", "field": "ada-parsel" },
        { "type": "field", "id": "baslangic", "field": "Başlangıç" },
        { "type": "field", "id": "bitis", "field": "Bitiş" },
        { "type": "field", "id": "kat", "field": "Kat" },
        { "type": "field", "id": "alan", "field": "Alan" },
        { "type": "field", "id": "aciklama", "field": "Açıklama", "checks": [
            { "rule": "uzunluk(Açıklama) >= 5", "message": "Açıklama en az 5 karakter olmalı.", "on": "save" }] }
    ] });
    let typed: kentos_contracts::FormBody = serde_json::from_value(body.clone()).expect("a body");
    let form = json!({ "id": "ruhsat", "name": "Ruhsat", "layer": "ruhsat", "draft": body.clone(),
        "releases": [{ "version": 1, "digest": kentos_contracts::body_digest(&typed), "body": body }] });
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms = vec![serde_json::from_value(form).expect("a form")];
    doc.model.set_settings(settings);
    app.features.layer = Some("ruhsat".into());
}

/// Kayıt kuralları on the permits, the dates' rule chosen.
fn permit_rules(app: &mut App) {
    permits(app);
    run(app, "data.layerRules");
    let _ = app.update(Message::LayerRules(
        crate::forms::layer_rules::Event::Choose(2),
    ));
}

/// Formla düzenle on the first permit: the end before the start (the rule under Bitiş), the area over
/// the floors' (under Alan, its unit after it), the status's description under it.
fn permit_entry(app: &mut App) {
    permits(app);
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
    entry(
        app,
        entry::Event::Value("Bitiş".into(), "01.02.2026".into()),
    );
    entry(app, entry::Event::Value("Alan".into(), "1600".into()));
}

/// The record changed while the window was open: Kaydet says so, Yeniden yükle and Üzerine yaz.
fn permit_conflict(app: &mut App) {
    permits(app);
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
    entry(app, entry::Event::Value("Kat".into(), "4".into()));
    let uid = app
        .document
        .as_ref()
        .and_then(|d| d.model.uid(Slot(1)))
        .map(|u| u.to_string())
        .unwrap_or_default();
    let doc = app.document.as_mut().expect("a drawing");
    let _ = kentos_native_application::set::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        serde_json::from_value(
            json!({ "uids": [uid], "attrs": { "Açıklama": "Başkası yazdı." },
                                       "operation": "attributes" }),
        )
        .expect("an input"),
    );
    entry(app, entry::Event::Save);
}

/// Alanlar on the permits, the floor count chosen: Tekil, En az hariç, Birim.
fn permit_fields(app: &mut App) {
    permits(app);
    let _ = app.open_layer_fields("ruhsat");
    let _ = app.update(Message::LayerFields(crate::layer_fields::Event::Choose(4)));
}

/// The status's value list: a description and the days a value holds.
fn permit_values(app: &mut App) {
    permits(app);
    let _ = app.open_layer_fields("ruhsat");
    let _ = app.update(Message::LayerFields(crate::layer_fields::Event::OpenList(
        0,
    )));
}

/// Formlar on the permit form, the note chosen: its links name the layer rule reading it.
fn permit_designer(app: &mut App) {
    permits(app);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("aciklama".into()));
}

/// Its last step: the mirrors, Geri and Kaydet.
fn stepped_summary(app: &mut App) {
    stepped_survey(app);
    let _ = app.update(Message::FormEntry(entry::Event::Value(
        "Kat".into(),
        "6".into(),
    )));
    let _ = app.update(Message::FormEntry(entry::Event::Tab(1)));
    let _ = app.update(Message::FormEntry(entry::Event::Tab(2)));
}

/// The step form in Formlar: Adım adım on, the mirror marked in the items, Kopyala and Yapıştır.
fn stepped_designer(app: &mut App) {
    stepped_survey(app);
    app.form_entry = None;
    app.dialog = None;
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("ad-ozet".into()));
}

/// Değer bileşimleri's window on the buildings' group (docs/adr/0423 §5).
fn contingents_window(app: &mut App) {
    typed_buildings(app);
    run(app, "data.contingents");
}

/// A timber house given steel: no row of the group allows steel with a hipped roof; the roof's
/// choices follow the material (docs/adr/0423 §2). Kaydet is pressed, so the problem is shown.
fn combination_refused(app: &mut App) {
    typed_buildings(app);
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
    entry(app, entry::Event::Value("Malzeme".into(), "celik".into()));
    entry(app, entry::Event::Save);
}

/// Alt türler's window on the buildings, the industrial subtype chosen (docs/adr/0422 §5).
fn subtypes_window(app: &mut App) {
    use crate::forms::subtypes::Event;
    typed_buildings(app);
    run(app, "data.subtypes");
    let _ = app.update(Message::Subtypes(Event::Choose("sanayi".into())));
}

/// A timber house's form, its kind changed to industrial: timber is not offered, the note is
/// required, the values are kept until the user empties them (docs/adr/0422 §2).
fn subtype_changed(app: &mut App) {
    typed_buildings(app);
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
    entry(app, entry::Event::Value("Tür".into(), "sanayi".into()));
}

/// Hesapları yenile (docs/adr/0421): the survey form gains a calculated note after the buildings
/// were written; one press writes it on every building, the window then closed over the table.
fn recompute(app: &mut App) {
    buildings_with_a_form(app);
    let form = published(
        "hasar",
        "Hasar tespiti",
        "bina",
        json!([
            { "type": "field", "id": "durum", "field": "Durum", "visibleWhen": "Kat > 0" },
            { "type": "field", "id": "kat", "field": "Kat" },
            { "type": "field", "id": "not", "field": "Not", "calculate": "Kod || ': ' || Kat || ' katlı, ' || Durum" }
        ]),
    );
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms = vec![serde_json::from_value(form).expect("a form")];
    doc.model.set_settings(settings);
    run(app, "data.forms");
    designer(app, designer::Event::Recompute);
    designer(app, designer::Event::Cancel);
}

/// Formlar's preview with two scenarios (docs/adr/0420): kept, then the floor count made required, so
/// that one fails and says why.
fn scenarios(app: &mut App) {
    designer_links(app);
    designer(app, designer::Event::Preview(true));
    let typed = |app: &mut App, key: &str, v: &str| {
        let _ = app.update(Message::FormPreview(entry::Event::Value(
            key.into(),
            v.into(),
        )));
    };
    typed(app, "Durum", "iyi");
    typed(app, "Kat", "8");
    designer(
        app,
        designer::Event::ScenarioName("Sekiz katlı iyi bina".into()),
    );
    designer(app, designer::Event::SaveScenario);
    typed(app, "Kat", "");
    designer(app, designer::Event::ScenarioName("Katsız kayıt".into()));
    designer(app, designer::Event::SaveScenario);
    // The form changes: a building has fewer than five floors.
    designer(app, designer::Event::SelectItem("kat".into()));
    designer(
        app,
        designer::Event::RuleText(designer::Rule::Check("kat".into(), 0), "Kat < 5".into()),
    );
    designer(
        app,
        designer::Event::Text(designer::Text::CheckMessage(0), "En çok dört kat.".into()),
    );
    designer(app, designer::Event::Preview(true));
}

/// Formlar on the survey form, the floor count chosen: what reads it (plan FORM-DESIGN-09).
fn designer_links(app: &mut App) {
    buildings_with_a_form(app);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("kat".into()));
}

/// Alanlar on the buildings, the floor count chosen: it is shared by area when a building is split and
/// summed when buildings are merged (docs/adr/0419).
fn field_policies(app: &mut App) {
    use crate::layer_fields::Event;
    styled_buildings(app);
    let _ = app.open_layer_fields("bina");
    let _ = app.update(Message::LayerFields(Event::Choose(2)));
    let _ = app.update(Message::LayerFields(Event::Split(
        2,
        Some(kentos_contracts::SplitPolicy::Ratio),
    )));
    let _ = app.update(Message::LayerFields(Event::Merge(
        2,
        Some(kentos_contracts::MergePolicy::Sum),
    )));
}

/// Koşullu biçim's window on the buildings' rules, the floor count's rule chosen.
fn styles_window(app: &mut App) {
    styled_buildings(app);
    run(app, "data.tableStyles");
    let _ = app.update(Message::TableStyles(
        crate::forms::table_styles::Event::Choose(2),
    ));
}

/// Two buildings and their floors (docs/adr/0415): the floors are records a composite relation ties
/// to their building; the survey form keeps the building's code unique.
fn floors(app: &mut App) {
    let mut o = Objects::new();
    let mut ids = Vec::new();
    for (x, code) in [(0.0, "B-1"), (24.0, "B-2")] {
        let id = o.path(
            "bina",
            &[[x, 0.0], [x + 18.0, 0.0], [x + 18.0, 14.0], [x, 14.0]],
            true,
        );
        o.data(id, &[("Kod", code), ("Kat", "2")], Some(code));
        ids.push(id);
    }
    let mut floors = Vec::new();
    for (building, no, use_) in [
        (0, "Zemin", "Dükkân"),
        (0, "1", "Konut"),
        (1, "Zemin", "Konut"),
    ] {
        let f = o.record("kat");
        o.data(f, &[("No", no), ("Kullanım", use_)], None);
        floors.push((f, building));
    }
    let form = published(
        "bina-formu",
        "Bina denetimi",
        "bina",
        json!([
            { "type": "field", "id": "kod", "field": "Kod", "unique": true },
            { "type": "field", "id": "kat", "field": "Kat" },
            { "type": "related", "id": "katlar", "label": "Katlar", "layer": "kat", "field": "Bina" }
        ]),
    );
    open_edited(app, o, move |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut bina = crate::tools_scenes::layer("bina", "Bina", "#3E63DD", 0.35);
        bina["fields"] =
            json!([{ "name": "Kod", "kind": "text" }, { "name": "Kat", "kind": "integer" }]);
        let mut kat = crate::tools_scenes::layer("kat", "Kat planı", "#30A46C", 0.35);
        kat["fields"] = json!([{ "name": "Bina", "kind": "text" }, { "name": "No", "kind": "text" },
                               { "name": "Kullanım", "kind": "text" }]);
        layers.push(bina);
        layers.push(kat);
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("bina");
        d["settings"]["forms"] = json!([form]);
        d["settings"]["relations"] = json!([{ "id": "katlar", "name": "Binanın katları", "parent": "bina",
                                              "child": "kat", "field": "Bina", "onDelete": "cascade" }]);
    });
    let uid = |app: &App, slot: u32| {
        app.document
            .as_ref()
            .and_then(|d| d.model.uid(Slot(slot)))
            .map(|u| u.to_string())
            .unwrap_or_default()
    };
    for (f, b) in floors {
        let (floor, building) = (uid(app, f), uid(app, ids[b]));
        if let Some(doc) = app.document.as_mut() {
            let _ = kentos_native_application::registry::run(
                &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
                kentos_contracts::CAD_ENTITIES_SET,
                kentos_contracts::CallOp::Execute,
                json!({ "uids": [floor], "attrs": { "Bina": building }, "operation": "attributes" }),
            );
        }
    }
    app.selection.set([Slot(ids[0])]);
}

/// Çoğalt on the first building: its copy and the copies of its two floors selected, the floors' table.
fn duplicated(app: &mut App) {
    floors(app);
    let _ = app.update(Message::Features(crate::features::Event::Duplicate));
    app.features.layer = Some("kat".into());
    let _ = app.update(Message::Run("data.featureTable"));
}

/// A building whose height is calculated from its floors and may be typed over (docs/adr/0417): its
/// form open, the height typed over.
fn overridable_height(app: &mut App) {
    let mut o = Objects::new();
    let id = o.path(
        "bina",
        &[[0.0, 0.0], [18.0, 0.0], [18.0, 14.0], [0.0, 14.0]],
        true,
    );
    o.data(id, &[("Kod", "B-1"), ("Kat", "3")], Some("B-1"));
    let form = published(
        "yukseklik",
        "Bina yüksekliği",
        "bina",
        json!([
            { "type": "field", "id": "kod", "field": "Kod" },
            { "type": "field", "id": "kat", "field": "Kat" },
            { "type": "field", "id": "yukseklik", "field": "Yükseklik", "calculate": "Kat * 3", "overridable": true }
        ]),
    );
    open_edited(app, o, move |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut bina = crate::tools_scenes::layer("bina", "Bina", "#3E63DD", 0.35);
        bina["fields"] = json!([{ "name": "Kod", "kind": "text" }, { "name": "Kat", "kind": "integer" },
                                { "name": "Yükseklik", "kind": "decimal", "scale": 1 }]);
        layers.push(bina);
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("bina");
        d["settings"]["forms"] = json!([form]);
    });
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
}

/// The height typed over: ↺ beside it gives it back to the calculation.
fn overridden_height(app: &mut App) {
    overridable_height(app);
    let _ = app.update(Message::FormEntry(entry::Event::Value(
        "Yükseklik".into(),
        "10.5".into(),
    )));
}

/// The owners' table (docs/adr/0410 §4): records without a geometry in the Tablo tab.
fn owner_table(app: &mut App) {
    owners(app);
    app.features.layer = Some("malik".into());
    let _ = app.update(Message::Run("data.featureTable"));
    app.selection.set([Slot(4)]);
}

/// Kayıt ekle in the owners' table: a new record, opened by the owners' form.
fn owner_added(app: &mut App) {
    owner_table(app);
    let _ = app.update(Message::Features(crate::features::Event::AddRecord));
}

/// Alanlar's value list of the care field: several of its values may be chosen (docs/adr/0408 §1).
fn tree_fields(app: &mut App) {
    trees(app);
    let _ = app.open_layer_fields("agac");
    let _ = app.update(Message::LayerFields(crate::layer_fields::Event::OpenList(
        4,
    )));
}

/// Formlar on the tree form: the photos item chosen.
fn tree_designing(app: &mut App) {
    trees(app);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("fotograflar".into()));
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

/// The address form (docs/adr/0404; `LOOK_BODY` of `scripts/fixtures/forms_cases.py`): the province, the district
/// chained to it, the neighbourhood chained and filtered, the parcel by its persistent id, the meters that refer to
/// the building.
fn address_form() -> Value {
    let body = json!({ "revision": 1, "items": [
        { "type": "group", "id": "adres", "label": "Adres", "items": [
            { "type": "field", "id": "il", "field": "İl", "lookup": { "layer": "il", "key": "kod", "label": "ad" } },
            { "type": "field", "id": "ilce", "field": "İlçe",
              "lookup": { "layer": "ilce", "key": "kod", "label": "ad", "matches": [{ "field": "il", "record": "İl" }] } },
            { "type": "field", "id": "mahalle", "field": "Mahalle",
              "lookup": { "layer": "mahalle", "key": "kod", "label": "ad", "matches": [{ "field": "ilce", "record": "İlçe" }],
                          "filter": "durum = 'etkin'" } },
            { "type": "field", "id": "parsel", "field": "Parsel",
              "lookup": { "layer": "parsel", "key": "$uid", "label": "ada" } }
        ]},
        { "type": "related", "id": "sayaclar", "label": "Binanın sayaçları", "layer": "sayac", "field": "bina" }
    ]});
    let typed: kentos_contracts::FormBody = serde_json::from_value(body.clone()).expect("a body");
    json!({ "id": "adres", "name": "Bina adresi", "layer": "bina", "draft": body.clone(),
            "releases": [{ "version": 1, "digest": kentos_contracts::body_digest(&typed), "body": body }] })
}

/// Buildings with the address form; the provinces, districts, neighbourhoods, parcels and meters on their own layers;
/// the first building's two meters refer to it.
fn address_buildings(app: &mut App) {
    let mut o = Objects::new();
    let mut ids = Vec::new();
    for [x, y] in [[0.0, 0.0], [24.0, 0.0]] {
        ids.push(o.path(
            "bina",
            &[[x, y], [x + 18.0, y], [x + 18.0, y + 14.0], [x, y + 14.0]],
            true,
        ));
    }
    let rows: [(&str, &[(&str, &str)]); 14] = [
        ("il", &[("kod", "06"), ("ad", "Ankara")]),
        ("il", &[("kod", "34"), ("ad", "İstanbul")]),
        ("il", &[("kod", "35"), ("ad", "İzmir")]),
        ("ilce", &[("kod", "0601"), ("ad", "Çankaya"), ("il", "06")]),
        ("ilce", &[("kod", "0602"), ("ad", "Altındağ"), ("il", "06")]),
        ("ilce", &[("kod", "3401"), ("ad", "Kadıköy"), ("il", "34")]),
        (
            "mahalle",
            &[
                ("kod", "M10"),
                ("ad", "Kızılay"),
                ("ilce", "0601"),
                ("durum", "etkin"),
            ],
        ),
        (
            "mahalle",
            &[
                ("kod", "M2"),
                ("ad", "Bahçelievler"),
                ("ilce", "0601"),
                ("durum", "etkin"),
            ],
        ),
        (
            "mahalle",
            &[
                ("kod", "M3"),
                ("ad", "Eski mahalle"),
                ("ilce", "0601"),
                ("durum", "kapalı"),
            ],
        ),
        (
            "mahalle",
            &[
                ("kod", "M4"),
                ("ad", "Ulus"),
                ("ilce", "0602"),
                ("durum", "etkin"),
            ],
        ),
        ("parsel", &[("ada", "101/5")]),
        ("parsel", &[("ada", "101/12")]),
        ("sayac", &[("no", "S-1")]),
        ("sayac", &[("no", "S-2")]),
    ];
    let mut meters = Vec::new();
    for (k, (layer, attrs)) in rows.iter().enumerate() {
        let at = [-10.0 + 4.0 * k as f64, 28.0];
        let id = o.point(layer, at, None);
        o.data(id, attrs, None);
        if *layer == "sayac" {
            meters.push(id);
        }
    }
    let form = address_form();
    open_edited(app, o, move |d| {
        let text = |name: &str| json!({ "name": name, "kind": "text" });
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut bina = crate::tools_scenes::layer("bina", "Bina", "#3E63DD", 0.35);
        bina["fields"] = json!([text("İl"), text("İlçe"), text("Mahalle"), text("Parsel")]);
        layers.push(bina);
        for (id, name, fields) in [
            ("il", "İller", vec!["kod", "ad"]),
            ("ilce", "İlçeler", vec!["kod", "ad", "il"]),
            ("mahalle", "Mahalleler", vec!["kod", "ad", "ilce", "durum"]),
            ("parsel", "Parseller", vec!["ada"]),
            ("sayac", "Sayaçlar", vec!["no", "bina"]),
        ] {
            let mut l = crate::tools_scenes::layer(id, name, "#8E8C99", 0.25);
            l["fields"] = Value::Array(fields.into_iter().map(text).collect());
            layers.push(l);
        }
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("bina");
        d["settings"]["forms"] = json!([form]);
    });
    // The meters refer to the first building by its persistent id.
    if let Some(doc) = app.document.as_mut() {
        let building = doc
            .model
            .uid(Slot(ids[0]))
            .map(|u| u.to_string())
            .unwrap_or_default();
        let uids: Vec<String> = meters
            .iter()
            .filter_map(|m| doc.model.uid(Slot(*m)).map(|u| u.to_string()))
            .collect();
        let _ = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            kentos_contracts::CAD_ENTITIES_SET,
            kentos_contracts::CallOp::Execute,
            json!({ "uids": uids, "attrs": { "bina": building }, "operation": "attributes" }),
        );
    }
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

/// Formla düzenle on the first building through the address form: Ankara and Çankaya chosen, the
/// neighbourhoods Çankaya has open offered, the meters referring to it.
fn address_editing(app: &mut App) {
    address_buildings(app);
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
    entry(app, entry::Event::Value("İl".into(), "Ankara".into()));
    entry(app, entry::Event::Value("İlçe".into(), "Çankaya".into()));
}

/// Formlar: the neighbourhood's list from another layer: its layer, key, label, chain and filter.
fn address_designing(app: &mut App) {
    address_buildings(app);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("mahalle".into()));
}

/// İlişkiler (docs/adr/0405): the buildings' meters as a composition, its policy chosen.
fn relations_window(app: &mut App) {
    address_buildings(app);
    run(app, "data.relations");
    let _ = app.update(Message::Relations(relations::Event::New));
    for event in [
        relations::Event::Name("Binanın sayaçları".into()),
        relations::Event::Parent("bina".into()),
        relations::Event::Child("sayac".into()),
        relations::Event::Field("bina".into()),
        relations::Event::Policy(kentos_contracts::RelationDelete::Cascade),
    ] {
        let _ = app.update(Message::Relations(event));
    }
}

/// Kayıtları denetle (docs/adr/0406 §1): the calculating form's buildings, some of them breaking its
/// rules (a code twice, a status missing), warnings shown too.
fn records_audit(app: &mut App) {
    calc_buildings(app);
    if let Some(doc) = app.document.as_mut() {
        let uid = doc
            .model
            .uid(Slot(2))
            .map(|u| u.to_string())
            .unwrap_or_default();
        let _ = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            kentos_contracts::CAD_ENTITIES_SET,
            kentos_contracts::CallOp::Execute,
            json!({ "uids": [uid], "attrs": { "Durum": "iyi", "Kod": "B-3", "Kat": "2" }, "operation": "attributes" }),
        );
    }
    run(app, "data.formAudit");
    let _ = app.update(Message::FormAudit(audit::Event::Warnings(true)));
}

/// Formla düzenle with the storeys on a slider (docs/adr/0406 §2).
fn slider_editing(app: &mut App) {
    calc_buildings(app);
    if let Some(doc) = app.document.as_mut() {
        let mut s = doc.model.settings().clone();
        for f in &mut s.forms {
            if let Some(kentos_contracts::FormItem::Field(k)) =
                items_mut_find(&mut f.draft.items, "kat")
            {
                k.widget = Some(kentos_contracts::FormWidget::Slider);
            }
            for r in &mut f.releases {
                if let Some(kentos_contracts::FormItem::Field(k)) =
                    items_mut_find(&mut r.body.items, "kat")
                {
                    k.widget = Some(kentos_contracts::FormWidget::Slider);
                }
                r.digest = kentos_contracts::body_digest(&r.body);
            }
        }
        doc.model.set_settings(s);
    }
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
}

/// The item `id` anywhere in `items`, to change.
fn items_mut_find<'a>(
    items: &'a mut [kentos_contracts::FormItem],
    id: &str,
) -> Option<&'a mut kentos_contracts::FormItem> {
    for i in items.iter_mut() {
        if i.id() == id {
            return Some(i);
        }
        if let Some(found) = i.items_mut().and_then(|xs| items_mut_find(xs, id)) {
            return Some(found);
        }
    }
    None
}

/// A street block of four buildings, two with records; the layer's form in the project.
pub(crate) fn buildings(app: &mut App, published: bool) {
    buildings_with(app, form(published));
}

/// The four buildings with `form` on their layer.
pub(crate) fn buildings_with(app: &mut App, form: Value) {
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

pub(crate) fn designer(app: &mut App, event: designer::Event) {
    let _ = app.update(Message::Forms(event));
}

pub(crate) fn entry(app: &mut App, event: entry::Event) {
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
        ("form-adres", address_editing),
        ("form-adres-tasarim", address_designing),
        ("form-iliskiler", relations_window),
        ("form-kayit-denetle", records_audit),
        ("form-kaydirici", slider_editing),
        ("form-agac", tree_editing),
        ("form-agac-eksik", tree_missing),
        ("form-agac-tasarim", tree_designing),
        ("form-alan-coklu", tree_fields),
        ("form-malik", owner_editing),
        ("form-bina-malikler", building_owners),
        ("form-kayit-tablosu", owner_table),
        ("form-kayit-ekle", owner_added),
        ("form-tarih-saat", timed),
        ("form-kosullu-tablo", styled_buildings),
        ("form-kosullu-bicim", styles_window),
        ("form-alan-kullanim", field_usage),
        ("form-cogalt", duplicated),
        ("form-gorunumu", form_view),
        ("form-baglar", designer_links),
        ("form-senaryolar", scenarios),
        ("form-hesap-yenile", recompute),
        ("form-alt-turler", subtypes_window),
        ("form-alt-tur-degisti", subtype_changed),
        ("form-bilesimler", contingents_window),
        ("form-bilesim-sorunu", combination_refused),
        ("form-iliskili-tablo", units_table),
        ("form-iliskili-tasarim", units_designer),
        ("form-alt-tur-sekme", kind_tabs),
        ("form-degisiklikler", form_changes),
        ("form-is-akisi-tasarim", workflow_designer),
        ("form-is-akisi-adim", workflow_steps),
        ("form-is-akisi-kilitli", workflow_locked),
        ("form-is-akisi-gecmis", workflow_log),
        ("form-taslak", workflow_draft),
        ("form-konum", located_parcel),
        ("form-paket", imported_form),
        ("form-tarih-araligi", dated_parcel),
        ("form-adim-adim", stepped_survey),
        ("form-adim-ozet", stepped_summary),
        ("form-adim-tasarim", stepped_designer),
        ("form-kayit-kurallari", permit_rules),
        ("form-kural-formda", permit_entry),
        ("form-cakisma", permit_conflict),
        ("form-alan-ekleri", permit_fields),
        ("form-deger-gunleri", permit_values),
        ("form-kural-baglari", permit_designer),
        ("form-kaynak-giris", sources_entry),
        ("form-iliski-kartlar", meters_cards),
        ("form-iliski-bekleyen", meters_pending),
        ("form-iliski-yigini", meters_nested),
        ("form-iliski-tasarim", meters_designer),
        ("form-onay-goruntusu", meters_snapshot),
        ("form-bolucu-grafik-eylem", meters_chart),
        ("form-tasarim-iliski-paleti", meters_palette),
        ("form-tasarim-eylem", meters_action),
        ("form-tasarim-bolumler", meters_sections),
        ("form-tasarim-yorumlar", meters_review),
        ("form-tasarim-kalan-taslak", meters_kept),
        ("form-alan-notlari", reviewer_notes),
        ("form-alan-notu-gosterilen", reviewer_notes_shown),
        ("form-yeni-surum", new_release),
        ("form-yayin-kirilan", breaking_publish),
        ("form-eylem-adim", step_action),
        ("form-degisiklik-alanlari", change_areas),
        ("form-ekler-bilgi", attachment_facts),
        ("form-deger-paketi", sources_domains),
        ("form-degerleri-bicime", sources_migrated),
        ("deger-listeleri", domains_window),
        ("deger-listesi-alan", domains_field),
        ("form-cipler", chosen_facilities),
        ("form-hesap-degistir", overridden_height),
        ("form-kalkmis-deger", retired_list),
        ("form-alan-politika", field_policies),
        (
            "form-ceviri-giris",
            crate::form_access_scenes::translated_entry,
        ),
        (
            "form-ceviriler",
            crate::form_access_scenes::translations_designer,
        ),
        (
            "form-sorun-gezinme",
            crate::form_access_scenes::issues_walked,
        ),
        (
            "form-kapanis-sorusu",
            crate::form_access_scenes::closing_asked,
        ),
        ("form-satir-satir", crate::form_access_scenes::across_group),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;

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
        body.items.push(serde_json::from_value(json!(
            { "type": "action", "id": "gonder", "label": "Gönder", "kind": "step", "step": "Gönder" })).unwrap());
        body.items.push(
            serde_json::from_value(json!(
            { "type": "action", "id": "goster", "label": "Göster", "kind": "show" }))
            .unwrap(),
        );
        form.draft = body.clone();
        form.releases = vec![kentos_contracts::FormRelease {
            version: 1,
            digest: kentos_contracts::body_digest(&body),
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
        assert_eq!(kat.split, Some(kentos_contracts::SplitPolicy::Ratio));
        assert_eq!(kat.merge, Some(kentos_contracts::MergePolicy::Sum));
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
        let value =
            |app: &App, name: &str| app.form_entry.as_ref().and_then(|e| e.after_value(name));
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
        assert!(
            !doc.can_undo() || doc.settings().domains[0].values.as_ref().map(Vec::len) == Some(4)
        );
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
        let now =
            kentos_native_application::forms::record_snapshot(doc, "bina", &uid, &attrs).unwrap();
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
            matches!(refused, kentos_contracts::CommandResult::Failed { error } if error.code == "rule_broken")
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

    fn pasted_id(app: &App) -> String {
        app.forms_window.as_ref().unwrap().item.clone().unwrap()
    }

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
            super::super::forms::entry::ordered_for_tests(
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
        use kentos_contracts::FormChangeKind as K;
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
            kentos_contracts::FormItem::Tab { visible_when, .. } => visible_when.clone(),
            _ => None,
        };
        assert_eq!(rule.as_deref(), Some("[Tür] içinde ('sanayi', 'depo')"));
        // Taking both away leaves no rule: the tab is shown for every kind.
        designer(&mut app, designer::Event::ShowInKind("sanayi".into()));
        designer(&mut app, designer::Event::ShowInKind("depo".into()));
        let body = app.forms_window.as_ref().unwrap().body.as_ref().unwrap();
        assert!(matches!(
            &body.items[1],
            kentos_contracts::FormItem::Tab {
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
            kentos_contracts::Cell::Code {
                code: "ahsap".into(),
            },
        )));
        let _ = app.update(Message::Contingents(Event::Cell(
            5,
            1,
            kentos_contracts::Cell::Code { code: "duz".into() },
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
        .filter(|u| u.how != kentos_contracts::FieldUseHow::Key && !u.kept)
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
        entry(&mut app, entry::Event::OpenRelated(meter));
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
        assert_eq!(kept[0].on_delete, kentos_contracts::RelationDelete::Cascade);

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
                kentos_contracts::CAD_ENTITIES_DELETE,
                kentos_contracts::CallOp::Execute,
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
}
