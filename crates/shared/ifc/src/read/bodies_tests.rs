//! The engine's bodies against another reader's (docs/adr/8061 §7 "Kendi dosyalarımız", "Büyük koordinat"): every
//! body IfcOpenShell makes of the files KentOS writes (fixtures/ifc/v1/okuma/, openings cut, world coordinates) the
//! engine makes too, its box and volume within a tolerance that comes from the file and the numbers, not from the
//! result: the file's precision (`IfcGeometricRepresentationContext.Precision`, in metres) plus the float32 rounding of
//! the engine's positions about their origin (`Body::rounding`, half a float32 step at the largest of them); the
//! volume's tolerance that times the body's area. A curved face is two readers' chords apart, not the same polygon: its share is the chord's sagitta.
//!
//! ```text
//! cargo test -p kentos-ifc --features geometry read::bodies_tests -- --nocapture
//! ```

use super::bodies::{Body, volume_and_area};
use super::reference_tests::reference;
use super::tests::{WRITTEN, never, read_ok, samples};

/// A body's box.
pub(super) fn bbox(b: &Body) -> [[f64; 3]; 2] {
    let mut out = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for p in &b.positions {
        for k in 0..3 {
            out[0][k] = out[0][k].min(p[k]);
            out[1][k] = out[1][k].max(p[k]);
        }
    }
    out
}

/// The float32 rounding of a body's corners, as the engine stored them (`Body::rounding`).
pub(super) fn f32_error(b: &Body) -> f64 {
    b.rounding
}

/// The bodies whose arcs the engine divides coarser than the file's precision (docs/adr/8061 §7, the trial's finding):
/// the curved building's arc walls, rooms, slabs and spiral stair. The engine does not divide a profile's arcs finer
/// than its default density whatever the quality asked; KentOS's exact volumes (the file's own Qto, from the section
/// model) put its mesh 0.3–0.9 % under, the other reader's (1 mm deflection) 0.01 %. Listed, not tolerated: a body
/// leaving this list, or one joining it, fails the test.
const COARSE_ARCS: [&str; 8] = [
    "01alMXW75m080000000003",
    "01alMXW75m080000000008",
    "01alMXW75m08000000000B",
    "01alMXW75m08000000000G",
    "01alMXW75m08000000000H",
    "01alMXW75m08000000000I",
    "01alMXW75m08000000000J",
    "01alMXW75m08000000000K",
];

/// What a sample's bodies came to against the other reader's.
pub(super) struct BodyCheck {
    pub(super) made: usize,
    pub(super) theirs: usize,
    pub(super) not_shown: usize,
    pub(super) worst_box: f64,
    pub(super) worst_volume: f64,
    pub(super) coarse: usize,
    /// A body the other reader makes and the engine does not, or one past the tolerance (the listed coarse arcs
    /// aside), or a listed coarse arc within it now.
    pub(super) unexpected: Vec<String>,
}

/// The engine's bodies of `bytes` (read as `m`) against the other reader's `r`.
pub(super) fn body_check(m: &super::Model, bytes: &[u8], r: &serde_json::Value) -> BodyCheck {
    let precision = r["precision"].as_f64().unwrap_or(1e-5);
    let made = m.bodies(bytes, &never).expect("the bodies");
    let theirs = r["bodies"].as_object().cloned().unwrap_or_default();
    let mut out = BodyCheck {
        made: made.bodies.len(),
        theirs: theirs.len(),
        not_shown: made.not_shown.len(),
        worst_box: 0.0,
        worst_volume: 0.0,
        coarse: 0,
        unexpected: Vec::new(),
    };
    for (gid, t) in &theirs {
        let Some(ours) = made.bodies.iter().find(|b| &m.objects[b.object].global_id == gid) else {
            let why = made.not_shown.iter().find(|n| &m.objects[n.object].global_id == gid).map(|n| n.reason.clone());
            out.unexpected.push(format!("{gid}: the engine made no body ({why:?})"));
            continue;
        };
        let class = &m.objects[ours.object].class;
        // An alignment is a line, not a solid: the two readers draw it differently (docs/adr/8061 §11.4).
        if class == "IfcAlignment" {
            continue;
        }
        let tol = precision + f32_error(ours);
        let b = bbox(ours);
        let tb: Vec<f64> = t["box"].as_array().expect("box").iter().flat_map(|c| c.as_array().expect("corner").iter().filter_map(|v| v.as_f64())).collect();
        let box_diff = (0..6).map(|k| (b[k / 3][k % 3] - tb[k]).abs()).fold(0.0, f64::max);
        let (v, _) = volume_and_area(ours);
        let (tv, ta) = (t["volume"].as_f64().expect("volume"), t["area"].as_f64().expect("area"));
        let volume_diff = (v - tv).abs();
        let over = box_diff > tol || volume_diff > ta * tol;
        if COARSE_ARCS.contains(&gid.as_str()) {
            out.coarse += 1;
            if !over {
                out.unexpected.push(format!("{class} {gid} is within the tolerance now: take it off COARSE_ARCS"));
            }
            continue;
        }
        out.worst_box = out.worst_box.max(box_diff);
        out.worst_volume = out.worst_volume.max(volume_diff / tv.max(1e-12));
        if over {
            out.unexpected.push(format!(
                "{class} {gid}: box {box_diff:.2e} (tolerance {tol:.2e}), volume {v:.6} against {tv:.6} ({volume_diff:.2e}, tolerance {:.2e})",
                ta * tol
            ));
        }
    }
    out
}

