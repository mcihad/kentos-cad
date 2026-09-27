//! The model designer's words and rules apart from the window (the web's
//! ui/processing/model/designerPlan.ts; docs/specs/model-designer.md): the
//! title and the status line, what a box and an edge say, the menu a wire
//! dropped on a step opens, how a source is named, where a new box goes,
//! when typing joins one undo step, and the diagram's geometry (box sizes,
//! an edge's curve and label, fitting, zooming, the grid). The model's own
//! edits are model_edit.rs's. fixtures/processing/v1/designer.json holds
//! both; the web and the desktop play it.

use crate::model::{Model, ModelIssue, ModelStep, ValueSource, step_name};
use crate::model_edit::{NodeRef, edges_of, sources_for};
use crate::types::Tool;

type Lookup<'a> = &'a dyn Fn(&str) -> Option<Tool>;

/// The designer's words (the web's `DESIGNER_TEXTS`).
pub mod texts {
    pub const TITLE: &str = "Model tasarımcısı";

    /// The window's title: “Model tasarımcısı: <ad>”, “adsız” for a blank
    /// name, and “ •” while there are unsaved changes.
    pub fn title_of(label: &str, dirty: bool) -> String {
        let label = if label.is_empty() { "adsız" } else { label };
        format!("{TITLE}: {label}{}", if dirty { " •" } else { "" })
    }

    pub fn not_found(id: &str) -> String {
        format!("Model bulunamadı: {id}.")
    }

    pub mod footer {
        pub const LAYOUT: &str = "Düzenle";
        pub const LAYOUT_TIP: &str = "Kutuları bağlantı sırasına göre sütunlara dizer";
        pub const CLOSE: &str = "Kapat";
        pub const SAVE_RUN: &str = "Kaydet ve çalıştır…";
        pub const SAVE: &str = "Kaydet";
    }

    pub mod status {
        pub fn problems(n: usize, first: &str) -> String {
            format!("{n} sorun var; model kaydedilebilir ama çalışmaz. {first}")
        }

        pub fn ready(steps: usize, inputs: usize) -> String {
            format!("{steps} adım, {inputs} girdi. Model çalışmaya hazır.")
        }

        pub const EMPTY: &str = "Soldan bir girdi ve bir araç ekleyerek başlayın.";
    }

    pub mod save {
        pub const UNNAMED: &str = "Adsız model";

        pub fn saved(label: &str, problems: usize) -> String {
            let rest = if problems > 0 {
                format!("; {problems} sorun giderilene kadar çalışmaz")
            } else {
                String::new()
            };
            format!("“{label}” modeli kaydedildi{rest}.")
        }
    }

    pub mod unsaved {
        pub const AFTER: &str = "Pencere kapanırsa bu değişiklikler kaybolur.";
        pub const VERB: &str = "kapat";
    }

    pub mod remove {
        pub const TITLE: &str = "Modeli sil";
        pub const ACTION: &str = "Modeli sil";

        pub fn question(label: &str) -> String {
            format!("“{label}” modeli silinsin mi? Bu geri alınamaz.")
        }

        pub fn done(label: &str) -> String {
            format!("“{label}” modeli silindi.")
        }
    }

    pub mod pick {
        pub const POINT: &str = "Nokta";
        pub const UNNAMED: &str = "nokta";

        pub fn command(label: &str) -> String {
            format!("Model tasarımcısı: {label}")
        }
    }

    pub mod connect {
        pub const REPLACES: &str = "Mevcut bağlantının yerine geçer";

        pub fn header(step: &str) -> String {
            format!("{step}: hangi girdi?")
        }

        pub fn from_step(output: &str, param: &str) -> String {
            format!("{output} → {param}")
        }

        pub fn none(what: &str) -> String {
            format!("“{what}” bu adımın hiçbir girdisine uymuyor")
        }
    }

