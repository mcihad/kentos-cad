//! The Paylaş window's words and rules against fixtures/cloud/v1/share.json,
//! which the web's sharePlan.test.ts plays too (docs/adr/0111). Dates are
//! the device's time; the cases are in Europe/Istanbul (UTC+3 all year).

use kentos_cloud::ApiFailure;
use kentos_contracts::{
    AccessBlock, GrantRole, InvitationAccepted, InvitationChange, InvitationState,
    ProjectAccessHolder, ProjectAccessList, ProjectInvitation, ProjectRole, ProjectStorage,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::local_time::Zone;
use super::share_plan::*;
use super::words::epoch;

const FIXTURE: &str = include_str!("../../../../fixtures/cloud/v1/share.json");

/// A text made from one value, with the key of its sample in the fixture.
type Made = (fn(&str) -> String, &'static str);

fn fixture() -> Value {
    serde_json::from_str(FIXTURE).expect("the fixture reads")
}

fn zone() -> Zone {
    Zone::fixed(3 * 3600)
}

fn from<T: DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).unwrap_or_else(|e| panic!("{e}: {v}"))
}

fn text(v: &Value) -> &str {
    v.as_str().unwrap_or_else(|| panic!("a text: {v}"))
}

fn items(v: &Value) -> &Vec<Value> {
    v.as_array().unwrap_or_else(|| panic!("a list: {v}"))
}

fn role(v: &Value) -> ProjectRole {
    from(v)
}

fn grant(v: &Value) -> GrantRole {
    from(v)
}

#[test]
fn roles_blocks_storage_and_states_are_the_webs() {
    let f = fixture();
    let roles = &f["roles"];
    for (key, label) in roles["labels"].as_object().expect("labels") {
        assert_eq!(role_label(role(&json!(key))), text(label), "{key}");
    }
    let order: Vec<GrantRole> = from(&roles["grant"]);
    assert_eq!(order, GRANT_ROLES);
    let invite: Vec<GrantRole> = from(&roles["invite"]);
    assert_eq!(invite, INVITE_ROLES);
    for (key, hint) in roles["hints"].as_object().expect("hints") {
        assert_eq!(role_hint(grant(&json!(key))), text(hint), "{key}");
    }
    for (key, t) in f["blocks"].as_object().expect("blocks") {
        let block: AccessBlock = from(&json!(key));
        assert_eq!(block_text(block), text(t), "{key}");
    }
    for (key, s) in f["storage"].as_object().expect("storage") {
        let storage: ProjectStorage = from(&json!(key));
        assert_eq!(
            storage_text(storage),
            (text(&s["title"]), text(&s["detail"])),
            "{key}"
        );
    }
    for (key, t) in f["invitationStates"].as_object().expect("states") {
        let state: InvitationState = from(&json!(key));
        assert_eq!(state_label(state), text(t), "{key}");
    }
    let days = &f["inviteDays"];
    assert_eq!(
        u64::from(INVITE_DAYS),
        days["default"].as_u64().expect("default")
    );
    assert_eq!(
        u64::from(INVITE_MAX_DAYS),
        days["max"].as_u64().expect("max")
    );
    let choices = items(&days["choices"]);
    assert_eq!(choices.len(), INVITE_DAY_CHOICES.len());
    for (c, days) in choices.iter().zip(INVITE_DAY_CHOICES) {
        assert_eq!(c["days"].as_u64(), Some(u64::from(days)));
        assert_eq!(day_text(days), text(&c["text"]));
    }
}

