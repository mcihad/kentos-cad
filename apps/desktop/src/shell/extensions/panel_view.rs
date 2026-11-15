//! The dock's Eklenti tab drawn (docs/adr/8057 §24): the panel shown, its components as KentOS draws them, the open
//! panels' tabs above it; a command's rows as a table, a tree or cards. What the panels hold and what the user does with
//! them are panels.rs's.

use iced::widget::{column, container, row, scrollable};
use iced::{Element, Fill};
use kentos_extensions::ui_panel::Component;
use kentos_ui::label;
use kentos_ui::widget::Form;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::tabs::{Tab, Tabs};
use serde_json::Value;

use super::panels::{OpenPanel, PanelEvent, SCROLLER, number_words};
use super::{Event, msg};
use crate::app::{App, Message};
use crate::exchange::words;

/// Rows a table shows; the rest are counted.
const SHOWN_ROWS: usize = 500;
/// Cards a card component shows (each is a few rows tall); the rest are counted.
const SHOWN_CARDS: usize = 100;

impl App {
    /// The dock's Eklenti tab: the panel shown, its components as KentOS draws them; the open panels' tabs above it
    /// when there are several.
    pub(crate) fn extension_panel_view(&self) -> Element<'_, Message> {
        let Some(p) = self.extensions.panel() else {
            return container(label::muted(
                "Açık eklenti paneli yok: bir eklentinin sekmesindeki panel düğmesiyle açılır.",
            ))
            .padding(12)
            .into();
        };
        let name = self.extension_name(&p.ext);
        let why = self
            .extensions
            .python_reasons
            .get(&p.ext)
            .and_then(Option::as_deref);
        let event = |e: PanelEvent| msg(Event::Panel(e));
        // Enter in a text or number field presses the panel's primary button, as a form's.
        let enter = p
            .default_button()
            .filter(|_| p.running.is_none() && why.is_none())
            .map(|b| event(PanelEvent::Press(b.to_owned())));
        let mut form = Form::new().label_width(kentos_ui::theme::typography::scaled(110.0));
        let mut tables: Vec<Element<'_, Message>> = Vec::new();
        for c in &p.definition.body {
            match c {
                // A text the panel's whole width: in the field column it wrapped every few words in a narrow dock.
                Component::Text { text, .. } => form = form.wide(label::caption(text.clone())),
                Component::TextField {
                    id,
                    label: words,
                    placeholder,
                    ..
                } => {
                    let id = id.clone();
                    let value = p.texts.get(&id).map_or("", String::as_str);
                    let mut field =
                        kentos_ui::widget::text_field(placeholder.as_deref().unwrap_or(""), value)
                            .on_input(move |t| event(PanelEvent::Text(id.clone(), t)));
                    if let Some(press) = enter.clone() {
                        field = field.on_submit(press);
                    }
                    form = form.field(words.clone(), field);
                }
                Component::NumberField {
                    id,
                    label: words,
                    unit,
                    ..
                } => {
                    let id = id.clone();
                    let value = p.texts.get(&id).map_or("", String::as_str);
                    let mut field = kentos_ui::widget::input_field::InputField::new("", value)
                        .on_input(move |t| event(PanelEvent::Text(id.clone(), t)));
                    if let Some(unit) = unit {
                        field = field.suffix(unit.clone());
                    }
                    if let Some(press) = enter.clone() {
                        field = field.on_submit(press);
                    }
                    form = form.field(words.clone(), field);
                }
                Component::DateField {
                    id,
                    label: words,
                    on_change,
                    ..
                } => {
                    use kentos_ui::attribute::Date;
                    let typed = p.texts.get(id).map_or("", String::as_str);
                    let day = kentos_extensions::ui_panel::typed_date(typed)
                        .and_then(|iso| kentos_extensions::ui_panel::iso_date(&iso))
                        .and_then(|(y, m, d)| Date::new(y as i32, m as u8, d as u8));
                    let (typing, picking) = (id.clone(), id.clone());
                    let mut field = kentos_ui::widget::text_field("GG.AA.YYYY", typed)
                        .on_input(move |t| event(PanelEvent::Text(typing.clone(), t)));
                    // Enter applies a typed date when it runs a command on change (a panel may have no button to
                    // press: Parsel kartı's cards came only from the calendar), else presses the primary button.
                    let submit = match on_change {
                        Some(_) => Some(event(PanelEvent::Picked(id.clone(), typed.to_owned()))),
                        None => enter.clone(),
                    };
                    if let Some(message) = submit {
                        field = field.on_submit(message);
                    }
                    let picker =
                        kentos_ui::widget::DatePicker::date(day, move |d: Option<Date>| {
                            event(PanelEvent::Picked(
                                picking.clone(),
                                d.map_or_else(String::new, |d| {
                                    format!("{:02}.{:02}.{:04}", d.day(), d.month(), d.year())
                                }),
                            ))
                        })
                        .anchor(field)
                        .clearable(true)
                        .now(local_now());
                    form = form.field(words.clone(), picker);
                }
                // A time of day: typed (14:30, 14.30, 9:05) or chosen from KentOS's time picker, an hour then minutes.
                Component::TimeField {
                    id,
                    label: words,
                    on_change,
                    ..
                } => {
                    use kentos_ui::attribute::Time;
                    let typed = p.texts.get(id).map_or("", String::as_str);
                    let time = kentos_extensions::ui_panel::typed_time(typed)
                        .and_then(|t| kentos_extensions::ui_panel::iso_time(&t))
                        .and_then(|(h, m)| Time::new(h as u8, m as u8, 0));
                    let (typing, picking) = (id.clone(), id.clone());
                    let mut field = kentos_ui::widget::text_field("SS:DD", typed)
                        .on_input(move |t| event(PanelEvent::Text(typing.clone(), t)));
                    // Enter as a date field's.
                    let submit = match on_change {
                        Some(_) => Some(event(PanelEvent::Picked(id.clone(), typed.to_owned()))),
                        None => enter.clone(),
                    };
                    if let Some(message) = submit {
                        field = field.on_submit(message);
                    }
                    let picker =
                        kentos_ui::widget::TimePicker::new(time, move |t: Option<Time>| {
                            event(PanelEvent::Picked(
                                picking.clone(),
                                t.map_or_else(String::new, |t| {
                                    format!("{:02}:{:02}", t.hour(), t.minute())
                                }),
                            ))
                        })
                        .anchor(field)
                        .clearable(true)
                        .now(local_now());
                    form = form.field(words.clone(), picker);
                }
                // A day and a time: typed (15.11.2026 14:30) or chosen from the calendar and the clock side by side.
                Component::DateTimeField {
                    id,
                    label: words,
                    on_change,
                    ..
                } => {
                    use kentos_ui::attribute::{Date, DateTime, Time};
                    let typed = p.texts.get(id).map_or("", String::as_str);
                    let moment = kentos_extensions::ui_panel::typed_date_time(typed)
                        .and_then(|v| kentos_extensions::ui_panel::iso_date_time(&v))
                        .and_then(|((y, mo, d), (h, mi))| {
                            Some(DateTime::new(
                                Date::new(y as i32, mo as u8, d as u8)?,
                                Time::new(h as u8, mi as u8, 0)?,
                            ))
                        });
                    let (typing, picking) = (id.clone(), id.clone());
                    let mut field = kentos_ui::widget::text_field("GG.AA.YYYY SS:DD", typed)
                        .on_input(move |t| event(PanelEvent::Text(typing.clone(), t)));
                    let submit = match on_change {
                        Some(_) => Some(event(PanelEvent::Picked(id.clone(), typed.to_owned()))),
                        None => enter.clone(),
                    };
                    if let Some(message) = submit {
                        field = field.on_submit(message);
                    }
                    let picker = kentos_ui::widget::DatePicker::date_time(
                        moment,
                        move |v: Option<DateTime>| {
                            event(PanelEvent::Picked(
                                picking.clone(),
                                v.map_or_else(String::new, |v| {
                                    format!(
                                        "{:02}.{:02}.{:04} {:02}:{:02}",
                                        v.date.day(),
                                        v.date.month(),
                                        v.date.year(),
                                        v.time.hour(),
                                        v.time.minute()
                                    )
                                }),
                            ))
                        },
                    )
                    .anchor(field)
                    .clearable(true)
                    .now(local_now());
                    form = form.field(words.clone(), picker);
                }
                Component::Select {
                    id,
                    label: words,
                    options,
                    ..
                } => {
                    let at = options
                        .iter()
                        .position(|o| p.choices.get(id) == Some(&o.value));
                    let (id, values): (String, Vec<String>) = (
                        id.clone(),
                        options.iter().map(|o| o.value.clone()).collect(),
                    );
                    let pick = Select::new(
                        options.iter().map(|o| Choice::new(o.label.clone())),
                        at,
                        move |i| event(PanelEvent::Choose(id.clone(), values[i].clone())),
                    );
                    form = form.field(words.clone(), pick);
                }
                // KentOS's chips, each an option that toggles.
                Component::MultiSelect {
                    id,
                    label: words,
                    options,
                    ..
                } => {
                    let chosen = p.multi.get(id);
                    let chips = kentos_ui::widget::chips::toggles(options.iter().map(|o| {
                        (
                            o.label.clone(),
                            chosen.is_some_and(|c| c.contains(&o.value)),
                            event(PanelEvent::Toggle(id.clone(), o.value.clone())),
                        )
                    }));
                    form = form.field(words.clone(), chips);
                }
                Component::LayerPicker {
                    id,
                    label: words,
                    geometry,
                    ..
                } => {
                    let layers = self.panel_layers(geometry.as_deref());
                    let at = layers
                        .iter()
                        .position(|(l, _)| p.choices.get(id) == Some(l));
                    let (id, ids): (String, Vec<String>) =
                        (id.clone(), layers.iter().map(|(l, _)| l.clone()).collect());
                    let pick = Select::new(
                        layers.into_iter().map(|(_, name)| Choice::new(name)),
                        at,
                        move |i| event(PanelEvent::Choose(id.clone(), ids[i].clone())),
                    );
                    form = form.field(words.clone(), pick);
                }
                Component::FieldPicker {
                    id,
                    label: words,
                    layer,
                    kinds,
                    ..
                } => {
                    let fields = self.panel_fields(p.layer_of(Some(layer)), kinds);
                    let at = fields
                        .iter()
                        .position(|(f, _)| p.choices.get(id) == Some(f));
                    let (id, names): (String, Vec<String>) =
                        (id.clone(), fields.iter().map(|(f, _)| f.clone()).collect());
                    let pick = Select::new(
                        fields.into_iter().map(|(_, words)| Choice::new(words)),
                        at,
                        move |i| event(PanelEvent::Choose(id.clone(), names[i].clone())),
                    );
                    form = form.field(words.clone(), pick);
                }
                Component::FeaturePicker {
                    id, label: words, ..
                } => {
                    // Two lines: beside both buttons the count had no room in a narrow dock and Temizle ran past its
                    // edge (real window, 14 November 2026).
                    let picked = p.picks.get(id).map_or(0, Vec::len);
                    let count: Element<'_, Message> = if picked == 0 {
                        label::muted("Seçilmedi").into()
                    } else {
                        label::caption(format!("{picked} nesne")).into()
                    };
                    let mut line = row![count].spacing(8).align_y(iced::Center);
                    if picked > 0 {
                        line = line.push(words::ghost(
                            "Temizle",
                            Some(event(PanelEvent::Unpick(id.clone()))),
                        ));
                    }
                    let cell = column![
                        line,
                        words::secondary("Seçimi al", Some(event(PanelEvent::Pick(id.clone())))),
                    ]
                    .spacing(4);
                    form = form.field(words.clone(), cell);
                }
                Component::Checkbox {
                    id, label: words, ..
                } => {
                    let on = p.checks.get(id).copied().unwrap_or(false);
                    form = form.row(words::check(
                        on,
                        words.clone(),
                        Some(event(PanelEvent::Check(id.clone(), !on))),
                    ));
                }
                Component::Button {
                    id,
                    label: words,
                    primary,
                    ..
                } => {
                    // Its caption stays while it runs: with "…" it broke onto two lines in a narrow dock and the form
                    // jumped (real window, 15 November 2026); the run's line under the form names it.
                    let on = (p.running.is_none() && why.is_none())
                        .then(|| event(PanelEvent::Press(id.clone())));
                    form = form.row(if *primary {
                        words::primary(words.clone(), on)
                    } else {
                        words::secondary(words.clone(), on)
                    });
                }
                Component::Table {
                    id, label: words, ..
                } => {
                    tables.push(self.panel_table(p, id, words.as_deref()));
                }
                Component::Tree {
                    id,
                    label: words,
                    group,
                } => {
                    tables.push(panel_tree(p, id, words.as_deref(), group));
                }
                Component::Stats {
                    id,
                    label: words,
                    title,
                    value,
                    unit,
                    note,
                } => {
                    tables.push(panel_stats(
                        p,
                        id,
                        words.as_deref(),
                        [title, value, unit, note].map(|c| c.as_deref()),
                    ));
                }
                Component::Card {
                    id,
                    label: words,
                    title,
                    tag,
                } => {
                    tables.push(panel_cards(
                        p,
                        id,
                        words.as_deref(),
                        title.as_deref(),
                        tag.as_deref(),
                    ));
                }
            }
        }
        let header = row![
            column![
                label::strong(p.definition.title.clone()),
                label::caption(name)
            ]
            .width(Fill),
            words::ghost(
                "Kapat",
                Some(event(PanelEvent::Close(self.extensions.active_panel))),
            ),
        ]
        .spacing(8);
        let mut body = column![header, form].spacing(12).padding(12).width(Fill);
        // Its button's command running: the button, how far it is from its ctx.progress (moving until it says), and
        // Durdur. A change's command runs without a button.
        if let Some(run) = self
            .extensions
            .runners
            .get(&p.ext)
            .and_then(|r| r.run.as_ref())
            .filter(|run| run.origin.as_ref().is_some_and(|o| o.panel == p.local))
        {
            let button = p.definition.body.iter().find_map(|c| match c {
                Component::Button { id, label, .. } if p.running.as_deref() == Some(id) => {
                    Some(label.as_str())
                }
                _ => None,
            });
            let (share, said) = run.progress.as_ref().map_or((None, None), |progress| {
                let (share, said) = super::runner::progress_words(progress);
                (share, Some(said))
            });
            let text = match (button, said) {
                (Some(button), Some(said)) => format!("{button} · {said}"),
                (Some(button), None) => format!("“{button}” çalışıyor…"),
                (None, Some(said)) => said,
                (None, None) => "Çalışıyor…".to_owned(),
            };
            let stop: Element<'_, Message> = if run.stopping.is_some() {
                label::muted("Durduruluyor…").into()
            } else {
                words::ghost("Durdur", Some(msg(Event::StopRun(p.ext.clone()))))
            };
            body = body.push(
                column![
                    kentos_ui::widget::progress::bar(share).width(Fill),
                    row![container(label::caption(text)).width(Fill), stop]
                        .spacing(8)
                        .align_y(iced::Center),
                ]
                .spacing(6),
            );
        }
        if let Some(why) = why {
            body = body.push(words::line(
                words::Kind::Warn,
                label::caption(format!("Düğmeler komut çalıştırır: {why}")),
            ));
        }
        if let Some((kind, said)) = &p.said {
            body = body.push(words::line(*kind, label::caption(said.clone())));
        }
        for table in tables {
            body = body.push(table);
        }
        // Its own scroller (the pane's would take the tabs along), as far down as the panel was left.
        let content = scrollable(body)
            .id(iced::widget::Id::new(SCROLLER))
            .height(Fill)
            .on_scroll(move |v| event(PanelEvent::Scrolled(v.absolute_offset().y)));
        if self.extensions.panels.len() < 2 {
            return content.into();
        }
        // Several open: a tab each, as the drawings' (a dot: its command runs, or it answered while another was shown).
        let tabs = Tabs::new(
            self.extensions.panels.iter().map(|p| {
                Tab::new(p.definition.title.as_str()).note(p.running.is_some() || p.unseen)
            }),
            self.extensions.active_panel,
            move |at| event(PanelEvent::Activate(at)),
        )
        .on_close(move |at| event(PanelEvent::Close(at)))
        .on_reorder(move |from, to| event(PanelEvent::Move(from, to)));
        column![tabs, content].into()
    }

    /// The open drawing's layers a layer picker offers: none ("—") first, then each layer (groups give their names to
    /// the path) of `geometry`'s type or untyped.
    fn panel_layers(&self, geometry: Option<&str>) -> Vec<(String, String)> {
        fn walk(
            nodes: &[kentos_commands::LayerNode],
            path: &str,
            geometry: Option<&str>,
            out: &mut Vec<(String, String)>,
        ) {
            for n in nodes {
                let name = if path.is_empty() {
                    n.name.clone()
                } else {
                    format!("{path} › {}", n.name)
                };
                if n.kind == kentos_commands::LayerNodeType::Group {
                    walk(&n.children, &name, geometry, out);
                    continue;
                }
                let fits = match (geometry, n.geometry) {
                    (None, _) | (_, None) => true,
                    (Some(want), Some(kind)) => serde_json::to_value(kind)
                        .ok()
                        .and_then(|v| v.as_str().map(|k| k == want))
                        .unwrap_or(false),
                };
                if fits {
                    out.push((n.id.clone(), name));
                }
            }
        }
        let mut out = vec![(String::new(), "—".to_owned())];
        if let Some(doc) = &self.document {
            walk(doc.model.layers().nodes(), "", geometry, &mut out);
        }
        out
    }

    /// The fields a field picker offers: none ("—") first, then the fields of `layer` (of `kinds` when given), each by
    /// its alias and name; only none while no layer is chosen.
    pub(crate) fn panel_fields(
        &self,
        layer: Option<&str>,
        kinds: &[String],
    ) -> Vec<(String, String)> {
        let mut out = vec![(String::new(), "—".to_owned())];
        let node = layer.and_then(|l| self.document.as_ref()?.model.layers().get(l));
        for f in node.into_iter().flat_map(|n| &n.fields) {
            let kind = serde_json::to_value(f.kind).ok();
            if !kinds.is_empty()
                && !kinds
                    .iter()
                    .any(|k| kind.as_ref().and_then(Value::as_str) == Some(k))
            {
                continue;
            }
            let words = match f
                .alias
                .as_deref()
                .filter(|a| !a.trim().is_empty() && *a != f.name)
            {
                Some(alias) => format!("{alias} ({})", f.name),
                None => f.name.clone(),
            };
            out.push((f.name.clone(), words));
        }
        out
    }

    /// A table's rows, the first [`SHOWN_ROWS`]; empty until its button's command answered.
    fn panel_table<'a>(
        &self,
        p: &'a OpenPanel,
        id: &str,
        words: Option<&'a str>,
    ) -> Element<'a, Message> {
        use kentos_ui::widget::table::{Column as Col, Row, Table};
        let Some((columns, rows)) = p.tables.get(id) else {
            return column![
                label::caption(words.unwrap_or("Sonuçlar").to_owned()),
                label::muted(p.waiting_words(id, "listelenir")),
            ]
            .spacing(4)
            .into();
        };
        // No rows: no columns either (a command's rows name them), so no empty header bar.
        if rows.is_empty() {
            return column![
                label::caption(words.unwrap_or("Sonuçlar").to_owned()),
                label::muted("Satır yok."),
            ]
            .spacing(4)
            .into();
        }
        // A row's `uid` names its object: a click shows it; it is not a column.
        let shown: Vec<&String> = columns.iter().filter(|c| c.as_str() != "uid").collect();
        let mut table = Table::new(shown.iter().map(|c| Col::new((*c).clone()).width(Fill)))
            .empty("Satır yok.");
        for row in rows.iter().take(SHOWN_ROWS) {
            let mut line = Row::new(
                shown
                    .iter()
                    .map(|c| label::caption(cell_words(row.get(c.as_str()))).into()),
            );
            if let Some(uid) = row.get("uid").and_then(Value::as_str) {
                line = line.on_press(msg(Event::Panel(PanelEvent::Show(uid.to_owned()))));
            }
            table = table.push(line);
        }
        let mut out = column![
            label::caption(words.unwrap_or("Sonuçlar").to_owned()),
            table
        ]
        .spacing(4);
        if rows.len() > SHOWN_ROWS {
            out = out.push(label::muted(format!(
                "… {} satır daha",
                rows.len() - SHOWN_ROWS
            )));
        }
        out.into()
    }
}

