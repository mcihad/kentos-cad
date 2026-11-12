//! Form scenes on buildings: fields and calculations, trees and owners, timed and styled records, the
//! left panel, units and related sums, the related records' actions and workflows.

use super::*;

/// The buildings' fields: a status from a list (required), the storeys by a stable id, a note, a date
/// and a yes/no.
pub(crate) fn fields() -> Value {
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
        let typed: kentos_commands::FormBody =
            serde_json::from_value(body.clone()).expect("a body");
        f["releases"] = json!([{ "version": 1, "digest": kentos_commands::body_digest(&typed), "body": body, "note": "İlk yayın" }]);
    }
    f
}

/// The calculating form's fields (as `scripts/fixtures/forms_cases.py`'s `CALC_FIELDS`).
pub(super) fn calc_fields() -> Value {
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
pub(super) fn calc_form() -> Value {
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
    let typed: kentos_commands::FormBody = serde_json::from_value(body.clone()).expect("a body");
    json!({ "id": "bina-hesabi", "name": "Bina hesabı", "layer": "bina", "draft": body.clone(), "enforce": true,
            "releases": [{ "version": 1, "digest": kentos_commands::body_digest(&typed), "body": body }] })
}

/// A file of the shared fixtures, by its path from the repository's root.
pub(super) fn fixture(path: &str) -> String {
    format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR"))
}

/// The tree inventory (docs/adr/0407; `EXTRA_BODY` of `scripts/fixtures/forms_cases.py`): the name
/// offered from the layer, the colour picked, the kind listed, the height, the photos counted (one to
/// three) and the documents shown for a tall tree.
pub(super) fn tree_form() -> Value {
    let body = json!({ "revision": 1, "items": [
        { "type": "field", "id": "ad", "field": "Ad", "suggest": true },
        { "type": "field", "id": "renk", "field": "Renk", "widget": "color", "default": "#2E7D32" },
        { "type": "field", "id": "cins", "field": "Cins" },
        { "type": "field", "id": "boy", "field": "Boy" },
        { "type": "field", "id": "bakim", "field": "Bakım" },
        { "type": "attachments", "id": "fotograflar", "label": "Fotoğraflar", "accept": "photo", "min": 1, "max": 3 },
        { "type": "attachments", "id": "belgeler", "label": "Belgeler", "accept": "document", "visibleWhen": "Boy > 10" }
    ]});
    let typed: kentos_commands::FormBody = serde_json::from_value(body.clone()).expect("a body");
    json!({ "id": "agac", "name": "Ağaç envanteri", "layer": "agac", "draft": body.clone(),
            "releases": [{ "version": 1, "digest": kentos_commands::body_digest(&typed), "body": body }] })
}

/// A park's trees under the inventory form: the first with two photos and a report, the second
/// without a photo, the others named for the suggestions.
pub(super) fn trees(app: &mut App) {
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
pub(super) fn tree_editing(app: &mut App) {
    trees(app);
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
}

/// The second tree: no photo yet, and a name being typed: the layer's names offered.
pub(super) fn tree_missing(app: &mut App) {
    trees(app);
    app.selection.set([Slot(2)]);
    run(app, "data.formEdit");
    entry(app, entry::Event::Value("Ad".into(), "ı".into()));
}

/// The name field focused: the layer's names holding what is typed are offered (docs/adr/0407 §3).
pub(super) fn name_focused(s: &mut kentos_ui::snapshot::Snapshot, app: &mut App) {
    focused(s, app, "Ad");
}

/// The province field focused: the provinces holding “an” are offered (docs/adr/0431).
pub(super) fn province_focused(s: &mut kentos_ui::snapshot::Snapshot, app: &mut App) {
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
        ("form-cikinca-tasarim", left_designer, properties_bottom),
        (
            "form-iliskili-tasarim",
            related_sums_designer,
            properties_down,
        ),
        ("form-eylemler-tasarim", actions_designer, properties_down),
    ]
}

pub(super) fn neighbourhood_focused(s: &mut kentos_ui::snapshot::Snapshot, app: &mut App) {
    focused(s, app, "Mahalle");
}

/// The designer's properties scrolled to the list's extras (docs/adr/0439).
pub(super) fn properties_down(s: &mut kentos_ui::snapshot::Snapshot, app: &mut App) {
    properties_at(s, app, 0.55);
}

/// The designer's properties scrolled to their end: a field's checks (docs/adr/0760).
pub(super) fn properties_bottom(s: &mut kentos_ui::snapshot::Snapshot, app: &mut App) {
    properties_at(s, app, 1.0);
}

pub(super) fn properties_at(s: &mut kentos_ui::snapshot::Snapshot, app: &mut App, y: f32) {
    use iced::futures::StreamExt as _;
    let task: iced::Task<Message> = iced::widget::operation::snap_to(
        crate::forms::designer_view::PROPERTIES_SCROLL,
        iced::widget::scrollable::RelativeOffset { x: 0.0, y },
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
pub(super) fn published(id: &str, name: &str, layer: &str, items: Value) -> Value {
    let body = json!({ "revision": 1, "items": items });
    let typed: kentos_commands::FormBody = serde_json::from_value(body.clone()).expect("a body");
    json!({ "id": id, "name": name, "layer": layer, "draft": body.clone(),
            "releases": [{ "version": 1, "digest": kentos_commands::body_digest(&typed), "body": body }] })
}

/// Three buildings and two owners (docs/adr/0409): an owner holds several buildings by their ids
/// (a many-to-many field); a building's form lists its owners; the relation unlinks. The owners are
/// records without a geometry, a table's rows (docs/adr/0410).
pub(super) fn owners(app: &mut App) {
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
                kentos_commands::CAD_ENTITIES_SET,
                kentos_commands::CallOp::Execute,
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
pub(super) fn owner_editing(app: &mut App) {
    owners(app);
    app.selection.set([Slot(4)]);
    run(app, "data.formEdit");
}

/// The second building by its form: its two owners, Yeni and Bağla.
pub(super) fn building_owners(app: &mut App) {
    owners(app);
    app.selection.set([Slot(2)]);
    run(app, "data.formEdit");
}

/// A damage survey's moment and shift (docs/adr/0412): a date and time field and a time field, by
/// their pickers in Formla düzenle.
pub(super) fn timed(app: &mut App) {
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
pub(super) fn styled_buildings(app: &mut App) {
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
pub(super) fn buildings_with_a_form(app: &mut App) {
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

/// The buildings' survey form with checks kept for leaving the field (docs/adr/0760): a warning past
/// ten storeys and a short note refused once the field is left; the form opened on B-4.
pub(crate) fn left_checks(app: &mut App) {
    styled_buildings(app);
    let form = published(
        "hasar",
        "Hasar tespiti",
        "bina",
        json!([
            { "type": "field", "id": "durum", "field": "Durum" },
            { "type": "field", "id": "kat", "field": "Kat", "label": "Kat sayısı",
              "checks": [{ "rule": "Kat < 40", "message": "Kat 40'tan az olmalı." },
                         { "rule": "Kat <= 10", "message": "10 kattan yüksek binada ruhsat ayrıca denetlenir.",
                           "warning": true, "on": "leave" }] },
            { "type": "field", "id": "not", "field": "Not", "label": "Not",
              "checks": [{ "rule": "uzunluk([Not]) >= 10", "message": "Not en az 10 karakter olmalı.", "on": "leave" }] }
        ]),
    );
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms = vec![serde_json::from_value(form).expect("a form")];
    doc.model.set_settings(settings);
    let _ = app.update(Message::Run("data.featureTable"));
    app.selection.set([Slot(4)]);
    run(app, "data.formEdit");
}

/// Formlar on the survey form, its storeys chosen: the checks with Ne zaman, one asked on leaving.
pub(super) fn left_designer(app: &mut App) {
    left_checks(app);
    let _ = app.update(Message::FormEntry(entry::Event::Cancel));
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("kat".into()));
}

/// Twelve storeys typed, the field still being typed in: no word under it yet.
pub(super) fn left_typing(app: &mut App) {
    left_checks(app);
    entry(app, entry::Event::Value("Kat".into(), "12".into()));
    entry(app, entry::Event::Value("Not".into(), "kısa".into()));
}

/// The storeys and the note left: the warning and the note's error under them.
pub(super) fn left_done(app: &mut App) {
    left_typing(app);
    entry(app, entry::Event::Focus("kat".into(), false));
    entry(app, entry::Event::Focus("not".into(), false));
}

/// Alanlar on the buildings: the floor count chosen and being renamed, its places listed.
pub(super) fn field_usage(app: &mut App) {
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
pub(super) fn form_view(app: &mut App) {
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
pub(super) fn retired_state(app: &mut App) {
    buildings_with_a_form(app);
    let doc = app.document.as_mut().expect("a drawing");
    let mut fields = doc.model.layers().get("bina").expect("bina").fields.clone();
    for c in fields[1].values.iter_mut().flatten() {
        c.retired = c.code == "orta";
    }
    let _ = doc.model.set_layer_fields("bina", fields, &[]);
}

/// Alanlar's value list of the state: Orta hasarlı retired.
pub(super) fn retired_list(app: &mut App) {
    retired_state(app);
    let _ = app.open_layer_fields("bina");
    let _ = app.update(Message::LayerFields(crate::layer_fields::Event::OpenList(
        1,
    )));
}

/// Buildings sorted by their kind (docs/adr/0422): Tür is the subtype field; an industrial building
/// is concrete or steel (steel by default) and has a note; a depot is concrete or stone. A published
/// form shows the four fields.
pub(super) fn typed_buildings(app: &mut App) {
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
pub(super) fn units_table(app: &mut App) {
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
            kentos_commands::CAD_ENTITIES_SET,
            kentos_commands::CallOp::Execute,
            json!({ "uids": children, "attrs": { "Bina": parent }, "operation": "attributes" }),
        );
    }
    app.selection.set([Slot(building)]);
    run(app, "data.formEdit");
}

/// A building whose form counts its units, sums their areas and lists their owners from the relation
/// (docs/adr/0761): the four units linked by a command, the document worked the building out in the same
/// step; a fifth unit stands unlinked.
pub(crate) fn related_sums(app: &mut App) {
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
        ("5", "3", "64.30", "Elif Şahin"),
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
    let mut form = published(
        "bina-formu",
        "Bina",
        "bina",
        json!([
            { "type": "field", "id": "kod", "field": "Kod" },
            { "type": "group", "id": "ozet", "label": "Bölümlerden", "columns": 2, "items": [
                { "type": "field", "id": "sayi", "field": "Bölüm sayısı",
                  "calculate": "ilişkili_toplam('Bağımsız bölümler', 'say')" },
                { "type": "field", "id": "alan", "field": "Toplam alan",
                  "calculate": "ilişkili_toplam('Bağımsız bölümler', 'topla', Alan)" },
                { "type": "field", "id": "malikler", "field": "Malikler",
                  "calculate": "ilişkili_toplam('Bağımsız bölümler', 'değerleri_birleştir', Malik)" },
                { "type": "field", "id": "ust", "field": "Üst kattakiler",
                  "calculate": "ilişkili_toplam('Bağımsız bölümler', 'say', No, Kat >= 2)" } ] },
            { "type": "related", "id": "bolumler", "label": "Bağımsız bölümler", "layer": "bolum", "field": "Bina",
              "view": "table", "columns": ["No", "Kat", "Alan", "Malik"] }
        ]),
    );
    form["enforce"] = json!(true);
    open_edited(app, o, move |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut bina = crate::tools_scenes::layer("bina", "Bina", "#3E63DD", 0.35);
        bina["fields"] = json!([{ "name": "Kod", "kind": "text" },
                                { "name": "Bölüm sayısı", "kind": "integer" },
                                { "name": "Toplam alan", "kind": "decimal", "scale": 2 },
                                { "name": "Malikler", "kind": "text" },
                                { "name": "Üst kattakiler", "kind": "integer" }]);
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
    link_units(app, building, &units[..4]);
    app.selection.set([Slot(building)]);
    run(app, "data.formEdit");
}

/// A building's form whose owners item offers `actions` (docs/adr/0762), the owners' layer locked
/// when `locked`; the owners have a form of their own. Two owners linked, one free to link.
pub(crate) fn owner_actions(app: &mut App, actions: Value, locked: bool) {
    let mut o = Objects::new();
    let building = o.path(
        "bina",
        &[[0.0, 0.0], [24.0, 0.0], [24.0, 16.0], [0.0, 16.0]],
        true,
    );
    o.data(building, &[("Kod", "B-1")], Some("B-1"));
    let mut owners = Vec::new();
    for (name, share) in [
        ("Ayşe Yılmaz", "1/2"),
        ("Mehmet Kaya", "1/2"),
        ("Zeynep Arslan", ""),
    ] {
        let id = o.record("malik");
        o.data(id, &[("Ad", name), ("Hisse", share)], Some(name));
        owners.push(id);
    }
    let mut related = json!({ "type": "related", "id": "malikler", "label": "Malikler", "layer": "malik",
                              "field": "Bina" });
    if !actions.is_null() {
        related["actions"] = actions;
    }
    let form = published(
        "bina-formu",
        "Bina",
        "bina",
        json!([{ "type": "field", "id": "kod", "field": "Kod" }, related]),
    );
    let owner_form = published(
        "malik-formu",
        "Malik",
        "malik",
        json!([{ "type": "field", "id": "ad", "field": "Ad" },
               { "type": "field", "id": "hisse", "field": "Hisse" }]),
    );
    open_edited(app, o, move |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut bina = crate::tools_scenes::layer("bina", "Bina", "#3E63DD", 0.35);
        bina["fields"] = json!([{ "name": "Kod", "kind": "text" }]);
        let mut malik = crate::tools_scenes::layer("malik", "Malik", "#E5484D", 0.35);
        malik["fields"] = json!([{ "name": "Ad", "kind": "text" }, { "name": "Hisse", "kind": "text" },
                                 { "name": "Bina", "kind": "text" }]);
        layers.push(bina);
        layers.push(malik);
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("bina");
        d["settings"]["forms"] = json!([form, owner_form]);
        d["settings"]["relations"] = json!([{ "id": "bina-malik", "name": "Malikler",
                                              "parent": "bina", "child": "malik", "field": "Bina" }]);
    });
    link_units(app, building, &owners[..2]);
    // Locked after the links are written: a locked layer takes no link.
    if locked && let Some(doc) = app.document.as_mut() {
        let _ = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            kentos_commands::CAD_LAYERS_SET,
            kentos_commands::CallOp::Execute,
            json!({ "operation": "locked", "layer": "malik", "value": true }),
        );
    }
    app.selection.set([Slot(building)]);
    run(app, "data.formEdit");
}

/// A building's form listing from five layers whose sources stand each a way (docs/adr/0763): a local list, a
/// GeoPackage read whose file changed since, one whose file moved away, a database nobody listens on and one that
/// never answers. Formla düzenle says each apart with its way out.
pub(crate) fn sources_states(app: &mut App) {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/geopackage/v1");
    let mut o = Objects::new();
    let building = o.path(
        "bina",
        &[[0.0, 0.0], [24.0, 0.0], [24.0, 16.0], [0.0, 16.0]],
        true,
    );
    o.data(building, &[("Kod", "B-7")], Some("B-7"));
    for (layer, rows) in [
        ("mahalle", &[("01", "Kızılay"), ("02", "Bahçelievler")][..]),
        (
            "malik",
            &[("M-1", "Ayşe Yılmaz"), ("M-2", "Mehmet Kaya")][..],
        ),
        ("parsel", &[("101/5", "101 ada 5 parsel")][..]),
        ("tapu", &[("T-44", "Tapu 44")][..]),
        ("ruhsat", &[("R-9", "Ruhsat 9")][..]),
    ] {
        for (code, name) in rows {
            let id = o.record(layer);
            o.data(id, &[("kod", code), ("ad", name)], Some(name));
        }
    }
    let list = |id: &str, field: &str, layer: &str, label: &str| {
        json!({ "type": "field", "id": id, "field": field, "label": label,
                "lookup": { "layer": layer, "key": "kod", "label": "ad" } })
    };
    let form = published(
        "bina-formu",
        "Bina",
        "bina",
        json!([{ "type": "field", "id": "kod", "field": "Kod" },
               list("mahalle", "Mahalle", "mahalle", "Mahalle"),
               list("malik", "Malik", "malik", "Malik"),
               list("parsel", "Parsel", "parsel", "Parsel"),
               list("tapu", "Tapu", "tapu", "Tapu kaydı"),
               list("ruhsat", "Ruhsat", "ruhsat", "Yapı ruhsatı")]),
    );
    // A port nobody listens on, and one that takes the connection and never answers.
    let closed = std::net::TcpListener::bind("127.0.0.1:0").expect("port");
    let refused = closed.local_addr().expect("address").port();
    drop(closed);
    let silent = std::net::TcpListener::bind("127.0.0.1:0").expect("port");
    let quiet = silent.local_addr().expect("address").port();
    let link = |id: &str, kind: &str, path: String, table: &str, layer: &str, revision: String| {
        json!({ "id": id, "kind": kind, "path": path, "table": table, "layer": layer, "revision": revision,
                "digest": "f".repeat(64), "srid": 5256, "writable": true })
    };
    let sources = json!([
        link(
            "k-1",
            "geopackage",
            root.join("iliskili.gpkg").display().to_string(),
            "malik",
            "malik",
            "0".repeat(64)
        ),
        link(
            "k-2",
            "geopackage",
            root.join("tasinan-parseller.gpkg").display().to_string(),
            "parsel",
            "parsel",
            "1".repeat(64)
        ),
        link(
            "k-3",
            "postgis",
            format!("postgresql://kentos@127.0.0.1:{refused}/tapu"),
            "public.tapu",
            "tapu",
            "2".repeat(64)
        ),
        link(
            "k-4",
            "postgis",
            format!("postgresql://kentos@127.0.0.1:{quiet}/belediye"),
            "imar.ruhsat",
            "ruhsat",
            "3".repeat(64)
        ),
    ]);
    open_edited(app, o, move |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut bina = crate::tools_scenes::layer("bina", "Bina", "#3E63DD", 0.35);
        bina["fields"] = json!([{ "name": "Kod", "kind": "text" }, { "name": "Mahalle", "kind": "text" },
                                { "name": "Malik", "kind": "text" }, { "name": "Parsel", "kind": "text" },
                                { "name": "Tapu", "kind": "text" }, { "name": "Ruhsat", "kind": "text" }]);
        layers.push(bina);
        for (id, name, colour) in [
            ("mahalle", "Mahalleler", "#30A46C"),
            ("malik", "Malikler", "#E5484D"),
            ("parsel", "Parseller", "#F76B15"),
            ("tapu", "Tapu kayıtları", "#8E4EC6"),
            ("ruhsat", "Ruhsatlar", "#0090FF"),
        ] {
            let mut l = crate::tools_scenes::layer(id, name, colour, 0.35);
            l["fields"] =
                json!([{ "name": "kod", "kind": "text" }, { "name": "ad", "kind": "text" }]);
            layers.push(l);
        }
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("bina");
        d["settings"]["forms"] = json!([form]);
        d["settings"]["sources"] = sources;
    });
    app.selection.set([Slot(building)]);
    run(app, "data.formEdit");
    // The program registers its database provider at start; a picture does it here.
    kentos_postgis::register();
    app.entry_sources_now(std::time::Duration::from_millis(400));
    drop(silent);
}

/// A Veri girişi's database project (docs/adr/0764 §4): every action offered, the owners' form does not
/// show their building field: Değiştir open, Bağla and Bağı kopar quiet with the reason, Yeni and Sil too.
pub(crate) fn actions_entry(app: &mut App) {
    owner_actions(
        app,
        json!([
            "create", "update", "delete", "link", "unlink", "view", "show"
        ]),
        false,
    );
    if let Some(doc) = app.document.as_mut() {
        doc.model
            .set_object_policy(kentos_domain::ObjectPolicy::AttributesOnly);
    }
}

/// Malikler only linked, opened and shown: no Yeni, no Bağı kopar (docs/adr/0762 §1).
pub(super) fn actions_link_only(app: &mut App) {
    owner_actions(app, json!(["link", "view", "show"]), false);
}

/// Every action offered, the owners' layer locked: the writing ones quiet, the reason under them.
pub(super) fn actions_locked(app: &mut App) {
    owner_actions(
        app,
        json!([
            "create", "update", "delete", "link", "unlink", "view", "show"
        ]),
        true,
    );
}

/// Sil offered and pressed: the row asks first (docs/adr/0762 §4).
pub(super) fn actions_delete_asked(app: &mut App) {
    owner_actions(
        app,
        json!([
            "create", "update", "delete", "link", "unlink", "view", "show"
        ]),
        false,
    );
    let first = app
        .form_entry
        .as_ref()
        .expect("the window")
        .related_of("malikler")[0]
        .clone();
    let _ = app.update(Message::FormEntry(entry::Event::DeleteRelated(
        "malikler".into(),
        first,
    )));
}

/// An owner opened from an item that offers reading, not changing: its form read-only.
pub(super) fn actions_view_only(app: &mut App) {
    owner_actions(app, json!(["view", "show"]), false);
    let first = app
        .form_entry
        .as_ref()
        .expect("the window")
        .related_of("malikler")[0]
        .clone();
    let _ = app.update(Message::FormEntry(entry::Event::OpenRelated(
        "malikler".into(),
        first,
    )));
}

/// Formlar on the building form, its owners item chosen: the Eylemler row.
pub(super) fn actions_designer(app: &mut App) {
    owner_actions(app, json!(["link", "view", "show", "delete"]), false);
    let _ = app.update(Message::FormEntry(entry::Event::Cancel));
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("malikler".into()));
}

/// `units` linked to `building` by one command (`cad.entities.set`), as Bağla writes them.
pub(super) fn link_units(app: &mut App, building: u32, units: &[u32]) {
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
            kentos_commands::CAD_ENTITIES_SET,
            kentos_commands::CallOp::Execute,
            json!({ "uids": children, "attrs": { "Bina": parent }, "operation": "attributes" }),
        );
    }
}

/// The fifth unit linked while the building's form is open: the count, the area, the owners and the
/// upper floors' count follow in the same step.
pub(super) fn related_sums_linked(app: &mut App) {
    related_sums(app);
    let last = |layer: &str| {
        app.document
            .as_ref()
            .and_then(|d| {
                d.model
                    .by_layer_with_uids(layer)
                    .map(|(_, e)| e.base().id)
                    .max()
            })
            .unwrap_or(0)
    };
    let (building, fifth) = (last("bina"), last("bolum"));
    link_units(app, building, &[fifth]);
    let _ = app.refresh_entry();
}

/// Formlar on the building form, the units' count chosen: its calculation reading the relation.
pub(super) fn related_sums_designer(app: &mut App) {
    related_sums(app);
    let _ = app.update(Message::FormEntry(entry::Event::Cancel));
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("sayi".into()));
}

/// Formlar on the building form, its units item chosen: Görünüm Tablo and its columns (docs/adr/0425).
pub(super) fn units_designer(app: &mut App) {
    units_table(app);
    let _ = app.update(Message::FormEntry(entry::Event::Cancel));
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("bolumler".into()));
}

