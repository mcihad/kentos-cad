//! Kurum şablonu (docs/adr/9184): a CBS project an institution keeps as its template, read off the interface's
//! thread when the wizard is shown it. A new project takes its layers (data layers bound to their sources as they
//! are), styles and symbols, and the institution's settings: forms, domains, relations, joined views, text, dimension
//! and table styles, line types and hatches, service connections, networks, event links, topology rules and profile,
//! variables, address locators, layer states, view templates, standard documents and approved sheet templates. It
//! takes none of its objects, sheets or work of its own (lineage, review notes, survey adjustments, topology
//! exceptions, file-bound tables' revisions), and keeps the name, system and province the wizard chose.

use std::path::{Path, PathBuf};

use kentos_commands::{DOCUMENT_VERSION_2, DocumentSnapshotV2, Workspace};
use kentos_project::wizard::Draft;

use super::super::content;
use crate::document::Document;

/// A template read: what a new project takes of it, and what the page says it brings.
#[derive(Debug, Clone)]
pub struct Template {
    pub path: PathBuf,
    /// The template without its objects.
    snapshot: Box<DocumentSnapshotV2>,
    layers: usize,
    forms: usize,
    relations: usize,
    rules: usize,
}

impl Template {
    /// Its file's name, as the page says it.
    pub(crate) fn name(&self) -> String {
        file_name(&self.path)
    }

    /// What it brings: “12 katman, 3 form, 2 ilişki, 4 topoloji kuralı”; what it has none of is not said.
    pub(crate) fn words(&self) -> String {
        let mut said = vec![format!("{} katman", self.layers)];
        for (n, what) in [(self.forms, "form"), (self.relations, "ilişki"), (self.rules, "topoloji kuralı")] {
            if n > 0 {
                said.push(format!("{n} {what}"));
            }
        }
        said.join(", ")
    }

    /// The new project made of the template and the wizard's choices: the template's scale when it has one, the
    /// wizard's name, system, province and opening view.
    /// The new project the template makes, as a snapshot: its tables are made before it opens (data_place.rs).
    pub(crate) fn snapshot(&self, d: &Draft) -> Result<DocumentSnapshotV2, String> {
        let t = &self.snapshot.settings;
        let mut o = d.options();
        if t.plot_scale.is_finite() && t.plot_scale > 0.0 {
            o.plot_scale = t.plot_scale;
        }
        let fresh = content::new_project(&o)?;
        let mut s = fresh.settings;
        s.length_decimals = t.length_decimals;
        s.area_decimals = t.area_decimals;
        s.area_unit = t.area_unit;
        s.angle_unit = t.angle_unit;
        s.drawing_font = t.drawing_font.or(s.drawing_font);
        s.line_types = t.line_types.clone();
        s.hatch_patterns = t.hatch_patterns.clone();
        s.line_type_scale = t.line_type_scale;
        s.layer_states = t.layer_states.clone();
        s.text_styles = t.text_styles.clone();
        s.dimension_styles = t.dimension_styles.clone();
        s.leader_styles = t.leader_styles.clone();
        s.table_styles = t.table_styles.clone();
        // The rules and the profile; the template's exceptions were findings of its own objects.
        s.topology = t.topology.clone().map(|mut topology| {
            topology.exceptions.clear();
            topology
        });
        s.annotation = t.annotation.clone().or(s.annotation);
        s.connections = t.connections.clone();
        s.networks = t.networks.clone();
        s.variables = t.variables.clone();
        s.address_locators = t.address_locators.clone();
        s.view_templates = t.view_templates.clone();
        s.forms = t.forms.clone();
        s.domains = t.domains.clone();
        s.relations = t.relations.clone();
        s.join_views = t.join_views.clone();
        s.linear_events = t.linear_events.clone();
        s.profile_files = t.profile_files.clone();
        s.standard_docs = t.standard_docs.clone();
        s.approved_sheet_templates = t.approved_sheet_templates.clone();
        Ok(DocumentSnapshotV2 {
                dimension_sets: Vec::new(),
                casework: Vec::new(),
                constraints: Default::default(),
                format: fresh.format,
                version: DOCUMENT_VERSION_2,
                name: fresh.name,
                settings: s,
                origin: fresh.origin,
                home_view: fresh.home_view,
                layers: self.snapshot.layers.clone(),
                active_layer: self.snapshot.active_layer.clone(),
                entities: Vec::new(),
                uids: Vec::new(),
                styles: self.snapshot.styles.clone(),
                blocks: self.snapshot.blocks.clone(),
                project_id: None,
                migrated_from: None,
                sheets: None,
                requests: Vec::new(),
                arrays: Vec::new(),
            })
    }
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned())
}

/// Reads a template: a `.kcad` CBS project. A CAD project is refused with the reason; so is a file that is no
/// project.
pub(crate) fn read(path: &Path) -> Result<Template, String> {
    let name = file_name(path);
    let doc = Document::read(path).map_err(|e| format!("“{name}” şablon olarak okunamadı: {e}"))?;
    if doc.settings().workspace == Some(Workspace::Cad) {
        return Err(format!(
            "“{name}” bir CAD projesi; kurum şablonu bir CBS projesi olmalı. Kurumun CBS şablon projesini seçin."
        ));
    }
    let layers = doc.model.layers().leaves().len();
    let mut snapshot = doc.model.to_snapshot_v2();
    snapshot.entities.clear();
    snapshot.uids.clear();
    let forms = snapshot.settings.forms.len();
    let relations = snapshot.settings.relations.len();
    let rules = snapshot.settings.topology.as_ref().map_or(0, |t| t.rules.len());
    Ok(Template { path: path.to_path_buf(), snapshot: Box::new(snapshot), layers, forms, relations, rules })
}