/// A cell's words.
fn cell_words(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "—".to_owned(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => if *b { "evet" } else { "hayır" }.to_owned(),
        Some(Value::Number(n)) => n.as_f64().map_or_else(|| n.to_string(), number_words),
        Some(other) => other.to_string(),
    }
}

/// Rows grouped by a column's words, each group in the order its first row came.
fn group_rows<'a>(rows: &[&'a Value], column: &str) -> Vec<(String, Vec<&'a Value>)> {
    let mut out: Vec<(String, Vec<&'a Value>)> = Vec::new();
    for row in rows {
        let value = cell_words(row.get(column));
        match out.iter_mut().find(|(v, _)| *v == value) {
            Some((_, members)) => members.push(*row),
            None => out.push((value, vec![*row])),
        }
    }
    out
}

/// A tree's rows grouped by `group`'s columns (KentOS's tree view): each group in the order its first row came, its
/// count beside it, open unless the user closed it; a leaf the row's first other column, its object shown when
/// clicked. Rows past [`SHOWN_ROWS`] are counted.
fn panel_tree<'a>(
    p: &'a OpenPanel,
    id: &str,
    words: Option<&'a str>,
    group: &'a [String],
) -> Element<'a, Message> {
    use kentos_ui::widget::tree_view::{Column as Col, Node, TreeView};
    let title = label::caption(words.unwrap_or("Sonuçlar").to_owned());
    let Some((columns, rows)) = p.tables.get(id) else {
        return column![title, label::muted(p.waiting_words(id, "gruplanır")),]
            .spacing(4)
            .into();
    };
    if rows.is_empty() {
        return column![title, label::muted("Satır yok.")].spacing(4).into();
    }
    // A leaf's columns: the rows' own but the grouped ones and its uid; the first names the leaf.
    let leaf: Vec<&String> = columns
        .iter()
        .filter(|c| c.as_str() != "uid" && !group.contains(c))
        .collect();
    let mut cols = vec![Col::new(group[0].clone()).width(Fill)];
    cols.extend(
        leaf.iter()
            .skip(1)
            .map(|c| Col::new((*c).clone()).width(Fill)),
    );
    cols.push(Col::new("Sayı").width(40).align_right());
    let shown: Vec<&Value> = rows.iter().take(SHOWN_ROWS).collect();
    fn nodes<'a>(
        tree: &str,
        rows: &[&'a Value],
        group: &'a [String],
        leaf: &[&'a String],
        depth: usize,
        path: &str,
        folded: &std::collections::BTreeSet<String>,
    ) -> Vec<Node<'a, Message>> {
        if depth == group.len() {
            return rows
                .iter()
                .map(|row| {
                    let name = leaf
                        .first()
                        .map_or_else(|| "—".to_owned(), |c| cell_words(row.get(c.as_str())));
                    let mut cells: Vec<Element<'a, Message>> = leaf
                        .iter()
                        .skip(1)
                        .map(|c| label::caption(cell_words(row.get(c.as_str()))).into())
                        .collect();
                    cells.push(iced::widget::space::horizontal().width(0).into());
                    let mut node = Node::new(name).cells(cells);
                    if let Some(uid) = row.get("uid").and_then(Value::as_str) {
                        node = node.on_press(msg(Event::Panel(PanelEvent::Show(uid.to_owned()))));
                    }
                    node
                })
                .collect();
        }
        group_rows(rows, group[depth].as_str())
            .into_iter()
            .map(|(value, members)| {
                let key = format!("{tree}/{path}{value}");
                let open = !folded.contains(&key);
                let mut cells: Vec<Element<'a, Message>> = (1..leaf.len().max(1))
                    .map(|_| iced::widget::space::horizontal().width(0).into())
                    .collect();
                cells.push(label::caption(members.len().to_string()).into());
                let mut node = Node::new(value.clone())
                    .folder()
                    .cells(cells)
                    .expanded(open, msg(Event::Panel(PanelEvent::Fold(key))));
                if open {
                    node = node.extend(nodes(
                        tree,
                        &members,
                        group,
                        leaf,
                        depth + 1,
                        &format!("{path}{value}/"),
                        folded,
                    ));
                }
                node
            })
            .collect()
    }
    let tree = TreeView::new(cols)
        .header(leaf.len() > 1)
        .extend(nodes(id, &shown, group, &leaf, 0, "", &p.folded));
    let mut out = column![title, tree].spacing(4);
    if rows.len() > SHOWN_ROWS {
        out = out.push(label::muted(format!(
            "… {} satır daha",
            rows.len() - SHOWN_ROWS
        )));
    }
    out.into()
}

