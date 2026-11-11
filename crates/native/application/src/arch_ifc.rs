//! The building's IFC (docs/adr/8047): the adapter from KentOS's document to the IFC core. It gives `kentos_ifc::arch`
//! the section model (`sections::model`), the drawing's objects with their ids, the wall styles' names, the rooms' net
//! areas and the project's architecture settings; every IFC rule (classes, names, properties, geometry) is in the
//! kentos-ifc crate.

use kentos_domain::Document;
use kentos_ifc::arch::{Gap, Input, Written};
use kentos_ifc::building::Building;

use crate::sections;

/// The building to write: `stamp` its time (`YYYY-MM-DDThh:mm:ss`), `application` KentOS's name.
pub fn building(doc: &Document, stamp: &str, application: &str) -> (Building, Written) {
    let model = sections::model(doc);
    let entities: Vec<(u128, String, &kentos_commands::Entity)> = doc
        .entities_with_uids()
        .map(|(u, e)| (u.as_u128(), u.to_string(), e))
        .collect();
    let gaps: Vec<Gap> = model
        .gaps
        .iter()
        .map(|g| Gap {
            ring: g.ring.clone(),
            sill: g.sill,
            head: g.head,
            form: g.form,
            owner: g.owner.clone(),
        })
        .collect();
    let mut search = kentos_domain::space_follow::SpaceSearch::new(doc);
    let mut net_area = |room: &kentos_commands::SpaceFields| search.net_area(room);
    let wall_style_name = |style: Option<&str>| doc.wall_style_name(style);
    kentos_ifc::arch::building(Input {
        title: doc.name(),
        stamp,
        application,
        levels: model.levels.clone(),
        solids: &model.solids,
        owners: &model.owners,
        gaps,
        entities,
        wall_style_name: &wall_style_name,
        net_area: &mut net_area,
        arch: doc.settings().arch.as_ref(),
    })
}
