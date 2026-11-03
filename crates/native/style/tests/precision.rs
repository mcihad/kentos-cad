//! A sub-millimetre symbol must retain its height far from the drawing anchor.
use kentos_geometry_core::Vec2;
use kentos_native_style::{
    StylePalette,
    batches::{DecodeOptions, StyledLayer, decode},
    color::ViewColors,
    cpu::{CpuView, paint},
    library::StyleLibrary,
    picture::NoImages,
};
use kentos_style_core::style::{
    batch::BatchSink,
    prim::{FillPaint, Sink},
};

fn rectangle(center: Vec2) -> StyledLayer {
    let mut sink = BatchSink::new(Vec2::new(0., 0.));
    sink.set_position_low(true);
    let ring = vec![
        Vec2::new(center.x - 0.0003, center.y - 0.00025),
        Vec2::new(center.x + 0.0003, center.y - 0.00025),
        Vec2::new(center.x + 0.0003, center.y + 0.00025),
        Vec2::new(center.x - 0.0003, center.y + 0.00025),
    ];
    sink.fill(
        &FillPaint::Solid {
            color: "#FFFFFF".into(),
            opacity: 1.,
            level: 0.,
        },
        &[ring],
    );
    decode(
        sink.finish(),
        &DecodeOptions {
            palette: &StylePalette {
                fg: "#FFFFFF".into(),
                fg_dim: "#888888".into(),
                ink: "#000000".into(),
                paper: "#FFFFFF".into(),
            },
            plot_scale: 50.,
            library: &StyleLibrary::default(),
            view: ViewColors::default(),
            hold_weights: false,
        },
    )
    .unwrap()
}

#[test]
fn a_half_millimetre_rectangle_keeps_its_height_far_from_the_anchor() {
    let size = 64usize;
    for center in [
        Vec2::new(0., 0.),
        Vec2::new(4_000_000_000., 1_000_000.),
        Vec2::new(-4_000_000_000., -1_000_000.),
    ] {
        let layer = rectangle(center);
        let k = 20_000.;
        let view = CpuView {
            m: [k, 0., 0., -k],
            t: [32. - center.x * k, 32. + center.y * k],
            px_per_m: k,
            dpr: 2.,
            scale: 50.,
        };
        let rgba = paint(size as u32, size as u32, &view, &[&layer], &NoImages).unwrap();
        let rows = (0..size)
            .filter(|&y| (0..size).any(|x| rgba[(y * size + x) * 4 + 3] > 128))
            .count();
        assert!(
            (9..=11).contains(&rows),
            "center={center:?}: height={rows}, expected 10 pixels"
        );
    }
}

#[test]
fn legacy_backends_keep_the_same_high_stream_without_a_low_allocation() {
    let build = |precise| {
        let mut sink = BatchSink::new(Vec2::new(0., 0.));
        sink.set_position_low(precise);
        let ring = vec![
            Vec2::new(16960., 16960.),
            Vec2::new(16960.0006, 16960.),
            Vec2::new(16960., 16960.0005),
        ];
        sink.fill(
            &FillPaint::Solid {
                color: "#FFFFFF".into(),
                opacity: 1.,
                level: 0.,
            },
            &[ring],
        );
        sink.finish()
    };
    let old = build(false);
    let precise = build(true);
    assert_eq!(old.json, precise.json);
    assert_eq!(old.data, precise.data);
    assert!(old.low.is_empty());
    assert_eq!(precise.low.len(), precise.data.len());
    assert!(precise.low.iter().any(|v| *v != 0.));
}
