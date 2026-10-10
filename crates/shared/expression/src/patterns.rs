//! Düzenli ifade (docs/adr/0214 §2.4): `eşleşir`, `düzenli_bul`,
//! `düzenli_parça`, `düzenli_gruplar` and `düzenli_değiştir` through
//! `regex-lite` (Rust's `regex` syntax, ASCII classes and case folding). A
//! pattern is read once: a constant one when the expression compiles (one
//! that does not read is a compile error), one that comes from a value the
//! first time a run meets it, the last 16 kept. A pattern that does not read
//! gives an empty value.

use regex_lite::{CaptureLocations, Regex, RegexBuilder};

use crate::compound::{self, Item};
use crate::js::text::{MAX_STRING_UNITS, utf16_len};
use crate::library::Func;
use crate::scalar::{R, Scratch, V, as_text};

/// The longest pattern, in characters.
pub const MOST_CHARS: usize = 1000;
/// Patterns kept read.
const KEPT: usize = 16;
/// `regex-lite`'s limits: the compiled pattern's size and nesting.
const SIZE_LIMIT: usize = 1 << 20;
const NEST_LIMIT: u32 = 100;

/// A pattern read, or why it does not read.
pub fn read(pattern: &str) -> Result<Regex, String> {
    if pattern.chars().count() > MOST_CHARS {
        return Err(format!(
            "Düzenli ifade {MOST_CHARS} karakterden uzun olamaz."
        ));
    }
    RegexBuilder::new(pattern)
        .size_limit(SIZE_LIMIT)
        .nest_limit(NEST_LIMIT)
        .build()
        .map_err(|e| format!("Düzenli ifade okunamadı ({e})."))
}

/// A pattern read and the room its groups are found in, kept for the next
/// object; and the bytes a match can start with, when the pattern says so
/// plainly (`starts`): the search then begins at the first of them, and a
/// text without any has no match.
#[derive(Clone, Debug)]
pub struct Read {
    pub regex: Regex,
    groups: CaptureLocations,
    starts: Option<Box<[bool; 256]>>,
}

/// The pattern's special characters outside a class.
const SPECIAL: &[u8] = b".^$*+?()[]{}|\\";

