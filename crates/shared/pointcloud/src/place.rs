//! What Nokta bulutu ekle decides from the files' headers (docs/adr/0207
//! §2, §5, §8): the rule of the systems (a raster's, ADR 0204 §1, for every
//! member of a virtual cloud together) and where a new cloud's look is
//! sampled from.

use crate::source::{Cloud, Kind, Run};

/// The points a new cloud's look is chosen from, about.
pub const SAMPLE_POINTS: u64 = 50_000;
/// The most runs of a file a sample reads.
pub const SAMPLE_RUNS: usize = 8;

/// The rule of the systems for a cloud's files in a project.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// Every file's system is the project's: the cloud is added.
    Same { srid: u32 },
    /// No file names a system: added once the user says it is the project's (`srid` 0 then).
    Unknown,
    /// A file's system is another: refused.
    Other { srid: u32 },
    /// The files name different systems (or some none): refused.
    Mixed,
}

/// The rule for files whose systems are `epsgs` (none: unnamed) in a project of `project` (0: none).
pub fn rule(epsgs: &[Option<u32>], project: u32) -> Rule {
    let Some(first) = epsgs.first() else {
        return Rule::Unknown;
    };
    if epsgs.iter().any(|e| e != first) {
        return Rule::Mixed;
    }
    match *first {
        Some(srid) if srid == project => Rule::Same { srid },
        Some(srid) => Rule::Other { srid },
        None => Rule::Unknown,
    }
}

/// The cloud's system once the rule allows it: the files', or 0 when the user
/// confirmed the unnamed one is the project's; none when it is refused.
pub fn srid_of(rule: Rule, confirmed: bool) -> Option<u32> {
    match rule {
        Rule::Same { srid } => Some(srid),
        Rule::Unknown if confirmed => Some(0),
        Rule::Unknown | Rule::Other { .. } | Rule::Mixed => None,
    }
}

/// The runs a new cloud's sample is read from: a COPC's root node, else at
/// most eight runs spread evenly over the file (the first and the last among them).
pub fn sample_runs(cloud: &Cloud) -> Vec<Run> {
    if cloud.kind == Kind::Copc {
        return cloud.node_run(crate::copc::Key::ROOT).into_iter().collect();
    }
    let runs = cloud.runs();
    let n = runs.len();
    if n <= SAMPLE_RUNS {
        return runs;
    }
    (0..SAMPLE_RUNS)
        .map(|k| runs[k * (n - 1) / (SAMPLE_RUNS - 1)])
        .collect()
}

/// Every how many records the sample takes one, so that about [`SAMPLE_POINTS`] come of `runs`.
pub fn sample_step(runs: &[Run]) -> usize {
    let total: u64 = runs.iter().map(|r| r.count).sum();
    usize::try_from(total.div_ceil(SAMPLE_POINTS).max(1)).unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rule_of_the_systems() {
        assert_eq!(
            rule(&[Some(5254), Some(5254)], 5254),
            Rule::Same { srid: 5254 }
        );
        assert_eq!(rule(&[Some(5255)], 5254), Rule::Other { srid: 5255 });
        assert_eq!(rule(&[None, None], 5254), Rule::Unknown);
        assert_eq!(rule(&[Some(5254), None], 5254), Rule::Mixed);
        assert_eq!(rule(&[Some(5254), Some(5255)], 5254), Rule::Mixed);
        assert_eq!(srid_of(Rule::Unknown, true), Some(0));
        assert_eq!(srid_of(Rule::Unknown, false), None);
        assert_eq!(srid_of(Rule::Mixed, true), None);
    }

    #[test]
    fn the_sample_step() {
        let run = |count| Run {
            need: kentos_formats::raster::Need { offset: 0, len: 0 },
            count,
            first: 0,
            node: None,
        };
        assert_eq!(sample_step(&[run(10_000)]), 1);
        assert_eq!(sample_step(&[run(65_536), run(65_536)]), 3);
    }
}
