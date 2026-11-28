//! Satır ekle's layer (the CBS review's 11th item, docs/adr/9320): found when the row opens and shown above the table.
//! A row no layer takes keeps what was typed until Nokta katmanı oluştur makes one. A data layer's edit session and its
//! source's rights are the source's rules. The table keeps the name, the code and the elevation, read back apart from
//! the app.

use serde_json::json;

use super::{Event, Walk};
use crate::app::{App, Message};
use crate::files_testing::{drive, last_said};
use crate::layer_create::Event as Create;
use crate::tools_scenes::{Objects, layer, open_edited};

fn ev(app: &mut App, e: Event) {
    let task = app.update(Message::Gis(crate::gis::Event::Points(e)));
    drive(app, task);
}

/// The row's cells typed from Ad to Kod, the editor left in Kod.
fn fill(app: &mut App, values: [&str; 5]) {
    for (i, v) in values.iter().enumerate() {
        ev(app, Event::Input((*v).into()));
        if i < values.len() - 1 {
            ev(app, Event::Finish(Some(Walk::Right)));
        }
    }
}

fn enter(app: &mut App) {
    ev(app, Event::Finish(Some(Walk::Down)));
}

fn objects(app: &App) -> usize {
    app.document.as_ref().map_or(0, |d| d.model.len())
}

/// A new CBS project: its one layer Açıklama, active.
fn new_cbs(app: &mut App) {
    open_edited(app, Objects::new(), |d| {
        d["settings"]["workspace"] = json!("gis");
        let mut a = layer("aciklama", "Açıklama", "fg", 0.25);
        a["geometry"] = json!("annotation");
        d["layers"] = json!([a]);
        d["activeLayer"] = json!("aciklama");
    });
}

/// A CBS project with a table of points of `fields`, no Z, added as a data layer (`read_only` as the source's rights
/// say); the layer's id.
fn cbs_with_points(app: &mut App, tag: &str, fields: &[&str], read_only: bool) -> String {
    crate::data_layers::register();
    new_cbs(app);
    let path = crate::files_testing::scratch(tag).join("noktalar.gpkg");
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let address = path.display().to_string();
    {
        use kentos_data::Driver;
        let source = kentos_data_gpkg::Gpkg.open(&address).expect("open");
        let fields: Vec<(&str, kentos_data::FieldKind)> = fields
            .iter()
            .map(|f| (*f, kentos_data::FieldKind::Text { max: None }))
            .collect();
        crate::data_layers_tests::point_table(&*source, "noktalar", &fields, &[]);
    }
    let mut described = crate::data_layers::describe(&address, "noktalar").expect("described");
    described.data.read_only = read_only;
    app.add_data_layers(vec![described]).expect("added");
    crate::data_layers_tests::node(app, "noktalar")
        .expect("the layer")
        .id
        .clone()
}

