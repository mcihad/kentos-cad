//! Değer bileşimleri (docs/adr/0423 §5, `data.contingents`): a layer's value groups.
//!
//! ```text
//! ┌ Değer bileşimleri ─────────────────────────────────────────────────────────┐
//! │ Katman [Bina             ▾]                                                 │
//! │ Gruplar          │ Ad [Yapı                 ]  ☑ Kısıtlayıcı               │
//! │ Yapı · 2 alan    │ Alanlar (Tür)(Malzeme)(Çatı)                             │
//! │ Çatı · 2 alan    │ Tür        Malzeme      Alt tür                         │
//! │ [Yeni grup]      │ [Konut ▾]  [Herhangi ▾]  [—  ▾]  ×                       │
//! │                  │ [Sanayi▾]  [Betonarme▾]  [—  ▾]  ×                       │
//! │                  │ [Bileşim ekle]                                           │
//! │ [Grubu sil]                                          [Vazgeç] [Kaydet]      │
//! └────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The window edits a copy; Kaydet writes it through `cad.layers.contingents` in one step and closes.

use iced::widget::{Column, Row, button, column, container, row, scrollable, space};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    Cell, Combination, FieldGroup, LayerField, LayerNodeType, LayersContingents,
    contingents_problem,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, layers_contingents};
use kentos_ui::icon::icon;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::icons::from_web;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Değer bileşimleri";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const NEW: &str = "Yeni grup";
const REMOVE: &str = "Grubu sil";
const ADD_ROW: &str = "Bileşim ekle";

/// The window.
#[derive(Debug, Clone, Default)]
pub struct Window {
    pub layer: String,
    pub groups: Vec<FieldGroup>,
    kept: Vec<FieldGroup>,
    pub chosen: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Layer(String),
    Choose(usize),
    New,
    Remove,
    Name(String),
    Restrictive(bool),
    /// A field of the layer taken into the chosen group, or out of it, by key.
    ToggleField(String),
    AddRow,
    RemoveRow(usize),
    /// A cell of a row: the row, the column, the cell.
    Cell(usize, usize, Cell),
    /// A row's subtype; none: every record's.
    RowSubtype(usize, Option<String>),
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::Contingents(event)
}

/// Whether `f` can be in a group: a single-valued field with a value list.
fn groupable(f: &LayerField) -> bool {
    f.values.as_ref().is_some_and(|v| !v.is_empty()) && !f.holds_many()
}

impl App {
    /// Değer bileşimleri: the window on the active layer's groups (the table's layer when it shows one).
    pub(crate) fn open_contingents(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let layers = self.contingent_layers();
        let wanted = self
            .features
            .layer
            .clone()
            .or_else(|| Some(doc.model.layers().active().to_owned()));
        let Some(layer) = wanted
            .filter(|l| layers.iter().any(|(id, _)| id == l))
            .or_else(|| layers.first().map(|(id, _)| id.clone()))
        else {
            self.warn(
                "İki ya da daha çok değer listeli alanı olan katman yok; değer bileşimleri liste alanlarının birlikte alabileceği değerlerdir.",
            );
            return Task::none();
        };
        self.contingents = Some(self.contingents_window_of(layer));
        self.dialog = Some(Dialog::Contingents);
        Task::none()
    }

