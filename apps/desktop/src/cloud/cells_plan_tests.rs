//! The status bar's cloud cells say and do what the web's do: every section
//! of fixtures/cloud/v1/cells.json (format in fixtures/cloud/README.md),
//! whose answers scripts/fixtures/cells_cases.py wrote apart from either
//! program.

use kentos_contracts::{Health, ProjectPermission};
use serde_json::Value;

use super::cells_plan::{
    self as plan, AccountRow, DatabaseState, FileSave, FileState, LinkState, SaveInput, ServerState,
};
use super::local_time::Zone;
use super::revisions::NewerRevision;

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/cloud/v1/cells.json"))
        .expect("cells.json reads")
}

fn text(v: &Value) -> &str {
    v.as_str().expect("a text")
}

fn database(state: &str) -> DatabaseState {
    match state {
        "saved" => DatabaseState::Saved,
        "pending" => DatabaseState::Pending,
        "saving" => DatabaseState::Saving,
        "offline_pending" => DatabaseState::OfflinePending,
        "conflict" => DatabaseState::Conflict,
        "error" => DatabaseState::Error,
        "readonly" => DatabaseState::ReadOnly,
        "deleted" => DatabaseState::Deleted,
        "revoked" => DatabaseState::Revoked,
        "archived" => DatabaseState::Archived,
        other => panic!("a save state: {other}"),
    }
}

fn file(state: &str) -> FileState {
    match state {
        "saved" => FileState::Saved,
        "pending" => FileState::Pending,
        "encoding" => FileState::Encoding,
        "uploading" => FileState::Uploading,
        "verifying" => FileState::Verifying,
        "conflict" => FileState::Conflict,
        "outdated" => FileState::Outdated,
        "error" => FileState::Error,
        "readonly" => FileState::ReadOnly,
        "deleted" => FileState::Deleted,
        "revoked" => FileState::Revoked,
        "archived" => FileState::Archived,
        other => panic!("a file save state: {other}"),
    }
}

fn link(v: &Value) -> LinkState {
    match text(v) {
        "none" => LinkState::None,
        "connecting" => LinkState::Connecting,
        "online" => LinkState::Online,
        "reconnecting" => LinkState::Reconnecting,
        "offline" => LinkState::Offline,
        "auth_required" => LinkState::AuthRequired,
        other => panic!("a link state: {other}"),
    }
}

fn server(v: &Value) -> ServerState {
    match text(v) {
        "checking" => ServerState::Checking,
        "online" => ServerState::Online,
        "offline" => ServerState::Offline,
        "incompatible" => ServerState::Incompatible,
        other => panic!("a server state: {other}"),
    }
}

fn count(v: &Value) -> usize {
    usize::try_from(v.as_u64().expect("a count")).expect("small")
}

fn input(v: &Value) -> SaveInput {
    match text(&v["kind"]) {
        "none" => SaveInput::None,
        "database" => SaveInput::Database {
            state: database(text(&v["state"])),
            pending: count(&v["pending"]),
            conflicts: count(&v["conflicts"]),
        },
        _ => SaveInput::File(FileSave {
            state: file(text(&v["state"])),
            base: text(&v["base"]).to_owned(),
            progress: v["progress"].as_f64().expect("progress"),
            conflict_actual: v["conflictActual"].as_str().map(str::to_owned),
            newer_revision: v["newerRevision"].as_str().map(str::to_owned),
            dirty: v["dirty"].as_bool().unwrap_or(false),
        }),
    }
}

#[test]
fn the_save_cell_says_and_does_what_the_webs_does() {
    let f = fixture();
    for c in f["saveTexts"].as_array().expect("texts") {
        assert_eq!(
            database(text(&c["state"])).text(count(&c["sample"])),
            text(&c["text"]),
            "{c}"
        );
    }
    for c in f["views"].as_array().expect("views") {
        let i = input(&c["input"]);
        let view = plan::save_cell_view(&i);
        let v = &c["view"];
        assert_eq!(view.hidden, v["hidden"].as_bool().expect("hidden"), "{c}");
        assert_eq!(view.text, text(&v["text"]), "{c}");
        assert_eq!(view.state, v["state"].as_str(), "{c}");
        assert_eq!(plan::save_cell_action(&i), c["action"].as_str(), "{c}");
    }
    for c in f["ago"].as_array().expect("ago") {
        assert_eq!(
            plan::ago(c["at"].as_i64(), c["now"].as_i64().expect("now")),
            text(&c["text"]),
            "{c}"
        );
    }
}

