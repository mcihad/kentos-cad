//! Open a source CAD drawing, without an existing project and without writing its source.
//! Model lengths are normalised once to metres; paper geometry stays in paper units.
use crate::import_apply::{self, ImportPlan, LayerTarget, Styles};
use kentos_commands::{
    DrawingFont, DrawingUnit, DxfReadOptions, ImportReport, LayerStyle, Workspace,
};
use kentos_domain::Document;
use kentos_formats::watch::Watch;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Dxf,
    Dwg,
    Ncz,
}

impl Format {
    pub fn of(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "dxf" => Some(Self::Dxf),
            "dwg" => Some(Self::Dwg),
            "ncz" => Some(Self::Ncz),
            _ => None,
        }
    }
}

pub struct Opened {
    pub document: Document,
    pub report: ImportReport,
    pub unit: Option<kentos_commands::UnitDecision>,
}

/// Why a CAD file did not open: a code of the catalog's table and the words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    pub code: &'static str,
    pub message: String,
}

fn refused(code: &'static str, message: impl Into<String>) -> Refused {
    Refused {
        code,
        message: message.into(),
    }
}

/// What the drawing's own building refuses: the file's content does not fit.
impl From<String> for Refused {
    fn from(message: String) -> Self {
        refused("import_unreadable", message)
    }
}

/// The largest CAD file Aç reads (docs/adr/9211): the DWG bridge's DXF limit. Parsing is not what limits a big file:
/// the city plan's 657 MB DXF parses in 9 s (5.2 GB whole, 2.1 GB to the reader's million objects); its 3.2 M objects
/// do, and the reader opens the first million and says how many were left out. İçe aktar and the web keep the
/// formats' own 256 MiB.
pub const MAX_OPEN_BYTES: usize = 1024 * 1024 * 1024;

/// Byte reading and cancellation belong to the host. This stage also runs off the UI thread.
pub fn read(
    path: &Path,
    bytes: &[u8],
    format: Format,
    watch: &mut dyn Watch,
) -> Result<Opened, String> {
    read_in(path, bytes, format, None, watch).map_err(|r| r.message)
}

