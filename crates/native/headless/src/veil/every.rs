//! Every field hidden (veil.rs, docs/adr/8057 §103, PY-AI-13): the veil of an agent the user or the organisation sends
//! no attribute value, and how a free text hides a value there.

use std::collections::BTreeSet;

use kentos_domain::Document;
use kentos_native_application::python_call::{HIDDEN, redacted_names};

use super::Veil;

/// The keys whose texts say what an answer's parts are, not what the drawing holds: a free text's words are not
/// hidden there ([`Veil::everything`]), only a quoted value.
pub const STRUCTURAL: [&str; 14] = [
    "id", "uid", "uids", "layerId", "kind", "type", "command", "code", "field", "fields", "path",
    "rule", "version", "revision",
];

/// What a drawing's summary says to an agent sent no attribute value (`valuesWithheld`).
pub const WITHHELD: &str = "Bu ajana öznitelik değerleri gönderilmiyor: kullanıcının ya da kurumun seçimi (Uygulama \
                            ayarları › Yapay zekâ › ajan › Gönderilen veri). Değerleri ekranda kullanıcı görür.";

/// The fewest letters of a value a free text hides as words ([`Veil::everything`]).
const LEAST_WORD: usize = 3;

/// The most words a value hidden in a free text has (`Ali Veli`, `12/3 parsel`).
const MOST_WORDS: usize = 4;

impl Veil {
    /// Every field of `doc` hidden from an agent and every value they hold (docs/adr/8057 §103, PY-AI-13: the user's
    /// or the organisation's choice that no attribute value goes to an AI service): the names its objects' attributes
    /// use and those its forms and layers hide. None when the drawing has no attribute at all.
    pub fn everything(doc: &Document) -> Option<Veil> {
        let mut names: BTreeSet<String> = redacted_names(doc).into_iter().collect();
        let mut values = BTreeSet::new();
        for e in doc.entities() {
            for (k, v) in &e.base().attrs {
                names.insert(k.clone());
                if !v.is_empty() {
                    values.insert(v.clone());
                }
            }
        }
        if names.is_empty() {
            return None;
        }
        Some(Veil {
            names: names.into_iter().collect(),
            values,
            everything: true,
        })
    }

    /// The veil an agent reads `doc` through: every field when `everything`, else the fields the drawing hides.
    pub fn for_agent(doc: &Document, everything: bool) -> Option<Veil> {
        if everything {
            Veil::everything(doc)
        } else {
            Veil::of(doc)
        }
    }

    /// [`Veil::new`] hiding every field ([`Veil::everything`]): tests and other hosts.
    pub fn new_everything(names: Vec<String>, values: impl IntoIterator<Item = String>) -> Veil {
        Veil {
            everything: true,
            ..Veil::new(names, values)
        }
    }

    /// Whether every field is hidden.
    pub fn hides_everything(&self) -> bool {
        self.everything
    }

    /// A text with every value of [`LEAST_WORD`] letters and more it holds as whole words (up to [`MOST_WORDS`] of
    /// them, with what stands between) written as hidden, the longest first.
    pub(super) fn words(&self, text: &str) -> String {
        if self.values.is_empty() {
            return text.to_owned();
        }
        // The text in runs: a word (letters and digits) or what stands between two.
        let mut runs: Vec<(bool, &str)> = Vec::new();
        let mut start = 0;
        let mut word = None;
        for (i, c) in text.char_indices() {
            let is_word = c.is_alphanumeric();
            if word != Some(is_word) {
                if let Some(w) = word {
                    runs.push((w, &text[start..i]));
                }
                start = i;
                word = Some(is_word);
            }
        }
        if let Some(w) = word {
            runs.push((w, &text[start..]));
        }
        let mut out = String::with_capacity(text.len());
        let mut i = 0;
        while i < runs.len() {
            if !runs[i].0 {
                out.push_str(runs[i].1);
                i += 1;
                continue;
            }
            // The longest value starting at this word: one to MOST_WORDS words and what lies between them.
            let mut taken = None;
            let mut end = i;
            let mut words = 0;
            while end < runs.len() && words < MOST_WORDS {
                if runs[end].0 {
                    words += 1;
                    let phrase: String = runs[i..=end].iter().map(|r| r.1).collect();
                    if phrase.chars().count() >= LEAST_WORD && self.values.contains(&phrase) {
                        taken = Some(end);
                    }
                }
                end += 1;
            }
            match taken {
                Some(last) => {
                    out.push_str(HIDDEN);
                    i = last + 1;
                }
                None => {
                    out.push_str(runs[i].1);
                    i += 1;
                }
            }
        }
        out
    }
}
