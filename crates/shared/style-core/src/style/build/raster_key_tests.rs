//! The raster keys' tests (moved out of build.rs to keep it under its size baseline, CLAUDE.md §8).

use super::dataset_key;
use kentos_geometry_core::api::json::Json;

/// The key's part as the web writes it too (`apps/web/src/model/rasterRules.ts`
/// `rasterKey`, its test `rasterKey.test.ts`): the same text for the same slice.
#[test]
fn a_datasets_part_is_its_variable_mesh_and_slice() {
    let grid = Json::parse(
        r#"{"variable":"t2m","dims":[{"name":"time","index":1,"values":[0,3600000,7200000],"time":true},{"name":"level","index":0,"values":[850,500]}],"followTime":true}"#,
    )
    .unwrap();
    assert_eq!(
        dataset_key(&grid, None, None).unwrap(),
        r#"{"variable":"t2m","slice":[1,0]}"#
    );
    // Following the slider: the last step at or before the moment; none before the first.
    assert_eq!(
        dataset_key(&grid, Some(7_199_999.0), None).unwrap(),
        r#"{"variable":"t2m","slice":[1,0]}"#
    );
    assert_eq!(
        dataset_key(&grid, Some(7_200_000.0), None).unwrap(),
        r#"{"variable":"t2m","slice":[2,0]}"#
    );
    assert!(dataset_key(&grid, Some(-1.0), None).is_none());
    let mesh = Json::parse(r#"{"variable":"ucx","vector":"ucy","mesh":"mesh","dims":[{"name":"time","index":0,"values":[0,1800000],"time":true}]}"#).unwrap();
    let affine = [500000.0, 0.125, 0.0, 4420040.0, 0.0, -0.125];
    assert_eq!(
        dataset_key(&mesh, None, Some((&affine, [320.0, 320.0]))).unwrap(),
        r#"{"variable":"ucx","vector":"ucy","mesh":"mesh","slice":[0],"affine":[500000,0.125,0,4420040,0,-0.125],"size":[320,320]}"#
    );
}
