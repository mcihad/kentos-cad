//! `integrity.json` (plan/eklenti.md §3.1, PY-KEK-09): every content file's path, size and SHA-256; not itself. A file
//! in the package and not in the list, in the list and not in the package, or whose bytes do not match makes the
//! package invalid.
//!
//! The package's **content digest** is the SHA-256 of the list's canonical form: one line per file in byte order of
//! the paths, `<sha256> <size> <path>\n`. It does not depend on the ZIP's compression or dates, so it answers "the same
//! version with different content?". The catalog's approval and a signature bind to this digest from **outside**; the
//! list inside a package is not, alone, trust in its publisher.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::pointer;
use crate::problem::Problem;

pub const FILE: &str = "integrity.json";
pub const FORMAT: &str = "kentos.extension-integrity";
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Integrity {
    pub format: String,
    pub schema_version: u32,
    pub files: Vec<FileDigest>,
}

/// A content file: its path, size and SHA-256 (lower-case hex).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileDigest {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

/// SHA-256 in lower-case hex.
pub fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push(char::from(b"0123456789abcdef"[usize::from(byte >> 4)]));
        out.push(char::from(b"0123456789abcdef"[usize::from(byte & 15)]));
    }
    out
}

impl Integrity {
    /// The list of these files, in byte order of their paths.
    pub fn of<'a>(files: impl IntoIterator<Item = (&'a str, &'a [u8])>) -> Integrity {
        let mut files: Vec<FileDigest> = files
            .into_iter()
            .map(|(path, data)| FileDigest {
                path: path.to_owned(),
                size: data.len() as u64,
                sha256: sha256_hex(data),
            })
            .collect();
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Integrity {
            format: FORMAT.to_owned(),
            schema_version: SCHEMA_VERSION,
            files,
        }
    }

    /// The file as the packer writes it: two-space JSON, one file per object, a newline at the end.
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_default();
        text.push('\n');
        text
    }

    /// Reads `integrity.json`: its format and version, then its fields; unknown fields are refused.
    pub fn read(text: &str) -> Result<Integrity, Problem> {
        let problem = |code: &'static str, at: String, message: String| {
            Problem::new(code, message).file(FILE).at(at)
        };
        let integrity: Integrity = serde_json::from_str(text).map_err(|e| {
            problem(
                "integrity_json",
                pointer::at_error(text, &e),
                format!(
                    "integrity.json okunamadı ({}); paketi “kentos kek pack” ile yeniden oluşturun.",
                    e.to_string().split(" at line ").next().unwrap_or_default()
                ),
            )
        })?;
        if integrity.format != FORMAT || integrity.schema_version != SCHEMA_VERSION {
            return Err(problem(
                "integrity_format",
                "/format".to_owned(),
                format!(
                    "integrity.json “{}” sürüm {}; bu KentOS “{FORMAT}” sürüm {SCHEMA_VERSION} okur.",
                    integrity.format, integrity.schema_version
                ),
            ));
        }
        for (i, file) in integrity.files.iter().enumerate() {
            if file.sha256.len() != 64
                || !file
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(problem(
                    "integrity_json",
                    format!("/files/{i}/sha256"),
                    format!("“{}” için özet 64 küçük onaltılık hane olmalı.", file.path),
                ));
            }
        }
        Ok(integrity)
    }

    /// The content digest: `sha256:` and the SHA-256 of the canonical list.
    pub fn digest(&self) -> String {
        let mut sorted: Vec<&FileDigest> = self.files.iter().collect();
        sorted.sort_by(|a, b| a.path.cmp(&b.path));
        let mut hasher = Sha256::new();
        for file in sorted {
            hasher.update(format!("{} {} {}\n", file.sha256, file.size, file.path).as_bytes());
        }
        let digest = hasher.finalize();
        let mut out = String::from("sha256:");
        for byte in digest {
            out.push(char::from(b"0123456789abcdef"[usize::from(byte >> 4)]));
            out.push(char::from(b"0123456789abcdef"[usize::from(byte & 15)]));
        }
        out
    }

    /// Checks the list against the package's content files (every file but `integrity.json`): the problems, each
    /// naming its file.
    pub fn verify(&self, files: &BTreeMap<String, Vec<u8>>) -> Vec<Problem> {
        let mut problems = Vec::new();
        let mut listed: BTreeMap<&str, &FileDigest> = BTreeMap::new();
        for (i, file) in self.files.iter().enumerate() {
            if file.path == FILE {
                problems.push(
                    Problem::new(
                        "integrity_self",
                        "integrity.json kendini listelemez; satırı silin.",
                    )
                    .file(FILE)
                    .at(format!("/files/{i}")),
                );
            } else if listed.insert(file.path.as_str(), file).is_some() {
                problems.push(
                    Problem::new(
                        "integrity_duplicate",
                        format!("“{}” listede iki kez var.", file.path),
                    )
                    .file(FILE)
                    .at(format!("/files/{i}")),
                );
            }
        }
        for (path, data) in files {
            if path == FILE {
                continue;
            }
            match listed.get(path.as_str()) {
                None => problems.push(
                    Problem::new(
                        "integrity_unlisted",
                        "Dosya pakette var ama integrity.json'da yok: paket açıldıktan sonra eklenmiş olabilir. Paketi \
                         yeniden oluşturun.",
                    )
                    .file(path.clone()),
                ),
                Some(entry) if entry.size != data.len() as u64 || entry.sha256 != sha256_hex(data) => problems.push(
                    Problem::new(
                        "integrity_mismatch",
                        "Dosyanın boyu ya da özeti integrity.json'dakiyle tutmuyor: paketlendikten sonra değişmiş. \
                         Paketi yeniden oluşturun ya da kaynağından yeniden indirin.",
                    )
                    .file(path.clone()),
                ),
                Some(_) => {}
            }
        }
        for path in listed.keys() {
            if !files.contains_key(*path) {
                problems.push(
                    Problem::new(
                        "integrity_missing",
                        "integrity.json bu dosyayı listeliyor ama pakette yok. Paketi yeniden oluşturun.",
                    )
                    .file((*path).to_owned()),
                );
            }
        }
        problems
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(list: &[(&str, &str)]) -> BTreeMap<String, Vec<u8>> {
        list.iter()
            .map(|(p, d)| ((*p).to_owned(), d.as_bytes().to_vec()))
            .collect()
    }

    #[test]
    fn the_digest_is_the_canonical_lists_and_order_free() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let a = Integrity::of([("b.txt", &b"2"[..]), ("a.txt", &b"1"[..])]);
        let b = Integrity::of([("a.txt", &b"1"[..]), ("b.txt", &b"2"[..])]);
        assert_eq!(a, b);
        assert_eq!(a.digest(), b.digest());
        assert!(a.digest().starts_with("sha256:") && a.digest().len() == 7 + 64);
        let c = Integrity::of([("a.txt", &b"1"[..]), ("b.txt", &b"3"[..])]);
        assert_ne!(a.digest(), c.digest());
        let read = Integrity::read(&a.to_json()).unwrap();
        assert_eq!(read, a);
        assert!(a.to_json().ends_with("}\n"));
    }

    #[test]
    fn unlisted_missing_and_changed_files_are_named() {
        let package = files(&[("a.txt", "1"), ("b.txt", "2"), ("integrity.json", "{}")]);
        let list = Integrity::of([("a.txt", &b"1"[..]), ("b.txt", &b"2"[..])]);
        assert!(list.verify(&package).is_empty());
        let extra = files(&[("a.txt", "1"), ("b.txt", "2"), ("c.py", "x")]);
        let p = list.verify(&extra);
        assert_eq!(
            (p.len(), p[0].code, p[0].file.as_deref()),
            (1, "integrity_unlisted", Some("c.py"))
        );
        let changed = files(&[("a.txt", "1"), ("b.txt", "22")]);
        assert_eq!(list.verify(&changed)[0].code, "integrity_mismatch");
        let same_size = files(&[("a.txt", "1"), ("b.txt", "9")]);
        assert_eq!(list.verify(&same_size)[0].code, "integrity_mismatch");
        let missing = files(&[("a.txt", "1")]);
        assert_eq!(list.verify(&missing)[0].code, "integrity_missing");
    }

    #[test]
    fn a_list_of_another_format_or_with_unknown_fields_is_refused() {
        let other = r#"{"format": "x", "schemaVersion": 1, "files": []}"#;
        assert_eq!(Integrity::read(other).unwrap_err().code, "integrity_format");
        let unknown = r#"{"format": "kentos.extension-integrity", "schemaVersion": 1, "files": [], "signedBy": "me"}"#;
        let p = Integrity::read(unknown).unwrap_err();
        assert_eq!(
            (p.code, p.at.as_deref()),
            ("integrity_json", Some("/signedBy"))
        );
        let short = r#"{"format": "kentos.extension-integrity", "schemaVersion": 1,
            "files": [{"path": "a", "size": 1, "sha256": "ABC"}]}"#;
        assert_eq!(
            Integrity::read(short).unwrap_err().at.as_deref(),
            Some("/files/0/sha256")
        );
    }
}
