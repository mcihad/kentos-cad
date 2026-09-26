//! Saving and opening a large drawing on the desktop, measured (TODOS.md
//! FILE-24; docs/adr/0025, 0030): the app's own save (`document::write`: the
//! verified KCAD v2 bytes through a temporary file, flushed and read back)
//! and open (`Document::read`: the file read, decoded and made the
//! document), for drawings of parcels with 20 vertices, three attributes and
//! a label (`crates/shared/kcad/tests/measure.rs`), up to about 100 MB.
//!
//! Every operation runs in a process of its own, so its peak memory is its
//! own: the test starts this test binary again for each (`KENTOS_PERF_CHILD`)
//! and reads one line of JSON back. The files go to the worktree's `.run/`
//! (never committed, never the user's folders). Not a correctness test and
//! not run by default:
//!
//! ```text
//! KENTOS_PERF_LABEL=before KENTOS_PERF_OUT=docs/perf \
//!   cargo test --release -p kentos-desktop perf::kcad -- --ignored --nocapture --test-threads=1
//! ```
//!
//! `KENTOS_PERF_SIZES` (parcels, comma separated) and `KENTOS_PERF_RUNS`
//! change what is measured; `KENTOS_PERF_OUT` writes
//! `kcad-desktop-<label>-<date>.{json,md}` there instead of printing.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use kentos_contracts::{
    AngleUnit, AreaUnit, DocumentSnapshotV2, Entity, EntityBase, EntityId, LayerNode,
    LayerNodeType, LayerStyle, LineType, PathEntity, ProjectSettings, ProjectStyles, Vec2,
};
use kentos_render_wgpu::RenderSettings;
use kentos_render_wgpu::scene::{self, lod};
use kentos_ui::theme::Mode;
use serde_json::{Value, json};

use crate::document::{self, Document};

/// The drawing of measure.rs: `n` parcels of 20 vertices on one layer.
fn drawing(n: usize) -> DocumentSnapshotV2 {
    let mut entities = Vec::with_capacity(n);
    let mut uids = Vec::with_capacity(n);
    for i in 0..n {
        let (x0, y0) = (
            486_000.0 + (i % 300) as f64 * 31.7,
            4_420_000.0 + (i / 300) as f64 * 27.3,
        );
        let pts = (0..20)
            .map(|k| {
                let a = k as f64 / 20.0 * std::f64::consts::TAU;
                Vec2 {
                    x: x0 + 12.5 + 11.0 * a.cos(),
                    y: y0 + 12.5 + 9.0 * a.sin(),
                }
            })
            .collect();
        let attrs = BTreeMap::from([
            ("Ada".to_owned(), format!("{}", 100 + i / 50)),
            ("Parsel".to_owned(), format!("{}", i % 50 + 1)),
            ("Nitelik".to_owned(), "Arsa".to_owned()),
        ]);
        entities.push(Entity::Polygon(PathEntity {
            base: EntityBase {
                id: i as u32 + 1,
                layer_id: "parsel".into(),
                color: None,
                attrs,
                label: Some(format!("{}/{}", 100 + i / 50, i % 50 + 1)),
                symbol: None,
            },
            pts,
            bulges: None,
            holes: None,
        }));
        let mut id = [
            0x01, 0x92, 0xf5, 0xa0, 0x7c, 0x3e, 0x70, 0x00, 0x80, 0, 0, 0, 0, 0, 0, 0,
        ];
        id[10..].copy_from_slice(&(i as u64 + 1).to_be_bytes()[2..]);
        uids.push(EntityId(id));
    }
    DocumentSnapshotV2 {
        format: "kentos.document".into(),
        version: 2,
        name: format!("olcum-{n}"),
        settings: ProjectSettings {
            srid: 5256,
            length_decimals: 3,
            area_decimals: 2,
            area_unit: AreaUnit::M2,
            angle_unit: AngleUnit::Grad,
            plot_scale: 1000.0,
            workspace: None,
            drawing_font: None,
        },
        origin: Vec2 {
            x: 486_000.0,
            y: 4_420_000.0,
        },
        home_view: None,
        layers: vec![LayerNode {
            id: "parsel".into(),
            name: "Parsel".into(),
            kind: LayerNodeType::Layer,
            visible: true,
            locked: false,
            expanded: true,
            style: LayerStyle {
                color: "#E06C75".into(),
                line_type: LineType::Continuous,
                line_weight: 0.35,
                fill: None,
                point: None,
                label: None,
                pick_interior: None,
                renderer: None,
            },
            children: Vec::new(),
        }],
        active_layer: "parsel".into(),
        entities,
        uids,
        styles: ProjectStyles::default(),
        project_id: None,
        migrated_from: None,
    }
}

