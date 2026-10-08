//! Birleştir (docs/adr/0207 §7; PDAL's filters.merge, LAStools' lasmerge):
//! the files of the selected clouds (a virtual cloud's every member) in one
//! LAS 1.4 file: the widest point format they need (6, 7 or 8), the finest
//! scale, the first file's offset and system; a coordinate that does not
//! land on the result's grid is rounded and counted; extra bytes that
//! differ between the files are left out, said (`ops::convert`).

use std::collections::BTreeSet;

use serde_json::json;

use super::{
    CATEGORY, Out, add_param, broke, cloud_object, clouds, clouds_param, count_words, files_of,
    format_param, grid_notes, inputs, output_param, pass, place, result_layer_param, settle,
    targets, written,
};
use crate::files::Files;
use crate::types::{Feedback, OutputDef, OutputKind, Resolved, RunContext, RunResult, Tool};

pub fn tool() -> Tool {
    Tool {
        id: "pointcloud.merge".into(),
        label: "Bulutları birleştir".into(),
        category: CATEGORY.into(),
        description: "Seçili nokta bulutlarının (sanal bulutun bütün dosyalarının) noktalarını tek LAS, LAZ ya da COPC dosyasında toplar.".into(),
        help: Some([
            "Sonuç LAS 1.4'tür: nokta biçimi dosyaların gerektirdiğinin en genişi (6, 7 ya da 8), ölçek en incesi, öteleme ve sistem ilk dosyanınki.",
            "Koordinatlar tam sayı olarak kayıpsız taşınabiliyorsa taşınır; taşınamayan (ölçeği daha ince bir ızgaraya düşmeyen) noktalar yuvarlanır ve sayısı söylenir. Dosyaların ek baytları farklıysa alınmaz.",
            "Bulutlar aynı koordinat sisteminde olmalı.",
        ]
        .join("\n\n")),
        keywords: ["birleştir", "merge", "topla", "tek dosya", "nokta bulutu"]
            .map(String::from)
            .to_vec(),
        aliases: ["BULUTBIRLESTIR"].map(String::from).to_vec(),
        icon: Some("pointCloudMerge".into()),
        parameters: vec![
            clouds_param("Nokta bulutları", "Birleştirilecek bulutlar; birden çok dosyalı tek bir sanal bulut da olur."),
            format_param(),
            output_param("-birlesik", &[".laz", ".las"]),
            add_param(),
            result_layer_param("Birleşik bulut", "#7A5C9A"),
        ],
        outputs: vec![OutputDef::new("count", "Nokta", OutputKind::Number)],
        targets: targets(),
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(r: &Resolved<'_>, _cx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let list = clouds(r, "input");
    let sources: Vec<_> = list
        .iter()
        .flat_map(|c| c.cloud.sources.iter().cloned())
        .collect();
    if sources.len() < 2 {
        return RunResult::refused(
            "Birleştirmek için en az iki dosya gerekir: birden çok bulut ya da birden çok dosyalı bir sanal bulut seçin.".to_owned(),
        );
    }
    let systems: BTreeSet<u32> = list.iter().map(|c| c.cloud.srid).collect();
    if systems.len() > 1 {
        return RunResult::refused(
            "Bulutlar farklı koordinat sistemlerinde; KentOS bulutu yeniden izdüşürmez. Aynı sistemdeki bulutları seçin.".to_owned(),
        );
    }
    match go(&*files, r, &list, &sources, feedback) {
        Ok(result) => result,
        Err(why) => broke(why),
    }
}

fn go(
    files: &dyn Files,
    r: &Resolved<'_>,
    list: &[&kentos_contracts::PointCloudEntity],
    sources: &[kentos_contracts::CloudSource],
    feedback: &mut dyn Feedback,
) -> Result<RunResult, String> {
    let ins = inputs(files, sources)?;
    let plan =
        kentos_pointcloud::ops::convert::plan(&ins, r.text("format") != "las").map_err(|e| e.0)?;
    let at = place(files, r, &sources[0], "-birlesik")?;
    let copc = at.format == kentos_contracts::CloudFormat::Copc;
    let write_to = if copc { 0.6 } else { 1.0 };
    let mut out = Out::new(files, &at.written, plan.spec.clone())?;
    let rounded = pass(
        files,
        sources,
        &ins,
        &plan.spec,
        feedback,
        "Birleştiriliyor",
        0.0,
        write_to,
        &mut |recs, _| out.put(recs),
    )?;
    let (count, bounds) = out.finish()?;
    settle(files, &at, feedback, write_to, 1.0)?;
    grid_notes(rounded, plan.dropped_extra, feedback);
    let first = list[0];
    let object = cloud_object(
        &at,
        count,
        bounds,
        first.cloud.srid,
        first.cloud.style.clone(),
        &r.layer("layer").id,
    );
    let mut result = written(
        r,
        Some(object),
        format!(
            "{} dosyanın {} noktası “{}” dosyasında birleşti.",
            sources.len(),
            count_words(count),
            at.path
        ),
    );
    result.outputs.insert("count".to_owned(), json!(count));
    Ok(result)
}
