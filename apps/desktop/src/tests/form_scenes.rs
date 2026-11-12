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

use crate::gis;
use kentos_commands::FormItem;
use kentos_domain::Slot;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::forms::{audit, designer, entry, relations};
use crate::tools_scenes::{E, N, Objects, open_edited, run};
use crate::tools_screens::Scene;

/// The item `id` anywhere in `items`, to change.
fn items_mut_find<'a>(
    items: &'a mut [kentos_commands::FormItem],
    id: &str,
) -> Option<&'a mut kentos_commands::FormItem> {
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
    let _ = app.update(Message::Gis(gis::Event::Forms(event)));
}

pub(crate) fn entry(app: &mut App, event: entry::Event) {
    let _ = app.update(Message::Gis(gis::Event::FormEntry(event)));
}

/// Formlar: the form chosen, its note item selected: the rules that show and ask for it, its check.
fn designing(app: &mut App) {
    buildings(app, true);
    run(app, "data.forms");
    designer(app, designer::Event::SelectItem("aciklama".into()));
}

/// Formlar: the form's scope (docs/adr/0539), the records a Veri girişi reaches through it.
fn designing_scope(app: &mut App) {
    buildings(app, true);
    run(app, "data.forms");
    designer(app, designer::Event::Scope("Durum = 'inceleme'".into()));
}

/// Formlar: a scope reading a field the layer does not have: the preflight's issue, Yayınla off.
fn designing_scope_issue(app: &mut App) {
    buildings(app, true);
    run(app, "data.forms");
    designer(app, designer::Event::Scope("Sorumlu = @kullanici_eposta".into()));
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
        designer::Event::Widget(Some(kentos_commands::FormWidget::Number)),
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
    let _ = app.update(Message::Gis(gis::Event::FormPreview(entry::Event::Value(
        "kat-sayisi".into(),
        "7".into(),
    ))));
    let _ = app.update(Message::Gis(gis::Event::FormPreview(entry::Event::Group(
        "hesap".into(),
    ))));
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
        .gis
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
        ("form-kapsam", designing_scope),
        ("form-kapsam-sorun", designing_scope_issue),
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
        ("form-dizinler", parcel_indexes),
        ("form-dizin-bozuk", parcel_indexes_broken),
        ("form-dizin-yeni", parcel_index_new),
        ("form-kural-formda", permit_entry),
        ("form-cakisma", permit_conflict),
        ("form-alan-ekleri", permit_fields),
        ("form-deger-gunleri", permit_values),
        ("form-kural-baglari", permit_designer),
        ("form-kaynak-giris", sources_entry),
        ("form-haritadan-dugme", map_pick_offered),
        ("form-haritadan-seciliyor", map_picking),
        ("form-haritadan-secildi", map_picked),
        ("form-haritadan-uzakta", map_apart),
        ("form-cikinca-yazarken", left_typing),
        ("form-cikinca-cikti", left_done),
        ("form-iliskili-toplam", related_sums),
        ("form-iliskili-eklendi", related_sums_linked),
        ("form-eylemler-yalniz-bagla", actions_link_only),
        ("form-eylemler-kilitli", actions_locked),
        ("form-kaynak-durumlari", sources_states),
        ("form-alt-kayit-izni", actions_entry),
        ("form-eylemler-sil-sor", actions_delete_asked),
        ("form-eylemler-salt-okunur", actions_view_only),
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
        ("form-alan-disari-verilmez", field_withheld),
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
        (
            "form-iliskili-sayi",
            crate::form_access_scenes::related_bounded,
        ),
        (
            "form-yayin-alanlari",
            crate::form_access_scenes::stale_release,
        ),
        (
            "form-yayindan-kaldirildi",
            crate::form_access_scenes::withdrawn_designer,
        ),
    ]
}

#[cfg(test)]
#[path = "form_scenes/tests.rs"]
mod tests;

#[path = "form_scenes/buildings.rs"]
mod buildings;
pub(crate) use buildings::*;

#[path = "form_scenes/lists.rs"]
mod lists;
pub(crate) use lists::*;

#[path = "form_scenes/parcels.rs"]
mod parcels;
use parcels::*;
