//! Kırp (the web's `builtin/geometry/clip.ts`; docs/adr/0201 §3; ArcGIS Clip,
//! QGIS Kırp): each object's parts inside the cutting areas, which are joined
//! first; an area meets them in the core's overlay, a path is cut where it
//! crosses their boundary and keeps the pieces inside or on it, a point stays
//! when it is inside or on it. The objects keep their attributes.

use kentos_geometry_core::ops::geoprocess::calls::clip_run;
use serde_json::json;

use super::{
    AREA_KINDS, GEO_KINDS, attrs_of, empty_note, features_param, geo_scopes, input_notes,
    layer_param, new_object, shapes,
};
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, Resolved, RunContext, RunResult, ScopeKind, Target,
    Tool,
};

pub fn tool() -> Tool {
    Tool {
        id: "geometry.clip".into(),
        label: "Kırp".into(),
        category: "geometry".into(),
        description: "Nesnelerin kesen alanların içinde kalan parçalarını yeni katmana yazar; öznitelikler olduğu gibi kalır.".into(),
        help: Some([
            "Kesen alanlar önce birleşir; her nesnenin onların içinde ya da sınırında kalan kısmı yazılır: alanın ortak kısmı, çizginin içerideki parçaları, içerideki noktalar. Sınır üstündeki çizgi parçası içeride sayılır.",
            "Bir nesnenin birden çok parçası kalırsa sonuç tek, çok parçalı nesnedir. Kesen alanların dışında kalan nesne yazılmaz ve söylenir. Kesen alanların öznitelikleri alınmaz; onları da isteyen Kesişim'i kullanır.",
        ]
        .join("\n\n")),
        keywords: ["kırp", "kes", "clip", "sınırla", "pafta", "çalışma alanı", "kesen alan"]
            .map(String::from)
            .to_vec(),
        aliases: ["KIRP", "CLIP"].map(String::from).to_vec(),
        icon: Some("geoClip".into()),
        parameters: vec![
            features_param(
                "input",
                "Nesneler",
                &GEO_KINDS,
                geo_scopes(),
                "Kesilecek alanlar, çizgiler ve noktalar.",
            ),
            features_param(
                "cut",
                "Kesen alanlar",
                &AREA_KINDS,
                Some(vec![
                    ScopeKind::Selection,
                    ScopeKind::Layer,
                    ScopeKind::Visible,
                    ScopeKind::All,
                ]),
                "Kapalı alanlar, daireler, tam elipsler ve kapalı eğriler; birleşerek keser.",
            )
            .default_value(json!({ "scope": "selection" })),
            layer_param("Kırpılan", "#30A46C"),
        ],
        outputs: vec![
            OutputDef::new("clipped", "Kırpılan nesneler", OutputKind::Features),
            OutputDef::new("count", "Yazılan nesne sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let input = &v.features("input").entities;
    let cut = &v.features("cut").entities;
    input_notes(feedback, input, cut, false);
    feedback.progress(0.0, "Nesneler kırpılıyor");
    let layer = &v.layer("layer").id;
    let add: Vec<_> = clip_run(&shapes(input), &shapes(cut))
        .into_iter()
        .zip(input)
        .filter_map(|(s, e)| new_object(s?, layer, attrs_of(e)))
        .collect();
    empty_note(input.len() - add.len(), feedback);
    let count = add.len();
    RunResult {
        changes: Some(ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs: [("count".to_owned(), json!(count))].into_iter().collect(),
        summary: Some(format!("{count} nesne kırpılarak yazıldı.")),
        ..RunResult::default()
    }
}
