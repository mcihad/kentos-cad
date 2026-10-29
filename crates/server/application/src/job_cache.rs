//! The result cache of server jobs (docs/adr/0739 §2; plan/gis.md CBS-39): a job whose artifact is a pure function of
//! what it read takes the artifact of an earlier job of the same project that read the same, instead of building it
//! again.
//!
//! - **What is cached:** a `project.export`'s `.kcad`, a `project.atlas`'s PDF and a `project.processing` job's
//!   GeoPackage (`mode: sink`). A checkpoint, an import and a tool committing its result change the project; they are
//!   never cached.
//! - **The key:** the SHA-256 of a canonical JSON (docs/adr/0253 §2) of the kind, the input without what does not
//!   change the result (`expectedRevision`, which only refuses; `cpuSeconds` and `memoryMb`, a budget; `label`, a
//!   name), the project's data revision and event cursor (every write moves the cursor, so the state is the one the
//!   artifact was built from) and the stamp of the reach the result was cut by (docs/adr/0550). An atlas or a tool may
//!   read `@` values (the user, the date, the time; docs/adr/0214), so their key also holds the user who asked and the
//!   day (UTC): one user's PDF is never another's, and a new day builds it again.
//! - **Taking it:** the worker computes the key from the project as it is before it reads anything; a succeeded job of
//!   the project with that key whose object is still in the store gives the new job a second key of the same object
//!   (`Blobs::share`), and the job is finished as any other, with the access and the reach asked again. A job asked
//!   `fresh` is built again. The key a job is stored with is computed from the snapshot it read, never from what was
//!   read before it: a write in between gives a key no later lookup made before it can find.
//! - **The software:** the key also holds what built the file: the server's version and its program's size and change
//!   time ([`software`]), and for a job a worker's processor runs, the processor's own program
//!   (`Processor::software`). A server or a tool built again never takes an earlier build's file.
//! - **Removing it:** each job's artifact is its own key; the retention removes a job's key and leaves the others.

use std::sync::OnceLock;
use std::time::UNIX_EPOCH;

use kentos_contracts::canonical::canonical_json;
use kentos_contracts::{
    PROJECT_ATLAS_JOB, PROJECT_EXPORT_JOB, PROJECT_PROCESSING_JOB, ProcessingJobMode,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// The record's format, part of every key: a change of the rule changes every key.
pub const FORMAT: &str = "kentos.job-cache/1";

/// The input's members that do not change a cached kind's result.
const NOT_RESULT: [&str; 4] = ["expectedRevision", "cpuSeconds", "memoryMb", "label"];

/// What a key is made of besides the kind and the input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State<'a> {
    /// The project's data revision and event cursor.
    pub revision: i64,
    pub cursor: i64,
    /// The stamp of the caller's reach ([`kentos_forms::reach::Reach::stamp`]); none when nothing cuts.
    pub reach: Option<&'a str>,
    /// The user who asked, and the day (`YYYY-MM-DD`, UTC): held for the kinds that may read them.
    pub user: &'a str,
    pub day: &'a str,
    /// What builds the file ([`software`], and the processor's program for a job it runs).
    pub software: &'a str,
}

/// The mark of the program running this server: its version and the program file's size and change time. Read
/// once; the version alone when the program cannot be read.
pub fn software() -> &'static str {
    static MARK: OnceLock<String> = OnceLock::new();
    MARK.get_or_init(|| {
        let version = env!("CARGO_PKG_VERSION");
        match std::env::current_exe().and_then(std::fs::metadata) {
            Ok(m) => format!("{version}+{}", file_mark(&m)),
            Err(_) => version.to_owned(),
        }
    })
}

/// A program file's size and change time (nanoseconds since 1970), as a mark of its build.
pub fn file_mark(m: &std::fs::Metadata) -> String {
    let at = m
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    format!("{}:{at}", m.len())
}

/// Whether a kind's result can be taken from an earlier job: an export, an atlas, a tool writing a file.
pub fn cached(kind: &str, input: &Value) -> bool {
    match kind {
        PROJECT_EXPORT_JOB | PROJECT_ATLAS_JOB => true,
        PROJECT_PROCESSING_JOB => {
            serde_json::from_value::<ProcessingJobMode>(
                input.get("mode").cloned().unwrap_or(Value::Null),
            )
            .unwrap_or_default()
                == ProcessingJobMode::Sink
        }
        _ => false,
    }
}

/// Whether a kind's result may read the user and the day (`@` values).
fn personal(kind: &str) -> bool {
    kind != PROJECT_EXPORT_JOB
}

/// The input as far as it changes the result: an object without the members that do not.
fn result_input(input: &Value) -> Value {
    match input {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(k, _)| !NOT_RESULT.contains(&k.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        ),
        Value::Null => json!({}),
        other => other.clone(),
    }
}

