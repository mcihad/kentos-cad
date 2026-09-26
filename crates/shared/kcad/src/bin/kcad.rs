//! `kcad`: the developer's tool for KentOS project files (TODOS.md FILE-23,
//! docs/specs/kcad-v2.md §12). Built on the host only; the library it calls is
//! the one the apps use.
//!
//!     cargo run -q -p kentos-kcad --bin kcad -- inspect DOSYA
//!     cargo run -q -p kentos-kcad --bin kcad -- validate DOSYA…
//!     cargo run -q -p kentos-kcad --bin kcad -- sniff DOSYA…
//!     cargo run -q -p kentos-kcad --bin kcad -- migrate ESKİ.kcad YENİ.kcad
//!
//! `migrate` turns a v1 (JSON) drawing into a v2 file with its derived ids
//! and source record (docs/adr/0014). It never writes over an existing file,
//! and it writes only bytes that read back to the same drawing.
//! Exit status: 0 when every file is valid, 1 when one is not, 2 on a usage error.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::process::ExitCode;

use kentos_kcad::contracts::{DocumentSnapshotV1, LayerNode, LayerNodeType, migrate_v1};
use kentos_kcad::{Header, Sniff};

const USAGE: &str =
    "Kullanım: kcad inspect DOSYA | validate DOSYA… | sniff DOSYA… | migrate ESKİ.kcad YENİ.kcad";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((command, files)) = args.split_first() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match (command.as_str(), files) {
        ("inspect", [file]) => report(inspect(file)),
        ("validate", files) if !files.is_empty() => {
            let mut ok = true;
            for file in files {
                match read(file).and_then(|data| {
                    kentos_kcad::decode(&data).map_err(|e| format!("{}: {}", e.code.as_str(), e))
                }) {
                    Ok(doc) => println!("{file}: geçerli ({} nesne)", doc.entities.len()),
                    Err(e) => {
                        println!("{file}: {e}");
                        ok = false;
                    }
                }
            }
            if ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        ("sniff", files) if !files.is_empty() => {
            let mut ok = true;
            for file in files {
                match read(file) {
                    Ok(data) => println!("{file}: {}", kentos_kcad::sniff(&data).as_str()),
                    Err(e) => {
                        eprintln!("{e}");
                        ok = false;
                    }
                }
            }
            if ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        ("migrate", [from, to]) => report(migrate(from, to)),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn report(result: Result<String, String>) -> ExitCode {
    match result {
        Ok(text) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

fn read(file: &str) -> Result<Vec<u8>, String> {
    std::fs::read(file).map_err(|e| format!("{file}: okunamadı ({e})"))
}

fn inspect(file: &str) -> Result<String, String> {
    let data = read(file)?;
    let (header, doc) =
        kentos_kcad::read(&data).map_err(|e| format!("{file}: {}: {e}", e.code.as_str()))?;
    Ok(summary(&header, &doc))
}

fn summary(h: &Header, doc: &kentos_kcad::contracts::DocumentSnapshotV2) -> String {
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    for e in &doc.entities {
        *kinds.entry(e.kind()).or_default() += 1;
    }
    let mut versions: BTreeMap<u8, usize> = BTreeMap::new();
    for uid in &doc.uids {
        *versions.entry(uid.0[6] >> 4).or_default() += 1;
    }
    fn walk(nodes: &[LayerNode], layers: &mut usize, groups: &mut usize) {
        for n in nodes {
            match n.kind {
                LayerNodeType::Layer => *layers += 1,
                LayerNodeType::Group => *groups += 1,
            }
            walk(&n.children, layers, groups);
        }
    }
    let (mut layers, mut groups) = (0, 0);
    walk(&doc.layers, &mut layers, &mut groups);
    let hash: String = h.sha256[..8].iter().map(|b| format!("{b:02x}")).collect();
    let list = |m: Vec<String>| {
        if m.is_empty() {
            String::new()
        } else {
            format!(" — {}", m.join(", "))
        }
    };
    let mut out = String::new();
    let _ = writeln!(
        out,
        "KCAD {}.{} (en az okuyucu {}.{}), kodlama {} (CBOR profili 1), sıkıştırma yok",
        h.major, h.minor, h.major, h.min_reader_minor, h.encoding
    );
    let _ = writeln!(
        out,
        "Yük: {} bayt, SHA-256 doğru ({hash}…)",
        h.payload_length
    );
    let _ = writeln!(out, "Belge: {} sürüm {}", doc.format, doc.version);
    let _ = writeln!(out, "Ad: {}", doc.name);
    let _ = writeln!(
        out,
        "Koordinat sistemi: EPSG:{}, ölçek 1:{}",
        doc.settings.srid, doc.settings.plot_scale
    );
    let _ = writeln!(
        out,
        "Katmanlar: {layers} katman, {groups} grup; etkin: {}",
        doc.active_layer
    );
    let _ = writeln!(
        out,
        "Nesneler: {}{}",
        doc.entities.len(),
        list(kinds.iter().map(|(k, n)| format!("{k} {n}")).collect())
    );
    let _ = writeln!(
        out,
        "Kalıcı kimlikler: {}, benzersiz{}",
        doc.uids.len(),
        list(versions.iter().map(|(v, n)| format!("v{v}: {n}")).collect())
    );
    let _ = writeln!(
        out,
        "Proje kimliği: {}",
        doc.project_id
            .map_or_else(|| "yok".to_owned(), |p| p.to_text())
    );
    match &doc.migrated_from {
        Some(s) => {
            let _ = writeln!(
                out,
                "Göç kaynağı: {} sürüm {}, sha256 {}",
                s.format, s.version, s.source_sha256
            );
        }
        None => {
            let _ = writeln!(out, "Göç kaynağı: yok");
        }
    }
    let _ = write!(
        out,
        "Proje stilleri: {} öğe, {} kategori",
        doc.styles.items.len(),
        doc.styles.categories.len()
    );
    out
}

fn migrate(from: &str, to: &str) -> Result<String, String> {
    let data = read(from)?;
    match kentos_kcad::sniff(&data) {
        Sniff::Json => {}
        Sniff::Kcad | Sniff::KcadDamaged => {
            return Err(format!(
                "{from}: zaten KCAD v2 dosyası; göç yalnız eski (v1, JSON) çizimler içindir."
            ));
        }
        _ => return Err(format!("{from}: KentOS çizimi (v1, JSON) değil.")),
    }
    let text = std::str::from_utf8(&data)
        .map_err(|_| format!("{from}: metin UTF-8 değil; v1 çizimi okunamadı."))?;
    let v1 = DocumentSnapshotV1::from_json(text).map_err(|e| format!("{from}: {e}"))?;
    let v2 = migrate_v1(v1).map_err(|e| format!("{from}: {e}"))?;
    let bytes = kentos_kcad::encode_verified(&v2).map_err(|e| format!("{to}: {e}"))?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(to)
        .map_err(|e| {
            format!(
                "{to}: açılamadı ({e}); var olan bir dosyanın üzerine yazılmaz, başka bir ad verin."
            )
        })?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| format!("{to}: yazılamadı ({e})."))?;
    let (header, doc) = kentos_kcad::read(&bytes).map_err(|e| format!("{to}: {e}"))?;
    Ok(format!(
        "{to} yazıldı ({} bayt); {from} değiştirilmedi.\n{}",
        bytes.len(),
        summary(&header, &doc)
    ))
}

use std::fmt::Write as _;
