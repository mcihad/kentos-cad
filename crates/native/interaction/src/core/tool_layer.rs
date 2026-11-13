//! The layer an annotation tool draws on (usage review UR-003): the project's own for its dimensions, texts and
//! hatches (`settings.toolLayers`) when it names one and that layer is in the drawing, else the active layer. The
//! command that writes them says a locked layer (it refuses) and a hidden one (it warns), as for the active layer.

use kentos_commands::{AnnotationTool, EntityGeometry, LayerNodeType};
use kentos_domain::Document;

/// The layer `tool`'s objects go to.
pub fn layer_for(doc: &Document, tool: AnnotationTool) -> String {
    let layers = doc.layers();
    kentos_commands::tool_layer(&doc.settings().tool_layers, tool)
        .filter(|id| layers.get(id).is_some_and(|n| n.kind == LayerNodeType::Layer))
        .unwrap_or_else(|| layers.active())
        .to_owned()
}

/// The tool whose layer objects of these kinds go to, when they are all of one annotation kind.
pub fn tool_of(geometries: &[&EntityGeometry]) -> Option<AnnotationTool> {
    let kind = |g: &EntityGeometry| match g {
        EntityGeometry::Text { .. } => Some(AnnotationTool::Text),
        EntityGeometry::Dimension { .. } => Some(AnnotationTool::Dimension),
        EntityGeometry::Hatch { .. } => Some(AnnotationTool::Hatch),
        _ => None,
    };
    let first = kind(geometries.first()?)?;
    geometries.iter().all(|g| kind(g) == Some(first)).then_some(first)
}

/// The lock sentence of `tool`'s layer, when it is locked (the drawing tools' words).
pub fn locked(doc: &Document, tool: AnnotationTool) -> Option<String> {
    let layers = doc.layers();
    let id = layer_for(doc, tool);
    layers.is_locked(&id).then(|| {
        let name = layers.get(&id).map_or(id.as_str(), |n| n.name.as_str());
        if id == layers.active() {
            format!("“{name}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.")
        } else {
            format!(
                "{} “{name}” katmanına çizilir ve o katman kilitli. Kilidi Katmanlar panelinden açın ya da Proje ayarları › Araç katmanları'nı değiştirin.",
                tool.plural()
            )
        }
    })
}
