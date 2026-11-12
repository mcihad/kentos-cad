//! A tag has one positional grip. Use the existing semantic move command:
//! generic EntityGeometry intentionally has no tag representation.
use super::*;
use kentos_commands::{EntitiesTransform, Entity, Transform};
use kentos_native_application::transform;

pub(super) fn mixed(grips: &[HotGrip], cx: &Context<'_>) -> bool {
    grips
        .iter()
        .any(|g| matches!(cx.doc.get(g.slot), Some(Entity::Tag(_))))
        && !grips
            .iter()
            .all(|g| matches!(cx.doc.get(g.slot), Some(Entity::Tag(_))))
}

impl Select {
    pub(super) fn commit_tag_grip(
        &mut self,
        grips: &[HotGrip],
        dx: f64,
        dy: f64,
        copy: bool,
        cx: &mut Context<'_>,
    ) -> bool {
        if !grips
            .iter()
            .any(|g| matches!(cx.doc.get(g.slot), Some(Entity::Tag(_))))
        {
            return false;
        }
        if mixed(grips, cx) {
            cx.say(
                Level::Warn,
                "Etiket tutamacını diğer nesnelerden ayrı taşıyın; hiçbir nesne değiştirilmedi.",
            );
            return true;
        }
        // A layer can be locked while a hot grip waits. Do not partially
        // move the other hot tags if that happens.
        if grips.iter().any(|g| {
            cx.doc
                .get(g.slot)
                .is_some_and(|e| cx.doc.layers().is_locked(&e.base().layer_id))
        }) {
            cx.say(Level::Warn, "Etiketin katmanı kilitli; kilidi açıp yeniden taşıyın. Hiçbir nesne değiştirilmedi.");
            return true;
        }
        let input = EntitiesTransform {
            uids: grips
                .iter()
                .filter_map(|g| cx.doc.uid(g.slot))
                .map(|u| u.to_string())
                .collect(),
            transform: Transform::Move { dx, dy },
            copy: copy.then_some(true),
            layer_id: None,
            expected_revision: None,
        };
        points::written(
            transform::execute(&mut ExecutionContext::new(cx.doc), input),
            cx,
        );
        true
    }
}
