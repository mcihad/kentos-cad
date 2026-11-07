//! “Bu değer nereden geliyor?” (docs/adr/1100 §5; plan/cad-advanced.md CAD-ADV-12): for one drawn object, where its
//! colour, line type, line weight and text style come from — the object's own value, its layer, the block that draws
//! it (BYBLOCK parts), a sheet viewport's layer override — and whether the bound office standard defines that layer or
//! style the same way. Each part also says what resetting its override would give and to how many objects or
//! viewports the reset reaches, so the scope is seen before anything is written. Read only: the resets themselves are
//! the existing commands (the object's look by `cad.entities.edit`, a viewport's overrides by the sheet's viewport
//! edit). The resolution is `kentos_contracts::resolve_look`'s and the sheet's own `map_layers`, the renderer's rule.

use kentos_commands::{CommandResult, Entity, LayerNode, LayerStyle, StandardsResolved};
use kentos_domain::{Document, Slot};
use serde::Serialize;

/// The level a value comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Origin {
    /// The object's own value (an override of its layer's).
    Object,
    /// Its layer's (ByLayer).
    Layer,
    /// The insert's, for a block's part drawn ByBlock.
    Block,
    /// A sheet viewport's layer override, in that viewport only.
    Viewport,
    /// The bound office standard's definition of the layer or style.
    Standard,
    /// Nothing says it: the drawing's default.
    Default,
}

impl Origin {
    pub fn label(self) -> &'static str {
        match self {
            Origin::Object => "Nesne",
            Origin::Layer => "Katman",
            Origin::Block => "Blok",
            Origin::Viewport => "Görünüm penceresi",
            Origin::Standard => "Standart",
            Origin::Default => "Varsayılan",
        }
    }
}

/// One level of a value's chain: where, what it says there and a note (the layer's name, the standard's revision).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub origin: Origin,
    pub value: String,
    pub note: String,
    /// The level whose value is drawn in the model.
    pub wins: bool,
}

/// What a reset would give and how far it reaches.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reset {
    pub scope: String,
    /// The value afterwards.
    pub after: String,
    /// How many objects (or viewports) the reset changes.
    pub count: u32,
}

/// One part of the look explained.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Explained {
    /// `color`, `lineType`, `lineWeight`, `textStyle`.
    pub part: &'static str,
    /// Its Turkish name.
    pub label: &'static str,
    /// The value the model draws.
    pub value: String,
    pub origin: Origin,
    pub chain: Vec<Link>,
    /// The viewports that draw it otherwise: `Pafta · pencere: değer`.
    pub viewports: Vec<String>,
    /// What the sheets print it as, where a sheet's plot profile gives the object's layer a pen or style
    /// (docs/adr/1015): `Pafta · profil: değer`. Plot ink is derived: the model's value is not changed.
    pub printed: Vec<String>,
    pub resets: Vec<Reset>,
}

fn weight(w: f64) -> String {
    format!("{} mm", (w * 1000.0).round() / 1000.0)
}

fn line_type(style: &LayerStyle) -> String {
    style
        .line_pattern
        .clone()
        .unwrap_or_else(|| style.line_type.label().to_owned())
}

/// The sheet viewports (map items) whose layer list or overrides draw `layer` otherwise: each its place's words and
/// the layer style it draws with there; none where the viewport's own override freezes it (docs/adr/1110 §3).
fn viewport_styles(doc: &Document, node: &LayerNode) -> Vec<(String, Option<LayerStyle>)> {
    let mut out = Vec::new();
    let Some(sheets) = doc.sheets() else {
        return out;
    };
    let Ok((book, _)) = kentos_sheet::project::read_project_sheets(sheets) else {
        return out;
    };
    for sheet in &book.sheets {
        for item in &sheet.items {
            let kentos_sheet::kinds::ItemKind::Map(map) = &item.kind else {
                continue;
            };
            let drawn = kentos_sheet::layers::map_layers(
                doc.layers().nodes(),
                &doc.settings().layer_states,
                &map.layers,
                &map.layer_overrides,
            );
            let at = format!("{} · {}", sheet.name, item.name);
            if map
                .layer_overrides
                .iter()
                .any(|o| o.layer == node.id && o.frozen)
            {
                out.push((at, None));
            } else if let Some(n) = drawn.iter().find(|n| n.id == node.id)
                && n.style != node.style
            {
                out.push((at, Some(n.style.clone())));
            }
        }
    }
    out
}

