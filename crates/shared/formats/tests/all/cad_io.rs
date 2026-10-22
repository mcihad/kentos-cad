//! CAD-IO: independent minimal group streams and numeric expectations.
use kentos_contracts::DxfReadOptions;
use kentos_formats::dxf::{self, preflight};

fn drawing(version: &str, encoding: &str, entities: &str) -> Vec<u8> {
    format!("0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\n{version}\n9\n$DWGCODEPAGE\n3\n{encoding}\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n{entities}0\nENDSEC\n0\nEOF\n").into_bytes()
}

#[test]
fn declared_ascii_versions_read_and_future_or_binary_encodings_refuse_with_codes() {
    for version in preflight::ASCII_VERSIONS {
        let bytes = drawing(
            version,
            "ANSI_1254",
            "0\nLINE\n5\nAB\n8\n0\n10\n1\n20\n2\n11\n3\n21\n4\n",
        );
        let result = dxf::read_checked(&bytes, &DxfReadOptions::default()).expect(version);
        assert_eq!(result.entities.len(), 1, "{version}");
    }
    for (bytes, code) in [
        (drawing("AC1033", "UTF-8", ""), "unsupported_cad_version"),
        (
            drawing("AC1015", "ANSI_1251", ""),
            "unsupported_cad_encoding",
        ),
        (
            b"AutoCAD Binary DXF\r\n\x1a\0".to_vec(),
            "unsupported_cad_encoding",
        ),
        (vec![0xFF, 0xFE, 0, 0], "unsupported_cad_encoding"),
        (b"AC1032\0\0".to_vec(), "unsupported_cad_format"),
    ] {
        assert_eq!(
            dxf::read_checked(&bytes, &Default::default())
                .expect_err("refused")
                .code,
            code
        );
    }
}

#[test]
fn structurally_truncated_or_nested_sections_never_return_partial_geometry() {
    let valid = drawing(
        "AC1021",
        "UTF-8",
        "0\nLINE\n8\n0\n10\n1\n20\n2\n11\n3\n21\n4\n",
    );
    for tail in [2, 6, 14, 20] {
        let truncated = &valid[..valid.len() - tail];
        assert!(dxf::read_checked(truncated, &Default::default()).is_err());
    }
    for body in [
        "0\nSECTION\n2\nHEADER\n0\nEOF\n",
        "0\nSECTION\n2\nENTITIES\n0\nBLOCK\n0\nENDSEC\n0\nEOF\n",
        "0\nSECTION\n2\nHEADER\n0\nSECTION\n2\nENTITIES\n0\nENDSEC\n0\nEOF\n",
    ] {
        assert_eq!(
            dxf::read_checked(body.as_bytes(), &Default::default())
                .expect_err("broken")
                .code,
            "import_unreadable"
        );
    }
}

#[test]
fn foreign_rational_spline_keeps_its_exact_definition_without_kentos_metadata() {
    use kentos_contracts::{DrawingUnit, Entity};
    let source = drawing(
        "AC1032",
        "UTF-8",
        "0\nSPLINE\n5\nAB\n8\n0\n70\n4\n71\n2\n72\n6\n73\n3\n40\n0\n40\n0\n40\n0\n40\n1\n40\n1\n40\n1\n41\n1\n41\n0.7071067811865476\n41\n1\n10\n1\n20\n0\n10\n1\n20\n1\n10\n0\n20\n1\n",
    );
    let read = dxf::read(
        &source,
        &DxfReadOptions {
            unit: Some(DrawingUnit::M),
            ..Default::default()
        },
    )
    .expect("foreign rational curve");
    let Entity::Nurbs(n) = &read.entities[0] else {
        panic!("not a native NURBS");
    };
    assert_eq!(n.degree, 2);
    assert_eq!(n.knots, [0., 0., 0., 1., 1., 1.]);
    assert_eq!(
        n.weights.as_deref(),
        Some([1., std::f64::consts::FRAC_1_SQRT_2, 1.].as_slice())
    );
    assert_eq!((n.ctrl[0].x, n.ctrl[1].y, n.ctrl[2].y), (1., 1., 1.));
    let input = dxf::input_from_json(
        &serde_json::json!({"entities": read.entities,
        "layers": [], "scale": 1000., "lengthDecimals": 3, "grads": false})
        .to_string(),
    )
    .expect("writer input");
    let (bytes, _) = dxf::write(&input);
    let back = dxf::read(
        &bytes,
        &DxfReadOptions {
            unit: Some(DrawingUnit::M),
            ..Default::default()
        },
    )
    .expect("back");
    let Entity::Nurbs(b) = &back.entities[0] else {
        panic!("lost NURBS");
    };
    assert_eq!(
        (b.degree, &b.knots, &b.weights, &b.ctrl),
        (n.degree, &n.knots, &n.weights, &n.ctrl)
    );
}

