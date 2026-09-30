//! The rules a drawing's blocks keep (docs/adr/0144), in one place: the file
//! codec, both documents and the commands check them here, so a drawing one
//! refuses no other takes.
//!
//! - each definition's id once, and its name once, compared with Turkish case
//!   folding (`name_key`); a name is not empty nor only white space;
//! - an attribute definition's tag is not empty and is once per definition;
//! - every insert names a definition of the drawing, its scale positive and
//!   finite;
//! - no definition holds itself, directly or through others, and nesting is
//!   at most `MAX_BLOCK_DEPTH` levels (a definition without inserts is one
//!   level). A cycle longer than that is reported as too deep.
//!
//! Faults are found in a fixed order (definitions in list order, then their
//! nesting, then the drawing's own objects), so every implementation names
//! the same one first. The web's document keeps the same rules and says them
//! in the same words (apps/web/src/model/blocks.ts); the shared document
//! fixtures (fixtures/document-ops/v1/blocks.json) hold the two together.

use std::borrow::Borrow;
use std::collections::HashMap;

use crate::entity::{BlockDefinition, Entity, MAX_BLOCK_DEPTH};
use crate::identity::BlockId;

/// Where an insert is: in a definition's objects, or in the drawing's own
/// (`definition: None`); `entity` is its place in that list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Place {
    pub definition: Option<usize>,
    pub entity: usize,
}

/// The first broken rule; indices are places in `blocks` and in a
/// definition's `attributes`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockFault {
    EmptyName {
        definition: usize,
    },
    DuplicateId {
        definition: usize,
        first: usize,
    },
    DuplicateName {
        definition: usize,
        first: usize,
    },
    EmptyTag {
        definition: usize,
        attribute: usize,
    },
    DuplicateTag {
        definition: usize,
        attribute: usize,
        first: usize,
    },
    BadScale {
        at: Place,
    },
    UnknownBlock {
        at: Place,
    },
    /// The definition found again while following its own inserts.
    Cycle {
        definition: usize,
    },
    /// The definition whose nesting is deeper than `MAX_BLOCK_DEPTH`.
    TooDeep {
        definition: usize,
    },
}

impl BlockFault {
    /// What an edit that breaks the rule is told (the documents' refusals;
    /// a file's errors say it their own way): `name` gives a definition's name.
    pub fn message<'a>(&self, name: impl Fn(usize) -> &'a str) -> String {
        match self {
            BlockFault::EmptyName { .. } => "Blok adı boş olamaz; bir ad yazın.".to_owned(),
            BlockFault::DuplicateId { .. } => {
                "Bu kimlikte bir blok zaten var; blok eklenmedi.".to_owned()
            }
            BlockFault::DuplicateName { first, .. } => format!(
                "Çizimde “{}” adında bir blok var; başka bir ad verin.",
                name(*first)
            ),
            BlockFault::EmptyTag { definition, .. } => format!(
                "“{}” bloğunda boş bir öznitelik etiketi var; her özniteliğe bir etiket verin.",
                name(*definition)
            ),
            BlockFault::DuplicateTag { definition, .. } => format!(
                "“{}” bloğunda bir öznitelik etiketi iki kez var; her etiket bir kez olmalı.",
                name(*definition)
            ),
            BlockFault::BadScale { .. } => "Blok ölçeği pozitif bir sayı olmalı.".to_owned(),
            BlockFault::UnknownBlock { .. } => {
                "Yerleştirilen blok çizimde tanımlı değil.".to_owned()
            }
            BlockFault::Cycle { definition } => format!(
                "“{}” bloğu kendini içeremez (doğrudan ya da başka bloklar yoluyla).",
                name(*definition)
            ),
            BlockFault::TooDeep { definition } => format!(
                "Bloklar en çok {MAX_BLOCK_DEPTH} düzey iç içe olabilir; “{}” bloğu daha derin.",
                name(*definition)
            ),
        }
    }
}

/// A definition's name as names are compared: each character lowercased,
/// with Turkish I (I → ı, İ → i). Character by character, so no letter's
/// case depends on its neighbours.
pub fn name_key(name: &str) -> String {
    let mut key = String::with_capacity(name.len());
    for c in name.chars() {
        match c {
            'I' => key.push('ı'),
            'İ' => key.push('i'),
            c => key.extend(c.to_lowercase()),
        }
    }
    key
}

/// Whether `name` may name a definition: something besides white space.
pub fn name_ok(name: &str) -> bool {
    !name.chars().all(char::is_whitespace)
}

