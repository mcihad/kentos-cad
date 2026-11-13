//! A package's text files (plan/eklenti.md §3.1, D11): UTF-8, no character a person reviewing them cannot see, no
//! path of the author's computer.
//!
//! GlassWorm (2025) hid its code in variation selectors and invisible characters a reviewer saw as nothing; Unicode
//! tag characters hide instructions an AI agent reads. So in code and data (`.py`, JSON, TOML, the lock, SVG …) every
//! invisible formatting character is refused: zero width, joiners, direction controls, variation selectors, tags,
//! fillers; and control characters but tab and line breaks, which can rewrite a terminal during review. Prose (`.md`,
//! `.txt`, `LICENSE`) keeps an emoji's joiner and presentation selector. Turkish letters are free everywhere.

use crate::paths;
use crate::problem::Problem;

/// How a file is read for these rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// Code and data: no invisible character at all.
    Code,
    /// Prose: an emoji's joiner (U+200D) and presentation selector (U+FE0F) allowed.
    Prose,
}

/// How `path` is read: none for a file not of text (an image, a GeoPackage).
pub(crate) fn kind(path: &str) -> Option<Kind> {
    match paths::extension(path).as_str() {
        "py" | "pyi" | "json" | "kform" | "toml" | "lock" | "svg" | "csv" | "cfg" | "ini"
        | "yaml" | "yml" | "sql" | "xml" | "qml" | "css" => Some(Kind::Code),
        "md" | "txt" | "rst" => Some(Kind::Prose),
        // LICENSE, NOTICE, .gitignore.
        "" => Some(Kind::Prose),
        _ => None,
    }
}

/// An invisible or formatting character's name, when it is one.
fn invisible(c: char) -> Option<&'static str> {
    Some(match u32::from(c) {
        0x00AD => "yumuşak tire",
        0x034F => "grafem birleştirici",
        0x061C | 0x200E | 0x200F | 0x202A..=0x202E | 0x2066..=0x2069 => "yazı yönü denetimi",
        0x115F | 0x1160 | 0x3164 | 0xFFA0 => "Hangul dolgusu",
        0x180E => "Moğolca ünlü ayırıcı",
        0x200B => "sıfır genişlikli boşluk",
        0x200C => "sıfır genişlikli ayırıcı",
        0x200D => "sıfır genişlikli birleştirici",
        0x2060 => "sözcük birleştirici",
        0x2061..=0x2064 => "görünmez işleç",
        0x206A..=0x206F => "eski biçim denetimi",
        0xFE00..=0xFE0F | 0xE0100..=0xE01EF => "değişken seçici",
        0xFEFF => "sıfır genişlikli bölünmez boşluk (BOM)",
        0xFFF9..=0xFFFB => "satır arası açıklama denetimi",
        0x1D173..=0x1D17A => "müzik biçim denetimi",
        0xE0000..=0xE007F => "etiket karakteri",
        _ => return None,
    })
}

/// Problems shown for one file at most; the rest are counted.
const SHOWN: usize = 5;

