//! Detay paftaları (`sheet.details`, docs/adr/1112; plan/cad-advanced.md CAD-ADV-16): the general 2D detail set of
//! `cad.sheets.details` built in one window — the details' list, each detail's model scope taken from the selection
//! or a model table's part, an exact paper:model ratio, its own layer state, frozen layers and annotation
//! representation, the camera locked; the paper preview and the plan's placing follow the form (nothing written),
//! Paftaları kur writes the whole set as one change of the book (one undo), Esc or Kapat writes nothing.
//!
//! ```text
//! ┌ Detay paftaları ───────────────────────────────────────────────────────────────────── × ┐
//! │ Detaylar ─ # Ad        Kaynak          Oran    ×     │ ┌ paper preview ─────────────────┐ │
//! │            1 Bulon     0,30 × 0,15 m   1:1     ×     │ │ [D-501] ▭ ▭  [D-502] ▭          │ │
//! │  [Seçimden ekle] [Tablo parçası ekle]                │ └────────────────────────────────┘ │
//! │ Seçili detay ─ Katmanlar, Dondurulan, Açıklama, Kilit│ [pafta] [detay] [ölçek]            │
//! │ Set ─ İlk numara, Başlık                             │ Detay · Pafta · Oran · Kâğıtta     │
//! │                                               [Kapat] [■ Paftaları kur]                  │
//! └──────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, Text};
use iced::widget::{Canvas, button, row};
use iced::{Center, Element, Fill, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{
    CommandResult, DetailRequest, DetailTablePart, PaperScaleRatio, SheetsDetailed, SheetsDetails,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, sheets_details};
use kentos_ui::theme::Tokens;
use kentos_ui::widget::{Banner, Form, Switch};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::road_dialog::{self, RoadDialog, col, num};

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Detay paftaları";
pub(crate) const BUILD: &str = "Paftaları kur";
pub(crate) const FROM_SELECTION: &str = "Seçimden ekle";
pub(crate) const FROM_TABLE: &str = "Tablo parçası ekle";
/// The margin around the selection's box, of its longer side (Seçime sığdır's 5%, docs/adr/1112 §2).
pub(crate) const MARGIN: f64 = 0.05;

/// The exact ratios offered, paper:model (docs/adr/1090 §2): the sheet core's standard ones.
pub(crate) fn ratios() -> Vec<(f64, f64)> {
    kentos_sheet::viewport::standard_ratios()
        .iter()
        .map(|r| (r.paper, r.model))
        .collect()
}

/// A ratio's words: `1:20`, `2:1`, `1:37,5`.
pub(crate) fn ratio_words(paper: f64, model: f64) -> String {
    let n = |v: f64| {
        if v.fract() == 0.0 {
            format!("{v:.0}")
        } else {
            v.to_string().replace('.', ",")
        }
    };
    format!("{}:{}", n(paper), n(model))
}

/// Where a detail's model comes from.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Scope {
    /// A model box, canonical metres.
    Bounds([f64; 4]),
    /// A model table's part, from 0; its parts.
    Table { table: String, part: u32, parts: u32 },
}

/// One detail of the set as the window keeps it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Detail {
    pub(crate) name: String,
    pub(crate) scope: Scope,
    /// Its ratio among [`ratios`].
    pub(crate) ratio: usize,
    /// The layer state it draws; none: the drawing's layers.
    pub(crate) state: Option<String>,
    /// The layers frozen in its viewport only, by id.
    pub(crate) frozen: Vec<String>,
    /// Its annotation representation: a fixed scale's denominator; none: the viewport's own scale.
    pub(crate) annotation: Option<f64>,
    pub(crate) locked: bool,
}

impl Detail {
    pub(crate) fn request(&self) -> DetailRequest {
        let (paper, model) = ratios().get(self.ratio).copied().unwrap_or((1.0, 1.0));
        let (bounds, table) = match &self.scope {
            Scope::Bounds(b) => (Some(*b), None),
            Scope::Table { table, part, .. } => (
                None,
                Some(DetailTablePart {
                    table: table.clone(),
                    part: *part,
                }),
            ),
        };
        DetailRequest {
            name: self.name.clone(),
            bounds,
            table,
            ratio: PaperScaleRatio { paper, model },
            layers: self
                .state
                .as_ref()
                .map(|name| serde_json::json!({ "type": "theme", "name": name })),
            layer_overrides: (!self.frozen.is_empty()).then(|| {
                self.frozen
                    .iter()
                    .map(|l| serde_json::json!({ "layer": l, "frozen": true }))
                    .collect()
            }),
            annotation: self
                .annotation
                .map(|s| serde_json::json!({ "type": "fixed", "scale": s })),
            locked: (!self.locked).then_some(false),
        }
    }

