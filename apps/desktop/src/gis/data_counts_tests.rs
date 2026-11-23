//! İstatistik and Grafik over a data layer counted by its source (docs/adr/9181), on the report's fixed data: six
//! maintenance points whose bedel is 0, 100, 250, NULL, 50 and 100 and whose durum is acil, planlı and tamam twice
//! each. Every row (Göster: Tümü) is counted by the GeoPackage's own `GROUP BY`, and the rows are not read:
//!
//! - the summaries: bedel 5 filled, 1 empty, least 0, mean 100, sum 500; durum three values, two each; the count
//!   says the source counted them;
//! - the summaries and the chosen field's statistics are the rows read's, decimals and Turkish words too;
//! - in an edit session the rows the drawing holds are counted as it has them, and the count says so;
//! - a press on a value selects its rows in the layer's table (Shift adds, Ctrl takes away);
//! - Grafik's bars by durum and bedel's histogram come from the counts as the rows read give them, a bar's press
//!   selects its rows, and a chart reading two fields reads the rows;
//! - a field the source leaves to the rows read sends the tab back to them; a source that cannot count says why.
//!
//! The counts, the presses, the chart and the sameness with the rows read hold for a PostGIS table too, counted by the
//! server, when `KENTOS_DATA_PG` names a database (`postgresql://user@host:port/db`, its password in `PGPASSWORD`).

use std::collections::BTreeSet;
use std::sync::Arc;

use kentos_data::{Coord, Driver as _, Field, FieldKind, Geometry, GeometryField, NewFeature, NewTable, Value};
use kentos_geometry_core::ops::charts::fields::{Detail, Summary};
use kentos_geometry_core::ops::charts::model::{Aggregate, ChartKind, Spec};

use super::{Counts, Kept};
use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, drive, scratch};
use crate::gis;

/// A row of the `bakim` table: kod, bedel, durum.
type Row = (String, Option<f64>, String);

/// The report's six points.
fn report() -> Vec<Row> {
    let bedel = [Some(0.0), Some(100.0), Some(250.0), None, Some(50.0), Some(100.0)];
    let durum = ["acil", "acil", "planlı", "planlı", "tamam", "tamam"];
    (0..6).map(|i| (format!("B0{}", i + 1), bedel[i], durum[i].to_owned())).collect()
}

/// A `table` of `rows` made in `source` (one there before is replaced).
fn fill(source: &dyn kentos_data::Source, table: &str, rows: &[Row]) {
    let mut tx = source.begin().expect("begin");
    tx.create_table(
        &NewTable {
            name: table.into(),
            title: None,
            description: None,
            key: "fid".into(),
            fields: vec![
                Field::new("kod", FieldKind::Text { max: Some(8) }),
                Field::new("bedel", FieldKind::Real),
                Field::new("durum", FieldKind::Text { max: Some(16) }),
            ],
            geometry: Some(GeometryField {
                name: "geom".into(),
                type_name: "POINT".into(),
                z: false,
                m: false,
                srid: Some(5255),
            }),
            spatial_index: true,
            system: None,
        },
        true,
    )
    .expect("table");
    let features: Vec<NewFeature> = rows
        .iter()
        .enumerate()
        .map(|(i, (kod, bedel, durum))| NewFeature {
            fid: None,
            geometry: Some(Geometry::Point(Some(Coord::xy(484_000.0 + i as f64 * 10.0, 4_420_000.0)))),
            values: vec![
                ("kod".into(), Value::Text(kod.clone())),
                ("bedel".into(), bedel.map_or(Value::Null, Value::Real)),
                ("durum".into(), Value::Text(durum.clone())),
            ],
        })
        .collect();
    tx.insert(table, &features).expect("rows");
    tx.commit().expect("commit");
}

/// A GeoPackage with a `bakim` table of `rows`.
fn table(tag: &str, rows: &[Row]) -> std::path::PathBuf {
    let path = scratch(tag).join("bakim.gpkg");
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let source = kentos_data_gpkg::Gpkg.open(path.to_str().expect("utf-8")).expect("open");
    fill(source.as_ref(), "bakim", rows);
    path
}

/// A drawing with the file's `bakim` as a data layer, and the layer's id.
fn open(path: &std::path::Path) -> (App, String) {
    open_on(&path.display().to_string(), "bakim")
}

