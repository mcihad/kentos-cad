//! CAD-ADV-11: output must report the pattern actually selected by the style
//! compiler, not every definition in a drawing. All inspections are read-only.
use kentos_domain::Document;
use kentos_interaction::Spatial;
use kentos_plot::{book, maps::MapSource};
use kentos_sheet::{
    SheetBook,
    display::{self, Prim},
    project::project_sheets,
};
use serde_json::{Value, json};

fn source() -> (Value, Value) {
    let fx: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/native-preflight.json"
    ))
    .unwrap();
    (fx["model"].clone(), fx["book"].clone())
}
fn pattern(kind: &str) -> Value {
    match kind {
        "long" => {
            json!({"name":"P","unit":"paper","elements":(0..12).map(|i|json!({"length":if i%2==0 {1.} else {-1.}})).collect::<Vec<_>>()})
        }
        "shape" => {
            json!({"name":"P","unit":"paper","elements":[{"length":2.},{"length":0.,"shape":{"name":"FENCE","file":"missing.shx"}}]})
        }
        "short" => json!({"name":"P","unit":"paper","elements":[{"length":3.},{"length":-1.}]}),
        "unavailable" => json!({"name":"P","unit":"paper","elements":[{"length":2.},{"length":-1.},{"length":0.,"text":{"text":"HW","style":"MISSING_FONT_STYLE","scale":0.1}},{"length":-1.}]}),
        _ => {
            json!({"name":"P","unit":"paper","elements":[{"length":2.},{"length":-1.},{"length":0.,"text":{"text":"HW","style":"STANDARD","scale":0.1}},{"length":-1.}]})
        }
    }
}
fn document(raw: Value, page: Value) -> (Document, SheetBook) {
    let mut doc = Document::from_snapshot(serde_json::from_value(raw).unwrap()).unwrap();
    let book: SheetBook = serde_json::from_value(page).unwrap();
    doc.set_sheets(project_sheets(&book, |_| None).unwrap())
        .unwrap();
    (doc, book)
}
fn losses(doc: &Document, b: &SheetBook) -> Vec<kentos_sheet::preflight::Finding> {
    let render = book::base_inputs(doc, "Test", "2026-11-04");
    kentos_plot::preflight::inspect(doc, b, "ground", &render, None)
        .unwrap()
        .into_iter()
        .filter(|f| f.code == "linetype_unsupported")
        .collect()
}

