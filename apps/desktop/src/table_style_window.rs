//! Tablo stilleri (docs/adr/1127 §4): the project's table styles (docs/adr/1126) in the road windows' layout
//! (DESIGN.md §7.19). Over the form the kit's table lists the styles, each with how many tables follow it (the
//! drawing's and its blocks'); pressing a row chooses it. The form edits the chosen style: its name, then one of its
//! three cell styles at a time (Başlık, Sütun başı, Veri), each field empty for “the table's own”: its words' text
//! style, height and alignment, their colour and the cell's fill, the horizontal and vertical margins and the
//! borders' weight and colour, all on the paper. On the right a sample table is drawn as the drawing draws one that
//! follows the style (`styled_table`, at the project's plot scale), and under it the three cell styles side by side.
//!
//! The form is a draft. Kaydet writes it through `cad.styles.edit` (kind `table`: create, update, remove) in one undo
//! step “Tablo stilleri”; a removed style's tables lose the link. Kapat, Esc and × leave it.

use std::collections::HashMap;

use iced::widget::canvas::{self, Frame, Path, Stroke};
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{
    AnnotationStyleKind, CommandResult, DrawingFont, Entity, StyleEditOperation, StylesEdit, TableAlign,
    TableCellStyleDef, TableStyleDef, table_style_defs_problem,
};
use kentos_interaction::Level;
use kentos_ui::widget::Form;
use kentos_ui::widget::segmented::Segmented;

use crate::app::{App, Dialog, Message};
use crate::road_dialog::{self, RoadDialog, col, num};

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Tablo stilleri";
const SAVE: &str = "Kaydet";
const NEW: &str = "Yeni";
const REMOVE: &str = "Sil";

/// One of a style's cell styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cell {
    Title,
    Header,
    Data,
}

impl std::fmt::Display for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Cell::Title => "Başlık",
            Cell::Header => "Sütun başı",
            Cell::Data => "Veri",
        })
    }
}

const CELLS: [Cell; 3] = [Cell::Title, Cell::Header, Cell::Data];

/// A cell style's field typed as text: numbers in paper mm, colours as `#rrggbb`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Field {
    Height,
    Horizontal,
    Vertical,
    Weight,
    Color,
    Fill,
    BorderColor,
}

/// The fields in the form's order, with their labels.
const FIELDS: [(Field, &str); 7] = [
    (Field::Height, "Yükseklik (mm)"),
    (Field::Horizontal, "Yatay boşluk (mm)"),
    (Field::Vertical, "Dikey boşluk (mm)"),
    (Field::Weight, "Kenar kalınlığı (mm)"),
    (Field::BorderColor, "Kenar rengi"),
    (Field::Color, "Yazı rengi"),
    (Field::Fill, "Dolgu"),
];

#[derive(Debug, Clone)]
pub struct Window {
    styles: Vec<TableStyleDef>,
    chosen: Option<usize>,
    cell: Cell,
    /// The fields as typed, by style id, cell style and field; what is not typed is shown from the style.
    typed: HashMap<(String, Cell, Field), String>,
    /// How many tables follow each style, as when the window opened.
    usage: HashMap<String, usize>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Open,
    Choose(usize),
    Cell(Cell),
    Name(String),
    TextStyle(Option<String>),
    Align(Option<TableAlign>),
    Field(Field, String),
    New,
    Remove,
    Save,
    Close,
}

fn msg(event: Event) -> Message {
    Message::TableStyleDefs(event)
}

fn cell_of(s: &TableStyleDef, c: Cell) -> Option<&TableCellStyleDef> {
    match c {
        Cell::Title => s.title.as_ref(),
        Cell::Header => s.header.as_ref(),
        Cell::Data => s.data.as_ref(),
    }
}

fn cell_mut(s: &mut TableStyleDef, c: Cell) -> &mut TableCellStyleDef {
    match c {
        Cell::Title => s.title.get_or_insert_with(Default::default),
        Cell::Header => s.header.get_or_insert_with(Default::default),
        Cell::Data => s.data.get_or_insert_with(Default::default),
    }
}

