//! What a round trip did to the blocks, the tables and the project's relations (`file.exchange.check`, plan/map-cad.md
//! MAP-17, docs/adr/2380). The reader reads into a copy with no block definitions and no relations, so what comes back is
//! its own:
//!
//! - a block read back is changed when its definition is another (its name, its base, its objects' kinds and places
//!   within the tolerance, in order) or the insert's scale, rotation or mirror is another;
//! - a table read back is changed when its cells, its rows' and columns' sizes (within the tolerance) or its merged
//!   cells are others;
//! - a relation of the project between layers written is lost when the reader made none of the same layers and field,
//!   or made one whose keys do not reach objects of the parent layer by their persistent ids.

use kentos_commands::{InsertEntity, TableEntity, Vec2};
use kentos_domain::{Document, Uuid};

use crate::exchange_check::{places, to_original};

fn dist(a: Vec2, b: Vec2) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// Whether the insert read back (`b` in `back`) shows another block than the original (`a` in `doc`).
pub(crate) fn block_changed(doc: &Document, a: &InsertEntity, back: &Document, b: &InsertEntity, tolerance: f64) -> bool {
    let (Some(da), Some(db)) = (doc.block(a.block), back.block(b.block)) else { return true };
    if da.name != db.name || dist(da.base, db.base) > tolerance {
        return true;
    }
    let near = |x: f64, y: f64| (x - y).abs() <= 1e-9 * x.abs().max(1.0);
    let turn = (a.rotation - b.rotation).rem_euclid(std::f64::consts::TAU);
    if !near(a.scale, b.scale)
        || !near(a.y_scale.unwrap_or(a.scale), b.y_scale.unwrap_or(b.scale))
        || turn.min(std::f64::consts::TAU - turn) > 1e-9
        || a.mirror != b.mirror
    {
        return true;
    }
    da.entities.len() != db.entities.len()
        || da.entities.iter().zip(&db.entities).any(|(o, n)| {
            o.kind() != n.kind() || places(n).into_iter().any(|p| !(to_original(o, p) <= tolerance))
        })
}

/// What of the table read back is another: `cells`, `sizes` or `merges`; none when it is the same.
pub(crate) fn table_changed(a: &TableEntity, b: &TableEntity, tolerance: f64) -> Option<&'static str> {
    if a.cells != b.cells {
        return Some("cells");
    }
    let same = |x: &[f64], y: &[f64]| x.len() == y.len() && x.iter().zip(y).all(|(p, q)| (p - q).abs() <= tolerance);
    if !same(&a.rows, &b.rows) || !same(&a.columns, &b.columns) {
        return Some("sizes");
    }
    (a.merges != b.merges).then_some("merges")
}