#[test]
fn foreign_xref_path_attach_overlay_and_nonuniform_insert_survive_without_bind() {
    for (flags, mode) in [
        (4, kentos_contracts::XrefKind::Attach),
        (12, kentos_contracts::XrefKind::Overlay),
    ] {
        let source = format!(
            "0\nSECTION\n2\nBLOCKS\n0\nBLOCK\n2\nREF\n70\n{flags}\n1\nrefs/plan.dwg\n10\n0\n20\n0\n0\nENDBLK\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nINSERT\n5\nA1\n2\nREF\n8\n0\n10\n10\n20\n20\n41\n2\n42\n3\n50\n90\n0\nENDSEC\n0\nEOF\n"
        );
        let read = dxf::read(source.as_bytes(), &Default::default()).expect("xref");
        let x = read.blocks[0].xref.as_ref().expect("reference source");
        assert_eq!(
            (&x.path, x.kind, x.unloaded),
            (&"refs/plan.dwg".to_owned(), mode, true)
        );
        let kentos_contracts::Entity::Insert(i) = &read.entities[0] else {
            panic!("insert");
        };
        assert_eq!((i.p.x, i.p.y, i.scale, i.y_scale), (10., 20., 2., Some(3.)));
        let input = dxf::input_from_json(
            &serde_json::json!({"entities":read.entities,"blocks":read.blocks,
            "layers":[],"scale":1000.,"lengthDecimals":3,"grads":false})
            .to_string(),
        )
        .expect("input");
        let (bytes, _) = dxf::write(&input);
        let back = dxf::read(&bytes, &Default::default()).expect("xref back");
        assert_eq!(back.blocks[0].xref.as_ref().expect("source").kind, mode);
        assert_eq!(
            back.blocks[0].xref.as_ref().expect("source").path,
            "refs/plan.dwg"
        );
    }
}

#[test]
fn sortents_reorders_handles_before_emission_and_zero_is_drawn_last() {
    let mut bytes = drawing(
        "AC1021",
        "UTF-8",
        "0\nPOINT\n5\nA\n8\n0\n10\n10\n20\n0\n0\nPOINT\n5\nB\n8\n0\n10\n20\n20\n0\n0\nPOINT\n5\nC\n8\n0\n10\n30\n20\n0\n",
    );
    bytes.truncate(bytes.len() - "0\nEOF\n".len());
    bytes.extend_from_slice(b"0\nSECTION\n2\nOBJECTS\n0\nSORTENTSTABLE\n5\nEE\n100\nAcDbSortentsTable\n330\n17\n331\nA\n5\n0\n331\nB\n5\n2\n331\nC\n5\n1\n0\nENDSEC\n0\nEOF\n");
    let result = dxf::read(&bytes, &Default::default()).expect("draw order");
    let x: Vec<_> = result
        .entities
        .iter()
        .map(|e| match e {
            kentos_contracts::Entity::Point(p) => p.p.x,
            _ => panic!("point"),
        })
        .collect();
    assert_eq!(x, [30., 20., 10.]);
}

