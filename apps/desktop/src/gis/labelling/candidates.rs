//! Denetle's candidates (docs/adr/9145): before the engine places, which of the layer's objects in the counted window
//! have a label text, a sample of it, and why a layer has none, each reason apart: its labelling Yok, the layer hidden,
//! no object, none in the window, the text empty on every one, out of the label's scale range, smaller than its least
//! object; and a template's field the layer does not have, which the engine writes as typed. The scale and size rules
//! are the engine's own: the range read as every range is (`scale::visible_at`), the least object the bounds' shorter
//! side on the screen.

use kentos_commands::{Entity, LabelStyle, LabelsMode, LayerNode};
use kentos_geometry_core::entity::{Shape, entity_bounds};
use kentos_geometry_core::scale::{denominator, visible_at};
use kentos_native_application::expression_unit::reading_unit;
use kentos_native_application::geometry::shape;
use kentos_native_application::label_texts::{layer_texts, object_texts};

use crate::exchange::words::Kind;

/// The objects a sample is looked for in, from the layer's first: enough to pass a few empty ones, few enough to read
/// on every key.
pub(super) const SAMPLE_OBJECTS: usize = 64;
/// The objects whose attributes the field picker offers beside the layer's declared fields.
const PICKER_OBJECTS: usize = 500;
/// A sample longer than this is cut.
const SAMPLE_CHARS: usize = 40;

/// What Denetle found of a layer's label candidates in the window.
#[derive(Debug, Default)]
pub(super) struct Candidates {
    /// The labelling is Yok.
    pub off: bool,
    /// The layer is hidden: the engine labels no object of it.
    pub hidden: bool,
    /// The layer's objects.
    pub objects: usize,
    /// Of them, those in the window.
    pub in_view: usize,
    /// Of those, the ones with a label text.
    pub with_text: usize,
    /// The text of the first of them.
    pub sample: Option<String>,
    /// Of those with a text, the ones out of their class's scale range.
    pub out_of_scale: usize,
    /// The range of the first such class: its farthest and nearest ends in pixels a metre.
    pub range: (Option<f64>, Option<f64>),
    /// Of those with a text in range, the ones smaller on the screen than their class's least object.
    pub small: usize,
    /// The first such class's least object, px.
    pub least: f64,
    /// The template's fields every object in the window has empty.
    pub empty_fields: Vec<String>,
    /// The template's fields the layer neither declares nor any object in the window holds.
    pub unknown: Vec<String>,
    /// The layer's fields, declared and held, for the words.
    pub fields: Vec<String>,
}

/// The fields a template names (`{durum}`, `{Ada}/{Parsel}`), each once; `{label}`, the object's label, is none.
pub(super) fn template_fields(template: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else { break };
        let name = after[..close].trim();
        if !name.is_empty() && name != "label" && !name.starts_with('$') && !out.iter().any(|n| n == name) {
            out.push(name.to_owned());
        }
        rest = &after[close + 1..];
    }
    out
}

/// The styles of a draft layer's classes, in the engine's order: the rules' or the single label's.
pub(super) fn class_styles(node: &LayerNode) -> Vec<&LabelStyle> {
    match node.style.labels.as_ref() {
        Some(l) if l.mode == LabelsMode::Rules => l.classes.iter().map(|c| &c.style).collect(),
        _ => node.style.label.iter().collect(),
    }
}

/// The fields the classes' templates name, where the class has no expression (an expression names its own).
fn templates_fields(node: &LayerNode) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for s in class_styles(node) {
        if s.text.is_some() {
            continue;
        }
        for f in template_fields(s.template.as_deref().unwrap_or_default()) {
            if !out.contains(&f) {
                out.push(f);
            }
        }
    }
    out
}

/// Whether `name` is one of `names`, any case (the template's own rule).
fn holds(names: &[String], name: &str) -> bool {
    let folded = name.to_lowercase();
    names.iter().any(|n| n.to_lowercase() == folded)
}

/// The value of `name` on `e`, any case.
fn value_of<'e>(e: &'e Entity, name: &str) -> Option<&'e str> {
    let attrs = &e.base().attrs;
    attrs.get(name).map(String::as_str).or_else(|| {
        let folded = name.to_lowercase();
        attrs.iter().find(|(k, _)| k.to_lowercase() == folded).map(|(_, v)| v.as_str())
    })
}

/// The label texts of `list` as `node` labels them (the engine's texts, from the same code).
pub(super) fn texts_of(doc: &kentos_domain::Document, node: &LayerNode, list: &[&Entity]) -> Vec<Vec<(u16, String)>> {
    let shapes: Vec<Shape> = list.iter().map(|e| shape(e)).collect();
    let t = layer_texts(Some(node), reading_unit(doc.settings()));
    let names = |id: &str| doc.layers().get(id).map_or_else(|| id.to_owned(), |n| n.name.clone());
    object_texts(list, &t, &names, &|i| shapes.get(i)).into_iter().map(|(texts, _)| texts).collect()
}

