//! The analysis environment's cases (docs/adr/0643, fixtures/raster-env/v1, scripts/fixtures/raster_env_cases.py): each
//! case's environment, as `cad.processing.run` gives it, resolved on the drawing and given to the run; the written
//! raster's grid and level 0 against the independent reference; a case `sameAs` another wrote the same bytes (the same
//! DEM in another block layout); a refusal says its words.

use kentos_commands::RasterEnvironment;
use kentos_processing::raster_env;

use super::surface::{FixtureFiles, level0, ulps};
use super::*;

fn env_folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/raster-env/v1")
}

/// `#n` in the case's environment is the uid of the drawing's object n.
fn with_uids(v: &Value, doc: &Document) -> Value {
    match v {
        Value::String(s) if s.starts_with('#') => {
            let id: u32 = s[1..].parse().expect("an object id");
            json!(doc.uid(Slot(id)).expect("the object").to_string())
        }
        Value::Array(a) => Value::Array(a.iter().map(|x| with_uids(x, doc)).collect()),
        Value::Object(m) => Value::Object(
            m.iter()
                .map(|(k, x)| (k.clone(), with_uids(x, doc)))
                .collect(),
        ),
        _ => v.clone(),
    }
}

#[test]
fn the_environment_cases_do_what_they_say() {
    let folder = env_folder();
    let file: Value =
        serde_json::from_slice(&std::fs::read(folder.join("cases.json")).expect("the cases"))
            .expect("they read");
    let drawing = std::fs::read_to_string(folder.join(file["document"].as_str().unwrap()))
        .expect("the drawing");
    let rasters: BTreeMap<String, Vec<u8>> = file["rasters"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(n, p)| {
            (
                n.clone(),
                std::fs::read(folder.join(p.as_str().unwrap())).expect("a raster"),
            )
        })
        .collect();
    let registry = Registry::builtin();
    let mut written_by: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let (mut report, mut problems) = (Vec::new(), Vec::new());
    for c in file["cases"].as_array().expect("cases") {
        let id = c["id"].as_str().unwrap().to_owned();
        let doc = Document::from_snapshot(DocumentSnapshotV1::from_json(&drawing).expect("reads"))
            .expect("opens");
        let mut found = Vec::new();
        let given: Option<RasterEnvironment> = (!c["environment"].is_null()).then(|| {
            serde_json::from_value(with_uids(&c["environment"], &doc))
                .expect("the environment reads")
        });
        let refused = c["expect"]["refused"].as_str();
        // The command's environment resolved as `cad.processing.run` resolves it.
        let environment = match given
            .as_ref()
            .map(|g| raster_env::resolve(&doc, g))
            .transpose()
        {
            Ok(e) => e.filter(|e| !e.is_empty()),
            Err(e) => {
                match refused {
                    Some(want) if e.message.contains(want) => {}
                    _ => found.push(format!("{id}: refused when resolved: {}", e.message)),
                }
                report.push(format!("{} {id}", if found.is_empty() { "✓" } else { "✗" }));
                problems.extend(found);
                continue;
            }
        };
        let written = Arc::new(Mutex::new(BTreeMap::new()));
        let files: Arc<dyn Files> = Arc::new(FixtureFiles {
            rasters: rasters.clone(),
            written: written.clone(),
        });
        let tool = registry
            .tool(c["tool"].as_str().unwrap())
            .expect("the tool");
        let mut host = TestHost {
            doc,
            selection: slots(c.get("selection")),
            view: None,
        };
        let values = with_values(&tool, &host.doc, c.get("values"), &json!({}));
        let mut runner = Runner::new();
        runner.environment = environment;
        let mut log = Vec::new();
        let outcome = match runner.prepare(&host, &tool, &values, false, &mut log) {
            Prepared::Ready(job) => {
                let copy = host.doc.reading_copy();
                let result = Runner::compute(
                    &job,
                    &copy,
                    &mut Apart {
                        log: &mut log,
                        files: Some(files),
                    },
                );
                runner.finish(&mut host, job, result, false, &mut log)
            }
            Prepared::Done(o) => o,
        };
        match (outcome, refused) {
            (Outcome::Stopped { message, .. }, Some(want)) if message.contains(want) => {}
            (Outcome::Stopped { message, .. }, _) => {
                found.push(format!("{id}: stopped: {message}"))
            }
            (Outcome::Invalid { issues }, _) => found.push(format!("{id}: invalid: {issues:?}")),
            (Outcome::Ok { .. }, Some(want)) => found.push(format!("{id}: ran; expected “{want}”")),
            (Outcome::Ok { result, .. }, None) => {
                let kept = written.lock().unwrap_or_else(PoisonError::into_inner);
                let Some((_, bytes)) = kept.iter().next().filter(|_| kept.len() == 1) else {
                    found.push(format!(
                        "{id}: written {:?}",
                        kept.keys().collect::<Vec<_>>()
                    ));
                    report.push(format!("✗ {id}"));
                    problems.extend(found);
                    continue;
                };
                let want = &c["expect"];
                let reader = kentos_formats::raster::source::open_bytes(
                    bytes,
                    None,
                    kentos_processing::files::RASTER_READER_BUDGET,
                )
                .expect("the result opens");
                let grid = (reader.info.affine, reader.info.width, reader.info.height);
                let want_affine: Vec<f64> =
                    serde_json::from_value(want["grid"]["affine"].clone()).unwrap();
                let want_grid = (
                    Some(<[f64; 6]>::try_from(want_affine).unwrap()),
                    want["grid"]["width"].as_u64().unwrap() as u32,
                    want["grid"]["height"].as_u64().unwrap() as u32,
                );
                if grid != want_grid {
                    found.push(format!("{id}: grid {grid:?}, expected {want_grid:?}"));
                }
                // The record says the grids read and written (docs/adr/0643 §4).
                let out = result.grids.iter().find(|g| g.role == "output");
                if !result.grids.iter().any(|g| g.role == "input")
                    || out.map(|g| (Some(g.affine), g.width, g.height)) != Some(want_grid)
                {
                    found.push(format!("{id}: the record's grids {:?}", result.grids));
                }
                let got = level0(bytes);
                let values = want["values"].as_array().unwrap();
                let exact = want["rule"] == "exact";
                if got.len() != values.len() {
                    found.push(format!("{id}: {} samples for {}", got.len(), values.len()));
                } else if let Some(k) = got.iter().zip(values).position(|(g, w)| {
                    let w = w.as_f64().unwrap_or(f64::NAN);
                    if g.is_nan() || w.is_nan() {
                        !(g.is_nan() && w.is_nan())
                    } else if exact {
                        *g != w
                    } else {
                        ulps(*g as f32, w as f32) > 1
                    }
                }) {
                    found.push(format!("{id}: sample {k}: {} for {}", got[k], values[k]));
                }
                if let Some(other) = c["sameAs"].as_str()
                    && written_by.get(other) != Some(bytes)
                {
                    found.push(format!("{id}: not the same bytes as {other}"));
                }
                written_by.insert(id.clone(), bytes.clone());
            }
        }
        report.push(format!(
            "{} {id}: {}",
            if found.is_empty() { "✓" } else { "✗" },
            c["title"].as_str().unwrap_or("")
        ));
        problems.extend(found);
    }
    println!("{}", report.join("\n"));
    assert!(report.len() >= 30, "{} cases", report.len());
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}
