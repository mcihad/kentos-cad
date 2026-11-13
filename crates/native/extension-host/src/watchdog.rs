//! The watchdog's threshold and "Sorunlu eklentiyi bul" (plan/eklenti.md ADIM 17; D6).
//!
//! QGIS keeps a plugin that failed to load enabled and fails it again at every start. Here crashes in a row are
//! counted (`report_crash`); at [`CRASHES_TO_FAIL`] the extension goes to a **lasting** error with its reason and is not
//! tried at start-up until the user retries. A start without extensions is the registry's safe mode; when the window
//! misbehaves and no extension is named, [`Bisection`] finds it: first a start with none (is it an extension's at
//! all?), then halves of the suspects, about 1 + log₂ n restarts.

use serde::{Deserialize, Serialize};

/// Crashes in a row (activation or running) that stop an extension for good.
pub const CRASHES_TO_FAIL: u32 = 3;

/// Finding the extension behind a problem: each round enables [`Bisection::testing`] alone, the user says whether the
/// problem is still there. Kept in the registry's folder while it goes on (`sorun-bulma.json`), so a restart between
/// rounds goes on where it was.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Bisection {
    suspects: Vec<String>,
    testing: Vec<String>,
    /// The round with no extension has been answered.
    baseline: bool,
    rounds: u32,
}

/// Where a bisection stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Finding {
    /// Another round: enable these alone (none: start without extensions) and restart.
    Testing(Vec<String>),
    /// This extension.
    Found(String),
    /// The problem is there with no extension enabled: not an extension's.
    NotAnExtension,
}

impl Bisection {
    /// Starts with the extensions enabled when the problem was seen.
    pub fn new(enabled: Vec<String>) -> Bisection {
        Bisection {
            suspects: enabled,
            testing: Vec::new(),
            baseline: false,
            rounds: 0,
        }
    }

    /// What to do now.
    pub fn finding(&self) -> Finding {
        match self.suspects.as_slice() {
            [] => Finding::NotAnExtension,
            _ if !self.baseline => Finding::Testing(Vec::new()),
            [one] => Finding::Found(one.clone()),
            _ => Finding::Testing(self.testing.clone()),
        }
    }

    /// The user's answer for the round: whether the problem was still there with [`Bisection::testing`] alone enabled.
    pub fn answer(&mut self, problem_seen: bool) {
        if matches!(self.finding(), Finding::Found(_) | Finding::NotAnExtension) {
            return;
        }
        self.rounds += 1;
        if !self.baseline {
            self.baseline = true;
            if problem_seen {
                self.suspects.clear();
            }
        } else if problem_seen {
            self.suspects = std::mem::take(&mut self.testing);
        } else {
            let tested = std::mem::take(&mut self.testing);
            self.suspects.retain(|s| !tested.contains(s));
        }
        self.testing = self.suspects[..self.suspects.len() / 2].to_vec();
    }

    /// The extensions to enable this round, the others off.
    pub fn testing(&self) -> &[String] {
        if self.baseline { &self.testing } else { &[] }
    }

    /// Rounds answered so far.
    pub fn rounds(&self) -> u32 {
        self.rounds
    }

    /// The extensions still suspected.
    pub fn suspects(&self) -> &[String] {
        &self.suspects
    }

    /// The most rounds `n` suspects can take: the one with none, then halves.
    pub fn most_rounds(n: usize) -> u32 {
        let mut halves = 0;
        while (1usize << halves) < n {
            halves += 1;
        }
        1 + halves
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("tr.ornek.e{i}")).collect()
    }

    /// Runs a bisection answered truthfully: the problem is seen when `culprit` is enabled (none: always).
    fn find(n: usize, culprit: Option<usize>) -> (Finding, u32) {
        let all = names(n);
        let mut b = Bisection::new(all.clone());
        loop {
            match b.finding() {
                Finding::Testing(enabled) => {
                    let seen = culprit.is_none_or(|c| enabled.contains(&all[c]));
                    b.answer(seen);
                    assert!(b.rounds() <= 12, "runs away");
                }
                done => return (done, b.rounds()),
            }
        }
    }

    #[test]
    fn halves_find_the_culprit_in_one_plus_log2_rounds() {
        for n in [1usize, 2, 3, 5, 8, 13, 40] {
            for culprit in 0..n {
                let (found, rounds) = find(n, Some(culprit));
                assert_eq!(
                    found,
                    Finding::Found(format!("tr.ornek.e{culprit}")),
                    "{n} / {culprit}"
                );
                let most = 1 + (n as f64).log2().ceil() as u32;
                assert_eq!(Bisection::most_rounds(n), most);
                assert!(rounds <= most, "{n} extensions: {rounds} rounds > {most}");
            }
        }
    }

    #[test]
    fn a_problem_without_any_extension_is_not_an_extensions() {
        assert_eq!(find(6, None), (Finding::NotAnExtension, 1));
        assert_eq!(
            Bisection::new(Vec::new()).finding(),
            Finding::NotAnExtension
        );
        let b = Bisection::new(names(3));
        assert_eq!(
            b.finding(),
            Finding::Testing(Vec::new()),
            "first a start with none"
        );
        assert!(b.testing().is_empty());
    }
}
