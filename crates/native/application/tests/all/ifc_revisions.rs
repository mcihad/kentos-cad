//! IFC-10's first acceptance (docs/adr/8103; plan/ifc-uyum.md IFC-10): the section building's two revisions — KentOS's
//! file of the declared building (fixtures/arch/v1/ifc/kesit-binasi-tam.ifc) and the same file another program changed
//! (fixtures/ifc/v1/revizyon/kesit-binasi-r2.ifc, IfcOpenShell: scripts/fixtures/revision_pair.py) — compared: the
//! interior wall moved 0.50 m north and the east wall's fire rating REI 60 → REI 90 are the report, nothing else. The
//! moved wall written as a BCF topic reads back with the same object, which leads to the drawing's wall; the file is
//! kept (fixtures/ifc/v1/revizyon/tasinan-duvar.bcf) for the independent reader (scripts/fixtures/bcf_check.py).
//! `KENTOS_WRITE_BCF=1` writes it again.

use std::path::PathBuf;

use kentos_commands::{BcfCommentInput, BcfRead, BcfTopicInput, BcfWrite, CommandResult, IfcChange, IfcCompare, IfcValue};
use kentos_native_application::{ExecutionContext, bcf_exchange, ifc_compare};

fn fixture(rel: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures").join(rel).canonicalize().expect(rel).display().to_string()
}

const MOVED: &str = "01alMXGa9m080000000007";
const RATED: &str = "01alMXGa9m080000000002";

#[test]
fn the_moved_wall_and_the_new_fire_rating_are_the_report_and_nothing_else() {
    let mut doc = crate::ifc_required::declared();
    let input = IfcCompare {
        older: fixture("arch/v1/ifc/kesit-binasi-tam.ifc"),
        newer: fixture("ifc/v1/revizyon/kesit-binasi-r2.ifc"),
        tolerance: None,
    };
    let CommandResult::Completed { output: c, warnings } = ifc_compare::execute(&mut ExecutionContext::new(&mut doc), input)
    else {
        panic!("compared")
    };
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(c.added.is_empty() && c.removed.is_empty() && c.repeated.is_empty(), "{c:?}");
    let ids: Vec<&str> = c.changed.iter().map(|x| x.global_id.as_str()).collect();
    assert_eq!(ids, [RATED, MOVED], "in the newer file's order");
    let rated = &c.changed[0];
    assert_eq!(rated.class, "IfcWall");
    assert_eq!(
        rated.changes,
        [IfcChange::Property {
            set: "Pset_WallCommon".into(),
            from_type: false,
            name: "FireRating".into(),
            old: Some(IfcValue::Text { text: "REI 60".into() }),
            new: Some(IfcValue::Text { text: "REI 90".into() }),
        }]
    );
    let moved = &c.changed[1];
    let [IfcChange::Moved { by, distance }] = moved.changes.as_slice() else { panic!("{:?}", moved.changes) };
    assert!((by[0]).abs() < 1e-6 && (by[1] - 0.5).abs() < 1e-6 && by[2].abs() < 1e-6, "{by:?}");
    assert!((distance - 0.5).abs() < 1e-6);
    assert_eq!(c.unchanged + 2, c.newer.objects);
}

#[test]
fn the_moved_wall_written_as_an_issue_reads_back_as_the_drawing_s_wall() {
    let mut doc = crate::ifc_required::declared();
    let model = fixture("ifc/v1/revizyon/kesit-binasi-r2.ifc");
    let kept = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/ifc/v1/revizyon/tasinan-duvar.bcf");
    let dir = std::env::temp_dir().join(format!("kentos-bcf-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("tasinan-duvar.bcf");
    let topic = BcfTopicInput {
        guid: Some("0192f5a1-8103-7000-8000-000000000001".into()),
        title: "Zemin kattaki iç duvar 50 cm kuzeye taşınmış".into(),
        description: Some("İkinci revizyonda 01alMXGa9m080000000007 duvarı y yönünde 0,50 m kaymış; odaların alanı değişir.".into()),
        topic_type: Some("Issue".into()),
        status: Some("Open".into()),
        priority: Some("High".into()),
        labels: vec!["Mimari".into()],
        author: "mimar@buro".into(),
        date: Some("2026-11-17T12:00:00Z".into()),
        assigned_to: Some("statik@buro".into()),
        objects: vec![MOVED.into()],
        comments: vec![BcfCommentInput { author: "mimar@buro".into(), date: None, text: "Statik projeyle birlikte bakılsın.".into() }],
    };
    let input = BcfWrite { path: path.display().to_string(), overwrite: true, model: Some(model.clone()), topics: vec![topic] };
    let CommandResult::Completed { output: w, .. } = bcf_exchange::write::execute(&mut ExecutionContext::new(&mut doc), input)
    else {
        panic!("written")
    };
    assert_eq!((w.topics, w.viewpoints), (1, 1));
    let bytes = std::fs::read(&path).unwrap();
    if std::env::var_os("KENTOS_WRITE_BCF").is_some() {
        std::fs::write(&kept, &bytes).unwrap();
    }
    assert_eq!(std::fs::read(&kept).expect("the kept file"), bytes, "the same file each time");
    let read = BcfRead { path: path.display().to_string(), model: Some(model) };
    let CommandResult::Completed { output: r, .. } = bcf_exchange::read::execute(&mut ExecutionContext::new(&mut doc), read)
    else {
        panic!("read")
    };
    assert_eq!(r.version, "3.0");
    let t = &r.topics[0];
    assert_eq!(t.title, "Zemin kattaki iç duvar 50 cm kuzeye taşınmış");
    assert_eq!(t.files, ["kesit-binasi-r2.ifc"]);
    assert_eq!(t.comments.len(), 1);
    assert_eq!(t.viewpoints[0].selection, [MOVED]);
    // The camera looks at the wall's box in the second revision (x 0.105–4.445, y 4.925–5.075, z −0.13–2.78).
    let cam = t.viewpoints[0].camera.as_ref().expect("a camera");
    let middle = [(0.105 + 4.445) / 2.0, (4.925 + 5.075) / 2.0, (-0.13 + 2.78) / 2.0];
    let to = [middle[0] - cam.eye[0], middle[1] - cam.eye[1], middle[2] - cam.eye[2]];
    let d = &cam.direction;
    let cross = [to[1] * d[2] - to[2] * d[1], to[2] * d[0] - to[0] * d[2], to[0] * d[1] - to[1] * d[0]];
    assert!(cross.iter().all(|c| c.abs() < 1e-4), "{cross:?}");
    // The object leads to the file's wall and to the drawing's.
    let o = &r.objects[0];
    assert_eq!(o.model.as_ref().map(|m| m.class.as_str()), Some("IfcWall"));
    assert_eq!(o.uid.as_deref(), Some("0192f5a1-4242-7000-8000-000000000007"));
    let _ = std::fs::remove_dir_all(&dir);
}
