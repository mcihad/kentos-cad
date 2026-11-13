//! Semantic versions (semver.org 2.0.0) and the ranges a dependency or a host is given in (docs/adr/8057 §1).
//!
//! A version is `MAJOR.MINOR.PATCH`, with a pre-release after `-` (`1.2.0-beta.1`: the preview channel, plan/eklenti.md
//! §3.2) and build metadata after `+` (ignored in comparing). A range is comparators joined by commas, every one holding:
//! `>=1.2, <2`, `^1.2` (the same major, or the same minor below 1.0), `~1.2.3` (the same minor), `=1.2.3`, `>`, `<`,
//! `<=`; a bare version is `^`, as in Cargo. A pre-release version satisfies a range only when one of its comparators
//! names a pre-release of the same `MAJOR.MINOR.PATCH`: a preview is never taken unasked.

use std::cmp::Ordering;
use std::fmt;

/// A semantic version.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    /// The pre-release identifiers; empty for a release.
    pub pre: Vec<Ident>,
}

/// A pre-release identifier.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Ident {
    Number(u64),
    Text(String),
}

/// A number of a version: digits, no leading zero but the zero itself.
fn number(text: &str) -> Option<u64> {
    if text.is_empty()
        || !text.bytes().all(|b| b.is_ascii_digit())
        || (text.len() > 1 && text.starts_with('0'))
    {
        return None;
    }
    text.parse().ok()
}

fn identifiers(text: &str) -> Option<Vec<Ident>> {
    text.split('.')
        .map(|part| {
            if part.is_empty() || !part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
                None
            } else if part.bytes().all(|b| b.is_ascii_digit()) {
                number(part).map(Ident::Number)
            } else {
                Some(Ident::Text(part.to_owned()))
            }
        })
        .collect()
}

impl Version {
    pub fn new(major: u64, minor: u64, patch: u64) -> Version {
        Version {
            major,
            minor,
            patch,
            pre: Vec::new(),
        }
    }

    /// `1.2.3`, `1.2.3-beta.1`, `1.2.3+20261113`; none for anything else.
    pub fn parse(text: &str) -> Option<Version> {
        let text = text.trim();
        let (rest, build) = match text.split_once('+') {
            Some((rest, build)) => (rest, Some(build)),
            None => (text, None),
        };
        // Build metadata: identifiers that may start with a zero, not compared.
        if build.is_some_and(|build| {
            build.split('.').any(|part| {
                part.is_empty() || !part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            })
        }) {
            return None;
        }
        let (core, pre) = match rest.split_once('-') {
            Some((core, pre)) => (core, identifiers(pre)?),
            None => (rest, Vec::new()),
        };
        let mut parts = core.split('.');
        let version = Version {
            major: number(parts.next()?)?,
            minor: number(parts.next()?)?,
            patch: number(parts.next()?)?,
            pre,
        };
        parts.next().is_none().then_some(version)
    }

    /// A pre-release: the preview channel's.
    pub fn is_prerelease(&self) -> bool {
        !self.pre.is_empty()
    }

    fn core(&self) -> (u64, u64, u64) {
        (self.major, self.minor, self.patch)
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Version) -> Ordering {
        self.core().cmp(&other.core()).then_with(|| {
            match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => {
                    for (a, b) in self.pre.iter().zip(&other.pre) {
                        let order = match (a, b) {
                            (Ident::Number(a), Ident::Number(b)) => a.cmp(b),
                            (Ident::Number(_), Ident::Text(_)) => Ordering::Less,
                            (Ident::Text(_), Ident::Number(_)) => Ordering::Greater,
                            (Ident::Text(a), Ident::Text(b)) => a.cmp(b),
                        };
                        if order != Ordering::Equal {
                            return order;
                        }
                    }
                    self.pre.len().cmp(&other.pre.len())
                }
            }
        })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Version) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        for (i, ident) in self.pre.iter().enumerate() {
            f.write_str(if i == 0 { "-" } else { "." })?;
            match ident {
                Ident::Number(n) => write!(f, "{n}")?,
                Ident::Text(t) => f.write_str(t)?,
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Exact,
    Greater,
    GreaterEq,
    Less,
    LessEq,
    Caret,
    Tilde,
}

/// A comparator's version: the minor and patch may be left out (`>=1.2`, `^1`).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Partial {
    major: u64,
    minor: Option<u64>,
    patch: Option<u64>,
    pre: Vec<Ident>,
}

