//! Puts what a reader produced into the drawing (the web's `io/apply.ts`).
//! The targets are checked first, so nothing changes when one is missing or
//! locked; then the new layers are made and every object goes in, all as ONE
//! undo step named after the file (CLAUDE.md §4.8, §7): undo takes the
//! objects and the layers made for them (docs/adr/0076).
//!
//! The web also checks each object again here, as it checks a `.kcad` file's,
//! because they reach it from the formats worker as plain data. The desktop
//! gets the reader's typed objects; the one check left, that every number is
//! finite, runs where the file is read, off the UI thread ([`unusable`]).

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use kentos_contracts::{Entity, LayerNode, LayerNodeType, LayerStyle, Vec2};
use kentos_domain::{Document, Group, NewLayer, Slot};

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

/// The targets checked and the new layers named: what an import needs before
/// it changes anything.
struct Prepared {
    taken: HashSet<String>,
    /// Source layer → the id its new layer gets.
    new_ids: Vec<(String, String)>,
    /// Source layer → the layer its objects go to.
    targets: HashMap<String, String>,
}

/// Checks the plan's targets (an existing layer that is missing or locked
/// refuses the import) and names the layers to make; nothing changes.
fn prepare(doc: &Document, plan: &ImportPlan) -> Result<Prepared, String> {
    let mut taken = HashSet::new();
    node_ids(doc.layers().nodes(), &mut taken);
    let mut new_ids: Vec<(String, String)> = Vec::new();
    let mut targets = HashMap::new();
    for (source, target) in &plan.layers {
        match target {
            LayerTarget::New { name, .. } => {
                let id = fresh_id(name, &mut taken);
                targets.insert(source.clone(), id.clone());
                new_ids.push((source.clone(), id));
            }
            LayerTarget::Existing(id) => match doc.layers().get(id) {
                Some(node) if node.kind == LayerNodeType::Layer => {
                    if doc.layers().is_locked(id) {
                        return Err(format!(
                            "“{}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katman seçin.",
                            node.name
                        ));
                    }
                    targets.insert(source.clone(), id.clone());
                }
                _ => {
                    return Err(format!(
                        "Hedef katman ({id}) çizimde yok; pencereyi kapatıp yeniden açın."
                    ));
                }
            },
        }
    }
    Ok(Prepared {
        taken,
        new_ids,
        targets,
    })
}

