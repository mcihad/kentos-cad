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

use kentos_contracts::blocks::import_names;
use kentos_contracts::{
    BlockDefinition, BlockId, DimensionStyleDef, Entity, LayerNode, LayerNodeType, LayerStyle,
    ProjectSettings, TextStyleDef, Vec2, dimension_styles_problem, text_styles_problem,
};
use kentos_domain::{Document, Group, NewLayer, Slot, Uuid};

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
        /// The fields the file gives the layer (docs/adr/0199 §6).
        fields: Vec<kentos_contracts::LayerField>,
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
    /// The block definitions taken in, and the names changed on the way:
    /// as the file says, as it went in (docs/adr/0144 §5).
    pub blocks: usize,
    pub renamed: Vec<(String, String)>,
    /// The text and dimension styles added to the project (docs/adr/0183 §7).
    pub styles: (usize, usize),
}

/// The text and dimension styles a reader brought (docs/adr/0183 §7): their
/// ids the reader's (`dxf-text-1` …), the objects naming them so.
#[derive(Clone, Debug, Default)]
pub struct Styles {
    pub text: Vec<TextStyleDef>,
    pub dimension: Vec<DimensionStyleDef>,
}

/// The styles ready to go in (docs/adr/0183 §7; the web's `importedStyles`):
/// each the project's style of the same name (case aside) when it has one,
/// the file's values staying the objects' own; else a new style under a new
/// id. One the project may not keep (a name of Standart's, values out of
/// the rules) is none: its objects keep their look without a style.
#[derive(Debug, Default)]
struct ImportedStyles {
    /// The reader's id → the project's (none: no style).
    ids: HashMap<String, Option<String>>,
    text: Vec<TextStyleDef>,
    dimension: Vec<DimensionStyleDef>,
}

