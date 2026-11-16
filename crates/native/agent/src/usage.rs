//! What the agents spent and how far they may go (docs/adr/8056 §6; plan/eklenti.md ADIM 21, "maliyet/istek bütçesi"):
//! an agent's tokens, in and out, counted for its conversation and for the day on this computer, and the limits its
//! profile sets refusing a turn before it starts and stopping one that crosses them. An agent that calls tools in a
//! loop spends without anyone typing; a limit the user wrote is what stops it, said in a sentence, never a silent cut.
//!
//! The day's totals are kept beside the agents' file (`yapay-zeka-kullanimi.json`), by local date and agent, the last
//! [`DAYS_KEPT`] days: no prompt, no answer, no key, only numbers.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};

use crate::AgentProfile;

/// The file's name beside the agents'.
const FILE: &str = "yapay-zeka-kullanimi.json";
const FORMAT: &str = "kentos.agent-usage";
/// The days a ledger keeps.
pub const DAYS_KEPT: usize = 31;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Book {
    #[serde(default)]
    format: String,
    /// By local date (`2026-11-16`), then by agent id: the tokens spent.
    #[serde(default)]
    days: BTreeMap<String, BTreeMap<String, u64>>,
}

/// The tokens the agents spent on this computer, by day.
#[derive(Debug)]
pub struct UsageLedger {
    path: Option<PathBuf>,
    book: Mutex<Book>,
}

impl Default for UsageLedger {
    fn default() -> Self {
        UsageLedger::in_memory()
    }
}

impl UsageLedger {
    /// The ledger kept in `path`; a file that does not read starts empty (it holds only numbers).
    pub fn at(path: impl Into<PathBuf>) -> UsageLedger {
        let path = path.into();
        let book = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        UsageLedger {
            path: Some(path),
            book: Mutex::new(book),
        }
    }

    /// The device's ledger, beside its agents' file; in memory when no folder is known.
    pub fn device() -> UsageLedger {
        match crate::profiles::device_folder() {
            Some(folder) => UsageLedger::at(folder.join(FILE)),
            None => UsageLedger::in_memory(),
        }
    }

