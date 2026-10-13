//! Adres arama (docs/adr/0229 §3–§9, §13.1) against the independent
//! reference in `fixtures/geocode/v1/cases.json`
//! (`scripts/fixtures/geocode_cases.py`, no KentOS code): folding and words,
//! an address's parts, names' similarity, which attributes say what, and two
//! scenes (metres and millimetres) searched by address, by columns and by
//! place. Scores, kinds, statuses, texts and orders are exact; places agree
//! within 1e-6 of the project's unit.

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::geocode::fields::{self, AreaFields, DoorFields, StreetFields};
use kentos_geometry_core::ops::geocode::parse::from_columns;
use kentos_geometry_core::ops::geocode::{
    AreaInput, DoorInput, Input, Level, Locator, Parsed, StreetInput, Tag, find, fold, parse,
    similarity, tokens,
};
use kentos_geometry_core::vec2::Vec2;
use serde_json::Value;

fn fixture() -> Value {
    let path = format!(
        "{}/../../../fixtures/geocode/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.geocode-cases");
    assert_eq!(file["version"], 1);
    file
}

fn text(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}

fn opt(v: &Value) -> Option<String> {
    v.as_str().map(str::to_owned)
}

fn tag_name(t: Option<Tag>) -> Option<String> {
    t.map(|t| t.name().to_owned())
}

#[test]
fn words_fold_as_the_reference_folds_them() {
    let f = fixture();
    for case in f["folds"].as_array().expect("folds") {
        let t = text(&case["text"]);
        assert_eq!(fold(t), text(&case["folded"]), "{t}");
        let words: Vec<String> = tokens(&fold(t))
            .into_iter()
            .map(|t| match t {
                kentos_geometry_core::ops::geocode::Token::Word(w) => w,
                kentos_geometry_core::ops::geocode::Token::Slash => "/".into(),
                kentos_geometry_core::ops::geocode::Token::Comma => ",".into(),
                kentos_geometry_core::ops::geocode::Token::Dash => "-".into(),
            })
            .collect();
        let want: Vec<String> = case["tokens"]
            .as_array()
            .expect("tokens")
            .iter()
            .map(|w| text(w).to_owned())
            .collect();
        assert_eq!(words, want, "{t}");
    }
}

fn parsed_json(p: &Parsed) -> Value {
    serde_json::json!({
        "postcode": p.postcode,
        "street": p.street,
        "streetKind": tag_name(p.street_kind),
        "door": p.door,
        "neighbourhood": p.neighbourhood,
        "district": p.district,
        "province": p.province,
        "rest": p.rest,
        "untagged": p.untagged,
    })
}

#[test]
fn addresses_read_into_the_references_parts() {
    let f = fixture();
    for case in f["parses"].as_array().expect("parses") {
        let t = text(&case["text"]);
        assert_eq!(parsed_json(&parse(t)), case["parsed"], "{t}");
    }
}

#[test]
fn similarities_are_the_references() {
    let f = fixture();
    for case in f["similarities"].as_array().expect("similarities") {
        let (a, b) = (text(&case["a"]), text(&case["b"]));
        assert_eq!(
            u64::from(similarity(a, b)),
            case["similarity"].as_u64().expect("a number"),
            "{a} ~ {b}"
        );
    }
}

#[test]
fn fields_are_guessed_as_the_reference_guesses() {
    let f = fixture();
    for case in f["fields"].as_array().expect("fields") {
        let names: Vec<String> = case["fields"]
            .as_array()
            .expect("names")
            .iter()
            .map(|n| text(n).to_owned())
            .collect();
        let g = &case["guess"];
        match text(&case["kind"]) {
            "doors" => assert_eq!(
                fields::doors(&names),
                DoorFields {
                    street: opt(&g["street"]),
                    door: opt(&g["door"]),
                    neighbourhood: opt(&g["neighbourhood"]),
                    district: opt(&g["district"]),
                    province: opt(&g["province"]),
                    postcode: opt(&g["postcode"]),
                    building: opt(&g["building"]),
                    code: opt(&g["code"]),
                },
                "{names:?}"
            ),
            "streets" => assert_eq!(
                fields::streets(&names),
                StreetFields {
                    name: opt(&g["name"]),
                    neighbourhood: opt(&g["neighbourhood"]),
                    district: opt(&g["district"]),
                },
                "{names:?}"
            ),
            level => {
                let level = match level {
                    "neighbourhood" => Level::Neighbourhood,
                    "district" => Level::District,
                    _ => Level::Province,
                };
                assert_eq!(
                    fields::areas(level, &names),
                    AreaFields {
                        name: opt(&g["name"]),
                        district: opt(&g["district"]),
                    },
                    "{names:?}"
                );
            }
        }
    }
}