    pub mod canvas {
        pub const LABEL: &str = "Model diyagramı";
        pub const ZOOM_OUT: &str = "Uzaklaş";
        pub const ZOOM_IN: &str = "Yakınlaş";
        pub const FIT: &str = "Tümünü göster (çift tık)";
        pub const PORT: &str = "Sürükleyip bir adımın üzerine bırakın";
        pub const NO_LINKS: &str = "Bağlantı yok";

        pub fn input_meta(type_label: &str, optional: bool) -> String {
            format!(
                "Girdi: {type_label}{}",
                if optional { ", isteğe bağlı" } else { "" }
            )
        }

        pub fn links(n: usize) -> String {
            format!("{n} bağlantı")
        }
    }

    pub mod palette {
        pub const LABEL: &str = "Model parçaları";
        pub const INPUTS: &str = "Girdi ekle";
        pub const TOOLS: &str = "Araçlar";
        pub const SEARCH: &str = "Araç ara";
        pub const EMPTY: &str = "Aramayla eşleşen araç yok.";
        pub const TOOL_NOTE: &str = "Tıklayın ya da tuvale sürükleyin";
        pub const TIP: &str = "Bir aracı tıklayın ya da tuvale sürükleyin. Seçili kutu varsa yeni adım ona bağlanır; bağlantıyı değiştirmek için kutunun sağındaki noktadan sürükleyin.";
    }

    pub mod inspector {
        pub const LABEL: &str = "Seçilen kutunun ayarları";

        pub mod model {
            pub const KIND: &str = "Model";
            pub const LEAD: &str = "Adı ve açıklaması araç kutusunda ve menüde görünür.";
            pub const NAME: &str = "Ad";
            pub const NAME_ARIA: &str = "Model adı";
            pub const CATEGORY: &str = "Kategori";
            pub const DESCRIPTION: &str = "Açıklama";
            pub const DESCRIPTION_ARIA: &str = "Model açıklaması";
            pub const DESCRIPTION_HINT: &str = "Ne yapar, tek cümle";
            pub const OUTPUTS: &str = "Model çıktıları";
            pub const NO_STEP: &str = "adım yok";
            pub const ADD_OUTPUT: &str = "Çıktı ekle";
            pub const NO_OUTPUT: &str = "Eklenebilecek çıktı yok";
            pub const REMOVE: &str = "Modeli sil";

            pub fn output(step: &str, output: &str) -> String {
                format!("{step} › {output}")
            }

            pub fn remove_output(label: &str) -> String {
                format!("“{label}” çıktısını kaldır")
            }
        }

        pub mod problems {
            pub const TITLE: &str = "Sorunlar";
            pub const READY: &str = "Model çalışmaya hazır.";
            pub const START: &str = "Başlamak için soldan bir girdi ve bir araç ekleyin.";

            pub fn title_count(n: usize) -> String {
                format!("Sorunlar ({n})")
            }

            pub fn of_step(step: &str, message: &str) -> String {
                format!("{step}: {message}")
            }
        }

