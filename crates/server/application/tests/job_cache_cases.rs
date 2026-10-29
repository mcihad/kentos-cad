//! The shared cases of the server jobs' result cache and of rotated backups (docs/adr/0739; written by
//! `scripts/fixtures/job_cache_cases.py` from the ADR's rules, without KentOS code): the key's record and digest, the
//! rotation and the backup folders' names, as the server computes them.

use kentos_application::{backup, job_cache};
use serde_json::Value;

fn cases() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/server-jobs/v1/cache-rotation.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
}

#[test]
fn the_cache_keys_are_the_references() {
    let all = cases();
    let keys = all["keys"].as_array().expect("keys");
    assert!(keys.len() >= 20, "{}", keys.len());
    for case in keys {
        let name = case["name"].as_str().unwrap();
        let s = &case["state"];
        let (user, day) = (s["user"].as_str().unwrap(), s["day"].as_str().unwrap());
        let state = job_cache::State {
            revision: s["revision"].as_i64().unwrap(),
            cursor: s["cursor"].as_i64().unwrap(),
            reach: s["reach"].as_str(),
            user,
            day,
            software: s["software"].as_str().unwrap(),
        };
        let kind = case["kind"].as_str().unwrap();
        assert_eq!(
            job_cache::record(kind, &case["input"], &state).as_deref(),
            case["record"].as_str(),
            "{name}"
        );
        assert_eq!(
            job_cache::key(kind, &case["input"], &state).as_deref(),
            case["key"].as_str(),
            "{name}"
        );
    }
}

#[test]
fn the_rotations_are_the_references() {
    let all = cases();
    for case in all["rotations"].as_array().expect("rotations") {
        let name = case["name"].as_str().unwrap();
        let found: Vec<(String, bool)> = case["found"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| (f[0].as_str().unwrap().to_owned(), f[1].as_bool().unwrap()))
            .collect();
        let keep = case["keep"].as_u64().unwrap() as usize;
        let want: Vec<String> = case["removed"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_str().unwrap().to_owned())
            .collect();
        assert_eq!(backup::rotation(&found, keep), want, "{name}");
    }
    for case in all["names"].as_array().expect("names") {
        let name = case["name"].as_str().unwrap();
        let want = case["order"].as_array().map(|o| {
            (
                o[0].as_str().unwrap().to_owned(),
                o[1].as_u64().unwrap() as u32,
            )
        });
        assert_eq!(backup::folder_order(name), want, "{name}");
    }
}

#[test]
fn a_folder_name_reads_back_in_its_order() {
    let moment = time::OffsetDateTime::from_unix_timestamp(1_761_739_445).unwrap();
    let first = backup::folder_name(moment, 1);
    assert_eq!(first, "2025-10-29T12-04-05Z");
    assert_eq!(backup::folder_name(moment, 2), format!("{first}-2"));
    assert!(backup::folder_order(&first) < backup::folder_order(&format!("{first}-2")));
    assert!(
        backup::folder_order(&format!("{first}-2")) < backup::folder_order(&format!("{first}-10"))
    );
}
