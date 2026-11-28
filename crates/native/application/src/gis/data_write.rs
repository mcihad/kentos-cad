//! A data layer's changes written to its table, and its rows read to be taken into the drawing (docs/adr/9103 §2, §4,
//! plan/veri.md VY-05).
//!
//! Writing ([`write`]): the layer's objects become rows of its table and are written in one transaction of its source,
//! all or nothing:
//! - the geometry as the table holds it: an area's outline for a line column, a closed line made an area for an area
//!   column; arcs made chords where the column keeps no curve; moved into the table's system; Z and M as the column has
//!   them; a single part made a multi of one for a multi column (`kentos_data::fit`);
//! - the values as the fields hold them (`kentos_data::coerce`): a text read as its field's kind, an empty one NULL;
//! - a new row gets the values it has, the others take the table's defaults; a changed row the values and the geometry
//!   that changed;
//! - the key, a field the source makes itself and a name that is no field of the table are never written.
//!
//! Reading ([`rows_to_take`]): the rows of some keys with every field and their whole geometry (never simplified), as
//! objects of the layer in the project's system.

use std::collections::BTreeSet;

use kentos_commands::{
    DataLayer, DataText, EditGrouping, Entity, LayerNode, ProjectSettings, TextCell,
};
use kentos_data::{
    Cancel, Capabilities, Change, Condition, Coord, Curve, DataError, Family, Feature, Geometry,
    GeometryField, NewFeature, Request, Schema, Value,
};
use kentos_domain::{DataChanges, DataWritten, Document, Uuid};

use crate::geopackage::{attribute_text, geometry_of, object_of};
use crate::sources::TableSystem;

/// Rows asked of the source at a time while taking.
const CHUNK: usize = 2048;
/// The widest angle an arc's chord spans when it is made chords, degrees.
const CHORD_DEGREES: f64 = 5.0;

/// What a layer writes: its table and its changes, worked out from the drawing.
#[derive(Clone, Debug)]
pub struct LayerWrites {
    pub layer: String,
    /// The layer's name, for the words.
    pub name: String,
    pub data: DataLayer,
    /// The layer's fields, in its order: what a taken row's object holds (a conflict is found against them).
    pub fields: Vec<String>,
    pub changes: DataChanges,
}

/// Why a write did not happen: nothing of the layers written together was written.
#[derive(Clone, Debug, PartialEq)]
pub enum WriteFailure {
    /// The table could not be opened or read.
    Unreachable(String),
    /// The table, or a row made of an object, refused: the words, and the object when one was at fault.
    Refused(String, Option<Uuid>),
    /// Rows taken were changed or deleted by someone else since (docs/adr/9104 §3): the words, the taken objects and
    /// each row's clash field by field (docs/adr/9148).
    Conflict(String, Vec<Uuid>, Vec<crate::data_clash::RowClash>),
    /// The commit's answer was lost (docs/adr/9126): the source may have written it all or nothing. The words, and the
    /// receipt's id when one went in with the rows: the source is asked by it before anything is written again.
    Uncertain(String, Option<String>),
}

impl WriteFailure {
    pub fn message(&self) -> &str {
        match self {
            WriteFailure::Unreachable(m)
            | WriteFailure::Refused(m, _)
            | WriteFailure::Conflict(m, _, _)
            | WriteFailure::Uncertain(m, _) => m,
        }
    }
}

/// A write's receipt to go in with its rows (docs/adr/9126): its lasting id, given before the write starts, and who
/// writes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipting {
    pub id: String,
    pub by: Option<String>,
}

/// Where a data layer's source keeps write receipts: a database table's schema (`schema.table`, “public” without
/// one); a file has one place ("").
pub fn receipt_scope(data: &DataLayer) -> String {
    if data.driver == "postgis" {
        data.table
            .rsplit_once('.')
            .map_or_else(|| "public".to_owned(), |(schema, _)| schema.to_owned())
    } else {
        String::new()
    }
}

