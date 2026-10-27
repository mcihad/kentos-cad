use crate::common::Bench;
use kentos_commands::{DrawingUnit, Workspace};
use kentos_geometry_core::entity::{HatchPattern, Shape};
use kentos_geometry_core::geom::wall::Detail;

fn drawing(plot: f64) -> Bench {
    let mut b = Bench::new("wall");
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.drawing_unit = Some(DrawingUnit::M);
    settings.plot_scale = plot;
    settings.arch = None;
    b.doc.set_settings(settings);
    b.start("wall");
    assert!(b.run(|session, cx| session.configure("S", "Dış duvar 25", cx)));
    b.click(0., 0.);
    b.click(10., 0.);
    b.confirm();
    b.confirm();
    b.spatial.sync(&b.doc);
    b
}

fn hatches(b: &Bench) -> Vec<HatchPattern> {
    b.spatial
        .store()
        .get(f64::from(b.newest().base().id))
        .unwrap()
        .shapes()
        .filter_map(|shape| {
            if let Shape::Hatch { pattern, .. } = shape {
                Some(pattern.clone())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn zoom_rebuilds_wall_components_and_preserves_model_and_hatch_paper_scale() {
    for plot in [100., 200.] {
        let mut b = drawing(plot);
        let before = b.doc.to_snapshot_v2();
        let source_pieces = b
            .spatial
            .store()
            .get(f64::from(b.newest().base().id))
            .unwrap()
            .shapes()
            .count();
        b.spatial.set_screen_wall_scale(&b.doc, Some(20.));
        assert_eq!(b.spatial.store().wall_view().detail, Detail::Detailed);
        assert_eq!(b.spatial.store().wall_view().scale, plot);
        let detailed_pieces = b
            .spatial
            .store()
            .get(f64::from(b.newest().base().id))
            .unwrap()
            .shapes()
            .count();
        assert!(
            detailed_pieces > source_pieces,
            "zoom brings component boundaries and hatches"
        );
        let patterns = hatches(&b);
        assert!(patterns.iter().any(|pattern| pattern.kind == "pattern"));
        for pattern in patterns.iter().filter(|pattern| pattern.kind == "pattern") {
            let factor = match pattern.name.as_deref() {
                Some("ANSI31") => 1.,
                Some("ANSI37") => 0.5,
                other => panic!("unexpected built-in material {other:?}"),
            };
            assert_eq!(pattern.scale, Some(factor * plot / 1000.));
        }
        b.spatial.set_screen_wall_scale(&b.doc, Some(10.));
        assert_eq!(
            hatches(&b),
            patterns,
            "camera zoom never scales the paper pattern"
        );
        assert_eq!(b.doc.to_snapshot_v2(), before);
        b.spatial.set_screen_wall_scale(&b.doc, None);
        assert_eq!(b.spatial.store().wall_view().detail, Detail::Schematic);
    }
}

#[test]
fn sheet_index_scale_overrides_screen_detail_and_restores_plot_patterns() {
    let mut b = drawing(100.);
    b.spatial.set_screen_wall_scale(&b.doc, Some(20.));
    b.spatial.set_wall_scale(Some(100.));
    b.spatial.set_screen_wall_scale(&b.doc, Some(10.));
    assert_eq!(b.spatial.store().wall_view().detail, Detail::Schematic);
    assert_eq!(b.spatial.store().wall_view().scale, 100.);
}
