//! DXF dışa aktar (the web's `ui/io/DxfExportDialog.ts`): the objects of the
//! selection, of the visible layers or of the whole drawing, with their
//! layers, as an AutoCAD 2007 DXF (UTF-8, metres; the shared writer, off the
//! UI thread). Coordinates are written as the shortest decimal that reads
//! back to the same float64 (CLAUDE.md §23). What DXF cannot hold (labels,
//! attributes, symbols, theme colours, holes) rides along as KentOS data
//! that KentOS reads back; what changes on the way is said before writing
//! (the summary) and after (the writer's report, in the command line). An
//! export is not a save: the drawing stays unsaved if it was.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use iced::widget::{Column, column, row};
use iced::{Center, Element, Fill, Length, Task};
use kentos_contracts::{
    AngleUnit, DxfWriteInput, DxfWriteLayer, Entity, ExportReport, LayerNodeType,
};
use kentos_interaction::{Format, Level};
use kentos_ui::label;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::table::{Column as TableColumn, Row as TableRow, Table};
use kentos_ui::widget::tree_view::{Check, check_box};
use kentos_ui::widget::{Dialog, overlay, swatch};

use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message, off_thread};
use crate::app::{App, Message};
use crate::view::hex_color;

/// What an export writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Selection,
    Visible,
    All,
}

impl Scope {
    pub const ALL: [Scope; 3] = [Scope::Selection, Scope::Visible, Scope::All];

    fn label(self) -> &'static str {
        match self {
            Scope::Selection => "Seçili",
            Scope::Visible => "Görünen katmanlar",
            Scope::All => "Tümü",
        }
    }
}

/// A scope with its count, as the segmented control shows it: “Seçili (3)”.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counted(pub Scope, pub usize);

impl fmt::Display for Counted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.0.label(), self.1)
    }
}

/// Theme colours DXF has no colour for (ink is DXF colour 7 exactly).
const THEMED: [&str; 3] = ["fg", "fg-dim", "paper"];

#[derive(Debug, Clone)]
pub struct State {
    scope: Scope,
    /// Layers the user left out.
    excluded: BTreeSet<String>,
    writing: bool,
    status: Option<(bool, String)>,
    /// What the writer said and how many layers it wrote: logged once the file is written.
    written: Option<(ExportReport, usize)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Scope(Scope),
    Toggle(String),
    ToggleAll,
    Run,
    Bytes(Vec<u8>, ExportReport, usize),
}

fn event(e: Event) -> Message {
    message(Exchange::DxfExport(e))
}

impl State {
    pub fn new(app: &App) -> Self {
        Self {
            scope: if app.selection.is_empty() {
                Scope::Visible
            } else {
                Scope::Selection
            },
            excluded: BTreeSet::new(),
            writing: false,
            status: None,
            written: None,
        }
    }
}

