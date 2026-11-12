//! Each stage of `kentos_dwg::read` on real files, timed and counted: the signature, the codec's DWG reading (its
//! completion, recovered errors, skipped records and first diagnostics), the bridge DXF and KentOS's DXF reading.
//! Investigation only; nothing is written.
//!
//!     cargo run --release -p kentos-dwg --example diagnose -- FILE.dwg…
use std::io::Cursor;
use std::time::Instant;

fn main() {
    for path in std::env::args().skip(1) {
        println!("=== {path}");
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                println!("  okunamadı: {e}");
                continue;
            }
        };
        println!("  bayt {}", bytes.len());
        match kentos_dwg::version(&bytes) {
            Ok(v) => println!("  imza {}", v.signature()),
            Err(e) => {
                println!("  imza reddi [{}]: {}", e.code, e.message);
                continue;
            }
        }
        let t = Instant::now();
        let outcome = std::panic::catch_unwind(|| {
            acadrust::DwgReader::from_stream(Cursor::new(&bytes)).read_with_stats()
        });
        let outcome = match outcome {
            Err(_) => {
                println!("  kodek PANIC ({:.1} s)", t.elapsed().as_secs_f64());
                continue;
            }
            Ok(Err(e)) => {
                println!("  kodek hatası ({:.1} s): {e}", t.elapsed().as_secs_f64());
                continue;
            }
            Ok(Ok(o)) => o,
        };
        let s = &outcome.stats;
        println!(
            "  kodek {:.1} s: stream_completed={} recovered_errors={} skipped={} diagnostics={} entities={}",
            t.elapsed().as_secs_f64(),
            s.stream_completed,
            s.recovered_errors,
            s.skipped_source_records,
            s.diagnostics.len(),
            outcome.document.entities().count()
        );
        let mut kinds = std::collections::BTreeMap::<String, usize>::new();
        for d in &s.diagnostics {
            *kinds
                .entry(format!(
                    "{} [{:?}] {}",
                    d.code,
                    d.stage,
                    d.record_type.clone().unwrap_or_default()
                ))
                .or_default() += 1;
        }
        for (k, n) in kinds.iter().take(25) {
            println!("    {n:6} × {k}");
        }
        for d in s.diagnostics.iter().take(5) {
            println!(
                "    örnek: {} [{:?}] {:?}: {}",
                d.code, d.stage, d.record_handle, d.message
            );
        }
        let mut outcome = outcome;
        let t = Instant::now();
        let ls = [
            "LSDEFINITION",
            "LSSYMBOLCOMPONENT",
            "LSCOMPOUNDCOMPONENT",
            "LSSTROKEPATTERNCOMPONENT",
            "LSPOINTCOMPONENT",
            "LSINTERNALCOMPONENT",
        ];
        let gone: std::collections::HashSet<acadrust::Handle> = outcome
            .document
            .objects
            .iter()
            .filter(|(_, o)| match o {
                acadrust::objects::ObjectType::DgnLineStyle(_)
                | acadrust::objects::ObjectType::ProxyObject(_) => true,
                acadrust::objects::ObjectType::Unknown { type_name, .. } => {
                    ls.contains(&type_name.as_str())
                }
                _ => false,
            })
            .map(|(h, _)| *h)
            .collect();
        outcome.document.objects.retain(|h, _| !gone.contains(h));
        for o in outcome.document.objects.values_mut() {
            if let acadrust::objects::ObjectType::Dictionary(d) = o {
                d.entries.retain(|(_, h)| !gone.contains(h));
            }
        }
        println!(
            "  ayıklanan {} nesne, {:.1} s",
            gone.len(),
            t.elapsed().as_secs_f64()
        );
        let t = Instant::now();
        let dxf = match acadrust::DxfWriter::new(&outcome.document)
            .with_version(acadrust::DxfVersion::AC1032)
            .write_to_vec()
        {
            Ok(d) => d,
            Err(e) => {
                println!("  köprü DXF hatası: {e}");
                continue;
            }
        };
        println!(
            "  köprü DXF {:.1} MB, {:.1} s",
            dxf.len() as f64 / 1e6,
            t.elapsed().as_secs_f64()
        );
        if let Ok(dir) = std::env::var("KENTOS_DIAG_WRITE") {
            let name = std::path::Path::new(&path)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let _ = std::fs::write(
                std::path::Path::new(&dir).join(format!("{name}.bridge.dxf")),
                &dxf,
            );
        }
        let t = Instant::now();
        match kentos_formats::dxf::read_checked_watched_within(
            &dxf,
            &kentos_contracts::DxfReadOptions {
                allow_polyline_loss: true,
                closed_polylines: kentos_contracts::ClosedPolylines::Line,
                ..Default::default()
            },
            &mut kentos_formats::watch::Quiet,
            1 << 30,
        ) {
            Ok(r) => println!(
                "  görünüm {:?}; sınırlar {:?}\n  KentOS DXF {:.1} s: {} nesne, {} katman, {} blok; atlanan {} satır",
                r.view,
                r.bounds,
                t.elapsed().as_secs_f64(),
                r.entities.len(),
                r.layers.len(),
                r.blocks.len(),
                r.report.skipped.iter().map(|i| i.count as u64).sum::<u64>()
            ),
            Err(e) => println!(
                "  KentOS DXF hatası ({:.1} s) [{}]: {}",
                t.elapsed().as_secs_f64(),
                e.code,
                e.message
            ),
        }
        let t = Instant::now();
        match kentos_dwg::read(&bytes, &kentos_contracts::DxfReadOptions::default()) {
            Ok(r) => println!(
                "  kentos_dwg::read {:.1} s: tamam, {} nesne",
                t.elapsed().as_secs_f64(),
                r.entities.len()
            ),
            Err(e) => println!(
                "  kentos_dwg::read {:.1} s: RED [{}] {}",
                t.elapsed().as_secs_f64(),
                e.code,
                e.message
            ),
        }
    }
}
