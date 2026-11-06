//! Veri çıkarımı's preview over a large drawing (docs/adr/1131 §3): 100 000 objects — 60 000 lines, 30 000 Sandalye
//! inserts and 10 000 dynamic Sıra rows each laying three Sandalye by its array action, a tenth of them in the state that
//! hides them — counted by block and kind with lengths, nested, the way the Veri çıkarımı window previews them. Says
//! the time (best and median of five), the process's resident memory before, at its highest while it runs (sampled
//! every 10 ms through `ps`) and after, how far the watch got, and how soon a stop asked 20 ms in answers.
//!
//! ```text
//! cargo test --release -p kentos-native-application --test all extract_timing -- --ignored --nocapture
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use kentos_commands::{CommandResult, DocumentSnapshotV2, ExtractRun};
use kentos_domain::Document;
use kentos_formats::extract::Watch;
use kentos_native_application::extract_run;
use serde_json::json;

const SANDALYE: &str = "0192f5a1-1131-7000-8000-00000000b001";
const SIRA: &str = "0192f5a1-1131-7000-8000-00000000b002";

/// The process's resident memory, MB, as `ps` says it.
fn resident_mb() -> f64 {
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output();
    out.ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<f64>().ok())
        .map_or(0.0, |kb| kb / 1024.0)
}

/// The drawing: `lines` lines, `chairs` Sandalye and `rows` Sıra (every tenth Boş), on one layer.
fn drawing(lines: usize, chairs: usize, rows: usize) -> Document {
    let mut entities = Vec::with_capacity(lines + chairs + rows);
    let mut uids = Vec::new();
    let mut id = 0u64;
    let mut next = |e: serde_json::Value, entities: &mut Vec<serde_json::Value>| {
        id += 1;
        let mut e = e;
        e["id"] = json!(id);
        e["layerId"] = json!("a");
        e["attrs"] = json!({});
        entities.push(e);
        uids.push(format!("0192f5a1-1131-7000-8100-{id:012x}"));
    };
    for k in 0..lines {
        let y = (k / 1000) as f64 * 2.0;
        let x = (k % 1000) as f64 * 3.0;
        next(json!({"kind":"line","a":{"x":x,"y":y},"b":{"x":x + 2.5,"y":y}}), &mut entities);
    }
    for k in 0..chairs {
        let (x, y) = ((k % 500) as f64 * 1.0, 3000.0 + (k / 500) as f64);
        next(json!({"kind":"insert","block":SANDALYE,"p":{"x":x,"y":y},"scale":1.0,"rotation":0.0}), &mut entities);
    }
    for k in 0..rows {
        let (x, y) = ((k % 200) as f64 * 4.0, 5000.0 + (k / 200) as f64 * 2.0);
        let mut params = json!({"Boy": 2.0});
        if k % 10 == 0 {
            params["Görünüm"] = json!("Boş");
        }
        next(
            json!({"kind":"insert","block":SIRA,"p":{"x":x,"y":y},"scale":1.0,"rotation":0.0,"parameters":params}),
            &mut entities,
        );
    }
    let rect = json!({"kind":"polyline","id":1,"layerId":"","attrs":{},
        "pts":[{"x":0.0,"y":0.0},{"x":0.5,"y":0.0},{"x":0.5,"y":0.5},{"x":0.0,"y":0.5}],"closed":true});
    let snapshot = json!({"format":"kentos.document","version":2,"name":"Büyük","uids":uids,
        "settings":{"srid":0,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100.0,
                    "workspace":"cad","drawingUnit":"m"},
        "origin":{"x":0.0,"y":0.0},
        "layers":[{"id":"a","name":"Genel","type":"layer","visible":true,"locked":false,"expanded":true,
                   "style":{"color":"ink","lineType":"continuous","lineWeight":0.25},"children":[]}],
        "activeLayer":"a","styles":{"items":[],"categories":[]},
        "blocks":[
            {"id":SANDALYE,"name":"Sandalye","base":{"x":0.0,"y":0.0},"units":"m","entities":[rect]},
            {"id":SIRA,"name":"Sıra","base":{"x":0.0,"y":0.0},"units":"m",
             "entities":[{"kind":"line","id":1,"layerId":"","attrs":{},"a":{"x":0.0,"y":0.0},"b":{"x":1.0,"y":0.0}},
                         {"kind":"insert","id":2,"layerId":"","attrs":{},"block":SANDALYE,"p":{"x":0.0,"y":0.5},
                          "scale":1.0,"rotation":0.0}],
             "dynamic":{"parameters":[{"kind":"linear","name":"Boy","from":{"x":0.0,"y":0.0},"to":{"x":1.0,"y":0.0}},
                                      {"kind":"visibility","name":"Görünüm","states":[{"name":"Dolu"},{"name":"Boş","hidden":[2]}]}],
                        "actions":[{"kind":"array","parameter":"Boy","pieces":[2],"spacing":0.5}]}}],
        "entities":entities});
    let snapshot: DocumentSnapshotV2 = serde_json::from_value(snapshot).expect("reads");
    Document::from_snapshot_v2(snapshot).expect("opens")
}

