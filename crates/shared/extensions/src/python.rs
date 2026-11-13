//! A package's Python project, read lightly (plan/eklenti.md §3.4, ADIM 16; PY-UV-13, PY-UV-17): `pyproject.toml` and
//! `uv.lock` agree, and the lock covers the systems the manifest names. The full resolution is uv's, at install (ADIM
//! 18A); this is the lint that catches at packing what would otherwise fail on a user's computer: a lock older than the
//! project ("eskimiş kilit"), a dependency not locked, Windows named in the manifest but no Windows wheels required
//! (D3), a local path that exists only on the author's computer.
//!
//! A small TOML reader of its own (the subset `pyproject.toml` and uv write: tables, arrays of tables, strings, numbers,
//! booleans, arrays, inline tables; dates kept as text), so the inspector, which also runs in the browser and is a
//! security gate, takes no new dependency. It is not the security boundary: uv reads the files again at install.

use std::collections::{BTreeMap, BTreeSet};

use crate::compat::Os;
use crate::manifest::Manifest;
use crate::problem::Problem;

/// A TOML value.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Toml {
    Str(String),
    Int(i64),
    Bool(bool),
    Array(Vec<Toml>),
    Table(BTreeMap<String, Toml>),
    /// A float or a date, as written.
    Other(String),
}

impl Toml {
    fn str(&self) -> Option<&str> {
        match self {
            Toml::Str(s) => Some(s),
            _ => None,
        }
    }

    fn table(&self) -> Option<&BTreeMap<String, Toml>> {
        match self {
            Toml::Table(t) => Some(t),
            _ => None,
        }
    }

    fn array(&self) -> Option<&[Toml]> {
        match self {
            Toml::Array(a) => Some(a),
            _ => None,
        }
    }

    fn get(&self, path: &[&str]) -> Option<&Toml> {
        path.iter().try_fold(self, |at, key| at.table()?.get(*key))
    }
}

type Table = BTreeMap<String, Toml>;

struct Reader<'a> {
    b: &'a [u8],
    text: &'a str,
    i: usize,
}

/// A TOML document's root table, or the line and what is wrong there.
pub(crate) fn parse_toml(text: &str) -> Result<Toml, (usize, String)> {
    let mut r = Reader {
        b: text.as_bytes(),
        text,
        i: 0,
    };
    r.document().map_err(|message| (r.line(), message))
}

/// Descends to the table at `keys`, making the missing ones; an array of tables gives its last.
fn descend<'m>(mut table: &'m mut Table, keys: &[String]) -> Result<&'m mut Table, String> {
    for key in keys {
        let entry = table
            .entry(key.clone())
            .or_insert_with(|| Toml::Table(Table::new()));
        table = match entry {
            Toml::Table(t) => t,
            Toml::Array(items) => match items.last_mut() {
                Some(Toml::Table(t)) => t,
                _ => return Err(format!("“{key}” bir tablo dizisi değil")),
            },
            _ => return Err(format!("“{key}” hem değer hem tablo olarak yazılmış")),
        };
    }
    Ok(table)
}

