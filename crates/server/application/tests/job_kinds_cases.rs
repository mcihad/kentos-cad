//! What `project.import` and `project.atlas` jobs decide without a database (docs/adr/0561), against the shared cases
//! `fixtures/server-jobs/v1/kinds.json` that `scripts/fixtures/server_job_kinds_cases.py` writes from the ADR's rules
//! without KentOS code: the inputs' faults, the format a file's name says, the memory an import is estimated to need.

use kentos_application::processing_job::{check_atlas_input, check_import_input};
use kentos_contracts::{
    ImportFormat, ProjectAtlasInput, ProjectImportInput, import_memory_estimate,
};
use serde_json::Value;

const CASES: &str = include_str!("../../../../fixtures/server-jobs/v1/kinds.json");

fn cases() -> Value {
    serde_json::from_str(CASES).expect("the cases read")
}

#[test]
fn an_import_request_is_faulted_where_the_reference_says() {
    let cases = cases();
    let inputs = cases["imports"].as_array().expect("imports");
    assert!(inputs.len() >= 20);
    for case in inputs {
        let input: ProjectImportInput =
            serde_json::from_value(case["input"].clone()).expect("an input");
        let got = check_import_input(&input).map(|(path, _)| path);
        assert_eq!(
            got,
            case["fault"].as_str().map(str::to_owned),
            "{}",
            case["name"]
        );
    }
}

#[test]
fn an_atlas_request_is_faulted_where_the_reference_says() {
    let cases = cases();
    let inputs = cases["atlases"].as_array().expect("atlases");
    assert!(inputs.len() >= 20);
    for case in inputs {
        let input: ProjectAtlasInput =
            serde_json::from_value(case["input"].clone()).expect("an input");
        let got = check_atlas_input(&input).map(|(path, _)| path);
        assert_eq!(
            got,
            case["fault"].as_str().map(str::to_owned),
            "{}",
            case["name"]
        );
    }
}

#[test]
fn a_files_name_says_its_format_and_the_format_its_command() {
    let cases = cases();
    for case in cases["formats"].as_array().expect("formats") {
        let name = case["name"].as_str().expect("a name");
        let got = ImportFormat::of_name(name);
        let want = case["format"].as_str();
        assert_eq!(
            got.map(|f| f.extension()),
            want.map(|w| match w {
                "gpkg" => "gpkg",
                "fgb" => "fgb",
                "gml" => "gml",
                "csv" => "csv",
                "shp" => "zip",
                "geojson" => "geojson",
                "kml" => "kml",
                "parquet" => "parquet",
                _ => "dxf",
            }),
            "{name}"
        );
        assert_eq!(
            got.map(ImportFormat::command),
            case["command"].as_str(),
            "{name}"
        );
    }
}

#[test]
fn the_memory_an_import_is_estimated_to_need_is_the_references() {
    let cases = cases();
    for case in cases["memory"].as_array().expect("memory") {
        let (snapshot, file) = (
            case["snapshot"].as_u64().expect("snapshot"),
            case["file"].as_u64().expect("file"),
        );
        assert_eq!(
            import_memory_estimate(snapshot, file),
            case["estimate"].as_u64().expect("estimate"),
            "{snapshot} {file}"
        );
    }
}