/// [`read`] with the unit the file's numbers are in (`file_unit`, whatever the file names; docs/adr/0726 §2) and
/// the refusal's code: `cad_file_too_large`, `cancelled`, `unsupported_cad_version` (a DWG's signature),
/// `import_unreadable`.
pub fn read_in(
    path: &Path,
    bytes: &[u8],
    format: Format,
    file_unit: Option<kentos_commands::FileUnit>,
    watch: &mut dyn Watch,
) -> Result<Opened, Refused> {
    if bytes.len() > MAX_OPEN_BYTES {
        return Err(refused(
            "cad_file_too_large",
            format!(
                "CAD dosyası {} MiB sınırını aşıyor; dosyayı bölün.",
                MAX_OPEN_BYTES / (1024 * 1024)
            ),
        ));
    }
    if !watch.step(0, bytes.len() as u64) {
        return Err(refused("cancelled", kentos_formats::watch::STOPPED));
    }
    // A metric CAD template is the explicit fallback only when the source declares no known unit.
    // A declared source unit wins. No current project's units/scale/preferences participate.
    let mut options = DxfReadOptions {
        unit: Some(DrawingUnit::Mm),
        file_unit,
        scale: Some(1.0),
        closed_polylines: kentos_commands::ClosedPolylines::Line,
        // Aç shows the drawing: a polyline the reader cannot carry exactly (a width or a bulge under a block's unequal
        // scale) comes approximated and said in the open's notes. Refusing it kept a real 8 MB drawing of 197 858
        // objects closed (12 November 2026); İçe aktar keeps its own choice (`allowPolylineLoss`).
        allow_polyline_loss: true,
        ..Default::default()
    };
    // A DWG is decoded once for both reads below (its unit, then at its scale): the second decoding of a 32 MB DWG
    // cost 9 of its 24 s (12 November 2026).
    let mut bridge: Option<kentos_dwg::Bridge> = None;
    let mut read_at = |options: &DxfReadOptions, watch: &mut dyn Watch| match format {
        Format::Dxf => {
            kentos_formats::dxf::read_checked_watched_within(bytes, options, watch, MAX_OPEN_BYTES)
                .map_err(|e| refused("import_unreadable", e.message))
        }
        Format::Dwg => {
            if bridge.is_none() {
                bridge =
                    Some(kentos_dwg::bridge(bytes, watch).map_err(|e| refused(e.code, e.message))?);
            }
            let Some(made) = bridge.as_ref() else {
                return Err(refused(
                    "import_unreadable",
                    "DWG aktarma verisi üretilemedi.",
                ));
            };
            kentos_dwg::read_bridge(made, options, watch).map_err(|e| refused(e.code, e.message))
        }
        Format::Ncz => kentos_ncz::read(bytes, &kentos_commands::NczReadOptions::default(), watch)
            .map_err(|e| refused("import_unreadable", e)),
    };
    let mut result = read_at(&options, watch)?;
    if let Some((place, kind)) = import_apply::unusable(&result.entities) {
        return Err(refused(
            "import_unreadable",
            import_apply::unusable_text(place, kind),
        ));
    }
    if let Some(e) = import_apply::unusable_block(&result.blocks) {
        return Err(refused("import_unreadable", e));
    }
    // A survey grid's metres, whatever the file declares or not (plan/render.md RM-23, docs/adr/9210): read in the
    // millimetres a template declares (or the fallback's), they put a town into a few metres. The Suşehri DXF
    // declares millimetres and opened 12 m wide; so did four more of the samples. Read again in metres, said, and the
    // unit asked (`UnitSource::Grid`).
    let grid = if format != Format::Ncz && file_unit.is_none() {
        result
            .unit
            .as_ref()
            .filter(|u| u.used != 6)
            .and_then(|u| survey_grid(result.bounds, u.factor))
    } else {
        None
    };
    // Letters under a millimetre high in the unit read, a plan's as metres (docs/adr/9213): a template's millimetres
    // on a drawing made in metres. The house plan DXF of the samples declares millimetres and opened as a sheet 5 cm
    // wide, its letters 0.08 mm high; the view's deepest zoom (0.1 mm a pixel) stopped at the whole sheet. Read again
    // in metres, said, and the unit asked (`UnitSource::TextHeight`).
    let letters = if grid.is_none() && format != Format::Ncz && file_unit.is_none() {
        result
            .unit
            .as_ref()
            .filter(|u| u.used != 6)
            .and_then(|u| small_letters(&result.entities, u.factor).map(|h| (h, u.factor)))
    } else {
        None
    };
    if grid.is_some() || letters.is_some() {
        // Every read below too, the one at the drawing's scale among them.
        options.file_unit = Some(kentos_commands::FileUnit::M);
        result = read_at(&options, watch)?;
    }
    let unit = match result.unit.as_ref().map(|u| u.used) {
        Some(1) => DrawingUnit::In,
        Some(2) => DrawingUnit::Ft,
        Some(4) => DrawingUnit::Mm,
        Some(5) => DrawingUnit::Cm,
        _ => DrawingUnit::M,
    };
    let srid = result
        .declared_crs
        .as_ref()
        .and_then(|c| c.srid)
        .filter(|s| kentos_project::crs::system(*s).is_some())
        .unwrap_or(0);
    // The drawing scale its own texts and dimensions say (docs/adr/0626 §5), asked of nobody; an NCZ map's
    // 1:1000. The styles' paper sizes are read at the project's scale (docs/adr/0183 §7): read again at it.
    let plot_scale = if format == Format::Ncz {
        1000.0
    } else {
        crate::opened_scale::opened_scale(&result.entities, unit, srid != 0)
    };
    // The drawing is made in the source's unit; a line type's drawing-unit lengths are written in the reader's unit
    // (docs/adr/1001 §4): read again in the drawing's own when the file declares another than the fallback, else
    // a metre file's 1 m dash became 1000 m and its lines drew solid.
    if format != Format::Ncz && (plot_scale != 1.0 || unit != DrawingUnit::Mm) {
        let options = DxfReadOptions {
            scale: Some(plot_scale),
            unit: Some(unit),
            ..options.clone()
        };
        result = read_at(&options, watch)?;
    }
    // The reader's `unit` is only our fallback here, not a receiving project.
    // Its import report must not claim e.g. an inch drawing was converted to
    // a millimetre project: opening creates a drawing in the source's own unit.
    if let Some(decision) = result.unit.as_ref().filter(|u| !u.assumed) {
        for note in result.report.notes.iter_mut().filter(|n| n.what == "Birim") {
            note.reason = format!(
                "Kaynak {} biriminde; model fiziksel boyutunda açıldı.",
                decision.name
            );
        }
    }
    if let (Some(_), Some(u)) = (grid, result.unit.as_mut()) {
        u.source = kentos_commands::UnitSource::Grid;
    }
    if let (Some(_), Some(u)) = (letters, result.unit.as_mut()) {
        u.source = kentos_commands::UnitSource::TextHeight;
    }
    if let (Some((height, factor)), Some(decision)) = (letters, result.unit.as_ref()) {
        let declared = decision
            .declared
            .and_then(kentos_commands::units::insunits)
            .map(|(_, name)| name);
        let (read, metres) = (decimal(height * 1000.0), decimal(height / factor * 100.0));
        let reason = match declared {
            Some(name) => format!(
                "dosya {name} bildiriyor ama yazıları bu birimde {read} mm yüksekliğinde (ortanca), okunamayacak kadar \
                 küçük; metre okununca {metres} cm, bir planın yazısı. Değerler metre okundu. Birimi biliyorsanız seçin."
            ),
            None => format!(
                "dosya birim bildirmiyor; yazıları milimetre okununca {read} mm yüksekliğinde (ortanca), metre okununca \
                 {metres} cm, bir planın yazısı: değerler metre okundu. Birimi biliyorsanız seçin."
            ),
        };
        say_unit(&mut result.report, reason);
    }
    if let (Some([east, north]), Some(decision)) = (grid, result.unit.as_ref()) {
        let declared = decision
            .declared
            .and_then(kentos_commands::units::insunits)
            .map(|(_, name)| name);
        let reason = match declared {
            Some(name) => format!(
                "dosya {name} bildiriyor ama koordinatları ülkenin ölçme ızgarasında (Y {east:.0}, X {north:.0} \
                 yakınında); değerler metre okundu. Birimi biliyorsanız seçin."
            ),
            None => format!(
                "dosya birim bildirmiyor; koordinatları ülkenin ölçme ızgarasında (Y {east:.0}, X {north:.0} \
                 yakınında) olduğu için değerler metre okundu. Birimi biliyorsanız seçin."
            ),
        };
        say_unit(&mut result.report, reason);
    }
    let name = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let mut snapshot =
        kentos_project::new_project::new_project(&kentos_project::new_project::NewProject {
            name,
            srid,
            plot_scale,
            workspace: Workspace::Cad,
            drawing_font: DrawingFont::Barlow,
            province: None,
            drawing_unit: Some(unit),
            annotation: None,
            cad_work: None,
            heights: None,
        })?;
    // Keep the CAD zero layer, then reproduce the source layer tree without an import group.
    snapshot.layers.truncate(1);
    snapshot.layers[0].id = "0".into();
    snapshot.layers[0].name = "0".into();
    snapshot.active_layer = "0".into();
    snapshot.home_view = result.view.or(result.bounds);
    if let Some(zero) = result.layers.iter().find(|l| l.name == "0") {
        snapshot.layers[0].style = source_style(zero);
        snapshot.layers[0].visible = zero.visible;
        snapshot.layers[0].locked = false;
    }
    let mut doc = Document::from_snapshot(snapshot)?;
    crate::file_import::prepare_cad(&doc, path, &mut result).map_err(|s| s.into_error().message)?;
    let plan = ImportPlan {
        label: "CAD dosyası aç".into(),
        group: None,
        source_srid: (srid != 0).then_some(srid),
        layers: result
            .layers
            .iter()
            .map(|l| {
                (
                    l.name.clone(),
                    if l.name == "0" {
                        LayerTarget::Existing("0".into())
                    } else {
                        LayerTarget::New {
                            name: l.name.clone(),
                            style: Box::new(source_style(l)),
                            visible: l.visible,
                            locked: l.locked,
                            fields: l.fields.clone(),
                            plot: l.plot,
                            frozen: l.frozen,
                        }
                    },
                )
            })
            .collect(),
    };
    let result_zero = result
        .layers
        .iter()
        .find(|l| l.name == "0")
        .map(|l| (l.locked, l.plot, l.frozen));
    let report = result.report;
    let unit = result.unit;
    let styles = Styles {
        layouts: result.layouts,
        text: result.text_styles,
        dimension: result.dimension_styles,
        line_types: result.line_types,
        line_type_scale: result.line_type_scale,
        table: result.table_styles,
        cad_source: Some(crate::cad_layouts::Source {
            file: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            sha256: Some(kentos_commands::encoding::sha256_hex(bytes)),
        }),
    };
    import_apply::apply_styled_import(&mut doc, result.entities, result.blocks, styles, &plan)?;
    // Opening is the baseline, not an import the user can accidentally undo away.
    let mut snapshot = doc.to_snapshot_v2();
    if let Some(zero) = result_zero {
        snapshot.layers[0].locked = zero.0;
        snapshot.layers[0].plot = zero.1;
        snapshot.layers[0].frozen = zero.2;
    }
    let mut document = Document::from_snapshot_v2(snapshot)?;
    document.mark_unsaved();
    Ok(Opened {
        document,
        report,
        unit,
    })
}

