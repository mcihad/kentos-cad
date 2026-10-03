//! The magnetic model against NOAA's own test values (design §8a): the 12
//! of the model's page (`WMM2025_TEST_VALUES.txt`) and the 100 of the
//! coefficient package (`WMM2025_TestValues.txt`), kept as they came in
//! `fixtures/sheet/v1/wmm/`. Declination and inclination within 0.01° (the
//! files write them to 0.01°); the components within 0.1 nT of the page's
//! values (written to 0.1 nT) and 0.01 nT of the package's (to 1e-6 nT).

mod common;

use common::fixtures;
use kentos_sheet::wmm::field;

fn rows(file: &str) -> Vec<Vec<f64>> {
    std::fs::read_to_string(fixtures().join("wmm").join(file))
        .unwrap_or_else(|e| panic!("{file}: {e}"))
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .map(|l| {
            l.split_whitespace()
                .map(|v| v.parse::<f64>().unwrap_or(f64::NAN))
                .collect()
        })
        .collect()
}

#[test]
fn the_page_s_test_values() {
    // Date, height (km), latitude, longitude, X, Y, Z, H, F, I, D, GV, then the rates.
    let rows = rows("WMM2025_TEST_VALUES.txt");
    assert_eq!(rows.len(), 12);
    for r in rows {
        let f = field(r[2], r[3], r[1], r[0]).expect("a field");
        let at = format!("{} {} km ({}, {})", r[0], r[1], r[2], r[3]);
        for (have, want, name) in [
            (f.x, r[4], "X"),
            (f.y, r[5], "Y"),
            (f.z, r[6], "Z"),
            (f.h, r[7], "H"),
            (f.f, r[8], "F"),
        ] {
            assert!((have - want).abs() <= 0.1, "{at}: {name} {have} ≠ {want}");
        }
        assert!(
            (f.inclination - r[9]).abs() <= 0.01,
            "{at}: I {} ≠ {}",
            f.inclination,
            r[9]
        );
        assert!(
            (f.declination - r[10]).abs() <= 0.01,
            "{at}: D {} ≠ {}",
            f.declination,
            r[10]
        );
    }
}

#[test]
fn the_coefficient_package_s_test_values() {
    // Decimal year, height (km), latitude, longitude, D, I, H, X, Y, Z, F, then the rates.
    let rows = rows("WMM2025_TestValues.txt");
    assert_eq!(rows.len(), 100);
    let mut worst = (0.0_f64, 0.0_f64);
    for r in rows {
        let f = field(r[2], r[3], r[1], r[0]).expect("a field");
        let at = format!("{} {} km ({}, {})", r[0], r[1], r[2], r[3]);
        let dd = (f.declination - r[4]).abs();
        let di = (f.inclination - r[5]).abs();
        assert!(dd <= 0.01, "{at}: D {} ≠ {}", f.declination, r[4]);
        assert!(di <= 0.01, "{at}: I {} ≠ {}", f.inclination, r[5]);
        for (have, want, name) in [
            (f.h, r[6], "H"),
            (f.x, r[7], "X"),
            (f.y, r[8], "Y"),
            (f.z, r[9], "Z"),
            (f.f, r[10], "F"),
        ] {
            assert!((have - want).abs() <= 0.01, "{at}: {name} {have} ≠ {want}");
            worst.1 = worst.1.max((have - want).abs());
        }
        worst.0 = worst.0.max(dd.max(di));
    }
    println!(
        "en büyük açı farkı {:.4}°, en büyük bileşen farkı {:.6} nT",
        worst.0, worst.1
    );
}

/// Where the sheets are: Sivas Suşehri on 3 October 2026, east of true north by about 6°.
#[test]
fn turkey_s_declination_is_a_few_degrees_east() {
    let d = kentos_sheet::wmm::declination(40.16, 38.08, 2026.75).expect("a declination");
    assert!((5.0..8.0).contains(&d), "{d}");
}