#[test]
fn attached_shapes_print_from_embedded_bytes_and_missing_glyphs_block_the_complete_output() {
    let resource: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/kcad/v2/shape-resources.json"
    ))
    .unwrap();
    for scale in [1u32, 20, 100] {
        for wide in [false, true] {
            for glyph in ["FENCE", "1", "absent"] {
                let (mut raw, mut page) = source();
                raw["settings"]["lineTypes"] = json!([{"name":"P","unit":"paper","elements":[{"length":5.},{"length":-1.},{"length":0.,"shape":{"name":glyph,"file":"FENCE.SHX","scale":1.}},{"length":-2.}]}]);
                raw["styles"]["items"] = resource["styles"]["items"].clone();
                if wide {
                    raw["entities"][0] = json!({"kind":"polyline","id":1,"layerId":raw["entities"][0]["layerId"],"attrs":{},"pts":[{"x":0.,"y":0.},{"x":0.1,"y":0.}],"widths":[[0.001,0.003],[0.,0.]]});
                }
                raw["entities"][0]["linePattern"] = json!("P");
                page["sheets"][0]["items"][0]["kind"]["view"]["scale"] = json!(scale);
                page["sheets"][0]["items"][0]["kind"]["view"]["center"] = json!({"x":0.05,"y":0.});
                let (doc, book) = document(raw, page);
                let before = doc.to_snapshot_v2();
                assert_eq!(
                    !losses(&doc, &book).is_empty(),
                    glyph == "absent",
                    "scale={scale}, wide={wide}, glyph={glyph}"
                );
                assert_eq!(
                    book::book_pdf(&doc, &[], "Test", "2026-11-04").is_err(),
                    glyph == "absent",
                    "scale={scale}, wide={wide}, glyph={glyph}"
                );
                assert_eq!(doc.to_snapshot_v2(), before);
            }
        }
    }
}
#[test]
fn named_pattern_loss_follows_actual_visibility_and_overrides() {
    for kind in ["text", "unavailable", "shape", "long", "missing", "short"] {
        for mode in [
            "own", "layer", "unused", "outside", "hidden", "frozen", "builtin",
        ] {
            let (mut raw, mut page) = source();
            if kind != "missing" {
                raw["settings"]["lineTypes"] = json!([pattern(kind)]);
            }
            let line = &mut raw["entities"][0];
            if mode == "layer" || mode == "builtin" {
                raw["layers"][0]["style"]["linePattern"] = json!("P");
            } else if mode != "unused" {
                line["linePattern"] = json!("P");
            }
            if mode == "builtin" {
                raw["entities"][0]["lineType"] = json!("continuous");
            }
            if mode == "outside" {
                raw["entities"][0]["a"]["x"] = json!(10000.);
                raw["entities"][0]["b"]["x"] = json!(10005.);
            }
            if mode == "hidden" {
                raw["layers"][0]["visible"] = json!(false);
            }
            if mode == "frozen" {
                page["sheets"][0]["items"][0]["kind"]["layerOverrides"] =
                    json!([{"layer":"cizim","frozen":true}]);
            }
            let (doc, book) = document(raw, page);
            let before = doc.to_snapshot_v2();
            let found = losses(&doc, &book);
            let blocked = !matches!(kind, "short" | "long" | "text") && matches!(mode, "own" | "layer");
            assert_eq!(!found.is_empty(), blocked, "{kind}/{mode}: {found:?}");
            let pdf = book::book_pdf(&doc, &[], "Test", "2026-11-04");
            if blocked {
                assert!(
                    found
                        .iter()
                        .all(|f| f.severity == kentos_sheet::preflight::Severity::Error)
                );
                assert!(found[0].message.contains('P'));
                assert_eq!(
                    pdf.err().unwrap().code,
                    "linetype_unsupported",
                    "{kind}/{mode}"
                );
            } else {
                pdf.unwrap_or_else(|e| panic!("{kind}/{mode}: {e:?}"));
            }
            assert_eq!(doc.to_snapshot_v2(), before);
        }
    }
}
#[test]
fn complex_marks_without_any_gap_still_refuse_preview_vector_and_direct_cpu_output() {
    let (mut raw, page) = source();
    raw["settings"]["lineTypes"] = json!([pattern("shape")]);
    raw["entities"][0]["linePattern"] = json!("P");
    let (doc, book) = document(raw, page);
    let before = doc.to_snapshot_v2();
    assert_eq!(
        kentos_plot::preview::page(&doc, "ground", "Test", "2026-11-04", None, 72)
            .err()
            .unwrap()
            .code,
        "linetype_unsupported"
    );
    let render = book::base_inputs(&doc, "Test", "2026-11-04");
    let list = display::display_list(&book, "ground", &render).unwrap();
    let map = list
        .prims
        .iter()
        .find_map(|p| if let Prim::Map(m) = p { Some(m) } else { None })
        .unwrap();
    let mut spatial = Spatial::new();
    spatial.reload(&doc);
    let library = book::library_of(&doc);
    let c = map.view.center.unwrap();
    let src = MapSource {
        doc: &doc,
        store: spatial.store(),
        library: &library,
        origin: kentos_geometry_core::Vec2::new(c.x, c.y),
        map: None,
        level: None,
        phase: None,
        hidden: &[],
        pens: &[],
        mono: false,
        options: &[],
    };
    assert!(
        kentos_plot::maps::vector_content(
            &src,
            &map.layers,
            &map.layer_overrides,
            map.view.scale,
            map.extent.unwrap(),
            &[]
        )
        .is_err()
    );
    let error = kentos_plot::render_map_image(&src, map, &[], [64, 64], None)
        .err()
        .unwrap();
    assert!(
        error.contains('P') && error.contains("Eksik çizgi"),
        "{error}"
    );
    assert_eq!(doc.to_snapshot_v2(), before);
}
#[test]
fn explicit_supported_symbol_does_not_inherit_unused_layer_pattern_loss() {
    let (mut raw, page) = source();
    raw["settings"]["lineTypes"] = json!([pattern("text")]);
    raw["layers"][0]["style"]["linePattern"] = json!("P");
    raw["entities"][0]["symbol"] = json!("line-ok");
    raw["styles"]["items"] = json!([{"id":"line-ok","kind":"symbol","name":"Solid","symbol":{"type":"line","layers":[{"id":"stroke","type":"simpleLine","color":"#111111","width":0.25}]}}]);
    let (doc, book) = document(raw, page);
    assert!(losses(&doc, &book).is_empty());
    book::book_pdf(&doc, &[], "Test", "2026-11-04").unwrap();
}
#[test]
fn block_own_and_inherited_patterns_are_checked_only_for_visible_leaves() {
    for mode in ["own", "byblock", "hidden", "clipped", "outside"] {
        let (mut raw, page) = source();
        raw["settings"]["lineTypes"] = json!([pattern("unavailable")]);
        let block = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4051";
        let mut leaf = json!({"kind":"line","id":17,"layerId":"cizim","attrs":{},"a":{"x":0.,"y":0.},"b":{"x":2.,"y":0.},"linePattern":"P"});
        let mut insert = json!({"kind":"insert","id":1,"layerId":"cizim","attrs":{},"block":block,"p":{"x":0.,"y":0.},"scale":1.,"rotation":0.});
        if mode == "byblock" {
            leaf.as_object_mut().unwrap().remove("linePattern");
            insert["linePattern"] = json!("P");
        }
        if mode == "hidden" {
            leaf["layerId"] = json!("resources");
        }
        if mode == "outside" {
            insert["p"]["x"] = json!(10000.);
        }
        if mode == "clipped" {
            insert["clipping"] = json!({"boundary":[{"x":100.,"y":100.},{"x":101.,"y":100.},{"x":101.,"y":101.},{"x":100.,"y":101.}]});
        }
        raw["entities"] = json!([insert]);
        raw["blocks"] = json!([{"id":block,"name":"Pipe","base":{"x":0.,"y":0.},"units":"m","entities":[leaf]}]);
        let (doc, book) = document(raw, page);
        assert_eq!(
            !losses(&doc, &book).is_empty(),
            matches!(mode, "own" | "byblock"),
            "{mode}"
        );
    }
}