/// A field's value as the style holds it, as the form shows it.
fn shown(cell: Option<&TableCellStyleDef>, f: Field) -> String {
    let Some(c) = cell else { return String::new() };
    let n = |v: Option<f64>| v.map(|v| format!("{v}").replace('.', ",")).unwrap_or_default();
    match f {
        Field::Height => n(c.height),
        Field::Horizontal => n(c.horizontal_margin),
        Field::Vertical => n(c.vertical_margin),
        Field::Weight => n(c.border_weight),
        Field::Color => c.color.clone().unwrap_or_default(),
        Field::Fill => c.fill.clone().unwrap_or_default(),
        Field::BorderColor => c.border_color.clone().unwrap_or_default(),
    }
}

/// A typed field written into the cell style: empty takes it away; a number that does not read, or a colour not
/// `#rrggbb`, is the field's problem and leaves the style.
fn write(cell: &mut TableCellStyleDef, f: Field, text: &str) -> Result<(), String> {
    let t = text.trim();
    let number = || -> Result<Option<f64>, String> {
        if t.is_empty() {
            return Ok(None);
        }
        t.replace(',', ".").parse::<f64>().map(Some).map_err(|_| format!("“{t}” bir sayı değil"))
    };
    let colour = || -> Result<Option<String>, String> {
        if t.is_empty() {
            return Ok(None);
        }
        let ok = t.len() == 7 && t.starts_with('#') && t[1..].chars().all(|c| c.is_ascii_hexdigit());
        ok.then(|| Some(t.to_lowercase())).ok_or_else(|| format!("“{t}” #rrggbb biçiminde bir renk değil"))
    };
    match f {
        Field::Height => cell.height = number()?,
        Field::Horizontal => cell.horizontal_margin = number()?,
        Field::Vertical => cell.vertical_margin = number()?,
        Field::Weight => cell.border_weight = number()?,
        Field::Color => cell.color = colour()?,
        Field::Fill => cell.fill = colour()?,
        Field::BorderColor => cell.border_color = colour()?,
    }
    Ok(())
}

impl Window {
    fn chosen(&self) -> Option<&TableStyleDef> {
        self.chosen.and_then(|i| self.styles.get(i))
    }

    /// What keeps Kaydet closed: a field typed that does not read, then the table's own rule.
    fn problem(&self) -> Option<String> {
        for ((id, cell, field), text) in &self.typed {
            let Some(s) = self.styles.iter().find(|s| &s.id == id) else { continue };
            let mut probe = cell_of(s, *cell).cloned().unwrap_or_default();
            if let Err(why) = write(&mut probe, *field, text) {
                return Some(format!("“{}” {} hücresi: {why}", s.name, cell.to_string().to_lowercase()));
            }
        }
        table_style_defs_problem(&self.styles)
    }
}

/// How many tables follow each table style: the drawing's and its block definitions'.
fn usage(doc: &kentos_domain::Document) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    for e in doc.entities().chain(doc.blocks().iter().flat_map(|b| &b.entities)) {
        if let Entity::Table(t) = e
            && let Some(id) = &t.table_style
        {
            *out.entry(id.clone()).or_insert(0) += 1;
        }
    }
    out
}

impl App {
    fn saved_table_styles(&self) -> Vec<TableStyleDef> {
        self.document.as_ref().map(|d| d.model.settings().table_styles.clone()).unwrap_or_default()
    }

