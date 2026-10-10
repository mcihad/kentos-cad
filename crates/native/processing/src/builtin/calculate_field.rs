//! Öznitelik hesapla (the web's `builtin/calculateField.ts`, QGIS "Field
//! calculator"): writes an expression's value into an attribute of every
//! chosen object, new field or existing. Values are stored as text
//! (attributes are text until typed schemas arrive). A label that showed the
//! old value follows the new one.

use kentos_domain::Slot;
use kentos_style_core::expr::Value as ExprValue;
use kentos_style_core::expr::rows::As;
use serde_json::{Value, json};

use crate::expression::evaluate_in;
use crate::types::{
    ChangeSet, EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Patch, Resolved,
    Returns, RunContext, RunResult, Target, Tool,
};

pub fn tool() -> Tool {
    let expression = |name: &str, label: &str, returns: Returns, placeholder: &str| {
        ParamDef::new(
            name,
            label,
            ParamKind::Expression {
                returns,
                of: Some("input".into()),
                placeholder: Some(placeholder.into()),
            },
        )
    };
    Tool {
        id: "attributes.calculate".into(),
        label: "Öznitelik hesapla".into(),
        category: "attributes".into(),
        description: "Seçtiğiniz alana her nesne için bir ifadenin değerini yazar; alan yoksa oluşturur.".into(),
        help: Some([
            "Değer bir ifadedir: sabit bir metin ('Arsa'), başka alanlardan hesap (Ada || '/' || Parsel) ya da geometri ($alan, $uzunluk, $y, $x).",
            "Örnekler: metin($alan, 2) hesaplanan alanı iki ondalıkla; yuvarla([Tapu alanı] - $alan, 2) tapu farkını; 'P' || doldur($sıra, 5) sıra numarasını (P00001) yazar.",
            "Etiket o alanın eski değerini gösteriyorsa (parsel numarası gibi) yeni değeri gösterir. İşlem tek adımda geri alınır.",
        ]
        .join("\n\n")),
        keywords: [
            "öznitelik", "alan", "hesapla", "hesap makinesi", "değer", "yaz", "field", "calculator",
            "attribute", "update",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["OZHESAP", "ALANHESAP"].map(String::from).to_vec(),
        icon: Some("fieldCalc".into()),
        parameters: vec![
            ParamDef::new(
                "input",
                "Nesneler",
                ParamKind::Features {
                    kinds: None,
                    scopes: None,
                    writes: true,
                },
            )
            .default_value(json!({ "scope": "selection" }))
            .describe("Özniteliği yazılacak nesneler; kilitli katmandakiler alınmaz."),
            ParamDef::new(
                "field",
                "Yazılacak alan",
                ParamKind::Field {
                    of: vec!["input".into()],
                    allow_new: true,
                    multiple: false,
                },
            )
            .default_value(json!("Hesap alanı"))
            .describe("Listeden var olan bir alanı seçin ya da yeni bir ad yazın."),
            expression("value", "Değer", Returns::Value, "yuvarla($alan, 2)")
                .default_from(|d| json!(format!("metin($alan, {})", d.area_decimals))),
            expression("where", "Yalnızca şu koşulda", Returns::Condition, "Nitelik = 'Arsa'")
                .optional()
                .advanced()
                .describe("Boş bırakılırsa bütün nesnelere yazılır."),
            ParamDef::new(
                "empty",
                "Sonuç boşsa",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("keep", "Dokunma").hint("Alanın eski değeri kalır"),
                        EnumOption::new("clear", "Boşalt").hint("Alan boş metin olur"),
                    ],
                },
            )
            .default_value(json!("keep"))
            .advanced()
            .describe("Değer hesaplanamadığında (eksik alan, sayı olmayan metin) ne yapılacağı."),
            ParamDef::new("label", "Etiketi de güncelle", ParamKind::Boolean)
                .default_value(json!(true))
                .describe("Etiket bu alanın eski değerini gösteriyorsa yeni değeri gösterir."),
        ],
        outputs: vec![
            OutputDef::new("changed", "Değişen nesneler", OutputKind::Features),
            OutputDef::new("count", "Yazılan nesne sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: Some(|v| {
            let field = crate::text::js_trim(v.get("field").and_then(Value::as_str).unwrap_or(""));
            (!field.is_empty()).then(|| format!("“{field}” alanına yazılacak"))
        }),
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let field = v.text("field").to_owned();
    let list = &v.features("input").entities;
    feedback.progress(0.0, "Değerler hesaplanıyor");
    // The geometry values from the run's store; calls to other objects from its world (docs/adr/0214 §3).
    let layer_name = |id: &str| ctx.layer_name(id).to_owned();
    let how = ctx.evaluation(&layer_name);
    let condition = v
        .expr("where")
        .map(|e| evaluate_in(e, list, &how, As::Bool));
    let values = v
        .expr("value")
        .map(|e| evaluate_in(e, list, &how, As::Text))
        .unwrap_or_default();
    let (mut same, mut empty, mut filtered) = (0usize, 0usize, 0usize);
    let mut update = Vec::new();
    for (i, e) in list.iter().enumerate() {
        if condition
            .as_ref()
            .is_some_and(|c| c.get(i) != Some(&ExprValue::Bool(true)))
        {
            filtered += 1;
            continue;
        }
        let value = match values.get(i) {
            Some(ExprValue::Text(t)) => Some(t.to_string()),
            _ => None,
        };
        if value.is_none() && v.text("empty") == "keep" {
            empty += 1;
            continue;
        }
        // An empty result written is empty text.
        let text = value.unwrap_or_default();
        let base = e.base();
        let old = base.attrs.get(&field);
        if old == Some(&text) {
            same += 1;
            continue;
        }
        let mut attrs = base.attrs.clone();
        attrs.insert(field.clone(), text.clone());
        let label = (v.flag("label") && old.is_some() && base.label.as_ref() == old)
            .then(|| Some(text.clone()));
        update.push(Patch {
            id: Slot(base.id),
            attrs: Some(attrs),
            label,
            zs: None,
        });
        if i % 2000 == 1999 {
            feedback.progress(i as f64 / list.len() as f64, "Değerler hesaplanıyor");
            if feedback.canceled() {
                return RunResult::default();
            }
        }
    }
    let mut notes = Vec::new();
    if same > 0 {
        notes.push(format!("{same} nesnede değer zaten aynıydı"));
    }
    if empty > 0 {
        notes.push(format!("{empty} nesnede sonuç boş olduğu için dokunulmadı"));
    }
    if filtered > 0 {
        notes.push(format!("{filtered} nesne koşulu sağlamadı"));
    }
    let summary = format!(
        "{} nesnede “{field}” yazıldı{}.",
        update.len(),
        if notes.is_empty() {
            String::new()
        } else {
            format!("; {}", notes.join("; "))
        }
    );
    let changed: Vec<u32> = update.iter().map(|u| u.id.0).collect();
    let count = update.len();
    RunResult {
        changes: Some(ChangeSet {
            update,
            ..ChangeSet::default()
        }),
        outputs: [
            ("changed".to_owned(), json!(changed)),
            ("count".to_owned(), json!(count)),
        ]
        .into_iter()
        .collect(),
        summary: Some(summary),
        ..RunResult::default()
    }
}
