//! `kentos kek test <klasör> [--install-python] [--json]` (plan/eklenti.md ADIM 21, "başsız test koşucusu";
//! docs/adr/8057 §74): an extension's working folder tested as a continuous integration job tests it, without a window
//! and without touching the user's extensions.
//!
//! 1. **Verified** as `kek verify` verifies it: the package's gates and the İşlemler tools' descriptors whole. A
//!    refusal ends it.
//! 2. **Installed into a registry of its own** in the temporary folder, everything it asks for granted (the test is its
//!    author's), and taken away at the end, as `kek verify`'s dry run is.
//! 3. **Its environment prepared from its lock** in KentOS's Python folder as the desktop prepares it: the same key,
//!    the same folder, so that a test and the desktop share the environment of one lock. KentOS's Python is the one
//!    installed there; `--install-python` installs it with uv when it is not (a download its caller asked for).
//! 4. **Its tools' examples run** (§48) on synthetic drawings in its version's process, every tool's (a desktop-only
//!    tool's code runs the same way), each checked against what it expects; a failure's traceback is said with it.
//! 5. **Its own tests run** (§87): `tests/test_*.py` with Python's `unittest` in its environment, its code from the
//!    folder's `src` (`kentos_extension_tools::examples::package_tests`), after its examples.
//!
//! Exit codes: 0 every example passed (or there were none), 1 the extension failed (refused, its lock, an example), 2
//! the test could not run (a wrong command, an unreadable folder, no uv, no Python, no network).

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use kentos_extension_tools::examples::{Examples, Expression, PackageTests, Rule, RunRule};
use kentos_extension_tools::headless::{Headless, offer_with};
use kentos_extension_tools::{Library, Note, Plan, Python, run_in_pool, start_of};
use kentos_python_env::process::Pool;
use serde_json::{Value, json};

use setup::{environment, install_into, python_of};

mod setup;

/// Why the test ended before its examples: the extension's (exit 1) or the computer's (exit 2).
enum Stop {
    Failed(String),
    CouldNot(String),
}

/// The test's own registry under the temporary folder, removed when dropped.
struct Temp(PathBuf);

impl Temp {
    fn new() -> Result<Temp, String> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let dir =
            std::env::temp_dir().join(format!("kentos-kek-test-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Geçici klasör açılamadı ({}): {e}", dir.display()))?;
        Ok(Temp(dir))
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        // The installed files are read-only: the registry's own removal.
        let _ = kentos_extension_host::store::remove_tree(&self.0);
    }
}

/// Seconds as the report says them: `1,61 sn`.
fn seconds(ms: u64) -> String {
    format!("{:.2} sn", ms as f64 / 1000.0).replace('.', ",")
}

