//! Katman türü and Otomatik seçim (docs/adr/0622 §5). A typed layer holds one kind of object, as a GIS layer
//! does: points, lines, areas or annotation; a layer of any kind is Karışık. While the active layer is typed, a
//! drawing tool that writes another kind on it is off and its tip and its start say why; the drawing's own check
//! (`layer_geometry_mismatch`) holds every write to the rule, whatever made it. Otomatik seçim says whether a click
//! and a box select any shown layer's objects or the active layer's alone (`selectable::only_layer`); it is the
//! project's setting, written with `cad.settings.set`.

use kentos_commands::{
    LayerEditOperation, LayerGeometry, LayerGeometryChoice, LayerNodeType, LayersEdit, SettingsSet,
};
use kentos_native_application::{ExecutionContext, settings_set};
use kentos_ui::icon::Icon;
use kentos_ui::widget::Menu;

use crate::app::{App, Message};
use kentos_interaction::Level;

/// What a drawing tool writes on the active layer, as object kinds (`Entity::kind`); none for a tool that writes
/// nothing new there, or writes to a layer of its own (Parsel oluştur's parcel layer, a standard text layer), or
/// writes kinds this table does not foresee: the drawing's check refuses a misfit then.
pub(crate) fn tool_makes(tool: &str) -> Option<&'static [&'static str]> {
    Some(match tool {
        "point" | "vertexPoints" | "pointsBetween" | "intersectPoint" => &["point"],
        "line" | "parallel" | "perpIn" | "perpOut" | "tangentLine" => &["line"],
        "polyline" | "centerline" => &["polyline"],
        "arc" => &["arc"],
        "circle" => &["circle"],
        "ellipse" => &["ellipse"],
        "spline" => &["spline"],
        "nurbs" => &["nurbs"],
        "xline" => &["xline"],
        "ray" => &["ray"],
        // Menzil halkaları: its rings and its rays (docs/adr/0197 §3).
        "rangeRings" => &["circle", "line"],
        "polygon" | "rectangle" | "rectangle3" | "regularPolygon" | "sector" | "adjoin" | "revcloud" | "wipeout" | "planRoad"
        | "boundary" | "polygonize" => &["polygon"],
        "text" | "mtext" | "textAlong" | "placeTextFile" => &["text"],
        "dimension" | "dimContinue" | "dimBaseline" | "quickDimension" | "centerMark" | "dimCenterLine"
        | "dimElevation" => &["dimension"],
        "leader" => &["leader"],
        // Halka is a solid hatch (docs/adr/0057).
        "hatch" | "hatchSelected" | "donut" => &["hatch"],
        "blockInsert" => &["insert"],
        "imageInsert" => &["image"],
        "tablePlace" => &["table"],
        _ => return None,
    })
}

/// What a command writes on the active layer: a drawing tool's kinds, Tablo ekle's table.
fn command_makes(id: &str) -> Option<&'static [&'static str]> {
    match id {
        "table.insert" => Some(&["table"]),
        id => id.strip_prefix("tool.").and_then(tool_makes),
    }
}

/// Why a command that writes on the active layer is off: the layer is typed and does not hold what it writes;
/// a fitting layer is named when there is one.
pub(crate) fn misfit(doc: &kentos_domain::Document, id: &str) -> Option<String> {
    let kinds = command_makes(id)?;
    let layers = doc.layers();
    let active = layers.get(layers.active())?;
    let geometry = active.geometry?;
    let kind = kinds.iter().find(|k| !geometry.admits(k))?;
    let fits = |n: &kentos_commands::LayerNode| {
        n.kind == LayerNodeType::Layer
            && n.service.is_none()
            && !layers.is_locked(&n.id)
            && n.geometry.is_none_or(|g| kinds.iter().all(|k| g.admits(k)))
    };
    // A layer of the kind first, else one of any kind.
    let leaves = layers.leaves();
    let typed = leaves.iter().find(|n| fits(n) && n.geometry.is_some());
    let other = typed.or_else(|| leaves.iter().find(|n| fits(n)));
    let way = match other {
        Some(n) => format!("“{}” katmanını etkin yapın", n.name),
        None => "türüne uyan bir katmanı etkin yapın".to_owned(),
    };
    Some(format!(
        "Etkin katman “{}” bir {}; {} bu katmana yazılmaz. {way} ya da katmanın türünü Karışık yapın (sağ tık › Katman türü).",
        active.name,
        geometry.layer_words(),
        kentos_commands::kind_words(kind)
    ))
}

