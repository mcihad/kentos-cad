//! Model kalite kontrol formu (docs/adr/8047 §10): EK-9's form placed in the drawing is the permit set's sheet of its
//! own, EK-2's FR/MKF; Ruhsat takımı writes its answers again from the IFC model as it is when the set is made.

use crate::permit_set_tests::{building, particulars, texts};

/// A form placed earlier: the header and a row of answers now stale.
fn stale_form(doc: &mut kentos_domain::Document) {
    let mut rows: Vec<Vec<String>> = vec![
        kentos_ifc::quality::FORM_HEADER
            .iter()
            .map(|h| (*h).to_owned())
            .collect(),
    ];
    rows.push(
        ["1", "Genel", "eski", "", "Evet", ""]
            .map(str::to_owned)
            .to_vec(),
    );
    let look = kentos_interaction::table::Look {
        header: true,
        height: 0.25,
        ..Default::default()
    };
    let cells = kentos_geometry_core::ops::table::from_rows(rows, true);
    let geometry = kentos_interaction::table::new_table(
        doc,
        &cells,
        &look,
        kentos_interaction::Vec2::new(40.0, 0.0),
        None,
    );
    let input = kentos_commands::EntitiesCreate {
        layer_id: doc.layers().active().to_owned(),
        objects: vec![kentos_commands::NewObject {
            geometry,
            color: None,
            line_weight: None,
            line_type: None,
            line_pattern: None,
            attrs: None,
            label: None,
            label_of: None,
            label_scale: None,
            symbol: None,
            order: None,
            transparency: None,
        }],
        operation: None,
        expected_revision: None,
    };
    let done = kentos_native_application::create::execute(
        &mut kentos_native_application::ExecutionContext::new(doc),
        input,
    );
    assert!(
        matches!(done, kentos_commands::CommandResult::Completed { .. }),
        "{done:?}"
    );
}

/// The drawing's form tables' rows.
fn forms(doc: &kentos_domain::Document) -> Vec<Vec<Vec<String>>> {
    doc.entities()
        .filter_map(|e| match e {
            kentos_commands::Entity::Table(t)
                if t.cells
                    .first()
                    .is_some_and(|h| kentos_ifc::quality::is_form(h)) =>
            {
                Some(t.cells.clone())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn the_form_in_the_drawing_is_the_set_s_fr_mkf_sheet_with_its_answers_now() {
    let mut app = building();
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    stale_form(doc);
    // The künye first, as Ruhsat takımı writes it before its tables.
    let mut settings = doc.settings().clone();
    settings.variables = kentos_native_application::permit_set::with_particulars(
        &settings.variables,
        doc.name(),
        &particulars(),
    );
    doc.set_settings(settings);
    let tables = kentos_interaction::calc_tables::ensure(doc).expect("the tables");
    let last = tables.last().expect("tables");
    assert_eq!(
        (last.label, last.list),
        ("Model kalite kontrol formu", Some("MKF"))
    );
    // One form, its answers written again: a header, five groups and 21 questions.
    let form = forms(doc);
    assert_eq!(
        form.len(),
        1,
        "the stale form gave its place to the new one"
    );
    let rows = &form[0];
    assert_eq!(rows.len(), 1 + 5 + 21);
    let answer = |no: &str| {
        rows.iter()
            .find(|r| r[0] == no)
            .cloned()
            .expect("a question")
    };
    // The künye lacks the office's name and tax number; the drawing is in local coordinates.
    let info = answer("2");
    assert_eq!(info[4], "Hayır");
    assert!(
        info[5].contains("büronun adı, büronun vergi kimlik no"),
        "{}",
        info[5]
    );
    assert_eq!(answer("3")[4], "Evet", "the project's code is 260457");
    assert_eq!(answer("6")[4], "Hayır");
    assert_eq!(answer("1")[4], "", "the architect answers the file size");
    // The set: the form a sheet of its own after the plans, named by EK-2's form and model control form.
    let input = kentos_native_application::permit_set::PermitSet {
        phase: kentos_sheet::permit::Phase::Permit,
        values: particulars(),
        tables: tables
            .iter()
            .filter(|t| t.list.is_none())
            .map(|t| (t.label.to_owned(), t.bounds))
            .collect(),
        lists: tables
            .iter()
            .filter_map(|t| {
                t.list
                    .map(|kind| kentos_native_application::permit_set::ListSheet {
                        title: t.label.to_owned(),
                        kind,
                        bounds: t.bounds,
                        scale: 100,
                        scaled: false,
                    })
            })
            .collect(),
        renovation: false,
    };
    let (_, made, notes) = kentos_native_application::permit_set::execute(
        &mut kentos_native_application::ExecutionContext::new(doc),
        &input,
    )
    .expect("made");
    assert!(notes.is_empty(), "{notes:?}");
    let k = made
        .iter()
        .position(|m| m.title == "Model kalite kontrol formu")
        .unwrap_or_else(|| panic!("{:?}", made.iter().map(|m| &m.title).collect::<Vec<_>>()));
    let sheet = texts(&app, &made[k].sheet);
    assert!(
        sheet
            .iter()
            .any(|t| t.contains("260457_00_MM_XXXX_FR_MKF_NNN_01_000")),
        "{sheet:?}"
    );
}
