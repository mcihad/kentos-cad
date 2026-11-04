//! Read-only checks against live field sources. Opening an old file never rewrites its cached text.
use std::collections::{HashMap, HashSet};

use kentos_commands::{
    Entity, FieldSource, InspectionFinding, InspectionSeverity, ObjectValue, TextEntity,
};
use kentos_domain::{Document, Uuid};

use super::annotations::finding;

/// One text's source graph is bounded independently of the drawing scan.
const SOURCE_LIMIT: usize = 512;

pub(super) fn findings(doc: &Document, uid: Uuid, text: &TextEntity) -> Vec<InspectionFinding> {
    let mut out = Vec::new();
    let mut sources: Vec<_> = text
        .paragraph
        .fields
        .iter()
        .filter_map(|f| f.source.object())
        .map(|u| Uuid::from_bytes(u.0).to_string())
        .collect();
    let emit = |code, severity, message, repair| {
        finding(uid, &sources, code, severity, vec![text.p], message, repair)
    };

    if let Some(style) = text
        .face
        .text_style
        .as_ref()
        .and_then(|id| doc.settings().text_styles.iter().find(|s| &s.id == id))
        && let Some(file) = &style.font_file
        && kentos_geometry_core::text::fonts::resolve(file).0 != Some(style.font.id())
    {
        out.push(emit("text_font_substituted", InspectionSeverity::Warning,
            format!("Özgün font: {file} · Çizilen font: {}. Özgün fontun metrikleriyle eşdeğerlik doğrulanmadı.", style.font.label()),
            "Yazı stilini ve cad.fonts.report sonucunu inceleyin; desteklenen fonta dönüşümü açıkça onaylayın."));
    }
    if text.paragraph.layout.as_ref().is_some_and(|l|l.engine.is_some()) {
        let shape=crate::geometry::shape(&Entity::Text(text.clone()));
        if let Some(place)=kentos_geometry_core::entity::TextPlace::of(&shape) {
            let font=crate::geometry::drawing_font(doc.settings().drawing_font);
            match place.unicode_plan(font).and_then(|p|p.contours().map(|_|p)) {
                Err(error)=>out.push(emit("text_layout_unsupported",InspectionSeverity::Error,
                    format!("{} Metin kısmen çizilemez veya farklı metrikle ölçülemez.",error.message(&text.text)),
                    "Desteklenmeyen karakteri/biçim sınırını inceleyin; yazıyı sessizce değiştirmeyin.")),
                Ok(plan)=> {
                    let requested=place.font_or(font).id();
                    let fallbacks:std::collections::BTreeSet<_>=plan.glyphs.iter()
                        .filter_map(|g|kentos_geometry_core::text::shaping::font_key(g.glyph.font))
                        .filter(|(family,_,_)|*family!=requested).map(|(family,_,_)|family).collect();
                    if !fallbacks.is_empty() {
                        out.push(emit("text_font_fallback",InspectionSeverity::Info,
                            format!("İstenen font: {requested} · Bu yazının bazı harfleri paketli {} ile çiziliyor. Ekran, ölçülen kutu ve PDF aynı harf ölçülerini kullanır.",fallbacks.into_iter().collect::<Vec<_>>().join(", ")),
                            "Karakter kapsamı için kullanılan paketli fontları inceleyin; metin veya stil otomatik değiştirilmez."));
                    }
                }
            }
        }
    }
    if text.paragraph.fields.is_empty() {
        return out;
    }
    let mut pending = vec![uid];
    let mut seen = HashSet::new();
    let mut graph = HashMap::new();
    let mut texts = Vec::new();
    let mut stale = false;
    let mut broken = false;
    let mut own_now = None;
    let mut capped = false;
    while let Some(source) = pending.pop() {
        if seen.contains(&source) {
            continue;
        }
        if seen.len() >= SOURCE_LIMIT {
            capped = true;
            break;
        }
        seen.insert(source);
        let Some(Entity::Text(t)) = doc.slot_of(source).and_then(|s| doc.get(s)) else {
            continue;
        };
        let reads: Vec<_> = t
            .paragraph
            .fields
            .iter()
            .filter_map(|f| match f.source {
                FieldSource::Object {
                    uid,
                    value: ObjectValue::Text,
                    ..
                } => Some(Uuid::from_bytes(uid.0)),
                _ => None,
            })
            .collect();
        pending.extend(reads.iter().copied());
        graph.insert(source, reads);
        texts.push((source, t));
    }
    sources.extend(seen.iter().filter(|&&u| u != uid).map(ToString::to_string));
    sources.sort();
    sources.dedup();
    let emit = |code, severity, message, repair| {
        finding(uid, &sources, code, severity, vec![text.p], message, repair)
    };
    let cyclic = cycle_nodes(&graph);
    let empty = HashSet::new();
    let letters = |u| match doc.slot_of(u).and_then(|s| doc.get(s)) {
        Some(Entity::Text(t)) => Some(t.text.clone()),
        _ => None,
    };
    for (source, t) in texts {
        let now = doc.fields_written(
            t,
            if cyclic.contains(&source) {
                &cyclic
            } else {
                &empty
            },
            &letters,
        );
        stale |= now.text != t.text || now.paragraph.fields != t.paragraph.fields;
        broken |= now.paragraph.fields.iter().any(|f| f.broken.is_some());
        if source == uid {
            own_now = Some(now.text);
        }
    }
    if capped {
        out.push(emit("text_field_unverified", InspectionSeverity::Error,
            "Alan kaynağı zinciri 512 nesnelik denetim sınırını aşıyor; saklanan metnin güncelliği doğrulanamadı.".into(),
            "Kaynak zincirini sadeleştirin; güncellik doğrulanmadan teslim çıktısı üretmeyin."));
    }
    if stale {
        let message = match own_now.filter(|now| *now != text.text) {
            Some(now) => format!(
                "Saklanan metin: {} · Kaynaktan okunan: {now}. Alan metni güncel değil.",
                text.text
            ),
            None => format!(
                "Saklanan metin: {}. Bağlı yazılardan en az birinin alanı güncel değil; kaynak zinciri yenilenmeli.",
                text.text
            ),
        };
        out.push(emit("text_field_stale", InspectionSeverity::Error, message,
            "Açıklama → Alanları yenile ile kaynakları güncelleyin; sonucu kontrol edin. Bu denetim dosyayı değiştirmez."));
    }
    if broken {
        out.push(emit("text_field_broken", InspectionSeverity::Warning,
            "Yazının veya bağlı yazı kaynağının alanı okunamıyor; silinmiş kaynak, eksik değer veya döngü var.".into(),
            "Kaynakları inceleyin; kaynağı onarın veya alanı açıkça sabit metne dönüştürün."));
    }
    out
}

