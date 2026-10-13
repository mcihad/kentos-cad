//! The information card's texts for many objects (docs/adr/0225 §7): 10 000
//! parcels, a template of four placeholders (three attributes and the area),
//! ≤ 200 ms in release on this machine; the web's is
//! `apps/web/scripts/perf/attachments.test.ts`.

use std::collections::BTreeMap;

use kentos_contracts::{Entity, EntityBase, PathEntity, Vec2};
use kentos_native_application::expression_unit::Unit;
use kentos_native_application::geometry::shape;
use kentos_native_application::info_templates::compile_template;

fn parcel(i: usize) -> Entity {
    let (x, y) = ((i % 100) as f64 * 30.0, (i / 100) as f64 * 25.0);
    let p = |dx: f64, dy: f64| Vec2 {
        x: 487_000.0 + x + dx,
        y: 4_420_000.0 + y + dy,
    };
    Entity::Polygon(PathEntity {
        base: EntityBase {
            look: Default::default(),
            id: i as u32 + 1,
            layer_id: "parsel".to_owned(),
            color: None,
            attrs: BTreeMap::from([
                ("Ada".to_owned(), format!("{}", 100 + i / 100)),
                ("Parsel".to_owned(), format!("{}", 1 + i % 100)),
                (
                    "Mahalle".to_owned(),
                    if i.is_multiple_of(2) {
                        "Kızılay"
                    } else {
                        "Bahçelievler"
                    }
                    .to_owned(),
                ),
            ]),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        pts: vec![p(0.0, 0.0), p(28.0, 0.0), p(28.0, 23.0), p(0.0, 23.0)],
        bulges: None,
        holes: None,
        zs: None,
        ms: None,
        parts: None,
        closed: false,
        widths: None,
    })
}

#[test]
#[ignore = "timing; run with --ignored --nocapture in release"]
fn timing() {
    let list: Vec<Entity> = (0..10_000).map(parcel).collect();
    let refs: Vec<&Entity> = list.iter().collect();
    let shapes: Vec<_> = list.iter().map(shape).collect();
    let template = compile_template(
        "Parsel {Ada}/{Parsel}, {Mahalle}: {yuvarla($alan, 2)} m²",
        Unit::METRE,
    )
    .expect("compiles");
    let name = |_: &str| "Parsel".to_owned();
    let mut best = f64::INFINITY;
    let mut texts = Vec::new();
    for _ in 0..5 {
        let start = std::time::Instant::now();
        texts = template.texts(&refs, &name, &|i| shapes.get(i), &|_| None);
        best = best.min(start.elapsed().as_secs_f64() * 1e3);
    }
    assert_eq!(texts[0], "Parsel 100/1, Kızılay: 644 m²");
    assert_eq!(texts[9_999], "Parsel 199/100, Bahçelievler: 644 m²");
    println!("Bilgi kartı: 10 000 nesnenin metni {best:.2} ms (dört yer tutucu)");
    assert!(best <= 200.0, "{best} ms");
}
