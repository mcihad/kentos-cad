//! İnterpolasyon and Yoğunluk (docs/adr/0232): seven tools making a raster
//! from points or lines, each a run of the raster core's point job
//! (`kentos_raster::from_points`). The core gathers the objects' points;
//! the grid is worked out on the job's threads and written through the
//! host's files as a tiled GeoTIFF; a raster object (Kriging's standard
//! error a second, on its own layer) goes on a new layer right below the
//! input's, in the run's one step; cross-validation is a table.

mod tools;

use std::collections::BTreeMap;

use kentos_contracts::{Entity, RasterEntity, RasterFields, RasterSample, RasterStyle, Workspace};
use kentos_geometry_core::display::fixed;
use kentos_native_application::geometry::shape;
use kentos_raster::from_points::{CrossRow, CrossSummary, GridOf, PointInput, PointJob, PointSpec};
use kentos_raster::grid::Grid;
use kentos_raster::points::Source;
use serde_json::{Value, json};

use super::pointcloud::{STOPPED, broke, count_words};
use super::queries::attr;
use super::surface::{base, files_of, system_of};
use crate::files::Beside;
use crate::types::{
    ChangeSet, EnumOption, Feedback, ParamDef, ParamKind, Resolved, RunContext, RunResult,
    ScopeKind, Values,
};

pub use tools::{idw, kernel_density, kriging, line_density, natural_neighbor, spline, tin};

pub const INTERPOLATION: &str = "interpolation";
pub const DENSITY: &str = "density";

/// The kinds whose vertices an interpolation takes.
pub const POINT_KINDS: [&str; 4] = ["point", "line", "polyline", "polygon"];
/// The kinds Çizgi yoğunluğu takes: lines, curves and areas' boundaries.
pub const LINE_KINDS: [&str; 7] = [
    "line", "polyline", "polygon", "arc", "circle", "ellipse", "spline",
];

/// The scopes offered, a layer first.
fn scopes() -> Option<Vec<ScopeKind>> {
    Some(vec![
        ScopeKind::Layer,
        ScopeKind::Selection,
        ScopeKind::Visible,
        ScopeKind::All,
    ])
}

/// The objects parameter.
pub fn input_param(label: &str, kinds: &[&str], description: &str) -> ParamDef {
    ParamDef::new(
        "input",
        label,
        ParamKind::Features {
            kinds: Some(kinds.iter().map(|k| (*k).to_owned()).collect()),
            scopes: scopes(),
            writes: false,
        },
    )
    .describe(description)
}

/// A field of the input's objects, optional.
pub fn field_param(name: &str, label: &str, description: &str) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Field {
            of: vec!["input".into()],
            allow_new: false,
            multiple: false,
        },
    )
    .optional()
    .describe(description)
}

/// Hücre boyu, Kapsam and Izgara rasteri.
pub fn grid_params() -> [ParamDef; 3] {
    [
        super::surface::number("cellSize", "Hücre boyu", 0.0, 0.0, 1e6, "m").describe(
            "0: kendiliğinden, kutunun kısa kenarının 250'de biri yuvarlanarak (1, 2, 2,5, 5 × 10ᵏ); ızgara bu boyun katlarına oturur.",
        ),
        super::surface::choice(
            "extent",
            "Kapsam",
            &[("points", "Girdinin kutusu"), ("raster", "Rasterin ızgarası")],
        )
        .describe("Rasterin ızgarası: sonuç seçilen rasterle hücre hücre üst üste gelir."),
        ParamDef::new(
            "grid",
            "Izgara rasteri",
            ParamKind::Features {
                kinds: Some(vec!["raster".to_owned()]),
                scopes: Some(vec![ScopeKind::Selection, ScopeKind::Layer]),
                writes: false,
            },
        )
        .optional()
        .describe("Izgarası (yeri, hücre boyu, boyu) alınan raster.")
        .shown_when(|v: &Values| v.get("extent").and_then(Value::as_str) == Some("raster")),
    ]
}

/// Çapraz doğrulama.
pub fn cross_param() -> ParamDef {
    ParamDef::new("cross", "Çapraz doğrulama", ParamKind::Boolean)
        .default_value(json!(false))
        .describe("Her nokta dışarıda bırakılıp öbürlerinden tahmin edilir; tablo ve karesel ortalama hata.")
}

/// The result's layer: a new one goes right below the input's (the points over their surface).
pub fn result_layer(name: &str, param: &str) -> ParamDef {
    let mut p = crate::builtin::pointcloud::result_layer_param(name, RASTER_LAYER);
    p.name = param.to_owned();
    if let ParamKind::Layer { below, .. } = &mut p.kind {
        *below = Some("input".to_owned());
    }
    p
}

