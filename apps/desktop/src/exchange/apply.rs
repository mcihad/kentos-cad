//! Puts what a reader produced into the drawing (the web's `io/apply.ts`).
//! The targets are checked first, so nothing changes when one is missing or
//! locked; then the new layers are made (not undoable, as on the web and as
//! the processing runner's new target layers) and every object goes in as
//! ONE undo step named after the file (CLAUDE.md §4.8, §7).
//!
//! The web also checks each object again here, as it checks a `.kcad` file's,
//! because they reach it from the formats worker as plain data. The desktop
//! gets the reader's typed objects; the one check left, that every number is
//! finite, runs where the file is read, off the UI thread ([`unusable`]).

use std::collections::HashSet;

use kentos_contracts::{Entity, LayerNode, LayerNodeType, LayerStyle};
use kentos_domain::{Document, NewLayer, Slot};

/// Where the objects of one source layer go.
#[derive(Clone, Debug, PartialEq)]
pub enum LayerTarget {
    Existing(String),
    New {
        name: String,
        /// Boxed: a style is many times the size of an id.
        style: Box<LayerStyle>,
        visible: bool,
        locked: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportPlan {
    /// The undo step's name (“Koordinat listesi: noktalar.ncn”).
    pub label: String,
    /// Source layer name → target, new layers made in this order; objects of
    /// a layer missing here are left out.
    pub layers: Vec<(String, LayerTarget)>,
    /// A group at the top of the tree the new layers go into (made if
    /// missing); without it they go to the top of the tree.
    pub group: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct Applied {
    pub slots: Vec<Slot>,
    /// The names of the layers made.
    pub created: Vec<String>,
}

/// The web's `foldTurkish`: trimmed, upper case, the Turkish letters as plain
/// ones (`Ç` → `C`, `İ` and `I` alike). Upper-casing `i` gives `I` rather
/// than the Turkish `İ`, which folds to `I` all the same.
pub fn fold_turkish(text: &str) -> String {
    text.trim()
        .to_uppercase()
        .chars()
        .map(|c| match c {
            'Ç' => 'C',
            'Ğ' => 'G',
            'İ' => 'I',
            'Ö' => 'O',
            'Ş' => 'S',
            'Ü' => 'U',
            c => c,
        })
        .collect()
}

/// The project layer with this name, ignoring case and Turkish marks (DXF
/// layer names ignore case).
pub fn layer_named<'a>(doc: &'a Document, name: &str) -> Option<&'a LayerNode> {
    let key = fold_turkish(name);
    doc.layers()
        .leaves()
        .into_iter()
        .find(|l| fold_turkish(&l.name) == key)
}

/// An id no node of the tree has, readable from the name (`import-parsel-2`).
fn fresh_id(name: &str, taken: &mut HashSet<String>) -> String {
    let mut slug = String::new();
    for c in fold_turkish(name).to_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            slug.push(c);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    let base = format!("import-{}", if slug.is_empty() { "katman" } else { slug });
    let mut id = base.clone();
    let mut k = 2;
    while taken.contains(&id) {
        id = format!("{base}-{k}");
        k += 1;
    }
    taken.insert(id.clone());
    id
}

fn node_ids(nodes: &[LayerNode], out: &mut HashSet<String>) {
    for n in nodes {
        out.insert(n.id.clone());
        node_ids(&n.children, out);
    }
}

pub fn apply_import(
    doc: &mut Document,
    entities: Vec<Entity>,
    plan: &ImportPlan,
) -> Result<Applied, String> {
    let mut taken = HashSet::new();
    node_ids(doc.layers().nodes(), &mut taken);
    let mut new_ids: Vec<(String, String)> = Vec::new();
    for (source, target) in &plan.layers {
        match target {
            LayerTarget::New { name, .. } => {
                new_ids.push((source.clone(), fresh_id(name, &mut taken)));
            }
            LayerTarget::Existing(id) => match doc.layers().get(id) {
                Some(node) if node.kind == LayerNodeType::Layer => {
                    if doc.layers().is_locked(id) {
                        return Err(format!(
                            "“{}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katman seçin.",
                            node.name
                        ));
                    }
                }
                _ => {
                    return Err(format!(
                        "Hedef katman ({id}) çizimde yok; pencereyi kapatıp yeniden açın."
                    ));
                }
            },
        }
    }
    let target = |source: &str| -> Option<&str> {
        let (_, t) = plan.layers.iter().find(|(s, _)| s == source)?;
        match t {
            LayerTarget::Existing(id) => Some(id.as_str()),
            LayerTarget::New { .. } => new_ids
                .iter()
                .find(|(s, _)| s == source)
                .map(|(_, id)| id.as_str()),
        }
    };
    let chosen: Vec<Entity> = entities
        .into_iter()
        .filter_map(|mut e| {
            let layer = target(&e.base().layer_id)?.to_owned();
            e.base_mut().layer_id = layer;
            Some(e)
        })
        .collect();

    let mut created = Vec::new();
    if !new_ids.is_empty() {
        let parent = match &plan.group {
            Some(group) => {
                let key = fold_turkish(group);
                let found = doc
                    .layers()
                    .nodes()
                    .iter()
                    .find(|n| n.kind == LayerNodeType::Group && fold_turkish(&n.name) == key)
                    .map(|n| n.id.clone());
                Some(found.unwrap_or_else(|| {
                    let new = NewLayer {
                        id: Some(fresh_id(group, &mut taken)),
                        ..NewLayer::group(group.clone())
                    };
                    doc.add_layer(new, None)
                }))
            }
            None => None,
        };
        for (source, t) in &plan.layers {
            let LayerTarget::New {
                name,
                style,
                visible,
                locked,
            } = t
            else {
                continue;
            };
            let id = new_ids
                .iter()
                .find(|(s, _)| s == source)
                .map(|(_, id)| id.clone());
            let new = NewLayer {
                id,
                name: name.clone(),
                kind: LayerNodeType::Layer,
                visible: *visible,
                locked: *locked,
                style: (**style).clone(),
            };
            doc.add_layer(new, parent.as_deref());
            created.push(name.clone());
        }
    }
    let slots = doc
        .add_many(chosen, &plan.label)
        .map_err(|e| format!("{e}. Hiçbir nesne eklenmedi."))?;
    Ok(Applied { slots, created })
}

/// The first object with a number that is not finite (NaN or infinite),
/// which the drawing could neither show nor save: `(its place from 1, its
/// kind)`. A number of an object survives a JSON round trip unchanged only
/// when it is finite (JSON has no NaN; `serde_json` writes it as null).
pub fn unusable(entities: &[Entity]) -> Option<(usize, &'static str)> {
    entities.iter().enumerate().find_map(|(i, e)| {
        let back = serde_json::to_value(e)
            .ok()
            .and_then(|v| serde_json::from_value::<Entity>(v).ok());
        (back.as_ref() != Some(e)).then_some((i + 1, e.kind()))
    })
}

/// What the import window says when [`unusable`] finds one (the web's words).
pub fn unusable_text(place: usize, kind: &str) -> String {
    format!(
        "Dosyadan okunan nesneler çizime uymuyor (İçe aktarılan nesne {place} ({kind}): sonlu olmayan bir sayı taşıyor). Hiçbir şey eklenmedi; dosyayla birlikte bildirin."
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use kentos_contracts::{
        DocumentSnapshotV1, EntityBase, LineEntity, LineType, PointEntity, Vec2,
    };

    use super::*;

    /// Layers “Noktalar” (a) and “Kilitli” (k, locked), as the web's test.
    fn doc() -> Document {
        let text = r#"{"format":"kentos.document","version":1,"name":"t","settings":{"srid":5256,"lengthDecimals":3,
            "areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000},"origin":{"x":0,"y":0},
            "layers":[{"id":"a","name":"Noktalar","type":"layer","visible":true,"locked":false,"expanded":true,
            "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]},
            {"id":"k","name":"Kilitli","type":"layer","visible":true,"locked":true,"expanded":true,
            "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
            "activeLayer":"a","entities":[],"styles":{"items":[],"categories":[]}}"#;
        Document::from_snapshot(DocumentSnapshotV1::from_json(text).expect("reads")).expect("opens")
    }

    fn base(layer: &str) -> EntityBase {
        EntityBase {
            id: 0,
            layer_id: layer.into(),
            color: None,
            attrs: BTreeMap::new(),
            label: None,
            symbol: None,
        }
    }

    fn point(layer: &str, x: f64, y: f64) -> Entity {
        let mut b = base(layer);
        b.attrs.insert("Ad".into(), format!("P{x}"));
        b.label = Some(format!("P{x}"));
        Entity::Point(PointEntity {
            base: b,
            p: Vec2 { x, y },
            z: None,
        })
    }

    fn line(layer: &str) -> Entity {
        Entity::Line(LineEntity {
            base: base(layer),
            a: Vec2 { x: 0.0, y: 0.0 },
            b: Vec2 { x: 1.0, y: 1.0 },
        })
    }

    fn style(color: &str) -> LayerStyle {
        LayerStyle {
            color: color.into(),
            ..kentos_domain::default_style()
        }
    }

    #[test]
    fn every_object_is_one_undo_step_and_layers_merge_by_name() {
        let mut doc = doc();
        let objects: Vec<Entity> = (0..1000)
            .map(|i| {
                point(
                    "0",
                    452_000.0 + f64::from(i),
                    4_412_000.0 + f64::from(i) / 3.0,
                )
            })
            .collect();
        let target = layer_named(&doc, "NOKTALAR").expect("found").id.clone();
        let plan = ImportPlan {
            label: "Koordinat listesi: a.ncn".into(),
            layers: vec![("0".into(), LayerTarget::Existing(target))],
            group: None,
        };
        let applied = apply_import(&mut doc, objects, &plan).expect("applied");
        assert_eq!(applied.slots.len(), 1000);
        assert_eq!(doc.len(), 1000);
        // Coordinates exactly as read; ids are the document's.
        let Some(Entity::Point(first)) = doc.get(applied.slots[0]) else {
            panic!("a point");
        };
        assert_eq!((first.p.x, first.p.y), (452_000.0, 4_412_000.0));
        assert_eq!(doc.undo().as_deref(), Some("Koordinat listesi: a.ncn"));
        assert_eq!(doc.len(), 0);
        assert_eq!(doc.redo().as_deref(), Some("Koordinat listesi: a.ncn"));
        assert_eq!(doc.len(), 1000);
    }

    #[test]
    fn new_layers_go_into_the_named_group_and_unchosen_layers_stay_out() {
        let mut doc = doc();
        let plan = ImportPlan {
            label: "DXF: plan.dxf".into(),
            layers: vec![
                (
                    "PARSEL".into(),
                    LayerTarget::New {
                        name: "PARSEL".into(),
                        style: Box::new(style("#FF0000")),
                        visible: true,
                        locked: false,
                    },
                ),
                (
                    "YOL".into(),
                    LayerTarget::New {
                        name: "YOL".into(),
                        style: Box::new(style("fg")),
                        visible: false,
                        locked: true,
                    },
                ),
            ],
            group: Some("plan.dxf".into()),
        };
        let applied = apply_import(
            &mut doc,
            vec![line("PARSEL"), line("YOL"), line("DEFPOINTS")],
            &plan,
        )
        .expect("applied");
        assert_eq!(applied.created, ["PARSEL", "YOL"]);
        let group = doc
            .layers()
            .nodes()
            .iter()
            .find(|n| n.name == "plan.dxf")
            .expect("the group")
            .clone();
        assert_eq!(group.kind, LayerNodeType::Group);
        assert_eq!(group.id, "import-plan-dxf");
        let children: Vec<_> = group
            .children
            .iter()
            .map(|c| {
                (
                    c.id.as_str(),
                    c.name.as_str(),
                    c.visible,
                    c.locked,
                    c.style.color.as_str(),
                )
            })
            .collect();
        assert_eq!(
            children,
            [
                ("import-parsel", "PARSEL", true, false, "#FF0000"),
                ("import-yol", "YOL", false, true, "fg"),
            ]
        );
        assert_eq!(doc.len(), 2);
        assert_eq!(
            doc.layers().get("import-yol").map(|l| l.style.line_type),
            Some(LineType::Continuous)
        );

        // A second import finds the group and its layers again.
        let parsel = layer_named(&doc, "parsel").expect("found").id.clone();
        let again = apply_import(
            &mut doc,
            vec![line("PARSEL")],
            &ImportPlan {
                label: "DXF: plan.dxf".into(),
                layers: vec![("PARSEL".into(), LayerTarget::Existing(parsel))],
                group: Some("plan.dxf".into()),
            },
        )
        .expect("applied");
        assert!(again.created.is_empty());
        let groups = doc
            .layers()
            .nodes()
            .iter()
            .filter(|n| n.name == "plan.dxf")
            .count();
        assert_eq!(groups, 1);
    }

    #[test]
    fn a_locked_or_missing_target_changes_nothing() {
        let mut doc = doc();
        let locked = apply_import(
            &mut doc,
            vec![point("0", 1.0, 2.0)],
            &ImportPlan {
                label: "x".into(),
                layers: vec![("0".into(), LayerTarget::Existing("k".into()))],
                group: None,
            },
        );
        assert!(
            locked
                .expect_err("refused")
                .contains("“Kilitli” katmanı kilitli")
        );
        let before = doc.layers().leaves().len();
        let missing = apply_import(
            &mut doc,
            vec![point("0", 1.0, 2.0)],
            &ImportPlan {
                label: "x".into(),
                layers: vec![
                    (
                        "1".into(),
                        LayerTarget::New {
                            name: "Yeni".into(),
                            style: Box::new(style("fg")),
                            visible: true,
                            locked: false,
                        },
                    ),
                    ("0".into(), LayerTarget::Existing("yok".into())),
                ],
                group: None,
            },
        );
        assert!(missing.expect_err("refused").contains("(yok) çizimde yok"));
        assert_eq!(doc.len(), 0);
        assert_eq!(doc.layers().leaves().len(), before);
        assert!(!doc.can_undo());
    }

    #[test]
    fn names_fold_as_on_the_web() {
        assert_eq!(fold_turkish(" Çığ İzmir ölü şüphe "), "CIG IZMIR OLU SUPHE");
        let mut taken = HashSet::from(["import-yol".to_owned()]);
        assert_eq!(fresh_id("Yol", &mut taken), "import-yol-2");
        assert_eq!(
            fresh_id("Şehir Planı (2026)", &mut taken),
            "import-sehir-plani-2026"
        );
        assert_eq!(fresh_id("***", &mut taken), "import-katman");
    }

    #[test]
    fn an_object_with_a_number_that_is_not_finite_is_found() {
        let good = point("0", 1.0, 2.0);
        assert_eq!(unusable(&[good.clone(), good.clone()]), None);
        let nan = point("0", f64::NAN, 2.0);
        assert_eq!(unusable(&[good.clone(), nan]), Some((2, "point")));
        let Entity::Point(mut high) = good else {
            panic!("a point")
        };
        high.z = Some(f64::INFINITY);
        assert_eq!(unusable(&[Entity::Point(high)]), Some((1, "point")));
    }
}