/// The ifraz template's sheet at `CENTER` (Ankara, TUREF / TM33), its north arrow set as given.
fn sheet_with(north: serde_json::Value) -> (kentos_sheet::SheetBook, kentos_sheet::ItemId) {
    use kentos_sheet::ops::{Op, SetItemProps, apply};
    use kentos_sheet::template::*;
    let t = system_templates()
        .iter()
        .find(|t| t.meta.id == "sys:ifraz-paftasi")
        .expect("the ifraz template");
    let ids = InstanceIds {
        sheet: "s1".into(),
        master: t.master.as_ref().map(|_| "m1".into()),
        items: t.sheet.items.iter().map(|i| i.id.clone()).collect(),
        master_items: t
            .master
            .iter()
            .flat_map(|m| m.items.iter().map(|i| format!("m-{}", i.id)))
            .collect(),
    };
    let options = InstanceOptions {
        paper: None,
        name: None,
        center: Some(common::CENTER),
        scale: None,
        values: common::sample_values(),
    };
    let inst = instantiate(t, &ids, &options).expect("instantiate");
    let book = kentos_sheet::SheetBook {
        sheets: vec![inst.sheet],
        masters: inst.master.into_iter().collect(),
        assets: inst.assets,
        ..kentos_sheet::SheetBook::default()
    };
    let arrow = book.sheets[0]
        .items
        .iter()
        .find(|i| matches!(i.kind, kentos_sheet::kinds::ItemKind::NorthArrow(_)))
        .map(|i| i.id.clone())
        .expect("a north arrow");
    let mut patch = serde_json::json!({ "type": "northArrow" });
    for (k, v) in north.as_object().expect("an object") {
        patch[k] = v.clone();
    }
    let op = Op::SetItemProps(SetItemProps {
        id: arrow.clone(),
        patch: serde_json::json!({ "kind": patch }),
    });
    (apply(&book, &op).expect("applied").book, arrow)
}

fn texts_of(list: &kentos_sheet::display::DisplayList, item: &str) -> Vec<String> {
    list.prims
        .iter()
        .filter_map(|p| match p {
            kentos_sheet::display::Prim::Text(t) if t.item == item => Some(t.text.clone()),
            _ => None,
        })
        .collect()
}

/// `north: magnetic`: the model's declination at the map's centre on the sheet's date, written
/// with its source; typed by hand, written “elle”; the arrow turned by it.
#[test]
fn a_magnetic_north_arrow_takes_the_model_s_declination_unless_typed() {
    use kentos_sheet::display::display_list;
    let (book, arrow) = sheet_with(serde_json::json!({ "north": "magnetic", "note": true }));
    let mut inputs = common::sample_inputs(&book, "s1", true);
    inputs.project.date = "2026-10-03".into();
    let list = display_list(&book, "s1", &inputs).expect("a list");
    // The expected value: the model at CENTER's latitude and longitude (TM33) on 2026-10-03.
    let g = kentos_sheet::geodesy::tm_inverse(&common::tm33(), common::CENTER.x, common::CENTER.y)
        .expect("a place");
    let d = kentos_sheet::wmm::declination(g.lat, g.lon, 2026.0 + 275.0 / 365.0).expect("a value");
    let minutes = (d * 60.0).round() as i64;
    let want = format!(
        "Manyetik sapma {}°{:02}' D (WMM2025, 2026-10)",
        minutes / 60,
        minutes % 60
    );
    let texts = texts_of(&list, &arrow);
    assert!(texts.contains(&want), "{texts:?} ∌ {want}");
    // By hand: 2°30' east, for 2024.
    let (book, arrow) = sheet_with(serde_json::json!({
        "north": "magnetic", "note": true, "declination": 2500, "declinationHand": true, "declinationYear": 2024
    }));
    let list = display_list(&book, "s1", &inputs).expect("a list");
    let texts = texts_of(&list, &arrow);
    assert!(
        texts
            .iter()
            .any(|t| t == "Manyetik sapma 2°30' D (elle, 2024)"),
        "{texts:?}"
    );
    // A book of before the model: a typed declination is by hand.
    let (book, arrow) =
        sheet_with(serde_json::json!({ "north": "magnetic", "note": true, "declination": -1250 }));
    let texts = texts_of(&display_list(&book, "s1", &inputs).expect("a list"), &arrow);
    assert!(
        texts.iter().any(|t| t == "Manyetik sapma 1°15' B (elle)"),
        "{texts:?}"
    );
}

/// The north diagram: GK, CK and MK at the line tips, the three angles under them, and a note
/// that the angles are drawn wider than they are.
#[test]
fn the_north_diagram_names_the_three_norths_and_their_angles() {
    use kentos_sheet::display::display_list;
    let (book, arrow) = sheet_with(serde_json::json!({ "style": "diagram" }));
    let mut inputs = common::sample_inputs(&book, "s1", true);
    inputs.project.date = "2026-10-03".into();
    let texts = texts_of(&display_list(&book, "s1", &inputs).expect("a list"), &arrow);
    for t in ["GK", "CK", "MK", "Açılar ölçekli değildir."] {
        assert!(texts.iter().any(|x| x == t), "{t}: {texts:?}");
    }
    for start in ["GK–CK yakınsama ", "CK–MK manyetik sapma ", "GK–MK açısı "] {
        assert!(
            texts.iter().any(|x| x.starts_with(start)),
            "{start}: {texts:?}"
        );
    }
    assert!(
        texts.iter().any(|x| x.ends_with("(WMM2025, 2026-10)")),
        "{texts:?}"
    );
}

