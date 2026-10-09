//! Turns a features value (selection, visible, all, a layer, explicit ids)
//! into the objects a tool works on (the web's `processing/features.ts`).
//! The selection, the visible area and the drawing's geometry store come
//! from the host, so this stays free of UI and viewport.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};

use kentos_contracts::{Entity, RasterEntity};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::store::Store;
use kentos_style_core::js::collate::compare_tr;

use crate::geometry::record;
use crate::text::tr_lower;
use crate::types::FeatureSet;
use crate::values::{FeaturesValue, Scope};

/// What the dialog and a run read of the application: the drawing, the
/// selection, the visible area and the drawing's geometry store (the web's
/// `FeatureHost`, read side).
pub trait Scene {
    fn doc(&self) -> &Document;
    /// The selection, in the order it was made.
    fn selected(&self) -> Vec<Slot>;
    /// The world box on screen; none without a view (tests, a server).
    fn visible_bounds(&self) -> Option<Bounds>;
    /// The drawing's geometry store as the host keeps it: it answers the
    /// "visible" scope's box test and the dialog's expression previews.
    /// Without one the objects asked about get a store of their own.
    fn store(&self) -> Option<&Store> {
        None
    }
}

/// What a run changes: the drawing, and the selection of tools that select.
pub trait Host: Scene {
    fn doc_mut(&mut self) -> &mut Document;
    /// Replaces the selection.
    fn select(&mut self, ids: &[Slot]);
}

/// The dialog's name of a scope.
pub fn scope_label(scope: &Scope) -> &'static str {
    match scope {
        Scope::Selection => "Seçili nesneler",
        Scope::Visible => "Görünen alandakiler",
        Scope::All => "Tümü (görünen katmanlar)",
        Scope::Layer(_) => "Bir katman",
        Scope::Ids(_) => "Önceki adımın çıktısı",
    }
}

/// An object kind's name (the web's `ENTITY_KIND_LABEL`).
pub fn kind_label(kind: &str) -> &'static str {
    match kind {
        "point" => "Nokta",
        "line" => "Çizgi",
        "polyline" => "Çoklu çizgi",
        "polygon" => "Kapalı alan",
        "circle" => "Daire",
        "arc" => "Yay",
        "ellipse" => "Elips",
        "spline" => "Eğri",
        "xline" => "Yardımcı çizgi",
        "ray" => "Işın",
        "text" => "Yazı",
        "dimension" => "Ölçü",
        "hatch" => "Tarama",
        "insert" => "Blok",
        "leader" => "Kılavuz",
        "table" => "Tablo",
        "image" => "Resim",
        "raster" => "Raster",
        "pointcloud" => "Nokta bulutu",
        _ => "Nesne",
    }
}

fn layer_name<'a>(doc: &'a Document, id: &str) -> &'a str {
    doc.layers().get(id).map_or("?", |l| l.name.as_str())
}

/// Objects in scope, before the kind filter: a layer's filter leaves out
/// what it does not pass (docs/adr/0211 §1), in every scope but a model's own
/// step outputs.
fn in_scope<'d>(value: &FeaturesValue, host: &'d dyn Scene) -> Vec<&'d Entity> {
    let doc = host.doc();
    if let Scope::Ids(ids) = &value.scope {
        return ids.iter().filter_map(|id| doc.get(Slot(*id))).collect();
    }
    let out = kentos_native_application::layer_filter::left_out(doc);
    let mut list = scoped(value, host);
    if !out.is_empty() {
        list.retain(|e| !out.contains(&Slot(e.base().id)));
    }
    list
}

