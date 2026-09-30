//! An open file project's revisions follow the web's: every section of
//! fixtures/cloud/v1/file-revisions.json (format in fixtures/cloud/README.md),
//! whose answers scripts/fixtures/file_revisions_cases.py wrote apart from
//! either program.

use kentos_contracts::{EventRecord, FileRevisions, ProjectState};
use serde_json::Value;

use super::cells_plan::{self as cells, FileSave, SaveInput};
use super::local_time::Zone;
use super::revisions::{
    self as plan, Ended, EventsRead, NewerRevision, Offer, ProjectAnswer, ResyncStep,
    RevisionConflict, RevisionInput, RevisionState, SaveStage, SaveStep, Tone, Via,
};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../../fixtures/cloud/v1/file-revisions.json"
    ))
    .expect("file-revisions.json reads")
}

/// The fixture's zone: Europe/Istanbul, three hours east all year since 2016.
fn zone() -> Zone {
    Zone::fixed(3 * 3600)
}

fn text(v: &Value) -> &str {
    v.as_str().expect("a text")
}

fn newer(v: &Value) -> Option<NewerRevision> {
    v.as_object().map(|n| NewerRevision {
        revision: text(&n["revision"]).to_owned(),
        by: n["by"].as_str().unwrap_or_default().to_owned(),
        at: n.get("at").and_then(Value::as_str).map(str::to_owned),
    })
}

fn stage(v: &Value) -> SaveStage {
    match text(v) {
        "idle" => SaveStage::Idle,
        "encoding" => SaveStage::Encoding,
        "uploading" => SaveStage::Uploading,
        "verifying" => SaveStage::Verifying,
        other => panic!("a stage: {other}"),
    }
}

fn ended(v: &Value) -> Ended {
    match text(v) {
        "deleted" => Ended::Deleted,
        "revoked" => Ended::Revoked,
        "archived" => Ended::Archived,
        other => panic!("an end: {other}"),
    }
}

fn state(v: &Value) -> RevisionState {
    RevisionState {
        base: text(&v["base"]).to_owned(),
        newer: newer(&v["newer"]),
        conflict: v["conflict"].as_object().map(|c| RevisionConflict {
            expected: text(&c["expected"]).to_owned(),
            actual: text(&c["actual"]).to_owned(),
        }),
        dirty: v["dirty"].as_bool().expect("dirty"),
        stage: stage(&v["stage"]),
        failed: v["failed"].as_bool().expect("failed"),
        writable: v["writable"].as_bool().expect("writable"),
        ended: (!v["ended"].is_null()).then(|| ended(&v["ended"])),
    }
}

fn input(v: &Value) -> RevisionInput {
    match text(&v["kind"]) {
        "dirty" => RevisionInput::Dirty(v["dirty"].as_bool().expect("dirty")),
        "newest" => RevisionInput::Newest(newer(&v["newest"])),
        "stage" => RevisionInput::Stage(stage(&v["stage"])),
        "committed" => RevisionInput::Committed {
            revision: text(&v["revision"]).to_owned(),
            dirty: v["dirty"].as_bool().expect("dirty"),
        },
        "refused" => RevisionInput::Refused {
            actual: text(&v["actual"]).to_owned(),
        },
        "failed" => RevisionInput::Failed,
        "unchanged" => RevisionInput::Unchanged,
        "access" => RevisionInput::Access(v["writable"].as_bool().expect("writable")),
        "ended" => RevisionInput::Ended(ended(&v["why"])),
        other => panic!("an input: {other}"),
    }
}

fn save_step_name(s: SaveStep) -> &'static str {
    match s {
        SaveStep::Ended => "ended",
        SaveStep::ReadOnly => "readonly",
        SaveStep::Conflict => "conflict",
        SaveStep::Unchanged => "unchanged",
        SaveStep::Behind => "behind",
        SaveStep::Save => "save",
    }
}

fn via(v: &Value) -> Via {
    match text(v) {
        "newest" => Via::Newest,
        "conflict" => Via::Conflict,
        other => panic!("a way: {other}"),
    }
}