/// A sample cut to its first [`SAMPLE_CHARS`] letters, on one line.
pub(super) fn cut(text: &str) -> String {
    let one: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.chars().count() > SAMPLE_CHARS {
        format!("{}…", one.chars().take(SAMPLE_CHARS).collect::<String>())
    } else {
        one
    }
}

/// The candidates of `node` (the window's draft of the layer) in `window` at `scale` px a metre.
pub(super) fn count(doc: &kentos_domain::Document, node: &LayerNode, window: [f64; 4], scale: f64) -> Candidates {
    let mut c = Candidates {
        off: node.style.labels.as_ref().is_some_and(|l| l.mode == LabelsMode::Off),
        hidden: !doc.layers().is_visible(&node.id),
        fields: picker_fields(doc, &node.id),
        ..Candidates::default()
    };
    let objects: Vec<&Entity> = doc.by_layer(&node.id).collect();
    c.objects = objects.len();
    let in_view: Vec<(&Entity, [f64; 4])> = objects
        .iter()
        .filter_map(|e| {
            let b = entity_bounds(&shape(e));
            let seen = b.max_x >= window[0] && b.min_x <= window[2] && b.max_y >= window[1] && b.min_y <= window[3];
            seen.then_some((*e, [b.min_x, b.min_y, b.max_x, b.max_y]))
        })
        .collect();
    c.in_view = in_view.len();
    if c.off || c.in_view == 0 {
        return c;
    }
    let named = templates_fields(node);
    c.unknown = named
        .iter()
        .filter(|f| !holds(&c.fields, f) && in_view.iter().all(|(e, _)| value_of(e, f).is_none()))
        .cloned()
        .collect();
    c.empty_fields = named
        .iter()
        .filter(|f| !c.unknown.contains(f))
        .filter(|f| in_view.iter().all(|(e, _)| value_of(e, f).is_none_or(|v| v.trim().is_empty())))
        .cloned()
        .collect();
    let list: Vec<&Entity> = in_view.iter().map(|(e, _)| *e).collect();
    let texts = texts_of(doc, node, &list);
    let styles = class_styles(node);
    let n = denominator(scale);
    for ((_, b), texts) in in_view.iter().zip(&texts) {
        let texts: Vec<&(u16, String)> = texts.iter().filter(|(_, t)| !t.is_empty()).collect();
        let Some((class, first)) = texts.first().map(|(k, t)| (*k, t)) else {
            continue;
        };
        c.with_text += 1;
        if c.sample.is_none() {
            c.sample = Some(cut(first));
        }
        // An object counts once, by its first class, as the engine's first label of it is its main one.
        let Some(style) = styles.get(class as usize) else {
            continue;
        };
        if !visible_at(n, style.min_scale.map(denominator), style.max_scale.map(denominator)) {
            if c.out_of_scale == 0 {
                c.range = (style.min_scale, style.max_scale);
            }
            c.out_of_scale += 1;
        } else if let Some(m) = style.min_feature_px
            && (b[2] - b[0]).min(b[3] - b[1]) * scale < m
        {
            if c.small == 0 {
                c.least = m;
            }
            c.small += 1;
        }
    }
    c
}

/// The template's fields the layer neither declares nor any of its first objects holds: the engine writes them as
/// typed.
pub(super) fn unknown_fields(doc: &kentos_domain::Document, node: &LayerNode) -> Vec<String> {
    let declared: Vec<String> = node.fields.iter().map(|f| f.name.clone()).collect();
    let first: Vec<&Entity> = doc.by_layer(&node.id).take(PICKER_OBJECTS).collect();
    templates_fields(node)
        .into_iter()
        .filter(|f| !holds(&declared, f) && first.iter().all(|e| value_of(e, f).is_none()))
        .collect()
}

/// The fields the template's picker offers: the layer's declared fields, then the attributes its first objects hold.
pub(super) fn picker_fields(doc: &kentos_domain::Document, layer: &str) -> Vec<String> {
    let mut out: Vec<String> = doc
        .layers()
        .get(layer)
        .map(|n| n.fields.iter().map(|f| f.name.clone()).collect())
        .unwrap_or_default();
    for e in doc.by_layer(layer).take(PICKER_OBJECTS) {
        for k in e.base().attrs.keys() {
            if !holds(&out, k) {
                out.push(k.clone());
            }
        }
    }
    out
}