fn scoped<'d>(value: &FeaturesValue, host: &'d dyn Scene) -> Vec<&'d Entity> {
    let doc = host.doc();
    let shown = |e: &Entity| doc.layers().is_visible(&e.base().layer_id);
    match &value.scope {
        Scope::Selection => host
            .selected()
            .into_iter()
            .filter_map(|id| doc.get(id))
            .collect(),
        Scope::Visible => {
            // Construction lines reach everywhere: they are never "in view".
            let kept = |e: &Entity| shown(e) && !matches!(e.kind(), "xline" | "ray");
            let Some(view) = host.visible_bounds() else {
                return doc.entities().filter(|e| kept(e)).collect();
            };
            // Whose box overlaps the view is the geometry store's answer, in the document's order.
            let ids = match host.store() {
                Some(store) => store.in_box(&view),
                None => {
                    let mut store = Store::new();
                    store.put_many(doc.entities().map(record));
                    store.in_box(&view)
                }
            };
            ids.into_iter()
                .filter_map(|id| doc.get(Slot(id as u32)))
                .filter(|e| kept(e))
                .collect()
        }
        Scope::All => doc.entities().filter(|e| shown(e)).collect(),
        Scope::Layer(id) => {
            // A group means every layer under it.
            let mut leaves: HashSet<&str> = doc
                .layers()
                .leaves_of(id)
                .into_iter()
                .map(|l| l.id.as_str())
                .collect();
            leaves.insert(id.as_str());
            doc.entities()
                .filter(|e| leaves.contains(e.base().layer_id.as_str()))
                .collect()
        }
        Scope::Ids(ids) => ids.iter().filter_map(|id| doc.get(Slot(*id))).collect(),
    }
}

/// "12 kapalı alan" / "7 nesne (kapalı alan, çoklu çizgi)".
pub fn describe_count(entities: &[&Entity]) -> String {
    if entities.is_empty() {
        return "uygun nesne yok".into();
    }
    let mut kinds: Vec<&str> = Vec::new();
    for e in entities {
        if !kinds.contains(&e.kind()) {
            kinds.push(e.kind());
        }
    }
    if let [kind] = kinds[..] {
        return format!("{} {}", entities.len(), tr_lower(kind_label(kind)));
    }
    let names: Vec<String> = kinds.iter().map(|k| tr_lower(kind_label(k))).collect();
    format!("{} nesne ({})", entities.len(), names.join(", "))
}

pub fn resolve_features<'d>(
    value: &FeaturesValue,
    kinds: Option<&[String]>,
    host: &'d dyn Scene,
) -> FeatureSet<'d> {
    narrow(value, host, fitting(value, kinds, host))
}

/// Objects in scope that the tool can take, before the user's kind filter.
fn fitting<'d>(
    value: &FeaturesValue,
    kinds: Option<&[String]>,
    host: &'d dyn Scene,
) -> Vec<&'d Entity> {
    let mut list = in_scope(value, host);
    if let Some(kinds) = kinds {
        list.retain(|e| kinds.iter().any(|k| k == e.kind()));
    }
    list
}

fn narrow<'d>(
    value: &FeaturesValue,
    host: &'d dyn Scene,
    candidates: Vec<&'d Entity>,
) -> FeatureSet<'d> {
    let entities: Vec<&Entity> = match &value.kinds {
        Some(only) => candidates
            .into_iter()
            .filter(|e| only.iter().any(|k| k == e.kind()))
            .collect(),
        None => candidates,
    };
    let doc = host.doc();
    if entities.is_empty() {
        let description = format!("{} uygun nesne yok", where_in(&value.scope, doc));
        return FeatureSet {
            entities,
            description,
        };
    }
    let place = match &value.scope {
        Scope::Layer(id) => format!("“{}” katmanında", layer_name(doc, id)),
        scope => tr_lower(scope_label(scope)),
    };
    let description = format!("{}; {place}", describe_count(&entities));
    FeatureSet {
        entities,
        description,
    }
}

/// A features parameter as the dialog shows it before running.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InputSummary {
    pub count: usize,
    pub description: String,
    /// Kinds in scope the tool can take, with counts (the kind filter), before that filter.
    pub by_kind: Vec<(String, usize)>,
    /// Attribute names on the objects, most common first (field pickers, expressions).
    pub fields: Vec<(String, usize)>,
    /// A chosen file's table (docs/adr/0200 §7): the counts are its rows, not objects.
    pub rows: bool,
}

/// A chosen file's table as a field parameter reads it: its rows, and its
/// named columns (each once, in the file's order) with how many rows fill each.
pub fn summarize_file(header: &[String], rows: &[Vec<String>]) -> InputSummary {
    let mut fields: Vec<(String, usize)> = Vec::new();
    for (i, name) in header.iter().enumerate() {
        if name.is_empty() || fields.iter().any(|(n, _)| n == name) {
            continue;
        }
        let filled = rows
            .iter()
            .filter(|r| {
                r.get(i)
                    .is_some_and(|c| !crate::text::js_trim(c).is_empty())
            })
            .count();
        fields.push((name.clone(), filled));
    }
    InputSummary {
        count: rows.len(),
        description: String::new(),
        by_kind: Vec::new(),
        fields,
        rows: true,
    }
}