/// The save cell as the status bar reads it from a state (no upload under way: progress 0).
fn cell_of(s: &RevisionState) -> Value {
    let input = SaveInput::File(FileSave {
        state: plan::cell_state(s),
        base: s.base.clone(),
        progress: 0.0,
        conflict_actual: s.conflict.as_ref().map(|c| c.actual.clone()),
        newer_revision: s.newer.as_ref().map(|n| n.revision.clone()),
        dirty: s.dirty,
    });
    let view = cells::save_cell_view(&input);
    serde_json::json!({
        "state": view.state,
        "text": view.text,
        "action": cells::save_cell_action(&input),
    })
}

/// A whole offer as the file writes it.
fn offer_json(o: &Offer) -> Value {
    match o {
        Offer::None => serde_json::json!({ "kind": "none" }),
        Offer::Say(tone, line) => serde_json::json!({
            "kind": "say",
            "tone": if *tone == Tone::Info { "info" } else { "warn" },
            "line": line,
        }),
        Offer::Ask(q) => {
            let answers: Vec<Value> = q
                .answers
                .iter()
                .map(|a| {
                    let mut v = serde_json::json!({
                        "value": a.value.name(),
                        "label": a.label,
                        "does": match a.does {
                            plan::AnswerDoes::Copy => "copy",
                            plan::AnswerDoes::Local => "local",
                            plan::AnswerDoes::Latest => "latest",
                            plan::AnswerDoes::Nothing => "nothing",
                        },
                        "work": match a.work {
                            plan::AnswerWork::Saved => "saved",
                            plan::AnswerWork::Dropped => "dropped",
                            plan::AnswerWork::Kept => "kept",
                            plan::AnswerWork::None => "none",
                        },
                    });
                    if let Some(kind) = a.kind {
                        v["kind"] = Value::from(match kind {
                            plan::AnswerKind::Primary => "primary",
                            plan::AnswerKind::Danger => "danger",
                        });
                    }
                    if a.aside {
                        v["aside"] = Value::Bool(true);
                    }
                    v
                })
                .collect();
            serde_json::json!({
                "kind": "ask",
                "question": {
                    "id": q.id.name(),
                    "title": q.title,
                    "message": q.message,
                    "details": q.details,
                    "answers": answers,
                    "cancel": q.cancel.name(),
                },
            })
        }
    }
}

/// An offer as a trace names it: its kind, and the question's id and answers, or the line.
fn offer_short(o: &Offer) -> Value {
    match o {
        Offer::Ask(q) => serde_json::json!({
            "kind": "ask",
            "id": q.id.name(),
            "answers": q.answers.iter().map(|a| a.value.name()).collect::<Vec<_>>(),
        }),
        Offer::Say(_, line) => serde_json::json!({ "kind": "say", "line": line }),
        Offer::None => serde_json::json!({ "kind": "none" }),
    }
}

#[test]
fn a_v1_file_with_the_words() {
    let f = fixture();
    assert_eq!(f["format"], "kentos.fileRevisions");
    assert_eq!(f["version"], 1);
    assert_eq!(f["timeZone"], "Europe/Istanbul");
    assert_eq!(
        f["resyncRetryMs"].as_u64(),
        u64::try_from(plan::RESYNC_RETRY.as_millis()).ok()
    );
    let t = &f["texts"];
    let sample = |k: &str, i: usize| text(&t[k]["sample"][i]).to_owned();
    assert_eq!(
        plan::texts::open_failed(&sample("openFailed", 0), &sample("openFailed", 1)),
        text(&t["openFailed"]["text"])
    );
    assert_eq!(
        plan::texts::busy(&sample("busy", 0)),
        text(&t["busy"]["text"])
    );
    assert_eq!(
        plan::texts::unreadable(Ended::Deleted, &sample("unreadableDeleted", 0)),
        text(&t["unreadableDeleted"]["text"])
    );
    assert_eq!(
        plan::texts::unreadable(Ended::Revoked, &sample("unreadableRevoked", 0)),
        text(&t["unreadableRevoked"]["text"])
    );
    assert_eq!(
        plan::texts::resync_failed(&sample("resyncFailed", 0)),
        text(&t["resyncFailed"]["text"])
    );
    assert_eq!(
        plan::texts::detached(&sample("detached", 0)),
        text(&t["detached"]["text"])
    );
    assert_eq!(f["marks"]["newest"], plan::marks::NEWEST);
    assert_eq!(f["marks"]["base"], plan::marks::BASE);
    for c in f["who"].as_array().expect("who") {
        let n = newer(&c["newer"]).expect("a revision");
        assert_eq!(plan::revision_who(&n, &zone()), text(&c["text"]), "{c}");
    }
}

