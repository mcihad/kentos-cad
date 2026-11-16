//! The viewer on a large model (docs/adr/8073 §7): reading, the bodies, the viewer's input, the first scene and the
//! scene made again for a choice, timed. The model is `scripts/fixtures/ifc_large_case.py 700`'s (kept out of Git); not
//! run by default:
//!
//! ```text
//! KENTOS_IFC_LARGE=.run/ifc-buyuk/kesit-binasi-x700.ifc cargo test --release -p kentos-model-view-ui --test timing -- --ignored --nocapture
//! ```

use std::time::Instant;

use kentos_ifc::read::{self as ifc, Limits};
use kentos_model_view_ui::{Message, Viewer};

#[test]
#[ignore = "a measurement on a model kept out of Git"]
fn a_large_model_s_steps_are_timed() {
    let Ok(path) = std::env::var("KENTOS_IFC_LARGE") else {
        eprintln!("SKIPPED: KENTOS_IFC_LARGE names no file (a skipped measurement is not a result)");
        return;
    };
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(path);
    let never = || false;
    let t = Instant::now();
    let bytes = std::fs::read(&path).expect("the file");
    let model = ifc::read(&bytes, &Limits::default(), &never).expect("it reads");
    let read = t.elapsed();
    let t = Instant::now();
    let mut bodies = model.bodies(&bytes, &never).expect("the bodies");
    let openings = model.opening_bodies(&bytes, &never).expect("the openings");
    bodies.bodies.extend(openings.bodies);
    let made = t.elapsed();
    let t = Instant::now();
    let input = ifc::view::input("büyük", &model, &bodies);
    let adapted = t.elapsed();
    let triangles: usize = input.bodies.iter().map(|b| b.triangles.len()).sum();
    let mut viewer = Viewer::new("büyük");
    let t = Instant::now();
    viewer.load(input, Vec::new());
    let loaded = t.elapsed();
    let wall = viewer.view().and_then(|v| v.input().objects.iter().position(|o| o.class == "IfcWall")).expect("a wall");
    let t = Instant::now();
    let _ = viewer.update(Message::Choose(Some(wall)));
    let chosen = t.elapsed();
    let (shown, all) = viewer.view().expect("loaded").counts();
    println!(
        "{} nesne, {all} gövde ({shown} gösterilen), {triangles} üçgen, {} karo içeriği\n\
         okuma {read:.2?}, gövdeler {made:.2?}, girdi {adapted:.2?}, ilk sahne {loaded:.2?}, seçimle yeniden sahne {chosen:.2?}",
        model.objects.len(),
        viewer.scene().contents()
    );
}
