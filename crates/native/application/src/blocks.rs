//! Blocks between the contract and the shared core (docs/adr/0144): the
//! drawing's definitions as the core flattens them (`kentos_geometry_core::block`),
//! and an insert's placed pieces back as the drawing's own kind of objects,
//! for the hosts that draw objects rather than the store's records (the
//! desktop's selection highlight and plain scene).

use std::sync::Arc;

use kentos_commands::{BlockDefinition, Entity, EntityBase};
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::block::{Attribute, Blocks, Definition};
use kentos_geometry_core::entity::Entity as CoreEntity;
use kentos_geometry_core::store::Expanded;

use crate::geometry::{edit_geometry, entity_of, shape};

/// Room separators are root drawing boundaries. A loaded definition can retain their data,
/// but flattening it must not silently turn those boundaries into ordinary lines (ADR 0679).
pub(crate) fn contains_space_separator(doc: &kentos_domain::Document, id: kentos_commands::BlockId) -> bool {
    let mut pending = vec![id];
    let mut seen = std::collections::HashSet::new();
    while let Some(id) = pending.pop() {
        if !seen.insert(id) { continue; }
        let Some(block) = doc.block(id) else { continue; };
        for e in &block.entities {
            if kentos_domain::space_follow::separator_path(e).is_some() { return true; }
            if let Entity::Insert(i) = e { pending.push(i.block); }
        }
    }
    false
}

/// The box of what definition `block` draws placed at the origin at scale 1 with `values` (docs/adr/0726 §3): its
/// pieces only, not its base point; none for one that draws nothing. `[min x, min y, max x, max y]`, relative to the
/// base point.
pub fn definition_box_in(
    blocks: &Blocks,
    font: kentos_geometry_core::text::Font,
    block: kentos_commands::BlockId,
    values: Option<&std::collections::BTreeMap<String, kentos_commands::ParamValue>>,
) -> Option<[f64; 4]> {
    let insert = kentos_geometry_core::entity::Shape::Insert {
        params: values.and_then(kentos_formats::blocks::core_params),
        block: block.to_text(),
        p: kentos_geometry_core::Vec2::new(0.0, 0.0),
        scale: 1.0,
        y_scale: None,
        rotation: 0.0,
        mirror: None,
        attrs: None,
        places: None,
        clip: None,
    };
    let mut out: Option<[f64; 4]> = None;
    for piece in blocks.expand(&insert) {
        if kentos_geometry_core::block::shows_nothing(&piece.shape) {
            continue;
        }
        let b = kentos_geometry_core::entity::entity_bounds_in(&piece.shape, font);
        if ![b.min_x, b.min_y, b.max_x, b.max_y]
            .iter()
            .all(|v| v.is_finite())
        {
            continue;
        }
        out = Some(match out {
            None => [b.min_x, b.min_y, b.max_x, b.max_y],
            Some(a) => [
                a[0].min(b.min_x),
                a[1].min(b.min_y),
                a[2].max(b.max_x),
                a[3].max(b.max_y),
            ],
        });
    }
    out
}

/// A dynamic block's values as the core draws them (docs/adr/0318 §3); none when there are none.
pub fn core_params(
    values: &std::collections::BTreeMap<String, kentos_commands::ParamValue>,
) -> Option<kentos_geometry_core::entity::Params> {
    kentos_formats::blocks::core_params(values)
}

/// [`definition_box_in`] with the drawing's definitions and typeface.
pub fn definition_box(
    doc: &kentos_domain::Document,
    block: kentos_commands::BlockId,
    values: Option<&std::collections::BTreeMap<String, kentos_commands::ParamValue>>,
) -> Option<[f64; 4]> {
    definition_box_in(
        &core_blocks(doc.blocks()),
        crate::geometry::drawing_font(doc.settings().drawing_font),
        block,
        values,
    )
}

/// The drawing's definitions for the core.
pub fn core_blocks(blocks: &[Arc<BlockDefinition>]) -> Blocks {
    Blocks::new(
        blocks
            .iter()
            .map(|b| Definition {
                dynamic: b.dynamic.as_ref().and_then(kentos_formats::blocks::core_dynamic),
                underlay: kentos_formats::blocks::core_underlay(b),
                id: b.id.to_text(),
                base: kentos_geometry_core::Vec2::new(b.base.x, b.base.y),
                entities: b.entities.iter().map(core_entity).collect(),
                attributes: b
                    .attributes
                    .iter()
                    .map(|a| Attribute {
                        tag: a.tag.clone(),
                        value: a.value.clone().unwrap_or_default(),
                        p: kentos_geometry_core::Vec2::new(a.p.x, a.p.y),
                        height: a.height,
                        rotation: a.rotation,
                        align: a.align.and_then(|x| {
                            kentos_geometry_core::text::TextAlign::from_name(x.name())
                        }),
                        width_factor: a.width_factor,
                        invisible: a.options.invisible,
                        constant: a.options.constant,
                        movable: a.options.movable,
                        multiline: a.options.multiline,
                        box_width: a.options.box_width,
                    })
                    .collect(),
            })
            .collect(),
    )
}

