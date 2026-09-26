//! Untrusted input and round trips (TODOS.md FILE-06, FILE-22): nothing a file
//! holds may crash the reader, and every drawing the writer takes comes back
//! bit for bit. Inputs come from a fixed-seed generator, so a failure repeats.
//!
//! - random bytes, with and without the signature, and random payloads inside
//!   a valid container (lengths and SHA-256 right), so the CBOR and schema
//!   readers see garbage past the integrity check;
//! - every prefix of every valid fixture, and each fixture's payload cut and
//!   mutated byte by byte inside a valid container;
//! - random drawings (−0, subnormals, extremes, Unicode, deep layer trees,
//!   opaque values) written, read back equal, written again to the same bytes;
//! - what the writer refuses, with its code.

use std::collections::BTreeMap;

use kentos_kcad::Code;
use kentos_kcad::contracts::{
    AngleUnit, AreaUnit, Bounds, CircleEntity, DocumentSnapshotV2, DrawingFont, Entity, EntityBase,
    EntityId, HatchEntity, HatchPattern, HatchPatternType, LabelInk, LabelPlacement, LabelStyle,
    LayerNode, LayerNodeType, LayerStyle, LineType, MigrationSource, PathEntity, PointEntity,
    PointStyle, PointSymbol, ProjectId, ProjectSettings, ProjectStyles, RingGeometry, TextEntity,
    Vec2, Workspace,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// xorshift64*: a small generator with a fixed seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
    fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }
    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next() as u8).collect()
    }
    fn float(&mut self) -> f64 {
        match self.below(10) {
            0 => -0.0,
            1 => f64::from_bits(self.below(1 << 52)), // subnormal or zero
            2 => f64::MAX * if self.chance(50) { 1.0 } else { -1.0 },
            3 => f64::MIN_POSITIVE,
            4 => (self.below(10_000_000) as f64) / 7.0 + 400_000.0,
            _ => loop {
                let x = f64::from_bits(self.next());
                if x.is_finite() {
                    break x;
                }
            },
        }
    }
    fn text(&mut self) -> String {
        const PIECES: &[&str] = &[
            "", "a", "Ada", "ı", "İ", "ş", "Ğ", "ç", "\n", "\"", "\\", "\u{1f}", "🏠", "é", "ü",
            "Parsel 7", "{Ada}",
        ];
        (0..self.below(5))
            .map(|_| PIECES[self.below(PIECES.len() as u64) as usize])
            .collect()
    }
    fn uid(&mut self) -> [u8; 16] {
        loop {
            let mut id = [0u8; 16];
            id.copy_from_slice(&self.bytes(16));
            if id != [0; 16] {
                break id;
            }
        }
    }
}

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// A 2.0 container around any payload: right lengths, right SHA-256.
fn container(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&kentos_kcad::MAGIC);
    out.extend_from_slice(&[2, 0, 0, 36, 0, 1, 0]);
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(payload);
    let hash = Sha256::digest(&out);
    out.extend_from_slice(&hash);
    out
}

fn payload_of(file: &[u8]) -> &[u8] {
    &file[36..file.len() - 32]
}

#[test]
fn random_bytes_never_crash_the_reader() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for round in 0..20_000 {
        let n = rng.below(if round % 10 == 0 { 4096 } else { 96 }) as usize;
        let mut data = rng.bytes(n);
        match round % 4 {
            0 => {}
            1 => {
                let mut with = kentos_kcad::MAGIC.to_vec();
                with.append(&mut data);
                data = with;
            }
            _ => data = container(&data),
        }
        let _ = kentos_kcad::sniff(&data);
        assert!(
            kentos_kcad::decode(&data).is_err(),
            "round {round}: random bytes read as a drawing"
        );
    }
}

