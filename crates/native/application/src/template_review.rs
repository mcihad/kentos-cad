//! Safe template application keeps existing definitions, including text fonts. The old
//! standards fix may refont existing text; a package installation only adds missing styles.
use crate::checks::{Stop, error};
use crate::codes;
use kentos_commands::{
    DimensionStyleDef, DrawingStandard, LayerNode, LayerNodeType, TemplatePreserved,
    TemplateReview, TemplateSettingsMode, TemplatesApply, TextStyleDef, WorkTemplate,
    builtin_wall_styles, fold_name,
};
use kentos_domain::Document;
use serde_json::Value;

fn leaves<'a>(nodes: &'a [LayerNode], out: &mut Vec<&'a LayerNode>) {
    for node in nodes {
        if node.kind == LayerNodeType::Layer {
            out.push(node);
        }
        leaves(&node.children, out);
    }
}
fn kept(out: &mut Vec<TemplatePreserved>, kind: &str, id: &str, name: &str, matched_by: &str) {
    out.push(TemplatePreserved {
        kind: kind.into(),
        id: id.into(),
        name: name.into(),
        matched_by: matched_by.into(),
    });
}
pub(crate) fn same_name(a: &str, b: &str) -> bool {
    fold_name(a.trim()) == fold_name(b.trim())
}
pub(crate) fn match_key(id: bool, name: bool) -> &'static str {
    match (id, name) {
        (true, true) => "idAndName",
        (true, false) => "id",
        _ => "name",
    }
}

/// Read-only review in package order. An unreadable existing book is refused before any write.
pub(crate) fn review(
    doc: &Document,
    input: &TemplatesApply,
    t: &WorkTemplate,
    standard: &DrawingStandard,
) -> Result<Option<TemplateReview>, Stop> {
    let Some(mode) = input.settings_mode else {
        return Ok(None);
    };
    let sheets = crate::sheets_edit::sheets_in(doc).ok_or_else(|| {
        Stop::failed(error(
            codes::BOOK_UNREADABLE,
            "Projenin pafta kitabı okunamadı; şablon hiçbir şey yazmadan durdu.".into(),
            Some("sheets".into()),
        ))
    })?;
    let mut preserved = Vec::new();
    let mut layers = Vec::new();
    leaves(doc.layers().nodes(), &mut layers);
    for wanted in &standard.layers {
        if let Some(now) = layers.iter().find(|n| same_name(&n.name, &wanted.name)) {
            kept(&mut preserved, "layer", &now.id, &now.name, "name");
        }
    }
    let settings = doc.settings();
    for wanted in &standard.text_styles {
        if let Some(now) = settings
            .text_styles
            .iter()
            .find(|n| same_name(&n.name, &wanted.name))
        {
            kept(&mut preserved, "textStyle", &now.id, &now.name, "name");
        }
    }
    for wanted in &standard.dimension_styles {
        if let Some(now) = settings
            .dimension_styles
            .iter()
            .find(|n| same_name(&n.name, &wanted.name))
        {
            kept(&mut preserved, "dimensionStyle", &now.id, &now.name, "name");
        }
    }
    let raw = crate::blocks_library::BUILTIN_LIBRARIES
        .iter()
        .find(|(name, _)| *name == t.blocks)
        .ok_or_else(|| {
            Stop::failed(error(
                codes::INVALID_LIBRARY,
                "Şablonun blok kitaplığı bulunamadı.".into(),
                Some("name".into()),
            ))
        })?;
    let library: Value = serde_json::from_str(raw.1).map_err(|e| {
        Stop::failed(error(
            codes::INVALID_LIBRARY,
            e.to_string(),
            Some("name".into()),
        ))
    })?;
    if let Some(blocks) = library["blocks"].as_array() {
        for wanted in blocks {
            if let Some(now) = wanted["name"].as_str().and_then(|n| doc.block_named(n)) {
                kept(
                    &mut preserved,
                    "block",
                    &now.id.to_text(),
                    &now.name,
                    "name",
                );
            }
        }
    }
    for wanted in builtin_wall_styles() {
        for now in settings
            .wall_styles
            .iter()
            .filter(|n| n.id == wanted.id || same_name(&n.name, &wanted.name))
        {
            kept(
                &mut preserved,
                "wallStyle",
                &now.id,
                &now.name,
                match_key(now.id == wanted.id, same_name(&now.name, &wanted.name)),
            );
        }
    }
    if let Some(arch) = &settings.arch {
        for wanted in &t.levels {
            for now in arch
                .levels
                .iter()
                .filter(|n| n.id == wanted.id || same_name(&n.name, &wanted.name))
            {
                kept(
                    &mut preserved,
                    "level",
                    &now.id,
                    &now.name,
                    match_key(now.id == wanted.id, same_name(&now.name, &wanted.name)),
                );
            }
        }
    }
    for wanted in &t.sheets {
        if let Some(now) = sheets.iter().find(|n| n.id == wanted.id) {
            kept(&mut preserved, "sheet", &now.id, &now.name, "id");
        }
    }
    Ok(Some(TemplateReview {
        mode,
        previous_unit: settings.unit(),
        previous_plot_scale: settings.plot_scale,
        plot_scale: if mode == TemplateSettingsMode::Keep {
            settings.plot_scale
        } else {
            t.plot_scale
        },
        previous_space_method: settings.arch.as_ref().and_then(|a| a.space_method),
        space_method: settings
            .arch
            .as_ref()
            .and_then(|a| a.space_method)
            .unwrap_or(t.space_method),
        preserved,
    }))
}

pub(crate) fn add_styles(
    doc: &mut Document,
    standard: &DrawingStandard,
    fresh: &dyn Fn(usize) -> String,
) -> Vec<String> {
    let mut settings = doc.settings().clone();
    let mut added = Vec::new();
    for wanted in &standard.text_styles {
        if settings
            .text_styles
            .iter()
            .any(|now| same_name(&now.name, &wanted.name))
        {
            continue;
        }
        settings.text_styles.push(TextStyleDef {
            id: fresh(added.len()),
            name: wanted.name.trim().into(),
            font: wanted.font,
            bold: false,
            italic: false,
            oblique: None,
            height: wanted.height,
            width_factor: None,
            font_file: None,
        });
        added.push(wanted.name.trim().into());
    }
    for wanted in &standard.dimension_styles {
        if settings
            .dimension_styles
            .iter()
            .any(|now| same_name(&now.name, &wanted.name))
        {
            continue;
        }
        settings.dimension_styles.push(DimensionStyleDef {
            id: fresh(added.len()),
            name: wanted.name.trim().into(),
            height: wanted.height,
            ..Default::default()
        });
        added.push(wanted.name.trim().into());
    }
    doc.set_settings(settings);
    added
}