/// Resident memory now and its peak since the last reset, in MB (Linux `/proc/self/status`).
fn memory() -> (f64, f64) {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let field = |name: &str| {
        status
            .lines()
            .find_map(|l| l.strip_prefix(name))
            .and_then(|v| v.trim().trim_end_matches("kB").trim().parse::<f64>().ok())
            .map_or(0.0, |kb| kb / 1024.0)
    };
    (field("VmRSS:"), field("VmHWM:"))
}

/// Starts the peak over from the memory in use now (Linux ≥ 4.0).
fn reset_peak() {
    let _ = std::fs::write("/proc/self/clear_refs", "5");
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

/// One operation in this process: `save:<n>:<file>` or `open:<n>:<file>`; prints one line of JSON.
#[test]
#[ignore = "run by perf::kcad in a process of its own"]
fn child() {
    let Ok(task) = std::env::var("KENTOS_PERF_CHILD") else {
        return;
    };
    let mut parts = task.splitn(3, ':');
    let (op, n, file) = (
        parts.next().unwrap_or_default(),
        parts
            .next()
            .and_then(|n| n.parse::<usize>().ok())
            .unwrap_or(0),
        PathBuf::from(parts.next().unwrap_or_default()),
    );
    let out = match op {
        "save" => {
            let doc = Document::from_v2(drawing(n), None).expect("the drawing opens");
            let (rss, _) = memory();
            reset_peak();
            // What the save takes from the open drawing on the UI thread, then writes in the background.
            let t = Instant::now();
            let snapshot = doc.model.to_snapshot_v2();
            let snapshot_ms = ms(t);
            let t = Instant::now();
            document::write(&snapshot, &file).expect("writes");
            let write_ms = ms(t);
            let (_, peak) = memory();
            json!({
                "snapshotMs": snapshot_ms,
                "writeMs": write_ms,
                "bytes": std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0),
                "rssBeforeMb": rss,
                "peakAboveMb": peak - rss,
            })
        }
        _ => {
            let (rss, _) = memory();
            reset_peak();
            let t = Instant::now();
            let doc = Document::read(&file).expect("reads");
            let read_ms = ms(t);
            let (_, peak) = memory();
            assert_eq!(doc.entity_count(), n);
            // The first frame's scene, built on the UI thread (viewport.rs `scene`): the drawing fitted to 1440 px.
            let palette = crate::viewport::palette(Mode::Dark);
            let settings = RenderSettings::new(palette.background);
            let t = Instant::now();
            let origin = scene::scene_origin(&doc);
            let fixed = scene::build_fixed(&doc, &palette, origin);
            let extents = scene::extents(&doc).expect("extents");
            let scale = 1440.0 / (extents.max_x - extents.min_x).max(1.0);
            let band = lod::build_band(scale, settings.curve_tolerance_px);
            let curves = scene::build_curves(
                &doc,
                &palette,
                origin,
                lod::tolerance(band),
                settings.curve_segment_budget,
            );
            let scene_ms = ms(t);
            drop((fixed, curves));
            json!({
                "readMs": read_ms,
                "sceneMs": scene_ms,
                "rssBeforeMb": rss,
                "peakAboveMb": peak - rss,
            })
        }
    };
    println!("KENTOS_PERF {out}");
}