/// A drawing with `table` of the source at `address` as a data layer, and the layer's id.
fn open_on(address: &str, table: &str) -> (App, String) {
    crate::data_layers::register();
    let mut app = app_with_drawing();
    let described = crate::data_layers::describe(address, table).expect("described");
    let name = described.name.clone();
    app.add_data_layers(vec![described]).expect("added");
    let layer = crate::data_layers_tests::node(&app, &name).expect("the layer").id.clone();
    (app, layer)
}

fn ev(app: &mut App, e: crate::statistics::Event) {
    let task = app.update(Message::Gis(gis::Event::Statistics(e)));
    drive(app, task);
}

/// İstatistik open on `layer`.
fn statistics(app: &mut App, layer: &str) {
    let task = app.update(Message::Run("data.statistics"));
    drive(app, task);
    ev(app, crate::statistics::Event::Layer(layer.to_owned()));
}

/// The fields the tabs asked for counted by the source (a frame does it in the program).
fn counted(app: &mut App) {
    let task = app.data_counts_tasks();
    drive(app, task);
}

/// A row of İstatistik's summary by its field's label: its cells.
fn summary(app: &App, label: &str) -> Vec<String> {
    let (_, rows) = app.statistics_seen().expect("the tab is open");
    rows.into_iter()
        .map(|r| r.split(" | ").map(str::to_owned).collect::<Vec<_>>())
        .find(|cells| cells.first().is_some_and(|c| c == label))
        .unwrap_or_else(|| panic!("no summary of {label}"))
}

fn number(cell: &str) -> Option<f64> {
    cell.replace(',', ".").parse::<f64>().ok()
}

/// The count beside Göster and what the summary table says without rows.
fn words(app: &App) -> (String, String) {
    let now = app.gis.charts.stats.borrow();
    (app.stats_count_words(&now), app.stats_empty_words(&now))
}

/// The tab's summaries and the statistics of each field as İstatistik shows them now.
fn shown(app: &mut App) -> Vec<(Summary, Detail)> {
    let _ = app.statistics_seen();
    let keys = app.gis.charts.stats.borrow().keys.clone();
    let mut out = Vec::new();
    for key in keys {
        ev(app, crate::statistics::Event::Field(key.clone()));
        let _ = app.statistics_seen();
        let now = app.gis.charts.stats.borrow();
        let i = now.keys.iter().position(|k| *k == key).expect("the field");
        out.push((now.summaries[i].clone().expect("summed up"), now.detail.clone().expect("its statistics")));
    }
    out
}

/// The summaries and statistics of the rows read (docs/adr/9130): the reference a count by the source must meet.
fn read(app: &mut App, layer: &str) -> Vec<(Summary, Detail)> {
    // Asked, then read after the message, as a frame does it.
    let _ = app.data_layer_rows_keyed(layer);
    let task = app.data_rows_tasks();
    drive(app, task);
    let doc = app.document.as_ref().expect("a drawing");
    let mut d = crate::charts::data::LayerData::of(app, doc, layer);
    assert_eq!(d.from, crate::charts::data::From::Source, "read");
    let all: Vec<u32> = (0..d.slots.len() as u32).collect();
    (0..d.work.data.columns.len()).map(|i| (d.work.summary(i, &all), d.work.detail(i, &all).clone())).collect()
}

/// A field's statistics without its value rows' places (a count's are its values', the rows read's their rows').
fn without_places(d: &Detail) -> Detail {
    let mut d = d.clone();
    for v in &mut d.values {
        v.rows.clear();
    }
    d
}

#[test]
fn istatistik_counts_a_data_layer_in_its_source_and_reads_no_row() {
    let path = table("istatistik-sayim", &report());
    let (mut app, layer) = open(&path);
    the_report_is_counted(&mut app, &layer);
}

