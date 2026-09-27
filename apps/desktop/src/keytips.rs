//! The ribbon's key tips' rules (the web's `ui/ribbon/keytips.ts`,
//! docs/specs/ribbon.md §1): the letters of a label, the tips a list of
//! labels gets (one letter where one is free, else two, never one tip the
//! start of another), the first level's digits and letters, and what a key
//! does while the tips show. Both platforms play the `keyTips` part of
//! fixtures/shell/v1/ribbon.json.

/// The web's folding of Turkish letters to plain upper case.
fn fold(lower: char) -> Option<char> {
    Some(match lower {
        'ç' => 'C',
        'ğ' => 'G',
        'ı' | 'i' => 'I',
        'ö' => 'O',
        'ş' => 'S',
        'ü' => 'U',
        'â' => 'A',
        'î' => 'I',
        'û' => 'U',
        _ => return None,
    })
}

/// A character lowered as Turkish lowers it (I → ı, İ → i).
fn tr_lower(ch: char) -> String {
    match ch {
        'I' => "ı".to_owned(),
        'İ' => "i".to_owned(),
        _ => ch.to_lowercase().collect(),
    }
}

/// Letters of a label, upper case, Turkish letters folded, others dropped
/// (“Görünüm: ölçü” → `GORUNUMOLCU`, “3 nokta” → `3NOKTA`).
pub(crate) fn letters_of(label: &str) -> String {
    let mut out = String::new();
    for ch in label.chars() {
        let lower = tr_lower(ch);
        let mut chars = lower.chars();
        let (Some(one), None) = (chars.next(), chars.next()) else {
            continue;
        };
        let up: String = match fold(one) {
            Some(folded) => folded.to_string(),
            None => one.to_uppercase().collect(),
        };
        let mut up_chars = up.chars();
        if let (Some(c), None) = (up_chars.next(), up_chars.next())
            && (c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            out.push(c);
        }
    }
    out
}

/// The words' first letters of a label (Yeni proje → YP).
fn initials(label: &str) -> String {
    label
        .split(|c: char| c.is_whitespace() || matches!(c, '/' | '–' | '-'))
        .filter(|w| !w.is_empty())
        .filter_map(|w| letters_of(w).chars().next())
        .collect()
}

const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// Unique key tips for labels: a single letter where one is free (the
/// label's first letter, nobody else starting with it), otherwise two
/// (initials, then the first letter and another). Tips never start with a
/// single tip, so a typed letter is never ambiguous. `reserved` are taken
/// already (the first level's digits).
pub(crate) fn assign_key_tips(labels: &[&str], reserved: &[String]) -> Vec<String> {
    let mut used: Vec<String> = reserved.to_vec();
    let mut out = vec![String::new(); labels.len()];
    let mut singles: Vec<char> = Vec::new();
    // Pass 1: one letter for labels whose first letter nobody else starts with.
    let firsts: Vec<Option<char>> = labels
        .iter()
        .map(|l| letters_of(l).chars().next())
        .collect();
    for (i, first) in firsts.iter().enumerate() {
        let Some(f) = first else { continue };
        let count = firsts.iter().filter(|x| *x == first).count();
        let tip = f.to_string();
        if count == 1 && !used.contains(&tip) {
            out[i] = tip.clone();
            used.push(tip);
            singles.push(*f);
        }
    }
    // Pass 2: two letters for the rest, never starting with a single tip.
    for (i, label) in labels.iter().enumerate() {
        if !out[i].is_empty() {
            continue;
        }
        let mut letters = letters_of(label);
        if letters.is_empty() {
            letters = "X".to_owned();
        }
        let chars: Vec<char> = letters.chars().collect();
        let first = chars[0];
        let ini: Vec<char> = initials(label).chars().collect();
        let mut candidates: Vec<String> = Vec::new();
        candidates.push(if ini.len() >= 2 {
            ini[..2].iter().collect()
        } else {
            String::new()
        });
        candidates.extend(chars[1..].iter().map(|c| format!("{first}{c}")));
        candidates.extend(ALPHABET.chars().map(|c| format!("{first}{c}")));
        let free = |c: &String, used: &[String]| {
            c.chars().count() == 2
                && !used.contains(c)
                && c.chars().next().is_some_and(|s| !singles.contains(&s))
        };
        let mut tip = candidates.into_iter().find(|c| free(c, &used));
        if tip.is_none() {
            let mut starts: Vec<char> = Vec::new();
            if !singles.contains(&first) {
                starts.push(first);
            }
            starts.extend(ALPHABET.chars().filter(|c| !singles.contains(c)));
            'search: for s in starts {
                for c in ALPHABET.chars() {
                    let pair = format!("{s}{c}");
                    if !used.contains(&pair) {
                        tip = Some(pair);
                        break 'search;
                    }
                }
            }
        }
        if let Some(tip) = tip {
            used.push(tip.clone());
            out[i] = tip;
        }
    }
    out
}

