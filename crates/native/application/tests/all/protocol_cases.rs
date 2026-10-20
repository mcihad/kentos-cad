//! The protocol's shared cases (docs/adr/0253): fixtures/protocol/v1/cases.json,
//! written by scripts/fixtures/protocol_cases.py without KentOS code. The
//! canonical text and its floats, the drawing's stamps, the requests'
//! fingerprints, the plans' ids and summaries, and the answers to faulty
//! envelopes, to expectations and to requests sent again, through the
//! protocol's own call (`call::call`).

use std::collections::BTreeMap;

use kentos_commands::canonical::{canonical_json, id_of, stamp_of};
use kentos_commands::{BlockId, CallAnswer, CallEnvelope, DocumentSnapshotV2, uuid_from_text};
use kentos_domain::{Document, Uuid};
use kentos_native_application::call::call;
use kentos_native_application::registry;
use serde_json::{Value, json};

const CASES: &str = include_str!("../../../../../fixtures/protocol/v1/cases.json");

fn cases() -> Value {
    serde_json::from_str(CASES).expect("the cases read")
}

/// The cases' drawing, opened from its snapshot. The snapshot must read
/// back as written: the values the reference hashed are the values hashed here.
fn drawing(cases: &Value) -> Document {
    let snapshot: DocumentSnapshotV2 =
        serde_json::from_value(cases["snapshot"].clone()).expect("the snapshot reads");
    let back = serde_json::to_value(&snapshot).expect("writes");
    assert_eq!(
        back, cases["snapshot"],
        "the snapshot is not written in full: the reference must write what the contract writes"
    );
    Document::from_snapshot_v2(snapshot).expect("opens")
}

fn text(v: &Value) -> &str {
    v.as_str().unwrap_or_else(|| panic!("not a text: {v}"))
}

fn envelope(v: &Value, kept: &BTreeMap<String, String>) -> CallEnvelope {
    let mut v = v.clone();
    if let Some(plan) = v.pointer_mut("/expected/plan")
        && let Some(name) = plan.as_str().and_then(|p| p.strip_prefix('$'))
    {
        *plan = Value::String(kept[name].clone());
    }
    serde_json::from_value(v).expect("an envelope")
}

#[test]
fn the_canonical_text_and_its_digests_are_the_reference_s() {
    let cases = cases();
    let list = cases["canonical"].as_array().expect("canonical");
    assert!(list.len() >= 10);
    for c in list {
        let name = text(&c["name"]);
        assert_eq!(canonical_json(&c["value"]), text(&c["text"]), "{name}");
        assert_eq!(stamp_of(&c["value"]), text(&c["stamp"]), "{name}");
        assert_eq!(id_of(&c["value"]), text(&c["id"]), "{name}");
    }
}

