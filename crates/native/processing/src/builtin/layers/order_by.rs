//! İfadeye göre sırala (the web's `builtin/layers/orderBy.ts`; docs/adr/0217
//! §6; QGIS Order by expression): copies of the objects written in an
//! expression's order into the result layer (the attribute table and the
//! drawing order are theirs), the order field 1, 2, 3 …; the order is the
//! core's (`ops::order`). A new result layer takes the look and fields of
//! the first object's layer and goes right above it. One undo step.

use kentos_geometry_core::ops::order::{Key, order};
use kentos_style_core::expr::Value as ExprValue;
use kentos_style_core::expr::rows::As;
use serde_json::json;

use super::{copy_of, input_param, output_param};
use crate::expression::evaluate_in;
use crate::text::js_trim;
use crate::types::{
    ChangeSet, EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, Returns,
    RunContext, RunResult, ScopeKind, Target, Tool,
};

pub fn tool() -> Tool {
    Tool {
        id: "layers.orderBy".into(),
        label: "İfadeye göre sırala".into(),
        category: "layers".into(),
        description: "Nesnelerin kopyalarını bir ifadenin değerine göre sıralı olarak yeni bir katmana yazar; isterse sıra numarasını bir alana yazar.".into(),
        help: Some([
            "Örnekler: parselleri alanlarına göre büyükten küçüğe, noktaları adlarına göre doğal sırayla (P2, P10) dizmek; sıra numarasını bir alana yazmak.",
            "Değeri olmayan nesneler Boşlar'a göre başta ya da sonda. Türler karışıksa evet/hayır, sayı, metin sırasıyla; metinler doğal sırayla (sayılar değerleriyle, Türkçe harf sırasıyla). Eşit değerler girdinin sırasıyla kalır.",
            "Kopyalar sonuç katmanına sıralı yazılır; Öznitelik tablosu ve çizim sırası onların sırasıdır. Girdi değişmez.",
        ]
        .join("\n\n")),
        keywords: [
            "sırala", "sıra", "ifade", "artan", "azalan", "order", "order by", "sort",
        ]
        .map(String::from)
        .to_vec(),
        aliases: vec!["IFADEYLESIRALA".into()],
        icon: Some("orderByExpression".into()),
        parameters: vec![
            input_param(
                vec![
                    ScopeKind::Layer,
                    ScopeKind::Selection,
                    ScopeKind::Visible,
                    ScopeKind::All,
                ],
                "Sıralanacak nesneler; kilitli katmandakiler alınmaz.",
            ),
            ParamDef::new(
                "expression",
                "İfade",
                ParamKind::Expression {
                    returns: Returns::Value,
                    of: Some("input".into()),
                    placeholder: Some("$alan".into()),
                },
            )
            .describe("Nesneleri sıralayan değer."),
            ParamDef::new(
                "direction",
                "Yön",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("asc", "Artan").hint("Küçükten büyüğe, A'dan Z'ye"),
                        EnumOption::new("desc", "Azalan").hint("Büyükten küçüğe, Z'den A'ya"),
                    ],
                },
            )
            .default_value(json!("asc")),
            ParamDef::new(
                "nulls",
                "Boşlar",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("last", "Sonda").hint("Değeri olmayan nesneler sonda"),
                        EnumOption::new("first", "Başta").hint("Değeri olmayan nesneler başta"),
                    ],
                },
            )
            .default_value(json!("last")),
            ParamDef::new(
                "orderField",
                "Sıra alanı",
                ParamKind::Field {
                    of: vec!["input".into()],
                    allow_new: true,
                    multiple: false,
                },
            )
            .optional()
            .default_value(json!("Sıra"))
            .describe("Sıra numarası (1, 2, 3 …) bu alana yazılır; boşsa yazılmaz."),
            output_param("Sıralı", "#12A594", Some("input")),
        ],
        outputs: vec![
            OutputDef::new("objects", "Sıralı nesneler", OutputKind::Features),
            OutputDef::new("count", "Nesne sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    feedback.progress(0.0, "Değerler hesaplanıyor");
    // The geometry values from the run's store; calls to other objects from its world (docs/adr/0214 §3).
    let layer_name = |id: &str| ctx.layer_name(id).to_owned();
    let how = ctx.evaluation(&layer_name);
    let values = v
        .expr("expression")
        .map(|e| evaluate_in(e, inputs, &how, As::Value))
        .unwrap_or_default();
    let keys: Vec<Option<Key>> = (0..inputs.len())
        .map(|i| match values.get(i) {
            Some(ExprValue::Num(x)) => Some(Key::Num(*x)),
            Some(ExprValue::Text(t)) => Some(Key::Text(t.to_string())),
            Some(ExprValue::Bool(b)) => Some(Key::Bool(*b)),
            _ => None,
        })
        .collect();
    feedback.progress(0.5, "Sıralanıyor");
    let at = order(
        &keys,
        v.text("direction") == "desc",
        v.text("nulls") == "first",
    );
    let field = js_trim(v.text("orderField")).to_owned();
    let out = v.layer("output");
    let add: Vec<_> = at
        .iter()
        .enumerate()
        .map(|(n, &i)| {
            let e = inputs[i as usize];
            let attrs = (!field.is_empty()).then(|| {
                let mut a = e.base().attrs.clone();
                a.insert(field.clone(), (n + 1).to_string());
                a
            });
            copy_of(e, &out.id, attrs)
        })
        .collect();
    let count = add.len();
    RunResult {
        changes: Some(ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs: [("count".to_owned(), json!(count))].into_iter().collect(),
        summary: Some(format!(
            "{count} nesne sıralanıp “{}” katmanına yazıldı.",
            out.name
        )),
        like: inputs.first().map(|e| e.base().layer_id.clone()),
        ..RunResult::default()
    }
}
