//! The thematic renderers' rules (docs/adr/0213 §2) held to their
//! independent reference (fixtures/renderers/v1/cases.json, written by
//! scripts/fixtures/renderer_cases.py from the ADR): ramps, shares, sizes,
//! classes, dots, charts, groups, displacements, heat maps and Ters alan's
//! region.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geometry::Bounds;
use kentos_style_core::style::charts::{bars, pie};
use kentos_style_core::style::dots::{Inside, count_of, dots, rings_hash, seed_of};
use kentos_style_core::style::groups::{displaced, group};
use kentos_style_core::style::heat::{Grid, colors, largest, table, values};
use kentos_style_core::style::inverted::{Rule, region};
use kentos_style_core::style::model::{ChartKind, Placement};
use kentos_style_core::style::thematic::{
    class_of, hex, parse_rgba, ramp_color, share, size_at, step,
};
use serde_json::Value as Json;

fn fixture() -> Json {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/renderers/v1/cases.json"
    );
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn list<'a>(v: &'a Json, key: &str) -> &'a Vec<Json> {
    v[key].as_array().unwrap_or_else(|| panic!("{key}"))
}

fn n(v: &Json) -> f64 {
    v.as_f64().unwrap_or_else(|| panic!("sayı değil: {v}"))
}

fn pt(v: &Json) -> Vec2 {
    Vec2::new(n(&v[0]), n(&v[1]))
}

fn ring(v: &Json) -> Vec<Vec2> {
    v.as_array().into_iter().flatten().map(pt).collect()
}

fn ramp(v: &Json) -> Vec<[u8; 4]> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| c.as_str().and_then(parse_rgba))
        .collect()
}

fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * (1.0 + b.abs())
}

#[test]
fn ramps_shares_sizes_and_classes() {
    let f = fixture();
    for (i, c) in list(&f, "ramps").iter().enumerate() {
        let got = hex(ramp_color(&ramp(&c["ramp"]), n(&c["t"])));
        assert_eq!(got, c["color"].as_str().unwrap_or_default(), "rampa {i}");
    }
    for (i, c) in list(&f, "unclassed").iter().enumerate() {
        let k = step(share(n(&c["value"]), n(&c["min"]), n(&c["max"])));
        assert_eq!(
            u64::from(k),
            c["step"].as_u64().unwrap_or(999),
            "sürekli {i}"
        );
        let got = hex(ramp_color(&ramp(&c["ramp"]), f64::from(k) / 255.0));
        assert_eq!(got, c["color"].as_str().unwrap_or_default(), "sürekli {i}");
    }
    for (i, c) in list(&f, "sizes").iter().enumerate() {
        let e = match c["scaling"].as_str() {
            Some("radius") => 1.0,
            Some("flannery") => 0.57,
            _ => 0.5,
        };
        let k = step(share(n(&c["value"]), n(&c["minValue"]), n(&c["maxValue"])));
        assert_eq!(u64::from(k), c["step"].as_u64().unwrap_or(999), "boy {i}");
        let got = size_at(f64::from(k) / 255.0, n(&c["minSize"]), n(&c["maxSize"]), e);
        assert!(near(got, n(&c["size"]), 1e-12), "boy {i}: {got}");
    }
    for (i, c) in list(&f, "classes").iter().enumerate() {
        let breaks: Vec<f64> = list(c, "breaks").iter().map(n).collect();
        assert_eq!(
            class_of(n(&c["value"]), &breaks) as u64,
            c["class"].as_u64().unwrap_or(999),
            "sınıf {i}"
        );
    }
}

#[test]
fn dots_bit_for_bit() {
    let f = fixture();
    for (i, c) in list(&f, "dots").iter().enumerate() {
        let parts: Vec<Vec<Vec<Vec2>>> = list(c, "parts")
            .iter()
            .map(|rings| rings.as_array().into_iter().flatten().map(ring).collect())
            .collect();
        let slices: Vec<&[Vec<Vec2>]> = parts.iter().map(Vec::as_slice).collect();
        let count = count_of(Some(n(&c["value"])), n(&c["dotValue"]));
        assert_eq!(
            count as u64,
            c["count"].as_u64().unwrap_or(999),
            "noktalar {i}"
        );
        let mut got = Vec::new();
        if count > 0 {
            let inside = Inside::new(&slices).unwrap_or_else(|| panic!("noktalar {i}: alan yok"));
            let seed = c["seed"].as_u64().unwrap_or(0);
            let field = c["field"].as_u64().unwrap_or(0) as usize;
            dots(
                &inside,
                count,
                seed_of(seed, rings_hash(&slices), field),
                &mut got,
            );
        }
        let want: Vec<Vec2> = list(c, "dots").iter().map(pt).collect();
        assert_eq!(got.len(), want.len(), "noktalar {i}");
        for (k, (g, w)) in got.iter().zip(&want).enumerate() {
            assert!(
                g.x.to_bits() == w.x.to_bits() && g.y.to_bits() == w.y.to_bits(),
                "noktalar {i}, {k}: {g:?} ≠ {w:?}"
            );
        }
    }
}

