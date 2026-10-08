//! Yüksekliğe göre sınıfla (docs/adr/0207 §7; ArcGIS Classify LAS By Height,
//! PDAL's filters.hag_nn with filters.range): the ground's surface from the
//! ground points (2), each point's height above it; a point of class 0 or 1
//! (or, asked, any but ground) from the base to the low limit becomes low
//! vegetation (3), to the middle limit medium (4), to the high limit high
//! (5). Two passes (`ops::height`).

use kentos_contracts::CloudRender;
use kentos_pointcloud::ops::height::{Height, Params};
use serde_json::json;

use super::ground::number;
use super::{
    CATEGORY, Out, add_param, broke, cloud_object, clouds_param, count_words, files_of,
    format_param, grid_notes, inputs, one_cloud, output_param, pass, place, result_layer_param,
    settle, targets, written,
};
use crate::files::Files;
use crate::types::{
    Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext, RunResult, Tool,
    Values,
};

fn check(v: &Values) -> Option<String> {
    let n = |k: &str| v.get(k).and_then(serde_json::Value::as_f64);
    let (b, l, m, h) = (n("base")?, n("low")?, n("middle")?, n("high")?);
    (!(b <= l && l <= m && m <= h))
        .then(|| "Sınırlar sırayla büyümeli: taban ≤ düşük ≤ orta ≤ yüksek bitki.".to_owned())
}

pub fn tool() -> Tool {
    let d = Params::default();
    Tool {
        id: "pointcloud.classify".into(),
        label: "Yüksekliğe göre sınıfla".into(),
        category: CATEGORY.into(),
        description: "Zemin noktalarından (2) yüzeyi kurar, noktaları zeminden yüksekliklerine göre düşük (3), orta (4) ve yüksek bitki (5) sınıflarına alır; sonuç LAS, LAZ ya da COPC olarak yazılır.".into(),
        help: Some([
            "Önce Zemin süzgeci: bu araç zemin noktalarını (2) kullanır. Zemin yüzeyi her hücrenin en düşük zemin noktasından, boş hücreler komşularından doldurularak kurulur; noktanın yüksekliği yüzeyden çift doğrusal değerle ölçülür.",
            "Tabanla düşük sınır arası 3, düşükle orta arası 4, ortayla yüksek arası 5 olur; tabanın altı ve yüksek sınırın üstü değişmez.",
            "Yalnız sınıflanmamış (0, 1) noktalar sınıflanır; “Zemin olmayan bütün noktalar” açıksa zemin dışındaki her nokta.",
        ]
        .join("\n\n")),
        keywords: ["sınıfla", "yükseklik", "bitki", "vegetation", "height above ground", "hag", "lidar"]
            .map(String::from)
            .to_vec(),
        aliases: ["YUKSEKLIKSINIF", "BITKISINIF"].map(String::from).to_vec(),
        icon: Some("pointCloudClassify".into()),
        parameters: vec![
            clouds_param("Nokta bulutu", "Sınıflanacak bulut; zemin noktaları (2) olmalı."),
            number("cell", "Hücre", d.cell, 0.1, 100.0, "m"),
            number("base", "Taban", d.base, -100.0, 1000.0, "m"),
            number("low", "Düşük bitki üstü", d.low, -100.0, 1000.0, "m"),
            number("middle", "Orta bitki üstü", d.middle, -100.0, 1000.0, "m"),
            number("high", "Yüksek bitki üstü", d.high, -100.0, 1000.0, "m"),
            ParamDef::new("all", "Zemin olmayan bütün noktalar", ParamKind::Boolean)
                .default_value(json!(false))
                .describe("Kapalıyken yalnız 0 ve 1 sınıflı noktalar sınıflanır.")
                .advanced(),
            format_param(),
            output_param("-sinifli", &[".laz", ".las"]),
            add_param(),
            result_layer_param("Sınıflanmış bulut", "#3E8A4C"),
        ],
        outputs: vec![OutputDef::new("classed", "Sınıflanan nokta", OutputKind::Number)],
        targets: targets(),
        validate: Some(check),
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

fn go(
    files: &dyn Files,
    r: &Resolved<'_>,
    cloud: &kentos_contracts::PointCloudEntity,
    feedback: &mut dyn Feedback,
) -> Result<RunResult, String> {
    let d = Params::default();
    let p = Params {
        cell: r.number("cell").unwrap_or(d.cell),
        base: r.number("base").unwrap_or(d.base),
        low: r.number("low").unwrap_or(d.low),
        middle: r.number("middle").unwrap_or(d.middle),
        high: r.number("high").unwrap_or(d.high),
        all: r.flag("all"),
    };
    let sources = &cloud.cloud.sources;
    let ins = inputs(files, sources)?;
    let plan =
        kentos_pointcloud::ops::convert::plan(&ins, r.text("format") != "las").map_err(|e| e.0)?;
    let layout = plan.spec.layout();
    let (scale, offset) = (plan.spec.scale, plan.spec.offset);
    let mut h = Height::new(cloud.cloud.rect(), p)
        .ok_or("Sınıflamanın değerleri geçersiz ya da ızgara çok büyük.")?;
    let at = place(files, r, &sources[0], "-sinifli")?;
    let copc = at.format == kentos_contracts::CloudFormat::Copc;
    let write_to = if copc { 0.6 } else { 1.0 };
    pass(
        files,
        sources,
        &ins,
        &plan.spec,
        feedback,
        "Zemin noktaları okunuyor",
        0.0,
        write_to * 0.45,
        &mut |recs, _| {
            h.feed(&layout, recs, scale, offset);
            Ok(())
        },
    )?;
    if !h.surface() {
        return Err(
            "Bulutta zemin noktası (2) yok; önce Zemin süzgeci ile zemini ayırın.".to_owned(),
        );
    }
    let mut out = Out::new(files, &at.written, plan.spec.clone())?;
    let mut n = [0u64; 3];
    let rounded = pass(
        files,
        sources,
        &ins,
        &plan.spec,
        feedback,
        "Sınıflar yazılıyor",
        write_to * 0.5,
        write_to,
        &mut |recs, _| {
            let k = h.classify(&layout, recs, scale, offset);
            for i in 0..3 {
                n[i] += k[i];
            }
            out.put(recs)
        },
    )?;
    let (count, bounds) = out.finish()?;
    settle(files, &at, feedback, write_to, 1.0)?;
    grid_notes(rounded, plan.dropped_extra, feedback);
    let mut style = cloud.cloud.style.clone();
    style.render = CloudRender::Classification;
    let object = cloud_object(
        &at,
        count,
        bounds,
        cloud.cloud.srid,
        style,
        &r.layer("layer").id,
    );
    let classed = n.iter().sum::<u64>();
    let mut result = written(
        r,
        Some(object),
        format!(
            "{} nokta sınıflandı: düşük bitki {}, orta bitki {}, yüksek bitki {}; “{}” yazıldı.",
            count_words(classed),
            count_words(n[0]),
            count_words(n[1]),
            count_words(n[2]),
            at.path
        ),
    );
    result.outputs.insert("classed".to_owned(), json!(classed));
    Ok(result)
}