#[test]
fn the_cell_follows_the_state() {
    let f = fixture();
    for c in f["cellStates"].as_array().expect("cell states") {
        assert_eq!(
            plan::cell_state(&state(&c["state"])).name(),
            text(&c["expect"]),
            "{}",
            c["title"]
        );
    }
    for c in f["cells"].as_array().expect("cells") {
        assert_eq!(cell_of(&state(&c["state"])), c["expect"], "{}", c["title"]);
    }
}

#[test]
fn inputs_change_the_state_as_the_webs_do() {
    let f = fixture();
    for c in f["steps"].as_array().expect("steps") {
        let (next, say) = plan::step(&state(&c["from"]), &input(&c["input"]));
        assert_eq!(next, state(&c["expect"]["state"]), "{}", c["title"]);
        assert_eq!(Some(say), c["expect"]["say"].as_bool(), "{}", c["title"]);
    }
    for c in f["saveSteps"].as_array().expect("save steps") {
        assert_eq!(
            save_step_name(plan::save_step(&state(&c["state"]))),
            text(&c["expect"]),
            "{}",
            c["title"]
        );
    }
}

#[test]
fn events_say_what_to_ask() {
    let f = fixture();
    for c in f["events"].as_array().expect("events") {
        let own: Vec<String> = c["own"]
            .as_array()
            .expect("own")
            .iter()
            .map(|v| text(v).to_owned())
            .collect();
        let events: Vec<EventRecord> = c["events"]
            .as_array()
            .expect("events")
            .iter()
            .map(|e| EventRecord {
                blocks: Vec::new(),
                seq: text(&e["seq"]).to_owned(),
                data_revision: "0".to_owned(),
                kind: text(&e["kind"]).to_owned(),
                actor: None,
                request_id: e
                    .get("requestId")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                features: Vec::new(),
                meta: false,
            })
            .collect();
        let read = plan::read_events(&events, |id| own.iter().any(|o| o == id));
        let x = &c["expect"];
        let expect = EventsRead {
            cursor: x["cursor"].as_str().map(str::to_owned),
            end: x["end"]
                .as_object()
                .map(|e| (ended(&e["why"]), e["quiet"].as_bool().expect("quiet"))),
            access: x["access"].as_bool().expect("access"),
            newest: x["newest"].as_bool().expect("newest"),
        };
        assert_eq!(read, expect, "{}", c["title"]);
    }
    for c in f["newest"].as_array().expect("newest") {
        let revisions: FileRevisions =
            serde_json::from_value(c["revisions"].clone()).expect("a FileRevisions");
        assert_eq!(
            plan::newest_of(&revisions),
            newer(&c["expect"]),
            "{}",
            c["title"]
        );
    }
}

#[test]
fn a_resync_asks_the_project_and_never_reopens_it() {
    let f = fixture();
    for c in f["resync"].as_array().expect("resync") {
        let a = &c["answer"];
        let answer = match text(&a["kind"]) {
            "project" => ProjectAnswer::Project {
                state: serde_json::from_value::<ProjectState>(a["state"].clone())
                    .expect("a ProjectState"),
                event_cursor: text(&a["eventCursor"]).to_owned(),
            },
            "deleted" => ProjectAnswer::Deleted,
            "notFound" => ProjectAnswer::NotFound,
            "forbidden" => ProjectAnswer::Forbidden(text(&a["message"]).to_owned()),
            "unreachable" => ProjectAnswer::Unreachable,
            "failed" => ProjectAnswer::Failed(text(&a["message"]).to_owned()),
            other => panic!("an answer: {other}"),
        };
        let x = &c["expect"];
        let expect = match text(&x["kind"]) {
            "end" => ResyncStep::End {
                why: ended(&x["why"]),
                reason: text(&x["reason"]).to_owned(),
            },
            "retry" => ResyncStep::Retry,
            "follow" => ResyncStep::Follow {
                cursor: text(&x["cursor"]).to_owned(),
            },
            other => panic!("a step: {other}"),
        };
        assert_eq!(plan::resync_step(&answer), expect, "{}", c["title"]);
    }
}

