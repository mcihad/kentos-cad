//! Yinelenenleri sil (the web's `builtin/layers/dedupe.ts`; docs/adr/0217
//! §5; QGIS Delete duplicates by attribute, ArcGIS Delete Identical): of the
//! objects whose key is the same the first (in the input's order) stays, the
//! rest go. A key is the fields' values trimmed (with no field named: every
//! attribute with a value), and with Geometri de aynı olsun the kind and the
//! shape exactly; the shape is read only for objects whose values and first
//! place another object shares, hashed and then compared. One undo step.

use std::collections::{BTreeMap, HashMap};

use kentos_commands::{Entity, EntityBase};
use kentos_domain::Slot;
use serde_json::{Value, json};

use super::{fields_value, input_param};
use crate::parameters::field_names;
use crate::text::js_trim;
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext,
    RunResult, ScopeKind, Target, Tool,
};

pub fn tool() -> Tool {
    Tool {
        id: "layers.deleteDuplicates".into(),
        label: "Yinelenenleri sil".into(),
        category: "layers".into(),
        description: "Aynı öznitelikli (isterse aynı geometrili) nesnelerden ilkini bırakıp öbürlerini siler.".into(),
        help: Some([
            "Örnekler: iki kez içe aktarılmış parselleri temizlemek; aynı numaralı noktalardan yalnız ilkini bırakmak.",
            "Alanlar boşsa bütün öznitelikler karşılaştırılır (değeri boş alanlar yok sayılır). Değerler baştaki ve sondaki boşluklar atılarak karşılaştırılır. Geometri de aynı olsun açıkken türü ve şekli (köşeler, yaylar, kotlar) tam aynı olanlar yinelenen sayılır. Girdinin sırasıyla ilk nesne kalır.",
        ]
        .join("\n\n")),
        keywords: [
            "yinelenen", "tekrar", "çift", "sil", "temizle", "duplicate", "delete duplicates",
            "delete identical",
        ]
        .map(String::from)
        .to_vec(),
        aliases: vec!["YINELENENSIL".into()],
        icon: Some("deleteDuplicates".into()),
        parameters: vec![
            input_param(
                vec![
                    ScopeKind::Layer,
                    ScopeKind::Selection,
                    ScopeKind::Visible,
                    ScopeKind::All,
                ],
                "Yinelenenleri silinecek nesneler; kilitli katmandakiler alınmaz.",
            ),
            ParamDef::new(
                "fields",
                "Alanlar",
                ParamKind::Field {
                    of: vec!["input".into()],
                    allow_new: false,
                    multiple: true,
                },
            )
            .optional()
            .describe("Boş bırakılırsa bütün öznitelikler."),
            ParamDef::new("geometry", "Geometri de aynı olsun", ParamKind::Boolean)
                .default_value(json!(true))
                .describe("Türü ve şekli de aynı olanlar yinelenen sayılır."),
        ],
        outputs: vec![
            OutputDef::new("table", "Yinelenenler", OutputKind::Table),
            OutputDef::new("count", "Silinen nesne sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

/// An object's values as its key: the fields' values trimmed, or with no
/// field named every attribute with a value as name and value in turn, in the
/// names' code point order (the map's).
fn values_of<'a>(e: &'a Entity, names: &[String]) -> Vec<&'a str> {
    let attrs = &e.base().attrs;
    if !names.is_empty() {
        return names
            .iter()
            .map(|f| js_trim(attrs.get(f).map_or("", String::as_str)))
            .collect();
    }
    let mut values = Vec::with_capacity(attrs.len() * 2);
    for (k, x) in attrs {
        let x = js_trim(x);
        if !x.is_empty() {
            values.extend([k.as_str(), x]);
        }
    }
    values
}

/// The values as the table shows them.
fn shown(e: &Entity, names: &[String]) -> String {
    if !names.is_empty() {
        return fields_value(e, names);
    }
    let pairs = values_of(e, names);
    if pairs.is_empty() {
        return "—".into();
    }
    pairs
        .chunks(2)
        .map(|p| format!("{}: {}", p[0], p[1]))
        .collect::<Vec<_>>()
        .join("; ")
}

/// The object without what every object has: its kind and shape.
fn shape_of(e: &Entity) -> Entity {
    let mut shape = e.clone();
    *shape.base_mut() = EntityBase {
        look: Default::default(),
        id: 0,
        layer_id: String::new(),
        color: None,
        attrs: BTreeMap::new(),
        label: None,
        symbol: None,
        line_weight: None,
        label_pins: Vec::new(),
        attachments: Vec::new(),
    };
    shape
}

/// What is written hashed (FNV-1a), no text kept: the shape's JSON. A zero's
/// two signs are equal and hash alike (“-0.0” as “0.0”; the web's JSON writes
/// both as 0); objects of one hash are told apart by their shapes.
struct Hashing(u64);

impl std::io::Write for Hashing {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let written = if bytes == b"-0.0" { b"0.0" } else { bytes };
        for &b in written {
            self.0 = (self.0 ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3);
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// What equal shapes share, read at once: the kind and the first place, as
/// numbers (a zero's two signs are one).
fn rough(e: &Entity) -> (std::mem::Discriminant<Entity>, usize, u64, u64) {
    let (n, at) = match e {
        Entity::Point(p) => (0, Some(p.p)),
        Entity::Line(l) => (0, Some(l.a)),
        Entity::Polyline(p) | Entity::Polygon(p) => (p.pts.len(), p.pts.first().copied()),
        Entity::Circle(c) => (0, Some(c.c)),
        Entity::Arc(a) => (0, Some(a.c)),
        Entity::Text(t) => (0, Some(t.p)),
        _ => (0, None),
    };
    let bits = |v: f64| (v + 0.0).to_bits();
    let (x, y) = at.map_or((0, 0), |p| (bits(p.x), bits(p.y)));
    (std::mem::discriminant(e), n, x, y)
}

fn shape_hash(e: &Entity) -> u64 {
    let mut hashing = Hashing(0xcbf2_9ce4_8422_2325);
    let _ = serde_json::to_writer(&mut hashing, &shape_of(e));
    hashing.0
}

fn run(v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    let names = field_names(true, v.text("fields"));
    let geometry = v.flag("geometry");
    feedback.progress(0.0, "Anahtarlar karşılaştırılıyor");
    // The objects' classes of values, in the order they first come.
    let mut class_of = Vec::with_capacity(inputs.len());
    let mut sizes: Vec<u32> = Vec::new();
    {
        let mut classes: HashMap<Vec<&str>, u32> = HashMap::with_capacity(inputs.len());
        for e in inputs {
            let next = sizes.len() as u32;
            let c = *classes.entry(values_of(e, &names)).or_insert(next);
            if c == next {
                sizes.push(0);
            }
            sizes[c as usize] += 1;
            class_of.push(c);
        }
    }
    // An object alone with its values has no duplicate; with Geometri de aynı
    // olsun nor has one alone with its values and what equal shapes share
    // (`rough`): the shape of neither is read.
    let mut candidate: Vec<bool> = class_of.iter().map(|&c| sizes[c as usize] >= 2).collect();
    if geometry {
        let keys: Vec<_> = inputs
            .iter()
            .zip(&class_of)
            .map(|(e, &c)| (c, rough(e)))
            .collect();
        let mut counts: HashMap<_, u32> = HashMap::new();
        for (k, &is) in keys.iter().zip(&candidate) {
            if is {
                *counts.entry(*k).or_default() += 1;
            }
        }
        for (k, is) in keys.iter().zip(candidate.iter_mut()) {
            *is = *is && counts.get(k).copied().unwrap_or(0) >= 2;
        }
    }
    // Of a class and shape the first object stays.
    let mut first: HashMap<(u32, u64), usize> = HashMap::new();
    let mut others: HashMap<(u32, u64), Vec<usize>> = HashMap::new();
    let mut groups: Vec<(usize, usize)> = Vec::new();
    let mut group_at: HashMap<usize, usize> = HashMap::new();
    let mut remove = Vec::new();
    for (i, e) in inputs.iter().enumerate() {
        if !candidate[i] {
            continue;
        }
        let c = class_of[i];
        let key = (c, if geometry { shape_hash(e) } else { 0 });
        let kept = match first.get(&key) {
            None => {
                first.insert(key, i);
                continue;
            }
            Some(&k) if !geometry || shape_of(inputs[k]) == shape_of(e) => Some(k),
            // One hash, other shapes: the rare ones each have their own first.
            Some(_) => {
                let shape = shape_of(e);
                let list = others.entry(key).or_default();
                let found = list.iter().copied().find(|&k| shape_of(inputs[k]) == shape);
                if found.is_none() {
                    list.push(i);
                }
                found
            }
        };
        let Some(k) = kept else { continue };
        remove.push(Slot(e.base().id));
        let g = *group_at.entry(k).or_insert_with(|| {
            groups.push((k, 0));
            groups.len() - 1
        });
        groups[g].1 += 1;
    }
    let rows: Vec<Value> = groups
        .iter()
        .map(|&(k, n)| json!([shown(inputs[k], &names), n.to_string()]))
        .collect();
    let table = json!({ "columns": ["Değer", "Silinen"], "rows": rows });
    let count = remove.len();
    if count == 0 {
        return RunResult {
            outputs: [("table".to_owned(), table), ("count".to_owned(), json!(0))]
                .into_iter()
                .collect(),
            summary: Some("Yinelenen nesne yok.".into()),
            ..RunResult::default()
        };
    }
    RunResult {
        changes: Some(ChangeSet {
            remove,
            ..ChangeSet::default()
        }),
        outputs: [
            ("table".to_owned(), table),
            ("count".to_owned(), json!(count)),
        ]
        .into_iter()
        .collect(),
        summary: Some(format!(
            "{} grupta {count} yinelenen nesne silindi.",
            groups.len()
        )),
        ..RunResult::default()
    }
}