impl Reader<'_> {
    fn line(&self) -> usize {
        self.b[..self.i.min(self.b.len())]
            .iter()
            .filter(|b| **b == b'\n')
            .count()
            + 1
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn starts(&self, text: &str) -> bool {
        self.b[self.i.min(self.b.len())..].starts_with(text.as_bytes())
    }

    /// Spaces and tabs.
    fn blanks(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.i += 1;
        }
    }

    /// Blanks, comments and line breaks.
    fn space(&mut self) {
        loop {
            self.blanks();
            match self.peek() {
                Some(b'#') => {
                    while !matches!(self.peek(), None | Some(b'\n')) {
                        self.i += 1;
                    }
                }
                Some(b'\n' | b'\r') => self.i += 1,
                _ => return,
            }
        }
    }

    /// The rest of a line after a value or a header: blanks and a comment, then the line's end.
    fn end_of_line(&mut self) -> Result<(), String> {
        self.blanks();
        if self.peek() == Some(b'#') {
            while !matches!(self.peek(), None | Some(b'\n')) {
                self.i += 1;
            }
        }
        match self.peek() {
            None | Some(b'\n' | b'\r') => Ok(()),
            Some(_) => Err("değerden sonra satır bitmeli".to_owned()),
        }
    }

    fn document(&mut self) -> Result<Toml, String> {
        let mut root = Table::new();
        let mut current: Vec<String> = Vec::new();
        loop {
            self.space();
            match self.peek() {
                None => return Ok(Toml::Table(root)),
                Some(b'[') if self.starts("[[") => {
                    self.i += 2;
                    let keys = self.keys()?;
                    self.blanks();
                    if !self.starts("]]") {
                        return Err("tablo dizisi başlığı “]]” ile kapanmalı".to_owned());
                    }
                    self.i += 2;
                    self.end_of_line()?;
                    let (last, parents) = keys.split_last().ok_or("boş başlık")?;
                    let parent = descend(&mut root, parents)?;
                    match parent
                        .entry(last.clone())
                        .or_insert_with(|| Toml::Array(Vec::new()))
                    {
                        Toml::Array(items) => items.push(Toml::Table(Table::new())),
                        _ => return Err(format!("“{last}” hem tablo dizisi hem başka bir şey")),
                    }
                    current = keys;
                }
                Some(b'[') => {
                    self.i += 1;
                    let keys = self.keys()?;
                    self.blanks();
                    if self.peek() != Some(b']') {
                        return Err("tablo başlığı “]” ile kapanmalı".to_owned());
                    }
                    self.i += 1;
                    self.end_of_line()?;
                    descend(&mut root, &keys)?;
                    current = keys;
                }
                Some(_) => {
                    let keys = self.keys()?;
                    self.blanks();
                    if self.peek() != Some(b'=') {
                        return Err("anahtardan sonra “=” bekleniyordu".to_owned());
                    }
                    self.i += 1;
                    self.blanks();
                    let value = self.value()?;
                    self.end_of_line()?;
                    let table = descend(&mut root, &current)?;
                    insert(table, &keys, value)?;
                }
            }
        }
    }

    /// A dotted key: `a`, `"a.b"`, `a . 'b'`.
    fn keys(&mut self) -> Result<Vec<String>, String> {
        let mut keys = Vec::new();
        loop {
            self.blanks();
            let key = match self.peek() {
                Some(b'"') => self.basic_string()?,
                Some(b'\'') => self.literal_string()?,
                _ => {
                    let start = self.i;
                    while self
                        .peek()
                        .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                    {
                        self.i += 1;
                    }
                    if start == self.i {
                        return Err("anahtar bekleniyordu".to_owned());
                    }
                    self.text[start..self.i].to_owned()
                }
            };
            keys.push(key);
            self.blanks();
            if self.peek() == Some(b'.') {
                self.i += 1;
            } else {
                return Ok(keys);
            }
        }
    }

    fn value(&mut self) -> Result<Toml, String> {
        match self.peek() {
            Some(b'"') if self.starts("\"\"\"") => self.multiline_basic().map(Toml::Str),
            Some(b'"') => self.basic_string().map(Toml::Str),
            Some(b'\'') if self.starts("'''") => {
                self.i += 3;
                if self.peek() == Some(b'\n') {
                    self.i += 1;
                } else if self.starts("\r\n") {
                    self.i += 2;
                }
                let start = self.i;
                let end = self.text[start..]
                    .find("'''")
                    .ok_or("çok satırlı metin kapanmıyor")?;
                self.i = start + end + 3;
                Ok(Toml::Str(self.text[start..start + end].to_owned()))
            }
            Some(b'\'') => self.literal_string().map(Toml::Str),
            Some(b'[') => {
                self.i += 1;
                let mut items = Vec::new();
                loop {
                    self.space();
                    if self.peek() == Some(b']') {
                        self.i += 1;
                        return Ok(Toml::Array(items));
                    }
                    items.push(self.value()?);
                    self.space();
                    match self.peek() {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Toml::Array(items));
                        }
                        _ => return Err("dizide “,” ya da “]” bekleniyordu".to_owned()),
                    }
                }
            }
            Some(b'{') => {
                self.i += 1;
                let mut table = Table::new();
                loop {
                    self.space();
                    if self.peek() == Some(b'}') {
                        self.i += 1;
                        return Ok(Toml::Table(table));
                    }
                    let keys = self.keys()?;
                    self.blanks();
                    if self.peek() != Some(b'=') {
                        return Err("satır içi tabloda “=” bekleniyordu".to_owned());
                    }
                    self.i += 1;
                    self.blanks();
                    let value = self.value()?;
                    insert(&mut table, &keys, value)?;
                    self.space();
                    match self.peek() {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Toml::Table(table));
                        }
                        _ => return Err("satır içi tabloda “,” ya da “}” bekleniyordu".to_owned()),
                    }
                }
            }
            Some(_) => {
                let start = self.i;
                while self.peek().is_some_and(|b| {
                    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'+' | b'-' | b'.' | b':')
                }) {
                    self.i += 1;
                }
                let token = &self.text[start..self.i];
                match token {
                    "" => Err("değer bekleniyordu".to_owned()),
                    "true" => Ok(Toml::Bool(true)),
                    "false" => Ok(Toml::Bool(false)),
                    _ => Ok(token
                        .replace('_', "")
                        .parse::<i64>()
                        .map_or_else(|_| Toml::Other(token.to_owned()), Toml::Int)),
                }
            }
            None => Err("değer bekleniyordu".to_owned()),
        }
    }

    fn literal_string(&mut self) -> Result<String, String> {
        self.i += 1;
        let start = self.i;
        while !matches!(self.peek(), None | Some(b'\'' | b'\n')) {
            self.i += 1;
        }
        if self.peek() != Some(b'\'') {
            return Err("tek tırnaklı metin kapanmıyor".to_owned());
        }
        self.i += 1;
        Ok(self.text[start..self.i - 1].to_owned())
    }

    fn escape(&mut self, out: &mut String) -> Result<(), String> {
        self.i += 1;
        let c = self.peek().ok_or("kaçış dizisi yarım")?;
        self.i += 1;
        match c {
            b'b' => out.push('\u{8}'),
            b't' => out.push('\t'),
            b'n' => out.push('\n'),
            b'f' => out.push('\u{c}'),
            b'r' => out.push('\r'),
            b'e' => out.push('\u{1b}'),
            b'"' => out.push('"'),
            b'\\' => out.push('\\'),
            b'x' | b'u' | b'U' => {
                let n = match c {
                    b'x' => 2,
                    b'u' => 4,
                    _ => 8,
                };
                let hex = self
                    .text
                    .get(self.i..self.i + n)
                    .ok_or("kaçış dizisi yarım")?;
                let code = u32::from_str_radix(hex, 16).map_err(|_| "geçersiz kaçış dizisi")?;
                out.push(char::from_u32(code).ok_or("geçersiz Unicode kaçışı")?);
                self.i += n;
            }
            _ => return Err(format!("geçersiz kaçış dizisi “\\{}”", char::from(c))),
        }
        Ok(())
    }

    fn basic_string(&mut self) -> Result<String, String> {
        self.i += 1;
        let mut out = String::new();
        loop {
            match self.peek() {
                None | Some(b'\n') => return Err("çift tırnaklı metin kapanmıyor".to_owned()),
                Some(b'"') => {
                    self.i += 1;
                    return Ok(out);
                }
                Some(b'\\') => self.escape(&mut out)?,
                Some(_) => {
                    let c = self.text[self.i..].chars().next().ok_or("metin yarım")?;
                    out.push(c);
                    self.i += c.len_utf8();
                }
            }
        }
    }

    fn multiline_basic(&mut self) -> Result<String, String> {
        self.i += 3;
        if self.peek() == Some(b'\n') {
            self.i += 1;
        } else if self.starts("\r\n") {
            self.i += 2;
        }
        let mut out = String::new();
        loop {
            if self.starts("\"\"\"") {
                self.i += 3;
                // Up to two quotes before the closing three are the text's ("""a"""").
                for _ in 0..2 {
                    if self.peek() == Some(b'"') {
                        out.push('"');
                        self.i += 1;
                    }
                }
                return Ok(out);
            }
            match self.peek() {
                None => return Err("çok satırlı metin kapanmıyor".to_owned()),
                Some(b'\\') => {
                    // A line-ending backslash trims the break and the blanks after it.
                    let rest = &self.b[self.i + 1..];
                    let blanks = rest
                        .iter()
                        .take_while(|b| matches!(b, b' ' | b'\t'))
                        .count();
                    if matches!(rest.get(blanks), Some(b'\n' | b'\r')) {
                        self.i += 1;
                        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
                            self.i += 1;
                        }
                    } else {
                        self.escape(&mut out)?;
                    }
                }
                Some(_) => {
                    let c = self.text[self.i..].chars().next().ok_or("metin yarım")?;
                    out.push(c);
                    self.i += c.len_utf8();
                }
            }
        }
    }
}

