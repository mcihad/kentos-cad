//! Form scenes of permits, parcel indexes, contingents and subtypes, scenarios, field policies, floors,
//! owner tables, calculated buildings, addresses, relations, record audits and sliders.

use super::*;
use crate::gis;

/// Ruhsatlı yapılar (docs/adr/0438): a layer with record rules (dates in order, a note for an
/// inspection, floors and area), a unique parcel, a floor count whose 0 is left out, an area in m², a
/// status list with a value's description and days; a form whose note check is asked on saving.
pub(super) fn permits(app: &mut App) {
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
    let typed: kentos_commands::FormBody = serde_json::from_value(body.clone()).expect("a body");
    let form = json!({ "id": "ruhsat", "name": "Ruhsat", "layer": "ruhsat", "draft": body.clone(),
        "releases": [{ "version": 1, "digest": kentos_commands::body_digest(&typed), "body": body }] });
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms = vec![serde_json::from_value(form).expect("a form")];
    doc.model.set_settings(settings);
    app.gis.features.layer = Some("ruhsat".into());
}

/// Four parcels on two islands with a key over the island and the parcel, a plain index over the neighbourhood and a
/// key over the building number taking empty values as the same (docs/adr/0758).
pub(super) fn keyed_parcels(app: &mut App) {
    let mut o = Objects::new();
    let rows: [(&str, &str, &str, &str); 4] = [
        ("101", "5", "Merkez", "B1"),
        ("101", "6", "Merkez", "B2"),
        ("102", "5", "Yalı", "B3"),
        ("103", "", "Yalı", ""),
    ];
    for (i, (ada, parsel, mahalle, bina)) in rows.iter().enumerate() {
        let x = 22.0 * i as f64;
        let id = o.path(
            "parsel",
            &[[x, 0.0], [x + 18.0, 0.0], [x + 18.0, 14.0], [x, 14.0]],
            true,
        );
        let mut values = vec![("Ada", *ada), ("Mahalle", *mahalle)];
        if !parsel.is_empty() {
            values.push(("Parsel", *parsel));
        }
        if !bina.is_empty() {
            values.push(("Bina no", *bina));
        }
        let label = if parsel.is_empty() {
            (*ada).to_owned()
        } else {
            format!("{ada}/{parsel}")
        };
        o.data(id, &values, Some(label.as_str()));
    }
    open_edited(app, o, |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut parsel = crate::tools_scenes::layer("parsel", "Parsel", "#0E7490", 0.35);
        parsel["fields"] = json!([
            { "name": "Ada", "kind": "integer" },
            { "name": "Parsel", "kind": "integer" },
            { "name": "Mahalle", "kind": "text" },
            { "id": "bina-no", "name": "Bina no", "kind": "text" },
            { "name": "Alan", "kind": "decimal", "scale": 2, "unit": "m²" }]);
        parsel["indexes"] = json!([
            { "name": "Ada ve parsel", "fields": ["Ada", "Parsel"], "unique": true },
            { "name": "Mahalle", "fields": ["Mahalle"] },
            { "name": "Bina", "fields": ["bina-no"], "unique": true, "nullsEqual": true }]);
        layers.push(parsel);
        d["layers"] = Value::Array(layers);
        d["activeLayer"] = json!("parsel");
    });
    app.gis.features.layer = Some("parsel".into());
}

/// Dizinler on the parcels, the key over the island and the parcel chosen.
pub(super) fn parcel_indexes(app: &mut App) {
    keyed_parcels(app);
    run(app, "data.layerIndexes");
    let _ = app.update(Message::Gis(gis::Event::LayerIndexes(
        crate::forms::layer_indexes::Event::Choose(0),
    )));
}

/// Dizinler: the neighbourhood made unique, which two parcels already break; Kaydet is off.
pub(super) fn parcel_indexes_broken(app: &mut App) {
    keyed_parcels(app);
    run(app, "data.layerIndexes");
    use crate::forms::layer_indexes::Event as I;
    let _ = app.update(Message::Gis(gis::Event::LayerIndexes(I::Choose(1))));
    let _ = app.update(Message::Gis(gis::Event::LayerIndexes(I::Unique(true))));
}

