//! Keys written into a package's text (plan/eklenti.md ADIM 22A, "sır taraması (AI araç yapılandırmaları dahil)";
//! PY-CLOUD-05; D11): an API key, a token or a private key in a file a package may carry (`.py`, JSON, TOML, Markdown
//! …). A file that holds secrets by its kind is left out by its name (`archive.rs`, `content_secret`: `.env`, `.pem`,
//! `id_rsa`, AI tools' settings); this reads inside the others. An agent that writes an extension may put the key it
//! was given in the code; a published package hands it to everyone.
//!
//! Only what is a secret by its shape: a provider's prefix with the length and letters of its keys, a private key's
//! armour. A name (`api_key = …`) is not judged: a rule that blocks must be few and sure (D11), a guess warns nobody
//! usefully. A value that is a placeholder (`sk-ant-xxxx…`, `…EXAMPLE`) is not one. The words never repeat the value:
//! its kind and its line only.
//!
//! On packing and installing a found key is a warning (`secret_in_text`): a person's own package for an internal
//! service may carry its key on purpose. The catalog's secret gate blocks it (`secret_found`, docs/adr/8059 §15).

use std::collections::BTreeMap;

use crate::problem::Problem;
use crate::text;

/// A key's shape: its kind in words, how it begins, the characters after it and how many.
struct Form {
    words: &'static str,
    prefix: &'static str,
    body: fn(u8) -> bool,
    least: usize,
    most: usize,
}

fn alnum(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

fn word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

fn token(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'-'
}

fn upper(c: u8) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_digit()
}

const fn form(
    words: &'static str,
    prefix: &'static str,
    body: fn(u8) -> bool,
    least: usize,
    most: usize,
) -> Form {
    Form {
        words,
        prefix,
        body,
        least,
        most,
    }
}

/// The shapes, the narrower before the wider (an Anthropic key before any `sk-` key).
const FORMS: &[Form] = &[
    form("Anthropic API anahtarı", "sk-ant-", token, 32, 256),
    form("OpenAI API anahtarı", "sk-proj-", token, 40, 320),
    form("OpenAI API anahtarı", "sk-svcacct-", token, 40, 320),
    form("Stripe gizli anahtarı", "sk_live_", alnum, 24, 128),
    form("Stripe gizli anahtarı", "rk_live_", alnum, 24, 128),
    form("API anahtarı (sk-…)", "sk-", alnum, 32, 128),
    form("GitHub jetonu", "github_pat_", word, 60, 120),
    form("GitHub jetonu", "ghp_", alnum, 36, 36),
    form("GitHub jetonu", "gho_", alnum, 36, 36),
    form("GitHub jetonu", "ghu_", alnum, 36, 36),
    form("GitHub jetonu", "ghs_", alnum, 36, 36),
    form("GitHub jetonu", "ghr_", alnum, 36, 36),
    form("GitLab jetonu", "glpat-", token, 20, 64),
    form("Slack jetonu", "xoxb-", token, 20, 256),
    form("Slack jetonu", "xoxp-", token, 20, 256),
    form("Slack jetonu", "xoxa-", token, 20, 256),
    form("Slack jetonu", "xoxr-", token, 20, 256),
    form("Slack jetonu", "xoxs-", token, 20, 256),
    form("Google API anahtarı", "AIza", token, 35, 35),
    form("AWS erişim anahtarı", "AKIA", upper, 16, 16),
    form("AWS erişim anahtarı", "ASIA", upper, 16, 16),
    form("Hugging Face jetonu", "hf_", alnum, 30, 40),
    form("PyPI jetonu", "pypi-AgE", token, 50, 400),
    form("npm jetonu", "npm_", alnum, 36, 36),
];

/// A key found: its kind in words and its line (from 1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub words: &'static str,
    pub line: usize,
}

/// A value that stands for a key in an example: one letter over and over, or words a key never holds.
fn placeholder(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    let first = lower.bytes().next();
    lower.bytes().all(|b| Some(b) == first)
        || [
            "xxxx",
            "example",
            "your",
            "dummy",
            "fake",
            "sample",
            "placeholder",
            "redacted",
            "ornek",
            "0123456789",
            "1234567890",
            "abcdefghij",
        ]
        .iter()
        .any(|w| lower.contains(w))
}

