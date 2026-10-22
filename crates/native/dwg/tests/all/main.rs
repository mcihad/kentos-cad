//! Adapter conformance. Expected geometry comes from the independent ASCII
//! fixtures; this does not substitute for an independent DWG engine audit.
use kentos_contracts::{DwgVersion, DxfReadOptions};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/formats/v1")
            .join(name),
    )
    .expect("fixture")
}

#[test]
fn every_declared_binary_target_reopens_with_the_expected_signature() {
    let source = fixture("dxf-units/mm.dxf");
    for target in DwgVersion::ALL {
        let bytes =
            kentos_dwg::encode(&source, target).unwrap_or_else(|e| panic!("{target:?}: {e}"));
        assert_eq!(&bytes[..6], target.signature().as_bytes());
        let options = DxfReadOptions {
            unit: Some(kentos_contracts::DrawingUnit::M),
            // R14 cannot carry INSUNITS. Its source unit must be selected;
            // the default path is tested separately and must say "assumed".
            file_unit: (target == DwgVersion::AC1014).then_some(kentos_contracts::FileUnit::Mm),
            ..Default::default()
        };
        let got = kentos_dwg::read(&bytes, &options).unwrap_or_else(|e| panic!("{target:?}: {e}"));
        let expected = kentos_formats::dxf::read(&source, &options).expect("ASCII reference");
        assert_eq!(got.entities, expected.entities, "{target:?}");
        if target == DwgVersion::AC1014 {
            assert_eq!(got.unit.as_ref().map(|u| u.used), Some(4));
            let implicit =
                kentos_dwg::read(&bytes, &DxfReadOptions::default()).expect("legacy read");
            assert!(implicit.unit.expect("decision").assumed);
        } else {
            assert_eq!(got.unit, expected.unit, "{target:?}");
        }
        assert!(
            got.report
                .source
                .iter()
                .any(|f| f.label == "Kaynak DWG sürümü" && f.value == target.label())
        );
    }
}

#[test]
fn layers_blocks_annotations_and_curves_cross_the_codec_boundary() {
    for name in [
        "entities.dxf",
        "blocks.dxf",
        "texts.dxf",
        "styles.dxf",
        "dimension.dxf",
        "hatch.dxf",
        "ocs.dxf",
        "elevations.dxf",
    ] {
        let source = fixture(name);
        let options = DxfReadOptions::default();
        let expected = kentos_formats::dxf::read(&source, &options).expect("ASCII reference");
        let bytes = kentos_dwg::encode(&source, DwgVersion::AC1032)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let got = kentos_dwg::read(&bytes, &options).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            got.entities.len(),
            expected.entities.len(),
            "{name}: {:?}",
            got.report
        );
        assert_eq!(got.blocks.len(), expected.blocks.len(), "{name}");
    }
}

#[test]
fn unsupported_and_truncated_files_return_errors() {
    for bytes in [
        b"".as_slice(),
        b"AC1009",
        b"AC1033",
        b"not DWG",
        b"AC1032",
        b"AC1014\0\0\0",
    ] {
        assert!(
            kentos_dwg::read(bytes, &DxfReadOptions::default()).is_err(),
            "{bytes:?}"
        );
    }
}

#[test]
fn a_cancelled_read_returns_no_partial_result() {
    let bytes =
        kentos_dwg::encode(&fixture("dxf-units/mm.dxf"), DwgVersion::AC1032).expect("encoded");
    let mut stop = kentos_formats::watch::Steps(|_, _| false);
    assert_eq!(
        kentos_dwg::read_watched(&bytes, &DxfReadOptions::default(), &mut stop)
            .expect_err("stopped")
            .code,
        "cancelled"
    );
}

#[test]
fn constant_and_tapered_widths_survive_all_binary_targets_without_zero_override_loss() {
    let source = fixture("dxf-import/closed.dxf");
    let options = DxfReadOptions {
        unit: Some(kentos_contracts::DrawingUnit::M),
        file_unit: Some(kentos_contracts::FileUnit::Mm),
        closed_polylines: kentos_contracts::ClosedPolylines::Line,
        ..Default::default()
    };
    let expected = kentos_formats::dxf::read_checked(&source,&options).unwrap();
    for target in DwgVersion::ALL {
        let bytes = kentos_dwg::encode(&source,target).unwrap();
        let got = kentos_dwg::read(&bytes,&options).unwrap();
        assert_eq!(got.entities,expected.entities,"{target:?}");
    }
}
