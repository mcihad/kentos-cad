//! How a processing tool's window looks (the web's `ToolDialog` and
//! `processing.css`): the form on the left in groups (Girdi, Ayarlar,
//! Çıktı, a folding Gelişmiş ayarlar), each field with its label,
//! description and problem; on the right what the tool does, its help, a
//! preview, where it runs and its command-line names; at the bottom
//! Varsayılanlar, what the last run did, Kapat and Çalıştır.

use iced::widget::{Column, Row, button, column, container, row, scrollable, space};
use iced::{Alignment, Center, Element, Fill, Length};
use kentos_interaction::Format;
use kentos_processing::model_runner::MODEL_PREFIX;
use kentos_processing::parameters::is_visible;
use kentos_processing::{ParamDef, ParamKind};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::progress::{self, Tint};
use kentos_ui::widget::radio::RadioGroup;
use kentos_ui::widget::table::{self, Table};
use kentos_ui::widget::{Dialog, Menu, MenuButton, Tip, horizontal_divider, overlay, tip};

use super::dialog::ToolDialog;
use super::fields::{Env, control};
use super::plan::{self, Action, Choice, LineIcon, LineKind};
use super::{Event, Processing};
use crate::app::{App, Message};

fn ev(e: Event) -> Message {
    Message::Processing(e)
}

/// The form's scrollable: scrolled to its end after a run that gave a table.
pub(super) const FORM: &str = "islem-formu";
/// The window's size (the web's: 940 px wide, at most 660 tall).
const WIDTH: f32 = 940.0;
const HEIGHT: f32 = 660.0;
/// The right-hand panel and a field's control column.
const SIDE: f32 = 300.0;
/// The running run's bar (the web's `.ptool__progress`).
const PROGRESS: f32 = 140.0;

impl App {
    /// The open processing window, over the drawing.
    pub(crate) fn processing_view(&self) -> Element<'_, Message> {
        // Toplu çalıştır: the same window with the table in the form's place (batch.rs).
        if self.processing.batch.is_some() {
            return self.batch_view();
        }
        let (Some(window), Some(doc)) = (&self.processing.dialog, &self.document) else {
            return space().into();
        };
        let format = Format::of(doc.settings());
        let palette = crate::viewport::palette(self.canvas());
        let color = move |c: &str| {
            palette
                .resolve(c)
                .map_or_else(|| crate::view::hex_color(c), crate::view::rgba_color)
        };
        let env = Env {
            window,
            send: Message::Processing,
            pick_objects: true,
            builder: true,
            doc: &doc.model,
            format: &format,
            color: &color,
        };
        let server = self.server_place();
        let at_server = self.processing_at_server();
        let body = row![
            scrollable(form(
                window,
                &env,
                at_server,
                self.shows_bottom_tab(crate::bottom::BottomTab::Chart),
            ))
            .id(FORM)
            .direction(style::field::body_scrollbar())
            .width(Fill)
            .height(Fill),
            container(
                scrollable(side(window, &self.processing, &server))
                    .direction(style::field::body_scrollbar())
                    .height(Fill)
            )
            .width(Length::Fixed(typography::scaled(SIDE)))
            .height(Fill)
            .style(style::container::header),
        ]
        .spacing(16)
        .height(Fill);
        overlay::modal(
            Dialog::new(window.tool.label.clone())
                .push(body)
                .push(horizontal_divider())
                .push(footer(window, at_server))
                .width(WIDTH)
                .max_height(HEIGHT),
            ev(Event::Close),
        )
    }
}