impl Partial {
    fn parse(text: &str) -> Option<Partial> {
        let text = text.trim();
        let (core, pre) = match text.split_once('-') {
            Some((core, pre)) => (core, identifiers(pre)?),
            None => (text, Vec::new()),
        };
        let mut parts = core.split('.');
        let major = number(parts.next()?)?;
        let minor = parts.next().map(number);
        let patch = parts.next().map(number);
        if parts.next().is_some() || minor == Some(None) || patch == Some(None) {
            return None;
        }
        let (minor, patch) = (minor.flatten(), patch.flatten());
        // A pre-release names a whole version.
        if !pre.is_empty() && patch.is_none() {
            return None;
        }
        Some(Partial {
            major,
            minor,
            patch,
            pre,
        })
    }

    fn version(&self) -> Version {
        Version {
            major: self.major,
            minor: self.minor.unwrap_or(0),
            patch: self.patch.unwrap_or(0),
            pre: self.pre.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Comparator {
    op: Op,
    version: Partial,
}

impl Comparator {
    /// The bounds a comparator with a partial version allows: from (inclusive) and below (exclusive), either open.
    /// `=1.2.3`, `>1.2.3` and `<=1.2.3` are plain comparisons, in [`Comparator::holds`].
    fn bounds(&self) -> (Option<Version>, Option<Version>) {
        let p = &self.version;
        let low = p.version();
        let next_major = Version::new(p.major + 1, 0, 0);
        let next_minor = |minor: u64| Version::new(p.major, minor + 1, 0);
        match self.op {
            // `=1.2` and `~1.2` are every 1.2.x, `=1` and `~1` every 1.x.y; `~1.2.3` is from 1.2.3 below 1.3.0.
            Op::Exact | Op::Tilde => match p.minor {
                Some(minor) => (Some(low), Some(next_minor(minor))),
                None => (Some(low), Some(next_major)),
            },
            Op::GreaterEq => (Some(low), None),
            // `>1.2` is from 1.3.0, `>1` from 2.0.0.
            Op::Greater => match p.minor {
                Some(minor) => (Some(next_minor(minor)), None),
                None => (Some(next_major), None),
            },
            Op::Less => (None, Some(low)),
            // `<=1.2` is below 1.3.0, `<=1` below 2.0.0.
            Op::LessEq => match p.minor {
                Some(minor) => (None, Some(next_minor(minor))),
                None => (None, Some(next_major)),
            },
            Op::Caret => {
                let upper = match (p.major, p.minor, p.patch) {
                    (0, Some(0), Some(patch)) => Version::new(0, 0, patch + 1),
                    (0, Some(minor), _) => Version::new(0, minor + 1, 0),
                    _ => next_major,
                };
                (Some(low), Some(upper))
            }
        }
    }

    fn holds(&self, v: &Version) -> bool {
        let p = &self.version;
        match (self.op, p.minor, p.patch) {
            (Op::Exact, Some(_), Some(_)) => *v == p.version(),
            (Op::Greater, Some(_), Some(_)) => *v > p.version(),
            (Op::LessEq, Some(_), Some(_)) => *v <= p.version(),
            _ => {
                let (low, high) = self.bounds();
                low.is_none_or(|low| *v >= low) && high.is_none_or(|high| *v < high)
            }
        }
    }
}

/// A range of versions: every comparator holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionReq {
    comparators: Vec<Comparator>,
}

impl VersionReq {
    /// `>=1.2, <2`, `^1.2`, `~1.2.3`, `=1.2.3`, `1.2` (a bare version is `^`); none for anything else.
    pub fn parse(text: &str) -> Option<VersionReq> {
        let comparators = text
            .split(',')
            .map(|part| {
                let part = part.trim();
                let (op, rest) = [
                    (">=", Op::GreaterEq),
                    ("<=", Op::LessEq),
                    (">", Op::Greater),
                    ("<", Op::Less),
                    ("=", Op::Exact),
                    ("^", Op::Caret),
                    ("~", Op::Tilde),
                ]
                .iter()
                .find_map(|(sign, op)| part.strip_prefix(sign).map(|rest| (*op, rest)))
                .unwrap_or((Op::Caret, part));
                Partial::parse(rest).map(|version| Comparator { op, version })
            })
            .collect::<Option<Vec<_>>>()?;
        (!comparators.is_empty()).then_some(VersionReq { comparators })
    }

    /// Whether every comparator holds for `v`, a pre-release too: what withdrawing versions needs, where a preview
    /// inside a bad range is bad as well (`<1.4.2` holds for `1.4.2-beta.1`).
    pub fn contains(&self, v: &Version) -> bool {
        self.comparators.iter().all(|c| c.holds(v))
    }

    /// Whether `v` is in the range. A pre-release only when a comparator names one of the same core version.
    pub fn matches(&self, v: &Version) -> bool {
        if v.is_prerelease()
            && !self.comparators.iter().any(|c| {
                !c.version.pre.is_empty()
                    && c.version.major == v.major
                    && c.version.minor == Some(v.minor)
                    && c.version.patch == Some(v.patch)
            })
        {
            return false;
        }
        self.comparators.iter().all(|c| c.holds(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).expect(text)
    }

    #[test]
    fn versions_read_and_order_as_semver_says() {
        assert_eq!(v("1.2.3").to_string(), "1.2.3");
        assert_eq!(v("1.2.3-beta.1+build.5").to_string(), "1.2.3-beta.1");
        for bad in [
            "1.2",
            "01.2.3",
            "1.2.3.4",
            "1.2.x",
            "",
            "1.2.3-",
            "1.2.3-be$ta",
            "v1.2.3",
        ] {
            assert!(Version::parse(bad).is_none(), "{bad}");
        }
        let ordered = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
            "1.0.1",
            "1.10.0",
            "2.0.0",
        ];
        for pair in ordered.windows(2) {
            assert!(v(pair[0]) < v(pair[1]), "{} < {}", pair[0], pair[1]);
        }
        assert!(v("1.2.3-beta.1").is_prerelease());
    }

    #[test]
    fn ranges_hold_as_cargo_reads_them() {
        let holds =
            |req: &str, version: &str| VersionReq::parse(req).expect(req).matches(&v(version));
        assert!(
            holds(">=1.2, <2", "1.2.0")
                && holds(">=1.2, <2", "1.9.9")
                && !holds(">=1.2, <2", "2.0.0")
        );
        assert!(!holds(">=1.2, <2", "1.1.9"));
        assert!(holds("^1.2", "1.7.0") && !holds("^1.2", "2.0.0") && !holds("^1.2", "1.1.0"));
        assert!(holds("^0.2.3", "0.2.9") && !holds("^0.2.3", "0.3.0"));
        assert!(holds("^0.0.3", "0.0.3") && !holds("^0.0.3", "0.0.4"));
        assert!(holds("~1.2.3", "1.2.9") && !holds("~1.2.3", "1.3.0"));
        assert!(holds("=1.2.3", "1.2.3") && !holds("=1.2.3", "1.2.4"));
        assert!(holds("=1.2", "1.2.7") && !holds("=1.2", "1.3.0"));
        assert!(holds(">1.2.3", "1.2.4") && !holds(">1.2.3", "1.2.3"));
        assert!(holds(">1.2", "1.3.0") && !holds(">1.2", "1.2.9"));
        assert!(holds("<=1.2", "1.2.9") && !holds("<=1.2", "1.3.0"));
        assert!(holds("1.2", "1.5.0"), "a bare version is a caret");
        // A preview only when asked for by name.
        assert!(!holds(">=1.2, <2", "1.5.0-beta.1"));
        assert!(holds(">=1.5.0-beta.1, <2", "1.5.0-beta.2"));
        assert!(!holds(">=1.5.0-beta.1, <2", "1.6.0-beta.1"));
        let below = VersionReq::parse("<1.4.2").unwrap();
        assert!(below.contains(&v("1.4.2-beta.1")) && !below.matches(&v("1.4.2-beta.1")));
        assert!(!below.contains(&v("1.4.2")));
        for bad in ["", ">=", "1.x", "^1.2.3.4", ">=1.2-beta", "=>1.2", "1.2, "] {
            assert!(VersionReq::parse(bad).is_none(), "{bad}");
        }
    }
}