fn input() -> ExtractRun {
    serde_json::from_value(json!({"recipe":{
        "name":"Büyük çizim","kinds":["insert","line"],"scope":{"nested":true},
        "columns":[{"label":"Blok","value":{"of":"block"}},{"label":"Tür","value":{"of":"kind"}},
                   {"label":"Adet","value":{"of":"count"}},{"label":"Uzunluk","value":{"of":"length","unit":"m"}}],
        "group":true,"total":true}}))
    .expect("an input")
}

#[test]
#[ignore = "a timing, run on purpose"]
fn extract_timing() {
    let doc = drawing(60_000, 30_000, 10_000);
    assert_eq!(doc.len(), 100_000);
    let input = input();
    let before = resident_mb();
    // The highest resident memory while the extractions run, sampled every 10 ms.
    let running = Arc::new(AtomicBool::new(true));
    let peak = Arc::new(AtomicU64::new(before.to_bits()));
    let sampler = {
        let (running, peak) = (running.clone(), peak.clone());
        std::thread::spawn(move || {
            while running.load(Ordering::Relaxed) {
                let now = resident_mb();
                if now > f64::from_bits(peak.load(Ordering::Relaxed)) {
                    peak.store(now.to_bits(), Ordering::Relaxed);
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        })
    };
    let mut times = Vec::new();
    let mut last = None;
    for _ in 0..5 {
        let watch = Watch::default();
        let t = Instant::now();
        let answer = extract_run::plan_watched(&doc, &input, &watch);
        times.push(t.elapsed().as_secs_f64() * 1000.0);
        assert_eq!(watch.done.load(Ordering::Relaxed), watch.total.load(Ordering::Relaxed));
        last = Some(answer);
    }
    running.store(false, Ordering::Relaxed);
    sampler.join().expect("the sampler");
    let after = resident_mb();
    let CommandResult::Completed { output, warnings } = last.expect("ran") else {
        panic!("the extraction completes")
    };
    // 60 000 lines; 30 000 Sandalye and 9 000 × 3 shown in rows; 10 000 Sıra and their 10 000 frame lines; the Boş
    // rows' 3 000 Sandalye hidden.
    assert_eq!(output.occurrences, 60_000 + 30_000 + 27_000 + 10_000 + 10_000);
    assert_eq!(output.notes.hidden_states, 3_000, "{warnings:?}");
    times.sort_by(f64::total_cmp);
    // A stop asked 20 ms in: how soon the answer comes, and how far the watch got.
    let watch = Arc::new(Watch::default());
    let (w2, doc2, input2) = (watch.clone(), doc.reading_copy(), input.clone());
    let t = Instant::now();
    let worker = std::thread::spawn(move || extract_run::plan_watched(&doc2, &input2, &w2));
    std::thread::sleep(Duration::from_millis(20));
    let asked = Instant::now();
    watch.stop.store(true, Ordering::Relaxed);
    let stopped = worker.join().expect("the worker");
    let answered = asked.elapsed().as_secs_f64() * 1000.0;
    let whole = t.elapsed().as_secs_f64() * 1000.0;
    assert!(matches!(stopped, CommandResult::Cancelled), "{stopped:?}");
    println!("Veri çıkarımı önizlemesi, 100 000 nesne (60 000 çizgi, 30 000 blok, 10 000 dinamik sıra):");
    println!("  süre en iyi {:.1} ms, ortanca {:.1} ms (5 kez)", times[0], times[2]);
    println!(
        "  RSS önce {before:.0} MB, en yüksek {:.0} MB, sonra {after:.0} MB",
        f64::from_bits(peak.load(Ordering::Relaxed))
    );
    println!(
        "  durdurma: 20 ms'de istendi, {answered:.1} ms'de cevap (toplam {whole:.1} ms); okunan {} / {}",
        watch.done.load(Ordering::Relaxed),
        watch.total.load(Ordering::Relaxed)
    );
    println!("  satır {}, oluşum {}, gizli {} (görünürlük {})", output.rows.len(), output.occurrences, output.notes.hidden, output.notes.hidden_states);
}

/// The watch, small: a stop asked before the walk answers `Cancelled` with nothing made; without a stop the walk goes
/// over every top-level object once (docs/adr/1131 §3).
#[test]
fn a_stopped_extraction_is_cancelled_and_a_whole_one_walks_every_object() {
    let doc = drawing(600, 300, 100);
    let input = input();
    let watch = Watch::default();
    watch.stop.store(true, Ordering::Relaxed);
    assert!(matches!(extract_run::plan_watched(&doc, &input, &watch), CommandResult::Cancelled));
    assert_eq!(watch.done.load(Ordering::Relaxed), 1, "stopped at the first look");
    let watch = Watch::default();
    let CommandResult::Completed { output, .. } = extract_run::plan_watched(&doc, &input, &watch) else {
        panic!("completes")
    };
    assert_eq!(watch.done.load(Ordering::Relaxed), 1000);
    assert_eq!(watch.total.load(Ordering::Relaxed), 1000);
    assert_eq!(output.occurrences, 600 + 300 + 90 * 3 + 100 + 100);
    assert_eq!(output.notes.hidden_states, 30);
}