/// Checks a text file: problems (refused) and warnings.
pub(crate) fn scan(
    path: &str,
    data: &[u8],
    problems: &mut Vec<Problem>,
    warnings: &mut Vec<Problem>,
) {
    let Some(kind) = kind(path) else { return };
    let text = match std::str::from_utf8(data) {
        Ok(text) => text,
        Err(e) => {
            problems.push(
                Problem::new(
                    "text_encoding",
                    format!(
                        "Dosya UTF-8 değil ({}. baytta geçersiz dizi); UTF-8 olarak kaydedin.",
                        e.valid_up_to() + 1
                    ),
                )
                .file(path),
            );
            return;
        }
    };
    let data_file = matches!(
        paths::extension(path).as_str(),
        "json" | "kform" | "toml" | "lock"
    );
    let mut found = 0usize;
    let (mut line, mut column) = (1usize, 0usize);
    for (offset, c) in text.char_indices() {
        if c == '\n' {
            line += 1;
            column = 0;
            continue;
        }
        column += 1;
        let what = if c == '\u{FEFF}' && offset == 0 && !data_file {
            None
        } else if c == '\u{FEFF}' && offset == 0 {
            Some((
                "text_bom",
                "dosya BOM ile başlıyor; BOM'suz UTF-8 kaydedin".to_owned(),
            ))
        } else if (c.is_control() && !matches!(c, '\t' | '\r'))
            || matches!(u32::from(c), 0x80..=0x9F)
        {
            Some((
                "text_control",
                format!(
                    "denetim karakteri U+{:04X}; inceleme sırasında uçbirimi değiştirebilir. Silin.",
                    u32::from(c)
                ),
            ))
        } else if let Some(name) = invisible(c) {
            let emoji = kind == Kind::Prose && matches!(c, '\u{200D}' | '\u{FE0F}');
            (!emoji).then(|| {
                (
                    "text_invisible",
                    format!(
                        "görünmez karakter U+{:04X} ({name}); inceleyen kişi bunu göremez. Silin.",
                        u32::from(c)
                    ),
                )
            })
        } else {
            None
        };
        if let Some((code, message)) = what {
            found += 1;
            if found <= SHOWN {
                problems.push(
                    Problem::new(code, format!("Satır {line}, sütun {column}: {message}"))
                        .file(path)
                        .at(format!("satır {line}")),
                );
            }
        }
    }
    if found > SHOWN {
        problems.push(
            Problem::new(
                "text_invisible",
                format!(
                    "Bu dosyada {} görünmez ya da denetim karakteri daha var.",
                    found - SHOWN
                ),
            )
            .file(path),
        );
    }
    if let Some((line, shown)) = user_path(text) {
        let problem = Problem::new(
            "text_user_path",
            format!(
                "Satır {line}: kullanıcıya özgü mutlak yol ({shown}…); paketi başka bilgisayarda kırar ve kullanıcı \
                 adını sızdırır. Pakete göreli yol kullanın."
            ),
        )
        .file(path)
        .at(format!("satır {line}"));
        if kind == Kind::Code {
            problems.push(problem);
        } else {
            warnings.push(problem);
        }
    }
}

