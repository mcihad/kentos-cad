//! The frozen answers of the SVG editor's core (fixtures/svg/v1/cases.json,
//! recorded after the core agreed with the TypeScript it replaced; kept
//! since by `scripts/fixtures/ulp_diff.py`, docs/adr/9403 §6): every
//! operation of the table from the page's arguments, and the typed entries
//! (the snap index, the picture's bytes, the PNG's). Answers are compared as
//! the core's JSON reads them: key for key in order, numbers by value (−0 is
//! 0 in the file).

use kentos_geometry_core::api::json::{FromJson, Json, to_string};
use kentos_svg_core::{api, export, snap, trace};

fn fixture() -> Json {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/svg/v1/cases.json"
    );
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    Json::parse(&text).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn cases(file: &Json) -> &[Json] {
    match file.get("cases") {
        Json::Arr(c) => c,
        _ => panic!("cases"),
    }
}

fn text(v: &Json) -> &str {
    match v {
        Json::Str(s) => s,
        v => panic!("yazı değil: {}", to_string(v)),
    }
}

fn num(v: &Json) -> f64 {
    match v {
        Json::Num(x) => *x,
        v => panic!("sayı değil: {}", to_string(v)),
    }
}

fn numbers(v: &Json) -> Vec<f64> {
    match v {
        Json::Arr(items) => items.iter().map(num).collect(),
        v => panic!("dizi değil: {}", to_string(v)),
    }
}

fn bytes(v: &Json) -> Vec<u8> {
    numbers(v).into_iter().map(|x| x as u8).collect()
}

/// The core's answer read back as JSON.
fn read(answer: &str) -> Json {
    Json::parse(answer).unwrap_or_else(|e| panic!("yanıt okunamadı: {e}: {answer}"))
}

/// The answers that differ from the file's, when the file is being written anew: `KENTOS_WRITE_FROZEN` names a
/// folder, the test writes there what the core gives now for the places that differ, and
/// `scripts/fixtures/ulp_diff.py --apply` reads the difference and writes it into the file (the web's recorder is
/// gone, docs/adr/9402; the tool refuses anything but numbers moved in their last places).
struct Patches(Option<Vec<String>>);

impl Patches {
    fn new() -> Patches {
        Patches(std::env::var_os("KENTOS_WRITE_FROZEN").map(|_| Vec::new()))
    }

    /// `got` is the file's answer at `at` (a JSON list of keys and indices), or goes to the patches, or fails.
    fn check(&mut self, at: &str, name: &str, got: &Json, want: &Json) {
        if got == want {
            return;
        }
        match &mut self.0 {
            Some(list) => list.push(format!("{{\"at\":{at},\"value\":{}}}", to_string(got))),
            None => panic!(
                "{at} ({name}):\n  çekirdek: {}\n  dosya:    {}",
                to_string(got),
                to_string(want)
            ),
        }
    }

    fn write(self, name: &str) {
        let (Some(list), Some(dir)) = (self.0, std::env::var_os("KENTOS_WRITE_FROZEN")) else {
            return;
        };
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
        let path = dir.join(format!("{name}.patches.json"));
        std::fs::write(&path, format!("[{}]\n", list.join(",\n")))
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }
}

#[test]
fn table_operations_as_frozen() {
    let file = fixture();
    let mut checked = 0;
    let mut patches = Patches::new();
    for (i, c) in cases(&file).iter().enumerate() {
        let name = text(c.get("fn"));
        if api::find(name).is_none() {
            continue;
        }
        match api::run_named(name, &to_string(c.get("args"))) {
            Ok(answer) => patches.check(
                &format!("[\"cases\",{i},\"expect\"]"),
                name,
                &read(&answer),
                c.get("expect"),
            ),
            Err(message) => assert_eq!(message, text(c.get("throws")), "{i}. durum ({name})"),
        }
        checked += 1;
    }
    assert!(checked > 1000, "{checked}");
    patches.write("svg-cases-table");
}

#[test]
fn every_operation_has_frozen_cases() {
    let file = fixture();
    let names: Vec<&str> = cases(&file).iter().map(|c| text(c.get("fn"))).collect();
    for op in api::OPS {
        let n = names.iter().filter(|&&f| f == op.name).count();
        assert!(n >= 3, "“{}”: {n} durum", op.name);
    }
    for typed in [
        "SnapIndex",
        "inkMask",
        "traceContours",
        "traceBitmap",
        "crc32",
        "withPngDpi",
    ] {
        assert!(names.contains(&typed), "{typed}");
    }
}

