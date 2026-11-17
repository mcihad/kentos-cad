//! What a package's Python must not do, read as text when it is packed and when it is installed (plan/eklenti.md
//! ADIM 21, "AST kuralları"; docs/adr/8057 §72):
//!
//! - **Refused:** installing packages while it runs (`pip`, `ensurepip`): its packages are its `pyproject.toml` and
//!   `uv.lock`, prepared before the user turns it on, where the user sees them; changing where Python finds modules
//!   (`sys.path`, `site.addsitedir`, `PYTHONPATH`): code from outside the reviewed package; a network library without
//!   the `network` capability: the user consented to less than it does.
//! - **Warned:** an operating system command and `eval`/`exec`: no capability says them yet.
//!
//! A text rule, as gate 2's: docstrings and comments are not code (a "pip install" in a usage note is no call). It
//! catches the honest mistake and the habit an agent brings from scripts (`subprocess.check_call([sys.executable, "-m",
//! "pip", "install", …])`); it does not stop a determined author: the catalog's review and the operating system's
//! confinement are other layers.

use crate::manifest::{Capability, Manifest};
use crate::problem::Problem;

/// The network libraries whose import says the code goes to the network.
const NETWORK: [&str; 6] = ["socket", "requests", "urllib", "httpx", "aiohttp", "http"];

/// What changes where Python finds modules.
const PATHS: [&str; 10] = [
    "sys.path.append(",
    "sys.path.insert(",
    "sys.path.extend(",
    "sys.path.remove(",
    "sys.path[",
    "sys.path =",
    "sys.path=",
    "sys.path +=",
    "site.addsitedir(",
    "sys.meta_path",
];

/// What installs a package: pip named as a program or called, its bootstrapper, the old installer.
const INSTALLS: [&str; 8] = [
    "\"pip\"",
    "'pip'",
    "pip.main(",
    "pip._internal",
    "-m pip",
    "pip install",
    "ensurepip",
    "easy_install",
];

/// The lines of `text` that are code, with their numbers from 1: a string spanning lines (a docstring) skipped, a
/// comment cut.
pub(crate) fn code_lines(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    // The quotes of the string the line began in.
    let mut inside: Option<&'static str> = None;
    for (n, raw) in text.lines().enumerate() {
        let mut code = String::new();
        let mut rest = raw;
        loop {
            if let Some(quotes) = inside {
                match rest.find(quotes) {
                    Some(i) => {
                        rest = &rest[i + quotes.len()..];
                        inside = None;
                    }
                    None => break,
                }
                continue;
            }
            let opens = ["\"\"\"", "'''"]
                .into_iter()
                .filter_map(|q| rest.find(q).map(|i| (i, q)))
                .min();
            let hash = rest.find('#');
            match opens {
                Some((i, quotes)) if hash.is_none_or(|h| h > i) => {
                    code.push_str(&rest[..i]);
                    rest = &rest[i + quotes.len()..];
                    inside = Some(quotes);
                }
                _ => {
                    code.push_str(hash.map_or(rest, |h| &rest[..h]));
                    break;
                }
            }
        }
        if !code.trim().is_empty() {
            out.push((n + 1, code.trim().to_owned()));
        }
    }
    out
}

/// Whether `line` imports module `name` (`import name…`, `from name…`).
fn imports(line: &str, name: &str) -> bool {
    line.strip_prefix("import ")
        .or_else(|| line.strip_prefix("from "))
        .is_some_and(|r| r.split([' ', ',', '.']).next() == Some(name))
}

/// Whether `line` calls `name` (`eval(`): not a word ending in it, not a method of something.
fn calls(line: &str, name: &str) -> bool {
    line.match_indices(name).any(|(i, _)| {
        !line[..i]
            .chars()
            .last()
            .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.')
    })
}

