//! The PDF reader against the shared cases (fixtures/pdf/v1, docs/adr/0351 §2): files written and their pages
//! expected by scripts/fixtures/pdf_cases.py from its own drawing model, without reading the bytes back.

use kentos_formats::pdf::{PdfError, PdfPage, read_page};
use serde_json::Value;

const WITHIN: f64 = 1e-9;

fn dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/pdf/v1")
}

fn near(a: f64, b: &Value) -> bool {
    b.as_f64().is_some_and(|b| (a - b).abs() <= WITHIN)
}

fn point(p: [f64; 2], want: &Value) -> bool {
    near(p[0], &want[0]) && near(p[1], &want[1])
}

/// The distance of `p` from the cubic `c` (sampled finely, then refined).
fn off_curve(p: [f64; 2], c: &[[f64; 2]; 4]) -> f64 {
    let at = |t: f64| {
        let u = 1.0 - t;
        let (a, b, cc, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        [a * c[0][0] + b * c[1][0] + cc * c[2][0] + d * c[3][0], a * c[0][1] + b * c[1][1] + cc * c[2][1] + d * c[3][1]]
    };
    let dist = |t: f64| {
        let q = at(t);
        libm::hypot(q[0] - p[0], q[1] - p[1])
    };
    let mut best = (0.0, f64::INFINITY);
    for i in 0..=4000 {
        let t = f64::from(i) / 4000.0;
        let d = dist(t);
        if d < best.1 {
            best = (t, d);
        }
    }
    let (mut lo, mut hi) = ((best.0 - 1.0 / 4000.0).max(0.0), (best.0 + 1.0 / 4000.0).min(1.0));
    for _ in 0..60 {
        let (a, b) = (lo + (hi - lo) / 3.0, hi - (hi - lo) / 3.0);
        if dist(a) < dist(b) {
            hi = b;
        } else {
            lo = a;
        }
    }
    dist(lo).min(best.1)
}

fn controls(v: &Value) -> [[f64; 2]; 4] {
    let p = |i: usize| [v[i][0].as_f64().unwrap_or(f64::NAN), v[i][1].as_f64().unwrap_or(f64::NAN)];
    [p(0), p(1), p(2), p(3)]
}

/// A part's points against the expected: a point each, a curve the points up to its end, each near it.
fn part_matches(got: &[[f64; 2]], want: &[Value]) -> Result<(), String> {
    let mut i = 0;
    for w in want {
        if let Some(c) = w.get("curve") {
            let c = controls(c);
            loop {
                let Some(&p) = got.get(i) else {
                    return Err("eğri bitmeden noktalar bitti".into());
                };
                i += 1;
                let d = off_curve(p, &c);
                if d > 0.01 + WITHIN {
                    return Err(format!("{p:?} eğriden {d} pt uzak"));
                }
                if (p[0] - c[3][0]).abs() <= WITHIN && (p[1] - c[3][1]).abs() <= WITHIN {
                    break;
                }
            }
        } else {
            let Some(&p) = got.get(i) else {
                return Err(format!("{w} beklenirken noktalar bitti"));
            };
            if !point(p, w) {
                return Err(format!("{i}. nokta {p:?}, beklenen {w}"));
            }
            i += 1;
        }
    }
    if i != got.len() {
        return Err(format!("{} nokta fazla", got.len() - i));
    }
    Ok(())
}

fn page_matches(page: &PdfPage, want: &Value) -> Result<(), String> {
    for (k, v) in [("number", f64::from(page.number)), ("count", f64::from(page.count)), ("width", page.width), ("height", page.height)] {
        if !near(v, &want[k]) {
            return Err(format!("{k}: {v}, beklenen {}", want[k]));
        }
    }
    if serde_json::json!(page.layers) != want["layers"] || serde_json::json!(page.hidden) != want["hidden"] {
        return Err(format!("katmanlar {:?} {:?}, beklenen {} {}", page.layers, page.hidden, want["layers"], want["hidden"]));
    }
    let left = serde_json::json!({ "images": page.left.images, "shadings": page.left.shadings,
        "unreadable": page.left.unreadable, "truncated": page.left.truncated });
    if left != want["left"] {
        return Err(format!("alınmayanlar {left}, beklenen {}", want["left"]));
    }
    let paths = want["paths"].as_array().cloned().unwrap_or_default();
    if paths.len() != page.paths.len() {
        return Err(format!("{} yol, beklenen {}", page.paths.len(), paths.len()));
    }
    for (k, (got, w)) in page.paths.iter().zip(&paths).enumerate() {
        let at = |m: String| format!("{k}. yol: {m}");
        let stroke = got.stroke.as_ref().map(|s| serde_json::json!({ "color": s.color, "width": s.width, "dashed": s.dashed }));
        match (&stroke, w.get("stroke").filter(|s| !s.is_null())) {
            (None, None) => {}
            (Some(s), Some(e)) if s["color"] == e["color"] && s["dashed"] == e["dashed"] && near(got.stroke.as_ref().map_or(0.0, |s| s.width), &e["width"]) => {}
            _ => return Err(at(format!("çizgi {stroke:?}, beklenen {}", w["stroke"]))),
        }
        if serde_json::json!(got.fill) != w["fill"] || serde_json::json!(got.layer) != w["layer"] {
            return Err(at(format!("dolgu {:?} katman {:?}, beklenen {} {}", got.fill, got.layer, w["fill"], w["layer"])));
        }
        let parts = w["parts"].as_array().cloned().unwrap_or_default();
        if parts.len() != got.parts.len() {
            return Err(at(format!("{} parça, beklenen {}", got.parts.len(), parts.len())));
        }
        for (j, ((pts, closed), e)) in got.parts.iter().zip(&parts).enumerate() {
            if serde_json::json!(closed) != e["closed"] {
                return Err(at(format!("{j}. parçanın kapalılığı")));
            }
            part_matches(pts, e["points"].as_array().map_or(&[][..], Vec::as_slice))
                .map_err(|m| at(format!("{j}. parça: {m}")))?;
        }
    }
    let texts = want["texts"].as_array().cloned().unwrap_or_default();
    if texts.len() != page.texts.len() {
        return Err(format!("{} yazı, beklenen {}: {:?}", page.texts.len(), texts.len(), page.texts));
    }
    for (got, w) in page.texts.iter().zip(&texts) {
        if !(point(got.p, &w["p"]) && near(got.height, &w["height"]) && near(got.angle, &w["angle"])
            && serde_json::json!(got.text) == w["text"] && serde_json::json!(got.color) == w["color"]
            && serde_json::json!(got.layer) == w["layer"])
        {
            return Err(format!("yazı {got:?}, beklenen {w}"));
        }
    }
    Ok(())
}

#[test]
fn the_pdf_cases_read_as_written() {
    let text = std::fs::read_to_string(dir().join("cases.json")).expect("cases.json");
    let cases: Value = serde_json::from_str(&text).expect("JSON");
    let mut problems = Vec::new();
    let mut pages = 0;
    for case in cases["cases"].as_array().expect("cases") {
        let file = case["file"].as_str().unwrap_or("?");
        let bytes = std::fs::read(dir().join(file)).expect("file");
        for (n, want) in case["pages"].as_object().into_iter().flatten() {
            pages += 1;
            let n: u32 = n.parse().unwrap_or(0);
            match read_page(&bytes, n) {
                Ok(page) => {
                    if let Err(m) = page_matches(&page, want) {
                        problems.push(format!("{file} {n}: {m}"));
                    }
                }
                Err(e) => problems.push(format!("{file} {n}: {e}")),
            }
        }
        for (n, want) in case["errors"].as_object().into_iter().flatten() {
            let n: u32 = n.parse().unwrap_or(0);
            let got = read_page(&bytes, n);
            let ok = match (&got, want) {
                (Err(PdfError::NoPage { count }), w) if w.get("noPage").and_then(Value::as_u64) == Some(u64::from(*count)) => true,
                (Err(PdfError::Encrypted), w) if w.get("encrypted").is_some() => true,
                (Err(PdfError::NotPdf(_)), w) if w.get("notPdf").is_some() => true,
                _ => false,
            };
            if !ok {
                problems.push(format!("{file} {n}: {got:?}, beklenen {want}"));
            }
        }
    }
    assert!(pages >= 10, "{pages} sayfa");
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
