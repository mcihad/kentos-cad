// Denetle counts what the screen shows (14 November 2026, the owner: "denetle ölçeği de çalışmıyor olabilir"): the
// labelling as the window has it, not yet written ([`labels_report::Draft`]), and the drawing's texts keeping the
// labels off them as they do on the screen and the paper (docs/adr/0212 §3.5). The shared cases
// (fixtures/commands/v1/cad.labels.report.json) hold no text and no draft; these are the desktop's.

use kentos_commands::{CommandResult, LabelsReport, LabelsReported, LayersLabels};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, labels_report, layers_labels};
use serde_json::{Value, json};

const X: f64 = 500_000.0;
const Y: f64 = 4_400_000.0;

fn layer(id: &str, label: Option<Value>) -> Value {
    let mut style = json!({ "color": "fg", "lineType": "continuous", "lineWeight": 0.25 });
    match label {
        Some(l) => style["label"] = l,
        // Etiketleme: Yok.
        None => style["labels"] = json!({ "mode": "off", "classes": [] }),
    }
    json!({ "id": id, "name": id, "type": "layer", "visible": true, "locked": false, "expanded": true,
            "style": style, "children": [] })
}

/// A parcel of 40 × 30 m labelled "101/5" in the middle, its layer labelled as `label` says; and `texts`.
fn doc(label: Option<Value>, texts: &[Value]) -> Document {
    let rect = |x: f64, y: f64, w: f64, h: f64| json!([{ "x": x, "y": y }, { "x": x + w, "y": y }, { "x": x + w, "y": y + h }, { "x": x, "y": y + h }]);
    let mut entities = vec![
        json!({ "kind": "polygon", "id": 1, "layerId": "parsel", "attrs": {}, "label": "101/5",
                                    "pts": rect(X, Y, 40.0, 30.0) }),
    ];
    entities.extend(texts.iter().cloned());
    let setup = json!({
        "format": "kentos.document", "version": 1, "name": "Denetle",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                      "plotScale": 1000, "workspace": "gis" },
        "origin": { "x": X, "y": Y },
        "layers": [layer("parsel", label), layer("yazi", None)],
        "activeLayer": "parsel", "entities": entities, "styles": { "items": [], "categories": [] }
    });
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup");
    Document::from_snapshot(snapshot).expect("setup")
}

/// The parcel's layer at 1:1000 on paper, over the parcel and round it.
fn asked() -> LabelsReport {
    LabelsReport {
        px_per_metre: None,
        paper_scale: Some(1000),
        window: Some([X - 20.0, Y - 20.0, X + 60.0, Y + 50.0]),
        layers: Some(vec!["parsel".to_owned()]),
    }
}

fn report(doc: &mut Document, draft: labels_report::Draft<'_>) -> LabelsReported {
    match labels_report::execute_with(&mut ExecutionContext::new(doc), asked(), &[], draft) {
        CommandResult::Completed { output, .. } => output,
        other => panic!("report: {other:?}"),
    }
}

#[test]
fn denetle_counts_the_labelling_the_window_has_not_written_yet() {
    // The drawing labels nothing (Etiketleme: Yok); the window's draft labels the parcel in its middle.
    let mut d = doc(None, &[]);
    let saved = report(&mut d, labels_report::Draft::default());
    let draft = LayersLabels {
        layer: "parsel".to_owned(),
        label: Some(Some(
            serde_json::from_value(json!({ "placement": "center", "size": 10 })).expect("style"),
        )),
        labels: Some(None),
        expected_revision: None,
    };
    let node = match layers_labels::plan(&ExecutionContext::new(&mut d), &draft) {
        CommandResult::Completed { output, .. } => output.node,
        other => panic!("plan: {other:?}"),
    };
    let window = report(
        &mut d,
        labels_report::Draft {
            layer: Some(&node),
            texts: None,
        },
    );
    assert_eq!(
        (saved.totals.placed, window.totals.placed),
        (0, 1),
        "the drawing's labelling counts none, the window's one: {saved:?} {window:?}"
    );
    // Nothing was written.
    assert_eq!(
        report(&mut d, labels_report::Draft::default())
            .totals
            .placed,
        0
    );
}

#[test]
fn a_drawing_text_keeps_the_label_off_it_as_on_paper() {
    // A text 25 m high across the parcel at 1:1000 (94 px on paper): the parcel's label, kept inside, finds no place.
    let label = json!({ "placement": "center", "size": 10, "area": "parcel" });
    let big = json!({ "kind": "text", "id": 2, "layerId": "yazi", "attrs": {}, "p": { "x": X - 30.0, "y": Y + 3.0 },
                      "text": "MMMMMMMM", "height": 25.0, "rotation": 0 });
    let mut free = doc(Some(label.clone()), &[]);
    let mut covered = doc(Some(label), &[big]);
    let (a, b) = (
        report(&mut free, labels_report::Draft::default()),
        report(&mut covered, labels_report::Draft::default()),
    );
    assert_eq!((a.totals.placed, a.totals.unplaced), (1, 0), "{a:?}");
    assert_eq!(
        (b.totals.placed, b.totals.unplaced),
        (0, 1),
        "the text holds the parcel's middle: {b:?}"
    );
}