/// Whether the layer's source keeps write receipts (set up by the owner).
pub fn receipts_ready(data: &DataLayer) -> Result<bool, String> {
    let source = kentos_data::registry()
        .source(&data.address)
        .map_err(|e| e.to_string())?;
    match source.receipts(&receipt_scope(data)) {
        Ok(ready) => Ok(ready),
        Err(DataError::Unsupported(_)) => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

/// Makes the layer's source's write receipts table (docs/adr/9126): a table of its own, the data's tables untouched.
pub fn set_up_receipts(data: &DataLayer) -> Result<(), String> {
    let source = kentos_data::registry()
        .source(&data.address)
        .map_err(|e| e.to_string())?;
    source
        .set_up_receipts(&receipt_scope(data))
        .map_err(|e| e.to_string())
}

/// Each table's new rows' keys by the objects' ids, as a committed write's receipt says them.
pub type ReceiptKeys = std::collections::BTreeMap<String, Vec<(Uuid, i64)>>;

/// What a committed write's receipt says it wrote ([`ReceiptKeys`]). None when no write of `id` committed (its receipt
/// is not there).
pub fn receipt_keys(data: &DataLayer, id: &str) -> Result<Option<ReceiptKeys>, String> {
    let source = kentos_data::registry()
        .source(&data.address)
        .map_err(|e| e.to_string())?;
    let Some(receipt) = source
        .receipt(&receipt_scope(data), id)
        .map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };
    let value: serde_json::Value = serde_json::from_str(&receipt.written)
        .map_err(|e| format!("işlem kaydı okunamadı: {e}"))?;
    let mut out = std::collections::BTreeMap::new();
    for (table, w) in value.as_object().into_iter().flatten() {
        let keys = w["added"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|pair| {
                let uid = Uuid::parse_str(pair.get(0)?.as_str()?).ok()?;
                Some((uid, pair.get(1)?.as_i64()?))
            })
            .collect();
        out.insert(table.clone(), keys);
    }
    Ok(Some(out))
}

/// What `layer` would write now, read off the drawing; none for a layer that is not a data layer being edited.
pub fn writes_of(doc: &Document, layer: &str) -> Option<LayerWrites> {
    if !doc.editing(layer) {
        return None;
    }
    let node = doc.layers().get(layer)?;
    Some(LayerWrites {
        layer: layer.to_owned(),
        name: node.name.clone(),
        data: node.data.clone()?,
        fields: node.fields.iter().map(|f| f.name.clone()).collect(),
        changes: doc.data_changes(layer),
    })
}

/// The layers a session acts on together (docs/adr/9104 §2): each named layer alone, or with every data layer of its
/// source, or every data layer; those being edited when `editing`, else those that may be (not read-only). In the tree's
/// order.
pub fn grouped(
    doc: &Document,
    named: &[String],
    grouping: EditGrouping,
    editing: bool,
) -> Vec<String> {
    let layers = doc.layers();
    let fits = |n: &LayerNode| {
        n.data.as_ref().is_some_and(|d| {
            if editing {
                doc.editing(&n.id)
            } else {
                !d.read_only
            }
        })
    };
    let addresses: BTreeSet<&str> = named
        .iter()
        .filter_map(|id| layers.get(id)?.data.as_ref().map(|d| d.address.as_str()))
        .collect();
    layers
        .leaves()
        .into_iter()
        .filter(|n| match grouping {
            EditGrouping::Layer => named.contains(&n.id),
            EditGrouping::Source => {
                named.contains(&n.id)
                    || (fits(n)
                        && n.data
                            .as_ref()
                            .is_some_and(|d| addresses.contains(d.address.as_str())))
            }
            EditGrouping::All => named.contains(&n.id) || fits(n),
        })
        .map(|n| n.id.clone())
        .collect()
}

/// What `layers` write, in the groups written together: each alone, or a source's in one transaction (docs/adr/9104
/// §2). A layer with nothing to write is left out.
pub fn groups_of(
    doc: &Document,
    layers: &[String],
    grouping: EditGrouping,
) -> Vec<Vec<LayerWrites>> {
    let writes: Vec<LayerWrites> = layers
        .iter()
        .filter_map(|l| writes_of(doc, l))
        .filter(|w| !w.changes.is_empty())
        .collect();
    if grouping == EditGrouping::Layer {
        return writes.into_iter().map(|w| vec![w]).collect();
    }
    let mut out: Vec<Vec<LayerWrites>> = Vec::new();
    for w in writes {
        match out.iter_mut().find(|g| g[0].data.address == w.data.address) {
            Some(g) => g.push(w),
            None => out.push(vec![w]),
        }
    }
    out
}

/// What a write of `w` that gave `keys` to its new rows wrote, for the drawing to take out ([`Document::data_saved`]).
pub fn written(w: &LayerWrites, keys: &[i64]) -> DataWritten {
    DataWritten {
        added: w
            .changes
            .added
            .iter()
            .zip(keys)
            .map(|((uid, e), key)| (*uid, e.clone(), *key))
            .collect(),
        changed: w
            .changes
            .changed
            .iter()
            .map(|c| (c.uid, c.now.clone()))
            .collect(),
        deleted: w.changes.deleted.clone(),
    }
}

/// Writes `w`'s changes to its table in one transaction of its source, checked against what others wrote since its
/// rows were taken; the keys the table gave the new rows, in order. Nothing is written when it fails.
pub fn write(w: &LayerWrites, settings: &ProjectSettings) -> Result<Vec<i64>, WriteFailure> {
    write_group(std::slice::from_ref(w), settings, false)
        .map(|mut keys| keys.pop().unwrap_or_default())
}

/// One layer's rows as its table takes them, worked out before the transaction.
/// A text of a layer of texts as its row (docs/adr/9114 §4): a point at its place, its words, height, angle, alignment
/// and width factor written into their columns over its other values. The text is what is drawn, so its own parts win
/// over the same columns' values. Any other object, or a layer's that is not of texts, as it is.
fn as_row<'e>(text: Option<&DataText>, e: &'e Entity) -> std::borrow::Cow<'e, Entity> {
    let (Some(text), Entity::Text(t)) = (text, e) else {
        return std::borrow::Cow::Borrowed(e);
    };
    let mut attrs = t.base.attrs.clone();
    for (column, cell) in text.cells(&crate::data_text::parts_of(t)) {
        let value = match cell {
            TextCell::Text(s) => s,
            TextCell::Number(n) => n.to_string(),
        };
        attrs.insert(column, value);
    }
    let point = serde_json::json!({
        "kind": "point",
        "id": t.base.id,
        "layerId": t.base.layer_id,
        "attrs": attrs,
        "p": { "x": t.p.x, "y": t.p.y },
    });
    serde_json::from_value(point).map_or(std::borrow::Cow::Borrowed(e), std::borrow::Cow::Owned)
}

struct Planned<'a> {
    w: &'a LayerWrites,
    schema: Schema,
    system: Option<TableSystem>,
    inserts: Vec<NewFeature>,
    updates: Vec<Change>,
    /// The table takes new rows from this connection (docs/adr/9162).
    may_add: bool,
}

