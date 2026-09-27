//! Expressions on 10⁵ and 10⁶ objects (docs/perf/README.md, docs/adr/0100):
//! the table the browser sends (`rows`), evaluated into a column, every
//! value read back, as a processing run or a layer's style asks. p50 and p95
//! of repeated runs after a warm-up. Runs only on purpose:
//!   cargo test --release -p kentos-expression --test perf -- --ignored --nocapture
//! EXPRESSION_N sets the numbers of objects (comma separated), EXPRESSION_RUNS
//! the runs; with EXPRESSION_PERF_OUT (a directory) and EXPRESSION_PERF_LABEL
//! the table is also written to `<out>/expression-native-<label>-<date>.md`.

use std::fmt::Write as _;
use std::process::Command;
use std::time::Instant;

use kentos_expression::compile;
use kentos_expression::rows::{As, EMPTY, MEASURE_STRIDE, RowsInput, evaluate_rows};

/// A condition, a numbering, an area as text, a rounded number, a geometry condition, arithmetic on an attribute.
const CASES: [(&str, As); 6] = [
    ("Nitelik = 'Arsa' ve $alan > 500", As::Bool),
    ("'P' || doldur($sıra, 5)", As::Text),
    ("metin($alan, 2) || ' m²'", As::Text),
    ("yuvarla($alan, 2)", As::Number),
    ("$alan / 10000 > 0.05 ve $uzunluk < 400", As::Bool),
    ("Parsel * 2 + 1", As::Number),
];

/// The objects as the web's benchmark makes them (apps/web/scripts/perf/expression.test.ts):
/// Parsel 1…n, Nitelik Tarla for every third and Arsa otherwise, an area of 0…1000 m².
struct Objects {
    n: usize,
    parsel: Vec<String>,
    nitelik: Vec<&'static str>,
    measures: Vec<f64>,
}

fn objects(n: usize) -> Objects {
    let mut measures = vec![0.0; n * MEASURE_STRIDE];
    for i in 0..n {
        measures[i * MEASURE_STRIDE] = 7.0;
        measures[i * MEASURE_STRIDE + 2] = (i as f64 * 2.22) % 1000.0;
    }
    Objects {
        n,
        parsel: (1..=n).map(|i| i.to_string()).collect(),
        nitelik: (0..n)
            .map(|i| if i % 3 == 0 { "Tarla" } else { "Arsa" })
            .collect(),
        measures,
    }
}

/// The table for an expression's fields (text slots in its order), as the browser builds it.
fn table(o: &Objects, fields: &[String]) -> (String, Vec<i32>) {
    let mut texts = String::new();
    let mut lens = Vec::with_capacity(o.n * fields.len());
    for i in 0..o.n {
        for f in fields {
            let v = match f.as_str() {
                "Parsel" => o.parsel[i].as_str(),
                "Nitelik" => o.nitelik[i],
                _ => {
                    lens.push(-1);
                    continue;
                }
            };
            texts.push_str(v);
            lens.push(v.encode_utf16().count() as i32);
        }
    }
    (texts, lens)
}

/// p50 and p95 in milliseconds.
fn quantiles(ms: &mut [f64]) -> (f64, f64) {
    ms.sort_by(f64::total_cmp);
    let q = |p: f64| ms[((p * ms.len() as f64) as usize).min(ms.len() - 1)];
    (q(0.5), q(0.95))
}

fn env_list(name: &str, default: &[usize]) -> Vec<usize> {
    std::env::var(name)
        .ok()
        .map(|s| s.split(',').filter_map(|x| x.trim().parse().ok()).collect())
        .unwrap_or_else(|| default.to_vec())
}

/// A command's first line of output, or "?".
fn output(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.lines().next().map(str::to_string))
        .unwrap_or_else(|| "?".into())
}