    pub(crate) fn table_style_window_event(&mut self, event: Event) -> Task<Message> {
        if let Event::Open = event {
            let Some(doc) = self.document.as_ref() else { return Task::none() };
            let styles = doc.model.settings().table_styles.clone();
            self.table_style_window = Some(Window {
                chosen: (!styles.is_empty()).then_some(0),
                styles,
                cell: Cell::Data,
                typed: HashMap::new(),
                usage: usage(&doc.model),
            });
            self.dialog = Some(Dialog::TableStyleDefs);
            return Task::none();
        }
        if matches!(event, Event::Close) {
            self.table_style_window = None;
            self.dialog = None;
            return Task::none();
        }
        if matches!(event, Event::Save) {
            self.save_table_style_window();
            return Task::none();
        }
        let text_styles: Vec<String> = self
            .document
            .as_ref()
            .map(|d| d.model.settings().text_styles.iter().map(|s| s.name.clone()).collect())
            .unwrap_or_default();
        let Some(w) = self.table_style_window.as_mut() else { return Task::none() };
        match event {
            Event::Choose(i) => w.chosen = Some(i).filter(|i| *i < w.styles.len()),
            Event::Cell(c) => w.cell = c,
            Event::Name(name) => {
                if let Some(s) = w.chosen.and_then(|i| w.styles.get_mut(i)) {
                    s.name = name;
                }
            }
            Event::TextStyle(name) => {
                let cell = w.cell;
                if let Some(s) = w.chosen.and_then(|i| w.styles.get_mut(i)) {
                    cell_mut(s, cell).text_style = name.filter(|n| text_styles.contains(n));
                }
            }
            Event::Align(a) => {
                let cell = w.cell;
                if let Some(s) = w.chosen.and_then(|i| w.styles.get_mut(i)) {
                    cell_mut(s, cell).align = a;
                }
            }
            Event::Field(f, text) => {
                let cell = w.cell;
                if let Some(s) = w.chosen.and_then(|i| w.styles.get_mut(i)) {
                    let mut next = cell_of(s, cell).cloned().unwrap_or_default();
                    if write(&mut next, f, &text).is_ok() {
                        *cell_mut(s, cell) = next;
                    }
                    w.typed.insert((s.id.clone(), cell, f), text);
                }
            }
            Event::New => {
                // A copy of the chosen one (an empty style without one), under a name no style has.
                let mut s = w.chosen().cloned().unwrap_or(TableStyleDef {
                    id: String::new(),
                    name: String::new(),
                    title: None,
                    header: None,
                    data: None,
                });
                let taken: Vec<String> = w.styles.iter().map(|s| s.name.to_lowercase()).collect();
                let base = if s.name.is_empty() { "Tablo stili".to_owned() } else { s.name.clone() };
                s.name = (1..).map(|k| if k == 1 && s.name.is_empty() { base.clone() } else { format!("{base} {k}") })
                    .find(|n| !taken.contains(&n.to_lowercase()))
                    .unwrap_or(base);
                s.id = kentos_domain::Uuid::now_v7().to_string();
                w.styles.push(s);
                w.chosen = Some(w.styles.len() - 1);
            }
            Event::Remove => {
                if let Some(i) = w.chosen.filter(|i| *i < w.styles.len()) {
                    w.styles.remove(i);
                    w.chosen = if w.styles.is_empty() { None } else { Some(i.min(w.styles.len() - 1)) };
                }
            }
            Event::Open | Event::Close | Event::Save => {}
        }
        Task::none()
    }