fn point(v: &Value) -> Vec2 {
    let a = v.as_array().expect("a point");
    Vec2::new(a[0].as_f64().expect("x"), a[1].as_f64().expect("y"))
}

/// A scene's input, as a host would read it.
fn input(scene: &Value) -> Input {
    let order = |v: &Value| v["order"].as_u64().expect("an order") as u32;
    Input {
        per_metre: scene["perMetre"].as_f64().expect("a unit"),
        doors: scene["doors"]
            .as_array()
            .expect("doors")
            .iter()
            .map(|d| DoorInput {
                order: order(d),
                at: point(&d["at"]),
                street: text(&d["street"]).to_owned(),
                door: text(&d["door"]).to_owned(),
                neighbourhood: text(&d["neighbourhood"]).to_owned(),
                district: text(&d["district"]).to_owned(),
                province: text(&d["province"]).to_owned(),
                postcode: text(&d["postcode"]).to_owned(),
                building: text(&d["building"]).to_owned(),
                code: text(&d["code"]).to_owned(),
            })
            .collect(),
        streets: scene["streets"]
            .as_array()
            .expect("streets")
            .iter()
            .map(|s| StreetInput {
                order: order(s),
                shape: Shape::Polyline {
                    pts: s["pts"]
                        .as_array()
                        .expect("points")
                        .iter()
                        .map(point)
                        .collect(),
                    bulges: None,
                    holes: None,
                    parts: None,
                    closed: false,
                    widths: None,
                },
                name: text(&s["name"]).to_owned(),
                neighbourhood: text(&s["neighbourhood"]).to_owned(),
                district: text(&s["district"]).to_owned(),
            })
            .collect(),
        areas: scene["areas"]
            .as_array()
            .expect("areas")
            .iter()
            .map(|a| {
                let rings = a["rings"].as_array().expect("rings");
                AreaInput {
                    order: order(a),
                    level: match text(&a["level"]) {
                        "neighbourhood" => Level::Neighbourhood,
                        "district" => Level::District,
                        _ => Level::Province,
                    },
                    shape: Shape::Polygon {
                        pts: rings[0]
                            .as_array()
                            .expect("a ring")
                            .iter()
                            .map(point)
                            .collect(),
                        bulges: None,
                        holes: None,
                        parts: None,
                    },
                    name: text(&a["name"]).to_owned(),
                    district: text(&a["district"]).to_owned(),
                }
            })
            .collect(),
        ..Input::default()
    }
}

fn near(a: Vec2, want: &Value, what: &str) {
    let w = point(want);
    let tol = 1e-6;
    assert!(
        (a.x - w.x).abs() <= tol && (a.y - w.y).abs() <= tol,
        "{what}: {a:?}, the reference's {w:?}"
    );
}

fn texts_json(t: &kentos_geometry_core::ops::geocode::locator::Texts) -> Value {
    serde_json::json!({
        "street": t.street,
        "door": t.door,
        "neighbourhood": t.neighbourhood,
        "district": t.district,
        "province": t.province,
        "postcode": t.postcode,
        "building": t.building,
        "code": t.code,
    })
}

fn same_found(found: &kentos_geometry_core::ops::geocode::Found, want: &Value, what: &str) {
    assert_eq!(found.status.word(), text(&want["status"]), "{what}: status");
    let cands = want["candidates"].as_array().expect("candidates");
    assert_eq!(
        found.candidates.len(),
        cands.len(),
        "{what}: {:#?}",
        found.candidates
    );
    for (i, (c, w)) in found.candidates.iter().zip(cands).enumerate() {
        let at = format!("{what} #{i}");
        assert_eq!(c.kind.word(), text(&w["kind"]), "{at}: kind");
        assert_eq!(
            u64::from(c.score),
            w["score"].as_u64().expect("a score"),
            "{at}: score"
        );
        assert_eq!(c.label, text(&w["label"]), "{at}: label");
        assert_eq!(texts_json(&c.texts), w["texts"], "{at}: texts");
        assert_eq!(
            u64::from(c.order),
            w["order"].as_u64().expect("an order"),
            "{at}: order"
        );
        near(c.at, &w["at"], &at);
    }
}