fn insert(table: &mut Table, keys: &[String], value: Toml) -> Result<(), String> {
    let (last, parents) = keys.split_last().ok_or("boş anahtar")?;
    let table = descend(table, parents)?;
    if table.contains_key(last) {
        return Err(format!("“{last}” anahtarı iki kez yazılmış"));
    }
    table.insert(last.clone(), value);
    Ok(())
}

/// A Python package's name normalised (PEP 503): lower case, runs of `-`, `_` and `.` one `-`.
pub(crate) fn normalized(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut dash = false;
    for c in name.trim().chars() {
        if matches!(c, '-' | '_' | '.') {
            dash = true;
        } else {
            if dash && !out.is_empty() {
                out.push('-');
            }
            dash = false;
            out.extend(c.to_lowercase());
        }
    }
    out
}

/// A version specifier compared as uv writes it: no blanks, its clauses in order.
fn specifier(text: &str) -> String {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let mut parts: Vec<&str> = compact.split(',').filter(|p| !p.is_empty()).collect();
    parts.sort_unstable();
    parts.join(",")
}

/// A requirement (PEP 508): its normalised name and specifier; `pkg[extra]>=1 ; marker`, `pkg @ url`.
fn requirement(text: &str) -> Option<(String, String)> {
    let without_marker = text.split(';').next()?.trim();
    if let Some((name, _)) = without_marker.split_once('@') {
        let name = name.split('[').next()?.trim();
        return (!name.is_empty()).then(|| (normalized(name), String::new()));
    }
    let end = without_marker
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')))
        .unwrap_or(without_marker.len());
    let (name, mut rest) = without_marker.split_at(end);
    if name.is_empty() {
        return None;
    }
    rest = rest.trim_start();
    if rest.starts_with('[') {
        rest = &rest[rest.find(']')? + 1..];
    }
    let rest = rest.trim().trim_start_matches('(').trim_end_matches(')');
    Some((normalized(name), specifier(rest)))
}

