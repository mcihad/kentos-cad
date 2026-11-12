//! The time filter of export (docs/adr/0646 §2): `file.export.gpkg`, `.shp`, `.csv`, `.fgb`, `.parquet`, `.gml` and
//! `.xyz` write, of each written layer with time settings (or of every written layer when the question names the
//! fields), only the objects the question shows, by `cad.time.query`'s rules (`TimeFilter::passes`). A layer without
//! them is written whole, as the map shows it at every time; the day known applies to the layers that record their
//! versions. The objects left out are taken off a copy of the drawing; the open drawing does not change.

use std::borrow::Cow;

use kentos_commands::{LayerNodeType, QueryTime};
use kentos_domain::{Document, Slot};

use crate::checks::Stop;
use crate::codes;
use crate::time_filter::{TimeFilter, moment, window};

/// A refusal as the exporters give it: code, message, path.
type Refused = (String, String, String);

fn refused(stop: Stop) -> Refused {
    let e = stop.into_error();
    (e.code, e.message, e.path.unwrap_or_default())
}

/// The leaf layers an export of `layers` writes (every layer with objects when none).
fn written_layers(doc: &Document, layers: &[String]) -> Vec<String> {
    let tree = doc.layers();
    if layers.is_empty() {
        return tree
            .leaves()
            .into_iter()
            .filter(|n| doc.by_layer(&n.id).next().is_some())
            .map(|n| n.id.clone())
            .collect();
    }
    let mut out = Vec::new();
    for id in layers {
        match tree.get(id).map(|n| n.kind) {
            Some(LayerNodeType::Layer) => out.push(id.clone()),
            Some(LayerNodeType::Group) => {
                out.extend(tree.leaves_of(id).iter().map(|n| n.id.clone()))
            }
            None => {}
        }
    }
    out
}

/// The drawing an export of `layers` writes with `time`, and how many objects it leaves out; why not.
pub fn filtered<'a>(
    doc: &'a Document,
    layers: &[String],
    time: Option<&QueryTime>,
) -> Result<(Cow<'a, Document>, u32), Refused> {
    let Some(q) = time else {
        return Ok((Cow::Borrowed(doc), 0));
    };
    let asked =
        window(q.at.as_deref(), q.from.as_deref(), q.to.as_deref(), "time.").map_err(refused)?;
    if let Some(text) = &q.known_at {
        moment(text, "time.knownAt").map_err(refused)?;
    }
    if asked.is_none() && q.known_at.is_none() {
        return Err((
            codes::INVALID_PERIOD.into(),
            "Zaman sorusunda bir an (at), bir aralık (from, to) ya da bilinen gün (knownAt) verin."
                .into(),
            "time".into(),
        ));
    }
    let timed: Vec<(String, kentos_commands::LayerTime)> = written_layers(doc, layers)
        .into_iter()
        .filter_map(|id| {
            let fields = q
                .fields
                .clone()
                .or_else(|| doc.layers().get(&id).and_then(|n| n.time.clone()))?;
            Some((id, fields))
        })
        .collect();
    if timed.is_empty() {
        return Err((
            codes::NOT_TEMPORAL.into(),
            "Yazılan katmanların hiçbirinin zaman ayarı yok; zaman alanlarını (time.fields) verin."
                .into(),
            "time".into(),
        ));
    }
    if q.known_at.is_some() && !timed.iter().any(|(_, f)| f.recorded()) {
        return Err((
            codes::NOT_RECORDED.into(),
            "Yazılan katmanların hiçbiri sürümlerinin kayıt zamanını tutmuyor; bilinen günle yazmak için kayıt başlangıcı alanını (time.fields.recordedStart) verin.".into(),
            "time.knownAt".into(),
        ));
    }
    let mut out: Vec<Slot> = Vec::new();
    for (id, fields) in &timed {
        let layer_q = QueryTime {
            fields: Some(fields.clone()),
            known_at: if fields.recorded() {
                q.known_at.clone()
            } else {
                None
            },
            ..q.clone()
        };
        // A layer the question asks nothing of (only a day known, and it records no versions) is written whole.
        if asked.is_none() && layer_q.known_at.is_none() {
            continue;
        }
        let filter = TimeFilter::read(&layer_q, None, id).map_err(refused)?;
        for (uid, e) in doc.by_layer_with_uids(id) {
            if !filter.passes(&e.base().attrs)
                && let Some(slot) = doc.slot_of(uid)
            {
                out.push(slot);
            }
        }
    }
    if out.is_empty() {
        return Ok((Cow::Borrowed(doc), 0));
    }
    let mut copy = doc.clone();
    let left = copy.remove(&out) as u32;
    Ok((Cow::Owned(copy), left))
}

/// The warning an export with a time gives for the objects it left out.
pub fn left_out_warning(left: u32) -> Option<kentos_commands::CommandWarning> {
    (left > 0).then(|| kentos_commands::CommandWarning {
        code: codes::TIME_LEFT_OUT.into(),
        message: format!("Zamanın dışında kalan {left} nesne yazılmadı."),
        path: Some("time".into()),
    })
}
