//! A DXF's new dimensions (docs/adr/0147 §8) through the exchange windows,
//! as the user drives them: another program's ordinate, arc length and
//! jogged radius come in as KentOS's dimensions, the ordinate from another
//! origin and the aligned one by their blocks; out again, the window says
//! how they are written. fixtures/formats/v1/dimension-kinds.dxf; the
//! readers' own tests are crates/shared/formats/tests/dxf.rs.

use std::path::PathBuf;

use kentos_contracts::{DimensionEntity, DimensionStyle, Entity};

use super::tests::{count, fixture, run, said, send, shot};
use super::{Event, drawing_import, dxf_export};
use crate::app::{App, Picker};
use crate::files_testing::app_with_drawing;

fn dimensions(app: &App) -> Vec<DimensionEntity> {
    let model = &app.document.as_ref().expect("open").model;
    model
        .entities()
        .filter_map(|e| match e {
            Entity::Dimension(d) => Some(d.clone()),
            _ => None,
        })
        .collect()
}

fn import(app: &mut App) {
    app.picker = Picker::File(fixture("dimension-kinds.dxf"));
    run(app, "file.import.dxf");
}

#[test]
fn a_dxfs_new_dimensions_come_in_as_dimensions() {
    let mut app = app_with_drawing();
    let (before, had) = (count(&app), dimensions(&app).len());
    import(&mut app);
    send(&mut app, Event::DrawingImport(drawing_import::Event::Run));
    // Four dimensions, the two by their blocks: four lines and two values.
    assert_eq!(count(&app) - before, 10);
    let dims = dimensions(&app);
    let new = &dims[had..];
    let kinds: Vec<_> = new
        .iter()
        .map(|d| (d.style, d.angle, d.text.as_deref(), d.mask))
        .collect();
    assert_eq!(
        kinds,
        [
            (Some(DimensionStyle::Ordinate), Some(0.0), None, false),
            (
                Some(DimensionStyle::Ordinate),
                Some(90.0),
                Some("X=4412320.00"),
                false
            ),
            (Some(DimensionStyle::ArcLength), None, None, true),
            (Some(DimensionStyle::Jogged), None, None, false),
        ]
    );
    // The style's height (DIMTXT 2 × DIMSCALE 1.5).
    assert!(new.iter().all(|d| d.height == 3.0), "{new:?}");
    assert!(
        said(&app, "“dimension-kinds.dxf”: 10 nesne"),
        "the import is reported"
    );
    // One step takes them back.
    let _ = app.update(crate::app::Message::Run("edit.undo"));
    assert_eq!(count(&app), before);
}

/// KentOS's own DXF (fixtures/formats/v1/dxf-write/dimensions.dxf): all
/// seven come back as they were written, Semt and Eğim from their KentOS
/// data, the slope's elevations and the mask too.
#[test]
fn kentos_s_own_dimensions_come_back_as_they_were() {
    let mut app = app_with_drawing();
    let had = dimensions(&app).len();
    app.picker = Picker::File(fixture("dxf-write/dimensions.dxf"));
    run(&mut app, "file.import.dxf");
    send(&mut app, Event::DrawingImport(drawing_import::Event::Run));
    let dims = dimensions(&app);
    let kinds: Vec<_> = dims[had..]
        .iter()
        .map(|d| (d.style, d.mask, d.za, d.zb))
        .collect();
    assert_eq!(
        kinds,
        [
            (Some(DimensionStyle::Ordinate), false, None, None),
            (Some(DimensionStyle::Ordinate), false, None, None),
            (Some(DimensionStyle::ArcLength), false, None, None),
            (Some(DimensionStyle::Jogged), false, None, None),
            (Some(DimensionStyle::Azimuth), false, None, None),
            (
                Some(DimensionStyle::Slope),
                false,
                Some(102.4),
                Some(101.15)
            ),
            (None, true, None, None),
        ]
    );
}

/// The window over dimension-kinds.dxf (what came in by its block, and
/// why) and the drawing with its dimensions in; then KentOS's own file,
/// every kind back, and the DXF window writing them (the line on Semt and
/// Eğim). The web's are `shots.mjs dimensionsdxf`.
///
/// ```text
/// cargo test -p kentos-desktop exchange::dimension_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for size in [(1440.0, 900.0), (1100.0, 650.0)] {
        for mode in ["dark", "light"] {
            let mut app = app_with_drawing();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            import(&mut app);
            shot(
                &mut app,
                &out,
                &format!("aktar-{mode}-22-dxf-olculer"),
                size,
            );
            send(&mut app, Event::DrawingImport(drawing_import::Event::Run));
            app.selection.set(Vec::<kentos_domain::Slot>::new());
            let frame = kentos_render_wgpu::Bounds {
                min_x: 452_262.0,
                min_y: 4_412_278.0,
                max_x: 452_482.0,
                max_y: 4_412_350.0,
            };
            app.viewport.camera.fit(&frame, 24.0);
            shot(
                &mut app,
                &out,
                &format!("aktar-{mode}-23-dxf-olculer-alindi"),
                size,
            );
            let _ = app.update(crate::app::Message::Run("edit.undo"));
            app.picker = Picker::File(fixture("dxf-write/dimensions.dxf"));
            run(&mut app, "file.import.dxf");
            send(&mut app, Event::DrawingImport(drawing_import::Event::Run));
            app.selection.set(Vec::<kentos_domain::Slot>::new());
            app.viewport.camera.fit(&frame, 24.0);
            shot(
                &mut app,
                &out,
                &format!("aktar-{mode}-24-dxf-kentos-olculeri"),
                size,
            );
            run(&mut app, "file.export.dxf");
            send(
                &mut app,
                Event::DxfExport(dxf_export::Event::Scope(dxf_export::Scope::All)),
            );
            shot(
                &mut app,
                &out,
                &format!("aktar-{mode}-25-dxf-ver-olculer"),
                size,
            );
        }
    }
}
