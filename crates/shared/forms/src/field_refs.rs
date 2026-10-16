//! Where a layer's field is named (docs/adr/0414 §1) and what a rename carries (§2): the forms'
//! drafts and published releases, the lists from another layer, the related records, the
//! relations, and the layer's own table rules, filter, labels, time, information card and actions.
//! One walk over every place serves both: [`uses`] reads it, [`rename_settings`] and
//! [`rename_node`] write it. A place names the field by its key (a form item, a list, a relation),
//! by its name (an attribute's name), in an expression or in a template's `{…}`; a place in a
//! published release is kept as it is.

use kentos_contracts::{FieldUse, FieldUseHow, FormItem, LayerField, LayerNode, ProjectSettings};
use kentos_expression::editor::{
    names_field, rename_field, rename_in_template, template_names_field,
};

const SEP: &str = " › ";

/// A place that may name a field: the layer whose field it names, how, where and what it is.
struct Site<'a> {
    layer: &'a str,
    how: FieldUseHow,
    path: String,
    place: String,
    kept: bool,
}

type Visit<'v> = dyn FnMut(&Site<'_>, &mut String) + 'v;

/// The item properties that are expressions, with their words.
const ITEM_RULES: [(&str, &str); 6] = [
    ("visibleWhen", "Görünme koşulu"),
    ("requiredWhen", "Zorunluluk koşulu"),
    ("readOnlyWhen", "Salt okunurluk koşulu"),
    ("defaultExpr", "Varsayılan ifadesi"),
    ("calculate", "Hesap"),
    ("choiceFilter", "Seçenek süzgeci"),
];

fn find<'n>(nodes: &'n [LayerNode], id: &str) -> Option<&'n LayerNode> {
    nodes.iter().find_map(|n| {
        if n.id == id {
            Some(n)
        } else {
            find(&n.children, id)
        }
    })
}

/// A field item's name in the words: its label, else its field's label, else its key.
fn field_words(layers: &[LayerNode], layer: &str, key: &str) -> String {
    find(layers, layer)
        .and_then(|n| n.fields.iter().find(|f| f.key() == key))
        .map_or_else(|| key.to_owned(), |f| f.label().to_owned())
}

fn settings_sites(settings: &mut ProjectSettings, layers: &[LayerNode], f: &mut Visit<'_>) {
    for (fi, form) in settings.forms.iter_mut().enumerate() {
        let layer = form.layer.clone();
        let words = format!("“{}” formu", form.name);
        let at = format!("settings/forms/{fi}/draft/items");
        items(&mut form.draft.items, &layer, layers, &at, &words, false, f);
        for (ri, r) in form.releases.iter_mut().enumerate() {
            let words = format!("“{}” formunun {}. sürümü", form.name, r.version);
            let at = format!("settings/forms/{fi}/releases/{ri}/body/items");
            items(&mut r.body.items, &layer, layers, &at, &words, true, f);
        }
    }
    for (ri, r) in settings.relations.iter_mut().enumerate() {
        let site = Site {
            layer: &r.child,
            how: FieldUseHow::Key,
            path: format!("settings/relations/{ri}/field"),
            place: format!("“{}” ilişkisi{SEP}Alan", r.name),
            kept: false,
        };
        f(&site, &mut r.field);
    }
}

