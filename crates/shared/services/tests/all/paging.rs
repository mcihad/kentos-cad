//! How a data service take ends and what an OGC API service says it can do (docs/adr/0513), against
//! `fixtures/services/v1/paging.json` (`scripts/fixtures/feed_paging_cases.py`: the rule written without KentOS code).
//! Each take plays the same simulated service against KentOS's client: the requests it makes, the objects it takes
//! and receives and how it says the take ended.

use kentos_contracts::{FeatureFeed, FeedKind};
use kentos_services::feed::{Ending, Taking};
use kentos_services::profile::FeedProfile;
use kentos_services::query::param;
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = format!(
        "{}/../../../fixtures/services/v1/paging.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.feed-paging");
    assert_eq!(file["version"], 1);
    file
}

#[test]
fn a_conformance_page_reads_as_its_reference() {
    let file = fixture();
    for case in file["profiles"].as_array().expect("profiles") {
        let name = case["name"].as_str().expect("name");
        let page = case["page"].as_str();
        let got = page.and_then(FeedProfile::of_page).unwrap_or_default();
        let want = &case["profile"];
        let mut got_json = serde_json::to_value(&got).expect("serialises");
        got_json["filters"] = json!(got.filters());
        for (k, v) in want.as_object().expect("an object") {
            assert_eq!(&got_json[k], v, "{name}: {k}");
        }
        assert_eq!(got.filter_problem().is_none(), got.filters(), "{name}");
    }
}

/// The simulated service's answer to a request.
fn answer(service: &Value, url: &str) -> String {
    let kind = service["kind"].as_str().expect("kind");
    let total = service["total"].as_u64().expect("total");
    let cap = service["cap"].as_u64().unwrap_or(u64::MAX);
    let version = service["version"].as_str().unwrap_or("2.0.0");
    let wfs1 = kind == "wfs" && !version.starts_with('2');
    let number = |name: &str| param(url, name).and_then(|v| v.parse::<u64>().ok());
    let (start, asked) = match kind {
        "wfs" => (
            if wfs1 {
                0
            } else {
                number("STARTINDEX").unwrap_or(0)
            },
            number("COUNT")
                .or_else(|| number("MAXFEATURES"))
                .expect("a count"),
        ),
        "ogcFeatures" => (
            number("offset").unwrap_or(0),
            number("limit").expect("a limit"),
        ),
        _ => (
            number("resultOffset").unwrap_or(0),
            number("resultRecordCount").expect("a count"),
        ),
    };
    let given = asked.min(cap).min(total.saturating_sub(start));
    let features: Vec<Value> = (0..given)
        .map(|i| {
            json!({ "type": "Feature", "properties": { "n": start + i },
                    "geometry": { "type": "Point", "coordinates": [32.0, 39.0] } })
        })
        .collect();
    let mut body = json!({ "type": "FeatureCollection", "features": features });
    let more = start + given < total;
    if service["matched"].as_bool().unwrap_or(true) && !wfs1 {
        body["numberMatched"] = json!(total);
    }
    match kind {
        "ogcFeatures" if more && service["nextLinks"].as_bool().unwrap_or(true) => {
            body["links"] = json!([{ "rel": "next",
                "href": format!("https://ornek.org/ogc/collections/c/items?offset={}&limit={asked}", start + given) }]);
        }
        "arcgis" if more => body["exceededTransferLimit"] = json!(true),
        _ => {}
    }
    body.to_string()
}

fn feed_of(service: &Value) -> FeatureFeed {
    let kind = match service["kind"].as_str().expect("kind") {
        "wfs" => FeedKind::Wfs,
        "ogcFeatures" => FeedKind::OgcFeatures,
        _ => FeedKind::Arcgis,
    };
    let url = match kind {
        FeedKind::Wfs => "https://ornek.org/wfs",
        FeedKind::OgcFeatures => "https://ornek.org/ogc/collections/c/items",
        _ => "https://ornek.org/arcgis/rest/services/S/FeatureServer/0",
    };
    serde_json::from_value(json!({
        "kind": serde_json::to_value(kind).expect("kind"),
        "url": url,
        "name": "c",
        "srid": 4326,
        "version": (kind == FeedKind::Wfs).then(|| service["version"].clone()),
    }))
    .expect("a feed")
}

#[test]
fn a_take_ends_as_its_reference() {
    let file = fixture();
    for case in file["takes"].as_array().expect("takes") {
        let name = case["name"].as_str().expect("name");
        let service = &case["service"];
        let most = u32::try_from(case["most"].as_u64().expect("most")).expect("small");
        let feed = feed_of(service);
        let (mut taking, first) = Taking::start(&feed, None, most, true);
        let mut next = Some(first);
        let mut requests = 0u64;
        while let Some(req) = next {
            requests += 1;
            assert!(requests <= 2000, "{name}: runaway");
            let (_, more) = taking
                .answer(&answer(service, &req.url), "")
                .expect("reads");
            next = more;
        }
        let want = &case["expect"];
        assert_eq!(
            requests,
            want["requests"].as_u64().expect("requests"),
            "{name}: requests"
        );
        assert_eq!(
            taking.taken,
            want["taken"].as_u64().expect("taken"),
            "{name}: taken"
        );
        assert_eq!(
            taking.received(),
            want["received"].as_u64().expect("received"),
            "{name}: received"
        );
        assert_eq!(taking.matched, want["matched"].as_u64(), "{name}: matched");
        let ending =
            Ending::from_name(want["ending"].as_str().expect("ending")).expect("an ending");
        assert_eq!(taking.ending(), Some(ending), "{name}: ending");
    }
}
