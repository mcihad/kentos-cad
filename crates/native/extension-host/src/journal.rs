//! Each extension's log (docs/adr/8057 §4): what happened to it and what went wrong, so that when an extension fails
//! the user, its publisher and an agent all know clearly what, where and why (the owner, 13 November: "eklenti hata
//! verdiğinde açıkça bilelim sorunu").
//!
//! - A [`Record`] has a level, a stable code (`activated`, `integrity_mismatch`, `form_invalid` …), the version in
//!   use, the part (`etkinleştirme`, `form “Ruhsat”`, `veri kaynağı “Ortofoto”`), the words the user reads and the
//!   error as it came (`detail`: an I/O error, a server's answer; a Python traceback in ADIM 19).
//! - JSON Lines under `<kayıt>/logs`: `<id>.jsonl` per extension, `registry.jsonl` for what is no one extension's.
//!   Appended one write per line, so the desktop, `kentos` and the MCP server write side by side; at [`LIMIT`] a file
//!   becomes `<name>.1.jsonl` and starts again: at most about twice the limit per extension.
//! - Secrets are masked before writing ([`mask`]): a token or key in a URL, the password of `user:password@host`, an
//!   `Authorization` header, a `password=` in a connection string.
//! - Writing never fails what is logged: a log that cannot be written is said on standard error.
//!
//! The registry's own short history (`registry.json`'s `log`) is read once into here and no longer written.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A file's size at which it moves to `.1` and starts again.
pub const LIMIT: u64 = 256 * 1024;

/// The log of what is no one extension's: a package that could not be read, the registry itself.
pub const REGISTRY: &str = "registry";

/// Codes after which an earlier error is no longer the extension's problem: new files, or the user tried again.
const FRESH: [&str; 5] = ["installed", "updated", "reinstalled", "switched", "retried"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    Info,
    Warning,
    Error,
}

impl Level {
    pub fn label(self) -> &'static str {
        match self {
            Level::Info => "Bilgi",
            Level::Warning => "Uyarı",
            Level::Error => "Hata",
        }
    }
}

/// A line of a log.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    /// Milliseconds since 1970, UTC.
    pub at: u64,
    pub level: Level,
    /// Stable, for programs, searches and reports: `activated`, `integrity_mismatch`, `form_invalid` …
    pub code: String,
    /// The extension; none in the registry's own log.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<String>,
    /// The version in use when it happened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// What part of it: `etkinleştirme`, `form “Ruhsat”`, `veri kaynağı “Ortofoto”`, `kurulum` …
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
    /// What happened and what to do, in the words the user reads.
    pub message: String,
    /// The error as it came, secrets masked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// The program that wrote it: `kentos-cad`, `kentos`, `kentos-mcp`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub program: String,
}

impl Record {
    pub fn new(level: Level, code: &str, message: impl Into<String>) -> Record {
        Record {
            at: now_ms(),
            level,
            code: code.to_owned(),
            extension: None,
            version: None,
            part: None,
            message: message.into(),
            detail: None,
            program: String::new(),
        }
    }

    pub fn info(code: &str, message: impl Into<String>) -> Record {
        Record::new(Level::Info, code, message)
    }

    pub fn warning(code: &str, message: impl Into<String>) -> Record {
        Record::new(Level::Warning, code, message)
    }

    pub fn error(code: &str, message: impl Into<String>) -> Record {
        Record::new(Level::Error, code, message)
    }

    /// Whose it is: the extension and the version in use.
    pub fn of(mut self, extension: &str, version: Option<&str>) -> Record {
        self.extension = Some(extension.to_owned());
        self.version = version.map(str::to_owned);
        self
    }

