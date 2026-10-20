//! A schedule's rows from the drawing's objects (docs/adr/0184 §3, §6): the core's rules (`ops::table`) given the
//! objects in the order asked and the project's units. Tablo ekle and Tabloyu güncelle (`cad.tables.edit`'s refresh,
//! docs/adr/0327 §4) read them alike.

use kentos_commands::{AreaUnit, DrawingUnit, Entity};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::compare::Attrs;
use kentos_geometry_core::ops::table::{self as rules, Cells, Listed, ScheduleKind, Units};
use kentos_project::format::{Axes, Format};

use crate::elevation::paths as elevated_paths;
use crate::geometry::shape;

/// The project's settings a schedule writes its numbers by.
pub fn units(f: &Format) -> Units {
    Units {
        cad: f.axes == Axes::Cad,
        unit: match f.unit {
            DrawingUnit::Mm => rules::Unit::Mm,
            DrawingUnit::Cm => rules::Unit::Cm,
            DrawingUnit::M => rules::Unit::M,
            DrawingUnit::In => rules::Unit::In,
            DrawingUnit::Ft => rules::Unit::Ft,
        },
        area_unit: match f.area_unit {
            AreaUnit::Donum => rules::AreaUnit::Donum,
            AreaUnit::Ha => rules::AreaUnit::Ha,
            AreaUnit::M2 => rules::AreaUnit::M2,
        },
        length_decimals: f.length_decimals,
        area_decimals: f.area_decimals,
    }
}

/// An object as a schedule reads it: its shape, paths with their
/// elevations, label and attributes.
pub fn listed(e: &Entity) -> Listed {
    let base = e.base();
    Listed {
        shape: shape(e),
        paths: elevated_paths(e),
        label: base.label.clone(),
        attrs: Attrs(
            base.attrs
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        ),
    }
}

/// The schedule of `kind` the objects at `slots` write (in the order given).
pub fn schedule(doc: &Document, kind: ScheduleKind, slots: &[Slot], f: &Format) -> Cells {
    let objects: Vec<Listed> = slots
        .iter()
        .filter_map(|s| doc.get(*s))
        .map(listed)
        .collect();
    rules::schedule(kind, &objects, &units(f))
}

