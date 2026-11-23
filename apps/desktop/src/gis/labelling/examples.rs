//! The labelling window's examples (docs/adr/9164): what the layer's labels will be before they are placed, on the
//! same data. The first text and the longest of the layer's first objects, how many of them have none (no label is
//! written for them), the scales the label shows at with the view's own among them or not, and how tall its letters
//! are on paper. The texts are the engine's own (`texts_of`), the range is read as the engine reads it
//! (`scale::visible_at`).

use kentos_commands::{Entity, LabelStyle, LayerNode};
use kentos_geometry_core::scale::{denominator, visible_at};

use super::candidates::{SAMPLE_OBJECTS, class_styles, cut, texts_of};
use crate::exchange::words::Kind;

/// A CSS px on paper, in millimetres (96 a inch).
const PX_MM: f64 = 25.4 / 96.0;

/// The texts of the layer's first objects: the first, how many before it had none, the longest and its letters, the
/// objects read and those without a text.
#[derive(Debug, Default, PartialEq)]
pub(super) struct Texts {
    pub first: Option<(String, usize)>,
    pub longest: Option<(String, usize)>,
    pub read: usize,
    pub empty: usize,
}

/// The first [`SAMPLE_OBJECTS`] objects' texts as `node` labels them.
pub(super) fn texts(doc: &kentos_domain::Document, node: &LayerNode) -> Texts {
    let list: Vec<&Entity> = doc.by_layer(&node.id).take(SAMPLE_OBJECTS).collect();
    let mut out = Texts { read: list.len(), ..Texts::default() };
    for (i, texts) in texts_of(doc, node, &list).iter().enumerate() {
        let Some(text) = texts.iter().map(|(_, t)| t).find(|t| !t.trim().is_empty()) else {
            out.empty += 1;
            continue;
        };
        let one = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let letters = one.chars().count();
        if out.first.is_none() {
            out.first = Some((cut(&one), i));
        }
        if out.longest.as_ref().is_none_or(|(_, n)| letters > *n) {
            out.longest = Some((one, letters));
        }
    }
    out
}

/// A range's end as the words say it: 1:N.
fn ratio(px: f64) -> String {
    format!("1:{}", super::form::denominator_text(Some(px)))
}

/// The scales `style` shows at, and whether the view's (`scale`, px a metre) is one: “1:5.000 ile 1:500 arasında
/// görünür; şimdiki görünüm 1:1.535, görünür.”
pub(super) fn scale_words(style: &LabelStyle, scale: f64) -> (Kind, String) {
    let range = match (style.min_scale, style.max_scale) {
        (None, None) => "Her ölçekte görünür".to_owned(),
        (Some(far), None) => format!("{} ve daha yakın ölçeklerde görünür", ratio(far)),
        (None, Some(near)) => format!("{} ve daha uzak ölçeklerde görünür", ratio(near)),
        (Some(far), Some(near)) => format!("{} ile {} arasında görünür", ratio(far), ratio(near)),
    };
    if !(scale.is_finite() && scale > 0.0) {
        return (Kind::Info, format!("Ölçek: {range}."));
    }
    let seen = visible_at(denominator(scale), style.min_scale.map(denominator), style.max_scale.map(denominator));
    let now = if seen { "görünür" } else { "görünmez" };
    (
        if seen { Kind::Info } else { Kind::Warn },
        format!("Ölçek: {range}; şimdiki görünüm {}, {now}.", ratio(scale)),
    )
}

/// How tall `style`'s letters are on paper: its size in CSS px at 96 a inch.
pub(super) fn print_words(style: &LabelStyle) -> String {
    let mm = style.size * PX_MM;
    let grows = match (style.grow, style.max_size) {
        (Some(_), Some(most)) => format!(" Ölçekle büyür, en çok {most} px ({:.1} mm).", most * PX_MM),
        (Some(_), None) => " Ölçekle büyür.".to_owned(),
        _ => String::new(),
    };
    format!("Baskı: yazı {} px, kâğıtta yaklaşık {mm:.1} mm.{grows}", style.size)
}

/// The window's example lines for `node` (its draft) with `objects` objects at the view's `scale` (px a metre).
pub(super) fn example_lines(
    doc: &kentos_domain::Document,
    node: &LayerNode,
    objects: usize,
    scale: f64,
) -> Vec<(Kind, String)> {
    let mut lines = Vec::new();
    let t = texts(doc, node);
    match &t.first {
        Some((text, 0)) => lines.push((Kind::Info, format!("Örnek metin (ilk nesne): “{text}”."))),
        Some((text, before)) => {
            lines.push((Kind::Info, format!("Örnek metin: “{text}”; ilk {before} nesnenin metni boş.")))
        }
        None if objects > 0 => lines.push((Kind::Warn, format!("İlk {} nesnenin etiket metni boş.", t.read))),
        None => {}
    }
    if let (Some((first, _)), Some((longest, letters))) = (&t.first, &t.longest)
        && cut(longest) != *first
    {
        lines.push((Kind::Info, format!("En uzun metin: “{}” ({letters} harf).", cut(longest))));
    }
    if t.first.is_some() && t.empty > 0 {
        lines.push((
            Kind::Info,
            format!("İlk {} nesnenin {} tanesinde metin boş; onlara etiket yazılmaz.", t.read, t.empty),
        ));
    }
    if let Some(style) = class_styles(node).first() {
        lines.push(scale_words(style, scale));
        lines.push((Kind::Info, print_words(style)));
    }
    lines
}

#[cfg(test)]
#[path = "examples_tests.rs"]
mod examples_tests;
