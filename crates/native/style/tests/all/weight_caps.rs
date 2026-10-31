//! docs/adr/0793 §1: the model screen holds a paper width (a stroke's or a hatch's world width, paper mm drawn at the
//! plot scale) to 1/(METRES_PER_PX × plot scale) px a world unit; a width on the ground, in px, a marker, and every
//! width on paper (a sheet's map, `hold_weights` off) hold nothing.

use kentos_native_style::StylePalette;
use kentos_native_style::batches::{DecodeOptions, decode};
use kentos_native_style::library::StyleLibrary;
use kentos_native_style::program::METRES_PER_PX;
use kentos_style_core::style::batch::Batches;

fn batches() -> Batches {
    let stroke = |unit: &str| {
        format!(
            r##"{{"kind":"stroke","style":{{"color":"#000000","opacity":1,"width":0.035,"unit":"{unit}","dash":null,"dashOffset":0,"cap":"butt","join":"miter","blur":0,"level":0}},"from":0,"len":6,"bounds":[0,0,1,0],"w":0.0175,"h":0}}"##
        )
    };
    let hatch = r##"{"kind":"fill","style":{"kind":"hatch","color":"#000000","opacity":1,"angle":0,"spacing":1,"width":0.02,"offset":0,"dash":null,"dashOffset":0,"unit":"world","level":0},"from":6,"len":6,"bounds":[0,0,1,1],"w":0,"h":0}"##;
    let json = format!(
        "[{},{},{},{}]",
        stroke("world"),
        stroke("world"),
        stroke("px"),
        hatch
    );
    Batches {
        json,
        data: vec![0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0],
        pictures: Vec::new(),
        dropped: 0,
        // The second stroke is a symbol's metres.
        ground: vec![1],
    }
}

fn caps(hold: bool, plot_scale: f64) -> Vec<f64> {
    let palette = StylePalette {
        fg: "#E6E6E6".into(),
        fg_dim: "#8A8A8A".into(),
        ink: "#000000".into(),
        paper: "#FFFFFF".into(),
    };
    let library = StyleLibrary::default();
    let layer = decode(
        batches(),
        &DecodeOptions {
            palette: &palette,
            plot_scale,
            library: &library,
            view: Default::default(),
            hold_weights: hold,
        },
    )
    .expect("decodes");
    layer.batches.iter().map(|b| b.weight_cap).collect()
}

#[test]
fn a_paper_width_is_held_on_the_model_screen_only() {
    for scale in [1.0, 20.0, 100.0, 500.0] {
        let held = 1.0 / (METRES_PER_PX * scale);
        assert_eq!(caps(true, scale), [held, 0.0, 0.0, held], "1:{scale}");
        assert_eq!(caps(false, scale), [0.0; 4], "1:{scale} on paper");
    }
    // At 1:100 a 0.25 mm pen, 0.025 m in the world, is held to 0.94 px: AutoCAD's model space shows it a pixel wide.
    let px = 0.025 * caps(true, 100.0)[0];
    assert!((px - 0.25 / 0.264_58).abs() < 1e-9, "{px}");
}
