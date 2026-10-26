//! Zaman and Senaryo of docs/adr/0210 in a GIS project (`temporal.kcad`,
//! written by `scripts/fixtures/temporal_scene.py`): parcels split in 2018,
//! one ended in 2021 and a new one in 2022; buildings shown from their year
//! on; breakdowns on their days; a road and a hidden scenario widening it.
//! The web's are `shots.mjs temporal`. `tools_screens` takes them in the dark
//! and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=zaman-serit,zaman-surgu,zaman-yeni-surum,zaman-bilinen,zaman-ayarlari-kayit,zaman-surgu-2015,zaman-ayarlari,senaryo-olustur,senaryo-gosterilen,senaryo-uygula,zaman-karsilastir cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! KENTOS_SHOTS_ONLY=sensor-gozlemleri,sensor-gozlemleri-an,zaman-disa-aktar cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! The sensors' (docs/adr/0646 §6): a water meter layer of four meters through `cad.sensors.ingest`, one silent for
//! an hour and a half; Sensör gözlemleri now and at a moment typed; FlatGeobuf dışa aktar with
//! Zaman sürgüsünün zamanıyla.
//!
//! Test code only.

use crate::app::{App, Message};
use crate::document::Document;
use crate::tools_screens::Scene;

pub(crate) const DRAWING: &str = include_str!("../../../fixtures/interaction/v1/temporal.kcad");
const X0: f64 = 487_000.0;
const Y0: f64 = 4_420_000.0;

/// The drawing, the view on its parcels, the road and the scenario's park.
pub(crate) fn opened(app: &mut App) {
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let doc = Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.tab = "map";
    app.command_expanded = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: X0 - 15.0,
            min_y: Y0 - 8.0,
            max_x: X0 + 115.0,
            max_y: Y0 + 64.0,
        },
        24.0,
    );
}

fn run(app: &mut App, id: &'static str) {
    let task = app.run(id);
    crate::files_testing::drive(app, task);
}

/// The parcels' versions given their system time (docs/adr/0537): each recorded a few days after it began, 104's
/// recorded late (in 2024, though valid since 2022); the parcel layer told which attributes hold it.
fn recorded(app: &mut App) {
    use kentos_native_application::{ExecutionContext, registry};
    let doc = app.document.as_mut().expect("open");
    let days = [
        (1, "2010-03-10", Some("2018-06-05")),
        (2, "2018-06-05", None),
        (3, "2018-06-05", None),
        (4, "2012-01-03", None),
        (5, "2015-07-10", Some("2021-01-04")),
        (6, "2024-01-15", None),
    ];
    for (slot, start, end) in days {
        let uid = doc
            .model
            .uid(kentos_domain::Slot(slot))
            .expect("a parcel")
            .to_string();
        let mut attrs = serde_json::json!({ "kayit_tarihi": start });
        if let Some(e) = end {
            attrs["kayit_bitis"] = serde_json::json!(e);
        }
        let answer = registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            "cad.entities.set",
            kentos_commands::CallOp::Execute,
            serde_json::json!({ "uids": [uid], "attrs": attrs, "operation": "attributes" }),
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
    }
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc.model),
        "cad.layers.time",
        kentos_commands::CallOp::Execute,
        serde_json::json!({ "layer": "parsel", "time": {
            "start": "gecerlilik_baslangic", "end": "gecerlilik_bitis", "key": "parsel_no",
            "recordedStart": "kayit_tarihi", "recordedEnd": "kayit_bitis" } }),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    let doc = app.document.as_ref().expect("open");
    app.spatial.sync(&doc.model);
}

