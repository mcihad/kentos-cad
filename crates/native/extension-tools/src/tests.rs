//! The extensions' tools in a headless host (docs/adr/8057 §70) against the SDK's own blueprint installed in a registry
//! of the tests' own, its environment's Python a stand-in for `kentos.ext_host` (the system's Python 3 running a small
//! script that speaks its lines): listed only when the descriptor declares `headless-python`, run through İşlemler'
//! command as the desktop runs it, a built-in tool its code asks for answered, a desktop-only tool refused before any
//! process starts, the package's files checked before its code runs. Without a Python 3 the tests say so and pass.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use kentos_extension_host::{Choice, Grant, Options, Registry};
use kentos_extensions::blueprint::{Ask, Contribution, blueprint};
use kentos_extensions::manifest::Module;
use kentos_extensions::version::Version;
use kentos_processing::Feedback;
use kentos_processing::external::{self, Failure, Provider};
use kentos_python_env::extension::{self, Ready};
use kentos_python_env::python::Interpreter;
use serde_json::{Value, json};

use crate::headless::{Headless, Hook, find_python};
use crate::note::Note;
use crate::plan::Python;

const EXT: &str = "tr.ornek.basiz";
const TOOL: &str = "ext.tr.ornek.basiz.alan-ozeti";
const DESKTOP_ONLY: &str = "ext.tr.ornek.basiz.alan-ozeti-masaustu";

/// Speaks `kentos.ext_host`'s lines for the blueprint's tool: says how far it is, logs and prints, asks KentOS for a
/// buffer of its inputs (§47), measures them and answers with its descriptor's outputs. What it was asked to run is
/// written beside it.
const STAND_IN: &str = include_str!("tests/stand_in.py");

/// A folder removed when dropped.
struct Temp(PathBuf);