fn items(
    list: &mut [FormItem],
    layer: &str,
    layers: &[LayerNode],
    at: &str,
    words: &str,
    kept: bool,
    f: &mut Visit<'_>,
) {
    let mut put = |layer: &str, how, path: String, place: String, value: &mut String| {
        f(
            &Site {
                layer,
                how,
                path,
                place,
                kept,
            },
            value,
        );
    };
    for (k, it) in list.iter_mut().enumerate() {
        let p = format!("{at}/{k}");
        match it {
            FormItem::Tab { items: inner, .. } => {
                items(
                    inner,
                    layer,
                    layers,
                    &format!("{p}/items"),
                    words,
                    kept,
                    &mut put_back(&mut put),
                );
            }
            FormItem::Field(field) => {
                let name = field
                    .label
                    .clone()
                    .filter(|l| !l.is_empty())
                    .unwrap_or_else(|| field_words(layers, layer, &field.field));
                let w = format!("{words}{SEP}“{name}” öğesi");
                put(
                    layer,
                    FieldUseHow::Key,
                    format!("{p}/field"),
                    format!("{w}{SEP}Alan"),
                    &mut field.field,
                );
                let rules: [&mut Option<String>; 6] = [
                    &mut field.visible_when,
                    &mut field.required_when,
                    &mut field.read_only_when,
                    &mut field.default_expr,
                    &mut field.calculate,
                    &mut field.choice_filter,
                ];
                for (rule, (key, said)) in rules.into_iter().zip(ITEM_RULES) {
                    if let Some(v) = rule.as_mut().filter(|v| !v.is_empty()) {
                        put(
                            layer,
                            FieldUseHow::Expression,
                            format!("{p}/{key}"),
                            format!("{w}{SEP}{said}"),
                            v,
                        );
                    }
                }
                for (c, check) in field.checks.iter_mut().enumerate() {
                    put(
                        layer,
                        FieldUseHow::Expression,
                        format!("{p}/checks/{c}/rule"),
                        format!("{w}{SEP}{}. denetim", c + 1),
                        &mut check.rule,
                    );
                }
                if let Some(lk) = field.lookup.as_mut() {
                    let other = lk.layer.clone();
                    if lk.key != "$uid" {
                        put(
                            &other,
                            FieldUseHow::Key,
                            format!("{p}/lookup/key"),
                            format!("{w}{SEP}Listenin anahtarı"),
                            &mut lk.key,
                        );
                    }
                    if let Some(label) = lk.label.as_mut() {
                        put(
                            &other,
                            FieldUseHow::Key,
                            format!("{p}/lookup/label"),
                            format!("{w}{SEP}Listenin etiketi"),
                            label,
                        );
                    }
                    for (m, matched) in lk.matches.iter_mut().enumerate() {
                        put(
                            layer,
                            FieldUseHow::Key,
                            format!("{p}/lookup/matches/{m}/field"),
                            format!("{w}{SEP}{}. eşleşmenin alanı", m + 1),
                            &mut matched.field,
                        );
                        put(
                            &other,
                            FieldUseHow::Key,
                            format!("{p}/lookup/matches/{m}/record"),
                            format!("{w}{SEP}{}. eşleşmenin kayıt alanı", m + 1),
                            &mut matched.record,
                        );
                    }
                    if let Some(filter) = lk.filter.as_mut().filter(|v| !v.is_empty()) {
                        put(
                            &other,
                            FieldUseHow::Expression,
                            format!("{p}/lookup/filter"),
                            format!("{w}{SEP}Listenin süzgeci"),
                            filter,
                        );
                    }
                }
            }
            FormItem::Group {
                id,
                label,
                visible_when,
                items: inner,
                ..
            } => {
                let w = format!(
                    "{words}{SEP}“{}” grubu",
                    label.as_deref().filter(|l| !l.is_empty()).unwrap_or(id)
                );
                if let Some(v) = visible_when.as_mut().filter(|v| !v.is_empty()) {
                    put(
                        layer,
                        FieldUseHow::Expression,
                        format!("{p}/visibleWhen"),
                        format!("{w}{SEP}Görünme koşulu"),
                        v,
                    );
                }
                items(
                    inner,
                    layer,
                    layers,
                    &format!("{p}/items"),
                    words,
                    kept,
                    &mut put_back(&mut put),
                );
            }
            FormItem::Note { id, text } => {
                put(
                    layer,
                    FieldUseHow::Template,
                    format!("{p}/text"),
                    format!("{words}{SEP}“{id}” notu{SEP}Metin"),
                    text,
                );
            }
            FormItem::Related {
                id,
                label,
                layer: other,
                field,
                visible_when,
            } => {
                let w = format!(
                    "{words}{SEP}“{}” ilişkili kayıtları",
                    label.as_deref().filter(|l| !l.is_empty()).unwrap_or(id)
                );
                let other = other.clone();
                put(
                    &other,
                    FieldUseHow::Key,
                    format!("{p}/field"),
                    format!("{w}{SEP}Alan"),
                    field,
                );
                if let Some(v) = visible_when.as_mut().filter(|v| !v.is_empty()) {
                    put(
                        layer,
                        FieldUseHow::Expression,
                        format!("{p}/visibleWhen"),
                        format!("{w}{SEP}Görünme koşulu"),
                        v,
                    );
                }
            }
            FormItem::Attachments {
                id,
                label,
                visible_when,
                ..
            } => {
                if let Some(v) = visible_when.as_mut().filter(|v| !v.is_empty()) {
                    let w = format!(
                        "{words}{SEP}“{}” ekleri",
                        label.as_deref().filter(|l| !l.is_empty()).unwrap_or(id)
                    );
                    put(
                        layer,
                        FieldUseHow::Expression,
                        format!("{p}/visibleWhen"),
                        format!("{w}{SEP}Görünme koşulu"),
                        v,
                    );
                }
            }
        }
    }
}

