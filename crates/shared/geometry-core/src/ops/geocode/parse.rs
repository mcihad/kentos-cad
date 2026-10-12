//! An address's parts (docs/adr/0229 §4): a one-line address read into its
//! postcode, street (name and kind), door, neighbourhood, district and
//! province, the words no part took kept for the locator's districts; or
//! the same parts from a table's columns, each column read by its own rule.
//! A part's name is its key: its words, in their order, joined by single
//! spaces; `1203 / 1` joins as `1203/1`.

use super::fold::{Tag, Token, fold, numbered_word, tag_of, tokens};

/// An address's parts, every name a key of folded words.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Parsed {
    pub postcode: Option<String>,
    pub street: Option<String>,
    pub street_kind: Option<Tag>,
    pub door: Option<String>,
    pub neighbourhood: Option<String>,
    pub district: Option<String>,
    pub province: Option<String>,
    /// The words no part took, in their order.
    pub rest: Vec<String>,
    /// No street and no neighbourhood tag: the name is tried as a street's and a neighbourhood's.
    pub untagged: bool,
}

/// The tokens and which of them a part took.
struct Reading {
    t: Vec<Token>,
    used: Vec<bool>,
}

impl Reading {
    fn word(&self, i: usize) -> Option<&str> {
        self.t.get(i).and_then(Token::word)
    }

    /// A free word at `i`: a word no part took that is no tag.
    fn free(&self, i: usize) -> Option<&str> {
        self.word(i)
            .filter(|w| !self.used[i] && tag_of(w).is_none())
    }

    fn tag(&self, i: usize) -> Option<Tag> {
        self.word(i).and_then(tag_of)
    }

    /// The name read back from before `i`: free words, a numbered word, `/`
    /// and a numbered word joined; it stops at the start, a mark that does
    /// not join, a tag or a word a part took. Its words become taken.
    fn run_before(&mut self, i: usize) -> String {
        let mut words: Vec<String> = Vec::new();
        let mut j = i;
        while j > 0 {
            let k = j - 1;
            let Some(w) = self.free(k).map(str::to_owned) else {
                break;
            };
            if numbered_word(&w)
                && k >= 2
                && self.t[k - 1] == Token::Slash
                && self.free(k - 2).is_some_and(numbered_word)
            {
                let first = self.free(k - 2).map(str::to_owned).unwrap_or_default();
                words.push(format!("{first}/{w}"));
                self.used[k] = true;
                self.used[k - 1] = true;
                self.used[k - 2] = true;
                j = k - 2;
                continue;
            }
            words.push(w);
            self.used[k] = true;
            j = k;
        }
        words.reverse();
        words.join(" ")
    }

    /// A door from the numbered word at `i` (§4.5): its key with a letter
    /// after `/` or `-`, an inner door after `/` read and left; taken.
    fn door_at(&mut self, i: usize) -> Option<String> {
        let w = self.free(i).filter(|w| numbered_word(w))?.to_owned();
        let mut key = door_key(&w);
        self.used[i] = true;
        let mark = self.t.get(i + 1).cloned();
        if matches!(mark, Some(Token::Slash | Token::Dash))
            && let Some(next) = self.free(i + 2).map(str::to_owned)
        {
            let letter = next.chars().count() == 1 && !numbered_word(&next);
            if letter {
                key.push_str(&next);
                self.used[i + 1] = true;
                self.used[i + 2] = true;
            } else if mark == Some(Token::Slash) && numbered_word(&next) {
                self.used[i + 1] = true;
                self.used[i + 2] = true;
            }
        }
        Some(key)
    }
}

/// A door's key (§4): its leading ASCII digits without leading zeros (`0`
/// when all are), then the rest of the word.
pub fn door_key(word: &str) -> String {
    let digits = word.bytes().take_while(u8::is_ascii_digit).count();
    let trimmed = word[..digits].trim_start_matches('0');
    let number = if trimmed.is_empty() && digits > 0 {
        "0"
    } else {
        trimmed
    };
    format!("{number}{}", &word[digits..])
}

/// A door key's number: the value of its leading digits; none past 10⁹.
pub fn door_number(key: &str) -> Option<u64> {
    let digits: String = key.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() || digits.len() > 10 {
        return None;
    }
    digits.parse::<u64>().ok().filter(|n| *n <= 1_000_000_000)
}