/// A choice among options.
pub fn options(name: &str, label: &str, opts: &[(&str, &str)]) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Choice {
            options: opts.iter().map(|(v, l)| EnumOption::new(v, l)).collect(),
        },
    )
    .default_value(json!(opts[0].0))
}

/// The layers' colour (a raster's is not drawn).
pub const RASTER_LAYER: &str = "#7A6B5B";

/// The grid of the raster chosen for Kapsam, or why not.
pub(crate) fn grid_of(r: &Resolved<'_>) -> Result<Option<GridOf>, String> {
    if r.text("extent") != "raster" {
        return Ok(None);
    }
    let rasters: Vec<&RasterEntity> = r
        .features("grid")
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Raster(x) => Some(x),
            _ => None,
        })
        .collect();
    match rasters.as_slice() {
        [one] => Ok(Some(GridOf {
            affine: one.raster.affine,
            width: one.raster.width,
            height: one.raster.height,
        })),
        [] => Err("Izgara rasterini seçin: Kapsam rasterin ızgarası.".into()),
        many => Err(format!(
            "{} raster seçili; ızgara için tek raster seçin.",
            many.len()
        )),
    }
}

/// What a run takes: valued points, weighted points or lines.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Points,
    Weighted,
    Lines,
}

/// A number as the summaries write it.
fn num(v: f64) -> String {
    fixed(v, 3)
}

/// The raster object of the result: the grid, the file, a band's look.
fn raster_object(
    grid: &Grid,
    bands: u32,
    path: &str,
    srid: u32,
    style: RasterStyle,
    layer: &str,
) -> Entity {
    Entity::Raster(RasterEntity {
        base: base(layer, BTreeMap::new(), None),
        raster: RasterFields {
            affine: grid.affine,
            width: grid.width,
            height: grid.height,
            bands,
            sample: RasterSample::F32,
            asset: None,
            file: Some(path.to_owned()),
            url: None,
            srid,
            style,
            opacity: None,
        },
    })
}

/// The cross-validation's table (docs/adr/0232 §12).
fn cross_table(rows: &[CrossRow], entities: &[&Entity], cad: bool, kriging: bool) -> Value {
    let mut columns = vec!["Sıra", "Ad"];
    columns.extend(if cad { ["X", "Y"] } else { ["Y", "X"] });
    columns.extend(["Ölçülen", "Tahmin", "Fark"]);
    if kriging {
        columns.extend(["Standart hata", "Standart fark"]);
    }
    let rows: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            let name = entities
                .get(r.object as usize)
                .and_then(|e| e.base().label.clone())
                .unwrap_or_default();
            let mut row = vec![
                (r.point + 1).to_string(),
                name,
                num(r.x),
                num(r.y),
                num(r.measured),
            ];
            match r.predicted {
                Some(p) => {
                    row.push(num(p));
                    row.push(num(p - r.measured));
                }
                None => row.extend([String::new(), String::new()]),
            }
            if kriging {
                match (r.predicted, r.error) {
                    (Some(p), Some(e)) => {
                        row.push(num(e));
                        row.push(if e > 0.0 {
                            num((p - r.measured) / e)
                        } else {
                            String::new()
                        });
                    }
                    _ => row.extend([String::new(), String::new()]),
                }
            }
            row
        })
        .collect();
    json!({ "columns": columns, "rows": rows })
}