/// The text a key is the SHA-256 of; none for a kind that is not cached.
pub fn record(kind: &str, input: &Value, state: &State) -> Option<String> {
    if !cached(kind, input) {
        return None;
    }
    let personal = personal(kind);
    Some(canonical_json(&json!({
        "format": FORMAT,
        "kind": kind,
        "input": result_input(input),
        "revision": state.revision.to_string(),
        "cursor": state.cursor.to_string(),
        "reach": state.reach,
        "user": if personal { Value::from(state.user) } else { Value::Null },
        "day": if personal { Value::from(state.day) } else { Value::Null },
        "software": state.software,
    })))
}

/// The key: the SHA-256 of [`record`], as hex.
pub fn key(kind: &str, input: &Value, state: &State) -> Option<String> {
    record(kind, input, state).map(|text| {
        Sha256::digest(text.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    })
}

/// The day of `moment` (UTC) as the key holds it.
pub fn day_of(moment: time::OffsetDateTime) -> String {
    let d = moment.to_offset(time::UtcOffset::UTC).date();
    format!("{:04}-{:02}-{:02}", d.year(), u8::from(d.month()), d.day())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state<'a>(revision: i64, cursor: i64, reach: Option<&'a str>, user: &'a str) -> State<'a> {
        State {
            revision,
            cursor,
            reach,
            user,
            day: "2026-10-29",
            software: "0.1.0",
        }
    }

    #[test]
    fn the_key_holds_what_changes_the_result_and_nothing_else() {
        let s = state(4, 9, Some("r"), "u1");
        let a = key("project.export", &json!({ "expectedRevision": "4" }), &s);
        let b = key("project.export", &json!({}), &s);
        let c = key("project.export", &Value::Null, &s);
        assert!(a.is_some());
        assert_eq!(a, b);
        assert_eq!(b, c);
        // An export is the same file for every user with the same reach.
        assert_eq!(
            key("project.export", &json!({}), &state(4, 9, Some("r"), "u2")),
            b
        );
        // A write moves the revision or the cursor; another reach cuts other fields.
        for other in [
            state(5, 9, Some("r"), "u1"),
            state(4, 10, Some("r"), "u1"),
            state(4, 9, Some("q"), "u1"),
            state(4, 9, None, "u1"),
        ] {
            assert_ne!(key("project.export", &json!({}), &other), b);
        }
        // An atlas is the user's and the day's.
        let atlas = json!({ "sheet": "p1", "label": "Ada" });
        let mine = key("project.atlas", &atlas, &s);
        assert_eq!(
            mine,
            key("project.atlas", &json!({ "sheet": "p1" }), &s),
            "the label is a name"
        );
        assert_ne!(
            mine,
            key("project.atlas", &atlas, &state(4, 9, Some("r"), "u2"))
        );
        let tomorrow = State {
            day: "2026-10-30",
            ..s.clone()
        };
        assert_ne!(mine, key("project.atlas", &atlas, &tomorrow));
        // Another build of the server or of the tools: another file.
        let rebuilt = State {
            software: "0.1.0+2",
            ..s.clone()
        };
        assert_ne!(mine, key("project.atlas", &atlas, &rebuilt));
        assert_ne!(b, key("project.export", &json!({}), &rebuilt));
        assert!(software().starts_with(env!("CARGO_PKG_VERSION")));
        assert_eq!(software(), software());
    }

    #[test]
    fn only_results_that_change_nothing_are_cached() {
        let s = state(1, 1, None, "u");
        assert!(key("project.checkpoint", &json!({ "name": "x" }), &s).is_none());
        assert!(key("project.import", &json!({ "upload": "x" }), &s).is_none());
        assert!(
            key(
                "project.processing",
                &json!({ "tool": "t", "mode": "commit" }),
                &s
            )
            .is_none()
        );
        assert!(key("project.processing", &json!({ "tool": "t" }), &s).is_none());
        let sink = key(
            "project.processing",
            &json!({ "tool": "t", "mode": "sink", "cpuSeconds": 5 }),
            &s,
        );
        assert!(sink.is_some());
        assert_eq!(
            sink,
            key(
                "project.processing",
                &json!({ "mode": "sink", "tool": "t", "memoryMb": 64 }),
                &s
            )
        );
        assert_ne!(
            sink,
            key(
                "project.processing",
                &json!({ "tool": "t", "mode": "sink", "parameters": { "d": 2 } }),
                &s
            )
        );
    }

    #[test]
    fn the_day_is_utc() {
        let m = time::Date::from_calendar_date(2026, time::Month::October, 29)
            .and_then(|d| d.with_hms(23, 30, 0))
            .map(|t| t.assume_offset(time::UtcOffset::from_hms(-2, 0, 0).unwrap()))
            .unwrap();
        assert_eq!(day_of(m), "2026-10-30");
    }
}