        pub mod input {
            pub const LEAD: &str = "Model çalıştırılırken kullanıcıdan istenir.";
            pub const LABEL: &str = "Etiket";
            pub const LABEL_ARIA: &str = "Girdi etiketi";
            pub const DESCRIPTION: &str = "Açıklama";
            pub const DESCRIPTION_ARIA: &str = "Girdi açıklaması";
            pub const DESCRIPTION_HINT: &str = "Pencerede etiketin altında görünür";
            pub const OPTIONAL: &str = "İsteğe bağlı";
            pub const DEFAULT: &str = "Varsayılan";
            pub const SCOPE_ARIA: &str = "Varsayılan kapsam";
            pub const SCOPES: [(&str, &str); 3] = [
                ("selection", "Seçili"),
                ("visible", "Görünen"),
                ("all", "Tümü"),
            ];
            pub const KINDS: &str = "Uygun nesneler";
            pub const KINDS_NOTE: &str =
                "Hiçbiri seçili değilse her tür alınır; adımlar kendi türlerini ayrıca süzer.";
            pub const MIN: &str = "En az";
            pub const MAX: &str = "En çok";
            pub const NONE: &str = "yok";
            pub const INTEGER: &str = "Tam sayı";
            pub const TEXT_DEFAULT: &str = "Varsayılan metin";
            pub const ALLOW_EMPTY: &str = "Boş bırakılabilir";
            pub const NEW_LAYER: &str = "Varsayılan yeni katman";
            pub const NEW_LAYER_ARIA: &str = "Varsayılan yeni katman adı";
            pub const NEW_LAYER_NOTE: &str = "Çalıştırırken var olan bir katman da seçilebilir.";
            pub const POINT_NOTE: &str = "Çalıştırırken haritada gösterilir ya da Y,X yazılır.";
            pub const USERS: &str = "Kullanan adımlar";
            pub const NO_USERS: &str = "Henüz hiçbir adım bu girdiyi kullanmıyor. Kutunun sağındaki noktadan bir adıma sürükleyin.";
            pub const REMOVE: &str = "Girdiyi sil";

            pub fn kind(type_label: &str) -> String {
                format!("Girdi: {type_label}")
            }

            pub fn variable(name: &str) -> String {
                format!("Değişken adı: {name}")
            }
        }

        pub mod step {
            pub const UNKNOWN: &str = "Bilinmeyen araç";
            pub const CAPTION: &str = "Başlık";
            pub const CAPTION_ARIA: &str = "Adım başlığı";
            pub const CAPTION_NOTE: &str = "Diyagramda ve iletilerde görünür.";
            pub const PARAMS: &str = "Parametreler";
            pub const OPTIONAL: &str = "isteğe bağlı";
            pub const TOOL_DEFAULT: &str = "Aracın varsayılanı";
            pub const FIXED: &str = "Sabit değer";
            pub const MODEL_INPUT: &str = "Model girdisi";
            pub const AS_INPUT: &str = "Yeni model girdisi yap";
            pub const AS_INPUT_NOTE: &str = "Model çalıştırılırken bu değer sorulur";
            pub const REMOVE: &str = "Adımı sil";

            pub fn unknown_note(tool: &str) -> String {
                format!(
                    "“{tool}” bu sürümde yok. Adımı silin ya da aracı sağlayan eklentiyi yükleyin."
                )
            }

            pub fn advanced(n: usize) -> String {
                format!("Gelişmiş ({n})")
            }

            pub fn source_aria(label: &str) -> String {
                format!("{label}: kaynak")
            }

            pub fn input_source(label: &str) -> String {
                format!("Girdi: {label}")
            }

            pub fn output_source(step: &str, output: &str) -> String {
                format!("{step} › {output}")
            }
        }
    }
}

/// The line under the diagram's kind: a warning while there are problems.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusKind {
    Warn,
    Ok,
}

/// The line under the diagram: the problems and the first one, or how big
/// the model is and that it is ready, or how to begin.
pub fn designer_status(
    problems: &[ModelIssue],
    steps: usize,
    inputs: usize,
) -> (StatusKind, String) {
    match problems.first() {
        Some(first) => (
            StatusKind::Warn,
            texts::status::problems(problems.len(), &first.message),
        ),
        None if steps > 0 => (StatusKind::Ok, texts::status::ready(steps, inputs)),
        None => (StatusKind::Ok, texts::status::EMPTY.to_owned()),
    }
}

/// The name a model is saved under: its label, or “Adsız model” when that is blank.
pub fn saved_label(label: &str) -> String {
    if crate::text::js_trim(label).is_empty() {
        texts::save::UNNAMED.to_owned()
    } else {
        label.to_owned()
    }
}

