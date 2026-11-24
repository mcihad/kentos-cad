//! The mass haul diagrams on an alignment (docs/adr/0995). Where a view goes under everything drawn is the core's
//! (view_place.rs): Harita's pipe profiles use it too.

/// The mass haul diagrams whose volume's corridor is on `alignment` (docs/adr/0995): their ids and names, for KGM's
/// earthwork rows.
pub(crate) fn hauls_on(doc: Option<&kentos_domain::Document>, alignment: Option<&str>) -> Vec<(String, String)> {
    let (Some(doc), Some(alignment)) = (doc, alignment) else { return Vec::new() };
    doc.road_hauls()
        .iter()
        .filter(|h| {
            doc.road_volumes()
                .iter()
                .find(|v| v.id == h.volume)
                .and_then(|v| doc.road_corridors().iter().find(|c| c.id == v.corridor))
                .is_some_and(|c| c.alignment == alignment)
        })
        .map(|h| (h.id.clone(), h.name.clone()))
        .collect()
}