/// Iterative DFS: long imported field chains never consume the process stack.
fn cycle_nodes(graph: &HashMap<Uuid, Vec<Uuid>>) -> HashSet<Uuid> {
    let mut done = HashSet::new();
    let mut order = Vec::new();
    let mut reverse: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for (&node, children) in graph {
        for &child in children {
            reverse.entry(child).or_default().push(node);
        }
        let mut stack = vec![(node, false)];
        while let Some((node, leave)) = stack.pop() {
            if leave {
                order.push(node);
            } else if done.insert(node) {
                stack.push((node, true));
                stack.extend(
                    graph
                        .get(&node)
                        .into_iter()
                        .flatten()
                        .map(|&child| (child, false)),
                );
            }
        }
    }
    done.clear();
    let mut cyclic = HashSet::new();
    for root in order.into_iter().rev() {
        if done.contains(&root) {
            continue;
        }
        let mut component = Vec::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if done.insert(node) {
                component.push(node);
                stack.extend(reverse.get(&node).into_iter().flatten().copied());
            }
        }
        if component.len() > 1 || graph.get(&root).is_some_and(|edges| edges.contains(&root)) {
            cyclic.extend(component);
        }
    }
    cyclic
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlapping_cycles_include_cross_edges_and_exclude_acyclic_readers() {
        let ids: Vec<_> = (1..=5).map(|i| Uuid::from_bytes([i; 16])).collect();
        let graph = HashMap::from([
            (ids[0], vec![ids[1], ids[3]]),
            (ids[1], vec![ids[2]]),
            (ids[2], vec![ids[0]]),
            (ids[3], vec![ids[1]]),
            (ids[4], vec![ids[0]]),
        ]);
        assert_eq!(cycle_nodes(&graph), ids[..4].iter().copied().collect());
    }
}