/// Runs one operation in a fresh process of this test binary.
fn run_child(op: &str, n: usize, file: &Path) -> Value {
    let exe = std::env::current_exe().expect("the test binary");
    let output = Command::new(exe)
        .args(["perf::child", "--exact", "--ignored", "--nocapture"])
        .env("KENTOS_PERF_CHILD", format!("{op}:{n}:{}", file.display()))
        .output()
        .expect("runs");
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text
        .lines()
        .find_map(|l| l.strip_prefix("KENTOS_PERF "))
        .unwrap_or_else(|| {
            panic!(
                "{op} {n}: no result\n{text}\n{}",
                String::from_utf8_lossy(&output.stderr)
            )
        });
    serde_json::from_str(line).expect("JSON")
}

fn median(mut xs: Vec<f64>) -> f64 {
    xs.sort_by(f64::total_cmp);
    match xs.len() {
        0 => f64::NAN,
        n if n % 2 == 1 => xs[n / 2],
        n => (xs[n / 2 - 1] + xs[n / 2]) / 2.0,
    }
}

#[test]
#[ignore = "a measurement, run by hand in release mode"]
fn kcad() {
    let sizes: Vec<usize> = std::env::var("KENTOS_PERF_SIZES")
        .unwrap_or_else(|_| "25000,50000,100000,200000".into())
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    let runs: usize = std::env::var("KENTOS_PERF_RUNS")
        .ok()
        .and_then(|r| r.parse().ok())
        .unwrap_or(3);
    let label = std::env::var("KENTOS_PERF_LABEL").unwrap_or_else(|_| "latest".into());
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join(".run/perf");
    std::fs::create_dir_all(&dir).expect("a folder for the files");
    let mut rows = Vec::new();
    for &n in &sizes {
        let file = dir.join(format!("olcum-{n}.kcad"));
        let mut all = Vec::new();
        for run in 1..=runs {
            let _ = std::fs::remove_file(&file);
            let save = run_child("save", n, &file);
            let open = run_child("open", n, &file);
            println!("{n} parsel, koşu {run}: kayıt {save}, açma {open}");
            all.push((save, open));
        }
        let _ = std::fs::remove_file(&file);
        let pick = |f: &dyn Fn(&(Value, Value)) -> f64| median(all.iter().map(f).collect());
        rows.push(json!({
            "parcels": n,
            "bytes": all[0].0["bytes"],
            "snapshotMs": pick(&|r| r.0["snapshotMs"].as_f64().unwrap_or(f64::NAN)),
            "writeMs": pick(&|r| r.0["writeMs"].as_f64().unwrap_or(f64::NAN)),
            "savePeakAboveMb": pick(&|r| r.0["peakAboveMb"].as_f64().unwrap_or(f64::NAN)),
            "drawingRssMb": pick(&|r| r.0["rssBeforeMb"].as_f64().unwrap_or(f64::NAN)),
            "readMs": pick(&|r| r.1["readMs"].as_f64().unwrap_or(f64::NAN)),
            "sceneMs": pick(&|r| r.1["sceneMs"].as_f64().unwrap_or(f64::NAN)),
            "openPeakAboveMb": pick(&|r| r.1["peakAboveMb"].as_f64().unwrap_or(f64::NAN)),
            "runs": all.iter().map(|(s, o)| json!({ "save": s, "open": o })).collect::<Vec<_>>(),
        }));
    }
    let text = |cmd: &str, args: &[&str]| {
        Command::new(cmd)
            .args(args)
            .current_dir(&root)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
            .unwrap_or_default()
    };
    let cpu = std::fs::read_to_string("/proc/cpuinfo")
        .unwrap_or_default()
        .lines()
        .find_map(|l| l.strip_prefix("model name"))
        .map(|l| l.trim_start_matches([' ', '\t', ':']).to_owned())
        .unwrap_or_default();
    let threads = std::thread::available_parallelism().map_or(0, |n| n.get());
    let memory_gb = std::fs::read_to_string("/proc/meminfo")
        .unwrap_or_default()
        .lines()
        .find_map(|l| l.strip_prefix("MemTotal:"))
        .and_then(|v| v.trim().trim_end_matches("kB").trim().parse::<f64>().ok())
        .map_or(0.0, |kb| (kb / 1024.0 / 1024.0).round());
    let os = std::fs::read_to_string("/etc/os-release")
        .unwrap_or_default()
        .lines()
        .find_map(|l| l.strip_prefix("PRETTY_NAME="))
        .map(|l| l.trim_matches('"').to_owned())
        .unwrap_or_default();
    let commit = text("git", &["rev-parse", "--short", "HEAD"]);
    let dirty = !text("git", &["status", "--porcelain", "--untracked-files=no"]).is_empty();
    let rustc = text("rustc", &["--version"]);
    let kernel = text("uname", &["-r"]);
    let date = text("date", &["+%F"]);
    let report = json!({
        "label": label,
        "date": date,
        "commit": commit,
        "dirtyTree": dirty,
        "machine": { "cpu": cpu, "threads": threads, "memoryGb": memory_gb, "os": os, "kernel": kernel, "rustc": rustc },
        "setup": { "runs": runs, "sizes": sizes, "profile": "release (lto thin)", "files": ".run/perf" },
        "summary": rows,
    });
    let n0 = |v: &Value| {
        v.as_f64()
            .map_or("–".to_owned(), |x| format!("{}", x.round() as i64))
    };
    let mut md = vec![
        format!(
            "# KCAD v2 masaüstünde kaydet ve aç: {label} ({date}, {commit}{})",
            if dirty {
                ", kaydedilmemiş değişiklikle"
            } else {
                ""
            }
        ),
        String::new(),
        format!(
            "{cpu}, {threads} iş parçacığı, {memory_gb} GB; {os} ({kernel}); {rustc}, `--release` (lto thin). {runs} koşu, her hücre ortanca. Test `apps/desktop/src/perf.rs`: her işlem kendi sürecinde çalışır, bellek o sürecin en yüksek yerleşik belleğinin (VmHWM) işlemden önceki düzeyin üstündeki payıdır. Dosyalar `.run/perf`'e yazılır (yerel disk, `fsync` dahil)."
        ),
        String::new(),
        "Çizim: parsel başına 20 köşeli bir alan, üç öznitelik ve etiket, tek katman (`crates/shared/kcad/tests/measure.rs` ile aynı). “Anlık görüntü” kaydın arayüz iş parçacığındaki payıdır (`to_snapshot_v2`); “yazma” doğrulamalı kodlama, geçici dosya, `fsync`, geri okuma ve yer değiştirmedir (`document::write`). “Açma” dosyanın okunması, çözülmesi ve belgenin kurulmasıdır (`Document::read`); “sahne” ilk karenin arayüz iş parçacığında kurulan sahnesidir (çizgiler ve eğriler, bütün çizim görünürken).".to_owned(),
        String::new(),
        "| Parsel | Dosya | Anlık görüntü | Yazma | Kayıtta bellek artışı | Açma | Açışta bellek artışı | Sahne | Açık çizimle süreç |".to_owned(),
        "|---|---|---|---|---|---|---|---|---|".to_owned(),
    ];
    for r in &rows {
        md.push(format!(
            "| {} | {:.1} MB | {} ms | {} ms | {} MB | {} ms | {} MB | {} ms | {} MB |",
            r["parcels"],
            r["bytes"].as_f64().unwrap_or(0.0) / 1e6,
            n0(&r["snapshotMs"]),
            n0(&r["writeMs"]),
            n0(&r["savePeakAboveMb"]),
            n0(&r["readMs"]),
            n0(&r["openPeakAboveMb"]),
            n0(&r["sceneMs"]),
            n0(&r["drawingRssMb"]),
        ));
    }
    match std::env::var("KENTOS_PERF_OUT") {
        Ok(out) => {
            let base = root.join(out).join(format!("kcad-desktop-{label}-{date}"));
            std::fs::write(
                base.with_extension("json"),
                format!("{}\n", serde_json::to_string_pretty(&report).expect("JSON")),
            )
            .expect("writes the report");
            std::fs::write(base.with_extension("md"), format!("{}\n", md.join("\n")))
                .expect("writes the report");
            println!("Yazıldı: {}.{{json,md}}", base.display());
        }
        Err(_) => println!("{}", md.join("\n")),
    }
}
