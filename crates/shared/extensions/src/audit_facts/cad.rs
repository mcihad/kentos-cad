//! A drawing's CAD facts: the `cad` section of `kentos.auditFacts` 1 (docs/adr/8064). What the digital project
//! regulation's EK-3 and EK-4 rules read of a 2D drawing (Md. 7; plan/bim-plan.md BIM-03): every layer's name, its
//! objects, hatches and line type; the blocks inserted and the layers they stand on; the typefaces the dimensions write
//! in; the grids' layers; every room's number, name, storey code (EK-2 Tablo 2.14, as the sheets name storeys) and
//! centre; each layer's objects as model, annotation and free drawing, the sections and elevations with whether they are
//! still what the model draws, and the annotations whose tie broke (docs/adr/8065, Md. 9/1). The rules are Python's (bim-plan K10): the facts say what is drawn, never whether it is right.
//!
//! An optional section of version 1, as [`super::architecture`]: absent when the drawing has neither a layer holding
//! objects nor a room, so an empty drawing's facts stay what they were. The layers in the layer tree's order; the
//! blocks by name; the fonts by name; the grids and the rooms by persistent id. Places to 1 mm; a list holds at most
//! [`super::MOST_ITEMS`] items, the first in its order; past it the facts say `cut`.

use serde::{Deserialize, Serialize};

use super::{AREA, MOST_ITEMS, place, round};

/// The most objects a layer names (`first`, `firstHatches`).
pub const MOST_SHOWN: usize = 100;

/// The drawing's CAD facts.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct CadFacts {
    /// Every layer of the tree (its leaves), in the tree's order, holding objects or not.
    pub layers: Vec<CadLayer>,
    /// The blocks inserted, by name.
    pub blocks: Vec<CadBlock>,
    /// The typefaces the dimensions write their values in, each with how many dimensions, by name.
    pub dimension_fonts: Vec<FontUse>,
    /// Every grid (aks ağı), by persistent id: its axes are drawn in its layer's line type.
    pub grids: Vec<CadGrid>,
    /// Every room, by persistent id.
    pub rooms: Vec<CadRoom>,
    /// The sections and elevations drawn from their marks, with whether each is what the model would draw now
    /// (docs/adr/8065), by name.
    #[serde(default)]
    pub derived: Vec<CadDerived>,
    /// The tags whose object is gone and the dimensions whose tie to an object broke, by persistent id.
    #[serde(default)]
    pub broken: Vec<CadBroken>,
    /// The rooms of one storey that cover the same ground, a pair each (docs/adr/8067; EK-9 Tablo 9.1/13), by their
    /// ids; written only when there is one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub room_overlaps: Vec<CadRoomOverlap>,
}

/// Two rooms of one storey (or both on none) that cover the same ground: their persistent ids, the smaller first, the
/// ground they share (m², to 0.0001) and its centre. The facts say how much; whether it is too much is the rule's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CadRoomOverlap {
    pub a: String,
    pub b: String,
    pub area: f64,
    pub at: [f64; 2],
}

/// A section or an elevation drawn from its mark (docs/adr/8042, 8043).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CadDerived {
    /// `section` or `elevation`.
    pub kind: String,
    /// "B-B kesiti", "Kuzey görünüşü".
    pub name: String,
    /// Its mark's persistent id; none when the mark is gone.
    pub mark: Option<String>,
    /// `current`: what drawing it again gives; `stale`: the model changed since; `notDrawn`: its mark has no drawing
    /// yet; `gone`: its drawing's mark is no longer in the drawing.
    pub state: String,
    /// How many times it is placed.
    pub inserts: u64,
}

/// An annotation whose tie broke.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CadBroken {
    /// `tag` or `dimension`.
    pub kind: String,
    /// Its persistent id.
    pub uid: String,
}

/// A layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct CadLayer {
    /// Its id, what its objects' `layerId` names.
    pub id: String,
    /// Its name, as the layer tree shows it and a DXF writes it.
    pub name: String,
    /// How many objects it holds.
    pub objects: u64,
    /// How many of them are hatches (a hatch object, not a wall's own fill).
    pub hatches: u64,
    /// Its first objects by persistent id, at most [`MOST_SHOWN`]: what a finding about the layer selects.
    pub first: Vec<String>,
    /// Its first hatches by persistent id, at most [`MOST_SHOWN`].
    pub first_hatches: Vec<String>,
    /// How many of its objects are the model's (walls, openings, rooms, slabs, columns, stairs …, blocks placed).
    #[serde(default)]
    pub model: u64,
    /// How many are annotations tied to the drawing (dimensions, tags, texts, leaders, tables, marks).
    #[serde(default)]
    pub annotations: u64,
    /// How many are free drawing: lines, polylines, arcs, circles, curves, points, hatches.
    #[serde(default)]
    pub free: u64,
    /// Its first free objects by persistent id, at most [`MOST_SHOWN`].
    #[serde(default)]
    pub first_free: Vec<String>,
    /// Its built-in line type: `continuous`, `dashed`, `dashdot` or `dotted` (with a project line type, the nearest).
    pub line_type: String,
    /// The project line type it draws in, by name; none: its built-in one.
    pub line_pattern: Option<String>,
}

/// A block and its inserts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CadBlock {
    /// Its name, as the drawing's block library names it.
    pub name: String,
    /// How many times it is inserted.
    pub inserts: u64,
    /// The layers its inserts stand on, by name, each once, in order.
    pub layers: Vec<String>,
}

