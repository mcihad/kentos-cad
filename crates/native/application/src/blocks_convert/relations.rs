//! Conservative conversion boundaries. Root-only graphs cannot silently turn into block pieces.
use super::{entity, refuse};
use crate::checks::Stop;
use kentos_commands::{Entity, LineType, RepeatDimensionMap, Workspace};
use kentos_domain::Document;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub(super) fn appearance(
    doc: &Document,
    all: &HashSet<String>,
    layer: &str,
    stopped: &mut impl FnMut() -> bool,
) -> Result<(), Stop> {
    if doc
        .settings()
        .project_type()
        .is_some_and(|w| w != Workspace::Cad)
    {
        return Err(refuse(
            "Tekrarları bloğa dönüştürme 2B CAD çizimleri içindir. CBS nesneleri ve kayıtları dönüştürülmedi.",
            "source",
        ));
    }
    let node = doc
        .layers()
        .get(layer)
        .ok_or_else(|| refuse("Kaynak katmanı bulunamadı.", "source"))?;
    crate::checks::layer(doc, layer)?;
    if !doc.layers().is_visible(layer) || doc.layers().is_frozen(layer) {
        return Err(refuse(
            "Dönüşüm için görünür ve dondurulmamış bir katman seçin.",
            "source",
        ));
    }
    // A root row ceases to exist for every member except the retained insert identity.
    // Ordinary CAD layer display metadata is harmless; record/schema/feed/join behaviour is not.
    let layer_json = record(node, "layers")?;
    for (key, value) in &layer_json {
        if matches!(
            key.as_str(),
            "id" | "name"
                | "type"
                | "visible"
                | "locked"
                | "expanded"
                | "style"
                | "children"
                | "snap"
                | "plot"
                | "frozen"
                | "sourceCrs"
        ) {
            continue;
        }
        if !empty(value) {
            return Err(refuse(
                format!(
                    "Katmanın “{key}” kayıt veya davranış bilgisi var. Bu bilgi blok içindeki alt nesneye taşınamadığından dönüşüm yapılmadı."
                ),
                format!("layers.{layer}.{key}"),
            ));
        }
    }
    if node.style.label.is_some() || node.style.labels.is_some() || node.style.renderer.is_some() {
        return Err(refuse(
            "Katman nesneye göre etiket veya kural tabanlı görünüş üretiyor. Aynı görünüş alt nesneler için doğrulanmadan dönüşüm yapılmaz.",
            format!("layers.{layer}.style"),
        ));
    }
    let mut ordered: Vec<_> = all.iter().collect();
    ordered.sort();
    for uid in ordered {
        if stopped() {
            return Err(Stop::Cancelled);
        }
        let (e, _) = entity(doc, uid)?;
        let b = e.base();
        if b.layer_id != layer {
            return Err(refuse(
                "Bu dönüşüm aynı CAD katmanındaki bileşenler içindir. Farklı katmanlar birleştirilmedi.",
                "groups",
            ));
        }
        if !b.attrs.is_empty()
            || b.label.is_some()
            || !b.label_pins.is_empty()
            || !b.attachments.is_empty()
            || b.symbol.is_some()
        {
            return Err(refuse(
                "Kaynak veya aday nesne öznitelik, etiket, ek dosya ya da sembol taşıyor. Bu kök nesne verileri alt nesneye sessizce taşınmadı; veri eşlemesi gereklidir.",
                format!("objects.{uid}"),
            ));
        }
        if b.look.line_pattern.is_some()
            || node.style.line_pattern.is_some()
            || b.look.line_type.unwrap_or(node.style.line_type) != LineType::Continuous
        {
            return Err(refuse(
                "Kesikli/özel çizgi tipinin yönü ve başlangıç fazı blok dönüşümünde henüz korunmuyor. Görünüş değiştirilmedi.",
                format!("objects.{uid}.lineType"),
            ));
        }
    }
    Ok(())
}
fn empty(v: &Value) -> bool {
    v.is_null()
        || v.as_array().is_some_and(Vec::is_empty)
        || v.as_object().is_some_and(serde_json::Map::is_empty)
}

/// Prepared once: checking 1000 groups must not rescan/format all 50000 layer UUIDs per group.
pub(super) struct PaintOrder(HashMap<String, (usize, Option<i32>)>);
impl PaintOrder {
    pub(super) fn new(doc: &Document, layer: &str) -> Self {
        Self(
            doc.by_layer_with_uids(layer)
                .enumerate()
                .map(|(i, (u, e))| (u.to_string(), (i, e.base().look.order)))
                .collect(),
        )
    }
    pub(super) fn check(&self, members: &[String], group: usize) -> Result<(), Stop> {
        let first = members
            .first()
            .and_then(|u| self.0.get(u))
            .ok_or_else(|| refuse("Aday nesneler katmanda yok.", "groups"))?;
        if members.iter().enumerate().any(|(i, u)| {
            self.0
                .get(u)
                .is_none_or(|(rank, order)| *rank != first.0 + i || *order != first.1)
        }) {
            return Err(refuse(
                "Adayın parçaları arasında başka nesneler var veya parçaların çizim sırası farklı. Tek blokla kapatma/üst üste çizme sırası değişebileceğinden dönüşüm yapılmadı.",
                format!("groups[{group}]"),
            ));
        }
        Ok(())
    }
}
fn record(
    value: &impl serde::Serialize,
    path: &str,
) -> Result<serde_json::Map<String, Value>, Stop> {
    match serde_json::to_value(value) {
        Ok(Value::Object(map)) => Ok(map),
        _ => Err(refuse(
            "İlişki kaydı okunamadı; çizim değiştirilmedi.",
            path,
        )),
    }
}

