//! `cad.entities.copy` v1 and `cad.entities.paste` v1 (docs/adr/0309; AutoCAD's COPYCLIP, COPYBASE, PASTECLIP,
//! PASTEORIG and PASTEBLOCK): a copy of objects with the layers, blocks, styles and library items they use, and that
//! copy pasted into a drawing (the same or another) as one undo step “Yapıştır”.
//!
//! The rules are the exchange's (`kentos_domain::exchange`, docs/adr/0193, 0199 §7): a copy is the selection's
//! drawing ([`exchange::selection`]); a paste takes each of its layers with its objects ([`exchange::layer_take`]: the
//! layer by path, a met block name is ours, styles by name, links that leave the copy drop), or the whole copy as
//! one block ([`exchange::file_block`]). The shared cases are `fixtures/commands/v1/cad.entities.copy.json` and
//! `.paste.json`.

use std::collections::{HashMap, HashSet};

use kentos_contracts::{
    BlockId, Clip, CommandResult, CommandWarning, DocumentSnapshotV2, DrawingUnit, EntitiesCopy,
    EntitiesPaste, EntitiesPastePlan, EntitiesPasted, Entity, EntityId, LayerNode, LayerNodeType,
    PasteReport, TableSource, Vec2 as Wire,
};
use kentos_domain::exchange::{self, block_key, fold};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::entity::entity_bounds;
use kentos_geometry_core::geom::affine::translation;
use kentos_geometry_core::ops::transform::transform_shape;
use kentos_geometry_core::store::Store;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::exchange_write::{Counts, put_blocks, put_nodes};
use crate::geometry::{shape, with_shape};

/// The undo step's name.
pub const LABEL: &str = "Yapıştır";
/// The copy's drawing's name.
const CLIP_NAME: &str = "Pano";

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

/// The layer tree's nodes with their paths (“Kadastro / Bina”), in the tree's order.
fn walk(nodes: &[LayerNode], above: &str, out: &mut Vec<(String, String, LayerNodeType)>) {
    for n in nodes {
        let path = if above.is_empty() {
            n.name.clone()
        } else {
            format!("{above} / {}", n.name)
        };
        out.push((n.id.clone(), path.clone(), n.kind));
        walk(&n.children, &path, out);
    }
}

fn paths(nodes: &[LayerNode]) -> Vec<(String, String, LayerNodeType)> {
    let mut out = Vec::new();
    walk(nodes, "", &mut out);
    out
}

/// A unit's short name, as Proje ayarları shows it.
fn unit_name(u: Option<DrawingUnit>) -> String {
    serde_json::to_value(u.unwrap_or_default())
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| "m".into())
}

// ── Kopyala ─────────────────────────────────────────────────────────────

/// The links of a copied object to objects not copied with it dropped, as the exchange's selection drops them
/// (docs/adr/0193 §1): a linked label's object, an associative hatch's boundaries, a schedule's objects.
fn drop_links(e: &mut Entity, keep: &HashSet<EntityId>) {
    match e {
        Entity::Text(t) => {
            if t.label_of.is_some_and(|u| !keep.contains(&u)) {
                t.label_of = None;
                t.label_scale = None;
            }
        }
        Entity::Hatch(h) => {
            let kept = h
                .assoc
                .as_ref()
                .is_none_or(|a| a.objects().all(|u| keep.contains(u)));
            if !kept {
                h.assoc = None;
            }
        }
        Entity::Table(t) => {
            if let Some(
                TableSource::Coordinates { objects }
                | TableSource::Areas { objects }
                | TableSource::Attributes { objects },
            ) = &mut t.source
            {
                objects.retain(|u| keep.contains(u));
                if objects.is_empty() {
                    t.source = None;
                }
            }
        }
        _ => {}
    }
}

