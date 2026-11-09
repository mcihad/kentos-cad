//! Dönüşüm uygula (plan/map-cad.md MAP-13, docs/adr/3100; PDAL's filters.transformation, CloudCompare's Apply
//! Transformation): the cloud's points moved by an affine 4 × 4 matrix written row by row (Tarama kaydı's matrix takes
//! the moving scan onto the fixed one), the result written as LAS, LAZ or COPC on the source's grid; the source stays
//! as it was. Before anything is written the files' boxes are moved, so a matrix that would take points off the grid
//! is refused at once. The summary says how far the cloud's middle went and, for a rigid matrix, the rotation's angle;
//! a matrix that scales or shears is said to change lengths. The machine is the core's (`ops::transform`).
//!
//! docs/adr/3500: Tersini uygula applies the matrix's inverse (the core's fixed order). The object written keeps its
//! history as attributes: Kaynak, the first file of the chain (the source object's own Kaynak when it has one, else its
//! file), and Dönüşümler, the matrices applied from it, each as 16 numbers, the newest last after “ | ”.
//!
//! docs/adr/4300: Eksen ve açı gives the map instead of the matrix: a rotation of an angle (degrees) about an axis
//! (right-handed) about a centre (empty: the middle of the cloud's box), then a move; the core makes the matrix
//! (`Affine::axis_angle`), which the history keeps as any other.

use kentos_pointcloud::ops::transform::Affine;
use serde_json::json;

use super::{
    CATEGORY, Out, add_param, broke, cloud_object, clouds_param, count_words, files_of,
    format_param, grid_notes, inputs, one_cloud, output_param, pass, place, result_layer_param,
    settle, targets, written,
};
use crate::types::{
    EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext, RunResult, Tool,
};

/// The result's file name after the source's.
const SUFFIX: &str = "-donusmus";
/// The written object's attributes of its history (docs/adr/3500).
const ORIGIN: &str = "Kaynak";
const HISTORY: &str = "Dönüşümler";