    pub fn part(mut self, part: impl Into<String>) -> Record {
        self.part = Some(part.into());
        self
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Record {
        let detail = detail.into();
        if !detail.trim().is_empty() {
            self.detail = Some(detail);
        }
        self
    }

    /// The same event, whenever it happened: what a repeated state note is compared by.
    pub fn same_event(&self, other: &Record) -> bool {
        (
            self.level,
            &self.code,
            &self.version,
            &self.part,
            &self.message,
            &self.detail,
        ) == (
            other.level,
            &other.code,
            &other.version,
            &other.part,
            &other.message,
            &other.detail,
        )
    }
}

/// Milliseconds since 1970.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// Where the logs are and which program writes them.
#[derive(Clone, Debug)]
pub struct Journal {
    dir: PathBuf,
    program: String,
    limit: u64,
}

impl Journal {
    /// The logs of the registry in `root`, written as this program.
    pub fn new(root: &Path) -> Journal {
        let program = std::env::current_exe()
            .ok()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .unwrap_or_default();
        Journal {
            dir: root.join("logs"),
            program,
            limit: LIMIT,
        }
    }

    /// The same logs, moving to `.1` at `bytes`: a test's small limit.
    pub fn with_limit(mut self, bytes: u64) -> Journal {
        self.limit = bytes;
        self
    }

    /// The folder of the logs.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The file of an extension's log, or the registry's.
    pub fn file(&self, extension: Option<&str>) -> PathBuf {
        self.dir.join(format!("{}.jsonl", name_of(extension)))
    }

    fn older(&self, extension: Option<&str>) -> PathBuf {
        self.dir.join(format!("{}.1.jsonl", name_of(extension)))
    }

    /// The record as it is written: masked, with this program's name.
    pub fn prepare(&self, mut record: Record) -> Record {
        record.message = mask(&record.message);
        record.detail = record.detail.as_deref().map(mask);
        if record.program.is_empty() {
            record.program = self.program.clone();
        }
        record
    }

    /// Appends a record to its extension's log (the registry's when it names none); returns it as written.
    pub fn write(&self, record: Record) -> Record {
        let record = self.prepare(record);
        self.append(&record);
        record
    }

    /// Appends a record unless the log's newest is the same event: a state said at every start is said once, until
    /// it changes. Returns it when written.
    pub fn write_changed(&self, record: Record) -> Option<Record> {
        let record = self.prepare(record);
        let last = self.tail(record.extension.as_deref(), 1);
        if last.last().is_some_and(|l| l.same_event(&record)) {
            return None;
        }
        self.append(&record);
        Some(record)
    }

    /// One write per line: processes writing side by side do not cut each other's lines.
    pub fn append(&self, record: &Record) {
        let extension = record.extension.as_deref();
        let file = self.file(extension);
        let mut line = serde_json::to_string(record).unwrap_or_default();
        line.push('\n');
        let written = fs::create_dir_all(&self.dir)
            .and_then(|()| {
                if fs::metadata(&file).is_ok_and(|m| m.len() >= self.limit) {
                    fs::rename(&file, self.older(extension))?;
                }
                OpenOptions::new().create(true).append(true).open(&file)
            })
            .and_then(|mut f| f.write_all(line.as_bytes()));
        if let Err(e) = written {
            eprintln!("Eklenti günlüğü yazılamadı ({}): {e}", file.display());
        }
    }

    /// The newest `limit` records of an extension's log (the registry's for none), oldest first; a line that cannot
    /// be read is left out.
    pub fn tail(&self, extension: Option<&str>, limit: usize) -> Vec<Record> {
        let mut out: Vec<Record> = Vec::new();
        for file in [self.older(extension), self.file(extension)] {
            let Ok(text) = fs::read_to_string(&file) else {
                continue;
            };
            out.extend(
                text.lines()
                    .filter(|l| !l.trim().is_empty())
                    .filter_map(|l| serde_json::from_str::<Record>(l).ok()),
            );
        }
        let extra = out.len().saturating_sub(limit);
        out.drain(..extra);
        out
    }