/// The first absolute path into a user's home folder: `/home/<ad>/`, `/Users/<ad>/`, `C:\Users\<ad>\`. Its line and
/// how it starts. A path inside an address (`site.tr/home/x/`) is not one.
fn user_path(text: &str) -> Option<(usize, String)> {
    let bytes = text.as_bytes();
    let name_then_slash = |from: usize, slash: u8| {
        let rest = &bytes[from.min(bytes.len())..];
        let n = rest
            .iter()
            .take_while(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
            .count();
        n > 0 && rest.get(n) == Some(&slash)
    };
    let starts_free = |at: usize| {
        at == 0
            || matches!(
                bytes[at - 1],
                b' ' | b'\t' | b'\n' | b'"' | b'\'' | b'=' | b'(' | b'[' | b'{' | b',' | b'`'
            )
    };
    for (at, _) in text.char_indices() {
        let rest = &bytes[at..];
        let hit = if rest.starts_with(b"/home/") && starts_free(at) {
            name_then_slash(at + 6, b'/').then_some(6)
        } else if rest.starts_with(b"/Users/") && starts_free(at) {
            name_then_slash(at + 7, b'/').then_some(7)
        } else if rest.len() > 9
            && rest[0].is_ascii_alphabetic()
            && rest[1] == b':'
            && matches!(rest[2], b'\\' | b'/')
            && rest[3..8].eq_ignore_ascii_case(b"users")
            && rest[8] == rest[2]
            && starts_free(at)
        {
            name_then_slash(at + 9, rest[2]).then_some(9)
        } else {
            None
        };
        if let Some(len) = hit {
            let line = text[..at].matches('\n').count() + 1;
            return Some((line, text[at..at + len].to_owned()));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codes(path: &str, text: &str) -> (Vec<&'static str>, Vec<&'static str>) {
        let (mut p, mut w) = (Vec::new(), Vec::new());
        scan(path, text.as_bytes(), &mut p, &mut w);
        (
            p.iter().map(|x| x.code).collect(),
            w.iter().map(|x| x.code).collect(),
        )
    }

    #[test]
    fn invisible_characters_are_refused_in_code_and_turkish_is_free() {
        assert_eq!(
            codes("src/a.py", "print('Mezarlık: ğüşıöç İ')\n"),
            (vec![], vec![])
        );
        assert_eq!(codes("src/a.py", "x = 1\u{200B}\n").0, ["text_invisible"]);
        assert_eq!(codes("src/a.py", "s = 'a\u{FE01}'\n").0, ["text_invisible"]);
        assert_eq!(
            codes("src/a.py", "if admin: \u{202E}x\n").0,
            ["text_invisible"]
        );
        assert_eq!(
            codes("tools/a.tool.json", "{\"a\": \"\u{E0041}\"}").0,
            ["text_invisible"]
        );
        assert_eq!(codes("src/a.py", "a\u{1B}[2Jb").0, ["text_control"]);
        let mut problems = Vec::new();
        scan(
            "src/a.py",
            "a\n\u{200B}b".as_bytes(),
            &mut problems,
            &mut Vec::new(),
        );
        assert_eq!(problems[0].at.as_deref(), Some("satır 2"));
        assert!(
            problems[0]
                .message
                .contains("U+200B (sıfır genişlikli boşluk)"),
            "{}",
            problems[0].message
        );
    }

    #[test]
    fn prose_keeps_an_emojis_joiner_and_a_python_file_its_bom() {
        assert_eq!(
            codes("CHANGELOG.md", "Yeni ❤\u{FE0F} 👩\u{200D}💻\n"),
            (vec![], vec![])
        );
        assert_eq!(
            codes("CHANGELOG.md", "gizli\u{200B}\n").0,
            ["text_invisible"]
        );
        assert_eq!(codes("src/a.py", "\u{FEFF}print(1)\n"), (vec![], vec![]));
        assert_eq!(codes("manifest.json", "\u{FEFF}{}").0, ["text_bom"]);
        assert_eq!(
            codes("assets/a.png", "\u{200B}"),
            (vec![], vec![]),
            "not text"
        );
    }

    #[test]
    fn floods_are_counted_and_bad_utf8_is_named() {
        let (p, _) = codes("src/a.py", &"\u{200B}".repeat(9));
        assert_eq!(p.len(), SHOWN + 1);
        let mut problems = Vec::new();
        scan("src/a.py", b"ab\xffc", &mut problems, &mut Vec::new());
        assert_eq!(problems[0].code, "text_encoding");
        assert!(problems[0].message.contains("3. baytta"));
    }

    #[test]
    fn a_users_home_path_is_refused_in_code_and_warned_in_prose() {
        assert_eq!(
            codes("src/a.py", "p = \"/home/cihad/veri/x.gpkg\"\n").0,
            ["text_user_path"]
        );
        assert_eq!(
            codes("uv.lock", "source = { path = \"/Users/ayse/dev/pkg\" }\n").0,
            ["text_user_path"]
        );
        assert_eq!(
            codes("src/a.py", "p = r'C:\\Users\\ayse\\x'\n").0,
            ["text_user_path"]
        );
        assert_eq!(
            codes("src/a.py", "p = 'c:/users/ayse/x'\n").0,
            ["text_user_path"]
        );
        assert_eq!(
            codes("docs/a.md", "Örnek: /home/kullanici/proje/\n").1,
            ["text_user_path"]
        );
        assert_eq!(
            codes("src/a.py", "u = 'https://site.tr/home/page/x'\n"),
            (vec![], vec![])
        );
        assert_eq!(codes("src/a.py", "p = '/home/'\n"), (vec![], vec![]));
    }
}
