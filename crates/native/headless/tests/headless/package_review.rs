//! Gelen paketi incele (docs/adr/8104; plan/bim-plan.md BIM-18's first acceptance): KentOS's own package (the declared
//! section building published with its BIM part, TUREF / TM36) passes the administration's review on EPSG:5256; another
//! program's open model (buildingSMART's IFC 4.3 architecture sample) is opened alone; the package broken on purpose
//! (scripts/fixtures/review_packages.py: the model on TM33, the east wall without its fire rating, the ground floor's
//! plan from another revision) gives each finding in its layer, the fire rating's on the wall by its GlobalId; the other
//! broken packages give theirs (fixtures/bim/v1/paket/review_packages.json).

use std::path::PathBuf;

use kentos_commands::{CallEnvelope, CallOp, CommandResult, PackageReviewed};
use kentos_domain::Document;
use serde_json::{Value, json};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures")
        .canonicalize()
        .expect("the fixtures")
}

fn drawing() -> Document {
    let snap: kentos_commands::DocumentSnapshotV2 = kentos_kcad::decode(include_bytes!(
        "../../../../../fixtures/arch/v1/section-building.kcad"
    ))
    .unwrap();
    Document::from_snapshot_v2(snap).unwrap()
}

fn review(path: PathBuf, system: Option<u32>) -> PackageReviewed {
    let mut doc = drawing();
    let mut input = json!({"path": path.display().to_string()});
    if let Some(s) = system {
        input["system"] = json!(s);
    }
    let answer = kentos_headless::file_commands::call(
        &mut doc,
        CallEnvelope {
            command: "file.review.package".into(),
            version: None,
            op: CallOp::Execute,
            request_id: None,
            expected: None,
            relax_constraints: false,
            input,
        },
    );
    let CommandResult::Completed { output, .. } = &answer.result else {
        panic!("{answer:?}")
    };
    serde_json::from_value(output.clone()).expect("the review")
}

fn holds(r: &PackageReviewed) -> Vec<(&str, Option<bool>)> {
    r.layers.iter().map(|l| (l.key.as_str(), l.holds)).collect()
}

fn errors(r: &PackageReviewed) -> Vec<(&str, &str, Option<&str>)> {
    r.findings
        .iter()
        .filter(|f| f.error)
        .map(|f| (f.layer.as_str(), f.code.as_str(), f.file.as_deref()))
        .collect()
}

#[test]
fn kentos_s_own_package_passes_the_review_layer_by_layer() {
    let r = review(fixtures().join("bim/v1/paket/kentos"), Some(5256));
    assert!(errors(&r).is_empty(), "{:#?}", r.findings);
    assert_eq!(
        holds(&r),
        [
            ("paket", Some(true)),
            ("pafta", Some(true)),
            ("model", Some(true)),
            ("konum", Some(true)),
            ("form", Some(true))
        ]
    );
    assert_eq!(
        r.model.as_deref(),
        Some("260123_00_MM_XXXX_MD_XXX_NNN_01_000.ifc")
    );
    assert_eq!(r.rows.len(), 21);
    assert!(
        r.rows.iter().all(|row| row.agree != Some(false)),
        "{:#?}",
        r.rows
    );
}

#[test]
fn the_broken_package_gives_each_finding_in_its_layer_and_the_fire_rating_on_its_wall() {
    let r = review(fixtures().join("bim/v1/paket/bozuk"), Some(5256));
    let e = errors(&r);
    // The model on another zone: the place's.
    assert!(
        e.iter()
            .any(|(l, c, _)| *l == "konum" && *c == "review_place_zone"),
        "{e:#?}"
    );
    // The plan made from the drawing's revision 8: the sheets', on that sheet.
    assert!(
        e.iter().any(|(l, c, f)| *l == "pafta"
            && *c == "review_sheet_revision"
            && *f == Some("260123_00_MM_XXXX_PL_MMC_K00_01_000.pdf")),
        "{e:#?}"
    );
    assert_eq!(
        e.iter().filter(|(l, _, _)| *l == "pafta").count(),
        1,
        "{e:#?}"
    );
    // The wall without its fire rating: the model's, on the wall.
    let wall = "01alMXGa9m080000000002";
    let fire = r
        .findings
        .iter()
        .find(|f| f.layer == "model" && f.objects.iter().any(|g| g == wall))
        .unwrap_or_else(|| panic!("{:#?}", r.findings));
    assert!(
        fire.message.contains("FireRating") || fire.message.to_lowercase().contains("yangın"),
        "{}",
        fire.message
    );
    // The form still says Evet: the form's.
    assert!(
        e.iter()
            .any(|(l, c, _)| *l == "form" && *c == "review_form_disagrees"),
        "{e:#?}"
    );
    // Nothing of the package's own: its list holds every file as it is.
    assert!(!e.iter().any(|(l, _, _)| *l == "paket"), "{e:#?}");
    assert_eq!(
        holds(&r),
        [
            ("paket", Some(true)),
            ("pafta", Some(false)),
            ("model", Some(false)),
            ("konum", Some(false)),
            ("form", Some(false))
        ]
    );
}