/// The lower left of these objects' extent, each by its own box as the core measures it (an insert by its point):
/// the exchange's `extent_corner` over typed objects.
fn corner(objects: &[Entity]) -> Option<(f64, f64)> {
    let mut corner: Option<(f64, f64)> = None;
    for e in objects {
        let b = entity_bounds(&shape(e));
        if !(b.min_x.is_finite() && b.min_y.is_finite()) {
            continue;
        }
        corner = Some(match corner {
            None => (b.min_x, b.min_y),
            Some((x, y)) => (x.min(b.min_x), y.min(b.min_y)),
        });
    }
    corner
}

fn copy_check(doc: &Document, input: &EntitiesCopy) -> Result<Clip, Stop> {
    checks::uids(&input.uids, "Kopyalanacak nesne verilmedi.")?;
    if let Some(base) = input.base {
        checks::point(base, "Taban noktasının", "base")?;
    }
    let found = checks::named(doc, &input.uids)?;
    // The exchange's rules prune the layer tree, keep the placed blocks and the library items by what the objects
    // name; a representative of each layer, symbol, picture and block they use tells them as much as all of them,
    // and costs no JSON of the rest (docs/adr/0309 §6). The objects themselves stay typed.
    let chosen: HashSet<Slot> = found.iter().map(|(_, slot, _, _)| *slot).collect();
    let mut part = doc.to_snapshot_v2_of(|s| chosen.contains(&s));
    let objects = std::mem::take(&mut part.entities);
    let uids = std::mem::take(&mut part.uids);
    let mut seen: HashSet<(String, Option<String>, String)> = HashSet::new();
    for (i, (e, u)) in objects.iter().zip(&uids).enumerate() {
        let what = match e {
            Entity::Insert(ins) => Uuid::from_bytes(ins.block.0).to_string(),
            Entity::Image(_) | Entity::Raster(_) => i.to_string(),
            _ => String::new(),
        };
        let key = (e.base().layer_id.clone(), e.base().symbol.clone(), what);
        if seen.insert(key) {
            part.entities.push(e.clone());
            part.uids.push(*u);
        }
    }
    let shown = part.uids.clone();
    let mut drawing = exchange::selection(&part, &shown, CLIP_NAME)
        .map_err(|why| refuse(codes::ENTITY_NOT_FOUND, why, "uids"))?;
    let keep: HashSet<EntityId> = uids.iter().copied().collect();
    drawing.entities = objects
        .into_iter()
        .enumerate()
        .map(|(i, mut e)| {
            e.base_mut().id = u32::try_from(i + 1).unwrap_or(u32::MAX);
            drop_links(&mut e, &keep);
            e
        })
        .collect();
    drawing.uids = uids;
    let base = match input.base {
        Some(b) => b,
        None => {
            let (x, y) = corner(&drawing.entities).unwrap_or((0.0, 0.0));
            Wire { x, y }
        }
    };
    // The styles the objects and the blocks' objects name, not the drawing's others.
    let mut named: HashSet<String> = HashSet::new();
    let pieces = drawing
        .entities
        .iter()
        .chain(drawing.blocks.iter().flat_map(|b| b.entities.iter()));
    for e in pieces {
        let style = match e {
            Entity::Text(t) => t.face.text_style.as_ref(),
            Entity::Dimension(d) => d.look.dim_style.as_ref(),
            _ => None,
        };
        if let Some(id) = style {
            named.insert(id.clone());
        }
    }
    let text_styles = drawing
        .settings
        .text_styles
        .into_iter()
        .filter(|s| named.contains(&s.id))
        .collect();
    let dimension_styles = drawing
        .settings
        .dimension_styles
        .into_iter()
        .filter(|s| named.contains(&s.id))
        .collect();
    Ok(Clip {
        objects: drawing.entities,
        uids: drawing
            .uids
            .iter()
            .map(|u| Uuid::from_bytes(u.0).to_string())
            .collect(),
        layers: drawing.layers,
        blocks: drawing.blocks,
        text_styles,
        dimension_styles,
        library: (!drawing.styles.items.is_empty()).then_some(drawing.styles),
        unit: doc.settings().drawing_unit,
        base,
    })
}