/// Formlar on the building form in tabs, its Sanayi tab chosen and shown only for industrial and
/// depot buildings by Alt türlerde göster (docs/adr/0426).
pub(super) fn kind_tabs(app: &mut App) {
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
pub(super) fn form_changes(app: &mut App) {
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
pub(super) fn workflow_buildings(app: &mut App) {
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
    let typed: kentos_commands::FormBody = serde_json::from_value(body.clone()).expect("a body");
    let form = json!({ "id": "denetim", "name": "Bina denetimi", "layer": "bina", "draft": body.clone(),
        "releases": [{ "version": 1, "digest": kentos_commands::body_digest(&typed), "body": body }] });
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms = vec![serde_json::from_value(form).expect("a form")];
    doc.model.set_settings(settings);
    app.features.layer = Some("bina".into());
}

/// Formlar's İş akışı on the inspection form (docs/adr/0428 §4).
pub(super) fn workflow_designer(app: &mut App) {
    workflow_buildings(app);
    run(app, "data.forms");
    designer(app, designer::Event::Flow);
}

/// A sent building's form: its steps Onayla and Geri gönder beside Kaydet.
pub(super) fn workflow_steps(app: &mut App) {
    workflow_buildings(app);
    app.selection.set([Slot(2)]);
    run(app, "data.formEdit");
}

/// An approved building's form: read-only, Düzelt its only step (docs/adr/0428 §2).
pub(super) fn workflow_locked(app: &mut App) {
    workflow_buildings(app);
    app.selection.set([Slot(3)]);
    run(app, "data.formEdit");
}

/// A returned building's form: its log of steps, read-only, and Gönder again (docs/adr/0429).
pub(super) fn workflow_log(app: &mut App) {
    workflow_buildings(app);
    app.selection.set([Slot(4)]);
    run(app, "data.formEdit");
}
