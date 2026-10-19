//! Atlası PDF yap (docs/adr/0534, plan/gis.md CBS-21): the window previews what the atlas will be before anything is
//! written and writes it through `file.export.atlas`, so a script and the window do the same.
//!
//! - **The sheet and its atlas.** The sheets of the project's book; the chosen one's atlas (coverage layer, filter,
//!   sort, page name) is edited here and written with `cad.sheets.edit`'s `setAtlas`.
//! - **The bindings are chosen, not guessed.** The theme (a layer state the atlas maps draw), the moment of the
//!   temporal layers, and the coordinate system, which is the project's.
//! - **The preview** is the command's plan: the pages (name, scale, centre), what the layer's filter and the time left
//!   out, which attributes the pages carry and which they hold back, and the state they are pinned to. Dışa aktar…
//!   hands that pin to the command: a drawing or a bound file that has moved on since is refused and the preview is
//!   asked again; a drawing that moves on while the pages are made is said when the file is written.

use std::path::PathBuf;

use iced::widget::{Column, container, row, scrollable, text_input};
use iced::{Element, Fill, Task};
use kentos_contracts::{AtlasExported, CommandResult, CommandWarning, ExportAtlas};
use kentos_interaction::Level;
use kentos_native_application::ExecutionContext;
use kentos_plot::atlas::{AtlasInput, AtlasPlanned};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog, focus_ring, overlay};
use kentos_ui::{label, style};

use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message, off_thread};
use crate::app::{App, Message, Picker};

/// The most pages the preview lists; the rest are counted.
const LISTED: usize = 300;

/// A sheet of the book, with its atlas.
#[derive(Debug, Clone)]
pub struct SheetRow {
    pub id: String,
    pub name: String,
    pub atlas: Option<kentos_sheet::model::Atlas>,
}

/// The atlas's definition as the window edits it (`Atlas`).
#[derive(Debug, Clone, Default)]
pub struct Definition {
    /// The coverage layer: 0 none, then the layers in order.
    pub layer: usize,
    pub filter: String,
    pub sort: String,
    pub descending: bool,
    pub name: String,
}

#[derive(Debug)]
pub struct State {
    pub sheets: Vec<SheetRow>,
    pub sheet: usize,
    /// The drawing's layers (id, name).
    pub layers: Vec<(String, String)>,
    /// The project's layer states; the choice 0 keeps what each map shows.
    pub themes: Vec<String>,
    pub theme: usize,
    pub time: String,
    pub definition: Definition,
    /// What the atlas would be, or why it cannot be.
    pub plan: Result<Box<AtlasPlanned>, String>,
    /// A text was edited since the preview: the preview is of the last choices.
    pub dirty: bool,
    pub status: Option<(bool, String)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Sheet(usize),
    Theme(usize),
    Time(String),
    Layer(usize),
    Filter(String),
    Sort(String),
    Descending,
    PageName(String),
    /// Writes the atlas definition into the sheet.
    Apply,
    /// The preview again.
    Refresh,
    Save,
    SavePicked(Option<PathBuf>),
    Written(Box<Result<(AtlasExported, Vec<CommandWarning>), String>>),
}

fn event(e: Event) -> Message {
    message(Exchange::Atlas(e))
}

impl Definition {
    fn of(atlas: Option<&kentos_sheet::model::Atlas>, layers: &[(String, String)]) -> Definition {
        let Some(a) = atlas else {
            return Definition {
                name: "@atlas_kimlik".into(),
                ..Definition::default()
            };
        };
        let by = |id: &str| layers.iter().position(|(l, n)| l == id || n == id);
        Definition {
            layer: by(&a.layer).map_or(0, |i| i + 1),
            filter: a.filter.clone(),
            sort: a
                .sort
                .first()
                .map(|k| k.expression.clone())
                .unwrap_or_default(),
            descending: a.sort.first().is_some_and(|k| k.descending),
            name: a.page_name.clone(),
        }
    }

    /// The `setAtlas` operation's atlas, or none while no layer is chosen.
    fn atlas(&self, layers: &[(String, String)]) -> Option<serde_json::Value> {
        let layer = layers.get(self.layer.checked_sub(1)?)?;
        let sort: Vec<serde_json::Value> = if self.sort.trim().is_empty() {
            Vec::new()
        } else {
            vec![
                serde_json::json!({ "expression": self.sort.trim(), "descending": self.descending }),
            ]
        };
        Some(serde_json::json!({
            "layer": layer.0, "filter": self.filter.trim(), "sort": sort,
            "pageName": if self.name.trim().is_empty() { "@atlas_kimlik" } else { self.name.trim() },
        }))
    }
}

