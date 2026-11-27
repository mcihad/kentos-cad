//! The data layers' rows in an export's scope (docs/adr/9338). A data layer's rows are its source's, not the drawing's
//! objects (docs/adr/9101), so GeoJSON dışa aktar and Paketle take them from the source as the tables do (data_rows.rs:
//! read whole, off the interface's thread, kept for the drawing as it is).
//!
//! - **Seçili:** the rows chosen in the layer's table, as read with their geometry (data_selection.rs).
//! - **Görünen katmanlar:** the visible data layers' rows that pass the layer's filter, as the map draws them.
//! - **Tümü:** every data layer's rows, its filter aside.
//!
//! The rows the drawing holds (taken or new in the edit session) are its objects and come with them. While a source is
//! read the window says so; a source that cannot be read is named with why, and its layer is left out.

use std::sync::Arc;

use kentos_commands::Entity;

use super::dxf_export::Scope;
use crate::app::App;

/// A data layer's rows as an export window holds them.
#[derive(Debug, Clone)]
pub(crate) enum Rows {
    /// Being read.
    Reading,
    /// Read: the source's rows the drawing does not hold, and which of them the layer's filter lets through (none: all).
    Ready { rows: Arc<Vec<Entity>>, shown: Option<Arc<Vec<usize>>> },
    /// The source could not be read: why.
    Failed(String),
}

/// A data layer of the drawing with its rows.
#[derive(Debug, Clone)]
pub(crate) struct SourceLayer {
    pub id: String,
    pub visible: bool,
    pub rows: Rows,
}

/// The data layers' rows for every scope, gathered when the window opens and again when a source's rows come.
#[derive(Debug, Clone, Default)]
pub(crate) struct SourceScopes {
    /// Every data layer of the drawing, in the tree's order.
    pub layers: Vec<SourceLayer>,
    /// The rows chosen in a data layer's table: the layer, and its rows read (none: still read).
    pub chosen: Option<(String, Option<Vec<Entity>>)>,
}

impl SourceScopes {
    /// The scope's source rows by layer, borrowed: each data layer in it that has rows.
    pub(crate) fn rows(&self, scope: Scope) -> Vec<&Entity> {
        match scope {
            Scope::Selection => self.chosen.iter().flat_map(|(_, rows)| rows.iter().flatten()).collect(),
            Scope::Visible => self
                .layers
                .iter()
                .filter(|l| l.visible)
                .flat_map(|l| match &l.rows {
                    Rows::Ready { rows, shown: Some(shown) } => shown.iter().filter_map(|&i| rows.get(i)).collect(),
                    Rows::Ready { rows, shown: None } => rows.iter().collect(),
                    _ => Vec::new(),
                })
                .collect(),
            Scope::All => self
                .layers
                .iter()
                .flat_map(|l| match &l.rows {
                    Rows::Ready { rows, .. } => rows.iter().collect(),
                    _ => Vec::new(),
                })
                .collect(),
        }
    }

    /// The data layers of the scope still read.
    pub(crate) fn reading(&self, scope: Scope) -> Vec<&str> {
        match scope {
            Scope::Selection => self
                .chosen
                .iter()
                .filter(|(_, rows)| rows.is_none())
                .map(|(id, _)| id.as_str())
                .collect(),
            _ => self
                .in_scope(scope)
                .filter(|l| matches!(l.rows, Rows::Reading))
                .map(|l| l.id.as_str())
                .collect(),
        }
    }

    /// The data layers of the scope whose source could not be read, with why.
    pub(crate) fn failed(&self, scope: Scope) -> Vec<(&str, &str)> {
        if scope == Scope::Selection {
            return Vec::new();
        }
        self.in_scope(scope)
            .filter_map(|l| match &l.rows {
                Rows::Failed(why) => Some((l.id.as_str(), why.as_str())),
                _ => None,
            })
            .collect()
    }

    /// Whether the layer's rows are a source's (a data layer of the drawing).
    pub(crate) fn is_source(&self, layer: &str) -> bool {
        self.layers.iter().any(|l| l.id == layer)
    }

    /// How many of a data layer's rows its filter keeps, of how many (none: no filter, or not read).
    pub(crate) fn filtered(&self, layer: &str) -> Option<(usize, usize)> {
        self.layers.iter().find(|l| l.id == layer).and_then(|l| match &l.rows {
            Rows::Ready { rows, shown: Some(shown) } => Some((shown.len(), rows.len())),
            _ => None,
        })
    }

    fn in_scope(&self, scope: Scope) -> impl Iterator<Item = &SourceLayer> {
        self.layers.iter().filter(move |l| scope == Scope::All || l.visible)
    }
}