#[test]
fn long_dash_custom_symbol_is_supported_without_a_named_pattern() {
    let (mut raw, page) = source();
    raw["entities"][0]["symbol"] = json!("line-long");
    raw["styles"]["items"] = json!([{"id":"line-long","kind":"symbol","name":"Long","symbol":{"type":"line","layers":[{"id":"stroke","type":"simpleLine","color":"#111111","width":0.25,"dash":[1.,1.,2.,1.,3.,1.,4.,1.,5.,1.]}]}}]);
    let (doc, book) = document(raw, page);
    assert!(losses(&doc, &book).is_empty());
    book::book_pdf(&doc, &[], "Test", "2026-11-04").unwrap();
}

#[test]
fn named_patterns_keep_either_paper_size_or_drawing_length_across_viewports() {
    for (drawing_unit, metres) in [("mm", 0.001), ("cm", 0.01), ("m", 1.), ("ft", 0.3048)] {
        for pattern_unit in ["paper", "drawing"] {
            for (global, own) in [(1., 1.), (2., 3.)] {
                for scale in [1., 20., 100.] {
                    let (mut raw, mut page) = source();
                    let lengths: [f64; 12] = [1., -1., 2., -1., 3., -1., 4., -1., 5., -1., 6., -1.];
                    raw["settings"]["drawingUnit"] = json!(drawing_unit);
                    if global != 1. {
                        raw["settings"]["lineTypeScale"] = json!(global);
                    }
                    if own != 1. {
                        raw["entities"][0]["lineTypeScale"] = json!(own);
                    }
                    raw["settings"]["lineTypes"] = json!([{"name":"FULL12","unit":pattern_unit,
            "elements":lengths.iter().map(|l|json!({"length":l})).collect::<Vec<_>>() }]);
                    raw["entities"][0]["linePattern"] = json!("FULL12");
                    page["sheets"][0]["items"][0]["kind"]["view"]["scale"] = json!(scale as u32);
                    let (doc, book) = document(raw, page);
                    let before = doc.to_snapshot_v2();
                    let render = book::base_inputs(&doc, "Test", "2026-11-04");
                    let list = display::display_list(&book, "ground", &render).unwrap();
                    let map = list
                        .prims
                        .iter()
                        .find_map(|p| if let Prim::Map(m) = p { Some(m) } else { None })
                        .unwrap();
                    let mut spatial = Spatial::new();
                    spatial.reload(&doc);
                    let library = book::library_of(&doc);
                    let c = map.view.center.unwrap();
                    let src = MapSource {
                        doc: &doc,
                        store: spatial.store(),
                        library: &library,
                        origin: kentos_geometry_core::Vec2::new(c.x, c.y),
                        map: None,
                        level: None,
                        phase: None,
                        hidden: &[],
                        pens: &[],
                        mono: false,
                        options: &[],
                    };
                    let kentos_sheet::pdf::MapContent::Vector(v) =
                        kentos_plot::maps::vector_content(
                            &src,
                            &map.layers,
                            &map.layer_overrides,
                            scale,
                            map.extent.unwrap(),
                            &[],
                        )
                        .unwrap()
                    else {
                        panic!("vector output expected")
                    };
                    let dashes: Vec<_> = v
                        .layers
                        .iter()
                        .flat_map(|l| &l.paths)
                        .filter_map(|p| p.stroke.as_ref())
                        .filter(|s| !s.dash.is_empty())
                        .collect();
                    assert_eq!(dashes.len(), 1);
                    assert_eq!(dashes[0].dash.len(), 12);
                    for (actual, expected) in dashes[0].dash.iter().zip(lengths) {
                        let expected = expected.abs()
                            * global
                            * own
                            * if pattern_unit == "paper" {
                                1.
                            } else {
                                metres * 1000. / scale
                            };
                        assert!(
                            (actual - expected).abs() < 1e-9,
                            "scale {scale}: {actual} vs {expected}"
                        );
                    }
                    book::book_pdf(&doc, &[], "Test", "2026-11-04").unwrap();
                    assert_eq!(doc.to_snapshot_v2(), before);
                }
            }
        }
    }
}

