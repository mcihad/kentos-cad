//! A drawing as a DXF file (docs/adr/0252): the objects by layer in the
//! tree's order and the writer's input as the desktop's DXF dışa aktar
//! builds it — layers with their groups, a dimension without its own text
//! with the value its look writes, the blocks the objects place, a local
//! project's unit, the project's text and dimension styles — so a script, an
//! agent and the window write the same file (`kentos_formats::dxf::write`).

use std::collections::BTreeMap;

use kentos_commands::{AngleUnit, DxfWriteInput, DxfWriteLayer, Entity, LayerNodeType};
use kentos_domain::Document;
use kentos_project::format::Format;

/// Shared wording for the desktop's preview and the native export report.
pub const DXF_SPACE_CHOICES_WARNING: &str = "Mahal ad/kullanım listeleri ve kat numara önekleri DXF'e yazılmaz; yeniden açıldığında öneri olarak gelmez. Bu proje tercihlerini korumak için KCAD olarak kaydedin.";

/// Objects by layer: the tree's order, then layers the tree does not have
/// (their objects go to DXF layer 0), empty ones left out.
pub fn by_layer<'a>(
    doc: &Document,
    objects: impl IntoIterator<Item = &'a Entity>,
) -> Vec<(String, Vec<&'a Entity>)> {
    let mut out: Vec<(String, Vec<&Entity>)> = doc
        .layers()
        .leaves()
        .iter()
        .map(|l| (l.id.clone(), Vec::new()))
        .collect();
    let mut at: BTreeMap<String, usize> = out
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (id.clone(), i))
        .collect();
    for e in objects {
        let id = &e.base().layer_id;
        match at.get(id) {
            Some(&i) => out[i].1.push(e),
            None => {
                at.insert(id.clone(), out.len());
                out.push((id.clone(), vec![e]));
            }
        }
    }
    out.retain(|(_, list)| !list.is_empty());
    out
}

/// A layer as the writer receives it (the groups above it, outermost first).
fn write_layer(doc: &Document, id: &str) -> Option<DxfWriteLayer> {
    let layers = doc.layers();
    let l = layers.get(id)?;
    if l.kind != LayerNodeType::Layer {
        return None;
    }
    let mut path = Vec::new();
    let mut up = layers.parent(id);
    while let Some(group) = up {
        path.insert(0, group.name.clone());
        up = layers.parent(&group.id);
    }
    Some(DxfWriteLayer {
        line_pattern: l.style.line_pattern.clone(),
        plot: (!layers.printed(id)).then_some(false),
        frozen: layers.is_frozen(id).then_some(true),
        id: id.to_owned(),
        name: l.name.clone(),
        path,
        color: l.style.color.clone(),
        visible: layers.is_on(id),
        locked: layers.is_locked(id),
        line_type: l.style.line_type,
        line_weight: l.style.line_weight,
    })
}