impl App {
    /// A scope's objects for an export window: the drawing's and the data layers' rows (docs/adr/9338). Görünen
    /// katmanlar is what the map draws: the visible layers' objects their filters let through.
    pub(crate) fn export_objects<'a>(&'a self, sources: &'a SourceScopes, scope: Scope) -> Vec<&'a Entity> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let mut out: Vec<&Entity> = match scope {
            Scope::Visible => doc
                .model
                .entities()
                .filter(|e| {
                    doc.model.layers().is_visible(&e.base().layer_id)
                        && self.spatial.filter_shown(kentos_domain::Slot(e.base().id))
                })
                .collect(),
            _ => self.scope_entities(scope),
        };
        out.extend(sources.rows(scope));
        out
    }

    /// The scope's objects by layer, in the tree's order.
    pub(crate) fn export_by_layer<'a>(&'a self, sources: &'a SourceScopes, scope: Scope) -> Vec<(String, Vec<&'a Entity>)> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        kentos_headless::export::by_layer(&doc.model, self.export_objects(sources, scope))
    }

    /// Every data layer's rows as the export windows count them: asked of the sources where they are not kept yet
    /// (read after this message), the layer's filter worked out over the rows read.
    pub(crate) fn source_scopes(&self) -> SourceScopes {
        let Some(doc) = &self.document else {
            return SourceScopes::default();
        };
        let tree = doc.model.layers();
        let unit = self.reading_unit();
        let layers = tree
            .leaves()
            .into_iter()
            .filter(|l| l.data.is_some())
            .map(|node| {
                let rows = match self.data_rows_of(&node.id) {
                    None | Some(crate::data_rows::Rows::Reading) => Rows::Reading,
                    Some(crate::data_rows::Rows::Failed(why)) => Rows::Failed(why),
                    Some(crate::data_rows::Rows::Ready(read)) => {
                        let held = doc.model.data_hidden(&node.id);
                        let rows: Vec<Entity> =
                            read.iter().filter(|(k, _)| !held.contains(k)).map(|(_, e)| e.clone()).collect();
                        let shown = node.filter.as_ref().and_then(|f| {
                            let compiled = kentos_native_application::layer_filter::compile_filter(f, unit).ok()?;
                            let list: Vec<&Entity> = rows.iter().collect();
                            let shapes: Vec<_> = if compiled.reads_geometry() {
                                rows.iter().map(kentos_native_application::geometry::shape).collect()
                            } else {
                                Vec::new()
                            };
                            let name = |id: &str| tree.get(id).map_or_else(String::new, |l| l.name.clone());
                            let met = compiled.condition(&list, &name, |i| shapes.get(i));
                            Some(Arc::new(met.iter().enumerate().filter(|(_, m)| **m).map(|(i, _)| i).collect()))
                        });
                        Rows::Ready { rows: Arc::new(rows), shown }
                    }
                };
                SourceLayer { id: node.id.clone(), visible: tree.is_visible(&node.id), rows }
            })
            .collect();
        SourceScopes { layers, chosen: self.chosen_source_rows() }
    }

    /// A source's rows came, or the table's chosen rows were read: an open export window gathers its scopes again.
    pub(crate) fn export_sources_read(&mut self) {
        // The file exports' plans made again on the drawing with the rows (source_copy.rs, docs/adr/9344).
        use super::Window as W;
        match &self.exchange {
            Some(W::FgbExport(_)) => {
                let plan = self.fgb_plan();
                if let Some(W::FgbExport(s)) = &mut self.exchange {
                    s.plan = plan;
                }
            }
            Some(W::GmlExport(s)) => {
                let plan = self.gml_plan(s.schema);
                if let Some(W::GmlExport(s)) = &mut self.exchange {
                    s.plan = plan;
                }
            }
            Some(W::CsvExport(s)) => {
                let plan = self.csv_plan(s.geometry, s.separator);
                if let Some(W::CsvExport(s)) = &mut self.exchange {
                    s.plan = plan;
                }
            }
            Some(W::ShpExport(s)) => {
                let plan = self.shp_plan(s.encoding);
                if let Some(W::ShpExport(s)) = &mut self.exchange {
                    s.plan = plan;
                }
            }
            Some(W::ParquetExport(_)) => self.parquet_replan(),
            _ => {}
        }
        if !matches!(
            self.exchange,
            Some(super::Window::GeoJsonExport(_) | super::Window::Package(_) | super::Window::Wgs84Export(_))
        ) {
            return;
        }
        let sources = self.source_scopes();
        match &mut self.exchange {
            Some(super::Window::GeoJsonExport(s)) => s.sources = sources,
            Some(super::Window::Package(s)) => s.sources = sources,
            Some(super::Window::Wgs84Export(s)) => s.sources = sources,
            _ => {}
        }
    }
}