#[test]
fn loss_samples_expose_real_handle_type_property_line_and_severity() {
    let bytes = drawing(
        "AC1021",
        "UTF-8",
        "0\n3DSOLID\n5\nABC\n8\n0\n100\nAcDbModelerGeometry\n70\n1\n1\nACIS\n",
    );
    let result = dxf::read(&bytes, &Default::default()).expect("reported unsupported entity");
    let loss = result
        .report
        .skipped
        .iter()
        .find(|r| r.what == "3DSOLID")
        .expect("loss");
    assert_eq!(loss.severity, Some(kentos_contracts::ReportSeverity::Error));
    assert_eq!(loss.details[0].entity_type, "3DSOLID");
    assert_eq!(loss.details[0].source_id.as_deref(), Some("ABC"));
    assert_eq!(loss.details[0].property, "entity");
    assert_eq!(loss.details[0].source_line, loss.lines.first().copied());
}

#[test]
fn foreign_layouts_paper_entities_and_viewports_cross_the_writer_as_real_records() {
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/formats/v1/cad-io/layouts.dxf"),
    )
    .expect("fixture");
    let read = dxf::read(
        &bytes,
        &DxfReadOptions {
            unit: Some(kentos_contracts::DrawingUnit::M),
            ..Default::default()
        },
    )
    .expect("foreign layouts");
    assert_eq!(
        read.entities.len(),
        1,
        "paper entities do not leak into model space"
    );
    assert_eq!(read.layouts.len(), 2);
    assert_eq!(
        (read.layouts[0].width_mm, read.layouts[1].height_mm),
        (420., 297.)
    );
    assert_eq!(read.layouts[0].entities.len(), 2);
    assert_eq!(
        read.layouts[1].entities.len(),
        1,
        "inactive paperspace block"
    );
    let viewport = &read.layouts[0].viewports[0];
    assert!((viewport.view_center.x - 90.).abs() < 1e-10);
    assert!((viewport.view_center.y - 205.).abs() < 1e-10);
    assert_eq!(viewport.view_height, 200.);
    let input = dxf::input_from_json(
        &serde_json::json!({"entities":read.entities,"layouts":read.layouts,
        "layers":[],"scale":1000.,"lengthDecimals":3,"grads":false})
        .to_string(),
    )
    .expect("input");
    let (bytes, report) = dxf::write(&input);
    assert!(report.skipped.is_empty(), "{report:?}");
    let back = dxf::read(
        &bytes,
        &DxfReadOptions {
            unit: Some(kentos_contracts::DrawingUnit::M),
            ..Default::default()
        },
    )
    .expect("real layouts back");
    assert_eq!(back.entities.len(), 1);
    assert_eq!(back.layouts.len(), 2);
    assert_eq!(back.layouts[0].entities.len(), 2);
    assert_eq!(back.layouts[1].entities.len(), 1);
    assert!((back.layouts[0].viewports[0].view_center.x - 90.).abs() < 1e-10);
    let kentos_contracts::Entity::Line(line) = &back.layouts[0].entities[0] else {
        panic!("paper line");
    };
    assert_eq!(
        (line.a.x, line.a.y, line.b.x, line.b.y),
        (0.01, 0.02, 0.03, 0.04)
    );
}

