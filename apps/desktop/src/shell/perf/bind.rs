//! Binding a large database table on the desktop, measured (docs/adr/0757): what the UI thread does from Bağla to the
//! table's objects on screen, frame by frame, and how long the whole takes. Needs a PostGIS holding the table; not a
//! correctness test and not run by default:
//!
//! ```text
//! KENTOS_PG_URL=postgresql://postgres@127.0.0.1:PORT/postgres PGPASSWORD=… KENTOS_PG_BIND=kbs.parseller \
//!   cargo test --release -p kentos-desktop --lib perf::bind -- --ignored --nocapture --test-threads=1
//! ```

use std::time::{Duration, Instant};

use super::drawing;
use super::frame::{Harness, Parts};
use crate::app::{App, Message};
use crate::document::Document;
use crate::exchange::binding::{Ask, Table};

#[test]
#[ignore = "needs a PostGIS holding the table (KENTOS_PG_URL, KENTOS_PG_BIND)"]
fn bind() {
    let (Ok(url), Ok(table)) = (
        std::env::var("KENTOS_PG_URL"),
        std::env::var("KENTOS_PG_BIND"),
    ) else {
        eprintln!("KENTOS_PG_URL and KENTOS_PG_BIND name the table");
        return;
    };
    kentos_postgis::register();
    // At once on the interface's thread (as every bind was before), then on a thread of its own.
    for at_once in [true, false] {
        measure(at_once, &url, &table);
    }
}

fn measure(at_once: bool, url: &str, table: &str) {
    let way = if at_once {
        "arayüzde, hemen"
    } else {
        "kendi iş parçacığında"
    };
    let (mut app, _) = App::boot(None);
    let doc = Document::from_v2(drawing(0), None).expect("the drawing opens");
    let mut h = Harness::new();
    h.settle(&mut app);
    let _ = h.frame_with(&mut app, &[], move |app| {
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    });
    h.settle(&mut app);
    let ask = Ask {
        source: url.to_owned(),
        shown: "ölçüm".into(),
        tables: vec![Table {
            name: table.to_owned(),
            geometry: true,
            rows: None,
        }],
        reproject: false,
        area: None,
        key: None,
        file: false,
        then: String::new(),
    };
    let began = Instant::now();
    let mut frames: Vec<Parts> = Vec::new();
    frames.push(h.frame_with(&mut app, &[], |app| {
        if !at_once {
            let _ = app.start_binding(ask);
            return;
        }
        // The command itself in the frame, as the windows ran it before (docs/adr/0757's Bağlam).
        let input = kentos_commands::SourcesEdit {
            edit: kentos_commands::SourcesEditOp::Link {
                extent: None,
                path: ask.source.clone(),
                table: ask.tables[0].name.clone(),
                layer: None,
                reproject: false,
                key: None,
            },
        };
        if let Some(doc) = app.document.as_mut() {
            let _ = kentos_native_application::sources::edit::execute(
                &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
                input,
            );
        }
    }));
    // Frames as a display gives them while the bind goes on (each looks at its job), then until the drawing settles.
    let mut bound_at = None;
    while began.elapsed() < Duration::from_secs(120) {
        let t = Instant::now();
        frames.push(h.frame_with(&mut app, &[], |app| {
            if app.binding_now() {
                let _ = app.update(Message::Exchange(Box::new(
                    crate::exchange::Event::Binding(crate::exchange::binding::Event::Frame),
                )));
            }
        }));
        if bound_at.is_none() && !app.binding_now() {
            bound_at = Some(began.elapsed());
        }
        if let Some(at) = bound_at
            && began.elapsed() > at + Duration::from_millis(400)
        {
            break;
        }
        if let Some(rest) = Duration::from_millis(16).checked_sub(t.elapsed()) {
            std::thread::sleep(rest);
        }
    }
    let objects = app.document.as_ref().map_or(0, |d| d.model.len());
    let cpu: Vec<f64> = frames.iter().map(Parts::cpu).collect();
    let longest = cpu.iter().copied().fold(0.0, f64::max);
    let over = cpu.iter().filter(|&&c| c > 50.0).count();
    println!(
        "{table}, {way}: {objects} nesne; bağlama {:.0} ms sürdü, {} kare; arayüzün en uzun karesi {longest:.0} ms (50 ms'yi aşan {over} kare)",
        bound_at.map_or(f64::NAN, |d| d.as_secs_f64() * 1000.0),
        frames.len()
    );
    for (i, p) in frames.iter().enumerate().filter(|(_, p)| p.cpu() > 16.0) {
        println!("  kare {i}: {p:?}");
    }
}
