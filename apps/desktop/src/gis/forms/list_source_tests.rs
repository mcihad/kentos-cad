//! A form field's Seçenek kaynağı (the CBS review's 3rd item, docs/adr/9322): the project's lists of the field's kind are
//! offered by name and one chosen links the layer field (one undo step); Liste oluştur ve bağla makes a list named after
//! the field and links it on Kaydet; Değer listeleri opened from Formlar comes back to the designer as it was, the
//! unsaved draft and the chosen item kept, whether Kaydet or Vazgeç closed it. Two lists of one name are refused.

use kentos_commands::{FieldChoice, FieldDomain, LayerFieldKind};

use crate::app::{App, Dialog, Message};
use crate::files_testing::{app_with_drawing, drive};
use crate::forms::designer::Event;
use crate::forms::domains::Event as Lists;
use crate::gis;

fn ev(app: &mut App, e: Event) {
    let task = app.update(Message::Gis(gis::Event::Forms(e)));
    drive(app, task);
}

fn lists(app: &mut App, e: Lists) {
    let task = app.update(Message::Gis(gis::Event::Domains(e)));
    drive(app, task);
}

/// A layer of the test's own whose form has `durum` (text) on it, chosen, unsaved; the project's lists: Saha Durumu
/// (text) and Kat sayısı (an integer range). The layer's id and the item's id.
fn opened() -> (App, String, String) {
    let mut app = app_with_drawing();
    let doc = app.document.as_mut().expect("open");
    let mut settings = doc.model.settings().clone();
    settings.domains = vec![
        FieldDomain {
            id: "saha-durumu".into(),
            name: "Saha Durumu".into(),
            description: None,
            kind: LayerFieldKind::Text,
            values: Some(vec![
                FieldChoice::new("A", "Aktif"),
                FieldChoice::new("B", "Bakımda"),
            ]),
            min: None,
            max: None,
            min_exclusive: false,
            max_exclusive: false,
            unit: None,
        },
        FieldDomain {
            id: "kat-sayisi".into(),
            name: "Kat sayısı".into(),
            description: None,
            kind: LayerFieldKind::Integer,
            values: None,
            min: Some("1".into()),
            max: Some("40".into()),
            min_exclusive: false,
            max_exclusive: false,
            unit: None,
        },
    ];
    doc.model.set_settings(settings);
    let done = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        "cad.layers.edit",
        kentos_commands::CallOp::Execute,
        serde_json::json!({ "operation": "add", "name": "Saha" }),
    )
    .expect("runs");
    let layer = done["output"]["layer"]
        .as_str()
        .expect("the layer")
        .to_owned();
    let task = app.update(Message::Run("data.forms"));
    drive(&mut app, task);
    ev(&mut app, Event::NewForm);
    ev(&mut app, Event::NewLayer(layer.clone()));
    ev(&mut app, Event::NewName("Saha formu".into()));
    ev(&mut app, Event::Create);
    ev(&mut app, Event::NewFieldName("durum".into()));
    ev(&mut app, Event::CreateField);
    let item = {
        let w = app.gis.forms_window.as_ref().expect("the designer");
        let body = w.body.as_ref().expect("the draft");
        body.walk()
            .into_iter()
            .find_map(|(_, i)| match i {
                kentos_commands::FormItem::Field(f) if f.field == "durum" => Some(f.id.clone()),
                _ => None,
            })
            .expect("the item")
    };
    ev(&mut app, Event::SelectItem(item.clone()));
    (app, layer, item)
}

fn field_domain(app: &App, layer: &str) -> Option<String> {
    app.layer_fields_of(layer)
        .into_iter()
        .find(|f| f.name == "durum")
        .and_then(|f| f.domain)
}

#[test]
fn a_project_list_of_the_fields_kind_is_offered_and_links_the_layer_field() {
    let (mut app, layer, item) = opened();
    let names: Vec<String> = app
        .lists_for(LayerFieldKind::Text)
        .into_iter()
        .map(|d| d.name)
        .collect();
    assert_eq!(names, ["Saha Durumu"], "the lists of the field's kind only");
    assert_eq!(field_domain(&app, &layer), None);
    ev(&mut app, Event::LinkDomain("saha-durumu".into()));
    assert_eq!(field_domain(&app, &layer).as_deref(), Some("saha-durumu"));
    let codes: Vec<String> = app
        .layer_fields_of(&layer)
        .into_iter()
        .find(|f| f.name == "durum")
        .and_then(|f| f.values)
        .unwrap_or_default()
        .into_iter()
        .map(|c| c.code)
        .collect();
    assert_eq!(codes, ["A", "B"], "the field's codes are the list's");
    // The designer stays on the item, its draft unsaved.
    let w = app.gis.forms_window.as_ref().expect("the designer");
    assert_eq!((w.item.as_deref(), w.dirty), (Some(item.as_str()), true));
    assert_eq!(app.dialog, Some(Dialog::Gis(gis::GisDialogKind::Forms)));
    // One undo step: unlinked again.
    assert!(app.document.as_mut().expect("open").model.undo().is_some());
    assert_eq!(field_domain(&app, &layer), None);
}

