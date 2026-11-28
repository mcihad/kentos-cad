//! The building's IFC file and the model quality control form's facts at 1, 20 and 100 apartments (plan/bim-plan.md
//! §7's baseline, docs/adr/8074 §6): the 2+1 apartment of the acceptance flow copied into rows of ten (`flat_timing`),
//! its model taken as `cad.arch.ifc` takes it, written, read back with its bodies and its facts (what the Mimari audit
//! hands the standards package), and the package's form answered on them; time and the resident memory's peak. Beside
//! them what 3B görünüm makes on its own thread after the model and its text (docs/adr/8119): the file read again with
//! its bodies and its openings' volumes. Not run by default:
//!
//! ```text
//! cargo test --release -p kentos-headless --test headless ifc_timing -- --ignored --nocapture
//! ```
//!
//! `KENTOS_PERF_FLATS` (comma separated) changes the counts; `KENTOS_PERF_KEEP` keeps each count's facts in the temporary
//! folder.

use std::time::Instant;

use super::flat_timing::{copied_with, drawn, peak};

#[test]
#[ignore = "a measurement, run by hand in release mode"]
fn ifc_timing() {
    let counts: Vec<usize> = std::env::var("KENTOS_PERF_FLATS")
        .ok()
        .map(|v| v.split(',').filter_map(|n| n.trim().parse().ok()).collect())
        .unwrap_or_else(|| vec![1, 20, 100]);
    let package = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../catalog/standards/dijital-proje/src");
    println!(
        "daire | nesne | duvar | mahal | okuma kopyası ms | kesit modeli ms | model ms | yazma ms | MB dosya | \
         okuma+gövde+olgu ms | tepe MB | olgu MB | form ms | 3B okuma+gövde ms | 3B gövde"
    );
    for n in counts {
        let (mut s, _) = drawn();
        copied_with(&mut s, n, false);
        let doc = s.document();
        let objects = doc.entities().count();
        // What the window's thread pays when Denetim asks (docs/adr/8115): a reading copy of the drawing.
        let t = Instant::now();
        let copy = doc.reading_copy();
        let copied = t.elapsed().as_secs_f64() * 1e3;
        drop(copy);
        // The section model alone: the model's mapping takes it first.
        let t = Instant::now();
        let _ = kentos_native_application::sections::model(doc);
        let section = t.elapsed().as_secs_f64() * 1e3;
        let t = Instant::now();
        let (b, written) = kentos_native_application::arch_ifc::building(
            doc,
            "2026-11-16T12:00:00",
            "KentOS Mimari",
        );
        let model = t.elapsed().as_secs_f64() * 1e3;
        let t = Instant::now();
        let text = kentos_ifc::building::write(&b);
        let write = t.elapsed().as_secs_f64() * 1e3;
        let limits = kentos_ifc::read::Limits::default();
        let t = Instant::now();
        let (facts, top) =
            peak(|| kentos_ifc::read::file_facts(text.as_bytes(), "bina.ifc", &limits, &|| false));
        let read = t.elapsed().as_secs_f64() * 1e3;
        let facts = facts.expect("the file reads back");
        // 3B görünüm's reading (docs/adr/8119): the model, its bodies and its openings' volumes.
        let t = Instant::now();
        let m = kentos_ifc::read::read(text.as_bytes(), &limits, &|| false)
            .expect("the file reads back");
        let shown = m
            .bodies(text.as_bytes(), &|| false)
            .expect("its bodies")
            .bodies
            .len()
            + m.opening_bodies(text.as_bytes(), &|| false)
                .expect("its openings")
                .bodies
                .len();
        let view3d = t.elapsed().as_secs_f64() * 1e3;
        let json = serde_json::to_string(&facts).expect("the facts as text");
        let path = std::env::temp_dir().join(format!(
            "kentos-ifc-olgular-{}-{n}.json",
            std::process::id()
        ));
        std::fs::write(&path, &json).expect("the facts kept");
        let t = Instant::now();
        let answered = std::process::Command::new("python3")
            .args(["-I", "-B", "-c", "import json, sys; sys.path.insert(0, sys.argv[1]); from dijital_proje import form; \
                    r = form.cevapla(json.load(open(sys.argv[2], encoding='utf-8'))); print(len(r['satirlar']))"])
            .arg(&package)
            .arg(&path)
            .output()
            .expect("python3 runs");
        let form = t.elapsed().as_secs_f64() * 1e3;
        // `KENTOS_PERF_KEEP` keeps the facts for a profile of the form.
        if std::env::var_os("KENTOS_PERF_KEEP").is_none() {
            let _ = std::fs::remove_file(&path);
        } else {
            println!("{}", path.display());
        }
        assert!(
            answered.status.success(),
            "{}",
            String::from_utf8_lossy(&answered.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&answered.stdout).trim(), "21");
        println!(
            "{n} | {objects} | {} | {} | {copied:.2} | {section:.1} | {model:.1} | {write:.1} | {:.2} | {read:.1} | {top:.0} | {:.2} | \
             {form:.0} | {view3d:.1} | {shown}",
            written.walls,
            written.spaces,
            text.len() as f64 / 1e6,
            json.len() as f64 / 1e6
        );
    }
}
