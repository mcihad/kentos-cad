//! Ruhsat takımı (docs/adr/3037, M37): the drawing's particulars asked once (written into its variables, where every
//! sheet's title block and PAİY 57(10)'s table read them), then the phase's set made in one undo step and Paftaları
//! toplu yayınla opened on it, checked, one PDF/A-2 file a sheet named by the digital project regulation's EK-2: the
//! user presses Yayınla.
use super::{Event as Exchange, Window, message};
use crate::app::{App, Message};
use iced::widget::{button, column, row};
use iced::{Element, Fill, Task};
use kentos_sheet::permit::{self, Phase};
use kentos_ui::{label, style, widget::Dialog};

/// One particular in the window.
#[derive(Clone, Debug)]
pub struct Field {
    pub name: &'static str,
    pub label: &'static str,
    pub value: String,
    /// The regulation asks it and it has no default: an empty one is said missing on the sheets.
    pub required: bool,
}

pub struct State {
    pub phase: Phase,
    pub fields: Vec<Field>,
    pub status: String,
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PermitSet")
            .field("phase", &self.phase)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Phase(Phase),
    Field(usize, String),
    Make,
    Close,
}

fn event(e: Event) -> Message {
    message(Exchange::PermitSet(e))
}

/// The window's groups of particulars, by name, in the order a title block and PAİY 57(10) read them.
pub const GROUPS: [(&str, &[&str]); 5] = [
    (
        "Arsa",
        &[
            "il",
            "ilce",
            "mahalle",
            "pafta",
            "ada",
            "parsel",
            "tapu_kaydi",
            "mevcut_yapilar",
        ],
    ),
    (
        "Yapı",
        &[
            "proje_adi",
            "yapi_sahibi",
            "muteahhit",
            "tasiyici_sistem",
            "blok_no",
        ],
    ),
    (
        "Müellif",
        &[
            "mimar",
            "mimar_unvan",
            "sicil_no",
            "btb_no",
            "buro_adres",
            "tasarlayan",
            "cizen",
            "kontrol_eden",
        ],
    ),
    (
        "Dijital teslim (EK-1, EK-2)",
        &["proje_kodu", "bina_kodu", "revizyon_no", "proje_no"],
    ),
    ("Denetim", &["denetciler", "yapi_denetim"]),
];

impl App {
    pub(crate) fn permit_set_open(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let fold = |s: &str| kentos_sheet::expr::fold(s);
        let vars = &doc.model.settings().variables;
        let fields = permit::PARTICULARS
            .iter()
            .map(|(name, label, default)| {
                let have =
                    vars.iter()
                        .find(|v| fold(&v.name) == fold(name))
                        .map(|v| match &v.value {
                            kentos_commands::VariableValue::Text(t) => t.clone(),
                            kentos_commands::VariableValue::Number(n) => n.to_string(),
                            _ => String::new(),
                        });
                let value = have.unwrap_or_else(|| match *name {
                    "proje_adi" => doc.model.name().to_owned(),
                    _ => default.unwrap_or_default().to_owned(),
                });
                Field {
                    name,
                    label,
                    value,
                    required: default.is_none(),
                }
            })
            .collect();
        self.close_exchange();
        self.exchange = Some(Window::PermitSet(Box::new(State {
            phase: Phase::Permit,
            fields,
            status: String::new(),
        })));
        self.dialog = Some(crate::app::Dialog::Exchange);
        Task::none()
    }