/// A card component's rows, a card each, the first [`SHOWN_CARDS`] (KentOS's info card, on the card surface):
/// `title`'s column heads it (none: the first column), `tag`'s beside it, the other columns under it by name, a
/// number in figures; a row with a `uid` is a card pressed to show its object, as a table's row.
fn panel_cards<'a>(
    p: &'a OpenPanel,
    id: &str,
    words: Option<&'a str>,
    title: Option<&'a str>,
    tag: Option<&'a str>,
) -> Element<'a, Message> {
    use iced::widget::button;
    use iced::widget::text::Wrapping;
    use kentos_ui::style;
    use kentos_ui::theme::typography;
    use kentos_ui::widget::{InfoCard, Pairs};
    let heading = label::caption(words.unwrap_or("Sonuçlar").to_owned());
    let Some((columns, rows)) = p.tables.get(id) else {
        return column![
            heading,
            label::muted(p.waiting_words(id, "kart kart listelenir")),
        ]
        .spacing(4)
        .into();
    };
    if rows.is_empty() {
        return column![heading, label::muted("Kart yok.")]
            .spacing(4)
            .into();
    }
    let shown: Vec<&str> = columns
        .iter()
        .map(String::as_str)
        .filter(|c| *c != "uid")
        .collect();
    let head = title.or_else(|| shown.first().copied());
    let lines: Vec<&str> = shown
        .iter()
        .copied()
        .filter(|c| Some(*c) != head && Some(*c) != tag)
        .collect();
    let size = typography::caption();
    let mut out = column![heading].spacing(6);
    for row in rows.iter().take(SHOWN_CARDS) {
        let name = head.map_or_else(|| "—".to_owned(), |c| cell_words(row.get(c)));
        // As wide as the panel, every card the same.
        let mut card = InfoCard::new(label::strong(name).wrapping(Wrapping::WordOrGlyph))
            .min_width(f32::INFINITY);
        if let Some(tag) = tag {
            card =
                card.tag(label::caption(cell_words(row.get(tag))).wrapping(Wrapping::WordOrGlyph));
        }
        if !lines.is_empty() {
            card = card.body(
                lines.iter().fold(
                    Pairs::new()
                        .spacing_x(12.0)
                        .spacing_y(2.0)
                        .align_values(iced::Right),
                    |pairs, c| {
                        let value = row.get(*c);
                        let text = if value.is_some_and(Value::is_number) {
                            label::mono(cell_words(value))
                        } else {
                            label::body(cell_words(value))
                        };
                        pairs.push(
                            label::caption((*c).to_owned()).wrapping(Wrapping::None),
                            text.size(size)
                                .align_x(iced::Right)
                                .wrapping(Wrapping::WordOrGlyph),
                        )
                    },
                ),
            );
        }
        let inside = container(card).padding([8, 10]).width(Fill);
        out = out.push(match row.get("uid").and_then(Value::as_str) {
            Some(uid) => Element::from(
                button(inside)
                    .on_press(msg(Event::Panel(PanelEvent::Show(uid.to_owned()))))
                    .padding(0)
                    .width(Fill)
                    .style(style::button::card(false)),
            ),
            None => inside.style(style::container::bordered).into(),
        });
    }
    if rows.len() > SHOWN_CARDS {
        out = out.push(label::muted(format!(
            "… {} kart daha",
            rows.len() - SHOWN_CARDS
        )));
    }
    out.into()
}

