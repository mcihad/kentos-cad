//! Opens CAD files the way the desktop's Aç does (`cad_open::read_in`), with the unit given (KENTOS_FILE_UNIT: mm,
//! cm, m; cm when none), and says what came of each: the time, the refusal or the report's notes. Investigation only;
//! nothing is written.
//!
//!     cargo run --release -p kentos-native-application --example open_cad -- FILE…
use kentos_native_application::cad_open::{Format, read_in};
use std::time::Instant;

fn main() {
    let unit = match std::env::var("KENTOS_FILE_UNIT").as_deref() {
        Ok("mm") => kentos_commands::FileUnit::Mm,
        Ok("m") => kentos_commands::FileUnit::M,
        _ => kentos_commands::FileUnit::Cm,
    };
    for arg in std::env::args().skip(1) {
        let path = std::path::PathBuf::from(&arg);
        println!("=== {arg}");
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                println!("  okunamadı: {e}");
                continue;
            }
        };
        let Some(format) = Format::of(&path) else {
            println!("  CAD biçimi değil");
            continue;
        };
        let t = Instant::now();
        match read_in(
            &path,
            &bytes,
            format,
            Some(unit),
            &mut kentos_formats::watch::Quiet,
        ) {
            Ok(opened) => {
                println!("  AÇILDI {:.1} s", t.elapsed().as_secs_f64());
                for n in opened.report.notes.iter().take(12) {
                    println!("    not: {} × {}: {}", n.count, n.what, n.reason);
                }
                for n in opened.report.skipped.iter().take(12) {
                    println!("    atlanan: {} × {}: {}", n.count, n.what, n.reason);
                }
            }
            Err(e) => println!(
                "  RED {:.1} s [{}]: {}",
                t.elapsed().as_secs_f64(),
                e.code,
                e.message
            ),
        }
    }
}
