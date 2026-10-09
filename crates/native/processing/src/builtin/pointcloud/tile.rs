//! Karola (docs/adr/0207 §7; PDAL's filters.splitter, LAStools' lastile):
//! the cloud's points in tiles of the given size, their corners on the
//! size's multiples; a point on a tile's edge goes to the tile on its right
//! and above it (`ops::tile`). Each tile is a file `<ad>_<x>_<y>` beside
//! the others, its lower left corner in its name; asked, a virtual cloud
//! (.vpc) of them too. A first pass counts the tiles' points; the tiles are
//! written in groups of at most 200 open files, a pass each.

use std::collections::{BTreeMap, HashMap};

use kentos_contracts::{CloudFormat, CloudSource, EntityBase, PointCloudEntity, PointCloudFields};
use kentos_pointcloud::ops::tile::{name, tile_of};
use serde_json::json;

use super::ground::number;
use super::{
    CATEGORY, Out, add_param, broke, clouds_param, count_words, extension, files_of, format_param,
    grid_notes, inputs, one_cloud, output_param, pass, result_layer_param, targets, written,
};
use crate::files::Files;
use crate::types::{
    Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext, RunResult, Tool,
};

/// The most tiles written in one pass (open files).
const OPEN_MOST: usize = 200;

pub fn tool() -> Tool {
    Tool {
        id: "pointcloud.tile".into(),
        label: "Karola".into(),
        category: CATEGORY.into(),
        description: "Nokta bulutunu verilen boyda karolara böler: her karo kendi dosyasıdır (<ad>_<x>_<y>), isterseniz karoların sanal bulutu (.vpc) da yazılır.".into(),
        help: Some([
            "Karoların köşeleri karo boyunun katlarındadır; sınırdaki nokta sağdaki ve üstteki karoya düşer. Dosyanın adındaki x ve y karonun sol alt köşesidir.",
            "Çıktı dosyası karoların klasörünü ve adını verir: “ada.laz” seçilirse karolar o klasörde “ada_x_y.laz” olur; boşsa kaynağın yanında kaynağın adıyla.",
            "Çizime ekle açıksa karolar tek sanal bulut olarak eklenir.",
        ]
        .join("\n\n")),
        keywords: ["karola", "tile", "böl", "pafta", "split", "nokta bulutu"]
            .map(String::from)
            .to_vec(),
        aliases: ["KAROLA", "BULUTKAROLA"].map(String::from).to_vec(),
        icon: Some("pointCloudTile".into()),
        parameters: vec![
            clouds_param("Nokta bulutu", "Karolara bölünecek bulut."),
            number("size", "Karo boyu", 100.0, 1.0, 100_000.0, "m"),
            ParamDef::new("vpc", "Sanal bulut da yaz", ParamKind::Boolean)
                .default_value(json!(true))
                .describe("Karoların yanına <ad>.vpc yazılır (QGIS ve PDAL'ın sanal bulutu)."),
            format_param(),
            output_param("", &[".laz", ".las"]),
            add_param(),
            result_layer_param("Karolar", "#9A7A4C"),
        ],
        outputs: vec![OutputDef::new("tiles", "Karo", OutputKind::Number)],
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

/// A finished tile: its place in the grid, its file, points and bounds.
type Done = ((i64, i64), String, u64, [f64; 6]);

fn go(
    files: &dyn Files,
    r: &Resolved<'_>,
    cloud: &PointCloudEntity,
    feedback: &mut dyn Feedback,
) -> Result<RunResult, String> {
    let size = r.number("size").unwrap_or(100.0);
    if size.is_nan() || size <= 0.0 {
        return Err("Karo boyu sıfırdan büyük olmalı.".to_owned());
    }
    let format = match r.text("format") {
        "las" => CloudFormat::Las,
        _ => CloudFormat::Laz,
    };
    if r.text("format") == "copc" {
        feedback.warn(
            "Karolar LAZ olarak yazıldı; karoların dizini ilk gösterilişlerinde hazırlanır."
                .to_owned(),
        );
    }
    let sources = &cloud.cloud.sources;
    let ins = inputs(files, sources)?;
    let plan =
        kentos_pointcloud::ops::convert::plan(&ins, format == CloudFormat::Laz).map_err(|e| e.0)?;
    let layout = plan.spec.layout();
    let (scale, offset) = (plan.spec.scale, plan.spec.offset);
    // Where the tiles go: the chosen file's folder and name, or beside the source with its name.
    let base = files.output_path(r.text("output"), Some((&sources[0]).into()), "", "")?;
    let (folder, stem) = match base.rfind(['/', '\\']) {
        Some(i) => (base[..=i].to_owned(), base[i + 1..].to_owned()),
        None => (String::new(), base.clone()),
    };
    let stem = {
        let lower = stem.to_ascii_lowercase();
        [".copc.laz", ".laz", ".las"]
            .iter()
            .find(|e| lower.ends_with(*e))
            .map_or(stem.clone(), |e| stem[..stem.len() - e.len()].to_owned())
    };
    // The first pass: each tile's points.
    let mut counts: BTreeMap<(i64, i64), u64> = BTreeMap::new();
    pass(
        files,
        sources,
        &ins,
        &plan.spec,
        feedback,
        "Karolar sayılıyor",
        0.0,
        0.2,
        &mut |recs, _| {
            for rec in recs.chunks_exact(layout.len) {
                let x = f64::from(layout.x(rec)) * scale[0] + offset[0];
                let y = f64::from(layout.y(rec)) * scale[1] + offset[1];
                *counts.entry(tile_of(x, y, size)).or_default() += 1;
            }
            Ok(())
        },
    )?;
    let tiles: Vec<(i64, i64)> = counts.keys().copied().collect();
    let groups: Vec<&[(i64, i64)]> = tiles.chunks(OPEN_MOST).collect();
    let mut made: Vec<(String, u64, [f64; 6])> = Vec::with_capacity(tiles.len());
    let mut rounded = 0;
    for (g, group) in groups.iter().enumerate() {
        let span = 0.8 / groups.len() as f64;
        let from = 0.2 + span * g as f64;
        let mut outs: HashMap<(i64, i64), (String, Out<'_>)> = HashMap::new();
        for &t in *group {
            let path = format!("{folder}{}{}", name(&stem, t, size), extension(format));
            let out = Out::new(files, &path, plan.spec.clone())?;
            outs.insert(t, (path, out));
        }
        let mut parts: HashMap<(i64, i64), Vec<u8>> = HashMap::new();
        rounded = pass(
            files,
            sources,
            &ins,
            &plan.spec,
            feedback,
            "Karolar yazılıyor",
            from,
            from + span,
            &mut |recs, _| {
                for rec in recs.chunks_exact(layout.len) {
                    let x = f64::from(layout.x(rec)) * scale[0] + offset[0];
                    let y = f64::from(layout.y(rec)) * scale[1] + offset[1];
                    let t = tile_of(x, y, size);
                    if outs.contains_key(&t) {
                        parts.entry(t).or_default().extend_from_slice(rec);
                    }
                }
                for (t, bytes) in parts.iter_mut() {
                    if bytes.is_empty() {
                        continue;
                    }
                    if let Some((_, out)) = outs.get_mut(t) {
                        out.put(bytes)?;
                    }
                    bytes.clear();
                }
                Ok(())
            },
        )?;
        let mut done: Vec<Done> = Vec::new();
        for (t, (path, out)) in outs {
            let (count, bounds) = out.finish()?;
            done.push((t, path, count, bounds));
        }
        done.sort_by_key(|d| d.0);
        made.extend(done.into_iter().map(|(_, p, c, b)| (p, c, b)));
    }
    grid_notes(rounded, plan.dropped_extra, feedback);
    let vpc_path = format!("{folder}{stem}.vpc");
    if r.flag("vpc") {
        let items: Vec<kentos_pointcloud::vpc::Item> = made
            .iter()
            .map(|(p, c, b)| kentos_pointcloud::vpc::Item {
                href: format!("./{}", p.rsplit(['/', '\\']).next().unwrap_or(p)),
                count: *c,
                bounds: *b,
                wgs84: super::wgs84_corners(b, cloud.cloud.srid),
            })
            .collect();
        let epsg = (cloud.cloud.srid != 0).then_some(cloud.cloud.srid);
        let text = kentos_pointcloud::vpc::write(&items, epsg, None);
        let mut sink = files.create(&vpc_path)?;
        sink.write(text.as_bytes())?;
        sink.finish()?;
    }
    let total: u64 = made.iter().map(|m| m.1).sum();
    let sources: Vec<CloudSource> = made
        .iter()
        .map(|(p, c, b)| CloudSource {
            asset: None,
            file: Some(p.clone()),
            url: None,
            format,
            count: *c,
            bounds: *b,
        })
        .collect();
    let object = (!sources.is_empty()).then(|| {
        let mut bounds = [
            f64::INFINITY,
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for s in &sources {
            for k in 0..3 {
                bounds[k] = bounds[k].min(s.bounds[k]);
                bounds[k + 3] = bounds[k + 3].max(s.bounds[k + 3]);
            }
        }
        kentos_contracts::Entity::PointCloud(PointCloudEntity {
            base: EntityBase {
                id: 0,
                layer_id: r.layer("layer").id.clone(),
                color: None,
                attrs: Default::default(),
                label: None,
                symbol: None,
                line_weight: None,
            },
            cloud: PointCloudFields {
                sources,
                bounds,
                count: total,
                srid: cloud.cloud.srid,
                style: cloud.cloud.style.clone(),
                opacity: None,
            },
        })
    });
    let mut result = written(
        r,
        object,
        format!(
            "{} nokta {} karoya bölündü; “{folder}” klasörüne yazıldı{}.",
            count_words(total),
            made.len(),
            if r.flag("vpc") {
                format!(", sanal bulutu “{stem}.vpc”")
            } else {
                String::new()
            }
        ),
    );
    result.outputs.insert("tiles".to_owned(), json!(made.len()));
    Ok(result)
}