/// The preflight: outside the model's years a warning; with a coordinate system the core cannot
/// invert (or none given) an error with its fixes; without any, `needs_crs` alone.
#[test]
fn the_preflight_says_when_the_model_cannot_be_used() {
    use kentos_sheet::preflight::{Severity, preflight};
    let (book, arrow) = sheet_with(serde_json::json!({ "north": "magnetic" }));
    let found = |inputs: &kentos_sheet::display::RenderInputs| {
        preflight(&book, "s1", inputs)
            .expect("findings")
            .into_iter()
            .filter(|f| f.item.as_deref() == Some(arrow.as_str()))
            .map(|f| (f.severity, f.code, f.fixes.len()))
            .collect::<Vec<_>>()
    };
    let mut inputs = common::sample_inputs(&book, "s1", true);
    inputs.project.date = "2031-02-01".into();
    assert!(found(&inputs).contains(&(Severity::Warning, "magnetic_out_of_model".into(), 2)));
    // Its fixes: the variables (the date), or the declination by hand.
    let out = preflight(&book, "s1", &inputs)
        .expect("findings")
        .into_iter()
        .find(|f| f.code == "magnetic_out_of_model")
        .expect("the warning");
    assert_eq!(
        out.fixes
            .iter()
            .map(|f| (f.label.as_str(), f.action.as_deref(), f.ops.len()))
            .collect::<Vec<_>>(),
        [
            ("Değişkenleri aç", Some("sheet.variables"), 0),
            ("Sapmayı elle gir", None, 1)
        ]
    );
    // By hand, from the model's value then to the minute (as the inspector's switch), nothing
    // having been typed.
    let by_hand = serde_json::to_value(&out.fixes[1].ops[0]).expect("an op");
    let patch = &by_hand["patch"]["kind"];
    let info = kentos_sheet::display::north_info(&book, "s1", &arrow, &inputs).expect("info");
    assert_eq!(patch["declinationHand"], true);
    assert_eq!(
        patch["declination"].as_i64(),
        info.declination
            .map(|d| ((d * 60.0).round() / 60.0 * 1000.0).round() as i64)
    );
    inputs.project.date = "2026-10-03".into();
    assert!(found(&inputs).is_empty(), "{:?}", found(&inputs));
    // A system of no TM parameters.
    inputs.crs = Some(kentos_sheet::display::CrsInfo {
        name: "TUREF".into(),
        tm: None,
    });
    assert!(found(&inputs).contains(&(Severity::Error, "magnetic_no_place".into(), 2)));
    // No coordinate system at all: the arrow's `needs_crs` says it.
    let inputs = common::sample_inputs(&book, "s1", false);
    let codes: Vec<String> = found(&inputs).into_iter().map(|f| f.1).collect();
    assert!(
        codes.contains(&"needs_crs".to_owned()) && !codes.contains(&"magnetic_no_place".to_owned()),
        "{codes:?}"
    );
}

/// The north diagram's lines stay in the arrow's frame (the ifraz template's is 36 mm wide, and
/// narrower ones): a line breaks before its source, then at spaces, and gets smaller only if it
/// must, never below 1.5 mm; what does not fit even so is said (`text_overflow`).
#[test]
fn the_north_diagram_s_text_stays_in_its_frame() {
    use kentos_sheet::display::{Prim, display_list};
    use kentos_sheet::ops::{Op, SetItemProps, apply};
    use kentos_sheet::preflight::preflight;
    let (book, arrow) = sheet_with(serde_json::json!({ "style": "diagram", "north": "magnetic" }));
    let mut inputs = common::sample_inputs(&book, "s1", true);
    inputs.project.date = "2026-10-03".into();
    let sized = |w: i32, h: i32| {
        let op = Op::SetItemProps(SetItemProps {
            id: arrow.clone(),
            patch: serde_json::json!({ "frame": { "width": w, "height": h } }),
        });
        apply(&book, &op).expect("applied").book
    };
    let overflow = |b: &kentos_sheet::SheetBook| {
        preflight(b, "s1", &inputs)
            .expect("findings")
            .into_iter()
            .any(|f| f.code == "text_overflow" && f.item.as_deref() == Some(arrow.as_str()))
    };
    for (w, h) in [(36_000, 36_000), (28_000, 36_000), (22_000, 40_000)] {
        let b = sized(w, h);
        let list = display_list(&b, "s1", &inputs).expect("a list");
        let item = b.sheets[0]
            .items
            .iter()
            .find(|i| i.id == arrow)
            .expect("the arrow");
        let c = item.content_rect();
        let mut out = Vec::new();
        let mut lines = Vec::new();
        for p in &list.prims {
            let Prim::Text(t) = p else { continue };
            if t.item != arrow {
                continue;
            }
            lines.push(t.text.clone());
            let (l, r) = (t.at[0], t.at[0] + t.width);
            if l < c.left || r > c.left + c.width || t.size < kentos_sheet::text::LEGIBLE_MIN {
                out.push(format!("{} [{l}…{r}] size {}", t.text, t.size));
            }
        }
        assert!(
            out.is_empty(),
            "{w}: frame {}…{}: {out:#?}",
            c.left,
            c.left + c.width
        );
        assert!(!overflow(&b), "{w}: {lines:?}");
        // The source stays whole, on its own line when the line breaks.
        assert!(
            lines.iter().any(|l| l.ends_with("(WMM2025, 2026-10)")),
            "{w}: {lines:?}"
        );
    }
    // A frame too small for the angles even at 1.5 mm: said.
    assert!(overflow(&sized(12_000, 14_000)));
}