/// The rules on Python file `path` of a package whose manifest is `manifest`: each finding with its line.
pub(crate) fn scan(
    manifest: &Manifest,
    path: &str,
    text: &str,
    problems: &mut Vec<Problem>,
    warnings: &mut Vec<Problem>,
) {
    let networked = manifest
        .requested_capabilities
        .iter()
        .any(|c| c.id == Capability::Network);
    let found = |code: &'static str, n: usize, message: String| {
        Problem::new(code, message)
            .file(path)
            .at(format!("satır {n}"))
    };
    for (n, line) in code_lines(text) {
        if imports(&line, "pip") || INSTALLS.iter().any(|p| line.contains(p)) {
            problems.push(found(
                "python_runtime_install",
                n,
                "Çalışırken paket kuruyor: eklentinin paketleri pyproject.toml ve uv.lock'ta yazılır, KentOS ortamı \
                 eklenti açılmadan hazırlar ve kullanıcı görür. Kuran satırı silin, paketi pyproject.toml'a ekleyin \
                 (uv add)."
                    .to_owned(),
            ));
        }
        if PATHS.iter().any(|p| line.contains(p))
            || (line.contains("PYTHONPATH") && line.contains('='))
        {
            problems.push(found(
                "python_sys_path",
                n,
                "Python'un modül aradığı yeri değiştiriyor (sys.path): incelenen paketin dışından kod yükler. Satırı \
                 silin; modül src/ altına, paket pyproject.toml'a."
                    .to_owned(),
            ));
        }
        if !networked && let Some(lib) = NETWORK.into_iter().find(|l| imports(&line, l)) {
            problems.push(found(
                "python_network_undeclared",
                n,
                format!(
                    "Ağ kitaplığı ({lib}) kullanılıyor ama manifest network yeteneği istemiyor: kullanıcı ağa \
                     çıkmayı onaylamadı. requestedCapabilities'e nedeni ve adresleriyle (hosts) ekleyin ya da ağı \
                     kullanmayın."
                ),
            ));
        }
        if imports(&line, "subprocess") || line.contains("os.system(") || line.contains("os.popen(")
        {
            warnings.push(found(
                "python_os_command",
                n,
                "İşletim sistemi komutu çalıştırıyor: eklenti kullanıcının haklarıyla çalışır, kullanıcıya sorulmaz. \
                 Gerçekten gerekli mi?"
                    .to_owned(),
            ));
        }
        if calls(&line, "eval(") || calls(&line, "exec(") {
            warnings.push(found(
                "python_eval",
                n,
                "eval/exec metni kod olarak çalıştırır: verinin içindeki bir metin eklentinin haklarıyla çalışabilir."
                    .to_owned(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A docstring's and a comment's words are not code; the code after a one-line docstring is.
    #[test]
    fn docstrings_and_comments_are_not_code() {
        let text = "def a():\n    \"\"\"Kurulum: pip install numpy\n    sys.path.append('x')\n    \"\"\"\n    \
                    return 1  # pip install x\nb = '''tek satır''' + eval_(2)\n";
        let lines: Vec<(usize, String)> = code_lines(text);
        assert_eq!(
            lines,
            [
                (1, "def a():".to_owned()),
                (5, "return 1".to_owned()),
                (6, "b =  + eval_(2)".to_owned())
            ]
        );
    }

    /// Each rule by its line: installing and paths refused, an undeclared network refused, a command and eval warned;
    /// a word that only ends in a name, or a name inside another, is no call.
    #[test]
    fn each_rule_is_said_on_its_line() {
        let manifest: Manifest = serde_json::from_str(include_str!(
            "../../../../fixtures/extensions/v1/ornek-altlik/manifest.json"
        ))
        .expect("a manifest");
        let mut without = manifest.clone();
        without
            .requested_capabilities
            .retain(|c| c.id != Capability::Network);
        let text = "import requests\n\
                    subprocess.check_call([sys.executable, \"-m\", \"pip\", \"install\", \"x\"])\n\
                    sys.path.insert(0, '/opt/kod')\n\
                    import subprocess\n\
                    eval(girdi)\n\
                    pipeline = retrieval(evaluate(1))\n\
                    import pip_tools_like\n\
                    os.environ['PYTHONPATH'] = '/tmp'\n";
        let run = |m: &Manifest| {
            let (mut problems, mut warnings) = (Vec::new(), Vec::new());
            scan(m, "src/a.py", text, &mut problems, &mut warnings);
            let said = |list: Vec<Problem>| -> Vec<(&'static str, String)> {
                list.into_iter()
                    .map(|p| (p.code, p.at.unwrap_or_default()))
                    .collect()
            };
            (said(problems), said(warnings))
        };
        let (problems, warnings) = run(&without);
        let at = |code: &'static str, n: usize| (code, format!("satır {n}"));
        assert_eq!(
            problems,
            [
                at("python_network_undeclared", 1),
                at("python_runtime_install", 2),
                at("python_sys_path", 3),
                at("python_sys_path", 8)
            ]
        );
        assert_eq!(warnings, [at("python_os_command", 4), at("python_eval", 5)]);
        // With the capability, the network is the manifest's word.
        let (problems, _) = run(&manifest);
        assert!(
            !problems
                .iter()
                .any(|(c, _)| *c == "python_network_undeclared")
        );
    }
}