#[test]
fn planar_widths_bulges_closure_and_elevation_are_standard_dxf_without_metadata() {
    use kentos_contracts::{ClosedPolylines, DrawingUnit, Entity};
    for kind in ["LWPOLYLINE", "POLYLINE"] {
        let body = if kind == "LWPOLYLINE" {
            "0\nLWPOLYLINE\n5\nAF\n8\n0\n90\n3\n70\n1\n38\n7\n10\n0\n20\n0\n40\n0.2\n41\n0.4\n42\n0.5\n10\n3\n20\n0\n40\n0.3\n41\n0.3\n10\n3\n20\n4\n40\n0.1\n41\n0.2\n"
        } else {
            "0\nPOLYLINE\n5\nAF\n8\n0\n70\n1\n10\n0\n20\n0\n30\n7\n0\nVERTEX\n10\n0\n20\n0\n40\n0.2\n41\n0.4\n42\n0.5\n0\nVERTEX\n10\n3\n20\n0\n40\n0.3\n41\n0.3\n0\nVERTEX\n10\n3\n20\n4\n40\n0.1\n41\n0.2\n0\nSEQEND\n"
        };
        let options = DxfReadOptions {
            unit: Some(DrawingUnit::M),
            closed_polylines: ClosedPolylines::Line,
            ..Default::default()
        };
        let read = dxf::read(&drawing("AC1032", "UTF-8", body), &options).expect("source");
        let Entity::Polyline(p) = &read.entities[0] else {
            panic!("polyline");
        };
        assert!(p.closed);
        assert_eq!(p.bulges.as_ref().expect("bulge")[0], 0.5);
        assert_eq!(p.widths.as_ref().expect("width")[0], [0.2, 0.4]);
        assert_eq!(p.zs, Some(vec![Some(7.); 3]));
        let input = dxf::input_from_json(&serde_json::json!({"entities":read.entities,"layers":[],"scale":1000.,"lengthDecimals":3,"grads":false,"annotations":{"metadata":false}}).to_string()).expect("input");
        let (bytes, _) = dxf::write(&input);
        // Annotation metadata is a separate switch; remove all XDATA so this
        // assertion proves standard 38/40/41/42/70 behavior without a private payload.
        let text = String::from_utf8(bytes).unwrap();
        let lines: Vec<_> = text.lines().collect();
        let bytes = lines.chunks_exact(2).filter(|p| p[0].trim().parse::<i32>().unwrap() < 1000)
            .map(|p| format!("{}\n{}\n",p[0],p[1])).collect::<String>().into_bytes();
        let back = dxf::read(&bytes, &options).expect("back");
        let Entity::Polyline(b) = &back.entities[0] else {
            panic!("polyline");
        };
        assert_eq!(
            (&b.pts, &b.widths, &b.bulges, &b.zs, b.closed),
            (&p.pts, &p.widths, &p.bulges, &p.zs, p.closed)
        );
    }
}

#[test]
fn json_writer_preserves_flattened_object_overrides_and_polyline_fields() {
    let input = serde_json::json!({"entities":[{"kind":"polyline","id":1,"layerId":"0","attrs":{},
        "pts":[{"x":0.,"y":0.},{"x":3.,"y":4.}],"closed":true,"widths":[[0.2,0.4],[0.1,0.2]],
        "lineType":"dashed","lineTypeScale":2.,"transparency":0.5,"order":12}],
        "layers":[],"scale":1000.,"lengthDecimals":3,"grads":false});
    let optimized = dxf::input_from_json(&input.to_string()).expect("streaming input");
    let derived: kentos_contracts::DxfWriteInput = serde_json::from_value(input).expect("contract");
    assert_eq!(optimized.entities, derived.entities);
}

#[test]
fn twenty_independent_polyline_sources_refuse_by_default_and_allow_only_explicit_loss() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/formats/v1/cad-io");
    let mut count = 0;
    for entry in std::fs::read_dir(root).unwrap() {
        let file = entry.unwrap().path();
        let name = file.file_name().unwrap().to_string_lossy();
        if !(name.starts_with("ocs-") || name.starts_with("nonuniform-")) { continue; }
        let bytes = std::fs::read(&file).unwrap();
        let mut opts = DxfReadOptions { explode_blocks: name.starts_with("nonuniform-"), ..Default::default() };
        let error = dxf::read_checked(&bytes, &opts).expect_err(&name);
        assert_eq!(error.code, "polyline_geometry_loss", "{name}");
        opts.allow_polyline_loss = true;
        let read = dxf::read_checked(&bytes, &opts).expect(&name);
        assert_eq!(read.entities.len(), 1, "{name}");
        let loss = read.report.notes.iter().find(|r| r.details.iter().any(|d| d.property.starts_with("polyline.loss."))).expect("structured property");
        assert_eq!(loss.severity, Some(kentos_contracts::ReportSeverity::Warning));
        assert!(loss.details.len() <= 5);
        assert!(loss.details.iter().any(|d| d.source_id.as_deref() == Some("AF")), "{name}: {loss:?}");
        count += 1;
    }
    assert_eq!(count, 20);
}