/// A stats component's rows, a figure each (KentOS's stat card), side by side as the panel's width allows: `columns`
/// names its title (none: the first column), its figure (none: the second), its unit and its note; a row with a `uid`
/// is a card pressed to show its object. The first [`SHOWN_CARDS`].
fn panel_stats<'a>(
    p: &'a OpenPanel,
    id: &str,
    words: Option<&'a str>,
    columns: [Option<&'a str>; 4],
) -> Element<'a, Message> {
    use kentos_ui::widget::StatCard;
    let heading = label::caption(words.unwrap_or("Sonuçlar").to_owned());
    let Some((names, rows)) = p.tables.get(id) else {
        return column![
            heading,
            label::muted(p.waiting_words(id, "gösterge kartı olur"))
        ]
        .spacing(4)
        .into();
    };
    if rows.is_empty() {
        return column![heading, label::muted("Gösterge yok.")]
            .spacing(4)
            .into();
    }
    let shown: Vec<&str> = names
        .iter()
        .map(String::as_str)
        .filter(|c| *c != "uid")
        .collect();
    let [title, value, unit, note] = columns;
    let title = title.or_else(|| shown.first().copied());
    let value = value.or_else(|| shown.get(1).copied());
    // A cell the row gives: a unit or a note it leaves out is not written as "—".
    let given = |row: &'a Value, column: Option<&str>| {
        column
            .and_then(|c| row.get(c))
            .filter(|v| !v.is_null())
            .map(|v| cell_words(Some(v)))
    };
    let mut tiles = iced::widget::Row::new().spacing(8);
    for row in rows.iter().take(SHOWN_CARDS) {
        let mut card = StatCard::new(
            given(row, title).unwrap_or_else(|| "—".to_owned()),
            given(row, value).unwrap_or_else(|| "—".to_owned()),
        )
        // Two side by side in a side panel of 300 px (at 132 one card took each line, real window).
        .width(iced::Length::Fixed(kentos_ui::theme::typography::scaled(
            124.0,
        )));
        if let Some(unit) = given(row, unit) {
            card = card.unit(unit);
        }
        if let Some(note) = given(row, note) {
            card = card.note(note);
        }
        if let Some(uid) = row.get("uid").and_then(Value::as_str) {
            card = card.on_press(msg(Event::Panel(PanelEvent::Show(uid.to_owned()))));
        }
        tiles = tiles.push(card);
    }
    column![heading, tiles.wrap().vertical_spacing(8)]
        .spacing(6)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_trees_groups_keep_the_order_of_their_first_row_and_count_them() {
        let rows = [
            json!({ "uid": "a", "Değer": "102", "Nesne": "parsel=1" }),
            json!({ "uid": "b", "Değer": "101", "Nesne": "parsel=1" }),
            json!({ "uid": "c", "Değer": "102", "Nesne": "parsel=2" }),
            json!({ "uid": "d", "Nesne": "parsel=3" }),
        ];
        let refs: Vec<&Value> = rows.iter().collect();
        let groups = group_rows(&refs, "Değer");
        let summary: Vec<(&str, usize)> =
            groups.iter().map(|(v, m)| (v.as_str(), m.len())).collect();
        assert_eq!(
            summary,
            [("102", 2), ("101", 1), ("—", 1)],
            "no value: its own group"
        );
        // A tree's group closed and opened again.
        let value = json!({
            "format": "kentos.ui.panel", "schemaVersion": 1, "title": "Ağaç",
            "body": [
                { "type": "button", "id": "say", "label": "Say", "command": "deger-say" },
                { "type": "tree", "id": "agac", "group": ["Değer"] }
            ]
        });
        let p = OpenPanel::new("x", "y", kentos_extensions::ui_panel::read(&value).unwrap());
        assert_eq!(
            p.table_of(None).as_deref(),
            Some("agac"),
            "a button's rows go to the tree"
        );
    }
}

/// Now as this computer's clock has it: the pickers' today and Şimdi (the forms' too).
fn local_now() -> kentos_ui::attribute::DateTime {
    kentos_ui::attribute::DateTime::now(
        kentos_interaction::time_version::LOCAL_OFFSET.load(std::sync::atomic::Ordering::Relaxed)
            / 60,
    )
}