    fn source_words(&self) -> String {
        match &self.scope {
            Scope::Bounds(b) => format!(
                "{} × {} m",
                road_dialog::grouped(b[2] - b[0], 2),
                road_dialog::grouped(b[3] - b[1], 2)
            ),
            Scope::Table { part, parts, .. } => format!("Tablo, parça {}/{parts}", part + 1),
        }
    }
}

/// The window's state, kept while the program runs.
#[derive(Debug, Clone)]
pub struct DetailSet {
    pub(crate) details: Vec<Detail>,
    pub(crate) chosen: Option<usize>,
    pub(crate) first: String,
    pub(crate) title: String,
    pub(crate) result: Option<Result<SheetsDetailed, String>>,
    pub(crate) warnings: Vec<String>,
}

impl Default for DetailSet {
    fn default() -> Self {
        DetailSet {
            details: Vec::new(),
            chosen: None,
            first: "D-501".into(),
            title: "Detaylar".into(),
            result: None,
            warnings: Vec::new(),
        }
    }
}

impl DetailSet {
    pub(crate) fn input(&self) -> SheetsDetails {
        SheetsDetails {
            details: self.details.iter().map(Detail::request).collect(),
            source: None,
            options: serde_json::json!({}),
            first: Some(self.first.trim().to_owned()).filter(|f| !f.is_empty()),
            title: Some(self.title.trim().to_owned()).filter(|t| !t.is_empty()),
            expected_revision: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    FromSelection,
    FromTable,
    Choose(usize),
    Name(usize, String),
    Ratio(usize, usize),
    Part(usize),
    Remove(usize),
    State(usize),
    Freeze(usize),
    Thaw(String),
    Annotation(usize),
    Locked(bool),
    First(String),
    Title(String),
    Build,
    Close,
}

fn msg(e: Event) -> Message {
    Message::SheetDetails(e)
}

/// The paper preview: each sheet the plan makes side by side, each detail's frame on it with its name.
struct Paper {
    /// Each sheet's size, mm, and its number.
    sheets: Vec<(f64, f64, String)>,
    /// Each detail: its sheet's index, its frame (mm: left, top, width, height) and its words.
    frames: Vec<(usize, [f64; 4], String)>,
}

impl<M> canvas::Program<M> for Paper {
    type State = ();

    fn draw(&self, _state: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let tokens = Tokens::of(theme);
        if self.sheets.is_empty() {
            return vec![frame.into_geometry()];
        }
        let gap = 20.0;
        let total_w: f64 = self.sheets.iter().map(|s| s.0).sum::<f64>() + gap * (self.sheets.len() - 1) as f64;
        let total_h = self.sheets.iter().map(|s| s.1).fold(0.0, f64::max);
        let pad = 16.0;
        let k = ((f64::from(bounds.width) - 2.0 * pad) / total_w).min((f64::from(bounds.height) - 2.0 * pad - 14.0) / total_h);
        let mut x0 = (f64::from(bounds.width) - total_w * k) / 2.0;
        let y0 = pad + 14.0;
        let mut origins = Vec::new();
        for (w, h, number) in &self.sheets {
            let at = Point::new(x0 as f32, y0 as f32);
            let size = iced::Size::new((w * k) as f32, (h * k) as f32);
            frame.fill_rectangle(at, size, tokens.surface);
            frame.stroke(&Path::rectangle(at, size), Stroke::default().with_color(tokens.muted).with_width(1.0));
            frame.fill_text(Text {
                content: number.clone(),
                position: Point::new(at.x, at.y - 14.0),
                color: tokens.muted,
                size: iced::Pixels(11.0),
                ..Text::default()
            });
            origins.push((x0, y0));
            x0 += (w + gap) * k;
        }
        for (sheet, [l, t, w, h], words) in &self.frames {
            let Some((x, y)) = origins.get(*sheet) else { continue };
            let at = Point::new((x + l * k) as f32, (y + t * k) as f32);
            let size = iced::Size::new((w * k).max(1.0) as f32, (h * k).max(1.0) as f32);
            frame.fill_rectangle(at, size, tokens.accent.scale_alpha(0.16));
            frame.stroke(&Path::rectangle(at, size), Stroke::default().with_color(tokens.accent).with_width(1.0));
            frame.fill_text(Text {
                content: words.clone(),
                position: Point::new(at.x + 3.0, at.y + 2.0),
                color: tokens.text,
                size: iced::Pixels(10.0),
                ..Text::default()
            });
        }
        vec![frame.into_geometry()]
    }
}

impl App {
    /// Detay paftaları (`sheet.details`): the window, with a first detail of the selection when there is one.
    pub(crate) fn open_sheet_details(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        let set = self.sheet_details.take().unwrap_or_default();
        self.sheet_details = Some(set);
        if self.sheet_details.as_ref().is_some_and(|s| s.details.is_empty()) && !self.selection.ids().is_empty() {
            self.sheet_details_add_selection();
        }
        self.sheet_details_preview();
        self.dialog = Some(Dialog::SheetDetails);
    }

    /// The layer states of the drawing, by name, and its layers (not groups) by id and path.
    fn sheet_details_layers(&self) -> (Vec<String>, Vec<(String, String)>) {
        let Some(doc) = self.document.as_ref() else {
            return (Vec::new(), Vec::new());
        };
        let states = doc.model.settings().layer_states.iter().map(|s| s.name.clone()).collect();
        fn walk(nodes: &[kentos_commands::LayerNode], here: &str, out: &mut Vec<(String, String)>) {
            for n in nodes {
                let path = if here.is_empty() { n.name.clone() } else { format!("{here} / {}", n.name) };
                if n.kind == kentos_commands::LayerNodeType::Layer {
                    out.push((n.id.clone(), path.clone()));
                }
                walk(&n.children, &path, out);
            }
        }
        let mut layers = Vec::new();
        walk(doc.model.layers().nodes(), "", &mut layers);
        (states, layers)
    }

    /// A detail of the selection's box at 1:10 (none chosen yet), named after its place in the list.
    fn sheet_details_add_selection(&mut self) {
        let Some(doc) = self.document.as_ref() else { return };
        let ids: Vec<String> = self.selection.ids().iter().filter_map(|s| doc.model.uid(*s)).map(|u| u.to_string()).collect();
        if ids.is_empty() {
            self.warn(format!("{TITLE}: çizimde seçili nesne yok. Detayın kapsamını seçip yeniden “{FROM_SELECTION}”e basın."));
            return;
        }
        let Ok(Some(b)) = kentos_native_application::sheets_fit::object_extent(&doc.model, Some(&ids)) else {
            self.warn(format!("{TITLE}: seçimin kapsamı bulunamadı (boyutu olmayan nesneler)."));
            return;
        };
        // A margin of 5% of the longer side all round, as Seçime sığdır's: a single straight line still has a height.
        let m = MARGIN * (b[2] - b[0]).max(b[3] - b[1]);
        let bounds = [b[0] - m, b[1] - m, b[2] + m, b[3] + m];
        let ten = ratios().iter().position(|r| *r == (1.0, 10.0)).unwrap_or(0);
        let Some(set) = self.sheet_details.as_mut() else { return };
        let n = set.details.len() + 1;
        set.details.push(Detail {
            name: format!("Detay {n}"),
            scope: Scope::Bounds(bounds),
            ratio: ten,
            state: None,
            frozen: Vec::new(),
            annotation: None,
            locked: true,
        });
        set.chosen = Some(set.details.len() - 1);
    }

    /// One detail per part of the selected model table (docs/adr/1090 §3), at 1:20.
    fn sheet_details_add_table(&mut self) {
        let Some(doc) = self.document.as_ref() else { return };
        let table = self.selection.ids().iter().find_map(|s| {
            let uid = doc.model.uid(*s)?.to_string();
            sheets_details::table_parts(&doc.model, &uid).map(|n| (uid, n))
        });
        let Some((uid, parts)) = table else {
            self.warn(format!("{TITLE}: seçimde model tablosu yok. Tabloyu seçip yeniden “{FROM_TABLE}”e basın."));
            return;
        };
        let twenty = ratios().iter().position(|r| *r == (1.0, 20.0)).unwrap_or(0);
        let Some(set) = self.sheet_details.as_mut() else { return };
        for part in 0..parts {
            set.details.push(Detail {
                name: if parts > 1 { format!("Tablo {}/{parts}", part + 1) } else { "Tablo".into() },
                scope: Scope::Table { table: uid.clone(), part, parts },
                ratio: twenty,
                state: None,
                frozen: Vec::new(),
                annotation: None,
                locked: true,
            });
        }
        set.chosen = Some(set.details.len() - 1);
    }

    /// The plan of `cad.sheets.details` for the window's details: where each detail goes (nothing written).
    pub(crate) fn sheet_details_preview(&mut self) {
        let (Some(doc), Some(set)) = (self.document.as_mut(), self.sheet_details.as_mut()) else {
            return;
        };
        set.warnings.clear();
        if set.details.is_empty() {
            set.result = None;
            return;
        }
        let input = set.input();
        set.result = Some(match sheets_details::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, warnings } => {
                set.warnings = warnings.into_iter().map(|w| w.message).collect();
                Ok(output)
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            _ => Err("Yapılamadı.".into()),
        });
    }

    pub(crate) fn sheet_details_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::FromSelection => self.sheet_details_add_selection(),
            Event::FromTable => self.sheet_details_add_table(),
            Event::Build => return self.sheet_details_build(),
            Event::Close => {
                self.dialog = None;
                return Task::none();
            }
            other => {
                let (states, layers) = self.sheet_details_layers();
                let Some(set) = self.sheet_details.as_mut() else { return Task::none() };
                let chosen = set.chosen.filter(|k| *k < set.details.len());
                match other {
                    Event::Choose(k) => set.chosen = Some(k),
                    Event::Name(k, t) => {
                        if let Some(d) = set.details.get_mut(k) {
                            d.name = t;
                        }
                    }
                    Event::Ratio(k, r) => {
                        if let Some(d) = set.details.get_mut(k) {
                            d.ratio = r;
                        }
                    }
                    Event::Remove(k) => {
                        if k < set.details.len() {
                            set.details.remove(k);
                        }
                        set.chosen = (!set.details.is_empty()).then(|| k.min(set.details.len() - 1));
                    }
                    Event::Part(p) => {
                        if let Some(Detail { scope: Scope::Table { part, parts, .. }, .. }) = chosen.and_then(|k| set.details.get_mut(k)) {
                            *part = u32::try_from(p).unwrap_or(0).min(parts.saturating_sub(1));
                        }
                    }
                    Event::State(i) => {
                        if let Some(d) = chosen.and_then(|k| set.details.get_mut(k)) {
                            d.state = i.checked_sub(1).and_then(|i| states.get(i).cloned());
                        }
                    }
                    Event::Freeze(i) => {
                        if let Some(d) = chosen.and_then(|k| set.details.get_mut(k))
                            && let Some((id, _)) = layers.get(i)
                            && !d.frozen.contains(id)
                        {
                            d.frozen.push(id.clone());
                        }
                    }
                    Event::Thaw(id) => {
                        if let Some(d) = chosen.and_then(|k| set.details.get_mut(k)) {
                            d.frozen.retain(|l| *l != id);
                        }
                    }
                    Event::Annotation(i) => {
                        if let Some(d) = chosen.and_then(|k| set.details.get_mut(k)) {
                            d.annotation = i.checked_sub(1).and_then(|i| ratios().get(i).map(|r| r.1 / r.0));
                        }
                    }
                    Event::Locked(on) => {
                        if let Some(d) = chosen.and_then(|k| set.details.get_mut(k)) {
                            d.locked = on;
                        }
                    }
                    Event::First(t) => set.first = t,
                    Event::Title(t) => set.title = t,
                    _ => {}
                }
            }
        }
        self.sheet_details_preview();
        Task::none()
    }

    /// Paftaları kur: the set written as one change of the book, one undo step; the window closes.
    fn sheet_details_build(&mut self) -> Task<Message> {
        let (Some(doc), Some(set)) = (self.document.as_mut(), self.sheet_details.as_ref()) else {
            return Task::none();
        };
        let input = set.input();
        // A book edit is no undo step of the drawing by itself (docs/adr/0251 §3); in a group it is one (ADR 0590): the
        // whole set, one undo.
        let group = doc.model.begin_group(TITLE);
        let answer = sheets_details::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if matches!(answer, CommandResult::Completed { .. }) {
            doc.model.end_group(group);
        } else {
            doc.model.cancel_group(group);
        }
        match answer {
            CommandResult::Completed { output, warnings } => {
                let numbers: Vec<String> = output
                    .made
                    .iter()
                    .filter_map(|m| m["number"].as_str().map(str::to_owned))
                    .collect();
                self.dialog = None;
                self.sheet_details = Some(DetailSet::default());
                self.say_warnings(&warnings);
                self.say(
                    Level::Success,
                    format!(
                        "{} detay {} paftaya yerleşti: {}. Görünümler kilitli; tek adımda geri alınır.",
                        output.details.len(),
                        numbers.len(),
                        numbers.join(", ")
                    ),
                );
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                self.warn(format!("{TITLE}: {}", error.message));
            }
            _ => {}
        }
        Task::none()
    }

    pub(crate) fn sheet_details_view(&self) -> Element<'_, Message> {
        let Some(set) = &self.sheet_details else {
            return label::body("").into();
        };
        let (states, layers) = self.sheet_details_layers();
        let words: Vec<String> = ratios().iter().map(|(p, m)| ratio_words(*p, *m)).collect();
        let add = |caption: &'static str, icon_name: &'static str, e: Event| {
            button(
                row![
                    kentos_ui::icon::icon(crate::icons::from_web(Some(icon_name))).size(14.0),
                    label::body(caption)
                ]
                .spacing(6)
                .align_y(Center),
            )
            .padding(kentos_ui::theme::metrics::action())
            .style(style::button::secondary)
            .on_press(msg(e))
        };
        let list = road_dialog::edit_table(
            vec![col("Ad", 120.0), col("Kaynak", 118.0), col("Oran", 64.0), col("", 28.0)],
            set.details.len(),
            {
                let details = set.details.clone();
                let words = words.clone();
                move |i, k| match k {
                    0 => road_dialog::edit(&details[i].name, false, move |t| msg(Event::Name(i, t))),
                    1 => kentos_ui::widget::table::text(details[i].source_words()).into(),
                    2 => road_dialog::pick(words.clone(), Some(details[i].ratio), move |r| msg(Event::Ratio(i, r))),
                    _ => road_dialog::drop_cell(msg(Event::Remove(i))),
                }
            },
            set.chosen,
            |i| msg(Event::Choose(i)),
            "Detay yok: kapsamı seçip Seçimden ekle'ye basın.",
        );
        let mut form = Form::new()
            .label_width(118.0)
            .section("Detaylar")
            .wide(list)
            .row(row![add(FROM_SELECTION, "selectWindow", Event::FromSelection), add(FROM_TABLE, "table", Event::FromTable)].spacing(8))
            .help("Kapsam seçili nesnelerin kutusudur; tablo parçası bölünmüş model tablosunun bir parçasıdır.");
        if let Some(d) = set.chosen.and_then(|k| set.details.get(k)) {
            let state_words: Vec<String> = std::iter::once("Çizimin katmanları".to_owned()).chain(states.iter().cloned()).collect();
            let state_at = d.state.as_ref().and_then(|s| states.iter().position(|x| x == s)).map_or(0, |i| i + 1);
            let annotation_words: Vec<String> =
                std::iter::once("Pencerenin ölçeği".to_owned()).chain(words.iter().cloned()).collect();
            let annotation_at = d
                .annotation
                .and_then(|a| ratios().iter().position(|r| (r.1 / r.0 - a).abs() < 1e-9))
                .map_or(0, |i| i + 1);
            form = form.section(format!("Seçili detay: {}", d.name));
            if let Scope::Table { part, parts, .. } = &d.scope {
                let options: Vec<String> = (1..=*parts).map(|p| format!("{p}. parça")).collect();
                form = form.field("Tablo parçası", road_dialog::pick(options, Some(*part as usize), |p| msg(Event::Part(p))));
            }
            form = form
                .field("Katmanlar", road_dialog::pick(state_words, Some(state_at), |i| msg(Event::State(i))))
                .help("Katman durumu bağlı pencere, başka kullanıcının katman aç/kapasıyla değişmez.");
            let names: Vec<String> = layers.iter().map(|(_, p)| p.clone()).collect();
            form = form.field("Dondur", road_dialog::pick(names, None, |i| msg(Event::Freeze(i))));
            for id in &d.frozen {
                let path = layers.iter().find(|(l, _)| l == id).map_or_else(|| id.clone(), |(_, p)| p.clone());
                let thaw = id.clone();
                form = form.field(
                    "",
                    row![
                        label::body(path).width(Fill),
                        button(label::caption("×")).style(style::button::ghost).padding([0, 6]).on_press(msg(Event::Thaw(thaw)))
                    ]
                    .align_y(Center),
                );
            }
            form = form
                .field("Açıklama", road_dialog::pick(annotation_words, Some(annotation_at), |i| msg(Event::Annotation(i))))
                .field("Kilit", Switch::new(d.locked, |on| msg(Event::Locked(on))).label("Görünüm kilitli"))
                .help("Kilitli görünümde ekranı yakınlaştırmak baskı ölçeğini değiştirmez.");
        }
        form = form
            .section("Set")
            .field("İlk numara", kentos_ui::widget::text_field("D-501", &set.first).on_input(|t| msg(Event::First(t))))
            .field("Başlık", kentos_ui::widget::text_field("Detaylar", &set.title).on_input(|t| msg(Event::Title(t))));
        let mut d = RoadDialog::new(TITLE, form, msg(Event::Close))
            .form_width(420.0)
            .width(1180.0)
            .empty("Detay eklenince paftaları ve her detayın kâğıttaki yeri burada görünür; çizim değişmez.");
        let mut can = false;
        match &set.result {
            Some(Err(why)) => d = d.note(Banner::error(why.clone())),
            Some(Ok(out)) => {
                can = !out.details.is_empty();
                let size = |id: &str| out.edited.sheets.iter().find(|s| s.id == id).map(|s| (s.width, s.height));
                let made: Vec<(String, f64, f64, String)> = out
                    .made
                    .iter()
                    .filter_map(|m| {
                        let id = m["sheet"].as_str()?.to_owned();
                        let (w, h) = size(&id).unwrap_or((420.0, 297.0));
                        Some((id, w, h, m["number"].as_str().unwrap_or_default().to_owned()))
                    })
                    .collect();
                let mut frames = Vec::new();
                let mut rows = Vec::new();
                for det in &out.details {
                    let sheet = det["sheet"].as_str().unwrap_or_default();
                    let at = made.iter().position(|m| m.0 == sheet).unwrap_or(0);
                    let f = &det["frame"];
                    let mm = |k: &str| f[k].as_f64().unwrap_or(0.0) / 1000.0;
                    let name = det["name"].as_str().unwrap_or_default().to_owned();
                    let denominator = det["denominator"].as_f64().unwrap_or(1.0);
                    frames.push((at, [mm("left"), mm("top"), mm("width"), mm("height")], name.clone()));
                    let number = made.get(at).map(|m| m.3.clone()).unwrap_or_default();
                    let ratio = if denominator >= 1.0 {
                        ratio_words(1.0, denominator)
                    } else {
                        ratio_words(1.0 / denominator, 1.0)
                    };
                    rows.push(vec![
                        name,
                        number,
                        ratio,
                        format!("{} × {}", road_dialog::grouped(mm("width"), 1), road_dialog::grouped(mm("height"), 1)),
                    ]);
                }
                // The ratios from the largest paper size to the smallest (2:1, 1:1, 1:5 … 1:100), each once.
                let mut by_size: Vec<(f64, String)> = out
                    .details
                    .iter()
                    .zip(&rows)
                    .map(|(det, r)| (det["denominator"].as_f64().unwrap_or(1.0), r[2].clone()))
                    .collect();
                by_size.sort_by(|a, b| a.0.total_cmp(&b.0));
                by_size.dedup_by(|a, b| a.1 == b.1);
                let scales: Vec<String> = by_size.into_iter().map(|(_, w)| w).collect();
                let paper = Paper {
                    sheets: made.iter().map(|m| (m.1, m.2, m.3.clone())).collect(),
                    frames,
                };
                d = d
                    .picture(Canvas::new(paper).width(Fill).height(Fill), 240.0)
                    .legend("Her paftada detayların kâğıttaki çerçevesi; çizim yazılmadan önce.")
                    .figure("Pafta", made.len().to_string())
                    .figure("Detay", out.details.len().to_string())
                    .figure("Oran", scales.into_iter().collect::<Vec<_>>().join(" · "));
                for w in &set.warnings {
                    d = d.note(Banner::warning(w.clone()));
                }
                d = d.detail(road_dialog::table(
                    vec![col("Detay", 120.0), col("Pafta", 70.0), col("Oran", 64.0), num("Kâğıtta (mm)", 110.0)],
                    rows,
                    "Detay yok",
                ));
            }
            None => {}
        }
        d.primary(BUILD, can.then(|| msg(Event::Build))).view()
    }
}

#[cfg(test)]
#[path = "sheet_details_window_tests.rs"]
mod tests;