/// Writes the layers of one source together in one transaction (docs/adr/9104 §2): first each taken row is read again
/// inside it, held, and checked against the row as it was taken; a row changed or deleted by someone else since stops the
/// write (unless `force`), then every layer's new, changed and deleted rows. All or nothing; the new rows' keys, a list a
/// layer.
pub fn write_group(
    group: &[LayerWrites],
    settings: &ProjectSettings,
    force: bool,
) -> Result<Vec<Vec<i64>>, WriteFailure> {
    write_group_receipted(group, settings, force, None)
}

/// The group's writes worked out against its source: each layer's schema, system, new rows (their keys where the
/// table does not make them) and changed rows; refused for a row the table cannot take.
fn plan_group<'a>(
    group: &'a [LayerWrites],
    settings: &ProjectSettings,
) -> Result<(std::sync::Arc<dyn kentos_data::Source>, Vec<Planned<'a>>), WriteFailure> {
    let unreachable = |w: &LayerWrites, e: DataError| {
        WriteFailure::Unreachable(format!("“{}” tablosu açılamadı: {e}", w.data.table))
    };
    let Some(first) = group.first() else {
        return Err(WriteFailure::Unreachable("yazılacak katman yok".to_owned()));
    };
    let source = kentos_data::registry()
        .source(&first.data.address)
        .map_err(|e| unreachable(first, e))?;
    let mut planned = Vec::with_capacity(group.len());
    for w in group {
        let schema = source
            .schema(&w.data.table)
            .map_err(|e| unreachable(w, e))?;
        let caps = source
            .capabilities(&w.data.table)
            .map_err(|e| unreachable(w, e))?;
        allowed(w, &caps)?;
        let srid = w.data.geometry.as_ref().and_then(|g| g.srid);
        let system = TableSystem::of(settings, srid).map_err(WriteFailure::Unreachable)?;
        let (inserts, updates) = {
            let row = Rows {
                schema: &schema,
                curves: caps.curves,
                system: system.as_ref(),
                text: w.data.text.as_ref(),
            };
            let mut inserts = Vec::with_capacity(w.changes.added.len());
            for (uid, e) in &w.changes.added {
                inserts.push(row.new_row(e).map_err(|m| refused(w, &m, Some(*uid)))?);
            }
            // A key the table does not make: the next ones after its greatest. Another writer taking them meanwhile
            // makes the table refuse the write (its primary key), and nothing is written.
            if !schema.key_made && !inserts.is_empty() {
                let last = greatest_key(source.as_ref(), &w.data.table, &schema.key)
                    .map_err(|e| unreachable(w, e))?;
                for (i, row) in inserts.iter_mut().enumerate() {
                    row.fid = Some(last + 1 + i as i64);
                }
            }
            let mut updates = Vec::with_capacity(w.changes.changed.len());
            for c in &w.changes.changed {
                if let Some(change) = row
                    .change(c.key, &c.now, &c.row)
                    .map_err(|m| refused(w, &m, Some(c.uid)))?
                {
                    updates.push(change);
                }
            }
            (inserts, updates)
        };
        planned.push(Planned {
            w,
            schema,
            system,
            inserts,
            updates,
            may_add: caps.insert,
        });
    }
    Ok((source, planned))
}

/// The clashes a save of `group` would meet now (docs/adr/9193): the taken rows others changed or deleted since they
/// were taken, read as the save reads them, in a transaction that writes nothing and is let go.
pub fn clashes_of_group(
    group: &[LayerWrites],
    settings: &ProjectSettings,
) -> Result<Vec<crate::data_clash::RowClash>, WriteFailure> {
    let Some(first) = group.first() else {
        return Ok(Vec::new());
    };
    let (source, planned) = plan_group(group, settings)?;
    let mut tx = source.begin().map_err(|e| {
        WriteFailure::Unreachable(format!("“{}” tablosu açılamadı: {e}", first.data.table))
    })?;
    let found = conflicts(
        tx.as_mut(),
        &planned,
        crate::data_text::default_height(settings),
    );
    let _ = tx.rollback();
    match found {
        Ok(None) => Ok(Vec::new()),
        Ok(Some(WriteFailure::Conflict(_, _, rows))) => Ok(rows),
        Ok(Some(other)) => Err(other),
        Err((w, e)) => Err(refused(w, &e.to_string(), None)),
    }
}

