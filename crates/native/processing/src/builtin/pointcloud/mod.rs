//! Nokta bulutu işlemleri (docs/adr/0207 §7; the desktop's, the web's side
//! waiting): Nokta bulutu alan sorgusu, Seyrelt, Zemin süzgeci, Yüksekliğe
//! göre sınıfla, Bulutu kırp, Bulutları birleştir, Karola, Rasterleştir and
//! Sınır çıkar. Each reads its
//! clouds whole, file by file in their order, through the host's files
//! ([`crate::files::Files`]); the decisions are the point cloud core's
//! machines (`kentos_pointcloud::ops`), the same for every host. They run in
//! the background with their progress and Durdur; a result file is
//! written beside its place and renamed when whole, and, asked, joins the
//! drawing on a new layer in the run's step.

pub mod area_stats;
pub mod boundary;
pub mod classify;
pub mod clip;
pub mod ground;
pub mod merge;
pub mod rasterize;
pub mod thin;
pub mod tile;

use std::collections::BTreeMap;
use std::sync::Arc;

use kentos_contracts::{
    CloudFormat, CloudSource, Entity, EntityBase, PointCloudEntity, PointCloudFields,
    PointCloudStyle,
};
use kentos_pointcloud::ops::convert::{Converter, Input};
use kentos_pointcloud::write::{Spec, Writer};
use serde_json::json;

use crate::files::{Files, NO_FILES, Sink};
use crate::types::{
    EnumOption, Feedback, NewLayerStyle, ParamDef, ParamKind, Resolved, RunResult, ScopeKind,
    Target, Values,
};

/// The tools' category.
pub const CATEGORY: &str = "pointcloud";

/// Where they run: always in the background (a cloud is large whatever its object count).
pub fn targets() -> Vec<Target> {
    vec![Target::Worker]
}

/// The clouds parameter.
pub fn clouds_param(label: &str, description: &str) -> ParamDef {
    ParamDef::new(
        "input",
        label,
        ParamKind::Features {
            kinds: Some(vec!["pointcloud".to_owned()]),
            scopes: Some(vec![ScopeKind::Selection, ScopeKind::Layer, ScopeKind::All]),
            writes: false,
        },
    )
    .describe(description)
}

/// The result's format: LAZ (the default), LAS or COPC.
pub fn format_param() -> ParamDef {
    ParamDef::new(
        "format",
        "Çıktı biçimi",
        ParamKind::Choice {
            options: vec![
                EnumOption::new("laz", "LAZ (sıkıştırılmış)"),
                EnumOption::new("las", "LAS"),
                EnumOption::new("copc", "COPC (dizinli LAZ)"),
            ],
        },
    )
    .default_value(json!("laz"))
    .describe("COPC hemen kat kat çizilir; LAS ve LAZ'ın dizini ilk gösterilişte hazırlanır.")
}

/// Where the result is written: a chosen path, or beside the source.
pub fn output_param(suffix: &str, accept: &[&str]) -> ParamDef {
    ParamDef::new(
        "output",
        "Çıktı dosyası",
        ParamKind::SaveFile {
            accept: accept.iter().map(|a| (*a).to_owned()).collect(),
            suffix: suffix.to_owned(),
        },
    )
    .optional()
    .describe("Boş bırakılırsa kaynağın yanına, adının sonuna eklenerek yazılır.")
}

/// Çizime ekle.
pub fn add_param() -> ParamDef {
    ParamDef::new("add", "Çizime ekle", ParamKind::Boolean)
        .default_value(json!(true))
        .describe("Sonuç yeni katmanda nesne olarak eklenir; bu işlemin adımında.")
}

fn adds(v: &Values) -> bool {
    v.get("add")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true)
}