/// The first level's tips: a digit for each of the first nine quick access
/// buttons that can be used now, then a letter or two for each visible tab,
/// never a digit.
pub(crate) fn first_level_tips(
    quick_access: usize,
    tab_labels: &[&str],
) -> (Vec<String>, Vec<String>) {
    let digits: Vec<String> = (1..=quick_access.min(9)).map(|d| d.to_string()).collect();
    let tabs = assign_key_tips(tab_labels, &digits);
    (digits, tabs)
}

/// Which tips show: the tabs and the quick access bar, or the open tab's controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Level {
    Tabs,
    Controls,
}

/// What a key does while the tips show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    /// A tip typed in full: its control runs (a tab opens and its controls get tips).
    Run(String),
    /// The start of one or more tips: the others dim; what is typed so far.
    Typed(String),
    /// Esc at the open tab's controls: back to the tabs.
    Back,
    /// The tips go away.
    Hide,
    /// Nothing happens: Alt or Shift alone, or a letter no tip starts with.
    Ignore,
}

/// A key pressed while the tips show (`key` as the web names it: `d`,
/// `Escape`, `Backspace`, `Alt` …), at a level, with `typed` letters so
/// far. Esc takes back what was typed, then goes back a level, then away;
/// Backspace takes back one letter. A letter (Turkish letters folded) runs
/// the tip it completes or narrows to the tips it starts; any other key, or
/// a letter with Ctrl or ⌘, sends the tips away.
pub(crate) fn key_tip_step(
    level: Level,
    typed: &str,
    tips: &[String],
    key: &str,
    ctrl: bool,
) -> Step {
    if key == "Alt" || key == "Shift" {
        return Step::Ignore;
    }
    if key == "Escape" {
        return if !typed.is_empty() {
            Step::Typed(String::new())
        } else if level == Level::Controls {
            Step::Back
        } else {
            Step::Hide
        };
    }
    if key == "Backspace" && !typed.is_empty() {
        let mut shorter = typed.to_owned();
        shorter.pop();
        return Step::Typed(shorter);
    }
    let ch = letters_of(key);
    if ch.chars().count() != 1 || ctrl {
        return Step::Hide;
    }
    let next = format!("{typed}{ch}");
    if tips.contains(&next) {
        Step::Run(next)
    } else if tips.iter().any(|t| t.starts_with(&next)) {
        Step::Typed(next)
    } else {
        Step::Ignore
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    const FIXTURE: &str = include_str!("../../../fixtures/shell/v1/ribbon.json");

    fn key_tips() -> Value {
        let all: Value = serde_json::from_str(FIXTURE).expect("ribbon.json reads");
        all["keyTips"].clone()
    }

    fn strings(v: &Value) -> Vec<String> {
        v.as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn letters_are_folded_and_upper_case() {
        for case in key_tips()["letters"].as_array().expect("cases") {
            let label = case["label"].as_str().unwrap_or_default();
            assert_eq!(
                letters_of(label),
                case["letters"].as_str().unwrap_or_default(),
                "{label}"
            );
        }
    }

    #[test]
    fn tips_are_unique_and_none_starts_another() {
        for case in key_tips()["assign"].as_array().expect("cases") {
            let labels = strings(&case["labels"]);
            let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
            let reserved = strings(&case["reserved"]);
            assert_eq!(
                assign_key_tips(&labels, &reserved),
                strings(&case["tips"]),
                "{}",
                case["title"]
            );
        }
    }

    #[test]
    fn the_first_level_gives_digits_to_the_bar_and_letters_to_the_tabs() {
        for case in key_tips()["firstLevel"].as_array().expect("cases") {
            let labels = strings(&case["tabLabels"]);
            let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
            let count = case["quickAccess"].as_u64().unwrap_or_default() as usize;
            let (digits, tabs) = first_level_tips(count, &labels);
            assert_eq!(digits, strings(&case["tips"]["quickAccess"]), "{case}");
            assert_eq!(tabs, strings(&case["tips"]["tabs"]), "{case}");
        }
    }

    #[test]
    fn keys_run_narrow_go_back_or_send_the_tips_away() {
        for case in key_tips()["steps"].as_array().expect("cases") {
            let level = match case["level"].as_str() {
                Some("controls") => Level::Controls,
                _ => Level::Tabs,
            };
            let step = key_tip_step(
                level,
                case["typed"].as_str().unwrap_or_default(),
                &strings(&case["tips"]),
                case["key"].as_str().unwrap_or_default(),
                case["ctrl"].as_bool().unwrap_or_default(),
            );
            let want = &case["step"];
            let expected = match want["kind"].as_str() {
                Some("run") => Step::Run(want["tip"].as_str().unwrap_or_default().to_owned()),
                Some("typed") => Step::Typed(want["typed"].as_str().unwrap_or_default().to_owned()),
                Some("back") => Step::Back,
                Some("hide") => Step::Hide,
                _ => Step::Ignore,
            };
            assert_eq!(step, expected, "{case}");
        }
    }
}