/// [`write_group`] with a receipt (docs/adr/9126): where the source keeps write receipts, the receipt goes in with the
/// rows, in the same transaction, saying the keys the new rows got. A commit whose answer is lost is
/// [`WriteFailure::Uncertain`], never a refusal: with the receipt's id when it went in, so the source can be asked.
pub fn write_group_receipted(
    group: &[LayerWrites],
    settings: &ProjectSettings,
    force: bool,
    receipting: Option<&Receipting>,
) -> Result<Vec<Vec<i64>>, WriteFailure> {
    let Some(first) = group.first() else {
        return Ok(Vec::new());
    };
    let unreachable = |w: &LayerWrites, e: DataError| {
        WriteFailure::Unreachable(format!("“{}” tablosu açılamadı: {e}", w.data.table))
    };
    let (source, planned) = plan_group(group, settings)?;
    // A receipt where the source keeps them; a source that keeps none writes as before.
    let scope = receipt_scope(&first.data);
    let receipting = match receipting {
        Some(r) => match source.receipts(&scope) {
            Ok(true) => Some(r),
            Ok(false) | Err(DataError::Unsupported(_)) => None,
            Err(e) => return Err(unreachable(first, e)),
        },
        None => None,
    };
    let mut tx = source.begin().map_err(|e| unreachable(first, e))?;
    if !force {
        match conflicts(
            tx.as_mut(),
            &planned,
            crate::data_text::default_height(settings),
        ) {
            Ok(None) => {}
            Ok(Some(failure)) => {
                let _ = tx.rollback();
                return Err(failure);
            }
            Err((w, e)) => {
                let _ = tx.rollback();
                return Err(refused(w, &e.to_string(), None));
            }
        }
    }
    let mut keys = Vec::with_capacity(planned.len());
    for p in &planned {
        let table = p.w.data.table.as_str();
        let wrote = (|| -> Result<Vec<i64>, DataError> {
            let keys = if p.inserts.is_empty() {
                Vec::new()
            } else {
                tx.insert(table, &p.inserts)?
            };
            if !p.updates.is_empty() {
                tx.update(table, &p.updates)?;
            }
            if !p.w.changes.deleted.is_empty() {
                tx.delete(table, &p.w.changes.deleted)?;
            }
            Ok(keys)
        })();
        match wrote {
            Ok(k) => keys.push(k),
            Err(e) => {
                // Nothing of the group is written; a rollback that fails too leaves the source to drop it.
                let _ = tx.rollback();
                return Err(refused(p.w, &e.to_string(), None));
            }
        }
    }
    if let Some(r) = receipting {
        let written: serde_json::Map<String, serde_json::Value> = planned
            .iter()
            .zip(&keys)
            .map(|(p, k)| {
                let added: Vec<serde_json::Value> =
                    p.w.changes
                        .added
                        .iter()
                        .zip(k)
                        .map(|((uid, _), key)| serde_json::json!([uid.to_string(), key]))
                        .collect();
                (
                    p.w.data.table.clone(),
                    serde_json::json!({
                        "added": added,
                        "changed": p.updates.len(),
                        "deleted": p.w.changes.deleted.len(),
                    }),
                )
            })
            .collect();
        let receipt = kentos_data::Receipt {
            id: r.id.clone(),
            by: r.by.clone(),
            written: serde_json::Value::Object(written).to_string(),
        };
        if let Err(e) = tx.receipt(&scope, &receipt) {
            let _ = tx.rollback();
            return Err(refused(
                first,
                &format!("işlem kaydı yazılamadı: {e}"),
                None,
            ));
        }
    }
    tx.commit().map_err(|e| match e {
        DataError::Uncertain(_) => WriteFailure::Uncertain(
            format!("“{}” tablosuna {e}", first.data.table),
            receipting.map(|r| r.id.clone()),
        ),
        e => refused(first, &e.to_string(), None),
    })?;
    Ok(keys)
}

/// The taken rows others changed or deleted since they were taken (docs/adr/9104 §3): each read again inside the
/// transaction and compared with the row as it came, as the take made its object.
fn conflicts<'a>(
    tx: &mut dyn kentos_data::Transaction,
    planned: &[Planned<'a>],
    height: f64,
) -> Result<Option<WriteFailure>, (&'a LayerWrites, DataError)> {
    let mut clashes: Vec<crate::data_clash::RowClash> = Vec::new();
    for p in planned {
        let originals = &p.w.changes.originals;
        if originals.is_empty() {
            continue;
        }
        let keys: Vec<i64> = originals.keys().copied().collect();
        let rows = tx.rows(&p.w.data.table, &keys).map_err(|e| (p.w, e))?;
        let by_key: std::collections::HashMap<i64, &Feature> =
            rows.iter().map(|f| (f.fid, f)).collect();
        // The layer's fields out of the schema's order, as the take read them.
        let at: Vec<Option<usize>> = p.w.fields.iter().map(|n| p.schema.index(n)).collect();
        let text = p.w.data.text.as_ref();
        // What is here of each taken row: its object now; none for a row deleted here.
        let here: std::collections::HashMap<i64, &Entity> =
            p.w.changes
                .changed
                .iter()
                .map(|c| (c.key, &*c.now))
                .collect();
        for (key, (uid, row)) in originals {
            // The table's row now as an object, compared as rows (a text's parts as its columns).
            let theirs = match by_key.get(key) {
                None => None,
                Some(f) => {
                    let values: Vec<Value> = at
                        .iter()
                        .map(|i| {
                            i.and_then(|i| f.values.get(i).cloned())
                                .unwrap_or(Value::Null)
                        })
                        .collect();
                    let read = Feature {
                        fid: f.fid,
                        geometry: f.geometry.clone(),
                        values,
                    };
                    match object_from(
                        &read,
                        &p.w.fields,
                        &p.w.layer,
                        p.system.as_ref(),
                        text,
                        height,
                    ) {
                        Ok(e) => Some(e),
                        // A row that no longer reads as an object of the layer: a clash with nothing to show.
                        Err(_) => {
                            clashes.push(crate::data_clash::RowClash {
                                layer: p.w.layer.clone(),
                                uid: *uid,
                                key: *key,
                                first: (**row).clone(),
                                mine: here.get(key).map(|e| (*e).clone()),
                                theirs: None,
                                fields: Vec::new(),
                                geometry: false,
                                may_add: p.may_add,
                            });
                            continue;
                        }
                    }
                }
            };
            let first = as_row(text, row).into_owned();
            let mine = here.get(key).map(|e| as_row(text, e).into_owned());
            let theirs_row = theirs.as_ref().map(|e| as_row(text, e).into_owned());
            if let Some((fields, geometry)) =
                crate::data_clash::clash_of(&first, mine.as_ref(), theirs_row.as_ref())
            {
                clashes.push(crate::data_clash::RowClash {
                    layer: p.w.layer.clone(),
                    uid: *uid,
                    key: *key,
                    first: (**row).clone(),
                    mine: here.get(key).map(|e| (*e).clone()),
                    theirs,
                    fields,
                    geometry,
                    may_add: p.may_add,
                });
            }
        }
    }
    if clashes.is_empty() {
        return Ok(None);
    }
    let uids = clashes.iter().map(|c| c.uid).collect();
    Ok(Some(WriteFailure::Conflict(
        crate::data_clash::clash_words(&clashes),
        uids,
        clashes,
    )))
}

