//! Zemin süzgeci (docs/adr/0207 §7; Netcad's Yüzey Filtresi, ArcGIS Classify
//! LAS Ground, PDAL's filters.smrf): the simple morphological filter of
//! Pingel, Clarke and McBride as the ADR defines it (`ops::ground`): the
//! ground's surface from the last returns' least heights, the low outliers
//! and the objects opened away; a point within the threshold and the scaled
//! slope of the surface is ground (2), a 2 that is not becomes 1, other
//! classes stay. Two passes: the cells, then the classes written.

use kentos_contracts::CloudRender;
use kentos_pointcloud::ops::ground::{Ground, Params};
use serde_json::json;

use super::{
    CATEGORY, Out, add_param, broke, cloud_object, clouds_param, count_words, files_of,
    format_param, grid_notes, inputs, one_cloud, output_param, pass, place, result_layer_param,
    settle, targets, written,
};
use crate::files::Files;
use crate::types::{
    Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext, RunResult, Tool,
};

/// A number parameter in metres or a share.
pub(super) fn number(
    name: &str,
    label: &str,
    value: f64,
    min: f64,
    max: f64,
    unit: &str,
) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Number {
            min: Some(min),
            max: Some(max),
            integer: false,
            unit: unit.into(),
            placeholder: None,
        },
    )
    .default_value(json!(value))
}

pub fn tool() -> Tool {
    let d = Params::default();
    Tool {
        id: "pointcloud.ground".into(),
        label: "Zemin süzgeci".into(),
        category: CATEGORY.into(),
        description: "Nokta bulutunda zemini ayırır (SMRF): zemin noktaları 2 sınıfına alınır, zemin olmayan 2'ler 1 olur, öbür sınıflar kalır; sonuç LAS, LAZ ya da COPC olarak yazılır.".into(),
        help: Some([
            "Basit morfolojik süzgeç (Pingel ve ark. 2013): son dönüşlerin her hücredeki en düşük kotundan yüzey; boş hücreler komşularından doldurulur; düşük aykırılar ve nesneler (binalar, ağaçlar) aşamalı açmayla ayıklanır; kalan yüzeyden eşik ve eğimle zemin seçilir.",
            "Hücre zeminin ayrıntısıdır (1 m); pencere en büyük nesnenin genişliği kadar olmalı (18 m); eğim arazinin en dik yeri (0,15 = %15); eşik ve ölçek çarpanı noktanın yüzeye ne kadar yakın olacağıdır.",
            "Sonraki adım Yüksekliğe göre sınıfla: zeminden yüksekliğe göre bitki sınıfları.",
        ]
        .join("\n\n")),
        keywords: ["zemin", "ground", "smrf", "filtre", "yüzey", "dtm", "sınıf", "lidar"]
            .map(String::from)
            .to_vec(),
        aliases: ["ZEMINSUZ", "YUZEYFILTRE", "ZEMIN"].map(String::from).to_vec(),
        icon: Some("pointCloudGround".into()),
        parameters: vec![
            clouds_param("Nokta bulutu", "Zemini ayrılacak bulut."),
            number("cell", "Hücre", d.cell, 0.1, 100.0, "m"),
            number("window", "Pencere", d.window, 0.0, 500.0, "m"),
            number("slope", "Eğim", d.slope, 0.0, 10.0, ""),
            number("threshold", "Eşik", d.threshold, 0.0, 100.0, "m").advanced(),
            number("scalar", "Ölçek çarpanı", d.scalar, 0.0, 100.0, "").advanced(),
            ParamDef::new("lastOnly", "Yalnız son dönüşler", ParamKind::Boolean)
                .default_value(json!(true))
                .describe("Yüzeyi yalnız son (ve tek) dönüşler kurar; bitki örtüsünün altındaki zemin için.")
                .advanced(),
            format_param(),
            output_param("-zemin", &[".laz", ".las"]),
            add_param(),
            result_layer_param("Zemini ayrılmış bulut", "#8A6A3E"),
        ],
        outputs: vec![OutputDef::new("ground", "Zemin noktası", OutputKind::Number)],
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

fn go(
    files: &dyn Files,
    r: &Resolved<'_>,
    cloud: &kentos_contracts::PointCloudEntity,
    feedback: &mut dyn Feedback,
) -> Result<RunResult, String> {
    let d = Params::default();
    let p = Params {
        cell: r.number("cell").unwrap_or(d.cell),
        slope: r.number("slope").unwrap_or(d.slope),
        window: r.number("window").unwrap_or(d.window),
        threshold: r.number("threshold").unwrap_or(d.threshold),
        scalar: r.number("scalar").unwrap_or(d.scalar),
        last_only: r
            .values
            .get("lastOnly")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true),
    };
    let sources = &cloud.cloud.sources;
    let ins = inputs(files, sources)?;
    let plan =
        kentos_pointcloud::ops::convert::plan(&ins, r.text("format") != "las").map_err(|e| e.0)?;
    let layout = plan.spec.layout();
    let (scale, offset) = (plan.spec.scale, plan.spec.offset);
    let mut g = Ground::new(cloud.cloud.rect(), p)
        .ok_or("Zemin süzgecinin ızgarası kurulamadı: hücre çok küçük ya da değerler geçersiz.")?;
    let at = place(files, r, &sources[0], "-zemin")?;
    let copc = at.format == kentos_contracts::CloudFormat::Copc;
    let write_to = if copc { 0.6 } else { 1.0 };
    pass(
        files,
        sources,
        &ins,
        &plan.spec,
        feedback,
        "Hücreler okunuyor",
        0.0,
        write_to * 0.45,
        &mut |recs, _| {
            g.feed(&layout, recs, scale, offset);
            Ok(())
        },
    )?;
    feedback.progress(write_to * 0.45, "Zemin yüzeyi kuruluyor");
    if !g.surface() {
        return Err("Bulutta yüzeyi kuracak nokta yok (son dönüş yok ya da bulut boş).".to_owned());
    }
    if feedback.canceled() {
        return Err(super::STOPPED.to_owned());
    }
    let mut out = Out::new(files, &at.written, plan.spec.clone())?;
    let mut ground = 0u64;
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
            ground += g.classify(&layout, recs, scale, offset);
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
    let mut result = written(
        r,
        Some(object),
        format!(
            "{} noktadan {} nokta zemin (2) sınıfında; “{}” yazıldı.",
            count_words(count),
            count_words(ground),
            at.path
        ),
    );
    result.outputs.insert("ground".to_owned(), json!(ground));
    Ok(result)
}