#[test]
fn random_cbor_like_payloads_never_crash_the_reader() {
    // Payloads made of plausible heads (maps, arrays, keys the schema knows, floats), so the
    // reader goes deep into the schema before it refuses.
    let mut rng = Rng(42);
    const KEYS: &[&str] = &[
        "format",
        "version",
        "document",
        "name",
        "layers",
        "origin",
        "styles",
        "entities",
        "settings",
        "activeLayer",
        "point",
        "polygon",
        "uid",
        "attrs",
        "layerId",
        "p",
        "pts",
        "holes",
        "items",
        "categories",
        "id",
        "type",
        "style",
        "children",
    ];
    for _ in 0..20_000 {
        let mut p = Vec::new();
        for _ in 0..rng.below(60) {
            match rng.below(9) {
                0 => p.push(0xa0 | rng.below(4) as u8),
                1 => p.push(0x80 | rng.below(4) as u8),
                2 => {
                    let k = KEYS[rng.below(KEYS.len() as u64) as usize];
                    p.push(0x60 | k.len() as u8);
                    p.extend_from_slice(k.as_bytes());
                }
                3 => {
                    p.push(0xfb);
                    p.extend_from_slice(&rng.float().to_bits().to_be_bytes());
                }
                4 => p.push(0x50),
                5 => p.extend_from_slice(&rng.bytes(3)),
                6 => p.push(0x02),
                7 => p.extend_from_slice(b"\x6fkentos.document"),
                _ => p.push(rng.next() as u8),
            }
        }
        let _ = kentos_kcad::decode(&container(&p));
    }
}

#[test]
fn every_prefix_and_every_mutation_of_the_fixtures_is_refused_or_read_without_crashing() {
    for name in ["minimal.kcad", "drawing.kcad", "migrated.kcad"] {
        let file = fixture(name);
        for end in 0..file.len() {
            assert!(
                kentos_kcad::decode(&file[..end]).is_err(),
                "{name}[..{end}] read"
            );
        }
        let payload = payload_of(&file);
        for end in 0..payload.len() {
            assert!(
                kentos_kcad::decode(&container(&payload[..end])).is_err(),
                "{name}: payload[..{end}] read"
            );
        }
        let mut rng = Rng(7);
        for at in 0..payload.len() {
            let mut changed = payload.to_vec();
            changed[at] ^= 1 << rng.below(8);
            // A changed byte may still be a drawing (another float, another letter); it must not crash.
            let _ = kentos_kcad::decode(&container(&changed));
            changed[at] = rng.next() as u8;
            let _ = kentos_kcad::decode(&container(&changed));
        }
    }
}

// ── Random drawings ─────────────────────────────────────────────────────

fn base(rng: &mut Rng, layer: &str) -> EntityBase {
    let mut attrs = BTreeMap::new();
    for _ in 0..rng.below(4) {
        attrs.insert(rng.text(), rng.text());
    }
    EntityBase {
        id: rng.below(1000) as u32 + 1,
        layer_id: layer.to_owned(),
        color: rng.chance(30).then(|| "#FF0000".to_owned()),
        attrs,
        label: rng.chance(30).then(|| rng.text()),
        symbol: rng.chance(10).then(|| rng.text()),
    }
}

fn point(rng: &mut Rng) -> Vec2 {
    Vec2 {
        x: rng.float(),
        y: rng.float(),
    }
}

fn points(rng: &mut Rng, least: u64) -> Vec<Vec2> {
    (0..least + rng.below(5)).map(|_| point(rng)).collect()
}

fn opaque(rng: &mut Rng, depth: u32) -> Value {
    match rng.below(if depth > 5 { 6 } else { 8 }) {
        0 => Value::Null,
        1 => Value::Bool(rng.chance(50)),
        2 => json!(rng.next()),
        3 => json!(-(rng.below(1 << 62) as i64) - 1),
        4 => json!(rng.float()),
        5 => Value::String(rng.text()),
        6 => Value::Array((0..rng.below(4)).map(|_| opaque(rng, depth + 1)).collect()),
        _ => Value::Object(
            (0..rng.below(4))
                .map(|_| (rng.text(), opaque(rng, depth + 1)))
                .collect(),
        ),
    }
}

fn layer(rng: &mut Rng, depth: u32, ids: &mut Vec<String>) -> LayerNode {
    let group = depth < 4 && rng.chance(30);
    let id = format!("k{}", ids.len());
    if !group {
        ids.push(id.clone());
    }
    let label = rng.chance(40).then(|| LabelStyle {
        placement: LabelPlacement::Along,
        size: rng.float(),
        grow: rng.chance(50).then(|| rng.float()),
        max_size: rng.chance(50).then(|| rng.float()),
        weight: rng.chance(50).then_some(500),
        template: rng.chance(50).then(|| rng.text()),
        min_feature_px: rng.chance(50).then(|| rng.float()),
        min_scale: rng.chance(50).then(|| rng.float()),
        max_scale: rng.chance(50).then(|| rng.float()),
        ink: rng.chance(50).then_some(LabelInk::FgDim),
    });
    LayerNode {
        id,
        name: rng.text(),
        kind: if group {
            LayerNodeType::Group
        } else {
            LayerNodeType::Layer
        },
        visible: rng.chance(80),
        locked: rng.chance(20),
        expanded: rng.chance(50),
        style: LayerStyle {
            color: "fg".to_owned(),
            line_type: LineType::Dashdot,
            line_weight: rng.float(),
            fill: rng.chance(30).then(|| "#7FB2E526".to_owned()),
            point: rng.chance(30).then(|| PointStyle {
                symbol: PointSymbol::Cross,
                size: rng.float(),
            }),
            label,
            pick_interior: rng.chance(30).then(|| rng.chance(50)),
            renderer: rng
                .chance(30)
                .then(|| json!({ "type": "single", "x": opaque(rng, 2) })),
        },
        children: if group {
            (0..1 + rng.below(3))
                .map(|_| layer(rng, depth + 1, ids))
                .collect()
        } else {
            Vec::new()
        },
    }
}