/// A meter layer (Sayaçlar: a temporal layer of records, its asset field `sayac`, its moment `an`, its receipt
/// `alim`) and the meters' readings through `cad.sensors.ingest` (docs/adr/0646 §4): S-04 silent since 09:50.
pub(crate) fn meters(app: &mut App) {
    use kentos_native_application::{ExecutionContext, registry};
    let doc = app.document.as_mut().expect("open");
    let mut call = |command: &str, input: serde_json::Value| {
        let answer = registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            command,
            kentos_commands::CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{command}: {answer}");
        answer
    };
    let added = call(
        "cad.layers.edit",
        serde_json::json!({ "operation": "add", "kind": "layer", "name": "Sayaçlar" }),
    );
    let layer = added["output"]["layer"].as_str().expect("a layer").to_owned();
    call(
        "cad.layers.time",
        serde_json::json!({ "layer": layer, "time": {
            "start": "an", "key": "sayac", "recordedStart": "alim", "recordedEnd": "alim_bitis" } }),
    );
    let mut observations = Vec::new();
    for (meter, from, n, base) in [("S-01", 10, 12, 4.2), ("S-02", 10, 12, 3.1), ("S-03", 10, 12, 5.6), ("S-04", 10, 6, 2.4)] {
        for k in 0..n {
            let minutes = from * 60 - 60 + k * 10;
            observations.push(serde_json::json!({
                "asset": meter,
                "at": format!("2024-06-01T{:02}:{:02}:00Z", minutes / 60, minutes % 60),
                "value": ((base + 0.05 * k as f64) * 100.0_f64).round() / 100.0,
                "quality": if k % 5 == 4 { "şüpheli" } else { "iyi" },
            }));
        }
    }
    call(
        "cad.sensors.ingest",
        serde_json::json!({ "layer": layer, "valueField": "debi", "qualityField": "kalite",
            "receivedAt": "2024-06-01T11:55:00Z", "observations": observations }),
    );
    let doc = app.document.as_ref().expect("open");
    app.spatial.sync(&doc.model);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        // The CBS ribbon's Harita tab with its Zaman and Senaryo panels.
        ("zaman-serit", |app| opened(app)),
        // The slider open at its last position (the latest state): 2024.
        ("zaman-surgu", |app| {
            opened(app);
            run(app, "time.slider");
        }),
        // Back to 2015: parcel 101 whole, 103 not yet, one building.
        ("zaman-surgu-2015", |app| {
            opened(app);
            run(app, "time.slider");
            let k = app.time.slider.last - 9;
            let _ = app.update(Message::Time(crate::temporal::Event::Go(k)));
        }),
        // Yeni sürüm oluştur through cad.time.version (docs/adr/0622 §9): parcel 102 ends on 1 January 2020 and its new
        // version, starting that day, is selected; the slider stays on 2024, where the new version shows.
        ("zaman-yeni-surum", |app| {
            opened(app);
            run(app, "time.slider");
            app.selection.set([kentos_domain::Slot(4)]);
            run(app, "tool.timeVersion");
            let _ = app.submit_line("01.01.2020");
        }),
        // Zaman ayarları over the parcels.
        ("zaman-ayarlari", |app| {
            opened(app);
            run(app, "time.layer");
        }),
        // Zaman ayarları with the versions' system time: Kayıt başlangıcı and Kayıt bitişi (docs/adr/0537).
        ("zaman-ayarlari-kayit", |app| {
            opened(app);
            recorded(app);
            run(app, "time.layer");
        }),
        // 2024 as it was known on 1 January 2023: parcel 104 (recorded only in 2024) not yet there.
        ("zaman-bilinen", |app| {
            opened(app);
            recorded(app);
            run(app, "time.slider");
            let _ = app.update(Message::Time(crate::temporal::Event::Known(
                "01.01.2023".into(),
            )));
            let _ = app.update(Message::Time(crate::temporal::Event::KnownDone));
        }),
        // Senaryo oluştur with the road and the parcels checked.
        ("senaryo-olustur", |app| {
            opened(app);
            run(app, "scenario.create");
            let _ = app.update(Message::Time(crate::temporal::Event::Scenario(
                crate::temporal::scenario::Event::Check("yol".into(), true),
            )));
        }),
        // Yol genişletme A shown: the wide road and the new park; the status bar says so.
        ("senaryo-gosterilen", |app| {
            opened(app);
            let _ = app.update(Message::Time(crate::temporal::Event::Show("alt-a".into())));
        }),
        // Senaryoyu uygula's question.
        ("senaryo-uygula", |app| {
            opened(app);
            let _ = app.update(Message::Time(crate::temporal::Event::Show("alt-a".into())));
            let _ = app.update(Message::Time(crate::temporal::Event::AskApply(
                "alt-a".into(),
            )));
        }),
        // Zamanı karşılaştır: the parcels at the slider's position before and its own, paired by parcel number.
        ("zaman-karsilastir", |app| {
            opened(app);
            run(app, "time.slider");
            // 2019: the step before is 2018, the year 101 was split.
            let k = app.time.slider.last - 5;
            let _ = app.update(Message::Time(crate::temporal::Event::Go(k)));
            run(app, "time.compare");
            let _ = app.update(Message::DataCompare(crate::data_compare::Event::Compare));
        }),
        // Sensör gözlemleri now: the moment asked is now, every meter's stream stopped long ago.
        ("sensor-gozlemleri", |app| {
            opened(app);
            meters(app);
            run(app, "time.sensors");
        }),
        // At 11:50 on 1 June 2024, typed: S-04 silent for an hour and a half, the others current.
        ("sensor-gozlemleri-an", |app| {
            opened(app);
            meters(app);
            run(app, "time.sensors");
            let _ = app.update(Message::Time(crate::temporal::Event::Sensors(
                crate::temporal::sensors::Event::At("01.06.2024 11:50".into()),
            )));
        }),
        // FlatGeobuf dışa aktar with the slider open on 2015 and Zaman sürgüsünün zamanıyla ticked.
        ("zaman-disa-aktar", |app| {
            opened(app);
            run(app, "time.slider");
            let k = app.time.slider.last - 9;
            let _ = app.update(Message::Time(crate::temporal::Event::Go(k)));
            let _ = app.update(Message::Time(crate::temporal::Event::ExportTime));
            run(app, "file.export.fgb");
        }),
    ]
}

