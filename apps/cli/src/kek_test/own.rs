//! The package's own tests (docs/adr/8057 §87): `tests/test_*.py` beside its code, run by Python's own `unittest` in
//! the extension's environment, its code found in `src` and KentOS's library as the environment has it. `unittest`,
//! not pytest: it needs no package the lock must bring (an agent in KentOS's workshop cannot run uv), and KentOS's own
//! Python tests are written with it.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// How long the tests may take before they are stopped.
const LIMIT: Duration = Duration::from_secs(300);
/// The characters of their output the report keeps, its end.
const KEPT: usize = 6_000;

/// The package's tests run with `python` (its environment's), from `folder`; none when it has no `tests/test_*.py`.
pub(super) fn run(folder: &Path, python: &Path) -> Option<Value> {
    let tests = folder.join("tests");
    let any = std::fs::read_dir(&tests).ok()?.flatten().any(|e| {
        let name = e.file_name();
        let name = name.to_string_lossy();
        name.starts_with("test_") && name.ends_with(".py")
    });
    if !any {
        return None;
    }
    let began = Instant::now();
    // Absolute: the tests start in the folder, where a relative path (`kek test samples/…`) would point elsewhere.
    let folder = std::path::absolute(folder).unwrap_or_else(|_| folder.to_path_buf());
    let src = folder.join("src");
    let path = match std::env::var_os("PYTHONPATH") {
        Some(old) => std::env::join_paths([src.as_os_str(), old.as_os_str()]).unwrap_or_default(),
        None => src.into_os_string(),
    };
    let child = Command::new(python)
        // The start folder is the top too: its modules import as they are, no `tests/__init__.py` asked for.
        .args(["-B", "-m", "unittest", "discover", "-s", "tests", "-v"])
        .current_dir(&folder)
        .env("PYTHONPATH", path)
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(e) => {
            return Some(
                json!({ "status": "not_run", "summary": format!("Sınamalar başlatılamadı: {e}.") }),
            );
        }
    };
    let (out, err) = (child.stdout.take(), child.stderr.take());
    let read = |pipe: Option<std::process::ChildStdout>| {
        std::thread::spawn(move || {
            let mut text = String::new();
            if let Some(mut p) = pipe {
                let _ = std::io::Read::read_to_string(&mut p, &mut text);
            }
            text
        })
    };
    let out = read(out);
    let err = std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(mut p) = err {
            let _ = std::io::Read::read_to_string(&mut p, &mut text);
        }
        text
    });
    let mut timed_out = false;
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) if began.elapsed() > LIMIT => {
                let _ = child.kill();
                let _ = child.wait();
                timed_out = true;
                break None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => break None,
        }
    };
    let output = format!(
        "{}{}",
        out.join().unwrap_or_default(),
        err.join().unwrap_or_default()
    );
    let ms = began.elapsed().as_millis() as u64;
    Some(read_result(&output, code, timed_out, ms))
}

/// The run's report from unittest's words: how many ran, failed and errored, its status and a sentence.
pub(super) fn read_result(output: &str, code: Option<i32>, timed_out: bool, ms: u64) -> Value {
    let ran = output
        .lines()
        .find_map(|l| {
            l.strip_prefix("Ran ")?
                .split(' ')
                .next()?
                .parse::<u64>()
                .ok()
        })
        .unwrap_or(0);
    let count = |key: &str| -> u64 {
        output
            .lines()
            .rev()
            .find(|l| l.starts_with("FAILED ("))
            .and_then(|l| l.split(key).nth(1))
            .and_then(|rest| {
                rest.split(|c: char| !c.is_ascii_digit())
                    .next()?
                    .parse()
                    .ok()
            })
            .unwrap_or(0)
    };
    let (failures, errors) = (count("failures="), count("errors="));
    let tail: String = {
        let chars: Vec<char> = output.chars().collect();
        chars[chars.len().saturating_sub(KEPT)..].iter().collect()
    };
    let (status, summary) = if output.contains("NO TESTS RAN") {
        (
            "failed",
            "tests/ klasöründeki dosyalarda çalışan sınama yok: sınamalar unittest.TestCase sınıfında yazılır \
             (pytest biçimindeki düz işlevler unittest'le çalışmaz)."
                .to_owned(),
        )
    } else if timed_out {
        (
            "failed",
            format!(
                "Sınamalar {} saniyede bitmedi, durduruldu.",
                LIMIT.as_secs()
            ),
        )
    } else if code == Some(0) {
        if ran == 0 {
            (
                "passed",
                "tests/ klasöründe çalışacak sınama bulunmadı.".to_owned(),
            )
        } else {
            ("passed", format!("{ran} sınamanın hepsi geçti."))
        }
    } else if failures + errors > 0 {
        (
            "failed",
            format!(
                "{ran} sınamadan {} tanesi kaldı ({failures} başarısız, {errors} hata).",
                failures + errors
            ),
        )
    } else {
        (
            "failed",
            "Sınamalar çalıştırılamadı (içe aktarma hatası olabilir).".to_owned(),
        )
    };
    json!({
        "status": status, "summary": summary, "ran": ran, "failures": failures, "errors": errors, "ms": ms,
        "output": tail,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// unittest's own words read: passed, failed with failures and errors, nothing to run, stopped.
    #[test]
    fn unittests_words_are_read() {
        let ok = "test_a (tests.test_x.T.test_a) ... ok\n\n------\nRan 3 tests in 0.004s\n\nOK\n";
        let r = read_result(ok, Some(0), false, 12);
        assert_eq!(
            (r["status"].as_str(), r["ran"].as_u64()),
            (Some("passed"), Some(3))
        );
        assert_eq!(r["summary"], "3 sınamanın hepsi geçti.");
        let bad = "Ran 4 tests in 0.01s\n\nFAILED (failures=1, errors=2)\n";
        let r = read_result(bad, Some(1), false, 12);
        assert_eq!(
            r["summary"],
            "4 sınamadan 3 tanesi kaldı (1 başarısız, 2 hata)."
        );
        let none = "\n----------------------------------------------------------------------\nRan 0 tests in 0.000s\n\nNO TESTS RAN\n";
        let r = read_result(none, Some(5), false, 1);
        assert_eq!(r["status"], "failed");
        assert!(r["summary"].as_str().unwrap().contains("unittest.TestCase"));
        assert_eq!(read_result("", None, true, 1)["status"], "failed");
    }
}