#[test]
fn the_windows_words_are_the_webs() {
    let f = fixture();
    let t = &f["texts"];
    let sample = |v: &Value| (v["sample"].clone(), text(&v["text"]).to_owned());
    assert_eq!(TITLE, text(&t["title"]));
    assert_eq!(TAB_PEOPLE, text(&t["tabs"]["people"]));
    assert_eq!(TAB_INVITES, text(&t["tabs"]["invites"]));
    assert_eq!(CLOSE, text(&t["close"]));
    let p = &t["people"];
    for (mine, key) in [
        (people::ADD, "add"),
        (people::ROLE, "role"),
        (people::UNTIL, "until"),
        (people::SHARE, "share"),
        (people::TITLE, "title"),
        (people::LOADING, "loading"),
        (people::YOU, "you"),
        (people::REMOVE, "remove"),
        (people::BAD_DATE, "badDate"),
        (people::READ_FAILED, "readFailed"),
        (people::SHARE_FAILED, "shareFailed"),
        (people::CHANGE_FAILED, "changeFailed"),
        (people::REVOKE_FAILED, "revokeFailed"),
        (people::GUEST_ROLE, "guestRole"),
    ] {
        assert_eq!(mine, text(&p[key]), "people.{key}");
    }
    let name_texts: [Made; 5] = [
        (people::remove_label, "removeLabel"),
        (people::role_label, "roleLabel"),
        (people::adding, "adding"),
        (people::changing, "changing"),
        (people::revoking, "revoking"),
    ];
    for (make, key) in name_texts {
        let (s, want) = sample(&p[key]);
        assert_eq!(make(text(&s)), want, "people.{key}");
    }
    let (s, want) = sample(&p["count"]);
    assert_eq!(people::count(s.as_u64().expect("a count") as usize), want);
    let (s, want) = sample(&p["storage"]);
    assert_eq!(people::storage(text(&s)), want);
    let fd = &t["find"];
    assert_eq!(find::PLACEHOLDER, text(&fd["placeholder"]));
    assert_eq!(find::FAILED, text(&fd["failed"]));
    let (s, want) = sample(&fd["has"]);
    assert_eq!(find::has(text(&s)), want);
    let i = &t["invites"];
    for (mine, key) in [
        (invites::EMAIL, "email"),
        (invites::PLACEHOLDER, "placeholder"),
        (invites::ROLE, "role"),
        (invites::WAIT, "wait"),
        (invites::SEND, "send"),
        (invites::TITLE, "title"),
        (invites::LOADING, "loading"),
        (invites::EMPTY, "empty"),
        (invites::NO_RIGHT, "noRight"),
        (invites::REVOKE, "revoke"),
        (invites::CREATED, "created"),
        (invites::READ_FAILED, "readFailed"),
        (invites::SEND_FAILED, "sendFailed"),
        (invites::REVOKE_FAILED, "revokeFailed"),
        (invites::LINK_LABEL, "linkLabel"),
        (invites::COPY, "copy"),
        (invites::COPIED, "copied"),
        (invites::COPIED_SAY, "copiedSay"),
        (invites::NO_LINK, "noLink"),
        (invites::NO_LINK_WARN, "noLinkWarn"),
    ] {
        assert_eq!(mine, text(&i[key]), "invites.{key}");
    }
    let (s, want) = sample(&i["count"]);
    assert_eq!(invites::count(s.as_u64().expect("a count") as usize), want);
    let email_texts: [Made; 3] = [
        (invites::revoke_label, "revokeLabel"),
        (invites::inviting, "inviting"),
        (invites::revoking, "revoking"),
    ];
    for (make, key) in email_texts {
        let (s, want) = sample(&i[key]);
        assert_eq!(make(text(&s)), want, "invites.{key}");
    }
}

/// The access list of a case: the fixture's with the case's fields over it.
fn access_with(f: &Value, over: &Value) -> ProjectAccessList {
    let mut list = f["access"].clone();
    for (k, v) in over.as_object().expect("fields") {
        list[k] = v.clone();
    }
    from(&list)
}

