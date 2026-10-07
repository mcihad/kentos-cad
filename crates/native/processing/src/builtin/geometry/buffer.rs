//! Tampon (the web's `builtin/geometry/buffer.ts`; docs/adr/0201 §2; Netcad's
//! Tampon Bölge, ArcGIS Buffer and Multiple Ring Buffer): the places within a
//! distance of each object, written to the output layer. A point's buffer is
//! a disc, a segment's a capsule, an arc's a band with discs at its ends, an
//! area's all of them around its boundary and the area itself; a minus
//! distance shrinks an area. One side of a path is its strips on that side
//! with its ends flat; rings are the bands between (k − 1)·d and k·d. The
//! pieces and their joins are the core's (`ops::geoprocess::buffer`).

use kentos_geometry_core::ops::geoprocess::buffer::Side;
use kentos_geometry_core::ops::geoprocess::calls::buffer_run;
use serde_json::{Value, json};

use super::{
    GEO_KINDS, empty_note, features_param, geo_scopes, input_notes, layer_param, new_object, shapes,
};
use crate::builtin::queries::attr;
use crate::text::{js_number, js_trim};
use crate::types::{
    ChangeSet, EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved,
    RunContext, RunResult, Target, Tool, Values,
};

pub fn tool() -> Tool {
    Tool {
        id: "geometry.buffer".into(),
        label: "Tampon".into(),
        category: "geometry".into(),
        description: "Nesnelerin çevresinde verilen uzaklıktaki alanı çizer: noktada daire, çizgide koridor, alanda dışa ya da (eksi uzaklıkla) içe; halkalarla ya da tek yandan da.".into(),
        help: Some([
            "Noktanın tamponu dairedir; çizginin her kenarı iki yanından uzaklık kadar genişler, uçları yarım dairedir; alanın tamponu alanı da içine alır. Yaylar ve daireler kesin kalır; elips ve eğri 0,1 mm içinde doğru parçalarıyla girer.",
            "Eksi uzaklık yalnız alanlarda anlamlıdır: alan kenarlarından içe küçülür, köşeleri keskin kalır; ortasını aşan küçültme bir şey bırakmaz. Tek yanlı tampon (Sol, Sağ) çizginin çizildiği yöne göredir, uçları düzdür; alan ve noktalar iki yandan alınır.",
            "Halka sayısı birden çoksa her halka (k − 1)·d ile k·d arasındaki şerittir; her parçaya “Uzaklık” (k·d) ve “Halka” (k) yazılır. Birleştir açıkken bütün tamponlar halka başına tek alandır ve nesnelerin öznitelikleri yazılmaz.",
            "Uzaklık alanı seçilirse her nesnenin uzaklığı bu alandan okunur (ondalık nokta ya da virgül); okunamayan nesne alınmaz ve söylenir.",
        ]
        .join("\n\n")),
        keywords: [
            "tampon", "tampon bölge", "buffer", "koridor", "uzaklık", "halka", "çoklu halka",
            "multiple ring buffer", "offset", "etki alanı",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["TAMPON", "TAMPONBOLGE", "BUFFER"].map(String::from).to_vec(),
        icon: Some("geoBuffer".into()),
        parameters: vec![
            features_param(
                "input",
                "Nesneler",
                &GEO_KINDS,
                geo_scopes(),
                "Tamponu çizilecek alanlar, çizgiler ve noktalar; yazı, ölçü ve blok gibi nesneler alınmaz.",
            ),
            ParamDef::new(
                "distance",
                "Uzaklık",
                ParamKind::Number {
                    min: None,
                    max: None,
                    integer: false,
                    unit: "m".into(),
                },
            )
            .default_value(json!(5))
            .describe("Alanlarda eksi değer alanı içe küçültür."),
            ParamDef::new(
                "side",
                "Yan",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("both", "İki yan").hint("Çizginin iki yanı ve uçları"),
                        EnumOption::new("left", "Sol").hint("Çizildiği yöne göre solu; uçları düz"),
                        EnumOption::new("right", "Sağ").hint("Çizildiği yöne göre sağı; uçları düz"),
                    ],
                },
            )
            .default_value(json!("both"))
            .describe("Yalnız çizgilerde; alanlar ve noktalar iki yandan alınır."),
            ParamDef::new(
                "rings",
                "Halka sayısı",
                ParamKind::Number {
                    min: Some(1.0),
                    max: Some(20.0),
                    integer: true,
                    unit: String::new(),
                },
            )
            .default_value(json!(1))
            .describe("Birden çoksa her halka (k − 1)·d ile k·d arasıdır."),
            ParamDef::new("dissolve", "Birleştir", ParamKind::Boolean)
                .default_value(json!(false))
                .describe("Bütün tamponlar halka başına tek alan olur; öznitelikler yazılmaz."),
            ParamDef::new(
                "distanceField",
                "Uzaklık alanı",
                ParamKind::Field {
                    of: vec!["input".into()],
                    allow_new: false,
                    multiple: false,
                },
            )
            .optional()
            .advanced()
            .describe("Seçilirse her nesnenin uzaklığı bu alandan okunur; Uzaklık kullanılmaz."),
            layer_param("Tampon", "#0090FF"),
        ],
        outputs: vec![
            OutputDef::new("buffers", "Tamponlar", OutputKind::Features),
            OutputDef::new("count", "Tampon sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: Some(validate),
        preview: Some(preview),
        run: Some(run),
    }
}

fn field_of(values: &Values) -> &str {
    js_trim(
        values
            .get("distanceField")
            .and_then(Value::as_str)
            .unwrap_or(""),
    )
}

fn validate(values: &Values) -> Option<String> {
    if !field_of(values).is_empty() {
        return None;
    }
    let d = values
        .get("distance")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let rings = values.get("rings").and_then(Value::as_f64).unwrap_or(1.0);
    if d.abs() < 0.001 {
        return Some("“Uzaklık” en az 0.001 m olmalı (eksi değerde mutlak değeri).".into());
    }
    if d.abs() > 100000.0 {
        return Some("“Uzaklık” en çok 100000 m olabilir.".into());
    }
    if d < 0.0 && rings > 1.0 {
        return Some(
            "Halkalar yalnız artı uzaklıkla çizilir; Halka sayısını 1 yapın ya da uzaklığı artı yazın."
                .into(),
        );
    }
    None
}

fn preview(values: &Values) -> Option<String> {
    let field = field_of(values);
    if !field.is_empty() {
        return Some(format!("Uzaklık “{field}” alanından okunacak"));
    }
    let rings = values.get("rings").and_then(Value::as_f64).unwrap_or(1.0);
    let d = values
        .get("distance")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    (rings > 1.0).then(|| format!("{} halka: {} m arayla", js_number(rings), js_number(d)))
}

fn run(v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let input = &v.features("input").entities;
    let field = v.text("distanceField");
    let distance = js_number(v.number("distance").unwrap_or(0.0));
    let distances: Vec<Option<String>> = input
        .iter()
        .map(|e| {
            if field.is_empty() {
                Some(distance.clone())
            } else {
                attr(e, field).map(str::to_owned)
            }
        })
        .collect();
    let rings = v.number("rings").unwrap_or(1.0).max(1.0) as usize;
    let dissolve = v.flag("dissolve");
    let side = Side::from_key(v.text("side")).unwrap_or(Side::Both);
    input_notes(feedback, input, &[], false);
    feedback.progress(0.0, "Tamponlar çiziliyor");
    let r = buffer_run(&shapes(input), &distances, side, rings, dissolve);
    if !r.unread.is_empty() {
        feedback.warn(format!(
            "{} nesnenin uzaklığı “{field}” alanından sayı olarak okunamadı; alınmadı.",
            r.unread.len()
        ));
    }
    if !r.inward.is_empty() {
        feedback.warn(format!(
            "{} nesnenin uzaklığı eksi; halkalar yalnız artı uzaklıkla çizilir, alınmadı.",
            r.inward.len()
        ));
    }
    empty_note(r.empty.len(), feedback);
    let layer = &v.layer("layer").id;
    let add: Vec<_> = r
        .pieces
        .into_iter()
        .filter_map(|p| {
            let mut attrs = p
                .source
                .map(|i| input[i].base().attrs.clone())
                .unwrap_or_default();
            if let Some(d) = p.distance {
                attrs.insert("Uzaklık".to_owned(), d);
            }
            if rings > 1 {
                attrs.insert("Halka".to_owned(), p.ring.to_string());
            }
            new_object(p.shape, layer, attrs)
        })
        .collect();
    let used = input.len() - r.unread.len() - r.inward.len() - r.empty.len();
    let count = add.len();
    let summary = if dissolve {
        format!("{used} nesnenin tamponu birleştirilerek yazıldı ({count} alan).")
    } else {
        format!("{used} nesnenin tamponu yazıldı ({count} alan).")
    };
    RunResult {
        changes: Some(ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs: [("count".to_owned(), json!(count))].into_iter().collect(),
        summary: Some(summary),
        ..RunResult::default()
    }
}
