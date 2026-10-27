//! Read-only derived geometry for an explicit paper viewport; all records retain original model slots.
use kentos_commands::{CommandError, Entity, SheetModelScope};
use kentos_domain::Document;
use kentos_geometry_core::{entity::Shape, store::Store, Vec2};
pub use kentos_sheet::model_scope::Viewport;

pub fn resolve(doc: &Document, scope: &SheetModelScope) -> Result<Viewport, Box<CommandError>> {
    let fail = |code: &str, message: String| {
        Box::new(crate::checks::error(
            code,
            message,
            Some("modelScope".into()),
        ))
    };
    let kept = doc
        .sheets()
        .ok_or_else(|| fail("scope_mismatch", "Projede pafta kitabı yok.".into()))?;
    let (book, _) =
        kentos_sheet::project::read_project_sheets(kept).map_err(|e| fail(&e.code, e.message))?;
    Viewport::resolve_with_views(
        &book,
        &scope.sheet,
        &scope.item,
        &doc.settings().named_views,
    )
    .map_err(|e| fail(&e.code, e.message))
}

pub fn entity_at(entity: &Entity, scale: Option<f64>) -> Option<std::borrow::Cow<'_, Entity>> {
    use kentos_commands::annotative::{at_scale, AtScale};
    match scale.map(|s| at_scale(entity, s)) {
        Some(AtScale::Hidden) => None,
        Some(AtScale::Drawn(e)) => Some(std::borrow::Cow::Owned(*e)),
        _ => Some(std::borrow::Cow::Borrowed(entity)),
    }
}

pub fn record_at(entity: &Entity, scale: Option<f64>) -> Option<(f64, &str, bool, Shape)> {
    let base = entity.base();
    let represented = entity_at(entity, scale)?;
    Some((
        f64::from(base.id),
        base.layer_id.as_str(),
        base.label.as_deref().is_some_and(|s| !s.is_empty()),
        crate::geometry::shape(&represented),
    ))
}

pub fn visible_layers(doc: &Document, scope: &Viewport) -> std::collections::HashSet<String> {
    kentos_sheet::layers::map_layers(
        doc.layers().nodes(),
        &doc.settings().layer_states,
        &scope.layers,
        &scope.layer_overrides,
    )
    .into_iter()
    .map(|n| n.id.clone())
    .collect()
}

/// Cache the original ids within the viewport's boundary from the spatial tree; no scan or source clone on each
/// pointer.
pub fn apply_boundary(store: &mut Store, scope: &Viewport) {
    let ring = scope
        .model_boundary()
        .into_iter()
        .map(|p| Vec2::new(p[0], p[1]))
        .collect::<Vec<_>>();
    // Whatever the view hides or draws faint now (docs/adr/0716): the map's level may change without the boundary.
    let inside = store.touching_ring(&ring);
    store.set_scope_visible(Some(inside));
}

/// The paper source's hidden construction phases and non-plotting PDF underlays are not pick/snap targets.
pub fn hidden_entities(doc: &Document) -> Vec<f64> {
    let view = doc
        .settings()
        .arch
        .as_ref()
        .and_then(|a| a.phase_view)
        .unwrap_or_default();
    let by_uid = |u: &str| {
        kentos_domain::Uuid::parse_str(u)
            .ok()
            .and_then(|u| doc.slot_of(u))
            .and_then(|s| doc.get(s))
    };
    let no_plot: std::collections::HashSet<_> = doc
        .blocks()
        .iter()
        .filter(|b| {
            b.xref
                .as_ref()
                .and_then(|x| x.underlay.as_ref())
                .is_some_and(|u| !u.plot)
        })
        .map(|b| b.id)
        .collect();
    doc.entities()
        .filter(|e| {
            view.look(e.phase(by_uid)) == kentos_commands::PhaseLook::Hidden
                || matches!(e,Entity::Insert(i) if no_plot.contains(&i.block))
        })
        .map(|e| f64::from(e.base().id))
        .collect()
}
