//! Rasterleştir (docs/adr/0207 §7; ArcGIS LAS Dataset To Raster, PDAL's
//! writers.gdal): the cloud's points as a grid of heights on the cell's
//! multiples: each cell's least, most or mean height, its count, or IDW
//! (the points within the radius weighted by the inverse square of their
//! distance); the classes taken all or those listed (Zemin: a DTM). The
//! result is a tiled, Deflate, 32-bit GeoTIFF (ADR 0204's writer), empty
//! cells nodata −9999, and, asked, a raster object in a ramp lit by its
//! relief (`ops::raster`).

use kentos_contracts::{
    Entity, EntityBase, RasterEntity, RasterFields, RasterRender, RasterSample, RasterStretch,
    RasterStyle,
};
use kentos_formats::raster::write::{Geo, Image, Writer};
use kentos_formats::raster::{Samples, TILE};
use kentos_pointcloud::ops::raster::{Frame, NODATA, Rasterize, Value};
use serde_json::json;

use super::ground::number;
use super::{
    CATEGORY, add_param, broke, clouds_param, count_words, files_of, inputs, one_cloud,
    output_param, pass, result_layer_param, targets, written,
};
use crate::files::Files;
use crate::types::{
    EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext,
    RunResult, Tool, Values,
};

fn value(v: &Values) -> &str {
    v.get("value")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("min")
}

fn classes_of(v: &Values) -> &str {
    v.get("classes")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("all")
}

/// The classes listed: “2, 6, 9”; none when the text names none (or one past 255).
fn listed(text: &str) -> Option<Vec<u8>> {
    let mut out: Vec<u8> = Vec::new();
    for part in text.split([',', ';', ' ']).filter(|p| !p.trim().is_empty()) {
        out.push(part.trim().parse().ok()?);
    }
    out.sort_unstable();
    out.dedup();
    (!out.is_empty()).then_some(out)
}

fn check(v: &Values) -> Option<String> {
    if classes_of(v) == "list"
        && listed(
            v.get("list")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(""),
        )
        .is_none()
    {
        return Some("Sınıfları virgülle yazın (0–255): ör. 2, 6.".to_owned());
    }
    None
}

pub fn tool() -> Tool {
    Tool {
        id: "pointcloud.rasterize".into(),
        label: "Rasterleştir".into(),
        category: CATEGORY.into(),
        description: "Nokta bulutundan yükseklik rasteri (GeoTIFF) üretir: hücrenin en düşük, en yüksek ya da ortalama kotu, nokta sayısı ya da IDW; zemin noktalarından sayısal arazi modeli (DTM).".into(),
        help: Some([
            "Izgara hücre boyunun katlarına oturur; boş hücre nodata'dır (−9999). Hücre boş bırakılırsa noktaların ortalama aralığının iki katı alınır.",
            "IDW: hücrenin merkezine yarıçap içindeki noktaların, uzaklığın karesinin tersiyle ağırlıklı ortalaması; merkezdeki nokta değerin kendisidir.",
            "Sınıflar: hepsi, yalnız zemin (2; DTM) ya da yazılanlar. Sonuç 32 bitlik, karolu ve Deflate'li GeoTIFF'tir; çizime ekle açıksa rampa ve gölgeyle raster olarak eklenir.",
        ]
        .join("\n\n")),
        keywords: ["rasterleştir", "dem", "dtm", "dsm", "sayısal arazi modeli", "idw", "geotiff", "grid"]
            .map(String::from)
            .to_vec(),
        aliases: ["RASTERLESTIR", "BULUTDEM", "DTMURET"].map(String::from).to_vec(),
        icon: Some("pointCloudRaster".into()),
        parameters: vec![
            clouds_param("Nokta bulutu", "Rasterleştirilecek bulut."),
            ParamDef::new(
                "cell",
                "Hücre",
                ParamKind::Number {
                    min: Some(0.01),
                    max: Some(10_000.0),
                    integer: false,
                    unit: "m".into(),
                    placeholder: Some("Otomatik".into()),
                },
            )
            .optional()
            .describe("Boşsa noktaların ortalama aralığının iki katı."),
            ParamDef::new(
                "value",
                "Değer",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("min", "En düşük"),
                        EnumOption::new("max", "En yüksek"),
                        EnumOption::new("mean", "Ortalama"),
                        EnumOption::new("count", "Nokta sayısı"),
                        EnumOption::new("idw", "IDW"),
                    ],
                },
            )
            .default_value(json!("min")),
            number("radius", "IDW yarıçapı", 1.0, 0.01, 10_000.0, "m").shown_when(|v| value(v) == "idw"),
            ParamDef::new(
                "classes",
                "Sınıflar",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("all", "Hepsi"),
                        EnumOption::new("ground", "Zemin (2)"),
                        EnumOption::new("list", "Yazılanlar"),
                    ],
                },
            )
            .default_value(json!("all")),
            ParamDef::new(
                "list",
                "Sınıf numaraları",
                ParamKind::Text {
                    placeholder: Some("2, 6".into()),
                    max_length: Some(200),
                    allow_empty: true,
                },
            )
            .default_value(json!(""))
            .shown_when(|v| classes_of(v) == "list"),
            output_param("-dem", &[".tif"]),
            add_param(),
            result_layer_param("Yükseklik rasteri", "#7A6B5B"),
        ],
        outputs: vec![OutputDef::new("cells", "Dolu hücre", OutputKind::Number)],
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

/// The default cell: twice the points' mean spacing over the plan, rounded up to the centimetre.
pub fn default_cell(bounds: &[f64; 6], count: u64) -> f64 {
    let area = (bounds[3] - bounds[0]) * (bounds[4] - bounds[1]);
    if area.is_nan() || area <= 0.0 || count == 0 {
        return 1.0;
    }
    let spacing = (area / count as f64).sqrt();
    ((2.0 * spacing * 100.0).ceil() / 100.0).max(0.01)
}

