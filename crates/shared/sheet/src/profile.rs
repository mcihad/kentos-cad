//! Work-mode profiles (design §11a). A project's mode (`Workspace`) only
//! chooses what the sheet's ribbon shows; it never changes what the data
//! means, and every item works in every mode. What a mode shows is data
//! (`data/profiles.json`): its tools and groups, their presets, the names an
//! item goes by there (“Görünüm penceresi” in CAD, “Harita” in GIS), the
//! default template and the gallery's order. A mode with no row uses the
//! common profile; a row may be the union of others (`union`). Removing a
//! mode is removing its row: no code names one. (The Hibrit mode's row, the
//! union of CAD and GIS, went with the mode: docs/adr/0165.)
//!
//! A tool whose presets a mode's row does not name takes the common
//! profile's (a shape is a shape in every mode); a row that names them
//! replaces them for that tool (the CAD border's zone marks).
//!
//! What a tool needs comes from the project, not the mode: a coordinate
//! system (`georeferenced`), layers with attributes (`attributeLayers`).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use kentos_contracts::{ProjectType, Workspace};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::error::{Result, SheetError};
use crate::kinds::{ItemKind, MapView, TableSource, default_kind};
use crate::model::Item;
use crate::template::TemplateMeta;
use crate::units::RectUm;