#[test]
fn long_custom_stroke_phase_reaches_the_actual_pdf_content_stream() {
    // A hand-summed 27 mm recipe; phase is consumed distance, as on screen.
    let dash = [1., 1., 2., 1., 3., 1., 4., 1., 5., 1., 6., 1.];
    for phase in [0., 1.25, -1.25] {
        let (mut raw, page) = source();
        raw["entities"][0]["symbol"] = json!("phase");
        raw["styles"]["items"] = json!([{"id":"phase","kind":"symbol","name":"Phase",
            "symbol":{"type":"line","layers":[{"id":"l","type":"simpleLine",
                "unit":"mm","width":0.25,"color":"#111111","dash":dash,"dashOffset":phase}]}}]);
        let (doc, _book) = document(raw, page);
        let pdf = book::book_pdf(&doc, &[], "Test", "2026-11-04").unwrap();
        let streams = crate::files::pdf_streams(&pdf.bytes);
        let lines: Vec<_> = streams
            .iter()
            .filter_map(|s| std::str::from_utf8(s).ok())
            .flat_map(str::lines)
            .filter(|l| l.ends_with(" d") && l.starts_with('[') && !l.starts_with("[]"))
            .collect();
        assert_eq!(lines.len(), 1, "{lines:?}");
        let (lengths, rest) = lines[0].split_once(']').unwrap();
        assert_eq!(lengths[1..].split_whitespace().count(), 12);
        let written: f64 = rest.split_whitespace().next().unwrap().parse().unwrap();
        let want = if phase < 0. { 25.75 } else { phase } * 72. / 25.4;
        assert!(
            (written - want).abs() < 0.00002,
            "phase {phase}: {written} vs {want}"
        );
    }
}