impl Temp {
    fn new(name: &str) -> Temp {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let dir = std::env::temp_dir().join(format!(
            "kentos-eklenti-basiz-{name}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("a folder");
        Temp(dir)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        // The installed files are read-only: the registry's own removal.
        let _ = kentos_extension_host::store::remove_tree(&self.0);
    }
}

/// The system's Python 3; none without one.
fn python3() -> Option<PathBuf> {
    ["python3", "python"].into_iter().find_map(|name| {
        let ok = std::process::Command::new(name)
            .arg("-c")
            .arg("import sys; sys.exit(0 if sys.version_info >= (3, 8) else 1)")
            .status()
            .is_ok_and(|s| s.success());
        ok.then(|| PathBuf::from(name))
    })
}

/// What a host was told and what İşlemler heard.
#[derive(Default)]
struct Heard {
    notes: Vec<Note>,
    lines: Vec<String>,
    progress: Vec<(f64, String)>,
}

struct Listener(Arc<Mutex<Heard>>);

impl Feedback for Listener {
    fn progress(&mut self, fraction: f64, label: &str) {
        self.0
            .lock()
            .unwrap()
            .progress
            .push((fraction, label.to_owned()));
    }
    fn info(&mut self, message: String) {
        self.0.lock().unwrap().lines.push(message);
    }
    fn warn(&mut self, message: String) {
        self.0.lock().unwrap().lines.push(message);
    }
    fn canceled(&self) -> bool {
        false
    }
}

/// The blueprint of an extension with one İşlemler tool, and a copy of it that declares the desktop alone: installed
/// (everything granted, Python chosen) in `root`'s registry, its environment in `python_root` with the stand-in as its
/// Python. The extension's package folder.
fn install(root: &Path, python_root: &Path, python: &Path) -> PathBuf {
    let mut files = blueprint(&Ask {
        id: EXT.into(),
        name: "Başsız".into(),
        summary: "Başsız konakta çalışan bir İşlemler aracı.".into(),
        vertical: false,
        module: Module::Cbs,
        contributions: vec![Contribution::Processing],
    })
    .expect("a blueprint");
    let mut descriptor: Value = serde_json::from_str(&files["tools/alan-ozeti.tool.json"]).unwrap();
    assert_eq!(
        descriptor["capabilities"]["runtimes"],
        json!(["desktop-python", "headless-python"]),
        "a new extension's tool runs headless too"
    );
    descriptor["id"] = json!("alan-ozeti-masaustu");
    descriptor["capabilities"]["runtimes"] = json!(["desktop-python"]);
    files.insert(
        "tools/alan-ozeti-masaustu.tool.json".into(),
        serde_json::to_string_pretty(&descriptor).unwrap(),
    );
    let mut manifest: Value = serde_json::from_str(&files["manifest.json"]).unwrap();
    let mut second = manifest["entrypoints"]["processing"][0].clone();
    second["id"] = json!("alan-ozeti-masaustu");
    second["descriptor"] = json!("tools/alan-ozeti-masaustu.tool.json");
    manifest["entrypoints"]["processing"]
        .as_array_mut()
        .unwrap()
        .push(second);
    files.insert(
        "manifest.json".into(),
        serde_json::to_string_pretty(&manifest).unwrap(),
    );
    install_files(root, python_root, python, &files)
}

/// `files` (a working folder's, path → text) installed (everything granted, Python chosen) in `root`'s registry, its
/// environment in `python_root` with the stand-in as its Python. The extension's package folder.
fn install_files(
    root: &Path,
    python_root: &Path,
    python: &Path,
    files: &std::collections::BTreeMap<String, String>,
) -> PathBuf {
    let folder = root.with_extension("kaynak");
    for (path, text) in files {
        let at = folder.join(path);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(at, text).unwrap();
    }
    let host = kentos_extensions::Host::this(Version::parse(env!("CARGO_PKG_VERSION")).unwrap());
    let mut registry = Registry::open(root, host, Options::default()).expect("a registry");
    let granted = kentos_extensions::Manifest::read(&files["manifest.json"])
        .unwrap()
        .capabilities()
        .into_iter()
        .collect();
    let choice = Choice {
        grant: Grant {
            capabilities: granted,
            python: true,
        },
        enable: true,
        replace_source: false,
    };
    registry
        .install_folder(&folder, &choice)
        .unwrap_or_else(|p| panic!("{p:#?}"));
    let package = registry.package_dir(EXT).expect("installed");
    // Its environment, as the desktop leaves a ready one: the stand-in is its Python. Installed from a working folder,
    // it is a development environment (docs/adr/8057 §97).
    let interpreter = Interpreter {
        version: "3.12.13".into(),
        path: python_root.join("yorumlayici/cpython-3.12/bin/python3.12"),
    };
    let key = extension::key_for(&package, "uv.lock", EXT, "0.1.0", &interpreter, &[])
        .unwrap()
        .for_development(true);
    let dir = extension::place(python_root, &key);
    std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join("host.py");
    std::fs::write(&script, STAND_IN).unwrap();
    let launcher = dir.join("python");
    std::fs::write(
        &launcher,
        format!(
            "#!/bin/sh\nexec \"{}\" -u \"{}\"\n",
            python.display(),
            script.display()
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o755)).unwrap();
    let ready = Ready {
        format: "kentos.extension-env".into(),
        key,
        python: launcher,
        packages: Vec::new(),
        sdk: String::new(),
        made_at: 1,
    };
    std::fs::write(
        dir.join(extension::RECORD),
        serde_json::to_string(&ready).unwrap(),
    )
    .unwrap();
    package
}

/// KentOS's Python as the tests give it: no uv runs.
fn given(python_root: &Path) -> Python {
    Python {
        interpreter: Interpreter {
            version: "3.12.13".into(),
            path: python_root.join("yorumlayici/cpython-3.12/bin/python3.12"),
        },
        root: python_root.to_path_buf(),
        uv: PathBuf::from("/yok/uv"),
    }
}

fn hook(heard: &Arc<Mutex<Heard>>) -> Hook {
    let heard = Arc::clone(heard);
    Arc::new(move |note| heard.lock().unwrap().notes.push(note))
}

fn square(x: f64, side: f64) -> Value {
    json!({ "type": "Polygon",
            "coordinates": [[[x, 0.0], [x + side, 0.0], [x + side, side], [x, side], [x, 0.0]]] })
}

/// The stand-in's inputs as İşlemler gives a tool's: two parcels, each with its persistent id and shape.
fn input() -> Value {
    let feature = |n: u32, g: Value| {
        json!({ "uid": format!("00000000-0000-0000-0000-00000000000{n}"), "layer": "Parsel", "geometry": g,
                "entity": { "kind": "polygon", "attrs": { "no": n.to_string() } } })
    };
    json!({ "params": {}, "layers": {}, "srid": 5256,
            "inputs": { "alanlar": { "description": "2 alan",
                                      "features": [feature(1, square(0.0, 10.0)), feature(2, square(50.0, 20.0))] } } })
}

/// A headless host lists the tool that declares `headless-python` and runs it in its version's process as the desktop
/// does: what the code says reaches İşlemler' feedback, the built-in tool its code asks for runs on the code's own
/// objects (§47), its outputs come back whole; the desktop-only tool is not listed and says what its descriptor needs.
#[test]
fn a_headless_host_runs_the_tools_that_declare_it() {
    let Some(python) = python3() else {
        eprintln!("Python 3 yok: sınama atlandı.");
        return;
    };
    kentos_processing::command::register();
    let temp = Temp::new("calisir");
    let (root, python_root) = (temp.0.join("eklentiler"), temp.0.join("python"));
    install(&root, &python_root, &python);
    let heard = Arc::new(Mutex::new(Heard::default()));
    let headless = Headless::new(Some(root.clone()), Some(python_root.clone()), hook(&heard))
        .with_python(given(&python_root));
    assert!(headless.read(), "the registry is read");
    assert!(!headless.read(), "not again while nothing changed");
    let tools = headless.tools();
    let ids: Vec<&str> = tools.iter().map(|t| t.tool.id.as_str()).collect();
    assert_eq!(ids, [TOOL], "only the tool that declares headless-python");
    assert!(tools[0].headless);
    assert!(
        tools[0].digest.starts_with("sha256:"),
        "a model's step pins it"
    );
    assert!(tools[0].environment.starts_with("Python 3.12.13 · "));
    let unavailable = headless.unavailable();
    let why = unavailable
        .get(DESKTOP_ONLY)
        .expect("the desktop-only tool");
    assert!(
        why.contains("yalnız KentOS masaüstünde")
            && why.contains("tools/alan-ozeti-masaustu.tool.json")
            && why.contains("headless-python"),
        "{why}"
    );
    // A test runner's host (`kentos kek test`, §74) lists the desktop-only tool too: its code runs the same way.
    let every = Headless::new(Some(root.clone()), Some(python_root.clone()), hook(&heard))
        .with_python(given(&python_root))
        .every_runtime();
    assert!(every.read());
    let ids: Vec<String> = every.tools().into_iter().map(|t| t.tool.id).collect();
    assert_eq!(ids, [TOOL, DESKTOP_ONLY]);
    assert!(every.unavailable().is_empty());
    let mut listener = Listener(Arc::clone(&heard));
    let answer = headless
        .run(&tools[0], input(), &mut listener)
        .unwrap_or_else(|f| panic!("{f:?}"));
    assert_eq!(answer["outputs"]["toplam"], json!(500.0), "{answer}");
    assert_eq!(answer["outputs"]["tablo"]["rows"][1], json!([2, 400.0]));
    assert_eq!(
        answer["summary"], "2 alan; tampon 2",
        "the buffer it asked for ran on its own objects"
    );
    let heard = heard.lock().unwrap();
    assert!(
        heard
            .progress
            .iter()
            .any(|(f, t)| *f == 0.5 && t == "ölçülüyor"),
        "{:?}",
        heard.progress
    );
    assert!(heard.lines.iter().any(|l| l == "eklenti: tr.ornek.basiz"));
    assert!(heard.lines.iter().any(|l| l == "yazdırılan satır"));
    assert!(
        matches!(&heard.notes[0], Note::Started { part, .. } if part == "processing:alan-ozeti")
    );
    assert!(
        matches!(
            &heard.notes[1],
            Note::Worked {
                code: "run_finished",
                ..
            }
        ),
        "{:?}",
        heard.notes
    );
    drop(heard);
    // The extension's log keeps the run as the desktop writes it (docs/adr/8057 §73): Ayrıntı reads it there.
    let log = |n: usize| kentos_extension_host::Journal::new(&root).tail(Some(EXT), n);
    let kept: Vec<(String, Option<String>)> =
        log(2).into_iter().map(|r| (r.code, r.version)).collect();
    let version = Some("0.1.0".to_owned());
    assert_eq!(
        kept,
        [
            ("run_started".to_owned(), version.clone()),
            ("run_finished".to_owned(), version)
        ]
    );
    // Its code fails: the caller gets the sentence, the log the traceback and what it printed.
    let mut failing = input();
    failing["params"] = json!({ "hata": true });
    let mut listener = Listener(Arc::new(Mutex::new(Heard::default())));
    let Err(Failure::Failed(why)) = headless.run(&tools[0], failing, &mut listener) else {
        panic!("it failed");
    };
    assert_eq!(why, "“Alan özeti” hata verdi: ValueError: istenen hata");
    let failed = log(1).remove(0);
    assert_eq!(
        (failed.level, failed.code.as_str()),
        (kentos_extension_host::Level::Error, "run_failed")
    );
    let detail = failed.detail.unwrap_or_default();
    assert!(
        detail.starts_with("ara çıktı") && detail.contains("ValueError: istenen hata"),
        "{detail}"
    );
    assert!(!failed.program.is_empty(), "which program wrote it");
    headless.close();
}

/// The package's files are checked against their digest before its code starts, as the desktop's activation checks
/// them: a file changed after installing refuses the run, naming it; nothing runs without KentOS's Python on.
#[test]
fn a_package_changed_after_installing_does_not_run() {
    let Some(python) = python3() else {
        eprintln!("Python 3 yok: sınama atlandı.");
        return;
    };
    let temp = Temp::new("degisti");
    let (root, python_root) = (temp.0.join("eklentiler"), temp.0.join("python"));
    let package = install(&root, &python_root, &python);
    let changed = package.join("src/basiz/logic.py");
    let mut permissions = std::fs::metadata(&changed).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    std::fs::set_permissions(&changed, permissions).unwrap();
    std::fs::write(&changed, "print('başka kod')\n").unwrap();
    let heard = Arc::new(Mutex::new(Heard::default()));
    let headless = Headless::new(Some(root), Some(python_root.clone()), hook(&heard))
        .with_python(given(&python_root));
    headless.read();
    let tool = headless.tools().remove(0);
    let mut listener = Listener(Arc::clone(&heard));
    let Err(Failure::Failed(why)) = headless.run(&tool, input(), &mut listener) else {
        panic!("refused");
    };
    assert!(
        why.contains("kurulduğu hâlinden farklı") && why.contains("src/basiz/logic.py"),
        "{why}"
    );
    assert_eq!(headless.running(), 0, "no process started");
    // Python off (the user's choice), or no data folder: said where it is turned on.
    let off = find_python(Some(&python_root)).unwrap_err();
    assert!(off.starts_with("Python kapalı"), "{off}");
    assert!(find_python(None).unwrap_err().contains("veri klasörü yok"));
}

/// An extension's expression function in a headless host (docs/adr/8057 §80): known to the expressions either way; run
/// only when the manifest says the extension runs headless (the blueprint does; a test runner's host runs it anyway),
/// otherwise refused where it is called with the reason and nothing started. A batch is answered in the version's
/// process, a row the code fails empty and the batch's failure noted once, in the extension's log too.
#[test]
fn an_expression_function_runs_headless_when_its_extension_says_so() {
    use kentos_expression::external::Arg;
    const FUNCTION: &str = "ext.tr.ornek.basiz.buyuk-harf";
    let Some(python) = python3() else {
        eprintln!("Python 3 yok: sınama atlandı.");
        return;
    };
    let text = |t: &str| Arg::Text(t.to_owned());
    let rows = [vec![text("istanbul")], vec![text("ılık")], vec![Arg::Null]];
    for headless_too in [false, true] {
        let temp = Temp::new(if headless_too {
            "ifade"
        } else {
            "ifade-masaustu"
        });
        let (root, python_root) = (temp.0.join("eklentiler"), temp.0.join("python"));
        let mut files = blueprint(&Ask {
            id: EXT.into(),
            name: "Başsız".into(),
            summary: "Başsız konakta çalışan bir ifade işlevi.".into(),
            vertical: false,
            module: Module::Cbs,
            contributions: vec![Contribution::Expression],
        })
        .expect("a blueprint");
        let mut manifest: Value = serde_json::from_str(&files["manifest.json"]).unwrap();
        assert_eq!(
            manifest["runtimes"],
            json!(["desktop", "headless"]),
            "a new extension's function runs headless too"
        );
        if !headless_too {
            manifest["runtimes"] = json!(["desktop"]);
            files.insert(
                "manifest.json".into(),
                serde_json::to_string_pretty(&manifest).unwrap(),
            );
        }
        install_files(&root, &python_root, &python, &files);
        let heard = Arc::new(Mutex::new(Heard::default()));
        let make = || {
            Headless::new(Some(root.clone()), Some(python_root.clone()), hook(&heard))
                .with_python(given(&python_root))
        };
        let headless = make();
        assert!(headless.read());
        let defs = headless.functions();
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, [FUNCTION]);
        let schema = kentos_expression::Schema {
            externals: defs.clone(),
            ..Default::default()
        };
        let compiled = kentos_expression::compile_with(&format!("{FUNCTION}(ad) || '!'"), &schema);
        if !headless_too {
            let why = compiled.expect_err("refused where it is called").message;
            assert!(
                why.starts_with(&format!("{FUNCTION}() burada çalışmaz: “Başsız” eklentisi"))
                    && why.contains("`headless`"),
                "{why}"
            );
            assert_eq!(headless.answer(FUNCTION, &rows), vec![Arg::Null; 3]);
            assert_eq!(headless.running(), 0, "nothing started");
            // A test runner's host (`kentos kek test`) runs it the same way.
            let every = make().every_runtime();
            assert!(every.read());
            assert!(every.functions()[0].refused.is_none());
            assert_eq!(every.answer(FUNCTION, &rows[..1]), [text("İSTANBUL")]);
            every.close();
            continue;
        }
        assert!(compiled.is_ok(), "{compiled:?}");
        assert_eq!(
            headless.answer(FUNCTION, &rows),
            [text("İSTANBUL"), text("ILIK"), Arg::Null]
        );
        assert_eq!(headless.running(), 1, "its version's process, kept");
        // A row the code fails: that row empty, the other answered, the batch's failure noted once.
        assert_eq!(
            headless.answer(FUNCTION, &[vec![text("hata")], vec![text("sivas")]]),
            [Arg::Null, text("SİVAS")]
        );
        let failed: Vec<String> = heard
            .lock()
            .unwrap()
            .notes
            .iter()
            .filter_map(|n| match n {
                Note::Failed { code, message, .. } if *code == "expression_failed" => {
                    Some(message.clone())
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            failed,
            [format!(
                "“{FUNCTION}”: 1 satır hata verdi: ValueError: istenen hata"
            )]
        );
        let logged = kentos_extension_host::Journal::new(&root)
            .tail(Some(EXT), 1)
            .remove(0);
        assert_eq!(
            (logged.code.as_str(), logged.version.as_deref()),
            ("expression_failed", Some("0.1.0"))
        );
        headless.close();
    }
}

/// Offered to the process, the tool is what `cad.processing.run` runs on a drawing, as on the desktop; a desktop-only
/// tool is refused there before any process starts; an extension turned off leaves the list and its process closes.
/// The process's lists are this test's alone: no other test here offers.
#[test]
fn offered_tools_run_through_the_commands_and_follow_the_registry() {
    let Some(python) = python3() else {
        eprintln!("Python 3 yok: sınama atlandı.");
        return;
    };
    kentos_processing::command::register();
    let temp = Temp::new("sunulur");
    let (root, python_root) = (temp.0.join("eklentiler"), temp.0.join("python"));
    install(&root, &python_root, &python);
    let heard = Arc::new(Mutex::new(Heard::default()));
    let note = hook(&heard);
    let (made_root, made_python) = (root.clone(), python_root.clone());
    assert!(crate::headless::offer_with(move || {
        Headless::new(Some(made_root), Some(made_python.clone()), note)
            .with_python(given(&made_python))
    }));
    let listed: Vec<String> = external::tools().into_iter().map(|t| t.id).collect();
    assert_eq!(listed, [TOOL]);
    let made = crate::synthetic::synthetic_on(
        &[
            json!({ "layer": "parsel", "geometry": square(0.0, 10.0), "attrs": { "no": "1" } }),
            json!({ "layer": "parsel", "geometry": square(50.0, 20.0), "attrs": { "no": "2" } }),
        ],
        5256,
    )
    .expect("a drawing");
    let mut doc = made.doc;
    let run = |doc: &mut kentos_domain::Document, tool: &str| {
        let envelope = serde_json::from_value::<kentos_commands::CallEnvelope>(json!({
            "command": "cad.processing.run", "op": "execute",
            "input": { "tool": tool, "parameters": { "alanlar": { "scope": "layer", "layerId": "parsel" } } }
        }))
        .unwrap();
        serde_json::to_value(kentos_headless::call_envelope(doc, None, envelope)).unwrap()
    };
    // The desktop-only tool first: refused with what its descriptor must say, and nothing started.
    let refused = run(&mut doc, DESKTOP_ONLY);
    assert_eq!(refused["status"], "failed", "{refused}");
    assert_eq!(refused["error"]["code"], "unknown_tool");
    let words = refused["error"]["message"].as_str().unwrap();
    assert!(
        words.contains("yalnız KentOS masaüstünde") && words.contains("headless-python"),
        "{words}"
    );
    let calls = |python_root: &Path| -> Vec<String> {
        let found: Vec<PathBuf> = std::fs::read_dir(python_root.join("ortamlar"))
            .unwrap()
            .flatten()
            .map(|e| e.path().join("cagrilar.txt"))
            .filter(|p| p.exists())
            .collect();
        found
            .iter()
            .flat_map(|p| {
                std::fs::read_to_string(p)
                    .unwrap()
                    .lines()
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .collect()
    };
    assert!(calls(&python_root).is_empty(), "no process asked");
    // The headless tool: run as the desktop runs it, its outputs İşlemler's; on a thread of a job's stack, the default
    // 2 MiB, as an agent's run goes. The overflow the agents' plan-and-execute had there in the debug MCP (the built-in
    // tool the code asks for, a whole command inside the tool's) was measured by hand (docs/adr/8057 §70); this
    // shorter path keeps the thread's size.
    let ran = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(2 << 20)
            .spawn_scoped(scope, || run(&mut doc, TOOL))
            .unwrap()
            .join()
            .unwrap()
    });
    assert_eq!(ran["status"], "completed", "{ran}");
    let text = ran.to_string();
    assert!(text.contains("2 alan; tampon 2"), "{text}");
    assert!(text.contains("500"), "{text}");
    assert_eq!(calls(&python_root), ["alan-ozeti"]);
    // Its examples (§48), as the desktop and `kentos kek test` run them: each on a drawing of its own; the one that
    // expects what the tool gives passes, the other says what it expected and what came.
    let mut tool = external::find(TOOL).expect("listed");
    let example = |title: &str, total: f64| {
        json!({ "title": title,
                "features": [{ "layer": "parsel", "geometry": square(0.0, 10.0) },
                             { "layer": "parsel", "geometry": square(50.0, 20.0) }],
                "params": { "alanlar": { "scope": "layer", "layerId": "parsel" } },
                "expect": { "outputs": { "toplam": total }, "summary": "2 alan" } })
    };
    tool.examples = vec![example("İki alan", 500.0), example("Yanlış toplam", 499.0)];
    let report = crate::examples::run(
        &crate::examples::Examples {
            tools: vec![tool],
            ..Default::default()
        },
        json!({ "kind": "examples" }),
        &AtomicBool::new(false),
    );
    assert_eq!(
        report["summary"], "2 örnekten 1 tanesi geçti, 1 tanesi kaldı.",
        "{report}"
    );
    assert_eq!(report["examples"][0]["status"], "passed", "{report}");
    let why = report["examples"][1]["problems"][0].as_str().unwrap();
    assert!(
        why.contains("“toplam”: 499") && why.contains("500"),
        "{why}"
    );
    // Turned off on the desktop: gone from the list at the next look, its process closed.
    let host = kentos_extensions::Host::this(Version::parse(env!("CARGO_PKG_VERSION")).unwrap());
    let mut registry = Registry::open(&root, host, Options::default()).unwrap();
    registry.disable(EXT).unwrap();
    crate::headless::refresh();
    assert!(external::tools().is_empty(), "turned off, not listed");
    assert!(external::unavailable(DESKTOP_ONLY).is_none());
    let gone = run(&mut doc, TOOL);
    assert_eq!(gone["error"]["code"], "unknown_tool", "{gone}");
    let offered = crate::headless::offered().expect("offered");
    assert_eq!(offered.running(), 0, "its process closed");
    crate::headless::close();
}

/// The Python library's look at the registry before a command that is not İşlemler' (docs/adr/8057 §80): the first
/// is due, the next ones not within two seconds, the one after them is.
#[test]
fn the_registry_is_read_at_most_every_two_seconds_between_commands() {
    let start = std::time::Instant::now();
    let mut last = None;
    assert!(crate::headless::due(&mut last, start));
    assert!(!crate::headless::due(
        &mut last,
        start + std::time::Duration::from_millis(1999)
    ));
    assert!(crate::headless::due(
        &mut last,
        start + std::time::Duration::from_secs(2)
    ));
    assert!(!crate::headless::due(
        &mut last,
        start + std::time::Duration::from_secs(3)
    ));
}