    /// Kaydet: each style removed, made or changed through `cad.styles.edit`, in one undo step.
    fn save_table_style_window(&mut self) {
        let Some(w) = self.table_style_window.clone() else { return };
        if w.problem().is_some() {
            return;
        }
        let saved = self.saved_table_styles();
        let Some(doc) = self.document.as_mut() else { return };
        let edit = |operation, style: Option<String>, table: Option<TableStyleDef>| StylesEdit {
            kind: AnnotationStyleKind::Table,
            operation,
            style,
            name: None,
            text: None,
            dimension: None,
            leader: None,
            wall: None,
            table,
            styles: None,
            library: None,
            replace: None,
            expected_revision: None,
        };
        let mut calls = Vec::new();
        for s in saved.iter().filter(|s| !w.styles.iter().any(|n| n.id == s.id)) {
            calls.push(edit(StyleEditOperation::Remove, Some(s.id.clone()), None));
        }
        for s in &w.styles {
            match saved.iter().find(|o| o.id == s.id) {
                None => calls.push(edit(StyleEditOperation::Create, None, Some(s.clone()))),
                Some(o) if o != s => calls.push(edit(StyleEditOperation::Update, None, Some(s.clone()))),
                Some(_) => {}
            }
        }
        let group = doc.model.begin_group(TITLE);
        let (mut followed, mut locked, mut refused) = (0u32, 0u32, None);
        for input in calls {
            match kentos_native_application::styles_edit::execute(
                &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
                input,
            ) {
                CommandResult::Completed { output, .. } => {
                    followed += output.followed;
                    locked += output.locked;
                }
                CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                    refused = Some(error.message);
                    break;
                }
                _ => {}
            }
        }
        doc.model.end_group(group);
        if let Some(why) = refused {
            self.warn(why);
            return;
        }
        let mut said = "Tablo stilleri kaydedildi".to_owned();
        if followed > 0 {
            said.push_str(&format!("; {followed} tablo silinen stilden ayrıldı"));
        }
        if locked > 0 {
            said.push_str(&format!("; kilitli katmandaki {locked} tablo olduğu gibi kaldı"));
        }
        self.say(Level::Success, format!("{said}. Geri almak için Geri al."));
        let usage = self.document.as_ref().map(|d| usage(&d.model)).unwrap_or_default();
        if let Some(w) = self.table_style_window.as_mut() {
            w.typed.clear();
            w.usage = usage;
        }
    }

    pub(crate) fn table_style_window_view(&self) -> Element<'_, Message> {
        let (Some(w), Some(doc)) = (&self.table_style_window, self.document.as_ref()) else {
            return iced::widget::text("").into();
        };
        let settings = doc.model.settings();
        // The styles, each with how many tables follow it.
        let names: Vec<(String, usize)> =
            w.styles.iter().map(|s| (s.name.clone(), w.usage.get(&s.id).copied().unwrap_or(0))).collect();
        let list = road_dialog::edit_table(
            vec![col("Ad", 170.0), num("Tablo", 56.0)],
            names.len(),
            move |i, k| {
                let (name, n) = &names[i];
                let cell: Element<'_, Message> = match k {
                    0 => kentos_ui::widget::table::text(name.clone()).into(),
                    _ => kentos_ui::widget::table::number(n.to_string()).into(),
                };
                cell
            },
            w.chosen,
            |i| msg(Event::Choose(i)),
            "Projede tablo stili yok. Yeni ile ekleyin.",
        );
        let mut form = Form::new().label_width(130.0);
        if let Some(s) = w.chosen() {
            let cell = cell_of(s, w.cell);
            let texts: Vec<String> = settings.text_styles.iter().map(|t| t.name.clone()).collect();
            let text_at = cell.and_then(|c| c.text_style.as_ref()).and_then(|n| texts.iter().position(|t| t.eq_ignore_ascii_case(n)));
            let text_pick = {
                let options: Vec<String> = std::iter::once("Tablonun".to_owned()).chain(texts.iter().cloned()).collect();
                let texts = texts.clone();
                road_dialog::pick(
                    options,
                    Some(text_at.map_or(0, |i| i + 1)),
                    move |i| msg(Event::TextStyle(i.checked_sub(1).and_then(|i| texts.get(i).cloned()))),
                )
            };
            let aligns = [None, Some(TableAlign::Left), Some(TableAlign::Center), Some(TableAlign::Right)];
            let align_at = aligns.iter().position(|a| *a == cell.and_then(|c| c.align));
            let align_pick = road_dialog::pick(["Tablonun", "Sol", "Orta", "Sağ"], align_at, move |i| {
                msg(Event::Align(aligns.get(i).copied().flatten()))
            });
            form = form
                .section("Stil")
                .field("Ad", kentos_ui::widget::text_field("Ad", &s.name).on_input(|t| msg(Event::Name(t))))
                .section("Hücre")
                .row(Segmented::new(CELLS, w.cell, |c| msg(Event::Cell(c))).compact())
                .field("Yazı stili", text_pick)
                .field("Hizalama", align_pick);
            for (f, label) in FIELDS {
                let typed = w.typed.get(&(s.id.clone(), w.cell, f)).cloned().unwrap_or_else(|| shown(cell, f));
                let hint = if matches!(f, Field::Color | Field::Fill | Field::BorderColor) { "#rrggbb" } else { "Tablonun" };
                form = form.field(label, kentos_ui::widget::text_field(hint, &typed).on_input(move |t| msg(Event::Field(f, t))));
            }
            form = form.help("Boş alan tablonun kendi değerini kullanır. Ölçüler kâğıtta, proje ölçeğinde çizilir.");
        }
        let problem = w.problem();
        let saved = self.saved_table_styles();
        let changed = w.styles != saved;
        let mut d = RoadDialog::new(TITLE, form, msg(Event::Close)).head(list).form_width(330.0);
        match w.chosen() {
            None => d = d.empty("Bir stil seçin ya da Yeni ile ekleyin."),
            Some(s) => {
                let mut drafted = settings.clone();
                drafted.table_styles = w.styles.clone();
                let sample = sample_table(&s.id, settings.plot_scale);
                let shape = kentos_native_application::geometry::styled_table(&sample, &drafted);
                let face = kentos_commands::styled(&sample, &drafted).face.clone();
                // On paper, as it prints: a cell's fill is the paper's light colour, its words the ink.
                let canvas = crate::viewport::Canvas::Paper;
                let preview = Preview {
                    shape,
                    plot_scale: settings.plot_scale,
                    face,
                    font: settings.drawing_font.unwrap_or(DrawingFont::Barlow),
                    colors: crate::labels::colors(canvas, &crate::viewport::palette(canvas)),
                };
                d = d
                    .picture(canvas::Canvas::new(preview).width(Fill).height(Fill), 240.0)
                    .legend("Örnek tablo, kâğıtta basıldığı gibi: başlık, sütun başı ve veri satırları");
                let mm = |v: Option<f64>| v.map_or_else(|| "—".to_owned(), |v| road_dialog::grouped(v, 2));
                let rows = CELLS
                    .iter()
                    .map(|c| {
                        let x = cell_of(s, *c);
                        let get = |f: fn(&TableCellStyleDef) -> Option<f64>| mm(x.and_then(f));
                        vec![
                            c.to_string(),
                            x.and_then(|x| x.text_style.clone()).unwrap_or_else(|| "—".into()),
                            get(|x| x.height),
                            get(|x| x.horizontal_margin),
                            get(|x| x.vertical_margin),
                            get(|x| x.border_weight),
                            x.and_then(|x| x.border_color.clone()).unwrap_or_else(|| "—".into()),
                        ]
                    })
                    .collect();
                d = d.detail(road_dialog::table(
                    vec![
                        col("Hücre", 90.0),
                        col("Yazı stili", 100.0),
                        num("Yükseklik (mm)", 100.0),
                        num("Yatay (mm)", 80.0),
                        num("Dikey (mm)", 80.0),
                        num("Kenar (mm)", 80.0),
                        col("Kenar rengi", 90.0),
                    ],
                    rows,
                    "",
                ));
                let n = w.usage.get(&s.id).copied().unwrap_or(0);
                d = d.status(match &problem {
                    Some(p) => p.clone(),
                    None if n > 0 => format!("{n} tablo bu stili izliyor; kendi değeri olmayan yerde stili çizer."),
                    None => "Bu stili izleyen tablo yok; tablonun özelliklerinden seçilir.".to_owned(),
                });
            }
        }
        d.ghost(NEW, Some(msg(Event::New)))
            .ghost(REMOVE, w.chosen.map(|_| msg(Event::Remove)))
            .primary(SAVE, (changed && problem.is_none()).then(|| msg(Event::Save)))
            .width(1120.0)
            .view()
    }
}