/// Two style names the same, case aside (the tables' rule).
fn same_name(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

impl ImportedStyles {
    fn new(settings: &ProjectSettings, styles: Styles) -> Self {
        let mut out = Self::default();
        for mut s in styles.text {
            let read = s.id.clone();
            let id = match settings
                .text_styles
                .iter()
                .find(|p| same_name(&p.name, &s.name))
            {
                Some(p) => Some(p.id.clone()),
                None => {
                    s.id = Uuid::now_v7().to_string();
                    let table: Vec<TextStyleDef> = settings
                        .text_styles
                        .iter()
                        .chain(&out.text)
                        .chain(std::iter::once(&s))
                        .cloned()
                        .collect();
                    text_styles_problem(&table).is_none().then(|| {
                        let id = s.id.clone();
                        out.text.push(s);
                        id
                    })
                }
            };
            out.ids.insert(read, id);
        }
        for mut s in styles.dimension {
            let read = s.id.clone();
            let id = match settings
                .dimension_styles
                .iter()
                .find(|p| same_name(&p.name, &s.name))
            {
                Some(p) => Some(p.id.clone()),
                None => {
                    s.id = Uuid::now_v7().to_string();
                    let table: Vec<DimensionStyleDef> = settings
                        .dimension_styles
                        .iter()
                        .chain(&out.dimension)
                        .chain(std::iter::once(&s))
                        .cloned()
                        .collect();
                    dimension_styles_problem(&table).is_none().then(|| {
                        let id = s.id.clone();
                        out.dimension.push(s);
                        id
                    })
                }
            };
            out.ids.insert(read, id);
        }
        out
    }

    /// An object naming the project's style, or none.
    fn restyle(&self, e: &mut Entity) {
        let to = |id: Option<String>| id.and_then(|id| self.ids.get(&id).cloned().flatten());
        match e {
            Entity::Text(t) => t.face.text_style = to(t.face.text_style.take()),
            Entity::Dimension(d) => d.look.dim_style = to(d.look.dim_style.take()),
            _ => {}
        }
    }

    /// The new styles into the project's tables: a setting, not an undo step
    /// (docs/adr/0183 §1); undoing the import leaves them, unused.
    fn add(&mut self, doc: &mut Document) -> (usize, usize) {
        let added = (self.text.len(), self.dimension.len());
        if added != (0, 0) {
            let mut settings = doc.settings().clone();
            settings.text_styles.append(&mut self.text);
            settings.dimension_styles.append(&mut self.dimension);
            doc.set_settings(settings);
        }
        added
    }
}

/// The blocks an import brings (docs/adr/0144 §5), ready to go in: each a
/// new id and a name the drawing does not have yet (`blocks::import_names`),
/// the inserts among their objects pointing at the new ids, and each object
/// on the drawing layer its source layer goes to, else on none of its own
/// (`""`, the block's: Patlat puts it on the insert's layer).
#[derive(Debug, Default)]
pub struct ImportedBlocks {
    defs: Vec<BlockDefinition>,
    ids: HashMap<BlockId, BlockId>,
    renamed: Vec<(String, String)>,
}

impl ImportedBlocks {
    fn new(
        doc: &Document,
        blocks: Vec<BlockDefinition>,
        targets: &HashMap<String, String>,
        styles: &ImportedStyles,
    ) -> Self {
        let names = import_names(
            doc.blocks().iter().map(|b| b.name.as_str()),
            blocks.iter().map(|b| b.name.as_str()),
        );
        let ids: HashMap<BlockId, BlockId> = blocks
            .iter()
            .map(|b| (b.id, BlockId(*Uuid::now_v7().as_bytes())))
            .collect();
        let mut renamed = Vec::new();
        let defs = blocks
            .into_iter()
            .zip(names)
            .map(|(mut b, name)| {
                if name != b.name {
                    renamed.push((std::mem::replace(&mut b.name, name), b.name.clone()));
                }
                b.id = ids[&b.id];
                for e in &mut b.entities {
                    point_at(e, &ids);
                    styles.restyle(e);
                    let base = e.base_mut();
                    base.layer_id = targets.get(&base.layer_id).cloned().unwrap_or_default();
                }
                b
            })
            .collect();
        Self { defs, ids, renamed }
    }

    /// The definitions into the drawing (inside the import's transaction).
    fn add(&mut self, doc: &mut Document) -> Result<(), String> {
        for d in std::mem::take(&mut self.defs) {
            doc.add_block(d).map_err(|r| {
                format!(
                    "Dosyanın blokları çizime uymuyor ({r}). Hiçbir şey eklenmedi; dosyayla birlikte bildirin."
                )
            })?;
        }
        Ok(())
    }
}

/// The first object placing a block the import did not bring (a reader never
/// makes one, and the drawing could not save it), in the web's words; `before`
/// objects were checked in earlier batches.
fn stray_insert(entities: &[Entity], known: &HashSet<BlockId>, before: usize) -> Option<String> {
    entities.iter().enumerate().find_map(|(i, e)| match e {
        Entity::Insert(e) if !known.contains(&e.block) => Some(format!(
            "Dosyadan okunan nesneler çizime uymuyor (İçe aktarılan nesne {} (insert) › blok: {} çizimde tanımlı değil). Hiçbir şey eklenmedi; dosyayla birlikte bildirin.",
            before + i + 1,
            e.block
        )),
        _ => None,
    })
}

/// An insert placed by its block's new id.
fn point_at(e: &mut Entity, ids: &HashMap<BlockId, BlockId>) {
    if let Entity::Insert(i) = e
        && let Some(&id) = ids.get(&i.block)
    {
        i.block = id;
    }
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
            fields,
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
            snap: None,
            fields: fields.clone(),
            service: None,
            feed: None,
            time: None,
            scenario: None,
            replaces: None,
        };
        doc.add_layer(new, parent.as_deref(), false)
            .map_err(|r| r.to_string())?;
        created.push(name.clone());
    }
    Ok(created)
}

/// `e` onto its target layer, or none when its layer is left out; an insert
/// placing its block by the id it has in the drawing.
fn retarget(
    mut e: Entity,
    targets: &HashMap<String, String>,
    ids: &HashMap<BlockId, BlockId>,
    styles: &ImportedStyles,
) -> Option<Entity> {
    let layer = targets.get(&e.base().layer_id)?.clone();
    e.base_mut().layer_id = layer;
    point_at(&mut e, ids);
    styles.restyle(&mut e);
    Some(e)
}

/// Everything an import chose into the drawing as ONE undo step: the new
/// layers, the block definitions (docs/adr/0144 §5), then the objects.
pub fn apply_import(
    doc: &mut Document,
    entities: Vec<Entity>,
    blocks: Vec<BlockDefinition>,
    plan: &ImportPlan,
) -> Result<Applied, String> {
    apply_styled_import(doc, entities, blocks, Styles::default(), plan)
}

