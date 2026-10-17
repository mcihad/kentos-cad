//! Veri aktar (the web's `builtin/transfer.ts`; docs/adr/0216; ArcGIS Append
//! with matching fields, QGIS Refactor fields and Join attributes by field
//! value, Netcad Veri Aktar): a source's rows (another layer's objects, or a
//! CSV, TXT or Excel file's rows) go into the target layer's objects whose
//! key matches, through a field map (each target field's source column or
//! default), typed fields by their rules; optionally the source objects'
//! geometry, and the rows no object took as new objects. A report says what
//! happened to each object and row; Yalnız raporla changes nothing. Keys
//! match as Anahtarla birleştir's (`ops::statistics::join_plan`). One undo
//! step.

use std::collections::{BTreeMap, HashSet};

use kentos_contracts::fields::{LayerField, LayerFieldKind, check_value};
use kentos_contracts::{Entity, EntityBase, EntityGeometry, Vec2};
use kentos_domain::Slot;
use kentos_geometry_core::ops::statistics::join_plan;
use kentos_native_application::geometry::entity_of;
use serde_json::{Value, json};

use super::join_source::{missing, source_table};
use super::queries::attr;
use crate::field_map::{self, MapRow};
use crate::text::js_trim;
use crate::types::{
    ChangeSet, EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Patch, Resolved,
    RunContext, RunResult, ScopeKind, Target, Tool,
};

/// The report's columns.
const COLUMNS: [&str; 4] = ["Anahtar", "Sonuç", "Değişiklikler", "Not"];

/// The most rows the report has: past it the run is refused.
const MOST_ROWS: usize = 1_000_000;

fn from_layer(v: &crate::Values) -> bool {
    v.get("sourceKind").and_then(Value::as_str) == Some("layer")
}