/// How a step parameter's source reads on its list.
pub fn source_text(model: &Model, src: Option<&ValueSource>, lookup: Lookup<'_>) -> String {
    use texts::inspector::step;
    match src {
        None => step::TOOL_DEFAULT.to_owned(),
        Some(ValueSource::Value(_)) => step::FIXED.to_owned(),
        Some(ValueSource::Input(name)) => step::input_source(
            model
                .inputs
                .iter()
                .find(|i| i.name() == name)
                .map_or(name.as_str(), |i| i.label()),
        ),
        Some(ValueSource::Output { step: id, output }) => {
            let from = model.steps.iter().find(|s| &s.id == id);
            let label = from
                .and_then(|s| lookup(&s.tool))
                .and_then(|t| t.outputs.into_iter().find(|o| &o.name == output))
                .map_or_else(|| output.clone(), |o| o.label);
            step::output_source(
                &from.map_or_else(|| id.clone(), |s| step_name(s, lookup)),
                &label,
            )
        }
    }
}

/// A step box's second line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepMeta {
    /// Its first problem, in the warning's colour.
    Warn(String),
    Text(String),
}

/// A step box's second line: its first problem, the tool's name under a
/// caption, how many links, or none.
pub fn step_meta(step: &ModelStep, tool: Option<&Tool>, problem: Option<&str>) -> StepMeta {
    if let Some(problem) = problem {
        return StepMeta::Warn(problem.to_owned());
    }
    if step.caption.is_some()
        && let Some(tool) = tool
    {
        return StepMeta::Text(tool.label.clone());
    }
    let links = step
        .values
        .iter()
        .filter(|(_, v)| !matches!(v, ValueSource::Value(_)))
        .count();
    StepMeta::Text(if links > 0 {
        texts::canvas::links(links)
    } else {
        texts::canvas::NO_LINKS.to_owned()
    })
}

/// An edge's label: the parameter it feeds, or the first and how many more.
pub fn edge_label(labels: &[String]) -> String {
    match labels {
        [] => String::new(),
        [one] => one.clone(),
        [first, rest @ ..] => format!("{first} +{}", rest.len()),
    }
}

/// A line of the menu a wire dropped on a step opens.
#[derive(Clone, Debug, PartialEq)]
pub struct ConnectChoice {
    /// The parameter it feeds.
    pub param: String,
    pub label: String,
    /// Said when the parameter is fed by something else now.
    pub detail: Option<String>,
    /// The parameter is fed by exactly this already.
    pub checked: bool,
    pub src: ValueSource,
}

/// The menu a wire dropped on a step opens.
#[derive(Clone, Debug, PartialEq)]
pub struct ConnectMenu {
    pub header: String,
    pub items: Vec<ConnectChoice>,
    /// The one line when nothing fits.
    pub none: Option<String>,
}

/// A wire from a box's port dropped on a step: which of the step's
/// parameters the source can feed, each output of a source step in turn
/// (never a cycle, never a type that does not fit), or the one line that
/// says nothing fits. None when the step or its tool is not there.
pub fn connect_choices(
    model: &Model,
    from: &NodeRef,
    to_step: &str,
    lookup: Lookup<'_>,
) -> Option<ConnectMenu> {
    let step = model.steps.iter().find(|s| s.id == to_step)?;
    let tool = lookup(&step.tool)?;
    let source_step = match from {
        NodeRef::Step(id) => model.steps.iter().find(|s| &s.id == id),
        NodeRef::Input(_) => None,
    };
    let outputs: Vec<Option<crate::types::OutputDef>> = match from {
        NodeRef::Input(_) => vec![None],
        NodeRef::Step(_) => source_step
            .and_then(|s| lookup(&s.tool))
            .map(|t| t.outputs.into_iter().map(Some).collect())
            .unwrap_or_default(),
    };
    let mut items = Vec::new();
    for out in &outputs {
        for p in &tool.parameters {
            let fits = sources_for(model, to_step, p, lookup)
                .into_iter()
                .find(|o| match (from, &o.src, out) {
                    (NodeRef::Input(name), ValueSource::Input(n), _) => n == name,
                    (NodeRef::Step(id), ValueSource::Output { step, output }, Some(out)) => {
                        step == id && *output == out.name
                    }
                    _ => false,
                });
            let Some(fits) = fits else {
                continue;
            };
            let current = step.source(&p.name);
            let same = current == Some(&fits.src);
            items.push(ConnectChoice {
                param: p.name.clone(),
                label: match out {
                    Some(out) => texts::connect::from_step(&out.label, &p.label),
                    None => p.label.clone(),
                },
                detail: (current.is_some() && !same).then(|| texts::connect::REPLACES.to_owned()),
                checked: same,
                src: fits.src,
            });
        }
    }
    let what = match from {
        NodeRef::Input(name) => model
            .inputs
            .iter()
            .find(|i| i.name() == name)
            .map_or_else(|| "undefined".to_owned(), |i| i.label().to_owned()),
        NodeRef::Step(_) => {
            source_step.map_or_else(|| "undefined".to_owned(), |s| step_name(s, lookup))
        }
    };
    let none = items.is_empty().then(|| texts::connect::none(&what));
    Some(ConnectMenu {
        header: texts::connect::header(&step_name(step, lookup)),
        items,
        none,
    })
}

