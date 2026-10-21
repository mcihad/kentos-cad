//! Bağlı görünümlerin bağı, ekran ölçeği ve görünüm başına zaman (docs/adr/0576 §1, §2) against
//! `fixtures/views/v1/links.json` (written by scripts/fixtures/view_link_cases.py from the ADR): the same IEEE
//! double operations, so the numbers are equal.

use kentos_geometry_core::time::{Step, Unit, Window};
use kentos_geometry_core::tools::view_link::{
    Frame, Link, follow, own_step, own_window, screen_scale,
};
use kentos_geometry_core::vec2::Vec2;
use serde_json::Value;

fn cases() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/views/v1/links.json"
    );
    let v: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON");
    assert_eq!(v["format"], "kentos.view-link-cases");
    v
}

fn num(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn frame(v: &Value) -> Frame {
    Frame {
        centre: Vec2::new(num(&v["centre"][0]), num(&v["centre"][1])),
        scale: num(&v["scale"]),
        twist: num(&v["twist"]),
    }
}

fn step(v: &Value) -> Step {
    Step::new(
        v["n"].as_i64().expect("n"),
        Unit::from_name(v["unit"].as_str().expect("unit")).expect("a unit"),
    )
    .expect("a step")
}

#[test]
fn the_other_view_follows_as_the_link_says() {
    let file = cases();
    let list = file["follow"].as_array().expect("cases");
    assert!(list.len() >= 9);
    for c in list {
        let link = Link::from_key(c["link"].as_str().expect("link")).expect("a link");
        let got = follow(link, frame(&c["active"]), frame(&c["other"]));
        assert_eq!(got, frame(&c["expect"]), "{}", c["link"]);
    }
}

#[test]
fn the_screen_scale_is_one_over_the_scale_in_metres_per_pixel() {
    let file = cases();
    for c in file["scale"].as_array().expect("cases") {
        let got = screen_scale(num(&c["scale"]));
        assert_eq!(got, num(&c["expect"]), "{}", c["scale"]);
        if let Some(n) = c.get("n") {
            assert!((got - num(n)).abs() < 1e-9 * num(n), "1:{n}");
        }
    }
}

#[test]
fn a_view_of_its_own_time_stands_at_the_nearest_position_of_the_grid() {
    let file = cases();
    let list = file["window"].as_array().expect("cases");
    assert!(list.len() >= 25);
    for c in list {
        let got = own_window(
            num(&c["anchor"]),
            step(&c["step"]),
            c["last"].as_i64().expect("last"),
            c["ranged"].as_bool().expect("ranged"),
            num(&c["moment"]),
        );
        let want: Vec<f64> = c["expect"]["window"]
            .as_array()
            .expect("a window")
            .iter()
            .map(num)
            .collect();
        match (got, want.as_slice()) {
            (Window::Instant(a), [x]) => assert_eq!(a, *x, "{c}"),
            (Window::Range(a, b), [x, y]) => assert_eq!((a, b), (*x, *y), "{c}"),
            _ => panic!("window of another shape: {c}"),
        }
    }
}

#[test]
fn the_arrows_move_a_step_within_the_grid() {
    let file = cases();
    for c in file["walk"].as_array().expect("cases") {
        let got = own_step(
            num(&c["anchor"]),
            step(&c["step"]),
            c["last"].as_i64().expect("last"),
            num(&c["moment"]),
            c["by"].as_i64().expect("by"),
        );
        assert_eq!(got, num(&c["expect"]), "{c}");
    }
}