/// An object as the core takes it: its shape, and the fields a block's
/// flattening and Patlat read (its layer, its own colour and line weight,
/// its attributes, label and symbol), as the web's JSON gives them.
pub fn core_entity(e: &Entity) -> CoreEntity {
    let base = e.base();
    let mut rest = vec![
        ("id".to_owned(), Json::Num(f64::from(base.id))),
        ("layerId".to_owned(), Json::Str(base.layer_id.clone())),
    ];
    if let Some(c) = &base.color {
        rest.push(("color".to_owned(), Json::Str(c.clone())));
    }
    rest.push((
        "attrs".to_owned(),
        Json::Obj(
            base.attrs
                .iter()
                .map(|(k, v)| (k.clone(), Json::Str(v.clone())))
                .collect(),
        ),
    ));
    if let Some(label) = &base.label {
        rest.push(("label".to_owned(), Json::Str(label.clone())));
    }
    if let Some(symbol) = &base.symbol {
        rest.push(("symbol".to_owned(), Json::Str(symbol.clone())));
    }
    if let Some(w) = base.line_weight {
        rest.push(("lineWeight".to_owned(), Json::Num(w)));
    }
    // Its own line type, scale and transparency and the parts that follow its layer: what a block's piece draws
    // with (docs/adr/0260 §1, 0312 §4).
    let look = &base.look;
    if let Some(t) = look.line_type {
        rest.push(("lineType".to_owned(), Json::Str(t.name().to_owned())));
    }
    if let Some(p) = &look.line_pattern {
        rest.push(("linePattern".to_owned(), Json::Str(p.clone())));
    }
    if let Some(s) = look.line_type_scale {
        rest.push(("lineTypeScale".to_owned(), Json::Num(s)));
    }
    if let Some(t) = look.transparency {
        rest.push(("transparency".to_owned(), Json::Num(t)));
    }
    if !look.by_layer.is_empty() {
        rest.push((
            "byLayer".to_owned(),
            Json::Arr(
                look.by_layer
                    .iter()
                    .map(|p| Json::Str(p.name().to_owned()))
                    .collect(),
            ),
        ));
    }
    // Keep the semantic break when point references read an opening's cached
    // jambs. It affects reference validity, never the rendered symbol (ADR0694).
    if let Entity::Opening(o) = e
        && let Some(broken) = o.opening.broken
    {
        rest.push(("broken".to_owned(), Json::Str(broken.name().to_owned())));
    }
    CoreEntity {
        shape: shape(e),
        rest,
    }
}

/// The words of `piece_gone` (docs/adr/0263 §3).
pub const NO_PIECE: &str = "Blok yolunda böyle bir parça yok: yolun bir sırası tanımın dışında, yol bir yerleştirmede bitiyor ya da yerleştirme olmayan bir nesneden geçiyor.";

/// Why a place on an object (or on a piece inside it) is not there (docs/adr/0263 §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaceMiss {
    /// The block path names no piece in the insert (`piece_gone`).
    Piece,
    /// The object, or the piece, has no such place (`no_such_place`).
    Place,
}

/// Where a place is now on `e`, or, with a block path, on the piece it names inside `e` (an insert), placed
/// with the drawing's definitions.
pub fn place_point(
    blocks: &[Arc<BlockDefinition>],
    e: &Entity,
    path: &[u32],
    place: kentos_commands::SubPlace,
) -> Result<kentos_geometry_core::Vec2, PlaceMiss> {
    use kentos_geometry_core::ops::subref::{piece, point};
    let core = kentos_domain::dimension_ties::core_place(place);
    let shape = core_entity(e);
    if path.is_empty() {
        return point(&shape, core).ok_or(PlaceMiss::Place);
    }
    let blocks = kentos_domain::dimension_ties::core_blocks(blocks);
    let piece = piece(&blocks, &shape, path).ok_or(PlaceMiss::Piece)?;
    point(&piece, core).ok_or(PlaceMiss::Place)
}

/// The shape a place is on: `e`'s own, or with a block path the piece it names inside `e` (an insert), placed
/// with the drawing's definitions; none for a path that names no piece (docs/adr/0270).
pub fn place_shape(
    blocks: &[Arc<BlockDefinition>],
    e: &Entity,
    path: &[u32],
) -> Option<kentos_geometry_core::entity::Shape> {
    let shape = core_entity(e);
    if path.is_empty() {
        return Some(shape.shape);
    }
    let blocks = kentos_domain::dimension_ties::core_blocks(blocks);
    kentos_geometry_core::ops::subref::piece(&blocks, &shape, path).map(|p| p.shape)
}

/// An insert's pieces as objects: each its placed shape, on the insert's
/// layer, with its own colour and line weight, else the insert's; the
/// insert's slot, no attributes. A piece the contract cannot hold, and an
/// attribute's text that shows nothing (§7), is left out.
pub fn piece_entities(insert: &Entity, x: &Expanded) -> Vec<Entity> {
    let own = insert.base();
    x.shapes
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            if kentos_geometry_core::block::shows_nothing(s) {
                return None;
            }
            let geometry = edit_geometry(s.clone())?;
            let base = EntityBase {
                look: Default::default(),
                id: own.id,
                layer_id: own.layer_id.clone(),
                color: x
                    .colors
                    .get(i)
                    .cloned()
                    .flatten()
                    .or_else(|| own.color.clone()),
                attrs: Default::default(),
                label: None,
                symbol: None,
                line_weight: x.weights.get(i).copied().flatten().or(own.line_weight),
                label_pins: Vec::new(),
                attachments: Vec::new(),
            };
            Some(entity_of(&geometry, base))
        })
        .collect()
}
