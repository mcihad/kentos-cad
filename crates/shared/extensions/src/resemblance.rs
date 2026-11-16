//! An extension that imitates another (plan/eklenti.md ADIM 22A, D13; docs/adr/8059 §14): its name, its id or its icon
//! read as another's. In 2026 imitation was the largest group of the extensions an editor's marketplace took down
//! (1,109 of 2,140), and a copy of a well-known formatter reached a thousand installs in two days because its display
//! name was not unique (docs/research/2026-11-13-eklenti-ekosistemleri.md, V14, V21).
//!
//! Two texts are compared by their **skeleton**, what a reader sees rather than what the bytes say: Turkish letters and
//! their bases fold together (ı, İ, i; ş, s …), Cyrillic and Greek letters that look Latin are read as those (а, е, о,
//! р, с, і …), so are the digits and pairs that pass for letters (0 for o; 1, l and I for i; rn for m; vv for w), and
//! what is not a letter or a digit (spaces, hyphens, dots, invisible characters) is left out. The same skeleton is an
//! **imitation** (`exact`): the eye cannot tell the two apart. A skeleton one letter, or two swapped letters, from the
//! other's is a **resemblance**: often an honest name (two municipalities' "Ula" and "Urla"), so it is said, never
//! enough to keep a package out.
//!
//! - The catalog's publisher gate keeps an imitation of another publisher's extension out ([`imitations`]): a rule
//!   that is exact, as D11 asks of the rules that block. A publisher's own extensions are not compared with each
//!   other.
//! - The desktop's install review shows both to the person installing a package: a `.kek` sent by mail carries no
//!   publisher anyone checked, so every installed extension but the one it updates is compared.
//! - An icon is the same when its bytes are; KentOS's blueprint's icon, which every new extension starts with, is
//!   no one's.

use serde::{Deserialize, Serialize};

use crate::integrity::sha256_hex;
use crate::release::Reason;

/// An extension as the comparison reads it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Known {
    pub id: String,
    pub name: String,
    /// Its icon file's SHA-256 (lower-case hex), when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Who publishes it, as the caller knows it (a catalog's publisher account): two extensions of the same publisher
    /// are not compared. None where nobody checked who published it (a package on the person's computer).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
}

/// What reads as another's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Part {
    Name,
    Id,
    Icon,
}

/// One extension read as another.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resemblance {
    pub part: Part,
    /// The same skeleton (an icon: the same bytes): the eye cannot tell them apart.
    pub exact: bool,
    /// The candidate's own words for the part (its name, its id).
    pub own: String,
    /// The other extension's id and name.
    pub other_id: String,
    pub other_name: String,
}

impl Resemblance {
    /// Its stable code: `imitates_*` keeps a package out of a catalog, `resembles_*` is said.
    pub fn code(&self) -> &'static str {
        match (self.part, self.exact) {
            (Part::Name, true) => "imitates_name",
            (Part::Name, false) => "resembles_name",
            (Part::Id, true) => "imitates_id",
            (Part::Id, false) => "resembles_id",
            (Part::Icon, _) => "imitates_icon",
        }
    }

    /// One sentence for a person.
    pub fn words(&self) -> String {
        let other = format!("“{}” ({})", self.other_name, self.other_id);
        match (self.part, self.exact) {
            (Part::Name, true) => format!("Adı, {other} eklentisinin adıyla aynı okunuyor."),
            (Part::Name, false) => format!("Adı, {other} eklentisinin adına çok benziyor."),
            (Part::Id, true) => format!(
                "Kimliği ({}), {other} eklentisinin kimliğiyle aynı okunuyor.",
                self.own
            ),
            (Part::Id, false) => format!(
                "Kimliği ({}), {other} eklentisinin kimliğine çok benziyor.",
                self.own
            ),
            (Part::Icon, _) => format!("Simgesi, {other} eklentisinin simgesiyle bayt bayt aynı."),
        }
    }

    /// As a gate's finding: its code, its words and the other extension.
    pub fn reason(&self) -> Reason {
        let mut reason = Reason::new(self.code(), self.words());
        reason.dependency = Some(self.other_id.clone());
        reason
    }
}

/// The shortest skeleton a near name is compared at: below it one letter is too much of the word.
const NEAR_NAME: usize = 6;
/// The shortest skeleton a near id is compared at.
const NEAR_ID: usize = 10;

/// What `candidate` reads as among `known`, imitations first: the extension itself (the same id, an update) and the
/// same publisher's are not compared.
pub fn resemblances(candidate: &Known, known: &[Known]) -> Vec<Resemblance> {
    let blueprint = sha256_hex(crate::blueprint::ICON.as_bytes());
    let name = skeleton(&candidate.name);
    let id = skeleton(&candidate.id);
    let mut out = Vec::new();
    for other in known {
        if other.id == candidate.id
            || (candidate.publisher.is_some() && candidate.publisher == other.publisher)
        {
            continue;
        }
        let found = |part: Part, exact: bool, own: &str| Resemblance {
            part,
            exact,
            own: own.to_owned(),
            other_id: other.id.clone(),
            other_name: other.name.clone(),
        };
        let other_name = skeleton(&other.name);
        if !name.is_empty() && name == other_name {
            out.push(found(Part::Name, true, &candidate.name));
        } else if near(&name, &other_name, NEAR_NAME) {
            out.push(found(Part::Name, false, &candidate.name));
        }
        let other_id = skeleton(&other.id);
        if id == other_id {
            out.push(found(Part::Id, true, &candidate.id));
        } else if near(&id, &other_id, NEAR_ID) {
            out.push(found(Part::Id, false, &candidate.id));
        }
        if let (Some(mine), Some(theirs)) = (&candidate.icon, &other.icon)
            && mine == theirs
            && *mine != blueprint
        {
            out.push(found(Part::Icon, true, &candidate.name));
        }
    }
    out.sort_by(|a, b| (!a.exact, a.part, &a.other_id).cmp(&(!b.exact, b.part, &b.other_id)));
    out
}

