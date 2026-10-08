//! Seyrelt (docs/adr/0207 §7; Netcad's Nokta Bulutu Seyreltme, PDAL's
//! filters.voxelcenternearestneighbor, filters.sample and filters.decimation):
//! Hücreyle keeps in each 3D cell the point nearest its centre (two passes:
//! decide, then write), Yarıçapla drops a point nearer than the radius to
//! one kept (one pass, in the file's order), Her n'inci keeps the first and
//! every n-th after it. The machines are the core's (`ops::thin`).

use kentos_pointcloud::ops::thin::{Cells, Radius, nth};
use serde_json::json;

use super::{
    CATEGORY, Out, add_param, broke, cloud_object, clouds_param, count_words, files_of,
    format_param, grid_notes, inputs, one_cloud, output_param, pass, place, result_layer_param,
    settle, targets, written,
};
use crate::types::{
    EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext,
    RunResult, Tool, Values,
};

fn method(v: &Values) -> &str {
    v.get("method")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("cell")
}

pub fn tool() -> Tool {
    Tool {
        id: "pointcloud.thin".into(),
        label: "Seyrelt".into(),
        category: CATEGORY.into(),
        description: "Nokta bulutunu seyreltir: her hücrede merkezine en yakın nokta, birbirine yarıçaptan yakın olmayan noktalar ya da her n'inci nokta kalır; sonuç LAS, LAZ ya da COPC olarak yazılır.".into(),
        help: Some([
            "Hücreyle: bulut, kenarı verilen boyda küplere bölünür; her küpte merkezine en yakın nokta kalır (eşitlikte dosyada önce gelen). Düzgün yoğunluk ister.",
            "Yarıçapla: noktalar dosyanın sırasıyla okunur; kalan bir noktaya yarıçaptan yakın olan atılır (üç boyutlu uzaklık).",
            "Her n'inci: ilk nokta ve ardından her n'inci nokta kalır; en hızlısı, yoğunluğu korumaz.",
            "Noktaların bütün alanları (sınıf, renk, dönüş, GPS zamanı, ek baytlar) olduğu gibi yazılır.",
        ]
        .join("\n\n")),
        keywords: ["seyrelt", "thin", "decimate", "voxel", "sample", "nokta bulutu", "lidar"]
            .map(String::from)
            .to_vec(),
        aliases: ["SEYRELT", "BULUTSEYRELT"].map(String::from).to_vec(),
        icon: Some("pointCloudThin".into()),
        parameters: vec![
            clouds_param("Nokta bulutu", "Seyreltilecek bulut (sanal bulutun bütün dosyaları birlikte)."),
            ParamDef::new(
                "method",
                "Yöntem",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("cell", "Hücreyle"),
                        EnumOption::new("radius", "Yarıçapla"),
                        EnumOption::new("nth", "Her n'inci"),
                    ],
                },
            )
            .default_value(json!("cell")),
            ParamDef::new(
                "size",
                "Hücre ya da yarıçap",
                ParamKind::Number {
                    min: Some(0.001),
                    max: Some(10_000.0),
                    integer: false,
                    unit: "m".into(),
                    placeholder: None,
                },
            )
            .default_value(json!(0.5))
            .shown_when(|v| method(v) != "nth"),
            ParamDef::new(
                "n",
                "n",
                ParamKind::Number {
                    min: Some(2.0),
                    max: Some(1_000_000.0),
                    integer: true,
                    unit: "adet".into(),
                    placeholder: None,
                },
            )
            .default_value(json!(10))
            .shown_when(|v| method(v) == "nth"),
            format_param(),
            output_param("-seyrek", &[".laz", ".las"]),
            add_param(),
            result_layer_param("Seyreltilmiş bulut", "#4C9A6A"),
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
    match go(&*files, r, cloud, feedback) {
        Ok(result) => result,
        Err(why) => broke(why),
    }
}

