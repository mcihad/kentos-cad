//! GNSS baz ağı (docs/adr/1380; `survey.baselines`): a file of stations and baselines (`K;…`, `N;…`, `B;…` lines) read
//! and adjusted by `cad.survey.baselines`, read only. The window shows the network's m0 and χ² test, each station's
//! standard deviations in east, north, up (mm) and each baseline's residual and test; Minimal datum and the variance
//! factor adjust it again; robust estimation (docs/adr/4960, Huber or Danish) reweights the baselines and shows each
//! one's factor. Nothing is written: taking the coordinates is a later, separate step.

use std::path::PathBuf;

use iced::widget::{Column, row};
use iced::{Center, Element, Task};
use kentos_commands::{BaselineDatum, BaselineRobust, CommandResult, RobustMethod, SurveyBaselines, SurveyBaselinesAdjusted};
use kentos_native_application::{ExecutionContext, survey_baselines};
use kentos_ui::label;
use kentos_ui::widget::{Dialog as Frame, Form, NumberInput, Segmented, Switch};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet};
use crate::traces::Control;

pub const TITLE: &str = "GNSS baz ağı";
const CLOSE: &str = "Kapat";
const OPEN: &str = "Dosya seç…";

#[derive(Clone, Debug, Default)]
pub struct State {
    pub view: View,
    pub file: Option<PathBuf>,
    pub text: Option<String>,
    pub minimal: bool,
    pub scale: f64,
    pub robust: Robust,
    /// Robust estimation's constant c; 0 until the user gives one (the method's own then).
    pub c: f64,
    pub answer: Option<Result<SurveyBaselinesAdjusted, String>>,
}

/// What the table shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum View {
    #[default]
    Baselines,
    Stations,
}

impl std::fmt::Display for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            View::Baselines => "Bazlar",
            View::Stations => "İstasyonlar",
        })
    }
}

/// Robust estimation (docs/adr/4960): none, or the weight function.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Robust {
    #[default]
    Off,
    Huber,
    Danish,
}

impl std::fmt::Display for Robust {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Robust::Off => "Yok",
            Robust::Huber => "Huber",
            Robust::Danish => "Danimarka",
        })
    }
}

impl Robust {
    fn method(self) -> Option<RobustMethod> {
        match self {
            Robust::Off => None,
            Robust::Huber => Some(RobustMethod::Huber),
            Robust::Danish => Some(RobustMethod::Danish),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    View(View),
    Pick,
    Picked(Option<PathBuf>),
    Minimal(bool),
    Scale(f64),
    Robust(Robust),
    C(f64),
    Close,
}

fn msg(e: Event) -> Message {
    Message::Baselines(e)
}

fn mm(v: f64) -> String {
    format!("{:.1}", v * 1000.0)
}

impl App {
    fn baselines_run(&mut self) {
        let Some(text) = self.baselines.text.clone() else { return };
        let input = SurveyBaselines {
            text: Some(text),
            scale: (self.baselines.scale != 1.0).then_some(self.baselines.scale),
            datum: if self.baselines.minimal { BaselineDatum::Minimal } else { BaselineDatum::Given },
            robust: self.baselines.robust.method().map(|method| BaselineRobust {
                method,
                c: (self.baselines.c > 0.0).then_some(self.baselines.c),
            }),
            ..SurveyBaselines::default()
        };
        let Some(doc) = self.document.as_mut() else { return };
        self.baselines.answer = Some(match survey_baselines::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            _ => Err("Dengeleme yapılamadı.".to_owned()),
        });
    }

    /// GNSS baz ağı (`survey.baselines`): the window, then the file.
    pub(crate) fn open_baselines(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return Task::none();
        }
        self.baselines = State { scale: 1.0, ..State::default() };
        self.dialog = Some(Dialog::Baselines);
        Task::none()
    }