/// The visit a nested walk takes: the same `put`, seen as a visit.
fn put_back<'p>(
    put: &'p mut impl FnMut(&str, FieldUseHow, String, String, &mut String),
) -> impl FnMut(&Site<'_>, &mut String) + 'p {
    move |s: &Site<'_>, v: &mut String| put(s.layer, s.how, s.path.clone(), s.place.clone(), v)
}

/// A layer's own places (it and, for a group, its layers').
fn node_sites(node: &mut LayerNode, f: &mut Visit<'_>) {
    let id = node.id.clone();
    let w = format!("“{}” katmanı", node.name);
    let at = format!("layers/{id}");
    let mut put = |how, path: String, place: String, value: &mut String| {
        f(
            &Site {
                layer: &id,
                how,
                path,
                place,
                kept: false,
            },
            value,
        );
    };
    // The subtypes name the fields by key (docs/adr/0422 §1).
    if let Some(sub) = node.subtypes.as_mut() {
        put(
            FieldUseHow::Key,
            format!("{at}/subtypes/field"),
            format!("{w}{SEP}Alt türler{SEP}Alt tür alanı"),
            &mut sub.field,
        );
        for (i, t) in sub.types.iter_mut().enumerate() {
            let words = if t.label.is_empty() {
                t.code.clone()
            } else {
                t.label.clone()
            };
            for (j, r) in t.fields.iter_mut().enumerate() {
                put(
                    FieldUseHow::Key,
                    format!("{at}/subtypes/types/{i}/fields/{j}/field"),
                    format!("{w}{SEP}Alt türler{SEP}“{words}”"),
                    &mut r.field,
                );
            }
        }
    }
    for (i, s) in node.table_styles.iter_mut().enumerate() {
        let rule = format!("{w}{SEP}Koşullu biçim, {}. kural", i + 1);
        if let Some(field) = s.field.as_mut() {
            put(
                FieldUseHow::Name,
                format!("{at}/tableStyles/{i}/field"),
                format!("{rule}{SEP}Alan"),
                field,
            );
        }
        put(
            FieldUseHow::Expression,
            format!("{at}/tableStyles/{i}/condition"),
            format!("{rule}{SEP}Koşul"),
            &mut s.condition,
        );
    }
    if let Some(e) = node
        .filter
        .as_mut()
        .and_then(|f| f.expression.as_mut())
        .filter(|e| !e.is_empty())
    {
        put(
            FieldUseHow::Expression,
            format!("{at}/filter/expression"),
            format!("{w}{SEP}Süzgeç"),
            e,
        );
    }
    if let Some(t) = node
        .style
        .label
        .as_mut()
        .and_then(|l| l.text.as_mut())
        .filter(|t| !t.is_empty())
    {
        put(
            FieldUseHow::Expression,
            format!("{at}/style/label/text"),
            format!("{w}{SEP}Etiket{SEP}Metin"),
            t,
        );
    }
    if let Some(labels) = node.style.labels.as_mut() {
        for (c, class) in labels.classes.iter_mut().enumerate() {
            let cw = format!("{w}{SEP}“{}” etiket sınıfı", class.name);
            if let Some(v) = class.when.as_mut().filter(|v| !v.is_empty()) {
                put(
                    FieldUseHow::Expression,
                    format!("{at}/style/labels/classes/{c}/when"),
                    format!("{cw}{SEP}Koşul"),
                    v,
                );
            }
            if let Some(v) = class.style.text.as_mut().filter(|v| !v.is_empty()) {
                put(
                    FieldUseHow::Expression,
                    format!("{at}/style/labels/classes/{c}/style/text"),
                    format!("{cw}{SEP}Metin"),
                    v,
                );
            }
        }
    }
    if let Some(time) = node.time.as_mut() {
        if !time.start.is_empty() {
            put(
                FieldUseHow::Name,
                format!("{at}/time/start"),
                format!("{w}{SEP}Zaman{SEP}Başlangıç"),
                &mut time.start,
            );
        }
        if let Some(v) = time.end.as_mut().filter(|v| !v.is_empty()) {
            put(
                FieldUseHow::Name,
                format!("{at}/time/end"),
                format!("{w}{SEP}Zaman{SEP}Bitiş"),
                v,
            );
        }
        if let Some(v) = time.key.as_mut().filter(|v| !v.is_empty()) {
            put(
                FieldUseHow::Name,
                format!("{at}/time/key"),
                format!("{w}{SEP}Zaman{SEP}Kimlik"),
                v,
            );
        }
    }
    if let Some(card) = node.card.as_mut() {
        if let Some(v) = card.title.as_mut().filter(|v| !v.is_empty()) {
            put(
                FieldUseHow::Template,
                format!("{at}/card/title"),
                format!("{w}{SEP}Bilgi kartı{SEP}Başlık"),
                v,
            );
        }
        for (i, v) in card.fields.iter_mut().enumerate() {
            put(
                FieldUseHow::Name,
                format!("{at}/card/fields/{i}"),
                format!("{w}{SEP}Bilgi kartı{SEP}Alanlar"),
                v,
            );
        }
        if let Some(v) = card.text.as_mut().filter(|v| !v.is_empty()) {
            put(
                FieldUseHow::Template,
                format!("{at}/card/text"),
                format!("{w}{SEP}Bilgi kartı{SEP}Metin"),
                v,
            );
        }
    }
    for (i, a) in node.actions.iter_mut().enumerate() {
        let place = format!("{w}{SEP}“{}” eylemi{SEP}Hedef", a.name);
        put(
            FieldUseHow::Template,
            format!("{at}/actions/{i}/target"),
            place,
            &mut a.target,
        );
    }
}

fn nodes_sites(nodes: &mut [LayerNode], f: &mut Visit<'_>) {
    for n in nodes {
        if n.kind == kentos_contracts::LayerNodeType::Layer {
            node_sites(n, f);
        }
        nodes_sites(&mut n.children, f);
    }
}

/// Whether a place's value names `field`.
fn names(how: FieldUseHow, value: &str, field: &LayerField) -> bool {
    match how {
        FieldUseHow::Key => value == field.key(),
        FieldUseHow::Name => value == field.name,
        FieldUseHow::Expression => names_field(value, &field.name),
        FieldUseHow::Template => template_names_field(value, &field.name),
    }
}

/// Where `field` of `layer` is named, in the order of docs/adr/0414 §1.
pub fn uses(
    settings: &ProjectSettings,
    nodes: &[LayerNode],
    layer: &str,
    field: &LayerField,
) -> Vec<FieldUse> {
    let mut out = Vec::new();
    let mut visit = |s: &Site<'_>, v: &mut String| {
        if s.layer == layer && names(s.how, v, field) {
            out.push(FieldUse {
                place: s.place.clone(),
                path: s.path.clone(),
                how: s.how,
                kept: s.kept,
            });
        }
    };
    let mut settings = settings.clone();
    settings_sites(&mut settings, nodes, &mut visit);
    let mut nodes = nodes.to_vec();
    nodes_sites(&mut nodes, &mut visit);
    out
}

