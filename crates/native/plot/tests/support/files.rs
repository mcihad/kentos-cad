//! What an acceptance drawing's DXF and PDF hold, read without KentOS code:
//! the DXF's group codes, the PDF's page sizes and its stroked straight
//! segments (Flate streams inflated, `cm` followed). The Rust twin of
//! python/tests/acceptance_files.py; the facts they are held to are worked
//! out by scripts/fixtures/acceptance_room.py (fixtures/acceptance/v1).
//! Shared by the plotter's, the MCP server's and the desktop's acceptance
//! tests (`#[path]`).

#![allow(dead_code)]

use serde_json::Value;

/// PDF points per millimetre.
const MM: f64 = 72.0 / 25.4;

// ── DXF ──────────────────────────────────────────────────────────────────

type Entity = Vec<(i32, String)>;

fn dxf_entities(data: &[u8]) -> (Vec<(String, String)>, Vec<Entity>) {
    let text = String::from_utf8_lossy(data).replace("\r\n", "\n");
    let lines: Vec<&str> = text.split('\n').collect();
    let pairs: Vec<(i32, String)> = lines
        .chunks(2)
        .filter(|c| c.len() == 2)
        .filter_map(|c| Some((c[0].trim().parse().ok()?, c[1].trim().to_owned())))
        .collect();
    let mut header = Vec::new();
    for w in pairs.windows(2) {
        if w[0].0 == 9 {
            header.push((w[0].1.clone(), w[1].1.clone()));
        }
    }
    let start = pairs
        .iter()
        .position(|(c, v)| *c == 2 && v == "ENTITIES")
        .expect("an ENTITIES section");
    let mut ents: Vec<Entity> = Vec::new();
    for (c, v) in &pairs[start + 1..] {
        if *c == 0 && v == "ENDSEC" {
            break;
        }
        if *c == 0 {
            ents.push(vec![(*c, v.clone())]);
        } else if let Some(e) = ents.last_mut() {
            e.push((*c, v.clone()));
        }
    }
    (header, ents)
}

fn values(e: &Entity, code: i32) -> Vec<f64> {
    e.iter()
        .filter(|(c, _)| *c == code)
        .filter_map(|(_, v)| v.parse().ok())
        .collect()
}

fn points(e: &Entity) -> Vec<[f64; 2]> {
    values(e, 10)
        .into_iter()
        .zip(values(e, 20))
        .map(|(x, y)| [x, y])
        .collect()
}

fn pt(v: &Value) -> [f64; 2] {
    [
        v[0].as_f64().expect("a number"),
        v[1].as_f64().expect("a number"),
    ]
}

