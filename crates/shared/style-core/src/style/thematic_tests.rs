//! The thematic renderers through a layer build (docs/adr/0213): what each
//! draws, as batches and pictures, on small layers.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::store::Store;

use super::batch::Batches;
use super::build::{LayerObjects, MODE_RENDERER, Program, View, ViewFrame, build_layer_in};
use super::thematic::{hex, parse_rgba, ramp_color, share, size_at, step};

fn v(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

fn square(x0: f64, y0: f64, side: f64) -> Shape {
    Shape::Polygon {
        pts: vec![
            v(x0, y0),
            v(x0 + side, y0),
            v(x0 + side, y0 + side),
            v(x0, y0 + side),
        ],
        holes: None,
        bulges: None,
        parts: None,
    }
}

fn point(x: f64, y: f64) -> Shape {
    Shape::Point {
        p: v(x, y),
        z: None,
        parts: None,
    }
}

/// A layer built as the page builds it: every object through the renderer, the
/// simple set 0, the table of the fields its expressions read (`attrs`).
fn build(
    program: &str,
    objects: &[(Shape, Vec<(&str, &str)>)],
    clip: Option<Bounds>,
    frame: Option<ViewFrame<'_>>,
) -> (Vec<Json>, Batches) {
    let program = Program::read(program).expect("program");
    let mut store = Store::new();
    let (mut ids, mut how, mut texts, mut lens) =
        (Vec::new(), Vec::new(), String::new(), Vec::new());
    for (i, (shape, attrs)) in objects.iter().enumerate() {
        let id = (i + 1) as f64;
        store.put(id, "k", false, shape.clone());
        ids.push(id);
        how.extend_from_slice(&[MODE_RENDERER, 0, 0, 0]);
        for f in &program.fields {
            match attrs.iter().find(|(k, _)| k == f) {
                Some((_, value)) => {
                    texts.push_str(value);
                    lens.push(value.encode_utf16().count() as i32);
                }
                None => lens.push(-1),
            }
        }
    }
    let o = LayerObjects {
        ids: &ids,
        objects: &how,
        texts: &texts,
        text_lens: &lens,
        numbers: &[],
        pieces: &[],
    };
    let out = build_layer_in(
        &store,
        &program,
        &o,
        clip.as_ref(),
        v(0.0, 0.0),
        1000.0,
        false,
        View::default(),
        frame,
    )
    .expect("build");
    let Json::Arr(batches) = Json::parse(&out.json).expect("json") else {
        panic!("not an array: {}", out.json);
    };
    (batches, out)
}

fn text(v: &Json) -> &str {
    match v {
        Json::Str(s) => s,
        _ => "",
    }
}

fn program(renderer: &str) -> String {
    let simple = r##"{"marker":{"type":"marker","layers":[{"type":"shape","shape":"circle","size":2,"fill":"#808080"}]},"line":{"type":"line","layers":[{"type":"simpleLine","color":"#808080","width":0.25}]},"fill":{"type":"fill","layers":[{"type":"simpleFill","color":"#C0C0C0"}]}}"##;
    format!(
        r##"{{"symbols":{{}},"renderer":{renderer},"sets":[{simple}],"refs":[],"colors":["#3E63DD"],"assets":{{}}}}"##
    )
}

/// The batches of a kind with their colours (fills' and strokes' `color`, markers' fill or text).
fn of(batches: &[Json], kind: &str) -> Vec<String> {
    batches
        .iter()
        .filter(|b| text(b.get("kind")) == kind)
        .map(|b| {
            let s = b.get("style");
            match kind {
                "marker" => match s.get("kind") {
                    Json::Str(k) if k == "text" => format!("text:{}", text(s.get("text"))),
                    _ => text(s.get("fill")).to_owned(),
                },
                _ => text(s.get("color")).to_owned(),
            }
        })
        .collect()
}

/// The numbers of the batches of a kind, by batch.
fn data_of(batches: &[Json], data: &[f32], kind: &str) -> Vec<Vec<f32>> {
    batches
        .iter()
        .filter(|b| text(b.get("kind")) == kind)
        .map(|b| {
            let (Json::Num(from), Json::Num(len)) = (b.get("from"), b.get("len")) else {
                panic!("from, len");
            };
            data[*from as usize..(*from + *len) as usize].to_vec()
        })
        .collect()
}

const RAMP: &str = r##"["#FFF5B8","#FDB863","#E66101","#A50F15"]"##;
const FILL: &str = r##"{"type":"fill","layers":[{"type":"simpleFill","color":"#000000"},{"type":"simpleLine","color":"#333333","width":0.2}]}"##;

#[test]
fn unclassed_colours_the_fills_and_keeps_the_edges() {
    let r = format!(
        r#"{{"type":"unclassed","expr":"D","min":0,"max":100,"ramp":{RAMP},"symbols":{{"fill":{FILL}}}}}"#
    );
    let objects = [
        (square(0.0, 0.0, 10.0), vec![("D", "0")]),
        (square(20.0, 0.0, 10.0), vec![("D", "50")]),
        (square(40.0, 0.0, 10.0), vec![("D", "100")]),
        (square(60.0, 0.0, 10.0), vec![]),
    ];
    let (b, _) = build(&program(&r), &objects, None, None);
    let stops: Vec<[u8; 4]> = ["#FFF5B8", "#FDB863", "#E66101", "#A50F15"]
        .iter()
        .filter_map(|c| parse_rgba(c))
        .collect();
    let want: Vec<String> = [0.0, 50.0, 100.0]
        .iter()
        .map(|d| {
            hex(ramp_color(
                &stops,
                f64::from(step(share(*d, 0.0, 100.0))) / 255.0,
            ))
        })
        .collect();
    assert_eq!(
        of(&b, "fill"),
        want,
        "renkler sırayla; değeri olmayan çizilmez"
    );
    assert_eq!(
        of(&b, "stroke"),
        vec!["#333333".to_owned()],
        "çerçeve rengi değişmez"
    );
}

#[test]
fn proportional_sizes_points_and_areas() {
    let marker = r##"{"type":"marker","layers":[{"type":"shape","shape":"circle","size":4,"fill":"#3E63DD"}]}"##;
    let r = format!(
        r#"{{"type":"proportional","expr":"N","minValue":0,"maxValue":100,"minSize":2,"maxSize":10,"unit":"mm","scaling":"area","symbols":{{"marker":{marker},"fill":{FILL}}}}}"#
    );
    let objects = [
        (point(0.0, 0.0), vec![("N", "0")]),
        (point(10.0, 0.0), vec![("N", "25")]),
        (square(20.0, 0.0, 10.0), vec![("N", "100")]),
    ];
    let (b, out) = build(&program(&r), &objects, None, None);
    let markers = data_of(&b, &out.data, "marker");
    assert_eq!(markers.len(), 1, "boyları ayrı, toplu çizimi bir");
    let sizes: Vec<f32> = markers[0].chunks(5).map(|m| m[3]).collect();
    // Paper mm at 1:1000: a millimetre is a metre.
    let want: Vec<f32> = [0.0, 25.0, 100.0]
        .iter()
        .map(|n| {
            size_at(
                f64::from(step(share(*n, 0.0, 100.0))) / 255.0,
                2.0,
                10.0,
                0.5,
            ) as f32
        })
        .collect();
    for (g, w) in sizes.iter().zip(&want) {
        assert!((g - w).abs() < 1e-4, "{g} ≠ {w}");
    }
    assert_eq!(
        of(&b, "fill"),
        vec!["#000000".to_owned()],
        "alanın kendisi zemin"
    );
}

#[test]
fn bivariate_takes_the_grids_colour() {
    let r = format!(
        r##"{{"type":"bivariate","exprX":"X","exprY":"Y","breaksX":[10],"breaksY":[5],"colors":["#E8E8E8","#5AC8C8","#BE64AC","#3B4994"],"symbols":{{"fill":{FILL}}}}}"##
    );
    let objects = [
        (square(0.0, 0.0, 1.0), vec![("X", "1"), ("Y", "1")]),
        (square(2.0, 0.0, 1.0), vec![("X", "10"), ("Y", "1")]),
        (square(4.0, 0.0, 1.0), vec![("X", "1"), ("Y", "9")]),
        (square(6.0, 0.0, 1.0), vec![("X", "20"), ("Y", "5")]),
        (square(8.0, 0.0, 1.0), vec![("X", "20")]),
    ];
    let (b, _) = build(&program(&r), &objects, None, None);
    assert_eq!(
        of(&b, "fill"),
        vec!["#E8E8E8", "#5AC8C8", "#BE64AC", "#3B4994"]
    );
}

#[test]
fn dot_density_puts_its_dots_inside() {
    let r = r##"{"type":"dotDensity","fields":[{"expr":"A","color":"#E15759"},{"expr":"B","color":"#4E79A7"}],"dotValue":10,"dotSize":1,"unit":"px","seed":3}"##;
    let objects = [(square(0.0, 0.0, 100.0), vec![("A", "125"), ("B", "40")])];
    let (b, out) = build(&program(r), &objects, None, None);
    assert_eq!(of(&b, "marker"), vec!["#E15759", "#4E79A7"]);
    let counts: Vec<usize> = data_of(&b, &out.data, "marker")
        .iter()
        .map(|d| d.len() / 5)
        .collect();
    assert_eq!(counts, vec![13, 4]);
    for d in data_of(&b, &out.data, "marker") {
        for m in d.chunks(5) {
            assert!(
                (0.0..=100.0).contains(&m[0]) && (0.0..=100.0).contains(&m[1]),
                "{m:?}"
            );
        }
    }
    assert_eq!(out.dropped, 0);
}

#[test]
fn a_pie_and_its_outline() {
    let r = r##"{"type":"chart","kind":"pie","fields":[{"expr":"A","color":"#E15759"},{"expr":"B","color":"#4E79A7"},{"expr":"C","color":"#59A14F"}],"size":10,"unit":"mm","outline":{"color":"#FFFFFF","width":0.2}}"##;
    let objects = [(point(0.0, 0.0), vec![("A", "1"), ("B", "2"), ("C", "0")])];
    let (b, out) = build(&program(r), &objects, None, None);
    assert_eq!(
        of(&b, "fill"),
        vec!["#E15759", "#4E79A7"],
        "sıfır dilim yok"
    );
    assert_eq!(of(&b, "stroke"), vec!["#FFFFFF"]);
    // The two slices fill the circle of 10 m (10 mm at 1:1000) in 72 steps of 5°: ½ · 72 · 5² · sin 5°.
    let area: f64 = data_of(&b, &out.data, "fill")
        .iter()
        .flat_map(|d| {
            d.chunks(6).map(|t| {
                f64::from(((t[2] - t[0]) * (t[5] - t[1]) - (t[4] - t[0]) * (t[3] - t[1])).abs())
                    / 2.0
            })
        })
        .sum();
    let want = 0.5 * 72.0 * 25.0 * kentos_geometry_core::jsmath::sin(5f64.to_radians());
    assert!((area - want).abs() < 1e-3, "{area} ≠ {want}");
}

#[test]
fn a_heat_map_is_one_picture_over_its_box() {
    let r =
        r##"{"type":"heatmap","radius":8,"unit":"px","ramp":["#2B83BA00","#D7191C"],"quality":2}"##;
    let objects = [
        (point(10.0, 10.0), vec![]),
        (point(12.0, 10.0), vec![]),
        (square(0.0, 0.0, 5.0), vec![]),
    ];
    let clip = Bounds {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 40.0,
        max_y: 20.0,
    };
    let frame = ViewFrame {
        px_per_m: 1.0,
        picture: "heat:k:1",
    };
    let (b, out) = build(&program(r), &objects, Some(clip), Some(frame));
    assert_eq!(out.pictures.len(), 1);
    let p = &out.pictures[0];
    assert_eq!((p.key.as_str(), p.width, p.height), ("heat:k:1", 20, 10));
    assert_eq!(p.rgba.len(), 20 * 10 * 4);
    // The hottest cell is opaque red; a far one clear.
    assert!(p.rgba.chunks(4).any(|c| c == [0xD7, 0x19, 0x1C, 0xFF]));
    assert_eq!(&p.rgba[(19 * 4)..(19 * 4 + 4)], &[0, 0, 0, 0]);
    assert_eq!(b.len(), 1, "yalnız resim: alan çizilmez");
    assert_eq!(text(b[0].get("style").get("kind")), "image");
    // Without a view a heat map draws nothing.
    let (b, out) = build(&program(r), &objects, None, None);
    assert!(b.is_empty() && out.pictures.is_empty());
}

#[test]
fn clusters_count_and_singles_keep_their_look() {
    let r = r#"{"type":"cluster","distance":10,"unit":"px"}"#;
    let objects = [
        (point(0.0, 0.0), vec![]),
        (point(3.0, 0.0), vec![]),
        (point(0.0, 4.0), vec![]),
        (point(100.0, 100.0), vec![]),
        (square(50.0, 50.0, 5.0), vec![]),
    ];
    let frame = ViewFrame {
        px_per_m: 1.0,
        picture: "",
    };
    let (b, out) = build(&program(r), &objects, None, Some(frame));
    let markers = of(&b, "marker");
    assert!(
        markers.contains(&"#808080".to_owned()),
        "tek nokta basit görünüşüyle: {markers:?}"
    );
    assert!(
        markers.contains(&"#3E63DD".to_owned()),
        "küme katmanın renginde: {markers:?}"
    );
    assert!(
        markers.contains(&"text:3".to_owned()),
        "sayısı: {markers:?}"
    );
    assert_eq!(
        of(&b, "fill"),
        vec!["#C0C0C0"],
        "alan iç işleyicisiyle (basit görünüş)"
    );
    let centre: Vec<f32> = data_of(&b, &out.data, "marker")
        .into_iter()
        .zip(&markers)
        .find(|(_, m)| m.as_str() == "#3E63DD")
        .map(|(d, _)| d[..2].to_vec())
        .unwrap_or_default();
    assert_eq!(centre, vec![1.0, 4.0 / 3.0]);
}

#[test]
fn displaced_points_go_round_their_centre() {
    let r = r##"{"type":"displacement","tolerance":2,"unit":"px","placement":"ring","circle":{"color":"#7D7D7D","width":1}}"##;
    let objects = [
        (point(5.0, 5.0), vec![]),
        (point(5.0, 5.0), vec![]),
        (point(5.5, 5.0), vec![]),
    ];
    let frame = ViewFrame {
        px_per_m: 1.0,
        picture: "",
    };
    let (b, out) = build(&program(r), &objects, None, Some(frame));
    let markers = data_of(&b, &out.data, "marker");
    assert_eq!(markers.len(), 1);
    let at: Vec<(f32, f32)> = markers[0].chunks(5).map(|m| (m[0], m[1])).collect();
    assert_eq!(at.len(), 3);
    // All at the same distance from the centre, none at it.
    let c = (5.5 / 3.0 + 10.0 / 3.0, 5.0);
    let r0 = ((at[0].0 - c.0 as f32).powi(2) + (at[0].1 - c.1 as f32).powi(2)).sqrt();
    assert!(r0 > 1.0);
    for p in &at {
        let r = ((p.0 - c.0 as f32).powi(2) + (p.1 - c.1 as f32).powi(2)).sqrt();
        assert!((r - r0).abs() < 1e-3, "{r} ≠ {r0}");
    }
    assert_eq!(of(&b, "stroke"), vec!["#7D7D7D"], "halkanın dairesi");
}

#[test]
fn inverted_fills_the_box_outside_the_areas() {
    let r = format!(r#"{{"type":"inverted","symbols":{{"fill":{FILL}}}}}"#);
    let objects = [
        (square(10.0, 10.0, 20.0), vec![]),
        (point(1.0, 1.0), vec![]),
    ];
    let clip = Bounds {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 100.0,
        max_y: 50.0,
    };
    let (b, out) = build(&program(&r), &objects, Some(clip), None);
    assert_eq!(of(&b, "fill"), vec!["#000000"]);
    assert_eq!(
        of(&b, "stroke"),
        vec!["#333333"],
        "alanların kenarı, kutunun değil"
    );
    let area: f64 = data_of(&b, &out.data, "fill")
        .iter()
        .flat_map(|d| {
            d.chunks(6).map(|t| {
                f64::from(((t[2] - t[0]) * (t[5] - t[1]) - (t[4] - t[0]) * (t[3] - t[1])).abs())
                    / 2.0
            })
        })
        .sum();
    assert!((area - (5000.0 - 400.0)).abs() < 1e-3, "{area}");
    assert!(of(&b, "marker").is_empty(), "nokta çizilmez");
}
