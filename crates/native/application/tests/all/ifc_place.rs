//! The building's place on the map (docs/adr/8072 §3; IFC-05): the section building drawn in a local system the
//! project fits from three control points onto TUREF / TM36 (fixtures/arch/v1/ifc/yerel-sistem.json, written by
//! scripts/fixtures/location_cases.py from a synthetic survey, no KentOS code). The IFC names the base system and
//! carries the local plane as IFC 4.3's scaled map conversion: turned, the model's metre scaled east and north, the
//! heights not; the file's zero at the project's ±0,00 kotu, none assumed when it is not known. The road's alignments
//! stand in the same place. `KENTOS_WRITE_IFC_LOCATION=1` writes fixtures/arch/v1/ifc/yerel-sistem.ifc for
//! scripts/fixtures/curved_ifc_check.py's location part, which reads it with IfcOpenShell.

use kentos_commands::{
    CallOp, CrsBase, CrsDefinition, CrsPlane, CrsSystem, DocumentSnapshotV2, HeightKind, HeightReference,
    LocalDefinition, SitePlan,
};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, arch_ifc, ifc_system, registry};
use kentos_project::definition_form::{FitRow, PlaneKind, fit_plane};
use serde_json::{Value, json};

const CASE: &str = include_str!("../../../../../fixtures/arch/v1/ifc/yerel-sistem.json");

fn case() -> Value {
    serde_json::from_str(CASE).unwrap()
}

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap()
}

/// The section building in the case's local system: its plane fitted from the control points as the Özel koordinat
/// sistemi window fits it (then given to `plane`), Antalya heights, ±0.00 at `datum`.
fn placed(plane: impl Fn(CrsPlane) -> CrsPlane, datum: Option<f64>) -> Document {
    let c = case();
    let rows: Vec<FitRow> = c["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            let typed: Vec<String> =
                p["typed"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_owned()).collect();
            typed.try_into().unwrap()
        })
        .collect();
    let fit = fit_plane(&rows, PlaneKind::Similarity).expect("three control points fit a similarity");
    let snap: DocumentSnapshotV2 =
        kentos_kcad::decode(include_bytes!("../../../../../fixtures/arch/v1/section-building.kcad")).unwrap();
    let mut doc = Document::from_snapshot_v2(snap).unwrap();
    let mut settings = doc.settings().clone();
    settings.srid = 0;
    settings.custom_crs = Some(CrsDefinition {
        name: "Şantiye".into(),
        system: CrsSystem::Local(LocalDefinition {
            base: CrsBase { srid: Some(5256), definition: None },
            plane: plane(fit.plane),
        }),
    });
    settings.heights = Some(HeightReference {
        kind: HeightKind::Orthometric,
        vertical_srid: Some(5775),
        geoid: None,
    });
    settings.arch.as_mut().expect("the building's settings").site =
        datum.map(|d| SitePlan { datum: Some(d), ..Default::default() });
    doc.set_settings(settings);
    doc
}