/// Where the DXF differs from the facts: units, counts by type, the closed
/// polylines' vertices, the lines' ends in order, the dimensions'
/// measurements, the hatches' pattern and boundary.
pub fn dxf_problems(data: &[u8], want: &Value) -> Vec<String> {
    let tol = want["tolerance"].as_f64().expect("a tolerance");
    let close = |a: f64, b: f64| (a - b).abs() <= tol;
    let same = |ps: &[[f64; 2]], qs: &[[f64; 2]]| {
        ps.len() == qs.len()
            && ps
                .iter()
                .zip(qs)
                .all(|(p, q)| close(p[0], q[0]) && close(p[1], q[1]))
    };
    let mut out = Vec::new();
    let (header, ents) = dxf_entities(data);
    let units = header
        .iter()
        .find(|(k, _)| k == "$INSUNITS")
        .and_then(|(_, v)| v.parse::<i64>().ok());
    if units != want["insunits"].as_i64() {
        out.push(format!(
            "$INSUNITS {units:?}, beklenen {}",
            want["insunits"]
        ));
    }
    let mut counts = std::collections::BTreeMap::<String, i64>::new();
    for e in &ents {
        *counts.entry(e[0].1.clone()).or_default() += 1;
    }
    let wanted: std::collections::BTreeMap<String, i64> = want["counts"]
        .as_object()
        .expect("counts")
        .iter()
        .map(|(k, v)| (k.clone(), v.as_i64().expect("a count")))
        .collect();
    if counts != wanted {
        out.push(format!("nesneler {counts:?}, beklenen {wanted:?}"));
    }
    let of = |kind: &str| -> Vec<&Entity> { ents.iter().filter(|e| e[0].1 == kind).collect() };
    let list = |key: &str| want[key].as_array().cloned().unwrap_or_default();
    for (e, w) in of("LWPOLYLINE").into_iter().zip(list("polylines")) {
        let flags = values(e, 70).first().copied().unwrap_or(0.0) as i64;
        let pts: Vec<[f64; 2]> = w["points"]
            .as_array()
            .expect("points")
            .iter()
            .map(pt)
            .collect();
        if (flags & 1 == 1) != w["closed"].as_bool().unwrap_or(false) || !same(&points(e), &pts) {
            out.push(format!("LWPOLYLINE {:?}, beklenen {pts:?}", points(e)));
        }
    }
    for (i, (e, w)) in of("LINE").into_iter().zip(list("lines")).enumerate() {
        let got = [
            [values(e, 10)[0], values(e, 20)[0]],
            [values(e, 11)[0], values(e, 21)[0]],
        ];
        let w = [pt(&w[0]), pt(&w[1])];
        if !same(&got, &w) {
            out.push(format!("LINE {i}: {got:?}, beklenen {w:?}"));
        }
    }
    for (i, (e, w)) in of("DIMENSION")
        .into_iter()
        .zip(list("dimensions"))
        .enumerate()
    {
        let m = values(e, 42).first().copied().unwrap_or(f64::NAN);
        if !close(m, w.as_f64().expect("a measure")) {
            out.push(format!("DIMENSION {i}: ölçü {m}, beklenen {w}"));
        }
    }
    for (i, (e, w)) in of("HATCH").into_iter().zip(list("hatches")).enumerate() {
        let angle = values(e, 52).first().copied().unwrap_or(f64::NAN);
        let spacing = values(e, 41).first().copied().unwrap_or(f64::NAN);
        let want_b: Vec<[f64; 2]> = w["boundary"]
            .as_array()
            .expect("boundary")
            .iter()
            .map(pt)
            .collect();
        // The boundary's vertices follow the elevation point (the first 10/20).
        let got: Vec<[f64; 2]> = points(e).into_iter().skip(1).take(want_b.len()).collect();
        if !(close(angle, w["angle"].as_f64().expect("an angle"))
            && close(spacing, w["spacing"].as_f64().expect("a spacing"))
            && same(&got, &want_b))
        {
            out.push(format!(
                "HATCH {i}: açı {angle}, aralık {spacing}, sınır {got:?}"
            ));
        }
    }
    out
}

// ── PDF ──────────────────────────────────────────────────────────────────

/// Every object by number: its dictionary bytes and its stream, inflated where FlateDecode.
fn objects(data: &[u8]) -> std::collections::BTreeMap<u32, (Vec<u8>, Option<Vec<u8>>)> {
    let mut out = std::collections::BTreeMap::new();
    let find = |hay: &[u8], needle: &[u8], from: usize| {
        hay[from..]
            .windows(needle.len())
            .position(|w| w == needle)
            .map(|p| p + from)
    };
    let mut at = 0;
    while let Some(k) = find(data, b" 0 obj", at) {
        // The object's number: the digits before " 0 obj".
        let mut s = k;
        while s > 0 && data[s - 1].is_ascii_digit() {
            s -= 1;
        }
        let Ok(n) = std::str::from_utf8(&data[s..k])
            .unwrap_or("")
            .parse::<u32>()
        else {
            at = k + 6;
            continue;
        };
        let body_start = k + 6;
        let Some(end) = find(data, b"endobj", body_start) else {
            break;
        };
        let body = &data[body_start..end];
        match find(body, b"stream", 0).filter(|&p| !body[..p].ends_with(b"end")) {
            None => {
                out.insert(n, (body.to_vec(), None));
            }
            Some(p) => {
                let head = body[..p].to_vec();
                let mut raw = &body[p + 6..];
                if raw.starts_with(b"\r\n") {
                    raw = &raw[2..];
                } else if raw.starts_with(b"\n") {
                    raw = &raw[1..];
                }
                let stop = raw
                    .windows(9)
                    .rposition(|w| w == b"endstream")
                    .unwrap_or(raw.len());
                let mut raw = &raw[..stop];
                while raw.last().is_some_and(|c| *c == b'\n' || *c == b'\r') {
                    raw = &raw[..raw.len() - 1];
                }
                let flate = head.windows(12).any(|w| w == b"/FlateDecode");
                let bytes = if flate {
                    miniz_oxide::inflate::decompress_to_vec_zlib(raw).expect("a Flate stream")
                } else {
                    raw.to_vec()
                };
                out.insert(n, (head, Some(bytes)));
            }
        }
        at = end + 6;
    }
    out
}