pub fn tool() -> Tool {
    Tool {
        id: "attributes.transfer".into(),
        label: "Veri aktar".into(),
        category: "attributes".into(),
        description: "Başka bir katmandan ya da CSV, TXT, Excel dosyasından ortak anahtarla öznitelikleri alan eşlemesiyle aktarır; isterse geometriyi günceller, eşleşmeyenleri ekler.".into(),
        help: Some([
            "Örnekler: tapu listesindeki malik, hisse ve yüzölçümünü parsel numarasıyla parsellere aktarmak; başka bir ekibin güncel ölçümünü mevcut katmana geometrisiyle işlemek, yeni parselleri eklemek; koordinat listesinden noktaları adlarıyla güncellemek ve yenilerini eklemek.",
            "Anahtarlar baştaki ve sondaki boşluklar atılarak karşılaştırılır; iki taraf da sayıysa sayı olarak (002 ile 2 aynı). Kaynakta yinelenen anahtarda Yinelenen anahtar ne olacağını söyler: ilk satırı alınır, bu anahtarlı hedeflere hiçbir şey aktarılmaz (satırları eklenmez) ya da araç çalışmaz; hedefte aynı anahtarlı nesnelerin hepsi güncellenir.",
            "Alan eşleme her hedef alana bir kaynak sütun ya da varsayılan değer verir: hücre boşsa varsayılan yazılır. Eşleme boşsa anahtardan başka bütün sütunlar kendi adlarıyla aktarılır. Hedef katmanın alanı tipliyse değer alanın kuralıyla yazılır; uymayan değer yazılmaz ve raporda nedeni söylenir.",
            "Eşleşmeyenleri ekle açıkken hiçbir nesnenin almadığı satırlar hedef katmana eklenir: katmandan geometrisiyle, dosyadan Doğu ve Kuzey sütunlarından nokta olarak; iki sütun da boşsa geometrisiz kayıt (tablo satırı) olarak. Yalnız raporla açıkken çizim değişmez; rapor ne olacağını gösterir.",
        ]
        .join("\n\n")),
        keywords: [
            "veri aktar", "aktar", "anahtar", "alan eşleme", "eşle", "güncelle", "ekle", "append",
            "upsert", "field map", "refactor", "tapu", "csv", "excel",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["VERIAKTAR", "EKLEGUNCELLE"].map(String::from).to_vec(),
        icon: Some("dataTransfer".into()),
        parameters: vec![
            ParamDef::new(
                "target",
                "Hedef katman",
                ParamKind::Features {
                    kinds: None,
                    scopes: Some(vec![ScopeKind::Layer]),
                    writes: true,
                },
            )
            .allow_empty()
            .describe("Güncellenecek nesnelerin katmanı; eşleşmeyen satırlar da buna eklenir."),
            ParamDef::new(
                "targetKey",
                "Hedef anahtar",
                ParamKind::Field {
                    of: vec!["target".into()],
                    allow_new: true,
                    multiple: false,
                },
            )
            .describe("Hedef nesnelerin anahtar alanı (parsel numarası, kimlik)."),
            ParamDef::new(
                "sourceKind",
                "Kaynak",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("file", "Dosya").hint("CSV, TXT ya da Excel dosyası"),
                        EnumOption::new("layer", "Katman")
                            .hint("Başka nesnelerin öznitelikleri ve geometrisi"),
                    ],
                },
            )
            .default_value(json!("file")),
            ParamDef::new(
                "file",
                "Dosya",
                ParamKind::File {
                    accept: [".csv", ".txt", ".xlsx", ".ods"].map(String::from).to_vec(),
                },
            )
            .shown_when(|v| !from_layer(v))
            .describe("İlk satırı sütun adları olan CSV, TXT, Excel (.xlsx) ya da ODS dosyası."),
            ParamDef::new(
                "source",
                "Kaynak nesneler",
                ParamKind::Features {
                    kinds: None,
                    scopes: Some(vec![
                        ScopeKind::Layer,
                        ScopeKind::All,
                        ScopeKind::Visible,
                        ScopeKind::Selection,
                    ]),
                    writes: false,
                },
            )
            .shown_when(from_layer)
            .describe("Özniteliklerin ve geometrinin alınacağı nesneler."),
            ParamDef::new(
                "sourceKey",
                "Kaynak anahtar",
                ParamKind::Field {
                    of: vec!["source".into(), "file".into()],
                    allow_new: false,
                    multiple: false,
                },
            )
            .optional()
            .describe("Boş bırakılırsa hedef anahtarla aynı ad."),
            ParamDef::new(
                "map",
                "Alan eşleme",
                ParamKind::FieldMap {
                    target: "target".into(),
                    source: vec!["source".into(), "file".into()],
                    key: vec!["sourceKey".into(), "targetKey".into()],
                },
            )
            .describe("Her hedef alana hangi kaynak sütun ya da hücre boşken hangi değer; boşsa anahtardan başka bütün sütunlar kendi adlarıyla."),
            ParamDef::new(
                "existing",
                "Var olan değerler",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("overwrite", "Üzerine yaz").hint(
                            "Hedefteki değer kaynağınkiyle değişir; değeri olmayan alan boşalır",
                        ),
                        EnumOption::new("empty", "Yalnız boşlara")
                            .hint("Hedefte dolu olan değer kalır"),
                    ],
                },
            )
            .default_value(json!("overwrite")),
            super::join_source::duplicates_param(),
            ParamDef::new("geometry", "Geometriyi güncelle", ParamKind::Boolean)
                .default_value(json!(false))
                .shown_when(from_layer)
                .describe("Eşleşen nesne kaynağın geometrisini alır (türleri aynıysa)."),
            ParamDef::new("append", "Eşleşmeyenleri ekle", ParamKind::Boolean)
                .default_value(json!(false))
                .describe("Hiçbir nesnenin almadığı satırlar hedef katmana yeni nesne olarak eklenir; dosyadan, Doğu ve Kuzey sütunları boşsa geometrisiz kayıt olarak."),
            coordinate("east", "Doğu sütunu", "Eklenen noktanın doğu koordinatı (CBS projesinde Y)."),
            coordinate("north", "Kuzey sütunu", "Eklenen noktanın kuzey koordinatı (CBS projesinde X)."),
            ParamDef::new("reportOnly", "Yalnız raporla", ParamKind::Boolean)
                .default_value(json!(false))
                .describe("Çizim değişmez; rapor ne olacağını gösterir."),
        ],
        outputs: vec![
            OutputDef::new("table", "Aktarım raporu", OutputKind::Table),
            OutputDef::new("changed", "Güncellenen nesneler", OutputKind::Features),
            OutputDef::new("count", "Güncellenen nesne sayısı", OutputKind::Number),
            OutputDef::new("added", "Eklenen nesne sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

/// A file's coordinate column for the points a run adds.
fn coordinate(name: &str, label: &str, description: &str) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Field {
            of: vec!["file".into()],
            allow_new: false,
            multiple: false,
        },
    )
    .optional()
    .shown_when(|v| !from_layer(v) && v.get("append").and_then(Value::as_bool) == Some(true))
    .describe(description)
}