#[test]
fn charts_to_1e9() {
    let f = fixture();
    for (i, c) in list(&f, "charts").iter().enumerate() {
        let values: Vec<f64> = list(c, "values").iter().map(n).collect();
        let place = pt(&c["place"]);
        let got = match c["kind"].as_str() {
            Some("pie") => pie(place, n(&c["size"]), &values),
            Some("bar") => bars(
                ChartKind::Bar,
                place,
                n(&c["size"]),
                n(&c["width"]),
                n(&c["maxValue"]),
                &values,
            ),
            _ => bars(
                ChartKind::Stacked,
                place,
                n(&c["size"]),
                n(&c["width"]),
                n(&c["maxValue"]),
                &values,
            ),
        };
        let want = list(c, "pieces");
        assert_eq!(got.len(), want.len(), "grafik {i}");
        for (g, w) in got.iter().zip(want) {
            assert_eq!(
                g.field as u64,
                w["field"].as_u64().unwrap_or(999),
                "grafik {i}"
            );
            let wr = ring(&w["ring"]);
            assert_eq!(g.ring.len(), wr.len(), "grafik {i}");
            for (a, b) in g.ring.iter().zip(&wr) {
                assert!(
                    near(a.x, b.x, 1e-9) && near(a.y, b.y, 1e-9),
                    "grafik {i}: {a:?} ≠ {b:?}"
                );
            }
        }
    }
}

#[test]
fn groups_bit_for_bit_and_displacements() {
    let f = fixture();
    for (i, c) in list(&f, "groups").iter().enumerate() {
        let pts: Vec<Vec2> = list(c, "points").iter().map(pt).collect();
        let got = group(&pts, n(&c["distance"]));
        let want = list(c, "groups");
        assert_eq!(got.len(), want.len(), "gruplar {i}");
        for (g, w) in got.iter().zip(want) {
            let members: Vec<u64> = list(w, "members").iter().filter_map(Json::as_u64).collect();
            assert_eq!(
                g.members.iter().map(|&m| m as u64).collect::<Vec<_>>(),
                members,
                "gruplar {i}"
            );
            let (gc, wc) = (g.centre(), pt(&w["centre"]));
            assert!(
                gc.x.to_bits() == wc.x.to_bits() && gc.y.to_bits() == wc.y.to_bits(),
                "gruplar {i}: {gc:?} ≠ {wc:?}"
            );
        }
    }
    for (i, c) in list(&f, "displaced").iter().enumerate() {
        let placement = match c["placement"].as_str() {
            Some("rings") => Placement::Rings,
            Some("grid") => Placement::Grid,
            _ => Placement::Ring,
        };
        let (offsets, rings) = displaced(
            placement,
            c["n"].as_u64().unwrap_or(0) as usize,
            n(&c["s"]),
            n(&c["c"]),
            n(&c["spacing"]),
        );
        let want: Vec<Vec2> = list(c, "offsets").iter().map(pt).collect();
        assert_eq!(offsets.len(), want.len(), "yayma {i}");
        for (a, b) in offsets.iter().zip(&want) {
            assert!(
                near(a.x, b.x, 1e-9) && near(a.y, b.y, 1e-9),
                "yayma {i}: {a:?} ≠ {b:?}"
            );
        }
        let want_rings: Vec<f64> = list(c, "rings").iter().map(n).collect();
        assert_eq!(rings.len(), want_rings.len(), "yayma {i}");
        for (a, b) in rings.iter().zip(&want_rings) {
            assert!(near(*a, *b, 1e-12), "yayma {i}");
        }
    }
}