/// `cad.entities.copy` v1: Panoya kopyala and Taban noktasıyla kopyala. Writes nothing.
pub mod entities_copy {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesCopy) -> CommandResult<()> {
        match copy_check(cx.doc, input) {
            Ok(_) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    /// The copy; the same answer as [`execute`].
    pub fn plan(cx: &ExecutionContext<'_>, input: &EntitiesCopy) -> CommandResult<Clip> {
        answer(cx.doc, input)
    }

    pub fn execute(cx: &mut ExecutionContext<'_>, input: EntitiesCopy) -> CommandResult<Clip> {
        answer(cx.doc, &input)
    }

    /// The copy of `input`'s objects in `doc`.
    pub fn answer(doc: &Document, input: &EntitiesCopy) -> CommandResult<Clip> {
        match copy_check(doc, input) {
            Ok(clip) => {
                // The clipboard takes no constraints (docs/adr/0337 §3): said when the objects have some.
                let ids: Vec<kentos_contracts::EntityId> = input
                    .uids
                    .iter()
                    .filter_map(|u| {
                        kentos_contracts::uuid_from_text(u).map(kentos_contracts::EntityId)
                    })
                    .collect();
                let held = crate::constraint_copy::any_constrained(doc, &ids);
                CommandResult::Completed {
                    output: clip,
                    warnings: (held > 0)
                        .then(|| crate::constraint_copy::not_copied(held))
                        .into_iter()
                        .collect(),
                }
            }
            Err(stop) => stop.into(),
        }
    }
}

// ── Yapıştır ────────────────────────────────────────────────────────────

/// What a paste writes: the drawing its layers, blocks and styles come from, the objects, the report.
struct Prepared {
    drawing: DocumentSnapshotV2,
    objects: Vec<Entity>,
    report: PasteReport,
    warnings: Vec<CommandWarning>,
}

/// The copy as a drawing of its own beside `ours` (the exchange's rules read two drawings), without its objects:
/// those go in as each rule needs them (docs/adr/0309 §6). With it the objects' ids.
fn theirs_of(
    ours: &DocumentSnapshotV2,
    clip: &Clip,
) -> Result<(DocumentSnapshotV2, Vec<EntityId>), Stop> {
    let mut theirs = ours.clone();
    theirs.name = CLIP_NAME.into();
    theirs.layers.clone_from(&clip.layers);
    let uids = clip
        .uids
        .iter()
        .map(|u| Uuid::parse_str(u).map(|u| EntityId(*u.as_bytes())))
        .collect::<Result<_, _>>()
        .map_err(|_| {
            refuse(
                codes::INVALID_CLIP,
                "Pano içeriğinin bir kimliği geçerli bir UUID değil.",
                "clip.uids",
            )
        })?;
    theirs.blocks.clone_from(&clip.blocks);
    theirs.settings.text_styles.clone_from(&clip.text_styles);
    theirs
        .settings
        .dimension_styles
        .clone_from(&clip.dimension_styles);
    theirs.styles = clip.library.clone().unwrap_or_default();
    Ok((theirs, uids))
}

/// `theirs` with the copy's objects at `at` (indices into it), numbered by their place in the copy.
fn with_objects(
    theirs: &DocumentSnapshotV2,
    clip: &Clip,
    uids: &[EntityId],
    at: &[usize],
) -> DocumentSnapshotV2 {
    let mut out = theirs.clone();
    out.entities = at
        .iter()
        .map(|&i| {
            let mut e = clip.objects[i].clone();
            e.base_mut().id = u32::try_from(i + 1).unwrap_or(u32::MAX);
            e
        })
        .collect();
    out.uids = at.iter().map(|&i| uids[i]).collect();
    out
}

/// What tells a pasted object's mapping apart: its symbol, text style, dimension style, block, and for a picture
/// the object itself (its asset).
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Look {
    symbol: Option<String>,
    text_style: Option<String>,
    dim_style: Option<String>,
    block: Option<BlockId>,
    picture: Option<usize>,
}

fn text_style(e: &Entity) -> Option<&String> {
    match e {
        Entity::Text(t) => t.face.text_style.as_ref(),
        _ => None,
    }
}

fn dim_style(e: &Entity) -> Option<&String> {
    match e {
        Entity::Dimension(d) => d.look.dim_style.as_ref(),
        _ => None,
    }
}

fn look_of(e: &Entity, i: usize) -> Look {
    Look {
        symbol: e.base().symbol.clone(),
        text_style: text_style(e).cloned(),
        dim_style: dim_style(e).cloned(),
        block: match e {
            Entity::Insert(ins) => Some(ins.block),
            _ => None,
        },
        picture: matches!(e, Entity::Image(_) | Entity::Raster(_)).then_some(i),
    }
}

/// The exchange's mapping of one layer's objects (`exchange::layer_take`'s `map_object`), learnt from what it did
/// to the representatives: the layer, the blocks and the styles; the links dropped.
#[derive(Default)]
struct Taught {
    layer: Option<String>,
    blocks: HashMap<BlockId, BlockId>,
    text_styles: HashMap<String, Option<String>>,
    dim_styles: HashMap<String, Option<String>>,
    /// The pictures, each its own representative, by their places in the copy.
    pictures: HashMap<usize, Entity>,
}

impl Taught {
    /// From the representatives (their places in the copy and themselves) and what the exchange made of them.
    fn learn<'a>(pairs: impl Iterator<Item = ((usize, &'a Entity), &'a Entity)>) -> Self {
        let mut t = Self::default();
        for ((i, from), to) in pairs {
            t.layer.get_or_insert_with(|| to.base().layer_id.clone());
            if let (Entity::Insert(a), Entity::Insert(b)) = (from, to) {
                t.blocks.insert(a.block, b.block);
            }
            if let Some(id) = text_style(from) {
                t.text_styles.insert(id.clone(), text_style(to).cloned());
            }
            if let Some(id) = dim_style(from) {
                t.dim_styles.insert(id.clone(), dim_style(to).cloned());
            }
            if matches!(from, Entity::Image(_) | Entity::Raster(_)) {
                t.pictures.insert(i, to.clone());
            }
        }
        t
    }

    /// `e`, the copy's `i`th object, as the exchange maps it.
    fn apply(&self, i: usize, e: &Entity) -> Entity {
        if let Some(made) = self.pictures.get(&i) {
            return made.clone();
        }
        let mut e = e.clone();
        if let Some(layer) = &self.layer {
            e.base_mut().layer_id.clone_from(layer);
        }
        match &mut e {
            Entity::Insert(ins) => {
                if let Some(to) = self.blocks.get(&ins.block) {
                    ins.block = *to;
                }
            }
            Entity::Text(t) => {
                if let Some(id) = &t.face.text_style {
                    t.face.text_style = self.text_styles.get(id).cloned().flatten();
                }
                t.label_of = None;
                t.label_scale = None;
            }
            Entity::Dimension(d) => {
                if let Some(id) = &d.look.dim_style {
                    d.look.dim_style = self.dim_styles.get(id).cloned().flatten();
                }
            }
            Entity::Hatch(h) => h.assoc = None,
            Entity::Table(t) => t.source = None,
            _ => {}
        }
        e
    }
}