/// `northInfo`: the map's centre on the ground, the convergence, the declination and its source,
/// the date and where it came from, whether the model holds then; typed by hand; refusals.
#[test]
fn a_north_arrow_says_what_it_shows_and_from_what() {
    use kentos_sheet::display::{DateSource, DeclinationSource, NorthMissing, north_info};
    let (book, arrow) = sheet_with(serde_json::json!({ "north": "magnetic" }));
    let mut inputs = common::sample_inputs(&book, "s1", true);
    inputs.project.date = "2026-10-03".into();
    let i = north_info(&book, "s1", &arrow, &inputs).expect("the arrow's");
    let g = kentos_sheet::geodesy::tm_inverse(&common::tm33(), common::CENTER.x, common::CENTER.y)
        .expect("a place");
    let year = 2026.0 + 275.0 / 365.0;
    assert_eq!((i.lat, i.lon), (Some(g.lat), Some(g.lon)));
    assert_eq!(i.convergence, Some(g.convergence));
    assert_eq!(
        i.declination,
        kentos_sheet::wmm::declination(g.lat, g.lon, year)
    );
    assert_eq!(
        (i.source, i.model.as_str(), i.valid_from, i.valid_until),
        (Some(DeclinationSource::Model), "WMM2025", 2025.0, 2030.0)
    );
    assert_eq!(
        (i.date.as_deref(), i.date_source, i.year, i.in_model),
        (
            Some("2026-10-03"),
            Some(DateSource::Today),
            Some(year),
            Some(true)
        )
    );
    assert!(i.map.is_some() && i.missing.is_none());
    // The sheet's own date, past the model's years.
    let mut dated = book.clone();
    dated.sheets[0]
        .variables
        .push(kentos_sheet::model::Variable {
            name: "tarih".into(),
            label: "Tarih".into(),
            kind: kentos_sheet::model::VarKind::Date,
            value: kentos_sheet::model::VarValue::Text("2031-02-01".into()),
        });
    let i = north_info(&dated, "s1", &arrow, &inputs).expect("the arrow's");
    assert_eq!(
        (i.date.as_deref(), i.date_source, i.in_model),
        (Some("2031-02-01"), Some(DateSource::Sheet), Some(false))
    );
    // Typed by hand.
    let (book, arrow) = sheet_with(serde_json::json!({
        "north": "magnetic", "declination": -1250, "declinationHand": true, "declinationYear": 2024
    }));
    let i = north_info(&book, "s1", &arrow, &inputs).expect("the arrow's");
    assert_eq!(
        (i.declination, i.source, i.hand_year),
        (Some(-1.25), Some(DeclinationSource::Hand), Some(2024))
    );
    // A system the core cannot turn back: no place, said.
    let mut other = inputs.clone();
    other.crs = Some(kentos_sheet::display::CrsInfo {
        name: "TUREF".into(),
        tm: None,
    });
    let (book, arrow) = sheet_with(serde_json::json!({ "north": "magnetic" }));
    let i = north_info(&book, "s1", &arrow, &other).expect("the arrow's");
    assert_eq!(
        (i.lat, i.declination, i.missing),
        (None, None, Some(NorthMissing::NoPlace))
    );
    // Not a north arrow, or not there.
    let map = book.sheets[0]
        .items
        .iter()
        .find(|x| matches!(x.kind, kentos_sheet::kinds::ItemKind::Map(_)))
        .expect("a map")
        .id
        .clone();
    assert_eq!(
        north_info(&book, "s1", &map, &inputs).unwrap_err().code,
        "not_north_arrow"
    );
    assert_eq!(
        north_info(&book, "s1", "yok", &inputs).unwrap_err().code,
        "unknown_item"
    );
}
