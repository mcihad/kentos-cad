//! The locked packages a computer has no wheel for (plan/eklenti.md ADIM 18A; PY-UV-20, PY-UV-21, D3; docs/adr/8057
//! §94). uv installs a wheel as it is; when none fits a computer it builds the package from its source distribution
//! there: the package's own build code runs with the user's rights, fetches build tools the lock does not name and
//! wants a compiler for a C extension, and a failed build reads like the extension's fault. A server builds nothing
//! (§66). `[tool.uv] required-environments` asks wheels only of packages without a source distribution, so a lock that
//! passes it may still build on some computer: `kentos kek verify`, the install review and the public catalog's
//! dependency gate say which package, where.
//!
//! Read from the lock alone, nothing run: for each computer, the packages it installs (the project's dependencies
//! followed with their markers evaluated for it, every optional feature included, development groups not), then
//! whether one of each package's wheels fits it (PEP 425 tags; platform tags as `packaging` reads them). A computer the
//! lock does not support (`supported-markers`, `requires-python`) is not judged. Simplified on purpose: the newest
//! glibc or macOS a platform tag asks is not compared, KentOS's Linux is glibc's (musl wheels do not count), and a
//! marker this reader does not know holds, so a package is judged rather than skipped.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use super::{Toml, normalized, parse_toml, py_holds, py_version};
use crate::compat::Os;
use crate::manifest::{Manifest, PythonProject};

/// The Python KentOS installs when the user asks (`crates/native/python-env/compat.json`'s `install`).
pub const KENTOS_PYTHON: (u64, u64) = (3, 12);

/// The minor versions of the Python 3 KentOS runs (compat.json's `range`, `>=3.10,<3.15`).
pub const PYTHON_MINORS: std::ops::RangeInclusive<u64> = 10..=14;

/// A processor KentOS runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Machine {
    X86_64,
    Aarch64,
}

/// The computers KentOS runs on (compat.json's `platforms`).
pub const PLATFORMS: [(Os, Machine); 6] = [
    (Os::Linux, Machine::X86_64),
    (Os::Linux, Machine::Aarch64),
    (Os::Windows, Machine::X86_64),
    (Os::Windows, Machine::Aarch64),
    (Os::Macos, Machine::Aarch64),
    (Os::Macos, Machine::X86_64),
];

impl Machine {
    /// This computer's; none on another processor.
    pub fn current() -> Option<Machine> {
        match std::env::consts::ARCH {
            "x86_64" => Some(Machine::X86_64),
            "aarch64" => Some(Machine::Aarch64),
            _ => None,
        }
    }

    /// As Rust and the wheels' Linux tags name it.
    pub fn id(self) -> &'static str {
        match self {
            Machine::X86_64 => "x86_64",
            Machine::Aarch64 => "aarch64",
        }
    }

    /// Python's `platform.machine()` on `os`: what markers name it by.
    pub fn platform_machine(self, os: Os) -> &'static str {
        match (os, self) {
            (Os::Windows, Machine::X86_64) => "AMD64",
            (Os::Windows, Machine::Aarch64) => "ARM64",
            (Os::Macos, Machine::Aarch64) => "arm64",
            (_, m) => m.id(),
        }
    }
}

/// A computer an environment is made on, with its Python.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Target {
    pub os: Os,
    pub machine: Machine,
    /// Python 3's major and minor version (`(3, 12)`).
    pub python: (u64, u64),
}

impl Target {
    /// This computer with `python` (KentOS's when none is known); none on a system KentOS does not run on.
    pub fn here(python: Option<(u64, u64)>) -> Option<Target> {
        Some(Target {
            os: Os::current()?,
            machine: Machine::current()?,
            python: python.unwrap_or(KENTOS_PYTHON),
        })
    }

