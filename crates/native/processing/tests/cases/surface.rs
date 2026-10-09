//! Yüzey analizi's shared cases (fixtures/processing/v1/surface.json,
//! written by scripts/fixtures/surface_processing_cases.py; docs/adr/0231):
//! each runs a surface tool as the desktop does, on the drawing's reading
//! copy apart from the drawing, with files that read the cases' rasters and
//! keep what the run writes. Besides the cases' usual checks, a written
//! raster's level 0 is the independent surface reference's
//! (fixtures/terrain/v1/cases.json: a 32-bit sample within one unit in the
//! last place, a byte exact) and the contour lines are the contour
//! reference's (fixtures/contours/v1/cases.json), bit for bit. The web plays
//! the same file (apps/web/src/processing/surface.test.ts).

use kentos_contracts::{CloudSource, RasterFields};
use kentos_formats::raster::source::{Reader, open_bytes};
use kentos_processing::files::{
    Beside, CloudRead, RASTER_READER_BUDGET, RasterOpen, Sink, with_extension,
};

use super::*;

fn surface_fixture(rel: &str) -> Vec<u8> {
    let path = folder().join(rel);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The cases' rasters by their file's name, and what a run writes, by its name.
struct FixtureFiles {
    rasters: BTreeMap<String, Vec<u8>>,
    written: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
}

struct Kept {
    name: String,
    bytes: Vec<u8>,
    written: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
}

impl Sink for Kept {
    fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    fn patch(&mut self, at: u64, bytes: &[u8]) -> Result<(), String> {
        let at = at as usize;
        self.bytes[at..at + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }

    fn finish(self: Box<Self>) -> Result<(), String> {
        self.written
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(self.name, self.bytes);
        Ok(())
    }
}

impl Files for FixtureFiles {
    fn open_cloud(&self, _: &CloudSource) -> Result<Box<dyn CloudRead + '_>, String> {
        Err("no clouds here".into())
    }

    fn output_path(
        &self,
        asked: &str,
        beside: Option<Beside<'_>>,
        suffix: &str,
        ext: &str,
    ) -> Result<String, String> {
        let asked = asked.trim();
        if !asked.is_empty() {
            return Ok(with_extension(asked, ext));
        }
        Ok(format!("{}{suffix}{ext}", beside.ok_or("no place")?.stem()))
    }

    fn create(&self, path: &str) -> Result<Box<dyn Sink + '_>, String> {
        Ok(Box::new(Kept {
            name: path.to_owned(),
            bytes: Vec::new(),
            written: self.written.clone(),
        }))
    }

    fn copc(&self, _: &str, _: &str, _: &mut dyn Feedback, _: f64, _: f64) -> Result<(), String> {
        Err("no clouds here".into())
    }

    fn remove(&self, _: &str) {}

    fn open_raster(&self, raster: &RasterFields) -> Result<RasterOpen<'_>, String> {
        let name = raster.file.as_deref().ok_or("a linked raster")?;
        let bytes = self
            .rasters
            .get(name)
            .ok_or_else(|| format!("{name}: not among the cases' rasters"))?;
        let reader = open_bytes(bytes, None, RASTER_READER_BUDGET).map_err(|e| e.0)?;
        Ok(RasterOpen {
            reader,
            block: Box::new(move |n| {
                Ok(bytes[n.offset as usize..(n.offset + n.len) as usize].to_vec())
            }),
            jpeg: Box::new(|_| Err("no JPEG here".into())),
        })
    }
}

/// The units in the last place between two 32-bit values (NaN only with NaN).
fn ulps(a: f32, b: f32) -> u32 {
    if a.is_nan() || b.is_nan() {
        return if a.is_nan() && b.is_nan() {
            0
        } else {
            u32::MAX
        };
    }
    let key = |v: f32| {
        let i = v.to_bits() as i32;
        if i < 0 { i32::MIN - i } else { i }
    };
    key(a).abs_diff(key(b))
}

/// A written GeoTIFF's level 0 whole: its samples, bands interleaved.
fn level0(bytes: &[u8]) -> Vec<f64> {
    let mut reader: Reader =
        open_bytes(bytes, None, RASTER_READER_BUDGET).expect("the result reads");
    let (w, h) = (reader.levels[0].width, reader.levels[0].height);
    for need in reader.needs(0, 0, 0, w, h) {
        let (a, b) = (need.offset as usize, (need.offset + need.len) as usize);
        reader
            .put_block(&need, &bytes[a..b])
            .expect("a block of the result");
    }
    let r = reader.region(0, 0, 0, w, h).expect("the level whole");
    (0..(w * h * r.bands) as usize)
        .map(|k| r.samples.get(k))
        .collect()
}

