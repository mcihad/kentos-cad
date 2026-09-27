//! Expressions on 10⁵ and 10⁶ objects (docs/perf/README.md, docs/adr/0100):
//! the table the browser sends (`rows`), evaluated into a column, every
//! value read back, as a processing run or a layer's style asks; the column
//! engine against the tree evaluator it replaced (`reference`, the same
//! build), each on the same table. p50 and p95 of repeated runs after a
//! warm-up. Runs only on purpose:
//!   cargo test --release -p kentos-expression --test perf -- --ignored --nocapture
//! EXPRESSION_N sets the numbers of objects (comma separated), EXPRESSION_RUNS
//! the runs; with EXPRESSION_PERF_OUT (a directory) and EXPRESSION_PERF_LABEL
//! the table is also written to `<out>/expression-native-<label>-<date>.md`.

mod reference;

use std::fmt::Write as _;
use std::process::Command;
use std::time::Instant;

use kentos_expression::exec::Slot;
use kentos_expression::geometry::Shapes;
use kentos_expression::rows::{
    As, Column, EMPTY, Layout, MEASURE_STRIDE, RowsInput, Table, evaluate_rows,
};
use kentos_expression::{
    Builtin, FieldDef, FieldSource, FieldType, Geometry, Objects as Host, Schema, Value, compile,
    compile_with,
};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::store::draw::measure_record;
use kentos_geometry_core::vec2::Vec2;

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
    // rows[case][size] = ((old p50, old p95), (new p50, new p95))
    let mut rows = vec![Vec::new(); CASES.len()];
    for &n in &sizes {
        let o = objects(n);
        println!("\n{n} nesne, bütün değerler okunarak (tablo dahil değil)");
        for (k, (source, want)) in CASES.into_iter().enumerate() {
            let e = compile(source).unwrap_or_else(|e| panic!("{source}: {}", e.text()));
            let old =
                reference::compile(source).unwrap_or_else(|e| panic!("{source}: {}", e.text()));
            let (texts, lens) = table(&o, &e.fields);
            let input = || RowsInput {
                n,
                texts: &texts,
                text_lens: &lens,
                numbers: &[],
                measures: if e.needs.measured { &o.measures } else { &[] },
                scale: f64::NAN,
            };
            // The old path: its table, the tree walked per object.
            let old_rows =
                || reference::evaluate_rows(&old, &input(), want).unwrap_or_else(|m| panic!("{m}"));
            let new_rows = || evaluate_rows(&e, &input(), want).unwrap_or_else(|m| panic!("{m}"));
            let bits = |c: &Column| {
                (
                    c.kinds.clone(),
                    c.numbers.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                    c.texts.clone(),
                    c.text_lens.clone(),
                )
            };
            assert!(
                bits(&old_rows()) == bits(&new_rows()),
                "{source}: the engines disagree"
            );
            let once = |run_once: &dyn Fn() -> Column| {
                let t = Instant::now();
                let c = run_once();
                let filled = c.kinds.iter().filter(|&&k| k != EMPTY).count();
                let elapsed = t.elapsed().as_secs_f64() * 1000.0;
                assert_eq!(filled, n, "{source}");
                elapsed
            };
            // Old and new take turns, so the machine's other work falls on both alike.
            let (mut old_ms, mut new_ms) = (Vec::with_capacity(runs), Vec::with_capacity(runs));
            for run in 0..warm + runs {
                let (x, y) = (once(&old_rows), once(&new_rows));
                if run >= warm {
                    old_ms.push(x);
                    new_ms.push(y);
                }
            }
            let (a, b) = (quantiles(&mut old_ms), quantiles(&mut new_ms));
            println!(
                "{source:<40} eski p50 {:7.2} ms, yeni p50 {:7.2} ms (p95 {:.2} / {:.2}), {:.1}×",
                a.0,
                b.0,
                a.1,
                b.1,
                a.0 / b.0
            );
            rows[k].push((a, b));
        }
    }
    // One object at a time (`Expr::evaluate` through a scope, as the style
    // engine draws a symbol): the first size, each object's value read.
    let n = sizes[0];
    let o = objects(n);
    println!("\n{n} nesne, tek tek (Expr::evaluate)");
    let mut single = Vec::new();
    for (source, _) in CASES {
        let e = compile(source).unwrap_or_else(|e| panic!("{source}: {}", e.text()));
        let old = reference::compile(source).unwrap_or_else(|e| panic!("{source}: {}", e.text()));
        let (texts, lens) = table(&o, &e.fields);
        let input = || RowsInput {
            n,
            texts: &texts,
            text_lens: &lens,
            numbers: &[],
            measures: if e.needs.measured { &o.measures } else { &[] },
            scale: f64::NAN,
        };
        let new_table = Table::new(input(), Layout::new(e.fields.len(), e.needs))
            .unwrap_or_else(|m| panic!("{m}"));
        let old_table = reference::OldTable::new(
            input(),
            reference::OldLayout::new(old.fields.len(), old.needs),
        )
        .unwrap_or_else(|m| panic!("{m}"));
        let once = |each: &dyn Fn(usize) -> bool| {
            let t = Instant::now();
            let filled = (0..n).filter(|&i| each(i)).count();
            let elapsed = t.elapsed().as_secs_f64() * 1000.0;
            assert_eq!(filled, n, "{source}");
            elapsed
        };
        let (mut old_ms, mut new_ms) = (Vec::with_capacity(runs), Vec::with_capacity(runs));
        for run in 0..warm + runs {
            let x = once(&|i| old.evaluate(&old_table.row(i, None)) != Value::Null);
            let y = once(&|i| e.evaluate(&new_table.row(i, None)) != Value::Null);
            if run >= warm {
                old_ms.push(x);
                new_ms.push(y);
            }
        }
        let (a, b) = (quantiles(&mut old_ms), quantiles(&mut new_ms));
        println!(
            "{source:<40} eski p50 {:7.2} ms, yeni p50 {:7.2} ms, {:.1}×",
            a.0,
            b.0,
            a.0 / b.0
        );
        single.push((a, b));
    }
    let typed = typed_and_shapes(&sizes, runs, warm);
    let Ok(out) = std::env::var("EXPRESSION_PERF_OUT") else {
        return;
    };
    let label = std::env::var("EXPRESSION_PERF_LABEL").unwrap_or_else(|_| "olcum".into());
    let date = output("date", &["+%Y-%m-%d"]);
    let commit = output("git", &["rev-parse", "--short", "HEAD"]);
    let mut md = format!(
        "# İfade motoru, native: {label} ({date}, {commit})\n\n{}. {runs} koşu ({warm} ısınma), p50 / p95 ms.\n\n\
         Test `crates/shared/expression/tests/perf.rs`: web'in ölçümündeki nesneler (Parsel 1…n, her üçüncüsü Tarla ötekiler Arsa, 0…1000 m² alan) \
         tarayıcının gönderdiği tabloda (`rows`); ifade bir sütuna değerlendirilir, bütün değerler okunur. Tablonun kurulması dahil değildir. \
         “Eski” aynı yapıdaki ağaç değerlendiricisidir (`tests/reference`, 23cc8f7'nin kodu), “yeni” sütun motoru; ikisi aynı tabloda aynı sütunu verir.\n\n| İfade | Biçim |",
        machine()
    );
    for n in &sizes {
        let _ = write!(md, " {n} nesne: eski / yeni (p50) | kat |");
    }
    md.push_str("\n|---|---|");
    for _ in &sizes {
        md.push_str("---|---|");
    }
    md.push('\n');
    for (k, (source, want)) in CASES.iter().enumerate() {
        let _ = write!(md, "| `{}` | {want:?} |", source.replace('|', "\\|"));
        for ((a50, _), (b50, _)) in &rows[k] {
            let _ = write!(md, " {a50:.2} / {b50:.2} | {:.1}× |", a50 / b50);
        }
        md.push('\n');
    }
    let _ = write!(
        md,
        "\nTek tek (`Expr::evaluate`, bir kapsamdan; stil motorunun bir sembolü çizerken yaptığı gibi), {} nesne, p50 ms:\n\n| İfade | Eski | Yeni | Kat |\n|---|---|---|---|\n",
        sizes[0]
    );
    for ((source, _), ((a50, _), (b50, _))) in CASES.iter().zip(&single) {
        let _ = writeln!(
            md,
            "| `{}` | {a50:.2} | {b50:.2} | {:.1}× |",
            source.replace('|', "\\|"),
            a50 / b50
        );
    }
    md.push_str(
        "\nTipli alan ve şekillerden geometri (ADR 0100 §3; yeni motor, p50 ms). `Kat` bir kez sayı sütunu (kullanıcı alanı), bir kez metin \
         özniteliği; `$alan` bir kez şekillerden (`geometry::Shapes`, hesap dahil), bir kez deponun ölçü kaydıyla (kaydın hesabı dahil). \
         Nesneler 10 × 10 … 13 × 13 m kareler.\n\n| İfade | Yol |",
    );
    for n in &sizes {
        let _ = write!(md, " {n} nesne |");
    }
    md.push_str("\n|---|---|");
    for _ in &sizes {
        md.push_str("---|");
    }
    md.push('\n');
    for (label, path, times) in &typed {
        let _ = write!(md, "| `{}` | {path} |", label.replace('|', "\\|"));
        for t in times {
            let _ = write!(md, " {t:.2} |");
        }
        md.push('\n');
    }
    md.push_str("\np95 (eski / yeni):\n\n");
    for (k, (source, _)) in CASES.iter().enumerate() {
        let _ = write!(md, "- `{}`:", source.replace('|', "\\|"));
        for (n, ((_, a95), (_, b95))) in sizes.iter().zip(&rows[k]) {
            let _ = write!(md, " {n} nesnede {a95:.2} / {b95:.2} ms;");
        }
        md.push('\n');
    }
    let path = format!("{out}/expression-native-{label}-{date}.md");
    std::fs::write(&path, md).unwrap_or_else(|e| panic!("{path}: {e}"));
    println!("\n{path} yazıldı.");
}

