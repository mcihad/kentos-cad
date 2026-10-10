//! Vektör katmanlarını birleştir (the web's `builtin/layers/merge.ts`;
//! docs/adr/0217 §3; QGIS Merge vector layers, ArcGIS Merge): the chosen
//! layers' objects copied into one layer, the layers in the tree's order,
//! each in the drawing's order; a field map (docs/adr/0216 §2) gives the
//! copies' attributes, else they keep their own; the source layer's name
//! goes into a field. Into a layer with typed fields the values go by their
//! rules, its defaults fill what is left empty, and an object with a required
//! field left empty is not added. One undo step.

use std::collections::BTreeMap;

use kentos_contracts::fields::check_value;
use serde_json::json;

use super::{copy_of, output_param};
use crate::field_map::{self, MapRow};
use crate::text::js_trim;
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext,
    RunResult, Target, Tool,
};

pub fn tool() -> Tool {
    Tool {
        id: "layers.merge".into(),
        label: "Vektör katmanlarını birleştir".into(),
        category: "layers".into(),
        description: "Seçilen katmanların nesnelerini alan eşlemesiyle tek katmanda toplar; kaynak katmanın adını bir alana yazar.".into(),
        help: Some([
            "Örnekler: paftalara bölünmüş parsel katmanlarını tek parsel katmanında toplamak; farklı ekiplerin bina katmanlarını ortak alan adlarıyla birleştirmek.",
            "Nesneler kopyalanır, girdiler değişmez: katmanlar ağaçtaki sırasıyla, her katmanda çizim sırasıyla. Alan eşleme boşsa nesneler bütün öznitelikleriyle gelir; doluysa yalnız eşlemenin alanlarıyla (hücre boşsa varsayılan). Kaynak katman alanı doluysa nesnenin geldiği katmanın adı oraya yazılır.",
            "Sonuç alanları tipli bir katmansa değerler alanların kuralıyla yazılır: uymayan değer yazılmaz, katmanın varsayılanları boş alanlara gelir, zorunlu alanı boş kalan nesne eklenmez. Kilitli katmanın nesneleri alınmaz.",
        ]
        .join("\n\n")),
        keywords: [
            "birleştir", "katman", "topla", "alan eşleme", "merge", "merge vector layers",
            "append", "pafta",
        ]
        .map(String::from)
        .to_vec(),
        aliases: vec!["KATMANTOPLA".into()],
        icon: Some("layersMergeVector".into()),
        parameters: vec![
            ParamDef::new("layers", "Katmanlar", ParamKind::Layers { min: 2 })
                .describe("Birleştirilecek katmanlar; ağaçtaki sırasıyla okunur."),
            ParamDef::new(
                "map",
                "Alan eşleme",
                ParamKind::FieldMap {
                    target: "layers".into(),
                    source: vec!["layers".into()],
                    key: Vec::new(),
                },
            )
            .describe("Her sonuç alanına hangi kaynak alan ya da hücre boşken hangi değer; boşsa bütün öznitelikler kendi adlarıyla."),
            ParamDef::new(
                "sourceField",
                "Kaynak katman alanı",
                ParamKind::Field {
                    of: vec!["layers".into()],
                    allow_new: true,
                    multiple: false,
                },
            )
            .optional()
            .default_value(json!("Kaynak katman"))
            .describe("Nesnenin geldiği katmanın adı bu alana yazılır; boşsa yazılmaz."),
            output_param("Birleşik", "#8E4EC6", None),
        ],
        outputs: vec![
            OutputDef::new("added", "Birleşik nesneler", OutputKind::Features),
            OutputDef::new("count", "Nesne sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

/// A map row's value for an object: its source's cell when not blank, else the default when not blank.
fn mapped(m: &MapRow, attrs: &BTreeMap<String, String>) -> Option<String> {
    let source = js_trim(&m.source);
    let cell = if source.is_empty() {
        ""
    } else {
        attrs.get(source).map_or("", String::as_str)
    };
    if !js_trim(cell).is_empty() {
        Some(cell.to_owned())
    } else if !js_trim(&m.default).is_empty() {
        Some(m.default.clone())
    } else {
        None
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let out = v.layer("output");
    let fields = if out.is_new {
        &[][..]
    } else {
        ctx.fields(&out.id)
    };
    let map = v
        .values
        .get("map")
        .and_then(field_map::read)
        .unwrap_or_default();
    let source_field = js_trim(v.text("sourceField")).to_owned();
    let sets = v.layer_sets("layers");
    let mut add = Vec::new();
    let (mut misfits, mut not_added) = (0usize, 0usize);
    feedback.progress(0.0, "Nesneler kopyalanıyor");
    for layer in sets {
        for e in &layer.entities {
            let mut attrs: Vec<(String, String)> = if map.is_empty() {
                e.base()
                    .attrs
                    .iter()
                    .map(|(k, x)| (k.clone(), x.clone()))
                    .collect()
            } else {
                map.iter()
                    .filter_map(|m| {
                        mapped(m, &e.base().attrs).map(|x| (js_trim(&m.target).to_owned(), x))
                    })
                    .collect()
            };
            if !source_field.is_empty() {
                // The source layer's name over a value the map gave the same field.
                attrs.retain(|(k, _)| *k != source_field);
                attrs.push((source_field.clone(), layer.name.clone()));
            }
            // Into typed fields: the values by their rules, the defaults, the required fields.
            let mut kept: BTreeMap<String, String> = BTreeMap::new();
            for (name, value) in attrs {
                match fields.iter().find(|f| f.name == name) {
                    None => {
                        kept.insert(name, value);
                    }
                    Some(f) => match check_value(f, &value) {
                        Ok(canonical) => {
                            kept.insert(name, canonical);
                        }
                        Err(_) => misfits += 1,
                    },
                }
            }
            for f in fields {
                let default = f.default.as_deref().unwrap_or("");
                if !js_trim(kept.get(&f.name).map_or("", String::as_str)).is_empty()
                    || js_trim(default).is_empty()
                {
                    continue;
                }
                if let Ok(canonical) = check_value(f, default)
                    && !canonical.is_empty()
                {
                    kept.insert(f.name.clone(), canonical);
                }
            }
            if fields.iter().any(|f| {
                f.required && js_trim(kept.get(&f.name).map_or("", String::as_str)).is_empty()
            }) {
                not_added += 1;
                continue;
            }
            add.push(copy_of(e, &out.id, Some(kept)));
        }
    }
    if misfits > 0 {
        feedback.warn(format!("{misfits} değer alanına uymadığı için yazılmadı."));
    }
    if not_added > 0 {
        feedback.warn(format!(
            "{not_added} nesne zorunlu alanı boş kaldığı için eklenmedi."
        ));
    }
    let count = add.len();
    RunResult {
        changes: Some(ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs: [("count".to_owned(), json!(count))].into_iter().collect(),
        summary: Some(format!(
            "{} katmandan {count} nesne “{}” katmanına birleştirildi.",
            sets.len(),
            out.name
        )),
        ..RunResult::default()
    }
}