#[test]
fn scenes_answer_as_the_reference_answers() {
    let f = fixture();
    for s in f["scenes"].as_array().expect("scenes") {
        let name = text(&s["name"]);
        let loc = Locator::build(&input(&s["scene"]));
        let counts = loc.counts();
        let c = &s["counts"];
        assert_eq!(
            counts.doors as u64,
            c["doors"].as_u64().unwrap_or(0),
            "{name}: doors"
        );
        assert_eq!(
            counts.streets as u64,
            c["streets"].as_u64().unwrap_or(0),
            "{name}: streets"
        );
        assert_eq!(
            counts.lines as u64,
            c["lines"].as_u64().unwrap_or(0),
            "{name}: lines"
        );
        assert_eq!(
            counts.neighbourhoods as u64,
            c["neighbourhoods"].as_u64().unwrap_or(0),
            "{name}: neighbourhoods"
        );
        assert_eq!(
            counts.areas as u64,
            c["areas"].as_u64().unwrap_or(0),
            "{name}: areas"
        );
        assert_eq!(
            loc.empty_streets as u64,
            c["emptyStreets"].as_u64().unwrap_or(0)
        );
        assert_eq!(
            loc.unread_doors as u64,
            c["unreadDoors"].as_u64().unwrap_or(0)
        );
        let min = s["minScore"].as_u64().expect("a score") as u32;
        let limit = s["limit"].as_u64().expect("a limit") as usize;
        for q in s["forward"].as_array().expect("forward") {
            let t = text(&q["text"]);
            let found = find(&loc, t, &parse(t), min, limit);
            same_found(&found, q, &format!("{name}: {t}"));
        }
        for q in s["columns"].as_array().expect("columns") {
            let c = &q["columns"];
            let parsed = from_columns(
                text(&c["neighbourhood"]),
                text(&c["street"]),
                text(&c["door"]),
                text(&c["district"]),
                text(&c["province"]),
                text(&c["postcode"]),
            );
            let found = find(&loc, "", &parsed, min, limit);
            same_found(&found, q, &format!("{name}: {c}"));
        }
        for q in s["reverse"].as_array().expect("reverse") {
            let p = point(&q["at"]);
            let reach = q["reach"].as_f64().expect("a reach");
            let what = format!("{name}: {p:?} within {reach} m");
            let got = loc.reverse(p, reach);
            let want = &q["place"];
            match (got, want.is_null()) {
                (None, true) => {}
                (Some(place), false) => {
                    assert_eq!(place.kind.word(), text(&want["kind"]), "{what}");
                    assert_eq!(place.label, text(&want["label"]), "{what}");
                    assert_eq!(texts_json(&place.texts), want["texts"], "{what}");
                    near(place.at, &want["at"], &what);
                    match (place.distance, want["distance"].as_f64()) {
                        (None, None) => {}
                        (Some(d), Some(w)) => assert!((d - w).abs() <= 1e-9, "{what}: {d} {w}"),
                        (d, w) => panic!("{what}: distance {d:?}, the reference's {w:?}"),
                    }
                }
                (got, _) => panic!("{what}: {got:?}, the reference's {want}"),
            }
        }
    }
}

// ── docs/adr/0229 §17: the budgets ──────────────────────────────────────