/// A point of the diagram (world px).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
}

impl Pt {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// The diagram's measures (the web's `CANVAS`).
pub mod canvas {
    pub const INPUT_W: f64 = 190.0;
    pub const INPUT_H: f64 = 52.0;
    pub const STEP_W: f64 = 240.0;
    pub const STEP_H: f64 = 60.0;
    /// Boxes snap to this (px); the background grid is twice it.
    pub const GRID: f64 = 10.0;
    pub const ZOOM_MIN: f64 = 0.35;
    pub const ZOOM_MAX: f64 = 2.0;
    /// The zoom buttons' factor.
    pub const ZOOM_STEP: f64 = 1.25;
    /// The wheel zooms by exp(−deltaY × WHEEL).
    pub const WHEEL: f64 = 0.0015;
    /// Fitting leaves this margin (px) and never zooms in past 1:1.
    pub const FIT_PAD: f64 = 48.0;
    /// The view of an empty diagram.
    pub const EMPTY: super::View = super::View {
        x: 24.0,
        y: 24.0,
        k: 1.0,
    };
    /// A press becomes a drag after this many px; a tool carried from the palette after `PALETTE_DRAG`.
    pub const DRAG: f64 = 3.0;
    pub const PALETTE_DRAG: f64 = 5.0;
    /// Next to a box, and below the lowest one (spot_near, auto_layout's columns and rows).
    pub const COLUMN: f64 = 290.0;
    pub const ROW: f64 = 100.0;
    /// An edge's control points stand at least this far out.
    pub const BEND: f64 = 40.0;
    /// Edge labels at their target: they end this far left of the step's
    /// entry point, the first row's baseline this far above it, each
    /// further row this much higher.
    pub const LABEL_GAP: f64 = 8.0;
    pub const LABEL_RISE: f64 = 6.0;
    pub const LABEL_ROW: f64 = 13.0;
}

/// The diagram's view: screen = world × k + (x, y).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub x: f64,
    pub y: f64,
    pub k: f64,
}

/// JavaScript's `Math.round`: halves go up.
fn js_round(v: f64) -> f64 {
    (v + 0.5).floor()
}

/// A world position snapped to the grid.
pub fn snap(v: f64) -> f64 {
    js_round(v / canvas::GRID) * canvas::GRID
}

/// Where a new box goes: right of the selected one, else at the left below every other box.
pub fn spot_near(model: &Model, selected: Option<&NodeRef>) -> Pt {
    let pos = match selected {
        Some(NodeRef::Input(name)) => model.input_positions.get(name).copied(),
        Some(NodeRef::Step(id)) => model
            .steps
            .iter()
            .find(|s| &s.id == id)
            .and_then(|s| s.position),
        None => None,
    };
    if let Some((x, y)) = pos {
        return Pt::new(x + canvas::COLUMN, y);
    }
    let lowest = model
        .steps
        .iter()
        .map(|s| s.position.map_or(0.0, |p| p.1))
        .chain(model.input_positions.values().map(|p| p.1))
        .fold(None, |m: Option<f64>, y| Some(m.map_or(y, |m| m.max(y))));
    Pt::new(40.0, lowest.map_or(40.0, |y| y + canvas::ROW))
}