/// The group named in the plan (found or made) and the new layers in it:
/// inside the import's transaction or group, so undo takes them too.
fn make_layers(
    doc: &mut Document,
    plan: &ImportPlan,
    prepared: &mut Prepared,
) -> Result<Vec<String>, String> {
    let mut created = Vec::new();
    if prepared.new_ids.is_empty() {
        return Ok(created);
    }
    let parent = match &plan.group {
        Some(group) => {
            let key = fold_turkish(group);
            let found = doc
                .layers()
                .nodes()
                .iter()
                .find(|n| n.kind == LayerNodeType::Group && fold_turkish(&n.name) == key)
                .map(|n| n.id.clone());
            match found {
                Some(id) => Some(id),
                None => {
                    let new = NewLayer {
                        id: Some(fresh_id(group, &mut prepared.taken)),
                        ..NewLayer::group(group.clone())
                    };
                    Some(doc.add_layer(new, None, false).map_err(|r| r.to_string())?)
                }
            }
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
        let id = prepared
            .new_ids
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
        doc.add_layer(new, parent.as_deref(), false)
            .map_err(|r| r.to_string())?;
        created.push(name.clone());
    }
    Ok(created)
}

/// `e` onto its target layer, or none when its layer is left out.
fn retarget(mut e: Entity, targets: &HashMap<String, String>) -> Option<Entity> {
    let layer = targets.get(&e.base().layer_id)?.clone();
    e.base_mut().layer_id = layer;
    Some(e)
}

pub fn apply_import(
    doc: &mut Document,
    entities: Vec<Entity>,
    plan: &ImportPlan,
) -> Result<Applied, String> {
    let mut prepared = prepare(doc, plan)?;
    let chosen: Vec<Entity> = entities
        .into_iter()
        .filter_map(|e| retarget(e, &prepared.targets))
        .collect();
    let mut created = Vec::new();
    let slots = doc.transact(&plan.label, |doc| {
        created = make_layers(doc, plan, &mut prepared)?;
        doc.add_many(chosen, &plan.label)
            .map_err(|e| format!("{e}. Hiçbir nesne eklenmedi."))
    })?;
    Ok(Applied { slots, created })
}

/// Objects written between two looks at the clock.
const BATCH: usize = 2048;

/// A large import going into the drawing a slice of time at a time, so the
/// window keeps drawing (and the objects appear as they go in), yet as ONE
/// undo step: its layers and every object are made inside a group, which
/// [`Progressive::stop`] reverts whole.
#[derive(Debug)]
pub struct Progressive {
    group: Option<Group>,
    targets: HashMap<String, String>,
    entities: std::vec::IntoIter<Entity>,
    label: String,
    total: usize,
    seen: usize,
    pub slots: Vec<Slot>,
    pub created: Vec<String>,
}

impl Progressive {
    /// Checks the targets, opens the group and makes the new layers in it;
    /// nothing is left behind when a target refuses. The objects come with [`feed`](Self::feed).
    pub fn start(doc: &mut Document, plan: &ImportPlan) -> Result<Self, String> {
        let mut prepared = prepare(doc, plan)?;
        let group = doc.begin_group(&plan.label);
        let made = doc.transact(&plan.label, |doc| make_layers(doc, plan, &mut prepared));
        let created = match made {
            Ok(c) => c,
            Err(e) => {
                doc.cancel_group(group);
                return Err(e);
            }
        };
        Ok(Self {
            group: Some(group),
            targets: prepared.targets,
            total: 0,
            entities: Vec::new().into_iter(),
            label: plan.label.clone(),
            seen: 0,
            slots: Vec::new(),
            created,
        })
    }

    /// The objects to write, the reader's (their `layerId` the source layer's name).
    pub fn feed(&mut self, entities: Vec<Entity>) {
        self.total = entities.len();
        self.seen = 0;
        self.slots.reserve(entities.len());
        self.entities = entities.into_iter();
    }

    /// How far it is, 0..1.
    pub fn share(&self) -> f32 {
        if self.total == 0 {
            1.0
        } else {
            self.seen as f32 / self.total as f32
        }
    }

    /// Objects looked at so far, and in all.
    pub fn counts(&self) -> (usize, usize) {
        (self.seen, self.total)
    }

    /// Writes objects for about `budget`; true once every object is in and
    /// the undo step is closed. A refusal reverts everything the import did.
    pub fn step(&mut self, doc: &mut Document, budget: Duration) -> Result<bool, String> {
        let until = Instant::now() + budget;
        loop {
            let mut batch = Vec::with_capacity(BATCH);
            for e in self.entities.by_ref().take(BATCH) {
                self.seen += 1;
                if let Some(e) = retarget(e, &self.targets) {
                    batch.push(e);
                }
            }
            let last = self.seen >= self.total;
            if !batch.is_empty() {
                match doc.add_many(batch, &self.label) {
                    Ok(slots) => self.slots.extend(slots),
                    Err(e) => {
                        self.cancel(doc);
                        return Err(format!("{e}. Hiçbir nesne eklenmedi."));
                    }
                }
            }
            if last {
                if let Some(group) = self.group.take() {
                    doc.end_group(group);
                }
                return Ok(true);
            }
            if Instant::now() >= until {
                return Ok(false);
            }
        }
    }

    fn cancel(&mut self, doc: &mut Document) {
        if let Some(group) = self.group.take() {
            doc.cancel_group(group);
        }
    }

    /// Stops the import: everything it made is reverted, and nothing is recorded.
    pub fn stop(mut self, doc: &mut Document) {
        self.cancel(doc);
    }
}

/// The first object with a number that is not finite (NaN or infinite),
/// which the drawing could neither show nor save: `(its place from 1, its
/// kind)`. Every float of every object is looked at once, in place (a JSON
/// round trip of each object, which this used to be, took seconds for a
/// large file).
pub fn unusable(entities: &[Entity]) -> Option<(usize, &'static str)> {
    entities
        .iter()
        .position(|e| !finite(e))
        .map(|i| (i + 1, entities[i].kind()))
}

/// A path's numbers are finite: its rings' points and bulges, a multi-part
/// area's every part too (docs/adr/0143).
fn path_finite(e: &kentos_contracts::PathEntity) -> bool {
    let ring = |pts: &[Vec2], bulges: Option<&[f64]>| {
        pts.iter().all(|v| v.x.is_finite() && v.y.is_finite())
            && bulges.is_none_or(|b| b.iter().all(|x| x.is_finite()))
    };
    let holes = |holes: &Option<Vec<kentos_contracts::RingGeometry>>| {
        holes
            .iter()
            .flatten()
            .all(|h| ring(&h.pts, h.bulges.as_deref()))
    };
    ring(&e.pts, e.bulges.as_deref())
        && holes(&e.holes)
        && e.parts
            .iter()
            .flatten()
            .all(|part| ring(&part.pts, part.bulges.as_deref()) && holes(&part.holes))
}

fn finite(e: &Entity) -> bool {
    let p = |v: &Vec2| v.x.is_finite() && v.y.is_finite();
    let ps = |v: &[Vec2]| v.iter().all(p);
    let fs = |v: &[f64]| v.iter().all(|x| x.is_finite());
    // Its own line weight, when it has one, is a weight a drawing can hold (docs/adr/0139).
    let weight = e
        .base()
        .line_weight
        .is_none_or(|w| (0.0..=kentos_contracts::MAX_LINE_WEIGHT).contains(&w));
    weight
        && match e {
            Entity::Point(e) => p(&e.p) && e.z.is_none_or(f64::is_finite),
            Entity::Line(e) => p(&e.a) && p(&e.b),
            Entity::Polyline(e) | Entity::Polygon(e) => path_finite(e),
            Entity::Circle(e) => p(&e.c) && e.r.is_finite(),
            Entity::Arc(e) => p(&e.c) && fs(&[e.r, e.a0, e.a1]),
            Entity::Ellipse(e) => p(&e.c) && p(&e.major) && fs(&[e.ratio, e.t0, e.t1]),
            Entity::Spline(e) => ps(&e.pts),
            Entity::Xline(e) | Entity::Ray(e) => p(&e.p) && p(&e.dir),
            Entity::Text(e) => p(&e.p) && e.height.is_finite() && e.rotation.is_finite(),
            Entity::Dimension(e) => {
                p(&e.a)
                    && p(&e.b)
                    && e.offset.is_finite()
                    && e.height.is_finite()
                    && e.angle.is_none_or(f64::is_finite)
                    && e.c.as_ref().is_none_or(p)
            }
            Entity::Hatch(e) => {
                ps(&e.ring)
                    && e.holes.as_deref().is_none_or(|h| h.iter().all(|r| ps(r)))
                    && e.pattern.angle.is_finite()
                    && e.pattern.spacing.is_finite()
            }
        }
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
            line_weight: None,
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
            za: None,
            zb: None,
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

    /// The layers go into the named group, in the import's one step (docs/adr/0076).
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
        // One step: undo takes the objects and the layers made for them, redo brings both back.
        assert_eq!(doc.undo().as_deref(), Some("DXF: plan.dxf"));
        assert_eq!(doc.len(), 0);
        assert!(doc.layers().get("import-plan-dxf").is_none());
        assert_eq!(doc.redo().as_deref(), Some("DXF: plan.dxf"));
        assert_eq!(doc.len(), 2);
        assert_eq!(
            doc.layers()
                .get("import-plan-dxf")
                .map(|g| g.children.len()),
            Some(2)
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
        // A multi-part area's other part is checked too (docs/adr/0143).
        let v = |x: f64, y: f64| Vec2 { x, y };
        let area: Entity = serde_json::from_value(serde_json::json!({
            "kind": "polygon", "id": 1, "layerId": "0", "attrs": {},
            "pts": [{ "x": 0, "y": 0 }, { "x": 1, "y": 0 }, { "x": 1, "y": 1 }],
            "parts": [{ "pts": [{ "x": 5, "y": 0 }, { "x": 6, "y": 0 }, { "x": 6, "y": 1 }] }]
        }))
        .expect("an area");
        assert_eq!(unusable(std::slice::from_ref(&area)), None);
        let Entity::Polygon(mut bad) = area else {
            panic!("an area")
        };
        bad.parts.as_mut().expect("parts")[0].pts[1] = v(f64::INFINITY, 0.0);
        assert_eq!(unusable(&[Entity::Polygon(bad)]), Some((1, "polygon")));
    }
}