/// The layer the result joins (shown while Çizime ekle is on).
pub fn result_layer_param(name: &str, color: &str) -> ParamDef {
    ParamDef::new(
        "layer",
        "Çıktı katmanı",
        ParamKind::Layer {
            new_layer_style: NewLayerStyle {
                color: Some(color.to_owned()),
                line_weight: Some(0.25),
                ..NewLayerStyle::default()
            },
        },
    )
    .default_value(json!({ "newName": name }))
    .describe("Bu adda katman yoksa oluşturulur.")
    .shown_when(adds)
}

/// The host's files, or why the run cannot go on (boxed: a run's result is large).
pub fn files_of(feedback: &dyn Feedback) -> Result<Arc<dyn Files>, Box<RunResult>> {
    feedback
        .files()
        .ok_or_else(|| Box::new(RunResult::refused(NO_FILES.to_owned())))
}

/// The clouds of the input, as objects.
pub fn clouds<'a>(r: &Resolved<'a>, name: &str) -> Vec<&'a PointCloudEntity> {
    r.features(name)
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::PointCloud(c) => Some(c),
            _ => None,
        })
        .collect()
}

/// A run's one cloud: refused with none or several.
pub fn one_cloud<'a>(r: &Resolved<'a>) -> Result<&'a PointCloudEntity, Box<RunResult>> {
    match clouds(r, "input").as_slice() {
        [one] => Ok(one),
        [] => Err(Box::new(RunResult::refused(
            "İşlenecek nokta bulutu yok: bir nokta bulutu seçin.".to_owned(),
        ))),
        many => Err(Box::new(RunResult::refused(format!(
            "{} nokta bulutu seçildi; bu araç bir bulutu işler. Bir bulut seçin (birden çok dosyayı Bulutları birleştir tek dosyada toplar).",
            many.len()
        )))),
    }
}

/// A file's stem as the result names take it.
pub fn stem(s: &CloudSource) -> String {
    let full = s
        .file
        .as_deref()
        .or(s.url.as_deref())
        .or(s.asset.as_deref())
        .unwrap_or("bulut");
    let base = full.rsplit(['/', '\\']).next().unwrap_or(full);
    let base = base.split(['?', '#']).next().unwrap_or(base);
    let lower = base.to_ascii_lowercase();
    let cut = [".copc.laz", ".laz", ".las", ".xyz", ".pts", ".txt", ".csv"]
        .iter()
        .find(|e| lower.ends_with(*e))
        .map_or(base.len(), |e| base.len() - e.len());
    let s = &base[..cut];
    if s.is_empty() {
        "bulut".to_owned()
    } else {
        s.to_owned()
    }
}

/// Every file's header and layout.
pub fn inputs(files: &dyn Files, sources: &[CloudSource]) -> Result<Vec<Input>, String> {
    sources
        .iter()
        .map(|s| files.open_cloud(s).map(|r| r.input().clone()))
        .collect()
}

/// Durdur, as the passes say it: an empty error.
pub const STOPPED: &str = "";

/// What a pass gives each piece of records to: the records, which it may
/// change, and the first one's number in the whole run.
pub type Each<'a> = dyn FnMut(&mut Vec<u8>, u64) -> Result<(), String> + 'a;

/// One pass over every file in order: each piece of records moved to
/// `spec`'s layout and grid, numbered from its first in the whole run, given
/// to `each` (which may change them). The points moved onto the result's
/// grid are counted; the share goes from `from` to `to` over the points.
#[allow(clippy::too_many_arguments)]
pub fn pass(
    files: &dyn Files,
    sources: &[CloudSource],
    inputs: &[Input],
    spec: &Spec,
    feedback: &mut dyn Feedback,
    label: &str,
    from: f64,
    to: f64,
    each: &mut Each<'_>,
) -> Result<u64, String> {
    let total: u64 = inputs.iter().map(|i| i.head.count).sum::<u64>().max(1);
    let mut seen = 0u64;
    let mut rounded = 0;
    let mut raw = Vec::new();
    let mut moved = Vec::new();
    for (s, input) in sources.iter().zip(inputs) {
        let conv = Converter::new(input, spec);
        let mut read = files.open_cloud(s)?;
        loop {
            if feedback.canceled() {
                return Err(STOPPED.to_owned());
            }
            raw.clear();
            if !read.next(&mut raw)? {
                break;
            }
            moved.clear();
            rounded += conv.convert(&raw, &mut moved).map_err(|e| e.0)?;
            let n = (moved.len() / conv.layout().len.max(1)) as u64;
            each(&mut moved, seen)?;
            seen += n;
            feedback.progress(
                from + (to - from) * (seen as f64 / total as f64).min(1.0),
                label,
            );
        }
    }
    Ok(rounded)
}

