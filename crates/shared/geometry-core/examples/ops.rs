//! The core's operations by name over standard input, one call a line
//! (`name<TAB>[arguments as JSON]`), one answer a line (the operation's
//! JSON, or `{"error": …}`): for the independent references that run
//! elsewhere and compare many points at once, such as
//! scripts/fixtures/crs_sweep.py (pyproj). A tool, never shipped.
//!
//! ```text
//! cargo run -q --release -p kentos-geometry-core --example ops < calls.tsv
//! ```

use std::io::{BufRead, Write};

use kentos_geometry_core::api::json::ToJson;

fn main() {
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else {
            break;
        };
        let (name, args) = line.split_once('\t').unwrap_or((line.as_str(), "[]"));
        let answer = kentos_geometry_core::api::run_named(name, args).unwrap_or_else(|e| {
            let mut text = String::from("{\"error\":");
            e.write_json(&mut text);
            text.push('}');
            text
        });
        if writeln!(out, "{answer}").is_err() {
            break;
        }
    }
}
