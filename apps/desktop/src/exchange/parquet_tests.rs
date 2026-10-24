//! GeoParquet içe ve dışa aktar on the desktop (docs/adr/0628): the window shows the plan of `file.import.parquet`;
//! İçe aktar takes the rows as one undo step; a file in another system waits for the move; Dışa aktar… writes through
//! `file.export.parquet`; Var olan katmanla birleştir meets the rows with a chosen layer's objects by their lasting ids
//! or a key. Pictures: `parquet-*` (`screens`, ignored).

use std::path::PathBuf;

use kentos_commands::{ExportParquet, MergeDuplicates, ParquetCompression};

use super::merge_form::{Duplicates, MergeEvent};
use super::parquet_exchange::Event as P;
use super::tests::{count, run, said, send};
use super::{Event, Window};
use crate::app::{App, Message, Picker};
use crate::files_testing::{app_with_drawing, scratch};

fn fixture(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(path)
}

fn importing(app: &App) -> &super::parquet_exchange::ImportState {
    match &app.exchange {
        Some(Window::ParquetImport(s)) => s,
        _ => panic!("the GeoParquet import window"),
    }
}

fn open(app: &mut App, path: PathBuf) {
    app.picker = Picker::File(path);
    run(app, "file.import.parquet");
}

/// The index the merge section's Katman list gives a layer (0: the one the file's name meets).
fn layer_index(app: &App, id: &str) -> usize {
    importing(app)
        .merge
        .layer_choices()
        .iter()
        .position(|(l, _)| l == id)
        .expect("the layer is offered")
        + 1
}