/// Threads compressing a result's chunks at once: the machine's but one, at most four.
pub fn compress_threads() -> usize {
    std::thread::available_parallelism()
        .map_or(1, |n| n.get().saturating_sub(1))
        .clamp(1, 4)
}

/// Whole chunks compressed at once, one thread each; the same bytes as one after the other.
pub fn compress_parallel(
    items: &[kentos_pointcloud::chunks::LazItem],
    chunks: &[&[u8]],
) -> kentos_pointcloud::Result<Vec<Vec<u8>>> {
    if chunks.len() < 2 {
        return kentos_pointcloud::write::compress_each(items, chunks);
    }
    std::thread::scope(|s| {
        let handles: Vec<_> = chunks
            .iter()
            .map(|c| s.spawn(move || kentos_pointcloud::chunks::compress(items, c)))
            .collect();
        handles
            .into_iter()
            .map(|h| {
                h.join().unwrap_or_else(|_| {
                    Err(kentos_pointcloud::PcError::new(
                        "Bir parça sıkıştırılamadı.",
                    ))
                })
            })
            .collect()
    })
}

/// A result file being written.
pub struct Out<'f> {
    sink: Box<dyn Sink + 'f>,
    writer: Writer,
}

impl<'f> Out<'f> {
    pub fn new(files: &'f dyn Files, path: &str, spec: Spec) -> Result<Out<'f>, String> {
        let mut sink = files.create(path)?;
        let (writer, head) = Writer::new(spec).map_err(|e| e.0)?;
        sink.write(&head)?;
        Ok(Out { sink, writer })
    }

    pub fn put(&mut self, records: &[u8]) -> Result<(), String> {
        let bytes = self
            .writer
            .put_with(records, compress_threads(), &mut compress_parallel)
            .map_err(|e| e.0)?;
        self.sink.write(&bytes)
    }

    /// The file whole: its points and bounds.
    pub fn finish(self) -> Result<(u64, [f64; 6]), String> {
        let Out { mut sink, writer } = self;
        let count = writer.count();
        let bounds = writer.bounds();
        let (tail, patches) = writer
            .finish_with(&mut compress_parallel)
            .map_err(|e| e.0)?;
        sink.write(&tail)?;
        for (at, b) in patches {
            sink.patch(at, &b)?;
        }
        sink.finish()?;
        Ok((count, bounds))
    }
}

/// Where a cloud result goes: its path and format, and the file the writer
/// writes (a COPC is first a LAZ beside it, then indexed).
pub struct Place {
    pub path: String,
    pub format: CloudFormat,
    pub written: String,
}

/// The extension of a format.
pub fn extension(f: CloudFormat) -> &'static str {
    match f {
        CloudFormat::Las => ".las",
        CloudFormat::Copc => ".copc.laz",
        CloudFormat::Laz | CloudFormat::Xyz => ".laz",
    }
}

/// The result's place from the run's values.
pub fn place(
    files: &dyn Files,
    r: &Resolved<'_>,
    beside: &CloudSource,
    suffix: &str,
) -> Result<Place, String> {
    let format = match r.text("format") {
        "las" => CloudFormat::Las,
        "copc" => CloudFormat::Copc,
        _ => CloudFormat::Laz,
    };
    let path = files.output_path(r.text("output"), Some(beside), suffix, extension(format))?;
    let written = if format == CloudFormat::Copc {
        format!("{path}.yaziliyor.laz")
    } else {
        path.clone()
    };
    Ok(Place {
        path,
        format,
        written,
    })
}

