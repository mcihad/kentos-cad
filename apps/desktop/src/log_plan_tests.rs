//! The desktop writes its log as the web does: every section of
//! fixtures/shell/v1/log.json (format in fixtures/shell/README.md), whose
//! answers scripts/fixtures/log_cases.py wrote apart from either program.

use kentos_interaction::Level;
use serde_json::Value;

use crate::cloud::local_time::Zone;
use crate::log_plan::{self as plan, Listing, Log};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/shell/v1/log.json"))
        .expect("log.json reads")
}

fn text(v: &Value) -> &str {
    v.as_str().expect("a text")
}

fn level(name: &str) -> Level {
    match name {
        "command" => Level::Command,
        "info" => Level::Info,
        "success" => Level::Success,
        "warn" => Level::Warn,
        "error" => Level::Error,
        other => panic!("a level: {other}"),
    }
}

#[test]
fn the_panels_words_tabs_and_levels_are_the_webs() {
    let f = fixture();
    let tabs: Vec<&str> = f["tabs"]
        .as_array()
        .expect("tabs")
        .iter()
        .map(|t| text(&t["label"]))
        .collect();
    assert_eq!(
        tabs,
        [plan::TAB_HISTORY, plan::TAB_COORDS, plan::TAB_MESSAGES]
    );
    let t = &f["texts"];
    assert_eq!(text(&t["clear"]), plan::CLEAR);
    assert_eq!(text(&t["close"]), plan::CLOSE);
    assert_eq!(text(&t["open"]), plan::OPEN);
    assert_eq!(text(&t["empty"]["history"]), plan::EMPTY_HISTORY);
    assert_eq!(text(&t["empty"]["messages"]), plan::EMPTY_MESSAGES);
    assert_eq!(f["limit"], plan::LIMIT);
    assert_eq!(
        f["followWithin"].as_f64(),
        Some(f64::from(plan::FOLLOW_WITHIN))
    );
    for l in f["levels"].as_array().expect("levels") {
        let lv = level(text(&l["level"]));
        assert_eq!(plan::level_icon(lv), l["icon"].as_str(), "{l}");
        let listed: Vec<&str> = l["listedIn"]
            .as_array()
            .expect("tabs")
            .iter()
            .map(text)
            .collect();
        assert_eq!(
            plan::listed_in(Listing::History, lv),
            listed.contains(&"history")
        );
        assert_eq!(
            plan::listed_in(Listing::Messages, lv),
            listed.contains(&"messages"),
            "{l}"
        );
    }
}

#[test]
fn times_echoes_and_the_status_bars_lines_are_the_webs() {
    let f = fixture();
    assert_eq!(text(&f["timeZone"]), "Europe/Istanbul");
    // Istanbul keeps +03:00 all year.
    let istanbul = Zone::fixed(3 * 3600);
    for c in f["times"].as_array().expect("times") {
        assert_eq!(
            plan::log_time(c["at"].as_i64().expect("at"), &istanbul),
            text(&c["text"]),
            "{}",
            c["title"]
        );
    }
    for c in f["echo"].as_array().expect("echoes") {
        assert_eq!(plan::echo(text(&c["typed"])), text(&c["text"]));
    }
    for c in f["flash"].as_array().expect("flash") {
        let shown = plan::flash_of(level(text(&c["level"])), text(&c["text"]));
        match c["shown"].as_object() {
            None => assert_eq!(shown, None, "{c}"),
            Some(s) => {
                let shown = shown.expect("shown");
                assert_eq!(shown.icon, s["icon"].as_str().expect("icon"), "{c}");
                assert_eq!(Some(shown.ms), s["ms"].as_u64(), "{c}");
            }
        }
    }
}

#[test]
fn what_is_kept_and_what_the_badge_counts_are_the_webs() {
    let f = fixture();
    for c in f["kept"].as_array().expect("kept") {
        let mut log = Log::default();
        for _ in 0..c["pushed"].as_u64().expect("pushed") {
            log.push(Level::Info, "satır", 0);
        }
        assert_eq!(log.len() as u64, c["length"].as_u64().expect("length"));
        assert_eq!(log.first_id(), c["first"].as_u64().expect("first"));
        assert_eq!(log.last_id(), c["last"].as_u64().expect("last"));
    }
    for script in f["badge"].as_array().expect("badge") {
        let mut log = Log::default();
        for s in script["steps"].as_array().expect("steps") {
            let step = &s["step"];
            match step.as_str() {
                Some("look") => log.look(),
                Some("clear") => log.clear(),
                Some(name) => {
                    log.push(level(name), "satır", 0);
                }
                None => {
                    for _ in 0..step["times"].as_u64().expect("times") {
                        log.push(level(text(&step["push"])), "satır", 0);
                    }
                }
            }
            assert_eq!(
                log.unseen() as u64,
                s["count"].as_u64().expect("count"),
                "{}: {s}",
                script["title"]
            );
            assert_eq!(log.len() as u64, s["lines"].as_u64().expect("lines"));
        }
    }
}