/// An attribute's value as a change shows it: none or empty is “(boş)”.
fn shown(v: Option<&str>) -> &str {
    match v {
        None | Some("") => "(boş)",
        Some(v) => v,
    }
}

/// The map the run uses: the one given, else every source column but the key under its own name.
fn map_of(map: Vec<MapRow>, header: &[String], k: usize) -> Vec<MapRow> {
    if !map.is_empty() {
        return map;
    }
    header
        .iter()
        .enumerate()
        .filter(|(i, h)| {
            *i != k && !h.is_empty() && header.iter().position(|x| x == *h) == Some(*i)
        })
        .map(|(_, h)| MapRow::new(h, h, ""))
        .collect()
}

/// An object of `shape`'s kind and shape with the fields every object has from `base`.
fn shaped(shape: &Entity, base: EntityBase) -> Entity {
    let mut e = shape.clone();
    *e.base_mut() = base;
    e
}

/// A map's row as the run reads it: its target trimmed, its source's column, its default as typed.
struct Mapped<'a> {
    target: &'a str,
    column: Option<usize>,
    default: &'a str,
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let target = v.features("target");
    let targets = &target.entities;
    let layer_id = target
        .layer_id
        .clone()
        .or_else(|| targets.first().map(|t| t.base().layer_id.clone()))
        .unwrap_or_default();
    let from_file = !from_layer(&v.values);
    let objects: Option<&[&Entity]> =
        (!from_file).then(|| v.features("source").entities.as_slice());
    let (header, rows) = if from_file {
        source_table(None, v.values.get("file"))
    } else {
        source_table(objects, None)
    };
    let target_key = v.text("targetKey");
    let key = match js_trim(v.text("sourceKey")) {
        "" => target_key,
        name => name,
    };
    let Some(k) = header.iter().position(|h| h == key) else {
        return RunResult::refused(missing(key, &header));
    };
    let given = v
        .values
        .get("map")
        .and_then(field_map::read)
        .unwrap_or_default();
    let map = map_of(given, &header, k);
    let mut mapped = Vec::with_capacity(map.len());
    for m in &map {
        let source = js_trim(&m.source);
        let column = if source.is_empty() {
            None
        } else {
            match header.iter().position(|h| h == source) {
                Some(i) => Some(i),
                None => return RunResult::refused(missing(source, &header)),
            }
        };
        mapped.push(Mapped {
            target: js_trim(&m.target),
            column,
            default: &m.default,
        });
    }
    let append = v.flag("append");
    let east = js_trim(v.text("east"));
    let north = js_trim(v.text("north"));
    let mut corner = (0, 0);
    // Neither column chosen: a file's new rows are records without a geometry (docs/adr/0410 §6).
    let records = append && from_file && east.is_empty() && north.is_empty();
    if append && from_file && !records {
        if east.is_empty() || north.is_empty() {
            return RunResult::refused(
                "Doğu ve Kuzey sütunlarının ikisini de seçin; ikisi de boşsa satırlar geometrisiz kayıt olarak eklenir.".into(),
            );
        }
        for (c, at) in [(east, &mut corner.0), (north, &mut corner.1)] {
            match header.iter().position(|h| h == c) {
                Some(i) => *at = i,
                None => return RunResult::refused(missing(c, &header)),
            }
        }
    }
    feedback.progress(0.0, "Anahtarlar eşleniyor");
    let cell = |r: usize, i: usize| rows[r].get(i).copied().unwrap_or("");
    let value = |r: usize, m: &Mapped<'_>| -> Option<String> {
        if let Some(i) = m.column {
            let c = cell(r, i);
            if !js_trim(c).is_empty() {
                return Some(c.to_owned());
            }
        }
        (!js_trim(m.default).is_empty()).then(|| m.default.to_owned())
    };
    let target_keys: Vec<Option<&str>> = targets.iter().map(|t| attr(t, target_key)).collect();
    let source_keys: Vec<Option<&str>> = (0..rows.len()).map(|r| Some(cell(r, k))).collect();
    let skip = v.text("duplicates") == "skip";
    let (plan, repeated_keys) = match super::join_source::with_policy(
        join_plan(&target_keys, &source_keys),
        &target_keys,
        &source_keys,
        v.text("duplicates"),
    ) {
        Ok(p) => p,
        Err(why) => return RunResult::refused(why),
    };
    let is_repeated = |text: &str| {
        skip && kentos_geometry_core::ops::statistics::join_key(text)
            .is_some_and(|k| repeated_keys.contains(&k))
    };
    let fields = ctx.fields(&layer_id);
    let field_of = |name: &str| fields.iter().find(|f| f.name == name);
    let mut report: Vec<[String; 4]> = Vec::new();
    let mut update: Vec<Patch> = Vec::new();
    let mut add: Vec<Entity> = Vec::new();
    let mut misfits = 0usize;
    let mut unmatched = 0usize;
    let only_empty = v.text("existing") == "empty";
    let geometry = v.flag("geometry");
    for (t, m) in targets.iter().zip(&plan.matches) {
        let key_text = js_trim(attr(t, target_key).unwrap_or("")).to_owned();
        let Some(r) = *m else {
            unmatched += 1;
            let note = if key_text.is_empty() {
                "Anahtarı boş."
            } else if is_repeated(&key_text) {
                "Anahtar kaynakta yineleniyor; aktarılmadı."
            } else {
                ""
            };
            report.push([key_text, "Eşleşmedi".into(), String::new(), note.into()]);
            continue;
        };
        let mut attrs = t.base().attrs.clone();
        let mut changes: Vec<String> = Vec::new();
        let mut notes: Vec<String> = Vec::new();
        for m in &mapped {
            let name = m.target;
            let old = attrs.get(name).cloned();
            if only_empty && old.as_deref().is_some_and(|o| !js_trim(o).is_empty()) {
                continue;
            }
            let field = field_of(name);
            let Some(mut text) = value(r, m) else {
                let Some(old) = old else {
                    continue;
                };
                if let Some(Err(why)) = field.map(|f| check_value(f, "")) {
                    notes.push(why.message);
                    misfits += 1;
                    continue;
                }
                attrs.remove(name);
                changes.push(format!("{name}: {} → (boş)", shown(Some(&old))));
                continue;
            };
            if let Some(f) = field {
                match check_value(f, &text) {
                    Ok(canonical) => text = canonical,
                    Err(why) => {
                        notes.push(why.message);
                        misfits += 1;
                        continue;
                    }
                }
            }
            if old.as_deref() == Some(text.as_str()) {
                continue;
            }
            changes.push(format!("{name}: {} → {text}", shown(old.as_deref())));
            attrs.insert(name.to_owned(), text);
        }
        let mut shape: Option<Box<Entity>> = None;
        if let (true, Some(objects)) = (geometry, objects) {
            let src = objects[r];
            if src.kind() != t.kind() {
                notes.push("Geometrinin türü farklı; geometri güncellenmedi.".into());
            } else if shaped(src, t.base().clone()) != **t {
                // The source's shape whole: what the target had and the source has not goes.
                shape = Some(Box::new((*src).clone()));
                changes.push("geometri".into());
            }
        }
        let result = if changes.is_empty() {
            "Değişmedi"
        } else {
            update.push(Patch {
                id: Slot(t.base().id),
                attrs: (attrs != t.base().attrs).then_some(attrs),
                label: None,
                zs: None,
                geometry: shape,
                layer: None,
            });
            "Güncellendi"
        };
        report.push([key_text, result.into(), changes.join("; "), notes.join(" ")]);
    }
    let updated = update.len();
    // The source rows no object took, in their order; a row's key matched against the rows themselves gives the
    // first row of that key (one pass, the core's rule).
    let used: HashSet<usize> = plan.matches.iter().flatten().copied().collect();
    let firsts = join_plan(&source_keys, &source_keys).matches;
    let mut not_added = 0usize;
    let mut unused_rows = 0usize;
    let point_field = |name: &str| LayerField::new(name, LayerFieldKind::Decimal);
    for r in 0..rows.len() {
        if used.contains(&r) {
            continue;
        }
        let key_text = js_trim(cell(r, k)).to_owned();
        if key_text.is_empty() {
            unused_rows += 1;
            report.push([
                String::new(),
                "Kullanılmadı".into(),
                String::new(),
                "Anahtarı boş.".into(),
            ]);
            continue;
        }
        if is_repeated(&key_text) {
            unused_rows += 1;
            report.push([
                key_text,
                "Kullanılmadı".into(),
                String::new(),
                "Anahtar kaynakta yineleniyor; aktarılmadı.".into(),
            ]);
            continue;
        }
        if firsts.get(r).copied().flatten() != Some(r) {
            unused_rows += 1;
            report.push([
                key_text,
                "Kullanılmadı".into(),
                String::new(),
                "Anahtar kaynakta yineleniyor; ilk satır alındı.".into(),
            ]);
            continue;
        }
        if !append {
            unused_rows += 1;
            report.push([
                key_text,
                "Kullanılmadı".into(),
                String::new(),
                String::new(),
            ]);
            continue;
        }
        // A new object, in this order: the key, the map's values, the layer fields' defaults, the required
        // fields, the geometry; a key its field does not take, a required field left empty or an unread point
        // leaves it out.
        let mut attrs: BTreeMap<String, String> = BTreeMap::new();
        let mut written: Vec<String> = Vec::new();
        let mut notes: Vec<String> = Vec::new();
        let mut ok = true;
        let key_value = match field_of(target_key) {
            Some(f) => check_value(f, &key_text),
            None => Ok(key_text.clone()),
        };
        match key_value {
            Ok(canonical) => {
                written.push(format!("{target_key}: {canonical}"));
                attrs.insert(target_key.to_owned(), canonical);
            }
            Err(why) => {
                notes.push(why.message);
                ok = false;
            }
        }
        for m in &mapped {
            let name = m.target;
            if name == target_key {
                continue;
            }
            let Some(mut text) = value(r, m) else {
                continue;
            };
            if let Some(f) = field_of(name) {
                match check_value(f, &text) {
                    Ok(canonical) => text = canonical,
                    Err(why) => {
                        notes.push(why.message);
                        misfits += 1;
                        continue;
                    }
                }
            }
            written.push(format!("{name}: {text}"));
            attrs.insert(name.to_owned(), text);
        }
        for f in fields {
            let default = f.default.as_deref().unwrap_or("");
            if attrs.contains_key(&f.name) || js_trim(default).is_empty() {
                continue;
            }
            if let Ok(canonical) = check_value(f, default)
                && !canonical.is_empty()
            {
                written.push(format!("{}: {canonical}", f.name));
                attrs.insert(f.name.clone(), canonical);
            }
        }
        for f in fields {
            if f.required
                && f.name != target_key
                && js_trim(attrs.get(&f.name).map_or("", String::as_str)).is_empty()
            {
                if let Err(why) = check_value(f, "") {
                    notes.push(why.message);
                }
                ok = false;
            }
        }
        let base = EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: layer_id.clone(),
            color: None,
            attrs,
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        };
        let made = match objects {
            Some(objects) => Some(shaped(objects[r], base)),
            None if records => Some(Entity::Record(kentos_contracts::RecordEntity { base })),
            None => {
                let (x, y) = (js_trim(cell(r, corner.0)), js_trim(cell(r, corner.1)));
                let read = |text: &str, name: &str| {
                    check_value(&point_field(name), text)
                        .ok()
                        .filter(|_| !text.is_empty())
                        .and_then(|c| c.parse::<f64>().ok())
                };
                match (read(x, "Doğu"), read(y, "Kuzey")) {
                    (Some(x), Some(y)) => Some(entity_of(
                        &EntityGeometry::Point {
                            p: Vec2 { x, y },
                            z: None,
                            parts: None,
                        },
                        base,
                    )),
                    _ => {
                        notes.push("Doğu ya da kuzey okunamadı; nokta eklenmedi.".into());
                        ok = false;
                        None
                    }
                }
            }
        };
        match made.filter(|_| ok) {
            Some(e) => {
                add.push(e);
                report.push([
                    key_text,
                    "Eklendi".into(),
                    written.join("; "),
                    notes.join(" "),
                ]);
            }
            None => {
                not_added += 1;
                report.push([key_text, "Eklenmedi".into(), String::new(), notes.join(" ")]);
            }
        }
    }
    if report.len() > MOST_ROWS {
        return RunResult::refused(format!(
            "Rapor {} satır olurdu (en çok {MOST_ROWS}); kaynağı ya da hedefi daraltın.",
            report.len()
        ));
    }
    if plan.repeated > 0 {
        feedback.warn(if skip {
            format!(
                "{} anahtar kaynakta birden çok kez var; bu anahtarlı hedeflere bir şey aktarılmadı.",
                plan.repeated
            )
        } else {
            format!(
                "{} anahtar kaynakta birden çok kez var; ilk satırları alındı.",
                plan.repeated
            )
        });
    }
    if misfits > 0 {
        feedback.warn(format!(
            "{misfits} değer alanına uymadığı için yazılmadı; nedenleri raporda."
        ));
    }
    if not_added > 0 {
        feedback.warn(format!(
            "{not_added} kaynak satırı eklenemedi; nedenleri raporda."
        ));
    }
    let added = add.len();
    let summary = format!(
        "{updated} nesne güncellendi, {added} eklendi; {unmatched} hedefin eşi yok, {} kaynak satırı kullanılmadı.",
        unused_rows + not_added
    );
    // The rows' texts moved into the table, not copied: a report has tens of thousands.
    let rows: Vec<Value> = report
        .into_iter()
        .map(|r| Value::Array(r.into_iter().map(Value::String).collect()))
        .collect();
    let table = json!({ "columns": COLUMNS, "rows": rows });
    if v.flag("reportOnly") {
        return RunResult {
            outputs: [
                ("table".to_owned(), table),
                ("count".to_owned(), json!(0)),
                ("added".to_owned(), json!(0)),
            ]
            .into_iter()
            .collect(),
            summary: Some(format!("Yalnız rapor, çizim değişmedi: {summary}")),
            ..RunResult::default()
        };
    }
    let changed: Vec<u32> = update.iter().map(|u| u.id.0).collect();
    RunResult {
        changes: Some(ChangeSet {
            update,
            add,
            ..ChangeSet::default()
        }),
        outputs: [
            ("table".to_owned(), table),
            ("changed".to_owned(), json!(changed)),
            ("count".to_owned(), json!(updated)),
            ("added".to_owned(), json!(added)),
        ]
        .into_iter()
        .collect(),
        summary: Some(summary),
        ..RunResult::default()
    }
}
