//! A text's editing rules (docs/adr/0145 §3): Artır's next number and Bul ve
//! değiştir's matching, with or without wildcards. Both platforms call these
//! (the web through WASM); fixtures/text/v1 holds them to the independent
//! reference (scripts/fixtures/text_cases.py).

/// The text with the number it ends with one more (Artır): the ASCII digits
/// at its end as a decimal number, one more, as many digits as they were at
/// least (`A-009` → `A-010`, `99` → `100`); none for a text that does not end
/// with a digit.
pub fn increment(text: &str) -> Option<String> {
    let digits = text.bytes().rev().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let (head, tail) = text.split_at(text.len() - digits);
    let mut out: Vec<u8> = tail.bytes().collect();
    // One more, carried from the last digit.
    let mut i = out.len();
    loop {
        if i == 0 {
            out.insert(0, b'1');
            break;
        }
        i -= 1;
        if out[i] == b'9' {
            out[i] = b'0';
        } else {
            out[i] += 1;
            break;
        }
    }
    Some(format!("{head}{}", String::from_utf8_lossy(&out)))
}

/// How Bul ve değiştir looks (docs/adr/0145 §3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Find {
    /// `*` is any run of characters and the pattern matches the whole text.
    pub wildcard: bool,
    /// Upper and lower case the same, Turkish: I is ı's, İ is i's.
    pub caseless: bool,
    /// Without wildcards, only a match with no letter, digit or `_` next to it.
    pub whole_word: bool,
}

crate::json_struct!(Find {
    wildcard,
    caseless,
    whole_word => "wholeWord"
});

/// A character as a caseless match takes it: Turkish I → ı, İ → i, any other
/// its lowercase when that is one character.
pub(crate) fn fold(c: char) -> char {
    match c {
        'I' => 'ı',
        'İ' => 'i',
        _ => {
            let mut low = c.to_lowercase();
            match (low.next(), low.next()) {
                (Some(l), None) => l,
                _ => c,
            }
        }
    }
}

fn same(a: char, b: char, caseless: bool) -> bool {
    if caseless { fold(a) == fold(b) } else { a == b }
}

fn word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether `find` is at `text[at..]`.
fn at(text: &[char], at: usize, find: &[char], caseless: bool) -> bool {
    at + find.len() <= text.len()
        && text[at..at + find.len()]
            .iter()
            .zip(find)
            .all(|(a, b)| same(*a, *b, caseless))
}

/// `text` with `find` replaced by `with` as `how` says; none when nothing
/// matches (docs/adr/0145 §3). Without wildcards every occurrence, left to
/// right, none overlapping; with them the pattern matches the whole text and
/// the replacement's `*`s take what the pattern's did, in order.
pub fn replace(text: &str, find: &str, with: &str, how: Find) -> Option<String> {
    if find.is_empty() {
        return None;
    }
    let text: Vec<char> = text.chars().collect();
    let find: Vec<char> = find.chars().collect();
    if how.wildcard {
        let captures = glob(&text, &find, how.caseless)?;
        let mut taken = captures.into_iter();
        return Some(
            with.chars()
                .map(|c| {
                    if c == '*' {
                        taken.next().unwrap_or_default()
                    } else {
                        c.to_string()
                    }
                })
                .collect(),
        );
    }
    let mut out = String::with_capacity(text.len());
    let (mut i, mut hit) = (0, false);
    while i < text.len() {
        if at(&text, i, &find, how.caseless) {
            let end = i + find.len();
            let bounded = !how.whole_word
                || ((i == 0 || !word_char(text[i - 1]))
                    && (end == text.len() || !word_char(text[end])));
            if bounded {
                out.push_str(with);
                i = end;
                hit = true;
                continue;
            }
        }
        out.push(text[i]);
        i += 1;
    }
    hit.then_some(out)
}

/// Whether `text` answers a search for `pattern` (the point editor's
/// search box, docs/adr/0153 §2): caseless the Turkish way; with a `*` the
/// pattern matches the whole text (`*` any run of characters), without one
/// it is somewhere in it. An empty pattern is in every text.
pub fn search(text: &str, pattern: &str) -> bool {
    let text: Vec<char> = text.chars().collect();
    let pattern: Vec<char> = pattern.chars().collect();
    if pattern.contains(&'*') {
        return glob(&text, &pattern, true).is_some();
    }
    pattern.len() <= text.len()
        && (0..=text.len() - pattern.len()).any(|i| at(&text, i, &pattern, true))
}

/// What each `*` of `pattern` takes when it matches all of `text`, each as
/// short as it can be from the left; none without a match. The literal runs
/// between the stars are found leftmost: a later place never lets the rest
/// match where the leftmost does not (a star follows each run but the last).
fn glob(text: &[char], pattern: &[char], caseless: bool) -> Option<Vec<String>> {
    let parts: Vec<&[char]> = pattern.split(|c| *c == '*').collect();
    let (first, last) = (parts[0], parts[parts.len() - 1]);
    if parts.len() == 1 {
        return (text.len() == first.len() && at(text, 0, first, caseless)).then(Vec::new);
    }
    if !at(text, 0, first, caseless) || text.len() < first.len() + last.len() {
        return None;
    }
    // The last run ends the text; the ones between are found leftmost before it.
    let end = text.len() - last.len();
    if !at(text, end, last, caseless) {
        return None;
    }
    let mut captures = Vec::with_capacity(parts.len() - 1);
    let mut pos = first.len();
    for run in &parts[1..parts.len() - 1] {
        let found = (pos..=end.checked_sub(run.len())?).find(|&q| at(text, q, run, caseless))?;
        captures.push(text[pos..found].iter().collect());
        pos = found + run.len();
    }
    if pos > end {
        return None;
    }
    captures.push(text[pos..end].iter().collect());
    Some(captures)
}