fn drawing(rng: &mut Rng) -> DocumentSnapshotV2 {
    let mut ids = Vec::new();
    let mut layers: Vec<LayerNode> = (0..1 + rng.below(3))
        .map(|_| layer(rng, 0, &mut ids))
        .collect();
    if ids.is_empty() {
        ids.push("son".to_owned());
        layers.push(LayerNode {
            id: "son".to_owned(),
            name: "Son".to_owned(),
            kind: LayerNodeType::Layer,
            visible: true,
            locked: false,
            expanded: true,
            style: LayerStyle {
                color: "fg".to_owned(),
                line_type: LineType::Continuous,
                line_weight: 0.18,
                fill: None,
                point: None,
                label: None,
                pick_interior: None,
                renderer: None,
            },
            children: Vec::new(),
        });
    }
    let mut entities = Vec::new();
    for _ in 0..rng.below(20) {
        let on = ids[rng.below(ids.len() as u64) as usize].clone();
        let b = base(rng, &on);
        entities.push(match rng.below(6) {
            0 => Entity::Point(PointEntity {
                base: b,
                p: point(rng),
                z: rng.chance(50).then(|| rng.float()),
            }),
            1 => Entity::Polyline(PathEntity {
                base: b,
                pts: points(rng, 2),
                bulges: rng
                    .chance(50)
                    .then(|| (0..rng.below(4)).map(|_| rng.float()).collect()),
                holes: None,
            }),
            2 => Entity::Polygon(PathEntity {
                base: b,
                pts: points(rng, 3),
                bulges: rng.chance(50).then(|| vec![rng.float(); 3]),
                holes: rng.chance(50).then(|| {
                    (0..rng.below(3))
                        .map(|_| RingGeometry {
                            pts: points(rng, 3),
                            bulges: rng.chance(50).then(|| vec![-0.0, rng.float()]),
                        })
                        .collect()
                }),
            }),
            3 => Entity::Circle(CircleEntity {
                base: b,
                c: point(rng),
                r: rng.float(),
            }),
            4 => Entity::Text(TextEntity {
                base: b,
                p: point(rng),
                text: rng.text(),
                height: rng.float(),
                rotation: rng.float(),
            }),
            _ => Entity::Hatch(HatchEntity {
                base: b,
                ring: points(rng, 3),
                holes: rng.chance(50).then(|| vec![points(rng, 3)]),
                pattern: HatchPattern {
                    kind: HatchPatternType::Cross,
                    angle: rng.float(),
                    spacing: rng.float(),
                },
            }),
        });
    }
    let uids = entities.iter().map(|_| EntityId(rng.uid())).collect();
    DocumentSnapshotV2 {
        format: "kentos.document".to_owned(),
        version: 2,
        name: rng.text(),
        settings: ProjectSettings {
            srid: rng.next() as u32,
            length_decimals: rng.below(10) as u32,
            area_decimals: rng.below(10) as u32,
            area_unit: AreaUnit::Ha,
            angle_unit: AngleUnit::Deg,
            plot_scale: rng.float(),
            workspace: rng.chance(50).then_some(Workspace::Gis),
            drawing_font: rng.chance(50).then_some(DrawingFont::ArchitectsDaughter),
        },
        origin: point(rng),
        home_view: rng.chance(50).then(|| Bounds {
            min_x: rng.float(),
            min_y: rng.float(),
            max_x: rng.float(),
            max_y: rng.float(),
        }),
        active_layer: ids[0].clone(),
        layers,
        entities,
        uids,
        styles: ProjectStyles {
            items: (0..rng.below(3)).map(|_| opaque(rng, 0)).collect(),
            categories: (0..rng.below(3)).map(|_| opaque(rng, 0)).collect(),
        },
        project_id: rng.chance(50).then(|| ProjectId(rng.uid())),
        migrated_from: rng.chance(30).then(|| MigrationSource::v1("ab".repeat(32))),
    }
}

