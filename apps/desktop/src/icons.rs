//! The web app's icons (`ui/icons.ts`): the inventory carries their SVG
//! (docs/adr/0054) and KentOS UI draws them as the web does, so a command
//! looks the same on both. A name the inventory does not have takes the
//! closest KentOS UI icon by meaning, and an unknown name the plain button
//! glyph.

use std::collections::HashMap;
use std::sync::OnceLock;

use kentos_ui::icon::Icon;

/// The web's icon set, name → SVG markup (the catalog keeps it from the inventory).
static WEB: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

/// Keeps the web's icon set; the first set kept stays.
pub fn keep_web_set(icons: HashMap<&'static str, &'static str>) {
    let _ = WEB.set(icons);
}

/// The web's SVG of an icon, when the inventory has it.
pub fn web_markup(name: &str) -> Option<&'static str> {
    WEB.get()?.get(name).copied()
}

pub fn from_web(name: Option<&str>) -> Icon {
    let Some(name) = name else {
        return Icon::Button;
    };
    web_markup(name).map_or_else(|| closest(name), Icon::Svg)
}

/// The KentOS UI icon closest in meaning to a web icon.
fn closest(name: &str) -> Icon {
    match name {
        // Selection and view
        "select" => Icon::Select,
        "pan" | "move" => Icon::Pan,
        "zoomExtents" => Icon::ZoomExtents,
        "zoomIn" | "zoomWindow" => Icon::ZoomIn,
        "zoomOut" => Icon::ZoomOut,
        "zoomSelection" | "stakeout" | "surveyStakeout" => Icon::Target,
        "deselect" => Icon::ClearSelection,
        "selectAll" => Icon::SelectAll,
        "invertSelection" => Icon::InvertSelection,
        "selectExpression" | "search" => Icon::Search,
        "eye" => Icon::Eye,
        "fullscreen" => Icon::Maximize,
        "moon" | "sun" => Icon::Contrast,
        "grid" | "toolbox" => Icon::Grid,
        "snap" => Icon::Magnet,
        "ortho" | "polar" | "tracking" | "surveyPolar" | "surveyForward" | "surveyResection"
        | "surveyTraverse" => Icon::Crosshair,
        "panelBottom" => Icon::ViewRows,
        "panelRight" | "dock" => Icon::ViewColumns,
        "ribbon" => Icon::Tabs,
        "chevronUp" => Icon::ChevronUp,
        // Files and project
        "fileNew" | "modelNew" => Icon::DocumentNew,
        "fileOpen" | "folder" | "folderAdd" => Icon::Open,
        "save" => Icon::Save,
        "saveAs" => Icon::SaveAs,
        "export" | "cloudUpload" => Icon::Export,
        "import" => Icon::Import,
        "print" | "sheet" => Icon::Print,
        "history" => Icon::Clock,
        "trash" | "erase" => Icon::Eraser,
        // Editing
        "undo" => Icon::Undo,
        "redo" => Icon::Redo,
        "copy" | "cut" | "paste" | "pasteOriginal" => Icon::Copy,
        // Drawing
        "line" | "xline" | "ray" | "parallel" | "perpIn" | "perpOut" | "lengthen" | "extend"
        | "trim" | "break" => Icon::Line,
        "polyline" | "spline" | "join" | "toPolyline" | "offset" | "vertex" => Icon::Polyline,
        "polygon" | "area" | "areaUnion" | "areaIntersect" | "areaSubtract" | "areaSplit"
        | "partsJoin" | "partsSplit" | "boundary" | "toArea" | "parcel" | "subdivide" | "hatch"
        | "revcloud" => Icon::Polygon,
        "rectangle" | "rectangle3" | "regularPolygon" | "array" | "arrayPolar" | "align" => {
            Icon::Rectangle
        }
        "circle" | "arc" | "ellipse" | "donut" | "fillet" | "chamfer" => Icon::Circle,
        "point" | "spot" | "divide" | "numberVertices" => Icon::Point,
        "text" | "penTool" | "edit" => Icon::Type,
        "dimension" | "measure" | "edgeLengths" | "scale" | "stretch" | "rotate" | "mirror"
        | "explode" => Icon::Ruler,
        // Layers, style, data
        "layers" | "layerAdd" | "layerStyle" => Icon::Layers,
        "styles" | "symbolAssign" | "symbolClear" | "lineWeight" => Icon::Drop,
        "legend" => Icon::Legend,
        "table" | "parcelReport" => Icon::Table,
        "fieldCalc" | "processing" => Icon::Hash,
        "contours" | "profile" | "slope" | "volume" => Icon::Cube,
        "crs" | "crsQuery" | "crsTransform" | "cloud" | "server" | "signIn" | "signOut" => {
            Icon::Globe
        }
        "modeHybrid" | "modeCad" | "modeGis" | "modePlan3d" | "modeDisaster" => Icon::Layout,
        // Help and settings
        "info" => Icon::Info,
        "keyboard" | "terminal" => Icon::Terminal,
        "settings" => Icon::Properties,
        "conflict" => Icon::Warning,
        _ => Icon::Button,
    }
}