/// İstatistik over the report's points (`layer`) counted by their source, the rows never read.
fn the_report_is_counted(app: &mut App, layer: &str) {
    statistics(app, layer);
    // Asked of the source: the tab says it counts.
    assert_eq!(app.statistics_seen().expect("the tab is open").0, "0 nesne");
    assert_eq!(words(app), ("Kaynak sayıyor…".to_owned(), "Kaynak sayıyor…".to_owned()));
    counted(app);
    let bedel = summary(app, "bedel");
    // Alan | Tür | Dolu | Boş | Benzersiz | En az | En çok | Ortalama | Toplam
    assert_eq!((bedel[2].as_str(), bedel[3].as_str(), bedel[4].as_str()), ("5", "1", "4"), "{bedel:?}");
    assert_eq!((number(&bedel[5]), number(&bedel[6])), (Some(0.0), Some(250.0)), "{bedel:?}");
    assert_eq!((number(&bedel[7]), number(&bedel[8])), (Some(100.0), Some(500.0)), "{bedel:?}");
    let durum = summary(app, "durum");
    assert_eq!((durum[2].as_str(), durum[4].as_str()), ("6", "3"), "{durum:?}");
    assert_eq!(app.statistics_seen().expect("open").0, "6 nesne");
    assert_eq!(words(app).0, "6 satır, kaynakta sayıldı");
    // durum's values: two each, a third each.
    ev(app, crate::statistics::Event::Field("durum".into()));
    let _ = app.statistics_seen();
    let values: Vec<(String, usize, String)> = app.gis.charts.stats.borrow().detail.as_ref().expect("its statistics").values
        .iter()
        .map(|v| (v.text.clone(), v.count, v.share.clone()))
        .collect();
    assert_eq!(
        values,
        [("acil".into(), 2, "33.3".into()), ("planlı".into(), 2, "33.3".into()), ("tamam".into(), 2, "33.3".into())]
    );
    // No row was read for it: the rows' reader was never asked.
    assert_eq!(app.layer_rows_from(), None);
}

/// 240 rows: decimals at several scales, a value many times, empty ones; words that sort in the Turkish alphabet.
fn decimals() -> Vec<Row> {
    let bedel = [Some(12.5), Some(0.25), None, Some(-3.75), Some(1250.0), Some(12.50), Some(7.125), Some(0.0)];
    let durum = ["İnceleme", "ılık", "acil", "çalışma", "Işık", "acil", "", "Ödendi"];
    (0..240).map(|i| (format!("K{i:03}"), bedel[i % 8].map(|b| b + (i / 8) as f64 % 3.0), durum[i % 7].to_owned())).collect()
}

#[test]
fn a_count_by_the_source_gives_the_rows_reads_statistics() {
    let path = table("istatistik-esit", &decimals());
    let (mut app, layer) = open(&path);
    the_counts_are_the_rows_reads(&mut app, &layer);
}

/// Every field's summary and statistics as the source counted them, then as the rows read give them: the same.
fn the_counts_are_the_rows_reads(app: &mut App, layer: &str) {
    statistics(app, layer);
    let _ = app.statistics_seen();
    counted(app);
    let by_source = shown(app);
    assert!(app.gis.charts.stats.borrow().counted.is_some(), "counted by the source");
    let by_rows = read(app, layer);
    assert_eq!(by_source.len(), by_rows.len());
    for ((s, d), (rs, rd)) in by_source.iter().zip(&by_rows) {
        assert_eq!(s, rs, "{}", s.key);
        assert_eq!(without_places(d), without_places(rd), "{}", s.key);
    }
}

/// Row `fid` of the data layer taken and its `field` changed on this device.
fn change(app: &mut App, layer: &str, fid: i64, field: &str, value: &str) {
    let doc = app.document.as_ref().expect("a drawing");
    let node = doc.model.layers().get(layer).cloned().expect("the layer");
    let settings = doc.model.settings().clone();
    let rows = kentos_native_application::data_write::rows_to_take(&node, &settings, &[fid], &|| false).expect("read");
    let doc = app.document.as_mut().expect("a drawing");
    let taken = doc.model.take_data_rows(layer, rows).expect("taken")[0].expect("the row");
    let slot = doc.model.slot_of(taken).expect("its object");
    let mut e = doc.model.get(slot).cloned().expect("the row");
    e.base_mut().attrs.insert(field.into(), value.into());
    assert!(doc.model.update(slot, e));
}

