//! The fixed mixed drawing of CAD-ADV-19 (docs/adr/1062): `n` objects made by this code from a seed, never a file in
//! the repository. Each 25 m cell holds ten objects: four lines, two texts, a circle or an arc, an insert of a block
//! three deep (a block placing a block placing a block), a dense hatch or a dimension, and a short polyline; every
//! tenth cell's polyline is a long one of 400 vertices running across ten cells. Six layers, one per kind of object,
//! so a layer's parts are of one kind. The same `n` and seed give the same drawing on any run: `fingerprint` hashes
//! every number and string made, and a measurement prints it beside its results.
//!
//! The objects are made as the drawing's JSON is read (`serde_json::from_value`): what is measured is the document
//! made of them, not this building.

use std::collections::BTreeMap;

use kentos_commands::{BlockDefinition, DocumentSnapshotV2, Entity, EntityId, LayerNode};
use serde_json::{Value, json};

/// The seed of every measured drawing (the item's number).
pub(crate) const SEED: u64 = 19;
/// Objects in a cell, and a cell's side in metres.
const PER_CELL: usize = 10;
const CELL: f64 = 25.0;
/// A long polyline's vertices.
const LONG: usize = 400;
const E0: f64 = 486_000.0;
const N0: f64 = 4_420_000.0;

/// xorshift64*: the same numbers on every machine.
pub(super) struct Rng(u64);

impl Rng {
    pub(super) fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A number in `[lo, hi)`, rounded to the millimetre as a surveyed coordinate is.
    fn mm(&mut self, lo: f64, hi: f64) -> f64 {
        let u = (self.next() >> 11) as f64 / (1u64 << 53) as f64;
        ((lo + (hi - lo) * u) * 1000.0).round() / 1000.0
    }
}

/// FNV-1a over what was made, for the report: two runs with the same `n` and seed print the same.
#[derive(Default)]
pub(super) struct Fingerprint(u64);