/// The standard's definition of the layer, with its source; none when no bound standard names it.
fn standard_layer<'a>(
    r: Option<&'a StandardsResolved>,
    name: &str,
) -> Option<(&'a kentos_commands::StandardLayer, String)> {
    let r = r?;
    let key = kentos_commands::fold_name(name.trim());
    let layer = r
        .standard
        .layers
        .iter()
        .find(|l| kentos_commands::fold_name(l.name.trim()) == key)?;
    let origin = r
        .provenance
        .layers
        .get(&layer.name)
        .unwrap_or(&r.provenance.rules);
    Some((layer, format!("{} {}", origin.id, origin.revision)))
}

struct Part<'a> {
    part: &'static str,
    label: &'static str,
    own: Option<String>,
    layer: String,
    viewport: &'a dyn Fn(&LayerStyle) -> String,
    standard: Option<String>,
    /// The objects on the layer with their own value.
    owners: u32,
}

fn explain_part(
    p: Part<'_>,
    node: &LayerNode,
    std_note: Option<&str>,
    vps: &[(String, Option<LayerStyle>)],
    is_insert_part: bool,
) -> Explained {
    let origin = if p.own.is_some() {
        Origin::Object
    } else {
        Origin::Layer
    };
    let value = p.own.clone().unwrap_or_else(|| p.layer.clone());
    let mut chain = vec![
        Link {
            origin: Origin::Object,
            value: p.own.clone().unwrap_or_else(|| "—".into()),
            note: if p.own.is_some() {
                "nesnenin kendi değeri".into()
            } else {
                "katmana göre".into()
            },
            wins: origin == Origin::Object,
        },
        Link {
            origin: Origin::Layer,
            value: p.layer.clone(),
            note: node.name.clone(),
            wins: origin == Origin::Layer,
        },
    ];
    if let (Some(s), Some(note)) = (&p.standard, std_note) {
        let agrees = *s == p.layer;
        chain.push(Link {
            origin: Origin::Standard,
            value: s.clone(),
            note: format!(
                "{note} · {}",
                if agrees {
                    "aynı"
                } else {
                    "farklı; Standardı uygula düzeltir"
                }
            ),
            wins: false,
        });
    }
    if is_insert_part {
        chain.push(Link {
            origin: Origin::Block,
            value: value.clone(),
            note: "ByBlock parçalar bunu alır".into(),
            wins: false,
        });
    }
    let viewports: Vec<String> = vps
        .iter()
        .filter_map(|(at, style)| match style {
            None => Some(format!("{at}: dondurulmuş")),
            Some(style) => {
                let there = (p.viewport)(style);
                (p.own.is_none() && there != p.layer).then(|| format!("{at}: {there}"))
            }
        })
        .collect();
    let mut resets = Vec::new();
    if p.own.is_some() {
        resets.push(Reset {
            scope: "bu nesne".into(),
            after: p.layer.clone(),
            count: 1,
        });
        if p.owners > 1 {
            resets.push(Reset {
                scope: "katmanda kendi değerli nesneler".into(),
                after: p.layer.clone(),
                count: p.owners,
            });
        }
    }
    if !viewports.is_empty() {
        resets.push(Reset {
            scope: "bütün pencereler".into(),
            after: p.layer.clone(),
            count: u32::try_from(viewports.len()).unwrap_or(u32::MAX),
        });
    }
    Explained {
        part: p.part,
        label: p.label,
        value,
        origin,
        chain,
        viewports,
        printed: Vec::new(),
        resets,
    }
}