/// The references `N 0 R` in `text`, in order.
fn refs(text: &str) -> Vec<u32> {
    let words: Vec<&str> = text.split_whitespace().collect();
    words
        .windows(3)
        .filter(|w| w[1] == "0" && w[2].starts_with('R'))
        .filter_map(|w| w[0].trim_start_matches('[').parse().ok())
        .collect()
}

/// The text between `key` and the next `]` (an array's items).
fn array_after<'a>(head: &'a str, key: &str) -> Option<&'a str> {
    let k = head.find(key)? + key.len();
    let open = head[k..].find('[')? + k + 1;
    let close = head[open..].find(']')? + open;
    Some(&head[open..close])
}

/// The stroked straight segments of a content stream, in page points, through `cm`.
fn segments(content: &[u8]) -> Vec<[f64; 4]> {
    let mut out = Vec::new();
    let mut stack: Vec<f64> = Vec::new();
    let mut ctm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut saved: Vec<[f64; 6]> = Vec::new();
    let mut path: Vec<Vec<(f64, f64)>> = Vec::new();
    let apply =
        |m: &[f64; 6], x: f64, y: f64| (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]);
    let mut i = 0;
    let n = content.len();
    while i < n {
        let c = content[i];
        if c == b'(' {
            // A literal string: skipped with its nesting and escapes.
            let mut depth = 1;
            i += 1;
            while i < n && depth > 0 {
                match content[i] {
                    b'\\' => i += 1,
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
                i += 1;
            }
            stack.clear();
            continue;
        }
        if c.is_ascii_whitespace() || c == b'[' || c == b']' {
            i += 1;
            continue;
        }
        if c == b'<' {
            // A dictionary or a hex string: skipped to its close.
            let close: &[u8] = if content.get(i + 1) == Some(&b'<') {
                b">>"
            } else {
                b">"
            };
            let rest = &content[i..];
            let at = rest
                .windows(close.len())
                .position(|w| w == close)
                .unwrap_or(rest.len());
            i += at + close.len();
            continue;
        }
        let start = i;
        while i < n && !content[i].is_ascii_whitespace() && !b"[]()<>/".contains(&content[i]) {
            i += 1;
        }
        if c == b'/' {
            i = start + 1;
            while i < n && !content[i].is_ascii_whitespace() && !b"[]()<>/".contains(&content[i]) {
                i += 1;
            }
            continue;
        }
        let token = std::str::from_utf8(&content[start..i]).unwrap_or("");
        if let Ok(v) = token.parse::<f64>() {
            stack.push(v);
            continue;
        }
        let k = stack.len();
        match token {
            "cm" if k >= 6 => {
                let [a, b, c2, d, e, f] = [
                    stack[k - 6],
                    stack[k - 5],
                    stack[k - 4],
                    stack[k - 3],
                    stack[k - 2],
                    stack[k - 1],
                ];
                let m = ctm;
                ctm = [
                    a * m[0] + b * m[2],
                    a * m[1] + b * m[3],
                    c2 * m[0] + d * m[2],
                    c2 * m[1] + d * m[3],
                    e * m[0] + f * m[2] + m[4],
                    e * m[1] + f * m[3] + m[5],
                ];
            }
            "q" => saved.push(ctm),
            "Q" => {
                if let Some(m) = saved.pop() {
                    ctm = m;
                }
            }
            "m" if k >= 2 => path.push(vec![apply(&ctm, stack[k - 2], stack[k - 1])]),
            "l" if k >= 2 => {
                let p = apply(&ctm, stack[k - 2], stack[k - 1]);
                if let Some(sub) = path.last_mut() {
                    sub.push(p);
                }
            }
            "h" => {
                if let Some(sub) = path.last_mut()
                    && let Some(&first) = sub.first()
                {
                    sub.push(first);
                }
            }
            "re" if k >= 4 => {
                let (x, y, w, h) = (stack[k - 4], stack[k - 3], stack[k - 2], stack[k - 1]);
                path.push(vec![
                    apply(&ctm, x, y),
                    apply(&ctm, x + w, y),
                    apply(&ctm, x + w, y + h),
                    apply(&ctm, x, y + h),
                    apply(&ctm, x, y),
                ]);
            }
            "S" | "s" | "B" | "B*" | "b" | "b*" => {
                for sub in path.drain(..) {
                    for w in sub.windows(2) {
                        out.push([w[0].0, w[0].1, w[1].0, w[1].1]);
                    }
                }
            }
            "f" | "F" | "f*" | "n" => path.clear(),
            _ => {}
        }
        stack.clear();
    }
    out
}

