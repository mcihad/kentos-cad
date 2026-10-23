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

/// Byte reading and cancellation belong to the host. This stage also runs off the UI thread.
pub fn read(
    path: &Path,
    bytes: &[u8],
    format: Format,
    watch: &mut dyn Watch,
) -> Result<Opened, String> {
    if bytes.len() > kentos_formats::dxf::preflight::MAX_BYTES {
        return Err("CAD dosyası 256 MiB sınırını aşıyor; dosyayı bölün.".into());
    }
    if !watch.step(0, bytes.len() as u64) {
        return Err(kentos_formats::watch::STOPPED.into());
    }
    // A metric CAD template is the explicit fallback only when the source declares no known unit.
    // A declared source unit wins. No current project's units/scale/preferences participate.
    let options = DxfReadOptions {
        unit: Some(DrawingUnit::Mm),
        scale: Some(1.0),
        closed_polylines: kentos_commands::ClosedPolylines::Line,
        ..Default::default()
    };
    let mut result = match format {
        Format::Dxf => kentos_formats::dxf::read_watched(bytes, &options, watch)?,
        Format::Dwg => {
            kentos_dwg::read_watched(bytes, &options, watch).map_err(|e| e.to_string())?
        }
        Format::Ncz => kentos_ncz::read(bytes, &kentos_commands::NczReadOptions::default(), watch)?,
    };
    if let Some(e) = kentos_formats::dxf::polyline_policy::refusal(
        result.report.notes.iter().chain(&result.report.skipped),
    ) {
        return Err(format!(
            "{} Kayıp seçeneklerini incelemek için İçe aktar kullanın.",
            e.message
        ));
    }
    if let Some((place, kind)) = import_apply::unusable(&result.entities) {
        return Err(import_apply::unusable_text(place, kind));
    }
    if let Some(e) = import_apply::unusable_block(&result.blocks) {
        return Err(e);
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
    let name = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let mut snapshot =
        kentos_project::new_project::new_project(&kentos_project::new_project::NewProject {
            name,
            srid,
            plot_scale: if format == Format::Ncz { 1000.0 } else { 1.0 },
            workspace: Workspace::Cad,
            drawing_font: DrawingFont::Barlow,
            province: None,
            drawing_unit: Some(unit),
            annotation: None,
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
            "../../../../fixtures/formats/v1/cad-open.json"
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
            let close =
                |a: f64, b: f64| assert!((a - b).abs() < 1e-9, "{}: {a} != {b}", case["name"]);
            assert_eq!(
                serde_json::to_value(doc.settings().unit()).expect("unit"),
                case["unit"],
                "{}",
                case["name"]
            );
            assert_eq!(doc.settings().plot_scale, 1.0);
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

    #[test]
    fn cancellation_and_corruption_never_create_a_drawing() {
        let bytes = include_bytes!("../../../../fixtures/formats/v1/dxf-units/mm.dxf");
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