/// A layer type's glyph: the layer tree's row draws it in the layer's colour (as QGIS's symbol), the menus beside
/// its name.
pub(crate) fn glyph(geometry: Option<LayerGeometry>) -> Icon {
    crate::icons::from_web(Some(match geometry {
        Some(LayerGeometry::Point) => "layerPoint",
        Some(LayerGeometry::Line) => "layerLine",
        Some(LayerGeometry::Area) => "layerArea",
        Some(LayerGeometry::Annotation) => "layerAnnotation",
        None => "layers",
    }))
}

/// The choices Katman türü ▸ offers, in its order: Karışık first, as a new layer is.
const CHOICES: [LayerGeometryChoice; 5] = [
    LayerGeometryChoice::Mixed,
    LayerGeometryChoice::Point,
    LayerGeometryChoice::Line,
    LayerGeometryChoice::Area,
    LayerGeometryChoice::Annotation,
];

/// A choice's name in the menu.
fn choice_label(choice: LayerGeometryChoice) -> &'static str {
    match choice.geometry() {
        Some(g) => g.label(),
        None => "Karışık",
    }
}

/// The commands that add a typed layer (the Katmanlar panel's Yeni katman ▾ in a CBS project).
pub(crate) const NEW_TYPED: [(&str, LayerGeometry); 4] = [
    ("layer.newPoint", LayerGeometry::Point),
    ("layer.newLine", LayerGeometry::Line),
    ("layer.newArea", LayerGeometry::Area),
    ("layer.newAnnotation", LayerGeometry::Annotation),
];

impl App {
    /// Katman türü ▸: the five choices, the layer's checked; a choice its objects do not fit is refused when
    /// chosen, with the reason.
    pub(crate) fn layer_type_menu(&self, node: &kentos_commands::LayerNode) -> Menu<Message> {
        let current = LayerGeometryChoice::of(node.geometry);
        CHOICES.iter().fold(Menu::new(), |menu, choice| {
            menu.radio(
                choice_label(*choice),
                *choice == current,
                Message::Layer(crate::layering::Event::Geometry(node.id.clone(), *choice)),
            )
            .icon(glyph(choice.geometry()))
        })
    }

    /// A layer's type set (`cad.layers.edit`'s `geometry`), said; refused when an object on it is of another kind.
    pub(crate) fn set_layer_geometry(&mut self, id: String, choice: LayerGeometryChoice) {
        let Some(doc) = &self.document else {
            return;
        };
        let Some(name) = doc.model.layers().get(&id).map(|n| n.name.clone()) else {
            return;
        };
        let mut input = LayersEdit::new(LayerEditOperation::Geometry);
        input.layer = Some(id);
        input.geometry = Some(choice);
        if let Some(done) = self.edit_layers(input)
            && done.changed
        {
            self.say(
                Level::Success,
                match choice.geometry() {
                    Some(g) => format!(
                        "“{name}” artık bir {}: yalnız {} alır.",
                        g.layer_words(),
                        holds(g)
                    ),
                    None => format!("“{name}” artık karışık bir katman: her türden nesne alır."),
                },
            );
        }
    }

    /// `layer.newPoint` … `layer.newAnnotation`: a layer of that type next to the active one, made active.
    pub(crate) fn new_typed_layer(&mut self, geometry: LayerGeometry) {
        if let Some(why) = self.tree_locked() {
            self.warn(why);
            return;
        }
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let name = doc.model.layers().unique_name(new_name(geometry));
        let mut input = LayersEdit::new(LayerEditOperation::Add);
        input.name = Some(name.clone());
        input.parent = Some(doc.model.layers().active().to_owned());
        input.activate = Some(true);
        input.geometry = Some(LayerGeometryChoice::of(Some(geometry)));
        if self.edit_layers(input).is_some() {
            self.say(
                Level::Success,
                format!(
                    "“{name}” eklendi ve etkin yapıldı: bir {}, yalnız {} alır.",
                    geometry.layer_words(),
                    holds(geometry)
                ),
            );
        }
    }

    /// Why a command that writes on the active layer is off now (`misfit`).
    pub(crate) fn layer_misfit(&self, id: &str) -> Option<String> {
        misfit(&self.document.as_ref()?.model, id)
    }