/// Dizinler: a new index over the neighbourhood and the area, the fields put in by their chips.
pub(super) fn parcel_index_new(app: &mut App) {
    keyed_parcels(app);
    run(app, "data.layerIndexes");
    use crate::forms::layer_indexes::Event as I;
    let _ = app.update(Message::Gis(gis::Event::LayerIndexes(I::New)));
    let _ = app.update(Message::Gis(gis::Event::LayerIndexes(I::Name(
        "Mahalle ve alan".into(),
    ))));
    let _ = app.update(Message::Gis(gis::Event::LayerIndexes(I::ToggleField(
        "Mahalle".into(),
    ))));
    let _ = app.update(Message::Gis(gis::Event::LayerIndexes(I::ToggleField(
        "Alan".into(),
    ))));
}

/// Kayıt kuralları on the permits, the dates' rule chosen.
pub(super) fn permit_rules(app: &mut App) {
    permits(app);
    run(app, "data.layerRules");
    let _ = app.update(Message::Gis(gis::Event::LayerRules(
        crate::forms::layer_rules::Event::Choose(2),
    )));
}

/// Formla düzenle on the first permit: the end before the start (the rule under Bitiş), the area over
/// the floors' (under Alan, its unit after it), the status's description under it.
pub(super) fn permit_entry(app: &mut App) {
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
pub(super) fn permit_conflict(app: &mut App) {
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
pub(super) fn permit_fields(app: &mut App) {
    permits(app);
    let _ = app.open_layer_fields("ruhsat");
    let _ = app.update(Message::Gis(gis::Event::LayerFields(
        crate::layer_fields::Event::Choose(4),
    )));
}

/// The status's value list: a description and the days a value holds.
pub(super) fn permit_values(app: &mut App) {
    permits(app);
    let _ = app.open_layer_fields("ruhsat");
    let _ = app.update(Message::Gis(gis::Event::LayerFields(
        crate::layer_fields::Event::OpenList(0),
    )));
}

/// Formlar on the permit form, the note chosen: its links name the layer rule reading it.
pub(super) fn permit_designer(app: &mut App) {
    permits(app);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("aciklama".into()));
}

/// Its last step: the mirrors, Geri and Kaydet.
pub(super) fn stepped_summary(app: &mut App) {
    stepped_survey(app);
    let _ = app.update(Message::Gis(gis::Event::FormEntry(entry::Event::Value(
        "Kat".into(),
        "6".into(),
    ))));
    let _ = app.update(Message::Gis(gis::Event::FormEntry(entry::Event::Tab(1))));
    let _ = app.update(Message::Gis(gis::Event::FormEntry(entry::Event::Tab(2))));
}

/// The step form in Formlar: Adım adım on, the mirror marked in the items, Kopyala and Yapıştır.
pub(super) fn stepped_designer(app: &mut App) {
    stepped_survey(app);
    app.gis.form_entry = None;
    app.dialog = None;
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("ad-ozet".into()));
}

/// Değer bileşimleri's window on the buildings' group (docs/adr/0423 §5).
pub(super) fn contingents_window(app: &mut App) {
    typed_buildings(app);
    run(app, "data.contingents");
}

/// A timber house given steel: no row of the group allows steel with a hipped roof; the roof's
/// choices follow the material (docs/adr/0423 §2). Kaydet is pressed, so the problem is shown.
pub(super) fn combination_refused(app: &mut App) {
    typed_buildings(app);
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
    entry(app, entry::Event::Value("Malzeme".into(), "celik".into()));
    entry(app, entry::Event::Save);
}

/// Alt türler's window on the buildings, the industrial subtype chosen (docs/adr/0422 §5).
pub(super) fn subtypes_window(app: &mut App) {
    use crate::forms::subtypes::Event;
    typed_buildings(app);
    run(app, "data.subtypes");
    let _ = app.update(Message::Gis(gis::Event::Subtypes(Event::Choose(
        "sanayi".into(),
    ))));
}

/// A timber house's form, its kind changed to industrial: timber is not offered, the note is
/// required, the values are kept until the user empties them (docs/adr/0422 §2).
pub(super) fn subtype_changed(app: &mut App) {
    typed_buildings(app);
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
    entry(app, entry::Event::Value("Tür".into(), "sanayi".into()));
}

