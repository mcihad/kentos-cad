//! Tadilat durumu in a DXF (docs/adr/3039 §4): the sample's existing wall, its new door and its window closed up, each
//! drawn on its layer's state layer (Duvar-MEVCUT, Kapı-ILAVE, Kapı-IPTAL), the iptal one dashed; the passage takes
//! its wall's state and has no lines of its own. `KENTOS_RENOVATION_DXF=<file>` keeps the file, for another reader (GDAL's ogrinfo).

use kentos_commands::{DocumentSnapshotV2, Entity, LineType};
use kentos_domain::Document;
use kentos_headless::export;

#[test]
fn each_state_is_drawn_on_its_layer() {
    let snapshot: DocumentSnapshotV2 = serde_json::from_str(include_str!(
        "../../../../../fixtures/kcad/v2/opening-phases.json"
    ))
    .unwrap();
    let mut doc = Document::from_snapshot_v2(snapshot).unwrap();
    // Tümü: every state written (the sample is in Mevcut durum).
    let mut s = doc.settings().clone();
    if let Some(a) = s.arch.as_mut() {
        a.phase_view = None;
    }
    doc.set_settings(s);
    let mut input = export::dxf_input(&doc, &export::by_layer(&doc, doc.entities()));
    input.annotations.metadata = false;
    let (bytes, report) = kentos_formats::dxf::write(&input);
    if let Some(path) = std::env::var_os("KENTOS_RENOVATION_DXF") {
        std::fs::write(path, &bytes).unwrap();
    }
    let read = kentos_formats::dxf::read(&bytes, &Default::default()).unwrap();
    let mut layers: Vec<&str> = read
        .entities
        .iter()
        .map(|e| e.base().layer_id.as_str())
        .collect();
    layers.sort_unstable();
    layers.dedup();
    // The passage has no lines of its own (a gap in its wall), so no Kapı-MEVCUT.
    assert_eq!(
        layers,
        ["Duvar-MEVCUT", "Kapı-ILAVE", "Kapı-IPTAL"],
        "{layers:?}"
    );
    let line_type = |name: &str| {
        read.layers
            .iter()
            .find(|l| l.name == name)
            .map(|l| l.line_type)
    };
    assert_eq!(line_type("Kapı-IPTAL"), Some(LineType::Dashed));
    assert_eq!(line_type("Duvar-MEVCUT"), Some(LineType::Continuous));
    assert!(
        report
            .notes
            .iter()
            .any(|n| n.what == "Tadilat durumu" && n.count == 4),
        "{:?}",
        report.notes
    );
    assert!(!doc.entities().any(|e| matches!(e, Entity::Text(_))));
}

/// The view the drawing is in is what the DXF gets (docs/adr/3039 §4, as design options): in Tümü everything is
/// written; in Mevcut durum the new door is not written and said, and its wall's lines have no gap where it would be; in
/// Yeni durum the closed window is not written (no Kapı-IPTAL).
#[test]
fn the_view_is_what_the_dxf_gets() {
    use kentos_commands::PhaseView;
    let snapshot: DocumentSnapshotV2 = serde_json::from_str(include_str!(
        "../../../../../fixtures/kcad/v2/opening-phases.json"
    ))
    .unwrap();
    let mut doc = Document::from_snapshot_v2(snapshot).unwrap();
    let written = |doc: &Document| {
        let mut input = export::dxf_input(doc, &export::by_layer(doc, doc.entities()));
        input.annotations.metadata = false;
        let notes = input.notes.clone();
        let (bytes, _) = kentos_formats::dxf::write(&input);
        let read = kentos_formats::dxf::read(&bytes, &Default::default()).unwrap();
        // The wall's lines along its face (y = +0.15): their spans along x, sorted.
        let mut face: Vec<(f64, f64)> = read
            .entities
            .iter()
            .filter_map(|e| match e {
                Entity::Line(l) if (l.a.y - 0.15).abs() < 1e-6 && (l.b.y - 0.15).abs() < 1e-6 => {
                    Some((l.a.x.min(l.b.x), l.a.x.max(l.b.x)))
                }
                Entity::Polyline(p)
                    if p.pts.len() == 2 && p.pts.iter().all(|q| (q.y - 0.15).abs() < 1e-6) =>
                {
                    Some((p.pts[0].x.min(p.pts[1].x), p.pts[0].x.max(p.pts[1].x)))
                }
                _ => None,
            })
            .collect();
        face.sort_by(|a, b| a.0.total_cmp(&b.0));
        let cm = |v: f64| (v * 100.0).round() / 100.0;
        let gaps: Vec<(f64, f64)> = face
            .windows(2)
            .filter(|w| w[1].0 - w[0].1 > 1e-6)
            .map(|w| (cm(w[0].1), cm(w[1].0)))
            .collect();
        let mut layers: Vec<String> = read
            .entities
            .iter()
            .map(|e| e.base().layer_id.clone())
            .collect();
        layers.sort();
        layers.dedup();
        (gaps, layers, notes)
    };
    let set = |doc: &mut Document, view: PhaseView| {
        let mut s = doc.settings().clone();
        let mut arch = s.arch.clone().unwrap_or_default();
        arch.phase_view = (view != PhaseView::All).then_some(view);
        s.arch = Some(arch);
        doc.set_settings(s);
    };
    set(&mut doc, PhaseView::All);
    let (gaps, layers, notes) = written(&doc);
    // A window keeps its wall's face lines across it (its frame and glass), so the gaps are the door's and the passage's.
    assert_eq!(gaps, [(1.05, 1.95), (5.1, 5.7)], "Tümü: every opening");
    assert_eq!(layers, ["Duvar-MEVCUT", "Kapı-ILAVE", "Kapı-IPTAL"]);
    assert!(
        !notes
            .iter()
            .any(|n| n.what == "Tadilat görünümünün gizlediği nesne")
    );
    set(&mut doc, PhaseView::Existing);
    let (gaps, layers, notes) = written(&doc);
    assert_eq!(
        gaps,
        [(5.1, 5.7)],
        "Mevcut durum: no new door, no gap for it"
    );
    assert_eq!(layers, ["Duvar-MEVCUT", "Kapı-IPTAL"]);
    let note = notes
        .iter()
        .find(|n| n.what == "Tadilat görünümünün gizlediği nesne")
        .expect("said");
    assert_eq!(note.count, 1);
    set(&mut doc, PhaseView::Proposed);
    let (gaps, layers, _) = written(&doc);
    assert_eq!(
        gaps,
        [(1.05, 1.95), (5.1, 5.7)],
        "Yeni durum: the window closed"
    );
    assert_eq!(layers, ["Duvar-MEVCUT", "Kapı-ILAVE"]);
}