/// The checks of the copy itself.
fn clip_check(clip: &Clip) -> Result<(), Stop> {
    if clip.objects.is_empty() {
        return Err(refuse(
            codes::NO_ENTITIES,
            "Pano boş: yapıştırılacak nesne yok. Önce nesneleri kopyalayın.",
            "clip.objects",
        ));
    }
    if clip.uids.len() != clip.objects.len() {
        return Err(refuse(
            codes::INVALID_CLIP,
            format!(
                "Pano içeriğinde {} nesne var ama {} kimlik; her nesnenin bir kimliği olmalı.",
                clip.objects.len(),
                clip.uids.len()
            ),
            "clip.uids",
        ));
    }
    let layers: HashSet<String> = paths(&clip.layers)
        .into_iter()
        .filter(|(_, _, kind)| *kind == LayerNodeType::Layer)
        .map(|(id, _, _)| id)
        .collect();
    let blocks: HashSet<_> = clip.blocks.iter().map(|b| b.id).collect();
    for (i, e) in clip.objects.iter().enumerate() {
        if !layers.contains(&e.base().layer_id) {
            return Err(refuse(
                codes::INVALID_CLIP,
                format!(
                    "Pano içeriğinin {}. nesnesinin katmanı (“{}”) içinde yok.",
                    i + 1,
                    e.base().layer_id
                ),
                &format!("clip.objects[{i}].layerId"),
            ));
        }
        if let Entity::Insert(ins) = e
            && !blocks.contains(&ins.block)
        {
            return Err(refuse(
                codes::INVALID_CLIP,
                format!("Pano içeriğinin {}. nesnesinin bloğu içinde yok.", i + 1),
                &format!("clip.objects[{i}].block"),
            ));
        }
    }
    Ok(())
}