/// The sample the window draws: a title row over its two columns, a heading row and two data rows, sized on the paper
/// (mm) at the plot scale `scale`, following style `id`.
fn sample_table(id: &str, scale: f64) -> kentos_commands::TableEntity {
    let k = scale / 1000.0;
    serde_json::from_value(serde_json::json!({
        "id": 0, "layerId": "", "attrs": {}, "p": {"x": 0.0, "y": 0.0}, "rotation": 0.0, "height": 2.5 * k,
        "rows": [10.0 * k, 8.0 * k, 7.0 * k, 7.0 * k], "columns": [36.0 * k, 26.0 * k, 26.0 * k],
        "cells": [["Parsel listesi", "", ""], ["Ada", "Parsel", "Alan (m²)"], ["101", "12", "512,40"],
                  ["102", "7", "640,15"]],
        "merges": [{"row": 0, "col": 0, "rows": 1, "cols": 3}], "header": true, "tableStyle": id,
    }))
    .expect("the sample table")
}

/// The sample drawn: its fills, its own lines a hairline, its borders at their weight and colour, its words at their
/// cell style's height; fitted in the picture.
struct Preview {
    shape: kentos_geometry_core::entity::Shape,
    plot_scale: f64,
    face: kentos_commands::TextFace,
    font: DrawingFont,
    colors: crate::labels::Colors,
}