    /// `edit.autoSelect`: Otomatik seçim turned over (`cad.settings.set`'s `autoSelect`), said.
    pub(crate) fn toggle_auto_select(&mut self) {
        let Some(doc) = self.document.as_mut() else {
            self.output("Açık çizim yok.");
            return;
        };
        let on = !doc.model.settings().auto_select();
        let result = settings_set::execute(
            &mut ExecutionContext::new(&mut doc.model),
            SettingsSet {
                auto_select: Some(on),
                ..Default::default()
            },
        );
        if self.answered(result).is_none() {
            return;
        }
        let said = if on {
            "Otomatik seçim açık: tıklama ve kutu bütün görünen katmanların nesnelerini seçer.".to_owned()
        } else {
            let active = self.document.as_ref().map_or_else(String::new, |d| {
                let layers = d.model.layers();
                layers
                    .get(layers.active())
                    .map_or_else(String::new, |n| n.name.clone())
            });
            format!(
                "Otomatik seçim kapalı: yalnız etkin katmanın (“{active}”) nesneleri seçilir. Başka katmanın nesnesi için o katmanı etkin yapın."
            )
        };
        self.say(Level::Success, said);
    }

    /// Whether Otomatik seçim is on in the open drawing (the ribbon's and the status bar's check).
    pub(crate) fn auto_select_on(&self) -> bool {
        self.document
            .as_ref()
            .is_some_and(|d| d.model.settings().auto_select())
    }
}

/// A new layer's name by its type.
fn new_name(geometry: LayerGeometry) -> &'static str {
    match geometry {
        LayerGeometry::Point => "Yeni nokta katmanı",
        LayerGeometry::Line => "Yeni çizgi katmanı",
        LayerGeometry::Area => "Yeni alan katmanı",
        LayerGeometry::Annotation => "Yeni açıklama katmanı",
    }
}

/// What a typed layer takes, as a sentence says it.
fn holds(geometry: LayerGeometry) -> &'static str {
    match geometry {
        LayerGeometry::Point => "nokta",
        LayerGeometry::Line => "çizgi, çoklu çizgi, yay, eğri, daire ve elips",
        LayerGeometry::Area => "alan, daire ve elips",
        LayerGeometry::Annotation => "yazı, ölçü, kılavuz, tablo, etiket, tarama, blok ve resim",
    }
}

#[cfg(test)]
mod tests {
    use super::{misfit, tool_makes};
    use kentos_commands::LayerGeometry;

    #[test]
    fn every_kind_a_tool_makes_is_a_kind_of_object() {
        // Each kind the table names is one a layer type knows of, or one none holds (a construction line).
        for tool in kentos_interaction::Session::tools() {
            for kind in tool_makes(tool).unwrap_or_default() {
                let known = kentos_commands::LAYER_GEOMETRIES
                    .iter()
                    .any(|g| g.admits(kind))
                    || matches!(*kind, "xline" | "ray");
                assert!(known, "{tool}: {kind}");
            }
        }
        // The table's tools are the session's.
        for tool in [
            "point", "line", "polyline", "polygon", "circle", "text", "dimension", "hatch", "leader",
            "blockInsert", "imageInsert", "rangeRings",
        ] {
            assert!(kentos_interaction::Session::tools().contains(&tool), "{tool}");
        }
    }

    #[test]
    fn a_typed_active_layer_turns_off_the_tools_that_draw_another_kind() {
        let mut app = crate::files_testing::app_with_drawing();
        let doc = &mut app.document.as_mut().expect("a drawing").model;
        let active = doc.layers().active().to_owned();
        let name = doc.layers().get(&active).map(|n| n.name.clone()).expect("the active layer");
        // Karışık: everything.
        assert_eq!(misfit(doc, "tool.line"), None);
        // An area layer.
        doc.set_layer_geometry(&active, Some(LayerGeometry::Area), "Katman türü")
            .expect("its objects are areas");
        assert_eq!(misfit(doc, "tool.polygon"), None);
        assert_eq!(misfit(doc, "tool.rectangle3"), None);
        assert_eq!(misfit(doc, "tool.circle"), None, "a closed curve fits an area layer");
        assert_eq!(misfit(doc, "tool.move"), None, "a tool that writes nothing new");
        let why = misfit(doc, "tool.line").expect("a line on an area layer");
        assert!(
            why.starts_with(&format!("Etkin katman “{name}” bir alan katmanı; çizgi bu katmana yazılmaz.")),
            "{why}"
        );
        assert!(misfit(doc, "table.insert").is_some());
        assert!(app.why_disabled("tool.line").is_some());
        assert!(!app.available("tool.line"));
        assert!(app.available("tool.polygon"));
        app.new_typed_layer(LayerGeometry::Line);
        assert!(!app.available("tool.polygon"));
        assert!(!app.available("tool.rectangle3"), "a rotated rectangle is an area too");
        assert!(app.available("tool.line"));
        assert!(app.available("tool.spot"), "spot elevations use their own layer");
    }
}
