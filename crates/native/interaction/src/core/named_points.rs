//! A point named in the command line (docs/adr/0152 §4, 0188 §3): `#ad` takes the place of the one point so named.

use crate::Vec2;

/// The place of the one point named `name` (docs/adr/0152 §4; the web's
/// `namedPoint`), or why none is taken, in `#ad`'s words. The name is
/// matched with the points' labels, the spaces round them dropped; hidden
/// layers count too: a name is an identity. Nokta adından takes it too
/// (docs/adr/0188 §3).
pub(crate) fn named_point_at(doc: &kentos_domain::Document, name: &str) -> Result<Vec2, String> {
    let found: Vec<Vec2> = doc
        .entities()
        .filter_map(|e| match e {
            kentos_commands::Entity::Point(q)
                if q.base.label.as_deref().map(crate::js_trim) == Some(name) =>
            {
                Some(Vec2::new(q.p.x, q.p.y))
            }
            _ => None,
        })
        .collect();
    match found.as_slice() {
        [] => Err(format!("#{name}: bu adda nokta yok.")),
        [p] => Ok(*p),
        more => Err(format!(
            "#{name}: bu adda {} nokta var; koordinatı yazın.",
            more.len()
        )),
    }
}
