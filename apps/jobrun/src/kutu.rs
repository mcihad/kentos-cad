//! The cloud audit box (docs/adr/8057 §61, §63): on Linux the rule's Python runs in a Landlock domain behind a seccomp
//! filter. The job runner's box mode (`kentos-jobrun --kutu …`, [`enter`]) is a fresh process of one thread: it builds
//! the ruleset, restricts itself, installs the filter and then executes the interpreter, which keeps both, as does
//! everything it starts; nothing can lift them. The job runner itself stays outside, so it can still read `/proc` for
//! the budget and write the job's answer.
//!
//! - **Files** ([`allowed`]): read and execute only beneath the package's unpacked folder, the prepared environment
//!   (its folder, the base installation `pyvenv.cfg` names, a folder a `.pth` file adds that holds KentOS's library:
//!   an editable SDK), the resolved interpreter, the system's library folders and the loader's cache, `/dev/urandom`;
//!   write only beneath the job's own scratch: `pyc` (the bytecode) and `tmp` (the rule's `TMPDIR`), and `/dev/null`.
//!   The drawing, the request and the package file in the job's folder are not readable. A `.pth` file can come with
//!   a dependency the extension's author locked: what else it names, the box does not open.
//! - **Network** (`seccomp`): no socket. `socket(2)` is refused for every family, `socketpair(2)` but for a UNIX
//!   stream or sequenced-packet pair, which stays inside the box (asyncio's self-pipe), and `io_uring_setup(2)`, whose
//!   rings can open sockets. Landlock closes TCP as well (ABI 4) and keeps signals and abstract UNIX sockets inside the
//!   domain (ABI 6).
//! - **Where the box cannot be whole** ([`plan`]: no Landlock or an ABI below 4, no seccomp filter, an architecture
//!   other than x86_64 and aarch64, not Linux), the server's setting decides ([`POLICY_ENV`]): `require`, the default,
//!   refuses to run the rule; `report` runs it with what closes (Landlock without the filter, or nothing) and the job's
//!   summary says what is open.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The box mode's first argument.
pub const ARG: &str = "--kutu";
/// The box mode's word for Landlock without the filter: `report` where the filter cannot be installed.
pub const NO_FILTER: &str = "--suzgecsiz";
/// The server's setting, passed to the job runner: `require` (or unset) or `report`.
pub const POLICY_ENV: &str = "KENTOS_AUDIT_CONFINE";
/// What the box mode writes before it gives up, so the job says why.
pub const MARK: &str = "KENTOS-KUTU";
/// The job's scratch beside the bytecode: the rule's `TMPDIR`.
pub const SCRATCH: &str = "tmp";

/// What the server does where the box cannot be whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    /// The rule does not run (the default: the cloud box's promise holds or nothing runs).
    Require,
    /// The rule runs with what closes and the job says what is open.
    Report,
}

/// The server's choice, from its setting; anything but `report` requires the box.
pub fn policy() -> Policy {
    match std::env::var(POLICY_ENV).as_deref() {
        Ok("report") => Policy::Report,
        _ => Policy::Require,
    }
}

/// What Landlock closes here beyond the files and TCP it must (ABI 4): signals and abstract UNIX sockets (ABI 6),
/// pathname UNIX sockets (ABI 9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Landlock {
    pub scopes: bool,
    pub unix_paths: bool,
}

/// How the rule's process starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Plan {
    /// In the whole box, Landlock and the filter; what it closes, in words for the job runner's error output.
    Whole(String),
    /// `report` where the filter cannot be installed: in Landlock alone; why, and what is open.
    LandlockOnly(String),
    /// `report` where Landlock cannot close: outside the box; why.
    Outside(String),
}

impl Plan {
    /// What the job's summary adds: nothing for the whole box, what is open for any other.
    pub fn note(&self) -> Option<String> {
        match self {
            Plan::Whole(_) => None,
            Plan::LandlockOnly(open) => Some(format!(" (kutu tam kapatılamadı: {open})")),
            Plan::Outside(why) => Some(format!(" (kutu kapatılamadı: {why})")),
        }
    }
}