/// A marker compared as uv writes it: single quotes, single blanks, the `and` clauses in order.
fn marker(text: &str) -> String {
    let quoted = text.replace('"', "'");
    let single: Vec<&str> = quoted.split_whitespace().collect();
    let joined = single.join(" ");
    let mut clauses: Vec<&str> = joined.split(" and ").map(str::trim).collect();
    clauses.sort_unstable();
    clauses.join(" and ")
}

/// The extra a requires-dist marker names (`extra == 'charts'`).
fn marker_extra(text: &str) -> Option<String> {
    let at = text.find("extra")?;
    let rest = text[at + "extra".len()..]
        .trim_start()
        .strip_prefix("==")?
        .trim_start();
    let quote = rest.chars().next().filter(|c| *c == '\'' || *c == '"')?;
    let value = rest[1..].split(quote).next()?;
    Some(normalized(value))
}

/// A Python version's parts (`3.12.1`).
fn py_version(text: &str) -> Option<Vec<u64>> {
    text.trim().split('.').map(|p| p.parse().ok()).collect()
}

/// Two versions compared as PEP 440 does for plain numbers: the shorter padded with zeros.
fn py_cmp(a: &[u64], b: &[u64]) -> std::cmp::Ordering {
    let n = a.len().max(b.len());
    let at = |v: &[u64], i: usize| v.get(i).copied().unwrap_or(0);
    (0..n)
        .map(|i| at(a, i).cmp(&at(b, i)))
        .find(|o| o.is_ne())
        .unwrap_or(std::cmp::Ordering::Equal)
}

