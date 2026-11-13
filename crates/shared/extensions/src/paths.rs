//! A path inside a package (plan/eklenti.md §3.1): what a ZIP entry, an `integrity.json` line and a manifest field may
//! name. ASCII letters, digits, `.`, `_`, `-` and `/` only, so Unicode normalisation, look-alike letters and case
//! folding differ nowhere; relative, no `.`, `..` or empty segment, no segment Windows would change on extraction (a
//! trailing dot, a device name as `CON` or `aux.txt`). The case-folded form finds two names a case-insensitive file
//! system would merge.

/// Bytes a path may have.
pub(crate) const MAX_LEN: usize = 240;

/// What is wrong with `path` as a path inside a package, in Turkish; none when nothing.
pub(crate) fn problem(path: &str) -> Option<String> {
    if path.is_empty() {
        return Some("Yol boş.".to_owned());
    }
    if path.len() > MAX_LEN {
        return Some(format!(
            "Yol {} karakter; en çok {MAX_LEN} olur.",
            path.len()
        ));
    }
    if path.contains('\\') {
        return Some(
            "Yolda ters bölü (\\) var; paket içinde klasörler “/” ile ayrılır.".to_owned(),
        );
    }
    if path.starts_with('/') {
        return Some(
            "Yol mutlak (“/” ile başlıyor); paket içi yollar pakete görelidir.".to_owned(),
        );
    }
    if path.len() >= 2 && path.as_bytes()[1] == b':' {
        return Some(
            "Yol bir sürücü harfiyle başlıyor; paket içi yollar pakete görelidir.".to_owned(),
        );
    }
    if let Some(c) = path
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/')))
    {
        return Some(if c.is_ascii() {
            format!(
                "Yolda “{}” karakteri var; dosya adlarında yalnız İngilizce harf, rakam, “.”, “_” ve “-” olur.",
                c.escape_default()
            )
        } else {
            format!(
                "Yolda “{c}” karakteri var; dosya adları ASCII'dir (içerik Türkçe olabilir, adlar olmaz): Türkçe harfi \
                 karşılığıyla değiştirin (ı→i, ş→s, ğ→g, ü→u, ö→o, ç→c)."
            )
        });
    }
    for segment in path.split('/') {
        match segment {
            "" => {
                return Some(
                    "Yolda boş bir klasör adı var (“//” ya da “/” ile bitiyor).".to_owned(),
                );
            }
            "." | ".." => {
                return Some(format!(
                    "Yolda “{segment}” var; paket dışına ya da yerine işaret eden yol olmaz."
                ));
            }
            _ => {}
        }
        if segment.ends_with('.') {
            return Some(format!(
                "“{segment}” nokta ile bitiyor; Windows bu noktayı siler ve iki dosya karışır."
            ));
        }
        let stem = segment
            .split('.')
            .next()
            .unwrap_or(segment)
            .to_ascii_uppercase();
        let device = matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        ) || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit());
        if device {
            return Some(format!(
                "“{segment}” Windows'ta bir aygıtın adı; dosya o adla açılmaz. Adını değiştirin."
            ));
        }
    }
    None
}

/// The folders above the file and the file: `a/b/c.py` is 3.
pub(crate) fn depth(path: &str) -> usize {
    path.split('/').count()
}

/// The form a case-insensitive file system compares by.
pub(crate) fn folded(path: &str) -> String {
    path.to_ascii_lowercase()
}

/// The path's extension, lower case, without the dot (`py` for `src/a/b.py`); empty when none.
pub(crate) fn extension(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => ext.to_ascii_lowercase(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_paths_are_plain_relative_ascii() {
        for good in [
            "manifest.json",
            "src/mebis/logic.py",
            "assets/mebis.svg",
            ".gitignore",
            "a-b_c.d/e.f",
        ] {
            assert_eq!(problem(good), None, "{good}");
        }
        for bad in [
            "",
            "/etc/passwd",
            "C:/x.py",
            "c:x.py",
            "../x",
            "a/../b",
            "a/./b",
            "a//b",
            "a/",
            "a\\b",
            "mezarlık.py",
            "a b.py",
            "a\u{0}b",
            "dosya.",
            "con.txt",
            "Aux",
            "lpt1.py",
            "COM9",
        ] {
            assert!(problem(bad).is_some(), "{bad:?}");
        }
        assert!(problem(&"a".repeat(MAX_LEN + 1)).is_some());
        assert_eq!(
            problem("console.py"),
            None,
            "a device name only as the whole stem"
        );
        assert!(problem("mezarlık.py").unwrap().contains("ı→i"));
    }

    #[test]
    fn depth_fold_and_extension() {
        assert_eq!(depth("a/b/c.py"), 3);
        assert_eq!(folded("Src/Logic.PY"), "src/logic.py");
        assert_eq!(extension("src/a/b.PY"), "py");
        assert_eq!(extension(".gitignore"), "");
        assert_eq!(extension("LICENSE"), "");
        assert_eq!(extension("uv.lock"), "lock");
    }
}
