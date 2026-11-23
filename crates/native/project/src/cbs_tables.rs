//! A CBS project's layers as its data store's tables (docs/adr/9196 §2, plan/ayrim.md AY-05): a purpose's tree with each
//! layer kept either as a table of the data store (its kind of geometry, Z where heights are its point, a column for its
//! label) or as the project file's annotation (the draft, the sheet frame, the grid, the texts). The wizard makes the
//! tables before the project opens; any client that makes a CBS project uses the same tree (docs/adr/9197).

use kentos_contracts::{LayerField, LayerFieldKind, LayerGeometry, LayerNode, LayerNodeType};

/// How a purpose's layer is kept.
#[derive(Debug, Clone, PartialEq)]
pub enum Kept {
    /// A table of the data store: what its rows are, with Z or not, and its columns.
    Table {
        geometry: LayerGeometry,
        z: bool,
        fields: Vec<LayerField>,
    },
    /// An annotation layer of the project file.
    Annotation,
}

/// The column a table layer's label reads: what an object's own label was in the project file.
pub const LABEL_FIELD: &str = "etiket";

/// The label column.
fn label_field() -> LayerField {
    LayerField {
        alias: Some("Etiket".to_owned()),
        ..LayerField::new(LABEL_FIELD, LayerFieldKind::Text)
    }
}

/// How a city map's or a cadastral sheet's layer is kept, by its id (docs/adr/9196 §2):
/// - the draft, the sheet frame, the grid and the texts are the project's annotation;
/// - blocks, parcels and buildings are areas, road axes, kerbs and contours lines, spot heights and traverse points
///   points; heights are their Z (contours, spot heights, traverse points);
/// - a layer that wrote its object's own label writes it into `etiket`.
///
/// A layer the trees do not have is a drawing's: kept as an annotation layer, never a table of a guessed kind.
pub fn kept(id: &str) -> Kept {
    let table = |geometry, z, labelled: bool| Kept::Table {
        geometry,
        z,
        fields: if labelled { vec![label_field()] } else { Vec::new() },
    };
    match id {
        "ada" | "parsel" => table(LayerGeometry::Area, false, true),
        "yapi" => table(LayerGeometry::Area, false, false),
        "yol-ekseni" | "kaldirim" => table(LayerGeometry::Line, false, false),
        "esyukselti" => table(LayerGeometry::Line, true, false),
        "ana-esyukselti" => table(LayerGeometry::Line, true, true),
        "kot" | "poligon" => table(LayerGeometry::Point, true, true),
        _ => Kept::Annotation,
    }
}

/// A layer's table name: its id with what a table name cannot take written `_` (`ana-esyukselti` → `ana_esyukselti`).
pub fn table_name(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

/// A purpose's tree as a CBS project keeps it: each table layer typed with its geometry, its columns set and its label
/// reading `etiket`; each other layer typed Açıklama; the groups as they are. The tables are made by the caller.
pub fn cbs_tree(nodes: Vec<LayerNode>) -> Vec<LayerNode> {
    nodes.into_iter().map(cbs_node).collect()
}

fn cbs_node(mut n: LayerNode) -> LayerNode {
    if n.kind == LayerNodeType::Group {
        n.children = cbs_tree(std::mem::take(&mut n.children));
        return n;
    }
    match kept(&n.id) {
        Kept::Table { geometry, fields, .. } => {
            n.geometry = Some(geometry);
            if fields.iter().any(|f| f.name == LABEL_FIELD)
                && let Some(label) = n.style.label.as_mut()
            {
                // `{label}` was the object's own label; a row's is its column.
                let template = label.template.take().unwrap_or_else(|| "{label}".to_owned());
                label.template = Some(template.replace("{label}", &format!("{{{LABEL_FIELD}}}")));
            }
            n.fields = fields;
        }
        Kept::Annotation => n.geometry = Some(LayerGeometry::Annotation),
    }
    n
}

/// An institution's template tree as a CBS project keeps it (docs/adr/9196 §2): a typed layer (points, lines, areas)
/// is a table with its own columns; a layer of any kind holds no table's kind and is the project's annotation; data and
/// service layers and groups stay as they are.
pub fn template_tree(nodes: Vec<LayerNode>) -> Vec<LayerNode> {
    nodes
        .into_iter()
        .map(|mut n| {
            if n.kind == LayerNodeType::Group {
                n.children = template_tree(std::mem::take(&mut n.children));
            } else if n.data.is_none() && n.service.is_none() && n.geometry.is_none() {
                n.geometry = Some(LayerGeometry::Annotation);
            }
            n
        })
        .collect()
}

/// A table layer of a CBS tree: the node, its group (none at the top), what its rows are, with Z or not, its columns.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub id: String,
    pub parent: Option<String>,
    pub geometry: LayerGeometry,
    pub z: bool,
    pub fields: Vec<LayerField>,
}

