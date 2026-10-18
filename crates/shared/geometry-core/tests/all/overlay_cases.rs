//! The overlay's correctness corpus (docs/adr/0500, plan/gis.md CBS-13): fixtures/overlay/v1/cases.json, written by
//! scripts/fixtures/overlay_cases.py from GEOS (straight edges) and closed forms (arcs), neither KentOS code. Each case's
//! union, intersection and two differences must measure what the reference says, within the case's budget, with as many
//! areas and holes; every result must be valid; union must be idempotent; the identities between the four must hold;
//! sets joined one after another must measure as all at once (the shape of TODOS.md GEO-01).

use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::ops::geoprocess::validity::problems;
use kentos_geometry_core::ops::geoprocess::{
    areas_measure, intersect_all, subtract_all, union_all,
};
use serde_json::Value;

fn fixture() -> Value {
    let path = format!(
        "{}/../../../fixtures/overlay/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("the fixture")).expect("JSON");
    assert_eq!(file["format"], "kentos.overlay-cases");
    assert_eq!(file["version"], 1);
    file
}

fn areas(v: &Value) -> Vec<Area> {
    v.as_array()
        .expect("areas")
        .iter()
        .map(|a| Area::from_json(&Json::parse(&a.to_string()).expect("JSON")).expect("an area"))
        .collect()
}

/// The result's areas and holes, as the reference counts them.
fn counts(r: &[Area]) -> (usize, usize) {
    (r.len(), r.iter().map(|a| a.holes.len()).sum())
}

fn shape(a: &Area) -> Shape {
    Shape::Polygon {
        pts: a.outer.pts.clone(),
        bulges: a.outer.bulges.clone(),
        holes: (!a.holes.is_empty()).then(|| a.holes.clone()),
        parts: None,
    }
}

fn check(name: &str, what: &str, got: &[Area], want: &Value, budget: f64, out: &mut Vec<String>) {
    let measure = areas_measure(got);
    let expect = want["area"].as_f64().expect("area");
    if (measure - expect).abs() > budget {
        out.push(format!(
            "{name} {what}: {measure} m² ≠ {expect} m² (pay {budget})"
        ));
    }
    let n = (
        want["areas"].as_u64().expect("areas") as usize,
        want["holes"].as_u64().expect("holes") as usize,
    );
    if counts(got) != n {
        out.push(format!(
            "{name} {what}: {:?} alan ve delik, beklenen {n:?}",
            counts(got)
        ));
    }
    for a in got {
        let p = problems(&shape(a));
        if !p.is_empty() {
            out.push(format!(
                "{name} {what}: geçersiz sonuç {:?}",
                p.iter().map(|x| x.kind.key()).collect::<Vec<_>>()
            ));
        }
    }
}

#[test]
fn every_overlay_case_measures_as_the_reference_and_its_results_are_valid() {
    let file = fixture();
    let mut out = Vec::new();
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 25, "{} durum", cases.len());
    for c in cases {
        let name = c["name"].as_str().expect("name");
        let budget = c["budget"].as_f64().expect("budget");
        let (a, b) = (areas(&c["a"]), areas(&c["b"]));
        let both: Vec<Area> = a.iter().chain(&b).cloned().collect();
        let union = union_all(&both);
        let inter = intersect_all(&a, &b);
        let a_b = subtract_all(&a, &b);
        let b_a = subtract_all(&b, &a);
        let e = &c["expect"];
        check(name, "birleşim", &union, &e["union"], budget, &mut out);
        check(
            name,
            "kesişim",
            &inter,
            &e["intersection"],
            budget,
            &mut out,
        );
        check(name, "a − b", &a_b, &e["aMinusB"], budget, &mut out);
        check(name, "b − a", &b_a, &e["bMinusA"], budget, &mut out);
        // Identities: |A ∪ B| = |A| + |B| − |A ∩ B|; |A − B| + |A ∩ B| = |A|.
        let (ua, ub) = (areas_measure(&union_all(&a)), areas_measure(&union_all(&b)));
        let (mu, mi) = (areas_measure(&union), areas_measure(&inter));
        if (mu - (ua + ub - mi)).abs() > 2.0 * budget {
            out.push(format!(
                "{name}: |A∪B| {mu} ≠ |A|+|B|−|A∩B| {}",
                ua + ub - mi
            ));
        }
        if (areas_measure(&a_b) + mi - ua).abs() > 2.0 * budget {
            out.push(format!("{name}: |A−B|+|A∩B| ≠ |A| {ua}"));
        }
        // Idempotent: the union joined again, and with a, is itself.
        let mut again = union.clone();
        again.extend(a.iter().cloned());
        let twice = union_all(&again);
        if (areas_measure(&twice) - mu).abs() > budget || counts(&twice) != counts(&union) {
            out.push(format!(
                "{name}: birleşim kendisi ve a ile yeniden birleşince {} m², {:?} (önce {mu} m², {:?})",
                areas_measure(&twice),
                counts(&twice),
                counts(&union)
            ));
        }
        // Joined one after another, each with the result before it.
        if let Some(chain) = c["chain"].as_array() {
            let mut acc: Vec<Area> = Vec::new();
            for set in chain {
                acc.extend(areas(set));
                acc = union_all(&acc);
            }
            check(name, "zincir", &acc, &e["union"], budget, &mut out);
        }
    }
    assert!(out.is_empty(), "{}", out.join("\n"));
}
