//! Nokta bulutu araçlarının ortak durumları (fixtures/processing/v1/pointcloud.json, written by
//! scripts/fixtures/pointcloud_processing_cases.py; docs/adr/0207 §7, 0652): each runs a point cloud tool as the desktop
//! does, with files that read the cases' clouds and keep what the run writes. Besides the cases' usual checks, a
//! written cloud (`cloudOf`) is read back: its point count, its points' X, Y and Z whole numbers in the file's order (a
//! COPC's sorted, its index orders them), its classes or its whole records, each as the independent reference's FNV-1a
//! 64; a written raster (`cloudRasterOf`) is a COG whose cells' float32 bits are the reference's; `written` names every
//! file a run leaves.

use kentos_commands::CloudFormat;
use kentos_processing::cloud_files::cloud_read;

use super::surface::{FixtureFiles, Held, level0, surface_fixture};
use super::*;

fn fnv(bytes: &[u8]) -> String {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01B3);
    }
    format!("{h:016x}")
}

/// A written LAS, LAZ or COPC's records in the file's order, and its point format.
fn records_of(bytes: &[u8]) -> Result<(Vec<Vec<u8>>, u8), String> {
    let mut read = cloud_read(Box::new(Held(bytes.to_vec())), CloudFormat::Laz, None)?;
    let layout = read.input().layout;
    let mut out = Vec::new();
    let mut raw = Vec::new();
    loop {
        raw.clear();
        if !read.next(&mut raw)? {
            break;
        }
        out.extend(raw.chunks_exact(layout.len).map(<[u8]>::to_vec));
    }
    Ok((out, layout.format))
}

/// What a written cloud's check says against its records: the faults.
fn cloud_faults(id: &str, name: &str, bytes: &[u8], want: &Value) -> Vec<String> {
    let mut found = Vec::new();
    let (records, format) = match records_of(bytes) {
        Ok(r) => r,
        Err(e) => return vec![format!("{id}: “{name}” does not read: {e}")],
    };
    if want["copc"] == true
        && !bytes[..bytes.len().min(2048)]
            .windows(4)
            .any(|w| w == b"copc")
    {
        found.push(format!("{id}: “{name}” has no COPC info"));
    }
    if let Some(n) = want["count"].as_u64()
        && records.len() as u64 != n
    {
        found.push(format!(
            "{id}: “{name}” has {} points for {n}",
            records.len()
        ));
    }
    if let Some(h) = want["points"].as_str() {
        let mut triples: Vec<[u8; 12]> = records
            .iter()
            .map(|r| r[..12].try_into().expect("twelve bytes"))
            .collect();
        if want["copc"] == true {
            let key = |t: &[u8; 12]| {
                let n = |k: usize| i32::from_le_bytes(t[k..k + 4].try_into().expect("four bytes"));
                (n(0), n(4), n(8))
            };
            triples.sort_by_key(key);
        }
        let got = fnv(&triples.concat());
        if got != h {
            found.push(format!("{id}: “{name}” points {got} for {h}"));
        }
    }
    if let Some(h) = want["classes"].as_str() {
        let at = if format >= 6 { 16 } else { 15 };
        let mask = if format >= 6 { 0xFF } else { 0x1F };
        let classes: Vec<u8> = records.iter().map(|r| r[at] & mask).collect();
        let got = fnv(&classes);
        if got != h {
            found.push(format!("{id}: “{name}” classes {got} for {h}"));
        }
    }
    if let Some(h) = want["records"].as_str() {
        let got = fnv(&records.concat());
        if got != h {
            found.push(format!("{id}: “{name}” records {got} for {h}"));
        }
    }
    found
}

/// What a written raster's check says: the faults.
fn raster_faults(id: &str, name: &str, bytes: &[u8], want: &Value) -> Vec<String> {
    let mut found = Vec::new();
    if !kentos_formats::raster::cog::declared(&bytes[..bytes.len().min(256)])
        || kentos_formats::raster::cog::of_bytes(bytes).as_deref() != Ok(bytes)
    {
        found.push(format!("{id}: “{name}” is not a COG"));
    }
    let values = level0(bytes);
    let cells = want["cols"].as_u64().unwrap_or(0) * want["rows"].as_u64().unwrap_or(0);
    if values.len() as u64 != cells {
        found.push(format!(
            "{id}: “{name}” has {} cells for {cells}",
            values.len()
        ));
    }
    let filled = values.iter().filter(|v| **v != -9999.0).count() as u64;
    if Some(filled) != want["filled"].as_u64() {
        found.push(format!(
            "{id}: “{name}” has {filled} filled cells for {}",
            want["filled"]
        ));
    }
    let bits: Vec<u8> = values
        .iter()
        .flat_map(|v| (*v as f32).to_le_bytes())
        .collect();
    let got = fnv(&bits);
    if Some(got.as_str()) != want["values"].as_str() {
        found.push(format!(
            "{id}: “{name}” values {got} for {}",
            want["values"]
        ));
    }
    found
}

#[test]
fn the_point_cloud_cases_do_what_they_say() {
    let file = case_file("pointcloud.json");
    let tol = file["tolerance"].as_f64().expect("a tolerance");
    let more: Vec<String> = file["toleranceFields"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let clouds: BTreeMap<String, Vec<u8>> = file["rasters"]
        .as_object()
        .expect("the cases' files")
        .iter()
        .map(|(name, path)| {
            (
                name.clone(),
                surface_fixture(path.as_str().expect("a path")),
            )
        })
        .collect();
    let registry = Registry::builtin();
    let (mut problems, mut report) = (Vec::new(), Vec::new());
    for c in file["cases"].as_array().expect("cases") {
        let written = Arc::new(Mutex::new(BTreeMap::new()));
        let files: Arc<dyn Files> = Arc::new(FixtureFiles {
            rasters: clouds.clone(),
            written: written.clone(),
        });
        let seen = play_with(c, &registry, true, &json!({}), Some(files));
        let mut found = check_full(c, seen, tol, tol, &more);
        let id = c["id"].as_str().unwrap_or("?");
        let kept = written.lock().unwrap_or_else(PoisonError::into_inner);
        let mut names: Vec<String> = Vec::new();
        for (key, check) in [
            (
                "cloudOf",
                cloud_faults as fn(&str, &str, &[u8], &Value) -> Vec<String>,
            ),
            ("cloudRasterOf", raster_faults),
        ] {
            for (name, want) in c["expect"][key].as_object().cloned().unwrap_or_default() {
                names.push(name.clone());
                match kept.get(&name) {
                    Some(bytes) => found.extend(check(id, &name, bytes, &want)),
                    None => found.push(format!("{id}: “{name}” was not written")),
                }
            }
        }
        let wanted: BTreeSet<String> = match c["expect"]["written"].as_array() {
            Some(list) => list
                .iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect(),
            None => names.into_iter().collect(),
        };
        let left: BTreeSet<String> = kept.keys().cloned().collect();
        if left != wanted {
            found.push(format!("{id}: written {left:?}, expected {wanted:?}"));
        }
        report.push(format!(
            "{} {id}: {}",
            if found.is_empty() { "✓" } else { "✗" },
            c["title"].as_str().unwrap_or("")
        ));
        problems.extend(found);
    }
    println!("{}", report.join("\n"));
    assert!(report.len() >= 20, "{} cases", report.len());
    assert!(
        problems.is_empty(),
        "\n{}\n\n{}",
        report.join("\n"),
        problems.join("\n")
    );
}