pub fn tool() -> Tool {
    Tool {
        id: "pointcloud.transform".into(),
        label: "Dönüşüm uygula".into(),
        category: CATEGORY.into(),
        description: "Nokta bulutunun bütün noktalarını 4 × 4'lük afin matrisle taşır (Tarama kaydı'nın matrisi gibi); sonuç kaynağın ızgarasında LAS, LAZ ya da COPC olarak yazılır, kaynak dosya değişmez.".into(),
        help: Some([
            "Matris satır satır 16 sayıdır: ilk üç sütunun üst üç satırı dönüklük (ya da başka doğrusal kısım), dördüncü sütunun üst üç satırı öteleme, son satır 0 0 0 1. Sayılar boşluk, satır sonu, virgül ya da noktalı virgülle ayrılır; ondalık için nokta.",
            "Tarama kaydı'nın Buluta uygula düğmesi oturtulan taramanın matrisini buraya getirir: hareketli taramanın noktaları sabit taramanın üstüne taşınır.",
            "Eksen ve açı: bulut bir eksen (X Y Z, uzunluğu önemsiz) çevresinde açı kadar (derece; sağ el kuralı: eksenin ucundan bakınca saatin tersine) bir merkez (boş: bulutun kutusunun ortası) çevresinde döner, sonra ötelenir. KentOS bunu Tarama kaydı'nın tarifiyle tek matrise çevirir; geçmişe o matris yazılır. Euler açıları yok: sıraları kaynakta yazılı değil.",
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
            // docs/adr/4300: the map as a matrix, or as an axis and an angle.
            ParamDef::new(
                "form",
                "Dönüşüm",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("matrix", "Matris").hint("Satır satır 16 sayı"),
                        EnumOption::new("axisAngle", "Eksen ve açı").hint("Bir eksen çevresinde dönme ve öteleme"),
                    ],
                },
            )
            .default_value(json!("matrix"))
            .shown_when(not_undoing),
            ParamDef::new(
                "matrix",
                "Matris",
                ParamKind::Text {
                    placeholder: Some("1 0 0 0  0 1 0 0  0 0 1 0  0 0 0 1".into()),
                    max_length: Some(4000),
                    allow_empty: false,
                },
            )
            .describe("Satır satır 16 sayı: dönüklük ve öteleme, son satır 0 0 0 1 (Tarama kaydı'nın matrisi).")
            .shown_when(by_matrix),
            ParamDef::new(
                "axis",
                "Eksen",
                ParamKind::Text { placeholder: Some("0 0 1".into()), max_length: Some(200), allow_empty: false },
            )
            .default_value(json!("0 0 1"))
            .describe("Dönme ekseninin yönü, X Y Z (uzunluğu önemsiz); açı sağ el kuralıyla: eksenin ucundan bakınca saatin tersine.")
            .shown_when(by_axis),
            ParamDef::new(
                "angle",
                "Açı",
                ParamKind::Number { min: None, max: None, integer: false, unit: "°".into(), placeholder: None },
            )
            .default_value(json!(0.0))
            .describe("Dönme açısı, derece.")
            .shown_when(by_axis),
            ParamDef::new(
                "center",
                "Merkez",
                ParamKind::Text { placeholder: Some("boş: bulutun kutusunun ortası".into()), max_length: Some(200), allow_empty: true },
            )
            .optional()
            .describe("Dönmenin merkezi, X Y Z (projenin koordinatları); boş: bulutun kutusunun ortası.")
            .shown_when(by_axis),
            ParamDef::new(
                "translation",
                "Öteleme",
                ParamKind::Text { placeholder: Some("0 0 0".into()), max_length: Some(200), allow_empty: true },
            )
            .optional()
            .describe("Dönmeden sonra öteleme, X Y Z metre; boş: 0 0 0.")
            .shown_when(by_axis),
            ParamDef::new("invert", "Tersini uygula", ParamKind::Boolean)
                .default_value(json!(false))
                .describe("Matrisin tersi uygulanır: kaydın taşıdığı bulut eski yerine döner.")
                .shown_when(not_undoing),
            // docs/adr/3900: the last step of the cloud's history taken back.
            ParamDef::new("undoLast", "Son dönüşümü geri al", ParamKind::Boolean)
                .default_value(json!(false))
                .describe("Bulutun geçmişindeki son matrisin tersi uygulanır; o adım geçmişten düşer."),
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

/// Whether Son dönüşümü geri al is off: the matrix and Tersini uygula are asked only then.
fn not_undoing(v: &crate::types::Values) -> bool {
    v.get("undoLast").and_then(serde_json::Value::as_bool) != Some(true)
}

/// Whether the map is given as an axis and an angle (docs/adr/4300).
fn axis_form(v: &crate::types::Values) -> bool {
    v.get("form").and_then(serde_json::Value::as_str) == Some("axisAngle")
}

fn by_matrix(v: &crate::types::Values) -> bool {
    not_undoing(v) && !axis_form(v)
}

fn by_axis(v: &crate::types::Values) -> bool {
    not_undoing(v) && axis_form(v)
}

/// Eksen ve açı's map (docs/adr/4300): the axis, the angle, the centre (empty: the middle of the cloud's box) and the
/// move (empty: none).
fn axis_map(r: &Resolved<'_>, cloud: &kentos_commands::PointCloudEntity) -> Result<Affine, String> {
    let axis = Affine::triple(r.text("axis"), "Eksen").map_err(|e| e.0)?;
    let center = match r.text("center").trim() {
        "" => {
            let b = cloud.cloud.bounds;
            [(b[0] + b[3]) / 2.0, (b[1] + b[4]) / 2.0, (b[2] + b[5]) / 2.0]
        }
        text => Affine::triple(text, "Merkez").map_err(|e| e.0)?,
    };
    let translation = match r.text("translation").trim() {
        "" => [0.0; 3],
        text => Affine::triple(text, "Öteleme").map_err(|e| e.0)?,
    };
    Affine::axis_angle(axis, r.number("angle").unwrap_or(0.0), center, translation).map_err(|e| e.0)
}

/// Son dönüşümü geri al (docs/adr/3900): the inverse of the last matrix of the cloud's history, and the history without
/// it (empty when it was the only one); refused without a history.
fn undone(cloud: &kentos_commands::PointCloudEntity) -> Result<(Affine, String), String> {
    let had = cloud.base.attrs.get(HISTORY).map(|h| h.trim()).unwrap_or_default();
    let steps: Vec<&str> = had.split(" | ").map(str::trim).filter(|t| !t.is_empty()).collect();
    let Some((last, before)) = steps.split_last() else {
        return Err("Bulutun dönüşüm geçmişi (Dönüşümler) yok; geri alınacak adım yok.".to_owned());
    };
    let affine = Affine::parse(last).map_err(|e| format!("Geçmişin son matrisi okunmadı: {}", e.0))?;
    Ok((affine.inverse(), before.join(" | ")))
}

fn run(r: &Resolved<'_>, _cx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let (files, cloud) = match (files_of(feedback), one_cloud(r)) {
        (Ok(f), Ok(c)) => (f, c),
        (Err(e), _) | (_, Err(e)) => return *e,
    };
    let (affine, history) = if r.flag("undoLast") {
        match undone(cloud) {
            Ok((a, h)) => (a, Some(h)),
            Err(why) => return RunResult::refused(why),
        }
    } else {
        let given = if r.text("form") == "axisAngle" { axis_map(r, cloud) } else { Affine::parse(r.text("matrix")).map_err(|e| e.0) };
        match given {
            Ok(a) if r.flag("invert") => (a.inverse(), None),
            Ok(a) => (a, None),
            Err(why) => return RunResult::refused(why),
        }
    };
    match go(&*files, r, cloud, &affine, history, feedback) {
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
    undone: Option<String>,
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
    let mut object = cloud_object(
        &at,
        count,
        bounds,
        cloud.cloud.srid,
        cloud.cloud.style.clone(),
        &r.layer("layer").id,
    );
    // The history (docs/adr/3500): the chain's first file and the matrices applied from it, this one last.
    let had = |k: &str| cloud.base.attrs.get(k).cloned();
    let origin = had(ORIGIN).or_else(|| sources.first().and_then(|s| s.file.clone())).unwrap_or_default();
    // Son dönüşümü geri al (docs/adr/3900): the history without its last step; none left, no Dönüşümler.
    let history = match undone {
        Some(before) => before,
        None => match had(HISTORY) {
            Some(h) if !h.trim().is_empty() => format!("{h} | {}", affine.text()),
            _ => affine.text(),
        },
    };
    let attrs = &mut object.base_mut().attrs;
    attrs.insert(ORIGIN.to_owned(), origin);
    if !history.is_empty() {
        attrs.insert(HISTORY.to_owned(), history);
    }
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