#[test]
fn kisiler_lists_rows_count_storage_and_policy_as_the_web() {
    let f = fixture();
    for c in items(&f["people"]) {
        let id = text(&c["id"]);
        let list = access_with(&f, &c["list"]);
        let view = people_view(
            &list,
            text(&c["me"]),
            c["mayShare"].as_bool().expect("mayShare"),
            &zone(),
        );
        let want = &c["expect"];
        assert_eq!(view.storage.0, text(&want["storage"]["lead"]), "{id}");
        assert_eq!(view.storage.1, text(&want["storage"]["detail"]), "{id}");
        assert_eq!(view.count, text(&want["count"]), "{id}");
        assert_eq!(view.policy, text(&want["policy"]), "{id}");
        let rows = items(&want["rows"]);
        assert_eq!(view.rows.len(), rows.len(), "{id}");
        for (row, w) in view.rows.iter().zip(rows) {
            let got = json!({
                "userId": row.user_id,
                "name": row.name,
                "roleLabel": row.role_label,
                "source": row.source,
                "blocked": row.blocked,
                "you": row.you,
                "owner": row.owner,
                "guest": row.guest,
                "canChange": row.can_change,
                "canRevoke": row.can_revoke,
                "initials": row.initials,
                "sub": row.sub,
                "roleTip": row.role_tip,
                "fixed": row.fixed,
            });
            let mut got = got.as_object().expect("an object").clone();
            if let Some(e) = &row.email {
                got.insert("email".into(), json!(e));
            }
            if let Some(g) = row.grant {
                got.insert("grant".into(), json!(g));
            }
            if let Some(e) = &row.expires_at {
                got.insert("expiresAt".into(), json!(e));
            }
            assert_eq!(Value::Object(got), *w, "{id}: {}", row.name);
        }
    }
}

#[test]
fn sources_tips_shares_and_questions_are_the_webs() {
    let f = fixture();
    for c in items(&f["sources"]) {
        let holder: ProjectAccessHolder = from(&c["holder"]);
        assert_eq!(source_text(&holder, &zone()), text(&c["text"]));
    }
    for c in items(&f["shareTips"]) {
        let tip = share_tip(
            c["mayShare"].as_bool().expect("mayShare"),
            c["chosen"].as_bool().expect("chosen"),
        );
        assert_eq!(tip, text(&c["text"]));
    }
    for c in items(&f["shared"]) {
        let got = shared_text(
            text(&c["name"]),
            grant(&c["role"]),
            c["changed"].as_bool().expect("changed"),
            c["had"].as_bool().expect("had"),
        );
        assert_eq!(got, text(&c["text"]));
    }
    for c in items(&f["revokeQuestions"]) {
        let q = revoke_question(
            text(&c["name"]),
            text(&c["project"]),
            c["guest"].as_bool().expect("guest"),
        );
        let want = &c["expect"];
        assert_eq!(q.title, text(&want["title"]));
        assert_eq!(q.message, text(&want["message"]));
        assert_eq!(json!(q.details), want["details"]);
        assert_eq!(q.action, text(&want["action"]));
    }
}

#[test]
fn the_finder_searches_and_offers_an_invitation_as_the_web() {
    let f = fixture();
    let finder = &f["finder"];
    for c in items(&finder["nobody"]) {
        let (t, invite) = nobody_found(
            text(&c["query"]),
            c["personal"].as_bool().expect("personal"),
        );
        assert_eq!(t, text(&c["expect"]["text"]));
        assert_eq!(json!(invite), c["expect"]["invite"]);
    }
    for c in items(&finder["notes"]) {
        let r: Option<ProjectRole> = from(&c["role"]);
        assert_eq!(json!(candidate_note(r)), c["text"]);
    }
    for c in items(&finder["searches"]) {
        let q = text(&c["query"]);
        assert_eq!(Some(searches_for(q)), c["searches"].as_bool(), "“{q}”");
    }
    for c in items(&f["emails"]) {
        assert_eq!(
            json!(email_problem(text(&c["email"]))),
            c["problem"],
            "{}",
            c["email"]
        );
    }
}

