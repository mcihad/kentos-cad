//! The standards packages' rules called from Rust (catalog/standards; plan/bim-plan.md K10, docs/adr/8063, 8075 §2.6):
//! the rule is Python, the same script a cloud job, the audit and the permit package run, and KentOS hands it data. One
//! call is one short process: `python -I -B -c <bootstrap> <package's src>`, the arguments as JSON on its input, the
//! function's answer as JSON on its output, stopped past [`MOST_TIME`]. The interpreter is [`interpreter`]: the rule
//! needs nothing beyond Python itself. Shared by the desktop, the sheet publication and the command line, so a rule is
//! asked the same way wherever a command runs.
//!
//! The package is found under `KENTOS_STANDARDS_DIR`, else in the checkout's `catalog/standards` above the program (a
//! test's program deeper in `target/` too), else, in a development build, in the checkout it was built from.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

/// The longest a rule may take: a call is a decision over a few facts, not a measure.
pub const MOST_TIME: Duration = Duration::from_secs(15);

/// Imports the module from the package's `src` (the first argument), calls the function with the input's `args`, and
/// writes its answer; an error is the exception's last line on standard error.
const BOOTSTRAP: &str = "import importlib, json, sys
sys.path.insert(0, sys.argv[1])
call = json.load(sys.stdin)
answer = getattr(importlib.import_module(call['module']), call['function'])(*call['args'])
sys.stdout.write(json.dumps(answer, ensure_ascii=False))
";

/// The Python a rule runs in: `KENTOS_PYTHON`, else the user's personal environment while Python is on and it is ready
/// (KentOS's own Python with KentOS's library; on Windows the only one there is), else a checkout's own environment
/// above the program (`.run/py`, which `scripts/python/test.sh` makes; the program is in `target/<profile>/`, a test in
/// its `deps/`), else `python3`.
pub fn interpreter() -> PathBuf {
    if let Some(path) = std::env::var_os("KENTOS_PYTHON") {
        return PathBuf::from(path);
    }
    if let Some(root) = kentos_python_env::places::root()
        && kentos_python_env::settings::PythonSettings::load(&root).enabled
        && let Some(own) = kentos_python_env::personal::ready(&root)
    {
        return own;
    }
    if let Ok(exe) = std::env::current_exe() {
        for folder in exe.ancestors().skip(1).take(4) {
            let own = folder.join(".run/py/bin/python");
            if own.exists() {
                return own;
            }
        }
    }
    PathBuf::from("python3")
}

/// The standards package `name`'s folder (`dijital-proje`).
pub fn package(name: &str) -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("KENTOS_STANDARDS_DIR") {
        let found = PathBuf::from(dir).join(name);
        return found.is_dir().then_some(found);
    }
    let near = |root: &std::path::Path| {
        let found = root.join("catalog/standards").join(name);
        found.join("manifest.json").is_file().then_some(found)
    };
    // An installation's resources (CLAUDE.md §26, docs/adr/9312): `<kaynak>/catalog/standards`.
    if let Some(found) = kentos_launch::install::resources().and_then(|r| near(&r)) {
        return Some(found);
    }
    // The program in `target/<profile>/`, a test's in `target/<profile>/deps/` or, with Cargo's newer build folder,
    // `target/<profile>/build/<crate>/<hash>/out/`.
    let exe = std::env::current_exe().ok();
    let found = exe
        .as_deref()
        .and_then(|exe| exe.ancestors().skip(1).take(8).find_map(near));
    // A development build knows its checkout (this crate is crates/native/standards).
    #[cfg(debug_assertions)]
    let found =
        found.or_else(|| near(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")));
    found
}

/// `module.function(*args)` of the package `name`, its answer as JSON. The error says why and what to do.
pub fn call(name: &str, module: &str, function: &str, args: Vec<Value>) -> Result<Value, String> {
    let dir = package(name).ok_or_else(|| {
        format!(
            "Standart kural paketi “{name}” bulunamadı: KentOS'un catalog/standards klasörünü KENTOS_STANDARDS_DIR ile \
             gösterin."
        )
    })?;
    let python = interpreter();
    let mut command = Command::new(&python);
    // Isolated (no user site, no PYTHON* settings) and without writing bytecode into the package's folder, which may be
    // read-only where KentOS is installed.
    command
        .arg("-I")
        .arg("-B")
        .arg("-c")
        .arg(BOOTSTRAP)
        .arg(dir.join("src"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Started from a windowed program, a console program would flash a black window.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command.spawn().map_err(|e| {
        format!(
            "Python başlatılamadı ({}): {e}. Ayarlar › Python'dan bir Python seçin.",
            python.display()
        )
    })?;
    let input =
        serde_json::json!({ "module": module, "function": function, "args": args }).to_string();
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input.as_bytes());
    }
    let reader = |pipe: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut text = String::new();
            if let Some(mut p) = pipe {
                let _ = p.read_to_string(&mut text);
            }
            text
        })
    };
    let out = reader(
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let err = reader(
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() > MOST_TIME => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "Kural {} saniyede bitmedi ve durduruldu ({name}: {module}.{function}).",
                    MOST_TIME.as_secs()
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(e) => return Err(format!("Kuralın süreci izlenemedi: {e}.")),
        }
    };
    let (out, err) = (
        out.join().unwrap_or_default(),
        err.join().unwrap_or_default(),
    );
    if !status.success() {
        let why = err
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("bilinmeyen hata");
        return Err(format!(
            "Kural hata verdi ({name}: {module}.{function}): {why}"
        ));
    }
    serde_json::from_str(&out)
        .map_err(|e| format!("Kuralın cevabı okunamadı ({name}: {module}.{function}): {e}."))
}

/// The standards package `name`'s version, as its manifest says it (`0.7.0`): what a publication records beside the
/// rule's answer.
pub fn version(name: &str) -> Option<String> {
    let text = std::fs::read_to_string(package(name)?.join("manifest.json")).ok()?;
    let manifest: Value = serde_json::from_str(&text).ok()?;
    manifest.get("version")?.as_str().map(str::to_owned)
}