impl Fingerprint {
    fn add(&mut self, bytes: &[u8]) {
        if self.0 == 0 {
            self.0 = 0xcbf2_9ce4_8422_2325;
        }
        for b in bytes {
            self.0 ^= u64::from(*b);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn value(&mut self, v: &Value) {
        match v {
            Value::Number(n) => self.add(&n.as_f64().unwrap_or(0.0).to_bits().to_le_bytes()),
            Value::String(s) => self.add(s.as_bytes()),
            Value::Array(a) => a.iter().for_each(|v| self.value(v)),
            Value::Object(o) => o.iter().for_each(|(k, v)| {
                self.add(k.as_bytes());
                self.value(v);
            }),
            Value::Bool(b) => self.add(&[u8::from(*b)]),
            Value::Null => self.add(&[0]),
        }
    }

    pub(super) fn hex(&self) -> String {
        format!("{:016x}", self.0)
    }
}

/// The layers: id, name, colour.
pub(super) const LAYERS: [(&str, &str, &str); 6] = [
    ("cizgi", "Çizgi", "#C8CCD4"),
    ("yazi", "Yazı", "#E5C07B"),
    ("daire", "Daire", "#61AFEF"),
    ("blok", "Blok", "#98C379"),
    ("tarama", "Tarama", "#C678DD"),
    ("olcu", "Ölçü", "#56B6C2"),
];

pub(super) const BLOCK_1: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d1901";
const BLOCK_2: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d1902";
const BLOCK_3: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d1903";

fn p(x: f64, y: f64) -> Value {
    json!({ "x": x, "y": y })
}

/// The three nested blocks: Kapak (a 1 m square, a 0.3 m circle, a solid triangle), Grup (two lines, a text, two
/// Kapak), Ada (a 3 × 2 m outline, a text, two Grup turned). An Ada insert draws 31 objects.
fn blocks() -> Vec<BlockDefinition> {
    let leaf = json!({ "id": BLOCK_3, "name": "Kapak", "base": p(0.0, 0.0), "entities": [
        { "kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": p(0.0, 0.0), "b": p(1.0, 0.0) },
        { "kind": "line", "id": 2, "layerId": "", "attrs": {}, "a": p(1.0, 0.0), "b": p(1.0, 1.0) },
        { "kind": "line", "id": 3, "layerId": "", "attrs": {}, "a": p(1.0, 1.0), "b": p(0.0, 1.0) },
        { "kind": "line", "id": 4, "layerId": "", "attrs": {}, "a": p(0.0, 1.0), "b": p(0.0, 0.0) },
        { "kind": "circle", "id": 5, "layerId": "", "attrs": {}, "c": p(0.5, 0.5), "r": 0.3 },
        { "kind": "hatch", "id": 6, "layerId": "", "attrs": {},
          "ring": [p(0.1, 0.1), p(0.4, 0.1), p(0.1, 0.4)], "pattern": { "type": "solid", "angle": 0, "spacing": 1 } },
    ]});
    let group = json!({ "id": BLOCK_2, "name": "Grup", "base": p(0.0, 0.0), "entities": [
        { "kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": p(0.0, -0.2), "b": p(2.4, -0.2) },
        { "kind": "line", "id": 2, "layerId": "", "attrs": {}, "a": p(0.0, 1.2), "b": p(2.4, 1.2) },
        { "kind": "text", "id": 3, "layerId": "", "attrs": {}, "p": p(0.1, -0.6), "text": "G-1", "height": 0.25, "rotation": 0 },
        { "kind": "insert", "id": 4, "layerId": "", "attrs": {}, "block": BLOCK_3, "p": p(0.0, 0.0), "scale": 1, "rotation": 0 },
        { "kind": "insert", "id": 5, "layerId": "", "attrs": {}, "block": BLOCK_3, "p": p(1.4, 0.0), "scale": 1, "rotation": 0.5 },
    ]});
    let top = json!({ "id": BLOCK_1, "name": "Ada", "base": p(0.0, 0.0), "entities": [
        { "kind": "polyline", "id": 1, "layerId": "", "attrs": {}, "closed": true,
          "pts": [p(-0.3, -0.9), p(5.7, -0.9), p(5.7, 3.1), p(-0.3, 3.1)] },
        { "kind": "text", "id": 2, "layerId": "", "attrs": {}, "p": p(0.0, 2.5), "text": "ADA 19", "height": 0.4, "rotation": 0 },
        { "kind": "insert", "id": 3, "layerId": "", "attrs": {}, "block": BLOCK_2, "p": p(0.0, 0.0), "scale": 1, "rotation": 0 },
        { "kind": "insert", "id": 4, "layerId": "", "attrs": {}, "block": BLOCK_2, "p": p(3.0, 0.3), "scale": 1, "rotation": 0.2 },
    ]});
    // `KENTOS_MIXED_BLOCK_LEAVES_OUT` (a diagnosis, never a measured drawing): the blocks without the pieces of
    // these kinds (`text,circle,hatch,line,polyline,insert`), to see which piece the GPU's time goes to.
    let out = std::env::var("KENTOS_MIXED_BLOCK_LEAVES_OUT").unwrap_or_default();
    [leaf, group, top]
        .into_iter()
        .map(|mut b| {
            if let Some(list) = b["entities"].as_array_mut() {
                list.retain(|e| !out.split(',').any(|k| !k.is_empty() && e["kind"] == k));
            }
            serde_json::from_value(b).expect("a block definition")
        })
        .collect()
}

/// A cell's corner: rows of `side` cells.
fn corner(cell: usize, side: usize) -> (f64, f64) {
    (
        E0 + (cell % side) as f64 * CELL,
        N0 + (cell / side) as f64 * CELL,
    )
}

/// Cells per row for `n` objects: a square of cells.
pub(crate) fn side(n: usize) -> usize {
    ((n.div_ceil(PER_CELL)) as f64).sqrt().ceil().max(1.0) as usize
}

/// The drawing's box in world metres.
pub(crate) fn extent(n: usize) -> [f64; 4] {
    let s = side(n);
    let rows = n.div_ceil(PER_CELL).div_ceil(s);
    [E0, N0, E0 + s as f64 * CELL, N0 + rows as f64 * CELL]
}

/// The `k`th object of cell `cell` (`k` < 10), as the drawing's JSON.
fn object(rng: &mut Rng, cell: usize, k: usize, side: usize) -> Value {
    let (x0, y0) = corner(cell, side);
    let at = |rng: &mut Rng| p(x0 + rng.mm(0.5, CELL - 0.5), y0 + rng.mm(0.5, CELL - 0.5));
    match k {
        0..=3 => {
            json!({ "kind": "line", "layerId": "cizgi", "attrs": {}, "a": at(rng), "b": at(rng) })
        }
        4 | 5 => {
            let words = ["Ada", "Parsel", "Yol", "Kot", "Bina", "Park"];
            json!({ "kind": "text", "layerId": "yazi", "attrs": {}, "p": at(rng),
                "text": format!("{} {}/{}", words[(cell + k) % words.len()], 100 + cell / 50, cell % 50 + k),
                "height": rng.mm(0.3, 2.0), "rotation": rng.mm(0.0, std::f64::consts::TAU) })
        }
        6 if cell % 2 == 0 => {
            json!({ "kind": "circle", "layerId": "daire", "attrs": {}, "c": at(rng), "r": rng.mm(0.2, 6.0) })
        }
        6 => {
            let a0 = rng.mm(0.0, std::f64::consts::TAU);
            json!({ "kind": "arc", "layerId": "daire", "attrs": {}, "c": at(rng), "r": rng.mm(0.5, 8.0),
                "a0": a0, "a1": a0 + rng.mm(0.3, 5.5) })
        }
        7 => json!({ "kind": "insert", "layerId": "blok", "attrs": {}, "block": BLOCK_1,
            "p": p(x0 + rng.mm(2.0, 14.0), y0 + rng.mm(2.0, 18.0)), "scale": rng.mm(0.5, 1.5),
            "rotation": rng.mm(0.0, std::f64::consts::TAU), "mirror": cell % 7 == 3 }),
        8 if cell % 2 == 0 => {
            let (x, y) = (x0 + rng.mm(1.0, 12.0), y0 + rng.mm(1.0, 12.0));
            let (w, h) = (rng.mm(4.0, 11.0), rng.mm(4.0, 11.0));
            let pattern = if cell % 4 == 0 {
                json!({ "type": "cross", "angle": rng.mm(0.0, 90.0), "spacing": 0.2 })
            } else {
                // A dashed family, as a brick or ANSI pattern draws.
                json!({ "type": "pattern", "angle": 0, "spacing": 1, "name": "OLCUM", "scale": 0.25, "lines": [
                    { "angle": 45.0, "origin": [0.0, 0.0], "offset": [0.0, 0.5], "dashes": [] },
                    { "angle": 135.0, "origin": [0.0, 0.0], "offset": [0.0, 1.0], "dashes": [0.5, -0.25, 0.0, -0.25] },
                ] })
            };
            json!({ "kind": "hatch", "layerId": "tarama", "attrs": {},
                "ring": [p(x, y), p(x + w, y), p(x + w, y + h), p(x, y + h)],
                "holes": [[p(x + 1.0, y + 1.0), p(x + 2.0, y + 1.0), p(x + 2.0, y + 2.0), p(x + 1.0, y + 2.0)]],
                "pattern": pattern })
        }
        8 => {
            let (x, y) = (x0 + rng.mm(1.0, 10.0), y0 + rng.mm(1.0, 20.0));
            json!({ "kind": "dimension", "layerId": "olcu", "attrs": {}, "a": p(x, y), "b": p(x + rng.mm(2.0, 14.0), y),
                "offset": rng.mm(0.8, 2.0), "height": 0.35 })
        }
        _ if cell % 10 == 0 => {
            // A long line: 400 vertices over ten cells, a contour or a road's edge.
            let (mut x, mut y) = (x0 + 1.0, y0 + rng.mm(2.0, CELL - 2.0));
            let pts: Vec<Value> = (0..LONG)
                .map(|_| {
                    x += CELL * 10.0 / LONG as f64;
                    y += rng.mm(-0.6, 0.6);
                    p((x * 1000.0).round() / 1000.0, (y * 1000.0).round() / 1000.0)
                })
                .collect();
            json!({ "kind": "polyline", "layerId": "cizgi", "attrs": {}, "pts": pts, "closed": false })
        }
        _ => {
            let pts: Vec<Value> = (0..6).map(|_| at(rng)).collect();
            json!({ "kind": "polyline", "layerId": "cizgi", "attrs": {}, "pts": pts, "closed": false })
        }
    }
}

/// The measured drawing: `n` objects from `seed`, and its fingerprint.
pub(crate) fn drawing(n: usize, seed: u64) -> (DocumentSnapshotV2, String) {
    let mut doc = super::drawing(0);
    doc.name = format!("karma-{n}");
    let template = doc.layers[0].clone();
    doc.layers = LAYERS
        .iter()
        .map(|(id, name, color)| {
            let mut l: LayerNode = template.clone();
            l.id = (*id).into();
            l.name = (*name).into();
            l.style.color = (*color).into();
            l.style.line_weight = 0.18;
            l
        })
        .collect();
    doc.active_layer = "cizgi".into();
    doc.blocks = blocks();
    let side = side(n);
    let mut rng = Rng::new(seed);
    let mut print = Fingerprint::default();
    let mut entities = Vec::with_capacity(n);
    let mut uids = Vec::with_capacity(n);
    for i in 0..n {
        let mut v = object(&mut rng, i / PER_CELL, i % PER_CELL, side);
        print.value(&v);
        v["id"] = json!(i + 1);
        let e: Entity = serde_json::from_value(v).expect("an object");
        entities.push(e);
        let mut id = [
            0x01, 0x92, 0xf5, 0xa0, 0x7c, 0x3e, 0x70, 0x00, 0x80, 0x19, 0, 0, 0, 0, 0, 0,
        ];
        id[10..].copy_from_slice(&(i as u64 + 1).to_be_bytes()[2..]);
        uids.push(EntityId(id));
    }
    doc.entities = entities;
    doc.uids = uids;
    (doc, print.hex())
}

/// How many of each kind the drawing holds (for the report).
pub(super) fn kinds(doc: &DocumentSnapshotV2) -> BTreeMap<&'static str, usize> {
    let mut out = BTreeMap::new();
    for e in &doc.entities {
        let k = match e {
            Entity::Line(_) => "line",
            Entity::Text(_) => "text",
            Entity::Circle(_) => "circle",
            Entity::Arc(_) => "arc",
            Entity::Insert(_) => "insert",
            Entity::Hatch(_) => "hatch",
            Entity::Dimension(_) => "dimension",
            Entity::Polyline(p) if p.pts.len() >= LONG => "longPolyline",
            Entity::Polyline(_) => "polyline",
            _ => "other",
        };
        *out.entry(k).or_default() += 1;
    }
    out
}

#[test]
fn the_same_seed_makes_the_same_drawing_and_another_seed_another() {
    let (a, pa) = drawing(2_000, SEED);
    let (b, pb) = drawing(2_000, SEED);
    let (_, pc) = drawing(2_000, SEED + 1);
    assert_eq!(pa, pb);
    assert_ne!(pa, pc);
    assert_eq!(a.entities, b.entities);
    let k = kinds(&a);
    // Ten objects a cell: 4 lines, 2 texts, 1 circle/arc, 1 insert, 1 hatch/dimension, 1 polyline (every tenth long).
    assert_eq!(k["line"], 800);
    assert_eq!(k["text"], 400);
    assert_eq!(k["circle"] + k["arc"], 200);
    assert_eq!(k["insert"], 200);
    assert_eq!(k["hatch"] + k["dimension"], 200);
    assert_eq!(k["longPolyline"], 20);
    assert_eq!(k["polyline"], 180);
    // The document takes every object, the nested blocks included.
    let doc = crate::document::Document::from_v2(a, None).expect("the drawing opens");
    assert_eq!(doc.entity_count(), 2_000);
}