/// The designer's own undo keeps this many steps; typing into one field
/// within `COALESCE_MS` is one step.
pub const HISTORY_DEPTH: usize = 100;
pub const COALESCE_MS: u64 = 1200;

/// Whether a change joins the undo step before it: the same field, typed
/// into again within `COALESCE_MS` (times in ms).
pub fn joins(key: Option<&str>, last: Option<(&str, i64)>, now: i64) -> bool {
    matches!((key, last), (Some(key), Some((last_key, at)))
        if key == last_key && now - at < COALESCE_MS as i64)
}

/// An edge from a port (a box's right side) to a step's left side: a cubic
/// whose control points stand out horizontally.
pub fn curve(a: Pt, b: Pt) -> (Pt, Pt) {
    let dx = canvas::BEND.max((b.x - a.x).abs() / 2.0);
    (Pt::new(a.x + dx, a.y), Pt::new(b.x - dx, b.y))
}

/// Where an input's port is (its right side, halfway down).
pub fn input_port(at: Pt) -> Pt {
    Pt::new(at.x + canvas::INPUT_W, at.y + canvas::INPUT_H / 2.0)
}

/// Where a step's port is (its right side, halfway down).
pub fn step_port(at: Pt) -> Pt {
    Pt::new(at.x + canvas::STEP_W, at.y + canvas::STEP_H / 2.0)
}

/// Where an edge ends on a step (its left side, halfway down).
pub fn step_entry(at: Pt) -> Pt {
    Pt::new(at.x, at.y + canvas::STEP_H / 2.0)
}

/// Where an input box is: its place, else (40, 40).
pub fn input_at(model: &Model, name: &str) -> Pt {
    model
        .input_positions
        .get(name)
        .map_or(Pt::new(40.0, 40.0), |&(x, y)| Pt::new(x, y))
}

/// Where a step box is: its place, else (0, 0).
pub fn step_at(step: &ModelStep) -> Pt {
    step.position
        .map_or(Pt::new(0.0, 0.0), |(x, y)| Pt::new(x, y))
}

/// An edge's label: its text, the tip naming every parameter it feeds,
/// and where it stands.
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeLabel {
    pub from: NodeRef,
    pub to: String,
    pub text: String,
    pub title: String,
    /// The label's right end on its baseline (world px).
    pub at: Pt,
}

