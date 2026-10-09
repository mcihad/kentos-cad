//! Uzaklık ve maliyet in the desktop's window (docs/adr/0236) over the
//! valley with its cost raster, villages and road (fixtures/interaction/v1/
//! distance.kcad): En düşük maliyetli yol writes its paths with their
//! numbers on a new layer right above the cost raster's, undone whole;
//! Uzaklık yüzeyi from objects puts its raster right below the sources'
//! layer, from a raster right above it; Birikimli maliyet with the
//! elevation model and Maliyet koridoru say what they made.

use kentos_contracts::Entity;
use serde_json::{Value, json};

use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::distance_scenes::{from_road, layer, open_distance, paths};

fn value(app: &mut App, name: &str, v: Value) {
    let _ = app.update(Message::Processing(Event::Value(name.into(), v)));
}

fn run(app: &mut App) -> RunStatus {
    super::surface_tests::run(app)
}

fn valley() -> App {
    let (mut app, _) = App::boot(None);
    open_distance(&mut app);
    app
}

fn layer_named(app: &App, name: &str) -> (String, usize) {
    let layers = app.document.as_ref().expect("open").model.layers();
    let id = layers
        .leaves()
        .into_iter()
        .find(|l| l.name == name)
        .map(|l| l.id.clone())
        .unwrap_or_else(|| panic!("{name}"));
    let (_, at) = layers.place_of(&id).expect("placed");
    (id, at)
}

fn ok(app: &mut App) -> String {
    match run(app) {
        RunStatus::Ok { text, .. } => text,
        s => panic!("{s:?}"),
    }
}

fn window(app: &mut App, tool: &'static str, values: &[(&str, Value)]) {
    let _ = app.update(Message::Run(tool));
    for (name, v) in values {
        value(app, name, v.clone());
    }
}

#[test]
fn the_paths_go_right_above_the_cost_raster_in_one_step() {
    let mut app = valley();
    window(&mut app, "processing.run.distance.path", &paths());
    assert_eq!(ok(&mut app), "2 yol yazıldı.");
    let (made, at) = layer_named(&app, "En düşük maliyetli yol");
    let (_, cost) = layer_named(&app, "Maliyet");
    assert_eq!(at + 1, cost);
    let doc = &mut app.document.as_mut().expect("open").model;
    let lines: Vec<_> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Polyline(p) if p.base.layer_id == made => Some(p),
            _ => None,
        })
        .collect();
    assert_eq!(lines.len(), 2);
    // From Köy A's cell to Köy B's and Köy C's, numbered by the destinations' order.
    let near = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() <= 0.8 && (a.1 - b.1).abs() <= 0.8;
    let ends = [(487_904.0, 4_420_480.0), (487_600.0, 4_420_560.0)];
    for (k, p) in lines.iter().enumerate() {
        let a = &p.base.attrs;
        assert_eq!(
            a.get("Yol").map(String::as_str),
            Some((k + 1).to_string().as_str())
        );
        assert_eq!(a.get("Kaynak").map(String::as_str), Some("1"));
        for name in ["Maliyet", "Uzunluk", "Yüzey uzunluğu", "En büyük eğim"] {
            assert!(a.contains_key(name), "{name}: {a:?}");
        }
        let grade: f64 = a["En büyük eğim"].parse().expect("a number");
        assert!(grade <= 30.0, "{grade}");
        let surface: f64 = a["Yüzey uzunluğu"].parse().expect("a number");
        let plan: f64 = a["Uzunluk"].parse().expect("a number");
        assert!(surface > plan, "{surface} {plan}");
        let first = p.pts.first().expect("a vertex");
        let last = p.pts.last().expect("a vertex");
        assert!(
            near((first.x, first.y), (487_296.0, 4_420_150.0)),
            "{first:?}"
        );
        assert!(near((last.x, last.y), ends[k]), "{last:?}");
    }
    assert_eq!(doc.undo().as_deref(), Some("En düşük maliyetli yol"));
    assert!(
        doc.layers()
            .leaves()
            .iter()
            .all(|l| l.name != "En düşük maliyetli yol")
    );
}

#[test]
fn the_distance_from_objects_goes_below_them_and_from_a_raster_above_it() {
    let mut app = valley();
    window(&mut app, "processing.run.distance.euclidean", &from_road());
    let text = ok(&mut app);
    assert!(
        text.starts_with("1 kaynaktan 480 × 324 hücrelik raster; “"),
        "{text}"
    );
    let (made, at) = layer_named(&app, "Uzaklık");
    let (_, road) = layer_named(&app, "Mevcut yol");
    assert_eq!(at, road + 1);
    let doc = &app.document.as_ref().expect("open").model;
    assert!(doc.entities().any(|e| matches!(e, Entity::Raster(r)
        if r.base.layer_id == made && r.raster.style.ramp.as_deref() == Some("Viridis"))));
    let _ = app.update(Message::Processing(Event::Close));
    // From the elevation model's cells: every valued cell a source, the empty corner the distance to them.
    let mut app = valley();
    let out = crate::files_testing::scratch("uzk-dem").join("dem-uzaklik.tif");
    window(
        &mut app,
        "processing.run.distance.euclidean",
        &[
            ("from", json!("raster")),
            ("input", layer("dem")),
            ("output", json!(out.to_string_lossy())),
        ],
    );
    let text = ok(&mut app);
    assert!(text.contains(" kaynak hücre; en uzak hücre "), "{text}");
    let (_, at) = layer_named(&app, "Uzaklık");
    let (_, model) = layer_named(&app, "Yükseklik modeli");
    assert_eq!(at + 1, model);
}

#[test]
fn the_cost_with_the_surface_and_the_corridor_say_what_they_made() {
    let mut app = valley();
    let dir = crate::files_testing::scratch("uzk-maliyet");
    window(
        &mut app,
        "processing.run.distance.cost",
        &[
            ("input", layer("maliyet")),
            ("sources", layer("baslangic")),
            ("useSurface", json!(true)),
            ("surface", layer("dem")),
            ("slope", json!(30)),
            ("output", json!(dir.join("maliyet.tif").to_string_lossy())),
        ],
    );
    let text = ok(&mut app);
    assert!(
        text.contains(" 1 kaynak hücre; en büyük birikimli maliyet "),
        "{text}"
    );
    // The lake, the empty corner and the cells no climb of at most 30 % reaches.
    assert!(text.contains(" Değersiz "), "{text}");
    let _ = app.update(Message::Processing(Event::Close));
    window(
        &mut app,
        "processing.run.distance.corridor",
        &[
            ("input", layer("maliyet")),
            ("sources", layer("baslangic")),
            ("targets", layer("ikinci")),
            ("threshold", json!("percent")),
            ("percent", json!(5)),
            ("output", json!(dir.join("koridor.tif").to_string_lossy())),
        ],
    );
    let text = ok(&mut app);
    assert!(text.contains(" En ucuz yolun maliyeti "), "{text}");
    assert!(text.contains("; koridorda "), "{text}");
    // Yükseklik modeliyle on without a model: refused.
    let _ = app.update(Message::Processing(Event::Close));
    window(
        &mut app,
        "processing.run.distance.cost",
        &[
            ("input", layer("maliyet")),
            ("sources", layer("baslangic")),
            ("useSurface", json!(true)),
            ("surface", json!({ "scope": "layer", "layerId": "yol" })),
        ],
    );
    match run(&mut app) {
        RunStatus::Error(e) => assert!(e.contains("Yükseklik modelini seçin"), "{e}"),
        s => panic!("{s:?}"),
    }
}
