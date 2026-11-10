//! Vaziyet planı (M32, docs/adr/8032; Planlı Alanlar İmar Yönetmeliği 23): a 15 × 6 m building of four 3 m storeys on a
//! 30 × 18 m parcel drawn round it, 4 m from its south edge, 10 m from its east, 8 m from its north and 5 m from its
//! west. Parsel takes the parcel by its edge; the south edge clicked once is the front (5 m), the north twice the back
//! (3 m); Enter writes the site plan and draws the setback line: x −2…22, y 1…11. The front is 1 m short; 450 m² of
//! garden is 15 trees. A fifth storey widens the side and back gardens by half a metre and the line follows.

use kentos_launch::Module;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::drive;
use crate::recovery::Recovery;
use crate::settings::Settings;
use crate::viewport::Event;

fn command(app: &mut App, name: &str, input: Value) -> Value {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

fn click(app: &mut App, x: f64, y: f64) {
    let [sx, sy] = app
        .viewport
        .camera
        .world_to_screen(kentos_interaction::Vec2::new(x, y));
    let at = iced::Point::new(sx as f32, sy as f32);
    let _ = app.update(Message::Viewport(Event::Moved(at)));
    let _ = app.update(Message::Viewport(Event::Pressed(at)));
    let _ = app.update(Message::Viewport(Event::Released(at)));
}

fn storeys(n: usize) -> Value {
    let names = ["Zemin kat", "1. kat", "2. kat", "3. kat", "4. kat"];
    json!(
        (0..n)
            .map(|i| json!({ "id": format!("k{i}"), "name": names[i], "elevation": 3.0 * i as f64, "height": 3.0 }))
            .collect::<Vec<_>>()
    )
}

/// The envelope's box: x0, y0, x1, y1.
fn bounds(pts: &[kentos_commands::Vec2]) -> [f64; 4] {
    pts.iter().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |b, p| [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)],
    )
}

