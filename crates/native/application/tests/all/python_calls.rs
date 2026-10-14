//! The command history's Python code (docs/adr/0284): every completed call
//! of the shared command cases (fixtures/commands/v1) written as Python
//! parses, names the SDK's wrapper and gives the input; `python3` checks the
//! syntax here, `scripts/python/history_calls.py` checks the rest with the
//! SDK (KENTOS_PYTHON_CALLS_OUT writes what it reads).

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use kentos_native_application::python_call::{python_call, python_path};
use serde_json::{Value, json};

/// Every completed call of the cases: its command, version and input.
fn calls() -> Vec<(String, u32, Value)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/commands/v1");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&root)
        .expect("the command cases")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    let mut out = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).expect("a case file");
        let cases: Value = serde_json::from_str(&text).expect("a case file reads");
        let (Some(command), Some(version)) =
            (cases["command"].as_str(), cases["commandVersion"].as_u64())
        else {
            continue;
        };
        for case in cases["cases"].as_array().into_iter().flatten() {
            for step in case["steps"].as_array().into_iter().flatten() {
                if step["op"] == "execute"
                    && step["result"]["status"] == "completed"
                    && step["input"].is_object()
                {
                    out.push((
                        command.to_owned(),
                        u32::try_from(version).unwrap_or(1),
                        step["input"].clone(),
                    ));
                }
            }
        }
    }
    out
}

#[test]
fn every_completed_call_is_python_that_parses() {
    let calls = calls();
    assert!(calls.len() > 500, "the cases' calls: {}", calls.len());
    let written: Vec<Value> = calls
        .iter()
        .map(|(command, version, input)| {
            json!({
                "command": command,
                "path": python_path(command),
                "input": input,
                "code": python_call(command, *version, None, input),
            })
        })
        .collect();
    let all = Value::Array(written);
    if let Ok(out) = std::env::var("KENTOS_PYTHON_CALLS_OUT") {
        std::fs::write(&out, serde_json::to_vec(&all).expect("JSON")).expect("writes the calls");
    }
    let script = "import ast, json, sys\nbad = 0\nfor c in json.load(sys.stdin):\n    try:\n        ast.parse(c['code'])\n    except SyntaxError as e:\n        bad += 1\n        print(c['command'], e, c['code'])\nsys.exit(1 if bad else 0)\n";
    let mut python = Command::new("python3")
        .args(["-c", script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("python3 runs (the history's Python is checked with it)");
    python
        .stdin
        .take()
        .expect("its input")
        .write_all(&serde_json::to_vec(&all).expect("JSON"))
        .expect("the calls to python3");
    let done = python.wait_with_output().expect("python3 ends");
    assert!(
        done.status.success(),
        "{}",
        String::from_utf8_lossy(&done.stdout)
    );
}

/// The nested and tagged inputs as the code writes them: a transform's tagged
/// kind and a polyline's points, read back by `ast.literal_eval`.
#[test]
fn nested_and_tagged_inputs_are_python_literals() {
    let input = json!({
        "uids": ["0190a0b0-0000-7000-8000-000000000001"],
        "transform": {"kind": "rotate", "base": {"x": 487020.0, "y": 4420015.0}, "angle": 0.5},
        "copy": false,
    });
    let code = python_call(
        "cad.entities.transform",
        1,
        Some("Nesneleri dönüştür"),
        &input,
    );
    assert!(
        code.contains("command = cad.entities.transform\n"),
        "{code}"
    );
    assert!(code.contains("\"copy\": False"), "{code}");
    assert!(code.contains("\"kind\": \"rotate\""), "{code}");
    let script = format!(
        "import ast, json\ntree = ast.parse({code:?})\ncall = [n for n in ast.walk(tree) if isinstance(n, ast.Call) and getattr(n.func, 'attr', '') == 'from_json'][0]\nprint(json.dumps(ast.literal_eval(call.args[0]), sort_keys=True))\n"
    );
    let out = Command::new("python3")
        .args(["-c", &script])
        .output()
        .expect("python3 runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let back: Value = serde_json::from_slice(&out.stdout).expect("the literal as JSON");
    assert_eq!(back, input);
}
