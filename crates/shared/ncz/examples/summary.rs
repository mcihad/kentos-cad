//! What an import of each file gives: the objects by kind, the layers, the
//! report and the time it took. Not part of the app.
//!
//!   cargo run --release -p kentos-ncz --example summary -- plan.ncz

use std::time::Instant;

use kentos_contracts::NczReadOptions;
use kentos_formats::watch::Quiet;

fn main() {
    for path in std::env::args().skip(1) {
        let Ok(data) = std::fs::read(&path) else {
            eprintln!("{path}: okunamadı");
            continue;
        };
        let t0 = Instant::now();
        let read = kentos_ncz::read(&data, &NczReadOptions::default(), &mut Quiet);
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        println!("=== {path}  {:.1} MB  {ms:.0} ms", data.len() as f64 / 1e6);
        match read {
            Err(e) => println!("  HATA: {e}"),
            Ok(r) => {
                println!("  nesne {}  katman {}", r.entities.len(), r.layers.len());
                println!("  türler {:?}", r.report.counts);
                for f in &r.report.source {
                    println!("  kaynak {}: {}", f.label, f.value);
                }
                if let Some(d) = &r.declared_crs {
                    println!("  sistem {:?} {}", d.srid, d.text);
                }
                for i in &r.report.notes {
                    println!("  not {} ×{}: {}", i.what, i.count, i.reason);
                }
                for i in &r.report.skipped {
                    println!("  alınmadı {} ×{}: {}", i.what, i.count, i.reason);
                }
                for l in r.layers.iter().take(usize::MAX) {
                    println!("  katman {} {} {}", l.name, l.color, l.count);
                }
            }
        }
    }
}
