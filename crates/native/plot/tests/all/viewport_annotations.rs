use kentos_contracts::DocumentSnapshotV1;
use kentos_domain::Document;
use kentos_interaction::Spatial;
use kentos_geometry_core::Vec2;
use kentos_plot::{
    maps::{MapSource, vector_content},
    book::library_of,
};
use kentos_sheet::{
    *,
    display::{self, Prim, RenderInputs},
    ops::{self, Op},
    pdf::MapContent,
};
#[test]
fn twenty_native_plot_cases_combine_representations_freeze_and_cad_overrides_without_mutating_model()
 {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/viewport-annotations.json"
    ))
    .unwrap();
    for c in data["cases"].as_array().unwrap() {
        let snapshot: DocumentSnapshotV1 = serde_json::from_value(c["model"].clone()).unwrap();
        let doc = Document::from_snapshot(snapshot).unwrap();
        let before = doc.to_snapshot_v2();
        let mut book = SheetBook::default();
        book.sheets
            .push(serde_json::from_value(c["sheet"].clone()).unwrap());
        let op: Op = serde_json::from_value(c["operation"].clone()).unwrap();
        let book = ops::apply(&book, &op).unwrap().book;
        let list = display::display_list(&book, "page", &RenderInputs::default()).unwrap();
        let m = list
            .prims
            .iter()
            .find_map(|p| match p {
                Prim::Map(m) => Some(m),
                _ => None,
            })
            .unwrap();
        let scaled = doc.at_scale(m.view.annotation_denominator());
        let source = scaled.as_ref().unwrap_or(&doc);
        let mut spatial = Spatial::new();
        spatial.set_wall_scale(Some(m.view.scale));
        spatial.reload(source);
        let library = library_of(source);
        let src = MapSource {
            map: None,
            level: m.level.as_ref(),
            doc: source,
            store: spatial.store(),
            library: &library,
            origin: Vec2::new(0., 0.),
        };
        let labels = kentos_plot::labels::map_labels(source, &spatial, m);
        let text = labels
            .iter()
            .filter(|t| t.text == "ANNO")
            .collect::<Vec<_>>();
        assert_eq!(
            text.len(),
            usize::from(c["expected"]["visible"].as_bool().unwrap()),
            "{} labels {labels:?}",
            c["name"]
        );
        if let Some(t) = text.first() {
            assert!(
                t.color
                    .eq_ignore_ascii_case(c["expected"]["color"].as_str().unwrap())
            );
            assert!(
                (t.size - c["expected"]["heightPaperMm"].as_f64().unwrap()).abs() < 1e-5,
                "{} text {}",
                c["name"],
                t.size
            );
        }
        let MapContent::Vector(vector) = vector_content(
            &src,
            &m.layers,
            &m.layer_overrides,
            m.view.scale,
            m.extent.unwrap(),
            &labels,
        )
        .unwrap() else {
            panic!()
        };
        let strokes = vector
            .layers
            .iter()
            .flat_map(|l| l.paths.iter())
            .filter_map(|p| p.stroke.as_ref())
            .collect::<Vec<_>>();
        let color = c["expected"]["color"].as_str().unwrap();
        let changed = strokes
            .iter()
            .find(|s| s.color.eq_ignore_ascii_case(color))
            .unwrap_or_else(|| panic!("{}: {strokes:?}", c["name"]));
        assert!((changed.width - c["expected"]["weightUm"].as_f64().unwrap() / 1000.).abs() < 1e-5);
        assert_eq!(
            !changed.dash.is_empty(),
            c["expected"]["dash"].as_bool().unwrap()
        );
        assert!(
            strokes
                .iter()
                .any(|s| s.color.eq_ignore_ascii_case("#00AA00")),
            "explicit entity colour survives a ByLayer override"
        );
        assert_eq!(doc.to_snapshot_v2(), before);
        assert_eq!(doc.entities().count(), 3);
    }
}

#[test]
fn a_viewport_theme_can_show_globally_hidden_text_and_its_layer_list_can_hide_it() {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/viewport-annotations.json"
    ))
    .unwrap();
    let c = &data["cases"][12];
    let mut snapshot: DocumentSnapshotV1 = serde_json::from_value(c["model"].clone()).unwrap();
    snapshot.layers[1].visible = false;
    snapshot.settings.layer_states = serde_json::from_value(
        serde_json::json!([{"id":"print","name":"Baskı","nodes":[{"node":"anno","visible":true}]}]),
    )
    .unwrap();
    let doc = Document::from_snapshot(snapshot).unwrap();
    let spatial = Spatial::of(&doc);
    let mut book = SheetBook::default();
    book.sheets
        .push(serde_json::from_value(c["sheet"].clone()).unwrap());
    let list = display::display_list(&book, "page", &RenderInputs::default()).unwrap();
    let mut m = list
        .prims
        .iter()
        .find_map(|p| match p {
            Prim::Map(m) => Some(m.clone()),
            _ => None,
        })
        .unwrap();
    assert!(kentos_plot::labels::map_labels(&doc, &spatial, &m).is_empty());
    m.layers = serde_json::from_value(serde_json::json!({"type":"theme","name":"Baskı"})).unwrap();
    assert_eq!(
        kentos_plot::labels::map_labels(&doc, &spatial, &m)
            .iter()
            .filter(|l| l.text == "ANNO")
            .count(),
        1
    );
    m.layers =
        serde_json::from_value(serde_json::json!({"type":"list","layers":["geom"]})).unwrap();
    assert!(kentos_plot::labels::map_labels(&doc, &spatial, &m).is_empty());
}