/// Milliseconds since 1970 of a time the cases write with milliseconds.
fn millis(iso: &str) -> i64 {
    let seconds = epoch(iso).expect("a time");
    let ms = iso
        .split('.')
        .nth(1)
        .and_then(|s| s.get(..3))
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    seconds * 1000 + ms
}

#[test]
fn ends_waits_and_dates_are_the_webs() {
    let f = fixture();
    for c in items(&f["expiry"]) {
        let days = c["days"].as_u64().expect("days") as u32;
        let got = invite_expiry(days, millis(text(&c["now"])));
        assert_eq!(json!(got), c["expiresAt"], "{days} days");
    }
    for c in items(&f["endOfDay"]) {
        let date = text(&c["date"]);
        assert_eq!(json!(end_of_day(date, &zone())), c["expiresAt"], "{date}");
    }
    for c in items(&f["dates"]) {
        assert_eq!(date_text(text(&c["iso"]), &zone()), text(&c["text"]));
    }
    for c in items(&f["inviteTips"]) {
        let tip = invite_tip(
            c["mayShare"].as_bool().expect("mayShare"),
            text(&c["email"]),
        );
        assert_eq!(tip, text(&c["text"]));
    }
}

#[test]
fn invitations_are_ordered_counted_asked_and_linked_as_the_web() {
    let f = fixture();
    let inv = &f["invitations"];
    let list: Vec<ProjectInvitation> = from(&inv["sorted"]["list"]);
    let order: Vec<String> = sort_invitations(&list).into_iter().map(|i| i.id).collect();
    assert_eq!(json!(order), inv["sorted"]["order"]);
    for c in items(&inv["counts"]) {
        let states: Vec<InvitationState> = from(&c["states"]);
        let list: Vec<ProjectInvitation> = states
            .into_iter()
            .map(|state| ProjectInvitation {
                id: String::new(),
                email: String::new(),
                role: GrantRole::Viewer,
                state,
                created_by: String::new(),
                created_by_name: String::new(),
                created_at: "2026-09-20T09:00:00Z".into(),
                expires_at: "2026-10-04T09:00:00Z".into(),
                accepted_by_name: None,
                accepted_at: None,
            })
            .collect();
        assert_eq!(invites_count(&list), text(&c["text"]));
    }
    for c in items(&inv["subs"]) {
        let i: ProjectInvitation = from(&c["invitation"]);
        assert_eq!(invitation_sub(&i, &zone()), text(&c["text"]));
    }
    let invitations: Vec<ProjectInvitation> = from(&inv["questionInvitations"]);
    let access = access_with(&f, &json!({}));
    for c in items(&inv["questions"]) {
        let id = text(&c["id"]);
        let email = text(&c["email"]);
        let q = invite_question(email, &invitations, Some(&access));
        let want = &c["expect"];
        let Some(q) = q else {
            assert!(want.is_null(), "{id}");
            continue;
        };
        let mut question = serde_json::Map::new();
        if let Some(w) = &q.waiting {
            question.insert("waiting".into(), serde_json::to_value(w).expect("json"));
        }
        if let Some((name, role)) = &q.holder {
            question.insert("holder".into(), json!({ "name": name, "role": role }));
        }
        assert_eq!(Value::Object(question), want["question"], "{id}");
        let ask = invite_ask(email.trim(), &q, &zone());
        let w = &want["ask"];
        assert_eq!(ask.title, text(&w["title"]), "{id}");
        assert_eq!(ask.message, text(&w["message"]), "{id}");
        assert_eq!(json!(ask.details), w["details"], "{id}");
        assert_eq!(ask.action, text(&w["go"]), "{id}");
        assert_eq!(ask.cancel, text(&w["cancel"]), "{id}");
    }
    assert_eq!(invite_rules(true), text(&inv["rules"]["personal"]));
    assert_eq!(invite_rules(false), text(&inv["rules"]["organization"]));
    for c in items(&inv["links"]) {
        let change: InvitationChange = from(&c["change"]);
        let days = c["days"].as_u64().expect("days") as u32;
        let got = invited_link(&change, days, &zone());
        let w = &c["expect"];
        assert_eq!(got.what, text(&w["what"]));
        assert_eq!(got.warn, text(&w["warn"]));
        assert_eq!(Some(got.link), w["link"].as_bool());
    }
    let r = &inv["revokeQuestion"];
    let q = invitation_revoke_question(text(&r["email"]));
    assert_eq!(q.title, text(&r["expect"]["title"]));
    assert_eq!(q.message, text(&r["expect"]["message"]));
    assert_eq!(json!(q.details), r["expect"]["details"]);
    assert_eq!(q.action, text(&r["expect"]["action"]));
    assert_eq!(q.cancel, text(&r["expect"]["cancel"]));
    for c in items(&inv["initials"]) {
        assert_eq!(invitation_initial(text(&c["email"])), text(&c["text"]));
    }
    let l = &inv["link"];
    assert_eq!(
        invitation_link(text(&l["token"]), text(&l["base"])),
        text(&l["link"])
    );
    for c in items(&f["accepted"]) {
        let a: InvitationAccepted = from(&c["accepted"]);
        assert_eq!(accepted_how(&a), text(&c["text"]));
    }
}

