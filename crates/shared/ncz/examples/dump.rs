//! The NCZ parse of each file as text, one record per line, every float as
//! its bits: what a reading is compared with another reader's by (the
//! reference plugin's, docs/adr/0138). Not part of the app.
//!
//!   cargo run --release -p kentos-ncz --example dump -- a.ncz b.ncz > out.txt

use std::fmt::Write as _;

use kentos_formats::watch::Quiet;
use kentos_ncz::attributes::{self, Cell};
use kentos_ncz::format::{self, Entity, Header, SmartClass, Sink};

fn esc(s: &str) -> String {
    s.replace('\0', "\\0")
}

fn bits(v: f64) -> String {
    if v.is_nan() {
        "nan".into()
    } else {
        format!("{:016x}", v.to_bits())
    }
}

struct Dump(String);

impl Sink for Dump {
    fn entity(&mut self, e: &Entity) -> bool {
        let o = &mut self.0;
        let color = e.color.map_or_else(|| "None".to_owned(), |c| c.to_string());
        let _ = write!(
            o,
            "E|{}|{}|{}|{}|{}|{}|",
            e.kind.name(),
            e.layer_code,
            esc(&e.layer_name),
            color,
            esc(&e.name),
            esc(&e.label)
        );
        for v in [
            e.text_height,
            e.rotation,
            e.box_width,
            e.box_height,
            e.scale,
            e.grid_x,
            e.grid_y,
            e.radius,
            e.start_angle,
            e.end_angle,
        ] {
            let _ = write!(o, "{}|", bits(v));
        }
        let _ = write!(o, "{}|{}|{}", bits(e.line_width), u8::from(e.closed), e.coords.len());
        if e.smart != SmartClass::None {
            let _ = write!(o, "|S:{}", e.smart.name());
            for p in &e.properties {
                let _ = write!(o, "~{}={}/{}/{}{}", p.name, p.value, p.display, u8::from(p.user), u8::from(p.null));
            }
        }
        for c in &e.coords {
            let _ = write!(o, "|{};{};{}", bits(c.x), bits(c.y), bits(c.z));
        }
        o.push('\n');
        true
    }
}

fn main() {
    for path in std::env::args().skip(1) {
        println!("=== {path}");
        let data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("{path}: {e}");
                continue;
            }
        };
        let mut dump = Dump(String::new());
        let mut h = Header::default();
        let outcome = format::read(&data, &mut dump, &mut h, &mut Quiet);
        print!("{}", dump.0);
        let mut o = String::new();
        for n in &h.layer_names {
            let _ = writeln!(o, "L|{}", esc(n));
        }
        for c in &h.layer_colors {
            let _ = writeln!(o, "C|{c}");
        }
        let _ = writeln!(o, "V|{}\nP|{}\nS|{}", esc(&h.version_name), esc(&h.projection_text), esc(&h.epsg));
        for (t, &n) in h.unsupported.iter().enumerate() {
            if n != 0 {
                let _ = writeln!(o, "U|{t}|{n}");
            }
        }
        for t in attributes::tables(&data, &mut Quiet, 0, 0).unwrap_or_default() {
            for r in &t.rows {
                let _ = write!(o, "T|{}|{}", t.table_ref, r.row_index);
                for (k, v) in &r.columns {
                    let _ = write!(o, "|{k}=");
                    let _ = match v {
                        Cell::None => write!(o, "None"),
                        Cell::Int(i) => write!(o, "i{i}"),
                        Cell::Float(f) => write!(o, "f{}", bits(*f)),
                        Cell::Text(s) => write!(o, "s{}", esc(s)),
                    };
                }
                o.push('\n');
            }
        }
        print!("{o}");
        let dropped: Vec<String> = h
            .dropped
            .iter()
            .enumerate()
            .filter(|(_, n)| **n != 0)
            .map(|(t, n)| format!("{t}={n}"))
            .collect();
        eprintln!("outcome {outcome:?} dropped: {} smart_marks={}", dropped.join(" "), h.smart_marks);
    }
}