#[test]
fn another_program_s_open_model_is_opened_alone() {
    let r = review(
        fixtures().join("ifc/v1/disaridan/bsi-ifc4x3-building-architecture.ifc"),
        Some(5256),
    );
    let e = errors(&r);
    assert!(
        e.iter()
            .any(|(l, c, _)| *l == "paket" && *c == "review_model_only"),
        "{e:#?}"
    );
    assert!(
        e.iter()
            .any(|(l, c, _)| *l == "konum" && *c == "review_place_zone"),
        "{e:#?}"
    );
    assert_eq!(r.files.len(), 1);
    assert_eq!(r.rows.len(), 21);
    // No sheets and no form to judge: not judged, not failed.
    let layers = holds(&r);
    assert_eq!(layers[1], ("pafta", None));
    assert_eq!(layers[4], ("form", None));
}

/// The other broken packages: each code the note says, in its layer, on its file.
/// Another discipline's model (docs/adr/8114): the section building's structural model by another program, its file named
/// by EK-2 for the structural discipline (ST), is answered by EK-9 Table 9.2's 16 questions; the architectural one keeps
/// Table 9.1's 21.
#[test]
fn a_structural_model_is_answered_by_table_9_2() {
    let dir = std::env::temp_dir().join(format!("kentos-review-92-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("260123_00_ST_XXXX_MD_XXX_NNN_01_000.ifc");
    std::fs::copy(
        fixtures().join("ifc/v1/statik/kesit-binasi-statik-datum.ifc"),
        &file,
    )
    .expect("the model copied");
    let r = review(file, Some(5256));
    assert_eq!(r.table.as_deref(), Some("9.2"));
    assert_eq!(r.rows.len(), 16);
    assert_eq!(r.rows[11].title, "IFC Sınıfları");
    let _ = std::fs::remove_dir_all(&dir);
    let own = review(fixtures().join("bim/v1/paket/kentos"), Some(5256));
    assert_eq!((own.table.as_deref(), own.rows.len()), (Some("9.1"), 21));
}

#[test]
fn every_broken_package_gives_what_it_breaks() {
    let note: Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures().join("bim/v1/paket/review_packages.json")).unwrap(),
    )
    .unwrap();
    for (name, layers) in note["packages"].as_object().unwrap() {
        if name == "bozuk" {
            continue;
        }
        let r = review(fixtures().join("bim/v1/paket").join(name), Some(5256));
        for (layer, wants) in layers.as_object().unwrap() {
            for want in wants.as_array().unwrap() {
                let (code, file) = match want {
                    Value::Array(pair) => (pair[0].as_str().unwrap(), pair[1].as_str()),
                    other => (other.as_str().unwrap(), None),
                };
                assert!(
                    r.findings.iter().any(|f| f.layer == *layer
                        && f.code == code
                        && (file.is_none() || f.file.as_deref() == file)),
                    "{name}: {layer} {code} {file:?} yok: {:#?}",
                    r.findings
                );
            }
        }
    }
}

#[test]
fn a_package_that_is_not_there_or_not_a_package_is_refused() {
    let mut doc = drawing();
    for path in ["", "paket", "/yok/boyle/bir/paket"] {
        let answer = kentos_headless::file_commands::call(
            &mut doc,
            CallEnvelope {
                command: "file.review.package".into(),
                version: None,
                op: CallOp::Execute,
                request_id: None,
                expected: None,
                relax_constraints: false,
                input: json!({"path": path}),
            },
        );
        let CommandResult::Failed { error } = &answer.result else {
            panic!("{path}: {answer:?}")
        };
        assert_eq!(error.code, "invalid_file_path", "{path}");
    }
}