/// The table layers of a CBS tree in the tree's order: every layer typed points, lines or areas that is no data or
/// service layer yet. Z where the purpose's layer keeps heights (`kept`).
pub fn tables(nodes: &[LayerNode]) -> Vec<Table> {
    fn walk(nodes: &[LayerNode], parent: Option<&str>, out: &mut Vec<Table>) {
        for n in nodes {
            if n.kind == LayerNodeType::Group {
                walk(&n.children, Some(&n.id), out);
                continue;
            }
            if n.data.is_some() || n.service.is_some() {
                continue;
            }
            let Some(geometry) = n.geometry.filter(|g| *g != LayerGeometry::Annotation) else {
                continue;
            };
            let z = matches!(kept(&n.id), Kept::Table { z: true, .. });
            out.push(Table {
                id: n.id.clone(),
                parent: parent.map(str::to_owned),
                geometry,
                z,
                fields: n.fields.clone(),
            });
        }
    }
    let mut out = Vec::new();
    walk(nodes, None, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::new_project::{city_layers, standard_layers};

    fn leaves(nodes: &[LayerNode], out: &mut Vec<LayerNode>) {
        for n in nodes {
            if n.kind == LayerNodeType::Group {
                leaves(&n.children, out);
            } else {
                out.push(n.clone());
            }
        }
    }

    /// A cadastral sheet keeps its data in tables and its annotation in the project file; every label reads a column.
    #[test]
    fn a_cadastral_sheets_data_is_tables_and_its_sheet_annotation() {
        let tree = cbs_tree(standard_layers(1000.0));
        let mut all = Vec::new();
        leaves(&tree, &mut all);
        let annotation: Vec<&str> = all
            .iter()
            .filter(|n| n.geometry == Some(LayerGeometry::Annotation))
            .map(|n| n.id.as_str())
            .collect();
        assert_eq!(annotation, ["taslak", "pafta", "karelaj", "yazi"]);
        let made = tables(&tree);
        let ids: Vec<&str> = made.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(
            ids,
            ["ada", "parsel", "yapi", "yol-ekseni", "kaldirim", "esyukselti", "ana-esyukselti", "kot", "poligon"]
        );
        // Each table in its group; heights in Z.
        assert!(made.iter().all(|t| t.parent.is_some()));
        assert!(made.iter().find(|t| t.id == "kot").is_some_and(|t| t.z && t.geometry == LayerGeometry::Point));
        // Heights are Z; a label reads `etiket`.
        let ada = all.iter().find(|n| n.id == "ada").expect("ada");
        assert_eq!(ada.style.label.as_ref().and_then(|l| l.template.as_deref()), Some("{etiket} ada"));
        assert_eq!(ada.fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), [LABEL_FIELD]);
        let parsel = all.iter().find(|n| n.id == "parsel").expect("parsel");
        assert_eq!(parsel.style.label.as_ref().and_then(|l| l.template.as_deref()), Some("{etiket}"));
        assert!(matches!(kept("kot"), Kept::Table { geometry: LayerGeometry::Point, z: true, .. }));
        assert!(matches!(kept("yapi"), Kept::Table { geometry: LayerGeometry::Area, z: false, .. }));
        // The sheet frame keeps its object's own label.
        let pafta = all.iter().find(|n| n.id == "pafta").expect("pafta");
        assert!(pafta.style.label.as_ref().and_then(|l| l.template.as_deref()).is_some_and(|t| t.contains("{label}")));
        // A table's name takes no hyphen.
        assert_eq!(table_name("ana-esyukselti"), "ana_esyukselti");
    }

    /// A template's typed layers are tables, its layers of any kind annotation.
    #[test]
    fn a_templates_typed_layers_are_tables() {
        let mut mixed = standard_layers(1000.0).remove(0);
        mixed.id = "cizim".into();
        let mut roads = mixed.clone();
        roads.id = "yollar".into();
        roads.geometry = Some(LayerGeometry::Line);
        let tree = template_tree(vec![mixed, roads]);
        assert_eq!(tree[0].geometry, Some(LayerGeometry::Annotation));
        let made = tables(&tree);
        assert_eq!(made.len(), 1);
        assert_eq!((made[0].id.as_str(), made[0].geometry, made[0].z), ("yollar", LayerGeometry::Line, false));
    }

    /// A city map's draft is the project's annotation: it makes no table.
    #[test]
    fn a_city_maps_draft_is_annotation() {
        let tree = cbs_tree(city_layers());
        assert_eq!(tree[0].geometry, Some(LayerGeometry::Annotation));
        assert!(tables(&tree).is_empty());
        // A layer the trees do not have is never a table of a guessed kind.
        assert_eq!(kept("bilinmeyen"), Kept::Annotation);
    }
}
