//! `kentos kek verify` (plan/eklenti.md ADIM 21, EKL-06; docs/adr/8057 §18): a package's working folder checked
//! without a window, in gates whose every finding an agent can mend in one step.
//!
//! - **Gate 1, the package:** everything [`pack`] checks (paths, the manifest and its references, what the
//!   capabilities cover, the integrity list it would write).
//! - **Gate 2, the Python:** every entry point's handler (`module:function`) is a function of `src/`, marked once with
//!   its kind's decorator and its local id (`@ext.audit("yapi-no")`), and no other function claims that id; warnings
//!   for a network library imported without the `network` capability, an operating system command and
//!   `eval`/`exec`.
//! - **Gate 4, the report:** each finding with its file, its line or JSON pointer, and what to write.
//!
//! The Python is read as text, never imported or run: a decorator on its own line (`@ext.command("x")`) is what the
//! SDK's examples write. Gate 3, a dry run on a synthetic drawing, needs KentOS's Python and comes with the desktop.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::archive::{Limits, inspect, pack};
use crate::manifest::{Capability, Manifest};
use crate::problem::Problem;

/// A folder's verification: what keeps it from being packed or run, and what works but should be mended.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Verification {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<Manifest>,
    /// The content digest the package would have.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    pub problems: Vec<Problem>,
    pub warnings: Vec<Problem>,
}

impl Verification {
    pub fn is_valid(&self) -> bool {
        self.problems.is_empty()
    }
}

/// The folder (path → bytes, `/` between folders) verified, gate by gate.
pub fn verify(folder: &BTreeMap<String, Vec<u8>>, limits: &Limits) -> Verification {
    let mut out = Verification::default();
    match pack(folder, limits) {
        Ok(packed) => {
            let inspection = inspect(&packed.bytes, limits);
            out.manifest = inspection.manifest;
            out.digest = Some(packed.digest);
            out.problems.extend(inspection.problems);
            // The packer and the inspector read the same manifest: a warning of it is said once.
            for warning in packed.warnings.into_iter().chain(inspection.warnings) {
                if !out.warnings.contains(&warning) {
                    out.warnings.push(warning);
                }
            }
        }
        Err(problems) => {
            out.problems.extend(problems);
            // The Python is still read when the manifest is: one run says both gates' findings.
            out.manifest = folder
                .get("manifest.json")
                .and_then(|b| serde_json::from_slice::<Manifest>(b).ok());
        }
    }
    if let Some(manifest) = out.manifest.clone() {
        python_gate(&manifest, folder, &mut out.problems, &mut out.warnings);
    }
    out
}

/// An entry point as the Python must give it: its decorator, local id, handler and where the manifest says it.
struct Entry<'a> {
    decorator: &'static str,
    kind: &'static str,
    local: &'a str,
    handler: &'a str,
    at: String,
}

fn entries(m: &Manifest) -> Vec<Entry<'_>> {
    fn add<'a>(
        out: &mut Vec<Entry<'a>>,
        decorator: &'static str,
        kind: &'static str,
        key: &str,
        list: impl Iterator<Item = (&'a str, &'a str)>,
    ) {
        for (i, (local, handler)) in list.enumerate() {
            out.push(Entry {
                decorator,
                kind,
                local,
                handler,
                at: format!("/entrypoints/{key}/{i}/handler"),
            });
        }
    }
    let e = &m.entrypoints;
    let mut out = Vec::new();
    add(
        &mut out,
        "command",
        "komut",
        "commands",
        e.commands
            .iter()
            .map(|c| (c.id.as_str(), c.handler.as_str())),
    );
    add(
        &mut out,
        "processing",
        "İşlemler aracı",
        "processing",
        e.processing
            .iter()
            .map(|c| (c.id.as_str(), c.handler.as_str())),
    );
    add(
        &mut out,
        "audit",
        "denetim",
        "audits",
        e.audits.iter().map(|c| (c.id.as_str(), c.handler.as_str())),
    );
    add(
        &mut out,
        "expr",
        "ifade işlevi",
        "expressions",
        e.expressions
            .iter()
            .map(|c| (c.id.as_str(), c.handler.as_str())),
    );
    add(
        &mut out,
        "map_tool",
        "harita aracı",
        "mapTools",
        e.map_tools
            .iter()
            .map(|c| (c.id.as_str(), c.handler.as_str())),
    );
    out
}