#[test]
fn every_body_the_other_reader_makes_the_engine_makes_alike() {
    let (mut table, mut unexpected) = (Vec::new(), Vec::new());
    for (i, (name, bytes)) in samples().enumerate() {
        let c = body_check(&read_ok(bytes), bytes, &reference(i));
        table.push(format!(
            "{name}: {} bodies, the other reader {}; not shown {}; worst box {:.2e} m, worst volume {:.2e} relative; coarse arcs {}",
            c.made, c.theirs, c.not_shown, c.worst_box, c.worst_volume, c.coarse
        ));
        unexpected.extend(c.unexpected.into_iter().map(|u| format!("{name}: {u}")));
    }
    println!("{}", table.join("\n"));
    assert!(unexpected.is_empty(), "{}", unexpected.join("\n"));
}

/// The engine's bodies against the section model (docs/adr/8061 §7 "kesit modeline"): every body whose object carries
/// the net volume KentOS wrote from its section model (`Qto_…BaseQuantities.NetVolume`, worked out from the source, not
/// from a mesh) has that volume, within the file's precision and the float32 rounding times its area. The coarse arcs
/// are the engine's finding (above).
#[test]
fn every_body_has_the_volume_the_section_model_wrote() {
    let (mut compared, mut over) = (0, Vec::new());
    for (name, bytes) in WRITTEN {
        let m = read_ok(bytes);
        let made = m.bodies(bytes, &never).expect("the bodies");
        for b in &made.bodies {
            let o = &m.objects[b.object];
            if COARSE_ARCS.contains(&o.global_id.as_str()) {
                continue;
            }
            let sets = m.property_sets(bytes, b.object).expect("its sets");
            let Some(net) = sets
                .iter()
                .filter(|s| s.kind == super::SetKind::Quantities)
                .flat_map(|s| &s.values)
                .find(|v| v.name == "NetVolume")
                .and_then(|v| match v.value {
                    super::ValueKind::Number { value, .. } => Some(value),
                    _ => None,
                })
            else {
                continue;
            };
            let (v, a) = volume_and_area(b);
            let tol = a * (1e-5 + f32_error(b));
            compared += 1;
            if (v - net).abs() > tol {
                over.push(format!("{name}: {} #{} {:?}: mesh {v:.6}, section model {net:.6} (tolerance {tol:.2e})", o.class, o.step, o.name));
            }
        }
    }
    println!("{compared} bodies against the section model's net volume");
    assert!(compared > 100, "{compared}");
    assert!(over.is_empty(), "{}", over.join("\n"));
}

/// An object with a shape that has no body is listed with why: here only the grid, whose shape is its axes' lines.
#[test]
fn only_the_grid_s_lines_are_not_shown() {
    for (name, bytes) in WRITTEN {
        let m = read_ok(bytes);
        let made = m.bodies(bytes, &never).expect("the bodies");
        for n in &made.not_shown {
            assert_eq!(m.objects[n.object].class, "IfcGrid", "{name}: {} {}", m.objects[n.object].class, n.reason);
        }
    }
}

/// A building moved onto TUREF coordinates (587 312.345 E, 4 400 612.789 N, 1 285.40 m, as the location case's base):
/// every corner is where the same building's is plus the move, to the file's precision (docs/adr/8061 §7 "Büyük
/// koordinat"). The engine's float32 positions stay about each body's float64 origin.
#[test]
fn a_building_on_turef_coordinates_keeps_its_corners() {
    let (_, bytes) = WRITTEN[3];
    let text = String::from_utf8(bytes.to_vec()).expect("UTF-8");
    let shift = [587_312.345, 4_400_612.789, 1_285.4];
    let moved = text
        .replace("#13=IFCAXIS2PLACEMENT3D(#6,", "#13=IFCAXIS2PLACEMENT3D(#9999995,")
        .replace("ENDSEC;\nEND-ISO-10303-21;", "#9999995=IFCCARTESIANPOINT((587312.345,4400612.789,1285.4));\nENDSEC;\nEND-ISO-10303-21;");
    assert_ne!(moved, text, "the site's placement moved");
    let (a, b) = (read_ok(bytes), read_ok(moved.as_bytes()));
    let (here, there) = (a.bodies(bytes, &never).expect("bodies"), b.bodies(moved.as_bytes(), &never).expect("bodies"));
    assert_eq!(here.bodies.len(), there.bodies.len());
    let mut worst = 0.0f64;
    for (p, q) in here.bodies.iter().zip(&there.bodies) {
        assert_eq!(a.objects[p.object].global_id, b.objects[q.object].global_id);
        assert_eq!(p.positions.len(), q.positions.len());
        for (u, v) in p.positions.iter().zip(&q.positions) {
            for k in 0..3 {
                worst = worst.max((v[k] - (u[k] + shift[k])).abs());
            }
        }
    }
    println!("TUREF: worst corner {worst:.2e} m over {} bodies", here.bodies.len());
    assert!(worst <= 1e-5, "a corner {worst:.2e} m off, past the file's precision 1e-5");
}

#[test]
#[ignore = "a look at what the engine makes, by hand"]
fn look() {
    for (i, (name, bytes)) in WRITTEN.iter().enumerate() {
        if !matches!(*name, "blok-b" | "yol" | "egri-bina") {
            continue;
        }
        let r = reference(i);
        let m = read_ok(bytes);
        let made = m.bodies(bytes, &never).expect("the bodies");
        for n in &made.not_shown {
            println!("{name}: not shown {} #{} {:?}: {}", m.objects[n.object].class, m.objects[n.object].step, m.objects[n.object].name, n.reason);
        }
        if *name != "blok-b" {
            for b in &made.bodies {
                let o = &m.objects[b.object];
                println!("{name}: {} #{} {:?} box {:?} {} triangles; theirs {}", o.class, o.step, o.name, bbox(b), b.triangles.len(), r["bodies"].get(&o.global_id).map_or("—".to_owned(), |t| t["box"].to_string()));
            }
        }
    }
}