    pub fn in_memory() -> UsageLedger {
        UsageLedger {
            path: None,
            book: Mutex::new(Book::default()),
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    fn lock(&self) -> MutexGuard<'_, Book> {
        self.book.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// What `agent` spent on `day` (`2026-11-16`).
    pub fn spent(&self, agent: &str, day: &str) -> u64 {
        self.lock()
            .days
            .get(day)
            .and_then(|d| d.get(agent))
            .copied()
            .unwrap_or(0)
    }

    /// `tokens` more for `agent` on `day`, written at once (the older days than [`DAYS_KEPT`] let go). A ledger that
    /// cannot be written keeps counting in memory: a full disk does not stop the agent, its limit still holds.
    pub fn add(&self, agent: &str, day: &str, tokens: u64) {
        let mut book = self.lock();
        book.format = FORMAT.to_owned();
        let entry = book
            .days
            .entry(day.to_owned())
            .or_default()
            .entry(agent.to_owned())
            .or_insert(0);
        *entry = entry.saturating_add(tokens);
        while book.days.len() > DAYS_KEPT {
            let Some(oldest) = book.days.keys().next().cloned() else {
                break;
            };
            book.days.remove(&oldest);
        }
        if let Some(path) = &self.path {
            if let Some(folder) = path.parent() {
                let _ = std::fs::create_dir_all(folder);
            }
            if let Ok(text) = serde_json::to_string_pretty(&*book) {
                let _ = std::fs::write(path, text);
            }
        }
    }
}

/// A number of tokens as the panel writes it: thousands with dots (“200.000”).
pub fn tokens_words(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

/// What an agent spent `today` and its limits, in a line for its row in the settings.
pub fn spending_words(profile: &AgentProfile, today: u64) -> String {
    let limits = match (profile.conversation_tokens, profile.daily_tokens) {
        (None, None) => "token sınırı yok".to_owned(),
        (Some(c), None) => format!("sınır sohbet başına {} token", tokens_words(c)),
        (None, Some(d)) => format!("sınır günde {} token", tokens_words(d)),
        (Some(c), Some(d)) => format!(
            "sınır sohbet başına {}, günde {} token",
            tokens_words(c),
            tokens_words(d)
        ),
    };
    format!("Bugün {} token harcadı · {limits}", tokens_words(today))
}

/// A limit as the user types it: thousands with dots or spaces (“200.000”, “200 000”); empty, no limit. Why it is not
/// one otherwise, in a sentence for under the field.
pub fn read_tokens(typed: &str) -> Result<Option<u64>, String> {
    let digits: String = typed
        .chars()
        .filter(|c| !matches!(c, '.' | ' ' | '\u{a0}' | '\u{202f}' | '_'))
        .collect();
    if digits.is_empty() {
        return Ok(None);
    }
    match digits.parse::<u64>() {
        Ok(0) => {
            Err("Sınır sıfırdan büyük olmalı; sınır istemiyorsanız alanı boş bırakın.".to_owned())
        }
        Ok(n) if digits.chars().all(|c| c.is_ascii_digit()) => Ok(Some(n)),
        _ => Err("Yalnız rakam yazın (örneğin 200.000); boş bırakılırsa sınır yoktur.".to_owned()),
    }
}

/// Why `profile` may not go on with `conversation` tokens spent in this conversation and `today` today; none while
/// both stay under its limits.
pub fn over_budget(profile: &AgentProfile, conversation: u64, today: u64) -> Option<String> {
    if let Some(most) = profile.conversation_tokens
        && conversation >= most
    {
        return Some(format!(
            "Bu sohbet “{}” için yazdığınız sınıra ulaştı: {} / {} token. Yeni sohbet başlatın ya da sınırı Uygulama \
             ayarları › Yapay zekâ'dan yükseltin.",
            profile.name,
            tokens_words(conversation),
            tokens_words(most)
        ));
    }
    if let Some(most) = profile.daily_tokens
        && today >= most
    {
        return Some(format!(
            "“{}” bugün için yazdığınız sınıra ulaştı: {} / {} token. Yarın ya da sınırı Uygulama ayarları › Yapay \
             zekâ'dan yükseltince devam eder.",
            profile.name,
            tokens_words(today),
            tokens_words(most)
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderKind;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kentos-ajan-kullanim-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// A day's tokens add up by agent, are kept in the file and read back; the oldest day goes after 31.
    #[test]
    fn the_ledger_keeps_a_days_tokens_by_agent() {
        let dir = temp("defter");
        let path = dir.join(FILE);
        let ledger = UsageLedger::at(&path);
        ledger.add("a1", "2026-11-16", 1_200);
        ledger.add("a1", "2026-11-16", 300);
        ledger.add("a2", "2026-11-16", 50);
        assert_eq!(ledger.spent("a1", "2026-11-16"), 1_500);
        assert_eq!(
            UsageLedger::at(&path).spent("a1", "2026-11-16"),
            1_500,
            "written"
        );
        for day in 1..=40 {
            ledger.add("a1", &format!("2026-10-{day:02}"), 1);
        }
        let book = ledger.lock();
        assert_eq!(book.days.len(), DAYS_KEPT);
        assert!(book.days.contains_key("2026-11-16"), "the newest kept");
        drop(book);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("kentos.agent-usage") && !text.contains("prompt"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The limits refuse with their numbers; none written, no limit; a profile file without them reads as before.
    #[test]
    fn the_limits_say_when_they_are_reached() {
        let mut p = AgentProfile::new(ProviderKind::DeepSeek, "Deneme", "deepseek-chat");
        assert_eq!(
            over_budget(&p, 10_000_000, 10_000_000),
            None,
            "no limit written"
        );
        p.conversation_tokens = Some(200_000);
        p.daily_tokens = Some(1_000_000);
        assert_eq!(over_budget(&p, 199_999, 0), None);
        let said = over_budget(&p, 200_000, 0).unwrap();
        assert!(
            said.contains("200.000 / 200.000 token") && said.contains("Yeni sohbet"),
            "{said}"
        );
        let said = over_budget(&p, 0, 1_000_500).unwrap();
        assert!(
            said.contains("bugün") && said.contains("1.000.500 / 1.000.000"),
            "{said}"
        );
        assert_eq!(read_tokens(" 200.000 "), Ok(Some(200_000)));
        assert_eq!(read_tokens("1 000 000"), Ok(Some(1_000_000)));
        assert_eq!(read_tokens(""), Ok(None));
        for wrong in ["0", "1,5", "200k", "-5", "+5", "99999999999999999999999"] {
            assert!(read_tokens(wrong).is_err(), "{wrong}");
        }
        assert_eq!(
            spending_words(&p, 19_819),
            "Bugün 19.819 token harcadı · sınır sohbet başına 200.000, günde 1.000.000 token"
        );
        p.conversation_tokens = None;
        assert_eq!(
            spending_words(&p, 0),
            "Bugün 0 token harcadı · sınır günde 1.000.000 token"
        );
        p.daily_tokens = None;
        assert_eq!(
            spending_words(&p, 5),
            "Bugün 5 token harcadı · token sınırı yok"
        );
        assert_eq!(tokens_words(0), "0");
        assert_eq!(tokens_words(1_234_567), "1.234.567");
        let old: AgentProfile = serde_json::from_str(
            r#"{"id":"x","name":"Eski","kind":"deepseek","model":"deepseek-chat"}"#,
        )
        .unwrap();
        assert_eq!((old.conversation_tokens, old.daily_tokens), (None, None));
    }
}