/// A field list as the words name it: the first few, and how many more.
fn field_list(fields: &[String]) -> String {
    const SHOWN: usize = 6;
    let mut words = fields.iter().take(SHOWN).cloned().collect::<Vec<_>>().join(", ");
    if fields.len() > SHOWN {
        words.push_str(&format!(" ve {} alan daha", fields.len() - SHOWN));
    }
    words
}

/// The words for template fields the layer does not have.
pub(super) fn unknown_words(unknown: &[String], fields: &[String]) -> String {
    let named: Vec<String> = unknown.iter().map(|f| format!("{{{f}}}")).collect();
    let have = if fields.is_empty() {
        "Katmanda alan yok.".to_owned()
    } else {
        format!("Katmanın alanları: {}.", field_list(fields))
    };
    format!("Şablondaki {} katmanın alanı değil: etikette yazıldığı gibi çıkar. {have}", named.join(", "))
}

/// A range's end as the words say it: 1:N, or none.
fn end(px: Option<f64>) -> String {
    let n = super::form::denominator_text(px);
    if n.is_empty() { "sınırsız".to_owned() } else { format!("1:{n}") }
}

/// Denetle's words for the candidates, after its counts: the candidates and a sample, then each reason apart. `none`:
/// the engine placed, left out and hid nothing; `unplaced`: the labels it found no free place for.
pub(super) fn lines(c: &Candidates, scale: f64, none: bool, unplaced: u32) -> Vec<(Kind, String)> {
    let mut out: Vec<(Kind, String)> = Vec::new();
    if c.off {
        out.push((Kind::Info, "Etiketleme “Yok”: katman etiketlenmez.".to_owned()));
        return out;
    }
    if c.objects == 0 {
        out.push((Kind::Warn, "Etiket adayı yok: katmanın nesnesi yok.".to_owned()));
        return out;
    }
    if c.hidden {
        out.push((Kind::Warn, "Katman gizli: gizli katmanın etiketi çizilmez; katmanı görünür yapın.".to_owned()));
        return out;
    }
    if c.in_view == 0 {
        out.push((
            Kind::Warn,
            format!(
                "Etiket adayı yok: katmanın {} nesnesinin hiçbiri görünür alanda değil; görünümü onlara yaklaştırın.",
                c.objects
            ),
        ));
        return out;
    }
    let empty = c.in_view - c.with_text;
    let mut candidates = format!("Adaylar: görünümde {} nesne, etiket metni olan {}", c.in_view, c.with_text);
    if empty > 0 {
        candidates.push_str(&format!(", metni boş {empty}"));
    }
    match &c.sample {
        Some(s) => candidates.push_str(&format!("; örnek metin “{s}”.")),
        None => candidates.push('.'),
    }
    out.push((Kind::Info, candidates));
    if !c.unknown.is_empty() {
        out.push((Kind::Warn, unknown_words(&c.unknown, &c.fields)));
    }
    if c.with_text == 0 {
        let why = if c.empty_fields.is_empty() {
            format!(
                "Etiket adayı yok: görünümdeki {} nesnenin etiket metni boş; Metin sekmesindeki şablonu ya da ifadeyi denetleyin.",
                c.in_view
            )
        } else {
            let named: Vec<String> = c.empty_fields.iter().map(|f| format!("{{{f}}}")).collect();
            format!("Etiket adayı yok: şablondaki {} görünümdeki {} nesnenin hepsinde boş.", named.join(", "), c.in_view)
        };
        out.push((Kind::Warn, why));
        return out;
    }
    if c.out_of_scale > 0 {
        out.push((
            Kind::Warn,
            format!(
                "{} aday bu ölçekte gösterilmez: görünüm {}; etiketlerin görünür ölçekleri en uzak {}, en yakın {}. Öncelik sekmesindeki Görünür ölçekleri genişletin ya da görünümü aralığa getirin.",
                c.out_of_scale,
                end(Some(scale)),
                end(c.range.0),
                end(c.range.1)
            ),
        ));
    }
    if c.small > 0 {
        out.push((
            Kind::Warn,
            format!(
                "{} aday en küçük nesne sınırından ({} px) küçük: yaklaştırın ya da Öncelik sekmesindeki En küçük nesneyi küçültün.",
                c.small,
                super::form::shown(c.least)
            ),
        ));
    }
    if unplaced > 0 {
        out.push((
            Kind::Warn,
            format!(
                "{unplaced} etiket başka etiket ya da engelle çakıştığı için yer bulamadı: Öncelik sekmesinde önceliği artırın ya da Çakışma'yı Gerekirse yapın; Yerleşmeyen etiketleri göster yerlerini çizer."
            ),
        ));
    }
    if none && c.out_of_scale + c.small < c.with_text {
        out.push((
            Kind::Warn,
            format!("Etiket adayı motora ulaşmadı: görünümde metni olan {} nesne var.", c.with_text - c.out_of_scale - c.small),
        ));
    }
    out
}