impl App {
    /// The window's state for the open drawing, the sheet `keep` (by id) chosen when it is still there.
    fn atlas_state(&self, keep: Option<&str>, old: Option<&State>) -> Result<State, String> {
        let Some(doc) = &self.document else {
            return Err("Açık çizim yok.".into());
        };
        let book = match doc
            .model
            .sheets()
            .map(kentos_sheet::project::read_project_sheets)
        {
            None => {
                return Err(
                    "Projede pafta yok: önce Pafta kipinden ya da bir şablondan pafta ekleyin."
                        .into(),
                );
            }
            Some(Err(e)) => return Err(e.message),
            Some(Ok((book, _))) => book,
        };
        let sheets: Vec<SheetRow> = book
            .sheets
            .iter()
            .map(|s| SheetRow {
                id: s.id.clone(),
                name: s.name.clone(),
                atlas: s.atlas.clone(),
            })
            .collect();
        if sheets.is_empty() {
            return Err("Pafta kitabında pafta yok: önce bir pafta ekleyin.".into());
        }
        let layers: Vec<(String, String)> = doc
            .model
            .layers()
            .leaves()
            .into_iter()
            .map(|n| (n.id.clone(), n.name.clone()))
            .collect();
        let sheet = keep
            .and_then(|id| sheets.iter().position(|s| s.id == id))
            .or_else(|| sheets.iter().position(|s| s.atlas.is_some()))
            .unwrap_or(0);
        let themes: Vec<String> = doc
            .settings()
            .layer_states
            .iter()
            .map(|s| s.name.clone())
            .collect();
        let definition = Definition::of(sheets[sheet].atlas.as_ref(), &layers);
        Ok(State {
            theme: old.map_or(0, |o| o.theme.min(themes.len())),
            time: old.map(|o| o.time.clone()).unwrap_or_default(),
            sheets,
            sheet,
            layers,
            themes,
            definition,
            plan: Err(String::new()),
            dirty: false,
            status: None,
        })
    }

    pub(super) fn atlas_export_open(&mut self) -> Task<Message> {
        match self.atlas_state(None, None) {
            Ok(state) => {
                self.open_window(Window::AtlasExport(Box::new(state)));
                self.atlas_replan();
            }
            Err(why) => self.output(why),
        }
        Task::none()
    }

    /// The command's plan for the window's choices.
    fn atlas_replan(&mut self) {
        let Some(Window::AtlasExport(s)) = &self.exchange else {
            return;
        };
        let Some(doc) = &self.document else {
            return;
        };
        let plan = match s.sheets.get(s.sheet) {
            None => Err("Pafta yok.".to_owned()),
            Some(row) => {
                let theme = s
                    .theme
                    .checked_sub(1)
                    .and_then(|i| s.themes.get(i))
                    .cloned();
                let time = (!s.time.trim().is_empty()).then(|| s.time.trim().to_owned());
                let user = std::env::var("USER").unwrap_or_default();
                let date = kentos_plot::book::today();
                kentos_plot::atlas::plan(
                    &doc.model,
                    &AtlasInput {
                        sheet: &row.id,
                        theme: theme.as_deref(),
                        time: time.as_deref(),
                        user: &user,
                        date: &date,
                        ..AtlasInput::default()
                    },
                )
                .map(Box::new)
                .map_err(|e| e.message)
            }
        };
        if let Some(Window::AtlasExport(s)) = &mut self.exchange {
            s.plan = plan;
            s.dirty = false;
        }
    }