impl App {
    /// The scope's objects (web `scopeEntities`).
    pub(super) fn scope_entities(&self, scope: Scope) -> Vec<&Entity> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        match scope {
            Scope::Selection => self
                .selection
                .ids()
                .iter()
                .filter_map(|s| doc.model.get(*s))
                .collect(),
            Scope::All => doc.model.entities().collect(),
            Scope::Visible => doc
                .model
                .entities()
                .filter(|e| doc.model.layers().is_visible(&e.base().layer_id))
                .collect(),
        }
    }

    /// The scope's objects by layer: the tree's order, then layers the tree
    /// does not have (their objects go to DXF layer 0), empty ones left out.
    pub(super) fn by_layer(&self, scope: Scope) -> Vec<(String, Vec<&Entity>)> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let mut out: Vec<(String, Vec<&Entity>)> = doc
            .model
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
        for e in self.scope_entities(scope) {
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

    /// The objects that will be written: the scope's, less the layers left out.
    fn chosen<'a>(&'a self, s: &State) -> Vec<(String, Vec<&'a Entity>)> {
        let mut all = self.by_layer(s.scope);
        all.retain(|(id, _)| !s.excluded.contains(id));
        all
    }

    pub(super) fn dxf_export_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Run => return self.dxf_export_run(),
            Event::Bytes(bytes, report, layers) => {
                if let Some(Window::DxfExport(s)) = &mut self.exchange {
                    s.written = Some((report, layers));
                    let name = self.export_name(".dxf");
                    return self.save_export(bytes, name, ("AutoCAD DXF", "dxf"));
                }
                return Task::none();
            }
            _ => {}
        }
        let ids: Vec<String> = match &self.exchange {
            Some(Window::DxfExport(s)) => self
                .by_layer(s.scope)
                .into_iter()
                .map(|(id, _)| id)
                .collect(),
            _ => return Task::none(),
        };
        let Some(Window::DxfExport(s)) = &mut self.exchange else {
            return Task::none();
        };
        match e {
            Event::Scope(scope) => s.scope = scope,
            Event::Toggle(id) => {
                if !s.excluded.remove(&id) {
                    s.excluded.insert(id);
                }
            }
            Event::ToggleAll => {
                if ids.iter().any(|id| s.excluded.contains(id)) {
                    for id in &ids {
                        s.excluded.remove(id);
                    }
                } else {
                    s.excluded.extend(ids);
                }
            }
            Event::Run | Event::Bytes(..) => {}
        }
        Task::none()
    }

    /// A layer as the writer receives it (the groups above it, outermost first).
    fn write_layer(&self, id: &str) -> Option<DxfWriteLayer> {
        let doc = self.document.as_ref()?;
        let layers = doc.model.layers();
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
            id: id.to_owned(),
            name: l.name.clone(),
            path,
            color: l.style.color.clone(),
            visible: layers.is_visible(id),
            locked: layers.is_locked(id),
            line_type: l.style.line_type,
            line_weight: l.style.line_weight,
        })
    }

    fn dxf_export_run(&mut self) -> Task<Message> {
        let (Some(Window::DxfExport(s)), Some(doc)) = (&self.exchange, &self.document) else {
            return Task::none();
        };
        if s.writing {
            return Task::none();
        }
        let chosen = self.chosen(s);
        let entities: Vec<Entity> = chosen
            .iter()
            .flat_map(|(_, list)| list.iter().map(|e| (*e).clone()))
            .collect();
        if entities.is_empty() {
            return Task::none();
        }
        let settings = doc.settings();
        let format = Format::of(settings);
        // What a dimension without its own text shows (project units): its
        // block draws it, the dimension keeps no text.
        let mut dimension_values = BTreeMap::new();
        for e in &entities {
            if let Entity::Dimension(d) = e
                && d.text.as_deref().is_none_or(str::is_empty)
                && let Some(l) = kentos_formats::dxf::dimension_layout(d)
            {
                dimension_values.insert(
                    d.base.id,
                    dimension_text(&format, l.prefix, l.unit, l.value),
                );
            }
        }
        let layers: Vec<DxfWriteLayer> = chosen
            .iter()
            .filter_map(|(id, _)| self.write_layer(id))
            .collect();
        let count = layers.len();
        let input = DxfWriteInput {
            entities,
            layers,
            scale: settings.plot_scale,
            length_decimals: settings.length_decimals,
            grads: settings.angle_unit == AngleUnit::Grad,
            dimension_values,
        };
        if let Some(Window::DxfExport(s)) = &mut self.exchange {
            s.writing = true;
            s.status = Some((false, "Yazılıyor…".to_owned()));
        }
        off_thread(
            move || kentos_formats::dxf::write(&input),
            move |(bytes, report)| Exchange::DxfExport(Event::Bytes(bytes, report, count)),
        )
    }

    pub(super) fn dxf_export_written(
        &mut self,
        outcome: Option<Result<String, String>>,
    ) -> Task<Message> {
        let Some(Window::DxfExport(s)) = &mut self.exchange else {
            return Task::none();
        };
        s.writing = false;
        match outcome {
            None => s.status = None,
            Some(Err(e)) => s.status = Some((true, format!("Yazılamadı: {e}"))),
            Some(Ok(name)) => {
                let (report, layers) = s.written.take().unwrap_or_default();
                let written: u32 = report.counts.values().sum();
                self.say(
                    Level::Success,
                    format!(
                        "“{name}” yazıldı: {written} nesne, {layers} katman (AutoCAD 2007 DXF)."
                    ),
                );
                for item in &report.notes {
                    self.output(words::report_text(item));
                }
                if !report.skipped.is_empty() {
                    let skipped: Vec<String> =
                        report.skipped.iter().map(words::report_text).collect();
                    self.warn(format!(
                        "“{name}” içine yazılmayanlar: {}",
                        skipped.join(" ")
                    ));
                }
                self.close_exchange();
            }
        }
        Task::none()
    }

    pub(super) fn dxf_export_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let counts: Vec<Counted> = Scope::ALL
            .iter()
            .map(|scope| Counted(*scope, self.scope_entities(*scope).len()))
            .collect();
        let current = counts
            .iter()
            .copied()
            .find(|c| c.0 == s.scope)
            .unwrap_or(Counted(s.scope, 0));
        let scope = Segmented::new_with(
            counts.clone(),
            current,
            |c| event(Event::Scope(c.0)),
            |c| c.1 > 0,
        );
        let chosen = self.chosen(s);
        let any = chosen.iter().any(|(_, list)| !list.is_empty());
        let mut body = Column::new()
            .spacing(12)
            .push(words::field(
                "Yazılacak nesneler",
                scope,
                Some("AutoCAD 2007 DXF (UTF-8, birim metre); koordinatlar yuvarlanmadan, tam yazılır.".to_owned()),
            ))
            .push(self.dxf_export_layers(s))
            .push(self.dxf_export_summary(&chosen));
        if let Some((error, words)) = &s.status {
            body = body.push(words::text_line(
                if *error { Line::Error } else { Line::Info },
                words.clone(),
            ));
        }
        overlay::blocking(
            Dialog::new("DXF dışa aktar")
                // As tall as its content; a long body (a drawing with many layers) scrolls, the buttons stay in view.
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary(
                    "Dışa aktar…",
                    (any && !s.writing).then(|| event(Event::Run)),
                ))
                .width(820.0)
                .max_height(780.0),
        )
    }

    fn dxf_export_layers<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let all = self.by_layer(s.scope);
        if all.is_empty() {
            return words::empty("Bu kapsamda nesne yok.");
        }
        let Some(doc) = &self.document else {
            return words::empty("Bu kapsamda nesne yok.");
        };
        let layers = doc.model.layers();
        let out = all.iter().filter(|(id, _)| s.excluded.contains(id)).count();
        let every = match out {
            0 => Check::Checked,
            n if n == all.len() => Check::Unchecked,
            _ => Check::Mixed,
        };
        let head = row![
            check_box(every, Some(event(Event::ToggleAll))),
            label::caption(format!("Bütün katmanlar ({})", all.len())),
        ]
        .spacing(8)
        .align_y(Center);
        let rows = all.iter().map(|(id, list)| {
            let layer = layers.get(id);
            let name = match layer {
                Some(_) => self.layer_path(id),
                None => "Katmanı olmayan nesneler".to_owned(),
            };
            let state = match layer {
                None => "DXF katmanı 0 olarak".to_owned(),
                Some(_) => [
                    (!layers.is_visible(id)).then_some("gizli: DXF'te kapalı"),
                    layers.is_locked(id).then_some("kilitli"),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(", "),
            };
            let checked = !s.excluded.contains(id);
            let name_cell: Element<'a, Message> = match layer {
                Some(l) => row![swatch(hex_color(&l.style.color)), label::body(name)]
                    .spacing(6)
                    .align_y(Center)
                    .into(),
                None => label::body(name).into(),
            };
            TableRow::new([
                check_box(
                    if checked {
                        Check::Checked
                    } else {
                        Check::Unchecked
                    },
                    Some(event(Event::Toggle(id.clone()))),
                ),
                name_cell,
                label::body(list.len().to_string()).into(),
                label::caption(state).into(),
            ])
        });
        let table = words::fitted(
            Table::new([
                TableColumn::new("").width(22),
                TableColumn::new("Katman").width(Length::FillPortion(3)),
                TableColumn::new("Nesne").width(60).align_right(),
                TableColumn::new("DXF'te").width(Length::FillPortion(2)),
            ])
            .extend(rows),
            all.len(),
        );
        column![head, table].spacing(6).width(Fill).into()
    }

    fn dxf_export_summary<'a>(
        &'a self,
        chosen: &[(String, Vec<&'a Entity>)],
    ) -> Element<'a, Message> {
        let Some(doc) = &self.document else {
            return words::summary(Vec::new());
        };
        let layers = doc.model.layers();
        let list: Vec<&Entity> = chosen.iter().flat_map(|(_, l)| l.iter().copied()).collect();
        let mut kinds = Vec::new();
        for e in &list {
            words::count(&mut kinds, e.kind());
        }
        let nodes: Vec<_> = chosen.iter().filter_map(|(id, _)| layers.get(id)).collect();
        let dimensions = kinds
            .iter()
            .find(|(k, _)| *k == "dimension")
            .map_or(0, |(_, n)| *n);
        let islands = list
            .iter()
            .filter(|e| matches!(e, Entity::Polygon(p) if p.holes.as_ref().is_some_and(|h| !h.is_empty())))
            .count();
        let data = list.iter().any(|e| {
            let b = e.base();
            b.label.is_some() || b.symbol.is_some() || !b.attrs.is_empty()
        });
        let themed = nodes
            .iter()
            .any(|l| THEMED.contains(&l.style.color.as_str()))
            || list.iter().any(|e| {
                e.base()
                    .color
                    .as_deref()
                    .is_some_and(|c| THEMED.contains(&c))
            });
        let styled = nodes
            .iter()
            .filter(|l| l.style.renderer.is_some() || l.style.fill.is_some())
            .count();
        let hidden = nodes.iter().filter(|l| !layers.is_visible(&l.id)).count();
        let mut names: BTreeMap<String, usize> = BTreeMap::new();
        for l in &nodes {
            *names.entry(l.name.trim().to_uppercase()).or_default() += 1;
        }
        let repeated = names.values().any(|n| *n > 1);
        let mut lines = vec![if list.is_empty() {
            words::text_line(
                Line::Warn,
                "Yazılacak nesne yok. Başka bir kapsam ya da en az bir katman seçin.",
            )
        } else {
            words::text_line(
                Line::Ok,
                format!(
                    "{} nesne {} katmanla yazılacak: {}.",
                    list.len(),
                    chosen.len(),
                    words::kind_counts(&kinds)
                ),
            )
        }];
        if dimensions > 0 {
            lines.push(words::text_line(Line::Info, format!("{dimensions} ölçü DXF ölçüsü olarak yazılır ve KentOS'taki gibi görünür; başka bir program ölçüyü düzenlerse kendi kurallarıyla yeniden çizer. KentOS'a ölçü olarak geri okunur.")));
        }
        if islands > 0 {
            lines.push(words::text_line(Line::Info, format!("{islands} adalı alanın adaları ayrı kapalı çoklu çizgiler olarak yazılır; KentOS'a geri okununca yine adalı alan olur.")));
        }
        if data {
            lines.push(words::text_line(Line::Info, "Etiketler, öznitelikler ve semboller nesnelerle birlikte KentOS verisi olarak yazılır: başka programlar göstermez, KentOS geri okur."));
        }
        if themed {
            lines.push(words::text_line(Line::Info, "Tema renkleri DXF'te sabit renk olur (ana mürekkep 7, ikincil 8); KentOS'a geri okununca yine tema rengidir."));
        }
        if styled > 0 {
            lines.push(words::text_line(Line::Info, format!("{styled} katmanın stili (semboller, dolgular) DXF'e yazılmaz; rengi, çizgi tipi ve kalınlığı yazılır.")));
        }
        if repeated {
            lines.push(words::text_line(Line::Info, "Aynı adlı katmanlar DXF'te grup adlarıyla ayrılır (“Grup - Katman”); DXF katmanları düz bir listedir."));
        }
        if hidden > 0 {
            lines.push(words::text_line(
                Line::Info,
                format!("{hidden} gizli katman DXF'te kapalı yazılır."),
            ));
        }
        words::summary(lines)
    }
}

/// A dimension's measured value as drawn: prefix and value in project units,
/// a length without its unit (the web's `dimensionText`).
pub fn dimension_text(format: &Format, prefix: &str, unit: &str, value: f64) -> String {
    format.dimension(prefix, unit, value)
}
