//! Opens CAD files the way the desktop's Aç does (`cad_open::read_in`), with the unit given (KENTOS_FILE_UNIT: mm,
//! cm, m; cm when none), and says what came of each: the time, the refusal or the report's notes; with KENTOS_SAVE_KCAD
//! also whether Kaydet's KCAD encoding takes the drawing, with KENTOS_SHOW="BLOCK/OBJECT" that block's object as the
//! opened drawing holds it, with KENTOS_LOOKS how many objects draw with a line type or transparency of their own.
//! Investigation only; nothing is written.
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
                // KENTOS_SHOW="BLOCK/ENTITY": that block's object as the opened drawing holds it.
                if let Ok(at) = std::env::var("KENTOS_SHOW")
                    && let Some((b, e)) = at.split_once('/')
                    && let (Ok(b), Ok(e)) = (b.parse::<usize>(), e.parse::<usize>())
                {
                    let snap = opened.document.to_snapshot_v2();
                    if let Some(x) = snap.blocks.get(b).and_then(|d| d.entities.get(e)) {
                        println!(
                            "  blok {b} ({}) nesne {e}: {}",
                            snap.blocks[b].name,
                            serde_json::to_string(x).unwrap_or_default()
                        );
                    }
                }
                // KENTOS_LOOKS: how many objects draw with a line type of their own (dashes, dots, a named pattern),
                // in the drawing and in its blocks' definitions.
                if std::env::var_os("KENTOS_LOOKS").is_some() {
                    let snap = opened.document.to_snapshot_v2();
                    let count = |list: &mut dyn Iterator<Item = &kentos_commands::Entity>| {
                        let mut n = [0usize; 4];
                        for e in list {
                            let look = &e.base().look;
                            match look.line_type {
                                Some(kentos_commands::LineType::Continuous) | None => {}
                                Some(_) => n[0] += 1,
                            }
                            n[1] += usize::from(look.line_pattern.is_some());
                            n[2] += usize::from(look.transparency.is_some());
                            n[3] += 1;
                        }
                        n
                    };
                    let model = count(&mut snap.entities.iter());
                    let blocks = count(&mut snap.blocks.iter().flat_map(|b| b.entities.iter()));
                    println!(
                        "  GÖRÜNÜŞ çizim {} nesne: kesikli/noktalı {}, adlı desen {}, saydam {}; bloklar {} nesne: kesikli/noktalı {}, adlı desen {}, saydam {}",
                        model[3], model[0], model[1], model[2], blocks[3], blocks[0], blocks[1], blocks[2]
                    );
                }
                // KENTOS_SAVE_KCAD: written as Kaydet writes it, read back and compared (`encode_verified`).
                if std::env::var_os("KENTOS_SAVE_KCAD").is_some() {
                    let t = Instant::now();
                    match kentos_kcad::encode_verified(&opened.document.to_snapshot_v2()) {
                        Ok(bytes) => println!(
                            "  KCAD KAYDEDİLDİ {:.1} s, {:.1} MB",
                            t.elapsed().as_secs_f64(),
                            bytes.len() as f64 / 1e6
                        ),
                        Err(e) => println!(
                            "  KCAD RED {:.1} s [{:?}]: {}",
                            t.elapsed().as_secs_f64(),
                            e.code,
                            e.message
                        ),
                    }
                }
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
