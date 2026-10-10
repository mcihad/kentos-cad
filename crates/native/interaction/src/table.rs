//! Tablo (docs/adr/0184 §3–§6): what the desktop's table windows and tool
//! read of a drawing, the web's `app/tables.ts`. The rules are the
//! geometry core's (`ops::table`): a schedule's rows from objects, a file's
//! rows as cells, a new table's sizes; here the drawing's objects are given
//! to them in its order, and a table is made of their cells.

use std::collections::HashSet;

use kentos_contracts::{
    AreaUnit, DrawingUnit, Entity, EntityBase, EntityGeometry, TableAlign, TableGrid, TableSheet,
    TableSource, TextFace,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::compare::Attrs;
use kentos_geometry_core::ops::table::{self as rules, Cells, Listed, ScheduleKind, Units};
use kentos_native_application::elevation::paths as elevated_paths;
use kentos_native_application::geometry::{drawing_font, entity_of, shape};

use crate::Vec2;
use crate::format::{Axes, Format};

/// The project's settings a schedule writes its numbers by.
pub fn units(f: &Format) -> Units {
    Units {
        cad: f.axes == Axes::Cad,
        unit: match f.unit {
            DrawingUnit::Mm => rules::Unit::Mm,
            DrawingUnit::Cm => rules::Unit::Cm,
            DrawingUnit::M => rules::Unit::M,
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

/// The objects at `slots`, in the drawing's order.
pub fn in_order(doc: &Document, slots: &[Slot]) -> Vec<Slot> {
    let wanted: HashSet<u32> = slots.iter().map(|s| s.0).collect();
    doc.entities()
        .map(|e| Slot(e.base().id))
        .filter(|s| wanted.contains(&s.0))
        .collect()
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

/// The source a schedule of `kind` keeps: the objects' persistent ids, in order.
pub fn source_of(doc: &Document, kind: ScheduleKind, slots: &[Slot]) -> TableSource {
    let objects = slots
        .iter()
        .filter_map(|s| doc.uid(*s))
        .map(|u| kentos_contracts::EntityId(*u.as_bytes()))
        .collect();
    match kind {
        ScheduleKind::Coordinates => TableSource::Coordinates { objects },
        ScheduleKind::Areas => TableSource::Areas { objects },
        ScheduleKind::Attributes => TableSource::Attributes { objects },
    }
}

/// A blank table's columns, text heights wide (Boş tablo: room to type in;
/// the web's `BLANK_COLUMN`).
pub const BLANK_COLUMN: f64 = 8.0;

/// What a new table looks like besides its cells: its heading row, text
/// height, face, lines and frame width (Tablo ekle's choices).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Look {
    pub header: bool,
    pub height: f64,
    pub face: TextFace,
    pub grid: Option<TableGrid>,
    pub frame: Option<f64>,
}

/// A blank table's cells: `rows` × `columns` empty ones.
pub fn blank_cells(rows: usize, columns: usize) -> Cells {
    Cells {
        cells: vec![vec![String::new(); columns]; rows],
        aligns: vec!["left".to_owned(); columns],
        fitted: 0,
        problem: None,
    }
}

/// What is said of a sheet with more rows than a table holds: the reader
/// stops past them, so the rule's count would be the reader's, not the
/// sheet's (the web's `SHEET_CUT`).
pub const SHEET_CUT: &str = "Sayfada 10.000'den çok satır var; bir tablo en çok 10.000 satırdır. Sayfayı bölün ya da fazla satırları silin.";

/// A file sheet's rows as cells (the empty rows and columns after the last
/// with words dropped); refused as [`SHEET_CUT`] when the reader stopped
/// past a table's rows (Tablo ekle, Tabloyu güncelle).
pub fn sheet_cells(sheet: &TableSheet, header: bool) -> Cells {
    let mut cells = rules::from_rows(sheet.rows.clone(), header);
    if cells.problem.is_some() && sheet.cut {
        cells.problem = Some(SHEET_CUT.to_owned());
    }
    cells
}

/// A new table of `cells` with its top left corner at `p`, as `look` says,
/// from `source`; every row and column as the core's sizes fit its words in
/// the project's typeface, a blank table's columns `BLANK_COLUMN` heights
/// wide (Tablo ekle).
pub fn new_table(
    doc: &Document,
    cells: &Cells,
    look: &Look,
    p: Vec2,
    source: Option<TableSource>,
) -> EntityGeometry {
    let n = cells.cells.len();
    let m = cells.cells.first().map_or(0, Vec::len);
    let aligns: Option<Vec<TableAlign>> = cells.aligns.iter().any(|a| a != "left").then(|| {
        cells
            .aligns
            .iter()
            .map(|a| TableAlign::from_name(a).unwrap_or_default())
            .collect()
    });
    let mut geometry = EntityGeometry::Table {
        p: kentos_contracts::Vec2 { x: p.x, y: p.y },
        rotation: 0.0,
        height: look.height,
        rows: vec![look.height; n],
        columns: vec![look.height; m],
        cells: cells.cells.clone(),
        merges: Vec::new(),
        aligns,
        header: look.header,
        grid: look.grid,
        frame: look.frame,
        face: look.face.clone(),
        source,
    };
    fit(doc, &mut geometry);
    if cells.cells.iter().all(|r| r.iter().all(String::is_empty))
        && let EntityGeometry::Table { columns, .. } = &mut geometry
    {
        *columns = vec![BLANK_COLUMN * look.height; m];
    }
    geometry
}

/// A table geometry's rows and columns fitted to its words (Yazıya sığdır).
pub fn fit(doc: &Document, geometry: &mut EntityGeometry) {
    let font = drawing_font(doc.settings().drawing_font);
    let base = EntityBase {
        id: 0,
        layer_id: String::new(),
        color: None,
        attrs: Default::default(),
        label: None,
        symbol: None,
        line_weight: None,
        label_pins: Vec::new(),
    };
    let Some(sizes) = rules::sizes(&shape(&entity_of(geometry, base)), font) else {
        return;
    };
    if let EntityGeometry::Table { rows, columns, .. } = geometry {
        *rows = sizes.rows;
        *columns = sizes.columns;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet(n: usize, cut: bool) -> TableSheet {
        TableSheet {
            name: None,
            rows: vec![vec!["a".to_owned()]; n],
            cut,
        }
    }

    /// The reader stops one row past a table's: a sheet it cut is refused in
    /// the sheet's words, not by the reader's count (the web's `sheetCells`).
    #[test]
    fn a_cut_sheet_is_refused_in_its_own_words() {
        assert_eq!(
            sheet_cells(&sheet(10_001, true), false).problem.as_deref(),
            Some(SHEET_CUT)
        );
        assert!(
            sheet_cells(&sheet(10_001, false), false)
                .problem
                .is_some_and(|p| p.contains("10001"))
        );
        let cells = sheet_cells(&sheet(3, true), true);
        assert_eq!(cells.problem, None);
        assert_eq!(cells.cells, vec![vec!["a".to_owned()]; 3]);
    }
}