/// Where a file's numbers lie when they are a national survey grid's metres (plan/render.md RM-23): the
/// middle of `bounds` (metres as read, `factor` metres a file unit) taken back to the file's numbers is a transverse
/// Mercator easting and a northing of Turkey's latitudes (UTM and the 3° zones: 36–42° N is 3.98–4.66 M), and the
/// drawing fits in a zone. None elsewhere: a drawing near its origin, as buildings are drawn, keeps the template's
/// millimetres.
fn survey_grid(bounds: Option<kentos_commands::Bounds>, factor: f64) -> Option<[f64; 2]> {
    let b = bounds?;
    if !(factor.is_finite() && factor > 0.0) {
        return None;
    }
    let [x0, y0, x1, y1] = [b.min_x, b.min_y, b.max_x, b.max_y].map(|v| v / factor);
    let (east, north) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let fits = (x1 - x0) <= 400_000.0 && (y1 - y0) <= 400_000.0;
    ((100_000.0..=900_000.0).contains(&east)
        && (3_850_000.0..=4_750_000.0).contains(&north)
        && fits)
        .then_some([east, north])
}

/// The report's “Birim” note says `reason`; the reader says a unit read in the drawing's own unit quietly, so it is
/// added when there is none.
fn say_unit(report: &mut kentos_commands::ImportReport, reason: String) {
    let mut said = false;
    for note in report.notes.iter_mut().filter(|n| n.what == "Birim") {
        note.reason.clone_from(&reason);
        said = true;
    }
    if !said {
        report.notes.insert(
            0,
            kentos_commands::ReportItem {
                severity: None,
                details: Vec::new(),
                what: "Birim".into(),
                count: 1,
                reason,
                lines: Vec::new(),
            },
        );
    }
}