/// The first “Blok n” (n = 1, 2, …) that no name of `taken` is, names
/// compared by [`name_key`]: what Blok oluştur offers.
pub fn free_name<'a>(taken: impl IntoIterator<Item = &'a str>) -> String {
    let keys: std::collections::HashSet<String> = taken.into_iter().map(name_key).collect();
    let mut n = 1_u64;
    while keys.contains(&name_key(&format!("Blok {n}"))) {
        n += 1;
    }
    format!("Blok {n}")
}

/// Whether an insert's scale is allowed: positive and finite.
pub fn scale_ok(scale: f64) -> bool {
    scale.is_finite() && scale > 0.0
}

/// The inserts of `block` among `entities`, directly (not through other
/// definitions): how many times it is placed there.
pub fn uses(entities: &[Entity], block: BlockId) -> usize {
    entities
        .iter()
        .filter(|e| matches!(e, Entity::Insert(i) if i.block == block))
        .count()
}

/// How often a definition is placed: its inserts among the drawing's own
/// objects, and inside definitions (the Bloklar panel's count; a block placed
/// anywhere is not deleted).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Placements {
    pub drawing: usize,
    pub nested: usize,
}

impl Placements {
    /// Whether an insert places it anywhere.
    pub fn used(self) -> bool {
        self.drawing + self.nested > 0
    }
}

/// Every definition's placements, in the definitions' order: one pass over
/// the drawing's objects and one over the definitions'.
pub fn placements<'a, B: Borrow<BlockDefinition>>(
    blocks: &[B],
    drawing: impl IntoIterator<Item = &'a Entity>,
) -> Vec<Placements> {
    let at: HashMap<BlockId, usize> = blocks
        .iter()
        .enumerate()
        .map(|(i, b)| (b.borrow().id, i))
        .collect();
    let mut out = vec![Placements::default(); blocks.len()];
    for e in drawing {
        if let Entity::Insert(i) = e
            && let Some(&k) = at.get(&i.block)
        {
            out[k].drawing += 1;
        }
    }
    for b in blocks {
        for e in &b.borrow().entities {
            if let Entity::Insert(i) = e
                && let Some(&k) = at.get(&i.block)
            {
                out[k].nested += 1;
            }
        }
    }
    out
}

/// A drawing's blocks and its own objects checked whole.
pub fn check<B: Borrow<BlockDefinition>>(
    blocks: &[B],
    entities: &[Entity],
) -> Result<(), BlockFault> {
    let index = definitions(blocks)?;
    for (i, block) in blocks.iter().enumerate() {
        inserts(&block.borrow().entities, Some(i), &index)?;
    }
    nesting(blocks, &index)?;
    inserts(entities, None, &index)
}

/// The definitions checked on their own (names, ids, tags); each id's place.
pub fn definitions<B: Borrow<BlockDefinition>>(
    blocks: &[B],
) -> Result<HashMap<BlockId, usize>, BlockFault> {
    let mut ids = HashMap::with_capacity(blocks.len());
    let mut names = HashMap::with_capacity(blocks.len());
    for (i, block) in blocks.iter().enumerate() {
        let block = block.borrow();
        if !name_ok(&block.name) {
            return Err(BlockFault::EmptyName { definition: i });
        }
        if let Some(&first) = ids.get(&block.id) {
            return Err(BlockFault::DuplicateId {
                definition: i,
                first,
            });
        }
        ids.insert(block.id, i);
        if let Some(&first) = names.get(&name_key(&block.name)) {
            return Err(BlockFault::DuplicateName {
                definition: i,
                first,
            });
        }
        names.insert(name_key(&block.name), i);
        let mut tags: HashMap<&str, usize> = HashMap::with_capacity(block.attributes.len());
        for (a, attribute) in block.attributes.iter().enumerate() {
            if attribute.tag.is_empty() {
                return Err(BlockFault::EmptyTag {
                    definition: i,
                    attribute: a,
                });
            }
            if let Some(&first) = tags.get(attribute.tag.as_str()) {
                return Err(BlockFault::DuplicateTag {
                    definition: i,
                    attribute: a,
                    first,
                });
            }
            tags.insert(&attribute.tag, a);
        }
    }
    Ok(ids)
}

/// The inserts among `entities` (a definition's, or the drawing's own when
/// `definition` is `None`): each scale allowed, each block known.
pub fn inserts(
    entities: &[Entity],
    definition: Option<usize>,
    index: &HashMap<BlockId, usize>,
) -> Result<(), BlockFault> {
    for (entity, e) in entities.iter().enumerate() {
        if let Entity::Insert(insert) = e {
            let at = Place { definition, entity };
            if !scale_ok(insert.scale) {
                return Err(BlockFault::BadScale { at });
            }
            if !index.contains_key(&insert.block) {
                return Err(BlockFault::UnknownBlock { at });
            }
        }
    }
    Ok(())
}

