//! Form scenes of lists and sources: listed buildings, value lists, chosen facilities, sources picked on
//! the map, meters, reviewers' notes, releases, attachments, searches and located, dated and surveyed parcels.

use super::*;
use crate::gis;

/// Buildings and depots whose material comes from the project's list “Yapı malzemesi” and whose floor
/// count from the range “Kat sayısı” (docs/adr/0430); a depot is of masonry (ym).
pub(super) fn listed_buildings(app: &mut App) {
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
    app.gis.features.layer = Some("bina".into());
}

/// Değer listeleri on the material list (docs/adr/0430 §4): Yığma dropped, its records moved to Betonarme.
pub(super) fn domains_window(app: &mut App) {
    use crate::forms::domains::Event;
    listed_buildings(app);
    run(app, "data.domains");
    let _ = app.update(Message::Gis(gis::Event::Domains(Event::RemoveCode(2))));
    let _ = app.update(Message::Gis(gis::Event::Domains(Event::Recode(
        "ym".into(),
        Some("bt".into()),
    ))));
}

/// Alanlar's value list of the building's material: linked to the project's list, its codes read-only.
pub(super) fn domains_field(app: &mut App) {
    listed_buildings(app);
    let _ = app.open_layer_fields("bina");
    let _ = app.update(Message::Gis(gis::Event::LayerFields(
        crate::layer_fields::Event::OpenList(1),
    )));
}

/// Facilities with a province from a long list and the services they have (docs/adr/0431): the province
/// searched, the services as chips; a published form.
pub(super) fn chosen_facilities(app: &mut App) {
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
    let typed: kentos_commands::FormBody = serde_json::from_value(body.clone()).expect("a body");
    let form = json!({ "id": "tesis", "name": "Tesis kaydı", "layer": "tesis", "draft": body.clone(),
        "releases": [{ "version": 1, "digest": kentos_commands::body_digest(&typed), "body": body }] });
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms = vec![serde_json::from_value(form).expect("a form")];
    doc.model.set_settings(settings);
    app.gis.features.layer = Some("tesis".into());
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
}

/// Parcels and buildings (docs/adr/0439): a building's parcel chosen from the parcels holding it, their
/// area, owner and day shown beside it and under it, copied into the building with a stamp; the
/// neighbourhoods searched a page at a time; the project's value lists.
pub(crate) fn sources(app: &mut App) {
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
    let typed: kentos_commands::FormBody = serde_json::from_value(body.clone()).expect("a body");
    let form = json!({ "id": "bina", "name": "Bina", "layer": "bina", "draft": body.clone(),
        "releases": [{ "version": 1, "digest": kentos_commands::body_digest(&typed), "body": body }] });
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
    app.gis.features.layer = Some("bina".into());
}

/// Formlar on the building form, its parcel item chosen: the list's order, columns, page, duplicates,
/// geometry, copies and stamp.
pub(super) fn sources_designer(app: &mut App) {
    sources(app);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("parsel".into()));
}

/// Formla düzenle on the first building: the parcels holding it offered with their columns, 101/5
/// chosen, its area, owner and day under it, copied into the building with the stamp.
pub(super) fn sources_entry(app: &mut App) {
    sources(app);
    app.selection.set([Slot(5)]);
    run(app, "data.formEdit");
    entry(app, entry::Event::Value("Parsel".into(), "101/5".into()));
}

/// Formla düzenle on the first building with Haritadan seç beside its parcel (docs/adr/0759 §1).
pub(super) fn map_pick_offered(app: &mut App) {
    sources(app);
    app.selection.set([Slot(5)]);
    run(app, "data.formEdit");
}

/// Haritadan seç pressed: the window steps aside, the prompt asks for a parcel; the pointer over
/// 101/5 lights it.
pub(super) fn map_picking(app: &mut App) {
    map_pick_offered(app);
    entry(app, entry::Event::PickOnMap("parsel".into()));
    crate::tools_scenes::hover(app, [3.0, 21.0]);
}

/// The parcel holding the building clicked: its value, copies and stamp in the form.
pub(super) fn map_picked(app: &mut App) {
    map_pick_offered(app);
    entry(app, entry::Event::PickOnMap("parsel".into()));
    crate::tools_scenes::click(app, [3.0, 21.0]);
}