/// The plan from what this system closes and the server's policy, or why the rule may not run. A missing filter
/// counts as a box that cannot close, as missing Landlock does: without it UDP and the other socket kinds are open,
/// and the box's promise is no network. Under `report` the rule keeps what does close.
pub fn plan(
    landlock: Result<Landlock, String>,
    filter: Result<(), String>,
    policy: Policy,
) -> Result<Plan, String> {
    let why = match (&landlock, &filter) {
        (Ok(landlock), Ok(())) => return Ok(Plan::Whole(whole(*landlock))),
        (Ok(_), Err(filter)) => filter.clone(),
        (Err(landlock), Ok(())) => landlock.clone(),
        (Err(landlock), Err(filter)) => format!("{landlock}; {filter}"),
    };
    match (policy, landlock) {
        (Policy::Require, _) => Err(why),
        (Policy::Report, Ok(landlock)) => {
            Ok(Plan::LandlockOnly(format!("{why}; {}", alone(landlock))))
        }
        (Policy::Report, Err(_)) => Ok(Plan::Outside(why)),
    }
}

/// This system's plan under the server's setting.
pub fn here() -> Result<Plan, String> {
    plan(landlock(), filter(), policy())
}

/// What the whole box closes, in words.
fn whole(landlock: Landlock) -> String {
    let signals = if landlock.scopes {
        "sinyaller kutuda"
    } else {
        "sinyaller açık (Landlock ABI 6 yok)"
    };
    format!(
        "dosyalar kapalı; ağ kapalı (soket açılamaz): TCP, UDP, yol adlı ve soyut UNIX soketleri, öteki türler, \
         io_uring; yalnız kutunun içinde kalan UNIX akış çifti açılır; {signals}"
    )
}

/// What Landlock alone closes, in words.
fn alone(landlock: Landlock) -> String {
    let scopes = if landlock.scopes {
        "sinyaller ve soyut UNIX soketleri kutuda"
    } else {
        "sinyaller ve soyut UNIX soketleri açık (ABI 6 yok)"
    };
    let unix = if landlock.unix_paths {
        "yol adlı UNIX soketleri kapalı"
    } else {
        "yol adlı UNIX soketleri açık (ABI 9 yok)"
    };
    format!("dosyalar ve TCP kapalı; {scopes}; {unix}; UDP ve öteki soket türleri açık")
}

/// What Landlock closes on this system, or why it cannot close.
pub fn landlock() -> Result<Landlock, String> {
    #[cfg(target_os = "linux")]
    {
        linux::landlock()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err("Landlock yalnız Linux'ta var".to_owned())
    }
}

/// Whether the seccomp filter can be installed on this system, or why not: Linux on x86_64 or aarch64, a kernel
/// that has the filter's actions.
pub fn filter() -> Result<(), String> {
    #[cfg(all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ))]
    {
        seccomp::available()
    }
    #[cfg(not(all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )))]
    {
        Err(format!(
            "seccomp süzgeci yalnız Linux'ta x86_64 ve aarch64 için var; bu sistem {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
    }
}

/// What the box lets the rule's processes touch: existing paths, without repeats.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Allowed {
    pub read: Vec<PathBuf>,
    pub write: Vec<PathBuf>,
}

/// The paths a rule of the package at `source`, run by `python` in the job's folder `dir`, needs.
pub fn allowed(dir: &Path, python: &Path, source: &Path) -> Allowed {
    let mut read = vec![source.to_path_buf(), python.to_path_buf()];
    // The environment: …/bin/python's folder; the base installation `home` names; a folder its `.pth` files add that
    // holds KentOS's library (an editable SDK). Any other folder a `.pth` names stays shut: a dependency's `.pth` is
    // the extension author's text.
    if let Some(env) = python.parent().and_then(Path::parent) {
        read.push(env.to_path_buf());
        if let Ok(cfg) = std::fs::read_to_string(env.join("pyvenv.cfg")) {
            for line in cfg.lines() {
                if let Some((key, value)) = line.split_once('=')
                    && key.trim() == "home"
                    && let Some(base) = Path::new(value.trim()).parent()
                {
                    read.push(base.join("lib"));
                    read.push(base.join("lib64"));
                }
            }
        }
        for lib in ["lib", "lib64"] {
            let Ok(versions) = std::fs::read_dir(env.join(lib)) else {
                continue;
            };
            for site in versions.flatten().map(|v| v.path().join("site-packages")) {
                let Ok(files) = std::fs::read_dir(&site) else {
                    continue;
                };
                for pth in files.flatten().map(|f| f.path()) {
                    if pth.extension().is_some_and(|e| e == "pth")
                        && let Ok(text) = std::fs::read_to_string(&pth)
                    {
                        read.extend(
                            text.lines()
                                .map(str::trim)
                                .filter(|l| l.starts_with('/'))
                                .map(PathBuf::from)
                                .filter(|p| p.join("kentos").join("__init__.py").is_file()),
                        );
                    }
                }
            }
        }
    }
    // The interpreter as it resolves and its installation's libraries.
    if let Ok(real) = std::fs::canonicalize(python) {
        if let Some(prefix) = real.parent().and_then(Path::parent) {
            read.push(prefix.join("lib"));
            read.push(prefix.join("lib64"));
        }
        read.push(real);
    }
    for system in [
        "/usr/lib",
        "/usr/lib64",
        "/lib",
        "/lib64",
        "/usr/local/lib",
        "/etc/ld.so.cache",
        "/dev/urandom",
    ] {
        read.push(PathBuf::from(system));
    }
    let write = vec![
        dir.join("pyc"),
        dir.join(SCRATCH),
        PathBuf::from("/dev/null"),
    ];
    let keep = |paths: Vec<PathBuf>| {
        let mut out: Vec<PathBuf> = Vec::new();
        for p in paths {
            if p.exists() && !out.contains(&p) {
                out.push(p);
            }
        }
        out
    };
    Allowed {
        read: keep(read),
        write: keep(write),
    }
}

/// The box mode's command line: `program --kutu [--suzgecsiz] --oku <p>… --yaz <p>… --tmp <dir> -- <python>
/// <args>…`; without `filter`, Landlock alone.
pub fn wrap(
    program: &Path,
    allowed: &Allowed,
    filter: bool,
    tmp: &Path,
    python: &Path,
    args: &[OsString],
) -> (PathBuf, Vec<OsString>) {
    let mut out: Vec<OsString> = vec![ARG.into()];
    if !filter {
        out.push(NO_FILTER.into());
    }
    for p in &allowed.read {
        out.push("--oku".into());
        out.push(p.into());
    }
    for p in &allowed.write {
        out.push("--yaz".into());
        out.push(p.into());
    }
    out.push("--tmp".into());
    out.push(tmp.into());
    out.push("--".into());
    out.push(python.into());
    out.extend(args.iter().cloned());
    (program.to_path_buf(), out)
}

/// The program whose box mode starts the rule: this job runner. In the crate's tests, the job runner's binary beside
/// the test's, built once.
#[cfg(not(test))]
pub fn program() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|e| format!("iş çalıştırıcısının yeri bulunamadı: {e}"))
}

