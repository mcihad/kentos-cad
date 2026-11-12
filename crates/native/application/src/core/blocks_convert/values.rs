//! Text variations become explicit attributes only when the existing attribute model can retain the result.
use super::{entity, refuse};
use crate::checks::Stop;
use kentos_commands::{AttributeDefinition, Entity};
use kentos_domain::Document;
use std::collections::BTreeMap;

pub(super) fn definitions(
    source: &[Entity],
    enabled: bool,
) -> Result<(Vec<Entity>, Vec<AttributeDefinition>), Stop> {
    if !enabled {
        return Ok((source.to_vec(), Vec::new()));
    }
    let mut entities = Vec::new();
    let mut attributes = Vec::new();
    let mut text_seen = false;
    for (k, e) in source.iter().enumerate() {
        let Entity::Text(t) = e else {
            if text_seen {
                return Err(refuse(
                    "Özniteliğe dönüşecek yazılar bileşenin çizim sırasında sonda olmalı; kapatma sırası değiştirilmedi.",
                    "source",
                ));
            }
            entities.push(e.clone());
            continue;
        };
        text_seen = true;
        let mut look = t.base.look.clone();
        look.order = None;
        if t.face != Default::default()
            || t.mask
            || t.path.is_some()
            || t.annotative.is_some()
            || t.label_of.is_some()
            || t.paragraph != Default::default()
            || t.base.color.is_some()
            || t.base.line_weight.is_some()
            || look != Default::default()
            || t.text.is_empty()
            || t.text.contains(['\n', '\r'])
        {
            return Err(refuse(
                "Bu yazının yazı tipi, fonu, paragrafı veya özel görünüşü mevcut öznitelik modelinde tam temsil edilmiyor. Yazı kaybedilmedi; aynı metinlerle normal blok dönüşümü kullanın.",
                format!("source[{k}]"),
            ));
        }
        attributes.push(AttributeDefinition {
            tag: format!("TEXT_{}", attributes.len() + 1),
            prompt: Some(format!("Yazı {}", attributes.len() + 1)),
            value: Some(t.text.clone()),
            p: t.p,
            height: t.height,
            rotation: t.rotation,
            align: t.align,
            width_factor: t.width_factor,
            options: Default::default(),
        });
    }
    if attributes.is_empty() {
        return Err(refuse(
            "Özniteliğe dönüştürülecek tek satırlı yazı seçilmedi.",
            "source",
        ));
    }
    Ok((entities, attributes))
}
pub(super) fn attributes(
    doc: &Document,
    members: &[String],
    source: &[Entity],
    definitions: &[AttributeDefinition],
) -> Result<BTreeMap<String, String>, Stop> {
    let mut result = BTreeMap::new();
    let mut at = 0;
    if definitions.is_empty() {
        return Ok(result);
    }
    for (k, e) in source.iter().enumerate() {
        if !matches!(e, Entity::Text(_)) {
            continue;
        }
        let (target, _) = entity(doc, &members[k])?;
        let Entity::Text(t) = target else {
            return Err(refuse("Adayın yazı eşlemesi değişti.", "groups"));
        };
        if t.text.is_empty() || t.text.contains(['\n', '\r']) {
            return Err(refuse(
                "Boş veya çok satırlı değer tek satırlı özniteliğe sessizce değiştirilmedi.",
                "groups",
            ));
        }
        let Some(definition) = definitions.get(at) else {
            return Err(refuse("Öznitelik eşlemesi eksik.", "groups"));
        };
        result.insert(definition.tag.clone(), t.text.clone());
        at += 1;
    }
    Ok(result)
}