/// A number with two decimals at most, a comma before them (a note's Turkish).
fn decimal(v: f64) -> String {
    let t = format!("{v:.2}");
    t.trim_end_matches('0')
        .trim_end_matches('.')
        .replace('.', ",")
}

/// The middle height of a drawing's texts when its letters are too small to be letters in the unit they were read
/// in and a plan's as metres (docs/adr/9213): under a millimetre high as read (metres as read; `factor` metres a file
/// unit), 2 cm to 2 m as metres. Plans are lettered 2 to 5 mm high on paper, 2 cm to 2 m in the model at 1:10 to
/// 1:500; a letter under a millimetre is printed by no scale a building's drawing is made at. None with fewer than
/// three texts in the model.
fn small_letters(entities: &[kentos_commands::Entity], factor: f64) -> Option<f64> {
    if !(factor.is_finite() && factor > 0.0) {
        return None;
    }
    let mut heights: Vec<f64> = entities
        .iter()
        .filter_map(|e| match e {
            kentos_commands::Entity::Text(t) => Some(t.height),
            _ => None,
        })
        .filter(|h| h.is_finite() && *h > 0.0)
        .collect();
    if heights.len() < 3 {
        return None;
    }
    heights.sort_by(f64::total_cmp);
    let middle = heights[heights.len() / 2];
    (middle < 0.001 && (0.02..=2.0).contains(&(middle / factor))).then_some(middle)
}