/// A typeface and how many use it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FontUse {
    /// Its name as KentOS names it (`Barlow`, `Arimo` …), or an SHX file's (`romans.shx`).
    pub font: String,
    pub count: u64,
}

/// A grid.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CadGrid {
    /// Its persistent id.
    pub uid: String,
    /// Its layer's id.
    pub layer: String,
}

/// A room as its number and name stand on the plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct CadRoom {
    /// Its persistent id.
    pub uid: String,
    pub number: Option<String>,
    pub name: Option<String>,
    /// Its storey's id.
    pub level: Option<String>,
    /// Its storey's code as the sheets' file names write it (EK-2 Tablo 2.14: K00, K01 …, B01 …); none on no storey.
    pub level_code: Option<String>,
    /// Its outline's centre, m.
    pub at: [f64; 2],
}

impl CadFacts {
    /// Whether it says anything: a layer holding objects, a room or a section drawn.
    pub fn is_empty(&self) -> bool {
        self.rooms.is_empty() && self.derived.is_empty() && self.layers.iter().all(|l| l.objects == 0)
    }

    /// These facts in the module's order, rounding and bounds, and whether a list was cut. Doing it again changes
    /// nothing.
    pub fn normalized(mut self) -> (CadFacts, bool) {
        for l in &mut self.layers {
            for list in [&mut l.first, &mut l.first_hatches, &mut l.first_free] {
                list.sort();
                list.dedup();
                list.truncate(MOST_SHOWN);
            }
        }
        for b in &mut self.blocks {
            b.layers.sort();
            b.layers.dedup();
        }
        self.blocks.sort_by(|a, b| a.name.cmp(&b.name));
        self.dimension_fonts.sort_by(|a, b| a.font.cmp(&b.font));
        self.grids.sort_by(|a, b| a.uid.cmp(&b.uid));
        for r in &mut self.rooms {
            r.at = place(r.at);
        }
        self.rooms.sort_by(|a, b| a.uid.cmp(&b.uid));
        self.derived.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.mark.cmp(&b.mark)));
        self.broken.sort_by(|a, b| a.uid.cmp(&b.uid).then_with(|| a.kind.cmp(&b.kind)));
        for o in &mut self.room_overlaps {
            if o.b < o.a {
                std::mem::swap(&mut o.a, &mut o.b);
            }
            o.area = round(o.area, AREA);
            o.at = place(o.at);
        }
        self.room_overlaps.retain(|o| o.area > 0.0);
        self.room_overlaps.sort_by(|x, y| x.a.cmp(&y.a).then_with(|| x.b.cmp(&y.b)));
        let cut = [
            self.layers.len(),
            self.blocks.len(),
            self.grids.len(),
            self.rooms.len(),
            self.derived.len(),
            self.broken.len(),
            self.room_overlaps.len(),
        ]
        .iter()
        .any(|n| *n > MOST_ITEMS);
        self.layers.truncate(MOST_ITEMS);
        self.blocks.truncate(MOST_ITEMS);
        self.grids.truncate(MOST_ITEMS);
        self.rooms.truncate(MOST_ITEMS);
        self.derived.truncate(MOST_ITEMS);
        self.broken.truncate(MOST_ITEMS);
        self.room_overlaps.truncate(MOST_ITEMS);
        (self, cut)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lists_are_ordered_and_places_rounded() {
        let facts = CadFacts {
            layers: vec![],
            blocks: vec![
                CadBlock { name: "MM-TEFR-YATAK".into(), inserts: 2, layers: vec!["B".into(), "A".into(), "B".into()] },
                CadBlock { name: "MM-KAPI-90".into(), inserts: 1, layers: vec!["A".into()] },
            ],
            dimension_fonts: vec![FontUse { font: "Barlow".into(), count: 3 }, FontUse { font: "Arimo".into(), count: 1 }],
            grids: vec![],
            rooms: vec![CadRoom {
                uid: "r".into(),
                number: Some("K00_DAI_001".into()),
                name: Some("Salon".into()),
                level: Some("l0".into()),
                level_code: Some("K00".into()),
                at: [1.000_49, -0.0001],
            }],
            derived: vec![],
            broken: vec![],
            room_overlaps: vec![
                CadRoomOverlap { a: "r2".into(), b: "r1".into(), area: 0.360_04, at: [2.000_4, 1.0] },
                CadRoomOverlap { a: "r3".into(), b: "r4".into(), area: 0.000_04, at: [0.0, 0.0] },
            ],
        };
        let (n, cut) = facts.normalized();
        assert!(!cut);
        // A pair by its ids in order, its area to 0.0001 m²; one under that is no overlap.
        assert_eq!(n.room_overlaps, [CadRoomOverlap { a: "r1".into(), b: "r2".into(), area: 0.36, at: [2.0, 1.0] }]);
        assert_eq!(n.blocks[0].name, "MM-KAPI-90");
        assert_eq!(n.blocks[1].layers, ["A", "B"]);
        assert_eq!(n.dimension_fonts[0].font, "Arimo");
        assert_eq!(n.rooms[0].at, [1.0, 0.0]);
        assert_eq!(n.clone().normalized().0, n);
        assert!(!n.is_empty());
        assert!(CadFacts::default().is_empty());
    }
}