/// Runs a point job: the result file (in the drawing's folder, named after
/// the input's layer, or where asked), its object or objects, the summary.
pub fn run_points(
    r: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
    tool: Value,
    (suffix, label): (&str, &str),
    from: Input,
) -> RunResult {
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let go = || -> Result<RunResult, String> {
        let entities = &r.features("input").entities;
        let Some(first) = entities.first() else {
            return Err("Girdi boş: nesne seçin.".into());
        };
        let field = match from {
            Input::Points => r.text("field"),
            Input::Weighted | Input::Lines => r.text("weightField"),
        }
        .trim();
        let texts: Option<Vec<Option<String>>> = (!field.is_empty()).then(|| {
            entities
                .iter()
                .map(|e| attr(e, field).map(str::to_owned))
                .collect()
        });
        let input = match from {
            Input::Lines => PointInput::Lines {
                shapes: entities.iter().map(|e| shape(e)).collect(),
                weights: texts,
            },
            Input::Points | Input::Weighted => PointInput::Sources {
                sources: entities.iter().map(|e| Source::of(e)).collect(),
                values: texts,
            },
        };
        let settings = ctx.doc.settings();
        let srid = settings.srid;
        let kind = tool
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let spec = PointSpec {
            tool: serde_json::from_value(tool)
                .map_err(|e| format!("Çözümlemenin ayarları okunamadı: {e}"))?,
            cell: r.number("cellSize").unwrap_or(0.0),
            grid: grid_of(r)?,
            epsg: (srid > 0).then_some(srid),
            system: system_of(ctx),
            cross: from == Input::Points && r.flag("cross"),
        };
        let layer_name = ctx.layer_name(&first.base().layer_id);
        let path = files.output_path(
            r.text("output"),
            Some(Beside::named(layer_name, "yuzey")),
            suffix,
            ".tif",
        )?;
        let (mut job, header) = PointJob::new(input, &spec, kentos_raster::par::threads())?;
        let mut sink = files.create(&path)?;
        sink.write(&header)?;
        while !job.done() {
            if feedback.canceled() {
                return Err(STOPPED.to_owned());
            }
            let bytes = job.step()?;
            sink.write(&bytes)?;
            feedback.progress(0.97 * job.share(), label);
        }
        let grid = job.grid();
        let bands = job.bands();
        let styles = (job.style(1), job.style(2));
        let done = job.finish()?;
        sink.write(&done.tail)?;
        sink.patch(0, &done.header)?;
        sink.finish()?;
        feedback.progress(1.0, label);

        let notes = &done.notes;
        if notes.merged > 0 {
            feedback.warn(format!(
                "{} köşe aynı yerdeki noktayla birleşti (değerlerinin ortalaması alındı).",
                count_words(notes.merged as u64)
            ));
        }
        if notes.unread > 0 {
            feedback.warn(format!(
                "{} nesnenin {} sayı olarak okunamadığı için alınmadı.",
                count_words(notes.unread as u64),
                if from == Input::Points {
                    "değeri"
                } else {
                    "ağırlığı"
                }
            ));
        }
        if notes.no_elevation > 0 {
            feedback.warn(format!(
                "{} köşenin kotu olmadığı için alınmadı.",
                count_words(notes.no_elevation as u64)
            ));
        }
        if notes.empty > 0 {
            feedback.warn(format!(
                "{} hücrede değer yok (noktaların kapsamı dışında ya da yeterli komşu yok).",
                count_words(notes.empty)
            ));
        }
        let mut add = Vec::new();
        if r.flag("add") {
            add.push(raster_object(
                &grid,
                bands,
                &path,
                srid,
                styles.0,
                &r.layer("layer").id,
            ));
            if bands == 2 {
                add.push(raster_object(
                    &grid,
                    bands,
                    &path,
                    srid,
                    styles.1,
                    &r.layer("errorLayer").id,
                ));
            }
        }
        let cells = format!(
            "{} × {} hücre",
            count_words(u64::from(grid.width)),
            count_words(u64::from(grid.height))
        );
        let mut summary = match from {
            Input::Points => format!(
                "{} noktadan {cells}lik raster; “{path}” yazıldı.",
                count_words(notes.taken as u64)
            ),
            Input::Weighted => format!(
                "{} noktanın yoğunluğu, yarıçap {} m; {cells}; “{path}” yazıldı.",
                count_words(notes.taken as u64),
                num(notes.radius.unwrap_or(f64::NAN))
            ),
            Input::Lines => format!(
                "{} çizgi parçasının yoğunluğu, yarıçap {} m; {cells}; “{path}” yazıldı.",
                count_words(notes.taken as u64),
                num(notes.radius.unwrap_or(f64::NAN))
            ),
        };
        if let Some(g) = &notes.variogram {
            summary.push_str(&format!(
                " Variogram: {}, külçe {}, kısmi eşik {}, erim {} m.",
                g.model.name(),
                num(g.nugget),
                num(g.sill),
                num(g.range)
            ));
        }
        let mut result = RunResult {
            changes: (!add.is_empty()).then(|| ChangeSet {
                add,
                ..ChangeSet::default()
            }),
            ..RunResult::default()
        };
        if spec.cross {
            let s = CrossSummary::of(&done.rows);
            if s.missing > 0 {
                feedback.warn(format!(
                    "{} noktanın çapraz doğrulaması yok (kabuğun üstünde ya da yeterli komşusu yok).",
                    count_words(s.missing as u64)
                ));
            }
            if s.count > 0 {
                summary.push_str(&format!(
                    " Çapraz doğrulama: {} noktada ortalama fark {}, karesel ortalama hata {}, ortalama mutlak fark {}.",
                    count_words(s.count as u64),
                    num(s.mean),
                    num(s.rmse),
                    num(s.mae)
                ));
                if let (Some(m), Some(q)) = (s.std_mean, s.std_rmse) {
                    summary.push_str(&format!(
                        " Standart farkların ortalaması {}, karesel ortalaması {}.",
                        num(m),
                        num(q)
                    ));
                }
            }
            let cad = settings.project_type() == Some(Workspace::Cad);
            result.outputs.insert(
                "table".to_owned(),
                cross_table(&done.rows, entities, cad, kind == "kriging"),
            );
        }
        result.summary = Some(summary);
        result.outputs.insert("file".to_owned(), json!(path));
        Ok(result)
    };
    go().unwrap_or_else(broke)
}
