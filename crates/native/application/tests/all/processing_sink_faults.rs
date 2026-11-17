//! `cad.processing.run` writing to a GeoPackage with its faults injected (docs/adr/8057 §120, plan/eklenti.md
//! ECO-QA-10): the request sent again after its answer was lost, a file that takes no write, a folder that takes no
//! file. A request sent again is answered as before and applied once; a fault leaves the file and the drawing as they
//! were and no passing copy beside them. A stop at any point and a file that is no GeoPackage are `processing_sink`'s.

use kentos_commands::{CallAnswer, CallEnvelope, CallOp, CommandResult};
use kentos_domain::Document;
use serde_json::{Value, json};

use super::processing_sink::{buffer_to, drawing, no_stray, qgis_copy};

/// The run sent as a request named `request`.
fn sent(doc: &mut Document, input: &Value, request: &str) -> CallAnswer {
    kentos_processing::command::register();
    let envelope = CallEnvelope {
        relax_constraints: false,
        command: "cad.processing.run".into(),
        version: None,
        op: CallOp::Execute,
        input: input.clone(),
        request_id: Some(request.into()),
        expected: None,
    };
    kentos_native_application::call::call(doc, None, envelope)
}

fn output(answer: &CallAnswer) -> &Value {
    match &answer.result {
        CommandResult::Completed { output, .. } => output,
        other => panic!("completed: {other:?}"),
    }
}

fn code(answer: &CallAnswer) -> String {
    match &answer.result {
        CommandResult::Failed { error } | CommandResult::Conflict { error } => error.code.clone(),
        other => format!("{other:?}"),
    }
}

#[test]
fn a_request_sent_again_is_answered_as_before_and_applied_once() {
    let (dir, path) = qgis_copy("again");
    let mut doc = drawing();
    let revision = doc.revision();
    let input = buffer_to(&path, "addTables");
    let first = sent(&mut doc, &input, "tampon:1");
    assert!(!first.replayed);
    let written = std::fs::read(&path).expect("reads");
    assert_eq!(
        output(&first)["sink"]["sha256"],
        json!(kentos_commands::encoding::sha256_hex(&written))
    );
    let rows = doc.journal().count();
    // Its answer lost on the way, the same request again: answered as before, its table not refused as another's.
    let again = sent(&mut doc, &input, "tampon:1");
    assert!(again.replayed && !again.undone);
    assert_eq!(output(&again), output(&first));
    assert_eq!(
        std::fs::read(&path).expect("reads"),
        written,
        "nothing written again"
    );
    assert_eq!(doc.journal().count(), rows, "one row in the journal");
    // Another request of the same run is another write: its table is there, refused, the file byte for byte.
    assert_eq!(code(&sent(&mut doc, &input, "tampon:2")), "table_exists");
    assert_eq!(std::fs::read(&path).expect("reads"), written);
    // The request's name with another call: refused, nothing written.
    let other = buffer_to(&path, "replaceTables");
    assert_eq!(code(&sent(&mut doc, &other, "tampon:1")), "request_reused");
    assert_eq!(std::fs::read(&path).expect("reads"), written);
    // A plan is made on a passing copy of the file; its refusal names the user's file.
    let planned = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc),
        "cad.processing.run",
        CallOp::Plan,
        input.clone(),
    )
    .expect("plans");
    assert_eq!(planned["error"]["code"], "table_exists", "{planned}");
    let said = planned["error"]["message"].as_str().unwrap_or_default();
    assert!(
        said.contains("var.gpkg") && !said.contains(".kentos-"),
        "{said}"
    );
    assert_eq!(doc.revision(), revision, "the drawing does not change");
    no_stray(&dir);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_request_sent_again_does_not_overwrite_what_is_there_now() {
    let (dir, _) = qgis_copy("again-new");
    let mut doc = drawing();
    let path = dir.join("yeni.gpkg");
    let mut input = buffer_to(&path, "new");
    input["sink"] = json!({ "path": path.display().to_string(), "overwrite": true });
    let first = sent(&mut doc, &input, "yeni:1");
    assert!(!first.replayed, "{:?}", first.result);
    // The file changed since by its user: a run again would overwrite it (overwrite: true), the request again does not.
    std::fs::write(&path, "kullanıcının yeni içeriği").expect("written");
    let again = sent(&mut doc, &input, "yeni:1");
    assert!(again.replayed);
    assert_eq!(
        std::fs::read_to_string(&path).expect("reads"),
        "kullanıcının yeni içeriği"
    );
    no_stray(&dir);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Whether the system's rights bind this process (a superuser writes anywhere: a folder's rights test nothing then).
#[cfg(unix)]
fn rights_hold(dir: &std::path::Path) -> bool {
    std::fs::write(dir.join("deneme"), b"").is_err()
}

#[cfg(unix)]
fn rights(path: &std::path::Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("rights");
}

#[cfg(unix)]
#[test]
fn a_file_that_takes_no_write_is_refused_and_left_as_it_was() {
    let (dir, path) = qgis_copy("readonly");
    let mut doc = drawing();
    let revision = doc.revision();
    let before = std::fs::read(&path).expect("reads");
    rights(&path, 0o444);
    rights(&dir, 0o555);
    if rights_hold(&dir) {
        for mode in ["addTables", "replaceTables"] {
            let got = super::processing_sink::run(&mut doc, buffer_to(&path, mode));
            assert_eq!(got["error"]["code"], "file_unwritable", "{mode}: {got}");
            let said = got["error"]["message"].as_str().unwrap_or_default();
            assert!(said.ends_with("Eski dosya olduğu gibi kaldı."), "{said}");
            assert_eq!(
                std::fs::read(&path).expect("reads"),
                before,
                "{mode}: the file"
            );
            assert_eq!(doc.revision(), revision, "{mode}: the drawing");
        }
    }
    rights(&dir, 0o755);
    rights(&path, 0o644);
    no_stray(&dir);
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn a_folder_that_takes_no_file_is_refused_and_nothing_is_made() {
    let (dir, _) = qgis_copy("readonly-folder");
    let mut doc = drawing();
    let path = dir.join("yeni.gpkg");
    let mut input = buffer_to(&path, "new");
    input["sink"] = json!({ "path": path.display().to_string() });
    rights(&dir, 0o555);
    if rights_hold(&dir) {
        let got = super::processing_sink::run(&mut doc, input);
        assert_eq!(got["error"]["code"], "file_unwritable", "{got}");
        // The user's file named, not the passing copy; no old file said to be left where there was none.
        let said = got["error"]["message"].as_str().unwrap_or_default();
        assert!(
            said.contains("yeni.gpkg") && !said.contains(".kentos-"),
            "{said}"
        );
        assert!(said.ends_with("Dosya yazılmadı."), "{said}");
        assert!(!path.exists());
    }
    rights(&dir, 0o755);
    no_stray(&dir);
    let _ = std::fs::remove_dir_all(&dir);
}