impl canvas::Program<Message> for Preview {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        use kentos_geometry_core::geom::table::table_geom;
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), self.colors.halo);
        let Some(g) = table_geom(&self.shape) else { return vec![frame.into_geometry()] };
        let (width, depth) = g.size();
        let k = ((f64::from(bounds.width) - 32.0) / width.max(1e-9)).min((f64::from(bounds.height) - 32.0) / depth.max(1e-9));
        let (left, top) = (
            (f64::from(bounds.width) - width * k) / 2.0,
            (f64::from(bounds.height) - depth * k) / 2.0,
        );
        // The table's top left is its point; y runs down the screen.
        let at = |p: kentos_geometry_core::Vec2| Point::new((left + (p.x - g.p.x) * k) as f32, (top + (g.p.y - p.y) * k) as f32);
        for f in g.fills() {
            let q = f.corners.map(at);
            let path = Path::new(|b| {
                b.move_to(q[0]);
                for p in &q[1..] {
                    b.line_to(*p);
                }
                b.close();
            });
            frame.fill(&path, crate::view::hex_color(&f.fill));
        }
        for [a, b] in g.plain_lines() {
            frame.stroke(&Path::line(at(a), at(b)), Stroke::default().with_width(1.0).with_color(self.colors.fg));
        }
        // A paper millimetre in pixels at this fit.
        let px_mm = self.plot_scale / 1000.0 * k;
        for b in g.borders() {
            let width = b.weight.map_or(1.0, |w| (w * px_mm).max(1.0)) as f32;
            let color = b.color.as_deref().map_or(self.colors.fg, crate::view::hex_color);
            frame.stroke(&Path::line(at(b.line[0]), at(b.line[1])), Stroke::default().with_width(width).with_color(color));
        }
        let cells = g.layout(kentos_native_application::geometry::drawing_font(Some(self.font))).cells;
        for c in cells {
            let Some(words) = g.cells.get(c.row).and_then(|r| r.get(c.col)) else { continue };
            let h = g.style_at(c.row, c.col).height.unwrap_or(g.height);
            let ink = c.color.as_deref().map_or(self.colors.label, crate::view::hex_color);
            crate::labels::sample_text(
                &mut frame,
                kentos_geometry_core::geom::table::cell_line(words, c.line),
                at(c.at),
                (h * k) as f32,
                &kentos_commands::TextFace { bold: c.bold || self.face.bold, ..self.face.clone() },
                1.0,
                self.font,
                (ink, Color::TRANSPARENT),
            );
        }
        vec![frame.into_geometry()]
    }
}

