//! Zaman and Senaryo of docs/adr/0210 in a GIS project (`temporal.kcad`,
//! written by `scripts/fixtures/temporal_scene.py`): parcels split in 2018,
//! one ended in 2021 and a new one in 2022; buildings shown from their year
//! on; breakdowns on their days; a road and a hidden scenario widening it.
//! The web's are `shots.mjs temporal`. `tools_screens` takes them in the dark
//! and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=zaman-serit,zaman-surgu,zaman-surgu-2015,zaman-ayarlari,senaryo-olustur,senaryo-gosterilen,senaryo-uygula,zaman-karsilastir cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
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
        kentos_contracts::DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
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
        // Zaman ayarları over the parcels.
        ("zaman-ayarlari", |app| {
            opened(app);
            run(app, "time.layer");
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
    ]
}