/// A layer with a typed number field and squares (docs/adr/0100 §3).
struct Squares<'d> {
    kat: &'d [f64],
    kat_text: &'d [String],
    shapes: Shapes<'d, Box<dyn Fn(usize) -> Option<&'d Shape> + 'd>>,
}

impl<'d> Host<'d> for Squares<'d> {
    fn len(&self) -> usize {
        self.kat.len()
    }

    fn field(&self, _name: &str, ty: FieldType, start: usize, mut slot: Slot<'_, 'd>) {
        let (kat, text) = (self.kat, self.kat_text);
        match ty {
            FieldType::Number => slot.numbers(|i| (true, kat[start + i])),
            _ => {
                for i in 0..slot.len() {
                    slot.text(i, Some(text[start + i].as_str()));
                }
            }
        }
    }

    fn geometry(&self, what: Geometry, start: usize, slot: Slot<'_, 'd>) {
        self.shapes.fill(what, start, slot);
    }

    fn builtin(&self, _what: Builtin, _start: usize, mut slot: Slot<'_, 'd>) {
        for i in 0..slot.len() {
            slot.number(i, None);
        }
    }
}

/// Typed number field against text, geometry from shapes against the
/// measures record: (expression, path, p50 per size).
fn typed_and_shapes(sizes: &[usize], runs: usize, warm: usize) -> Vec<(String, String, Vec<f64>)> {
    let mut rows: Vec<(String, String, Vec<f64>)> = Vec::new();
    fn add(rows: &mut Vec<(String, String, Vec<f64>)>, label: &str, path: &str, t: f64) {
        match rows.iter_mut().find(|r| r.0 == label && r.1 == path) {
            Some(r) => r.2.push(t),
            None => rows.push((label.into(), path.into(), vec![t])),
        }
    }
    let schema = Schema {
        fields: vec![FieldDef {
            name: "Kat".into(),
            ty: FieldType::Number,
            source: FieldSource::User,
            description: String::new(),
        }],
    };
    for &n in sizes {
        let kat: Vec<f64> = (0..n).map(|i| (i % 12) as f64).collect();
        let kat_text: Vec<String> = kat.iter().map(|k| k.to_string()).collect();
        let shapes: Vec<Shape> = (0..n)
            .map(|i| {
                let (x, y, s) = (
                    487000.0 + (i % 1000) as f64 * 20.0,
                    4420000.0 + (i / 1000) as f64 * 20.0,
                    10.0 + (i % 4) as f64,
                );
                Shape::Polygon {
                    pts: vec![
                        Vec2::new(x, y),
                        Vec2::new(x + s, y),
                        Vec2::new(x + s, y + s),
                        Vec2::new(x, y + s),
                    ],
                    bulges: None,
                    holes: None,
                }
            })
            .collect();
        let sq = &shapes;
        let layer = || Squares {
            kat: &kat,
            kat_text: &kat_text,
            shapes: Shapes::new(Box::new(move |i| sq.get(i))),
        };
        let time = |f: &dyn Fn() -> usize| {
            let mut ms = Vec::with_capacity(runs);
            for run in 0..warm + runs {
                let t = Instant::now();
                let filled = f();
                let elapsed = t.elapsed().as_secs_f64() * 1000.0;
                assert_eq!(filled, n);
                if run >= warm {
                    ms.push(elapsed);
                }
            }
            quantiles(&mut ms).0
        };
        let filled = |c: &Column| c.kinds.iter().filter(|&&k| k != EMPTY).count();
        let typed = compile_with("Kat * 2 + 1", &schema).unwrap_or_else(|e| panic!("{}", e.text()));
        let text = compile("Kat * 2 + 1").unwrap_or_else(|e| panic!("{}", e.text()));
        let l = layer();
        add(
            &mut rows,
            "Kat * 2 + 1",
            "sayı alanı",
            time(&|| filled(&typed.evaluate_objects(&l, As::Number))),
        );
        add(
            &mut rows,
            "Kat * 2 + 1",
            "metin özniteliği",
            time(&|| filled(&text.evaluate_objects(&l, As::Number))),
        );
        for src in ["$alan > 500", "yuvarla($alan, 2)"] {
            let e = compile(src).unwrap_or_else(|e| panic!("{}", e.text()));
            add(
                &mut rows,
                src,
                "şekillerden",
                time(&|| filled(&e.evaluate_objects(&layer(), As::Value))),
            );
            // The web's path: the store's measures record, then the table.
            add(
                &mut rows,
                src,
                "ölçü kaydıyla",
                time(&|| {
                    let mut measures = Vec::with_capacity(n * MEASURE_STRIDE);
                    for s in &shapes {
                        measure_record(Some(s), &mut measures);
                    }
                    let input = RowsInput {
                        n,
                        texts: "",
                        text_lens: &[],
                        numbers: &[],
                        measures: &measures,
                        scale: f64::NAN,
                    };
                    filled(&evaluate_rows(&e, &input, As::Value).unwrap_or_else(|m| panic!("{m}")))
                }),
            );
        }
        for src in ["$merkez_y", "$genişlik * $yükseklik"] {
            let e = compile(src).unwrap_or_else(|e| panic!("{}", e.text()));
            add(
                &mut rows,
                src,
                "şekillerden",
                time(&|| filled(&e.evaluate_objects(&layer(), As::Value))),
            );
        }
        println!("\n{n} nesne, tipli alan ve şekillerden geometri (yeni motor)");
        for (label, path, t) in &rows {
            println!(
                "{label:<32} {path:<18} p50 {:7.2} ms",
                t.last().copied().unwrap_or(f64::NAN)
            );
        }
    }
    rows
}