#[test]
fn in_an_edit_session_the_rows_the_drawing_holds_are_counted_as_it_has_them() {
    let path = table("istatistik-oturum", &report());
    let (mut app, layer) = open(&path);
    let _ = app.data_edits_event(crate::data_edits::Event::Start(layer.clone()));
    // B03's bedel 250 → 300; B06 (100, tamam) deleted; a new row (40, acil).
    change(&mut app, &layer, 3, "bedel", "300");
    let doc = app.document.as_mut().expect("a drawing");
    assert_eq!(doc.model.delete_data_rows(&layer, &[6]), Ok(1));
    let input = serde_json::from_value(serde_json::json!({
        "layerId": layer,
        "objects": [{ "geometry": { "kind": "point", "p": { "x": 484_100.0, "y": 4_420_000.0 } },
                      "attrs": { "kod": "B07", "bedel": "40", "durum": "acil" } }]
    }))
    .expect("an input");
    let done = kentos_native_application::create::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut app.document.as_mut().expect("a drawing").model),
        input,
    );
    assert!(matches!(done, kentos_commands::CommandResult::Completed { .. }), "{done:?}");
    statistics(&mut app, &layer);
    let _ = app.statistics_seen();
    counted(&mut app);
    // bedel: 0, 100, NULL, 50 from the source; 300 and 40 from the drawing.
    let bedel = summary(&app, "bedel");
    assert_eq!((bedel[2].as_str(), bedel[3].as_str()), ("5", "1"), "{bedel:?}");
    assert_eq!((number(&bedel[5]), number(&bedel[6]), number(&bedel[8])), (Some(0.0), Some(300.0), Some(490.0)), "{bedel:?}");
    // durum: acil B01, B02 and the new one; planlı B03 (taken) and B04; tamam B05.
    ev(&mut app, crate::statistics::Event::Field("durum".into()));
    let _ = app.statistics_seen();
    let values: Vec<(String, usize)> = app.gis.charts.stats.borrow().detail.as_ref().expect("its statistics").values
        .iter()
        .map(|v| (v.text.clone(), v.count))
        .collect();
    assert_eq!(values, [("acil".into(), 3), ("planlı".into(), 2), ("tamam".into(), 1)]);
    assert_eq!(words(&app).0, "6 satır, kaynakta sayıldı; kaynağa yazılmamış değişiklikler bu cihazdaki hâliyle");
    // As the rows read with the drawing's give them.
    let by_source = shown(&mut app);
    let by_rows = read(&mut app, &layer);
    for ((s, d), (rs, rd)) in by_source.iter().zip(&by_rows) {
        assert_eq!(s, rs, "{}", s.key);
        assert_eq!(without_places(d), without_places(rd), "{}", s.key);
    }
}

/// The value row of the chosen field's statistics showing `text`.
fn value_row(app: &App, text: &str) -> usize {
    app.gis.charts.stats.borrow().detail.as_ref().expect("its statistics").values
        .iter()
        .position(|v| v.text == text)
        .unwrap_or_else(|| panic!("no value {text}"))
}

fn chosen(app: &App, layer: &str) -> Vec<i64> {
    app.gis.features.data.selected_of(layer)
}

#[test]
fn a_press_on_a_counted_value_selects_its_rows_in_the_table() {
    let path = table("istatistik-secim", &report());
    let (mut app, layer) = open(&path);
    presses_select_the_rows(&mut app, &layer);
}

/// A press on the report's counted values (`layer`) selects their rows in the table: Shift adds, Ctrl takes away, the
/// empty value its row.
fn presses_select_the_rows(app: &mut App, layer: &str) {
    statistics(app, layer);
    let _ = app.statistics_seen();
    counted(app);
    ev(app, crate::statistics::Event::Field("durum".into()));
    let _ = app.statistics_seen();
    let acil = value_row(app, "acil");
    ev(app, crate::statistics::Event::Value(acil));
    assert_eq!(chosen(app, layer), [1, 2]);
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(said.iter().any(|t| t == "“acil”: 2 satır seçildi."), "{said:#?}");
    // Shift adds tamam's; Ctrl takes acil's away.
    app.modifiers = iced::keyboard::Modifiers::SHIFT;
    let tamam = value_row(app, "tamam");
    ev(app, crate::statistics::Event::Value(tamam));
    assert_eq!(chosen(app, layer), [1, 2, 5, 6]);
    app.modifiers = iced::keyboard::Modifiers::CTRL;
    ev(app, crate::statistics::Event::Value(acil));
    assert_eq!(chosen(app, layer), [5, 6]);
    app.modifiers = iced::keyboard::Modifiers::default();
    // The empty bedel: the row without one.
    ev(app, crate::statistics::Event::Field("bedel".into()));
    let _ = app.statistics_seen();
    let empty = value_row(app, kentos_geometry_core::ops::charts::data::EMPTY);
    ev(app, crate::statistics::Event::Value(empty));
    assert_eq!(chosen(app, layer), [4]);
}