    /// The extension's problem now: its newest error not followed by new files or a retry.
    pub fn problem(&self, extension: &str) -> Option<Record> {
        problem_in(&self.tail(Some(extension), 500)).cloned()
    }
}

/// Of a log's records (oldest first), the error still standing: the newest error not followed by new files, a retry,
/// or the same part working since (a data source that connects, a form that is imported, an activation that went well).
pub fn problem_in(records: &[Record]) -> Option<&Record> {
    let mut working: Vec<&str> = Vec::new();
    for record in records.iter().rev() {
        if FRESH.contains(&record.code.as_str()) {
            return None;
        }
        match (record.level, record.part.as_deref()) {
            (Level::Error, Some(part)) if working.contains(&part) => {}
            (Level::Error, _) => return Some(record),
            (Level::Info, Some(part)) => working.push(part),
            _ => {}
        }
    }
    None
}

fn name_of(extension: Option<&str>) -> &str {
    match extension {
        Some(id) if kentos_extensions::manifest::is_extension_id(id) => id,
        _ => REGISTRY,
    }
}

/// Words whose value is a secret wherever they name one (`?access_token=`, `password=`, `"apiKey": "…"`).
const SECRETS: [&str; 15] = [
    "token",
    "key",
    "secret",
    "password",
    "passwd",
    "pwd",
    "auth",
    "signature",
    "sig",
    "credential",
    "session",
    "cookie",
    "parola",
    "şifre",
    "jeton",
];

/// What stands in for a secret.
const HIDDEN: &str = "***";

/// Hides what must not reach a log: the value after a name that names a secret (`token=…`, `apikey=…`,
/// `"password": "…"`), the password of `user:password@host`, and what follows `Authorization:` or `Bearer`.
pub fn mask(text: &str) -> String {
    let text = mask_named(text);
    let text = mask_userinfo(&text);
    mask_bearer(&text)
}

fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '.')
}

fn ends_value(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '&' | ';' | ',' | '"' | '\'' | ')' | '#' | '}' | ']' | '<' | '>'
        )
}

/// `name=value` and `"name": "value"` (or `name: value`) where the name names a secret; an `Authorization` header's
/// whole value but its scheme.
fn mask_named(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        out.push(c);
        i += 1;
        if c != '=' && c != ':' {
            continue;
        }
        let name = name_before(&chars, i - 1);
        if name.is_empty() || !SECRETS.iter().any(|s| name.contains(s)) {
            continue;
        }
        // `scheme://` is not a name: `https:` would otherwise hide an address.
        if c == ':' && chars.get(i) == Some(&'/') {
            continue;
        }
        let mut j = i;
        if c == ':' {
            while j < chars.len() && chars[j] == ' ' {
                j += 1;
            }
        }
        let header = name.ends_with("authorization");
        let end = if header {
            chars[j..]
                .iter()
                .position(|&ch| ch == '\n')
                .map_or(chars.len(), |p| j + p)
        } else if j < chars.len() && matches!(chars[j], '"' | '\'') {
            let quote = chars[j];
            match chars[j + 1..].iter().position(|&ch| ch == quote) {
                Some(p) if p > 0 => {
                    j += 1;
                    j + p
                }
                _ => continue,
            }
        } else {
            chars[j..]
                .iter()
                .position(|&ch| ends_value(ch))
                .map_or(chars.len(), |p| j + p)
        };
        if end <= j {
            continue;
        }
        // The spaces and an opening quote stay.
        out.extend(&chars[i..j]);
        if header {
            let value: String = chars[j..end].iter().collect();
            if let Some((scheme, _)) = value
                .split_once(' ')
                .filter(|(s, _)| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphabetic()))
            {
                out.push_str(scheme);
                out.push(' ');
            }
        }
        out.push_str(HIDDEN);
        i = end;
    }
    out
}