#[test]
fn text_mark_workload_limit_is_a_visible_read_only_preflight_error() {
    let (mut raw,page)=source();
    let mut p=pattern("text");
    // Ten million repeats on the visible line must not allocate a prefix
    // and then pretend that the omitted suffix was rendered.
    for e in p["elements"].as_array_mut().unwrap() {
        if e["length"].as_f64().is_some_and(|n|n!=0.) {e["length"]=json!(e["length"].as_f64().unwrap()*1e-7);}
    }
    raw["settings"]["lineTypes"]=json!([p]);raw["entities"][0]["linePattern"]=json!("P");
    let (doc,book)=document(raw,page);let before=doc.to_snapshot_v2();
    assert!(!losses(&doc,&book).is_empty());
    assert_eq!(book::book_pdf(&doc,&[],"Test","2026-11-04").unwrap_err().code,"linetype_unsupported");
    assert_eq!(doc.to_snapshot_v2(),before);
}

#[test]
fn geometric_width_patterns_are_inspected_before_pdf_delivery() {
    for mode in ["short","long","text","unavailable","missing","shape","dense","dot","builtin"] {
        let (mut raw,page)=source();
        let def=match mode {
            "dense" => json!({"name":"P","unit":"paper","elements":[{"length":0.00001},{"length":-0.00001}]}),
            "dot" => json!({"name":"P","unit":"paper","elements":[{"length":0.},{"length":-2.}]}),
            _ => pattern(mode),
        };
        if mode!="missing" {raw["settings"]["lineTypes"]=json!([def]);}
        raw["entities"][0]=json!({"kind":"polyline","id":1,"layerId":"cizim","attrs":{},
            "pts":[{"x":0.,"y":0.},{"x":2.,"y":0.}],"widths":[[0.1,0.3],[0.,0.]],"linePattern":"P"});
        if mode=="builtin" {raw["entities"][0].as_object_mut().unwrap().remove("linePattern");raw["entities"][0]["lineType"]=json!("dashed");}
        let (doc,book)=document(raw,page);
        let before=doc.to_snapshot_v2();
        let blocked=matches!(mode,"unavailable"|"missing"|"shape"|"dense"|"dot");
        assert_eq!(!losses(&doc,&book).is_empty(),blocked,"{mode}");
        let pdf=book::book_pdf(&doc,&[],"Test","2026-11-04");
        if blocked {assert_eq!(pdf.err().unwrap().code,"linetype_unsupported","{mode}");}
        else {pdf.unwrap_or_else(|e|panic!("{mode}: {e:?}"));}
        assert_eq!(doc.to_snapshot_v2(),before);
    }
}