    /// The computers a package says it runs on (every system when it names none), on each processor KentOS runs on,
    /// with the Python its environment gets there: KentOS's when the lock's `requires-python` takes it, else the oldest
    /// KentOS runs that it takes.
    pub fn declared(systems: &[Os], requires_python: Option<&str>) -> Vec<Target> {
        let takes =
            |minor: u64| requires_python.is_none_or(|r| py_holds(r, &[3, minor]) != Some(false));
        let minor = if takes(KENTOS_PYTHON.1) {
            KENTOS_PYTHON.1
        } else {
            PYTHON_MINORS
                .clone()
                .find(|m| takes(*m))
                .unwrap_or(KENTOS_PYTHON.1)
        };
        PLATFORMS
            .iter()
            .filter(|(os, _)| systems.is_empty() || systems.contains(os))
            .map(|&(os, machine)| Target {
                os,
                machine,
                python: (3, minor),
            })
            .collect()
    }
}

/// Computers in a sentence, by system in the order given: “Windows (x86_64, aarch64), Linux (aarch64)”.
pub fn targets_words(targets: &[Target]) -> String {
    let mut by_os: Vec<(Os, Vec<&'static str>)> = Vec::new();
    for t in targets {
        match by_os.iter_mut().find(|(os, _)| *os == t.os) {
            Some((_, machines)) => machines.push(t.machine.id()),
            None => by_os.push((t.os, vec![t.machine.id()])),
        }
    }
    by_os
        .iter()
        .map(|(os, machines)| format!("{os} ({})", machines.join(", ")))
        .collect::<Vec<_>>()
        .join(", ")
}

/// What a package needs on a computer none of its wheels fits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Need {
    /// Built there from its source distribution (a repository's source always is).
    Source,
    /// Neither a wheel that fits nor a source: it does not install there.
    Nothing,
}

/// A locked package some computers have no wheel for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unwheeled {
    /// Its normalised name.
    pub name: String,
    pub version: String,
    /// The computers and what it needs on each, in the order asked.
    pub needs: Vec<(Target, Need)>,
}

impl Unwheeled {
    /// The computers it needs `need` on.
    pub fn targets(&self, need: Need) -> Vec<Target> {
        self.needs
            .iter()
            .filter(|(_, n)| *n == need)
            .map(|(t, _)| *t)
            .collect()
    }
}

/// The Python of the computers in `targets`, in words (“Python 3.12”): the first's, as [`Target::declared`] gives one.
fn python_words(targets: &[Target]) -> String {
    targets
        .first()
        .map(|t| format!("Python {}.{}", t.python.0, t.python.1))
        .unwrap_or_default()
}

/// What is said of a package without a wheel, a sentence per need: `python_source_build` for the computers it is built
/// on, `python_no_wheel` for those it does not install on. `kentos kek verify` warns with them, the public catalog's
/// dependency gate blocks with them until a reviewer gives a reason.
pub fn said(u: &Unwheeled) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    let built = u.targets(Need::Source);
    if !built.is_empty() {
        out.push((
            "python_source_build",
            format!(
                "“{}” {}: {}, {} için hazır kurulum dosyası (wheel) yok; o bilgisayarlarda kaynağından derlenir \
                 (paketin derleme betiği kullanıcının bilgisayarında çalışır, derleyici gerekebilir), sunucu ise hiç \
                 kurmaz. Bu bilgisayarlar için wheel'i olan bir sürüm kilitleyin ya da [tool.uv] environments ile \
                 onları dışarıda bırakın; genel katalog bunu ancak inceleyicinin gerekçesiyle yayımlar.",
                u.name,
                u.version,
                targets_words(&built),
                python_words(&built)
            ),
        ));
    }
    let nothing = u.targets(Need::Nothing);
    if !nothing.is_empty() {
        out.push((
            "python_no_wheel",
            format!(
                "“{}” {}: {}, {} için ne hazır kurulum dosyası (wheel) ne kaynağı var; o bilgisayarlarda kurulamaz. \
                 [tool.uv] required-environments'a ekleyip uv lock çalıştırın ya da environments ile onları dışarıda \
                 bırakın.",
                u.name,
                u.version,
                targets_words(&nothing),
                python_words(&nothing)
            ),
        ));
    }
    out
}