/// The name before a sign at `sign`, past a closing quote, lower-cased.
fn name_before(chars: &[char], sign: usize) -> String {
    let mut end = sign;
    if end > 0 && matches!(chars[end - 1], '"' | '\'') {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && is_name_char(chars[start - 1]) {
        start -= 1;
    }
    chars[start..end].iter().collect::<String>().to_lowercase()
}

/// `scheme://user:password@host` → `scheme://user:***@host`.
fn mask_userinfo(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("://") {
        let (before, after) = rest.split_at(at + 3);
        out.push_str(before);
        let authority_end = after
            .find(|c: char| matches!(c, '/' | '?' | '#') || ends_value(c))
            .unwrap_or(after.len());
        let authority = &after[..authority_end];
        match authority
            .rfind('@')
            .and_then(|a| authority[..a].find(':').map(|c| (c, a)))
        {
            Some((colon, at)) if at > colon + 1 => {
                out.push_str(&authority[..=colon]);
                out.push_str(HIDDEN);
                out.push_str(&authority[at..]);
            }
            _ => out.push_str(authority),
        }
        rest = &after[authority_end..];
    }
    out.push_str(rest);
    out
}

/// `Bearer <token>` and `Basic <credentials>` → the scheme and `***`.
fn mask_bearer(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        // ASCII only: the indices stay those of the text.
        let lower = rest.to_ascii_lowercase();
        let found = ["bearer ", "basic "]
            .iter()
            .filter_map(|w| lower.find(w).map(|i| (i, w.len())))
            .min();
        let Some((i, n)) = found else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..i + n]);
        let after = &rest[i + n..];
        let end = after.find(ends_value).unwrap_or(after.len());
        if end > 0 && after[..end] != *HIDDEN {
            out.push_str(HIDDEN);
        } else {
            out.push_str(&after[..end]);
        }
        rest = &after[end..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kentos-gunluk-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn secrets_never_reach_a_log() {
        let cases = [
            (
                "https://harita.ornek.bel.tr/wmts?SERVICE=WMTS&access_token=abc123&layer=orto",
                "https://harita.ornek.bel.tr/wmts?SERVICE=WMTS&access_token=***&layer=orto",
            ),
            (
                "postgresql://kentos:gizli-parola@db.ornek.bel.tr:5432/cbs",
                "postgresql://kentos:***@db.ornek.bel.tr:5432/cbs",
            ),
            (
                "host=db.ornek.bel.tr user=kentos password=s3cr3t dbname=cbs",
                "host=db.ornek.bel.tr user=kentos password=*** dbname=cbs",
            ),
            (
                r#"{"apiKey": "AIza-xyz", "katman": "orto"}"#,
                r#"{"apiKey": "***", "katman": "orto"}"#,
            ),
            (
                "Authorization: Bearer eyJhbGciOi.payload.sig",
                "Authorization: Bearer ***",
            ),
            ("şifre=1234 ve jeton: abc", "şifre=*** ve jeton: ***"),
            // What is not a secret stays as it was.
            (
                "Sunucu yanıt vermedi (HTTP 503): https://tiles.ornek.bel.tr/{z}/{x}/{y}.png",
                "Sunucu yanıt vermedi (HTTP 503): https://tiles.ornek.bel.tr/{z}/{x}/{y}.png",
            ),
            (
                "forms/ruhsat.kform: satır 3, sütun 7: expected `,`",
                "forms/ruhsat.kform: satır 3, sütun 7: expected `,`",
            ),
        ];
        for (raw, masked) in cases {
            assert_eq!(mask(raw), masked, "{raw}");
        }
    }

    #[test]
    fn a_log_keeps_its_newest_records_across_its_turn_and_masks_them() {
        let root = temp("tur");
        let journal = Journal::new(&root).with_limit(600);
        let id = "tr.ornek.kent-rehberi";
        for i in 0..20 {
            journal.write(
                Record::info("activated", format!("{i}. açılış"))
                    .of(id, Some("1.0.0"))
                    .detail(format!("https://x.ornek.bel.tr/?token=t{i}")),
            );
        }
        // Two files at most, each under the limit plus a line.
        assert!(journal.file(Some(id)).exists());
        assert!(journal.dir().join(format!("{id}.1.jsonl")).exists());
        assert_eq!(fs::read_dir(journal.dir()).unwrap().count(), 2);
        let tail = journal.tail(Some(id), 3);
        let words: Vec<&str> = tail.iter().map(|r| r.message.as_str()).collect();
        assert_eq!(words, ["17. açılış", "18. açılış", "19. açılış"]);
        assert_eq!(
            tail[2].detail.as_deref(),
            Some("https://x.ornek.bel.tr/?token=***")
        );
        assert_eq!(tail[2].version.as_deref(), Some("1.0.0"));
        assert!(!tail[2].program.is_empty(), "the program that wrote it");
        // A broken line is left out; the rest is read.
        let mut f = OpenOptions::new()
            .append(true)
            .open(journal.file(Some(id)))
            .unwrap();
        f.write_all(b"{ yar\xc4\xb1m\n").unwrap();
        journal.write(Record::error("form_invalid", "Form okunamadı.").of(id, Some("1.0.0")));
        let tail = journal.tail(Some(id), 2);
        assert_eq!(tail[1].code, "form_invalid");
        // What no extension names, and a name that is not an id, go to the registry's.
        journal.write(Record::warning("install_refused", "Paket okunamadı."));
        journal.write(Record::warning("install_refused", "Bozuk ad.").of("../dışarı", None));
        assert_eq!(journal.tail(None, 10).len(), 2);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_problem_is_the_newest_error_until_new_files_or_a_retry() {
        let root = temp("sorun");
        let journal = Journal::new(&root);
        let id = "tr.ornek.mebis";
        assert_eq!(journal.problem(id), None);
        journal
            .write(Record::error("integrity_mismatch", "Dosyalar değişmiş.").of(id, Some("1.0.0")));
        journal.write(Record::info("disabled", "Devre dışı bırakıldı.").of(id, Some("1.0.0")));
        assert_eq!(
            journal.problem(id).map(|r| r.code),
            Some("integrity_mismatch".to_owned()),
            "a later note does not clear it"
        );
        journal.write(Record::info("retried", "Yeniden deneniyor.").of(id, Some("1.0.0")));
        assert_eq!(journal.problem(id), None);
        journal.write(Record::error("form_invalid", "Form okunamadı.").of(id, Some("1.0.0")));
        journal.write(Record::info("updated", "1.0.1'e güncellendi.").of(id, Some("1.0.1")));
        assert_eq!(journal.problem(id), None, "new files");
        // A part that works again is mended; another part's error still stands.
        let source = "veri kaynağı “Ortofoto”";
        journal.write(
            Record::error("form_invalid", "Form okunamadı.")
                .of(id, None)
                .part("form “Ruhsat”"),
        );
        journal.write(
            Record::error("source_unreachable", "Bağlanılamadı.")
                .of(id, None)
                .part(source),
        );
        journal.write(
            Record::info("source_connected", "Bağlanıldı.")
                .of(id, None)
                .part(source),
        );
        assert_eq!(
            journal.problem(id).map(|r| r.code),
            Some("form_invalid".to_owned())
        );
        journal.write(
            Record::info("form_imported", "İçe aktarıldı.")
                .of(id, None)
                .part("form “Ruhsat”"),
        );
        assert_eq!(journal.problem(id), None, "both work again");
        // A state said at every start is written once until it changes.
        let note =
            || Record::warning("not_started", "Engellendi: kurum ilkesi.").of(id, Some("1.0.1"));
        journal.write_changed(note());
        journal.write_changed(note());
        assert_eq!(journal.tail(Some(id), 100).len(), 10);
        journal.write_changed(
            Record::warning("not_started", "Uyumsuz: KentOS 0.3 gerekiyor.").of(id, Some("1.0.1")),
        );
        assert_eq!(journal.tail(Some(id), 100).len(), 11);
        let _ = fs::remove_dir_all(&root);
    }
}