fn go(
    files: &dyn crate::files::Files,
    r: &Resolved<'_>,
    cloud: &kentos_contracts::PointCloudEntity,
    feedback: &mut dyn Feedback,
) -> Result<RunResult, String> {
    let sources = &cloud.cloud.sources;
    let ins = inputs(files, sources)?;
    let plan =
        kentos_pointcloud::ops::convert::plan(&ins, r.text("format") != "las").map_err(|e| e.0)?;
    let layout = plan.spec.layout();
    let (scale, offset) = (plan.spec.scale, plan.spec.offset);
    let at = place(files, r, &sources[0], "-seyrek")?;
    let size = r.number("size").unwrap_or(0.5);
    let n = r.number("n").unwrap_or(10.0).max(1.0) as u64;
    let copc = at.format == kentos_contracts::CloudFormat::Copc;
    let write_to = if copc { 0.6 } else { 1.0 };
    let mut out = Out::new(files, &at.written, plan.spec.clone())?;
    let mut keep: Vec<u8> = Vec::new();
    let rounded = match r.text("method") {
        "radius" => {
            let mut m = Radius::new(size).ok_or("Yarıçap sıfırdan büyük olmalı.")?;
            let mut kept = Vec::new();
            pass(
                files,
                sources,
                &ins,
                &plan.spec,
                feedback,
                "Seyreltiliyor",
                0.0,
                write_to,
                &mut |recs, _| {
                    m.feed(&layout, recs, scale, offset, &mut kept);
                    keep.clear();
                    for (rec, k) in recs.chunks_exact(layout.len).zip(&kept) {
                        if *k {
                            keep.extend_from_slice(rec);
                        }
                    }
                    out.put(&keep)
                },
            )?
        }
        "nth" => pass(
            files,
            sources,
            &ins,
            &plan.spec,
            feedback,
            "Seyreltiliyor",
            0.0,
            write_to,
            &mut |recs, first| {
                keep.clear();
                for (i, rec) in recs.chunks_exact(layout.len).enumerate() {
                    if nth(first + i as u64, n) {
                        keep.extend_from_slice(rec);
                    }
                }
                out.put(&keep)
            },
        )?,
        _ => {
            let mut m = Cells::new(size).ok_or("Hücre sıfırdan büyük olmalı.")?;
            pass(
                files,
                sources,
                &ins,
                &plan.spec,
                feedback,
                "Hücreler okunuyor",
                0.0,
                write_to / 2.0,
                &mut |recs, first| {
                    m.feed(&layout, recs, scale, offset, first);
                    Ok(())
                },
            )?;
            let kept = m.kept();
            drop(m);
            let mut next = 0usize;
            pass(
                files,
                sources,
                &ins,
                &plan.spec,
                feedback,
                "Seyreltilmiş bulut yazılıyor",
                write_to / 2.0,
                write_to,
                &mut |recs, first| {
                    keep.clear();
                    let count = (recs.len() / layout.len) as u64;
                    while next < kept.len() && kept[next] < first + count {
                        let i = (kept[next] - first) as usize;
                        keep.extend_from_slice(&recs[i * layout.len..(i + 1) * layout.len]);
                        next += 1;
                    }
                    out.put(&keep)
                },
            )?
        }
    };
    let (count, bounds) = out.finish()?;
    settle(files, &at, feedback, write_to, 1.0)?;
    grid_notes(rounded, plan.dropped_extra, feedback);
    let total: u64 = ins.iter().map(|i| i.head.count).sum();
    let object = cloud_object(
        &at,
        count,
        bounds,
        cloud.cloud.srid,
        cloud.cloud.style.clone(),
        &r.layer("layer").id,
    );
    let mut result = written(
        r,
        Some(object),
        format!(
            "{} noktadan {} nokta kaldı; “{}” yazıldı.",
            count_words(total),
            count_words(count),
            at.path
        ),
    );
    result.outputs.insert("count".to_owned(), json!(count));
    Ok(result)
}