#[test]
fn curved_shx_anchors_use_exact_arc_distance_in_actual_vector_and_pdf_output() {
    let resource: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/kcad/v2/shape-resources.json"
    ))
    .unwrap();
    for scale in [1u32, 20, 100] {
        for kind in ["arc", "circle", "ccw", "cw", "wide", "ellipse"] {
            let (mut raw, mut page) = source();
            let r = scale as f64 * 0.01;
            raw["settings"]["lineTypes"] = json!([{"name":"P","unit":"paper","elements":[{"length":5.},{"length":-1.},{"length":0.,"shape":{"name":"FENCE","file":"fence.shx"}},{"length":-2.}]}]);
            raw["styles"]["items"] = resource["styles"]["items"].clone();
            let mut entity = match kind {
                "arc" => {
                    json!({"kind":"arc","c":{"x":0.,"y":0.},"r":r,"a0":0.,"a1":std::f64::consts::PI})
                }
                "circle" => json!({"kind":"circle","c":{"x":0.,"y":0.},"r":r}),
                "ellipse" => {
                    json!({"kind":"ellipse","c":{"x":0.,"y":0.},"major":{"x":r,"y":0.},"ratio":0.5,"t0":0.,"t1":0.})
                }
                _ => {
                    json!({"kind":"polyline","pts":[{"x":r,"y":0.},{"x":-r,"y":0.}],"bulges":[if kind=="cw"{-1.}else{1.}]})
                }
            };
            if kind == "wide" {
                entity["widths"] = json!([[r * 0.02, r * 0.04], [0., 0.]]);
            }
            entity["id"] = 1.into();
            entity["layerId"] = raw["entities"][0]["layerId"].clone();
            entity["attrs"] = json!({});
            entity["linePattern"] = "P".into();
            raw["entities"][0] = entity;
            page["sheets"][0]["items"][0]["kind"]["view"]["scale"] = json!(scale);
            page["sheets"][0]["items"][0]["kind"]["view"]["center"] = json!({"x":0.,"y":0.});
            let (doc, book) = document(raw, page);
            let before = doc.to_snapshot_v2();
            assert!(losses(&doc, &book).is_empty(), "{scale}/{kind}");
            let render = book::base_inputs(&doc, "Test", "2026-11-04");
            let list = display::display_list(&book, "ground", &render).unwrap();
            let map = list
                .prims
                .iter()
                .find_map(|p| if let Prim::Map(m) = p { Some(m) } else { None })
                .unwrap();
            let mut spatial = Spatial::new();
            spatial.reload(&doc);
            let library = book::library_of(&doc);
            let src = MapSource {
                doc: &doc,
                store: spatial.store(),
                library: &library,
                origin: kentos_geometry_core::Vec2::new(0., 0.),
                map: None,
                level: None,
                phase: None,
                hidden: &[],
                pens: &[],
                mono: false,
                options: &[],
            };
            let kentos_sheet::pdf::MapContent::Vector(v) = kentos_plot::maps::vector_content(
                &src,
                &map.layers,
                &map.layer_overrides,
                scale as f64,
                map.extent.unwrap(),
                &[],
            )
            .unwrap() else {
                panic!("expected vectors")
            };
            let marks: Vec<_> = v
                .layers
                .iter()
                .flat_map(|l| &l.paths)
                .filter(|p| {
                    p.points.len() == 2 && p.stroke.as_ref().is_some_and(|s| s.dash.is_empty())
                })
                .collect();
            assert_eq!(
                marks.len(),
                if kind == "circle" {
                    16
                } else if kind == "ellipse" {
                    12
                } else {
                    8
                },
                "{scale}/{kind}"
            );
            for (n, mark) in marks.iter().enumerate() {
                let angle = (if kind == "cw" { -1. } else { 1. }) * (0.6 + (n / 2) as f64 * 0.8);
                let center = if kind == "ellipse" {
                    // Independent mpmath Legendre E inverse at s/r=.6+.8*n.
                    let points = [
                        [0.5996527729281464, 0.4001301513006689],
                        [-0.1886584527989604, 0.49102138145591545],
                        [-0.9161435650074582, 0.20042515329632737],
                        [-0.6202885168850302, -0.3921868673925],
                        [0.16663605360504063, -0.49300923562316207],
                        [0.9011903450343653, -0.21671176826423239],
                    ];
                    let p = points[n / 2];
                    [r * p[0], r * p[1]]
                } else {
                    [r * angle.cos(), r * angle.sin()]
                };
                for axis in 0..2 {
                    let actual = (mark.points[0][axis] + mark.points[1][axis]) * 0.5;
                    assert!(
                        (actual - center[axis]).abs() < 1e-11,
                        "{scale}/{kind}/{n}: {actual} != {}",
                        center[axis]
                    );
                }
            }
            book::book_pdf(&doc, &[], "Test", "2026-11-04").unwrap();
            assert_eq!(doc.to_snapshot_v2(), before);
        }
    }
}