/// Grafik open on `layer` with `spec`, its field counted.
fn chart(app: &mut App, layer: &str, spec: Spec) {
    app.open_chart(Some(layer.to_owned()), None, Some(spec));
    app.chart_refresh();
    counted(app);
    app.chart_refresh();
}

/// The table of the chart `spec` makes over the rows read (docs/adr/9130), and its notes.
fn chart_of_rows(app: &mut App, layer: &str, spec: &Spec) -> (Vec<Vec<String>>, Vec<String>) {
    let _ = app.data_layer_rows_keyed(layer);
    let task = app.data_rows_tasks();
    drive(app, task);
    let doc = app.document.as_ref().expect("a drawing");
    let mut d = crate::charts::data::LayerData::of(app, doc, layer);
    let all: Vec<u32> = (0..d.slots.len() as u32).collect();
    let m = d.work.chart(spec, &all).expect("a chart");
    (m.table.rows.clone(), m.notes.clone())
}

#[test]
fn grafik_draws_a_one_field_chart_from_the_counts_and_selects_a_bars_rows() {
    let path = table("grafik-sayim", &report());
    let (mut app, layer) = open(&path);
    grafik_draws_from_the_counts(&mut app, &layer);
}

/// Grafik over the report's points (`layer`): durum's bars and bedel's histogram from the counts, as the rows read give
/// them; a bar's press selects its rows; bedel's sum by durum reads the rows.
fn grafik_draws_from_the_counts(app: &mut App, layer: &str) {
    let bars = Spec { category: Some("durum".into()), ..Spec::new(ChartKind::Bar) };
    chart(app, layer, bars);
    let seen = app.chart_seen().expect("a chart");
    assert_eq!(seen.rows, ["acil | 2", "planlı | 2", "tamam | 2"]);
    assert!(app.gis.charts.chart.borrow().counted, "counted by the source");
    assert_eq!(app.layer_rows_from(), None, "no row read");
    // A bar's press: its rows chosen in the table.
    let acil = seen.rows.iter().position(|r| r.starts_with("acil")).expect("acil's bar") as u32;
    let task = app.update(Message::Gis(gis::Event::Charts(crate::charts::Event::Press(Some(acil)))));
    drive(app, task);
    assert_eq!(chosen(app, layer), [1, 2]);
    // bedel's histogram: as the rows read give it, the empty one said.
    let histogram = Spec { value: Some("bedel".into()), ..Spec::new(ChartKind::Histogram) };
    chart(app, layer, histogram);
    let (table, notes) = {
        let now = app.gis.charts.chart.borrow();
        assert!(now.counted, "counted by the source");
        (now.table.clone().expect("its numbers").rows, now.notes.clone())
    };
    let whole = app.whole_spec().expect("a spec");
    assert_eq!(chart_of_rows(app, layer, &whole), (table, notes));
    // bedel's sum by durum reads two fields: the rows are read for it.
    let sums = Spec { category: Some("durum".into()), value: Some("bedel".into()), aggregate: Aggregate::Sum, ..Spec::new(ChartKind::Bar) };
    app.open_chart(Some(layer.to_owned()), None, Some(sums));
    app.chart_refresh();
    assert!(!app.gis.charts.chart.borrow().counted, "read, not counted");
    let task = app.data_rows_tasks();
    drive(app, task);
    let seen = app.chart_seen().expect("a chart");
    assert_eq!(seen.rows, ["acil | 2 | 100", "planlı | 2 | 250", "tamam | 2 | 150"]);
}

#[test]
fn a_field_left_to_the_rows_reads_them_and_a_source_that_cannot_count_says_why() {
    let path = table("istatistik-geri", &report());
    let (mut app, layer) = open(&path);
    statistics(&mut app, &layer);
    let _ = app.statistics_seen();
    counted(&mut app);
    let stamp = app.counts_stamp().expect("a drawing");
    let put = |app: &mut App, field: &str, counts: Counts| {
        app.gis.data_counts.kept.borrow_mut().insert(
            (layer.clone(), field.to_owned()),
            Kept { stamp, held: Arc::new(BTreeSet::new()), counts },
        );
    };
    // A source that cannot count a field: the tab says why.
    put(&mut app, "bedel", Counts::Failed("bağlantı kesildi".into()));
    let _ = app.statistics_seen();
    assert_eq!(words(&app), ("Kaynak sayamadı".to_owned(), "Kaynak sayamadı: bağlantı kesildi".to_owned()));
    // A field with too many values: the rows are read and counted as before.
    put(&mut app, "bedel", Counts::Rows);
    let _ = app.statistics_seen();
    assert_eq!(app.layer_rows_from(), Some(crate::charts::data::From::Reading));
    let task = app.data_rows_tasks();
    drive(&mut app, task);
    let bedel = summary(&app, "bedel");
    assert_eq!((bedel[2].as_str(), number(&bedel[8])), ("5", Some(500.0)), "{bedel:?}");
    assert_eq!(words(&app).0, "6 satır, kaynaktan");
}