#[cfg(test)]
pub fn program() -> Result<PathBuf, String> {
    Ok(tests::job_runner())
}

/// The box mode: the arguments after [`ARG`]. Restricts this process, installs the filter (unless [`NO_FILTER`]) and
/// executes the interpreter; returns (an exit code) only if it could not, having said why (with [`MARK`]) on its
/// errors.
pub fn enter(args: impl Iterator<Item = OsString>) -> i32 {
    let mut read = Vec::new();
    let mut write = Vec::new();
    let mut tmp = None;
    let mut filter = true;
    let mut args = args;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--oku") => read.extend(args.next().map(PathBuf::from)),
            Some("--yaz") => write.extend(args.next().map(PathBuf::from)),
            Some("--tmp") => tmp = args.next().map(PathBuf::from),
            Some(NO_FILTER) => filter = false,
            Some("--") => break,
            _ => {
                eprintln!("{MARK}: kutunun bilinmeyen sözü: {}", arg.to_string_lossy());
                return 126;
            }
        }
    }
    let Some(python) = args.next() else {
        eprintln!("{MARK}: kutuda çalışacak yorumlayıcı verilmedi");
        return 126;
    };
    let rest: Vec<OsString> = args.collect();
    #[cfg(target_os = "linux")]
    {
        if let Err(why) = linux::restrict(&Allowed { read, write }) {
            eprintln!("{MARK}: kutu kapatılamadı: {why}");
            return 126;
        }
        if filter && let Err(why) = install_filter() {
            eprintln!("{MARK}: seccomp süzgeci kurulamadı: {why}");
            return 126;
        }
        use std::os::unix::process::CommandExt;
        let mut command = std::process::Command::new(&python);
        command.args(&rest).env_clear();
        if let Some(tmp) = &tmp {
            command.env("TMPDIR", tmp);
        }
        let failed = command.exec();
        eprintln!(
            "{MARK}: {} kutuda çalıştırılamadı: {failed}",
            Path::new(&python).display()
        );
        126
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (read, write, tmp, rest, filter);
        eprintln!(
            "{MARK}: kutu kapatılamadı: yalnız Linux'ta (Landlock); {} çalıştırılmadı",
            Path::new(&python).display()
        );
        126
    }
}

/// Installs the seccomp filter on this thread (the box mode's only one), or says why it cannot.
#[cfg(target_os = "linux")]
fn install_filter() -> Result<(), String> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        seccomp::install()
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        filter()
    }
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
mod seccomp;
#[cfg(test)]
pub(crate) mod tests;