// Explicit roots may appear as a UUID, an expression fragment or a JSON map key. The
// historic request journal and the snapshot's identity list are not live dependencies.
struct Scan<'a, F> {
    selected: &'a HashSet<String>,
    layer: &'a str,
    work: usize,
    stopped: &'a mut F,
}
impl<F: FnMut() -> bool> Scan<'_, F> {
    fn text(&mut self, text: &str, path: &str, layer_refs: bool) -> Result<(), Stop> {
        let bytes = text.as_bytes();
        for chunk in bytes.chunks(1024) {
            self.work = self.work.saturating_add(chunk.len());
            if (self.stopped)() {
                return Err(Stop::Cancelled);
            }
            if self.work > 32_000_000 {
                return Err(refuse(
                    "İlişki denetimi iş sınırına ulaştı; hiçbir nesne değiştirilmedi. Daha küçük bir çizimde tekrar deneyin.",
                    path,
                ));
            }
        }
        let linked = bytes.windows(36).any(|s| {
            s[8] == b'-'
                && s[13] == b'-'
                && s[18] == b'-'
                && s[23] == b'-'
                && std::str::from_utf8(s)
                    .is_ok_and(|s| self.selected.contains(&s.to_ascii_lowercase()))
        });
        if linked || (layer_refs && text == self.layer) {
            return Err(refuse(
                "Bu kayıt dönüştürülecek kök nesneye veya katman kayıtlarına bağlı. Bu ilişki blok alt nesnesine taşınamadığından işlem yapılmadı; bağ silinmedi.",
                path,
            ));
        }
        Ok(())
    }
    fn value(&mut self, v: &Value, path: &str, layer_refs: bool) -> Result<(), Stop> {
        self.work = self.work.saturating_add(1);
        if self.work > 32_000_000 {
            return Err(refuse(
                "İlişki denetimi iş sınırına ulaştı; hiçbir nesne değiştirilmedi.",
                path,
            ));
        }
        if (self.stopped)() {
            return Err(Stop::Cancelled);
        }
        match v {
            Value::String(s) => self.text(s, path, layer_refs)?,
            Value::Array(xs) => {
                for (i, x) in xs.iter().enumerate() {
                    self.value(x, &format!("{path}[{i}]"), layer_refs)?;
                }
            }
            Value::Object(map) => {
                for (k, x) in map {
                    self.text(k, path, false)?;
                    self.value(x, &format!("{path}.{k}"), layer_refs)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

pub(super) fn dependencies(
    doc: &Document,
    selected: &HashSet<String>,
    layer: &str,
    dimensions: &[RepeatDimensionMap],
    stopped: &mut impl FnMut() -> bool,
) -> Result<(), Stop> {
    let mut scan = Scan {
        selected,
        layer,
        work: 0,
        stopped,
    };
    let (head, _) = doc.to_snapshot_v2_shared();
    let mut fields = record(&head, "document")?;
    for field in ["uids", "requests", "entities"] {
        fields.remove(field);
    }
    let value = Value::Object(fields);
    // UUID scan covers constraints/arrays/dimension sets/modules/options/groups and root links in
    // settings, layers, definitions and sheets; no narrow copy_links count is treated as a gate.
    scan.value(&value, "document", false)?;
    // GIS/form/feed/join references can name a layer rather than an individual feature UUID.
    scan.value(&value["settings"], "settings", true)?;
    for node in doc.layers().nodes() {
        scan_layer(&mut scan, node)?;
    }
    let mut mapped_points: HashMap<&str, Vec<&str>> = HashMap::new();
    for d in dimensions {
        mapped_points.entry(&d.uid).or_default().push(&d.point);
    }
    for (uid, e) in doc.entities_with_uids() {
        let text = uid.to_string();
        if selected.contains(&text) {
            continue;
        }
        let path = format!("objects.{uid}");
        // A whole-drawing computed table can depend on kinds/counts without naming ANY root.
        // The first slice does not claim to preserve such implicit query semantics.
        if matches!(e, Entity::Table(_)) {
            return Err(refuse(
                "Çizimde tablo var; sorgusunun kök nesne sayısı/türüyle ilişkisi bu dönüşümde henüz denetlenmiyor. Tablo sonucu sessizce değiştirilmedi.",
                path,
            ));
        }
        let mut fields = record(e, &path)?;
        if let Some(points) = mapped_points.get(text.as_str()) {
            let refs = fields
                .get_mut("refs")
                .and_then(Value::as_object_mut)
                .ok_or_else(|| refuse("Taşınacak ölçü bağının kaydı okunamadı.", &path))?;
            for point in points {
                refs.remove(*point);
            }
        }
        // Own layerId is presentation, not a join. Other layer-named fields and all
        // UUIDs remain subject to the complete scan, once per object.
        fields.remove("layerId");
        scan.value(&Value::Object(fields), &path, true)?;
    }
    Ok(())
}
fn scan_layer<F: FnMut() -> bool>(
    scan: &mut Scan<'_, F>,
    node: &kentos_commands::LayerNode,
) -> Result<(), Stop> {
    let mut fields = record(node, "layers")?;
    for field in ["id", "name", "style", "children"] {
        fields.remove(field);
    }
    scan.value(&Value::Object(fields), &format!("layers.{}", node.id), true)?;
    for child in &node.children {
        scan_layer(scan, child)?;
    }
    Ok(())
}