/// The neighbouring parcel clicked: it does not hold the building; the value stays, why is said.
pub(super) fn map_apart(app: &mut App) {
    map_pick_offered(app);
    entry(app, entry::Event::PickOnMap("parsel".into()));
    crate::tools_scenes::click(app, [45.0, 12.0]);
}

/// The neighbourhoods searched: “ka” typed, a page of suggestions with their district, Daha fazla.
pub(super) fn sources_search(app: &mut App) {
    sources(app);
    app.selection.set([Slot(6)]);
    run(app, "data.formEdit");
    searched(app, "Mahalle", "a");
    let _ = app.update(Message::Gis(gis::Event::FormEntry(entry::Event::More(
        "Mahalle".into(),
    ))));
    searched(app, "Mahalle", "ka");
}

/// Değer listeleri with İçe aktar…, Dışa aktar… and Aynı kimlikli listeleri değiştir.
pub(super) fn sources_domains(app: &mut App) {
    sources(app);
    run(app, "data.domains");
}

/// Alanlar on the parcels after Değerleri biçime çevir: values typed before the fields were (an
/// import's 450,5, 01.03.2024) in their canonical texts, what did not fit said.
pub(super) fn sources_migrated(app: &mut App) {
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
    let _ = app.update(Message::Gis(gis::Event::LayerFields(
        crate::layer_fields::Event::Migrate,
    )));
}

/// Buildings and their meters (docs/adr/0440): a project relation, sixty meters of the first building
/// shown as cards a page at a time, the meters' own form, the building's workflow logging its snapshot.
pub(super) fn meters(app: &mut App) {
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
        let typed: kentos_commands::FormBody =
            serde_json::from_value(body.clone()).expect("a body");
        json!({ "id": id, "name": name, "layer": layer, "draft": body.clone(),
            "releases": [{ "version": 1, "digest": kentos_commands::body_digest(&typed), "body": body.clone() }] })
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
    app.gis.features.layer = Some("bina".into());
}

/// Formla düzenle on the first building: its meters as cards, 1–50 of 60.
pub(super) fn meters_cards(app: &mut App) {
    meters(app);
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
}

/// The second building: a free meter linked and the name typed, both waiting for Kaydet.
pub(super) fn meters_pending(app: &mut App) {
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
pub(super) fn meters_nested(app: &mut App) {
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
    entry(app, entry::Event::OpenRelated("sayaclar".into(), child));
}

/// Formlar on the building form, its meters item chosen: İlişki, Görünüm Kartlar, the columns; Başvuru.
pub(super) fn meters_designer(app: &mut App) {
    meters(app);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("sayaclar".into()));
}