#[test]
fn a_postgis_layer_is_counted_by_the_server_alike() {
    let Some(address) = std::env::var("KENTOS_DATA_PG").ok().filter(|a| !a.is_empty()) else {
        eprintln!("KENTOS_DATA_PG yok: PostGIS'te kaynağın sayımı atlandı");
        return;
    };
    crate::data_layers::register();
    let source = kentos_data::registry().source(&address).expect("the source");
    let id = std::process::id();
    let (a, b) = (format!("public.kentos_istatistik_{id}"), format!("public.kentos_istatistik_esit_{id}"));
    fill(source.as_ref(), &a, &report());
    fill(source.as_ref(), &b, &decimals());
    let (mut app, layer) = open_on(&address, &a);
    the_report_is_counted(&mut app, &layer);
    let (mut app, layer) = open_on(&address, &a);
    presses_select_the_rows(&mut app, &layer);
    let (mut app, layer) = open_on(&address, &a);
    grafik_draws_from_the_counts(&mut app, &layer);
    let (mut app, layer) = open_on(&address, &b);
    the_counts_are_the_rows_reads(&mut app, &layer);
    let mut tx = source.begin().expect("begin");
    tx.drop_table(&a).expect("drop");
    tx.drop_table(&b).expect("drop");
    tx.commit().expect("commit");
}

/// The pictures (`.run/gui-evidence/kullanim-9181/`): İstatistik over the report's data counted by the GeoPackage,
/// durum chosen with its values; Grafik's bars by durum from the same counts, acil's rows chosen.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    let path = table("istatistik-resim", &report());
    // The view on the points, the panel tall enough for the values.
    let framed = |app: &mut App, layer: &str| {
        app.zoom_to_layer(layer);
        app.bottom_dragged(Some(400.0), std::time::Instant::now());
    };
    crate::files_testing::usage_shots("9181", "01-istatistik", || {
        let (mut app, layer) = open(&path);
        statistics(&mut app, &layer);
        let _ = app.statistics_seen();
        counted(&mut app);
        ev(&mut app, crate::statistics::Event::Field("durum".into()));
        let _ = app.statistics_seen();
        framed(&mut app, &layer);
        app
    });
    crate::files_testing::usage_shots("9181", "02-grafik", || {
        let (mut app, layer) = open(&path);
        let bars = Spec { category: Some("durum".into()), ..Spec::new(ChartKind::Bar) };
        chart(&mut app, &layer, bars);
        let task = app.update(Message::Gis(gis::Event::Charts(crate::charts::Event::Press(Some(0)))));
        drive(&mut app, task);
        framed(&mut app, &layer);
        app
    });
}