#[test]
fn the_questions_and_lines_are_the_webs() {
    let f = fixture();
    for c in f["offers"].as_array().expect("offers") {
        let i = &c["input"];
        let o = plan::offer(
            text(&i["name"]),
            &state(&i["state"]),
            i["busy"].as_bool().expect("busy"),
            via(&i["via"]),
            &zone(),
        );
        assert_eq!(offer_json(&o), c["expect"], "{}", c["title"]);
    }
    for c in f["lines"].as_array().expect("lines") {
        let n = newer(&c["newer"]).expect("a revision");
        assert_eq!(
            plan::newer_line(
                text(&c["name"]),
                &n,
                text(&c["base"]),
                c["dirty"].as_bool().expect("dirty"),
                &zone()
            ),
            text(&c["text"]),
            "{}",
            c["title"]
        );
    }
    for c in f["tips"].as_array().expect("tips") {
        let i = &c["input"];
        let last = i["lastSaved"].as_object().map(|l| {
            (
                l["revision"].as_str().expect("revision"),
                l["at"].as_i64().expect("at"),
            )
        });
        let n = newer(&i["newer"]);
        let tip = cells::file_tip(&cells::FileTip {
            place: text(&i["where"]),
            base: text(&i["base"]),
            last_saved: last,
            now: i["now"].as_i64().expect("now"),
            newer: n.as_ref(),
            zone: &zone(),
            error: text(&i["error"]),
            link: match text(&i["link"]) {
                "online" => cells::LinkState::Online,
                "offline" => cells::LinkState::Offline,
                "connecting" => cells::LinkState::Connecting,
                _ => cells::LinkState::None,
            },
            dirty: i["dirty"].as_bool().expect("dirty"),
        });
        assert_eq!(tip.title, text(&c["expect"]["title"]));
        assert_eq!(tip.description, text(&c["expect"]["description"]), "{i}");
    }
    for c in f["historyMarks"].as_array().expect("marks") {
        let marks = plan::revision_marks(
            text(&c["revision"]),
            c["current"].as_str(),
            c["openBase"].as_str(),
        );
        let expect: Vec<&str> = c["expect"]
            .as_array()
            .expect("expect")
            .iter()
            .map(text)
            .collect();
        assert_eq!(marks, expect, "{}", c["title"]);
    }
}

#[test]
fn whole_sequences_go_as_the_file_does() {
    let f = fixture();
    for t in f["traces"].as_array().expect("traces") {
        let mut s = state(&t["start"]);
        for (i, x) in t["steps"].as_array().expect("steps").iter().enumerate() {
            let at = format!("{}: {}", t["title"], i + 1);
            if !x["input"].is_null() {
                let (next, say) = plan::step(&s, &input(&x["input"]));
                s = next;
                assert_eq!(Some(say), x["say"].as_bool(), "{at}");
                assert_eq!(cell_of(&s), x["cell"], "{at}");
            } else if !x["save"].is_null() {
                let first = plan::save_step(&s);
                if first == SaveStep::Behind {
                    let actual = s
                        .newer
                        .as_ref()
                        .map(|n| n.revision.clone())
                        .unwrap_or_default();
                    s = plan::step(&s, &RevisionInput::Refused { actual }).0;
                } else if first == SaveStep::Unchanged {
                    s = plan::step(&s, &RevisionInput::Unchanged).0;
                }
                assert_eq!(save_step_name(first), text(&x["save"]), "{at}");
                assert_eq!(cell_of(&s), x["cell"], "{at}");
            } else {
                let o = plan::offer(
                    "Kadastro paftası 2026",
                    &s,
                    x["busy"].as_bool().expect("busy"),
                    via(&x["offer"]),
                    &zone(),
                );
                assert_eq!(offer_short(&o), x["expect"], "{at}");
            }
        }
        assert_eq!(s, state(&t["end"]), "{}", t["title"]);
    }
}
