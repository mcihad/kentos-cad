//! Explicit xref reload of CAD files; no nested path is automatically opened.
use kentos_commands::{DocumentSnapshotV2, DrawingFont, DrawingUnit, DxfReadOptions, Workspace};
use kentos_domain::Document;
use std::path::Path;

pub fn drawing(bytes: &[u8], path: &Path) -> Result<DocumentSnapshotV2, String> {
    let opts = DxfReadOptions {
        unit: Some(DrawingUnit::M),
        ..Default::default()
    };
    let mut read = if bytes.starts_with(b"AC1") {
        kentos_dwg::read(bytes, &opts).map_err(|e| e.message)?
    } else {
        kentos_formats::dxf::read(bytes, &opts)?
    };
    for block in &mut read.blocks {
        if let Some(x) = &mut block.xref
            && !Path::new(&x.path).is_absolute()
            && !x.path.contains(':')
        {
            x.path = path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(x.path.replace('\\', "/"))
                .to_string_lossy()
                .into_owned();
        }
    }
    let snapshot =
        kentos_project::new_project::new_project(&kentos_project::new_project::NewProject {
            name: path.file_stem().map_or_else(
                || "CAD başvurusu".into(),
                |s| s.to_string_lossy().into_owned(),
            ),
            srid: 0,
            plot_scale: 1000.,
            workspace: Workspace::Cad,
            drawing_font: DrawingFont::Barlow,
            province: None,
            drawing_unit: Some(DrawingUnit::M),
            annotation: None,
            cad_work: None,
        })?;
    let mut doc = Document::from_snapshot(snapshot)?;
    let input: kentos_commands::ImportDxf =
        serde_json::from_value(serde_json::json!({"path":path})).map_err(|e| e.to_string())?;
    let plan = crate::file_import::import_plan_format(
        &doc,
        &input,
        &read.layers,
        &path.file_name().unwrap_or_default().to_string_lossy(),
        crate::file_import::CadFormat::Dxf,
    )
    .map_err(|e| e.message)?;
    let styles = crate::import_apply::Styles {
        cad_source:None,
        layouts: Vec::new(),
        text: read.text_styles,
        dimension: read.dimension_styles,
        line_types: read.line_types,
        line_type_scale: read.line_type_scale,
    };
    crate::import_apply::apply_styled_import(&mut doc, read.entities, read.blocks, styles, &plan)?;
    let mut result = doc.to_snapshot_v2();
    // Handles are scoped to one xref definition. They stay stable on reload/repath;
    // the enclosing block identity distinguishes two drawings using equal handles.
    for (i, stored) in result.uids.iter_mut().enumerate() {
        let source = read
            .source_handles
            .get(i)
            .and_then(|h| h.as_deref())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("index:{i}"));
        let hash =
            kentos_commands::encoding::sha256_hex(format!("kentos-cad-xref:{source}").as_bytes());
        let mut id = [0u8; 16];
        for (j, byte) in id.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hash[j * 2..j * 2 + 2], 16).map_err(|e| e.to_string())?;
        }
        id[6] = (id[6] & 0x0F) | 0x50;
        id[8] = (id[8] & 0x3F) | 0x80;
        *stored = kentos_commands::EntityId(id);
    }
    Ok(result)
}
