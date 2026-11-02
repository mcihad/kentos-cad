//! CAD-ADV-05: physical-unit preflight, dependent expressions, legacy values and one atomic history step.
use kentos_commands::*;
use kentos_domain::{Document, Slot};
use kentos_native_application::{
    ExecutionContext, constraint_solve::parameter_values, constraints_edit, constraints_query,
    parameters_edit,
};
use serde_json::{Value, json};

fn done<T: std::fmt::Debug>(r: CommandResult<T>) -> T {
    match r {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    }
}
fn drawing() -> Document {
    let mut raw: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/interaction/v1/objects.kcad"
    ))
    .unwrap();
    raw["settings"]["workspace"] = json!("cad");
    raw["origin"] = json!({"x":0,"y":0});
    raw["entities"] = json!([{"kind":"line","id":1,"layerId":"cizim","attrs":{},"a":{"x":487000,"y":4420000},"b":{"x":487004,"y":4420000}}]);
    Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap()
}
fn edits(d: &mut Document, edits: Value) -> ParametersEdited {
    done(parameters_edit::execute(
        &mut ExecutionContext::new(d),
        serde_json::from_value(json!({"edits":edits})).unwrap(),
    ))
}
fn parameter(name: &str, kind: &str, expression: &str) -> Value {
    json!({"kind":"add","parameter":{"name":name,"kind":kind,"expression":expression}})
}
fn set(name: &str, expression: &str) -> Value {
    json!({"kind":"set","name":name,"expression":expression})
}
fn refused(d: &mut Document, edits: Value) -> CommandError {
    let input: ParametersEdit = serde_json::from_value(json!({"edits":edits})).unwrap();
    let before = d.to_snapshot_v2();
    let revision = d.revision();
    let cx = ExecutionContext::new(d);
    let CommandResult::Failed { error: validated } = parameters_edit::validate(&cx, &input) else {
        panic!("validate accepted invalid units")
    };
    let CommandResult::Failed { error: planned } = parameters_edit::plan(&cx, &input) else {
        panic!("plan accepted invalid units")
    };
    let CommandResult::Failed { error } =
        parameters_edit::execute(&mut ExecutionContext::new(d), input)
    else {
        panic!("execute accepted invalid units")
    };
    assert_eq!(error, validated);
    assert_eq!(error, planned);
    assert_eq!(error.code, "parameter_kind");
    assert_eq!(d.revision(), revision);
    assert_eq!(d.to_snapshot_v2(), before);
    error
}

#[test]
fn incompatible_units_are_atomic_and_errors_identify_the_expression() {
    let mut d = drawing();
    for (kind, expr) in [
        ("length", "30deg"),
        ("angle", "2m"),
        ("number", "2m"),
        ("length", "2m + 30deg"),
        ("length", "sqrt(2m)"),
        ("number", "pi"),
    ] {
        let error = refused(&mut d, json!([parameter("bad", kind, expr)]));
        assert!(error.message.contains("“bad”"));
        assert_eq!(error.path.as_deref(), Some("parameters[0].expression"));
    }
    let pristine = d.to_snapshot_v2();
    edits(&mut d, json!([parameter("width", "length", "4m")]));
    let good = d.to_snapshot_v2();
    refused(&mut d, json!([set("width", "4m + 45deg")]));
    assert_eq!(d.to_snapshot_v2(), good);
    d.undo().unwrap(); // refusals did not create an extra undo step.
    assert_eq!(d.to_snapshot_v2(), pristine);
}

#[test]
fn same_unit_products_ratios_and_trigonometry_keep_the_existing_numeric_values() {
    let mut d = drawing();
    let before = d.to_snapshot_v2();
    let input: ParametersEdit = serde_json::from_value(json!({"edits":[
        parameter("width", "length", "4m"), parameter("height", "length", "3m"),
        parameter("diagonal", "length", "sqrt(width^2 + height^2)"),
        parameter("ratio", "number", "width / height"), parameter("turn", "angle", "pi/6"),
        parameter("projection", "length", "width * cos(turn)"), parameter("math_pi", "number", "pi / 1rad")
    ]})).unwrap();
    let planned = done(parameters_edit::plan_in(&d, &input));
    assert_eq!(d.to_snapshot_v2(), before);
    let actual = done(parameters_edit::execute(
        &mut ExecutionContext::new(&mut d),
        input,
    ));
    assert_eq!(planned.values, actual.values);
    let values = parameter_values(&d, d.constraints()).unwrap();
    assert!((values["diagonal"] - 5.).abs() < 1e-12);
    assert!((values["ratio"] - 4. / 3.).abs() < 1e-12);
    assert!((values["projection"] - 4. * (std::f64::consts::PI / 6.).cos()).abs() < 1e-12);
    assert!((values["math_pi"] - std::f64::consts::PI).abs() < 1e-12);
    refused(&mut d, json!([set("projection", "width + turn")]));
    let saved = d.to_snapshot_v2();
    assert_eq!(
        Document::from_snapshot_v2(saved.clone())
            .unwrap()
            .to_snapshot_v2(),
        saved
    );
    d.undo().unwrap();
    assert_eq!(d.to_snapshot_v2(), before);
}