#[cfg(test)]
mod tests {
    use kentos_ui::icon::{Icon, svg_elements};

    use crate::catalog::catalog;

    /// Every icon of the web's set is drawn whole (no element the reader
    /// skips), and every command, tool and panel icon the inventory names
    /// comes from that set: the desktop shows the web's icons.
    #[test]
    fn the_webs_icons_are_drawn_whole_and_every_named_one_is_there() {
        let _ = catalog();
        let set = super::WEB.get().expect("the inventory's icon set");
        assert!(set.len() > 150, "{} icons", set.len());
        for (name, markup) in set {
            let written = ["<path", "<rect", "<circle", "<ellipse"]
                .iter()
                .map(|tag| markup.matches(tag).count())
                .sum::<usize>();
            assert_eq!(svg_elements(markup), written, "{name}: {markup}");
        }
        let inventory: serde_json::Value =
            serde_json::from_str(include_str!("../../../docs/inventory/web.json")).expect("reads");
        let mut named = Vec::new();
        for list in ["commands", "tools"] {
            for item in inventory[list].as_array().into_iter().flatten() {
                named.extend(item["icon"].as_str());
            }
        }
        // The ribbon's panels and split buttons name theirs too (a method's own icon).
        let mut ribbon = Vec::new();
        icons_in(&inventory["layout"], &mut ribbon);
        named.extend(ribbon);
        assert!(named.len() > 100);
        for name in named {
            assert!(
                super::web_markup(name).is_some(),
                "{name} is not in the web's set"
            );
        }
        // And every icon this crate asks for by its name.
        for (file, name) in asked("from_web(Some(\"") {
            assert!(
                super::web_markup(&name).is_some(),
                "{file}: {name} is not in the web's set"
            );
        }
        for (file, name) in asked("svg_icon(\"") {
            assert!(
                crate::style::svgedit::known_icon(&name),
                "{file}: {name} is not an icon"
            );
        }
        // A command's icon is the web's drawing.
        let save = catalog().get("file.save").expect("Kaydet");
        assert!(matches!(save.icon, Icon::Svg(_)), "{:?}", save.icon);
    }

    /// Every `icon` a part of the inventory names.
    fn icons_in<'a>(value: &'a serde_json::Value, out: &mut Vec<&'a str>) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, v) in map {
                    match (key.as_str(), v.as_str()) {
                        ("icon", Some(name)) => out.push(name),
                        _ => icons_in(v, out),
                    }
                }
            }
            serde_json::Value::Array(items) => items.iter().for_each(|v| icons_in(v, out)),
            _ => {}
        }
    }

    /// The names this crate's sources give right after `call`, with their files.
    fn asked(call: &str) -> Vec<(String, String)> {
        fn sources(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).expect("the sources").flatten() {
                let path = entry.path();
                if path.is_dir() {
                    sources(&path, out);
                } else if path.extension().is_some_and(|x| x == "rs") {
                    out.push(path);
                }
            }
        }
        let mut files = Vec::new();
        sources(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut files,
        );
        let mut out = Vec::new();
        for file in files {
            let text = std::fs::read_to_string(&file).expect("a source");
            for (at, _) in text.match_indices(call) {
                let rest = &text[at + call.len()..];
                if let Some(end) = rest.find('"') {
                    out.push((file.display().to_string(), rest[..end].to_owned()));
                }
            }
        }
        assert!(!out.is_empty(), "no {call}");
        out
    }
}
