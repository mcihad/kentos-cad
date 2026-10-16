//! A form's preflight (docs/adr/0401 §4): what keeps a form from being used, said before a record
//! is touched. The designer shows it as the form is drawn; publishing and data entry refuse a form
//! with an error.

use kentos_contracts::{
    FormBody, FormField, FormIssue, FormItem, FormSeverity, FormWidget, LayerField, LayerFieldKind,
    body_problem, check_value,
};
use kentos_expression::{Variable, compile_with};

use crate::{codes, issue, record::schema_of, widget_of};

/// What is wrong with `body` over the layer's `fields`, errors and warnings, in the form's order.
pub fn preflight(body: &FormBody, fields: &[LayerField], variables: &[Variable]) -> Vec<FormIssue> {
    if let Some(problem) = body_problem(body) {
        return vec![issue(
            codes::BODY_INVALID,
            FormSeverity::Error,
            None,
            None,
            None,
            problem,
        )];
    }
    let mut p = Preflight {
        fields,
        schema: schema_of(fields, variables),
        issues: Vec::new(),
        bound: Vec::new(),
    };
    p.items(&body.items, false);
    if p.bound.is_empty() {
        p.issues.push(issue(
            codes::EMPTY,
            FormSeverity::Warning,
            None,
            None,
            None,
            "Formda alan yok; kayıt bu formla düzenlenemez.".into(),
        ));
    }
    // A field the layer requires: the form shows it always and lets it be written.
    for f in fields.iter().filter(|f| f.required) {
        let reach = p.bound.iter().find(|b| b.key == f.key());
        let message = match reach {
            None => Some(format!(
                "“{}” alanı katmanda zorunlu ama formda yok; ekleyin.",
                f.label()
            )),
            Some(b) if !b.always => Some(format!(
                "“{}” alanı katmanda zorunlu; form onu bir kuralla gizleyebilir ya da kilitleyebilir.",
                f.label()
            )),
            _ => None,
        };
        if let Some(message) = message {
            p.issues.push(issue(
                codes::REQUIRED_HIDDEN,
                FormSeverity::Error,
                reach.map(|b| b.item.as_str()),
                Some(f.key()),
                None,
                message,
            ));
        }
    }
    p.issues
}

/// Whether `widget` edits a field of `field`'s kind; the list widgets also need a value list (a
/// yes/no field's two values count as one).
pub fn widget_fits(widget: FormWidget, field: &LayerField) -> Result<(), &'static str> {
    use FormWidget as W;
    let listed = field.values.as_ref().is_some_and(|v| !v.is_empty());
    let kind_fits = match field.kind {
        LayerFieldKind::Text => matches!(
            widget,
            W::Text | W::TextArea | W::List | W::Radio | W::Segmented
        ),
        LayerFieldKind::Integer | LayerFieldKind::Decimal => {
            matches!(widget, W::Number | W::List | W::Radio | W::Segmented)
        }
        LayerFieldKind::Date => widget == W::Date,
        LayerFieldKind::Boolean => matches!(widget, W::Check | W::Switch | W::Radio | W::Segmented),
    };
    if !kind_fits {
        return Err(codes::WIDGET_UNFIT);
    }
    let needs_list = matches!(widget, W::List | W::Radio | W::Segmented);
    if needs_list && !listed && field.kind != LayerFieldKind::Boolean {
        return Err(codes::WIDGET_NEEDS_VALUES);
    }
    Ok(())
}

/// A field the form shows, and whether it is always shown and writable.
struct Bound {
    key: String,
    item: String,
    always: bool,
}

struct Preflight<'a> {
    fields: &'a [LayerField],
    schema: kentos_expression::Schema,
    issues: Vec<FormIssue>,
    bound: Vec<Bound>,
}