/// Where the group opened at `open` closes; None when the pattern is not
/// read plainly enough to say (classes and escapes are stepped over).
fn group_end(p: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = open;
    while i < p.len() {
        match p[i] {
            b'\\' => i += 1,
            b'[' => i = class_end(p, i)?,
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Where the class opened at `open` closes.
fn class_end(p: &[u8], open: usize) -> Option<usize> {
    let mut i = open + 1;
    if p.get(i) == Some(&b'^') {
        i += 1;
    }
    // A `]` first is a member.
    if p.get(i) == Some(&b']') {
        i += 1;
    }
    while i < p.len() {
        match p[i] {
            b'\\' => i += 1,
            b'[' => return None,
            b']' => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

/// Whether the atom ending at `end` (exclusive) must be there: no `?`, `*`
/// or `{0` after it.
fn required(p: &[u8], end: usize) -> bool {
    match p.get(end) {
        Some(b'?' | b'*') => false,
        Some(b'{') => !matches!(p.get(end + 1), Some(b'0' | b',')),
        _ => true,
    }
}

/// The bytes a match of `pattern` can start with, when its first atom says
/// so plainly: a literal character (ASCII or not), an escaped punctuation
/// mark, `\d` or a class of ASCII characters and ranges, which must be
/// there, inside groups that must be there too. None for anything else
/// (an alternation, a flag, an anchor, `.`, another escape, a class with a
/// negation or past ASCII …): such a pattern is searched from the start.
fn first_bytes(pattern: &str) -> Option<Box<[bool; 256]>> {
    let p = pattern.as_bytes();
    if p.contains(&b'|') {
        return None;
    }
    let mut set = Box::new([false; 256]);
    let mut i = 0;
    // The groups around the first atom: each must be there.
    while p.get(i) == Some(&b'(') {
        let end = group_end(p, i)?;
        if !required(p, end + 1) {
            return None;
        }
        i += match p.get(i + 1..i + 3) {
            Some(b"?:") => 3,
            Some([b'?', _]) => return None,
            _ => 1,
        };
    }
    let end = match *p.get(i)? {
        b'\\' => {
            match *p.get(i + 1)? {
                b'd' => set[usize::from(b'0')..=usize::from(b'9')].fill(true),
                c if c.is_ascii_punctuation() => set[usize::from(c)] = true,
                _ => return None,
            }
            i + 2
        }
        b'[' => {
            let close = class_end(p, i)?;
            let body = &p[i + 1..close];
            if body.first() == Some(&b'^') || body.is_empty() {
                return None;
            }
            let mut k = 0;
            while k < body.len() {
                let c = body[k];
                if c == b'\\' {
                    match *body.get(k + 1)? {
                        b'd' => set[usize::from(b'0')..=usize::from(b'9')].fill(true),
                        e if e.is_ascii_punctuation() => set[usize::from(e)] = true,
                        _ => return None,
                    }
                    k += 2;
                    continue;
                }
                if !c.is_ascii() {
                    return None;
                }
                if body.get(k + 1) == Some(&b'-') && body.get(k + 2).is_some_and(|&d| d != b']') {
                    let d = body[k + 2];
                    if !d.is_ascii() || d == b'\\' || d < c {
                        return None;
                    }
                    set[usize::from(c)..=usize::from(d)].fill(true);
                    k += 3;
                } else {
                    set[usize::from(c)] = true;
                    k += 1;
                }
            }
            close + 1
        }
        c if c.is_ascii() => {
            if SPECIAL.contains(&c) {
                return None;
            }
            set[usize::from(c)] = true;
            i + 1
        }
        // A character past ASCII: its first byte.
        c => {
            set[usize::from(c)] = true;
            i + pattern[i..].chars().next()?.len_utf8()
        }
    };
    required(p, end).then_some(set)
}

impl Read {
    fn of(regex: Regex, pattern: &str) -> Self {
        Self {
            groups: regex.capture_locations(),
            starts: first_bytes(pattern),
            regex,
        }
    }

    /// Where a match can begin; None when it cannot (the search begins at 0
    /// for a pattern that does not say).
    fn start(&self, t: &str) -> Option<usize> {
        match &self.starts {
            None => Some(0),
            Some(set) => t.bytes().position(|b| set[usize::from(b)]),
        }
    }

    fn is_match(&self, t: &str) -> bool {
        self.start(t)
            .is_some_and(|s| self.regex.find_at(t, s).is_some())
    }

    fn find<'t>(&self, t: &'t str) -> Option<regex_lite::Match<'t>> {
        self.regex.find_at(t, self.start(t)?)
    }
}

/// The patterns a run read last, the latest at the end (None: it did not read).
#[derive(Clone, Debug, Default)]
pub struct Kept {
    kept: Vec<(String, Option<Read>)>,
}

impl Kept {
    /// The pattern read, from the ones kept when it is one of them.
    pub fn get(&mut self, pattern: &str) -> Option<&mut Read> {
        let at = match self.kept.iter().rposition(|(p, _)| p == pattern) {
            Some(i) => i,
            None => {
                if self.kept.len() == KEPT {
                    self.kept.remove(0);
                }
                let read = read(pattern).ok().map(|regex| Read::of(regex, pattern));
                self.kept.push((pattern.to_owned(), read));
                self.kept.len() - 1
            }
        };
        self.kept[at].1.as_mut()
    }
}

impl Read {
    /// The groups of the first match, in the room kept for them (no
    /// allocation per object); None without a match.
    fn first(&mut self, t: &str) -> Option<&CaptureLocations> {
        let start = self.start(t)?;
        self.regex.captures_read_at(&mut self.groups, t, start)?;
        Some(&self.groups)
    }
}

/// Text made at `mark`, unless it is past JavaScript's longest string.
fn made(out: &str, mark: usize) -> R<'static> {
    if out.len() - mark > MAX_STRING_UNITS && utf16_len(&out[mark..]) > MAX_STRING_UNITS {
        R::Thrown
    } else {
        R::Made
    }
}

/// A regular expression function (`f` one of them) on its arguments.
pub fn call(f: Func, args: &[V], out: &mut String, s: &mut Scratch) -> R<'static> {
    let arg = |i: usize| args.get(i).copied().unwrap_or(V::Null);
    let (text, pattern) = (arg(0), arg(1));
    if text == V::Null || pattern == V::Null {
        return R::V(if f == Func::Matches {
            V::Bool(false)
        } else {
            V::Null
        });
    }
    let Scratch {
        a, b, c, patterns, ..
    } = s;
    let t = as_text(text, a);
    let Some(read) = patterns.get(as_text(pattern, b)) else {
        return R::V(V::Null);
    };
    match f {
        Func::Matches => R::V(V::Bool(read.is_match(t))),
        Func::RegexFind => R::V(V::Num(
            read.find(t)
                .map_or(0.0, |m| (utf16_len(&t[..m.start()]) + 1) as f64),
        )),
        Func::RegexPart => {
            let Some(groups) = read.first(t) else {
                return R::V(V::Null);
            };
            // The first group, or the match itself without one.
            let part = if groups.len() > 1 {
                groups.get(1)
            } else {
                groups.get(0)
            };
            out.push_str(part.map_or("", |(s, e)| &t[s..e]));
            R::Made
        }
        Func::RegexGroups => {
            let Some(found) = read.first(t) else {
                return R::V(V::Null);
            };
            let groups: Vec<Item> = (1..found.len())
                .map(|k| Item::Text(found.get(k).map_or("", |(s, e)| &t[s..e]).to_owned()))
                .collect();
            let mark = out.len();
            compound::push_array(out, &groups);
            made(out, mark)
        }
        Func::RegexReplace => {
            let with = as_text(arg(2), c);
            let mark = out.len();
            // A text without a byte a match can start with stays as it is.
            if read.start(t).is_none() {
                out.push_str(t);
            } else {
                out.push_str(&read.regex.replace_all(t, with));
            }
            made(out, mark)
        }
        _ => R::V(V::Null),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call_text(f: Func, args: &[V]) -> String {
        let mut out = String::new();
        match call(f, args, &mut out, &mut Scratch::default()) {
            R::Made => compound::shown(&out).to_owned(),
            R::V(V::Null) => "boş".into(),
            R::V(V::Num(x)) => x.to_string(),
            R::V(V::Bool(b)) => b.to_string(),
            r => format!("{r:?}"),
        }
    }

    #[test]
    fn patterns_find_take_and_replace() {
        let t = V::Text;
        assert_eq!(
            call_text(Func::Matches, &[t("A-12"), t("(?i)a-\\d+")]),
            "true"
        );
        assert_eq!(call_text(Func::Matches, &[V::Null, t(".")]), "false");
        assert_eq!(call_text(Func::RegexFind, &[t("Çam 12"), t("\\d")]), "5");
        assert_eq!(call_text(Func::RegexFind, &[t("abc"), t("\\d")]), "0");
        assert_eq!(
            call_text(
                Func::RegexPart,
                &[t("Ada 1245 Parsel 12"), t("Parsel (\\d+)")]
            ),
            "12"
        );
        assert_eq!(
            call_text(Func::RegexPart, &[t("E-5 yolu"), t("[A-Z]-\\d+")]),
            "E-5"
        );
        assert_eq!(call_text(Func::RegexPart, &[t("yok"), t("\\d")]), "boş");
        assert_eq!(
            call_text(Func::RegexGroups, &[t("1245/12"), t("(\\d+)/(\\d+)(x)?")]),
            r#"["1245","12",""]"#
        );
        assert_eq!(
            call_text(
                Func::RegexReplace,
                &[t("1245/12"), t("(\\d+)/(\\d+)"), t("$2-$1")]
            ),
            "12-1245"
        );
        // A pattern that does not read is empty, and so is one too long.
        assert_eq!(call_text(Func::RegexFind, &[t("a"), t("(")]), "boş");
        assert!(read(&"a".repeat(MOST_CHARS + 1)).is_err());
        assert!(
            read("(")
                .unwrap_err()
                .starts_with("Düzenli ifade okunamadı")
        );
    }

    /// The bytes a match can start with never leave one out: random patterns
    /// made of atoms, quantifiers and groups, on random texts of letters
    /// (Turkish ones too), digits and marks; the search with them gives what
    /// `regex-lite` alone gives, and some patterns have them.
    #[test]
    fn the_first_bytes_never_hide_a_match() {
        const ATOMS: [&str; 22] = [
            "a",
            "b",
            "x",
            "\\d",
            "[0-9]",
            "[a-c]",
            "[x-z0-2]",
            "[-a]",
            "[a-]",
            "[]a]",
            "Ç",
            "ş",
            "\\.",
            "-",
            " ",
            ".",
            "\\w",
            "[^a]",
            "[a-z&&[^x]]",
            "\\b",
            "^",
            "[ç-ü]",
        ];
        const QUANTS: [&str; 10] = ["", "", "+", "*", "?", "{2}", "{0,2}", "{1,3}", "+?", "??"];
        const WRAPS: [(&str, &str); 8] = [
            ("", ""),
            ("", ""),
            ("(", ")"),
            ("(?:", ")"),
            ("(", ")?"),
            ("(", ")*"),
            ("((", "))"),
            ("(?i)", ""),
        ];
        const CHARS: [char; 16] = [
            'a', 'b', 'x', 'y', 'z', '0', '1', '7', 'Ç', 'ş', 'ı', '.', '-', ' ', 'A', ']',
        ];
        let mut seed = 0x2545_f491_4f6c_dd1du64;
        let mut next = |n: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n as u64) as usize
        };
        let (mut patterns, mut with_bytes) = (0, 0);
        for _ in 0..4000 {
            let mut pattern = String::new();
            for _ in 0..1 + next(3) {
                let (open, close) = WRAPS[next(WRAPS.len())];
                pattern.push_str(open);
                pattern.push_str(ATOMS[next(ATOMS.len())]);
                pattern.push_str(QUANTS[next(QUANTS.len())]);
                pattern.push_str(close);
            }
            if next(10) == 0 {
                pattern.push_str("|b");
            }
            let Ok(regex) = read(&pattern) else {
                continue;
            };
            patterns += 1;
            let mut r = Read::of(regex.clone(), &pattern);
            with_bytes += usize::from(r.starts.is_some());
            for _ in 0..12 {
                let t: String = (0..next(12)).map(|_| CHARS[next(CHARS.len())]).collect();
                assert_eq!(r.is_match(&t), regex.is_match(&t), "{pattern:?} on {t:?}");
                let spans = |m: Option<regex_lite::Match<'_>>| m.map(|m| (m.start(), m.end()));
                assert_eq!(
                    spans(r.find(&t)),
                    spans(regex.find(&t)),
                    "{pattern:?} on {t:?}"
                );
                let want: Option<Vec<Option<(usize, usize)>>> = regex.captures(&t).map(|c| {
                    (0..c.len())
                        .map(|k| c.get(k).map(|m| (m.start(), m.end())))
                        .collect()
                });
                let got: Option<Vec<Option<(usize, usize)>>> = r
                    .first(&t)
                    .map(|g| (0..g.len()).map(|k| g.get(k)).collect());
                assert_eq!(got, want, "{pattern:?} on {t:?}");
            }
        }
        assert!(
            patterns > 2500 && with_bytes > 800,
            "{patterns} kalıp, {with_bytes} başlangıçlı"
        );
        // The plain cases.
        let bytes = |p: &str| {
            first_bytes(p).map(|s| {
                (0..=255u8)
                    .filter(|&b| s[usize::from(b)])
                    .collect::<Vec<u8>>()
            })
        };
        assert_eq!(bytes("(\\d+)"), Some((b'0'..=b'9').collect()));
        assert_eq!(bytes("Parsel (\\d+)"), Some(vec![b'P']));
        assert_eq!(bytes("[A-C]-\\d"), Some(vec![b'A', b'B', b'C']));
        assert_eq!(bytes("Çam"), Some(vec![0xC3]));
        for none in [
            "a?b", "(a)?b", "a|b", "(?i)a", "^a", ".a", "\\w+", "[^a]", "a{0,2}", "(?P<n>a)",
        ] {
            assert_eq!(bytes(none), None, "{none}");
        }
    }

    #[test]
    fn the_last_patterns_are_kept() {
        let mut kept = Kept::default();
        for k in 0..40 {
            assert!(kept.get(&format!("a{}", k % 20)).is_some());
        }
        assert_eq!(kept.kept.len(), KEPT);
        assert!(kept.get("(").is_none());
    }
}