/// A one-line address's parts (§4).
pub fn parse(text: &str) -> Parsed {
    let t = tokens(&fold(text));
    let used = vec![false; t.len()];
    let mut r = Reading { t, used };
    let mut out = Parsed::default();
    // 1. The postcode: its place becomes a comma, which cuts names.
    if let Some(i) = (0..r.t.len()).find(|&i| r.word(i).is_some_and(postcode_like)) {
        out.postcode = r.word(i).map(str::to_owned);
        r.t[i] = Token::Comma;
    }
    let n = r.t.len();
    // 2. The door's mark.
    let no = (0..n).find(|&i| r.tag(i) == Some(Tag::Number));
    // 3. The street: its tag's last before the mark, else its last.
    let streets: Vec<usize> = (0..n)
        .filter(|&i| r.tag(i).is_some_and(Tag::is_street))
        .collect();
    let k = no
        .and_then(|no| streets.iter().copied().filter(|&i| i < no).max())
        .or_else(|| streets.last().copied());
    if let Some(k) = k {
        r.used[k] = true;
        let name = r.run_before(k);
        if !name.is_empty() {
            out.street = Some(name);
            out.street_kind = r.tag(k);
        }
    }
    // 5. The door.
    if let Some(no) = no {
        r.used[no] = true;
        let mut j = no + 1;
        while r.t.get(j) == Some(&Token::Dash) {
            j += 1;
        }
        out.door = r.door_at(j);
    } else if let Some(k) = k {
        let mut j = k + 1;
        while r.t.get(j) == Some(&Token::Comma) {
            j += 1;
        }
        out.door = r.door_at(j);
    }
    // 4. The neighbourhood: its tag's last before the street, else its last.
    let areas: Vec<usize> = (0..n)
        .filter(|&i| r.tag(i).is_some_and(Tag::is_area))
        .collect();
    let m = k
        .and_then(|k| areas.iter().copied().filter(|&i| i < k).max())
        .or_else(|| areas.last().copied());
    if let Some(m) = m {
        r.used[m] = true;
        let name = r.run_before(m);
        if !name.is_empty() {
            out.neighbourhood = Some(name);
        }
    }
    if k.is_none() && m.is_none() {
        // 8. An untagged address: its name is the words up to the first comma, slash or door.
        out.untagged = true;
        if no.is_none() {
            let first = (1..n).find(|&i| r.free(i).is_some_and(numbered_word));
            if let Some(i) = first {
                out.door = r.door_at(i);
            }
        }
        let mut words = Vec::new();
        for i in 0..n {
            match &r.t[i] {
                Token::Word(_) if r.used[i] => {
                    if out.door.is_some() {
                        break;
                    }
                }
                Token::Word(w) => {
                    if tag_of(w).is_none() {
                        words.push(w.clone());
                        r.used[i] = true;
                    }
                }
                _ => break,
            }
        }
        if !words.is_empty() {
            out.street = Some(words.join(" "));
        }
    } else {
        // 6. The district and the province: the last `A / B` of free plain words.
        for s in (0..n).rev() {
            if r.t[s] != Token::Slash || r.used[s] {
                continue;
            }
            let plain = |r: &Reading, i: usize| r.free(i).is_some_and(|w| !numbered_word(w));
            let mut a = s;
            while a > 0 && plain(&r, a - 1) {
                a -= 1;
            }
            let mut b = s + 1;
            while b < n && plain(&r, b) {
                b += 1;
            }
            if a < s && b > s + 1 {
                let join = |r: &Reading, range: std::ops::Range<usize>| {
                    range
                        .filter_map(|i| r.word(i).map(str::to_owned))
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                out.district = Some(join(&r, a..s));
                out.province = Some(join(&r, s + 1..b));
                for i in a..b {
                    r.used[i] = true;
                }
                break;
            }
        }
    }
    // 7. The rest.
    out.rest = (0..n)
        .filter_map(|i| r.free(i).map(str::to_owned))
        .collect();
    out
}

/// Whether a word is a postcode: five ASCII digits, its first two 01–81.
fn postcode_like(w: &str) -> bool {
    w.len() == 5
        && w.bytes().all(|b| b.is_ascii_digit())
        && (1..=81).contains(&w[..2].parse::<u32>().unwrap_or(0))
}

/// A street column's value (§3): the name before its last street tag and
/// that kind, or all its words and no kind; none when empty.
pub fn street_value(text: &str) -> Option<(String, Option<Tag>)> {
    let t = tokens(&fold(text));
    let n = t.len();
    let mut r = Reading {
        t,
        used: vec![false; n],
    };
    let k = (0..n).rev().find(|&i| r.tag(i).is_some_and(Tag::is_street));
    let (name, kind) = match k {
        Some(k) => {
            r.used[k] = true;
            (r.run_before(k), r.tag(k))
        }
        None => (words_key(text), None),
    };
    (!name.is_empty()).then_some((name, kind))
}

/// A door column's value (§4): the first numbered word's door key.
pub fn door_value(text: &str) -> Option<String> {
    let t = tokens(&fold(text));
    let n = t.len();
    let mut r = Reading {
        t,
        used: vec![false; n],
    };
    let i = (0..n).find(|&i| r.free(i).is_some_and(numbered_word))?;
    r.door_at(i)
}

/// A neighbourhood column's value (§3): its words without the MAHALLE and KÖY tags.
pub fn area_value(text: &str) -> String {
    tokens(&fold(text))
        .iter()
        .filter_map(Token::word)
        .filter(|w| !tag_of(w).is_some_and(Tag::is_area))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Any other column's key: all its words.
pub fn words_key(text: &str) -> String {
    tokens(&fold(text))
        .iter()
        .filter_map(Token::word)
        .collect::<Vec<_>>()
        .join(" ")
}

/// An address given as columns (§4): each read by its own rule; empty columns give nothing.
pub fn from_columns(
    neighbourhood: &str,
    street: &str,
    door: &str,
    district: &str,
    province: &str,
    postcode: &str,
) -> Parsed {
    let some = |s: String| (!s.is_empty()).then_some(s);
    let (street, kind) = street_value(street).map_or((None, None), |(n, k)| (Some(n), k));
    Parsed {
        postcode: some(postcode.trim().to_owned()),
        street,
        street_kind: kind,
        door: door_value(door),
        neighbourhood: some(area_value(neighbourhood)),
        district: some(words_key(district)),
        province: some(words_key(province)),
        rest: Vec::new(),
        untagged: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_address_reads_into_its_parts() {
        let p = parse("Kızılay Mah. Atatürk Blv. No:12/3 06420 Çankaya/Ankara");
        assert_eq!(p.neighbourhood.as_deref(), Some("kizilay"));
        assert_eq!(p.street.as_deref(), Some("ataturk"));
        assert_eq!(p.street_kind, Some(Tag::Boulevard));
        assert_eq!(p.door.as_deref(), Some("12"));
        assert_eq!(p.postcode.as_deref(), Some("06420"));
        assert_eq!(p.district.as_deref(), Some("cankaya"));
        assert_eq!(p.province.as_deref(), Some("ankara"));
        assert!(p.rest.is_empty() && !p.untagged);
    }

    #[test]
    fn numbered_streets_and_lettered_doors() {
        let p = parse("Cumhuriyet Mahallesi 1203/1. Sokak No: 5/A");
        assert_eq!(p.street.as_deref(), Some("1203/1"));
        assert_eq!(p.door.as_deref(), Some("5a"));
        let p = parse("Atatürk Cad. 15. Sok. 7");
        assert_eq!(p.street.as_deref(), Some("15"));
        assert_eq!(p.street_kind, Some(Tag::Street));
        assert_eq!(p.door.as_deref(), Some("7"));
    }

    #[test]
    fn an_untagged_address_is_a_name_and_a_door() {
        let p = parse("Atatürk 12, Çankaya");
        assert!(p.untagged);
        assert_eq!(p.street.as_deref(), Some("ataturk"));
        assert_eq!(p.door.as_deref(), Some("12"));
        assert_eq!(p.rest, ["cankaya"]);
    }

    #[test]
    fn door_keys_drop_leading_zeros() {
        assert_eq!(door_key("012"), "12");
        assert_eq!(door_key("000"), "0");
        assert_eq!(door_key("12a"), "12a");
        assert_eq!(door_number("12a"), Some(12));
        assert_eq!(door_value("No: 012/B"), Some("12b".into()));
        assert_eq!(
            street_value("1203/1 Sokak"),
            Some(("1203/1".into(), Some(Tag::Street)))
        );
        assert_eq!(area_value("Kızılay Mahallesi"), "kizilay");
    }
}