/// Hesapları yenile (docs/adr/0421): the survey form gains a calculated note after the buildings
/// were written; one press writes it on every building, the window then closed over the table.
pub(super) fn recompute(app: &mut App) {
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
pub(super) fn scenarios(app: &mut App) {
    designer_links(app);
    designer(app, designer::Event::Preview(true));
    let typed = |app: &mut App, key: &str, v: &str| {
        let _ = app.update(Message::Gis(gis::Event::FormPreview(entry::Event::Value(
            key.into(),
            v.into(),
        ))));
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
pub(super) fn designer_links(app: &mut App) {
    buildings_with_a_form(app);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("kat".into()));
}

/// Alanlar on the buildings, the floor count chosen: it is shared by area when a building is split and
/// summed when buildings are merged (docs/adr/0419).
pub(super) fn field_policies(app: &mut App) {
    use crate::layer_fields::Event;
    styled_buildings(app);
    let _ = app.open_layer_fields("bina");
    let _ = app.update(Message::Gis(gis::Event::LayerFields(Event::Choose(2))));
    let _ = app.update(Message::Gis(gis::Event::LayerFields(Event::Split(
        2,
        Some(kentos_commands::SplitPolicy::Ratio),
    ))));
    let _ = app.update(Message::Gis(gis::Event::LayerFields(Event::Merge(
        2,
        Some(kentos_commands::MergePolicy::Sum),
    ))));
}

/// Alanlar on the buildings, a field chosen and marked Dışarı verilmez (docs/adr/0638 §5).
pub(super) fn field_withheld(app: &mut App) {
    use crate::layer_fields::Event;
    styled_buildings(app);
    let _ = app.open_layer_fields("bina");
    let _ = app.update(Message::Gis(gis::Event::LayerFields(Event::Choose(1))));
    let _ = app.update(Message::Gis(gis::Event::LayerFields(Event::Withheld(true))));
}

/// Koşullu biçim's window on the buildings' rules, the floor count's rule chosen.
pub(super) fn styles_window(app: &mut App) {
    styled_buildings(app);
    run(app, "data.tableStyles");
    let _ = app.update(Message::Gis(gis::Event::TableStyles(
        crate::forms::table_styles::Event::Choose(2),
    )));
}

/// Two buildings and their floors (docs/adr/0415): the floors are records a composite relation ties
/// to their building; the survey form keeps the building's code unique.
pub(super) fn floors(app: &mut App) {
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
                kentos_commands::CAD_ENTITIES_SET,
                kentos_commands::CallOp::Execute,
                json!({ "uids": [floor], "attrs": { "Bina": building }, "operation": "attributes" }),
            );
        }
    }
    app.selection.set([Slot(ids[0])]);
}

/// Çoğalt on the first building: its copy and the copies of its two floors selected, the floors' table.
pub(super) fn duplicated(app: &mut App) {
    floors(app);
    let _ = app.update(Message::Gis(gis::Event::Features(
        crate::features::Event::Duplicate,
    )));
    app.gis.features.layer = Some("kat".into());
    let _ = app.update(Message::Run("data.featureTable"));
}

/// A building whose height is calculated from its floors and may be typed over (docs/adr/0417): its
/// form open, the height typed over.
pub(super) fn overridable_height(app: &mut App) {
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
pub(super) fn overridden_height(app: &mut App) {
    overridable_height(app);
    let _ = app.update(Message::Gis(gis::Event::FormEntry(entry::Event::Value(
        "Yükseklik".into(),
        "10.5".into(),
    ))));
}

/// The owners' table (docs/adr/0410 §4): records without a geometry in the Tablo tab.
pub(super) fn owner_table(app: &mut App) {
    owners(app);
    app.gis.features.layer = Some("malik".into());
    let _ = app.update(Message::Run("data.featureTable"));
    app.selection.set([Slot(4)]);
}

/// Kayıt ekle in the owners' table: a new record, opened by the owners' form.
pub(super) fn owner_added(app: &mut App) {
    owner_table(app);
    let _ = app.update(Message::Gis(gis::Event::Features(
        crate::features::Event::AddRecord,
    )));
}

/// Alanlar's value list of the care field: several of its values may be chosen (docs/adr/0408 §1).
pub(super) fn tree_fields(app: &mut App) {
    trees(app);
    let _ = app.open_layer_fields("agac");
    let _ = app.update(Message::Gis(gis::Event::LayerFields(
        crate::layer_fields::Event::OpenList(4),
    )));
}

/// Formlar on the tree form: the photos item chosen.
pub(super) fn tree_designing(app: &mut App) {
    trees(app);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("fotograflar".into()));
}