/// What the paste of `input` writes into `doc`, checked in the contract's order.
fn paste_check(doc: &Document, input: &EntitiesPaste) -> Result<Prepared, Stop> {
    let clip = &input.clip;
    clip_check(clip)?;
    checks::point(clip.base, "Taban noktasının", "clip.base")?;
    if let Some(at) = input.at {
        checks::point(at, "Yapıştırma noktasının", "at")?;
    }
    let name = match &input.as_block {
        Some(n) if n.trim().is_empty() => {
            return Err(refuse(
                codes::EMPTY_NAME,
                "Bloğun adı boş; bir ad verin.",
                "asBlock",
            ));
        }
        Some(n) => Some(n.trim().to_owned()),
        None => None,
    };
    checks::revision(doc, input.expected_revision.as_deref())?;
    // The exchange's rules read none of our objects (docs/adr/0309 §6).
    let ours = doc.to_snapshot_v2_of(|_| false);
    let (theirs, uids) = theirs_of(&ours, clip)?;
    let at = input.at.unwrap_or(clip.base);
    let (drawing, objects, block) = match name {
        Some(name) => {
            let id = BlockId(*Uuid::now_v7().as_bytes());
            let every: Vec<usize> = (0..clip.objects.len()).collect();
            let theirs = with_objects(&theirs, clip, &uids, &every);
            let made = exchange::file_block(&ours, &theirs, &name, id)
                .map_err(|why| refuse(codes::INVALID_CLIP, why, "clip"))?;
            // The block's base is its objects' lower left; the copy's base point lands at `at`.
            let (cx, cy) = corner(&clip.objects).unwrap_or((clip.base.x, clip.base.y));
            let p = Wire {
                x: at.x + (cx - clip.base.x),
                y: at.y + (cy - clip.base.y),
            };
            let insert: Entity = serde_json::from_value(serde_json::json!({
                "kind": "insert", "id": 0, "layerId": doc.layers().active(), "attrs": {},
                "block": Uuid::from_bytes(id.0).to_string(), "p": p, "scale": 1, "rotation": 0
            }))
            .map_err(|e| refuse(codes::INVALID_CLIP, e.to_string(), "clip"))?;
            (made.drawing, vec![insert], Some(made.name))
        }
        None => {
            // Each layer of the copy with its objects, in the order its objects come; the objects back in the
            // copy's order.
            let tree = paths(&clip.layers);
            let mut order: Vec<String> = Vec::new();
            let mut on: HashMap<String, Vec<usize>> = HashMap::new();
            for (i, e) in clip.objects.iter().enumerate() {
                let id = &e.base().layer_id;
                if !on.contains_key(id) {
                    order.push(id.clone());
                }
                on.entry(id.clone()).or_default().push(i);
            }
            let mut running = ours.clone();
            let mut placed: Vec<Option<Entity>> = vec![None; clip.objects.len()];
            for id in &order {
                let Some((_, path, _)) = tree.iter().find(|(i, _, _)| i == id) else {
                    continue;
                };
                // A representative of each symbol, style, block and picture on the layer goes through the
                // exchange's rule; what it did to them it does to the others (docs/adr/0309 §6).
                let slots = &on[id];
                let mut seen: HashSet<Look> = HashSet::new();
                let reps: Vec<usize> = slots
                    .iter()
                    .copied()
                    .filter(|&i| seen.insert(look_of(&clip.objects[i], i)))
                    .collect();
                let sub = with_objects(&theirs, clip, &uids, &reps);
                let made = exchange::layer_take(&running, &sub, path)
                    .map_err(|why| refuse(codes::NOT_A_LAYER, why, "clip.layers"))?;
                let taught = Taught::learn(
                    reps.iter()
                        .map(|&i| (i, &clip.objects[i]))
                        .zip(&made.objects),
                );
                for &i in slots {
                    placed[i] = Some(taught.apply(i, &clip.objects[i]));
                }
                running = made.drawing;
            }
            let d = translation(at.x - clip.base.x, at.y - clip.base.y);
            let objects = placed
                .into_iter()
                .flatten()
                .filter_map(|e| with_shape(&e, transform_shape(&shape(&e), &d)))
                .collect();
            (running, objects, None)
        }
    };
    // What came in besides the objects.
    let before: HashSet<String> = paths(&ours.layers).into_iter().map(|(_, p, _)| p).collect();
    let layers = paths(&drawing.layers)
        .into_iter()
        .map(|(_, p, _)| p)
        .filter(|p| !before.contains(p))
        .collect();
    let our_blocks: HashSet<String> = ours.blocks.iter().map(|b| block_key(&b.name)).collect();
    let blocks = drawing
        .blocks
        .iter()
        .filter(|b| !our_blocks.contains(&block_key(&b.name)))
        .filter(|b| block.as_deref() != Some(b.name.as_str()))
        .map(|b| b.name.clone())
        .collect();
    let kept = clip
        .blocks
        .iter()
        .filter(|b| our_blocks.contains(&block_key(&b.name)))
        .map(|b| b.name.clone())
        .collect();
    let our_styles: HashSet<String> = ours
        .settings
        .text_styles
        .iter()
        .map(|s| fold(&s.name))
        .chain(ours.settings.dimension_styles.iter().map(|s| fold(&s.name)))
        .collect();
    let styles = drawing
        .settings
        .text_styles
        .iter()
        .map(|s| &s.name)
        .chain(drawing.settings.dimension_styles.iter().map(|s| &s.name))
        .filter(|n| !our_styles.contains(&fold(n)))
        .cloned()
        .collect();
    // A layer of ours that is locked takes nothing; a hidden one is said.
    let layer_tree = doc.layers();
    let mut warnings = Vec::new();
    let mut hidden: Vec<String> = Vec::new();
    for e in &objects {
        let id = &e.base().layer_id;
        if layer_tree.get(id).is_some() {
            if layer_tree.is_locked(id) {
                return Err(refuse(
                    codes::LAYER_LOCKED,
                    format!(
                        "“{}” katmanı kilitli; yapıştırılamadı. Kilidi Katmanlar panelinden açın.",
                        layer_tree.path(id)
                    ),
                    "clip",
                ));
            }
            if !layer_tree.is_visible(id) && !hidden.contains(id) {
                hidden.push(id.clone());
            }
        }
    }
    for id in &hidden {
        warnings.push(CommandWarning {
            code: codes::LAYER_HIDDEN.into(),
            message: format!(
                "“{}” katmanı gizli; yapıştırılan nesneler görünmeyecek.",
                layer_tree.path(id)
            ),
            path: Some("clip".into()),
        });
    }
    let (from, to) = (clip.unit, doc.settings().drawing_unit);
    if from.unwrap_or_default() != to.unwrap_or_default() {
        warnings.push(CommandWarning {
            code: codes::UNITS_DIFFER.into(),
            message: format!(
                "Kaynak çizimin birimi {}, bu çizimin {}; nesneler gerçek boylarıyla yapıştırıldı.",
                unit_name(from),
                unit_name(to)
            ),
            path: Some("clip.unit".into()),
        });
    }
    Ok(Prepared {
        drawing,
        objects,
        report: PasteReport {
            layers,
            blocks,
            kept,
            styles,
            block,
        },
        warnings,
    })
}