/// `found` said for a person, one sentence for each other extension and kind (`[Resemblance::exact]` or not), in the
/// order found: "Adı, kimliği (tr.ornek.vltrin) ve simgesi “Eklenti Vitrini” (tr.ornek.vitrin) eklentisininkilerle aynı
/// okunuyor." A gate's findings stay one a part ([`Resemblance::reason`]).
pub fn sentences(found: &[Resemblance]) -> Vec<String> {
    let mut groups: Vec<(&str, &str, bool, Vec<String>)> = Vec::new();
    for r in found {
        let part = match r.part {
            Part::Name => "adı".to_owned(),
            Part::Id => format!("kimliği ({})", r.own),
            Part::Icon => "simgesi".to_owned(),
        };
        match groups
            .iter_mut()
            .find(|(id, _, exact, _)| *id == r.other_id && *exact == r.exact)
        {
            Some((_, _, _, parts)) => parts.push(part),
            None => groups.push((&r.other_id, &r.other_name, r.exact, vec![part])),
        }
    }
    groups
        .into_iter()
        .map(|(id, name, exact, parts)| {
            let mut said = match parts.split_last() {
                Some((last, [])) => last.clone(),
                Some((last, rest)) => format!("{} ve {last}", rest.join(", ")),
                None => String::new(),
            };
            if let Some(first) = said.get(..1) {
                said = first.to_uppercase() + &said[1..];
            }
            let (whose, verb) = match (parts.len() > 1, exact) {
                (false, true) => ("eklentisininkiyle", "aynı okunuyor"),
                (true, true) => ("eklentisininkilerle", "aynı okunuyor"),
                (false, false) => ("eklentisininkine", "çok benziyor"),
                (true, false) => ("eklentisininkilere", "çok benziyor"),
            };
            format!("{said} “{name}” ({id}) {whose} {verb}.")
        })
        .collect()
}

/// The imitations among `known` as the publisher gate's findings: what keeps a package out of a catalog.
pub fn imitations(candidate: &Known, known: &[Known]) -> Vec<Reason> {
    resemblances(candidate, known)
        .iter()
        .filter(|r| r.exact)
        .map(Resemblance::reason)
        .collect()
}

/// What a reader sees of a text: lower case, Turkish letters on their bases, letters that look Latin read as those, the
/// digits and pairs that pass for letters folded in, anything but letters and digits left out.
pub fn skeleton(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        let folded = match c {
            'İ' | 'I' | 'ı' | 'i' | 'î' | 'Î' | 'ì' | 'í' | 'ï' | 'l' | 'L' | '1' | '|' | 'ł'
            | 'Ł' => 'i',
            'ç' | 'Ç' | 'ć' | 'č' => 'c',
            'ğ' | 'Ğ' => 'g',
            'ö' | 'Ö' | 'ô' | 'ó' | 'ò' | 'ø' | '0' => 'o',
            'ş' | 'Ş' | 'ś' | 'š' => 's',
            'ü' | 'Ü' | 'û' | 'ú' | 'ù' => 'u',
            'â' | 'Â' | 'á' | 'à' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            // Cyrillic letters that pass for Latin ones.
            'а' | 'А' => 'a',
            'в' | 'В' => 'b',
            'е' | 'Е' | 'ё' | 'Ё' => 'e',
            'к' | 'К' => 'k',
            'м' | 'М' => 'm',
            'н' | 'Н' => 'h',
            'о' | 'О' => 'o',
            'р' | 'Р' => 'p',
            'с' | 'С' => 'c',
            'т' | 'Т' => 't',
            'у' | 'У' => 'y',
            'х' | 'Х' => 'x',
            'ѕ' | 'Ѕ' => 's',
            'і' | 'І' | 'ї' | 'Ї' | 'ӏ' | 'Ӏ' => 'i',
            'ј' | 'Ј' => 'j',
            'ԁ' => 'd',
            'ԛ' => 'q',
            'ԝ' => 'w',
            // Greek letters that pass for Latin ones.
            'α' | 'Α' => 'a',
            'β' | 'Β' => 'b',
            'ε' | 'Ε' => 'e',
            'ζ' | 'Ζ' => 'z',
            'η' | 'Η' => 'h',
            'ι' | 'Ι' => 'i',
            'κ' | 'Κ' => 'k',
            'μ' | 'Μ' => 'm',
            'ν' | 'Ν' => 'v',
            'ο' | 'Ο' => 'o',
            'ρ' | 'Ρ' => 'p',
            'τ' | 'Τ' => 't',
            'υ' | 'Υ' => 'y',
            'χ' | 'Χ' => 'x',
            c if c.is_alphanumeric() => c.to_lowercase().next().unwrap_or(c),
            _ => continue,
        };
        out.push(folded);
    }
    // Pairs that read as one letter.
    out.replace("rn", "m").replace("vv", "w")
}

/// Whether two skeletons, both at least `shortest` letters, are one letter or two swapped letters apart.
pub(crate) fn near(a: &str, b: &str, shortest: usize) -> bool {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    a.len() >= shortest
        && b.len() >= shortest
        && a.len().abs_diff(b.len()) <= 1
        && edits(&a, &b) == 1
}

/// The fewest single-letter changes (insert, remove, replace, swap two neighbours) from one to the other.
fn edits(a: &[char], b: &[char]) -> usize {
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(d[i - 2][j - 2] + 1);
            }
            d[i][j] = best;
        }
    }
    d[a.len()][b.len()]
}

#[cfg(test)]
mod tests;