/// A name of three syllables from `i` (base 30), its first letter capital.
fn syllables(mut i: usize) -> String {
    const S: [&str; 30] = [
        "ka", "ra", "de", "niz", "yıl", "dız", "gül", "ba", "har", "ma", "vi", "ce", "tin", "ar",
        "su", "bey", "ta", "şı", "lı", "ova", "kent", "pı", "nar", "ça", "yır", "ko", "zan", "ser",
        "in", "göl",
    ];
    let mut out = String::new();
    for _ in 0..3 {
        out.push_str(S[i % 30]);
        i /= 30;
    }
    let mut c = out.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

/// The synthetic city of §17: 500 neighbourhoods of 200 m in a 25 × 20 grid, each with 20 streets of 180 m (2 000
/// names, each in five neighbourhoods) and 20 doors on each street (odd on one side, even on the other); their
/// areas, ten districts and the province. Returns the input and each door's (neighbourhood, street, door, place).
fn city() -> (Input, Vec<(String, String, u32, Vec2)>) {
    let kinds = ["Caddesi", "Sokak", "Bulvarı"];
    let mut input = Input {
        per_metre: 1.0,
        ..Input::default()
    };
    let mut doors = Vec::new();
    let mut order = 0u32;
    let mut next = || {
        order += 1;
        order
    };
    let square = |x0: f64, y0: f64, w: f64, h: f64| Shape::Polygon {
        pts: vec![
            Vec2::new(x0, y0),
            Vec2::new(x0 + w, y0),
            Vec2::new(x0 + w, y0 + h),
            Vec2::new(x0, y0 + h),
        ],
        bulges: None,
        holes: None,
        parts: None,
    };
    input.areas.push(AreaInput {
        order: next(),
        level: Level::Province,
        shape: square(-10.0, -10.0, 5020.0, 4020.0),
        name: "Ankara".into(),
        district: String::new(),
    });
    for d in 0..10 {
        input.areas.push(AreaInput {
            order: next(),
            level: Level::District,
            shape: square(
                f64::from(d % 5) * 1000.0,
                f64::from(d / 5) * 2000.0,
                1000.0,
                2000.0,
            ),
            name: format!("{}ilçe", syllables(9000 + d as usize)),
            district: String::new(),
        });
    }
    for h in 0..500usize {
        let (gx, gy) = ((h % 25) as f64 * 200.0, (h / 25) as f64 * 200.0);
        let hood = syllables(5000 + h);
        let district = format!(
            "{}ilçe",
            syllables(9000 + (h % 25) / 5 + 5 * ((h / 25) / 10))
        );
        input.areas.push(AreaInput {
            order: next(),
            level: Level::Neighbourhood,
            shape: square(gx, gy, 200.0, 200.0),
            name: format!("{hood} Mahallesi"),
            district: district.clone(),
        });
        for s in 0..20usize {
            let name = format!("{} {}", syllables((h * 20 + s) % 2000), kinds[s % 3]);
            let y = gy + 5.0 + s as f64 * 9.5;
            input.streets.push(StreetInput {
                order: next(),
                shape: Shape::Polyline {
                    pts: vec![Vec2::new(gx + 10.0, y), Vec2::new(gx + 190.0, y)],
                    bulges: None,
                    holes: None,
                    parts: None,
                    closed: false,
                    widths: None,
                },
                name: name.clone(),
                neighbourhood: hood.clone(),
                district: district.clone(),
            });
            for n in 1..=20u32 {
                let side = if n % 2 == 1 { -2.0 } else { 2.0 };
                let at = Vec2::new(gx + 10.0 + f64::from((n - 1) / 2) * 18.0, y + side);
                input.doors.push(DoorInput {
                    order: next(),
                    at,
                    street: name.clone(),
                    door: n.to_string(),
                    neighbourhood: hood.clone(),
                    district: district.clone(),
                    province: "Ankara".into(),
                    postcode: format!("06{:03}", h % 1000),
                    building: String::new(),
                    code: String::new(),
                });
                doors.push((hood.clone(), name.clone(), n, at));
            }
        }
    }
    (input, doors)
}

/// One letter of a name changed (its second letter, to another one).
fn mistyped(name: &str) -> String {
    let mut chars: Vec<char> = name.chars().collect();
    if chars.len() > 2 {
        chars[1] = if chars[1] == 'x' { 'q' } else { 'x' };
    }
    chars.into_iter().collect()
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

/// How long building and asking a city's address data take (docs/adr/0229 §17), release build, one thread:
/// `cargo test --release -p kentos-geometry-core --test all geocode::timing -- --ignored --nocapture`.
#[test]
#[ignore = "süre ölçümü: cargo test --release -p kentos-geometry-core --test all geocode::timing -- --ignored --nocapture"]
fn timing() {
    use std::time::Instant;
    let (input, doors) = city();
    assert_eq!((input.doors.len(), input.streets.len()), (200_000, 10_000));
    let mut over = Vec::new();
    let mut say = |name: &str, value: f64, unit: &str, budget: f64| {
        let ok = value <= budget;
        println!(
            "{name:<52} {value:>10.3} {unit}  bütçe {budget:>6} {unit}: {}",
            if ok {
                "bütçede"
            } else {
                "BÜTÇEYİ AŞIYOR"
            }
        );
        if !ok {
            over.push(name.to_owned());
        }
    };
    // The index, best of three.
    let mut best = f64::INFINITY;
    let mut loc = None;
    for _ in 0..3 {
        let t = Instant::now();
        let l = Locator::build(&input);
        best = best.min(t.elapsed().as_secs_f64() * 1000.0);
        loc = Some(l);
    }
    let loc = loc.expect("built");
    say(
        "Dizinin kurulması (200 000 kapı, 10 000 yol, 511 alan)",
        best,
        "ms",
        300.0,
    );
    // One address: exactly, and with a letter wrong and no neighbourhood; 1 000 of each, the median.
    let picked: Vec<&(String, String, u32, Vec2)> = doors.iter().step_by(200).collect();
    let mut exact = Vec::new();
    let mut matched = 0;
    for (hood, street, n, _) in &picked {
        let text = format!("{hood} Mah. {street} No:{n}");
        let q = parse(&text);
        let t = Instant::now();
        let found = find(&loc, &text, &q, 85, 20);
        exact.push(t.elapsed().as_secs_f64() * 1e6);
        if found.status.word() == "Eşleşti" {
            matched += 1;
        }
    }
    assert_eq!(
        matched,
        picked.len(),
        "every address typed exactly matches its door"
    );
    say("Tek adres, tam eşleşme (p50)", median(exact), "µs", 20.0);
    let mut typo = Vec::new();
    let mut found_any = 0;
    for (_, street, n, _) in &picked {
        let text = format!("{} No:{n}", mistyped(street));
        let q = parse(&text);
        let t = Instant::now();
        let found = find(&loc, &text, &q, 85, 20);
        typo.push(t.elapsed().as_secs_f64() * 1000.0);
        if !found.candidates.is_empty() {
            found_any += 1;
        }
    }
    assert!(
        found_any * 10 >= picked.len() * 9,
        "{found_any} / {}",
        picked.len()
    );
    say(
        "Tek adres, bir harf yanlış, mahallesiz (p50)",
        median(typo),
        "ms",
        2.0,
    );
    // A batch of 100 000 addresses, one in ten with a letter wrong; their parts first.
    let batch: Vec<String> = doors
        .iter()
        .step_by(2)
        .enumerate()
        .map(|(i, (hood, street, n, _))| {
            let street = if i % 10 == 0 {
                mistyped(street)
            } else {
                street.clone()
            };
            format!("{hood} Mah. {street} No:{n}")
        })
        .collect();
    assert_eq!(batch.len(), 100_000);
    let t = Instant::now();
    let parts: Vec<_> = batch.iter().map(|a| parse(a)).collect();
    say(
        "100 000 adresin parçalarına ayrılması",
        t.elapsed().as_secs_f64() * 1000.0,
        "ms",
        200.0,
    );
    let t = Instant::now();
    let mut ok = 0;
    for (a, q) in batch.iter().zip(&parts) {
        if find(&loc, a, q, 85, 20).status.word() == "Eşleşti" {
            ok += 1;
        }
    }
    say(
        "100 000 adreslik toplu eşleme (%10'u bir harf yanlış)",
        t.elapsed().as_secs_f64(),
        "s",
        3.0,
    );
    println!("{:<52} {ok} / 100 000 eşleşti", "");
    // 100 000 places' addresses, spread over the city.
    let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
    let mut rand = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let places: Vec<Vec2> = (0..100_000)
        .map(|_| Vec2::new(rand() * 5000.0, rand() * 4000.0))
        .collect();
    let t = Instant::now();
    let answered = places
        .iter()
        .filter(|p| loc.reverse(**p, 100.0).is_some())
        .count();
    say(
        "100 000 noktaya konumdan adres",
        t.elapsed().as_secs_f64(),
        "s",
        1.0,
    );
    assert_eq!(
        answered,
        places.len(),
        "every place is in the province at least"
    );
    assert!(over.is_empty(), "bütçeyi aşanlar: {over:?}");
}
