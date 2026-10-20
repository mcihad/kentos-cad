//! `cad.styles.impact` v1 (docs/adr/0314 §4; plan/cad.md CAD-STYLE-15): what changing a layer's look, a text or
//! dimension style or a line type would reach, read without writing. The rules are the look's own (docs/adr/0312
//! §4): an object takes from its layer what it has no value of its own for; a text and a dimension take each field
//! of their style that they still hold (`follow_text_style`); a block piece on a layer takes from it what its
//! `byLayer` names, and an insert shows it. A layer state that kept a look of its own brings it back when applied; a
//! sheet map that overrides the layer draws it otherwise (docs/adr/0314 §1). KentOS keeps no external reference.
//! The shared cases are `fixtures/commands/v1/cad.styles.impact.json` (`scripts/fixtures/style_impact_cases.py`).

use std::collections::HashSet;

use kentos_commands::{
    BlockId, CommandResult, Entity, IMPACT_UIDS, ImpactOwn, ImpactTarget, LayerNode, StyleImpact,
    StylesImpact,
};
use kentos_domain::{Document, Slot};
use serde_json::Value;

use crate::checks::{Stop, error};
use crate::{ExecutionContext, codes};

fn fail(code: &str, message: String) -> Stop {
    Stop::failed(error(code, message, Some("id".into())))
}

fn find<'a>(nodes: &'a [LayerNode], id: &str) -> Option<&'a LayerNode> {
    nodes.iter().find_map(|n| {
        if n.id == id {
            Some(n)
        } else {
            find(&n.children, id)
        }
    })
}

fn leaves(nodes: &[LayerNode], out: &mut Vec<LayerNode>) {
    for n in nodes {
        if n.children.is_empty() {
            out.push(n.clone());
        }
        leaves(&n.children, out);
    }
}

