//! MAP-02's acceptance through the real app (docs/adr/1230 §5): at E ≈ 500 000, N ≈ 4 400 000 m (TUREF / TM30) a
//! 2.001 × 1.999 m parcel and a 1 mm square are measured, then the view is zoomed and panned, the drawing reopened
//! with another render origin and saved and opened again as `.kcad`; every measure stays bit for bit, and Alan
//! hesapla's İçine tıkla, pressed at each view, writes the same ring. The reference is analytic: the decimal corners'
//! area (2.001 × 1.999 = 3.999999 m², 10⁻⁶ m²), within the corners' own float64 rounding (≤ 4.7·10⁻¹⁰ m a
//! coordinate at 4.4·10⁶ m, so ≤ 4·10⁻⁹ m² for the parcel's 8 m perimeter, 2·10⁻¹² m² for the square's 4 mm). A
//! single float32 at these coordinates steps by 0.5 m: a measure that had gone through one would be off by metres.
//! Test code only.

use iced::{Point, Rectangle, Size, Vector};
use kentos_commands::Entity;
use kentos_headless::{Method, measure_by};
use serde_json::json;

use crate::app::{App, Message};
use crate::files_testing::app_with_drawing;
use crate::tools_scenes::{E, N, Objects, click, open_edited, run};
use crate::viewport::Event;

/// The parcel's south-west corner and the square's, as offsets from the scenes' (E, N).
const PARCEL: [f64; 2] = [13_000.0005, -19_999.9995];
const SQUARE: [f64; 2] = [13_004.0, -19_998.0];

fn ground() -> Objects {
    let [x, y] = PARCEL;
    let mut o = Objects::new();
    o.path(
        "parsel",
        &[
            [x, y],
            [x + 2.001, y],
            [x + 2.001, y + 1.999],
            [x, y + 1.999],
        ],
        true,
    );
    let [x, y] = SQUARE;
    o.path(
        "parsel",
        &[
            [x, y],
            [x + 0.001, y],
            [x + 0.001, y + 0.001],
            [x, y + 0.001],
        ],
        true,
    );
    o
}

fn opened(origin: [f64; 2]) -> App {
    let mut app = app_with_drawing();
    let area = Rectangle::new(Point::ORIGIN, Size::new(1000.0, 800.0));
    let _ = app.update(Message::Viewport(Event::Resized(area)));
    open_edited(&mut app, ground(), |d| {
        d["settings"]["srid"] = json!(5254);
        // The ground's mean ellipsoidal height, for the ground measures (Proje ayarları › Ölçme).
        d["settings"]["survey"] = json!({ "groundHeight": 850.0 });
        d["origin"] = json!({ "x": origin[0], "y": origin[1] });
    });
    let _ = app.update(Message::Viewport(Event::Resized(area)));
    let _ = app.update(Message::Viewport(Event::Extents));
    app
}

/// Every polygon's persistent id and corners, in the drawing's order.
fn polygons(app: &App) -> Vec<(String, Vec<[f64; 2]>)> {
    let doc = &app.document.as_ref().expect("open").model;
    doc.entities_with_uids()
        .filter_map(|(uid, e)| match e {
            Entity::Polygon(p) => {
                Some((uid.to_string(), p.pts.iter().map(|v| [v.x, v.y]).collect()))
            }
            _ => None,
        })
        .collect()
}

/// The plane, ellipsoid and ground area and length of each polygon, as their bits.
fn measures(app: &App) -> Vec<[u64; 6]> {
    let doc = &app.document.as_ref().expect("open").model;
    polygons(app)
        .iter()
        .map(|(uid, _)| {
            let m = |method| measure_by(doc, uid, method).expect("measures");
            let (p, e, g) = (m(Method::Plane), m(Method::Ellipsoid), m(Method::Ground));
            let bits = |v: Option<f64>| v.expect("a value").to_bits();
            [
                bits(p.area),
                bits(p.length),
                bits(e.area),
                bits(e.length),
                bits(g.area),
                bits(g.length),
            ]
        })
        .collect()
}

/// Alan hesapla, İçine tıkla inside the parcel, Alan olarak çiz: the ring the tool measured, as a new polygon's.
fn measured_ring(app: &mut App) -> Vec<[f64; 2]> {
    let before = polygons(app).len();
    run(app, "tool.area");
    // İçine tıkla is the session's switch: pressed once, it stays on for the next runs.
    if !app.memory.area_inside {
        let _ = app.update(Message::PromptOption("I"));
    }
    assert!(app.memory.area_inside);
    click(app, [PARCEL[0] + 1.0, PARCEL[1] + 1.0]);
    let _ = app.update(Message::PromptOption("A"));
    run(app, "tool.cancel");
    let all = polygons(app);
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert_eq!(
        all.len(),
        before + 1,
        "Alan olarak çiz wrote one polygon: {said:?}"
    );
    let ring = all.last().expect("the new polygon").1.clone();
    // The measured one is taken away again: the drawing stays the two objects.
    run(app, "edit.undo");
    assert_eq!(polygons(app).len(), before);
    ring
}