/// The machine, as docs/perf's other files name it.
fn machine() -> String {
    let cpu = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("model name"))
                .and_then(|l| l.split(':').nth(1))
                .map(|m| m.trim().to_string())
        })
        .unwrap_or_else(|| "?".into());
    let threads = std::thread::available_parallelism().map_or(0, |n| n.get());
    let os = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("PRETTY_NAME="))
                .map(|l| l["PRETTY_NAME=".len()..].trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "?".into());
    format!(
        "{cpu}, {threads} iş parçacığı; {os} ({}); {}, `--release`",
        output("uname", &["-r"]),
        output("rustc", &["-V"])
    )
}

#[test]
#[ignore = "a measurement: cargo test --release -p kentos-expression --test perf -- --ignored --nocapture"]
fn measures_expressions_on_many_objects() {
    let runs = env_list("EXPRESSION_RUNS", &[20])[0];
    let warm = 3;
    let sizes = env_list("EXPRESSION_N", &[100_000, 1_000_000]);
    // rows[case][size] = (p50, p95)
    let mut rows = vec![Vec::new(); CASES.len()];
    for &n in &sizes {
        let o = objects(n);
        println!("\n{n} nesne, bütün değerler okunarak (tablo dahil değil)");
        for (k, (source, want)) in CASES.into_iter().enumerate() {
            let e = compile(source).unwrap_or_else(|e| panic!("{source}: {}", e.text()));
            let (texts, lens) = table(&o, &e.fields);
            let input = RowsInput {
                n,
                texts: &texts,
                text_lens: &lens,
                numbers: &[],
                measures: if e.needs.measured { &o.measures } else { &[] },
                scale: f64::NAN,
            };
            let mut ms = Vec::with_capacity(runs);
            for run in 0..warm + runs {
                let t = Instant::now();
                let c = evaluate_rows(&e, &input, want).unwrap_or_else(|m| panic!("{m}"));
                let filled = c.kinds.iter().filter(|&&k| k != EMPTY).count();
                let elapsed = t.elapsed().as_secs_f64() * 1000.0;
                assert_eq!(filled, n, "{source}");
                if run >= warm {
                    ms.push(elapsed);
                }
            }
            let (p50, p95) = quantiles(&mut ms);
            println!("{source:<40} p50 {p50:7.2} ms, p95 {p95:7.2} ms ({runs} koşu)");
            rows[k].push((p50, p95));
        }
    }
    let Ok(out) = std::env::var("EXPRESSION_PERF_OUT") else {
        return;
    };
    let label = std::env::var("EXPRESSION_PERF_LABEL").unwrap_or_else(|_| "olcum".into());
    let date = output("date", &["+%Y-%m-%d"]);
    let commit = output("git", &["rev-parse", "--short", "HEAD"]);
    let mut md = format!(
        "# İfade motoru, native: {label} ({date}, {commit})\n\n{}. {runs} koşu ({warm} ısınma), p50 / p95 ms.\n\n\
         Test `crates/shared/expression/tests/perf.rs`: web'in ölçümündeki nesneler (Parsel 1…n, her üçüncüsü Tarla ötekiler Arsa, 0…1000 m² alan) \
         tarayıcının gönderdiği tabloda (`rows`); ifade bir sütuna değerlendirilir, bütün değerler okunur. Tablonun kurulması dahil değildir.\n\n| İfade | Biçim |",
        machine()
    );
    for n in &sizes {
        let _ = write!(md, " {n} nesne |");
    }
    md.push_str("\n|---|---|");
    for _ in &sizes {
        md.push_str("---|");
    }
    md.push('\n');
    for (k, (source, want)) in CASES.iter().enumerate() {
        let _ = write!(md, "| `{}` | {want:?} |", source.replace('|', "\\|"));
        for (p50, p95) in &rows[k] {
            let _ = write!(md, " {p50:.2} / {p95:.2} |");
        }
        md.push('\n');
    }
    let path = format!("{out}/expression-native-{label}-{date}.md");
    std::fs::write(&path, md).unwrap_or_else(|e| panic!("{path}: {e}"));
    println!("\n{path} yazıldı.");
}