/// The drawing with the slots a reader gives (1, 2, 3 … in file order).
fn with_read_slots(mut doc: DocumentSnapshotV2) -> String {
    let text = serde_json::to_string(&doc.entities).expect("serializes");
    let mut list: Vec<Value> = serde_json::from_str(&text).expect("parses");
    for (i, e) in list.iter_mut().enumerate() {
        e["id"] = json!(i + 1);
    }
    doc.entities = serde_json::from_value(Value::Array(list)).expect("objects");
    serde_json::to_string(&doc).expect("serializes")
}

#[test]
fn random_drawings_round_trip_bit_for_bit_and_write_the_same_bytes_again() {
    let mut rng = Rng(20_260_926);
    for round in 0..600 {
        let doc = drawing(&mut rng);
        let bytes = kentos_kcad::encode_verified(&doc)
            .unwrap_or_else(|e| panic!("round {round}: {} {e}", e.code.as_str()));
        let back = kentos_kcad::decode(&bytes).expect("reads back");
        assert_eq!(
            serde_json::to_string(&back).expect("serializes"),
            with_read_slots(doc),
            "round {round}"
        );
        assert!(
            kentos_kcad::encode(&back).expect("writes") == bytes,
            "round {round}: not deterministic"
        );
    }
}

#[test]
fn the_writer_refuses_what_a_reader_would_refuse() {
    let mut rng = Rng(5);
    let mut good = drawing(&mut rng);
    while good.entities.len() < 2 {
        good = drawing(&mut rng);
    }
    let refuse = |change: &dyn Fn(&mut DocumentSnapshotV2), code: Code| {
        let mut doc = good.clone();
        change(&mut doc);
        let e = kentos_kcad::encode(&doc).expect_err("refused");
        assert_eq!(e.code, code, "{e}");
        assert!(
            e.message.starts_with("Çizim KCAD 2 olarak yazılamıyor"),
            "{e}"
        );
    };
    refuse(&|d| d.origin.x = f64::NAN, Code::NonFinite);
    refuse(&|d| d.settings.plot_scale = f64::INFINITY, Code::NonFinite);
    refuse(&|d| d.uids[1] = d.uids[0], Code::DuplicateUid);
    refuse(&|d| d.uids[0] = EntityId([0; 16]), Code::BadValue);
    refuse(
        &|d| {
            d.uids.pop();
        },
        Code::BadValue,
    );
    refuse(&|d| d.project_id = Some(ProjectId([0; 16])), Code::BadValue);
    refuse(&|d| d.version = 1, Code::BadValue);
    refuse(
        &|d| d.migrated_from = Some(MigrationSource::v1("ABCD".to_owned())),
        Code::BadValue,
    );
    refuse(
        &|d| d.layers[0].style.renderer = Some(Value::Null),
        Code::BadValue,
    );
    refuse(
        &|d| {
            d.entities[0] = Entity::Polyline(PathEntity {
                base: d.entities[0].base().clone(),
                pts: vec![Vec2 { x: 0.0, y: 0.0 }; 2],
                bulges: None,
                holes: Some(vec![]),
            })
        },
        Code::BadValue,
    );
    refuse(
        &|d| {
            let mut deep = json!([]);
            for _ in 0..70 {
                deep = json!([deep]);
            }
            d.styles.items = vec![deep];
        },
        Code::TooDeep,
    );
}

#[test]
fn a_declared_length_never_reserves_more_than_the_file_holds() {
    // An entities array claiming 2²⁴ objects in a 16 MB payload of `null`s: refused for the
    // first `null`, never reserving 2²⁴ objects (≈ 4 GB) ahead.
    let file = fixture("minimal.kcad");
    let payload = payload_of(&file);
    let key = b"\x68entities";
    let at = payload
        .windows(key.len())
        .position(|w| w == key)
        .expect("entities key")
        + key.len();
    let mut p = payload[..at].to_vec();
    p.extend_from_slice(&[0x9a, 0x01, 0x00, 0x00, 0x00]);
    p.resize(p.len() + (1 << 24), 0xf6);
    let e = kentos_kcad::decode(&container(&p)).expect_err("refused");
    assert_eq!(e.code, Code::WrongType, "{e}");
}