/// The block of buildings under the calculating form: the first one's record already written (code
/// B-3); the moment and the person fixed.
pub(super) fn calc_buildings(app: &mut App) {
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
    app.gis.form_memory.clock = Some("2026-10-16T14:30:00".into());
    app.gis.form_memory.person = Some("Ayşe Yılmaz".into());
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
pub(super) fn address_form() -> Value {
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
    let typed: kentos_commands::FormBody = serde_json::from_value(body.clone()).expect("a body");
    json!({ "id": "adres", "name": "Bina adresi", "layer": "bina", "draft": body.clone(),
            "releases": [{ "version": 1, "digest": kentos_commands::body_digest(&typed), "body": body }] })
}

/// Buildings with the address form; the provinces, districts, neighbourhoods, parcels and meters on their own layers;
/// the first building's two meters refer to it.
pub(super) fn address_buildings(app: &mut App) {
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
            kentos_commands::CAD_ENTITIES_SET,
            kentos_commands::CallOp::Execute,
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
pub(super) fn address_editing(app: &mut App) {
    address_buildings(app);
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
    entry(app, entry::Event::Value("İl".into(), "Ankara".into()));
    entry(app, entry::Event::Value("İlçe".into(), "Çankaya".into()));
}

/// Formlar: the neighbourhood's list from another layer: its layer, key, label, chain and filter.
pub(super) fn address_designing(app: &mut App) {
    address_buildings(app);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("mahalle".into()));
}

/// İlişkiler (docs/adr/0405): the buildings' meters as a composition, its policy chosen.
pub(super) fn relations_window(app: &mut App) {
    address_buildings(app);
    run(app, "data.relations");
    let _ = app.update(Message::Gis(gis::Event::Relations(relations::Event::New)));
    for event in [
        relations::Event::Name("Binanın sayaçları".into()),
        relations::Event::Parent("bina".into()),
        relations::Event::Child("sayac".into()),
        relations::Event::Field("bina".into()),
        relations::Event::Policy(kentos_commands::RelationDelete::Cascade),
    ] {
        let _ = app.update(Message::Gis(gis::Event::Relations(event)));
    }
}

/// Kayıtları denetle (docs/adr/0406 §1): the calculating form's buildings, some of them breaking its
/// rules (a code twice, a status missing), warnings shown too.
pub(super) fn records_audit(app: &mut App) {
    calc_buildings(app);
    if let Some(doc) = app.document.as_mut() {
        let uid = doc
            .model
            .uid(Slot(2))
            .map(|u| u.to_string())
            .unwrap_or_default();
        let _ = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            kentos_commands::CAD_ENTITIES_SET,
            kentos_commands::CallOp::Execute,
            json!({ "uids": [uid], "attrs": { "Durum": "iyi", "Kod": "B-3", "Kat": "2" }, "operation": "attributes" }),
        );
    }
    run(app, "data.formAudit");
    let _ = app.update(Message::Gis(gis::Event::FormAudit(audit::Event::Warnings(
        true,
    ))));
}

/// Formla düzenle with the storeys on a slider (docs/adr/0406 §2).
pub(super) fn slider_editing(app: &mut App) {
    calc_buildings(app);
    if let Some(doc) = app.document.as_mut() {
        let mut s = doc.model.settings().clone();
        for f in &mut s.forms {
            if let Some(kentos_commands::FormItem::Field(k)) =
                items_mut_find(&mut f.draft.items, "kat")
            {
                k.widget = Some(kentos_commands::FormWidget::Slider);
            }
            for r in &mut f.releases {
                if let Some(kentos_commands::FormItem::Field(k)) =
                    items_mut_find(&mut r.body.items, "kat")
                {
                    k.widget = Some(kentos_commands::FormWidget::Slider);
                }
                r.digest = kentos_commands::body_digest(&r.body);
            }
        }
        doc.model.set_settings(s);
    }
    app.selection.set([Slot(1)]);
    run(app, "data.formEdit");
}