/// A page of a PDF: its size and its stroked segments, millimetres from the bottom left.
pub struct Page {
    pub width: f64,
    pub height: f64,
    pub segments: Vec<[f64; 4]>,
}

/// Each page in order.
pub fn pdf_pages(data: &[u8]) -> Vec<Page> {
    let objs = objects(data);
    let text = |n: &u32| String::from_utf8_lossy(&objs[n].0).into_owned();
    let root = objs
        .keys()
        .find(|n| {
            let t = text(n);
            t.contains("/Type /Pages") || t.contains("/Type/Pages")
        })
        .copied()
        .expect("a page tree");
    let kids = refs(array_after(&text(&root), "/Kids").expect("its kids"));
    kids.iter()
        .map(|kid| {
            let head = text(kid);
            let b: Vec<f64> = array_after(&head, "/MediaBox")
                .expect("a media box")
                .split_whitespace()
                .filter_map(|v| v.parse().ok())
                .collect();
            let k = head.find("/Contents").expect("contents") + "/Contents".len();
            let rest = &head[k..];
            let contents = match rest.trim_start().strip_prefix('[') {
                Some(r) => refs(&r[..r.find(']').unwrap_or(r.len())]),
                None => refs(rest).into_iter().take(1).collect(),
            };
            let content: Vec<u8> = contents
                .iter()
                .flat_map(|r| {
                    let mut v = objs[r].1.clone().unwrap_or_default();
                    v.push(b'\n');
                    v
                })
                .collect();
            Page {
                width: (b[2] - b[0]) / MM,
                height: (b[3] - b[1]) / MM,
                segments: segments(&content)
                    .into_iter()
                    .map(|s| [s[0] / MM, s[1] / MM, s[2] / MM, s[3] / MM])
                    .collect(),
            }
        })
        .collect()
}

/// Where the PDF differs from the facts: the pages, their sizes, and on each
/// the segment drawn between the two given points (either way round).
pub fn pdf_problems(data: &[u8], want: &Value) -> Vec<String> {
    let tol = want["tolerance"].as_f64().expect("a tolerance");
    let pages = pdf_pages(data);
    let wanted = want["pages"].as_array().expect("pages");
    if pages.len() != wanted.len() {
        return vec![format!("{} sayfa, beklenen {}", pages.len(), wanted.len())];
    }
    let mut out = Vec::new();
    for (n, (p, w)) in pages.iter().zip(wanted).enumerate() {
        let (ww, wh) = (
            w["width"].as_f64().expect("w"),
            w["height"].as_f64().expect("h"),
        );
        if (p.width - ww).abs() > tol || (p.height - wh).abs() > tol {
            out.push(format!(
                "sayfa {}: {:.3} × {:.3} mm, beklenen {ww} × {wh}",
                n + 1,
                p.width,
                p.height
            ));
        }
        let (a, b) = (pt(&w["segment"][0]), pt(&w["segment"][1]));
        let near = |s: &[f64; 4], a: [f64; 2], b: [f64; 2]| {
            (s[0] - a[0]).abs() <= tol
                && (s[1] - a[1]).abs() <= tol
                && (s[2] - b[0]).abs() <= tol
                && (s[3] - b[1]).abs() <= tol
        };
        if !p.segments.iter().any(|s| near(s, a, b) || near(s, b, a)) {
            out.push(format!("sayfa {}: {a:?}–{b:?} çizgisi yok", n + 1));
        }
    }
    out
}

/// Where each page's words (poppler's `pdftotext`, an independent reader) lack the facts' texts;
/// none without `pdftotext`.
pub fn pdf_text_problems(path: &std::path::Path, want: &Value) -> Option<Vec<String>> {
    let mut out = Vec::new();
    for (n, page) in want["pages"].as_array()?.iter().enumerate() {
        let n = (n + 1).to_string();
        let run = std::process::Command::new("pdftotext")
            .args(["-layout", "-f", &n, "-l", &n])
            .arg(path)
            .arg("-")
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&run.stdout);
        for word in page["texts"].as_array().into_iter().flatten() {
            let word = word.as_str().unwrap_or_default();
            if !text.contains(word) {
                out.push(format!("sayfa {n}: “{word}” yazısı yok"));
            }
        }
    }
    Some(out)
}
