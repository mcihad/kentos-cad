//! Reads an IFC file through the reading face and times it (docs/adr/8061 §7 "Süre ve bellek", "İptal ve bütçe"):
//! the face (summary, objects, tree, findings), then the bodies when asked; the host stops it after a while when asked.
//! The peak memory is the operating system's (`/usr/bin/time -l` on macOS).
//!
//! ```text
//! cargo build --release -p kentos-ifc --features geometry --example ifc_read
//! /usr/bin/time -l target/release/examples/ifc_read FILE [--bodies] [--stop-after-ms N]
//! ```
//! Built without the `read` feature it only says so: its size is the base the engine's share is measured from.

fn main() {
    #[cfg(feature = "read")]
    run();
    #[cfg(not(feature = "read"))]
    println!("kentos-ifc read özelliği olmadan derlendi");
}

#[cfg(feature = "read")]
fn run() {
    use std::time::{Duration, Instant};

    let args: Vec<String> = std::env::args().collect();
    let Some(path) = args.get(1) else {
        eprintln!("kullanım: ifc_read DOSYA [--bodies] [--stop-after-ms N]");
        std::process::exit(2);
    };
    let bodies = args.iter().any(|a| a == "--bodies");
    let stop = args
        .iter()
        .position(|a| a == "--stop-after-ms")
        .and_then(|i| args.get(i + 1))
        .and_then(|n| n.parse::<u64>().ok())
        .map(Duration::from_millis);
    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("{path}: {e}");
        std::process::exit(1);
    });
    let start = Instant::now();
    let cancel = || stop.is_some_and(|s| start.elapsed() >= s);
    let read = kentos_ifc::read::read(&bytes, &kentos_ifc::read::Limits::default(), &cancel);
    let face = start.elapsed();
    match &read {
        Ok(m) => println!(
            "{} MB, {} ({}), {} kayıt, {} nesne, {} ilişki, {} bulgu; yüz {:.3} s",
            bytes.len() / 1_000_000,
            m.summary.schema,
            m.summary
                .header
                .originating_system
                .as_deref()
                .unwrap_or("?"),
            m.summary.records,
            m.objects.len(),
            m.relations.len(),
            m.findings.len(),
            face.as_secs_f64()
        ),
        Err(e) => println!(
            "{} MB: {e}; {:.3} s",
            bytes.len() / 1_000_000,
            face.as_secs_f64()
        ),
    }
    if let Ok(m) = &read {
        let mut codes = std::collections::BTreeMap::new();
        for f in &m.findings {
            *codes.entry(f.code).or_insert(0) += 1;
        }
        if let Some(first) = m.findings.first() {
            println!("bulgular {codes:?}; ilki: {}", first.message);
        }
    }
    #[cfg(feature = "geometry")]
    if bodies && let Ok(m) = &read {
        let from = Instant::now();
        match m.bodies(&bytes, &cancel) {
            Ok(b) => {
                let triangles: usize = b.bodies.iter().map(|x| x.triangles.len()).sum();
                let shaped = m.objects.iter().filter(|o| o.has_shape).count();
                println!(
                    "{} gövde ({shaped} şekilli nesne), {} gösterilemeyen, {triangles} üçgen; gövdeler {:.3} s",
                    b.bodies.len(),
                    b.not_shown.len(),
                    from.elapsed().as_secs_f64()
                );
                let mut why = std::collections::BTreeMap::new();
                for n in &b.not_shown {
                    let reason: String = n.reason.chars().take(90).collect();
                    *why.entry((m.objects[n.object].class.clone(), reason))
                        .or_insert(0) += 1;
                }
                for ((class, reason), count) in why {
                    println!("  gösterilemeyen {count} × {class}: {reason}");
                }
            }
            Err(e) => println!("gövdeler: {e}; {:.3} s", from.elapsed().as_secs_f64()),
        }
    }
    let _ = bodies;
    if let Some(s) = stop {
        println!(
            "durdurma {} ms sonra istendi; toplam {:.3} s",
            s.as_millis(),
            start.elapsed().as_secs_f64()
        );
    }
}