/// The form: Girdi, Ayarlar, Çıktı and Gelişmiş ayarlar.
fn form<'a>(
    window: &'a ToolDialog,
    env: &Env<'a, '_>,
    at_server: bool,
    chart: bool,
) -> Element<'a, Message> {
    let shown: Vec<&ParamDef> = window
        .tool
        .parameters
        .iter()
        .filter(|p| is_visible(p, &window.values))
        .collect();
    let is = |p: &&ParamDef, kind: &str| p.type_name() == kind;
    // Objects and lists of layers are what a tool reads (docs/adr/0217 §7).
    let input: Vec<&ParamDef> = shown
        .iter()
        .copied()
        .filter(|p| !p.advanced && (is(p, "features") || is(p, "layers")))
        .collect();
    let output: Vec<&ParamDef> = shown
        .iter()
        .copied()
        .filter(|p| !p.advanced && is(p, "layer"))
        .collect();
    let main: Vec<&ParamDef> = shown
        .iter()
        .copied()
        .filter(|p| !p.advanced && !is(p, "features") && !is(p, "layers") && !is(p, "layer"))
        .collect();
    let advanced: Vec<&ParamDef> = shown.iter().copied().filter(|p| p.advanced).collect();
    let open = window.advanced_open || advanced.iter().any(|p| window.issue_of(&p.name).is_some());
    let mut out = Column::new().spacing(18).padding([4, 8]);
    for (title, params) in [("Girdi", &input), ("Ayarlar", &main), ("Çıktı", &output)] {
        if !params.is_empty() {
            out = out.push(group(title, params, window, env));
        }
    }
    // Where the result goes here: the drawing, a GeoPackage or a database (sink.rs, docs/adr/0592 §3); on the server
    // the side column's Sonuç says it.
    if !at_server && !window.tool.id.starts_with(MODEL_PREFIX) {
        out = out.push(
            column![
                label::caption("Sonucun yeri")
                    .font(typography::ui_strong())
                    .style(style::text::muted),
                horizontal_divider(),
                container(super::sink::place_view(window)).padding([12, 0]),
            ]
            .spacing(6),
        );
    }
    // Analiz ortamı (environment.rs, docs/adr/0643 §5): the raster tools' area, cell size, snap raster and NoData.
    if window.takes_environment && !at_server {
        out = out.push(super::environment::section(window, env.doc, env.format));
    }
    if !advanced.is_empty() {
        let toggle = button(
            row![
                icon(if open {
                    Icon::ChevronDown
                } else {
                    Icon::ChevronRight
                })
                .size(14.0)
                .tone(Tone::Muted),
                label::caption("Gelişmiş ayarlar").font(typography::ui_strong()),
                container(label::mono_caption(advanced.len().to_string()))
                    .padding([0, 6])
                    .style(style::container::badge),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([0, 0])
        .width(Fill)
        .style(style::button::ghost)
        .on_press(ev(Event::Advanced));
        let mut section = column![toggle, horizontal_divider()].spacing(6);
        if open {
            section = section.push(rows(&advanced, window, env));
        }
        out = out.push(section);
    }
    if let super::RunStatus::Ok { table: Some(t), .. } = &window.status {
        out = out.push(result(t, chart));
    }
    out.into()
}

/// Whether a table cell is a number: a sign, digits and one decimal point or comma.
fn number_cell(text: &str) -> bool {
    let body = text.strip_prefix(['-', '+']).unwrap_or(text);
    let mut parts = body.splitn(2, ['.', ',']);
    let whole = parts.next().unwrap_or("");
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    digits(whole) && parts.next().is_none_or(digits)
}

/// The run's table under the form (Özet istatistik and the geometry tools'
/// reports, docs/adr/0200 §7, 0201): its title with Panoya kopyala and CSV
/// olarak kaydet, then the table; a column of numbers (every filled cell one)
/// right-aligned in figures, names and texts from the left. Grafik only where the program has its tab
/// (docs/adr/0802).
fn result(t: &super::dialog::ResultTable, chart: bool) -> Element<'_, Message> {
    let small = |glyph: &str, text: &'static str, e: Event| {
        button(
            row![
                icon(crate::icons::from_web(Some(glyph))).size(14.0),
                label::caption(text)
            ]
            .spacing(5)
            .align_y(Center),
        )
        .padding([3, 8])
        .style(style::button::ghost)
        .on_press(ev(e))
    };
    let numeric: Vec<bool> = (0..t.columns.len())
        .map(|i| {
            let cells = || {
                t.rows
                    .iter()
                    .filter_map(|r| r.get(i))
                    .filter(|c| !c.is_empty())
            };
            cells().next().is_some() && cells().all(|c| number_cell(c))
        })
        .collect();
    // Each column as wide as its widest text (the web's table sizes its columns so): the names in
    // the interface's face, the figures in mono.
    let size = typography::caption();
    let widths: Vec<f32> = t
        .columns
        .iter()
        .enumerate()
        .map(|(i, head)| {
            let text = !numeric[i];
            let widest = t
                .rows
                .iter()
                .filter_map(|r| r.get(i))
                .map(|c| {
                    if text {
                        typography::measured_width(c, size, false)
                    } else {
                        c.chars().count() as f32 * size * 0.62
                    }
                })
                .fold(typography::measured_width(head, size, true), f32::max);
            // The table puts its own spacing between the columns.
            widest + 6.0
        })
        .collect();
    let columns = t
        .columns
        .iter()
        .zip(&widths)
        .enumerate()
        .map(|(i, (c, w))| {
            let col = table::Column::new(c.as_str()).width(*w);
            if numeric[i] { col.align_right() } else { col }
        });
    let body: Vec<table::Row<'_, Message>> = t
        .rows
        .iter()
        .map(|r| {
            table::Row::new(r.iter().enumerate().map(|(i, c)| {
                if numeric.get(i).copied().unwrap_or(false) {
                    label::mono_caption(c.clone()).into()
                } else {
                    label::caption(c.clone()).into()
                }
            }))
        })
        .collect();
    let height = (28.0 * (t.rows.len() as f32 + 1.0) + 8.0).min(240.0);
    let mut head = row![
        container(
            label::caption("Sonuç")
                .font(typography::ui_strong())
                .style(style::text::muted)
        )
        .width(Fill),
    ]
    .spacing(4)
    .align_y(Center);
    if chart {
        head = head.push(tip(
            small("chart", "Grafik", Event::ChartTable),
            Tip::new("Grafik").body("Tabloyu alt panelin Grafik sekmesinde gösterir: ROC eğrisi, kesit, zaman serisi ve histogram kendi grafikleriyle, öbürleri çubukla."),
            iced::widget::tooltip::Position::Top,
        ));
    }
    column![
        head.push(
            row![
                small("copy", "Panoya kopyala", Event::CopyTable),
                // Kaydet ▾ (docs/adr/0223 §2): the table as XLSX, ODS or CSV.
                MenuButton::new(
                    container(
                        row![
                            icon(crate::icons::from_web(Some("export"))).size(14.0),
                            label::caption("Kaydet"),
                            icon(Icon::ChevronDown).size(12.0).tone(Tone::Muted),
                        ]
                        .spacing(5)
                        .align_y(Center),
                    )
                    .padding([3, 8]),
                    || crate::table_write::items(Menu::new(), |f| ev(Event::SaveTableAs(f))),
                ),
            ]
            .spacing(4)
            .align_y(Center)
        ),
        horizontal_divider(),
        container(Table::new(columns).extend(body).horizontal().height(height))
            .style(style::container::field_box)
            .width(Fill),
    ]
    .spacing(6)
    .into()
}

