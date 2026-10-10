//! Uzaklık matrisi (the web's `builtin/proximity/matrix.ts`; docs/adr/0215
//! §3.2; ArcGIS Generate Near Table, QGIS "Distance matrix"): the distances
//! from each object to its nearest `k` targets (0: all), within a bound: a
//! list (one pair a row, with its rank), a matrix (the targets as columns) or
//! a summary (each object's count, least, mean and most). The search is the
//! run's store's (`Store::nearest`). Edits nothing.

use kentos_contracts::Entity;
use kentos_domain::Slot;
use serde_json::json;

use super::{
    MOST_ROWS, PROXIMITY_KINDS, bound, features, field, length_text, max_param, measure_of,
    measure_param, name_of, number,
};
use crate::types::{
    EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext,
    RunResult, Target, Tool,
};

/// The most targets a matrix takes, as columns.
pub const MOST_COLUMNS: usize = 200;

pub fn tool() -> Tool {
    Tool {
        id: "proximity.matrix".into(),
        label: "Uzaklık matrisi".into(),
        category: "proximity".into(),
        description: "Her nesneden hedeflere uzaklıkları tablo olarak verir: en yakın k hedef ya da hepsi; liste, matris ya da özet.".into(),
        help: Some([
            "Örnekler: her mahalleden en yakın üç sağlık ocağına uzaklıklar; okullar arası uzaklık matrisi; her parselin yollara en kısa uzaklıklarının özeti.",
            "Liste her çifti bir satıra yazar (Kaynak, Hedef, Sıra, Uzaklık); Matris kaynakları satır, hedefleri sütun yapar (en çok 200 hedef); Özet her kaynağın hedef sayısını, en az, ortalama ve en çok uzaklığını verir. En yakın k 0 ise bütün hedefler alınır.",
            "Adlar Ad alanından; boşsa nesnenin etiketi, o da yoksa listedeki sırası. Tablo pencerede Panoya kopyala ve CSV olarak kaydet ile alınır. Çizim değişmez.",
        ]
        .join("\n\n")),
        keywords: [
            "uzaklık matrisi", "mesafe tablosu", "distance matrix", "near table", "en yakın k", "yakınlık",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["UZAKLIKMATRISI", "MESAFETABLOSU"].map(String::from).to_vec(),
        icon: Some("distanceMatrix".into()),
        parameters: vec![
            features(
                "input",
                "Kaynaklar",
                &PROXIMITY_KINDS,
                false,
                "Uzaklıkları ölçülen nesneler.",
            ),
            features(
                "targets",
                "Hedefler",
                &PROXIMITY_KINDS,
                false,
                "Uzaklığı ölçülen hedefler.",
            ),
            measure_param(),
            number("k", "En yakın k", Some(0.0), Some(10_000.0), true, "")
                .default_value(json!(5))
                .describe("0: bütün hedefler."),
            max_param().describe("0: sınırsız."),
            ParamDef::new(
                "form",
                "Biçim",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("list", "Liste")
                            .hint("Her çift bir satır: Kaynak, Hedef, Sıra, Uzaklık"),
                        EnumOption::new("matrix", "Matris").hint("Kaynaklar satır, hedefler sütun"),
                        EnumOption::new("summary", "Özet")
                            .hint("Her kaynağın hedef sayısı, en az, ortalama ve en çok uzaklığı"),
                    ],
                },
            )
            .default_value(json!("list")),
            field("name", "Ad alanı", "input", false, false)
                .optional()
                .describe("Kaynakların tablodaki adı; boşsa etiket, o da yoksa sıra."),
            field("targetName", "Hedef ad alanı", "targets", false, false)
                .optional()
                .describe("Hedeflerin tablodaki adı."),
        ],
        outputs: vec![
            OutputDef::new("table", "Uzaklık tablosu", OutputKind::Table),
            OutputDef::new("count", "Çift sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn too_many(rows: usize) -> RunResult {
    RunResult::refused(format!(
        "Tablo {rows} satır olurdu (en çok {MOST_ROWS}); En yakın k ya da En çok uzaklık verin."
    ))
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    let targets = &v.features("targets").entities;
    let form = v.text("form");
    let k = v.number("k").unwrap_or(0.0).max(0.0) as usize;
    let max = v.number("max").unwrap_or(0.0);
    if form == "matrix" && targets.len() > MOST_COLUMNS {
        return RunResult::refused(format!(
            "Matris en çok {MOST_COLUMNS} hedef alır; {} hedef var. Liste biçimini seçin.",
            targets.len()
        ));
    }
    let per_input = if k > 0 {
        k.min(targets.len())
    } else {
        targets.len()
    };
    if form == "list" && max <= 0.0 && inputs.len().saturating_mul(per_input) > MOST_ROWS {
        return too_many(inputs.len().saturating_mul(per_input));
    }
    feedback.progress(0.0, "Uzaklıklar ölçülüyor");
    let ids = |list: &[&Entity]| -> Vec<Slot> { list.iter().map(|e| Slot(e.base().id)).collect() };
    let found = ctx.geometry.nearest(
        &ids(inputs),
        &ids(targets),
        k,
        bound(max),
        measure_of(v.text("measure")),
    );
    if found.len() > MOST_ROWS {
        return too_many(found.len());
    }
    let source = |i: usize| name_of(inputs[i], v.text("name"), i);
    let target = |j: usize| name_of(targets[j], v.text("targetName"), j);
    let length = |d: f64| length_text(ctx.units, d);
    let columns: Vec<String>;
    let mut rows: Vec<Vec<String>> = Vec::new();
    match form {
        "list" => {
            columns = ["Kaynak", "Hedef", "Sıra", "Uzaklık"]
                .map(String::from)
                .to_vec();
            let mut last = usize::MAX;
            let mut rank = 0usize;
            for f in &found {
                rank = if f.input == last { rank + 1 } else { 1 };
                last = f.input;
                rows.push(vec![
                    source(f.input),
                    target(f.target),
                    rank.to_string(),
                    length(f.d),
                ]);
            }
        }
        "matrix" => {
            columns = std::iter::once("Kaynak".to_owned())
                .chain((0..targets.len()).map(target))
                .collect();
            let mut cells = vec![vec![String::new(); targets.len()]; inputs.len()];
            for f in &found {
                cells[f.input][f.target] = length(f.d);
            }
            for (i, row) in cells.into_iter().enumerate() {
                rows.push(std::iter::once(source(i)).chain(row).collect());
            }
        }
        _ => {
            columns = ["Kaynak", "Hedef sayısı", "En az", "Ortalama", "En çok"]
                .map(String::from)
                .to_vec();
            let mut lists: Vec<Vec<f64>> = vec![Vec::new(); inputs.len()];
            for f in &found {
                lists[f.input].push(f.d);
            }
            for (i, ds) in lists.iter().enumerate() {
                let (Some(&least), Some(&most)) = (ds.first(), ds.last()) else {
                    rows.push(vec![
                        source(i),
                        "0".to_owned(),
                        String::new(),
                        String::new(),
                        String::new(),
                    ]);
                    continue;
                };
                // In order, as the web adds them.
                let mut sum = 0.0;
                for d in ds {
                    sum += d;
                }
                rows.push(vec![
                    source(i),
                    ds.len().to_string(),
                    length(least),
                    length(sum / ds.len() as f64),
                    length(most),
                ]);
            }
        }
    }
    RunResult {
        outputs: [
            (
                "table".to_owned(),
                json!({ "columns": columns, "rows": rows }),
            ),
            ("count".to_owned(), json!(found.len())),
        ]
        .into_iter()
        .collect(),
        summary: Some(format!(
            "{} kaynaktan {} uzaklık ölçüldü.",
            inputs.len(),
            found.len()
        )),
        ..RunResult::default()
    }
}