/// The project's name and `requires-python` in a package's files, with its lock's text; none when they do not read
/// (the lint says that).
fn project_of<'f>(
    python: &PythonProject,
    files: &'f BTreeMap<String, Vec<u8>>,
) -> Option<(String, Option<String>, &'f str)> {
    let text = |path: &str| files.get(path).and_then(|b| std::str::from_utf8(b).ok());
    let project = parse_toml(text(&python.project_file)?).ok()?;
    let name = project
        .get(&["project", "name"])
        .and_then(Toml::str)?
        .to_owned();
    let requires = project
        .get(&["project", "requires-python"])
        .and_then(Toml::str)
        .map(str::to_owned);
    Some((name, requires, text(&python.lock_file)?))
}

/// The packages a package's lock installs without a wheel on the computers it says it runs on ([`Target::declared`]):
/// the public catalog's dependency gate. Empty without Python or when its files do not read.
pub fn of_package(manifest: &Manifest, files: &BTreeMap<String, Vec<u8>>) -> Vec<Unwheeled> {
    let Some((name, requires, lock)) = manifest.python.as_ref().and_then(|p| project_of(p, files))
    else {
        return Vec::new();
    };
    let targets = Target::declared(
        &manifest.compatibility.operating_systems,
        requires.as_deref(),
    );
    unwheeled(lock, &name, &targets)
}

/// The locked packages `target` must not build from source: every one a wheel of fits it there, so that only the
/// packages the install review listed as built here are (docs/adr/8057 §98, PY-UV-21): uv's `--no-build-package` for
/// each. Empty without Python or when its files do not read.
pub fn wheeled_on(
    python: &PythonProject,
    files: &BTreeMap<String, Vec<u8>>,
    target: Target,
) -> Vec<String> {
    let Some((name, _, lock)) = project_of(python, files) else {
        return Vec::new();
    };
    let built: BTreeSet<String> = unwheeled(lock, &name, &[target])
        .into_iter()
        .filter(|u| u.needs.iter().any(|(_, n)| *n == Need::Source))
        .map(|u| u.name)
        .collect();
    crate::release::locked_packages(python, files)
        .unwrap_or_default()
        .into_keys()
        .filter(|n| !built.contains(n))
        .collect()
}

/// What a package's lock means on one computer: the install review's (docs/adr/8057 §94).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OnComputer {
    pub target: Target,
    /// The lock supports it ([`supports`]); uv refuses to make the environment where it does not.
    pub supported: bool,
    /// Its packages without a wheel there.
    pub unwheeled: Vec<Unwheeled>,
}

/// What a package's lock means on `target`; none without Python or when its files do not read.
pub fn on_computer(
    manifest: &Manifest,
    files: &BTreeMap<String, Vec<u8>>,
    target: Target,
) -> Option<OnComputer> {
    let (name, _, lock) = project_of(manifest.python.as_ref()?, files)?;
    let parsed = parse_toml(lock).ok()?;
    Some(OnComputer {
        target,
        supported: supports(&parsed, &target),
        unwheeled: unwheeled(lock, &name, &[target]),
    })
}

/// Whether a lock supports `target`: its `requires-python` takes the Python and one of its `supported-markers` holds
/// there (none written: every computer).
fn supports(lock: &Toml, target: &Target) -> bool {
    let env = Env::of(target);
    let supported = strings(lock.get(&["supported-markers"]));
    let requires = lock.get(&["requires-python"]).and_then(Toml::str);
    requires.is_none_or(|r| py_holds(r, &[target.python.0, target.python.1]) != Some(false))
        && (supported.is_empty() || supported.iter().any(|m| env.holds(m)))
}