fn group<'a>(
    title: &'a str,
    params: &[&'a ParamDef],
    window: &'a ToolDialog,
    env: &Env<'a, '_>,
) -> Element<'a, Message> {
    column![
        label::caption(title)
            .font(typography::ui_strong())
            .style(style::text::muted),
        horizontal_divider(),
        rows(params, window, env),
    ]
    .spacing(6)
    .into()
}

fn rows<'a>(
    params: &[&'a ParamDef],
    window: &'a ToolDialog,
    env: &Env<'a, '_>,
) -> Element<'a, Message> {
    let mut list = Column::new();
    for (i, p) in params.iter().enumerate() {
        if i > 0 {
            list = list.push(horizontal_divider());
        }
        list = list.push(field_row(p, window, env));
    }
    list.into()
}

/// One parameter: its label and description, the control, its problem.
fn field_row<'a>(
    def: &'a ParamDef,
    window: &'a ToolDialog,
    env: &Env<'a, '_>,
) -> Element<'a, Message> {
    let mut heading = Row::new()
        .spacing(8)
        .align_y(Alignment::End)
        .push(label::body(def.label.clone()).font(typography::ui_strong()));
    if def.optional {
        heading = heading.push(label::caption("isteğe bağlı").style(style::text::muted));
    }
    let mut text = Column::new().spacing(2).push(heading);
    if let Some(d) = &def.description {
        text = text.push(label::caption(d.clone()).style(style::text::muted));
    }
    let mut control_col = Column::new().spacing(6).width(Fill).push(control(def, env));
    // A bound layer's input read whole from its source (sink.rs, docs/adr/0592 §3).
    if let Some(whole) = super::sink::whole_row(window, &def.name) {
        control_col = control_col.push(whole);
    }
    if let Some(issue) = window.issue_of(&def.name) {
        control_col = control_col.push(
            row![
                icon(Icon::Error).size(14.0).tone(Tone::Danger),
                label::caption(issue.message.clone()).style(style::text::danger)
            ]
            .spacing(6)
            .align_y(Alignment::Start),
        );
    }
    // Objects, expressions, the rasters' rows and a field map take the whole width under their label (the web's
    // `prow--stacked`).
    let stacked = matches!(
        def.kind,
        ParamKind::Features { .. }
            | ParamKind::Expression { .. }
            | ParamKind::RasterValues { .. }
            | ParamKind::RasterPairs { .. }
            | ParamKind::FieldMap { .. }
            | ParamKind::Layers { .. }
    );
    let content: Element<'a, Message> = if stacked {
        column![text, control_col].spacing(8).into()
    } else {
        row![
            container(text).width(Fill).padding([5, 0]),
            container(control_col).width(Length::Fixed(typography::scaled(SIDE)))
        ]
        .spacing(20)
        .align_y(Alignment::Start)
        .into()
    };
    container(content).padding([12, 0]).width(Fill).into()
}

