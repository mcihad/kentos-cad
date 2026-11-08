//! Dönüşüm uygula (plan/map-cad.md MAP-13, docs/adr/3100; PDAL's filters.transformation, CloudCompare's Apply
//! Transformation): the cloud's points moved by an affine 4 × 4 matrix written row by row (Tarama kaydı's matrix takes
//! the moving scan onto the fixed one), the result written as LAS, LAZ or COPC on the source's grid; the source stays
//! as it was. Before anything is written the files' boxes are moved, so a matrix that would take points off the grid
//! is refused at once. The summary says how far the cloud's middle went and, for a rigid matrix, the rotation's angle;
//! a matrix that scales or shears is said to change lengths. The machine is the core's (`ops::transform`).

use kentos_pointcloud::ops::transform::Affine;
use serde_json::json;

use super::{
    CATEGORY, Out, add_param, broke, cloud_object, clouds_param, count_words, files_of,
    format_param, grid_notes, inputs, one_cloud, output_param, pass, place, result_layer_param,
    settle, targets, written,
};
use crate::types::{
    Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext, RunResult, Tool,
};

/// The result's file name after the source's.
const SUFFIX: &str = "-donusmus";

pub fn tool() -> Tool {
    Tool {
        id: "pointcloud.transform".into(),
        label: "Dönüşüm uygula".into(),
        category: CATEGORY.into(),
        description: "Nokta bulutunun bütün noktalarını 4 × 4'lük afin matrisle taşır (Tarama kaydı'nın matrisi gibi); sonuç kaynağın ızgarasında LAS, LAZ ya da COPC olarak yazılır, kaynak dosya değişmez.".into(),
        help: Some([
            "Matris satır satır 16 sayıdır: ilk üç sütunun üst üç satırı dönüklük (ya da başka doğrusal kısım), dördüncü sütunun üst üç satırı öteleme, son satır 0 0 0 1. Sayılar boşluk, satır sonu, virgül ya da noktalı virgülle ayrılır; ondalık için nokta.",
            "Tarama kaydı'nın Buluta uygula düğmesi oturtulan taramanın matrisini buraya getirir: hareketli taramanın noktaları sabit taramanın üstüne taşınır.",
            "Her nokta taşındıktan sonra kaynağın ızgarasına (ölçeği ve ötelemesi) yarım sıfırdan uzağa yuvarlanır: bir koordinat en çok ölçeğin yarısı kadar değişir. Noktanın öbür alanları (sınıf, renk, dönüş, GPS zamanı, tarama açısı, ek baytlar) olduğu gibi kalır; ek baytlardaki normaller döndürülmez.",
            "Koordinat sistemi değişmez: matris noktaları aynı sistemde taşır, sistemler arası dönüşüm değildir.",
        ]
        .join("\n\n")),
        keywords: ["dönüşüm", "matris", "kayıt", "transform", "transformation", "matrix", "register", "nokta bulutu", "lidar"]
            .map(String::from)
            .to_vec(),
        aliases: ["DONUSUMUYGULA", "BULUTDONUSTUR"].map(String::from).to_vec(),
        icon: Some("cloudTransform".into()),
        parameters: vec![
            clouds_param("Nokta bulutu", "Taşınacak bulut (sanal bulutun bütün dosyaları birlikte)."),
            ParamDef::new(
                "matrix",
                "Matris",
                ParamKind::Text {
                    placeholder: Some("1 0 0 0  0 1 0 0  0 0 1 0  0 0 0 1".into()),
                    max_length: Some(4000),
                    allow_empty: false,
                },
            )
            .describe("Satır satır 16 sayı: dönüklük ve öteleme, son satır 0 0 0 1 (Tarama kaydı'nın matrisi)."),
            format_param(),
            output_param(SUFFIX, &[".laz", ".las"]),
            add_param(),
            result_layer_param("Dönüştürülmüş bulut", "#7A5BA6"),
        ],
        outputs: vec![
            OutputDef::new("count", "Taşınan nokta", OutputKind::Number),
            OutputDef::new("shift", "Ortanın kayması (m)", OutputKind::Number),
            OutputDef::new("angle", "Dönüklük (°)", OutputKind::Number),
        ],
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
    let affine = match Affine::parse(r.text("matrix")) {
        Ok(a) => a,
        Err(e) => return RunResult::refused(e.0),
    };
    match go(&*files, r, cloud, &affine, feedback) {
        Ok(result) => result,
        Err(why) => broke(why),
    }
}

/// A number with `d` decimals, a comma for the point.
fn comma(v: f64, d: usize) -> String {
    format!("{v:.d$}").replace('.', ",")
}

/// Half of each scale, the most a coordinate is rounded: one number when the three are the same.
fn half_scales(scale: [f64; 3]) -> String {
    let h = scale.map(|s| s / 2.0);
    let t = |v: f64| format!("{v}").replace('.', ",");
    if h[0] == h[1] && h[1] == h[2] {
        format!("{} m", t(h[0]))
    } else {
        format!("X {}, Y {}, Z {} m", t(h[0]), t(h[1]), t(h[2]))
    }
}

fn go(
    files: &dyn crate::files::Files,
    r: &Resolved<'_>,
    cloud: &kentos_commands::PointCloudEntity,
    affine: &Affine,
    feedback: &mut dyn Feedback,
) -> Result<RunResult, String> {
    let sources = &cloud.cloud.sources;
    let ins = inputs(files, sources)?;
    let plan =
        kentos_pointcloud::ops::convert::plan(&ins, r.text("format") != "las").map_err(|e| e.0)?;
    let layout = plan.spec.layout();
    let (scale, offset) = (plan.spec.scale, plan.spec.offset);
    // Every file's box moved onto the result's grid: refused before anything is written.
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for i in &ins {
        affine
            .fits(i.head.min, i.head.max, scale, offset)
            .map_err(|e| e.0)?;
        for k in 0..3 {
            lo[k] = lo[k].min(i.head.min[k]);
            hi[k] = hi[k].max(i.head.max[k]);
        }
    }
    let at = place(files, r, &sources[0], SUFFIX)?;
    let copc = at.format == kentos_commands::CloudFormat::Copc;
    let write_to = if copc { 0.6 } else { 1.0 };
    let mut out = Out::new(files, &at.written, plan.spec.clone())?;
    let rounded = pass(
        files,
        sources,
        &ins,
        &plan.spec,
        feedback,
        "Noktalar taşınıyor",
        0.0,
        write_to,
        &mut |recs, _| {
            affine
                .move_records(&layout, recs, scale, offset)
                .map_err(|e| e.0)?;
            out.put(recs)
        },
    )?;
    let (count, bounds) = out.finish()?;
    settle(files, &at, feedback, write_to, 1.0)?;
    grid_notes(rounded, plan.dropped_extra, feedback);
    let rigid = affine.rigid();
    if !rigid {
        feedback.warn(
            "Matris katı bir dönüşüm değil (ölçek ya da kayma içeriyor): uzunluklar ve açılar değişir.".to_owned(),
        );
    }
    feedback.info(format!(
        "Taşınan noktalar kaynağın ızgarasına yuvarlandı: bir koordinat en çok {} değişir.",
        half_scales(scale)
    ));
    let middle = [0, 1, 2].map(|k| (lo[k] + hi[k]) / 2.0);
    let shift = affine.shift(middle);
    let angle = rigid.then(|| affine.angle());
    let object = cloud_object(
        &at,
        count,
        bounds,
        cloud.cloud.srid,
        cloud.cloud.style.clone(),
        &r.layer("layer").id,
    );
    let turned = angle.map_or_else(String::new, |a| format!(", dönüklük {}°", comma(a, 4)));
    let mut result = written(
        r,
        Some(object),
        format!(
            "{} nokta taşındı; bulutun ortası {} m kaydı{turned}; “{}” yazıldı.",
            count_words(count),
            comma(shift, 3),
            at.path
        ),
    );
    result.outputs.insert("count".to_owned(), json!(count));
    result.outputs.insert("shift".to_owned(), json!(shift));
    if let Some(a) = angle {
        result.outputs.insert("angle".to_owned(), json!(a));
    }
    Ok(result)
}