/// The export's warnings for `doc`, checked and not written.
fn warnings(doc: &mut Document, name: &str) -> Vec<Value> {
    let path = std::env::temp_dir().join(format!("kentos-yer-{}-{name}.ifc", std::process::id()));
    let answer = registry::run(
        &mut ExecutionContext::new(doc),
        "cad.arch.ifc",
        CallOp::Validate,
        json!({"path": path.display().to_string(), "stamp": "2026-11-16T12:00:00"}),
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{answer}");
    answer["warnings"].as_array().cloned().unwrap_or_default()
}

/// The fitted plane is the independent one; the file names TUREF / TM36 in full with Antalya heights, its conversion
/// the fitted origin, turn and scale with ±0.00's height; each control point carried by the written values lands on
/// its survey within the fit's own residual; EK-9's sixth question says the turn; the road's file takes the same
/// plane with its zero at the heights' 0; nothing is said lost.
#[test]
fn a_local_model_is_turned_and_scaled_onto_its_base() {
    let c = case();
    let (metres, relative, degrees) =
        (f(&c["tolerance"]["metres"]), f(&c["tolerance"]["relative"]), f(&c["tolerance"]["degrees"]));
    let mut doc = placed(|p| p, Some(f(&c["datum"])));
    let Some(CrsSystem::Local(local)) = doc.settings().custom_crs.as_ref().map(|d| &d.system) else {
        panic!("a local system")
    };
    let CrsPlane::Similarity { east, north, rotation, scale } = local.plane else {
        panic!("a similarity")
    };
    let want = &c["plane"];
    assert!((east - f(&want["east"])).abs() <= metres && (north - f(&want["north"])).abs() <= metres);
    assert!((rotation - f(&want["rotation"])).abs() <= degrees);
    assert!((scale - f(&want["scale"])).abs() <= relative * scale);
    let (b, _) = arch_ifc::building(&doc, "2026-11-16T12:00:00", "KentOS Mimari");
    let crs = b.crs.as_ref().expect("the base system");
    assert_eq!(
        (crs.name.as_str(), crs.geodetic_datum.as_deref(), crs.map_zone.as_deref(), crs.vertical_datum.as_deref()),
        ("EPSG:5256", Some("TUREF"), Some("36"), Some("Antalya height (EPSG:5775)"))
    );
    assert_eq!((crs.origin, crs.factors), ([east, north, f(&c["datum"])], [scale, scale]));
    for p in c["points"].as_array().unwrap() {
        let (x, y) = (f(&p["local"][0]), f(&p["local"][1]));
        let [ct, st] = crs.x_axis;
        let e = crs.origin[0] + crs.factors[0] * ct * x - crs.factors[1] * st * y;
        let n = crs.origin[1] + crs.factors[0] * st * x + crs.factors[1] * ct * y;
        let (de, dn) = (e - f(&p["map"][0]), n - f(&p["map"][1]));
        assert!(
            (de - f(&p["residual"][0])).abs() <= metres && (dn - f(&p["residual"][1])).abs() <= metres,
            "{}: {de} {dn} against {}",
            p["name"],
            p["residual"]
        );
    }
    let text = kentos_ifc::building::write(&b);
    assert!(text.contains("IFCPROJECTEDCRS('EPSG:5256','TUREF / TM36 (EPSG:5256)','TUREF','Antalya height (EPSG:5775)'"));
    assert_eq!(text.matches("IFCMAPCONVERSIONSCALED(").count(), 1);
    let form = arch_ifc::form_rows(&doc);
    let sixth = form.iter().find(|r| r.iter().any(|v| v.contains("EPSG:5256"))).expect("question 6 answered");
    assert!(sixth.iter().any(|v| v.contains("döndürülmüş") && v.contains("1285.40")), "{sixth:?}");
    // The road's alignments in the same place: the same plane, their heights the project's own.
    let road = ifc_system::of(&doc, Some(0.0)).expect("the road's system");
    assert_eq!((road.origin, road.x_axis, road.factors), ([east, north, 0.0], crs.x_axis, crs.factors));
    assert_eq!(warnings(&mut doc, "yerel"), Vec::<Value>::new());
    if std::env::var_os("KENTOS_WRITE_IFC_LOCATION").is_some() {
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/arch/v1/ifc/yerel-sistem.ifc");
        std::fs::write(out, &text).expect("the file kept");
    }
}

/// A ±0.00 of unknown height is no zero: the file says no system (no conversion, no 0 m), the export says why and
/// where to give it, EK-9's sixth question answers no with the reason; the road's file keeps its place, its heights
/// the project's own. A local system by an affine transform that shears is refused the same way, with the reason; a
/// drawing with no system at all says nothing at export (the form does).
#[test]
fn what_the_file_cannot_say_is_said_and_nothing_is_assumed() {
    let mut doc = placed(|p| p, None);
    let (b, _) = arch_ifc::building(&doc, "2026-11-16T12:00:00", "KentOS Mimari");
    assert!(b.crs.is_none());
    assert!(b.crs_why.as_deref().is_some_and(|w| w.contains("±0,00'ın mutlak kotu bilinmiyor")), "{:?}", b.crs_why);
    let text = kentos_ifc::building::write(&b);
    assert!(!text.contains("IFCMAPCONVERSION") && !text.contains("IFCPROJECTEDCRS"));
    let said = warnings(&mut doc, "kotsuz");
    assert_eq!(said.len(), 1, "{said:?}");
    assert_eq!(said[0]["code"], "export_losses");
    assert!(said[0]["message"].as_str().unwrap().contains("konum") && said[0]["message"].as_str().unwrap().contains("İmar durumu"));
    let form = arch_ifc::form_rows(&doc);
    assert!(form.iter().any(|r| r.iter().any(|v| v.contains("±0,00'ın mutlak kotu bilinmiyor"))));
    assert!(ifc_system::of(&doc, Some(0.0)).is_ok(), "the road's heights are the project's own");
    // A shear: no similarity is fitted in its place.
    let sheared = placed(|_| CrsPlane::Affine { a: 1.0, b: 0.01, c: 587_000.0, d: 0.0, e: 1.0, f: 4_400_000.0 }, Some(1285.4));
    let why = ifc_system::of(&sheared, Some(1285.4)).expect_err("a shear");
    assert!(why.contains("“Şantiye”") && why.contains("kayma"), "{why}");
    // No system at all: quiet at export.
    let snap: DocumentSnapshotV2 =
        kentos_kcad::decode(include_bytes!("../../../../../fixtures/arch/v1/section-building.kcad")).unwrap();
    let mut local = Document::from_snapshot_v2(snap).unwrap();
    assert!(!ifc_system::has_system(&local));
    assert_eq!(warnings(&mut local, "sistemsiz"), Vec::<Value>::new());
}