/// The window as the review pictures show it: CETVEL's data cells chosen. `KENTOS_SHOTS_ONLY=tablo-stilleri cargo test
/// -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture`.
#[cfg(test)]
pub(crate) fn scenes() -> Vec<crate::tools_screens::Scene> {
    vec![
        ("tablo-stilleri", |app| {
            tests::drawing(app);
            let _ = app.table_style_window_event(Event::Open);
            let _ = app.table_style_window_event(Event::Cell(Cell::Title));
        }),
        ("tablo-ozellik-stili", |app| {
            let slot = tests::drawing(app);
            app.selection.set([slot]);
            // Genel folded: the table's rows, Tablo stili among them, in view.
            let _ = app.update(Message::Properties(crate::properties::Event::Toggle("general")));
        }),
    ]
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    /// A CAD drawing at 1:100 with two table styles and a table that follows CETVEL, in view; the table's slot.
    pub(crate) fn drawing(app: &mut App) -> kentos_domain::Slot {
        crate::files_testing::make_cad(app);
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.plot_scale = 100.0;
        settings.text_styles = serde_json::from_value(json!([{"id":"yazi-tablo","name":"A-TABLO","font":"arimo"}])).unwrap();
        settings.table_styles = serde_json::from_value(json!([
            {"id":"ofis-cetvel","name":"CETVEL","title":{"align":"center","height":3.5,"borderWeight":0.7,"borderColor":"#c0392b","fill":"#f3e9e7"},
             "header":{"height":2.5,"borderColor":"#2244dd","fill":"#e8edf9"},"data":{"textStyle":"A-TABLO","align":"right","horizontalMargin":1.5}},
            {"id":"yerel-sade","name":"SADE","data":{"align":"left"}}])).unwrap();
        doc.model.set_settings(settings);
        let layer = doc.model.layers().active().to_owned();
        let t: kentos_commands::Entity = serde_json::from_value(json!({"kind":"table","id":1,"layerId":layer,"attrs":{},
            "p":{"x":0.0,"y":3.0},"rotation":0.0,"height":0.25,"rows":[1.0,0.8,0.7,0.7],"columns":[3.6,2.6,2.6],
            "cells":[["Parsel listesi","",""],["Ada","Parsel","Alan (m²)"],["101","12","512,40"],["102","7","640,15"]],
            "merges":[{"row":0,"col":0,"rows":1,"cols":3}],"header":true,"tableStyle":"ofis-cetvel"})).unwrap();
        let slot = doc.model.add(t).expect("a slot");
        app.viewport.camera.fit(&kentos_render_wgpu::Bounds { min_x: -1.0, min_y: -0.6, max_x: 9.8, max_y: 3.6 }, 24.0);
        slot
    }

    #[test]
    fn the_property_row_gives_the_table_its_style_in_one_step() {
        let mut app = crate::files_testing::app_with_drawing();
        let slot = drawing(&mut app);
        let table_style = |app: &App| match app.document.as_ref().unwrap().model.get(slot) {
            Some(Entity::Table(t)) => t.table_style.clone(),
            _ => panic!("the table"),
        };
        let _ = app.update(Message::Properties(crate::properties::Event::TableStyle(vec![slot], Some("yerel-sade".into()))));
        assert_eq!(table_style(&app).as_deref(), Some("yerel-sade"));
        let _ = app.update(Message::Properties(crate::properties::Event::TableStyle(vec![slot], None)));
        assert_eq!(table_style(&app), None, "Yok takes the style away");
        assert_eq!(app.document.as_mut().unwrap().model.undo().as_deref(), Some("Değiştir"));
        assert_eq!(table_style(&app).as_deref(), Some("yerel-sade"));
    }

    #[test]
    fn the_window_saves_a_changed_style_and_a_removed_one_in_one_step() {
        let mut app = crate::files_testing::app_with_drawing();
        drawing(&mut app);
        let _ = app.table_style_window_event(Event::Open);
        let w = app.table_style_window.as_ref().expect("the window");
        assert_eq!(w.usage.get("ofis-cetvel"), Some(&1));
        let _ = app.table_style_window_event(Event::Cell(Cell::Data));
        let _ = app.table_style_window_event(Event::Field(Field::Height, "2,2".into()));
        let _ = app.table_style_window_event(Event::Field(Field::Weight, "kalın".into()));
        assert!(app.table_style_window.as_ref().unwrap().problem().is_some(), "a weight that does not read keeps Kaydet");
        let _ = app.table_style_window_event(Event::Field(Field::Weight, "".into()));
        assert_eq!(app.table_style_window.as_ref().unwrap().problem(), None);
        let _ = app.table_style_window_event(Event::Choose(1));
        let _ = app.table_style_window_event(Event::Remove);
        let _ = app.table_style_window_event(Event::Save);
        let doc = app.document.as_mut().unwrap();
        let styles = &doc.model.settings().table_styles;
        assert_eq!(styles.len(), 1);
        assert_eq!(styles[0].data.as_ref().and_then(|d| d.height), Some(2.2));
        assert_eq!(doc.model.undo().as_deref(), Some(TITLE), "one step");
        assert_eq!(doc.model.settings().table_styles.len(), 2);
    }
}