pub fn summarize_features(
    value: &FeaturesValue,
    kinds: Option<&[String]>,
    host: &dyn Scene,
) -> InputSummary {
    let candidates = fitting(value, kinds, host);
    let mut by_kind: Vec<(String, usize)> = Vec::new();
    for e in &candidates {
        match by_kind.iter_mut().find(|(k, _)| k == e.kind()) {
            Some((_, n)) => *n += 1,
            None => by_kind.push((e.kind().to_owned(), 1)),
        }
    }
    // Stable, as the web's sort is.
    by_kind.sort_by_key(|k| std::cmp::Reverse(k.1));
    let set = narrow(value, host, candidates);
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for e in set.entities.iter().take(20_000) {
        for k in e.base().attrs.keys() {
            *counts.entry(k.as_str()).or_default() += 1;
        }
    }
    let mut counts: BTreeMap<String, usize> = counts
        .into_iter()
        .map(|(name, n)| (name.to_owned(), n))
        .collect();
    // A raster's bands are what an expression names it by (Raster hesaplayıcı, docs/adr/0233 §3).
    let mut rasters: Vec<&RasterEntity> = set
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Raster(x) => Some(x),
            _ => None,
        })
        .collect();
    raster_order(&mut rasters, host.doc());
    for (x, name) in rasters.iter().zip(raster_names(&rasters, host.doc())) {
        for b in 2..=x.raster.bands {
            counts.insert(format!("{name}@{b}"), 1);
        }
        counts.insert(name, 1);
    }
    let mut fields: Vec<(String, usize)> = counts.into_iter().collect();
    fields.sort_by(|a, b| match b.1.cmp(&a.1) {
        Ordering::Equal => compare_tr(&a.0, &b.0),
        other => other,
    });
    InputSummary {
        count: set.entities.len(),
        description: set.description,
        by_kind,
        fields,
        rows: false,
    }
}

/// Rasters in a raster operation's order (docs/adr/0233 §2): the layers
/// from the top of the panel down, on a layer the one drawn later first
/// (the web's `rasterRun`).
pub fn raster_order(list: &mut [&RasterEntity], doc: &Document) {
    if list.len() < 2 {
        return;
    }
    let layers: HashMap<&str, usize> = doc
        .layers()
        .leaves()
        .into_iter()
        .enumerate()
        .map(|(k, l)| (l.id.as_str(), k))
        .collect();
    list.sort_by_key(|x| {
        (
            layers
                .get(x.base.layer_id.as_str())
                .copied()
                .unwrap_or(usize::MAX),
            std::cmp::Reverse(doc.place(Slot(x.base.id)).unwrap_or(0)),
        )
    });
}

/// The names an expression reads rasters in that order by (docs/adr/0233
/// §3): the layer's; a second raster on a layer `Ad (2)`, and so on.
pub fn raster_names(list: &[&RasterEntity], doc: &Document) -> Vec<String> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    list.iter()
        .map(|x| {
            let name = doc
                .layers()
                .get(&x.base.layer_id)
                .map_or_else(|| x.base.layer_id.clone(), |l| l.name.clone());
            let n = seen.entry(name.clone()).or_insert(0);
            *n += 1;
            if *n == 1 {
                name
            } else {
                format!("{name} ({n})")
            }
        })
        .collect()
}

/// Where the tool looked, as a sentence start ("Seçili nesneler arasında uygun nesne yok").
fn where_in(scope: &Scope, doc: &Document) -> String {
    match scope {
        Scope::Selection => "Seçili nesneler arasında".into(),
        Scope::Visible => "Görünen alanda".into(),
        Scope::All => "Görünen katmanlarda".into(),
        Scope::Layer(id) => format!("“{}” katmanında", layer_name(doc, id)),
        Scope::Ids(_) => "Önceki adımın çıktısında".into(),
    }
}
