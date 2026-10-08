//! Sınır çıkar (docs/adr/0207 §7; PDAL's filters.hexbin boundary, ArcGIS
//! Point Cloud Boundary): where the cloud has points: the cells of the grid
//! (on the cell's multiples) holding at least the given number, taken
//! together as right-angled rings with their holes (`ops::boundary`),
//! written as one multi-part area on a new layer.

use kentos_geometry_core::entity::{Part, Shape};
use kentos_geometry_core::geom::arrangement::Ring;
use kentos_geometry_core::vec2::Vec2;
use kentos_pointcloud::ops::boundary::parts;
use kentos_pointcloud::ops::raster::{Frame, Occupancy};
use serde_json::json;

use super::ground::number;
use super::{
    CATEGORY, broke, clouds_param, count_words, files_of, inputs, one_cloud, pass, targets,
};
use crate::builtin::geometry::{layer_param, new_object};
use crate::files::Files;
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext,
    RunResult, Tool,
};

pub fn tool() -> Tool {
    Tool {
        id: "pointcloud.boundary".into(),
        label: "Sınır çıkar".into(),
        category: CATEGORY.into(),
        description: "Nokta bulutunun kapladığı yeri alan olarak çizer: verilen boydaki hücrelerden en az şu kadar nokta düşenler birleşir, delikleriyle tek çok parçalı alan olur.".into(),
        help: Some([
            "Izgara hücre boyunun katlarına oturur. Kenarlar dik açılıdır ve hücrelerin kenarlarından geçer; yalnız köşeden değen hücreler ayrı parçalardır.",
            "En az nokta, seyrek gürültünün sınırı büyütmesini önler.",
        ]
        .join("\n\n")),
        keywords: ["sınır", "boundary", "kapsam", "footprint", "nokta bulutu"]
            .map(String::from)
            .to_vec(),
        aliases: ["BULUTSINIR", "SINIRCIKAR"].map(String::from).to_vec(),
        icon: Some("pointCloudBoundary".into()),
        parameters: vec![
            clouds_param("Nokta bulutu", "Sınırı çıkarılacak bulut."),
            number("cell", "Hücre", 1.0, 0.01, 10_000.0, "m"),
            ParamDef::new(
                "least",
                "En az nokta",
                ParamKind::Number {
                    min: Some(1.0),
                    max: Some(1_000_000.0),
                    integer: true,
                    unit: "adet".into(),
                    placeholder: None,
                },
            )
            .default_value(json!(1)),
            layer_param("Bulut sınırı", "#5B8FCF"),
        ],
        outputs: vec![OutputDef::new("parts", "Parça", OutputKind::Number)],
        targets: targets(),
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(r: &Resolved<'_>, _cx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let (files, cloud) = match (files_of(feedback), one_cloud(r)) {
        (Ok(f), Ok(c)) => (f, c),
        (Err(e), _) | (_, Err(e)) => return *e,
    };
    match go(&*files, r, cloud, feedback) {
        Ok(result) => result,
        Err(why) => broke(why),
    }
}

fn ring(pts: &[[f64; 2]]) -> Vec<Vec2> {
    pts.iter().map(|p| Vec2::new(p[0], p[1])).collect()
}

fn go(
    files: &dyn Files,
    r: &Resolved<'_>,
    cloud: &kentos_contracts::PointCloudEntity,
    feedback: &mut dyn Feedback,
) -> Result<RunResult, String> {
    let c = &cloud.cloud;
    let cell = r.number("cell").unwrap_or(1.0);
    let least = r.number("least").unwrap_or(1.0).max(1.0) as u32;
    let frame = Frame::over(c.rect(), cell)
        .ok_or("Izgara çok büyük olur: hücreyi büyütün (en çok 400 milyon hücre).")?;
    let sources = &c.sources;
    let ins = inputs(files, sources)?;
    let plan = kentos_pointcloud::ops::convert::plan(&ins, false).map_err(|e| e.0)?;
    let layout = plan.spec.layout();
    let (scale, offset) = (plan.spec.scale, plan.spec.offset);
    let mut occ = Occupancy::new(frame);
    pass(
        files,
        sources,
        &ins,
        &plan.spec,
        feedback,
        "Hücreler sayılıyor",
        0.0,
        0.9,
        &mut |recs, _| {
            occ.feed(&layout, recs, scale, offset);
            Ok(())
        },
    )?;
    feedback.progress(0.92, "Sınır çiziliyor");
    let filled = occ.filled(least);
    let cells = filled.iter().filter(|f| **f).count();
    let found = parts(&frame, &filled);
    if found.is_empty() {
        return Ok(RunResult {
            summary: Some("Hiçbir hücrede istenen sayıda nokta yok; alan çizilmedi.".to_owned()),
            ..RunResult::default()
        });
    }
    let holes = |p: &kentos_pointcloud::ops::boundary::Part| -> Option<Vec<Ring>> {
        (!p.holes.is_empty()).then(|| {
            p.holes
                .iter()
                .map(|h| Ring {
                    pts: ring(h),
                    bulges: None,
                })
                .collect()
        })
    };
    let first = &found[0];
    let rest: Vec<Part> = found[1..]
        .iter()
        .map(|p| Part {
            pts: ring(&p.outer),
            bulges: None,
            holes: holes(p),
        })
        .collect();
    let shape = Shape::Polygon {
        pts: ring(&first.outer),
        bulges: None,
        holes: holes(first),
        parts: (!rest.is_empty()).then_some(rest),
    };
    let mut attrs = std::collections::BTreeMap::new();
    attrs.insert("Hücre".to_owned(), crate::text::js_number(cell));
    attrs.insert("En az nokta".to_owned(), least.to_string());
    let add: Vec<_> = new_object(shape, &r.layer("layer").id, attrs)
        .into_iter()
        .collect();
    let n = found.len();
    Ok(RunResult {
        changes: Some(ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs: [("parts".to_owned(), json!(n))].into_iter().collect(),
        summary: Some(format!(
            "{} hücreden {} parçalı sınır çizildi ({} m'lik hücreler).",
            count_words(cells as u64),
            n,
            crate::text::js_number(cell)
        )),
        ..RunResult::default()
    })
}