/// The first building approved, then one of its meters changed: the step's snapshot is not the record's.
pub(super) fn meters_snapshot(app: &mut App) {
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
pub(crate) fn meters_rebodied(app: &mut App, items: Value) {
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
    form.releases = vec![kentos_commands::FormRelease {
        fields: None,
        version: 1,
        digest: kentos_commands::body_digest(&body),
        body,
        note: None,
    }];
    doc.model.set_settings(settings);
    app.gis.form_memory.clock = Some("2026-10-17T10:00:00".into());
    app.gis.form_memory.person = Some("Ayşe Yılmaz".into());
}

/// The building items of ADR 0441's pictures: a divider, the meters as a chart by kind, an action.
pub(crate) fn meters_items() -> Value {
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
pub(super) fn meters_chart(app: &mut App) {
    meters_rebodied(app, meters_items());
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
}

/// Formlar on the building form, İlişkiler's palette shown (docs/adr/0441 §1).
pub(super) fn meters_palette(app: &mut App) {
    meters_rebodied(app, meters_items());
    run(app, "data.forms");
    designer(app, designer::Event::Palette(designer::Palette::Relations));
}

/// The action chosen: its button's words and the processing tool picked from the registered ones.
pub(super) fn meters_action(app: &mut App) {
    meters_rebodied(app, meters_items());
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("hesapla".into()));
}

/// A section library in a folder of its own for the pictures and tests.
pub(crate) fn scene_library(app: &mut App, name: &str) {
    let dir = std::env::temp_dir().join(format!("kentos-form-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    app.gis.form_files = crate::forms::library::FormFiles::at(&dir);
}

/// The chart kept twice in the section library, the meter form's palette showing how it binds there.
pub(super) fn meters_sections(app: &mut App) {
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
pub(super) fn meters_review(app: &mut App) {
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
pub(super) fn meters_kept(app: &mut App) {
    meters_rebodied(app, meters_items());
    scene_library(app, "taslak");
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("ad".into()));
    designer(app, designer::Event::AddDivider);
    app.gis.forms_window = None;
    app.dialog = None;
    run(app, "data.forms");
}

/// The inspection buildings with a notes field the workflow's steps write to (docs/adr/0442 §4).
pub(super) fn noted_buildings(app: &mut App) {
    workflow_buildings(app);
    let doc = app.document.as_mut().expect("a drawing");
    let mut fields = doc.model.layers().get("bina").expect("bina").fields.clone();
    fields.push(kentos_commands::LayerField::new(
        "Notlar",
        kentos_commands::LayerFieldKind::Text,
    ));
    let _ = doc.model.set_layer_fields("bina", fields, &[]);
    let mut settings = doc.model.settings().clone();
    let form = settings.forms.first_mut().expect("the form");
    if let Some(w) = form.draft.workflow.as_mut() {
        w.notes = Some("Notlar".into());
    }
    let body = form.draft.clone();
    form.releases = vec![kentos_commands::FormRelease {
        fields: None,
        version: 1,
        digest: kentos_commands::body_digest(&body),
        body,
        note: None,
    }];
    doc.model.set_settings(settings);
    app.gis.form_memory.clock = Some("2026-10-17T10:00:00".into());
    app.gis.form_memory.person = Some("Ayşe Yılmaz".into());
}

/// The sent building under review: Alanlara not on, notes on two fields, the reason typed (docs/adr/0442 §4).
pub(super) fn reviewer_notes(app: &mut App) {
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
pub(super) fn reviewer_notes_shown(app: &mut App) {
    reviewer_notes(app);
    entry(app, entry::Event::Transition("Geri gönder".into()));
    app.selection.set([Slot(2)]);
    run(app, "data.formEdit");
}

/// The form published again while a record was being typed into: Yeni sürümle aç (docs/adr/0442 §2).
pub(super) fn new_release(app: &mut App) {
    workflow_buildings(app);
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
    entry(app, entry::Event::Value("Kat".into(), "4".into()));
    let doc = app.document.as_mut().expect("a drawing");
    let mut body = doc.model.settings().forms[0].draft.clone();
    body.items.retain(|i| i.id() != "gerekce");
    let _ = kentos_native_application::forms_edit::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        kentos_commands::FormsEdit {
            edit: kentos_commands::FormsOp::Update {
                id: "denetim".into(),
                body,
            },
            expected_revision: None,
        },
    );
    let _ = kentos_native_application::forms_edit::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        kentos_commands::FormsEdit {
            edit: kentos_commands::FormsOp::Publish {
                id: "denetim".into(),
                note: None,
            },
            expected_revision: None,
        },
    );
    entry(app, entry::Event::Save);
}

/// Yayınla over records the new release breaks: asked first (docs/adr/0442 §2).
pub(super) fn breaking_publish(app: &mut App) {
    workflow_buildings(app);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("gerekce".into()));
    designer(app, designer::Event::Required(true));
    designer(app, designer::Event::Publish);
}

/// An action that takes a workflow step, its step picked (docs/adr/0442 §5).
pub(super) fn step_action(app: &mut App) {
    workflow_buildings(app);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("kat".into()));
    designer(app, designer::Event::AddAction);
    designer(
        app,
        designer::Event::ActionKind(kentos_commands::ActionKind::Step),
    );
    designer(
        app,
        designer::Event::Text(designer::Text::Label, "Gönder".into()),
    );
}

/// Değişiklikler with the areas each change touches and what the draft newly needs (docs/adr/0442 §3).
pub(super) fn change_areas(app: &mut App) {
    workflow_buildings(app);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("kat".into()));
    designer(app, designer::Event::Required(false));
    designer(app, designer::Event::AddLocation);
    designer(app, designer::Event::AddAction);
    designer(app, designer::Event::Changes);
}