#[test]
fn changing_a_unitless_power_also_checks_previously_valid_dependent_dimensions() {
    let mut d = drawing();
    edits(
        &mut d,
        json!([
            parameter("width", "length", "4m"),
            parameter("power", "number", "2"),
            parameter("root", "length", "sqrt(width ^ power)")
        ]),
    );
    let error = refused(&mut d, json!([set("power", "1")]));
    assert!(error.message.contains("“root”"));
    assert_eq!(error.path.as_deref(), Some("parameters[2].expression"));
    edits(
        &mut d,
        json!([set("power", "1"), set("root", "width^power")]),
    );
    assert_eq!(parameter_values(&d, d.constraints()).unwrap()["root"], 4.0);
}

#[test]
fn physical_results_are_independent_of_project_length_and_angle_display_units() {
    for drawing_unit in [
        DrawingUnit::Mm,
        DrawingUnit::Cm,
        DrawingUnit::M,
        DrawingUnit::In,
        DrawingUnit::Ft,
    ] {
        for angle_unit in [AngleUnit::Deg, AngleUnit::Grad] {
            let mut snapshot = drawing().to_snapshot_v2();
            snapshot.settings.drawing_unit = Some(drawing_unit);
            snapshot.settings.angle_unit = angle_unit;
            let mut d = Document::from_snapshot_v2(snapshot).unwrap();
            edits(
                &mut d,
                json!([
                    parameter("length", "length", "1m + 25cm"),
                    parameter("turn", "angle", "pi/2"),
                    parameter("projection", "length", "length * sin(turn)"),
                    parameter("ratio", "number", "length / 250mm")
                ]),
            );
            let values = parameter_values(&d, d.constraints()).unwrap();
            let units = ParamUnits::of(d.settings());
            assert!((values["projection"] * units.metres_per_unit - 1.25).abs() < 1e-12);
            assert!(
                (values["turn"] * units.radians_per_unit - std::f64::consts::FRAC_PI_2).abs()
                    < 1e-12
            );
            assert!((values["ratio"] - 5.0).abs() < 1e-12);
            refused(&mut d, json!([set("length", "1m + 1rad")]));
        }
    }
}

#[test]
fn historical_pi_values_stay_readable_renamable_and_audited_until_explicitly_repaired() {
    for (angle_unit, half_turn) in [(AngleUnit::Deg, 180.0), (AngleUnit::Grad, 200.0)] {
        let d = drawing();
        let mut snapshot = d.to_snapshot_v2();
        snapshot.settings.angle_unit = angle_unit;
        snapshot.constraints.parameters = vec![
            serde_json::from_value(json!({"name":"old_pi","kind":"number","expression":"pi"}))
                .unwrap(),
        ];
        let mut d = Document::from_snapshot_v2(snapshot.clone()).unwrap();
        assert!(
            (parameter_values(&d, d.constraints()).unwrap()["old_pi"] - half_turn).abs() < 1e-12
        );
        let CommandResult::Completed { warnings, .. } =
            constraints_query::read(&d, &ConstraintsQuery::default())
        else {
            panic!()
        };
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "parameter_units_legacy");
        assert_eq!(d.to_snapshot_v2(), snapshot);
        edits(
            &mut d,
            json!([{"kind":"rename","name":"old_pi","to":"legacy"}, parameter("width", "length", "4m")]),
        );
        assert!(
            (parameter_values(&d, d.constraints()).unwrap()["legacy"] - half_turn).abs() < 1e-12
        );
        refused(&mut d, json!([set("legacy", "pi")])); // explicitly re-authored errors are not grandfathered.
        let before_repair = d.to_snapshot_v2();
        edits(&mut d, json!([set("legacy", "pi / 1rad")]));
        assert!(
            (parameter_values(&d, d.constraints()).unwrap()["legacy"] - std::f64::consts::PI).abs()
                < 1e-12
        );
        let CommandResult::Completed { warnings, .. } =
            constraints_query::read(&d, &ConstraintsQuery::default())
        else {
            panic!()
        };
        assert!(warnings.is_empty());
        d.undo().unwrap();
        assert_eq!(d.to_snapshot_v2(), before_repair);
    }
}

#[test]
fn add_then_rename_does_not_bypass_dimension_check() {
    let mut d = drawing();
    let error = refused(
        &mut d,
        json!([parameter("bad", "length", "45deg"), {"kind":"rename","name":"bad","to":"renamed"}]),
    );
    assert!(error.message.contains("“renamed”"));
}

#[test]
fn new_dimensional_constraint_value_uses_the_same_unit_check_and_keeps_geometry() {
    let mut d = drawing();
    let before = d.to_snapshot_v2();
    let uid = d.uid(Slot(1)).unwrap().to_string();
    let p = |place| json!({"uid":uid,"element":{"kind":"point","place":{"kind":place}}});
    let input: ConstraintsEdit = serde_json::from_value(json!({"edit":{"kind":"add","constraints":[
        {"rule":{"kind":"distance","a":p("start"),"b":p("end"),"parameter":"length"},"value":"45deg"}
    ]}})).unwrap();
    let CommandResult::Failed { error } =
        constraints_edit::plan(&ExecutionContext::new(&mut d), &input)
    else {
        panic!()
    };
    let CommandResult::Failed { error: actual } =
        constraints_edit::execute(&mut ExecutionContext::new(&mut d), input)
    else {
        panic!()
    };
    assert_eq!(error, actual);
    assert_eq!(error.code, "parameter_kind");
    assert_eq!(d.to_snapshot_v2(), before);
}
