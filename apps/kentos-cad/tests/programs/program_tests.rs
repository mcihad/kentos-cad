//! What spans the programs (docs/adr/9199 §4.2): every command the catalog lists as ported has a handler, and every
//! usage trace passes in every variant, with every module's parts registered as `kentos-cad` registers them.

use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};

use kentos_interaction::Level;

use crate::app::App;
use crate::traces::{PENDING, Trace, VARIANTS, Variant, play_trace};

#[test]
fn every_ported_command_has_a_handler() {
    for id in crate::catalog::PORTED {
        let (mut app, _) = App::boot(None);
        let _ = app.run(id);
        assert!(
            !app.log.said(Level::Error, "işleyicisi eksik"),
            "{id} is listed as ported but has no handler"
        );
    }
}

/// A panic's message, as the payload carries it.
fn said(payload: &(dyn std::any::Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("(iletisiz)")
}

/// Every trace × every variant, as `pnpm e2e:interaction` plays them on the web.
/// Each play has an app of its own, so the plays share out among the
/// machine's cores; the report keeps their order, and a play that panics
/// is a problem of its own, not the end of the others.
#[test]
fn every_trace_passes_in_every_variant() {
    let mut traces = Trace::all().expect("the traces read");
    assert!(traces.len() >= 4, "the four traces are there");
    // `KENTOS_TRACES_ONLY=time-slider,scenarios`: only these (while working on them).
    if let Ok(only) = std::env::var("KENTOS_TRACES_ONLY") {
        let ids: Vec<&str> = only.split(',').map(str::trim).collect();
        traces.retain(|t| ids.contains(&t.id.as_str()));
    }
    let plays: Vec<(Variant, &Trace)> = VARIANTS
        .iter()
        .flat_map(|&variant| traces.iter().map(move |trace| (variant, trace)))
        .collect();
    let next = AtomicUsize::new(0);
    let results = Mutex::new(vec![Vec::new(); plays.len()]);
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(plays.len());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                // A long play does not hold up the short ones behind it: each thread takes the next.
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&(variant, trace)) = plays.get(i) else {
                        break;
                    };
                    let problems = panic::catch_unwind(AssertUnwindSafe(|| {
                        play_trace(trace, variant).unwrap_or_else(|e| vec![e])
                    }))
                    .unwrap_or_else(|payload| vec![format!("panik: {}", said(&*payload))]);
                    results.lock().unwrap_or_else(PoisonError::into_inner)[i] = problems;
                }
            });
        }
    });
    let results = results.into_inner().unwrap_or_else(PoisonError::into_inner);
    let mut report = Vec::new();
    let mut failed = 0;
    for ((variant, trace), problems) in plays.iter().zip(&results) {
        let mark = if problems.is_empty() { "✓" } else { "✗" };
        report.push(format!(
            "{mark} [{}] {}: {}",
            variant.id, trace.id, trace.title
        ));
        for p in problems {
            report.push(format!("  {p}"));
        }
        if !problems.is_empty() {
            failed += 1;
        }
    }
    for (id, why) in PENDING {
        report.push(format!("– {id}: atlandı, geçmedi: {why}"));
    }
    println!("{}", report.join("\n"));
    assert_eq!(failed, 0, "\n{}", report.join("\n"));
}