/// The right-hand panel: the category, what the tool does, its help (a
/// model's steps), the preview, where it runs and its command-line names.
fn side<'a>(
    window: &'a ToolDialog,
    processing: &'a Processing,
    server: &plan::ServerPlace,
) -> Element<'a, Message> {
    let tool = &window.tool;
    let registry = &processing.registry;
    let model = tool
        .id
        .strip_prefix(MODEL_PREFIX)
        .and_then(|id| registry.model(id));
    let category = registry.category(&tool.category);
    let path = registry.category_path(&tool.category);
    let crumb_text = if model.is_some() {
        format!(
            "Modeller › {}",
            if path.is_empty() {
                "Genel"
            } else {
                path.as_str()
            }
        )
    } else {
        path
    };
    let crumb_icon = if model.is_some() {
        "processing"
    } else {
        category.map_or("processing", |c| c.icon)
    };
    let mut out = Column::new().spacing(14).padding(16);
    out = out.push(
        row![
            icon(crate::icons::from_web(Some(crumb_icon)))
                .size(14.0)
                .tone(Tone::Muted),
            label::caption(crumb_text).style(style::text::muted)
        ]
        .spacing(6)
        .align_y(Center),
    );
    out = out.push(
        row![
            container(
                icon(crate::icons::from_web(tool.icon.as_deref()))
                    .size(20.0)
                    .tone(Tone::Accent)
            )
            .center(Length::Fixed(typography::scaled(38.0)))
            .style(style::container::field_box),
            label::body(tool.description.clone()).font(typography::ui_strong()),
        ]
        .spacing(12)
        .align_y(Alignment::Start),
    );
    if model.is_none()
        && let Some(help) = &tool.help
    {
        for paragraph in help.split("\n\n").filter(|p| !p.trim().is_empty()) {
            out = out.push(label::body(paragraph.to_owned()).style(style::text::muted));
        }
    }
    if let Some(model) = model {
        let lookup = |id: &str| registry.tool(id);
        let order = kentos_processing::model::order_steps(model).unwrap_or_default();
        let mut steps = Column::new().spacing(6).push(side_title("Adımlar"));
        for (i, id) in order.iter().enumerate() {
            if let Some(step) = model.steps.iter().find(|s| &s.id == id) {
                let step_icon = lookup(&step.tool).and_then(|t| t.icon);
                steps = steps.push(
                    row![
                        label::mono_caption(format!("{}.", i + 1)).style(style::text::muted),
                        icon(crate::icons::from_web(step_icon.as_deref())).size(14.0),
                        label::body(kentos_processing::model::step_name(step, &lookup)),
                    ]
                    .spacing(6)
                    .align_y(Center),
                );
            }
        }
        // Model tasarımcısı on the model, or on a copy of a built-in one (the web's button, same words).
        let edit = button(
            row![
                icon(crate::icons::from_web(Some("edit"))).size(14.0),
                label::body(if registry.is_builtin_model(&model.id) {
                    "Kopyasını düzenle"
                } else {
                    "Modeli düzenle"
                })
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([4, 10])
        .style(style::button::secondary)
        .on_press(ev(Event::Panel(super::panel::Event::EditModel(
            model.id.clone(),
        ))));
        out = out.push(column![steps, edit].spacing(10));
    }
    if tool.preview.is_some() {
        let valid = !window.issues.iter().any(|i| i.param.is_some());
        let text = window.preview().unwrap_or_default();
        let value = if valid {
            label::mono(text)
        } else {
            label::body(text).style(style::text::muted)
        };
        out = out.push(
            container(column![side_title("Önizleme"), value].spacing(6))
                .padding(12)
                .width(Fill)
                .style(style::container::bordered),
        );
    }
    // Nerede çalışır (the web's plan, plan.rs): Otomatik and what it picks
    // for these inputs now, then each place the tool names; places this
    // program does not have are coming.
    let (options, hint, current) = plan::targets_view(
        &window.targets(registry, server),
        Choice::read(processing.memory.target(&tool.id)),
    );
    let mut targets =
        RadioGroup::new(current, |c: Choice| ev(Event::Target(c.id().to_owned()))).notes_at_end();
    for (i, o) in options.into_iter().enumerate() {
        let note = o.note.unwrap_or_default();
        targets = if o.disabled {
            targets.disabled_with(o.value, o.label, note)
        } else {
            targets.option(o.value, o.label, note)
        };
        if i == 0
            && let Some(hint) = hint
        {
            targets = targets.hint(hint);
        }
    }
    out = out.push(column![side_title("Nerede çalışır"), targets].spacing(8));
    // On the server the result goes to the project, or to a file the job keeps (docs/adr/0559 §5).
    if current == Some(Choice::At(kentos_processing::Target::Server)) {
        let result = RadioGroup::new(window.server_sink, |sink: bool| ev(Event::ServerSink(sink)))
            .option(
                false,
                "Sonucu projeye yaz",
                "tek adımda; proje araç çalışırken değişirse hiçbir şey yazılmaz",
            )
            .option(
                true,
                "Sonucu dosyaya yaz (GeoPackage)",
                "proje değişmez; bitince Sunucu işleri'nden indirilir",
            );
        out = out.push(column![side_title("Sonuç"), result].spacing(8));
    }
    if !tool.aliases.is_empty() {
        let mut names = Row::new().spacing(6);
        for a in &tool.aliases {
            names = names.push(
                container(label::mono_caption(a.clone()))
                    .padding([2, 6])
                    .style(style::container::token),
            );
        }
        out = out.push(column![side_title("Komut satırından"), names.wrap()].spacing(8));
    }
    out.into()
}

fn side_title<'a>(text: &'a str) -> Element<'a, Message> {
    label::caption(text)
        .font(typography::ui_strong())
        .style(style::text::muted)
        .into()
}