/// Counts by property, in a fixed order, those above zero.
struct Own(Vec<(&'static str, u32)>);

impl Own {
    fn new(names: &[&'static str]) -> Own {
        Own(names.iter().map(|n| (*n, 0)).collect())
    }
    fn add(&mut self, name: &str) {
        if let Some(c) = self.0.iter_mut().find(|(n, _)| *n == name) {
            c.1 += 1;
        }
    }
    fn list(self) -> Vec<ImpactOwn> {
        self.0
            .into_iter()
            .filter(|(_, c)| *c > 0)
            .map(|(p, c)| ImpactOwn {
                property: p.into(),
                count: c,
            })
            .collect()
    }
}

/// The object's own look values, by the contract's names.
fn own_values(e: &Entity) -> Vec<&'static str> {
    let b = e.base();
    let mut out = Vec::new();
    if b.color.is_some() {
        out.push("color");
    }
    if b.look.line_type.is_some() || b.look.line_pattern.is_some() {
        out.push("lineType");
    }
    if b.line_weight.is_some() {
        out.push("lineWeight");
    }
    if b.look.transparency.is_some() {
        out.push("transparency");
    }
    out
}

/// The definitions holding, nested, any of `ids`, and the drawing's inserts of them.
fn shown(doc: &Document, ids: &HashSet<BlockId>) -> u32 {
    let mut reach: HashSet<BlockId> = ids.clone();
    loop {
        let more: Vec<BlockId> = doc
            .blocks()
            .iter()
            .filter(|b| !reach.contains(&b.id))
            .filter(|b| {
                b.entities
                    .iter()
                    .any(|e| matches!(e, Entity::Insert(i) if reach.contains(&i.block)))
            })
            .map(|b| b.id)
            .collect();
        if more.is_empty() {
            break;
        }
        reach.extend(more);
    }
    doc.entities()
        .filter(|e| matches!(e, Entity::Insert(i) if reach.contains(&i.block)))
        .count() as u32
}

/// The sheet maps (by item id) whose overrides `pick` says touch it.
fn sheet_maps(
    doc: &Document,
    pick: impl Fn(&kentos_sheet::kinds::LayerOverride) -> bool,
) -> Vec<String> {
    let Some(sheets) = doc.sheets() else {
        return Vec::new();
    };
    let Ok((book, _)) = kentos_sheet::project::read_project_sheets(sheets) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for s in &book.sheets {
        for item in &s.items {
            if let kentos_sheet::kinds::ItemKind::Map(m) = &item.kind
                && m.layer_overrides.iter().any(&pick)
            {
                out.push(item.id.clone());
            }
        }
    }
    out
}

fn impact(doc: &Document, input: &StylesImpact) -> Result<StyleImpact, Stop> {
    let id = input.id.trim();
    if id.is_empty() {
        return Err(fail(
            codes::EMPTY_NAME,
            "Etkisi sorulan katmanın, stilin ya da çizgi tipinin kimliğini verin (id).".into(),
        ));
    }
    let mut out = StyleImpact {
        name: String::new(),
        follow: 0,
        uids: Vec::new(),
        own: Vec::new(),
        locked: 0,
        layers: Vec::new(),
        blocks: Vec::new(),
        block_pieces: 0,
        inserts: 0,
        layer_states: Vec::new(),
        sheet_maps: Vec::new(),
        external_references: 0,
        notes: Vec::new(),
        revision: doc.revision().to_string(),
    };
    let follow = |e: &Entity, out: &mut StyleImpact| {
        out.follow += 1;
        if out.uids.len() < IMPACT_UIDS
            && let Some(uid) = doc.uid(Slot(e.base().id))
        {
            out.uids.push(uid.to_string());
        }
        if doc.layers().is_locked(&e.base().layer_id) {
            out.locked += 1;
        }
    };
    let mut blocks: HashSet<BlockId> = HashSet::new();
    match input.target {
        ImpactTarget::Layer => {
            let layer = find(doc.layers().nodes(), id).ok_or_else(|| {
                fail(
                    codes::LAYER_NOT_FOUND,
                    format!("“{id}” kimlikli katman çizimde yok."),
                )
            })?;
            out.name = layer.name.clone();
            let mut own = Own::new(&["color", "lineType", "lineWeight", "transparency"]);
            for e in doc.by_layer(id) {
                let mine = own_values(e);
                mine.iter().for_each(|p| own.add(p));
                if mine.len() < 4 {
                    follow(e, &mut out);
                }
            }
            out.own = own.list();
            // A layer's look reaches objects on a locked layer too.
            out.locked = 0;
            for b in doc.blocks() {
                let pieces = b
                    .entities
                    .iter()
                    .filter(|e| e.base().layer_id == id && !e.base().look.by_layer.is_empty())
                    .count() as u32;
                if pieces > 0 {
                    out.block_pieces += pieces;
                    blocks.insert(b.id);
                    out.blocks.push(b.id.to_text());
                }
            }
            for s in &doc.settings().layer_states {
                if s.nodes
                    .iter()
                    .any(|n| n.node == id && n.style.as_ref().is_some_and(|st| *st != layer.style))
                {
                    out.layer_states.push(s.name.clone());
                }
            }
            out.sheet_maps = sheet_maps(doc, |o| o.layer == id);
        }
        ImpactTarget::LineType => {
            let def = doc
                .settings()
                .line_types
                .iter()
                .find(|t| kentos_commands::same_line_type_name(&t.name, id))
                .ok_or_else(|| {
                    fail(
                        codes::LINE_TYPE_NOT_FOUND,
                        format!("“{id}” adlı çizgi tipi projede yok."),
                    )
                })?;
            out.name = def.name.clone();
            let named =
                |p: Option<&str>| p.is_some_and(|p| kentos_commands::same_line_type_name(p, id));
            let mut all = Vec::new();
            leaves(doc.layers().nodes(), &mut all);
            let layers: Vec<&LayerNode> = all
                .iter()
                .filter(|l| named(l.style.line_pattern.as_deref()))
                .collect();
            out.layers = layers.iter().map(|l| l.id.clone()).collect();
            let mut own = Own::new(&["lineType"]);
            for e in doc.entities() {
                let b = e.base();
                let on_layer = layers.iter().any(|l| l.id == b.layer_id);
                if named(b.look.line_pattern.as_deref()) {
                    follow(e, &mut out);
                } else if on_layer {
                    if b.look.line_type.is_some() || b.look.line_pattern.is_some() {
                        own.add("lineType");
                    } else {
                        follow(e, &mut out);
                    }
                }
            }
            out.own = own.list();
            out.locked = 0;
            for b in doc.blocks() {
                let pieces = b
                    .entities
                    .iter()
                    .filter(|e| {
                        let base = e.base();
                        named(base.look.line_pattern.as_deref())
                            || (out.layers.contains(&base.layer_id)
                                && base
                                    .look
                                    .by_layer
                                    .contains(&kentos_commands::LookPart::LineType))
                    })
                    .count() as u32;
                if pieces > 0 {
                    out.block_pieces += pieces;
                    blocks.insert(b.id);
                    out.blocks.push(b.id.to_text());
                }
            }
            for s in &doc.settings().layer_states {
                if s.nodes.iter().any(|n| {
                    n.style
                        .as_ref()
                        .is_some_and(|st| named(st.line_pattern.as_deref()))
                }) {
                    out.layer_states.push(s.name.clone());
                }
            }
            out.sheet_maps = sheet_maps(doc, |o| named(o.line_pattern.as_deref()));
        }
        ImpactTarget::TextStyle => {
            let style = doc
                .settings()
                .text_styles
                .iter()
                .find(|s| s.id == id)
                .ok_or_else(|| {
                    fail(
                        codes::STYLE_NOT_FOUND,
                        format!("“{id}” kimlikli yazı stili projede yok."),
                    )
                })?;
            out.name = style.name.clone();
            let face = style.face();
            let height = style.height_at(doc.settings().plot_scale);
            let mut own = Own::new(&["font", "bold", "italic", "oblique", "widthFactor", "height"]);
            let count = |t: &kentos_commands::TextEntity, own: &mut Own| {
                if t.face.font != face.font {
                    own.add("font");
                }
                if t.face.bold != face.bold {
                    own.add("bold");
                }
                if t.face.italic != face.italic {
                    own.add("italic");
                }
                if t.face.oblique != face.oblique {
                    own.add("oblique");
                }
                if t.width_factor.unwrap_or(1.0) != style.width_factor.unwrap_or(1.0) {
                    own.add("widthFactor");
                }
                if height.is_some_and(|h| h != t.height) {
                    own.add("height");
                }
            };
            for e in doc.entities() {
                if let Entity::Text(t) = e
                    && t.face.text_style.as_deref() == Some(id)
                {
                    count(t, &mut own);
                    follow(e, &mut out);
                }
            }
            out.own = own.list();
            for b in doc.blocks() {
                let pieces = b
                    .entities
                    .iter()
                    .filter(|e| matches!(e, Entity::Text(t) if t.face.text_style.as_deref() == Some(id)))
                    .count() as u32;
                if pieces > 0 {
                    out.block_pieces += pieces;
                    blocks.insert(b.id);
                    out.blocks.push(b.id.to_text());
                }
            }
        }
        ImpactTarget::DimensionStyle => {
            let style = doc
                .settings()
                .dimension_styles
                .iter()
                .find(|s| s.id == id)
                .ok_or_else(|| {
                    fail(
                        codes::STYLE_NOT_FOUND,
                        format!("“{id}” kimlikli ölçü stili projede yok."),
                    )
                })?;
            out.name = style.name.clone();
            let look = serde_json::to_value(style.look()).unwrap_or(Value::Null);
            // Each field of the look, the style's and the objects', by name.
            let mut counts: Vec<(String, u32)> = Vec::new();
            for e in doc.entities() {
                if let Entity::Dimension(d) = e
                    && d.look.dim_style.as_deref() == Some(id)
                {
                    let mine = serde_json::to_value(&d.look).unwrap_or(Value::Null);
                    let mut keys: Vec<String> = Vec::new();
                    for k in look
                        .as_object()
                        .into_iter()
                        .chain(mine.as_object())
                        .flat_map(|o| o.keys())
                    {
                        if k != "dimStyle" && !keys.contains(k) {
                            keys.push(k.clone());
                        }
                    }
                    for k in keys {
                        if mine.get(k.as_str()) != look.get(k.as_str()) {
                            match counts.iter_mut().find(|(f, _)| *f == k) {
                                Some((_, c)) => *c += 1,
                                None => counts.push((k, 1)),
                            }
                        }
                    }
                    follow(e, &mut out);
                }
            }
            counts.sort();
            out.own = counts
                .into_iter()
                .filter(|(_, c)| *c > 0)
                .map(|(p, c)| ImpactOwn {
                    property: p,
                    count: c,
                })
                .collect();
            for b in doc.blocks() {
                let pieces = b
                    .entities
                    .iter()
                    .filter(|e| matches!(e, Entity::Dimension(d) if d.look.dim_style.as_deref() == Some(id)))
                    .count() as u32;
                if pieces > 0 {
                    out.block_pieces += pieces;
                    blocks.insert(b.id);
                    out.blocks.push(b.id.to_text());
                }
            }
        }
    }
    out.inserts = shown(doc, &blocks);
    out.notes = notes(&out, input.target);
    Ok(out)
}

/// A property's Turkish word, as people read it (`lineType` çizgi tipi).
pub fn property_word(p: &str) -> &str {
    match p {
        "color" => "renk",
        "lineType" => "çizgi tipi",
        "lineWeight" => "kalınlık",
        "transparency" => "saydamlık",
        "font" => "yazı tipi",
        "bold" => "kalın",
        "italic" => "eğik",
        "oblique" => "yatıklık",
        "widthFactor" => "genişlik çarpanı",
        "height" => "yükseklik",
        other => other,
    }
}

/// What the numbers mean, Turkish.
fn notes(o: &StyleImpact, target: ImpactTarget) -> Vec<String> {
    let mut n = Vec::new();
    n.push(format!("{} nesne değişikliği alır.", o.follow));
    if !o.own.is_empty() {
        let list: Vec<String> = o
            .own
            .iter()
            .map(|w| format!("{} {}", w.count, property_word(&w.property)))
            .collect();
        n.push(format!("Kendi değerini koruyanlar: {}.", list.join(", ")));
    }
    if o.locked > 0 {
        n.push(format!(
            "{} nesne kilitli katmanda: stil değişince olduğu gibi kalır.",
            o.locked
        ));
    }
    if target == ImpactTarget::Layer {
        n.push("Katmanın görünüşü kilitli katmandaki nesnelere de yansır.".into());
    }
    if o.block_pieces > 0 {
        n.push(format!(
            "{} blokta {} parça izler; {} yerleştirmede görünür.",
            o.blocks.len(),
            o.block_pieces,
            o.inserts
        ));
    }
    if !o.layer_states.is_empty() {
        n.push(format!(
            "Katman durumları kendi görünüşünü saklıyor ve uygulanınca geri getirir: {}.",
            o.layer_states.join(", ")
        ));
    }
    if !o.sheet_maps.is_empty() {
        n.push(format!(
            "{} pafta haritası onu kendi geçersiz kılmasıyla çizer.",
            o.sheet_maps.len()
        ));
    }
    n.push("Dış başvuru (xref) yok: KentOS dış başvuru taşımaz.".into());
    n
}

pub fn validate(cx: &ExecutionContext<'_>, input: &StylesImpact) -> CommandResult<()> {
    match impact(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &StylesImpact) -> CommandResult<StyleImpact> {
    match impact(cx.doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Read-only: the same answer as `plan`; nothing is written.
pub fn execute(cx: &mut ExecutionContext<'_>, input: StylesImpact) -> CommandResult<StyleImpact> {
    plan(cx, &input)
}