/// The packages `lock` installs on `targets` that have no wheel for some of them, by name then version; the project
/// `own` (its name) left out. Empty when the lock does not read: the lint says that.
pub fn unwheeled(lock: &str, own: &str, targets: &[Target]) -> Vec<Unwheeled> {
    let Ok(lock) = parse_toml(lock) else {
        return Vec::new();
    };
    let entries: Vec<&Toml> = lock
        .get(&["package"])
        .and_then(Toml::array)
        .unwrap_or(&[])
        .iter()
        .collect();
    let own = normalized(own);
    let Some(root) = entries.iter().copied().find(|e| name_of(e) == own) else {
        return Vec::new();
    };
    let mut found: BTreeMap<(String, String), Vec<(Target, Need)>> = BTreeMap::new();
    for target in targets {
        if !supports(&lock, target) {
            continue;
        }
        let env = Env::of(target);
        for entry in installed(&entries, root, &env) {
            if let Some(need) = need(entry, target) {
                found
                    .entry((name_of(entry), version_of(entry)))
                    .or_default()
                    .push((*target, need));
            }
        }
    }
    found
        .into_iter()
        .map(|((name, version), needs)| Unwheeled {
            name,
            version,
            needs,
        })
        .collect()
}

fn name_of(entry: &Toml) -> String {
    entry
        .get(&["name"])
        .and_then(Toml::str)
        .map(normalized)
        .unwrap_or_default()
}

fn version_of(entry: &Toml) -> String {
    entry
        .get(&["version"])
        .and_then(Toml::str)
        .unwrap_or_default()
        .to_owned()
}

fn strings(value: Option<&Toml>) -> Vec<String> {
    value
        .and_then(Toml::array)
        .unwrap_or(&[])
        .iter()
        .filter_map(Toml::str)
        .map(str::to_owned)
        .collect()
}

/// The packages `root` installs where `env` holds: its dependencies and every optional feature's, followed with their
/// markers and the extras they ask for, each package once.
fn installed<'a>(entries: &[&'a Toml], root: &'a Toml, env: &Env) -> Vec<&'a Toml> {
    installed_with(entries, root, env, None)
}

/// [`installed`] with only the optional features `extras` names, when it names them (an environment's chosen ones).
fn installed_with<'a>(
    entries: &[&'a Toml],
    root: &'a Toml,
    env: &Env,
    extras: Option<&[String]>,
) -> Vec<&'a Toml> {
    let mut out = Vec::new();
    let mut seen: BTreeSet<(String, String, String)> = BTreeSet::new();
    let mut queue: VecDeque<(&Toml, String)> = VecDeque::from([(root, String::new())]);
    if let Some(groups) = root.get(&["optional-dependencies"]).and_then(Toml::table) {
        queue.extend(
            groups
                .keys()
                .filter(|g| {
                    extras
                        .is_none_or(|chosen| chosen.iter().any(|c| normalized(c) == normalized(g)))
                })
                .map(|g| (root, g.clone())),
        );
    }
    while let Some((entry, extra)) = queue.pop_front() {
        if !seen.insert((name_of(entry), version_of(entry), extra.clone())) {
            continue;
        }
        let dependencies = if extra.is_empty() {
            if !std::ptr::eq(entry, root) {
                out.push(entry);
            }
            entry.get(&["dependencies"])
        } else {
            entry.get(&["optional-dependencies", extra.as_str()])
        };
        for dependency in dependencies.and_then(Toml::array).unwrap_or(&[]) {
            if dependency
                .get(&["marker"])
                .and_then(Toml::str)
                .is_some_and(|m| !env.holds(m))
            {
                continue;
            }
            let Some(next) = resolve(entries, dependency, env) else {
                continue;
            };
            queue.push_back((next, String::new()));
            queue.extend(
                strings(dependency.get(&["extra"]))
                    .iter()
                    .map(|x| (next, normalized(x))),
            );
        }
    }
    out
}