/// The object's colour, line type, line weight and (for a text) text style explained; none for a slot without an
/// object or whose layer is gone.
/// What each sheet with a plot profile prints `value` (the model's colour or line weight) of an object on `layer` as,
/// where the profile gives that layer a style or has a default one and the ink differs (docs/adr/1015). The profile's
/// own rule: `plot_profile::apply`, the PDF's.
fn printed(doc: &Document, layer: &str, part: &str, value: &str) -> Vec<String> {
    let mut out = Vec::new();
    let Some(sheets) = doc.sheets() else {
        return out;
    };
    let Ok((book, _)) = kentos_sheet::project::read_project_sheets(sheets) else {
        return out;
    };
    for sheet in &book.sheets {
        let Ok(Some(profile)) = kentos_sheet::plot_profile::for_sheet(&book, &sheet.id) else {
            continue;
        };
        let resolved = kentos_sheet::plot_profile::apply::Resolved::new(profile);
        let Some(style) = resolved.model(layer) else {
            continue;
        };
        let ink = match part {
            "color" if value.starts_with('#') => kentos_sheet::plot_profile::apply::color(value, style),
            // A theme token (`fg`, `ink`): the pen's own colour when the profile fixes one; not resolved here.
            "color" => match &style.color {
                kentos_sheet::plot_profile::PlotColor::Black => "#000000".to_owned(),
                kentos_sheet::plot_profile::PlotColor::Color { value } => value.clone(),
                kentos_sheet::plot_profile::PlotColor::Grayscale => format!("{value} gri tonda"),
                kentos_sheet::plot_profile::PlotColor::Object => continue,
            },
            _ => match style.line_weight {
                Some(um) => weight(f64::from(um) / 1000.0),
                None => continue,
            },
        };
        if ink != value {
            out.push(format!("{} · “{}” profili: {ink}", sheet.name, profile.name));
        }
    }
    out
}

pub fn explain(doc: &Document, slot: Slot) -> Vec<Explained> {
    let Some(e) = doc.get(slot) else {
        return Vec::new();
    };
    let base = e.base();
    let Some(node) = doc.layers().get(&base.layer_id) else {
        return Vec::new();
    };
    let resolved = match crate::standards::resolve_project(doc) {
        CommandResult::Completed { output, .. } => Some(output),
        _ => None,
    };
    let standard = standard_layer(resolved.as_ref(), &node.name);
    let note = standard.as_ref().map(|(_, n)| n.as_str());
    let vps = viewport_styles(doc, node);
    let on_layer = |has: &dyn Fn(&Entity) -> bool| {
        let n = doc
            .entities()
            .filter(|x| x.base().layer_id == base.layer_id && has(x))
            .count();
        u32::try_from(n).unwrap_or(u32::MAX)
    };
    let insert = matches!(e, Entity::Insert(_));
    let mut out = vec![explain_part(
        Part {
            part: "color",
            label: "Renk",
            own: base.color.clone(),
            layer: node.style.color.clone(),
            viewport: &|s: &LayerStyle| s.color.clone(),
            standard: standard.as_ref().map(|(l, _)| l.color.clone()),
            owners: on_layer(&|x: &Entity| x.base().color.is_some()),
        },
        node,
        note,
        &vps,
        insert,
    )];
    if e.draws_lines() || insert {
        let own_type = base
            .look
            .line_pattern
            .clone()
            .or_else(|| base.look.line_type.map(|t| t.label().to_owned()));
        out.push(explain_part(
            Part {
                part: "lineType",
                label: "Çizgi tipi",
                own: own_type,
                layer: line_type(&node.style),
                viewport: &line_type,
                standard: standard
                    .as_ref()
                    .map(|(l, _)| l.line_type.label().to_owned()),
                owners: on_layer(&|x: &Entity| {
                    x.base().look.line_type.is_some() || x.base().look.line_pattern.is_some()
                }),
            },
            node,
            note,
            &vps,
            insert,
        ));
        out.push(explain_part(
            Part {
                part: "lineWeight",
                // Çizgi kalınlığı, as Öznitelikler names it: a wall's Kalınlık is its width (docs/adr/3003).
                label: "Çizgi kalınlığı",
                own: base.line_weight.map(weight),
                layer: weight(node.style.line_weight),
                viewport: &|s: &LayerStyle| weight(s.line_weight),
                standard: standard.as_ref().map(|(l, _)| weight(l.line_weight)),
                owners: on_layer(&|x: &Entity| x.base().line_weight.is_some()),
            },
            node,
            note,
            &vps,
            insert,
        ));
    }
    for part in out.iter_mut().filter(|p| matches!(p.part, "color" | "lineWeight")) {
        part.printed = printed(doc, &base.layer_id, part.part, &part.value);
    }
    if let Entity::Text(t) = e {
        out.push(text_style(doc, &t.face, resolved.as_ref()));
    }
    if let Entity::Insert(i) = e
        && let Some(def) = doc.block(i.block)
    {
        if let Some(part) = block_parts(doc, base, &node.style, def) {
            out.push(part);
        }
        if let Some(x) = def.xref.as_ref()
            && let Some(part) = xref_layers(doc, x)
        {
            out.push(part);
        }
    }
    out
}