impl Preflight<'_> {
    /// `hideable`: an enclosing group has a rule that may hide it.
    fn items(&mut self, items: &[FormItem], hideable: bool) {
        for item in items {
            match item {
                FormItem::Tab { items, .. } => self.items(items, hideable),
                FormItem::Group {
                    id,
                    visible_when,
                    items,
                    ..
                } => {
                    if let Some(rule) = visible_when {
                        self.rule(id, None, None, "visibleWhen", rule);
                    }
                    self.items(items, hideable || visible_when.is_some());
                }
                FormItem::Field(f) => self.field(f, hideable),
                FormItem::Note { .. } => {}
            }
        }
    }

    fn field(&mut self, f: &FormField, hideable: bool) {
        let at = |code: &str, rule: Option<&str>, message: String| {
            issue(
                code,
                FormSeverity::Error,
                Some(&f.id),
                Some(&f.field),
                rule,
                message,
            )
        };
        let layer = self.fields.iter().find(|l| l.key() == f.field);
        if self.bound.iter().any(|b| b.key == f.field) {
            self.issues.push(at(
                codes::FIELD_TWICE,
                None,
                format!("“{}” alanı formda iki kez var.", f.field),
            ));
        }
        let always =
            !hideable && f.visible_when.is_none() && !f.read_only && f.read_only_when.is_none();
        self.bound.push(Bound {
            key: f.field.clone(),
            item: f.id.clone(),
            always,
        });
        let Some(layer) = layer else {
            self.issues.push(at(
                codes::FIELD_UNKNOWN,
                None,
                format!("Katmanda “{}” alanı yok.", f.field),
            ));
            return;
        };
        let widget = widget_of(f, layer);
        if let Err(code) = widget_fits(widget, layer) {
            let message = if code == codes::WIDGET_UNFIT {
                format!(
                    "“{}” bir {} alanı; {} onu düzenleyemez.",
                    layer.label(),
                    layer.kind.label().to_lowercase(),
                    widget.label().to_lowercase()
                )
            } else {
                format!(
                    "{} değer listesi ister; “{}” alanının listesi yok.",
                    widget.label(),
                    layer.label()
                )
            };
            self.issues.push(at(code, None, message));
        }
        if let Some(default) = &f.default {
            let mut loose = layer.clone();
            loose.required = false;
            if let Err(r) = check_value(&loose, default) {
                self.issues
                    .push(at(codes::DEFAULT_INVALID, Some("default"), r.message));
            }
        }
        if f.read_only && f.required && f.default.is_none() {
            self.issues.push(at(
                codes::REQUIRED_READ_ONLY,
                None,
                format!(
                    "“{}” hem zorunlu hem salt okunur; varsayılanı olmadan doldurulamaz.",
                    layer.label()
                ),
            ));
        }
        let own = Some(layer.name.as_str());
        for (rule, text) in [
            ("visibleWhen", &f.visible_when),
            ("requiredWhen", &f.required_when),
            ("readOnlyWhen", &f.read_only_when),
        ] {
            if let Some(text) = text {
                let itself = if rule == "requiredWhen" { None } else { own };
                self.rule(&f.id, Some(&f.field), itself, rule, text);
            }
        }
        for (i, c) in f.checks.iter().enumerate() {
            self.rule(&f.id, Some(&f.field), None, &format!("checks/{i}"), &c.rule);
        }
    }

    /// A rule compiles over the layer's fields; `itself`: the field it must not read.
    fn rule(&mut self, item: &str, key: Option<&str>, itself: Option<&str>, at: &str, text: &str) {
        let expr = match compile_with(text, &self.schema) {
            Ok(e) => e,
            Err(e) => {
                self.issues.push(issue(
                    codes::RULE_INVALID,
                    FormSeverity::Error,
                    Some(item),
                    key,
                    Some(at),
                    format!("Kural okunamadı: {}", e.text()),
                ));
                return;
            }
        };
        let warn = |code: &str, message: String| {
            issue(
                code,
                FormSeverity::Warning,
                Some(item),
                key,
                Some(at),
                message,
            )
        };
        for name in &expr.fields {
            if !self.fields.iter().any(|f| &f.name == name) {
                self.issues.push(warn(
                    codes::RULE_UNKNOWN_NAME,
                    format!("Kural katmanda olmayan “{name}” adını okuyor; değeri boş okunur."),
                ));
            } else if itself == Some(name.as_str()) {
                self.issues.push(warn(
                    codes::RULE_READS_ITSELF,
                    "Kural alanın kendi değerini okuyor; yazarken alan gizlenebilir ya da kilitlenebilir."
                        .to_owned(),
                ));
            }
        }
        for name in &expr.unknown_variables {
            self.issues.push(warn(
                codes::RULE_UNKNOWN_VARIABLE,
                format!("Projede “@{name}” değeri yok; boş okunur."),
            ));
        }
    }
}