fn source_style(l: &kentos_commands::ImportLayer) -> LayerStyle {
    LayerStyle {
        color: l.color.clone(),
        line_type: l.line_type,
        line_weight: l.line_weight.unwrap_or(0.25),
        line_pattern: l.line_pattern.clone(),
        ..kentos_domain::default_style()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::Entity;
    #[test]
    fn independent_full_size_unit_and_block_cases() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../fixtures/formats/v1/cad-open.json"
        ))
        .expect("fixture");
        for case in fixture["cases"].as_array().expect("cases") {
            let opened = read(
                Path::new("room.dxf"),
                case["dxf"].as_str().expect("DXF").as_bytes(),
                Format::Dxf,
                &mut kentos_formats::watch::Quiet,
            )
            .expect("open");
            let mut doc = opened.document;
            if case["unit"] != "mm" {
                assert!(
                    !opened
                        .report
                        .notes
                        .iter()
                        .any(|n| n.reason.contains("milimetreye çevrildi")),
                    "{}: opening must not report a conversion to a different project's unit",
                    case["name"]
                );
            }
            let close =
                |a: f64, b: f64| assert!((a - b).abs() < 1e-9, "{}: {a} != {b}", case["name"]);
            assert_eq!(
                serde_json::to_value(doc.settings().unit()).expect("unit"),
                case["unit"],
                "{}",
                case["name"]
            );
            // The scale the drawing's texts say, else its unit's (docs/adr/0626 §5; cad_open_cases.py).
            assert_eq!(
                doc.settings().plot_scale,
                case["scale"].as_f64().expect("a scale"),
                "{}",
                case["name"]
            );
            assert_eq!(doc.settings().workspace, Some(Workspace::Cad));
            assert_eq!(
                opened.unit.expect("decision").assumed,
                case["assumed"].as_bool().expect("assumed")
            );
            let line = doc
                .entities()
                .find_map(|e| {
                    if let Entity::Line(l) = e {
                        Some(l)
                    } else {
                        None
                    }
                })
                .expect("line");
            close(
                line.b.x - line.a.x,
                case["length"].as_f64().expect("length"),
            );
            let insert = doc
                .entities()
                .find_map(|e| {
                    if let Entity::Insert(i) = e {
                        Some(i)
                    } else {
                        None
                    }
                })
                .expect("insert");
            close(insert.scale, case["insertScale"].as_f64().expect("scale"));
            let block = &doc.blocks()[0];
            let Entity::Line(edge) = &block.entities[0] else {
                panic!("block line")
            };
            close(
                edge.b.x - edge.a.x,
                case["blockLength"].as_f64().expect("block size"),
            );
            assert!(doc.is_dirty());
            assert!(doc.undo().is_none(), "opening must not be undoable");
        }
    }

    /// A file whose numbers lie in the survey grid opens in metres, undeclared or declaring millimetres, and its unit
    /// is asked; one near its origin keeps the template's millimetres (plan/render.md RM-23, docs/adr/9210).
    #[test]
    fn an_undeclared_file_in_the_survey_grid_opens_in_metres() {
        let dxf = |x: f64, y: f64| {
            format!(
                "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1015\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n\
                 0\nLINE\n8\n0\n10\n{x}\n20\n{y}\n11\n{}\n21\n{y}\n0\nENDSEC\n0\nEOF\n",
                x + 10.0
            )
        };
        let open = |x: f64, y: f64| {
            read(
                Path::new("kent.dxf"),
                dxf(x, y).as_bytes(),
                Format::Dxf,
                &mut kentos_formats::watch::Quiet,
            )
            .expect("opens")
        };
        let length = |o: &Opened| {
            o.document
                .entities()
                .find_map(|e| {
                    if let Entity::Line(l) = e {
                        Some(l.b.x - l.a.x)
                    } else {
                        None
                    }
                })
                .expect("a line")
        };
        let town = open(422_000.0, 4_449_000.0);
        assert_eq!(town.document.settings().unit(), DrawingUnit::M);
        assert!((length(&town) - 10.0).abs() < 1e-9, "{}", length(&town));
        let decision = town.unit.as_ref().expect("a decision");
        assert_eq!(
            (decision.source, decision.declared),
            (kentos_commands::UnitSource::Grid, None),
            "asked"
        );
        assert!(
            town.report.notes.iter().any(|n| n.what == "Birim"
                && n.reason.contains("birim bildirmiyor; koordinatları ülkenin ölçme ızgarasında (Y 422005, X 4449000 yakınında)")),
            "{:?}",
            town.report.notes
        );
        // Declared millimetres on a survey drawing (a template's): metres too, and said.
        let declared = dxf(422_000.0, 4_449_000.0).replacen(
            "$ACADVER\n1\nAC1015\n",
            "$ACADVER\n1\nAC1015\n9\n$INSUNITS\n70\n4\n",
            1,
        );
        let town = read(
            Path::new("kent.dxf"),
            declared.as_bytes(),
            Format::Dxf,
            &mut kentos_formats::watch::Quiet,
        )
        .expect("opens");
        assert_eq!(town.document.settings().unit(), DrawingUnit::M);
        assert!((length(&town) - 10.0).abs() < 1e-9, "{}", length(&town));
        let decision = town.unit.as_ref().expect("a decision");
        assert_eq!(
            (decision.source, decision.declared),
            (kentos_commands::UnitSource::Grid, Some(4))
        );
        assert!(
            town.report.notes.iter().any(
                |n| n.what == "Birim" && n.reason.starts_with("dosya milimetre bildiriyor ama")
            ),
            "{:?}",
            town.report.notes
        );
        let room = open(0.0, 0.0);
        assert_eq!(room.document.settings().unit(), DrawingUnit::Mm);
        assert!((length(&room) - 0.01).abs() < 1e-12, "{}", length(&room));
    }

    /// A building drawn in metres in a template that declares millimetres (docs/adr/9213): its letters 0.1 mm high
    /// as declared, 10 cm as metres. It opens in metres, said, its unit asked; letters a millimetre drawing's own size
    /// keep its millimetres.
    #[test]
    fn letters_under_a_millimetre_open_a_plan_in_metres() {
        let dxf = |height: f64| {
            let texts: String = (0..4)
                .map(|i| {
                    format!(
                        "0\nTEXT\n8\n0\n10\n{}\n20\n1\n40\n{height}\n1\nOda {i}\n",
                        f64::from(i) * 3.0
                    )
                })
                .collect();
            format!(
                "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1015\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n0\nSECTION\n2\n\
                 ENTITIES\n0\nLINE\n8\n0\n10\n0\n20\n0\n11\n12.5\n21\n0\n{texts}0\nENDSEC\n0\nEOF\n"
            )
        };
        let open = |height: f64| {
            read(
                Path::new("ev.dxf"),
                dxf(height).as_bytes(),
                Format::Dxf,
                &mut kentos_formats::watch::Quiet,
            )
            .expect("opens")
        };
        let length = |o: &Opened| {
            o.document
                .entities()
                .find_map(|e| {
                    if let Entity::Line(l) = e {
                        Some(l.b.x - l.a.x)
                    } else {
                        None
                    }
                })
                .expect("a line")
        };
        let plan = open(0.1);
        assert_eq!(plan.document.settings().unit(), DrawingUnit::M);
        assert!((length(&plan) - 12.5).abs() < 1e-9, "{}", length(&plan));
        let decision = plan.unit.as_ref().expect("a decision");
        assert_eq!(
            (decision.source, decision.declared),
            (kentos_commands::UnitSource::TextHeight, Some(4)),
            "asked"
        );
        assert!(
            plan.report.notes.iter().any(|n| n.what == "Birim"
                && n.reason.starts_with(
                    "dosya milimetre bildiriyor ama yazıları bu birimde 0,1 mm yüksekliğinde"
                )
                && n.reason.contains("metre okununca 10 cm")),
            "{:?}",
            plan.report.notes
        );
        // Letters 2.5 mm high: a millimetre drawing as it says.
        let part = open(2.5);
        assert_eq!(part.document.settings().unit(), DrawingUnit::Mm);
        assert!((length(&part) - 0.0125).abs() < 1e-12, "{}", length(&part));
        assert_eq!(
            part.unit.as_ref().map(|u| u.source),
            Some(kentos_commands::UnitSource::Declared)
        );
    }

    /// A polyline the reader cannot carry exactly opens approximated and said (Aç); İçe aktar's rule still refuses
    /// it unless the loss is accepted. Refused, a real 8 MB drawing did not open for one such polyline (12 November
    /// 2026): an arc with a width in a block placed with unequal scales.
    #[test]
    fn an_approximated_polyline_opens_and_is_said() {
        let dxf = "0\nSECTION\n2\nBLOCKS\n0\nBLOCK\n8\n0\n2\n*U1\n70\n1\n10\n0\n20\n0\n30\n0\n3\n*U1\n\
                   0\nLWPOLYLINE\n8\n0\n90\n2\n70\n0\n43\n0.5\n10\n0\n20\n0\n42\n0.5\n10\n10\n20\n0\n\
                   0\nENDBLK\n8\n0\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nINSERT\n8\n0\n2\n*U1\n10\n0\n20\n0\n\
                   41\n2\n42\n1\n0\nENDSEC\n0\nEOF\n";
        let strict = DxfReadOptions {
            closed_polylines: kentos_commands::ClosedPolylines::Line,
            ..Default::default()
        };
        assert_eq!(
            kentos_formats::dxf::read_checked(dxf.as_bytes(), &strict)
                .err()
                .map(|e| e.code),
            Some(kentos_formats::dxf::polyline_policy::LOSS),
            "İçe aktar's rule"
        );
        let opened = read(
            Path::new("kayip.dxf"),
            dxf.as_bytes(),
            Format::Dxf,
            &mut kentos_formats::watch::Quiet,
        )
        .expect("Aç opens it");
        let said: Vec<&str> = opened
            .report
            .notes
            .iter()
            .map(|n| n.reason.as_str())
            .collect();
        assert!(
            said.contains(
                &"eşit olmayan ölçekle eklendiği için genişliği alınmadı; çizgi ekseniyle geldi"
            ),
            "{said:?}"
        );
        assert!(
            said.contains(&"yaylı kenarları eşit olmayan ölçekle eklendiği için noktalara bölündü"),
            "{said:?}"
        );
        assert!(
            opened
                .document
                .entities()
                .any(|e| matches!(e, Entity::Polyline(_))),
            "the polyline is there"
        );
    }

    #[test]
    fn cancellation_and_corruption_never_create_a_drawing() {
        let bytes = include_bytes!("../../../../../fixtures/formats/v1/dxf-units/mm.dxf");
        let mut stopped = kentos_formats::watch::Steps(|_, _| false);
        assert!(read(Path::new("room.dxf"), bytes, Format::Dxf, &mut stopped).is_err());
        for format in [Format::Dxf, Format::Dwg, Format::Ncz] {
            assert!(
                read(
                    Path::new("broken"),
                    b"not a drawing",
                    format,
                    &mut kentos_formats::watch::Quiet
                )
                .is_err()
            );
        }
    }
}