/// The result written whole: a COPC indexed from its LAZ (the share from `from` to `to`), the LAZ removed.
pub fn settle(
    files: &dyn Files,
    p: &Place,
    feedback: &mut dyn Feedback,
    from: f64,
    to: f64,
) -> Result<(), String> {
    if p.format != CloudFormat::Copc {
        return Ok(());
    }
    let done = files.copc(&p.written, &p.path, feedback, from, to);
    files.remove(&p.written);
    done
}

/// A cloud result as a new object on `layer`.
pub fn cloud_object(
    p: &Place,
    count: u64,
    bounds: [f64; 6],
    srid: u32,
    style: PointCloudStyle,
    layer: &str,
) -> Entity {
    let source = CloudSource {
        asset: None,
        file: Some(p.path.clone()),
        url: None,
        format: p.format,
        count,
        bounds,
    };
    Entity::PointCloud(PointCloudEntity {
        base: EntityBase {
            id: 0,
            layer_id: layer.to_owned(),
            color: None,
            attrs: BTreeMap::new(),
            label: None,
            symbol: None,
            line_weight: None,
        },
        cloud: PointCloudFields {
            sources: vec![source],
            bounds,
            count,
            srid,
            style,
            opacity: None,
        },
    })
}

/// A run's end that wrote a file: its summary, the object when asked.
pub fn written(r: &Resolved<'_>, object: Option<Entity>, summary: String) -> RunResult {
    let add: Vec<Entity> = object.into_iter().filter(|_| r.flag("add")).collect();
    RunResult {
        changes: (!add.is_empty()).then(|| crate::types::ChangeSet {
            add,
            ..crate::types::ChangeSet::default()
        }),
        summary: Some(summary),
        ..RunResult::default()
    }
}

/// A run that broke off: Durdur says nothing more, a fault refuses with its words.
pub fn broke(why: String) -> RunResult {
    if why == STOPPED {
        RunResult::default()
    } else {
        RunResult::refused(why)
    }
}

/// A whole number as the summaries write it (thousands apart).
pub fn count_words(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(ch);
    }
    out
}

/// A plan's corners `[x₁, y₁, …]` of `bounds` in WGS 84 (longitude, latitude)
/// from the system `srid`, for a virtual cloud's footprint; none without a way there.
pub fn wgs84_corners(bounds: &[f64; 6], srid: u32) -> Option<[[f64; 2]; 4]> {
    use kentos_geometry_core::crs::{Datum, System, transform};
    use kentos_geometry_core::vec2::Vec2;
    if srid == 0 {
        return None;
    }
    let from = kentos_project::crs::system(srid)?.transform_system()?;
    let to = System::Geographic {
        datum: Datum::Wgs84,
    };
    let [x1, y1, _, x2, y2, _] = *bounds;
    let mut out = [[0.0; 2]; 4];
    for (i, (x, y)) in [(x1, y1), (x2, y1), (x2, y2), (x1, y2)]
        .into_iter()
        .enumerate()
    {
        let p = transform(&from, &to, Vec2::new(x, y))?.point;
        out[i] = [p.x, p.y];
    }
    Some(out)
}

/// The note of points moved onto the result's grid, and of extra bytes left out.
pub fn grid_notes(rounded: u64, dropped_extra: bool, feedback: &mut dyn Feedback) {
    if rounded > 0 {
        feedback.warn(format!(
            "{} noktanın koordinatı sonucun ölçeğine yuvarlandı (kaynakların ölçek ve ötelemeleri farklı).",
            count_words(rounded)
        ));
    }
    if dropped_extra {
        feedback.warn(
            "Dosyaların ek baytları (extra bytes) birbirinden farklı; sonuca alınmadı.".to_owned(),
        );
    }
}