#[test]
fn the_commands_inputs_failures_and_lines_are_the_webs() {
    let f = fixture();
    for c in items(&f["envelopes"]) {
        let args: Vec<&str> = items(&c["args"]).iter().map(text).collect();
        let (name, input) = match text(&c["command"]) {
            "share" => (
                "project.share",
                share_input(args[2], grant(&json!(args[3])), args.get(4).copied()),
            ),
            "revoke" => ("project.access.revoke", revoke_input(args[2])),
            "invite" => (
                "project.invite",
                invite_input(args[2], grant(&json!(args[3])), args.get(4).copied()),
            ),
            "invitationRevoke" => (
                "project.invitation.revoke",
                invitation_revoke_input(args[2]),
            ),
            other => panic!("unknown command {other}"),
        };
        let want = &c["expect"];
        assert_eq!(name, text(&want["commandName"]));
        assert_eq!(want["version"], json!(1));
        assert_eq!(input, want["input"], "{name}");
    }
    for c in items(&f["failures"]) {
        let e = &c["error"];
        let failure = if let Some(plain) = e["plain"].as_str() {
            // What never came from the server (the web's plain Error).
            ApiFailure::new(0, "local", plain)
        } else {
            let status = e["status"].as_u64().expect("a status") as u16;
            let code = e["error"]
                .as_str()
                .unwrap_or(if status == 0 { "network" } else { "http" });
            let message = e["message"]
                .as_str()
                .or(e["fallback"].as_str())
                .unwrap_or("");
            ApiFailure::new(status, code, message)
                .with_retryable(e["retryable"].as_bool().unwrap_or(false))
        };
        assert_eq!(
            failure_text(&failure, text(&c["failed"])),
            text(&c["text"]),
            "{e}"
        );
    }
    for c in items(&f["lines"]) {
        let a: Vec<&str> = items(&c["args"]).iter().map(text).collect();
        let got = match text(&c["line"]) {
            "shareLog" => share_log(a[0], a[1]),
            "roleChanged" => role_changed_text(a[0], grant(&json!(a[1]))),
            "revoked" => revoked_text(a[0]),
            "invitedLog" => invited_log(a[0], a[1], grant(&json!(a[2]))),
            "invitationRevokedLog" => invitation_revoked_log(a[0], a[1]),
            "invitationRevokedSay" => invitation_revoked_say(a[0]),
            other => panic!("unknown line {other}"),
        };
        assert_eq!(got, text(&c["text"]));
    }
}