/// Whether a Python version holds a PEP 440 specifier's every clause; none for one this reader does not know.
fn py_holds(spec: &str, v: &[u64]) -> Option<bool> {
    use std::cmp::Ordering::{Equal, Greater, Less};
    for clause in spec.split(',').map(str::trim).filter(|c| !c.is_empty()) {
        let (op, rest) = ["~=", "===", "==", "!=", "<=", ">=", "<", ">"]
            .iter()
            .find_map(|op| clause.strip_prefix(op).map(|rest| (*op, rest.trim())))?;
        let holds = match op {
            "==" | "!=" if rest.ends_with(".*") => {
                let prefix = py_version(rest.strip_suffix(".*")?)?;
                let same = (0..prefix.len()).all(|i| v.get(i).copied().unwrap_or(0) == prefix[i]);
                same == (op == "==")
            }
            "==" => py_cmp(v, &py_version(rest)?) == Equal,
            "!=" => py_cmp(v, &py_version(rest)?) != Equal,
            "<=" => py_cmp(v, &py_version(rest)?) != Greater,
            ">=" => py_cmp(v, &py_version(rest)?) != Less,
            "<" => py_cmp(v, &py_version(rest)?) == Less,
            ">" => py_cmp(v, &py_version(rest)?) == Greater,
            "~=" => {
                let p = py_version(rest)?;
                if p.len() < 2 {
                    return None;
                }
                py_cmp(v, &p) != Less
                    && (0..p.len() - 1).all(|i| v.get(i).copied().unwrap_or(0) == p[i])
            }
            _ => return None,
        };
        if !holds {
            return Some(false);
        }
    }
    Some(true)
}

/// Whether two `requires-python` ranges hold for the same Pythons: uv writes `~=3.12` as `>=3.12, <4`, so the text
/// alone would call a fresh lock stale. Compared over Python 2.0 to 4.30.5; an operator this reader does not know falls
/// back to the text.
fn python_ranges_agree(a: &str, b: &str) -> bool {
    for major in 2..=4u64 {
        for minor in 0..=30u64 {
            for patch in 0..=5u64 {
                let v = [major, minor, patch];
                match (py_holds(a, &v), py_holds(b, &v)) {
                    (Some(x), Some(y)) if x == y => {}
                    (Some(_), Some(_)) => return false,
                    _ => return specifier(a) == specifier(b),
                }
            }
        }
    }
    true
}

/// What a project declares: (group, normalised name, specifier); group `""` for the main dependencies.
type Declared = BTreeSet<(String, String, String)>;