#[test]
fn a_parcel_s_garden_distances_draw_the_setback_line_and_follow_the_storeys() {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(
        Some(Module::Architecture),
        None,
        Settings::memory(),
        Recovery::off(),
    );
    app.db_store = Box::leak(Box::new(
        kentos_postgis::connections::ConnectionStore::open(None),
    ));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let task = app.update(Message::Run("file.new"));
    drive(&mut app, task);
    command(
        &mut app,
        "cad.settings.set",
        json!({ "arch": { "levels": storeys(4) } }),
    );
    let layer = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .layers()
        .active()
        .to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let wall = |a: (f64, f64), b: (f64, f64), w: f64| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": w, "arch": {"level": "k0"}});
    command(
        &mut app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [
            wall((0.125, 0.125), (14.875, 0.125), 0.25), wall((14.875, 0.125), (14.875, 5.875), 0.25),
            wall((14.875, 5.875), (0.125, 5.875), 0.25), wall((0.125, 5.875), (0.125, 0.125), 0.25),
            wall((5.0, 0.125), (5.0, 5.875), 0.15), wall((10.0, 0.125), (10.0, 5.875), 0.15)
        ]}),
    );
    let room = |x: f64| json!({"seed": p(x, 3.0), "arch": {"level": "k0"}});
    command(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [room(2.5), room(7.5), room(12.5)]}),
    );
    // The parcel: a closed line round the building, south edge first, anticlockwise.
    command(
        &mut app,
        "cad.entities.create",
        json!({"layerId": layer, "objects": [{"geometry": {"kind": "polyline", "closed": true,
            "pts": [p(-5.0, -4.0), p(25.0, -4.0), p(25.0, 14.0), p(-5.0, 14.0)]}}]}),
    );
    // The real window's drawing (fixtures/interaction/real/vaziyet-plani.txt): the building and its parcel.
    if let Some(out) = std::env::var_os("KENTOS_PAIY_OUT") {
        let doc = &app.document.as_ref().expect("a drawing").model;
        let bytes = kentos_kcad::encode(&doc.to_snapshot_v2()).expect("encodes");
        std::fs::write(std::path::Path::new(&out).join("parsel.kcad"), bytes).expect("written");
    }
    app.viewport.camera.set_size(1100.0, 600.0);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: -7.0,
            min_y: -6.0,
            max_x: 27.0,
            max_y: 16.0,
        },
        24.0,
    );
    // ±0.00's elevation typed first (Proje ayarları › Mimari › İmar durumu): the site plan holds it alone.
    command(
        &mut app,
        "cad.settings.set",
        json!({ "arch": { "levels": storeys(4), "site": { "datum": 1285.4 } } }),
    );
    let _ = app.update(Message::Run("tool.siteParcel"));
    assert_eq!(app.session.tool_id(), "siteParcel");
    click(&mut app, 10.0, -4.0); // the parcel, by its south edge
    click(&mut app, 10.0, -4.0); // south: Yan → Ön
    click(&mut app, 10.0, 14.0); // north: Yan → Ön
    click(&mut app, 10.0, 14.0); // north: Ön → Arka
    let _ = app.submit_line("");
    let said = crate::files_testing::last_said(&app);
    assert!(
        said.starts_with(
            "Parsel 540,00 m², kat adedi 4, ağaç 15; yapı yaklaşma sınırı çizildi, bahçe mesafeleri ve ±0,00 ölçülendi."
        ),
        "{said}"
    );
    // The site plan's marks (docs/adr/8032 §2): a distance dimension for each edge, ±0.00, its elevation.
    let marks = |app: &App| {
        let doc = &app.document.as_ref().expect("a drawing").model;
        let mut out: Vec<(String, String)> = doc
            .entities()
            .filter(|e| {
                e.base()
                    .attrs
                    .contains_key(kentos_interaction::parcel_tool::MARK_ATTR)
            })
            .map(|e| {
                let what = match e {
                    kentos_commands::Entity::Text(t) => t.text.clone(),
                    _ => e.kind().to_owned(),
                };
                (
                    e.base().attrs[kentos_interaction::parcel_tool::MARK_ATTR].clone(),
                    what,
                )
            })
            .collect();
        out.sort();
        out
    };
    let first = marks(&app);
    let kinds: Vec<&str> = first.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(
        kinds,
        [
            "Bahçe mesafesi",
            "Bahçe mesafesi",
            "Bahçe mesafesi",
            "Bahçe mesafesi",
            "±0,00",
            "±0,00"
        ],
        "{first:?}"
    );
    assert!(
        first.iter().any(|(_, w)| w == "±0.00 = +1285.40"),
        "{first:?}"
    );
    assert!(
        said.contains("Bahçe mesafesi dar: 1. kenar (Ön bahçe) 4,00 m < 5,00 m"),
        "{said}"
    );
    let doc = &app.document.as_ref().expect("a drawing").model;
    let arch = doc.settings().arch.clone().expect("Mimari's settings");
    let site = arch.site.clone().expect("the site plan");
    let kinds: Vec<kentos_commands::SiteEdgeKind> = site.edges.iter().map(|e| e.kind).collect();
    use kentos_commands::SiteEdgeKind::{Back, Front, Side};
    assert_eq!(
        kinds,
        [Front, Side, Back],
        "the last side garden is the default"
    );
    assert!((arch.zoning.expect("the parcel's area").parcel_area - 540.0).abs() < 1e-9);
    let envelope = |app: &App| {
        let doc = &app.document.as_ref().expect("a drawing").model;
        let site = doc
            .settings()
            .arch
            .as_ref()
            .and_then(|a| a.site.clone())
            .expect("the site plan");
        let uid =
            kentos_domain::Uuid::parse_str(site.envelope.as_deref().expect("its setback line"))
                .expect("an id");
        match doc.get(doc.slot_of(uid).expect("drawn")) {
            Some(kentos_commands::Entity::Polyline(l)) => {
                assert!(l.closed, "closed");
                assert_eq!(
                    l.base.look.line_type,
                    Some(kentos_commands::LineType::Dashed),
                    "dashed"
                );
                l.pts.clone()
            }
            other => panic!("a closed line: {other:?}"),
        }
    };
    let near = |a: [f64; 4], b: [f64; 4]| a.iter().zip(&b).all(|(x, y)| (x - y).abs() < 1e-6);
    let b = bounds(&envelope(&app));
    assert!(near(b, [-2.0, 1.0, 22.0, 11.0]), "{b:?}");
    let r = doc.site_report().expect("read");
    assert_eq!((r.floors, r.trees), (4, 15));
    let drawn: Vec<f64> = r
        .edges
        .iter()
        .map(|e| e.drawn.unwrap_or(f64::NAN))
        .collect();
    assert!(
        drawn
            .iter()
            .zip([4.0, 10.0, 8.0, 5.0])
            .all(|(a, b)| (a - b).abs() < 1e-6),
        "{drawn:?}"
    );
    // The Emsal ve TAKS table says the gardens: the front 1 m short, in the warning colour.
    let table = doc.zoning_table();
    let row = |words: &str| {
        table
            .rows
            .iter()
            .position(|r| r[0] == words)
            .unwrap_or_else(|| panic!("{words}: {:?}", table.rows))
    };
    assert_eq!(
        table.rows[row("Ön bahçe (m)")],
        ["Ön bahçe (m)", "5.00", "4.00", "-1.00"]
    );
    assert_eq!(
        table.rows[row("Yan bahçe (m)")],
        ["Yan bahçe (m)", "3.00", "5.00", "2.00"]
    );
    assert_eq!(
        table.rows[row("Arka bahçe (m)")],
        ["Arka bahçe (m)", "3.00", "8.00", "5.00"]
    );
    assert_eq!(table.rows[row("Ağaç (adet, 23/6)")][1], "15");
    let front = row("Ön bahçe (m)") as u32;
    assert!(
        table.styles.iter().any(|s| s.range.row == front
            && s.color.as_deref() == Some(kentos_domain::zoning::OVER_COLOR)),
        "{:?}",
        table.styles
    );
    assert!(
        crate::zoning_badge::words(doc, true).is_some_and(|(_, over)| over),
        "the badge red"
    );
    // Run again: the parcel taken up with its kinds, the marks drawn anew, not twice; ±0.00's elevation kept.
    let _ = app.update(Message::Run("tool.siteParcel"));
    let _ = app.submit_line("");
    assert_eq!(marks(&app).len(), first.len(), "{:?}", marks(&app));
    let arch = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .settings()
        .arch
        .clone()
        .expect("Mimari's settings");
    assert_eq!(
        arch.site.as_ref().and_then(|s| s.datum),
        Some(1285.4),
        "±0.00 kept"
    );
    // A fifth storey: 15 m, five storeys: the side and back gardens 3.5 m; the line follows at once.
    let mut a = serde_json::to_value(&arch).expect("the settings");
    a["levels"] = storeys(5);
    command(&mut app, "cad.settings.set", json!({ "arch": a }));
    let b = bounds(&envelope(&app));
    assert!(near(b, [-1.5, 1.0, 21.5, 10.5]), "{b:?}");
    assert_eq!(
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .site_report()
            .expect("read")
            .floors,
        5
    );
}