/// The names of the project's relations between layers written (`written`) that the trip lost, in the project's order.
pub(crate) fn relations_lost(doc: &Document, back: &Document, written: impl Fn(&str) -> bool) -> Vec<String> {
    let mut lost = Vec::new();
    for r in &doc.settings().relations {
        if !(written(&r.parent) && written(&r.child)) {
            continue;
        }
        let made = back.settings().relations.iter().any(|m| m.parent == r.parent && m.child == r.child && m.field == r.field);
        // Every child read back with a key reaches an object of the parent layer by its persistent id.
        let reaches = || {
            back.entities().filter(|e| e.base().layer_id == r.child).all(|e| match e.base().attrs.get(&r.field).map(|v| v.trim()) {
                None | Some("") => true,
                Some(key) => Uuid::parse_str(key)
                    .ok()
                    .and_then(|u| back.slot_of(u))
                    .and_then(|s| back.get(s))
                    .is_some_and(|p| p.base().layer_id == r.parent),
            })
        };
        if !(made && reaches()) {
            lost.push(r.name.clone());
        }
    }
    lost
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::{
        CallOp, CommandResult, DocumentSnapshotV2, Entity, ExchangeFormat, ExchangeMatrix, ExchangeMatrixed,
        ExchangeState,
    };
    use serde_json::Value;

    const DRAWING: &str = include_str!("../../../../fixtures/exchange/v1/devir-blok.json");

    /// The drawing with a change made to its JSON.
    fn variant(change: impl Fn(&mut Value)) -> Document {
        let mut v: Value = serde_json::from_str(DRAWING).expect("the drawing");
        change(&mut v);
        let snapshot: DocumentSnapshotV2 = serde_json::from_value(v).expect("a snapshot");
        Document::from_snapshot_v2(snapshot).expect("opens")
    }

    fn drawing() -> Document {
        variant(|_| {})
    }

    fn insert(doc: &Document) -> InsertEntity {
        doc.entities().find_map(|e| if let Entity::Insert(i) = e { Some(i.clone()) } else { None }).expect("an insert")
    }

    fn table(doc: &Document) -> TableEntity {
        doc.entities().find_map(|e| if let Entity::Table(t) = e { Some(t.clone()) } else { None }).expect("a table")
    }

    /// The entities' index of the first of a kind in the JSON.
    fn index_of(v: &Value, kind: &str) -> usize {
        v["entities"].as_array().expect("entities").iter().position(|e| e["kind"] == kind).expect(kind)
    }

    fn matrix(doc: &Document) -> ExchangeMatrixed {
        let input = ExchangeMatrix { formats: Vec::new(), layers: Vec::new(), tolerance: None };
        match crate::exchange_matrix::run(doc, &input, CallOp::Execute) {
            CommandResult::Completed { output, .. } => serde_json::from_value(output).expect("the answer"),
            other => panic!("{other:?}"),
        }
    }

    /// DXF keeps the block (its definition read back the same) and the table (its cells and sizes), every other format
    /// writes neither; only GeoPackage keeps the relation (its related tables read back, the keys reaching the parcel).
    #[test]
    fn the_blocks_tables_and_relations_rows_by_format() {
        let m = matrix(&drawing());
        let cell = |aspect: &str, f: ExchangeFormat| {
            let row = m.rows.iter().find(|r| r.aspect == aspect).expect(aspect);
            row.cells.iter().find(|c| c.format == f).cloned().expect("a cell")
        };
        for f in ExchangeFormat::ALL {
            let (dxf, gpkg) = (f == ExchangeFormat::Dxf, f == ExchangeFormat::Gpkg);
            let kept_if = |yes: bool| if yes { ExchangeState::Kept } else { ExchangeState::Lost };
            assert_eq!(cell("blocks", f).state, kept_if(dxf), "{f:?}");
            assert_eq!(cell("tables", f).state, kept_if(dxf), "{f:?}");
            let relations = cell("relations", f);
            assert_eq!(relations.state, kept_if(gpkg), "{f:?}");
            if !gpkg {
                assert_eq!(relations.detail.as_deref(), Some("Parselin malikleri"), "{f:?}");
            }
        }
        let rows: Vec<&str> = m.rows.iter().map(|r| r.aspect.as_str()).collect();
        assert_eq!(rows, kentos_commands::EXCHANGE_ASPECTS);
    }

    /// The rules seen to bite: a block whose circle or rotation is another, a table with another cell or column, a
    /// relation not made or whose key reaches no parcel.
    #[test]
    fn a_changed_block_table_or_relation_is_seen() {
        let doc = drawing();
        let (a, t) = (insert(&doc), table(&doc));
        assert!(!block_changed(&doc, &a, &doc, &a, 0.001));
        let wider = variant(|v| v["blocks"][0]["entities"][0]["r"] = 0.8.into());
        assert!(block_changed(&doc, &a, &wider, &insert(&wider), 0.001));
        let turned = variant(|v| {
            let i = index_of(v, "insert");
            v["entities"][i]["rotation"] = 0.1.into();
        });
        assert!(block_changed(&doc, &a, &turned, &insert(&turned), 0.001));
        assert_eq!(table_changed(&t, &t, 0.001), None);
        let other = variant(|v| {
            let i = index_of(v, "table");
            v["entities"][i]["cells"][1][1] = "8".into();
        });
        assert_eq!(table_changed(&t, &table(&other), 0.001), Some("cells"));
        let wide = variant(|v| {
            let i = index_of(v, "table");
            v["entities"][i]["columns"][0] = 10.5.into();
        });
        assert_eq!(table_changed(&t, &table(&wide), 0.001), Some("sizes"));
        let all = |_: &str| true;
        assert!(relations_lost(&doc, &doc, all).is_empty());
        let none = variant(|v| v["settings"]["relations"] = Value::Array(Vec::new()));
        assert_eq!(relations_lost(&doc, &none, all), ["Parselin malikleri"]);
        // The key holds a spot height's id: it reaches no object of the parcel layer.
        let astray = variant(|v| {
            let point = v["uids"][0].clone();
            let i = v["entities"].as_array().expect("entities").iter().position(|e| e["layerId"] == "malik").expect("malik");
            v["entities"][i]["attrs"]["parsel_id"] = point;
        });
        assert_eq!(relations_lost(&doc, &astray, all), ["Parselin malikleri"]);
        // A relation between layers not written is not counted.
        assert!(relations_lost(&doc, &none, |id: &str| id != "malik").is_empty());
    }
}