#[test]
fn the_tips_are_the_webs() {
    let f = fixture();
    let links = f["linkTexts"].as_object().expect("links");
    for (key, words) in links {
        assert_eq!(
            plan::link_text(link(&Value::from(key.as_str()))),
            text(words)
        );
    }
    for c in f["databaseTips"].as_array().expect("tips") {
        let i = &c["input"];
        let tip = plan::database_tip(&plan::DatabaseTip {
            state: database(text(&i["state"])),
            place: text(&i["where"]),
            last_saved: i["lastSaved"].as_i64(),
            now: i["now"].as_i64().expect("now"),
            link: link(&i["link"]),
            error: text(&i["error"]),
            durable_drafts: i["durableDrafts"].as_bool().expect("drafts"),
        });
        assert_eq!(tip.title, text(&c["expect"]["title"]));
        assert_eq!(tip.description, text(&c["expect"]["description"]), "{c}");
    }
    for c in f["fileTips"].as_array().expect("tips") {
        let i = &c["input"];
        let last = i["lastSaved"].as_object().map(|l| {
            (
                l["revision"].as_str().expect("revision"),
                l["at"].as_i64().expect("at"),
            )
        });
        let newer = i["newer"].as_object().map(|n| NewerRevision {
            revision: n["revision"].as_str().expect("revision").to_owned(),
            by: n["by"].as_str().expect("by").to_owned(),
            at: n.get("at").and_then(Value::as_str).map(str::to_owned),
        });
        let tip = plan::file_tip(&plan::FileTip {
            place: text(&i["where"]),
            base: text(&i["base"]),
            last_saved: last,
            now: i["now"].as_i64().expect("now"),
            newer: newer.as_ref(),
            zone: &Zone::fixed(3 * 3600),
            error: text(&i["error"]),
            link: link(&i["link"]),
            dirty: i["dirty"].as_bool().expect("dirty"),
        });
        assert_eq!(tip.title, text(&c["expect"]["title"]));
        assert_eq!(tip.description, text(&c["expect"]["description"]), "{c}");
    }
}

#[test]
fn the_server_cell_and_the_account_menu_are_the_webs() {
    let f = fixture();
    for (key, words) in f["serverTexts"].as_object().expect("texts") {
        assert_eq!(
            plan::server_text(server(&Value::from(key.as_str()))),
            text(words)
        );
    }
    for c in f["serverTips"].as_array().expect("tips") {
        let i = &c["input"];
        let health: Option<Health> = (!i["health"].is_null())
            .then(|| serde_json::from_value(i["health"].clone()).expect("a Health"));
        let tip = plan::server_tip(
            server(&i["state"]),
            health.as_ref(),
            text(&i["detail"]),
            i["dev"].as_bool().expect("dev"),
        );
        assert_eq!(tip.title, text(&c["expect"]["title"]));
        assert_eq!(tip.description, text(&c["expect"]["description"]), "{c}");
    }
    let actions: Vec<(String, String)> = f["projectActions"]
        .as_array()
        .expect("actions")
        .iter()
        .map(|a| {
            (
                text(&a["command"]).to_owned(),
                text(&a["permission"]).to_owned(),
            )
        })
        .collect();
    let ours: Vec<(String, String)> = plan::PROJECT_ACTIONS
        .iter()
        .map(|(c, p)| ((*c).to_owned(), p.name().to_owned()))
        .collect();
    assert_eq!(ours, actions);
    for c in f["accountRows"].as_array().expect("rows") {
        let i = &c["input"];
        let list = |key: &str| -> Vec<String> {
            i[key]
                .as_array()
                .expect("a list")
                .iter()
                .map(|v| text(v).to_owned())
                .collect()
        };
        let (disabled, denied) = (list("disabled"), list("denied"));
        let rows = plan::account_rows(
            i["user"].as_str(),
            i["project"]["tenantName"].as_str(),
            |id| disabled.iter().any(|d| d == id),
            |p: ProjectPermission| !denied.iter().any(|d| d == p.name()),
        );
        let expected: Vec<AccountRow> = c["rows"]
            .as_array()
            .expect("rows")
            .iter()
            .map(|r| match r["kind"].as_str() {
                Some("header") => AccountRow::Header(text(&r["label"]).to_owned()),
                Some("separator") => AccountRow::Separator,
                _ => {
                    let id = text(&r["command"]);
                    let command = plan::PROJECT_ACTIONS
                        .iter()
                        .map(|(c, _)| *c)
                        .chain([
                            "cloud.signIn",
                            "cloud.signOut",
                            "cloud.open",
                            "cloud.upload",
                            "cloud.uploadFile",
                            "server.check",
                        ])
                        .find(|c| *c == id)
                        .expect("a known command");
                    AccountRow::Command {
                        command,
                        detail: r["detail"].as_str().map(str::to_owned),
                    }
                }
            })
            .collect();
        assert_eq!(rows, expected, "{}", c["title"]);
    }
}