/// What the project offers the sheet's tools.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Capabilities {
    /// The project has a coordinate system.
    #[serde(default)]
    pub georeferenced: bool,
    /// The project has layers with attributes.
    #[serde(default)]
    pub attribute_layers: bool,
    /// The project's plot scale: a new map's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub plot_scale: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ToolDef {
    id: String,
    label: String,
    #[serde(default)]
    item: Option<String>,
    #[serde(default)]
    shortcut: Option<String>,
    #[serde(default)]
    requires: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GroupDef {
    id: String,
    label: String,
    tools: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PresetDef {
    id: String,
    label: String,
    item: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProfileDef {
    id: String,
    label: String,
    #[serde(default)]
    workspaces: Vec<Workspace>,
    #[serde(default)]
    union: Vec<String>,
    #[serde(default)]
    groups: Vec<GroupDef>,
    #[serde(default)]
    names: BTreeMap<String, String>,
    #[serde(default)]
    presets: BTreeMap<String, Vec<PresetDef>>,
    #[serde(default)]
    when_missing: BTreeMap<String, String>,
    #[serde(default)]
    default_template: Option<String>,
    #[serde(default)]
    gallery: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Profiles {
    #[allow(dead_code)]
    schema: String,
    #[allow(dead_code)]
    source: String,
    tools: Vec<ToolDef>,
    profiles: Vec<ProfileDef>,
}

fn data() -> &'static Profiles {
    static P: OnceLock<Profiles> = OnceLock::new();
    // The crate's own file, read by a test; a broken one leaves no profile, never a panic.
    P.get_or_init(|| {
        serde_json::from_str(include_str!("../data/profiles.json")).unwrap_or_default()
    })
}

/// Whether a tool is shown, shown but unusable (with the reason), or not shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum ToolState {
    Enabled,
    Disabled,
    Hidden,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct PresetInfo {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ToolInfo {
    pub id: String,
    /// The name in this mode.
    pub label: String,
    /// The item kind it adds (`type`), if it adds one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub item: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shortcut: Option<String>,
    pub state: ToolState,
    /// Why it is disabled or hidden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reason: Option<String>,
    pub presets: Vec<PresetInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ToolGroup {
    pub id: String,
    pub label: String,
    pub tools: Vec<ToolInfo>,
}

/// A mode's profile with the project's capabilities applied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Profile {
    pub id: String,
    pub label: String,
    pub groups: Vec<ToolGroup>,
    /// An item kind's name in this mode (`map` → “Görünüm penceresi”).
    pub names: BTreeMap<String, String>,
    pub default_template: String,
    /// Template ids in the gallery's order.
    pub gallery: Vec<String>,
}

struct Resolved {
    id: String,
    label: String,
    groups: Vec<GroupDef>,
    names: BTreeMap<String, String>,
    presets: BTreeMap<String, Vec<PresetDef>>,
    when_missing: BTreeMap<String, String>,
    default_template: String,
    gallery: Vec<String>,
    /// The workspaces whose templates are this mode's own.
    workspaces: Vec<Workspace>,
}

fn row(id: &str) -> Option<&'static ProfileDef> {
    data().profiles.iter().find(|p| p.id == id)
}

/// A profile's presets: the common profile's, with each tool the row names replaced by the row's own.
fn presets_of(def: &ProfileDef) -> BTreeMap<String, Vec<PresetDef>> {
    let mut out = row("common")
        .filter(|c| c.id != def.id)
        .map(|c| c.presets.clone())
        .unwrap_or_default();
    out.extend(def.presets.iter().map(|(k, v)| (k.clone(), v.clone())));
    out
}

fn resolve(workspace: Option<Workspace>) -> Resolved {
    let d = data();
    let def = workspace
        .and_then(|w| d.profiles.iter().find(|p| p.workspaces.contains(&w)))
        .or_else(|| row("common"));
    let Some(def) = def else {
        return Resolved {
            id: "common".into(),
            label: "Ortak".into(),
            groups: Vec::new(),
            names: BTreeMap::new(),
            presets: BTreeMap::new(),
            when_missing: BTreeMap::new(),
            default_template: String::new(),
            gallery: Vec::new(),
            workspaces: Vec::new(),
        };
    };
    if def.union.is_empty() {
        return Resolved {
            id: def.id.clone(),
            label: def.label.clone(),
            groups: def.groups.clone(),
            names: def.names.clone(),
            presets: presets_of(def),
            when_missing: def.when_missing.clone(),
            default_template: def.default_template.clone().unwrap_or_default(),
            gallery: def.gallery.clone(),
            workspaces: def.workspaces.clone(),
        };
    }
    // A union: groups and tools of every member in first-seen order, presets joined, a tool hidden only where every member hides it.
    let members: Vec<&ProfileDef> = def.union.iter().filter_map(|m| row(m)).collect();
    let mut groups: Vec<GroupDef> = Vec::new();
    let mut presets: BTreeMap<String, Vec<PresetDef>> = BTreeMap::new();
    let mut gallery: Vec<String> = Vec::new();
    let mut workspaces = def.workspaces.clone();
    for m in &members {
        for g in &m.groups {
            match groups.iter_mut().find(|x| x.id == g.id) {
                Some(x) => {
                    for t in &g.tools {
                        if !x.tools.contains(t) {
                            x.tools.push(t.clone());
                        }
                    }
                }
                None => groups.push(g.clone()),
            }
        }
        for (tool, ps) in &presets_of(m) {
            let list = presets.entry(tool.clone()).or_default();
            for p in ps {
                if !list.iter().any(|x| x.id == p.id) {
                    list.push(p.clone());
                }
            }
        }
        for t in &m.gallery {
            if !gallery.contains(t) {
                gallery.push(t.clone());
            }
        }
        workspaces.extend(m.workspaces.iter().copied());
    }
    let mut when_missing = BTreeMap::new();
    if let Some(first) = members.first() {
        for (tool, how) in &first.when_missing {
            if how == "hide"
                && members
                    .iter()
                    .all(|m| m.when_missing.get(tool).is_some_and(|h| h == "hide"))
            {
                when_missing.insert(tool.clone(), "hide".to_owned());
            }
        }
    }
    let mut names = members.first().map(|m| m.names.clone()).unwrap_or_default();
    names.extend(def.names.clone());
    Resolved {
        id: def.id.clone(),
        label: def.label.clone(),
        groups,
        names,
        presets,
        when_missing: if def.when_missing.is_empty() {
            when_missing
        } else {
            def.when_missing.clone()
        },
        default_template: def
            .default_template
            .clone()
            .or_else(|| members.first().and_then(|m| m.default_template.clone()))
            .unwrap_or_default(),
        gallery: if def.gallery.is_empty() {
            gallery
        } else {
            def.gallery.clone()
        },
        workspaces,
    }
}

fn missing_reason(cap: &str) -> &'static str {
    match cap {
        "georeferenced" => {
            "Koordinat sistemi gerekir: proje ayarlarından bir koordinat sistemi seçin."
        }
        "attributeLayers" => {
            "Öznitelikli katman gerekir: projede öznitelik tablosu olan bir katman yok."
        }
        _ => "Bu projede kullanılamaz.",
    }
}

fn has(caps: &Capabilities, cap: &str) -> bool {
    match cap {
        "georeferenced" => caps.georeferenced,
        "attributeLayers" => caps.attribute_layers,
        _ => false,
    }
}

/// The profile of a work mode (none: the common one) for a project with `caps`.
pub fn profile_for(workspace: Option<Workspace>, caps: &Capabilities) -> Profile {
    let r = resolve(workspace);
    let tools = &data().tools;
    let groups = r
        .groups
        .iter()
        .map(|g| ToolGroup {
            id: g.id.clone(),
            label: g.label.clone(),
            tools: g
                .tools
                .iter()
                .filter_map(|id| tools.iter().find(|t| t.id == *id))
                .map(|t| {
                    let missing = t.requires.iter().find(|c| !has(caps, c));
                    let (state, reason) = match missing {
                        None => (ToolState::Enabled, None),
                        Some(c) => (
                            if r.when_missing.get(&t.id).is_some_and(|h| h == "hide") {
                                ToolState::Hidden
                            } else {
                                ToolState::Disabled
                            },
                            Some(missing_reason(c).to_owned()),
                        ),
                    };
                    ToolInfo {
                        id: t.id.clone(),
                        label: r
                            .names
                            .get(&t.id)
                            .cloned()
                            .unwrap_or_else(|| t.label.clone()),
                        item: t.item.clone(),
                        shortcut: t.shortcut.clone(),
                        state,
                        reason,
                        presets: r
                            .presets
                            .get(&t.id)
                            .map(|ps| {
                                ps.iter()
                                    .map(|p| PresetInfo {
                                        id: p.id.clone(),
                                        label: p.label.clone(),
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                    }
                })
                .collect(),
        })
        .collect();
    Profile {
        id: r.id,
        label: r.label,
        groups,
        names: r.names,
        default_template: r.default_template,
        gallery: r.gallery,
    }
}

/// Every tool of the mode's profile with its state, flat, in the ribbon's order.
pub fn tool_availability(workspace: Option<Workspace>, caps: &Capabilities) -> Vec<ToolInfo> {
    profile_for(workspace, caps)
        .groups
        .into_iter()
        .flat_map(|g| g.tools)
        .collect()
}

/// The tool an item was made by (an attribute table is a table with a layer source; an overview map a map with `overviewOf`).
pub fn tool_of(item: &Item) -> &'static str {
    match &item.kind {
        ItemKind::Map(m) if m.overview_of.is_some() => "overviewMap",
        ItemKind::Map(_) => "map",
        ItemKind::Table(t) if matches!(t.source, TableSource::Layer(_)) => "attributeTable",
        ItemKind::Table(_) => "table",
        ItemKind::Text(_) => "text",
        ItemKind::ScaleBar(_) => "scaleBar",
        ItemKind::NorthArrow(_) => "northArrow",
        ItemKind::Legend(_) => "legend",
        ItemKind::Picture(_) => "picture",
        ItemKind::Shape(_) => "shape",
        ItemKind::Line(_) => "line",
        ItemKind::CoordinateList(_) => "coordinateList",
        ItemKind::TitleBlock(_) => "titleBlock",
        ItemKind::Border(_) => "border",
        ItemKind::Group(_) => "group",
    }
}

/// The inspector's note for an item another mode's tool made: it is edited here, but a new one is not added.
pub fn item_note(workspace: Option<Workspace>, item: &Item) -> Option<String> {
    let r = resolve(workspace);
    let tool = tool_of(item);
    if r.groups.iter().any(|g| g.tools.iter().any(|t| t == tool)) {
        return None;
    }
    let owner = data()
        .profiles
        .iter()
        .filter(|p| p.union.is_empty() && p.id != "common")
        .find(|p| p.groups.iter().any(|g| g.tools.iter().any(|t| t == tool)))?;
    let label = data()
        .tools
        .iter()
        .find(|t| t.id == tool)
        .map_or(tool, |t| t.label.as_str());
    Some(format!(
        "Bu öğe ({label}) {} projelerinin aracıdır; {} projelerinde yenisi eklenmez, var olan düzenlenir.",
        owner.label, r.label
    ))
}

/// What a new item is made from: a tool, one of its presets, the host's id and name, its frame, the map it reads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct NewItem {
    pub tool: String,
    /// None: the tool's first preset in this mode, or the kind's defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub preset: Option<String>,
    pub id: String,
    pub name: String,
    pub frame: RectUm,
    /// The map a scale bar, north arrow, legend or text reads, or an overview shows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub link: Option<String>,
}

/// A new item from a tool of the mode's profile: the kind's defaults, the preset over them,
/// the project's plot scale for a map, `link` as the map it reads.
pub fn new_item(workspace: Option<Workspace>, caps: &Capabilities, req: &NewItem) -> Result<Item> {
    let (tool, preset, id, name, frame, link) = (
        req.tool.as_str(),
        req.preset.as_deref(),
        req.id.as_str(),
        req.name.as_str(),
        req.frame,
        req.link.as_deref(),
    );
    let r = resolve(workspace);
    let def = data().tools.iter().find(|t| t.id == tool).ok_or_else(|| {
        SheetError::new(
            "unknown_tool",
            format!("“{tool}” diye bir pafta aracı yok."),
        )
    })?;
    let kind_name = def.item.as_deref().ok_or_else(|| {
        SheetError::new(
            "unknown_tool",
            format!("“{}” aracı öğe eklemez.", def.label),
        )
    })?;
    let kind = default_kind(kind_name)
        .ok_or_else(|| SheetError::new("unknown_tool", format!("“{kind_name}” öğe türü yok.")))?;
    let mut item = Item::new(id, name, frame, kind);
    let presets = r.presets.get(tool);
    let chosen = match preset {
        Some(p) => Some(
            presets
                .and_then(|ps| ps.iter().find(|x| x.id == p))
                .ok_or_else(|| {
                    SheetError::new(
                        "unknown_preset",
                        format!("“{p}” hazır biçimi bu kipte yok."),
                    )
                })?,
        ),
        None => presets.and_then(|ps| ps.first()),
    };
    if let Some(p) = chosen {
        let mut v = serde_json::to_value(&item).map_err(|e| SheetError::json("Öğe", &e))?;
        crate::json::merge(&mut v, &p.item);
        item = serde_json::from_value(v).map_err(|e| {
            SheetError::new(
                "bad_preset",
                format!("“{}” hazır biçimi öğeye uymuyor: {e}", p.label),
            )
        })?;
    }
    if let (ItemKind::Map(m), Some(s)) = (&mut item.kind, caps.plot_scale)
        && m.overview_of.is_none()
        && tool == "map"
        && let MapView::Fixed(f) = &mut m.view
    {
        f.scale = s;
    }
    if let Some(l) = link {
        match &mut item.kind {
            ItemKind::ScaleBar(s) => s.map = Some(l.to_owned()),
            ItemKind::NorthArrow(n) => n.map = Some(l.to_owned()),
            ItemKind::Legend(g) => g.map = Some(l.to_owned()),
            ItemKind::Text(t) => t.map = Some(l.to_owned()),
            ItemKind::Map(m) if tool == "overviewMap" => m.overview_of = Some(l.to_owned()),
            _ => {}
        }
    }
    Ok(item)
}

/// Where a template stands in the gallery of a project.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum TemplateFit {
    /// Made for the project's type.
    ProjectType,
    /// Made for the project's mode.
    Workspace,
    /// For every mode.
    Common,
    /// Another type's: shown with “Bütün türlerin şablonları”.
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct RankedTemplate {
    pub id: String,
    pub fit: TemplateFit,
    /// Shown without “Bütün türlerin şablonları”.
    pub matches: bool,
    /// “GIS şablonu”, “CAD şablonu” for another mode's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub badge: Option<String>,
}

fn is_common(t: &TemplateMeta) -> bool {
    t.workspaces.contains(&Workspace::Cad) && t.workspaces.contains(&Workspace::Gis)
}

/// The gallery's order for a project: its type's templates, then its mode's, then the common ones, then the rest (not shown by default, with a badge).
pub fn rank_templates(
    templates: &[TemplateMeta],
    workspace: Option<Workspace>,
    project_type: Option<ProjectType>,
) -> Vec<RankedTemplate> {
    let r = resolve(workspace);
    let every_mode = r.id == "common";
    let mut ranked: Vec<(TemplateFit, usize, usize, RankedTemplate)> = templates
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let own_mode = t.workspaces.iter().any(|w| r.workspaces.contains(w));
            let fit = if project_type.is_some_and(|p| t.project_types.contains(&p)) {
                TemplateFit::ProjectType
            } else if is_common(t) {
                TemplateFit::Common
            } else if own_mode {
                TemplateFit::Workspace
            } else if every_mode {
                TemplateFit::Common
            } else {
                TemplateFit::Other
            };
            let badge = (fit == TemplateFit::Other).then(|| {
                let mode = t
                    .workspaces
                    .first()
                    .and_then(|w| {
                        data()
                            .profiles
                            .iter()
                            .find(|p| p.union.is_empty() && p.workspaces.contains(w))
                    })
                    .map_or("Başka kip", |p| p.label.as_str());
                format!("{mode} şablonu")
            });
            let order = r
                .gallery
                .iter()
                .position(|g| *g == t.id)
                .unwrap_or(usize::MAX);
            (
                fit,
                order,
                i,
                RankedTemplate {
                    id: t.id.clone(),
                    fit,
                    matches: fit != TemplateFit::Other,
                    badge,
                },
            )
        })
        .collect();
    ranked.sort_by_key(|a| (a.0, a.1, a.2));
    ranked.into_iter().map(|(.., r)| r).collect()
}

/// Every preset of every profile as an item, for the tests: none may fail.
pub fn all_preset_items() -> Vec<(String, String, Result<Item>)> {
    let mut out = Vec::new();
    for p in &data().profiles {
        for (tool, ps) in &p.presets {
            for preset in ps {
                let ws = p.workspaces.first().copied();
                let item = new_item(
                    ws,
                    &Capabilities {
                        georeferenced: true,
                        attribute_layers: true,
                        plot_scale: Some(1000),
                    },
                    &NewItem {
                        tool: tool.clone(),
                        preset: Some(preset.id.clone()),
                        id: "p".into(),
                        name: "Önizleme".into(),
                        frame: RectUm::new(0, 0, 60_000, 40_000),
                        link: None,
                    },
                );
                out.push((format!("{}/{tool}", p.id), preset.id.clone(), item));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps(geo: bool) -> Capabilities {
        Capabilities {
            georeferenced: geo,
            attribute_layers: false,
            plot_scale: Some(500),
        }
    }

    fn req(
        tool: &str,
        preset: Option<&str>,
        id: &str,
        frame: RectUm,
        link: Option<&str>,
    ) -> NewItem {
        NewItem {
            tool: tool.into(),
            preset: preset.map(Into::into),
            id: id.into(),
            name: id.into(),
            frame,
            link: link.map(Into::into),
        }
    }

    fn tool<'a>(tools: &'a [ToolInfo], id: &str) -> Option<&'a ToolInfo> {
        tools.iter().find(|t| t.id == id)
    }

    #[test]
    fn the_data_reads() {
        assert!(!data().profiles.is_empty());
        assert!(!data().tools.is_empty());
        for p in &data().profiles {
            for g in &p.groups {
                for t in &g.tools {
                    assert!(data().tools.iter().any(|d| d.id == *t), "{}: {t}", p.id);
                }
            }
        }
        for (profile, preset, item) in all_preset_items() {
            assert!(item.is_ok(), "{profile} {preset}: {:?}", item.err());
        }
    }

    #[test]
    fn modes_choose_tools_and_names() {
        let cad = tool_availability(Some(Workspace::Cad), &caps(false));
        assert_eq!(tool(&cad, "map").unwrap().label, "Görünüm penceresi");
        // No coordinate system: the CAD ribbon has no north arrow nor grid.
        assert_eq!(tool(&cad, "northArrow").unwrap().state, ToolState::Hidden);
        assert!(tool(&cad, "attributeTable").is_none());
        let cad_geo = tool_availability(Some(Workspace::Cad), &caps(true));
        assert_eq!(
            tool(&cad_geo, "northArrow").unwrap().state,
            ToolState::Enabled
        );
        let gis = tool_availability(Some(Workspace::Gis), &caps(false));
        assert_eq!(tool(&gis, "map").unwrap().label, "Harita");
        let na = tool(&gis, "northArrow").unwrap();
        assert_eq!(na.state, ToolState::Disabled);
        assert!(na.reason.as_deref().unwrap().contains("Koordinat sistemi"));
        assert_eq!(
            tool(&gis, "attributeTable").unwrap().state,
            ToolState::Disabled
        );
        // A mode with no profile row: the common profile.
        let p3 = profile_for(Some(Workspace::Plan3d), &caps(true));
        assert_eq!(p3.id, "common");
        assert_eq!(profile_for(None, &caps(true)).id, "common");
    }

    #[test]
    fn a_new_item_takes_its_preset_and_the_plot_scale() {
        let m = new_item(
            Some(Workspace::Gis),
            &caps(true),
            &req("map", None, "m1", RectUm::new(0, 0, 100_000, 80_000), None),
        )
        .unwrap();
        match &m.kind {
            ItemKind::Map(map) => match &map.view {
                MapView::Fixed(f) => assert_eq!(f.scale, 500),
                MapView::Atlas(_) => panic!(),
            },
            _ => panic!(),
        }
        let b = new_item(
            Some(Workspace::Cad),
            &caps(false),
            &req(
                "border",
                None,
                "b",
                RectUm::new(0, 0, 100_000, 80_000),
                None,
            ),
        )
        .unwrap();
        assert!(matches!(&b.kind, ItemKind::Border(x) if x.zones.is_some() && x.centring_marks));
        let n = new_item(
            Some(Workspace::Gis),
            &caps(true),
            &req(
                "northArrow",
                Some("compass"),
                "n",
                RectUm::new(0, 0, 20_000, 20_000),
                Some("m1"),
            ),
        )
        .unwrap();
        assert!(matches!(&n.kind, ItemKind::NorthArrow(x) if x.map.as_deref() == Some("m1")));
        assert_eq!(
            new_item(
                Some(Workspace::Cad),
                &caps(true),
                &req("grid", None, "g", RectUm::new(0, 0, 10_000, 10_000), None)
            )
            .unwrap_err()
            .code,
            "unknown_tool"
        );
    }

    #[test]
    fn another_modes_item_gets_a_note() {
        let m = new_item(
            Some(Workspace::Gis),
            &caps(true),
            &req(
                "attributeTable",
                None,
                "t",
                RectUm::new(0, 0, 60_000, 40_000),
                None,
            ),
        )
        .unwrap();
        let note = item_note(Some(Workspace::Cad), &m).unwrap();
        assert!(note.contains("CBS projelerinin aracıdır"), "{note}");
        assert!(item_note(Some(Workspace::Gis), &m).is_none());
    }
}