#[test]
fn explicit_zero_vertex_width_overrides_constant_width_instead_of_falling_back() {
    let bytes = drawing("AC1032", "UTF-8", "0\nLWPOLYLINE\n90\n2\n43\n7\n10\n0\n20\n0\n40\n0\n41\n0\n10\n3\n20\n4\n40\n0\n41\n0\n");
    let read = dxf::read_checked(&bytes, &Default::default()).unwrap();
    let kentos_contracts::Entity::Polyline(p) = &read.entities[0] else { panic!("polyline") };
    assert_eq!(p.widths, None);
}

#[test]
fn nonuniform_and_mirrored_block_factors_survive_the_streaming_json_writer_exactly() {
    use kentos_contracts::Entity;
    for (sx,sy,angle) in [(2.0f64,3.0f64,90.),(-2.,3.,30.),(2.,-3.,270.),(1.,1.0000000001,0.)] {
        let body = format!("0\nSECTION\n2\nBLOCKS\n0\nBLOCK\n2\nB\n70\n0\n10\n0\n20\n0\n0\nLWPOLYLINE\n90\n2\n43\n0.2\n10\n0\n20\n0\n42\n0.5\n10\n3\n20\n0\n0\nENDBLK\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nINSERT\n2\nB\n10\n10\n20\n20\n41\n{sx}\n42\n{sy}\n50\n{angle}\n0\nENDSEC\n0\nEOF\n");
        let read = dxf::read_checked(body.as_bytes(), &Default::default()).unwrap();
        let Entity::Insert(i) = &read.entities[0] else { panic!("insert") };
        assert_eq!((i.scale,i.y_scale,i.mirror), (sx.abs(),Some(sy.abs()),sx*sy<0.));
        let input = dxf::input_from_json(&serde_json::json!({"entities":read.entities,"blocks":read.blocks,"layers":[],"scale":1000.,"lengthDecimals":3,"grads":false}).to_string()).unwrap();
        assert_eq!(input.entities, read.entities, "JSON must not drop yScale");
        let (bytes, _) = dxf::write_checked(&input).unwrap();
        let back = dxf::read_checked(&bytes, &Default::default()).unwrap();
        let Entity::Insert(b) = &back.entities[0] else { panic!("insert back") };
        assert_eq!((i.p,i.scale,i.y_scale,i.rotation,i.mirror),(b.p,b.scale,b.y_scale,b.rotation,b.mirror));
    }
}

#[test]
fn nonplanar_width_or_bulge_export_requires_acceptance_even_without_kentos_metadata() {
    for (widths,bulges) in [(Some(vec![[0.2,0.4],[0.,0.]]),None),(None,Some(vec![0.5]))] {
        let mut input = dxf::input_from_json(&serde_json::json!({"entities":[{"kind":"polyline","id":1,"layerId":"0","attrs":{},"pts":[{"x":0.,"y":0.},{"x":3.,"y":4.}],"zs":[7.,8.],"widths":widths,"bulges":bulges}],"layers":[],"scale":1000.,"lengthDecimals":3,"grads":false,"annotations":{"metadata":false}}).to_string()).unwrap();
        assert_eq!(dxf::write_checked(&input).unwrap_err().code,"polyline_geometry_loss");
        input.allow_polyline_loss=true;
        let (bytes,report)=dxf::write_checked(&input).unwrap();
        assert!(!bytes.is_empty());
        assert!(dxf::polyline_policy::refusal(report.notes.iter()).is_some());
    }
}

#[test]
fn writer_keeps_host_loss_properties_so_checked_export_cannot_bypass_them() {
    let input = dxf::input_from_json(&serde_json::json!({"entities":[],"layers":[],"scale":1000.,"lengthDecimals":3,"grads":false,"notes":[{"what":"Pafta çoklu çizgisi","reason":"genişlik kaybı","count":2,"lines":[],"severity":"warning","details":[{"entityType":"LWPOLYLINE","property":"polyline.loss.paper_width"}]}]}).to_string()).unwrap();
    assert_eq!(dxf::write_checked(&input).unwrap_err().code,"polyline_geometry_loss");
}