#[test]
fn a_new_cbs_projects_row_says_first_that_no_layer_takes_it_and_the_table_made_keeps_its_values() {
    crate::data_layers::register();
    let (mut app, _) = App::boot(None);
    new_cbs(&mut app);
    let _ = app.update(Message::Run("point.editor"));
    // Said when the row opens, before anything is typed.
    ev(&mut app, Event::AddRow);
    assert!(app.draft_target().is_none());
    assert!(
        last_said(&app).contains("Satırın yazılacağı nokta katmanı yok"),
        "{}",
        last_said(&app)
    );
    assert!(
        !last_said(&app).contains("Karışık"),
        "no layer of any kind is offered in a CBS project"
    );
    fill(&mut app, ["QA-A", "500000", "4320000", "12.5", "QA"]);
    let before = objects(&app);
    enter(&mut app);
    assert!(
        last_said(&app).ends_with("Yazdıklarınız satırda duruyor."),
        "{}",
        last_said(&app)
    );
    assert_eq!(objects(&app), before, "nothing written");
    let kept = app.gis.points.draft.clone().expect("the row stays");
    assert_eq!(
        [
            kept.name.as_str(),
            &kept.east,
            &kept.north,
            &kept.z,
            &kept.code
        ],
        ["QA-A", "500000", "4320000", "12.5", "QA"]
    );
    // Nokta katmanı oluştur: the window for a table of points keeping the name, the code and the elevation.
    ev(&mut app, Event::CreateLayer);
    assert!(
        app.gis.layer_create.is_some(),
        "the window asks where the table goes"
    );
    let gpkg = crate::files_testing::scratch("nokta-satiri").join("noktalar.gpkg");
    let send = |app: &mut App, e: Create| {
        let task = app.update(Message::Gis(crate::gis::Event::LayerCreate(e)));
        drive(app, task);
    };
    send(&mut app, Create::Path(gpkg.display().to_string()));
    send(&mut app, Create::Run);
    assert!(app.gis.layer_create.is_none(), "made: {}", last_said(&app));
    let made = app
        .draft_target()
        .expect("the table made is the row's layer");
    let (fields, z, table) = {
        let node = app
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(&made.id).cloned())
            .expect("the layer");
        let data = node.data.expect("a data layer");
        let names: Vec<String> = node.fields.iter().map(|f| f.name.clone()).collect();
        (names, data.geometry.is_some_and(|g| g.z), data.table)
    };
    assert_eq!(fields, ["ad", "kod"]);
    assert!(z, "the geometry keeps the elevation");
    assert!(!made.closed, "its edit session open");
    // The values typed before are still the row's: Enter writes them.
    let kept_now = app.gis.points.draft.clone().expect("the row");
    assert_eq!(
        (kept_now.name.as_str(), kept_now.code.as_str()),
        ("QA-A", "QA")
    );
    enter(&mut app);
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(
        doc.data_changes(&made.id).added.len(),
        1,
        "{}",
        last_said(&app)
    );
    // Saved: the table holds the row, read apart from the app.
    let task = app.data_edits_event(crate::data_edits::Event::Save(vec![made.id.clone()]));
    drive(&mut app, task);
    kentos_data::registry().forget(&gpkg.display().to_string());
    use kentos_data::Driver;
    let source = kentos_data_gpkg::Gpkg
        .open(&gpkg.display().to_string())
        .expect("open");
    let schema = source.schema(&table).expect("its schema");
    let rows =
        kentos_data::driver::collect(&*source, &table, &kentos_data::Request::all()).expect("rows");
    assert_eq!(rows.len(), 1);
    let value = |name: &str| {
        rows[0]
            .values
            .get(schema.index(name).expect("a column"))
            .cloned()
    };
    assert_eq!(value("ad"), Some(kentos_data::Value::Text("QA-A".into())));
    assert_eq!(value("kod"), Some(kentos_data::Value::Text("QA".into())));
    match &rows[0].geometry {
        Some(Ok(kentos_data::Geometry::Point(Some(c)))) => {
            assert_eq!((c.x, c.y, c.z), (500_000.0, 4_320_000.0, Some(12.5)));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_closed_data_layer_is_named_with_duzenlemeyi_baslat_and_a_table_without_columns_says_what_it_does_not_keep()
 {
    let (mut app, _) = App::boot(None);
    let id = cbs_with_points(&mut app, "kapali-nokta", &["durum"], false);
    let _ = app.update(Message::Run("point.editor"));
    ev(&mut app, Event::AddRow);
    let target = app.draft_target().expect("the table is the row's layer");
    assert_eq!((target.id.as_str(), target.closed), (id.as_str(), true));
    assert!(
        last_said(&app).contains("“noktalar” düzenlemeye kapalı"),
        "{}",
        last_said(&app)
    );
    fill(&mut app, ["N1", "484000", "4420000", "7", "K1"]);
    enter(&mut app);
    assert!(
        last_said(&app).contains("Düzenlemeyi başlat")
            && last_said(&app).ends_with("Yazdıklarınız satırda duruyor.")
    );
    assert_eq!(
        app.document
            .as_ref()
            .map(|d| d.model.data_changes(&id).added.len()),
        Some(0)
    );
    // Düzenlemeyi başlat: the session opened, the row written; the table has no place for the name, the code and Z.
    ev(&mut app, Event::StartEditing);
    assert!(app.draft_target().is_some_and(|t| !t.closed));
    enter(&mut app);
    assert_eq!(
        app.document
            .as_ref()
            .map(|d| d.model.data_changes(&id).added.len()),
        Some(1)
    );
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(
        said.iter().any(|t| t
            .contains("“noktalar” tablosunda ad sütunu, kod sütunu, geometride kot (Z) yok")
            && t.contains("Alanlar'da")),
        "{said:#?}"
    );
}

#[test]
fn a_read_only_source_takes_no_row() {
    let (mut app, _) = App::boot(None);
    let _ = cbs_with_points(&mut app, "salt-okunur-nokta", &["ad"], true);
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert!(
        super::target::choices(doc).is_empty(),
        "a read-only table is not offered"
    );
}

/// A CAD project: Çizim of any kind, Yol a line layer (active), Parsel an area layer, Nokta a point layer; Çizim and
/// Nokta locked with `locked`.
fn cad(app: &mut App, locked: bool) {
    open_edited(app, Objects::new(), |d| {
        let mut cizim = layer("cizim", "Çizim", "fg", 0.25);
        cizim["locked"] = json!(locked);
        let mut yol = layer("yol", "Yol", "#E5484D", 0.5);
        yol["geometry"] = json!("line");
        let mut parsel = layer("parsel", "Parsel", "#3E63DD", 0.35);
        parsel["geometry"] = json!("area");
        let mut nokta = layer("nokta", "Nokta", "fg", 0.25);
        nokta["geometry"] = json!("point");
        nokta["locked"] = json!(locked);
        d["layers"] = json!([cizim, yol, parsel, nokta]);
        d["activeLayer"] = json!("yol");
    });
}

#[test]
fn a_cad_projects_row_goes_to_the_layer_chosen_above_the_table_and_the_next_row_keeps_it() {
    let (mut app, _) = App::boot(None);
    cad(&mut app, false);
    let _ = app.update(Message::Run("point.editor"));
    let doc = &app.document.as_ref().expect("a drawing").model;
    let names: Vec<String> = super::target::choices(doc)
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(names, ["Çizim", "Nokta"], "the layers that take a point");
    // The active line layer takes none: the first that does is the row's.
    ev(&mut app, Event::AddRow);
    assert_eq!(app.draft_target().map(|t| t.id).as_deref(), Some("cizim"));
    ev(&mut app, Event::DraftLayer("nokta".into()));
    fill(&mut app, ["101", "487010", "4420010", "", ""]);
    enter(&mut app);
    let on = |app: &App, layer: &str| {
        app.document
            .as_ref()
            .map_or(0, |d| d.model.by_layer(layer).count())
    };
    assert_eq!((on(&app, "nokta"), on(&app, "cizim")), (1, 0));
    // The next row: its name one more, the layer chosen kept.
    let next = app.gis.points.draft.clone().expect("the next row");
    assert_eq!(
        (next.name.as_str(), next.layer.as_deref()),
        ("102", Some("nokta"))
    );
}

#[test]
fn a_cad_project_without_a_point_layer_makes_one_for_the_row() {
    let (mut app, _) = App::boot(None);
    // Çizim and Nokta locked: no layer takes a point.
    cad(&mut app, true);
    let _ = app.update(Message::Run("point.editor"));
    ev(&mut app, Event::AddRow);
    assert!(app.draft_target().is_none());
    assert!(
        last_said(&app).contains("Nokta katmanı oluştur ile bir nokta katmanı ekleyin"),
        "{}",
        last_said(&app)
    );
    fill(&mut app, ["201", "487020", "4420020", "3.5", "KZ"]);
    ev(&mut app, Event::CreateLayer);
    let made = app.draft_target().expect("a point layer made, the row's");
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(
        doc.layers().get(&made.id).and_then(|n| n.geometry),
        Some(kentos_commands::LayerGeometry::Point)
    );
    enter(&mut app);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let written: Vec<_> = doc.by_layer(&made.id).collect();
    assert_eq!(written.len(), 1, "{}", last_said(&app));
    match written[0] {
        kentos_commands::Entity::Point(p) => {
            assert_eq!((p.p.x, p.p.y, p.z), (487_020.0, 4_420_020.0, Some(3.5)));
            assert_eq!(p.base.attrs.get("Kod").map(String::as_str), Some("KZ"));
        }
        other => panic!("{other:?}"),
    }
}
