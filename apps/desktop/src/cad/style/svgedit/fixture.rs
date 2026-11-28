//! The SVG editor's rules held to their frozen answers
//! (`fixtures/style/v1/svgedit.json`, recorded from the TypeScript editor
//! this one took the place of; kept since by `scripts/fixtures/ulp_diff.py`,
//! docs/adr/9403 §6): the shape a drag
//! makes with each drawing tool, a box scaled by a handle, the knob's turn,
//! the measure's readout, the rulers' steps, a corner's tidy size, file
//! names and the starting grid, stroke widths and panel distance.

use kentos_geometry_core::api::json::{FromJson, Json, to_string};
use kentos_geometry_core::geometry::Bounds;
use kentos_svg_core::shape::{Obj, Pt};

use super::ToolId;
use super::draw_tool::{DragSpec, line_stroke_width, shape_from_drag, shape_stroke_width};
use super::files::file_slug;
use super::measure::readout;
use super::node_tool::nice_round;
use super::pointer::{knob_turn, scaled_box};
use super::rulers::nice_step;
use super::state::{default_grid, panel_unit};

fn fixture() -> Json {
    Json::parse(include_str!(
        "../../../../../../fixtures/style/v1/svgedit.json"
    ))
    .expect("the fixture reads")
}

fn num(v: &Json) -> f64 {
    f64::from_json(v).expect("a number")
}

fn pt(v: &Json) -> Pt {
    <[f64; 2]>::from_json(v).expect("a point")
}

fn bounds(v: &Json) -> Bounds {
    Bounds {
        min_x: num(v.get("minX")),
        min_y: num(v.get("minY")),
        max_x: num(v.get("maxX")),
        max_y: num(v.get("maxY")),
    }
}

fn items(v: &Json) -> &[Json] {
    match v {
        Json::Arr(a) => a,
        _ => panic!("a list"),
    }
}

fn text(v: &Json) -> &str {
    match v {
        Json::Str(s) => s,
        _ => panic!("a text"),
    }
}

#[test]
fn drags_draw_the_webs_shapes() {
    let data = fixture();
    // `KENTOS_WRITE_FROZEN=<folder>`: the shapes that differ go there as patches for
    // `scripts/fixtures/ulp_diff.py --apply`, which reads how far their numbers moved and refuses anything else (the
    // web's recorder is gone).
    let frozen = std::env::var_os("KENTOS_WRITE_FROZEN");
    let mut patches = Vec::new();
    for (i, case) in items(data.get("drags")).iter().enumerate() {
        let spec = case.get("spec");
        let tool = match text(spec.get("tool")) {
            "rect" => ToolId::Rect,
            "ellipse" => ToolId::Ellipse,
            "polygon" => ToolId::Polygon,
            _ => ToolId::Text,
        };
        let spec = DragSpec {
            tool,
            width: num(spec.get("width")),
            height: num(spec.get("height")),
            sides: num(spec.get("sides")),
            star: matches!(spec.get("star"), Json::Bool(true)),
            shift: matches!(spec.get("shift"), Json::Bool(true)),
            from_centre: matches!(spec.get("fromCentre"), Json::Bool(true)),
        };
        let got = shape_from_drag(&spec, pt(case.get("p0")), pt(case.get("p1")), "id");
        let want = match case.get("shape") {
            Json::Null => None,
            s => Some(Obj::from_json(s).expect("a shape")),
        };
        if let (Some(_), Some(shape)) = (&frozen, &got)
            && got.as_ref().map(to_string) != want.as_ref().map(to_string)
        {
            patches.push(format!(
                "{{\"at\":[\"drags\",{i},\"shape\"],\"value\":{}}}",
                to_string(shape)
            ));
            continue;
        }
        assert_eq!(
            got.as_ref().map(to_string),
            want.as_ref().map(to_string),
            "{spec:?}"
        );
    }
    if let Some(dir) = frozen {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).expect("the folder");
        std::fs::write(
            dir.join("svgedit-drags.patches.json"),
            format!("[{}]\n", patches.join(",\n")),
        )
        .expect("the patches");
    }
}

#[test]
fn handles_and_the_knob_scale_and_turn_as_the_web() {
    let data = fixture();
    for case in items(data.get("scaled")) {
        let got = scaled_box(
            &bounds(case.get("box")),
            num(case.get("handle")) as u8,
            pt(case.get("q")),
            matches!(case.get("shift"), Json::Bool(true)),
        );
        let want = match case.get("out") {
            Json::Null => None,
            b => Some(bounds(b)),
        };
        assert_eq!(got, want, "handle {}", num(case.get("handle")));
    }
    for case in items(data.get("turns")) {
        let got = knob_turn(
            &bounds(case.get("box")),
            pt(case.get("p0")),
            pt(case.get("p")),
            matches!(case.get("shift"), Json::Bool(true)),
        );
        assert_eq!(got.to_bits(), num(case.get("deg")).to_bits());
    }
}

#[test]
fn measures_steps_names_and_defaults_are_the_webs() {
    let data = fixture();
    for case in items(data.get("measures")) {
        let size = match case.get("sizeMm") {
            Json::Null => None,
            v => Some(num(v)),
        };
        let (main, more) = readout(
            pt(case.get("a")),
            pt(case.get("b")),
            num(case.get("width")),
            size,
        );
        assert_eq!(main, text(case.get("main")));
        assert_eq!(more, text(case.get("more")));
    }
    for case in items(data.get("niceStep")) {
        let c = items(case);
        assert_eq!(nice_step(num(&c[0])), num(&c[1]), "{}", num(&c[0]));
    }
    for case in items(data.get("niceRound")) {
        let c = items(case);
        assert_eq!(nice_round(num(&c[0]), num(&c[1])), num(&c[2]));
    }
    for case in items(data.get("slugs")) {
        let c = items(case);
        assert_eq!(file_slug(text(&c[0])), text(&c[1]));
    }
    for case in items(data.get("defaults")) {
        let w = num(case.get("width"));
        assert_eq!(default_grid(w), num(case.get("grid")));
        assert_eq!(panel_unit(w), num(case.get("unit")));
        assert_eq!(shape_stroke_width(w), num(case.get("shapeStroke")));
        assert_eq!(line_stroke_width(w), num(case.get("lineStroke")));
    }
}