/// Varsayılanlar, the status line (how the run goes or ended, or what to
/// fix), Kapat or Durdur, and Çalıştır: the web's plan (plan.rs).
fn footer<'a>(window: &'a ToolDialog, at_server: bool) -> Element<'a, Message> {
    let buttons = plan::footer_at(&window.status, at_server);
    let line = window.line();
    let reset = button(label::body("Varsayılanlar"))
        .padding([5, 10])
        .style(style::button::ghost)
        .on_press_maybe((!buttons.reset_disabled).then(|| ev(Event::Reset)));
    let small = |text: &'a str, e: Event| {
        button(label::body(text))
            .padding([4, 8])
            .style(style::button::ghost)
            .on_press(ev(e))
    };
    // Toplu çalıştır (docs/adr/0529 §4): the same tool over a table of rows of values.
    let batch = button(
        row![
            icon(crate::icons::from_web(Some("table"))).size(14.0),
            label::body("Toplu çalıştır…")
        ]
        .spacing(6)
        .align_y(Center),
    )
    .padding([5, 10])
    .style(style::button::ghost)
    .on_press_maybe((!window.running()).then(|| ev(Event::Batch(super::batch::Event::Open))));
    let mut status = Row::new().spacing(8).align_y(Center).width(Fill);
    if line.kind == LineKind::Running {
        let share = line.progress.map(|p| f32::from(p) / 100.0);
        status = status.push(
            progress::bar(share)
                .width(Length::Fixed(typography::scaled(PROGRESS)))
                .tint(Tint::Muted),
        );
    } else if let Some(mark) = line.icon {
        status = status.push(match mark {
            LineIcon::Success => icon(Icon::Success).size(16.0).tone(Tone::Success),
            LineIcon::Warning => icon(Icon::Warning).size(16.0).tone(Tone::Warning),
            LineIcon::Error => icon(Icon::Error).size(16.0).tone(Tone::Danger),
        });
    }
    if !line.text.is_empty() {
        status = status.push(label::body(line.text).width(Fill));
    }
    for action in line.actions {
        status = status.push(match action {
            Action::Zoom => small("Seçime yakınlaştır", Event::Results),
            Action::Select => small("Sonuçları seç", Event::Results),
            Action::Undo => small("Geri al", Event::Undo),
            Action::Jobs => small("Sunucu işleri", Event::ServerJobs),
        });
    }
    let close = button(label::body(buttons.close))
        .padding([5, 14])
        .style(style::button::secondary)
        .on_press(ev(if window.running() {
            Event::Stop
        } else {
            Event::Close
        }));
    let run = button(
        row![icon(Icon::Play).size(14.0), label::body(buttons.run)]
            .spacing(6)
            .align_y(Center),
    )
    .padding([5, 14])
    .style(style::button::primary)
    .on_press_maybe((!buttons.run_disabled).then(|| ev(Event::Run)));
    row![reset, batch, status, close, run]
        .spacing(10)
        .align_y(Center)
        .into()
}