/// As [`apply_import`], with the file's text and dimension styles (docs/adr/0183
/// §7): once the objects are in, the new ones join the project's tables.
pub fn apply_styled_import(
    doc: &mut Document,
    entities: Vec<Entity>,
    blocks: Vec<BlockDefinition>,
    styles: Styles,
    plan: &ImportPlan,
) -> Result<Applied, String> {
    let mut prepared = prepare(doc, plan)?;
    let mut styles = ImportedStyles::new(doc.settings(), styles);
    let mut imported = ImportedBlocks::new(doc, blocks, &prepared.targets, &styles);
    let count = imported.defs.len();
    let chosen: Vec<Entity> = entities
        .into_iter()
        .filter_map(|e| retarget(e, &prepared.targets, &imported.ids, &styles))
        .collect();
    let known: HashSet<BlockId> = imported.ids.values().copied().collect();
    if let Some(error) = stray_insert(&chosen, &known, 0) {
        return Err(error);
    }
    let mut created = Vec::new();
    let slots = doc.transact(&plan.label, |doc| {
        created = make_layers(doc, plan, &mut prepared)?;
        imported.add(doc)?;
        doc.add_many(chosen, &plan.label)
            .map_err(|e| format!("{e}. Hiçbir nesne eklenmedi."))
    })?;
    let styles = styles.add(doc);
    Ok(Applied {
        slots,
        created,
        blocks: count,
        renamed: imported.renamed,
        styles,
    })
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
    /// The reader's block id → the drawing's, and the drawing's ids an insert may place.
    ids: HashMap<BlockId, BlockId>,
    known: HashSet<BlockId>,
    /// The block definitions taken in, and the names changed on the way.
    pub blocks: usize,
    pub renamed: Vec<(String, String)>,
    /// The file's styles, the new ones added to the project once every object is in.
    styles: ImportedStyles,
    /// The text and dimension styles added (docs/adr/0183 §7).
    pub added_styles: (usize, usize),
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
    pub fn start(
        doc: &mut Document,
        blocks: Vec<BlockDefinition>,
        styles: Styles,
        plan: &ImportPlan,
    ) -> Result<Self, String> {
        let mut prepared = prepare(doc, plan)?;
        let styles = ImportedStyles::new(doc.settings(), styles);
        let mut imported = ImportedBlocks::new(doc, blocks, &prepared.targets, &styles);
        let count = imported.defs.len();
        let group = doc.begin_group(&plan.label);
        let made = doc.transact(&plan.label, |doc| {
            let created = make_layers(doc, plan, &mut prepared)?;
            imported.add(doc)?;
            Ok(created)
        });
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
            known: imported.ids.values().copied().collect(),
            ids: imported.ids,
            blocks: count,
            renamed: imported.renamed,
            styles,
            added_styles: (0, 0),
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
                if let Some(e) = retarget(e, &self.targets, &self.ids, &self.styles) {
                    batch.push(e);
                }
            }
            let last = self.seen >= self.total;
            if let Some(error) = stray_insert(&batch, &self.known, self.slots.len()) {
                self.cancel(doc);
                return Err(error);
            }
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
                self.added_styles = self.styles.add(doc);
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
            Entity::Insert(e) => p(&e.p) && fs(&[e.scale, e.rotation]),
            Entity::Image(e) => {
                let i = &e.image;
                p(&i.p)
                    && fs(&[i.width, i.height, i.rotation])
                    && i.clip.as_deref().is_none_or(ps)
                    && i.opacity.is_none_or(f64::is_finite)
            }
            Entity::Raster(e) => {
                fs(&e.raster.affine) && e.raster.opacity.is_none_or(f64::is_finite)
            }
            Entity::PointCloud(e) => kentos_native_application::edit::cloud_finite(&e.cloud),
            Entity::Leader(e) => ps(&e.pts) && fs(&[e.height, e.rotation]),
            Entity::Table(e) => {
                p(&e.p)
                    && fs(&[e.height, e.rotation])
                    && fs(&e.rows)
                    && fs(&e.columns)
                    && e.face.oblique.is_none_or(f64::is_finite)
            }
        }
}

