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
use kentos_commands::blocks::placed_blocks;
use kentos_commands::{DxfAnnotations, DxfForm, Entity, ExportReport};
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
    /// How tables, dimensions and leaders are written (docs/adr/0327 §6).
    annotations: DxfAnnotations,
}

/// A dimension's or a leader's form as the window names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Form(pub DxfForm);

impl fmt::Display for Form {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            DxfForm::Native => "Yerel nesne",
            DxfForm::Block => "Blok görünüşü",
        })
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Scope(Scope),
    Toggle(String),
    ToggleAll,
    Run,
    Bytes(Vec<u8>, ExportReport, usize),
    Dimensions(Form),
    Leaders(Form),
    Metadata,
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
            annotations: DxfAnnotations::default(),
        }
    }
}

/// An area with a hole in any of its parts (docs/adr/0143; the web's `hasIslands`).
fn islanded(e: &Entity) -> bool {
    let some =
        |h: &Option<Vec<kentos_commands::RingGeometry>>| h.as_ref().is_some_and(|h| !h.is_empty());
    matches!(e, Entity::Polygon(p) if some(&p.holes) || p.parts.iter().flatten().any(|q| some(&q.holes)))
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
        kentos_headless::export::by_layer(&doc.model, self.scope_entities(scope))
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
            Event::Dimensions(f) => s.annotations.dimensions = f.0,
            Event::Leaders(f) => s.annotations.leaders = f.0,
            Event::Metadata => s.annotations.metadata = !s.annotations.metadata,
            Event::Run | Event::Bytes(..) => {}
        }
        Task::none()
    }

    fn dxf_export_run(&mut self) -> Task<Message> {
        let (Some(Window::DxfExport(s)), Some(doc)) = (&self.exchange, &self.document) else {
            return Task::none();
        };
        if s.writing {
            return Task::none();
        }
        let chosen = self.chosen(s);
        if chosen.is_empty() {
            return Task::none();
        }
        // The headless host's builder: a script and the window write the same file (docs/adr/0252).
        let mut input = kentos_headless::export::dxf_input(&doc.model, &chosen);
        input.annotations = s.annotations;
        let count = input.layers.len();
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
            .push(dxf_export_forms(s))
            .push(self.dxf_export_summary(&chosen, &s.annotations));
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
        forms: &DxfAnnotations,
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
        // Semt and Eğim have no DXF kind: aligned dimensions with their own block (docs/adr/0147 §8).
        let without_kind = list
            .iter()
            .filter(|e| {
                matches!(e, kentos_commands::Entity::Dimension(d) if matches!(
                    d.style,
                    Some(kentos_commands::DimensionStyle::Azimuth | kentos_commands::DimensionStyle::Slope)
                ))
            })
            .count();
        let islands = list.iter().filter(|e| islanded(e)).count();
        let data = list.iter().any(|e| {
            let b = e.base();
            b.label.is_some() || b.symbol.is_some() || !b.attrs.is_empty()
        });
        // A text's mask is KentOS's alone: DXF's TEXT has none (docs/adr/0145 §7).
        let masked = list
            .iter()
            .filter(|e| matches!(e, kentos_commands::Entity::Text(t) if t.mask))
            .count();
        // A leader is a LEADER and its note's MTEXT; its open and dot arrowheads KentOS's data (docs/adr/0146 §8).
        let leaders: Vec<&kentos_commands::LeaderEntity> = list
            .iter()
            .filter_map(|e| match e {
                kentos_commands::Entity::Leader(l) => Some(l),
                _ => None,
            })
            .collect();
        let own_arrows = leaders
            .iter()
            .filter(|l| {
                matches!(
                    l.arrow,
                    Some(kentos_commands::LeaderArrow::Open | kentos_commands::LeaderArrow::Dot)
                )
            })
            .count();
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
            // What comes back is the form and the data written (docs/adr/0327 §6).
            let line = match (forms.dimensions, forms.metadata) {
                (DxfForm::Native, true) => format!("{dimensions} ölçü DXF ölçüsü olarak yazılır ve KentOS'taki gibi görünür; başka bir program ölçüyü düzenlerse kendi kurallarıyla yeniden çizer. KentOS'a ölçü olarak geri okunur."),
                (DxfForm::Native, false) => format!("{dimensions} ölçü DXF ölçüsü olarak yazılır; KentOS verisi yazılmadığından KentOS'a DXF'in taşıdığı kadarıyla geri okunur: bağları, stilden ayrılanları ve seçenekleri düşer."),
                (DxfForm::Block, true) => format!("{dimensions} ölçü KentOS'un çizdiği gibi adsız blok (*U) olarak yazılır: her programda aynı görünür, düzenlenemez. KentOS'a ölçü olarak geri okunur."),
                (DxfForm::Block, false) => format!("{dimensions} ölçü KentOS'un çizdiği gibi adsız blok (*U) olarak yazılır: her programda aynı görünür, düzenlenemez. KentOS verisi yazılmadığından KentOS'a blok olarak geri okunur."),
            };
            lines.push(words::text_line(Line::Info, line));
        }
        if without_kind > 0 {
            lines.push(words::text_line(Line::Info, format!("{without_kind} semt ve eğim ölçüsünün DXF'te karşılığı yoktur: hizalı ölçü olarak, kendi çizgileri ve değeriyle yazılır; başka programlar çizgilerini gösterir, KentOS ölçü olarak geri okur.")));
        }
        let inserts = kinds
            .iter()
            .find(|(k, _)| *k == "insert")
            .map_or(0, |(_, n)| *n);
        if inserts > 0 {
            let blocks = placed_blocks(doc.model.blocks(), list.iter().copied());
            // A block with attribute definitions goes with ATTDEFs, its inserts with ATTRIBs (docs/adr/0144 §7).
            let attributed = list.iter().any(|e| match e {
                kentos_commands::Entity::Insert(i) => doc
                    .model
                    .block(i.block)
                    .is_some_and(|b| !b.attributes.is_empty()),
                _ => false,
            });
            let how = if attributed {
                ", öznitelikleri ATTDEF ve ATTRIB olarak"
            } else {
                ""
            };
            lines.push(words::text_line(Line::Info, format!("{inserts} blok yerleştirmesi INSERT, yerleştirdikleri {blocks} blok içlerindekilerle birlikte BLOCK olarak yazılır{how}; KentOS'a blok olarak geri okunur.")));
        }
        if islands > 0 {
            lines.push(words::text_line(Line::Info, format!("{islands} adalı alanın adaları ayrı kapalı çoklu çizgiler olarak yazılır; KentOS'a geri okununca yine adalı alan olur.")));
        }
        if data {
            lines.push(words::text_line(Line::Info, "Etiketler, öznitelikler ve semboller nesnelerle birlikte KentOS verisi olarak yazılır: başka programlar göstermez, KentOS geri okur."));
        }
        if masked > 0 {
            lines.push(words::text_line(Line::Info, format!("{masked} yazının zemini KentOS verisi olarak yazılır: DXF yazısında zemin yoktur, başka programlar göstermez; KentOS geri okur.")));
        }
        // A linked text is a plain text in DXF (docs/adr/0175 §4).
        let linked = list
            .iter()
            .filter(|e| matches!(e, kentos_commands::Entity::Text(t) if t.label_of.is_some()))
            .count();
        if linked > 0 {
            lines.push(words::text_line(Line::Info, format!("{linked} bağlı yazı düz yazı olarak yazılır: nesnesine bağı DXF'e geçmez, geri okununca kendi başına bir yazıdır.")));
        }
        if !leaders.is_empty() {
            let arrows = if own_arrows > 0 {
                format!(
                    " {own_arrows} kılavuzun açık ya da nokta oku KentOS verisidir: başka programlar dolu ok gösterir."
                )
            } else {
                String::new()
            };
            let n = leaders.len();
            let line = match (forms.leaders, forms.metadata) {
                (DxfForm::Native, true) => format!("{n} kılavuz LEADER olarak, notları da onlara bağlı MTEXT'ler olarak yazılır; KentOS'a kılavuz olarak geri okunur.{arrows}"),
                (DxfForm::Native, false) => format!("{n} kılavuz LEADER olarak, notları da onlara bağlı MTEXT'ler olarak yazılır; KentOS verisi yazılmadığından kolları, uç bağları ve stili düşer.{arrows}"),
                (DxfForm::Block, true) => format!("{n} kılavuz KentOS'un çizdiği gibi adsız blok (*U) olarak yazılır: her programda aynı görünür, düzenlenemez. KentOS'a kılavuz olarak geri okunur."),
                (DxfForm::Block, false) => format!("{n} kılavuz KentOS'un çizdiği gibi adsız blok (*U) olarak yazılır: her programda aynı görünür, düzenlenemez. KentOS verisi yazılmadığından KentOS'a blok olarak geri okunur."),
            };
            lines.push(words::text_line(Line::Info, line));
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

/// A dimension's measured value as drawn: its look's prefix and suffix, its
/// kind's prefix, the value in its look's or the project's unit and decimals,
/// a length without its unit (the web's `dimensionText`, docs/adr/0183 §3).
pub fn dimension_text(
    format: &Format,
    prefix: &str,
    unit: &str,
    value: f64,
    look: &kentos_commands::DimensionLook,
) -> String {
    format.dimension_in(prefix, unit, value, look)
}

/// Açıklamalar (docs/adr/0327 §6): what dimensions and leaders become, and KentOS's data on them and on tables.
fn dxf_export_forms(s: &State) -> Element<'_, Message> {
    let forms = [Form(DxfForm::Native), Form(DxfForm::Block)];
    let choice = |now: DxfForm, on: fn(Form) -> Event| {
        Segmented::new_with(forms.to_vec(), Form(now), move |f| event(on(f)), |_| true)
    };
    let line = |name: &'static str, el: Element<'static, Message>| -> Element<'static, Message> {
        row![
            iced::widget::container(label::body(name)).width(Length::Fixed(110.0)),
            el
        ]
        .spacing(8)
        .align_y(Center)
        .into()
    };
    let a = s.annotations;
    let body = column![
        line("Ölçüler", choice(a.dimensions, Event::Dimensions).into()),
        line("Kılavuzlar", choice(a.leaders, Event::Leaders).into()),
        words::check(
            a.metadata,
            "KentOS verisini yaz: tablo, ölçü ve kılavuz KentOS'a bütün alanlarıyla geri gelir",
            Some(event(Event::Metadata)),
        ),
    ]
    .spacing(8);
    words::field(
        "Açıklamalar",
        body,
        Some(
            "Yerel nesne: başka programlar ölçü ya da kılavuz olarak düzenler. Blok görünüşü: KentOS'un çizdiği gibi görünür, düzenlenemez. \
             Tablolar her zaman blok görünüşüyle yazılır (DXF'in ACAD_TABLE'ı yazılmaz). Yazılanlar türleriyle raporda söylenir."
                .to_owned(),
        ),
    )
}

#[cfg(test)]
mod tests {
    use kentos_commands::Entity;

    /// An area whose only hole is in a later part has islands (docs/adr/0143).
    #[test]
    fn a_hole_in_any_part_is_an_island() {
        let area = |parts: &str| -> Entity {
            serde_json::from_str(&format!(
                r#"{{"kind":"polygon","id":1,"layerId":"a","attrs":{{}},"pts":[{{"x":0,"y":0}},{{"x":9,"y":0}},{{"x":9,"y":9}}]{parts}}}"#
            ))
            .expect("an area")
        };
        assert!(!super::islanded(&area("")));
        let hole = r#""holes":[{"pts":[{"x":21,"y":1},{"x":22,"y":1},{"x":22,"y":2}]}]"#;
        let part = format!(
            r#","parts":[{{"pts":[{{"x":20,"y":0}},{{"x":29,"y":0}},{{"x":29,"y":9}}],{hole}}}]"#
        );
        assert!(super::islanded(&area(&part)));
    }
}