/// The contour reference's lines as the tool's polylines on `layer`.
fn contour_objects(k: usize, layer: &str) -> Vec<Value> {
    let doc: Value = serde_json::from_slice(&surface_fixture("../../contours/v1/cases.json"))
        .expect("the contours read");
    let case = &doc["cases"][k];
    case["lines"]
        .as_array()
        .expect("the lines whole")
        .iter()
        .map(|l| {
            let main = l["index"].as_bool().unwrap();
            let n = l["pts"].as_array().unwrap().len();
            let mut o = json!({
                "kind": "polyline",
                "layerId": layer,
                "attrs": { "Kot": case["texts"][l["k"].to_string()], "Tür": if main { "Ana" } else { "Ara" } },
                "pts": l["pts"].as_array().unwrap().iter().map(|p| json!({ "x": p[0], "y": p[1] })).collect::<Vec<_>>(),
                "zs": vec![l["value"].clone(); n],
            });
            if main {
                o["lineWeight"] = json!(0.35);
            }
            o
        })
        .collect()
}

/// Plays a file of raster cases (surface.json, interpolation.json,
/// raster-ops.json, raster-vector.json): besides
/// the usual checks, each new layer's place (`layerAbove`, `layerBelow`) and
/// each written raster's level 0 against its reference (`rasterOf`: the
/// surface reference; `interpolationOf`: the interpolation reference, a
/// second band its `error`; `rasterOpsOf`: the raster operations' reference,
/// by its case's rule; `rasterVectorOf`: Rasterleştir's reference, exact;
/// `hydrologyOf`: the hydrology reference, `distanceOf`: the distance and
/// cost reference, by their case's rule).
fn raster_cases(name: &str, least: usize) {
    let file = case_file(name);
    let tol = file["tolerance"].as_f64().expect("a tolerance");
    let terrain: Value = serde_json::from_slice(&surface_fixture("../../terrain/v1/cases.json"))
        .expect("the surface reference reads");
    let interpolation: Value =
        serde_json::from_slice(&surface_fixture("../../interpolation/v1/cases.json"))
            .expect("the interpolation reference reads");
    let raster_ops: Value =
        serde_json::from_slice(&surface_fixture("../../raster-ops/v1/cases.json"))
            .expect("the raster operations' reference reads");
    let raster_vector: Value =
        serde_json::from_slice(&surface_fixture("../../raster-vector/v1/cases.json"))
            .expect("the raster and vector reference reads");
    let hydrology: Value =
        serde_json::from_slice(&surface_fixture("../../hydrology/v1/cases.json"))
            .expect("the hydrology reference reads");
    let distance: Value = serde_json::from_slice(&surface_fixture("../../distance/v1/cases.json"))
        .expect("the distance reference reads");
    let rasters: BTreeMap<String, Vec<u8>> = file["rasters"]
        .as_object()
        .map(|m| {
            m.iter()
                .map(|(name, path)| (name.clone(), surface_fixture(path.as_str().unwrap())))
                .collect()
        })
        .unwrap_or_default();
    let registry = Registry::builtin();
    let mut problems = Vec::new();
    let mut report = Vec::new();
    for c in file["cases"].as_array().expect("cases") {
        let written = Arc::new(Mutex::new(BTreeMap::new()));
        let files: Arc<dyn Files> = Arc::new(FixtureFiles {
            rasters: rasters.clone(),
            written: written.clone(),
        });
        let mut case = c.clone();
        // The contour lines are the contour reference's, in its order.
        if let Some(k) = c["expect"]["contoursOf"].as_u64() {
            let layer = c["expect"]["layers"][0]["id"]
                .as_str()
                .expect("the lines' layer");
            case["expect"]["added"] = Value::Array(contour_objects(k as usize, layer));
        }
        let seen = play_with(&case, &registry, true, &json!({}), Some(files));
        // Each new layer right above (below) the layer it names (its group, the place before (after) it).
        let mut placed = Vec::new();
        for (key, step) in [("layerAbove", 1usize), ("layerBelow", 0)] {
            for (new, at) in c["expect"][key].as_object().cloned().unwrap_or_default() {
                let layers = seen.host.doc.layers();
                let (a, b) = (layers.place_of(&new), layers.place_of(at.as_str().unwrap()));
                let next = |(g, i): &(Option<String>, usize), (h, j): &(Option<String>, usize)| {
                    g == h && if step == 1 { i + 1 == *j } else { j + 1 == *i }
                };
                if !matches!((&a, &b), (Some(x), Some(y)) if next(x, y)) {
                    placed.push(format!(
                        "{}: {key}: “{new}” is at {a:?}, “{at}” at {b:?}",
                        c["id"]
                    ));
                }
            }
        }
        let mut found = check(&case, seen, tol);
        found.extend(placed);
        let id = c["id"].as_str().unwrap_or("?");
        let kept = written.lock().unwrap_or_else(PoisonError::into_inner);
        let mut wanted: Vec<(String, Value, &str)> = Vec::new();
        for (key, kind) in [
            ("rasterOf", "terrain"),
            ("interpolationOf", "interpolation"),
            ("rasterOpsOf", "ops"),
            ("rasterVectorOf", "vector"),
            ("hydrologyOf", "hydrology"),
            ("distanceOf", "distance"),
        ] {
            for (name, of) in c["expect"][key].as_object().cloned().unwrap_or_default() {
                wanted.push((name, of, kind));
            }
        }
        if kept.len() != wanted.len() {
            found.push(format!(
                "{id}: written {:?}, expected {:?}",
                kept.keys().collect::<Vec<_>>(),
                wanted.iter().map(|w| &w.0).collect::<Vec<_>>()
            ));
        }
        // Each written raster's level 0 is its reference's case.
        for (name, of, kind) in &wanted {
            let Some(bytes) = kept.get(name) else {
                found.push(format!("{id}: “{name}” was not written"));
                continue;
            };
            let got = level0(bytes);
            if *kind == "vector" {
                // Rasterleştir's reference: every sample exact.
                let reference = raster_vector["cases"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|t| t["name"] == *of)
                    .unwrap_or_else(|| panic!("{id}: no raster and vector case {of}"));
                let values = reference["expect"]["raster"]["values"]
                    .as_array()
                    .expect("the values whole");
                if got.len() != values.len() {
                    found.push(format!("{id}: {} samples for {}", got.len(), values.len()));
                    continue;
                }
                let off = got.iter().zip(values).position(|(g, w)| {
                    let w = w.as_f64().unwrap_or(f64::NAN);
                    if g.is_nan() || w.is_nan() {
                        !(g.is_nan() && w.is_nan())
                    } else {
                        *g != w
                    }
                });
                if let Some(k) = off {
                    found.push(format!(
                        "{id}: “{name}” sample {k}: {} for {}",
                        got[k], values[k]
                    ));
                }
                continue;
            }
            if matches!(*kind, "ops" | "hydrology" | "distance") {
                // The raster operations', the hydrology or the distance reference: its samples by its case's rule.
                let file = match *kind {
                    "ops" => &raster_ops,
                    "hydrology" => &hydrology,
                    _ => &distance,
                };
                let reference = file["cases"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|t| t["name"] == *of)
                    .unwrap_or_else(|| panic!("{id}: no {kind} case {of}"));
                let want = &reference["expect"]["raster"];
                let exact = want["rule"] == "exact";
                let values = want["values"].as_array().expect("the values whole");
                if got.len() != values.len() {
                    found.push(format!("{id}: {} samples for {}", got.len(), values.len()));
                    continue;
                }
                let off = got.iter().zip(values).position(|(g, w)| {
                    let w = w.as_f64().unwrap_or(f64::NAN);
                    if g.is_nan() || w.is_nan() {
                        return !(g.is_nan() && w.is_nan());
                    }
                    if exact {
                        *g != w
                    } else {
                        ulps(*g as f32, w as f32) > 1
                    }
                });
                if let Some(k) = off {
                    found.push(format!(
                        "{id}: “{name}” sample {k}: {} for {}",
                        got[k], values[k]
                    ));
                }
                continue;
            }
            let (bands, byte, want): (usize, bool, Vec<Value>) = if *kind == "terrain" {
                let reference = terrain["cases"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|t| t["dem"] == "tepe" && t["name"] == *of)
                    .unwrap_or_else(|| panic!("{id}: no surface case {of}"));
                (
                    1,
                    reference["sample"] == "u8",
                    reference["values"]
                        .as_array()
                        .expect("the values whole")
                        .clone(),
                )
            } else {
                let reference = interpolation["cases"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|t| t["name"] == *of)
                    .unwrap_or_else(|| panic!("{id}: no interpolation case {of}"));
                let values = reference["expect"]["values"]
                    .as_array()
                    .expect("the values whole");
                match reference["expect"]["error"].as_array() {
                    // Two bands interleaved: the values, then the standard errors.
                    Some(errors) if got.len() == 2 * values.len() => (
                        2,
                        false,
                        values
                            .iter()
                            .zip(errors)
                            .flat_map(|(v, e)| [v.clone(), e.clone()])
                            .collect(),
                    ),
                    _ => (1, false, values.clone()),
                }
            };
            let _ = bands;
            if got.len() != want.len() {
                found.push(format!("{id}: {} samples for {}", got.len(), want.len()));
                continue;
            }
            let off = got.iter().zip(&want).position(|(g, w)| {
                if byte {
                    *g != w.as_f64().unwrap()
                } else {
                    ulps(*g as f32, w.as_f64().map_or(f32::NAN, |v| v as f32)) > 1
                }
            });
            if let Some(k) = off {
                found.push(format!(
                    "{id}: “{name}” sample {k}: {} for {}",
                    got[k], want[k]
                ));
            }
        }
        report.push(format!(
            "{} {}: {}",
            if found.is_empty() { "✓" } else { "✗" },
            id,
            c["title"].as_str().unwrap_or("")
        ));
        problems.extend(found);
    }
    println!("{}", report.join("\n"));
    assert!(report.len() >= least, "{} cases", report.len());
    assert!(
        problems.is_empty(),
        "\n{}\n\n{}",
        report.join("\n"),
        problems.join("\n")
    );
}