/// The table's greatest key; 0 for an empty table.
fn greatest_key(
    source: &dyn kentos_data::Source,
    table: &str,
    key: &str,
) -> Result<i64, DataError> {
    let request = Request {
        columns: Some(Vec::new()),
        geometry: false,
        order: vec![kentos_data::Order {
            field: key.to_owned(),
            descending: true,
        }],
        limit: Some(1),
        chunk: 1,
        ..Request::default()
    };
    let mut last = 0;
    source.features(table, &request, &Cancel::new(), &mut |part: Vec<
        Feature,
    >| {
        if let Some(f) = part.first() {
            last = f.fid;
        }
        false
    })?;
    Ok(last)
}

fn refused(w: &LayerWrites, why: &str, uid: Option<Uuid>) -> WriteFailure {
    WriteFailure::Refused(
        format!(
            "“{}” katmanının değişiklikleri tablosuna yazılamadı: {why}",
            w.name
        ),
        uid,
    )
}

/// The table's rights against what the layer writes.
fn allowed(w: &LayerWrites, caps: &Capabilities) -> Result<(), WriteFailure> {
    let text = w.data.text.as_ref();
    let rows = |c: &kentos_domain::DataChange| {
        (
            as_row(text, &c.now).into_owned(),
            as_row(text, &c.row).into_owned(),
        )
    };
    let geometry_changed = w
        .changes
        .changed
        .iter()
        .map(rows)
        .any(|(now, row)| geometry_of(&now) != geometry_of(&row));
    let values_changed = w
        .changes
        .changed
        .iter()
        .map(rows)
        .any(|(now, row)| now.base().attrs != row.base().attrs);
    let missing = [
        (
            !w.changes.added.is_empty() && !caps.insert,
            "yeni satır eklemeye",
        ),
        (
            geometry_changed && !caps.update_geometry,
            "geometri değiştirmeye",
        ),
        (values_changed && !caps.update_values, "değer değiştirmeye"),
        (
            !w.changes.deleted.is_empty() && !caps.delete,
            "satır silmeye",
        ),
    ];
    match missing.iter().find(|(m, _)| *m) {
        Some((_, what)) => Err(WriteFailure::Refused(
            format!(
                "“{}” tablosu {what} izin vermiyor (kaynağın ya da hesabın yetkisi yok); hiçbir şey yazılmadı.",
                w.data.table
            ),
            None,
        )),
        None => Ok(()),
    }
}

/// How an object becomes a row of the table (an edit session's save, an export's write, docs/adr/9106).
pub(crate) struct Rows<'a> {
    schema: &'a Schema,
    /// The source keeps arcs.
    curves: bool,
    system: Option<&'a TableSystem>,
    /// A layer of texts' mapping: its texts written as rows (docs/adr/9114 §4).
    text: Option<&'a DataText>,
}

impl<'a> Rows<'a> {
    /// Rows of the table `schema` describes; `curves` when it keeps arcs; `system` the move into its system.
    pub(crate) fn new(
        schema: &'a Schema,
        curves: bool,
        system: Option<&'a TableSystem>,
    ) -> Rows<'a> {
        Rows {
            schema,
            curves,
            system,
            text: None,
        }
    }
}

impl Rows<'_> {
    pub(crate) fn new_row(&self, e: &Entity) -> Result<NewFeature, String> {
        let e = &*as_row(self.text, e);
        let attrs = &e.base().attrs;
        let mut values = Vec::new();
        for field in self.writable() {
            match attrs.get(&field.name).filter(|t| !t.is_empty()) {
                Some(text) => values.push((field.name.clone(), value_of(field, Some(text))?)),
                // A field that takes no NULL and has no default is said here, with the object at fault.
                None if !field.nullable && field.default.is_none() => {
                    value_of(field, None)?;
                }
                None => {}
            }
        }
        Ok(NewFeature {
            fid: None,
            geometry: self.shape(e)?,
            values,
        })
    }

    /// A taken row's change; none when what the table holds did not change (a value of no field, say).
    fn change(&self, key: i64, now: &Entity, row: &Entity) -> Result<Option<Change>, String> {
        let (now, row) = (as_row(self.text, now), as_row(self.text, row));
        let (now, row) = (&*now, &*row);
        let (a, b) = (&now.base().attrs, &row.base().attrs);
        let names: BTreeSet<&String> = a
            .keys()
            .chain(b.keys())
            .filter(|n| a.get(*n) != b.get(*n))
            .collect();
        let mut values = Vec::new();
        for field in self.writable().filter(|f| names.contains(&f.name)) {
            let text = a.get(&field.name).filter(|t| !t.is_empty());
            values.push((field.name.clone(), value_of(field, text)?));
        }
        let geometry = if geometry_of(now) == geometry_of(row) {
            None
        } else {
            Some(self.shape(now)?)
        };
        Ok(
            (geometry.is_some() || !values.is_empty()).then_some(Change {
                fid: key,
                geometry,
                values,
            }),
        )
    }

    /// The fields a row's values are written to.
    fn writable(&self) -> impl Iterator<Item = &kentos_data::Field> {
        self.schema.fields.iter().filter(|f| {
            !f.generated
                && f.name != self.schema.key
                && !matches!(f.kind, kentos_data::FieldKind::Other)
        })
    }

    /// The object's geometry as the table's column holds it; none for a table without geometry.
    fn shape(&self, e: &Entity) -> Result<Option<Geometry>, String> {
        let Some(column) = &self.schema.geometry else {
            return Ok(None);
        };
        let g = match geometry_of(e) {
            Ok(Some((g, _))) => g,
            Ok(None) => return Ok(None),
            Err(word) => return Err(format!("{word} bir tablo satırı olamaz")),
        };
        let g = as_family(g, column.family())?;
        let keeps_curves = self.curves && keeps_curves(&column.type_name);
        let g = if keeps_curves || !g.is_curved() {
            g
        } else {
            chorded(&g)
        };
        let g = match self.system {
            Some(s) => s
                .moved_back(&g)
                .ok_or_else(|| "geometri tablonun koordinat sistemine çevrilemedi".to_owned())?,
            None => g,
        };
        Ok(Some(kentos_data::fit::fit(
            with_dims(g, column),
            &column.type_name,
        )))
    }
}

