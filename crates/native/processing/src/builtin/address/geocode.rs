//! Adresleri eşle (docs/adr/0229 §11): each row of a table, a file's or
//! objects' attributes, looked for in the project's address data or with an
//! online provider; a match (and, if asked, a tie) becomes a point at its
//! place with the row's cells and the match's fields, the rest are the
//! report's rows. An online provider is asked once for each address (the
//! rows of one folded address share its answer), within its batch limit.

use std::collections::BTreeMap;

use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::geocode::parse::from_columns;
use kentos_geometry_core::ops::geocode::{Status, find, parse};
use kentos_services::geocode::batch;
use kentos_services::geocode::read;
use kentos_services::geocode::request::{self, BATCH_LIMIT};
use kentos_services::geocode::score::pick;
use serde_json::{Value, json};

use super::super::convert::{Rows, column, read_rows, source_params};
use super::super::geometry::{layer_param, new_object};
use super::source_param;
use crate::address::{
    Match, Matched, NO_SYSTEM, Plane, Source, local_matches, match_attributes, online_match,
    resolve,
};
use crate::text::js_trim;
use crate::types::{
    ChangeSet, EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved,
    RunContext, RunResult, Target, Tool,
};

const COLUMNS: [(&str, &str, &str); 6] = [
    (
        "neighbourhood",
        "Mahalle sütunu",
        "Mahalle ya da köy (“Kızılay”, “Kızılay Mahallesi”).",
    ),
    (
        "street",
        "Yol sütunu",
        "Cadde, sokak, bulvar ya da yol adı türüyle (“Atatürk Bulvarı”, “1203/1 Sk.”).",
    ),
    (
        "door",
        "Kapı no sütunu",
        "Dış kapı numarası (“12”, “12/A”).",
    ),
    ("district", "İlçe sütunu", "İlçe."),
    ("province", "İl sütunu", "İl."),
    ("postcode", "Posta kodu sütunu", "Beş haneli posta kodu."),
];

fn single(v: &serde_json::Map<String, Value>) -> bool {
    v.get("form").and_then(Value::as_str) != Some("columns")
}

