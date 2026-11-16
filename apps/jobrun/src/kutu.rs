//! The cloud audit box (docs/adr/8057 §61): on Linux the rule's Python runs in a Landlock domain. The job runner's box
//! mode (`kentos-jobrun --kutu …`, [`enter`]) is a fresh process of one thread: it builds the ruleset, restricts itself
//! and then executes the interpreter, which keeps the domain, as does everything it starts; nothing can lift it. The
//! job runner itself stays outside, so it can still read `/proc` for the budget and write the job's answer.
//!
//! - **Files** ([`allowed`]): read and execute only beneath the package's unpacked folder, the prepared environment
//!   (its folder, the base installation `pyvenv.cfg` names, a folder a `.pth` file adds that holds KentOS's library:
//!   an editable SDK), the resolved interpreter, the system's library folders and the loader's cache, `/dev/urandom`;
//!   write only beneath the job's own scratch: `pyc` (the bytecode) and `tmp` (the rule's `TMPDIR`), and `/dev/null`.
//!   The drawing, the request and the package file in the job's folder are not readable. A `.pth` file can come with
//!   a dependency the extension's author locked: what else it names, the box does not open.
//! - **Network:** TCP bind and connect denied (Landlock ABI 4); abstract UNIX sockets and signals scoped to the domain
//!   (ABI 6); connecting to a pathname UNIX socket denied where the kernel can (ABI 9). Not covered by Landlock: UDP
//!   and the other socket kinds, and pathname UNIX sockets below ABI 9.
//! - **Where it cannot be closed** (no Landlock, an ABI below 4, not Linux), the server's setting decides
//!   ([`POLICY_ENV`]): `require`, the default, refuses to run the rule; `report` runs it outside the box and says
//!   "kutu kapatılamadı" in the job's summary (and on the job runner's error output, as it says what a closed box
//!   closes).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The box mode's first argument.
pub const ARG: &str = "--kutu";
/// The server's setting, passed to the job runner: `require` (or unset) or `report`.
pub const POLICY_ENV: &str = "KENTOS_AUDIT_CONFINE";
/// What the box mode writes before it gives up, so the job says why.
pub const MARK: &str = "KENTOS-KUTU";
/// The job's scratch beside the bytecode: the rule's `TMPDIR`.
pub const SCRATCH: &str = "tmp";

/// What the server does where the box cannot be closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    /// The rule does not run (the default: the cloud box's promise holds or nothing runs).
    Require,
    /// The rule runs outside the box and the job says so.
    Report,
}