/// The lock's entry a dependency names: by its name; when the lock holds the name more than once (a fork), by the
/// version and source the dependency gives, else the one whose fork (`resolution-markers`) holds where `env` holds.
fn resolve<'a>(entries: &[&'a Toml], dependency: &Toml, env: &Env) -> Option<&'a Toml> {
    let name = normalized(dependency.get(&["name"]).and_then(Toml::str)?);
    let same: Vec<&Toml> = entries
        .iter()
        .copied()
        .filter(|e| name_of(e) == name)
        .collect();
    if same.len() <= 1 {
        return same.first().copied();
    }
    if let Some(version) = dependency.get(&["version"]).and_then(Toml::str) {
        let source = dependency.get(&["source"]);
        let pinned = same.iter().copied().find(|e| {
            version_of(e) == version && source.is_none_or(|s| e.get(&["source"]) == Some(s))
        });
        if pinned.is_some() {
            return pinned;
        }
    }
    same.iter()
        .copied()
        .find(|e| {
            let forks = strings(e.get(&["resolution-markers"]));
            forks.is_empty() || forks.iter().any(|m| env.holds(m))
        })
        .or(same.first().copied())
}

/// What `entry` needs on `target` when none of its wheels fits; none when one fits, and for the author's own folder
/// (the lint refuses that source).
/// Installed from the package itself (a path, a folder, an editable or a virtual project): no wheel to take.
fn is_local(entry: &Toml) -> bool {
    entry
        .get(&["source"])
        .and_then(Toml::table)
        .is_some_and(|s| {
            s.keys()
                .any(|k| matches!(k.as_str(), "path" | "directory" | "editable" | "virtual"))
        })
}

fn need(entry: &Toml, target: &Target) -> Option<Need> {
    let source = entry.get(&["source"]).and_then(Toml::table);
    if is_local(entry) {
        return None;
    }
    let wheels = entry.get(&["wheels"]).and_then(Toml::array).unwrap_or(&[]);
    if wheels
        .iter()
        .filter_map(wheel_file)
        .any(|file| fits(file, target))
    {
        return None;
    }
    let buildable =
        entry.get(&["sdist"]).is_some() || source.is_some_and(|s| s.contains_key("git"));
    Some(if buildable {
        Need::Source
    } else {
        Need::Nothing
    })
}

/// A wheel's file name, from its address or path in the lock.
fn wheel_file(wheel: &Toml) -> Option<&str> {
    let at = ["url", "path", "filename"]
        .iter()
        .find_map(|key| wheel.get(&[*key]).and_then(Toml::str))?;
    let at = at.split(['?', '#']).next()?;
    at.rsplit('/').next()
}

/// Whether a wheel's tags fit `target` (PEP 425, compressed tag sets read): its Python and ABI as CPython
/// `target.python` takes them, one of its platforms `target`'s.
pub fn fits(file: &str, target: &Target) -> bool {
    let Some(stem) = file.strip_suffix(".whl") else {
        return false;
    };
    let parts: Vec<&str> = stem.split('-').collect();
    let n = parts.len();
    if n < 5 {
        return false;
    }
    let (pythons, abis, platforms) = (parts[n - 3], parts[n - 2], parts[n - 1]);
    platforms.split('.').any(|p| platform_fits(p, target))
        && pythons.split('.').any(|py| {
            abis.split('.')
                .any(|abi| python_fits(py, abi, target.python))
        })
}

/// A Python tag's version (`cp312` → 3 and 12, `py3` → 3 alone).
fn tag_version(tag: &str, prefix: &str) -> Option<(u64, Option<u64>)> {
    let digits = tag.strip_prefix(prefix)?;
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let major = digits[..1].parse().ok()?;
    let minor = (digits.len() > 1)
        .then(|| digits[1..].parse().ok())
        .flatten();
    Some((major, minor))
}

/// Whether CPython `major.minor` takes a wheel's Python and ABI tags, as `packaging.tags` lists them: its own
/// (`cp312-cp312`), the stable ABI from an older 3 (`cp39-abi3`), its own without an ABI (`cp312-none`), and pure
/// Python for 3 or an older 3 (`py3-none`, `py311-none`).
fn python_fits(python: &str, abi: &str, (major, minor): (u64, u64)) -> bool {
    match abi {
        "none" => {
            tag_version(python, "cp") == Some((major, Some(minor)))
                || matches!(tag_version(python, "py"), Some((m, None)) if m == major)
                || matches!(tag_version(python, "py"), Some((m, Some(k))) if m == major && k <= minor)
        }
        "abi3" => {
            major == 3
                && matches!(tag_version(python, "cp"), Some((3, Some(k))) if (2..=minor).contains(&k))
        }
        _ => {
            tag_version(abi, "cp") == Some((major, Some(minor)))
                && tag_version(python, "cp") == Some((major, Some(minor)))
        }
    }
}

