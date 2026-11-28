//! The section building declared whole (docs/adr/8076; plan/ifc-uyum.md IFC-02): its künye with the postal box and its
//! use, the parcel's İmar durumu and Vaziyet planı, the author's fire classes and exit roles (`cad.arch.performance`,
//! docs/adr/8068) and the rooms' finishes. Every mandatory row of the catalog for a class the model writes is then
//! carried: the required check finds no gap. The storeys' structural slab levels and heights come from their slabs, the
//! rooms' wall covering from their walls' faces, true north from the project's system; the building's heights are not
//! asked, its storeys' differing (EK-6 Tablo 6.7: only where the height is constant).
//!
//! `KENTOS_WRITE_IFC_BUILDING=1` writes fixtures/arch/v1/ifc/kesit-binasi-tam.ifc for the independent readers.

use kentos_commands::{CallOp, Entity, OpeningForm, SiteEdge, SiteEdgeKind, SitePlan};
use kentos_domain::Document;
use kentos_ifc::building::{Building, Value};
use kentos_native_application::{ExecutionContext, arch_ifc, registry};
use serde_json::{Value as Json, json};

fn run(doc: &mut Document, name: &str, input: Json) -> Json {
    let answer = registry::run(&mut ExecutionContext::new(doc), name, CallOp::Execute, input).unwrap();
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

fn uids(doc: &Document, keep: impl Fn(&Entity) -> bool) -> Vec<String> {
    doc.entities_with_uids().filter(|(_, e)| keep(e)).map(|(u, _)| u.to_string()).collect()
}

/// The author's declaration with no document behind it.
fn author(value: &str) -> Json {
    json!({"value": value, "basis": {"kind": "author"}})
}

/// The section building as an author completes it for the permit's model.
pub(crate) fn declared() -> Document {
    let mut snap = crate::arch_ifc::kunye_snapshot();
    // The künye's postal box, the use and the units by hand (a drawing without a building program).
    for (name, value) in [("posta_kutusu", "Yok"), ("kullanim_amaci", "Konut"), ("bb_sayisi", "2")] {
        snap.settings.variables.push(kentos_commands::ProjectVariable {
            name: name.into(),
            label: name.into(),
            value: kentos_commands::VariableValue::Text(value.into()),
            ..Default::default()
        });
    }
    // İmar durumu: a 400 m² parcel, emsal 1.50, TAKS 0.40.
    snap.settings.arch.get_or_insert_with(Default::default).zoning =
        Some(kentos_commands::Zoning { parcel_area: 400.0, far: Some(1.5), coverage: Some(0.4), ..Default::default() });
    // Every room's finishes, as the room list gives them.
    for e in &mut snap.entities {
        if let Entity::Space(s) = e {
            s.space.finishes.floor = Some("Seramik".into());
            s.space.finishes.wall = Some("Plastik boya".into());
            s.space.finishes.ceiling = Some("Plastik boya".into());
        }
    }
    let mut doc = Document::from_snapshot_v2(snap).unwrap();
    // The parcel around the walls: 5 m to the road in the south, 3 m at the sides and the back (Vaziyet planı).
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for e in doc.entities() {
        if let Entity::Wall(w) = e {
            for p in &w.wall.pts {
                lo = [lo[0].min(p.x), lo[1].min(p.y)];
                hi = [hi[0].max(p.x), hi[1].max(p.y)];
            }
        }
    }
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let made = run(
        &mut doc,
        "cad.entities.create",
        json!({"layerId": "0", "objects": [{"geometry": {"kind": "polyline", "closed": true, "pts": [
            p(lo[0] - 3.0, lo[1] - 5.0), p(hi[0] + 3.0, lo[1] - 5.0), p(hi[0] + 3.0, hi[1] + 3.0), p(lo[0] - 3.0, hi[1] + 3.0)
        ]}}]}),
    );
    let parcel = made["output"]["created"][0].as_str().expect("the parcel").to_owned();
    let mut settings = doc.settings().clone();
    let arch = settings.arch.get_or_insert_with(Default::default);
    let edge = |kind| SiteEdge { kind, setback: None };
    arch.site = Some(SitePlan {
        parcel: Some(parcel),
        edges: vec![edge(SiteEdgeKind::Front), edge(SiteEdgeKind::Side), edge(SiteEdgeKind::Back), edge(SiteEdgeKind::Side)],
        datum: Some(1285.4),
        envelope: None,
        ..Default::default()
    });
    doc.set_settings(settings);
    // The author's fire classes (Binaların Yangından Korunması Hakkında Yönetmelik) and exit roles.
    let rated = |e: &Entity| matches!(e, Entity::Wall(_) | Entity::Slab(_) | Entity::Column(_) | Entity::Beam(_) | Entity::Stair(_));
    let structure = uids(&doc, rated);
    run(&mut doc, "cad.arch.performance", json!({"operation": {"kind": "objects", "uids": structure, "fireRating": author("REI 60")}}));
    let door = |e: &Entity| matches!(e, Entity::Opening(o) if o.opening.form == OpeningForm::Door);
    let doors = uids(&doc, door);
    run(&mut doc, "cad.arch.performance", json!({"operation": {"kind": "objects", "uids": doors, "fireRating": author("EI 30"),
        "exit": {"exit": false, "basis": {"kind": "author"}}}}));
    let stairs = uids(&doc, |e| matches!(e, Entity::Stair(_)));
    run(&mut doc, "cad.arch.performance", json!({"operation": {"kind": "objects", "uids": stairs,
        "exit": {"exit": true, "basis": {"kind": "author"}}}}));
    run(&mut doc, "cad.arch.performance", json!({"operation": {"kind": "building", "fireProtectionClass": author("Konut, düşük tehlike")}}));
    doc
}

fn prop(psets: &[kentos_ifc::building::Pset], set: &str, key: &str) -> Option<Value> {
    psets.iter().find(|p| p.name == set)?.props.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

fn quantity(qtos: &[kentos_ifc::building::Qto], set: &str, key: &str) -> Option<Value> {
    qtos.iter().find(|q| q.name == set)?.quantities.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

fn length(v: Option<Value>) -> f64 {
    match v {
        Some(Value::Length(x)) | Some(Value::PositiveLength(x)) => x,
        other => panic!("no length: {other:?}"),
    }
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn model() -> Building {
    let (b, _) = arch_ifc::building(&declared(), "2026-11-17T12:00:00", "KentOS Mimari");
    b
}

#[test]
fn the_declared_section_building_carries_every_mandatory_field() {
    let b = model();
    let gaps = kentos_ifc::required::gaps(&b);
    let named = |ids: &[Option<u128>]| -> Vec<String> {
        b.spaces.iter().filter(|s| ids.contains(&s.id)).map(|s| s.name.clone()).collect()
    };
    assert!(gaps.is_empty(), "{:#?}", gaps.iter().map(|g| (&g.class, &g.path, named(&g.missing), g.of)).collect::<Vec<_>>());
    // The independent readers' file (docs/adr/8076 §9).
    if std::env::var_os("KENTOS_WRITE_IFC_BUILDING").is_some() {
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/arch/v1/ifc/kesit-binasi-tam.ifc");
        std::fs::write(out, kentos_ifc::building::write(&b)).expect("the file kept");
    }
}

#[test]
fn the_storeys_levels_and_heights_are_their_slabs_structures() {
    let b = model();
    let set = "Pset_BuildingStoreyCommon";
    let qto = "Qto_BuildingStoreyBaseQuantities";
    // Zemin kat on its ground slab (kaplama 2, şap 5, ısı 5, su 1 cm over 15 cm concrete): −0.13; 1. kat on Ara kat
    // (kaplama 2, şap 5): +2.93; under the terrace (kaplama 3, su 1, şap 5, ısı 8 cm over 15 cm): its concrete 5.83 to
    // 5.68.
    let ground = &b.storeys[0];
    assert!(near(length(prop(&ground.psets, set, "ElevationOfSSLRelative")), -0.13));
    assert!(near(length(quantity(&ground.qtos, qto, "GrossHeight")), 3.06));
    assert!(near(length(quantity(&ground.qtos, qto, "NetHeight")), 2.91));
    let first = &b.storeys[1];
    assert!(near(length(prop(&first.psets, set, "ElevationOfSSLRelative")), 2.93));
    assert!(near(length(quantity(&first.qtos, qto, "GrossHeight")), 2.9));
    assert!(near(length(quantity(&first.qtos, qto, "NetHeight")), 2.75));
    // Their heights differ: the building has no constant height, and neither is written nor asked.
    let building = "Qto_BuildingBaseQuantities";
    assert_eq!(quantity(&b.building.qtos, building, "Height"), None);
    assert_eq!(quantity(&b.building.qtos, building, "EavesHeight"), None);
}

#[test]
fn the_site_the_address_the_rooms_and_the_north() {
    let b = model();
    // EK-6 Tablo 6.55: the site is the parcel, its planned area the parcel's.
    assert_eq!(b.site.object_type.as_deref(), Some("Parsel"));
    assert_eq!(prop(&b.site.psets, "Pset_SiteCommon", "TotalArea"), Some(Value::Area(400.0)));
    // Tablo 6.7: the künye's postal box, as the author wrote it.
    assert_eq!(prop(&b.building.psets, "Pset_Address", "PostalBox"), Some(Value::Label("Yok".into())));
    // Tablo 6.58: each room's walls show it their 2 cm plaster.
    for s in &b.spaces {
        assert!(near(length(prop(&s.psets, "Pset_SpaceCoveringRequirements", "WallCoveringThickness")), 0.02), "{}", s.name);
    }
    // EK-7 Tablo 7.8: true north from TUREF / TM33's meridian through the walls, a unit vector.
    let [x, y] = b.true_north.expect("true north");
    assert!(near(x * x + y * y, 1.0) && y > 0.99, "{x} {y}");
    assert_eq!(b.north_why, None);
}