/// A million parcels counted by the source and read (docs/adr/9181 §3), one way a process so that each one's peak
/// memory is its own: `KENTOS_OLCUM_YOL` is `yapim` (the table made), `sayim` (İstatistik's every field counted by the
/// source and summed up) or `okuma` (the rows read and every field summed up, docs/adr/9130). The table is `parsel` of
/// the file `KENTOS_OLCUM_GPKG`, or `public.kentos_olcum_parsel` of the database `KENTOS_OLCUM_PG` (its password in
/// `PGPASSWORD`).
#[test]
#[ignore = "a measurement"]
fn measure_a_million_parcels() {
    use kentos_data::Curve;
    let (address, table) = match std::env::var("KENTOS_OLCUM_PG").ok().filter(|a| !a.is_empty()) {
        Some(pg) => (pg, "public.kentos_olcum_parsel"),
        None => (std::env::var("KENTOS_OLCUM_GPKG").expect("KENTOS_OLCUM_GPKG or KENTOS_OLCUM_PG"), "parsel"),
    };
    let way = std::env::var("KENTOS_OLCUM_YOL").unwrap_or_default();
    const N: usize = 1_000_000;
    crate::data_layers::register();
    if way == "yapim" {
        if table == "parsel" {
            let _ = std::fs::remove_file(&address);
            kentos_geopackage::GeoPackage::create(std::path::Path::new(&address)).expect("a file");
        }
        let source = kentos_data::registry().source(&address).expect("the source");
        let mut tx = source.begin().expect("begin");
        tx.create_table(
            &NewTable {
                name: table.into(),
                title: None,
                description: None,
                key: "fid".into(),
                fields: vec![
                    Field::new("kod", FieldKind::Text { max: Some(12) }),
                    Field::new("ada", FieldKind::INT32),
                    Field::new("parsel", FieldKind::INT32),
                    Field::new("mahalle", FieldKind::Text { max: Some(32) }),
                    Field::new("kullanim", FieldKind::Text { max: Some(16) }),
                    Field::new("alan", FieldKind::Real),
                    Field::new("bedel", FieldKind::Real),
                ],
                geometry: Some(GeometryField {
                    name: "geom".into(),
                    type_name: "POLYGON".into(),
                    z: false,
                    m: false,
                    srid: Some(5255),
                }),
                spatial_index: true,
                system: None,
            },
            true,
        )
        .expect("table");
        tx.commit().expect("commit");
        let mahalle = ["Çankaya", "Öveçler", "Işıklar", "İlkadım", "Kızılay", "Bahçelievler", "Şehitlik", "Ümitköy"];
        let kullanim = ["Konut", "Ticaret", "Park", "Eğitim", "Sağlık", "Sanayi"];
        for part in 0..N / 50_000 {
            let mut tx = source.begin().expect("begin");
            let rows: Vec<NewFeature> = (part * 50_000..(part + 1) * 50_000)
                .map(|i| {
                    let (x, y) = (480_000.0 + (i % 1000) as f64 * 20.0, 4_400_000.0 + (i / 1000) as f64 * 20.0);
                    let ring = vec![Coord::xy(x, y), Coord::xy(x + 18.0, y), Coord::xy(x + 18.0, y + 18.0), Coord::xy(x, y + 18.0), Coord::xy(x, y)];
                    NewFeature {
                        fid: None,
                        geometry: Some(Geometry::Polygon(vec![Curve::Line(ring)])),
                        values: vec![
                            ("kod".into(), Value::Text(format!("P{i:07}"))),
                            ("ada".into(), Value::Int((i / 200) as i64 % 5000)),
                            ("parsel".into(), Value::Int((i % 200) as i64 + 1)),
                            ("mahalle".into(), Value::Text(mahalle[(i / 7) % mahalle.len()].into())),
                            ("kullanim".into(), Value::Text(kullanim[(i * 7 + i / 13) % kullanim.len()].into())),
                            ("alan".into(), Value::Real(((i * 7919) % 900_000) as f64 / 100.0 + 100.0)),
                            ("bedel".into(), if i % 10 == 3 { Value::Null } else { Value::Real(((i * 31) % 2000) as f64 * 250.0) }),
                        ],
                    }
                })
                .collect();
            tx.insert(table, &rows).expect("rows");
            tx.commit().expect("commit");
        }
        println!("yapıldı: {table}");
        return;
    }
    let (mut app, layer) = open_on(&address, table);
    let start = std::time::Instant::now();
    let summaries: Vec<String> = match way.as_str() {
        "sayim" => {
            statistics(&mut app, &layer);
            let _ = app.statistics_seen();
            counted(&mut app);
            assert!(app.gis.charts.stats.borrow().counted.is_some(), "counted by the source");
            app.statistics_seen().expect("the tab is open").1
        }
        "okuma" => {
            let _ = app.data_layer_rows_keyed(&layer);
            let task = app.data_rows_tasks();
            drive(&mut app, task);
            let doc = app.document.as_ref().expect("a drawing");
            let mut d = crate::charts::data::LayerData::of(&app, doc, &layer);
            let all: Vec<u32> = (0..d.slots.len() as u32).collect();
            (0..d.work.data.columns.len())
                .map(|i| crate::statistics::summary_cells(&d.work.summary(i, &all)).join(" | "))
                .collect()
        }
        other => panic!("KENTOS_OLCUM_YOL: yapim, sayim or okuma, not {other:?}"),
    };
    let ms = start.elapsed().as_millis();
    for s in &summaries {
        println!("{s}");
    }
    println!("{way}: {ms} ms");
}
