//! The municipal report's 9th item (docs/adr/9166): a data layer's table bar has a drawing layer's tools in the same
//! order; Form says how a row's form opens and the table stays; Sütunlar hides a field in the data layer's table as in
//! a drawing layer's; Dışa aktar opens the layer's export; İncele opens İstatistik on the data layer.

use crate::app::{App, Message};
use crate::data_layers_tests::{node, parcels};
use crate::features::Event;
use crate::features::outgoing::ColumnsEvent;
use crate::files_testing::app_with_drawing;

fn ev(app: &mut App, e: Event) {
    let _ = app.update(Message::Gis(crate::gis::Event::Features(e)));
}

/// The parcels' table bound as a data layer and shown in the Tablo tab.
fn on_table(name: &str) -> (App, String) {
    crate::data_layers::register();
    let address = parcels(name, 30).display().to_string();
    let mut app = app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&address, "parseller").expect("described")])
        .expect("added");
    let id = node(&app, "parseller").expect("the layer").id.clone();
    app.gis.features.layer = Some(id.clone());
    app.command_expanded = true;
    app.bottom_tab = crate::bottom::BottomTab::Table;
    (app, id)
}

#[test]
fn form_says_how_a_rows_form_opens_and_the_table_stays() {
    let (mut app, _) = on_table("cubuk-form");
    ev(&mut app, Event::Pane(crate::features::Pane::Form));
    assert_eq!(app.gis.features.pane, crate::features::Pane::Table);
    let said = crate::files_testing::last_said(&app);
    // No row chosen: the way to a row's form is said (the CBS review's 2nd item opens a chosen row's form).
    assert!(said.contains("Formu açmak için tek bir kayıt seçin: tabloda bir satır ya da çizimde bir nesne"), "{said}");
}

#[test]
fn sutunlar_hides_a_field_of_the_data_layers_table() {
    let (mut app, id) = on_table("cubuk-sutun");
    let layer = node(&app, "parseller").expect("the layer").clone();
    assert_eq!(app.data_shown_fields(&layer), ["ada", "kullanim", "alan"]);
    ev(&mut app, Event::Columns(ColumnsEvent::Toggle("kullanim".into())));
    assert!(app.hidden_columns(Some(&id)).contains("kullanim"));
    assert_eq!(app.data_shown_fields(&layer), ["ada", "alan"]);
    ev(&mut app, Event::Columns(ColumnsEvent::ShowAll));
    assert_eq!(app.data_shown_fields(&layer), ["ada", "kullanim", "alan"]);
}

#[test]
fn incele_opens_istatistik_on_the_data_layer() {
    let (mut app, id) = on_table("cubuk-incele");
    ev(&mut app, Event::Analyse(crate::features::outgoing::Analyse::Statistics));
    assert_eq!(app.bottom_tab, crate::bottom::BottomTab::Statistics);
    assert_eq!(app.gis.charts.layer.as_deref(), Some(id.as_str()));
}

/// The picture (`.run/gui-evidence/kullanim-9166/`): the data layer's table with the drawing table's tools.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    crate::files_testing::usage_shots("9166", "01-veri-tablosu-cubugu", || {
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (mut app, _) = on_table(&format!("cubuk-resim-{n}"));
        let _ = app.data_table_event(crate::features::data::DataEvent::Range(0, 12));
        crate::data_layers_tests::answer_pages(&mut app);
        app
    });
}

/// The second round's T9 (docs/adr/9347): two layers being edited. The table's bar says the shown layer's session and
/// lists both under Düzenlenen katmanlar, one press making a layer the active one and the table's; Düzenlemeyi başlat's
/// tip names the active layer, what waits on it and that the other stays open.
#[test]
fn two_sessions_are_named_and_one_press_goes_to_either() {
    crate::data_layers::register();
    let path = crate::data_layers_tests::municipal("iki-oturum");
    let address = path.display().to_string();
    let mut app = crate::files_testing::app_with_drawing();
    for table in ["bakim", "mahalleler"] {
        app.add_data_layers(vec![crate::data_layers::describe(&address, table).expect("described")]).expect("added");
    }
    let bakim = node(&app, "bakim").expect("bakim").id.clone();
    let mahalleler = node(&app, "mahalleler").expect("mahalleler").id.clone();
    let _ = app.data_edits_event(crate::data_edits::Event::Start(bakim.clone()));
    let tip = app.edit_tip("layer.edit").expect("its tip");
    assert!(tip.starts_with("Etkin katman “bakim” düzenleniyor; kaynağa yazılmamış kaydı yok."), "{tip}");
    let _ = app.data_edits_event(crate::data_edits::Event::Start(mahalleler.clone()));
    let tip = app.edit_tip("layer.edit").expect("its tip");
    assert!(tip.contains("Düzenlemeyi bitir yalnız bu katmanı kapatır. Düzenlenen öbür katmanlar açık kalır: “bakim”."), "{tip}");
    // Düzenlenen katmanlar: bakim one press away, the active layer and the table's.
    ev(&mut app, Event::EditedLayer(bakim.clone()));
    let active = app.document.as_ref().map(|d| d.model.layers().active().to_owned());
    assert_eq!(active.as_deref(), Some(bakim.as_str()));
    assert_eq!(app.gis.features.layer.as_deref(), Some(bakim.as_str()));
    let _ = std::fs::remove_file(&path);
}

/// The picture (`.run/gui-evidence/kullanim-9347/01-duzenlenen-katmanlar`): two layers being edited, the table on one.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib data_bar_tests::sessions_screens -- --ignored --nocapture"]
fn sessions_screens() {
    crate::drawing_fonts::load();
    crate::files_testing::usage_shots("9347", "01-duzenlenen-katmanlar", || {
        crate::data_layers::register();
        let path = crate::data_layers_tests::municipal("iki-oturum-resim");
        let address = path.display().to_string();
        let mut app = crate::files_testing::app_with_drawing();
        for table in ["bakim", "mahalleler"] {
            app.add_data_layers(vec![crate::data_layers::describe(&address, table).expect("described")]).expect("added");
        }
        let bakim = node(&app, "bakim").expect("bakim").id.clone();
        let mahalleler = node(&app, "mahalleler").expect("mahalleler").id.clone();
        let _ = app.data_edits_event(crate::data_edits::Event::Start(mahalleler));
        let _ = app.data_edits_event(crate::data_edits::Event::Start(bakim.clone()));
        app.gis.features.layer = Some(bakim);
        app.command_expanded = true;
        app.bottom_tab = crate::bottom::BottomTab::Table;
        let _ = app.update(Message::BottomResized(240.0));
        let task = app.data_table_event(crate::features::data::DataEvent::Range(0, 12));
        crate::files_testing::drive(&mut app, task);
        crate::data_layers_tests::answer_pages(&mut app);
        app
    });
}