/// The writer's input for `chosen` (from [`by_layer`]).
pub fn dxf_input(doc: &Document, chosen: &[(String, Vec<&Entity>)]) -> DxfWriteInput {
    let mut entities: Vec<Entity> = chosen
        .iter()
        .flat_map(|(_, list)| list.iter().map(|e| (*e).clone()))
        .collect();
    // The document's draw order crosses layer boundaries. Grouping is only
    // for the layer chooser, never a reason to repaint one whole layer on top.
    let order: std::collections::HashMap<_, _> = doc.entities().enumerate()
        .map(|(i, e)| (e.base().id, i)).collect();
    entities.sort_by_key(|e| order.get(&e.base().id).copied().unwrap_or(usize::MAX));
    // A dynamic block's inserts as the plain blocks they draw (docs/adr/0319 §5): DXF has no place for KentOS's
    // parameters and actions.
    let (variants, mut notes) =
        kentos_native_application::block_variants::plain_for_export(doc.blocks(), &mut entities);
    let chosen_slots: std::collections::HashSet<_> = entities.iter().map(|e| e.base().id).collect();
    let recipe_count = doc
        .dimension_sets()
        .iter()
        .filter(|set| {
            set.members.iter().any(|member| {
                doc.slot_of(kentos_domain::Uuid::from_bytes(member.uid.0))
                    .is_some_and(|slot| chosen_slots.contains(&slot.0))
            })
        })
        .count();
    if recipe_count > 0 {
        notes.push(kentos_commands::ReportItem {
            severity: Some(kentos_commands::ReportSeverity::Warning), details: Vec::new(),
            what: "İlişkili duvar ölçü grubu".into(),
            count: u32::try_from(recipe_count).unwrap_or(u32::MAX),
            reason: "Ölçülerin mevcut geometrisi yazıldı; duvar ölçü grubunun kaynak tarifi ve otomatik üye ekleme/kaldırma davranışı DXF'e yazılmaz. Grubu korumak için KCAD olarak kaydedin.".into(),
            lines: Vec::new(),
        });
    }
    // Annotative annotations (docs/adr/0323 §5): DXF gets each as its own scale draws it.
    let scaled = entities
        .iter()
        .filter(|e| kentos_commands::annotative_of(e).is_some())
        .count();
    if scaled > 0 {
        notes.push(kentos_commands::ReportItem {
                severity: None,
                details: Vec::new(),
            what: "Ölçekli açıklama".into(),
            count: u32::try_from(scaled).unwrap_or(u32::MAX),
            reason: "Kendi ölçeğindeki hâliyle yazıldı; diğer ölçekleri ve oradaki yerleri DXF'e yazılmadı.".into(),
            lines: Vec::new(),
        });
    }
    // Constraints and parameters (docs/adr/0338 §4): DXF gets the geometry as solved; AutoCAD's constraint objects
    // are not written, so another program does not keep them.
    let constraints = doc.constraints();
    if !constraints.constraints.is_empty() {
        notes.push(kentos_commands::ReportItem {
                severity: None,
                details: Vec::new(),
            what: "Kısıt".into(),
            count: u32::try_from(constraints.constraints.len()).unwrap_or(u32::MAX),
            reason: format!(
                "Geometri çözülmüş hâliyle yazıldı; kısıtlar ve {} parametre DXF'e yazılmadı, başka programda düzenlenebilir kısıt olarak gelmez.",
                constraints.parameters.len()
            ),
            lines: Vec::new(),
        });
    }
    let (layouts, layout_notes) = kentos_native_application::cad_layouts::export(doc);
    notes.extend(layout_notes);
    let settings = doc.settings();
    if settings.arch.as_ref().is_some_and(|a| a.has_space_choices()) {
        notes.push(kentos_commands::ReportItem {
            severity: Some(kentos_commands::ReportSeverity::Warning),
            details: Vec::new(),
            what: "Mahal proje tercihleri".into(),
            count: 1,
            reason: DXF_SPACE_CHOICES_WARNING.into(),
            lines: Vec::new(),
        });
    }
    let format = Format::of(settings);
    // What a dimension without its own text shows (project units): its
    // block draws it, the dimension keeps no text.
    let mut dimension_values = BTreeMap::new();
    for e in &entities {
        if let Entity::Dimension(d) = e
            && let Some(l) = kentos_formats::dxf::dimension_layout(d)
        {
            // A text written over the value shows it where it says `<>` (docs/adr/0321 §3).
            dimension_values.insert(
                d.base.id,
                format.dimension_value(
                    l.prefix,
                    l.unit,
                    l.value,
                    &d.look,
                    d.text.as_deref().filter(|t| !t.is_empty()),
                ),
            );
        }
    }
    // The objects' ids: an associative hatch goes out with its boundary objects' handles (docs/adr/0333 §3).
    let uids = entities
        .iter()
        .filter_map(|e| {
            let id = e.base().id;
            doc.uid(kentos_domain::Slot(id))
                .map(|u| (id, kentos_commands::EntityId(u.into_bytes())))
        })
        .collect();
    DxfWriteInput {
        allow_polyline_loss: false,
        layouts,
        // The project's own detail for its walls (docs/adr/0339 §4); absent: by the scale.
        wall_detail: settings.arch.as_ref().and_then(|a| a.detail),
        uids,
        annotations: Default::default(),
        notes,
        // The project's line types and their scale (docs/adr/0313 §6).
        line_type_scale: settings.line_type_scale,
        line_types: settings.line_types.clone(),
        entities,
        layers: chosen
            .iter()
            .filter_map(|(id, _)| write_layer(doc, id))
            .collect(),
        scale: settings.plot_scale,
        length_decimals: settings.length_decimals,
        grads: settings.angle_unit == AngleUnit::Grad,
        dimension_values,
        // The writer takes those the objects place (docs/adr/0144 §5).
        blocks: doc
            .blocks()
            .iter()
            .map(|b| (**b).clone())
            .chain(variants)
            .collect(),
        // A local project's drawing goes out in its unit (docs/adr/0165 §2).
        unit: (!settings.has_system()).then(|| settings.unit()),
        // The project's styles as STYLE and DIMSTYLE records (docs/adr/0183 §7).
        text_styles: settings.text_styles.clone(),
        dimension_styles: settings.dimension_styles.clone(),
    }
}
