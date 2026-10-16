//! A form made of a layer's fields (docs/adr/0401 §7): one field item each, in the layer's order,
//! with the widget its kind takes; a field the layer requires is required on the form too.

use kentos_contracts::{
    FormBody, FormField, FormItem, FormWidget, LayerField, LayerFieldKind, MOST_FORM_NAME,
};

/// The draft a new form starts from: the layer's fields, in order (revision 1).
pub fn generate(fields: &[LayerField]) -> FormBody {
    let mut taken: Vec<String> = Vec::new();
    let items = fields
        .iter()
        .map(|f| {
            let id = item_id_of(&f.name, &taken);
            taken.push(id.clone());
            let mut item = FormField::new(id, f.key());
            item.required = f.required;
            FormItem::Field(item)
        })
        .collect();
    FormBody {
        revision: 1,
        items,
        ..Default::default()
    }
}

/// A new item's id from `name`: its letters lowercased the Turkish way and its digits, the rest as
/// `-`, not one of `taken` (a number added).
pub fn item_id_of(name: &str, taken: &[String]) -> String {
    let mut stem = String::new();
    for c in kentos_contracts::fold(name).chars() {
        if c.is_alphanumeric() {
            stem.push(c);
        } else if !stem.ends_with('-') {
            stem.push('-');
        }
    }
    let stem: String = stem
        .trim_matches('-')
        .chars()
        .take(MOST_FORM_NAME - 4)
        .collect();
    let stem = if stem.is_empty() {
        "alan".to_owned()
    } else {
        stem
    };
    if !taken.contains(&stem) {
        return stem;
    }
    (2..)
        .map(|n| format!("{stem}-{n}"))
        .find(|id| !taken.contains(id))
        .unwrap_or(stem)
}

/// The widget a field item uses: its own, else its field's kind's (a value list a list, a long text
/// several lines).
pub fn widget_of(item: &FormField, field: &LayerField) -> FormWidget {
    if let Some(w) = item.widget {
        return w;
    }
    if field.values.as_ref().is_some_and(|v| !v.is_empty()) {
        return FormWidget::List;
    }
    match field.kind {
        LayerFieldKind::Integer | LayerFieldKind::Decimal => FormWidget::Number,
        LayerFieldKind::Date => FormWidget::Date,
        LayerFieldKind::Boolean => FormWidget::Check,
        LayerFieldKind::Text if field.length.is_some_and(|n| n > 200) => FormWidget::TextArea,
        LayerFieldKind::Text => FormWidget::Text,
    }
}