/// A value of `field` from a drawing's attribute text; none: NULL.
fn value_of(field: &kentos_data::Field, text: Option<&String>) -> Result<Value, String> {
    let v = text.map_or(Value::Null, |t| Value::Text(t.clone()));
    kentos_data::coerce::coerce(field, &v).map_err(|e| e.to_string())
}

/// Whether a column of this type keeps arcs.
fn keeps_curves(type_name: &str) -> bool {
    matches!(
        type_name.trim().to_ascii_uppercase().as_str(),
        "GEOMETRY"
            | "CIRCULARSTRING"
            | "COMPOUNDCURVE"
            | "CURVEPOLYGON"
            | "MULTICURVE"
            | "MULTISURFACE"
            | "CURVE"
            | "SURFACE"
    )
}

/// `g` as a geometry of the column's family: an area's outlines for a line column, a closed line an area for an area
/// column. Refused: a point for a line or an area column, a line or an area for a point column, an open line for an
/// area column.
fn as_family(g: Geometry, family: Family) -> Result<Geometry, String> {
    let closed = |c: &Curve| {
        let pts = chords_of(c);
        pts.len() >= 4
            && pts
                .first()
                .zip(pts.last())
                .is_some_and(|(a, b)| a.x == b.x && a.y == b.y)
    };
    match (family, g) {
        (Family::Any, g) => Ok(g),
        (Family::Point, g @ (Geometry::Point(_) | Geometry::MultiPoint(_))) => Ok(g),
        (Family::Line, g @ (Geometry::Curve(_) | Geometry::MultiCurve(_))) => Ok(g),
        (Family::Line, Geometry::Polygon(mut rings)) => Ok(if rings.len() == 1 {
            Geometry::Curve(rings.remove(0))
        } else {
            Geometry::MultiCurve(rings)
        }),
        (Family::Line, Geometry::MultiPolygon(polys)) => {
            Ok(Geometry::MultiCurve(polys.into_iter().flatten().collect()))
        }
        (Family::Area, g @ (Geometry::Polygon(_) | Geometry::MultiPolygon(_))) => Ok(g),
        (Family::Area, Geometry::Curve(c)) if closed(&c) => Ok(Geometry::Polygon(vec![c])),
        (Family::Area, Geometry::MultiCurve(cs)) if cs.iter().all(closed) => Ok(
            Geometry::MultiPolygon(cs.into_iter().map(|c| vec![c]).collect()),
        ),
        (family, g) => Err(format!(
            "{} geometrisi {} tutan bir sütuna yazılmaz",
            g.type_name(),
            match family {
                Family::Point => "nokta",
                Family::Line => "çizgi",
                Family::Area => "alan",
                Family::Any => "her tür",
            }
        )),
    }
}

/// `g` with every position given Z and M as the column has them: a missing one 0, an extra one dropped.
fn with_dims(g: Geometry, column: &GeometryField) -> Geometry {
    let (z, m) = (column.z, column.m);
    let fix = |c: &Coord| Coord {
        x: c.x,
        y: c.y,
        z: z.then(|| c.z.unwrap_or(0.0)),
        m: m.then(|| c.m.unwrap_or(0.0)),
    };
    map_coords(&g, &fix)
}

fn map_coords(g: &Geometry, f: &dyn Fn(&Coord) -> Coord) -> Geometry {
    fn curve(c: &Curve, f: &dyn Fn(&Coord) -> Coord) -> Curve {
        match c {
            Curve::Line(p) => Curve::Line(p.iter().map(f).collect()),
            Curve::Arc(p) => Curve::Arc(p.iter().map(f).collect()),
            Curve::Compound(parts) => Curve::Compound(parts.iter().map(|c| curve(c, f)).collect()),
        }
    }
    match g {
        Geometry::Point(p) => Geometry::Point(p.as_ref().map(f)),
        Geometry::MultiPoint(ps) => Geometry::MultiPoint(ps.iter().map(f).collect()),
        Geometry::Curve(c) => Geometry::Curve(curve(c, f)),
        Geometry::Polygon(r) => Geometry::Polygon(r.iter().map(|c| curve(c, f)).collect()),
        Geometry::MultiCurve(cs) => Geometry::MultiCurve(cs.iter().map(|c| curve(c, f)).collect()),
        Geometry::MultiPolygon(ps) => Geometry::MultiPolygon(
            ps.iter()
                .map(|r| r.iter().map(|c| curve(c, f)).collect())
                .collect(),
        ),
        Geometry::Collection(gs) => {
            Geometry::Collection(gs.iter().map(|g| map_coords(g, f)).collect())
        }
    }
}