/// A block's own parts (docs/adr/1018): for colour, line type and line weight, how many of the definition's objects
/// draw their own value, the insert's (ByBlock) or their layer's (a `byLayer` part; an object on the block's layer
/// `""` takes the insert's layer), and the values each source gives through this insert. The rule is `resolve_look`'s, the renderer's. A nested insert is counted as one part, as it draws itself;
/// its own parts are explained by selecting it in the block editor. None for a definition without objects.
fn block_parts(
    doc: &Document,
    insert: &kentos_commands::EntityBase,
    insert_layer: &LayerStyle,
    def: &kentos_commands::BlockDefinition,
) -> Option<Explained> {
    use kentos_commands::{LookSource, OwnLook, resolve_look};
    if def.entities.is_empty() {
        return None;
    }
    let through = resolve_look(
        OwnLook {
            color: insert.color.as_deref(),
            line_weight: insert.line_weight,
            look: &insert.look,
        },
        insert_layer,
        None,
    );
    // Per look part (colour, line type, weight) and source (own, block, layer): the count and the values, in order.
    let mut tally: [[(u32, Vec<String>); 3]; 3] = Default::default();
    let index = |s: LookSource| match s {
        LookSource::Own => 0,
        LookSource::Block => 1,
        LookSource::Layer => 2,
    };
    for part in &def.entities {
        let b = part.base();
        let style = if b.layer_id.is_empty() {
            insert_layer
        } else {
            doc.layers().get(&b.layer_id).map_or(insert_layer, |n| &n.style)
        };
        let r = resolve_look(
            OwnLook {
                color: b.color.as_deref(),
                line_weight: b.line_weight,
                look: &b.look,
            },
            style,
            Some(&through),
        );
        let pattern = || b.look.line_pattern.clone();
        let values = [
            r.color.clone(),
            pattern()
                .filter(|_| r.from[1] == LookSource::Own)
                .unwrap_or_else(|| r.line_type.label().to_owned()),
            weight(r.line_weight),
        ];
        for (k, (from, value)) in [r.from[0], r.from[1], r.from[3]].into_iter().zip(values).enumerate() {
            let cell = &mut tally[k][index(from)];
            cell.0 += 1;
            if !cell.1.contains(&value) {
                cell.1.push(value);
            }
        }
    }
    let n = u32::try_from(def.entities.len()).unwrap_or(u32::MAX);
    let names = ["Renk", "Çizgi tipi", "Çizgi kalınlığı"];
    let origins = [Origin::Object, Origin::Block, Origin::Layer];
    let words = ["kendi değeri", "bloktan (ByBlock)", "katmandan (ByLayer)"];
    let mut chain = Vec::new();
    for (k, name) in names.iter().enumerate() {
        for (j, (count, values)) in tally[k].iter().enumerate() {
            if *count == 0 {
                continue;
            }
            let shown: Vec<&str> = values.iter().take(3).map(String::as_str).collect();
            let more = values.len().saturating_sub(3);
            chain.push(Link {
                origin: origins[j],
                value: if more > 0 {
                    format!("{} +{more}", shown.join(", "))
                } else {
                    shown.join(", ")
                },
                note: format!("{name}: {count}/{n} parça {}", words[j]),
                wins: false,
            });
        }
    }
    let by_block = tally.iter().map(|t| t[1].0).max().unwrap_or(0);
    let own = tally.iter().map(|t| t[0].0).max().unwrap_or(0);
    let origin = if by_block >= own.max(tally.iter().map(|t| t[2].0).max().unwrap_or(0)) {
        Origin::Block
    } else if own > 0 {
        Origin::Object
    } else {
        Origin::Layer
    };
    Some(Explained {
        part: "blockParts",
        label: "Blok parçaları",
        value: format!("“{}” · {n} parça", def.name),
        origin,
        chain,
        viewports: Vec::new(),
        printed: Vec::new(),
        resets: Vec::new(),
    })
}