    pub(crate) fn permit_set_event(&mut self, e: Event) -> Task<Message> {
        if matches!(e, Event::Close) {
            self.close_exchange();
            return Task::none();
        }
        let Some(Window::PermitSet(s)) = &mut self.exchange else {
            return Task::none();
        };
        match e {
            Event::Phase(p) => s.phase = p,
            Event::Field(i, v) => {
                if let Some(f) = s.fields.get_mut(i) {
                    f.value = v;
                }
            }
            Event::Make => {
                let input = kentos_native_application::permit_set::PermitSet {
                    phase: s.phase,
                    values: s
                        .fields
                        .iter()
                        .map(|f| (f.name.to_owned(), f.value.clone()))
                        .collect(),
                };
                let Some(doc) = self.document.as_mut() else {
                    return Task::none();
                };
                let made = kentos_native_application::permit_set::execute(
                    &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
                    &input,
                );
                match made {
                    Err(why) => {
                        if let Some(Window::PermitSet(s)) = &mut self.exchange {
                            s.status = why;
                        }
                    }
                    Ok((_, made)) => {
                        let missing =
                            kentos_native_application::permit_set::missing_particulars(&doc.model);
                        let phase = input.phase;
                        let numbers = match (made.first(), made.last()) {
                            (Some(a), Some(b)) => format!("{} … {}", a.number, b.number),
                            _ => String::new(),
                        };
                        let said = format!(
                            "{} takımı: {} pafta ({numbers}){}",
                            phase.name(),
                            made.len(),
                            if missing.is_empty() {
                                String::new()
                            } else {
                                format!(
                                    "; eksik bilgi: {}",
                                    missing
                                        .iter()
                                        .map(|(_, l)| *l)
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                )
                            }
                        );
                        self.say(kentos_interaction::Level::Info, said);
                        // Paftaları toplu yayınla on the set: a file a sheet, PDF/A-2, named by EK-2, checked.
                        let _ = self.sheet_publish_open();
                        if let Some(Window::SheetPublish(p)) = &mut self.exchange {
                            let ids: Vec<&str> = made.iter().map(|m| m.sheet.as_str()).collect();
                            for r in &mut p.rows {
                                r.selected = ids.contains(&r.id.as_str());
                            }
                            p.input.mode = kentos_commands::SheetPublishMode::Separate;
                            p.input.pdfa = true;
                            p.input.naming = permit::FILE_NAMING.into();
                            if let Some(dir) = std::path::Path::new(&p.input.path).parent() {
                                p.input.path = dir
                                    .join(format!(
                                        "{}-{}",
                                        phase.name(),
                                        kentos_plot::book::today()
                                    ))
                                    .display()
                                    .to_string();
                            }
                        }
                        return self.sheet_publish_event(super::sheet_publish::Event::Plan);
                    }
                }
            }
            Event::Close => {}
        }
        Task::none()
    }

    pub(crate) fn permit_set_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let phases = row(Phase::ALL.iter().map(|p| {
            let words = format!("{} · 1/{}", p.name(), p.scale());
            button(label::body(words))
                .padding(kentos_ui::theme::metrics::action())
                .on_press(event(Event::Phase(*p)))
                .style(if *p == s.phase {
                    style::button::primary
                } else {
                    style::button::secondary
                })
                .into()
        }))
        .spacing(6);
        let mut body = column![
            label::body(
                "Bilgiler çizimin değişkenlerine yazılır; bütün paftaların anteti ve ilk paftanın bilgi tablosu \
                 (PAİY 57/10) buradan okur. Yıldızlı alanlar zorunludur; boş kalırsa paftada ‹ad?› yazılır ve yayın \
                 bekler. Arsa alanı, emsal, kat adedi ve bağımsız bölümler çizimden okunur.",
            ),
            label::caption("Evre ve planların ölçeği"),
            phases,
        ]
        .spacing(8);
        for (title, names) in GROUPS {
            body = body.push(label::body(title).font(kentos_ui::theme::typography::ui_strong()));
            let mut pair: Vec<Element<'a, Message>> = Vec::new();
            for name in names {
                let Some((i, f)) = s.fields.iter().enumerate().find(|(_, f)| f.name == *name)
                else {
                    continue;
                };
                let caption = if f.required {
                    format!("{} *", f.label)
                } else {
                    f.label.to_owned()
                };
                let field = kentos_ui::widget::text_field("", &f.value)
                    .on_input(move |v| event(Event::Field(i, v)));
                pair.push(
                    column![label::caption(caption), field]
                        .spacing(2)
                        .width(Fill)
                        .into(),
                );
                if pair.len() == 2 {
                    body = body.push(row(std::mem::take(&mut pair)).spacing(10));
                }
            }
            if !pair.is_empty() {
                pair.push(iced::widget::space().width(Fill).into());
                body = body.push(row(pair).spacing(10));
            }
        }
        if !s.status.is_empty() {
            body = body.push(label::body(&s.status).style(style::text::warning));
        }
        Dialog::new("Ruhsat takımı")
            .scroll(body)
            .width(680.)
            .max_height(720.)
            .aside(
                button(label::body("Vazgeç"))
                    .on_press(event(Event::Close))
                    .padding(kentos_ui::theme::metrics::action())
                    .style(style::button::secondary),
            )
            .action(
                button(label::body("Paftaları oluştur"))
                    .on_press(event(Event::Make))
                    .padding(kentos_ui::theme::metrics::action())
                    .style(style::button::primary),
            )
            .modal(event(Event::Close))
    }
}