#[test]
fn heat_maps() {
    let f = fixture();
    for (i, c) in list(&f, "heat").iter().enumerate() {
        let b = &c["box"];
        let bounds = Bounds {
            min_x: n(&b[0]),
            min_y: n(&b[1]),
            max_x: n(&b[2]),
            max_y: n(&b[3]),
        };
        let quality = c["quality"].as_u64().unwrap_or(1) as u32;
        let grid = Grid::over(&bounds, n(&c["pxPerM"]), quality)
            .unwrap_or_else(|| panic!("ısı {i}: ızgara"));
        assert_eq!(
            grid.width as u64,
            c["grid"]["width"].as_u64().unwrap_or(0),
            "ısı {i}"
        );
        assert_eq!(
            grid.height as u64,
            c["grid"]["height"].as_u64().unwrap_or(0),
            "ısı {i}"
        );
        assert_eq!(
            grid.cell.to_bits(),
            n(&c["grid"]["cell"]).to_bits(),
            "ısı {i}"
        );
        let pts: Vec<(Vec2, f64)> = list(c, "points")
            .iter()
            .map(|p| (Vec2::new(n(&p[0]), n(&p[1])), n(&p[2])))
            .collect();
        let got = values(&grid, &pts, c["radius"].as_i64().unwrap_or(1));
        let want: Vec<f64> = list(c, "values").iter().map(n).collect();
        assert_eq!(got.len(), want.len(), "ısı {i}");
        for (k, (a, w)) in got.iter().zip(&want).enumerate() {
            assert_eq!(a.to_bits(), w.to_bits(), "ısı {i}, hücre {k}: {a} ≠ {w}");
        }
        let top = c["max"].as_f64().unwrap_or_else(|| largest(&got));
        assert_eq!(top.to_bits(), n(&c["top"]).to_bits(), "ısı {i}");
        let px = colors(&got, top, &table(&ramp(&c["ramp"]), n(&c["opacity"])));
        let want: Vec<u64> = list(c, "pixels").iter().filter_map(Json::as_u64).collect();
        assert_eq!(px.len(), want.len(), "ısı {i}");
        for (k, (a, w)) in px.iter().zip(&want).enumerate() {
            assert!(
                u64::from(*a).abs_diff(*w) <= 1,
                "ısı {i}, bayt {k}: {a} ≠ {w}"
            );
        }
    }
}

/// Whether `p` is in the region: inside the box and outside the areas by the rule.
fn in_region(p: Vec2, areas: &[Vec<Vec<Vec2>>], rule: Rule) -> bool {
    let (mut odd, mut winding) = (false, 0i32);
    for rings in areas {
        let mut covers = false;
        for r in rings {
            let n = r.len();
            for k in 0..n {
                let (a, b) = (r[k], r[(k + 1) % n]);
                if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x
                {
                    odd = !odd;
                    covers = !covers;
                }
            }
        }
        winding += i32::from(covers);
    }
    match rule {
        Rule::EvenOdd => !odd,
        Rule::NonZero => winding == 0,
    }
}

#[test]
fn inverted_region_adds_up_and_stays_outside() {
    let f = fixture();
    for (i, c) in list(&f, "inverted").iter().enumerate() {
        let b = &c["box"];
        let bounds = Bounds {
            min_x: n(&b[0]),
            min_y: n(&b[1]),
            max_x: n(&b[2]),
            max_y: n(&b[3]),
        };
        let areas: Vec<Vec<Vec<Vec2>>> = list(c, "areas")
            .iter()
            .map(|a| a.as_array().into_iter().flatten().map(ring).collect())
            .collect();
        let slices: Vec<&[Vec<Vec2>]> = areas.iter().map(Vec::as_slice).collect();
        let rule = if c["rule"].as_str() == Some("nonZero") {
            Rule::NonZero
        } else {
            Rule::EvenOdd
        };
        let traps = region(&bounds, &slices, rule);
        let mut total = 0.0;
        for t in &traps {
            let a = 0.5 * ((t[1].x - t[0].x) + (t[2].x - t[3].x)) * (t[3].y - t[0].y);
            assert!(a >= -1e-9, "ters {i}: eksi alanlı yamuk {t:?}");
            total += a;
            for q in t {
                assert!(
                    q.x >= bounds.min_x - 1e-9
                        && q.x <= bounds.max_x + 1e-9
                        && q.y >= bounds.min_y - 1e-9
                        && q.y <= bounds.max_y + 1e-9,
                    "ters {i}: kutunun dışında {q:?}"
                );
            }
            // Its middle (between the two sides at half height) is in the region (a sliver's is on its edge).
            if a > 1e-6 {
                let y = (t[0].y + t[3].y) / 2.0;
                let (l, r) = ((t[0].x + t[3].x) / 2.0, (t[1].x + t[2].x) / 2.0);
                let m = Vec2::new((l + r) / 2.0, y);
                assert!(in_region(m, &areas, rule), "ters {i}: {m:?} bölgede değil");
            }
        }
        let want = n(&c["area"]);
        assert!(
            near(total, want, 1e-9),
            "ters {i}: {total} ≠ {want} ({} yamuk)",
            traps.len()
        );
    }
}
