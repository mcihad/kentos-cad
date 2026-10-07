//! Mekânsal ve öznitelik sorgusu (docs/adr/0200) against the independent
//! reference in `fixtures/spatial-query/v1/cases.json`
//! (`scripts/fixtures/spatial_query_cases.py`, no KentOS code): the
//! relations, distances and centres of shape pairs (`ops::spatial_query`),
//! the same through the store (`Store::relate_pairs`), numbers, figures,
//! statistics and Özet istatistik's tables, and join keys and plans
//! (`ops::statistics`, by name as the web calls them through WASM).

use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::api::run_named;
use kentos_geometry_core::entity::Entity;
use kentos_geometry_core::ops::spatial_query::{
    Relation, center_in, contains, distance, geometry_of, intersects,
};
use kentos_geometry_core::ops::statistics::{Dec, figures, join_key, read_number};
use kentos_geometry_core::store::Store;
use serde_json::{Value, json};

fn cases() -> Value {
    let path = format!(
        "{}/../../../fixtures/spatial-query/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.spatial-query-cases");
    file
}

fn shape(v: &Value) -> kentos_geometry_core::entity::Shape {
    let json = Json::parse(&v.to_string()).expect("JSON");
    Entity::from_json(&json).expect("an entity").shape
}

#[test]
fn shape_pairs_relate_as_the_reference_says() {
    let f = cases();
    for r in f["relations"].as_array().expect("relations") {
        let (a, b) = (r["a"].as_str().expect("a"), r["b"].as_str().expect("b"));
        let ga = geometry_of(&shape(&f["shapes"][a])).expect("a geometry");
        let gb = geometry_of(&shape(&f["shapes"][b])).expect("a geometry");
        let name = format!("{a} › {b}");
        assert_eq!(intersects(&ga, &gb), r["intersects"], "{name}: kesişen");
        assert_eq!(contains(&ga, &gb), r["contains"], "{name}: içeren");
        assert_eq!(contains(&gb, &ga), r["within"], "{name}: içinde kalan");
        assert_eq!(center_in(&ga, &gb), r["centerIn"], "{name}: merkezi içinde");
        let want = r["distance"].as_f64().expect("a distance");
        let got = distance(&ga, &gb);
        assert!(
            (got - want).abs() <= 1e-9,
            "{name}: uzaklık {got}, beklenen {want}"
        );
    }
}

#[test]
fn centres_are_the_expressions() {
    let f = cases();
    for c in f["centers"].as_array().expect("centers") {
        let name = c["shape"].as_str().expect("shape");
        let g = geometry_of(&shape(&f["shapes"][name])).expect("a geometry");
        let p = g.center.expect("a centre");
        let want = (
            c["center"][0].as_f64().expect("x"),
            c["center"][1].as_f64().expect("y"),
        );
        assert!(
            (p.x - want.0).abs() <= 1e-9 && (p.y - want.1).abs() <= 1e-9,
            "{name}: {p:?}, beklenen {want:?}"
        );
    }
}

#[test]
fn the_store_pairs_objects_by_the_relations() {
    let f = cases();
    let names: Vec<&str> = f["shapes"]
        .as_object()
        .expect("shapes")
        .keys()
        .map(String::as_str)
        .collect();
    let mut store = Store::new();
    for (i, name) in names.iter().enumerate() {
        store.put(i as f64 + 1.0, "a", false, shape(&f["shapes"][*name]));
    }
    let id = |n: &str| names.iter().position(|m| *m == n).expect("a shape") as f64 + 1.0;
    for relation in [
        Relation::Intersects,
        Relation::Contains,
        Relation::Within,
        Relation::CenterIn,
    ] {
        let key = match relation {
            Relation::Intersects => "intersects",
            Relation::Contains => "contains",
            Relation::Within => "within",
            _ => "centerIn",
        };
        for r in f["relations"].as_array().expect("relations") {
            let (a, b) = (r["a"].as_str().expect("a"), r["b"].as_str().expect("b"));
            if a == b {
                continue;
            }
            let pairs = store.relate_pairs(&[id(a)], &[id(b)], relation, 0.0);
            assert_eq!(!pairs.is_empty(), r[key], "{a} › {b}: {key}");
        }
    }
    // Uzaklıkta: the distance itself counts.
    for r in f["relations"].as_array().expect("relations") {
        let (a, b) = (r["a"].as_str().expect("a"), r["b"].as_str().expect("b"));
        let d = r["distance"].as_f64().expect("a distance");
        assert!(
            !store
                .relate_pairs(&[id(a)], &[id(b)], Relation::Near, d)
                .is_empty(),
            "{a} › {b}: {d} m"
        );
        if d > 0.01 {
            assert!(
                store
                    .relate_pairs(&[id(a)], &[id(b)], Relation::Near, d - 0.01)
                    .is_empty(),
                "{a} › {b}: {d} − 1 cm"
            );
        }
    }
}

#[test]
fn numbers_and_figures_follow_the_rule() {
    let f = cases();
    for n in f["numbers"].as_array().expect("numbers") {
        let got = read_number(n["text"].as_str().expect("text")).map(Dec::text);
        assert_eq!(json!(got), n["number"], "{}", n["text"]);
    }
    for c in f["figures"].as_array().expect("figures") {
        let values: Vec<Option<String>> =
            serde_json::from_value(c["values"].clone()).expect("values");
        let values: Vec<Option<&str>> = values.iter().map(|v| v.as_deref()).collect();
        let scale = c["scale"].as_u64().map(|s| s as u32);
        let got = figures(&values, scale).expect("figures");
        let text = |d: Option<Dec>| json!(d.map(Dec::text));
        let row = json!({
            "read": got.read,
            "skipped": got.skipped,
            "sum": text(got.sum),
            "mean": text(got.mean),
            "min": text(got.min),
            "max": text(got.max),
            "std": got.std,
        });
        assert_eq!(row, c["expect"], "{}", c["values"]);
    }
    for c in f["statistics"].as_array().expect("statistics") {
        let args = json!([[c["values"]], c["stat"], [c["scale"]]]).to_string();
        let got: Value =
            serde_json::from_str(&run_named("statisticMany", &args).expect("statisticMany"))
                .expect("JSON");
        assert_eq!(got[0], c["expect"], "{} {}", c["stat"], c["values"]);
    }
    for c in f["summaries"].as_array().expect("summaries") {
        let args = json!([c["groups"], c["values"], c["grouped"]]).to_string();
        let got: Value =
            serde_json::from_str(&run_named("summarizeValues", &args).expect("summarizeValues"))
                .expect("JSON");
        assert_eq!(got, c["expect"], "{}", c["values"]);
    }
}

#[test]
fn keys_and_joins_meet_as_the_reference_says() {
    let f = cases();
    for k in f["keys"].as_array().expect("keys") {
        assert_eq!(
            json!(join_key(k["text"].as_str().expect("text"))),
            k["key"],
            "{}",
            k["text"]
        );
    }
    for j in f["joins"].as_array().expect("joins") {
        let args = json!([j["targets"], j["sources"]]).to_string();
        let got: Value =
            serde_json::from_str(&run_named("joinPlan", &args).expect("joinPlan")).expect("JSON");
        assert_eq!(got, j["expect"], "{}", j["sources"]);
    }
}