/// Whether a platform tag fits `target`'s system and processor.
fn platform_fits(tag: &str, target: &Target) -> bool {
    if tag == "any" {
        return true;
    }
    match (target.os, target.machine) {
        (Os::Windows, Machine::X86_64) => tag == "win_amd64",
        (Os::Windows, Machine::Aarch64) => tag == "win_arm64",
        (Os::Macos, machine) => {
            let ends: &[&str] = match machine {
                Machine::Aarch64 => &["_arm64", "_universal2"],
                Machine::X86_64 => &["_x86_64", "_universal2", "_intel", "_universal"],
            };
            tag.starts_with("macosx_") && ends.iter().any(|end| tag.ends_with(end))
        }
        (Os::Linux, machine) => tag
            .strip_suffix(machine.id())
            .and_then(|head| head.strip_suffix('_'))
            .is_some_and(|head| head == "linux" || head.starts_with("manylinux")),
    }
}

/// A computer as markers read it (PEP 508).
struct Env {
    sys_platform: &'static str,
    platform_system: &'static str,
    os_name: &'static str,
    platform_machine: &'static str,
    python_version: String,
    python_full_version: String,
}

impl Env {
    fn of(t: &Target) -> Env {
        Env {
            sys_platform: t.os.sys_platform(),
            platform_system: match t.os {
                Os::Windows => "Windows",
                Os::Macos => "Darwin",
                Os::Linux => "Linux",
            },
            os_name: if t.os == Os::Windows { "nt" } else { "posix" },
            platform_machine: t.machine.platform_machine(t.os),
            python_version: format!("{}.{}", t.python.0, t.python.1),
            python_full_version: format!("{}.{}.0", t.python.0, t.python.1),
        }
    }

    fn value(&self, name: &str) -> Option<String> {
        Some(match name {
            "sys_platform" => self.sys_platform.to_owned(),
            "platform_system" => self.platform_system.to_owned(),
            "os_name" => self.os_name.to_owned(),
            "platform_machine" => self.platform_machine.to_owned(),
            "python_version" => self.python_version.clone(),
            "python_full_version" | "implementation_version" => self.python_full_version.clone(),
            "implementation_name" => "cpython".to_owned(),
            "platform_python_implementation" => "CPython".to_owned(),
            _ => return None,
        })
    }

    /// Whether `marker` holds here; one this reader cannot read holds.
    fn holds(&self, marker: &str) -> bool {
        let tokens = tokens(marker);
        let mut parser = Parser {
            tokens: &tokens,
            at: 0,
            env: self,
        };
        match parser.or() {
            Some(holds) if parser.at == tokens.len() => holds,
            _ => true,
        }
    }
}

#[derive(Debug, PartialEq)]
enum Token {
    Open,
    Close,
    Quoted(String),
    Word(String),
    Op(String),
}

fn tokens(text: &str) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let word = |c: char| c.is_alphanumeric() || c == '_' || c == '.';
    while i < chars.len() {
        let c = chars[i];
        let start = i;
        if c.is_whitespace() {
            i += 1;
        } else if c == '(' || c == ')' {
            out.push(if c == '(' { Token::Open } else { Token::Close });
            i += 1;
        } else if c == '\'' || c == '"' {
            let end = chars[i + 1..]
                .iter()
                .position(|x| *x == c)
                .map_or(chars.len(), |p| i + 1 + p);
            out.push(Token::Quoted(chars[i + 1..end].iter().collect()));
            i = end + 1;
        } else if "=!<>~".contains(c) {
            while i < chars.len() && "=!<>~".contains(chars[i]) {
                i += 1;
            }
            out.push(Token::Op(chars[start..i].iter().collect()));
        } else if word(c) {
            while i < chars.len() && word(chars[i]) {
                i += 1;
            }
            out.push(Token::Word(chars[start..i].iter().collect()));
        } else {
            // Not PEP 508's: the parse fails and the marker holds.
            out.push(Token::Word(c.to_string()));
            i += 1;
        }
    }
    out
}