/// An external reference's layers (docs/adr/1110 §3): three levels apart — the source's look, the drawing's own
/// override of it (kept through a reload, set back by Katmanları kaynağa döndür) and each sheet viewport's own
/// override (no reference operation touches it). None for a reference without its layer group.
fn xref_layers(doc: &Document, x: &kentos_commands::XrefSource) -> Option<Explained> {
    let group = x.group.as_deref().filter(|g| doc.layers().get(g).is_some())?;
    let layers = crate::xrefs_review::group_paths(doc, group)
        .iter()
        .filter(|(id, _)| {
            doc.layers()
                .get(id)
                .is_some_and(|n| n.kind == kentos_commands::LayerNodeType::Layer)
        })
        .count();
    let overrides = crate::xrefs_review::viewport_overrides(doc, Some(group));
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    let mut resets = vec![Reset {
        scope: "Katmanları kaynağa döndür: çizimin ezmesi".into(),
        after: "kaynağın görünüşü".into(),
        count: count(layers),
    }];
    if !overrides.is_empty() {
        resets.push(Reset {
            scope: "pencerelerin kendi ezmeleri (dış başvuru işlemleri dokunmaz)".into(),
            after: "çizimin görünüşü".into(),
            count: count(overrides.len()),
        });
    }
    Some(Explained {
        part: "xrefLayers",
        label: "Dış başvuru katmanları",
        value: format!("{layers} katman"),
        origin: Origin::Layer,
        chain: vec![Link {
            origin: Origin::Layer,
            value: "çizimin görünüşü".into(),
            note: "kaynağınkinden ayrıldıysa yeniden yüklemede kalır".into(),
            wins: false,
        }],
        printed: Vec::new(),
        viewports: overrides
            .iter()
            .map(|o| {
                format!(
                    "{} · {}: {} — {}",
                    o.sheet,
                    o.viewport,
                    o.layer,
                    crate::xrefs_review::override_words(o)
                )
            })
            .collect(),
        resets,
    })
}

fn text_style(
    doc: &Document,
    face: &kentos_commands::TextFace,
    r: Option<&StandardsResolved>,
) -> Explained {
    let styles = &doc.settings().text_styles;
    let style = face
        .text_style
        .as_deref()
        .and_then(|id| styles.iter().find(|s| s.id == id));
    let name = style.map_or_else(
        || kentos_commands::STANDARD_STYLE.to_owned(),
        |s| s.name.clone(),
    );
    let origin = if style.is_some() {
        Origin::Object
    } else {
        Origin::Default
    };
    let mut chain = vec![Link {
        origin,
        value: name.clone(),
        note: if style.is_some() {
            "nesnenin seçtiği stil".into()
        } else {
            "stil seçilmemiş: Standart".into()
        },
        wins: true,
    }];
    if let (Some(style), Some(r)) = (style, r) {
        let key = style.name.trim().to_lowercase();
        if let Some(def) = r
            .resources
            .text_style_definitions
            .iter()
            .find(|d| d.name.trim().to_lowercase() == key)
        {
            let fields = crate::standards::style_differences(
                &crate::standards::resource_appearance(style),
                &crate::standards::resource_appearance(def),
            );
            let o = r
                .provenance
                .text_style_definitions
                .get(&def.name)
                .unwrap_or(&r.provenance.rules);
            chain.push(Link {
                origin: Origin::Standard,
                value: def.name.clone(),
                note: if fields.is_empty() {
                    format!("{} {} · aynı", o.id, o.revision)
                } else {
                    format!("{} {} · farklı: {}", o.id, o.revision, fields.join(", "))
                },
                wins: false,
            });
        }
    }
    let mut resets = Vec::new();
    if !face.overrides.is_empty() {
        chain.push(Link {
            origin: Origin::Object,
            value: face.overrides.join(", "),
            note: "stilden elle ayrılan alanlar".into(),
            wins: true,
        });
        resets.push(Reset {
            scope: "bu yazının elle ayrılan alanları".into(),
            after: name.clone(),
            count: 1,
        });
    }
    Explained {
        part: "textStyle",
        label: "Yazı stili",
        value: name,
        origin,
        chain,
        viewports: Vec::new(),
        printed: Vec::new(),
        resets,
    }
}