#[test]
fn typed_entries_as_frozen() {
    let file = fixture();
    let mut checked = 0;
    let mut patches = Patches::new();
    for (i, c) in cases(&file).iter().enumerate() {
        let name = text(c.get("fn"));
        let w = || num(c.get("width")) as usize;
        let h = || num(c.get("height")) as usize;
        let as_numbers = crate_truthy(c.get("numbers"));
        match name {
            "SnapIndex" => {
                let source = snap::SnapSource::from_json(c.get("source"))
                    .unwrap_or_else(|e| panic!("{i}: {e}"));
                let index = snap::SnapIndex::new(&source).unwrap_or_else(|e| panic!("{i}: {e}"));
                let Json::Arr(queries) = c.get("queries") else {
                    panic!("{i}: queries")
                };
                let Json::Arr(want) = c.get("expect") else {
                    panic!("{i}: expect")
                };
                for (k, q) in queries.iter().enumerate() {
                    let q = numbers_or_flags(q);
                    let from = (q[3] != 0.0).then_some([q[4], q[5]]);
                    let got = read(&to_string(&index.query([q[0], q[1]], q[2], from)));
                    patches.check(
                        &format!("[\"cases\",{i},\"expect\",{k}]"),
                        "SnapIndex",
                        &got,
                        &want[k],
                    );
                }
            }
            "inkMask" => {
                let (threshold, invert) = (num(c.get("threshold")), crate_truthy(c.get("invert")));
                let mask = if as_numbers {
                    trace::ink_mask(
                        w(),
                        h(),
                        numbers(c.get("data")).as_slice(),
                        threshold,
                        invert,
                    )
                } else {
                    trace::ink_mask(w(), h(), bytes(c.get("data")).as_slice(), threshold, invert)
                };
                assert_eq!(mask, bytes(c.get("expect")), "{i}. durum (inkMask)");
            }
            "traceContours" => {
                let got = read(&to_string(&trace::trace_contours(
                    &bytes(c.get("mask")),
                    w(),
                    h(),
                )));
                patches.check(
                    &format!("[\"cases\",{i},\"expect\"]"),
                    name,
                    &got,
                    c.get("expect"),
                );
            }
            "traceBitmap" => {
                let o = trace::TraceOptions::from_json(c.get("options"))
                    .unwrap_or_else(|e| panic!("{i}: {e}"));
                let r = if as_numbers {
                    trace::trace_bitmap(w(), h(), numbers(c.get("data")).as_slice(), &o)
                } else {
                    trace::trace_bitmap(w(), h(), bytes(c.get("data")).as_slice(), &o)
                };
                patches.check(
                    &format!("[\"cases\",{i},\"expect\"]"),
                    name,
                    &read(&to_string(&r)),
                    c.get("expect"),
                );
            }
            "crc32" => assert_eq!(
                f64::from(export::crc32(&bytes(c.get("bytes")))),
                num(c.get("expect")),
                "{i}. durum (crc32)"
            ),
            "withPngDpi" => {
                let got = export::with_png_dpi(&bytes(c.get("bytes")), num(c.get("dpi")));
                let want = match c.get("expect") {
                    Json::Null => None,
                    v => Some(bytes(v)),
                };
                assert_eq!(got, want, "{i}. durum (withPngDpi)");
            }
            _ => continue,
        }
        checked += 1;
    }
    assert!(checked >= 18, "{checked}");
    patches.write("svg-cases-typed");
}

/// A query's six values: x, y, radius, whether it has a start point (as 0 or 1), and that point.
fn numbers_or_flags(v: &Json) -> Vec<f64> {
    match v {
        Json::Arr(items) => items
            .iter()
            .map(|x| match x {
                Json::Bool(b) => f64::from(u8::from(*b)),
                x => num(x),
            })
            .collect(),
        v => panic!("dizi değil: {}", to_string(v)),
    }
}

fn crate_truthy(v: &Json) -> bool {
    kentos_svg_core::shape::truthy(v)
}
