//! The layer an annotation tool draws on (usage review UR-003, `.kcad` schema 255): a project may name, for its dimensions,
//! its texts and its hatches, the layer each goes to instead of the active one. None named: the active layer, as
//! always. A new CAD project names its Ölçü, Yazı and Tarama layers; a project made before keeps drawing on the
//! active layer until the user names them.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// A tool whose objects may go to a layer of their own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AnnotationTool {
    /// Ölçülendirme: every kind of dimension.
    Dimension,
    /// Yazı and çok satırlı yazı.
    Text,
    /// Tarama.
    Hatch,
}

impl AnnotationTool {
    pub const ALL: [AnnotationTool; 3] = [AnnotationTool::Dimension, AnnotationTool::Text, AnnotationTool::Hatch];

    /// Its objects at the head of a sentence (“Ölçüler”, “Yazılar”, “Taramalar”).
    pub fn plural(self) -> &'static str {
        match self {
            AnnotationTool::Dimension => "Ölçüler",
            AnnotationTool::Text => "Yazılar",
            AnnotationTool::Hatch => "Taramalar",
        }
    }

    /// Its name in a sentence (“ölçü”, “yazı”, “tarama”).
    pub fn word(self) -> &'static str {
        match self {
            AnnotationTool::Dimension => "ölçü",
            AnnotationTool::Text => "yazı",
            AnnotationTool::Hatch => "tarama",
        }
    }
}

/// One tool's layer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ToolLayer {
    pub tool: AnnotationTool,
    /// The layer's id.
    pub layer: String,
}

/// What is wrong with a project's tool layers, if anything: a tool named twice, a layer id empty.
pub fn tool_layers_problem(list: &[ToolLayer]) -> Option<String> {
    for (i, t) in list.iter().enumerate() {
        if t.layer.trim().is_empty() {
            return Some(format!("{}. araç katmanının katmanı boş", i + 1));
        }
        if list[..i].iter().any(|o| o.tool == t.tool) {
            return Some(format!("{} aracının katmanı iki kez yazılmış", t.tool.word()));
        }
    }
    None
}

/// The layer a project names for `tool`, if any.
pub fn tool_layer(list: &[ToolLayer], tool: AnnotationTool) -> Option<&str> {
    list.iter().find(|t| t.tool == tool).map(|t| t.layer.as_str())
}
