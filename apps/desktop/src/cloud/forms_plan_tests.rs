//! The project forms say and decide what the web's do: every section of
//! fixtures/cloud/v1/forms.json (format in fixtures/cloud/README.md), whose
//! answers scripts/fixtures/forms_cases.py wrote apart from either program.

use kentos_cloud::ApiFailure;
use kentos_contracts::{MembershipView, ProjectStorage, ProjectType};
use serde_json::Value;

use super::forms_plan::{self as plan, Metadata, convert, duplicate, fields, metadata, rename};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/cloud/v1/forms.json"))
        .expect("forms.json reads")
}

fn text(v: &Value) -> &str {
    v.as_str().expect("a text")
}

/// A text made from values: `{ sample: [..], text }`.
fn sample(v: &Value) -> (Vec<&str>, &str) {
    let values = v["sample"]
        .as_array()
        .expect("its sample")
        .iter()
        .map(text)
        .collect();
    (values, text(&v["text"]))
}

fn shown(v: &Value) -> Metadata {
    Metadata {
        name: text(&v["name"]).to_owned(),
        project_type: serde_json::from_value::<ProjectType>(v["projectType"].clone())
            .expect("a type"),
        description: text(&v["description"]).to_owned(),
        tags: v["tags"]
            .as_array()
            .expect("tags")
            .iter()
            .map(|t| text(t).to_owned())
            .collect(),
    }
}

/// A failure as the fixture describes the web's: the server's (`api`), the
/// page's own (`error`) or something that is not an error at all (`other`).
fn failure(e: &Value) -> ApiFailure {
    match text(&e["kind"]) {
        "api" => {
            let status = u16::try_from(e["status"].as_u64().expect("a status")).expect("small");
            let failure = if status == 0 {
                ApiFailure::new(0, "network", text(&e["fallback"]))
            } else {
                ApiFailure::new(status, text(&e["error"]), text(&e["message"]))
            };
            match e["path"].as_str() {
                Some(path) => failure.with_path(path),
                None => failure,
            }
        }
        "error" => ApiFailure::new(0, "local", text(&e["message"])),
        _ => ApiFailure::new(0, "local", ""),
    }
}