/// Writes what `prepared` brings: the layers, the blocks, the styles and library items, the objects.
fn write(doc: &mut Document, prepared: Prepared) -> CommandResult<EntitiesPasted> {
    let Prepared {
        drawing,
        objects,
        report,
        warnings,
    } = prepared;
    doc.name_step(LABEL);
    let mut counts = Counts::default();
    if let Err(why) = put_nodes(doc, &drawing.layers, None, &mut counts, LABEL)
        .and_then(|()| put_blocks(doc, &drawing.blocks, &mut counts))
    {
        return refuse(codes::INVALID_CLIP, why, "clip").into();
    }
    let mut settings = doc.settings().clone();
    if settings.text_styles != drawing.settings.text_styles
        || settings.dimension_styles != drawing.settings.dimension_styles
    {
        settings
            .text_styles
            .clone_from(&drawing.settings.text_styles);
        settings
            .dimension_styles
            .clone_from(&drawing.settings.dimension_styles);
        doc.set_settings(settings);
    }
    if doc.styles() != &drawing.styles {
        doc.set_styles(drawing.styles);
    }
    let slots = match doc.add_many(objects, LABEL) {
        Ok(slots) => slots,
        Err(_) => {
            return refuse(
                codes::SLOTS_EXHAUSTED,
                "Çizimde yeni nesneye yer kalmadı.",
                "clip",
            )
            .into();
        }
    };
    let created = slots
        .iter()
        .filter_map(|s| doc.uid(*s))
        .map(|u| u.to_string())
        .collect();
    CommandResult::Completed {
        output: EntitiesPasted {
            created,
            report,
            revision: doc.revision().to_string(),
        },
        warnings,
    }
}