/// The test of the working folder `folder`; `install` lets it install KentOS's Python.
pub(crate) fn run(folder: &Path, json: bool, install: bool) -> ExitCode {
    let (files, _) = match super::read_folder(folder) {
        Ok(read) => read,
        Err(message) => return super::fail(message),
    };
    let v = super::verified(&files);
    let mut report = json!({
        "kind": "test",
        "verify": { "problems": v.problems, "warnings": v.warnings, "digest": v.digest },
    });
    if !json {
        if let Some(m) = &v.manifest {
            say!("{} {} — {}", m.id, m.version, m.name);
        }
        super::print_placed(&v);
    }
    let manifest = match (&v.manifest, v.is_valid()) {
        (Some(m), true) => m.clone(),
        _ => {
            let said = format!("Doğrulanmadı: {} sorun.", v.problems.len());
            return end(report, json, "refused", &said, 1);
        }
    };
    report["extension"] = json!({ "id": manifest.id, "version": manifest.version.to_string() });
    if !json {
        say!("Doğrulandı: {} uyarı.", v.warnings.len());
    }
    let Some(project) = &manifest.python else {
        let said = "Eklentinin Python kodu yok: doğrulandı, çalıştırılacak örnek yok.";
        return end(report, json, "passed", said, 0);
    };
    let temp = match Temp::new() {
        Ok(temp) => temp,
        Err(why) => return end(report, json, "not_run", &why, 2),
    };
    let tell = |line: String| {
        if !json {
            say!("{line}");
        }
    };
    let prepared = install_into(&temp.0, folder, &files, &manifest).and_then(|package| {
        let python = python_of(install, &tell)?;
        let began = Instant::now();
        let (ready, made) = environment(
            &python,
            &package,
            &manifest,
            &project.project_file,
            &project.lock_file,
        )?;
        let ms = began.elapsed().as_millis() as u64;
        Ok((package, python, ready, made, ms))
    });
    let (package, python, ready, made, ms) = match prepared {
        Ok(prepared) => prepared,
        Err(Stop::Failed(why)) => return end(report, json, "failed", &why, 1),
        Err(Stop::CouldNot(why)) => return end(report, json, "not_run", &why, 2),
    };
    let packages: Vec<String> = ready
        .packages
        .iter()
        .map(|p| format!("{} {}", p.name, p.version))
        .collect();
    report["environment"] = json!({
        "python": python.interpreter.version, "packages": packages, "made": made, "ms": ms,
    });
    tell(format!(
        "Python ortamı {}: Python {}, {} paket ({}).",
        if made {
            "kilidinden hazırlandı"
        } else {
            "hazırdı"
        },
        python.interpreter.version,
        packages.len(),
        seconds(ms)
    ));
    let heard: Arc<Mutex<Vec<Note>>> = Arc::default();
    // The package's own tests (§87) run with its examples, from the author's folder.
    let tests = PackageTests::of(folder, ready.python.clone());
    let examples = examples_of(&temp.0, &python, &manifest.id, &heard, tests);
    // A traceback names the files where the author edits them, not the test's installed copy.
    let (installed, own) = (package.display().to_string(), folder.display().to_string());
    let notes: Vec<String> = heard
        .lock()
        .map(|n| {
            n.iter()
                .filter(|n| matches!(n, Note::Failed { .. }))
                .map(|n| n.said().replace(&installed, &own).trim_end().to_owned())
                .collect()
        })
        .unwrap_or_default();
    report["notes"] = json!(notes);
    let Some(mut examples) = examples else {
        let said = "Araçlarda, denetim kurallarında ve ifade işlevlerinde örnek, tests/ klasöründe sınama yok: eklenti \
                    doğrulandı ve ortamı hazırlandı, çalıştırılacak bir şey yok. Aracın örneği tanımında (examples), \
                    kuralınki kendi dosyasında (entrypoints.audits[].examples), ifadeninki manifestte \
                    (entrypoints.expressions[].examples); sınamalar tests/test_*.py'de, unittest ile.";
        return end(report, json, "passed", said, 0);
    };
    for row in examples["examples"].as_array_mut().into_iter().flatten() {
        if let Some(detail) = row["detail"].as_str() {
            row["detail"] = json!(detail.replace(&installed, &own));
        }
    }
    let listed = examples["examples"]
        .as_array()
        .is_some_and(|l| !l.is_empty());
    if !json {
        if listed {
            say!("Örnekler:");
        }
        for e in examples["examples"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
        {
            let passed = e["status"] == "passed";
            say!(
                "  {} {} › {} ({})",
                if passed { "✓" } else { "✗" },
                e["label"].as_str().unwrap_or_default(),
                e["title"].as_str().unwrap_or_default(),
                seconds(e["ms"].as_u64().unwrap_or_default())
            );
            for p in e["problems"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default()
            {
                say!("      • {}", p.as_str().unwrap_or_default());
            }
            // A rule's code failed: its traceback, the files named where the author edits them.
            if let Some(detail) = e["detail"].as_str() {
                say!("        {}", detail.trim_end().replace('\n', "\n        "));
            }
        }
        if let Some(t) = examples.get("tests") {
            say!(
                "Sınamalar (tests/, unittest): {} ({})",
                t["summary"].as_str().unwrap_or_default(),
                seconds(t["ms"].as_u64().unwrap_or_default())
            );
            if t["status"] != "passed" {
                say!(
                    "  {}",
                    t["output"]
                        .as_str()
                        .unwrap_or_default()
                        .trim_end()
                        .replace('\n', "\n  ")
                );
            }
        }
        if !notes.is_empty() {
            say!("Eklentinin söyledikleri:");
            for n in &notes {
                say!("  • {}", n.replace('\n', "\n    "));
            }
        }
    }
    let failed = examples["failed"].as_u64().unwrap_or_default();
    let tests_failed = kentos_extension_tools::examples::tests_failed(&examples);
    let said = examples["summary"].as_str().unwrap_or_default().to_owned();
    if let Some(tests) = examples.get("tests").cloned() {
        report["tests"] = tests;
    }
    report["examples"] = examples;
    if failed == 0 && !tests_failed {
        end(report, json, "passed", &said, 0)
    } else {
        end(report, json, "failed", &said, 1)
    }
}

/// The report's end: its status and summary, written as asked, and the exit code.
fn end(mut report: Value, json: bool, status: &str, said: &str, code: u8) -> ExitCode {
    report["status"] = json!(status);
    report["summary"] = json!(said);
    if json {
        say!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
    } else {
        say!("Sonuç: {said}");
    }
    ExitCode::from(code)
}

/// The examples of extension `ext`'s tools run as the desktop runs them, in its version's process, every tool listed
/// (`Headless::every_runtime`); the report, none when no tool gives an example. What a run notes goes to `heard`.
fn examples_of(
    root: &Path,
    python: &Python,
    ext: &str,
    heard: &Arc<Mutex<Vec<Note>>>,
    tests: Option<PackageTests>,
) -> Option<Value> {
    kentos_processing::command::register();
    let into = Arc::clone(heard);
    let (registry, given) = (root.to_path_buf(), python.clone());
    offer_with(move || {
        Headless::new(
            Some(registry),
            Some(given.root.clone()),
            Arc::new(move |n| {
                if let Ok(mut heard) = into.lock() {
                    heard.push(n);
                }
            }),
        )
        .with_python(given)
        .every_runtime()
    });
    let tools = kentos_extension_tools::examples::of(ext);
    let pool = Arc::new(Pool::default());
    let (rules, expressions) = match coded_of(root, python, ext, &pool) {
        Ok(coded) => coded,
        Err(why) => {
            return Some(
                json!({ "status": "failed", "failed": 1, "summary": why, "examples": [] }),
            );
        }
    };
    let all = Examples {
        tools,
        rules,
        expressions,
        tests,
    };
    if all.is_empty() {
        return None;
    }
    let head = json!({ "kind": "examples", "extension": { "id": ext } });
    // A tool's code may ask for a built-in tool (§47): a command inside the command, on a deep stack as on the desktop.
    let report = std::thread::Builder::new()
        .name("kentos-kek-test".into())
        .stack_size(kentos_extension_tools::calls::STACK)
        .spawn(move || kentos_extension_tools::examples::run(&all, head, &AtomicBool::new(false)))
        .ok()
        .and_then(|t| t.join().ok());
    kentos_extension_tools::headless::close();
    pool.close_all();
    Some(report.unwrap_or_else(|| {
        json!({ "status": "failed", "failed": 1, "summary": "Örnek koşusu beklenmedik biçimde durdu.", "examples": [] })
    }))
}

/// How long a rule's example may take, as a rule on the desktop.
const RULE_TIMEOUT: Duration = Duration::from_secs(120);

/// What `ext` installed in the test's registry `root` gives examples of besides its tools: its audit rules' (§75) and
/// its expression functions' (§78), each run in its version's process in `pool` as the desktop runs it; why not, when
/// a rule's examples cannot be read.
fn coded_of(
    root: &Path,
    python: &Python,
    ext: &str,
    pool: &Arc<Pool>,
) -> Result<(Vec<Rule>, Vec<Expression>), String> {
    let registry = kentos_extension_host::places::open_to_read(root, env!("CARGO_PKG_VERSION"))
        .map_err(|p| format!("Sınamanın kayıt defteri okunamadı: {p}"))?;
    // A contribution's run in its version's process: its plan found now, its process started at the first call.
    let runner = |local: &str, handler: &str| -> Result<RunRule, String> {
        let plan = Plan::of(&registry, ext, local, handler)
            .ok_or_else(|| format!("“{local}” kurulu değil."))?;
        let (pool, python) = (Arc::clone(pool), python.clone());
        Ok(Box::new(move |invoke| {
            let start = || {
                kentos_extension_host::verify_installed(&plan.source, &plan.digest)?;
                start_of(&plan, &python, Library::IfFound)
            };
            run_in_pool(
                &pool,
                &plan.key(),
                &plan,
                &start,
                invoke,
                RULE_TIMEOUT,
                &|| false,
            )
        }))
    };
    let mut rules = Vec::new();
    for (audit, read) in kentos_extension_tools::examples::rule_examples(&registry, ext) {
        let examples =
            read.map_err(|why| format!("“{}” kuralının örnekleri okunamadı: {why}", audit.id))?;
        rules.push(Rule {
            id: format!("ext.{ext}.{}", audit.id),
            title: audit.title.clone(),
            examples,
            run: runner(&audit.id, &audit.handler)?,
        });
    }
    let mut expressions = Vec::new();
    for entry in kentos_extension_tools::examples::expression_examples(&registry, ext) {
        expressions.push(Expression {
            id: format!("ext.{ext}.{}", entry.id),
            title: entry.title.clone().unwrap_or_else(|| entry.id.clone()),
            run: runner(&entry.id, &entry.handler)?,
            examples: entry.examples,
        });
    }
    Ok((rules, expressions))
}