/// Every edge's label, at its target step: right-aligned, ending
/// `LABEL_GAP` px left of the step's entry point, the first row's baseline
/// `LABEL_RISE` px above it, each further edge into the step one row
/// higher. The edge whose source port is lowest takes the row nearest the
/// entry (ties: the edges' order), so read top to bottom the labels follow
/// their sources. An edge into a step that is not in the model has none.
pub fn edge_labels(model: &Model, lookup: Lookup<'_>) -> Vec<EdgeLabel> {
    let edges = edges_of(model);
    let source_y = |from: &NodeRef| match from {
        NodeRef::Input(name) => input_port(input_at(model, name)).y,
        NodeRef::Step(id) => {
            step_port(
                model
                    .steps
                    .iter()
                    .find(|s| &s.id == id)
                    .map_or(Pt::new(0.0, 0.0), step_at),
            )
            .y
        }
    };
    let mut rows = vec![0usize; edges.len()];
    let mut seen: Vec<&str> = Vec::new();
    for e in &edges {
        if seen.contains(&e.to.as_str()) {
            continue;
        }
        seen.push(&e.to);
        let mut into: Vec<usize> = (0..edges.len()).filter(|&i| edges[i].to == e.to).collect();
        into.sort_by(|&a, &b| {
            source_y(&edges[b].from)
                .partial_cmp(&source_y(&edges[a].from))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.cmp(&b))
        });
        for (row, i) in into.into_iter().enumerate() {
            rows[i] = row;
        }
    }
    edges
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let step = model.steps.iter().find(|s| s.id == e.to)?;
            let tool = lookup(&step.tool);
            let names: Vec<String> = e
                .params
                .iter()
                .map(|p| {
                    tool.as_ref()
                        .and_then(|t| t.parameters.iter().find(|d| &d.name == p))
                        .map_or_else(|| p.clone(), |d| d.label.clone())
                })
                .collect();
            let entry = step_entry(step_at(step));
            Some(EdgeLabel {
                from: e.from.clone(),
                to: e.to.clone(),
                text: edge_label(&names),
                title: names.join(", "),
                at: Pt::new(
                    entry.x - canvas::LABEL_GAP,
                    entry.y - canvas::LABEL_RISE - rows[i] as f64 * canvas::LABEL_ROW,
                ),
            })
        })
        .collect()
}

/// Every box's extent: an input without a place at (40, 40), a step without
/// one at (0, 0); none for an empty model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

pub fn boxes_bounds(model: &Model) -> Option<Bounds> {
    let boxes: Vec<(Pt, f64, f64)> = model
        .inputs
        .iter()
        .map(|i| (input_at(model, i.name()), canvas::INPUT_W, canvas::INPUT_H))
        .chain(
            model
                .steps
                .iter()
                .map(|s| (step_at(s), canvas::STEP_W, canvas::STEP_H)),
        )
        .collect();
    if boxes.is_empty() {
        return None;
    }
    let x = boxes.iter().map(|b| b.0.x).fold(f64::INFINITY, f64::min);
    let y = boxes.iter().map(|b| b.0.y).fold(f64::INFINITY, f64::min);
    let right = boxes
        .iter()
        .map(|b| b.0.x + b.1)
        .fold(f64::NEG_INFINITY, f64::max);
    let bottom = boxes
        .iter()
        .map(|b| b.0.y + b.2)
        .fold(f64::NEG_INFINITY, f64::max);
    Some(Bounds {
        x,
        y,
        w: right - x,
        h: bottom - y,
    })
}

/// The view that frames every box in a canvas `width` × `height`, centred;
/// it zooms out only, never past 1:1.
pub fn fit_view(bounds: Option<Bounds>, width: f64, height: f64) -> View {
    let Some(b) = bounds.filter(|_| width != 0.0) else {
        return canvas::EMPTY;
    };
    let pad = canvas::FIT_PAD;
    let k = 1.0_f64
        .min((width - pad * 2.0) / b.w.max(1.0))
        .min((height - pad * 2.0) / b.h.max(1.0));
    View {
        k,
        x: (width - b.w * k) / 2.0 - b.x * k,
        y: (height - b.h * k) / 2.0 - b.y * k,
    }
}

/// The lowest scale zooming reaches: `ZOOM_MIN`, or the last fit's scale
/// when that is lower, so a model too big to see whole at `ZOOM_MIN` is
/// seen whole after fitting and the first wheel step does not jump.
pub fn zoom_floor(fitted_k: f64) -> f64 {
    canvas::ZOOM_MIN.min(fitted_k)
}

/// The view zoomed by `f` about a canvas point, which stays over the same
/// world point; the scale stays between `floor` and `ZOOM_MAX`.
pub fn zoom_at(view: View, p: Pt, f: f64, floor: f64) -> View {
    let k = canvas::ZOOM_MAX.min(floor.max(view.k * f));
    let wx = (p.x - view.x) / view.k;
    let wy = (p.y - view.y) / view.k;
    View {
        k,
        x: p.x - wx * k,
        y: p.y - wy * k,
    }
}