/// `cad.entities.paste` v1: Yapıştır, Özgün koordinatlara yapıştır, Blok olarak yapıştır.
pub mod entities_paste {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesPaste) -> CommandResult<()> {
        match paste_check(cx.doc, input) {
            Ok(p) => CommandResult::Completed {
                output: (),
                warnings: p.warnings,
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &EntitiesPaste,
    ) -> CommandResult<EntitiesPastePlan> {
        plan_in(cx.doc, cx.store, input)
    }

    /// [`plan`] over the document alone: the write on a copy, what it would make.
    pub fn plan_in(
        doc: &Document,
        _store: Option<&Store>,
        input: &EntitiesPaste,
    ) -> CommandResult<EntitiesPastePlan> {
        let prepared = match paste_check(doc, input) {
            Ok(p) => p,
            Err(stop) => return stop.into(),
        };
        let mut copy = doc.clone();
        match write(&mut copy, prepared) {
            CommandResult::Completed { output, warnings } => {
                let created = output
                    .created
                    .iter()
                    .filter_map(|u| Uuid::parse_str(u).ok())
                    .filter_map(|u| copy.slot_of(u))
                    .filter_map(|s| copy.get(s).cloned())
                    .map(|mut e| {
                        e.base_mut().id = 0;
                        e
                    })
                    .collect();
                CommandResult::Completed {
                    output: EntitiesPastePlan {
                        created,
                        report: output.report,
                        revision: doc.revision().to_string(),
                    },
                    warnings,
                }
            }
            other => crate::trim::relay(other),
        }
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: EntitiesPaste,
    ) -> CommandResult<EntitiesPasted> {
        crate::call::recorded(
            cx,
            kentos_contracts::CAD_ENTITIES_PASTE,
            kentos_contracts::CAD_ENTITIES_PASTE_VERSION,
            input,
            |cx, input| match paste_check(cx.doc, &input) {
                Ok(prepared) => write(cx.doc, prepared),
                Err(stop) => stop.into(),
            },
        )
    }
}