/// Checks the Python project against its lock and the manifest: problems, then warnings.
pub(crate) fn check(
    manifest: &Manifest,
    project_path: &str,
    project: &str,
    lock_path: &str,
    lock: &str,
    problems: &mut Vec<Problem>,
    warnings: &mut Vec<Problem>,
) {
    let fail = |problems: &mut Vec<Problem>, code: &'static str, file: &str, message: String| {
        problems.push(Problem::new(code, message).file(file));
    };
    let project = match parse_toml(project) {
        Ok(p) => p,
        Err((line, message)) => {
            problems.push(
                Problem::new(
                    "python_toml",
                    format!("Satır {line}: TOML okunamadı ({message})."),
                )
                .file(project_path)
                .at(format!("satır {line}")),
            );
            return;
        }
    };
    let Some(name) = project.get(&["project", "name"]).and_then(Toml::str) else {
        fail(
            problems,
            "python_project",
            project_path,
            "[project] tablosunda “name” yok.".to_owned(),
        );
        return;
    };
    let Some(requires_python) = project
        .get(&["project", "requires-python"])
        .and_then(Toml::str)
    else {
        fail(
            problems,
            "python_project",
            project_path,
            "[project] tablosunda “requires-python” yok: Python sürüm aralığının tek kaynağı odur (ör. \">=3.12\").".to_owned(),
        );
        return;
    };
    let mut declared = Declared::new();
    let strings = |value: Option<&Toml>| -> Option<Vec<String>> {
        match value {
            None => Some(Vec::new()),
            Some(v) => v
                .array()?
                .iter()
                .map(|x| x.str().map(str::to_owned))
                .collect(),
        }
    };
    let Some(main) = strings(project.get(&["project", "dependencies"])) else {
        fail(
            problems,
            "python_project",
            project_path,
            "[project] “dependencies” bir metin dizisi olmalı.".to_owned(),
        );
        return;
    };
    for text in &main {
        match requirement(text) {
            Some((n, s)) => {
                declared.insert((String::new(), n, s));
            }
            None => fail(
                problems,
                "python_project",
                project_path,
                format!("Bağımlılık “{text}” okunamadı (PEP 508)."),
            ),
        }
    }
    let mut extras = BTreeSet::new();
    if let Some(groups) = project
        .get(&["project", "optional-dependencies"])
        .and_then(Toml::table)
    {
        for (group, list) in groups {
            let group = normalized(group);
            extras.insert(group.clone());
            for text in strings(Some(list)).unwrap_or_default() {
                if let Some((n, s)) = requirement(&text) {
                    declared.insert((group.clone(), n, s));
                }
            }
        }
    }
    if let Some(python) = &manifest.python {
        for (key, feature) in &python.optional_features {
            if !extras.contains(&normalized(&feature.extra)) {
                fail(
                    problems,
                    "python_extra_missing",
                    project_path,
                    format!(
                        "Manifestteki “{key}” seçeneği “{}” grubunu açıyor ama [project.optional-dependencies] içinde \
                         öyle bir grup yok.",
                        feature.extra
                    ),
                );
            }
        }
    }
    let markers = |key: &str| -> Option<Vec<String>> {
        project.get(&["tool", "uv", key]).map(|v| {
            strings(Some(v))
                .unwrap_or_default()
                .iter()
                .map(|m| marker(m))
                .collect()
        })
    };
    let environments = markers("required-environments").unwrap_or_default();
    // `environments` narrows the resolution itself: a system left out of it gets no packages at all.
    let resolved = markers("environments");
    let systems: Vec<Os> = if manifest.compatibility.operating_systems.is_empty() {
        Os::ALL.to_vec()
    } else {
        manifest.compatibility.operating_systems.clone()
    };
    let uncovered = |list: &[String]| -> Vec<String> {
        systems
            .iter()
            .filter(|os| {
                let wanted = format!("sys_platform == '{}'", os.sys_platform());
                !list.iter().any(|e| e.contains(&wanted))
            })
            .map(|os| format!("{os} (sys_platform == '{}')", os.sys_platform()))
            .collect()
    };
    if !declared.is_empty() {
        let missing = uncovered(&environments);
        if !missing.is_empty() {
            fail(
                problems,
                "python_environments",
                project_path,
                format!(
                    "Manifest bu sistemlerde çalıştığını söylüyor ama [tool.uv] required-environments onlar için \
                     paket istemiyor: {}. Ekleyip “uv lock” çalıştırın; yoksa tekerleği olmayan sistemde kurulum \
                     kullanıcının bilgisayarında düşer.",
                    missing.join(", ")
                ),
            );
        }
    }
    if let Some(resolved) = &resolved {
        let missing = uncovered(resolved);
        if !missing.is_empty() {
            fail(
                problems,
                "python_environments",
                project_path,
                format!(
                    "[tool.uv] environments çözümü bu sistemlerin dışında bırakıyor ama manifest onlarda çalıştığını \
                     söylüyor: {}. Listeye ekleyin ya da manifestin operatingSystems'ini daraltın.",
                    missing.join(", ")
                ),
            );
        }
    }
    let lock = match parse_toml(lock) {
        Ok(l) => l,
        Err((line, message)) => {
            problems.push(
                Problem::new(
                    "python_toml",
                    format!("Satır {line}: TOML okunamadı ({message})."),
                )
                .file(lock_path)
                .at(format!("satır {line}")),
            );
            return;
        }
    };
    if !matches!(lock.get(&["version"]), Some(Toml::Int(1))) {
        warnings.push(
            Problem::new(
                "python_lock_version",
                "uv.lock bu KentOS'un tanıdığı sürümde (1) değil; kilit denetlenemedi. Kurulumda uv yeniden denetler.",
            )
            .file(lock_path),
        );
        return;
    }
    let stale = |problems: &mut Vec<Problem>, why: String| {
        problems.push(
            Problem::new(
                "python_lock_stale",
                format!("uv.lock pyproject.toml'dan eski: {why}. Proje klasöründe “uv lock” çalıştırıp yeniden paketleyin."),
            )
            .file(lock_path),
        );
    };
    match lock.get(&["requires-python"]).and_then(Toml::str) {
        Some(locked) if python_ranges_agree(locked, requires_python) => {}
        locked => stale(
            problems,
            format!(
                "requires-python projede “{requires_python}”, kilitte “{}”",
                locked.unwrap_or("yok")
            ),
        ),
    }
    let locked_markers: BTreeSet<String> = strings(lock.get(&["required-markers"]))
        .unwrap_or_default()
        .iter()
        .map(|m| marker(m))
        .collect();
    if locked_markers != environments.iter().cloned().collect::<BTreeSet<_>>() {
        stale(
            problems,
            "required-environments kilitteki required-markers ile tutmuyor".to_owned(),
        );
    }
    let supported: BTreeSet<String> = strings(lock.get(&["supported-markers"]))
        .unwrap_or_default()
        .iter()
        .map(|m| marker(m))
        .collect();
    if supported
        != resolved
            .unwrap_or_default()
            .into_iter()
            .collect::<BTreeSet<_>>()
    {
        stale(
            problems,
            "environments kilitteki supported-markers ile tutmuyor".to_owned(),
        );
    }
    let packages: &[Toml] = lock.get(&["package"]).and_then(Toml::array).unwrap_or(&[]);
    let own = normalized(name);
    let names: BTreeSet<String> = packages
        .iter()
        .filter_map(|p| p.get(&["name"]).and_then(Toml::str))
        .map(normalized)
        .collect();
    let root = packages.iter().find(|p| {
        p.get(&["name"])
            .and_then(Toml::str)
            .map(normalized)
            .as_deref()
            == Some(&own)
    });
    let Some(root) = root else {
        stale(problems, format!("projenin kendisi (“{own}”) kilitte yok"));
        return;
    };
    let mut locked = Declared::new();
    for entry in root
        .get(&["metadata", "requires-dist"])
        .and_then(Toml::array)
        .unwrap_or(&[])
    {
        let Some(n) = entry.get(&["name"]).and_then(Toml::str) else {
            continue;
        };
        let group = entry
            .get(&["marker"])
            .and_then(Toml::str)
            .and_then(marker_extra)
            .unwrap_or_default();
        let spec = entry
            .get(&["specifier"])
            .and_then(Toml::str)
            .map(specifier)
            .unwrap_or_default();
        locked.insert((group, normalized(n), spec));
    }
    if locked != declared {
        let differ: Vec<String> = declared
            .symmetric_difference(&locked)
            .map(|(group, n, _)| {
                if group.is_empty() {
                    n.clone()
                } else {
                    format!("{n} [{group}]")
                }
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        stale(
            problems,
            format!("bağımlılıklar tutmuyor ({})", differ.join(", ")),
        );
    }
    for (_, n, _) in &declared {
        if !names.contains(n) {
            stale(problems, format!("“{n}” kilitte çözülmemiş"));
        }
    }
    for package in packages {
        let n = package.get(&["name"]).and_then(Toml::str).unwrap_or("?");
        let Some(source) = package.get(&["source"]).and_then(Toml::table) else {
            continue;
        };
        for (kind, value) in source {
            let value = value.str().unwrap_or("");
            match kind.as_str() {
                "virtual" | "editable" if value == "." => {}
                "path" | "directory" | "editable" | "virtual" => problems.push(
                    Problem::new(
                        "python_local_source",
                        format!(
                            "“{n}” paketi yerel bir klasörden geliyor ({value}); o klasör yalnız yazarın bilgisayarında \
                             var. Bir paket deposundan alın."
                        ),
                    )
                    .file(lock_path),
                ),
                "git" | "url" => warnings.push(
                    Problem::new(
                        "python_vcs_source",
                        format!("“{n}” bir paket deposundan değil, doğrudan {value} adresinden geliyor; katalog incelemesi bunu ayrıca sorar."),
                    )
                    .file(lock_path),
                ),
                "registry" if !value.starts_with("https://pypi.org/") => warnings.push(
                    Problem::new(
                        "python_registry",
                        format!("“{n}” PyPI dışı bir paket deposundan ({value}); kurum ilkesi bu depoya izin vermeli."),
                    )
                    .file(lock_path),
                ),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests;