#[test]
fn the_forms_words_are_the_webs() {
    let f = fixture();
    let t = &f["texts"];
    assert_eq!(text(&t["cancel"]), plan::CANCEL);
    assert_eq!(text(&t["saving"]), plan::SAVING);
    assert_eq!(text(&t["place"]), plan::PLACE);
    assert_eq!(text(&t["noPlace"]), plan::NO_PLACE);
    let words = [
        (&t["fields"]["type"], fields::TYPE),
        (&t["fields"]["typeLabel"], fields::TYPE_LABEL),
        (&t["fields"]["description"], fields::DESCRIPTION),
        (
            &t["fields"]["descriptionPlaceholder"],
            fields::DESCRIPTION_PLACEHOLDER,
        ),
        (&t["fields"]["tags"], fields::TAGS),
        (&t["fields"]["tagsPlaceholder"], fields::TAGS_PLACEHOLDER),
        (&t["metadata"]["title"], metadata::TITLE),
        (&t["metadata"]["name"], metadata::NAME),
        (&t["metadata"]["save"], metadata::SAVE),
        (&t["rename"]["title"], rename::TITLE),
        (&t["rename"]["name"], rename::NAME),
        (&t["rename"]["hint"], rename::HINT),
        (&t["rename"]["save"], rename::SAVE),
        (&t["duplicate"]["title"], duplicate::TITLE),
        (&t["duplicate"]["name"], duplicate::NAME),
        (&t["duplicate"]["nameLabel"], duplicate::NAME_LABEL),
        (&t["duplicate"]["placeLabel"], duplicate::PLACE_LABEL),
        (&t["duplicate"]["make"], duplicate::MAKE),
        (&t["duplicate"]["running"], duplicate::RUNNING),
        (&t["convert"]["name"], convert::NAME),
        (&t["convert"]["nameLabel"], convert::NAME_LABEL),
        (&t["convert"]["placeLabel"], convert::PLACE_LABEL),
    ];
    for (web, desktop) in words {
        assert_eq!(text(web), desktop);
    }
    let consequences: Vec<&str> = t["duplicate"]["consequences"]
        .as_array()
        .expect("consequences")
        .iter()
        .map(text)
        .collect();
    assert_eq!(consequences, duplicate::CONSEQUENCES);
    type Made = (fn(&str) -> String, &'static str);
    let one: [(Made, &Value); 4] = [
        ((metadata::saved, "metadata.saved"), &t["metadata"]["saved"]),
        ((rename::saved, "rename.saved"), &t["rename"]["saved"]),
        ((rename::waiting, "rename.waiting"), &t["rename"]["waiting"]),
        ((duplicate::lead, "duplicate.lead"), &t["duplicate"]["lead"]),
    ];
    for ((make, what), v) in one {
        let (values, expected) = sample(v);
        assert_eq!(make(values[0]), expected, "{what}");
    }
    let (values, expected) = sample(&t["duplicate"]["done"]);
    assert_eq!(duplicate::done(values[0], values[1]), expected);
    assert_eq!(f["limits"]["name"], plan::NAME_MAX);
    assert_eq!(f["limits"]["description"], plan::DESCRIPTION_MAX);
}

#[test]
fn tags_copies_and_counts_are_the_webs() {
    let f = fixture();
    for c in f["tags"].as_array().expect("tags") {
        let expected: Vec<&str> = c["tags"]
            .as_array()
            .expect("its tags")
            .iter()
            .map(text)
            .collect();
        assert_eq!(plan::parse_tags(text(&c["text"])), expected, "{c}");
    }
    for c in f["copyNames"].as_array().expect("copies") {
        assert_eq!(plan::copy_name(text(&c["name"])), text(&c["copy"]));
    }
    for c in f["counts"].as_array().expect("counts") {
        assert_eq!(
            plan::count_text(text(&c["objects"])),
            text(&c["text"]),
            "{c}"
        );
    }
    for c in f["convertedLines"].as_array().expect("lines") {
        let to: ProjectStorage = serde_json::from_value(c["to"].clone()).expect("a mode");
        assert_eq!(
            plan::converted_line(text(&c["name"]), text(&c["objects"]), to),
            text(&c["text"])
        );
    }
}

#[test]
fn what_the_forms_send_and_when_their_buttons_are_on_is_the_webs() {
    let f = fixture();
    let before = shown(&f["metadata"]["shown"]);
    for c in f["metadata"]["cases"].as_array().expect("cases") {
        let now = shown(&c["now"]);
        let patch = plan::metadata_patch(&before, &now);
        assert_eq!(
            serde_json::to_value(&patch).expect("json"),
            c["patch"],
            "{}",
            c["title"]
        );
        assert_eq!(
            plan::metadata_savable(&now.name, &patch),
            c["savable"].as_bool().expect("savable"),
            "{}",
            c["title"]
        );
    }
    for c in f["rename"].as_array().expect("rename") {
        assert_eq!(
            plan::rename_savable(text(&c["typed"]), text(&c["current"])),
            c["savable"].as_bool().expect("savable"),
            "{c}"
        );
    }
    for c in f["duplicate"].as_array().expect("duplicate") {
        let places = usize::try_from(c["places"].as_u64().expect("places")).expect("small");
        assert_eq!(
            plan::duplicate_savable(places, text(&c["name"])),
            c["savable"].as_bool().expect("savable"),
            "{c}"
        );
    }
}

#[test]
fn the_workspaces_and_the_conversion_are_the_webs() {
    let f = fixture();
    let memberships: Vec<MembershipView> =
        serde_json::from_value(f["places"]["memberships"].clone()).expect("memberships");
    for c in f["places"]["cases"].as_array().expect("cases") {
        let places: Vec<(String, String)> = plan::creatable_places(&memberships, text(&c["first"]))
            .into_iter()
            .map(|p| (p.tenant_id, p.label))
            .collect();
        let expected: Vec<(String, String)> = c["places"]
            .as_array()
            .expect("places")
            .iter()
            .map(|p| {
                (
                    text(&p["tenantId"]).to_owned(),
                    text(&p["label"]).to_owned(),
                )
            })
            .collect();
        assert_eq!(places, expected, "{}", c["first"]);
    }
    for c in f["convert"].as_array().expect("convert") {
        let p = &c["project"];
        let storage: ProjectStorage = serde_json::from_value(p["storage"].clone()).expect("a mode");
        let form = plan::convert_form(
            text(&p["name"]),
            storage,
            c["openDirty"].as_bool().expect("dirty"),
        );
        let e = &c["expect"];
        let to: ProjectStorage = serde_json::from_value(e["to"].clone()).expect("a mode");
        assert_eq!(form.to, to, "{}", c["title"]);
        assert_eq!(form.title, text(&e["title"]));
        assert_eq!(form.lead, text(&e["lead"]));
        let consequences: Vec<&str> = e["consequences"]
            .as_array()
            .expect("consequences")
            .iter()
            .map(text)
            .collect();
        assert_eq!(form.consequences, consequences, "{}", c["title"]);
        assert_eq!(form.placeholder, text(&e["placeholder"]));
        assert_eq!(form.running, text(&e["running"]));
    }
}

#[test]
fn failures_are_said_as_the_web() {
    let f = fixture();
    for c in f["failures"].as_array().expect("failures") {
        assert_eq!(
            plan::failure_reason(&failure(&c["error"])),
            text(&c["text"]),
            "{}",
            c["title"]
        );
    }
    for c in f["convertFailures"].as_array().expect("failures") {
        assert_eq!(
            plan::convert_reason(&failure(&c["error"])),
            text(&c["text"]),
            "{}",
            c["title"]
        );
    }
}