#[test]
fn floats_are_written_as_the_reference_writes_them() {
    let cases = cases();
    let list = cases["floats"].as_array().expect("floats");
    assert!(list.len() > 3000, "{} floats", list.len());
    let mut wrong = Vec::new();
    for f in list {
        let (bits, want) = text(f).split_once(' ').expect("bits and text");
        let x = f64::from_bits(u64::from_str_radix(bits, 16).expect("hex bits"));
        let got = canonical_json(&json!(x));
        if got != want {
            wrong.push(format!("{bits}: {got} ≠ {want}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {}:\n{}",
        wrong.len(),
        list.len(),
        wrong.join("\n")
    );
}

#[test]
fn the_drawing_has_the_reference_stamps() {
    let cases = cases();
    let doc = drawing(&cases);
    let stamps = &cases["stamps"];
    let objects = stamps["objects"].as_object().expect("objects");
    assert_eq!(objects.len(), doc.entities().count());
    for (uid, want) in objects {
        let u = Uuid::parse_str(uid).expect("a uid");
        assert_eq!(doc.stamp_of_uid(u).as_deref(), Some(text(want)), "{uid}");
    }
    for (id, want) in stamps["blocks"].as_object().expect("blocks") {
        let b = BlockId(uuid_from_text(id).expect("a block id"));
        assert_eq!(doc.block_stamp(b).as_deref(), Some(text(want)), "{id}");
    }
    assert_eq!(doc.layers_stamp(), text(&stamps["layers"]));
    assert_eq!(doc.settings_stamp(), text(&stamps["settings"]));
    assert_eq!(doc.styles_stamp(), text(&stamps["styles"]));
    assert_eq!(doc.sheets_stamp(), text(&stamps["sheets"]));
}

#[test]
fn a_request_s_fingerprint_is_the_reference_s() {
    let cases = cases();
    for c in cases["fingerprints"].as_array().expect("fingerprints") {
        let name = text(&c["name"]);
        let e = &c["envelope"];
        let normalized = registry::normalize(text(&e["command"]), e["input"].clone())
            .unwrap_or_else(|u| panic!("{name}: {}", u.message));
        assert_eq!(normalized, c["normalized"], "{name}: the normalized input");
        let mut doc = drawing(&cases);
        let answer = call(&mut doc, None, envelope(e, &BTreeMap::new()));
        assert_eq!(status(&answer), "completed", "{name}: {answer:?}");
        let id = text(&e["requestId"]);
        let record = doc
            .request(id)
            .unwrap_or_else(|| panic!("{name}: no record"));
        assert_eq!(record.fingerprint, text(&c["fingerprint"]), "{name}");
    }
}

#[test]
fn a_plan_has_the_reference_s_id_and_summary_and_is_written_as_planned() {
    let cases = cases();
    for c in cases["plans"].as_array().expect("plans") {
        let name = text(&c["name"]);
        let e = &c["envelope"];
        let normalized = registry::normalize(text(&e["command"]), e["input"].clone())
            .unwrap_or_else(|u| panic!("{name}: {}", u.message));
        assert_eq!(normalized, c["normalized"], "{name}: the normalized input");
        let mut doc = drawing(&cases);
        let answer = call(&mut doc, None, envelope(e, &BTreeMap::new()));
        let v = serde_json::to_value(&answer).expect("an answer");
        assert_eq!(v["status"], "completed", "{name}: {v}");
        assert_eq!(v["planId"], c["planId"], "{name}: the plan's id");
        // A new object's id differs at every write: the reference says `#aN`, by their order.
        let fresh: BTreeMap<String, String> = v["summary"]["added"]
            .as_array()
            .map(|a| {
                a.iter()
                    .enumerate()
                    .map(|(i, u)| (text(u).to_owned(), format!("#a{i}")))
                    .collect()
            })
            .unwrap_or_default();
        same_as(
            &renamed(&v["summary"], &fresh),
            &c["summary"],
            &format!("{name} › summary"),
        );
        // Written as planned: on another copy, the execute that expects the plan.
        let mut written = drawing(&cases);
        let mut execute = e.clone();
        execute["op"] = json!("execute");
        execute["expected"] = json!({ "plan": c["planId"] });
        let answer = call(&mut written, None, envelope(&execute, &BTreeMap::new()));
        let w = serde_json::to_value(&answer).expect("an answer");
        assert_eq!(w["status"], "completed", "{name}: {w}");
        assert_eq!(w["planId"], c["planId"], "{name}: the write's plan");
        // The stamps after are the objects' own now.
        for (uid, stamp) in w["summary"]["stamps"]["objects"]
            .as_object()
            .into_iter()
            .flatten()
        {
            let u = Uuid::parse_str(uid).expect("a uid");
            assert_eq!(
                written.stamp_of_uid(u).as_deref(),
                stamp.as_str(),
                "{name}: {uid}"
            );
        }
    }
}

/// `v` with every text and key that `names` maps, mapped.
fn renamed(v: &Value, names: &BTreeMap<String, String>) -> Value {
    match v {
        Value::String(s) => Value::String(names.get(s).cloned().unwrap_or_else(|| s.clone())),
        Value::Array(a) => Value::Array(a.iter().map(|x| renamed(x, names)).collect()),
        Value::Object(m) => Value::Object(
            m.iter()
                .map(|(k, x)| {
                    (
                        names.get(k).cloned().unwrap_or_else(|| k.clone()),
                        renamed(x, names),
                    )
                })
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Every key of `want` is in `got` with the same value (the summary's
/// parts the reference states; the rest is the host's). An empty list or
/// map is not written: absent is empty.
fn same_as(got: &Value, want: &Value, at: &str) {
    let want = want.as_object().expect("an object");
    for (k, w) in want {
        let g = &got[k];
        let empty = match w {
            Value::Object(m) => m.is_empty(),
            Value::Array(a) => a.is_empty(),
            _ => false,
        };
        match (k.as_str(), w) {
            _ if empty && g.is_null() => {}
            // A new object's stamp holds its id, which every write makes anew.
            (_, Value::String(n)) if n == "new" => {
                let stamp = g.as_str().unwrap_or("");
                assert!(
                    stamp.len() == 16 && stamp.bytes().all(|b| b.is_ascii_hexdigit()),
                    "{at}.{k}: {g}"
                );
            }
            ("objects", Value::Object(_)) => {
                let got = g.as_object().unwrap_or_else(|| panic!("{at}.{k}: {g}"));
                let keys =
                    |m: &serde_json::Map<String, Value>| m.keys().cloned().collect::<Vec<_>>();
                assert_eq!(keys(got), keys(w.as_object().expect("a map")), "{at}.{k}");
                same_as(g, w, &format!("{at}.{k}"));
            }
            ("sources" | "stamps", Value::Object(_)) => same_as(g, w, &format!("{at}.{k}")),
            _ => assert_eq!(g, w, "{at}.{k}"),
        }
    }
}

fn status(answer: &CallAnswer) -> String {
    serde_json::to_value(answer).expect("an answer")["status"]
        .as_str()
        .unwrap_or("?")
        .to_owned()
}

/// Checks an answer against what a case expects of it.
fn expect(
    doc: &Document,
    answer: &CallAnswer,
    want: &Value,
    kept: &BTreeMap<String, String>,
    outputs: &[Value],
    at: &str,
) {
    let v = serde_json::to_value(answer).expect("an answer");
    assert_eq!(v["status"], want["status"], "{at}: {v}");
    if let Some(code) = want.get("code") {
        assert_eq!(v["error"]["code"], *code, "{at}: {v}");
    }
    if let Some(path) = want.get("path") {
        assert_eq!(v["error"]["path"], *path, "{at}: {v}");
    }
    if let Some(r) = want.get("retryable") {
        assert_eq!(
            v["error"]["retryable"].as_bool().unwrap_or(false),
            r.as_bool().unwrap_or(false),
            "{at}: {v}"
        );
    }
    if let Some(uids) = want.get("uids") {
        let got = v["error"].get("uids").cloned().unwrap_or(json!([]));
        assert_eq!(got, *uids, "{at}: {v}");
    }
    for flag in ["replayed", "undone"] {
        if let Some(w) = want.get(flag) {
            assert_eq!(
                v[flag].as_bool().unwrap_or(false),
                w.as_bool().unwrap_or(false),
                "{at}: {flag}: {v}"
            );
        }
    }
    if let Some(plan) = want.get("planId") {
        let name = text(plan).trim_start_matches('$');
        assert_eq!(v["planId"].as_str(), Some(kept[name].as_str()), "{at}: {v}");
    }
    if let Some(i) = want.get("sameOutputAs") {
        let i = usize::try_from(i.as_u64().expect("an index")).expect("an index");
        assert_eq!(v["output"], outputs[i], "{at}: the first answer's output");
    }
    if let Some(n) = want.get("count") {
        assert_eq!(
            Some(doc.entities().count() as u64),
            n.as_u64(),
            "{at}: objects"
        );
    }
}

#[test]
fn faulty_envelopes_are_refused_with_their_path() {
    let cases = cases();
    for c in cases["envelopes"].as_array().expect("envelopes") {
        let name = text(&c["name"]);
        let mut doc = drawing(&cases);
        let before = doc.revision();
        let answer = call(&mut doc, None, envelope(&c["envelope"], &BTreeMap::new()));
        expect(&doc, &answer, &c["expect"], &BTreeMap::new(), &[], name);
        assert_eq!(doc.revision(), before, "{name}: nothing written");
    }
}

#[test]
fn expectations_hold_or_nothing_is_written() {
    let cases = cases();
    for c in cases["expectations"].as_array().expect("expectations") {
        let name = text(&c["name"]);
        let mut doc = drawing(&cases);
        let before = doc.revision();
        let answer = call(&mut doc, None, envelope(&c["envelope"], &BTreeMap::new()));
        expect(&doc, &answer, &c["expect"], &BTreeMap::new(), &[], name);
        if c["expect"]["status"] != "completed" {
            assert_eq!(doc.revision(), before, "{name}: nothing written");
        }
    }
}

#[test]
fn requests_plans_and_undo_answer_as_the_reference_says() {
    let cases = cases();
    for c in cases["requests"].as_array().expect("requests") {
        let name = text(&c["name"]);
        let mut doc = drawing(&cases);
        let mut kept = BTreeMap::new();
        let mut outputs = Vec::new();
        for (i, step) in c["steps"].as_array().expect("steps").iter().enumerate() {
            let at = format!("{name} › {}", i + 1);
            if let Some(label) = step.get("undo") {
                assert_eq!(doc.undo().as_deref(), label.as_str(), "{at}: undo");
                if let Some(n) = step["expect"].get("count") {
                    assert_eq!(Some(doc.entities().count() as u64), n.as_u64(), "{at}");
                }
                outputs.push(Value::Null);
                continue;
            }
            let answer = call(&mut doc, None, envelope(&step["call"], &kept));
            expect(&doc, &answer, &step["expect"], &kept, &outputs, &at);
            let v = serde_json::to_value(&answer).expect("an answer");
            if let Some(name) = step.get("keepPlan") {
                kept.insert(text(name).to_owned(), text(&v["planId"]).to_owned());
            }
            outputs.push(v["output"].clone());
        }
    }
}
