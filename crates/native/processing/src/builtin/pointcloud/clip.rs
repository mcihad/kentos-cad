//! Kırp (docs/adr/0207 §7; PDAL's filters.crop, ArcGIS Extract LAS): the
//! points inside (or outside) the selected areas, their holes and parts and
//! arcs exact; a point on a straight edge is inside (`ops::region`). One
//! pass; the result keeps the source's records whole.

use kentos_contracts::Entity;
use kentos_geometry_core::ops::areas::areas_of_entity;
use kentos_geometry_core::vec2::Vec2;
use kentos_native_application::geometry::shape;
use kentos_pointcloud::ops::region::Region;
use serde_json::json;

use super::{
    CATEGORY, Out, add_param, broke, cloud_object, clouds_param, count_words, files_of,
    format_param, grid_notes, inputs, one_cloud, output_param, pass, place, result_layer_param,
    settle, targets, written,
};
use crate::builtin::geometry::{AREA_KINDS, features_param, geo_scopes};
use crate::files::Files;
use crate::types::{
    EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext,
    RunResult, Tool,
};

/// The selected areas together; none when they enclose nothing.
pub(super) fn region_of(entities: &[&Entity]) -> Option<Region> {
    let areas: Vec<_> = entities
        .iter()
        .flat_map(|e| areas_of_entity(&shape(e)))
        .collect();
    Region::new(&areas)
}

pub fn tool() -> Tool {
    Tool {
        id: "pointcloud.clip".into(),
        label: "Bulutu kırp".into(),
        category: CATEGORY.into(),
        description: "Nokta bulutunun seçili alanların içinde (ya da dışında) kalan noktalarını LAS, LAZ ya da COPC olarak yazar; alanların delikleri, parçaları ve yayları kesindir.".into(),
        help: Some([
            "Alanlar birlikte bir bölgedir: nokta alanlardan birinin içindeyse (deliğinde değilse) içeridedir. Düz kenarın üstündeki nokta içeride sayılır.",
            "Noktaların bütün alanları olduğu gibi yazılır.",
        ]
        .join("\n\n")),
        keywords: ["kırp", "clip", "crop", "kes", "alan", "nokta bulutu"]
            .map(String::from)
            .to_vec(),
        aliases: ["BULUTKIRP"].map(String::from).to_vec(),
        icon: Some("pointCloudClip".into()),
        parameters: vec![
            clouds_param("Nokta bulutu", "Kırpılacak bulut."),
            features_param(
                "areas",
                "Alanlar",
                &AREA_KINDS,
                geo_scopes(),
                "Bulutun kırpılacağı kapalı alanlar, daireler, kapalı elips ve eğriler.",
            ),
            ParamDef::new(
                "side",
                "Kalanlar",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("inside", "İçindekiler"),
                        EnumOption::new("outside", "Dışındakiler"),
                    ],
                },
            )
            .default_value(json!("inside")),
            format_param(),
            output_param("-kirpik", &[".laz", ".las"]),
            add_param(),
            result_layer_param("Kırpılmış bulut", "#4C7A9A"),
        ],
        outputs: vec![OutputDef::new("count", "Kalan nokta", OutputKind::Number)],
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
    let Some(region) = region_of(&r.features("areas").entities) else {
        return RunResult::refused(
            "Kırpılacak alan yok: kapalı alan, daire ya da kapalı eğri seçin.".to_owned(),
        );
    };
    match go(&*files, r, cloud, &region, feedback) {
        Ok(result) => result,
        Err(why) => broke(why),
    }
}

fn go(
    files: &dyn Files,
    r: &Resolved<'_>,
    cloud: &kentos_contracts::PointCloudEntity,
    region: &Region,
    feedback: &mut dyn Feedback,
) -> Result<RunResult, String> {
    let inside = r.text("side") != "outside";
    let sources = &cloud.cloud.sources;
    let ins = inputs(files, sources)?;
    let plan =
        kentos_pointcloud::ops::convert::plan(&ins, r.text("format") != "las").map_err(|e| e.0)?;
    let layout = plan.spec.layout();
    let (scale, offset) = (plan.spec.scale, plan.spec.offset);
    let at = place(files, r, &sources[0], "-kirpik")?;
    let copc = at.format == kentos_contracts::CloudFormat::Copc;
    let write_to = if copc { 0.6 } else { 1.0 };
    let mut out = Out::new(files, &at.written, plan.spec.clone())?;
    let index = region.index();
    let mut keep = Vec::new();
    let rounded = pass(
        files,
        sources,
        &ins,
        &plan.spec,
        feedback,
        "Kırpılıyor",
        0.0,
        write_to,
        &mut |recs, _| {
            keep.clear();
            for rec in recs.chunks_exact(layout.len) {
                let p = Vec2::new(
                    f64::from(layout.x(rec)) * scale[0] + offset[0],
                    f64::from(layout.y(rec)) * scale[1] + offset[1],
                );
                if region.contains(&index, p) == inside {
                    keep.extend_from_slice(rec);
                }
            }
            out.put(&keep)
        },
    )?;
    let (count, bounds) = out.finish()?;
    settle(files, &at, feedback, write_to, 1.0)?;
    grid_notes(rounded, plan.dropped_extra, feedback);
    if count == 0 {
        feedback.warn("Alanların içinde (ya da dışında) nokta yok; dosya boş yazıldı.".to_owned());
    }
    let object = (count > 0).then(|| {
        cloud_object(
            &at,
            count,
            bounds,
            cloud.cloud.srid,
            cloud.cloud.style.clone(),
            &r.layer("layer").id,
        )
    });
    let total: u64 = ins.iter().map(|i| i.head.count).sum();
    let mut result = written(
        r,
        object,
        format!(
            "{} noktadan {} nokta {}; “{}” yazıldı.",
            count_words(total),
            count_words(count),
            if inside {
                "alanların içinde"
            } else {
                "alanların dışında"
            },
            at.path
        ),
    );
    result.outputs.insert("count".to_owned(), json!(count));
    Ok(result)
}
