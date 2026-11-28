//! Samples downloaded for a measurement (docs/adr/8061 §11.7), kept out of Git: every FILE.ifc under the folder
//! `KENTOS_IFC_ORNEKLER` names, each with the other reader's FILE.okuma.json beside it
//! (`scripts/fixtures/ifc_read_cases.py --files FOLDER`), read as the other reader reads it: the same graph, the same
//! own sets, every body it makes made alike. A line a file.
//!
//! ```text
//! KENTOS_IFC_ORNEKLER=.run/ifc-ornekler/<commit> cargo test -p kentos-ifc --features geometry \
//!   read::outside_tests -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};

use super::bodies_tests::body_check;
use super::reference_tests::{graph_differences, set_differences};
use super::tests::never;
use super::{Limits, read};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            files(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ifc")) {
            out.push(p);
        }
    }
}

#[test]
#[ignore = "samples downloaded for a measurement, by hand"]
fn downloaded_samples_read_as_the_other_reader_reads_them() {
    let dir = std::env::var("KENTOS_IFC_ORNEKLER")
        .expect("KENTOS_IFC_ORNEKLER: the downloaded samples' folder");
    let mut all = Vec::new();
    files(Path::new(&dir), &mut all);
    all.sort();
    assert!(!all.is_empty(), "no IFC file under {dir}");
    let mut differences = Vec::new();
    for f in &all {
        let name = f.strip_prefix(&dir).unwrap_or(f).display().to_string();
        let reference: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(f.with_extension("okuma.json"))
                .expect("the other reader's reading beside it"),
        )
        .expect("it reads");
        let bytes = std::fs::read(f).expect("the file");
        let m = match read(&bytes, &Limits::default(), &never) {
            Ok(m) => m,
            Err(e) => {
                differences.push(format!("{name}: {e}"));
                continue;
            }
        };
        let graph = graph_differences(&m, &reference);
        let (sets, set_diffs) = set_differences(&m, &bytes, &reference);
        let b = body_check(&m, &bytes, &reference);
        println!(
            "{name}: {} {}; {} objects, {} findings; graph {} differences; {sets} objects' sets, {} differences; {} bodies (the other reader {}), {} not shown, worst box {:.2e} m, worst volume {:.2e}, {} unexpected",
            m.summary.schema,
            m.summary
                .header
                .originating_system
                .as_deref()
                .unwrap_or("?"),
            m.objects.len(),
            m.findings.len(),
            graph.len(),
            set_diffs.len(),
            b.made,
            b.theirs,
            b.not_shown,
            b.worst_box,
            b.worst_volume,
            b.unexpected.len()
        );
        differences.extend(
            graph
                .into_iter()
                .chain(set_diffs)
                .chain(b.unexpected)
                .map(|d| format!("{name}: {d}")),
        );
    }
    assert!(
        differences.is_empty(),
        "{} differences:\n{}",
        differences.len(),
        differences.join("\n")
    );
}