enum Operand {
    Var(String),
    Text(String),
}

struct Parser<'t> {
    tokens: &'t [Token],
    at: usize,
    env: &'t Env,
}

impl Parser<'_> {
    fn or(&mut self) -> Option<bool> {
        let mut holds = self.and()?;
        while self.word("or") {
            holds |= self.and()?;
        }
        Some(holds)
    }

    fn and(&mut self) -> Option<bool> {
        let mut holds = self.atom()?;
        while self.word("and") {
            holds &= self.atom()?;
        }
        Some(holds)
    }

    fn atom(&mut self) -> Option<bool> {
        if self.tokens.get(self.at) == Some(&Token::Open) {
            self.at += 1;
            let holds = self.or()?;
            if self.tokens.get(self.at) != Some(&Token::Close) {
                return None;
            }
            self.at += 1;
            return Some(holds);
        }
        let left = self.operand()?;
        let op = self.op()?;
        let right = self.operand()?;
        Some(self.compare(left, &op, right))
    }

    fn word(&mut self, w: &str) -> bool {
        let found = matches!(self.tokens.get(self.at), Some(Token::Word(x)) if x == w);
        if found {
            self.at += 1;
        }
        found
    }

    fn operand(&mut self) -> Option<Operand> {
        let operand = match self.tokens.get(self.at)? {
            Token::Quoted(text) => Operand::Text(text.clone()),
            Token::Word(w) if !matches!(w.as_str(), "and" | "or" | "in" | "not") => {
                Operand::Var(w.clone())
            }
            _ => return None,
        };
        self.at += 1;
        Some(operand)
    }

    fn op(&mut self) -> Option<String> {
        match self.tokens.get(self.at)? {
            Token::Op(op) => {
                self.at += 1;
                Some(op.clone())
            }
            Token::Word(w) if w == "in" => {
                self.at += 1;
                Some("in".to_owned())
            }
            Token::Word(w) if w == "not" => {
                self.at += 1;
                self.word("in").then(|| "not in".to_owned())
            }
            _ => None,
        }
    }

    /// A comparison; one of a variable this reader does not know, or of two texts, holds.
    fn compare(&self, left: Operand, op: &str, right: Operand) -> bool {
        let (var, text, flipped) = match (left, right) {
            (Operand::Var(v), Operand::Text(t)) => (v, t, false),
            (Operand::Text(t), Operand::Var(v)) => (v, t, true),
            _ => return true,
        };
        let Some(value) = self.env.value(&var) else {
            return true;
        };
        let version = matches!(
            var.as_str(),
            "python_version" | "python_full_version" | "implementation_version"
        );
        match op {
            "in" | "not in" => {
                let inside = if flipped {
                    value.contains(&text)
                } else {
                    text.contains(&value)
                };
                inside == (op == "in")
            }
            _ if version => {
                let op = if flipped { flip(op) } else { Some(op) };
                match (op, py_version(&value)) {
                    (Some(op), Some(v)) => py_holds(&format!("{op}{text}"), &v).unwrap_or(true),
                    _ => true,
                }
            }
            "==" | "===" => value == text,
            "!=" => value != text,
            _ => true,
        }
    }
}

/// The operator that says the same with its sides swapped.
fn flip(op: &str) -> Option<&'static str> {
    Some(match op {
        "<" => ">",
        ">" => "<",
        "<=" => ">=",
        ">=" => "<=",
        "==" => "==",
        "!=" => "!=",
        _ => return None,
    })
}

mod locked;
pub use locked::{LockedWheels, locked_wheels};

#[cfg(test)]
mod tests;