/// The server's choice, from its setting; anything but `report` requires the box.
pub fn policy() -> Policy {
    match std::env::var(POLICY_ENV).as_deref() {
        Ok("report") => Policy::Report,
        _ => Policy::Require,
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

/// The box mode's command line: `program --kutu --oku <p>… --yaz <p>… --tmp <dir> -- <python> <args>…`.
pub fn wrap(
    program: &Path,
    allowed: &Allowed,
    tmp: &Path,
    python: &Path,
    args: &[OsString],
) -> (PathBuf, Vec<OsString>) {
    let mut out: Vec<OsString> = vec![ARG.into()];
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

/// What the box closes on this system, in words for the job runner's error output; why it cannot be closed when it
/// cannot.
pub fn level() -> Result<String, String> {
    #[cfg(target_os = "linux")]
    {
        linux::level()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err("bulut denetim kutusu yalnız Linux'ta kapanır (Landlock)".to_owned())
    }
}

/// The box mode: the arguments after [`ARG`]. Restricts this process and executes the interpreter; returns (an exit
/// code) only if it could not, having said why (with [`MARK`]) on its errors.
pub fn enter(args: impl Iterator<Item = OsString>) -> i32 {
    let mut read = Vec::new();
    let mut write = Vec::new();
    let mut tmp = None;
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--oku") => read.extend(args.next().map(PathBuf::from)),
            Some("--yaz") => write.extend(args.next().map(PathBuf::from)),
            Some("--tmp") => tmp = args.next().map(PathBuf::from),
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
        let _ = (read, write, tmp, rest);
        eprintln!(
            "{MARK}: kutu kapatılamadı: yalnız Linux'ta (Landlock); {} çalıştırılmadı",
            Path::new(&python).display()
        );
        126
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use landlock::{
        ABI, Access, AccessFs, AccessNet, CompatLevel, Compatible, PathBeneath, PathFd, Ruleset,
        RulesetAttr, RulesetCreated, RulesetCreatedAttr, RulesetError, Scope,
    };

    use super::Allowed;

    /// Every box's ruleset: the file rights and TCP of ABI 4 required; newer file rights (device ioctl, pathname UNIX
    /// sockets) and the scopes (abstract UNIX sockets, signals) where the kernel has them.
    fn ruleset() -> Result<RulesetCreated, RulesetError> {
        Ruleset::default()
            .set_compatibility(CompatLevel::HardRequirement)
            .handle_access(AccessFs::from_all(ABI::V4))?
            .handle_access(AccessNet::from_all(ABI::V4))?
            .set_compatibility(CompatLevel::BestEffort)
            .handle_access(AccessFs::from_all(ABI::V9))?
            .scope(Scope::from_all(ABI::V9))?
            .create()
    }

    /// Whether the kernel has the scopes (ABI 6) and pathname UNIX sockets (ABI 9), asked without restricting anything.
    fn has_scopes() -> bool {
        Ruleset::default()
            .set_compatibility(CompatLevel::HardRequirement)
            .handle_access(AccessFs::from_all(ABI::V1))
            .and_then(|r| r.scope(Scope::from_all(ABI::V6)))
            .and_then(Ruleset::create)
            .is_ok()
    }

    fn has_unix_paths() -> bool {
        Ruleset::default()
            .set_compatibility(CompatLevel::HardRequirement)
            .handle_access(AccessFs::ResolveUnix)
            .and_then(Ruleset::create)
            .is_ok()
    }

    pub fn level() -> Result<String, String> {
        ruleset().map_err(|e| {
            format!("Landlock (ABI 4: dosyalar ve TCP) bu çekirdekte yok ya da kapalı: {e}")
        })?;
        let scopes = if has_scopes() {
            "sinyaller ve soyut UNIX soketleri kutuda"
        } else {
            "sinyaller ve soyut UNIX soketleri kutunun dışında (ABI 6 yok)"
        };
        let unix = if has_unix_paths() {
            "yol adlı UNIX soketleri kapalı"
        } else {
            "yol adlı UNIX soketleri açık (ABI 9 yok)"
        };
        Ok(format!(
            "dosyalar ve TCP kapalı; {scopes}; {unix}; UDP ve öteki soket türleri açık (Landlock kapsamaz)"
        ))
    }

    /// Restricts the calling thread (the box mode's only one) to `allowed`, irreversibly; inherited across exec.
    pub fn restrict(allowed: &Allowed) -> Result<(), String> {
        let read_dir = AccessFs::from_read(ABI::V9);
        let read_file = read_dir & AccessFs::from_file(ABI::V9);
        let mut ruleset = ruleset().map_err(|e| e.to_string())?;
        for path in &allowed.read {
            let access = if path.is_dir() { read_dir } else { read_file };
            let fd = PathFd::new(path).map_err(|e| format!("{}: {e}", path.display()))?;
            ruleset = ruleset
                .add_rule(PathBeneath::new(fd, access))
                .map_err(|e| e.to_string())?;
        }
        for path in &allowed.write {
            let access = if path.is_dir() {
                AccessFs::from_all(ABI::V9)
            } else {
                AccessFs::from_file(ABI::V9)
            };
            let fd = PathFd::new(path).map_err(|e| format!("{}: {e}", path.display()))?;
            ruleset = ruleset
                .add_rule(PathBeneath::new(fd, access))
                .map_err(|e| e.to_string())?;
        }
        let status = ruleset.restrict_self().map_err(|e| e.to_string())?;
        if status.ruleset == landlock::RulesetStatus::NotEnforced {
            return Err("Landlock kuralları uygulanmadı".to_owned());
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests;