#[test]
fn the_surface_cases_do_what_they_say() {
    raster_cases("surface.json", 15);
}

#[test]
fn the_raster_operations_cases_do_what_they_say() {
    raster_cases("raster-ops.json", 15);
}

/// Raster ve vektör and Taranmış harita's shared cases (fixtures/processing/v1/raster-vector.json,
/// scripts/fixtures/raster_vector_processing_cases.py; docs/adr/0234).
#[test]
fn the_raster_and_vector_cases_do_what_they_say() {
    raster_cases("raster-vector.json", 13);
}

/// Hidroloji's shared cases (fixtures/processing/v1/hydrology.json,
/// scripts/fixtures/hydrology_processing_cases.py; docs/adr/0235).
#[test]
fn the_hydrology_cases_do_what_they_say() {
    raster_cases("hydrology.json", 20);
}

/// Uzaklık ve maliyet's shared cases (fixtures/processing/v1/distance.json,
/// scripts/fixtures/distance_processing_cases.py; docs/adr/0236).
#[test]
fn the_distance_cases_do_what_they_say() {
    raster_cases("distance.json", 18);
}

/// The names an expression field offers on rasters (docs/adr/0233 §3): the
/// input's fields are every raster's name and its bands after the first,
/// each on one object (`inputFields` of raster-ops.json; the web reads the same).
#[test]
fn a_raster_input_offers_its_bands_names() {
    let file = case_file("raster-ops.json");
    let registry = Registry::builtin();
    let cases = file["inputFields"].as_array().expect("inputFields");
    assert!(!cases.is_empty());
    for f in cases {
        let id = f["id"].as_str().unwrap_or("?");
        let host = TestHost {
            doc: load(f["document"].as_str().expect("the drawing")),
            selection: Vec::new(),
            view: None,
        };
        let tool = registry
            .tool(f["tool"].as_str().expect("the tool"))
            .expect("a built-in tool");
        let values = with_values(&tool, &host.doc, Some(&f["values"]), &json!({}));
        let mut got = Runner::new()
            .describe_inputs(&tool, &values, &host)
            .remove("input")
            .expect("the input's summary")
            .fields;
        got.sort();
        let want: Vec<(String, usize)> = f["fields"]
            .as_array()
            .expect("fields")
            .iter()
            .map(|p| {
                (
                    p[0].as_str().expect("a name").to_owned(),
                    p[1].as_u64().expect("a count") as usize,
                )
            })
            .collect();
        assert_eq!(got, want, "{id}");
    }
}

/// İnterpolasyon and Yoğunluk's shared cases (fixtures/processing/v1/interpolation.json,
/// scripts/fixtures/interpolation_processing_cases.py; docs/adr/0232).
#[test]
fn the_interpolation_cases_do_what_they_say() {
    raster_cases("interpolation.json", 16);
}

/// A host without files (a test, the headless server): a raster tool refuses saying why.
#[test]
fn a_raster_tool_refuses_where_there_are_no_files() {
    let file = case_file("surface.json");
    let c = &file["cases"][0];
    let registry = Registry::builtin();
    let s = play_with(c, &registry, true, &json!({}), None);
    let got = observed(&s);
    assert_eq!(got["status"], "error", "{got}");
    assert_eq!(
        got["message"],
        kentos_processing::files::NO_RASTER_FILES,
        "{got}"
    );
}