/// Each definition's nesting depth (1 without inserts), or the cycle or the
/// too deep nesting; every insert in the definitions must name a known one
/// (`inserts` first).
pub fn nesting<B: Borrow<BlockDefinition>>(
    blocks: &[B],
    index: &HashMap<BlockId, usize>,
) -> Result<Vec<usize>, BlockFault> {
    let mut depth = vec![0usize; blocks.len()];
    let mut stack = Vec::with_capacity(MAX_BLOCK_DEPTH);
    for i in 0..blocks.len() {
        if depth[i] == 0 {
            visit(i, blocks, index, &mut depth, &mut stack)?;
        }
    }
    Ok(depth)
}

/// Depth-first, the stack never deeper than the limit: a definition found on
/// the stack is a cycle; one that would make the stack's first definition
/// deeper than the limit is too deep.
fn visit<B: Borrow<BlockDefinition>>(
    i: usize,
    blocks: &[B],
    index: &HashMap<BlockId, usize>,
    depth: &mut [usize],
    stack: &mut Vec<usize>,
) -> Result<usize, BlockFault> {
    stack.push(i);
    let mut deepest = 0;
    for e in &blocks[i].borrow().entities {
        let Entity::Insert(insert) = e else { continue };
        let Some(&k) = index.get(&insert.block) else {
            continue;
        };
        if depth[k] == 0 {
            if stack.contains(&k) {
                return Err(BlockFault::Cycle { definition: k });
            }
            if stack.len() == MAX_BLOCK_DEPTH {
                return Err(BlockFault::TooDeep {
                    definition: stack[0],
                });
            }
            visit(k, blocks, index, depth, stack)?;
        }
        if stack.len() + depth[k] > MAX_BLOCK_DEPTH {
            return Err(BlockFault::TooDeep {
                definition: stack[0],
            });
        }
        deepest = deepest.max(depth[k]);
    }
    stack.pop();
    depth[i] = deepest + 1;
    Ok(depth[i])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{AttributeDefinition, EntityBase, InsertEntity, Vec2};

    fn id(n: u8) -> BlockId {
        BlockId([n; 16])
    }

    fn insert(block: u8, scale: f64) -> Entity {
        Entity::Insert(InsertEntity {
            base: EntityBase {
                id: 1,
                layer_id: "0".into(),
                color: None,
                attrs: Default::default(),
                label: None,
                symbol: None,
                line_weight: None,
            },
            block: id(block),
            p: Vec2 { x: 0.0, y: 0.0 },
            scale,
            rotation: 0.0,
            mirror: false,
        })
    }

    fn block(n: u8, name: &str, inside: &[u8]) -> BlockDefinition {
        BlockDefinition {
            id: id(n),
            name: name.into(),
            base: Vec2 { x: 0.0, y: 0.0 },
            entities: inside.iter().map(|&b| insert(b, 1.0)).collect(),
            attributes: Vec::new(),
            description: None,
        }
    }

    #[test]
    fn names_fold_turkish_case() {
        assert_eq!(name_key("IŞIK"), "ışık");
        assert_eq!(name_key("İzmir"), "izmir");
        assert_eq!(name_key("Direk"), name_key("DİREK"));
        assert_ne!(name_key("DIREK"), name_key("direk"));
        let blocks = [block(1, "Rögar", &[]), block(2, "RÖGAR", &[])];
        assert_eq!(
            check(&blocks, &[]),
            Err(BlockFault::DuplicateName {
                definition: 1,
                first: 0
            })
        );
        assert_eq!(
            check(&[block(1, " \t", &[])], &[]),
            Err(BlockFault::EmptyName { definition: 0 })
        );
        // Unicode's White_Space: U+0085 is, U+FEFF is not (the web's list is the same).
        assert!(!name_ok("\u{85}\u{3000}") && name_ok("\u{feff}") && !name_ok(""));
    }

    #[test]
    fn the_first_free_name_is_offered() {
        assert_eq!(free_name([]), "Blok 1");
        assert_eq!(free_name(["Blok 1", "BLOK 2"]), "Blok 3");
        assert_eq!(free_name(["blok 1", "Blok 3"]), "Blok 2");
        assert_eq!(free_name(["Rögar"]), "Blok 1");
        // A window trims a name with `str::trim` (the web's `trimName` is the same).
        let trimmed = ["  Rögar \t", "\u{85}Direk\u{3000}", "\u{feff}A", " "].map(str::trim);
        assert_eq!(trimmed, ["Rögar", "Direk", "\u{feff}A", ""]);
    }

    /// A is placed twice in the drawing and twice in B; B once in C; C once
    /// in the drawing; an insert of a block the drawing does not define is
    /// no one's.
    #[test]
    fn placements_count_the_drawing_and_the_definitions() {
        let blocks = [
            block(1, "A", &[]),
            block(2, "B", &[1, 1]),
            block(3, "C", &[2]),
            block(4, "D", &[]),
        ];
        let drawing = [
            insert(1, 1.0),
            insert(3, 1.0),
            insert(1, 2.0),
            insert(9, 1.0),
        ];
        let found = placements(&blocks, &drawing);
        let placed = |drawing, nested| Placements { drawing, nested };
        assert_eq!(
            found,
            [placed(2, 2), placed(0, 1), placed(1, 0), placed(0, 0)]
        );
        let used: Vec<bool> = found.iter().map(|p| p.used()).collect();
        assert_eq!(used, [true, true, true, false]);
    }

    #[test]
    fn ids_and_tags_are_once() {
        let blocks = [block(1, "A", &[]), block(1, "B", &[])];
        assert_eq!(
            check(&blocks, &[]),
            Err(BlockFault::DuplicateId {
                definition: 1,
                first: 0
            })
        );
        let tag = |t: &str| AttributeDefinition {
            tag: t.into(),
            prompt: None,
            value: None,
            p: Vec2 { x: 0.0, y: 0.0 },
            height: 1.0,
            rotation: 0.0,
        };
        let mut a = block(1, "A", &[]);
        a.attributes = vec![tag("NO"), tag("NO")];
        assert_eq!(
            check(&[a.clone()], &[]),
            Err(BlockFault::DuplicateTag {
                definition: 0,
                attribute: 1,
                first: 0
            })
        );
        a.attributes = vec![tag("")];
        assert_eq!(
            check(&[a], &[]),
            Err(BlockFault::EmptyTag {
                definition: 0,
                attribute: 0
            })
        );
    }

    #[test]
    fn inserts_name_known_blocks_with_a_positive_scale() {
        let blocks = [block(1, "A", &[2])];
        assert_eq!(
            check(&blocks, &[]),
            Err(BlockFault::UnknownBlock {
                at: Place {
                    definition: Some(0),
                    entity: 0
                }
            })
        );
        let blocks = [block(1, "A", &[])];
        assert_eq!(check(&blocks, &[insert(1, 2.5)]), Ok(()));
        for scale in [0.0, -1.0, f64::INFINITY, f64::NAN] {
            assert_eq!(
                check(&blocks, &[insert(1, scale)]),
                Err(BlockFault::BadScale {
                    at: Place {
                        definition: None,
                        entity: 0
                    }
                })
            );
        }
    }

    #[test]
    fn cycles_are_refused() {
        assert_eq!(
            check(&[block(1, "A", &[1])], &[]),
            Err(BlockFault::Cycle { definition: 0 })
        );
        let blocks = [
            block(1, "A", &[2]),
            block(2, "B", &[3]),
            block(3, "C", &[1]),
        ];
        assert_eq!(
            check(&blocks, &[]),
            Err(BlockFault::Cycle { definition: 0 })
        );
    }

    #[test]
    fn nesting_is_at_most_sixteen_levels() {
        // Definition n holds an insert of n + 1; the last holds none.
        let chain = |levels: u8| -> Vec<BlockDefinition> {
            (1..=levels)
                .map(|n| {
                    let inside: &[u8] = if n < levels { &[n + 1] } else { &[] };
                    block(n, &format!("B{n}"), inside)
                })
                .collect()
        };
        let ok = chain(16);
        let index = definitions(&ok).unwrap();
        let depths = nesting(&ok, &index).unwrap();
        assert_eq!(depths[0], 16);
        assert_eq!(depths[15], 1);
        assert_eq!(
            check(&chain(17), &[]),
            Err(BlockFault::TooDeep { definition: 0 })
        );
        // Reached through a definition already measured: the last one listed first.
        let mut late: Vec<_> = chain(17);
        late.rotate_left(1);
        assert_eq!(
            check(&late, &[]),
            Err(BlockFault::TooDeep { definition: 16 })
        );
        // A cycle longer than the limit is too deep.
        let mut ring = chain(20);
        ring[19].entities.push(insert(1, 1.0));
        assert_eq!(
            check(&ring, &[]),
            Err(BlockFault::TooDeep { definition: 0 })
        );
    }
}