/// How many single letters change one name into the other (Levenshtein).
pub(crate) fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut last = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let kept = row[j + 1];
            row[j + 1] = (last + usize::from(ca != *cb))
                .min(row[j] + 1)
                .min(kept + 1);
            last = kept;
        }
    }
    row[b.len()]
}

/// A decorator line read: `@ext.<kind>("<id>")`, quotes either way.
fn decorator_of(line: &str) -> Option<(&str, &str)> {
    let rest = line.trim().strip_prefix("@ext.")?;
    let (kind, rest) = rest.split_once('(')?;
    let rest = rest.trim_start();
    let quote = rest.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let id = rest[1..].split(quote).next()?;
    Some((kind.trim(), id))
}

/// Gate 2: the handlers, their decorators and the ids they claim; the calls worth a warning.
fn python_gate(
    m: &Manifest,
    folder: &BTreeMap<String, Vec<u8>>,
    problems: &mut Vec<Problem>,
    warnings: &mut Vec<Problem>,
) {
    let sources: Vec<(&String, String)> = folder
        .iter()
        .filter(|(path, _)| path.starts_with("src/") && path.ends_with(".py"))
        .map(|(path, bytes)| (path, String::from_utf8_lossy(bytes).into_owned()))
        .collect();
    let problem = |code: &'static str, file: Option<&str>, at: String, message: String| {
        let mut p = Problem::new(code, message);
        p.file = file.map(str::to_owned);
        p.at = Some(at);
        p
    };
    for entry in entries(m) {
        let Some((module, function)) = entry.handler.split_once(':') else {
            continue; // The manifest's own check refuses it (gate 1).
        };
        let base = format!("src/{}", module.replace('.', "/"));
        let file = [format!("{base}.py"), format!("{base}/__init__.py")]
            .into_iter()
            .find(|f| folder.contains_key(f));
        // A module the package does not carry is gate 1's (`handler_missing`).
        let Some(file) = file else {
            continue;
        };
        let text = sources
            .iter()
            .find(|(p, _)| **p == file)
            .map(|(_, t)| t.as_str())
            .unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        let def = lines.iter().position(|l| {
            l.strip_prefix("def ")
                .is_some_and(|r| r.split('(').next().map(str::trim) == Some(function))
        });
        let Some(def) = def else {
            // The nearest functions the file has, for a handler mistyped.
            let defined: Vec<&str> = lines
                .iter()
                .filter_map(|l| l.strip_prefix("def ")?.split('(').next().map(str::trim))
                .collect();
            let prefix =
                |a: &str, b: &str| a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count();
            let mut near: Vec<(usize, &str)> = defined
                .iter()
                .map(|d| (distance(d, function), *d))
                .filter(|(n, d)| {
                    *n <= 3.max(function.chars().count() / 3) || prefix(d, function) >= 4
                })
                .collect();
            near.sort();
            let hint = match near.first() {
                Some((_, name)) => format!(" En yakını: “{name}”."),
                None => String::new(),
            };
            problems.push(problem(
                "python_function_missing",
                Some("manifest.json"),
                entry.at.clone(),
                format!(
                    "“{function}” işlevi {file} içinde yok (manifestin “{}” işleyicisi).{hint}",
                    entry.local
                ),
            ));
            continue;
        };
        // The decorators right above it.
        let marks: Vec<(&str, &str)> = lines[..def]
            .iter()
            .rev()
            .take_while(|l| l.trim_start().starts_with('@'))
            .filter_map(|l| decorator_of(l))
            .collect();
        let want = format!("@ext.{}(\"{}\")", entry.decorator, entry.local);
        let at = format!("satır {}", def + 1);
        match marks.as_slice() {
            [] => problems.push(problem(
                "python_decorator_missing",
                Some(&file),
                at,
                format!("“{function}” işaretsiz: üstüne {want} yazın."),
            )),
            [(kind, id)] if *kind != entry.decorator => problems.push(problem(
                "python_decorator_kind",
                Some(&file),
                at,
                format!(
                    "“{function}” @ext.{kind}(\"{id}\") işaretli ama manifestte bir {}: {want} yazın.",
                    entry.kind
                ),
            )),
            [(_, id)] if *id != entry.local => problems.push(problem(
                "python_decorator_id",
                Some(&file),
                at,
                format!(
                    "“{function}” “{id}” kimliğiyle işaretli ama manifest “{}” diyor: {want} yazın.",
                    entry.local
                ),
            )),
            [_] => {}
            _ => problems.push(problem(
                "python_decorator_twice",
                Some(&file),
                at,
                format!("“{function}” birden çok @ext işareti taşıyor; yalnız {want} kalsın."),
            )),
        }
    }
    // An id claimed by two functions.
    let mut claimed: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for (path, text) in &sources {
        for (n, line) in text.lines().enumerate() {
            if let Some((kind, id)) = decorator_of(line) {
                claimed
                    .entry((kind.to_owned(), id.to_owned()))
                    .or_default()
                    .push(format!("{path} satır {}", n + 1));
            }
        }
    }
    for ((kind, id), places) in claimed.into_iter().filter(|(_, p)| p.len() > 1) {
        problems.push(problem(
            "python_id_twice",
            None,
            places.join(", "),
            format!("@ext.{kind}(\"{id}\") birden çok işlevde: her kimliğin tek işlevi olur."),
        ));
    }
    // What works but deserves a look.
    let networked = m
        .requested_capabilities
        .iter()
        .any(|c| c.id == Capability::Network);
    for (path, text) in &sources {
        for (n, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or_default().trim();
            let at = format!("satır {}", n + 1);
            let imported = |name: &str| {
                line.strip_prefix("import ")
                    .or_else(|| line.strip_prefix("from "))
                    .is_some_and(|r| {
                        r.split([' ', ',', '.'])
                            .next()
                            .is_some_and(|first| first == name)
                    })
            };
            if !networked
                && let Some(lib) = ["socket", "requests", "urllib", "httpx", "aiohttp", "http"]
                    .into_iter()
                    .find(|l| imported(l))
            {
                warnings.push(problem(
                    "python_network_undeclared",
                    Some(path),
                    at.clone(),
                    format!(
                        "Ağ kitaplığı ({lib}) kullanılıyor ama manifest network yeteneği istemiyor: \
                         requestedCapabilities'e nedeniyle ekleyin ya da ağı kullanmayın."
                    ),
                ));
            }
            if imported("subprocess") || line.contains("os.system(") || line.contains("os.popen(") {
                warnings.push(problem(
                    "python_os_command",
                    Some(path),
                    at.clone(),
                    "İşletim sistemi komutu çalıştırıyor: eklenti kullanıcının haklarıyla çalışır, kullanıcıya \
                     sorulmaz. Gerçekten gerekli mi?"
                        .to_owned(),
                ));
            }
            let called = |name: &str| {
                line.match_indices(name).any(|(i, _)| {
                    !line[..i]
                        .chars()
                        .last()
                        .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.')
                })
            };
            if called("eval(") || called("exec(") {
                warnings.push(problem(
                    "python_eval",
                    Some(path),
                    at,
                    "eval/exec metni kod olarak çalıştırır: verinin içindeki bir metin eklentinin haklarıyla \
                     çalışabilir."
                        .to_owned(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The repository's demo extension's working folder, as the packer reads it.
    fn vitrin() -> BTreeMap<String, Vec<u8>> {
        fn walk(
            root: &std::path::Path,
            dir: &std::path::Path,
            out: &mut BTreeMap<String, Vec<u8>>,
        ) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(root, &path, out);
                } else {
                    let key = path
                        .strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    out.insert(key, std::fs::read(&path).unwrap());
                }
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/eklenti-vitrini/eklenti");
        let mut out = BTreeMap::new();
        walk(&root, &root, &mut out);
        out
    }

    fn codes(list: &[Problem]) -> Vec<&'static str> {
        list.iter().map(|p| p.code).collect()
    }

    fn edit(folder: &mut BTreeMap<String, Vec<u8>>, path: &str, from: &str, to: &str) {
        let text = String::from_utf8(folder[path].clone()).unwrap();
        assert!(text.contains(from), "{from}");
        folder.insert(path.to_owned(), text.replacen(from, to, 1).into_bytes());
    }

    #[test]
    fn the_demo_extension_passes_every_gate() {
        let v = verify(&vitrin(), &Limits::default());
        assert!(v.is_valid(), "{:#?}", v.problems);
        assert!(v.warnings.is_empty(), "{:#?}", v.warnings);
        assert!(v.digest.is_some_and(|d| d.starts_with("sha256:")));
    }

    #[test]
    fn a_handler_without_its_marked_function_is_said_with_what_to_write() {
        const LOGIC: &str = "src/vitrin/logic.py";
        // Unmarked.
        let mut f = vitrin();
        edit(&mut f, LOGIC, "@ext.audit(\"yapi-no\")\n", "");
        let v = verify(&f, &Limits::default());
        assert_eq!(codes(&v.problems), ["python_decorator_missing"]);
        let p = &v.problems[0];
        assert_eq!(p.file.as_deref(), Some(LOGIC));
        assert!(p.at.as_deref().unwrap().starts_with("satır "));
        assert!(
            p.message.contains("@ext.audit(\"yapi-no\")"),
            "{}",
            p.message
        );
        // The wrong kind, the wrong id.
        let mut f = vitrin();
        edit(
            &mut f,
            LOGIC,
            "@ext.audit(\"yapi-no\")",
            "@ext.command(\"yapi-no\")",
        );
        assert_eq!(
            codes(&verify(&f, &Limits::default()).problems),
            ["python_decorator_kind"]
        );
        let mut f = vitrin();
        edit(
            &mut f,
            LOGIC,
            "@ext.audit(\"yapi-no\")",
            "@ext.audit(\"yapi-numarasi\")",
        );
        assert_eq!(
            codes(&verify(&f, &Limits::default()).problems),
            ["python_decorator_id"]
        );
        // A handler whose function is not there, a module not in the package.
        let mut f = vitrin();
        edit(
            &mut f,
            "manifest.json",
            "vitrin.logic:yapi_no",
            "vitrin.logic:yapi_numara",
        );
        let v = verify(&f, &Limits::default());
        assert_eq!(codes(&v.problems), ["python_function_missing"]);
        assert_eq!(v.problems[0].file.as_deref(), Some("manifest.json"));
        assert!(
            v.problems[0].message.contains("En yakını: “yapi_no”"),
            "{}",
            v.problems[0].message
        );
        let mut f = vitrin();
        edit(
            &mut f,
            "manifest.json",
            "vitrin.logic:yapi_no",
            "vitrin.denetim:yapi_no",
        );
        let v = verify(&f, &Limits::default());
        assert_eq!(
            codes(&v.problems),
            ["handler_missing"],
            "the package's own check says it"
        );
        // One id claimed by two functions.
        let mut f = vitrin();
        edit(
            &mut f,
            LOGIC,
            "@ext.command(\"hata-ornegi\")",
            "@ext.command(\"katman-ozeti\")",
        );
        let v = verify(&f, &Limits::default());
        assert!(
            codes(&v.problems).contains(&"python_id_twice"),
            "{:#?}",
            v.problems
        );
    }

    #[test]
    fn an_icon_this_kentos_has_not_is_warned_once_with_the_nearest() {
        // docs/adr/8057 §21: Vitrin names two built-in icons the set writes in camelCase; a slip of a letter is a
        // warning naming the right one, said once though the packer and the inspector both read the manifest.
        let mut f = vitrin();
        edit(
            &mut f,
            "manifest.json",
            "\"icon\": \"space-number\"",
            "\"icon\": \"space-numbr\"",
        );
        let v = verify(&f, &Limits::default());
        assert!(v.is_valid(), "{:#?}", v.problems);
        assert_eq!(codes(&v.warnings), ["manifest_icon_unknown"]);
        let w = &v.warnings[0];
        assert_eq!(
            w.at.as_deref(),
            Some("/contributions/ribbonTabs/0/panels/0/items/1/icon")
        );
        assert!(
            w.message
                .contains("En yakın yerleşik ikonlar: space-number"),
            "{}",
            w.message
        );
    }

    #[test]
    fn a_network_library_without_the_capability_and_os_commands_are_warned() {
        let mut f = vitrin();
        edit(
            &mut f,
            "src/vitrin/logic.py",
            "import time\n",
            "import time\nimport requests\nimport subprocess\n",
        );
        let text = String::from_utf8(f["manifest.json"].clone()).unwrap();
        let mut manifest: serde_json::Value = serde_json::from_str(&text).unwrap();
        let caps = manifest["requestedCapabilities"].as_array_mut().unwrap();
        caps.retain(|c| c["id"] != "network");
        manifest["contributions"]
            .as_object_mut()
            .unwrap()
            .remove("dataSources");
        f.insert(
            "manifest.json".into(),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        );
        let v = verify(&f, &Limits::default());
        assert!(v.is_valid(), "{:#?}", v.problems);
        let python: Vec<&str> = codes(&v.warnings)
            .into_iter()
            .filter(|c| c.starts_with("python_"))
            .collect();
        assert_eq!(python, ["python_network_undeclared", "python_os_command"]);
        // A word inside a name is not a call.
        let mut f = vitrin();
        edit(
            &mut f,
            "src/vitrin/logic.py",
            "import time\n",
            "import time\nretrieval(1)\n",
        );
        assert!(verify(&f, &Limits::default()).warnings.is_empty());
    }
}