/// `g` with its arcs made chords, none spanning more than [`CHORD_DEGREES`].
pub(crate) fn chorded(g: &Geometry) -> Geometry {
    let line = |c: &Curve| Curve::Line(chords_of(c));
    match g {
        Geometry::Curve(c) => Geometry::Curve(line(c)),
        Geometry::Polygon(r) => Geometry::Polygon(r.iter().map(line).collect()),
        Geometry::MultiCurve(cs) => Geometry::MultiCurve(cs.iter().map(line).collect()),
        Geometry::MultiPolygon(ps) => {
            Geometry::MultiPolygon(ps.iter().map(|r| r.iter().map(line).collect()).collect())
        }
        Geometry::Collection(gs) => Geometry::Collection(gs.iter().map(chorded).collect()),
        other => other.clone(),
    }
}

/// A curve's positions with its arcs made chords.
fn chords_of(c: &Curve) -> Vec<Coord> {
    match c {
        Curve::Line(p) => p.clone(),
        Curve::Arc(p) => {
            let mut out: Vec<Coord> = p.first().copied().into_iter().collect();
            let mut i = 0;
            while i + 2 < p.len() {
                arc_chords(p[i], p[i + 1], p[i + 2], &mut out);
                i += 2;
            }
            out
        }
        Curve::Compound(parts) => {
            let mut out: Vec<Coord> = Vec::new();
            for part in parts {
                let pts = chords_of(part);
                let skip = usize::from(
                    out.last()
                        .zip(pts.first())
                        .is_some_and(|(a, b)| a.x == b.x && a.y == b.y),
                );
                out.extend(pts.into_iter().skip(skip));
            }
            out
        }
    }
}

/// The positions after `a` along the arc from `a` through `m` to `b`, `b` last; Z and M go evenly from `a`'s to `b`'s.
fn arc_chords(a: Coord, m: Coord, b: Coord, out: &mut Vec<Coord>) {
    let (ax, ay, mx, my, bx, by) = (a.x, a.y, m.x, m.y, b.x, b.y);
    let d = 2.0 * (ax * (my - by) + mx * (by - ay) + bx * (ay - my));
    if d.abs() < 1e-12 {
        // Its three positions on one line: the chord is the line.
        out.push(b);
        return;
    }
    let (a2, m2, b2) = (ax * ax + ay * ay, mx * mx + my * my, bx * bx + by * by);
    let cx = (a2 * (my - by) + m2 * (by - ay) + b2 * (ay - my)) / d;
    let cy = (a2 * (bx - mx) + m2 * (ax - bx) + b2 * (mx - ax)) / d;
    let r = ((ax - cx).powi(2) + (ay - cy).powi(2)).sqrt();
    let angle = |x: f64, y: f64| (y - cy).atan2(x - cx);
    let (t0, tm, t1) = (angle(ax, ay), angle(mx, my), angle(bx, by));
    let tau = std::f64::consts::TAU;
    // Counter-clockwise when the middle comes before the end going that way.
    let ccw = (tm - t0).rem_euclid(tau) < (t1 - t0).rem_euclid(tau);
    let sweep = if ccw {
        (t1 - t0).rem_euclid(tau)
    } else {
        -((t0 - t1).rem_euclid(tau))
    };
    let n = ((sweep.abs().to_degrees() / CHORD_DEGREES).ceil() as usize).max(2);
    let lerp = |p: Option<f64>, q: Option<f64>, k: f64| match (p, q) {
        (Some(p), Some(q)) => Some(p + (q - p) * k),
        (p, q) => p.or(q),
    };
    for i in 1..n {
        let k = i as f64 / n as f64;
        let t = t0 + sweep * k;
        out.push(Coord {
            x: cx + r * t.cos(),
            y: cy + r * t.sin(),
            z: lerp(a.z, b.z, k),
            m: lerp(a.m, b.m, k),
        });
    }
    out.push(b);
}

/// The rows of `keys` with every field and their whole geometry, as objects of the layer in the project's system,
/// to be taken into the drawing (docs/adr/9103 §2); a key not in the table is left out.
pub fn rows_to_take(
    node: &LayerNode,
    settings: &ProjectSettings,
    keys: &[i64],
    stopped: &dyn Fn() -> bool,
) -> Result<Vec<(i64, Entity)>, String> {
    let Some(data) = &node.data else {
        return Err(format!("“{}” bir veri katmanı değil.", node.name));
    };
    let source = kentos_data::registry()
        .source(&data.address)
        .map_err(|e| format!("“{}” tablosu açılamadı: {e}", data.table))?;
    let srid = data.geometry.as_ref().and_then(|g| g.srid);
    let system = TableSystem::of(settings, srid)?;
    let fields: Vec<String> = node.fields.iter().map(|f| f.name.clone()).collect();
    let height = crate::data_text::default_height(settings);
    let request = Request {
        condition: Some(Condition::Fids(keys.to_vec())),
        columns: Some(fields.clone()),
        geometry: data.geometry.is_some(),
        chunk: CHUNK,
        ..Request::default()
    };
    let mut out = Vec::with_capacity(keys.len());
    let mut failed = None;
    source
        .features(&data.table, &request, &Cancel::new(), &mut |part: Vec<
            Feature,
        >| {
            for f in &part {
                match object_from(
                    f,
                    &fields,
                    &node.id,
                    system.as_ref(),
                    data.text.as_ref(),
                    height,
                ) {
                    Ok(e) => out.push((f.fid, e)),
                    Err(m) => failed = Some(m),
                }
            }
            failed.is_none() && !stopped()
        })
        .map_err(|e| format!("“{}” tablosu okunamadı: {e}", data.table))?;
    match failed {
        Some(m) => Err(m),
        None => Ok(out),
    }
}