#[test]
fn zoom_pan_render_origin_and_save_reopen_leave_every_measure_and_ring_bit_for_bit() {
    let mut app = opened([500_000.0, 4_400_000.0]);
    let origin_a = app
        .viewport
        .draw_origin(app.document.as_ref().expect("open"));
    let source = polygons(&app);
    let base = measures(&app);
    // The analytic reference: the decimal corners' area and perimeter.
    let doc = &app.document.as_ref().expect("open").model;
    let plane = |uid: &str| measure_by(doc, uid, Method::Plane).expect("measures");
    let parcel = plane(&source[0].0);
    let square = plane(&source[1].0);
    assert!(
        (parcel.area.expect("an area") - 3.999_999).abs() < 4e-9,
        "{parcel:?}"
    );
    assert!(
        (parcel.length.expect("a length") - 8.0).abs() < 4e-9,
        "{parcel:?}"
    );
    assert!(
        (square.area.expect("an area") - 1e-6).abs() < 2e-12,
        "{square:?}"
    );
    assert!(
        (square.length.expect("a length") - 0.004).abs() < 2e-9,
        "{square:?}"
    );
    let ring = measured_ring(&mut app);
    // İçine tıkla's ring is the parcel's corners themselves (a region's ring may start at another corner).
    let mut corners = source[0].1.clone();
    let mut got = ring.clone();
    corners.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    got.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    assert_eq!(got, corners, "the measured ring is the source's corners");

    // From the fitted view: zoomed about a corner and panned by a fraction of a pixel and by pixels. İçine tıkla
    // reads the visible line work, so its ring is taken where the parcel stays whole on the area; the measures at
    // every zoom, the deepest (9000 times the fitted view) and the farthest (1/77) too.
    let views = [
        (1.7, Vector::new(0.37, -0.21), true),
        (0.5, Vector::new(123.4, -56.7), true),
        (2.4, Vector::new(-3.3, 8.8), true),
        (9000.0, Vector::new(0.013, 0.029), false),
        (0.013, Vector::new(250.0, 180.0), false),
    ];
    for (factor, by, whole) in views {
        let _ = app.update(Message::Viewport(Event::Extents));
        let corner = crate::tools_scenes::at(&app, PARCEL);
        let _ = app.update(Message::Viewport(Event::Zoomed { factor, at: corner }));
        let _ = app.update(Message::Viewport(Event::Panned { by, at: corner }));
        assert_eq!(polygons(&app), source, "the view changes no corner");
        assert_eq!(measures(&app), base, "the view changes no measure");
        if whole {
            let _ = app.update(Message::Viewport(Event::CenterOn(
                kentos_geometry_core::vec2::Vec2::new(E + PARCEL[0] + 1.0, N + PARCEL[1] + 1.0),
            )));
            assert_eq!(
                measured_ring(&mut app),
                ring,
                "the view changes the measured ring"
            );
        }
    }

    // Another render origin: the same drawing anchored 7 km away.
    let mut moved = opened([493_000.0, 4_393_000.0]);
    let origin_b = moved
        .viewport
        .draw_origin(moved.document.as_ref().expect("open"));
    assert_ne!(origin_a, origin_b, "the scene's origin moved");
    let corners_of = |app: &App| {
        polygons(app)
            .into_iter()
            .map(|(_, r)| r)
            .collect::<Vec<_>>()
    };
    assert_eq!(corners_of(&moved), corners_of(&app));
    assert_eq!(
        measures(&moved),
        base,
        "the render origin changes no measure"
    );
    assert_eq!(measured_ring(&mut moved), ring);

    // Saved and opened again.
    let bytes = kentos_kcad::encode(&app.document.as_ref().expect("open").model.to_snapshot_v2())
        .expect("encodes");
    let back = kentos_kcad::decode(&bytes).expect("decodes");
    let doc = crate::document::Document::from_v2(back, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    assert_eq!(
        polygons(&app),
        source,
        "save and reopen keep every corner and id"
    );
    assert_eq!(measures(&app), base, "save and reopen keep every measure");
}
