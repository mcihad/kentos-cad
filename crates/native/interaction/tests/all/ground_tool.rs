//! Arazi zemini (`tool.siteGround`, docs/adr/8174 §5): a click on a survey point makes the site's natural ground of its
//! layer's points (Tür T: its graded ground). The natural ground gives ±0.00 when the site has none, the mean of its
//! heights at the building's corners (PAİY 12/4); with both grounds and the parcel the garden's earthwork is said, out
//! of the building's footprint.

use crate::common::{Bench, E, N};
use kentos_commands::{CadWork, CallOp, DrawingUnit, SiteGround, Workspace};
use kentos_domain::NewLayer;
use kentos_native_application::{ExecutionContext, registry};
use kentos_interaction::Level;
use serde_json::json;

/// The natural ground's height at (x, y) east and north of (E, N): rising 10 cm a metre east.
fn slope(x: f64) -> f64 {
    1000.0 + 0.1 * x
}

/// The graded ground: flat.
const GRADED: f64 = 1001.0;

/// Zemin kat 3 m high; 25 cm walls whose outer faces make a 12 × 9 m outline, a partition at x 5. Survey points every 4
/// m on the layer Arazi over the slope round it (x −10…22, y −10…18), the graded ground's on Tesviye (2 m off them,
/// flat), the parcel a 24 × 21 m outline round the building (x −6…18, y −6…15), the site's. The tool started.
fn site() -> (Bench, String, String) {
    let mut b = Bench::new("line");
    b.session.exit();
    let mut s = b.doc.settings().clone();
    s.srid = 0;
    s.workspace = Some(Workspace::Cad);
    s.cad_work = Some(CadWork::Architecture);
    s.drawing_unit = Some(DrawingUnit::M);
    s.arch = Some(
        serde_json::from_value(json!({"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0}]}))
            .unwrap(),
    );
    b.doc.set_settings(s);
    let p = |x: f64, y: f64| json!({"x": E + x, "y": N + y});
    let answer = registry::run(
        &mut ExecutionContext::new(&mut b.doc),
        "cad.walls.create",
        CallOp::Execute,
        json!({"layerId": "cizim", "walls": [
            {"pts": [p(0.125, 0.125), p(11.875, 0.125), p(11.875, 8.875), p(0.125, 8.875), p(0.125, 0.125)],
             "width": 0.25, "justify": "center", "arch": {"level": "zemin"}},
            {"pts": [p(5.0, 0.25), p(5.0, 8.75)], "width": 0.1, "justify": "center", "arch": {"level": "zemin"}}
        ]}),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    let natural = b.doc.add_layer(NewLayer::layer("Arazi"), None, false).unwrap();
    let graded = b.doc.add_layer(NewLayer::layer("Tesviye"), None, false).unwrap();
    for i in 0..9 {
        for j in 0..8 {
            let (x, y) = (-10.0 + 4.0 * f64::from(i), -10.0 + 4.0 * f64::from(j));
            b.add_point_z(&natural, [x, y], slope(x));
        }
    }
    for i in 0..8 {
        for j in 0..8 {
            b.add_point_z(&graded, [-8.0 + 4.0 * f64::from(i), -8.0 + 4.0 * f64::from(j)], GRADED);
        }
    }
    let parcel = b.add_path("cizim", &[[-6.0, -6.0], [18.0, -6.0], [18.0, 15.0], [-6.0, 15.0]], true);
    let uid = b.doc.uid(parcel).unwrap().to_string();
    let mut s = b.doc.settings().clone();
    s.arch.as_mut().unwrap().site.get_or_insert_with(Default::default).parcel = Some(uid);
    b.doc.set_settings(s);
    b.level = Some("zemin".into());
    b.start("siteGround");
    (b, natural, graded)
}

fn site_plan(b: &Bench) -> kentos_commands::SitePlan {
    b.doc.settings().arch.as_ref().and_then(|a| a.site.clone()).unwrap_or_default()
}

#[test]
fn a_click_on_a_survey_point_makes_the_natural_ground_and_plus_minus_zero_from_the_corners() {
    let (mut b, natural, _) = site();
    assert_eq!(b.session.tool_id(), "siteGround");
    let before = b.log.len();
    b.click(-10.0, -10.0);
    let site = site_plan(&b);
    assert_eq!(site.natural_ground, Some(SiteGround::of_layer(&natural)));
    assert_eq!(site.graded_ground, None);
    // The building's corners at x 0 and 12: 1000 and 1001.2, their mean.
    assert_eq!(site.datum, Some(1000.6));
    assert_eq!(
        b.said(before),
        vec![
            (Level::Success, "Tabii zemin kuruldu: 112 üçgen, kot 999,00–1002,20 m."),
            (Level::Info, "±0,00 = 1000,60 m: bina köşelerinin tabii zemin kotlarının ortalaması (PAİY 12/4)."),
        ],
        "no earthwork without the graded ground"
    );
    // The site's own: an architecture project keeps no road settings (docs/adr/8174 §3.1).
    assert_eq!(b.doc.settings().road, None);
    assert_eq!(b.session.tool_id(), "siteGround", "the tool stays for the graded ground");
}

#[test]
fn the_graded_ground_says_the_garden_s_earthwork() {
    let (mut b, natural, graded) = site();
    b.click(-10.0, -10.0);
    assert!(b.type_text("T"));
    let before = b.log.len();
    b.click(-8.0, -8.0);
    let site = site_plan(&b);
    assert_eq!(site.graded_ground, Some(SiteGround::of_layer(&graded)));
    assert_eq!(site.datum, Some(1000.6), "±0.00 stays");
    // The graded ground over the natural one is 1 − 0.1·x: fill west of x 10, cut east of it, over the parcel (24 × 21)
    // out of the building (12 × 9): fill 21·12.8 − 9·5 = 223.8, cut 21·3.2 − 9·0.2 = 65.4, 504 − 108 = 396 m².
    assert_eq!(
        b.said(before),
        vec![
            (Level::Success, "Tesviye zemin kuruldu: 98 üçgen, kot 1001,00 m."),
            (Level::Info, "Bahçe hafriyatı, parselde bina oturumu dışında: kazı 65,40 m³, dolgu 223,80 m³ (396,00 m²)."),
        ]
    );
    assert_eq!(b.doc.settings().road, None);
    // Made again, a ground takes its place; the natural ground's layer is not the graded ground's.
    b.click(-8.0, -8.0);
    assert_eq!(site_plan(&b).graded_ground, Some(SiteGround::of_layer(&graded)));
    let settings = b.doc.settings().clone();
    b.click(-10.0, -10.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.last_text(),
        Some("Bu katman tabii zeminin katmanı: tabii ve tesviye zemin ayrı noktalardan kurulur, hafriyat ikisinin farkıdır.")
    );
    assert_eq!(*b.doc.settings(), settings);
    assert_eq!(site_plan(&b).natural_ground, Some(SiteGround::of_layer(&natural)));
}

#[test]
fn too_few_points_or_none_under_the_pointer_say_so_and_change_nothing() {
    let (mut b, _, _) = site();
    let few = b.doc.add_layer(NewLayer::layer("Az"), None, false).unwrap();
    b.add_point_z(&few, [30.0, 30.0], 1000.0);
    b.add_point_z(&few, [34.0, 30.0], 1000.0);
    let settings = b.doc.settings().clone();
    b.click(30.0, 30.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.last_text(), Some("Bu katmanın noktalarından tabii zemin kurulamadı: kotlu en az üç nokta gerekir."));
    b.click(40.0, 40.0);
    assert_eq!(b.last_text(), Some("Burada kotlu nokta yok: arazinin ölçü noktalarından birine tıklayın."));
    assert_eq!(*b.doc.settings(), settings);
}

/// The site with both grounds made (±0.00 at 1000.60 from the corners) and Zemin kotu started, layer keys on.
fn levels() -> Bench {
    let (mut b, _, _) = site();
    b.click(-10.0, -10.0);
    assert!(b.type_text("T"));
    b.click(-8.0, -8.0);
    b.session.exit();
    b.memory.layer_keys = true;
    b.start("groundLevel");
    b
}

/// The objects past the first `before` slots, by their kind's name, text and the mark's attribute.
fn written(b: &Bench, before: usize) -> Vec<(&'static str, String, String)> {
    let mut out: Vec<_> = b
        .doc
        .entities()
        .skip(before)
        .map(|e| {
            let (kind, text) = match e {
                kentos_commands::Entity::Circle(_) => ("circle", String::new()),
                kentos_commands::Entity::Line(_) => ("line", String::new()),
                kentos_commands::Entity::Text(t) => ("text", t.text.clone()),
                _ => ("other", String::new()),
            };
            (kind, text, e.base().attrs.get("Zemin kotu").cloned().unwrap_or_default())
        })
        .collect();
    out.sort();
    out
}

#[test]
fn zemin_kotu_writes_tzk_over_tsk_with_their_differences_in_one_step() {
    let mut b = levels();
    assert_eq!(b.session.tool_id(), "groundLevel");
    let before = b.doc.entities().count();
    // Inside the building at x 6: the natural ground at 1000.60, ±0.00 itself; the graded ground 40 cm over it.
    b.click(6.0, 4.5);
    assert_eq!(b.last_text(), Some("Zemin kotu yazıldı: tzk 1000.60 (±0.00), tsk 1001.00 (+0.40)."));
    assert_eq!(
        written(&b, before),
        vec![
            ("circle", String::new(), "simge".into()),
            ("line", String::new(), "simge".into()),
            ("line", String::new(), "simge".into()),
            ("text", "tsk 1001.00 (+0.40)".into(), "tsk".into()),
            ("text", "tzk 1000.60 (±0.00)".into(), "tzk".into()),
        ]
    );
    // On the symbols' layer; tzk over tsk, both right of the symbol.
    let texts: Vec<_> = b
        .doc
        .entities()
        .skip(before)
        .filter_map(|e| match e {
            kentos_commands::Entity::Text(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    for t in &texts {
        let name = b.doc.layers().get(&t.base.layer_id).map(|l| l.name.clone());
        assert_eq!(name.as_deref(), Some("MM-XXXX-SEMBOL"));
        assert!(t.p.x > E + 6.0, "right of the symbol");
    }
    let y = |w: &str| texts.iter().find(|t| t.text.starts_with(w)).map(|t| t.p.y).unwrap();
    assert!(y("tzk") > N + 4.5 && y("tsk") < N + 4.5, "tzk over the place, tsk under it");
    // One undo step takes the mark away; the tool stays for the next.
    b.doc.undo();
    assert_eq!(b.doc.entities().count(), before);
    assert_eq!(b.session.tool_id(), "groundLevel");
}

#[test]
fn zemin_kotu_says_why_nothing_is_written_off_the_grounds_or_without_them() {
    let mut b = levels();
    let before = b.doc.entities().count();
    b.click(60.0, 60.0);
    assert_eq!(b.last_text(), Some("Bu nokta zeminlerin dışında: ölçü noktalarının kapladığı yere tıklayın."));
    assert_eq!(b.doc.entities().count(), before);
    let (mut b, _, _) = site();
    b.session.exit();
    b.start("groundLevel");
    let before = b.doc.entities().count();
    b.click(6.0, 4.5);
    assert_eq!(b.last_text(), Some("Vaziyetin zemini yok: tabii ve tesviye zemini Arazi zemini ile kurun."));
    assert_eq!(b.doc.entities().count(), before);
}