    pub(super) fn atlas_export_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Sheet(i) => {
                if let Some(Window::AtlasExport(s)) = &mut self.exchange {
                    s.sheet = i;
                    s.definition =
                        Definition::of(s.sheets.get(i).and_then(|r| r.atlas.as_ref()), &s.layers);
                    s.status = None;
                }
                self.atlas_replan();
            }
            Event::Theme(i) => {
                if let Some(Window::AtlasExport(s)) = &mut self.exchange {
                    s.theme = i;
                }
                self.atlas_replan();
            }
            Event::Time(t) => {
                if let Some(Window::AtlasExport(s)) = &mut self.exchange {
                    s.time = t;
                    s.dirty = true;
                }
            }
            Event::Layer(i) => {
                if let Some(Window::AtlasExport(s)) = &mut self.exchange {
                    s.definition.layer = i;
                }
            }
            Event::Filter(t) => {
                if let Some(Window::AtlasExport(s)) = &mut self.exchange {
                    s.definition.filter = t;
                }
            }
            Event::Sort(t) => {
                if let Some(Window::AtlasExport(s)) = &mut self.exchange {
                    s.definition.sort = t;
                }
            }
            Event::Descending => {
                if let Some(Window::AtlasExport(s)) = &mut self.exchange {
                    s.definition.descending = !s.definition.descending;
                }
            }
            Event::PageName(t) => {
                if let Some(Window::AtlasExport(s)) = &mut self.exchange {
                    s.definition.name = t;
                }
            }
            Event::Refresh => self.atlas_replan(),
            Event::Apply => self.atlas_apply(),
            Event::Save => {
                if let Picker::File(path) = &self.picker {
                    return Task::done(event(Event::SavePicked(Some(path.clone()))));
                }
                let suggested = self.export_name("-atlas.pdf");
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Atlası PDF yap")
                            .add_filter("PDF", &["pdf"])
                            .set_file_name(suggested)
                            .save_file()
                            .await?;
                        let mut path = file.path().to_path_buf();
                        if path.extension().is_none() {
                            path.set_extension("pdf");
                        }
                        Some(path)
                    },
                    |p| event(Event::SavePicked(p)),
                );
            }
            Event::SavePicked(None) => {}
            Event::SavePicked(Some(path)) => {
                let (Some(doc), Some(Window::AtlasExport(s))) =
                    (&self.document, &mut self.exchange)
                else {
                    return Task::none();
                };
                let (Ok(planned), Some(row)) = (&s.plan, s.sheets.get(s.sheet)) else {
                    return Task::none();
                };
                let input = ExportAtlas {
                    path: path.display().to_string(),
                    sheet: row.id.clone(),
                    // The save dialog asked about writing over it.
                    overwrite: true,
                    fields: None,
                    theme: s
                        .theme
                        .checked_sub(1)
                        .and_then(|i| s.themes.get(i))
                        .cloned(),
                    time: (!s.time.trim().is_empty()).then(|| s.time.trim().to_owned()),
                    crs: None,
                    // The state the preview showed: a drawing that moved on since is refused.
                    pin: Some(planned.pin.clone()),
                };
                s.status = Some((false, format!("{} yazılıyor…", path.display())));
                // The pages are made on a copy of the drawing as it is now, on a thread of their own.
                let model = doc.model.clone();
                return off_thread(
                    move || {
                        kentos_headless::file_commands::export_atlas(&model, &input)
                            .map_err(|e| e.message)
                    },
                    |r| Exchange::Atlas(Event::Written(Box::new(r))),
                );
            }
            Event::Written(done) => match *done {
                Ok((out, warnings)) => {
                    // Whether the drawing moved on while the pages were made.
                    let moved = match (&self.exchange, &self.document) {
                        (Some(Window::AtlasExport(s)), Some(doc)) => {
                            s.plan.as_ref().ok().is_some_and(|p| {
                                kentos_plot::atlas::current_digest(&doc.model, p) != out.pin.digest
                            })
                        }
                        _ => false,
                    };
                    self.close_exchange();
                    self.announce(
                        if warnings.is_empty() && !moved {
                            Level::Success
                        } else {
                            Level::Warn
                        },
                        format!(
                            "{} yazıldı: {} sayfa; çizim durumu {}.",
                            out.path,
                            out.pages.len(),
                            out.pin.digest.get(..12).unwrap_or(&out.pin.digest)
                        ),
                    );
                    if moved {
                        self.say(
                            Level::Warn,
                            "Sayfalar yapılırken çizim değişti: çıktı yukarıdaki durumdan yapıldı. Güncel çizimle istiyorsanız Atlası PDF yap'ı yeniden çalıştırın.",
                        );
                    }
                    for w in warnings {
                        self.say(Level::Warn, w.message);
                    }
                }
                Err(why) => {
                    if let Some(Window::AtlasExport(s)) = &mut self.exchange {
                        s.status = Some((true, format!("Atlas yazılamadı. {why}")));
                    }
                }
            },
        }
        Task::none()
    }

    /// The atlas definition written into the sheet (`cad.sheets.edit`'s `setAtlas`); the preview again.
    fn atlas_apply(&mut self) {
        let Some(Window::AtlasExport(s)) = &self.exchange else {
            return;
        };
        let Some(row) = s.sheets.get(s.sheet) else {
            return;
        };
        let ops = serde_json::json!([{
            "op": "setAtlas", "sheet": row.id, "atlas": s.definition.atlas(&s.layers),
        }]);
        let keep = row.id.clone();
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let Ok(input) = serde_json::from_value(serde_json::json!({ "ops": ops })) else {
            return;
        };
        let answer = kentos_native_application::sheets_edit::execute(
            &mut ExecutionContext::new(&mut doc.model),
            input,
        );
        match answer {
            CommandResult::Completed { .. } => {
                let old = match self.exchange.take() {
                    Some(Window::AtlasExport(s)) => Some(s),
                    other => {
                        self.exchange = other;
                        None
                    }
                };
                match self.atlas_state(Some(&keep), old.as_deref()) {
                    Ok(mut fresh) => {
                        fresh.status = Some((false, "Atlas tanımı paftaya yazıldı.".into()));
                        self.exchange = Some(Window::AtlasExport(Box::new(fresh)));
                        self.atlas_replan();
                    }
                    Err(why) => self.output(why),
                }
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                if let Some(Window::AtlasExport(s)) = &mut self.exchange {
                    s.status = Some((true, error.message));
                }
            }
            _ => {}
        }
    }

    pub(super) fn atlas_export_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let format = self.number_format();
        let input = |hint: &str, value: &str, on: fn(String) -> Event| {
            focus_ring(
                text_input(hint, value)
                    .on_input(move |t| event(on(t)))
                    .padding([5, 8])
                    .width(Fill)
                    .size(kentos_ui::theme::typography::body())
                    .style(style::field::input),
            )
        };
        let mut body = Column::new().spacing(12);
        let mut preview = Column::new().spacing(12);
        body = body.push(words::field(
            "Pafta",
            Select::new(
                s.sheets.iter().map(|r| {
                    Choice::new(if r.atlas.is_some() {
                        r.name.clone()
                    } else {
                        format!("{} (atlası yok)", r.name)
                    })
                }),
                Some(s.sheet),
                |i| event(Event::Sheet(i)),
            ),
            None,
        ));
        // The bindings.
        let themes = std::iter::once(Choice::new("Her haritanın kendi katmanları".to_owned()))
            .chain(s.themes.iter().map(|t| Choice::new(t.clone())));
        body = body.push(words::field(
            "Tema",
            Select::new(themes, Some(s.theme), |i| event(Event::Theme(i))),
            Some(
                "Projenin bir katman durumu: atlas haritaları onunla çizilir; çizim değişmez."
                    .to_owned(),
            ),
        ));
        body = body.push(words::field(
            "Zaman",
            row![
                container(input("2024-06-30", &s.time, Event::Time)).width(Fill),
                words::secondary("Önizle", Some(event(Event::Refresh))),
            ]
            .spacing(10)
            .align_y(iced::Center),
            Some("Zamanlı katmanların gösterildiği an; boş: kesilmez. Kapsam nesneleri ve harita onunla kesilir.".to_owned()),
        ));
        // The atlas's own definition.
        let d = &s.definition;
        let layer_names = std::iter::once(Choice::new("Seçilmedi".to_owned()))
            .chain(s.layers.iter().map(|(_, n)| Choice::new(n.clone())));
        let definition = Column::new()
            .spacing(8)
            .push(label::strong("Atlas tanımı"))
            .push(words::field(
                "Kapsam katmanı",
                Select::new(layer_names, Some(d.layer), |i| event(Event::Layer(i))),
                Some("Katmanın her nesnesi (katmanın süzgecinden geçenler) bir sayfa olur.".to_owned()),
            ))
            .push(words::field(
                "Süzgeç",
                input("Parsel > 10", &d.filter, Event::Filter),
                Some("Hangi nesneler sayfa olur (ifade; boş: hepsi). Nesnenin alanları adıyla okunur.".to_owned()),
            ))
            .push(words::field(
                "Sıralama",
                row![
                    container(input("Parsel", &d.sort, Event::Sort)).width(Fill),
                    words::check(d.descending, "Azalan", Some(event(Event::Descending))),
                ]
                .spacing(10)
                .align_y(iced::Center),
                None,
            ))
            .push(words::field(
                "Sayfa adı",
                input("Ada || '/' || Parsel", &d.name, Event::PageName),
                Some("Sayfanın adı (ifade); aynı ad yinelenirse (2), (3) eklenir.".to_owned()),
            ))
            .push(
                row![
                    words::secondary("Tanımı uygula", Some(event(Event::Apply))),
                    label::caption("Paftaya yazılır; Pafta kipinde Geri al ile alınır."),
                ]
                .spacing(10)
                .align_y(iced::Center),
            );
        body = body.push(
            container(definition)
                .padding([8, 10])
                .width(Fill)
                .style(style::container::bordered),
        );
        let mut lines = Vec::new();
        let ready = match &s.plan {
            Ok(p) => {
                lines.push(words::text_line(
                    Line::Ok,
                    format!(
                        "{} sayfa; {} sistemde{}.",
                        p.pages.len(),
                        p.bindings.crs_name,
                        if p.bindings.geo { " (GeoPDF)" } else { "" }
                    ),
                ));
                if p.filtered_out > 0 {
                    lines.push(words::text_line(
                        Line::Info,
                        format!(
                            "{} nesne katmanın süzgecinden ya da zamandan geçmedi: ne sayfa olur ne haritada görünür.",
                            p.filtered_out
                        ),
                    ));
                }
                lines.push(words::text_line(
                    Line::Info,
                    format!(
                        "Sayfaların taşıdığı öznitelikler: {}.",
                        if p.fields.is_empty() {
                            "yok".to_owned()
                        } else {
                            p.fields.join(", ")
                        }
                    ),
                ));
                if !p.withheld.is_empty() {
                    lines.push(words::text_line(
                        Line::Info,
                        format!("Çıktıya girmeyen öznitelikler: {}.", p.withheld.join(", ")),
                    ));
                }
                let state = &p.pin.digest;
                lines.push(words::text_line(
                    Line::Info,
                    format!(
                        "Sabitlenen durum {}: {}.",
                        state.get(..12).unwrap_or(state),
                        p.pin
                            .data
                            .iter()
                            .map(|l| match &l.source {
                                Some(src) => format!(
                                    "{} ({} nesne, kaynak {})",
                                    l.name,
                                    l.objects,
                                    src.revision.get(..8).unwrap_or(&src.revision)
                                ),
                                None => format!("{} ({} nesne)", l.name, l.objects),
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ));
                for w in &p.warnings {
                    lines.push(words::text_line(Line::Warn, w.message.clone()));
                }
                for n in &p.notes {
                    lines.push(words::text_line(Line::Warn, n.clone()));
                }
                if s.dirty {
                    lines.push(words::text_line(
                        Line::Warn,
                        "Zaman değişti: önizlemeyi yenileyin (Önizle), sonra dışa aktarın.",
                    ));
                }
                !s.dirty
            }
            Err(why) => {
                if !why.is_empty() {
                    lines.push(words::text_line(Line::Warn, why.clone()));
                }
                false
            }
        };
        preview = preview.push(words::summary(lines));
        if let Ok(p) = &s.plan {
            let mut pages = Column::new().spacing(2).push(
                row![
                    container(label::caption("Sayfa")).width(48),
                    container(label::caption("Ad")).width(Fill),
                    container(label::caption("Ölçek")).width(72),
                    container(label::caption(format.axes_text("Merkez (Y, X)"))).width(200),
                ]
                .spacing(8),
            );
            for page in p.pages.iter().take(LISTED) {
                pages = pages.push(
                    row![
                        container(label::body(page.index.to_string())).width(48),
                        container(label::body(page.name.clone())).width(Fill),
                        container(label::body(format!("1/{}", page.scale))).width(72),
                        container(label::body(format!(
                            "{}, {}",
                            format.coord(page.center.x),
                            format.coord(page.center.y)
                        )))
                        .width(200),
                    ]
                    .spacing(8),
                );
            }
            if p.pages.len() > LISTED {
                pages = pages.push(label::caption(format!(
                    "… ve {} sayfa daha.",
                    p.pages.len() - LISTED
                )));
            }
            preview = preview.push(
                container(scrollable(container(pages).padding([6, 8])).height(300))
                    .width(Fill)
                    .style(style::container::bordered),
            );
        }
        if let Some((error, text)) = &s.status {
            preview = preview.push(words::text_line(
                if *error { Line::Error } else { Line::Info },
                text.clone(),
            ));
        }
        // The choices on the left, what they make on the right: the pages are seen at once.
        let body = row![
            container(body).width(iced::Length::FillPortion(5)),
            container(preview).width(iced::Length::FillPortion(6)),
        ]
        .spacing(16);
        overlay::blocking(
            Dialog::new("Atlası PDF yap")
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary(
                    "Dışa aktar…",
                    ready.then(|| event(Event::Save)),
                ))
                .width(1040.0)
                .max_height(760.0),
        )
    }
}