/// The keys of `layer`'s fields that a place names by key (a form item, a list, a relation): a
/// field so named keeps its key when it is renamed (§2).
pub fn keyed(settings: &ProjectSettings, layer: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut visit = |s: &Site<'_>, v: &mut String| {
        if s.layer == layer && s.how == FieldUseHow::Key && !out.contains(v) {
            out.push(v.clone());
        }
    };
    settings_sites(&mut settings.clone(), &[], &mut visit);
    out
}

/// The keys of `node`'s fields its own places name by key (its subtypes, docs/adr/0422): they too
/// stay when a field is renamed.
pub fn node_keyed(node: &LayerNode) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut visit = |s: &Site<'_>, v: &mut String| {
        if s.how == FieldUseHow::Key && !out.contains(v) {
            out.push(v.clone());
        }
    };
    node_sites(&mut node.clone(), &mut visit);
    out
}

/// The renaming of one value: a name's equal, an expression's or a template's references.
fn renamed(how: FieldUseHow, value: &str, from: &str, to: &str) -> Option<String> {
    match how {
        FieldUseHow::Key => None,
        FieldUseHow::Name => (value == from).then(|| to.to_owned()),
        FieldUseHow::Expression => rename_field(value, from, to),
        FieldUseHow::Template => rename_in_template(value, from, to),
    }
}

/// The settings with `layer`'s field `from` named `to` in the forms' drafts and lists (§2): the
/// keys stay (the field keeps its key), the published releases are kept. Returns how many places
/// changed.
pub fn rename_settings(settings: &mut ProjectSettings, layer: &str, from: &str, to: &str) -> usize {
    let mut n = 0;
    let mut visit = |s: &Site<'_>, v: &mut String| {
        if s.layer == layer
            && !s.kept
            && let Some(next) = renamed(s.how, v, from, to)
        {
            *v = next;
            n += 1;
        }
    };
    settings_sites(settings, &[], &mut visit);
    n
}

/// A layer with its field `from` named `to` in its own places (§2). Returns how many changed.
pub fn rename_node(node: &mut LayerNode, from: &str, to: &str) -> usize {
    let mut n = 0;
    let mut visit = |s: &Site<'_>, v: &mut String| {
        if let Some(next) = renamed(s.how, v, from, to) {
            *v = next;
            n += 1;
        }
    };
    node_sites(node, &mut visit);
    n
}