#[test]
fn liste_olustur_ve_bagla_makes_the_list_links_it_and_comes_back_as_it_was() {
    let (mut app, layer, item) = opened();
    ev(&mut app, Event::PropertiesScrolled(120.0));
    ev(&mut app, Event::NewDomainForField);
    assert_eq!(app.dialog, Some(Dialog::Gis(gis::GisDialogKind::Domains)));
    let w = app.gis.domains.as_ref().expect("Değer listeleri");
    let at = w.chosen.expect("the new list chosen");
    assert_eq!(
        (w.domains[at].name.as_str(), w.domains[at].kind),
        ("durum", LayerFieldKind::Text)
    );
    // Its codes typed; Kaydet: the list made and linked in one step, the designer back on the item, the draft kept.
    lists(&mut app, Lists::Code(0, "A".into()));
    lists(&mut app, Lists::Label(0, "Aktif".into()));
    lists(&mut app, Lists::AddCode);
    lists(&mut app, Lists::Code(1, "B".into()));
    lists(&mut app, Lists::Label(1, "Bakımda".into()));
    lists(&mut app, Lists::Save);
    assert_eq!(
        app.dialog,
        Some(Dialog::Gis(gis::GisDialogKind::Forms)),
        "back to the designer"
    );
    let id = app
        .document
        .as_ref()
        .and_then(|d| {
            d.model
                .settings()
                .domains
                .iter()
                .find(|d| d.name == "durum")
                .map(|d| d.id.clone())
        })
        .expect("the list made");
    assert_eq!(field_domain(&app, &layer), Some(id));
    let w = app.gis.forms_window.as_ref().expect("the designer");
    assert_eq!(
        (w.item.as_deref(), w.dirty, w.properties_offset),
        (Some(item.as_str()), true, 120.0)
    );
    // Vazgeç comes back the same way, nothing made.
    let before = app
        .document
        .as_ref()
        .map(|d| d.model.settings().domains.len());
    ev(&mut app, Event::NewDomainForField);
    lists(&mut app, Lists::Cancel);
    assert_eq!(app.dialog, Some(Dialog::Gis(gis::GisDialogKind::Forms)));
    assert_eq!(
        app.document
            .as_ref()
            .map(|d| d.model.settings().domains.len()),
        before
    );
}

#[test]
fn degerlistelerine_formdan_gidilince_taslak_ve_secim_kalir() {
    let (mut app, _layer, item) = opened();
    ev(&mut app, Event::OpenDomains);
    assert_eq!(app.dialog, Some(Dialog::Gis(gis::GisDialogKind::Domains)));
    lists(&mut app, Lists::Cancel);
    assert_eq!(
        app.dialog,
        Some(Dialog::Gis(gis::GisDialogKind::Forms)),
        "back to the designer"
    );
    let w = app.gis.forms_window.as_ref().expect("the designer");
    assert_eq!(
        (w.item.as_deref(), w.dirty),
        (Some(item.as_str()), true),
        "the unsaved item and the choice kept"
    );
    // From the ribbon it closes as before.
    let task = app.update(Message::Run("data.domains"));
    drive(&mut app, task);
    lists(&mut app, Lists::Cancel);
    assert_eq!(app.dialog, None);
}

#[test]
fn two_lists_of_one_name_are_refused() {
    let (mut app, _, _) = opened();
    ev(&mut app, Event::OpenDomains);
    lists(&mut app, Lists::New);
    lists(&mut app, Lists::Name("saha durumu".into()));
    lists(&mut app, Lists::Code(0, "X".into()));
    lists(&mut app, Lists::Label(0, "Diğer".into()));
    let w = app.gis.domains.as_ref().expect("Değer listeleri");
    let problem = kentos_commands::domains_problem(&w.domains).map(|(_, why)| why);
    assert_eq!(
        problem.as_deref(),
        Some("“saha durumu” adlı iki değer listesi var.")
    );
    // Kaydet is refused while it says so: nothing written.
    lists(&mut app, Lists::Save);
    assert_eq!(
        app.document
            .as_ref()
            .map(|d| d.model.settings().domains.len()),
        Some(2)
    );
}

/// The pictures (`.run/gui-evidence/kullanim-9322/`): Seçenek kaynağı with the project's list offered and its summary,
/// Değer listeleri made for the field with its first empty cell marked.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib list_source_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    crate::files_testing::usage_shots("9322", "01-secenek-kaynagi", || {
        let (mut app, _, _) = opened();
        ev(&mut app, Event::LinkDomain("saha-durumu".into()));
        app
    });
    crate::files_testing::usage_shots("9322", "02-liste-olustur-ve-bagla", || {
        let (mut app, _, _) = opened();
        ev(&mut app, Event::NewDomainForField);
        lists(&mut app, Lists::Code(0, "A".into()));
        app
    });
}