/// The keys in `text`, in their order; each place once.
pub fn found(text: &str) -> Vec<Found> {
    let bytes = text.as_bytes();
    let mut out: Vec<(usize, &'static str)> = Vec::new();
    let mut taken: Vec<(usize, usize)> = Vec::new();
    for f in FORMS {
        let mut from = 0;
        while let Some(i) = text[from..].find(f.prefix).map(|i| i + from) {
            from = i + f.prefix.len();
            if i > 0 && token(bytes[i - 1]) {
                continue;
            }
            let start = i + f.prefix.len();
            let run = bytes[start..].iter().take_while(|c| (f.body)(**c)).count();
            if run < f.least || run > f.most || taken.iter().any(|(a, b)| i >= *a && i < *b) {
                continue;
            }
            let body = &text[start..start + run];
            // A random key holds letters and digits both; a word after `sk-` does not.
            let mixed = body.bytes().any(|c| c.is_ascii_digit())
                && body.bytes().any(|c| c.is_ascii_alphabetic());
            if placeholder(body) || (f.prefix == "sk-" && !mixed) {
                continue;
            }
            taken.push((i, start + run));
            out.push((i, f.words));
        }
    }
    // A private key's armour: “-----BEGIN … PRIVATE KEY-----” (RSA, EC, OPENSSH, ENCRYPTED, PKCS#8).
    let mut from = 0;
    while let Some(i) = text[from..].find("-----BEGIN ").map(|i| i + from) {
        from = i + 11;
        let line_end = text[i..].find('\n').map_or(text.len(), |n| i + n);
        if text[i..line_end].contains("PRIVATE KEY-----") {
            out.push((i, "özel anahtar"));
        }
    }
    out.sort_by_key(|(i, _)| *i);
    out.into_iter()
        .map(|(i, words)| Found {
            words,
            line: text[..i].bytes().filter(|b| *b == b'\n').count() + 1,
        })
        .collect()
}

/// Findings shown for one file at most; the rest are counted in the last.
const SHOWN: usize = 5;

/// Packing and installing: a warning for each key in a text file of the package (`secret_in_text`).
pub(crate) fn scan(path: &str, data: &[u8], warnings: &mut Vec<Problem>) {
    if text::kind(path).is_none() {
        return;
    }
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let keys = found(text);
    for (n, key) in keys.iter().enumerate().take(SHOWN) {
        let more = if n + 1 == SHOWN && keys.len() > SHOWN {
            format!(" Bu dosyada {} tane daha var.", keys.len() - SHOWN)
        } else {
            String::new()
        };
        warnings.push(
            Problem::new(
                "secret_in_text",
                format!(
                    "{} gibi görünen bir değer var: paket paylaşılınca anahtar da gider. Değeri dosyadan çıkarın, \
                     kurulumdan sonra kullanıcının girdiği bir ayardan okuyun; paylaşıldıysa anahtarı yenileyin.{more}",
                    upper_first(key.words)
                ),
            )
            .file(path)
            .at(format!("satır {}", key.line)),
        );
    }
}

/// The catalog's secret gate: every key in the package's text files, with its file and line, as the gate's findings
/// say them (`secret_found`). None: the rule found nothing, which proves nothing of the gate a scanner keeps.
pub fn gate_findings(files: &BTreeMap<String, Vec<u8>>) -> Vec<(String, usize, String)> {
    let mut out = Vec::new();
    for (path, data) in files {
        if text::kind(path).is_none() {
            continue;
        }
        let Ok(text) = std::str::from_utf8(data) else {
            continue;
        };
        for key in found(text) {
            out.push((
                path.clone(),
                key.line,
                format!(
                    "Satır {}: {} gibi görünen bir değer var. Değeri paketten çıkarın, anahtarı yenileyin ve sürümü \
                     yeniden gönderin.",
                    key.line, key.words
                ),
            ));
        }
    }
    out
}

fn upper_first(words: &str) -> String {
    let mut chars = words.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A body of `n` characters that looks random, made here: the repository holds no key-shaped literal.
    fn body(n: usize, letters: &[u8]) -> String {
        (0..n)
            .map(|i| letters[(i * 7 + i / 3 + 5) % letters.len()] as char)
            .collect()
    }

    const ALNUM: &[u8] = b"aB3dE5gH7jK9mN2pQ4sT6vW8yZ0cF1";
    const UPPER: &[u8] = b"Q7W3E9R2T5Y8U4I6O1P0ZXCVBN";

    /// Each provider's shape is said by its kind and line, never by its value.
    #[test]
    fn a_key_is_known_by_its_shape() {
        let cases = [
            (
                format!("{}{}", "sk-a", "nt-api03-") + &body(40, ALNUM),
                "Anthropic API anahtarı",
            ),
            (
                format!("{}{}", "sk-p", "roj-") + &body(48, ALNUM),
                "OpenAI API anahtarı",
            ),
            (
                format!("{}{}", "s", "k-") + &body(32, ALNUM),
                "API anahtarı (sk-…)",
            ),
            (
                format!("{}{}", "gh", "p_") + &body(36, ALNUM),
                "GitHub jetonu",
            ),
            (
                format!("{}{}", "AI", "za") + &body(35, ALNUM),
                "Google API anahtarı",
            ),
            (
                format!("{}{}", "AK", "IA") + &body(16, UPPER),
                "AWS erişim anahtarı",
            ),
            (
                format!("{}{}", "xox", "b-") + &body(30, ALNUM),
                "Slack jetonu",
            ),
            (
                format!("{}{}", "hf", "_") + &body(34, ALNUM),
                "Hugging Face jetonu",
            ),
        ];
        for (key, words) in cases {
            let text = format!("import os\n\nANAHTAR = \"{key}\"\n");
            assert_eq!(found(&text), [Found { words, line: 3 }], "{words}");
        }
        let armour = format!("{}{}", "-----BEGIN OPENSSH ", "PRIVATE KEY-----\nabc\n");
        assert_eq!(
            found(&armour),
            [Found {
                words: "özel anahtar",
                line: 1
            }]
        );
    }

    /// Not keys: a placeholder, a short or long run, a word after `sk-`, a prefix inside a longer word, a name.
    #[test]
    fn what_only_looks_like_a_key_is_not_one() {
        for text in [
            format!("{}{}", "sk-ant-", "x".repeat(40)),
            format!("{}{}", "AKIA", "IOSFODNN7EXAMPLE"),
            format!("{}{}", "sk-", "your-api-key-here"),
            format!("{}{}", "ghp_", body(20, ALNUM)),
            format!("{}{}", "sk-", "scikitlearnandnumpyaresomepackagesyes"),
            format!("{}{}", "task-", body(40, ALNUM)),
            "api_key = os.environ[\"DEEPSEEK_API_KEY\"]".to_owned(),
            "-----BEGIN CERTIFICATE-----".to_owned(),
        ] {
            assert!(found(&text).is_empty(), "{text}");
        }
    }

    /// Packing warns with the file and line; the catalog's gate says the same with its own code's words.
    #[test]
    fn packing_warns_and_the_gate_finds() {
        let key = format!("{}{}", "sk-ant-api03-", body(40, ALNUM));
        let code = format!("\"\"\"Örnek.\"\"\"\n\nfrom kentos import ext\n\nKEY = \"{key}\"\n");
        let mut warnings = Vec::new();
        scan("src/arac.py", code.as_bytes(), &mut warnings);
        assert_eq!(warnings.len(), 1);
        let w = &warnings[0];
        assert_eq!(
            (w.code, w.file.as_deref(), w.at.as_deref()),
            ("secret_in_text", Some("src/arac.py"), Some("satır 5"))
        );
        assert!(
            w.message.starts_with("Anthropic API anahtarı gibi görünen"),
            "{}",
            w.message
        );
        assert!(!w.message.contains(&key), "the value is never said");
        // An image is not read; a file without a key gives nothing.
        let mut none = Vec::new();
        scan("assets/simge.png", code.as_bytes(), &mut none);
        scan("src/temiz.py", b"print('merhaba')\n", &mut none);
        assert!(none.is_empty());
        let files = BTreeMap::from([
            (
                "README.md".to_owned(),
                format!("Anahtar: {key}\n").into_bytes(),
            ),
            ("src/temiz.py".to_owned(), b"x = 1\n".to_vec()),
        ]);
        let gate = gate_findings(&files);
        assert_eq!(gate.len(), 1);
        assert_eq!((gate[0].0.as_str(), gate[0].1), ("README.md", 1));
        assert!(gate[0].2.contains("Anthropic API anahtarı") && !gate[0].2.contains(&key));
    }
}