#[test]
fn a_geoparquet_goes_in_as_one_undo_step() {
    let mut app = app_with_drawing();
    let before = count(&app);
    open(&mut app, fixture("geoparquet/v1/parsel.parquet"));
    let planned = importing(&app).plan.as_ref().expect("planned").0.objects;
    assert_eq!(planned, 3);
    send(&mut app, Event::Parquet(P::Run));
    assert!(app.exchange.is_none());
    assert_eq!(count(&app), before + planned as usize);
    assert!(
        said(&app, "“parsel.parquet” alındı"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(count(&app), before);
}

#[test]
fn a_file_in_another_system_waits_for_the_move_to_be_asked() {
    let mut app = app_with_drawing();
    open(&mut app, fixture("geoparquet/v1/wgs84.parquet"));
    assert!(importing(&app).plan.is_err(), "nothing moved silently");
    send(&mut app, Event::Parquet(P::Reproject));
    assert!(importing(&app).plan.is_ok());
}

#[test]
fn the_rows_taken_again_meet_their_objects_by_their_lasting_ids() {
    let mut app = app_with_drawing();
    open(&mut app, fixture("geoparquet/v1/parsel.parquet"));
    send(&mut app, Event::Parquet(P::Run));
    let after = count(&app);
    open(&mut app, fixture("geoparquet/v1/parsel.parquet"));
    send(&mut app, Event::Parquet(P::Merge(MergeEvent::On)));
    let (plan, _) = importing(&app).plan.as_ref().expect("planned");
    let report = &plan.merged[0];
    assert_eq!((report.added, report.unchanged), (0, 3), "{report:?}");
    send(&mut app, Event::Parquet(P::Run));
    assert_eq!(count(&app), after, "nothing doubled");
}

#[test]
fn a_layer_of_another_name_is_chosen_for_the_merge() {
    let mut app = app_with_drawing();
    // The drawing's cizim layer as a GeoParquet of another name, its objects' lasting ids in kentos_uid.
    let path = scratch("parquet-katman").join("disa.parquet");
    let model = app.document.as_ref().expect("a drawing").model.clone();
    let (written, _) = kentos_headless::file_commands::export_parquet(
        &model,
        &ExportParquet {
            path: path.display().to_string(),
            layers: vec!["cizim".into()],
            overwrite: true,
            compression: ParquetCompression::Snappy,
            covering: true,
        },
    )
    .expect("written");
    let before = count(&app);
    open(&mut app, path);
    send(&mut app, Event::Parquet(P::Merge(MergeEvent::On)));
    // No layer is named as the file: the rows would go onto a new one, but their lasting ids are the drawing's.
    let refused = importing(&app).plan.as_ref().expect_err("refused");
    assert!(refused.contains("kalıcı kimliği"), "{refused}");
    let cizim = layer_index(&app, "cizim");
    send(&mut app, Event::Parquet(P::Merge(MergeEvent::Layer(cizim))));
    assert_eq!(importing(&app).merge.layer(), Some("cizim".to_owned()));
    let (plan, _) = importing(&app).plan.as_ref().expect("planned");
    let report = &plan.merged[0];
    assert_eq!(report.layer, "cizim");
    assert_eq!(report.added, 0, "every row met its object: {report:?}");
    assert_eq!(
        report.updated + report.unchanged + report.rejected,
        written.rows
    );
    send(&mut app, Event::Parquet(P::Run));
    assert_eq!(count(&app), before, "nothing doubled");
}

#[test]
fn the_drawing_goes_out_as_one_geoparquet() {
    let mut app = app_with_drawing();
    run(&mut app, "file.export.parquet");
    let Some(Window::ParquetExport(s)) = &app.exchange else {
        panic!("the export window");
    };
    let planned = s.plan.as_ref().expect("planned").rows;
    assert!(
        s.plan
            .as_ref()
            .expect("planned")
            .columns
            .iter()
            .any(|c| c == "bbox.xmin"),
        "the covering column by default"
    );
    send(
        &mut app,
        Event::Parquet(P::Compression(super::parquet_exchange::Packing(
            ParquetCompression::None,
        ))),
    );
    send(&mut app, Event::Parquet(P::Covering));
    let Some(Window::ParquetExport(s)) = &app.exchange else {
        panic!("the export window");
    };
    let plan = s.plan.as_ref().expect("planned");
    assert_eq!(plan.compression, ParquetCompression::None);
    assert!(
        !plan.columns.iter().any(|c| c.starts_with("bbox.")),
        "{:?}",
        plan.columns
    );
    let path = scratch("parquet-disa").join("cizim.parquet");
    send(&mut app, Event::Parquet(P::Saved(Some(path.clone()))));
    // Written off the window's thread: the answer comes back as a message.
    let bytes = std::fs::read(&path).expect("written");
    let file = kentos_formats::parquet::File::open(bytes).expect("reads");
    assert_eq!(file.meta.num_rows, i64::from(planned));
    assert!(file.meta.value("geo").is_some(), "a GeoParquet");
}

#[test]
#[ignore = "pictures, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
        let picture = |app: &mut App, name: &str| {
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(app, App::view, &mut update);
            let _ = snapshot.render(app.view(), &app.theme());
            let file = out.join(format!("{name}-{width}x{height}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        };
        for mode in ["dark", "light"] {
            let fresh = || {
                let mut app = app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                app
            };
            let mut app = fresh();
            open(&mut app, fixture("geoparquet/v1/parsel.parquet"));
            picture(&mut app, &format!("parquet-{mode}-1-ice-aktar"));
            send(&mut app, Event::Parquet(P::Run));
            picture(&mut app, &format!("parquet-{mode}-2-alindi"));
            let mut app = fresh();
            open(&mut app, fixture("geoparquet/v1/wgs84.parquet"));
            picture(&mut app, &format!("parquet-{mode}-3-sistem"));
            // Var olan katmanla birleştir: the parcels taken, then a file that repeats a key met by parsel_no.
            let mut app = fresh();
            open(&mut app, fixture("import-merge/v1/parquet/parsel.parquet"));
            send(&mut app, Event::Parquet(P::Run));
            open(
                &mut app,
                fixture("import-merge/v1/parquet/yinelenen/parsel.parquet"),
            );
            send(&mut app, Event::Parquet(P::Merge(MergeEvent::On)));
            let key = importing(&app)
                .merge
                .view_names()
                .iter()
                .position(|n| n == "parsel_no")
                .expect("parsel_no is offered")
                + 1;
            send(&mut app, Event::Parquet(P::Merge(MergeEvent::Key(key))));
            picture(&mut app, &format!("parquet-{mode}-4-birlestir-durdu"));
            send(
                &mut app,
                Event::Parquet(P::Merge(MergeEvent::Duplicates(Duplicates(
                    MergeDuplicates::First,
                )))),
            );
            send(&mut app, Event::Parquet(P::Merge(MergeEvent::Rejects)));
            picture(&mut app, &format!("parquet-{mode}-5-birlestir"));
            let mut app = fresh();
            let path = scratch("parquet-resim").join("disa.parquet");
            let model = app.document.as_ref().expect("a drawing").model.clone();
            kentos_headless::file_commands::export_parquet(
                &model,
                &ExportParquet {
                    path: path.display().to_string(),
                    layers: vec!["cizim".into()],
                    overwrite: true,
                    compression: ParquetCompression::Snappy,
                    covering: true,
                },
            )
            .expect("written");
            open(&mut app, path);
            send(&mut app, Event::Parquet(P::Merge(MergeEvent::On)));
            picture(&mut app, &format!("parquet-{mode}-6-katman-yok"));
            let cizim = layer_index(&app, "cizim");
            send(&mut app, Event::Parquet(P::Merge(MergeEvent::Layer(cizim))));
            picture(&mut app, &format!("parquet-{mode}-7-katman-secildi"));
            let mut app = fresh();
            run(&mut app, "file.export.parquet");
            picture(&mut app, &format!("parquet-{mode}-8-disa-aktar"));
        }
    }
}
