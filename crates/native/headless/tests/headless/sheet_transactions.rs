//! The same independent model/paper cases as Python (ADR 0590).
use super::*;

fn state(s: &Session) -> Value {
    let summary = s.summary();
    json!({ "objects": summary["objects"], "revision": summary["revision"],
        "dirty": summary["dirty"], "undo": summary["canUndo"], "redo": summary["canRedo"],
        "sheets": s.document().sheets(), "stamps": summary["stamps"] })
}

#[test]
fn model_and_paper_transaction_cases() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/model-transactions.json"
    ))
    .expect("cases");
    let cases = data["cases"].as_array().expect("cases");
    assert!(cases.len() >= 20);
    for case in cases {
        let mut s = Session::from_bytes(data["setup"].to_string().as_bytes()).expect("drawing");
        for (i, step) in case["actions"].as_array().unwrap().iter().enumerate() {
            let at = format!("{} / {i}: {step}", case["name"]);
            let before = state(&s);
            match step["action"].as_str().unwrap() {
                "begin" => s.begin_group("Birlikte").expect("begin"),
                "end" => assert!(s.end_group(), "{at}"),
                "cancel" => assert!(s.cancel_group(), "{at}"),
                "undo" | "redo" => {
                    let done = if step["action"] == "undo" {
                        s.try_undo()
                    } else {
                        s.try_redo()
                    };
                    if let Some(code) = step["error"].as_str() {
                        assert_eq!(done.expect_err(&at).code, code, "{at}");
                    } else {
                        assert_eq!(
                            done.expect(&at).is_some(),
                            step["taken"].as_bool().unwrap(),
                            "{at}"
                        );
                    }
                }
                "command" => {
                    let done = s
                        .run(
                            step["command"].as_str().unwrap(),
                            None,
                            Op::parse(step["op"].as_str().unwrap_or("execute")).unwrap(),
                            step["input"].clone(),
                        )
                        .expect(&at);
                    assert_eq!(
                        done["status"],
                        step["status"].as_str().unwrap_or("completed"),
                        "{at}: {done}"
                    );
                    if let Some(code) = step["code"].as_str() {
                        assert_eq!(done["error"]["code"], code, "{at}");
                    }
                }
                action => panic!("unknown action {action}"),
            }
            assert_eq!(s.summary()["objects"], step["expect"]["objects"], "{at}");
            let names: Vec<_> = kentos_native_application::sheets_edit::sheets_in(s.document())
                .expect("sheets")
                .into_iter()
                .map(|s| s.name)
                .collect();
            assert_eq!(json!(names), step["expect"]["sheets"], "{at}");
            if step["unchanged"] == true {
                assert_eq!(state(&s), before, "{at}: all data/history flags stay");
            }
        }
    }
}