/// What the import window says when [`unusable`] finds one (the web's words).
pub fn unusable_text(place: usize, kind: &str) -> String {
    format!(
        "Dosyadan okunan nesneler çizime uymuyor (İçe aktarılan nesne {place} ({kind}): sonlu olmayan bir sayı taşıyor). Hiçbir şey eklenmedi; dosyayla birlikte bildirin."
    )
}

/// The first block definition with a number that is not finite (its base
/// point, or one of its objects'), said as the import window says it: the
/// web checks an import's definitions as a file's (`readBlockDefinitions`).
pub fn unusable_block(blocks: &[BlockDefinition]) -> Option<String> {
    blocks.iter().enumerate().find_map(|(i, b)| {
        let what = if b.base.x.is_finite() && b.base.y.is_finite() {
            let (place, kind) = unusable(&b.entities)?;
            format!("nesne {place} ({kind})")
        } else {
            "taban noktası".to_owned()
        };
        Some(format!(
            "Dosyadan okunan bloklar çizime uymuyor (Blok {} (“{}”) › {what}: sonlu olmayan bir sayı taşıyor). Hiçbir şey eklenmedi; dosyayla birlikte bildirin.",
            i + 1,
            b.name
        ))
    })
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
            parts: None,
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
        let applied = apply_import(&mut doc, objects, Vec::new(), &plan).expect("applied");
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
                        fields: Vec::new(),
                    },
                ),
                (
                    "YOL".into(),
                    LayerTarget::New {
                        name: "YOL".into(),
                        style: Box::new(style("fg")),
                        visible: false,
                        locked: true,
                        fields: Vec::new(),
                    },
                ),
            ],
            group: Some("plan.dxf".into()),
        };
        let applied = apply_import(
            &mut doc,
            vec![line("PARSEL"), line("YOL"), line("DEFPOINTS")],
            Vec::new(),
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
            Vec::new(),
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
            Vec::new(),
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
            Vec::new(),
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
                            fields: Vec::new(),
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

    /// A reader's block: its id, name, one line and, when given, an insert of another.
    fn block(n: u8, name: &str, inner: Option<u8>) -> BlockDefinition {
        // On the block's layer (a DXF's 0), and on a source layer the import may not bring.
        let mut entities = vec![line(""), line("DETAY")];
        if let Some(k) = inner {
            entities.push(insert("0", k));
        }
        // Numbered 1, 2, … as the reader numbers a definition's objects.
        for (k, e) in (1..).zip(&mut entities) {
            e.base_mut().id = k;
        }
        BlockDefinition {
            id: BlockId((u128::from(n)).to_be_bytes()),
            name: name.into(),
            base: Vec2 { x: 0.0, y: 0.0 },
            entities,
            attributes: Vec::new(),
            description: None,
        }
    }

    fn insert(layer: &str, n: u8) -> Entity {
        Entity::Insert(kentos_contracts::InsertEntity {
            base: base(layer),
            block: BlockId((u128::from(n)).to_be_bytes()),
            p: Vec2 { x: 5.0, y: 5.0 },
            scale: 1.0,
            rotation: 0.0,
            mirror: false,
        })
    }

    /// The file's blocks go in with its objects, in the same step (docs/adr/0144 §5): each
    /// under a new id and a name the drawing does not have yet, the inserts (the drawing's
    /// and those inside definitions) placing the new ids.
    #[test]
    fn blocks_go_in_with_the_objects_under_free_names() {
        let mut doc = doc();
        doc.add_block(BlockDefinition {
            name: "KAPI".into(),
            ..block(9, "", None)
        })
        .expect("the drawing's own");
        let plan = ImportPlan {
            label: "DXF: plan.dxf".into(),
            layers: vec![("0".into(), LayerTarget::Existing("a".into()))],
            group: None,
        };
        let blocks = vec![block(1, "No", None), block(2, "Kapı", Some(1))];
        let applied = apply_import(&mut doc, vec![insert("0", 2), line("0")], blocks, &plan)
            .expect("applied");
        assert_eq!(applied.blocks, 2);
        assert_eq!(
            applied.renamed,
            [("Kapı".to_owned(), "Kapı (2)".to_owned())]
        );
        let names: Vec<&str> = doc.blocks().iter().map(|b| b.name.as_str()).collect();
        assert_eq!(names, ["KAPI", "No", "Kapı (2)"]);
        let (no, kapi) = (doc.blocks()[1].id, doc.blocks()[2].id);
        assert!(
            ![no, kapi].contains(&BlockId((1u128).to_be_bytes())),
            "new ids"
        );
        let Some(Entity::Insert(placed)) = doc.get(applied.slots[0]) else {
            panic!("an insert")
        };
        assert_eq!(placed.block, kapi);
        let Entity::Insert(inner) = &doc.blocks()[2].entities[2] else {
            panic!("the nested insert")
        };
        assert_eq!(inner.block, no);
        // The objects' layers: the block's stays none, an unbrought one none, a brought one the drawing's.
        let layers: Vec<&str> = doc.blocks()[2]
            .entities
            .iter()
            .map(|e| e.base().layer_id.as_str())
            .collect();
        assert_eq!(layers, ["", "", "a"]);
        // One step: undo takes the objects and the definitions.
        assert_eq!(doc.undo().as_deref(), Some("DXF: plan.dxf"));
        assert_eq!(doc.len(), 0);
        assert_eq!(doc.blocks().len(), 1);
        assert_eq!(doc.redo().as_deref(), Some("DXF: plan.dxf"));
        assert_eq!(doc.blocks().len(), 3);
    }

    /// A large import's blocks go in when it starts, inside its one step; Durdur takes them back.
    #[test]
    fn a_large_import_adds_its_blocks_first_and_durdur_takes_them_back() {
        let mut doc = doc();
        let plan = ImportPlan {
            label: "DXF: büyük.dxf".into(),
            layers: vec![("0".into(), LayerTarget::Existing("a".into()))],
            group: None,
        };
        let mut work = Progressive::start(
            &mut doc,
            vec![block(1, "No", None)],
            Styles::default(),
            &plan,
        )
        .expect("started");
        assert_eq!(work.blocks, 1);
        let id = doc.blocks()[0].id;
        work.feed(vec![insert("0", 1); 3]);
        while !work
            .step(&mut doc, std::time::Duration::from_secs(1))
            .expect("steps")
        {}
        assert!(
            doc.entities()
                .all(|e| matches!(e, Entity::Insert(i) if i.block == id))
        );
        assert_eq!(doc.len(), 3);
        assert_eq!(doc.undo().as_deref(), Some("DXF: büyük.dxf"));
        assert!(doc.blocks().is_empty());

        let mut stopped = Progressive::start(
            &mut doc,
            vec![block(1, "No", None)],
            Styles::default(),
            &plan,
        )
        .expect("started");
        stopped.feed(vec![insert("0", 1)]);
        stopped.stop(&mut doc);
        assert!(doc.blocks().is_empty());
        assert_eq!(doc.len(), 0);
    }

    /// An insert of a block the file did not bring changes nothing, and is said.
    #[test]
    fn an_insert_of_a_block_the_file_did_not_bring_is_refused() {
        let mut doc = doc();
        let plan = ImportPlan {
            label: "DXF: plan.dxf".into(),
            layers: vec![("0".into(), LayerTarget::Existing("a".into()))],
            group: None,
        };
        let refused = apply_import(
            &mut doc,
            vec![line("0"), insert("0", 1), insert("0", 7)],
            vec![block(1, "No", None)],
            &plan,
        )
        .expect_err("refused");
        assert!(
            refused.contains(
                "(İçe aktarılan nesne 3 (insert) › blok: 00000000-0000-0000-0000-000000000007 çizimde tanımlı değil)"
            ),
            "{refused}"
        );
        assert!(doc.blocks().is_empty());
        assert_eq!(doc.len(), 0);
        assert!(!doc.can_undo());

        let mut work = Progressive::start(
            &mut doc,
            vec![block(1, "No", None)],
            Styles::default(),
            &plan,
        )
        .expect("started");
        work.feed(vec![insert("0", 1), insert("0", 7)]);
        let stopped = work.step(&mut doc, std::time::Duration::from_secs(1));
        assert!(
            stopped
                .expect_err("refused")
                .contains("(İçe aktarılan nesne 2 (insert)"),
        );
        assert!(doc.blocks().is_empty());
        assert_eq!(doc.len(), 0);
        assert!(!doc.can_undo());
    }

    #[test]
    fn a_block_with_a_number_that_is_not_finite_is_found() {
        let good = block(1, "No", None);
        assert_eq!(unusable_block(std::slice::from_ref(&good)), None);
        let mut far = block(2, "Uzak", None);
        far.base.x = f64::INFINITY;
        let mut bad = block(3, "Bozuk", None);
        bad.entities.push(point("0", f64::NAN, 0.0));
        assert!(
            unusable_block(&[good.clone(), far])
                .expect("found")
                .contains("(Blok 2 (“Uzak”) › taban noktası: sonlu olmayan bir sayı taşıyor)")
        );
        assert!(
            unusable_block(&[good, bad])
                .expect("found")
                .contains("(Blok 2 (“Bozuk”) › nesne 3 (point): sonlu olmayan bir sayı taşıyor)")
        );
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

    fn text_in(layer: &str, style: &str) -> Entity {
        Entity::Text(kentos_contracts::TextEntity {
            base: base(layer),
            p: Vec2 { x: 0.0, y: 0.0 },
            text: "Ada 101".into(),
            height: 2.0,
            rotation: 0.0,
            align: None,
            width_factor: None,
            mask: false,
            label_of: None,
            label_scale: None,
            paragraph: Default::default(),
            face: kentos_contracts::TextFace {
                text_style: Some(style.into()),
                font: Some(kentos_contracts::DrawingFont::Arimo),
                bold: true,
                ..Default::default()
            },
            path: None,
        })
    }

    fn text_style(id: &str, name: &str) -> TextStyleDef {
        TextStyleDef {
            id: id.into(),
            name: name.into(),
            font: kentos_contracts::DrawingFont::Arimo,
            bold: true,
            italic: false,
            oblique: None,
            height: Some(3.0),
            width_factor: None,
            font_file: Some("arialbd.ttf".into()),
        }
    }

    /// A file's styles (docs/adr/0183 §7): one of a name the project has
    /// (case aside) is the project's, the file's values staying the objects'
    /// own; a new one joins the project under a new id once the objects are
    /// in; one the project may not keep (Standart's name) is none, its objects
    /// keeping their look. A definition's objects follow them too. Undo takes
    /// the objects, not the styles (a setting).
    #[test]
    fn a_files_styles_join_the_project_or_name_its_own() {
        let mut doc = doc();
        let mut settings = doc.settings().clone();
        settings.text_styles = vec![text_style("0192f1a0-0000-7000-8000-000000000001", "Ada no")];
        doc.set_settings(settings);
        let plan = ImportPlan {
            label: "DXF: stiller.dxf".into(),
            layers: vec![("0".into(), LayerTarget::Existing("a".into()))],
            group: None,
        };
        let mut def = block(1, "Pafta", None);
        def.entities.push(text_in("", "dxf-text-2"));
        let styles = Styles {
            text: vec![
                text_style("dxf-text-1", "ADA NO"),
                text_style("dxf-text-2", "Yol adı"),
                text_style("dxf-text-3", "Standart"),
            ],
            dimension: Vec::new(),
        };
        let applied = apply_styled_import(
            &mut doc,
            vec![
                text_in("0", "dxf-text-1"),
                text_in("0", "dxf-text-2"),
                text_in("0", "dxf-text-3"),
            ],
            vec![def],
            styles,
            &plan,
        )
        .expect("applied");
        assert_eq!(applied.styles, (1, 0));
        let table = &doc.settings().text_styles;
        assert_eq!(
            table.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
            ["Ada no", "Yol adı"]
        );
        let new = table[1].id.clone();
        assert!(Uuid::parse_str(&new).is_ok(), "{new}");
        let named: Vec<Option<String>> = doc
            .entities()
            .map(|e| match e {
                Entity::Text(t) => t.face.text_style.clone(),
                _ => None,
            })
            .collect();
        assert_eq!(
            named,
            [
                Some("0192f1a0-0000-7000-8000-000000000001".to_owned()),
                Some(new.clone()),
                None
            ]
        );
        // Each keeps the file's look: the face without the style is still the text's.
        assert!(
            doc.entities()
                .all(|e| matches!(e, Entity::Text(t) if t.face.bold))
        );
        let inside = doc.blocks()[0].entities.iter().find_map(|e| match e {
            Entity::Text(t) => t.face.text_style.clone(),
            _ => None,
        });
        assert_eq!(inside, Some(new));
        assert_eq!(doc.undo().as_deref(), Some("DXF: stiller.dxf"));
        assert_eq!(doc.len(), 0);
        assert_eq!(doc.settings().text_styles.len(), 2);
    }
}