    fn contingents_window_of(&self, layer: String) -> Window {
        let groups = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(&layer))
            .map(|n| n.contingents.clone())
            .unwrap_or_default();
        Window {
            layer,
            chosen: (!groups.is_empty()).then_some(0),
            kept: groups.clone(),
            groups,
        }
    }

    pub(crate) fn contingents_event(&mut self, event: Event) -> Task<Message> {
        if let Event::Layer(id) = &event {
            self.contingents = Some(self.contingents_window_of(id.clone()));
            return Task::none();
        }
        let Some(w) = self.contingents.as_mut() else {
            return Task::none();
        };
        let at = w.chosen.filter(|i| *i < w.groups.len());
        let chosen = at.and_then(|i| w.groups.get_mut(i));
        match event {
            Event::Layer(_) => {}
            Event::Choose(i) => w.chosen = Some(i),
            Event::New => {
                let mut n = w.groups.len() + 1;
                while w.groups.iter().any(|g| g.name == format!("Grup {n}")) {
                    n += 1;
                }
                w.groups.push(FieldGroup {
                    name: format!("Grup {n}"),
                    fields: Vec::new(),
                    restrictive: true,
                    rows: Vec::new(),
                });
                w.chosen = Some(w.groups.len() - 1);
            }
            Event::Remove => {
                if let Some(i) = at {
                    w.groups.remove(i);
                    w.chosen = (!w.groups.is_empty()).then_some(i.min(w.groups.len() - 1));
                }
            }
            Event::Name(text) => {
                if let Some(g) = chosen {
                    g.name = text;
                }
            }
            Event::Restrictive(on) => {
                if let Some(g) = chosen {
                    g.restrictive = on;
                }
            }
            Event::ToggleField(key) => {
                if let Some(g) = chosen {
                    if let Some(c) = g.fields.iter().position(|k| *k == key) {
                        g.fields.remove(c);
                        for r in &mut g.rows {
                            if c < r.cells.len() {
                                r.cells.remove(c);
                            }
                        }
                    } else {
                        g.fields.push(key);
                        for r in &mut g.rows {
                            r.cells.push(Cell::Any);
                        }
                    }
                }
            }
            Event::AddRow => {
                if let Some(g) = chosen {
                    g.rows.push(Combination {
                        cells: vec![Cell::Any; g.fields.len()],
                        subtype: None,
                    });
                }
            }
            Event::RemoveRow(r) => {
                if let Some(g) = chosen
                    && r < g.rows.len()
                {
                    g.rows.remove(r);
                }
            }
            Event::Cell(r, c, cell) => {
                if let Some(cells) = chosen
                    .and_then(|g| g.rows.get_mut(r))
                    .map(|row| &mut row.cells)
                    && c < cells.len()
                {
                    cells[c] = cell;
                }
            }
            Event::RowSubtype(r, subtype) => {
                if let Some(row) = chosen.and_then(|g| g.rows.get_mut(r)) {
                    row.subtype = subtype;
                }
            }
            Event::Cancel => {
                self.contingents = None;
                self.dialog = None;
            }
            Event::Save => return self.save_contingents(),
        }
        Task::none()
    }

    /// Kaydet: the groups through `cad.layers.contingents` in one step; the window closes when they go in.
    fn save_contingents(&mut self) -> Task<Message> {
        let Some(w) = self.contingents.clone() else {
            return Task::none();
        };
        if w.groups == w.kept {
            self.contingents = None;
            self.dialog = None;
            return Task::none();
        }
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let result = layers_contingents::execute(
            &mut ExecutionContext::new(&mut doc.model),
            LayersContingents {
                layer: w.layer.clone(),
                groups: w.groups.clone(),
                expected_revision: None,
            },
        );
        if self.answered(result).is_none() {
            return Task::none();
        }
        self.contingents = None;
        self.dialog = None;
        let name = self.form_layer_name(&w.layer);
        let said = if w.groups.is_empty() {
            format!("“{name}” katmanının değer grupları kaldırıldı.")
        } else {
            format!(
                "“{name}” katmanının {} değer grubu kaydedildi.",
                w.groups.len()
            )
        };
        self.say(Level::Success, said);
        Task::none()
    }

    /// The layers with two or more single-valued listed fields, by their path.
    fn contingent_layers(&self) -> Vec<(String, String)> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let mut out = Vec::new();
        fn walk(nodes: &[kentos_commands::LayerNode], at: &str, out: &mut Vec<(String, String)>) {
            for n in nodes {
                let path = if at.is_empty() {
                    n.name.clone()
                } else {
                    format!("{at} / {}", n.name)
                };
                if n.kind == LayerNodeType::Layer
                    && n.service.is_none()
                    && n.fields.iter().filter(|f| groupable(f)).count() >= 2
                {
                    out.push((n.id.clone(), path.clone()));
                }
                walk(&n.children, &path, out);
            }
        }
        walk(doc.model.layers().nodes(), "", &mut out);
        out
    }

    pub(crate) fn contingents_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.contingents else {
            return iced::widget::text("").into();
        };
        let layers = self.contingent_layers();
        let ids: Vec<String> = layers.iter().map(|(id, _)| id.clone()).collect();
        let layer = Select::new(
            layers.iter().map(|(_, n)| Choice::new(n.clone())),
            ids.iter().position(|id| *id == w.layer),
            move |i| msg(Event::Layer(ids[i].clone())),
        );
        let fields = self.layer_fields_of(&w.layer);
        let subtypes = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(&w.layer))
            .and_then(|n| n.subtypes.clone());
        let mut list = Column::new().spacing(1).padding(2);
        for (i, g) in w.groups.iter().enumerate() {
            let face = column![
                label::body(g.name.clone()),
                label::caption(format!(
                    "{} alan · {} bileşim · {}",
                    g.fields.len(),
                    g.rows.len(),
                    if g.restrictive {
                        "kısıtlayıcı"
                    } else {
                        "uyarır"
                    }
                ))
                .style(muted),
            ]
            .spacing(1);
            list = list.push(
                button(face)
                    .on_press(msg(Event::Choose(i)))
                    .padding([4, 8])
                    .width(Fill)
                    .style(style::button::row(w.chosen == Some(i))),
            );
        }
        if w.groups.is_empty() {
            list = list.push(
                label::caption(
                    "Grup yok. Yeni grup ile birlikte değer alan alanları ve geçerli bileşimlerini tanımlayın.",
                )
                .style(muted),
            );
        }
        let at = w.chosen.filter(|i| *i < w.groups.len());
        let left = column![
            label::strong("Gruplar"),
            container(scrollable(list).height(Fill))
                .style(style::container::field_box)
                .width(Fill)
                .height(Fill),
            words::secondary(NEW, Some(msg(Event::New))),
        ]
        .spacing(8);
        let right: Element<'_, Message> = match at.map(|i| &w.groups[i]) {
            None => container(label::caption(
                "Bir grup seçin ya da yeni bir grup oluşturun.",
            ))
            .padding(12)
            .into(),
            Some(g) => group_editor(g, &fields, subtypes.as_ref()),
        };
        let problem =
            contingents_problem(&fields, subtypes.as_ref(), &w.groups).map(|(_, why)| why);
        // The groups and the chosen one take what the window has when it was given a size (docs/adr/0746),
        // else 400 high at most (12 px text), less in a low window; the chosen group scrolls, the buttons
        // stay in sight (docs/adr/0749).
        let sized = kentos_ui::widget::sizable::sizes::get(TITLE).is_some();
        let panes = container(
            row![
                container(left).width(250).height(Fill),
                container(
                    scrollable(right)
                        .direction(style::field::body_scrollbar())
                        .height(Fill)
                )
                .width(Fill)
                .height(Fill)
            ]
            .spacing(16),
        )
        .height(Fill)
        .max_height(if sized {
            f32::INFINITY
        } else {
            typography::scaled(400.0)
        });
        let mut frame = Frame::new(TITLE)
            .push(words::field("Katman", container(layer).width(320), None))
            .fill(panes);
        if let Some(why) = &problem {
            frame = frame.push(words::text_line(Kind::Warn, why.clone()));
        }
        frame = frame.push(
            row![
                words::ghost(REMOVE, at.map(|_| msg(Event::Remove))),
                space::horizontal().width(Fill),
                words::secondary(CANCEL, Some(msg(Event::Cancel))),
                words::primary(SAVE, problem.is_none().then(|| msg(Event::Save))),
            ]
            .spacing(8)
            .align_y(Center),
        );
        frame.width(940.0).modal(msg(Event::Cancel))
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn contingents_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.contingents.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(NEW) => Some(msg(Event::New)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(ADD_ROW) => Some(msg(Event::AddRow)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The chosen group: its name, whether it refuses, its fields and its rows.
fn group_editor<'a>(
    g: &'a FieldGroup,
    fields: &[LayerField],
    subtypes: Option<&kentos_commands::LayerSubtypes>,
) -> Element<'a, Message> {
    let name =
        kentos_ui::widget::text_field("Grubun adı", &g.name).on_input(|s| msg(Event::Name(s)));
    let mut chips = Row::new().spacing(4);
    for f in fields.iter().filter(|f| groupable(f)) {
        let on = g.fields.iter().any(|k| k == f.key());
        chips = chips.push(
            button(label::caption(f.label().to_owned()))
                .on_press(msg(Event::ToggleField(f.key().to_owned())))
                .padding([2, 8])
                .style(style::button::chip(on)),
        );
    }
    let cols: Vec<&LayerField> = g
        .fields
        .iter()
        .filter_map(|k| fields.iter().find(|f| f.key() == k))
        .collect();
    let width = 150.0;
    let mut head = Row::new().spacing(6);
    for f in &cols {
        head = head.push(container(label::caption(f.label().to_owned()).style(muted)).width(width));
    }
    if subtypes.is_some() {
        head = head.push(container(label::caption("Alt tür").style(muted)).width(width));
    }
    let mut rows = Column::new().spacing(4).push(head);
    for (r, combination) in g.rows.iter().enumerate() {
        let mut line = Row::new().spacing(6).align_y(Center);
        for (c, f) in cols.iter().enumerate() {
            let listed: Vec<(String, String)> = f
                .values
                .iter()
                .flatten()
                .filter(|v| !v.retired)
                .map(|v| (v.code.clone(), v.label.clone()))
                .collect();
            let mut choices = vec![Choice::new("Herhangi"), Choice::new("Boş")];
            choices.extend(listed.iter().map(|(_, l)| Choice::new(l.clone())));
            let now = match combination.cells.get(c) {
                Some(Cell::Any) | None => Some(0),
                Some(Cell::Empty) => Some(1),
                Some(Cell::Code { code }) => {
                    listed.iter().position(|(x, _)| x == code).map(|i| i + 2)
                }
            };
            let codes: Vec<String> = listed.iter().map(|(x, _)| x.clone()).collect();
            line = line.push(
                container(Select::new(choices, now, move |i| {
                    let cell = match i {
                        0 => Cell::Any,
                        1 => Cell::Empty,
                        i => Cell::Code {
                            code: codes[i - 2].clone(),
                        },
                    };
                    msg(Event::Cell(r, c, cell))
                }))
                .width(width),
            );
        }
        if let Some(s) = subtypes {
            let types: Vec<(String, String)> = s
                .types
                .iter()
                .map(|t| (t.code.clone(), s.label_of(fields, t).to_owned()))
                .collect();
            let mut choices = vec![Choice::new("Hepsi")];
            choices.extend(types.iter().map(|(_, l)| Choice::new(l.clone())));
            let now = match &combination.subtype {
                None => Some(0),
                Some(code) => types.iter().position(|(x, _)| x == code).map(|i| i + 1),
            };
            let codes: Vec<String> = types.iter().map(|(x, _)| x.clone()).collect();
            line = line.push(
                container(Select::new(choices, now, move |i| {
                    msg(Event::RowSubtype(
                        r,
                        i.checked_sub(1).map(|i| codes[i].clone()),
                    ))
                }))
                .width(width),
            );
        }
        line = line.push(kentos_ui::widget::tip(
            button(icon(from_web(Some("close"))).size(14.0))
                .style(style::button::ghost)
                .padding(kentos_ui::theme::metrics::icon_small(14.0))
                .on_press(msg(Event::RemoveRow(r))),
            kentos_ui::widget::Tip::new("Bileşimi sil"),
            iced::widget::tooltip::Position::Top,
        ));
        rows = rows.push(line);
    }
    column![
        row![
            words::field("Ad", container(name).width(280), None),
            container(words::check(
                g.restrictive,
                "Kısıtlayıcı",
                Some(msg(Event::Restrictive(!g.restrictive))),
            ))
            .padding(iced::Padding {
                top: 18.0,
                ..iced::Padding::ZERO
            }),
        ]
        .spacing(16),
        words::field(
            "Alanlar",
            chips.wrap(),
            Some("Gruba giren değer listeli alanlar; sırası bileşimlerin sütunlarıdır.".into()),
        ),
        label::strong("Bileşimler"),
        container(scrollable(rows).height(typography::scaled(220.0))).width(Fill),
        row![
            words::secondary(ADD_ROW, (g.fields.len() >= 2).then(|| msg(Event::AddRow))),
            label::caption(if g.restrictive {
                "Kısıtlayıcı: başka bileşim yazılmaz; formun ve tablonun seçenekleri daralır."
            } else {
                "Uyarır: başka bileşim yazılır, uyarılır."
            })
            .style(muted),
        ]
        .spacing(10)
        .align_y(Center),
    ]
    .spacing(10)
    .into()
}

fn muted(theme: &iced::Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(Tokens::of(theme).muted),
    }
}