fn go(
    files: &dyn Files,
    r: &Resolved<'_>,
    cloud: &kentos_contracts::PointCloudEntity,
    feedback: &mut dyn Feedback,
) -> Result<RunResult, String> {
    let c = &cloud.cloud;
    let cell = r
        .number("cell")
        .filter(|v| *v > 0.0)
        .unwrap_or_else(|| default_cell(&c.bounds, c.count));
    let frame = Frame::over(c.rect(), cell)
        .ok_or("Raster çok büyük olur: hücreyi büyütün (en çok 400 milyon hücre).")?;
    let v = match r.text("value") {
        "max" => Value::Max,
        "mean" => Value::Mean,
        "count" => Value::Count,
        "idw" => Value::Idw(r.number("radius").unwrap_or(cell)),
        _ => Value::Min,
    };
    let classes = match r.text("classes") {
        "ground" => Some(vec![2u8]),
        "list" => listed(r.text("list")),
        _ => None,
    };
    let sources = &c.sources;
    let ins = inputs(files, sources)?;
    let plan = kentos_pointcloud::ops::convert::plan(&ins, false).map_err(|e| e.0)?;
    let layout = plan.spec.layout();
    let (scale, offset) = (plan.spec.scale, plan.spec.offset);
    let mut grid = Rasterize::new(frame, v, classes.as_deref());
    pass(
        files,
        sources,
        &ins,
        &plan.spec,
        feedback,
        "Hücreler dolduruluyor",
        0.0,
        0.8,
        &mut |recs, _| {
            grid.feed(&layout, recs, scale, offset);
            Ok(())
        },
    )?;
    let values = grid.values();
    let filled = values.iter().filter(|v| **v != NODATA).count();
    let path = files.output_path(r.text("output"), Some((&sources[0]).into()), "-dem", ".tif")?;
    feedback.progress(0.85, "GeoTIFF yazılıyor");
    write_tiff(files, &path, &frame, &values, c.srid)?;
    let (lo, hi) = values
        .iter()
        .filter(|v| **v != NODATA)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &v| {
            (a.min(v), b.max(v))
        });
    let object =
        (filled > 0).then(|| raster_object(&frame, &path, c.srid, (lo, hi), &r.layer("layer").id));
    let mut result = written(
        r,
        object,
        format!(
            "{} × {} hücrelik raster (hücre {} m), {} dolu hücre; “{path}” yazıldı.",
            frame.cols,
            frame.rows,
            crate::text::js_number(cell),
            count_words(filled as u64)
        ),
    );
    result.outputs.insert("cells".to_owned(), json!(filled));
    Ok(result)
}

/// The grid as a tiled, Deflate, 32-bit GeoTIFF.
fn write_tiff(
    files: &dyn Files,
    path: &str,
    f: &Frame,
    values: &[f64],
    srid: u32,
) -> Result<(), String> {
    let (w, h) = (
        u32::try_from(f.cols).map_err(|_| "Raster çok geniş.")?,
        u32::try_from(f.rows).map_err(|_| "Raster çok yüksek.")?,
    );
    let image = Image {
        width: w,
        height: h,
        bands: 1,
        sample: RasterSample::F32,
        alpha: false,
        geo: Some(Geo {
            affine: f.affine(),
            epsg: (srid != 0).then_some(srid),
            geographic: false,
        }),
        nodata: Some(NODATA),
    };
    let (mut writer, head) = Writer::new(vec![image], 6).map_err(|e| e.0)?;
    let mut sink = files.create(path)?;
    sink.write(&head)?;
    let tiles_x = w.div_ceil(TILE);
    let tiles_y = h.div_ceil(TILE);
    let mut tile = vec![NODATA as f32; (TILE * TILE) as usize];
    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            tile.fill(NODATA as f32);
            for j in 0..TILE {
                let row = (ty * TILE + j) as usize;
                if row >= f.rows {
                    break;
                }
                for i in 0..TILE {
                    let col = (tx * TILE + i) as usize;
                    if col >= f.cols {
                        break;
                    }
                    tile[(j * TILE + i) as usize] = values[row * f.cols + col] as f32;
                }
            }
            let bytes = writer
                .tile(0, tx, ty, &Samples::F32(tile.clone()))
                .map_err(|e| e.0)?;
            sink.write(&bytes)?;
        }
    }
    let (tail, header) = writer.finish().map_err(|e| e.0)?;
    sink.write(&tail)?;
    sink.patch(0, &header)?;
    sink.finish()
}

/// The raster object of the grid: its ramp lit by its relief over its values' range.
fn raster_object(f: &Frame, path: &str, srid: u32, (lo, hi): (f64, f64), layer: &str) -> Entity {
    Entity::Raster(RasterEntity {
        base: EntityBase {
            id: 0,
            layer_id: layer.to_owned(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
        },
        raster: RasterFields {
            affine: f.affine(),
            width: f.cols as u32,
            height: f.rows as u32,
            bands: 1,
            sample: RasterSample::F32,
            asset: None,
            file: Some(path.to_owned()),
            url: None,
            srid,
            style: RasterStyle {
                render: RasterRender::RampShade,
                bands: vec![1],
                stretch: RasterStretch::MinMax,
                min: Some(lo),
                max: Some(if hi > lo { hi } else { lo + 1.0 }),
                ramp: Some("Arazi".to_owned()),
                invert: false,
                azimuth: None,
                altitude: None,
                z_factor: None,
                nodata: Some(NODATA),
                resampling: kentos_contracts::RasterResampling::Bilinear,
            },
            opacity: None,
        },
    })
}
