//! The web app's icon names (`ui/icons.ts`) mapped to the KentOS UI icon set.
//! The web draws about 140 icons, the UI set about 100: a web icon without
//! its own counterpart takes the closest one by meaning, and an unknown name
//! the plain button glyph.

use kentos_ui::icon::Icon;

pub fn from_web(name: Option<&str>) -> Icon {
    let Some(name) = name else {
        return Icon::Button;
    };
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
        | "boundary" | "toArea" | "parcel" | "subdivide" | "hatch" | "revcloud" => Icon::Polygon,
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