pub fn tool() -> Tool {
    let mut parameters = source_params("adres");
    parameters.push(
        ParamDef::new(
            "form",
            "Adres",
            ParamKind::Choice {
                options: vec![
                    EnumOption::new("single", "Tek alan").hint("Adres bir sütunda yazılı"),
                    EnumOption::new("columns", "Ayrı alanlar")
                        .hint("Mahalle, yol, kapı no … ayrı sütunlarda"),
                ],
            },
        )
        .default_value(json!("single")),
    );
    parameters.push(
        column(
            "address",
            "Adres sütunu",
            "Tek satırlık adres (“Kızılay Mah. Atatürk Blv. No:12 Çankaya/Ankara”).",
            false,
        )
        .shown_when(single),
    );
    for (name, label, description) in COLUMNS {
        parameters.push(column(name, label, description, true).shown_when(|v| !single(v)));
    }
    parameters.push(source_param());
    parameters.push(
        ParamDef::new(
            "minScore",
            "En düşük puan",
            ParamKind::Number {
                min: Some(0.0),
                max: Some(100.0),
                integer: true,
                unit: String::new(),
                placeholder: Some("Kaynağınki".into()),
            },
        )
        .optional()
        .default_value(Value::Null)
        .describe("Bundan düşük puanlı satır eşleşmedi sayılır; boşsa adres verisinin en düşük puanı, sağlayıcıda 85."),
    );
    parameters.push(
        ParamDef::new("tied", "Belirsizleri de yaz", ParamKind::Boolean)
            .default_value(json!(true))
            .describe("En iyi puanı başka yerde de olan satır, en iyisinin yerine Durum “Belirsiz” ile yazılır."),
    );
    parameters.push(layer_param("Adresler", "#E5484D"));
    Tool {
        id: "address.geocode".into(),
        label: "Adresleri eşle".into(),
        category: "address".into(),
        description: "Bir tablonun her satırındaki adresi bulur ve yerine nokta yazar.".into(),
        help: Some([
            "Örnekler: ruhsat, şikâyet ya da abone listelerindeki adresleri haritaya koymak; bir Excel dosyasındaki adresleri numarataj verisiyle eşlemek.",
            "Kaynak projenin adres verisiyse (kapı noktaları, yollar, mahalleler; Adres verisi penceresi) hiçbir veri cihazdan çıkmaz. Çevrimiçi bir sağlayıcı seçilirse her adres onun sunucusuna gider; sağlayıcının hız ve toplu iş sınırları uygulanır (Nominatim'de saniyede bir istek, en çok 1 000 adres).",
            "Eşleşen satır, adresin yerinde bir nokta olur: satırın dolu hücreleri ve Durum, Puan, Eşleşme türü, Eşleşen adres, Mahalle, Yol, Kapı no, İlçe, İl, Posta kodu, Kaynak, Girdi adresi öznitelikleri. Eşleşmeyenler tablodadır.",
        ]
        .join("\n\n")),
        keywords: [
            "adres", "geocoding", "geocode addresses", "batch nominatim", "numarataj", "kapı", "sokak", "mahalle",
            "coğrafi kodlama", "adresten konum",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["ADRESEŞLE", "ADRESESLE", "GEOCODE"].map(String::from).to_vec(),
        icon: Some("addressGeocode".into()),
        parameters,
        outputs: vec![
            OutputDef::new("count", "Yazılan nokta sayısı", OutputKind::Number),
            OutputDef::new("table", "Eşleşmeyen satırlar", OutputKind::Table),
        ],
        targets: vec![Target::Worker],
        validate: Some(|v| {
            if single(v) {
                return None;
            }
            let given = |n: &str| v.get(n).and_then(Value::as_str).is_some_and(|s| !js_trim(s).is_empty());
            (!given("street") && !given("neighbourhood"))
                .then(|| "Ayrı alanlarda en az yol ya da mahalle sütunu seçin.".into())
        }),
        preview: None,
        run: Some(run),
    }
}

/// A row's address: the text a provider is sent, the text shown, its parts.
struct Row {
    text: String,
    parts: kentos_geometry_core::ops::geocode::Parsed,
}

/// What a row's best place gave: its status and the place.
type Hit = (Status, Match);

fn rows_of(v: &Resolved<'_>, rows: &Rows) -> Vec<Row> {
    rows.cells
        .iter()
        .map(|c| {
            if single(&v.values) {
                let text = js_trim(&c[0]).to_owned();
                Row {
                    parts: parse(&text),
                    text,
                }
            } else {
                Row {
                    text: request::line_of(&c[0], &c[1], &c[2], &c[3], &c[4], &c[5]),
                    parts: from_columns(&c[0], &c[1], &c[2], &c[3], &c[4], &c[5]),
                }
            }
        })
        .collect()
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let host = feedback.addresses();
    let source = match resolve(v.text("via"), ctx.doc, host.as_deref()) {
        Ok(s) => s,
        Err(why) => return RunResult::refused(why),
    };
    let names: Vec<String> = if single(&v.values) {
        vec![js_trim(v.text("address")).to_owned()]
    } else {
        COLUMNS
            .iter()
            .map(|(n, _, _)| js_trim(v.text(n)).to_owned())
            .collect()
    };
    let table = match read_rows(v, &names) {
        Ok(rows) => rows,
        Err(why) => return RunResult::refused(why),
    };
    let rows = rows_of(v, &table);
    let total = rows.len();
    let mut hits: Vec<Option<Hit>> = Vec::with_capacity(total);
    let (source_name, attribution, min) = match &source {
        Source::Local(l) => (
            l.name.clone(),
            String::new(),
            v.number("minScore").map_or(l.min_score, |m| m as u32),
        ),
        Source::Online(p) => (
            p.name.clone(),
            p.attribution.clone(),
            v.number("minScore").map_or(85, |m| m as u32),
        ),
    };
    match &source {
        Source::Local(l) => {
            feedback.progress(0.0, "Adres dizini kuruluyor");
            let loc = kentos_native_application::addresses::build(ctx.doc, l);
            for (i, r) in rows.iter().enumerate() {
                if r.text.is_empty() {
                    hits.push(None);
                    continue;
                }
                let text = if single(&v.values) {
                    r.text.as_str()
                } else {
                    ""
                };
                let found = find(&loc, text, &r.parts, min, 20);
                hits.push(
                    local_matches(&found)
                        .into_iter()
                        .next()
                        .map(|m| (found.status, m)),
                );
                if i % 1000 == 999 {
                    feedback.progress(
                        i as f64 / total as f64,
                        &format!("{} / {total} adres", i + 1),
                    );
                    if feedback.canceled() {
                        return RunResult::default();
                    }
                }
            }
        }
        Source::Online(p) => {
            let Some(plane) = Plane::of(ctx.doc) else {
                return RunResult::refused(NO_SYSTEM.into());
            };
            let Some(host) = host else {
                return RunResult::refused("Bu ortamda çevrimiçi adres sağlayıcısı yok.".into());
            };
            let texts: Vec<String> = rows.iter().map(|r| r.text.clone()).collect();
            let (keys, at) = batch::unique(&texts);
            if let Some(why) = p.batch_refusal(keys.len()) {
                return RunResult::refused(why);
            }
            // The first row of each key asks.
            let mut first: Vec<usize> = vec![usize::MAX; keys.len()];
            for (i, k) in at.iter().enumerate() {
                if let Some(k) = k
                    && first[*k] == usize::MAX
                {
                    first[*k] = i;
                }
            }
            let mut answers: Vec<Option<Hit>> = Vec::with_capacity(keys.len());
            let mut failures = 0usize;
            let mut last_failure = String::new();
            for (n, &row) in first.iter().enumerate() {
                feedback.progress(
                    n as f64 / keys.len().max(1) as f64,
                    &format!("{} / {} adres", n + 1, keys.len()),
                );
                if feedback.canceled() {
                    return RunResult::default();
                }
                let r = &rows[row];
                let req = request::forward(p, &r.text, BATCH_LIMIT, None);
                let stop = || feedback.canceled();
                let body = match host.ask(p, &req, &stop) {
                    Ok(b) => b,
                    Err(why) => {
                        failures += 1;
                        last_failure = why;
                        answers.push(None);
                        continue;
                    }
                };
                let read = match read::forward(p.kind, &body) {
                    Ok(r) => r,
                    Err(why) => {
                        failures += 1;
                        last_failure = why;
                        answers.push(None);
                        continue;
                    }
                };
                let hit = pick(&r.parts, &read.candidates, min).and_then(|(i, _, _, status)| {
                    online_match(&r.parts, &read.candidates[i], &plane).map(|m| (status, m))
                });
                answers.push(hit);
            }
            if failures > 0 {
                feedback.warn(format!(
                    "{failures} adres için sağlayıcı cevap vermedi; son neden: {last_failure}"
                ));
            }
            for k in &at {
                hits.push(k.and_then(|k| answers[k].clone()));
            }
        }
    }
    let layer = &v.layer("layer").id;
    let tied_too = v.flag("tied");
    let (mut matched, mut tied, mut unmatched, mut empty) = (0usize, 0usize, 0usize, 0usize);
    let mut add = Vec::new();
    let mut report = Vec::new();
    for (i, (r, hit)) in rows.iter().zip(&hits).enumerate() {
        if r.text.is_empty() {
            empty += 1;
            report.push(json!([table.labels[i], "", "", "", "Adres boş"]));
            continue;
        }
        let written = match hit {
            Some((Status::Matched, _)) => {
                matched += 1;
                true
            }
            Some((Status::Tied, _)) => {
                tied += 1;
                tied_too
            }
            _ => {
                unmatched += 1;
                false
            }
        };
        match hit {
            Some((status, m)) if written => {
                let attrs: BTreeMap<String, String> = match_attributes(
                    &table.attrs[i],
                    &Matched {
                        status: *status,
                        found: m,
                        source: &source_name,
                        attribution: &attribution,
                        input: &r.text,
                    },
                );
                add.extend(new_object(
                    Shape::Point {
                        p: m.at,
                        z: None,
                        parts: None,
                    },
                    layer,
                    attrs,
                ));
            }
            Some((status, m)) => report.push(json!([
                table.labels[i],
                r.text,
                m.label,
                m.score.unwrap_or(0),
                status.word()
            ])),
            None => report.push(json!([table.labels[i], r.text, "", "", "Aday yok"])),
        }
    }
    let count = add.len();
    let mut out: BTreeMap<String, Value> =
        [("count".to_owned(), json!(count))].into_iter().collect();
    if !report.is_empty() {
        out.insert(
            "table".into(),
            json!({ "columns": ["Satır", "Girdi adresi", "En iyi aday", "Puan", "Durum"], "rows": report }),
        );
    }
    let mut summary =
        format!("{total} satırdan {matched} eşleşti, {tied} belirsiz, {unmatched} eşleşmedi");
    if empty > 0 {
        summary.push_str(&format!(", {empty} boş"));
    }
    summary.push_str(&format!("; {count} nokta yazıldı."));
    RunResult {
        changes: Some(ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs: out.into_iter().collect(),
        summary: Some(summary),
        ..RunResult::default()
    }
}