    /// The file read and adjusted (a test or a trace gives it here, the picker in the app).
    pub(crate) fn baselines_file(&mut self, path: PathBuf) {
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                self.baselines.text = Some(text);
                self.baselines.file = Some(path);
                self.baselines_run();
            }
            Err(e) => self.baselines.answer = Some(Err(format!("{} okunamadı: {e}", path.display()))),
        }
    }

    pub(crate) fn baselines_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::View(v) => self.baselines.view = v,
            Event::Pick => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("GNSS baz dosyası")
                            .add_filter("Baz listesi", &["txt", "csv"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Picked(p)),
                );
            }
            Event::Picked(Some(path)) => self.baselines_file(path),
            Event::Picked(None) => {}
            Event::Minimal(on) => {
                self.baselines.minimal = on;
                self.baselines_run();
            }
            Event::Scale(v) => {
                if v > 0.0 {
                    self.baselines.scale = v;
                    self.baselines_run();
                }
            }
            Event::Robust(r) => {
                self.baselines.robust = r;
                // The method's own constant, until the user gives one.
                self.baselines.c = 0.0;
                self.baselines_run();
            }
            Event::C(v) => {
                if v > 0.0 {
                    self.baselines.c = v;
                    self.baselines_run();
                }
            }
            Event::Close => {
                self.baselines = State::default();
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn baselines_view(&self) -> Element<'_, Message> {
        let s = &self.baselines;
        let file = s.file.as_ref().and_then(|p| p.file_name()).map_or("Dosya seçilmedi".to_owned(), |n| n.to_string_lossy().into_owned());
        // One label column, the boxes the same width (the owner: a form's fields line up).
        let head = Form::new()
            .label_width(130.0)
            .field("Baz dosyası", row![words::secondary(OPEN, Some(msg(Event::Pick))), label::caption(file)].spacing(10).align_y(Center))
            .field("Varyans çarpanı", NumberInput::new(s.scale, |v| msg(Event::Scale(v))).step(0.5).decimals(2).point().width(140))
            .row(Switch::new(s.minimal, |on| msg(Event::Minimal(on))).label("Minimal datum (sabit yokken ilk bilinen istasyonu tut)"))
            .field("Dayanıklı kestirim", {
                let choice = Segmented::new([Robust::Off, Robust::Huber, Robust::Danish], s.robust, |r| msg(Event::Robust(r)));
                // The constant shares the row: it belongs to the weight function (the owner: related small controls).
                let used = match &s.answer {
                    Some(Ok(a)) => a.robust.as_ref().map(|r| r.c),
                    _ => None,
                };
                match (s.robust, used) {
                    (Robust::Off, _) | (_, None) => row![choice].align_y(Center),
                    (_, Some(c)) => row![choice, label::caption("c"), NumberInput::new(c, |v| msg(Event::C(v))).step(0.5).decimals(2).point().width(110)]
                        .spacing(10)
                        .align_y(Center),
                }
            });
        let mut body = Column::new().spacing(10).push(head);
        let mut lines = vec![words::text_line(
            Kind::Info,
            "Satırlar: K;ad;X;Y;Z[;sX;sY;sZ] bilinen, N;ad yeni istasyon, B;başlangıç;bitiş;dX;dY;dZ;xx;xy;xz;yy;yz;zz baz (ECEF, m, m²). Hiçbir şey yazılmaz.",
        )];
        let mut table: Option<Element<'_, Message>> = None;
        match &s.answer {
            Some(Ok(a)) => {
                let test = match (a.m0, a.passed) {
                    (Some(m0), Some(true)) => (Kind::Ok, format!("m0 = {m0:.3}; χ² testi geçti (vᵀPv {:.2} ≤ {:.2}); serbestlik {}.", a.omega, a.chi2.unwrap_or(0.0), a.f)),
                    (Some(m0), Some(false)) => (Kind::Warn, format!("m0 = {m0:.3}; χ² testi geçmedi (vᵀPv {:.2} > {:.2}): ağırlıklar ya da kaba hata.", a.omega, a.chi2.unwrap_or(0.0))),
                    _ => (Kind::Warn, "Fazlalık yok: m0 ve χ² testi yok, bazlar denetlenemez.".to_owned()),
                };
                lines.push(words::text_line(test.0, test.1));
                // Docs/adr/4560: the network's size (the normal equations are solved sparse, so a large one is read too).
                lines.push(words::text_line(
                    Kind::Info,
                    format!("{} istasyon, {} baz: {} bilinmeyen, {} gözlem denklemi.", a.stations.len(), a.baselines.len(), a.u, a.n),
                ));
                // Docs/adr/4960: robust estimation weighs a baseline down, it does not remove it; the regulation wants
                // a discordant measurement measured again.
                if let Some(r) = &a.robust {
                    let name = if r.method == RobustMethod::Huber { "Huber" } else { "Danimarka" };
                    let settled = if r.converged {
                        format!("{} adımda oturdu", r.rounds)
                    } else {
                        format!("{} adımda oturmadı", r.rounds)
                    };
                    let (kind, text) = match r.downweighted {
                        0 => (Kind::Ok, format!("Dayanıklı kestirim ({name}, c {:.2}): {settled}; hiçbir bazın ağırlığı düşmedi.", r.c)),
                        n => (
                            Kind::Warn,
                            format!("Dayanıklı kestirim ({name}, c {:.2}): {settled}; {n} bazın ağırlığı düştü, silinmedi: denetleyip yeniden ölçün.", r.c),
                        ),
                    };
                    lines.push(words::text_line(if r.converged { kind } else { Kind::Warn }, text));
                }
                if let Some(h) = &a.held {
                    lines.push(words::text_line(Kind::Info, format!("Minimal datum: “{h}” tutuldu.")));
                }
                if let Some(i) = a.worst {
                    if let Some(b) = a.baselines.get(i as usize) {
                        let kind = if b.flag == "blunder" { Kind::Error } else { Kind::Info };
                        lines.push(words::text_line(kind, format!("En kötü baz: {} → {} (w {}).", b.from, b.to, b.w.map_or("—".to_owned(), |w| format!("{w:.2}")))));
                    }
                }
                // The results in the kit's data table: its rows scroll inside it (the owner: data tables scroll).
                table = Some(match s.view {
                    View::Stations => sheet(
                        vec![col("İstasyon", 140.0), num("σ doğu (mm)", 110.0), num("σ kuzey (mm)", 110.0), num("σ yukarı (mm)", 110.0)],
                        a.stations
                            .iter()
                            .filter(|st| !st.held)
                            .map(|st| vec![st.name.clone(), mm(st.senu[0]), mm(st.senu[1]), mm(st.senu[2])])
                            .collect(),
                        "Yeni istasyon yok.",
                        240.0,
                    ),
                    View::Baselines => sheet(
                        {
                            let mut cols = vec![col("Başlangıç", 90.0), col("Bitiş", 90.0), num("Artık (mm)", 90.0), num("w", 60.0)];
                            if a.robust.is_some() {
                                cols.push(num("Ağırlık çarpanı", 120.0));
                            }
                            cols.push(col("Durum", 110.0));
                            cols
                        },
                        a.baselines
                            .iter()
                            .enumerate()
                            .map(|(i, b)| {
                                let len = (b.v[0] * b.v[0] + b.v[1] * b.v[1] + b.v[2] * b.v[2]).sqrt();
                                let state = match (b.flag.as_str(), a.worst == Some(i as u32)) {
                                    ("blunder", _) => "Kaba hata",
                                    _ if b.factor.is_some_and(|f| f < 1.0) => "Ağırlık düştü",
                                    (_, true) => "En kötü",
                                    _ if b.w.is_none() => "Denetlenemez",
                                    _ => "Geçti",
                                };
                                let mut cells = vec![b.from.clone(), b.to.clone(), mm(len), b.w.map_or("—".to_owned(), |w| format!("{w:.2}"))];
                                if let Some(f) = b.factor {
                                    cells.push(format!("{f:.4}"));
                                }
                                cells.push(state.to_owned());
                                cells
                            })
                            .collect(),
                        "Baz yok.",
                        240.0,
                    ),
                });
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => {}
        }
        body = body.push(words::summary(lines));
        if let Some(table) = table {
            body = body
                .push(Segmented::new([View::Baselines, View::Stations], s.view, |v| msg(Event::View(v))))
                .push(table);
        }
        Frame::new(TITLE).push(body).action(words::primary(CLOSE, Some(msg(Event::Close)))).width(680.0).modal(msg(Event::Close))
    }

    pub(crate) fn baselines_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(OPEN) => Some(msg(Event::Pick)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