/// Every row of a data layer read whole for a check over the whole table (docs/adr/9124): what was read and why some
/// rows were left out.
#[derive(Clone, Debug, Default)]
pub struct AllRows {
    /// Each row's key and its object in the project's system, with every field; rows without geometry left out.
    pub rows: Vec<(i64, Entity)>,
    /// How many rows the source gave.
    pub read: usize,
    /// Rows left out: without geometry, or a geometry the project's system cannot take (said in `why`).
    pub skipped: usize,
    pub why: Vec<String>,
}

/// Every row of `node`'s table but the keys in `leave` (the rows the drawing holds, and those it deletes), with their
/// whole geometry (never simplified for a view), in the project's system: a check over the whole table, whatever part
/// of it the map shows. Stopped when `stopped` says so (an error).
pub fn all_rows(
    node: &LayerNode,
    settings: &ProjectSettings,
    leave: &BTreeSet<i64>,
    stopped: &dyn Fn() -> bool,
) -> Result<AllRows, String> {
    rows_read(node, settings, leave, stopped, false)
}

/// Every row of `node`'s table as [`all_rows`] reads them, a row without geometry kept as a record: a table's rows
/// whatever their geometry, as İstatistik and a join view count them (docs/adr/9134).
pub fn all_records(
    node: &LayerNode,
    settings: &ProjectSettings,
    leave: &BTreeSet<i64>,
    stopped: &dyn Fn() -> bool,
) -> Result<AllRows, String> {
    rows_read(node, settings, leave, stopped, true)
}

/// The table's rows read whole; `records`: one without geometry kept as a record, else left out.
fn rows_read(
    node: &LayerNode,
    settings: &ProjectSettings,
    leave: &BTreeSet<i64>,
    stopped: &dyn Fn() -> bool,
    records: bool,
) -> Result<AllRows, String> {
    let Some(data) = &node.data else {
        return Err(format!("“{}” bir veri katmanı değil.", node.name));
    };
    let source = kentos_data::registry()
        .source(&data.address)
        .map_err(|e| format!("“{}” tablosu açılamadı: {e}", data.table))?;
    let srid = data.geometry.as_ref().and_then(|g| g.srid);
    let system = TableSystem::of(settings, srid)?;
    let fields: Vec<String> = node.fields.iter().map(|f| f.name.clone()).collect();
    let height = crate::data_text::default_height(settings);
    let request = Request {
        columns: Some(fields.clone()),
        geometry: data.geometry.is_some(),
        chunk: CHUNK,
        ..Request::default()
    };
    let mut out = AllRows::default();
    let mut halted = false;
    source
        .features(&data.table, &request, &Cancel::new(), &mut |part: Vec<
            Feature,
        >| {
            for f in &part {
                out.read += 1;
                if leave.contains(&f.fid) {
                    continue;
                }
                if !records && f.shape().is_none_or(|g| g.is_empty()) {
                    out.skipped += 1;
                    continue;
                }
                match object_from(
                    f,
                    &fields,
                    &node.id,
                    system.as_ref(),
                    data.text.as_ref(),
                    height,
                ) {
                    Ok(e) => out.rows.push((f.fid, e)),
                    Err(m) => {
                        out.skipped += 1;
                        if out.why.len() < 3 {
                            out.why.push(m);
                        }
                    }
                }
            }
            halted = stopped();
            !halted
        })
        .map_err(|e| format!("“{}” tablosu okunamadı: {e}", data.table))?;
    if halted {
        return Err("durduruldu".into());
    }
    Ok(out)
}

/// A row as an object of the layer `layer`: its values (`fields` in order) as attributes, its geometry moved into the
/// project's system; a row without geometry a record. A layer of texts' row is the text it says (docs/adr/9114 §4;
/// one without words its point).
fn object_from(
    f: &Feature,
    fields: &[String],
    layer: &str,
    system: Option<&TableSystem>,
    text: Option<&DataText>,
    height: f64,
) -> Result<Entity, String> {
    let attrs: serde_json::Map<String, serde_json::Value> = fields
        .iter()
        .zip(&f.values)
        .filter_map(|(n, v)| attribute_text(v).map(|t| (n.clone(), serde_json::Value::String(t))))
        .collect();
    let shape = match (f.shape(), system) {
        (Some(g), Some(s)) => Some(s.moved(g).ok_or_else(|| {
            format!(
                "{} anahtarlı satırın geometrisi projenin sistemine çevrilemedi.",
                f.fid
            )
        })?),
        (Some(g), None) => Some(g.clone()),
        (None, _) => None,
    };
    if let (Some(text), Some(at)) = (text, shape.as_ref().and_then(crate::data_text::point_of))
        && let Some(e) = crate::data_text::object(text, at, layer, 1, attrs.clone(), height)
    {
        return Ok(e);
    }
    if let Some(g) = shape.filter(|g| !g.is_empty())
        && let Some(e) = object_of(&g, layer, 1, attrs.clone())
    {
        return Ok(e);
    }
    let record = serde_json::json!({ "kind": "record", "id": 1, "layerId": layer, "attrs": attrs });
    serde_json::from_value(record)
        .map_err(|e| format!("{} anahtarlı satır nesneye çevrilemedi: {e}", f.fid))
}