/// The first building's attachments read by `cad.attachments.inspect`: Bilgi (docs/adr/0442 §7).
pub(super) fn attachment_facts(app: &mut App) {
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
pub(super) fn searched_list(app: &mut App) {
    chosen_facilities(app);
    searched(app, "İl", "an");
}

/// `text` typed into the searched list of `key` and its turn come (docs/adr/0439 §6): what the pause
/// after typing delivers.
pub(super) fn searched(app: &mut App, key: &str, text: &str) {
    let _ = app.update(Message::Gis(gis::Event::FormEntry(entry::Event::Query(
        key.into(),
        text.into(),
    ))));
    let turn = app
        .gis
        .form_entry
        .as_ref()
        .and_then(|e| e.search_for_tests(key).0);
    if let Some(turn) = turn {
        let _ = app.update(Message::Gis(gis::Event::FormEntry(entry::Event::Search(
            key.into(),
            turn,
        ))));
    }
}

/// A drafted building with its floor count emptied: kept incomplete, Gönder asks for it (docs/adr/0432).
pub(super) fn workflow_draft(app: &mut App) {
    workflow_buildings(app);
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
    let _ = app.update(Message::Gis(gis::Event::FormEntry(entry::Event::Value(
        "Kat".into(),
        String::new(),
    ))));
}

/// A parcel's form with its place (docs/adr/0433): an L-shaped parcel on the small map, its centre's
/// coordinates, area and perimeter.
pub(super) fn located_parcel(app: &mut App) {
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
    let typed: kentos_commands::FormBody = serde_json::from_value(body.clone()).expect("a body");
    let form = json!({ "id": "parsel", "name": "Parsel kaydı", "layer": "kadastro", "draft": body.clone(),
        "releases": [{ "version": 1, "digest": kentos_commands::body_digest(&typed), "body": body }] });
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms = vec![serde_json::from_value(form).expect("a form")];
    doc.model.set_settings(settings);
    app.gis.features.layer = Some("kadastro".into());
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
}

/// The inspection form taken as a package onto the parcels' layer (docs/adr/0435): its fields added there,
/// the new form chosen in Formlar.
pub(super) fn imported_form(app: &mut App) {
    workflow_buildings(app);
    run(app, "data.forms");
    let text = app.form_package_text().expect("a package");
    if let Some(doc) = app.document.as_mut() {
        doc.model.set_active_layer("parsel");
    }
    if let Some(w) = app.gis.forms_window.as_mut() {
        w.chosen = None;
    }
    app.form_imported(Some(("bina-denetimi.kform".into(), text)));
}

/// A parcel's registration date out of its range (docs/adr/0436): the date field takes 2000-01-01 to
/// 2026-12-31, and 1.1.2027 is typed.
pub(super) fn dated_parcel(app: &mut App) {
    located_parcel(app);
    let doc = app.document.as_mut().expect("a drawing");
    let mut fields = doc
        .model
        .layers()
        .get("kadastro")
        .expect("kadastro")
        .fields
        .clone();
    fields.push(kentos_commands::LayerField {
        min: Some("2000-01-01".into()),
        max: Some("2026-12-31".into()),
        ..kentos_commands::LayerField::new("Tescil", kentos_commands::LayerFieldKind::Date)
    });
    let _ = doc.model.set_layer_fields("kadastro", fields, &[]);
    let mut settings = doc.model.settings().clone();
    if let Some(f) = settings.forms.first_mut()
        && let Some(kentos_commands::FormItem::Group { items, .. }) = f.draft.items.first_mut()
    {
        items.push(
            serde_json::from_value(json!({ "type": "field", "id": "tescil", "field": "Tescil" }))
                .expect("an item"),
        );
        let body = f.draft.clone();
        f.releases = vec![kentos_commands::FormRelease {
            fields: None,
            version: 1,
            digest: kentos_commands::body_digest(&body),
            body,
            note: None,
        }];
    }
    doc.model.set_settings(settings);
    app.gis.form_entry = None;
    app.dialog = None;
    run(app, "data.formEdit");
    let _ = app.update(Message::Gis(gis::Event::FormEntry(entry::Event::Value(
        "Tescil".into(),
        "1.1.2027".into(),
    ))));
}

/// A survey form of three steps (docs/adr/0437): Kimlik with the use as a tree, Yapı, and Özet showing the
/// name and the use again as mirrors.
pub(super) fn stepped_survey(app: &mut App) {
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
    let typed: kentos_commands::FormBody = serde_json::from_value(body.clone()).expect("a body");
    let form = json!({ "id": "yapi", "name": "Yapı tespiti", "layer": "yapi", "draft": body.clone(),
        "releases": [{ "version": 1, "digest": kentos_commands::body_digest(&typed), "body": body }] });
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms = vec![serde_json::from_value(form).expect("a form")];
    doc.model.set_settings(settings);
    app.gis.features.layer = Some("yapi".into());
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
}
