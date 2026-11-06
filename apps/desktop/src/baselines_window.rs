//! GNSS baz ağı (docs/adr/1380; `survey.baselines`): a file of stations and baselines (`K;…`, `N;…`, `B;…` lines) read
//! and adjusted by `cad.survey.baselines`, read only. The window shows the network's m0 and χ² test, each station's
//! standard deviations in east, north, up (mm) and each baseline's residual and test; Minimal datum and the variance
//! factor adjust it again. Nothing is written: taking the coordinates is a later, separate step.

use std::path::PathBuf;

use iced::widget::{Column, row};
use iced::{Center, Element, Task};
use kentos_commands::{BaselineDatum, CommandResult, SurveyBaselines, SurveyBaselinesAdjusted};
use kentos_native_application::{ExecutionContext, survey_baselines};
use kentos_ui::label;
use kentos_ui::widget::{Dialog as Frame, NumberInput, Switch};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

pub const TITLE: &str = "GNSS baz ağı";
const CLOSE: &str = "Kapat";
const OPEN: &str = "Dosya seç…";

#[derive(Clone, Debug, Default)]
pub struct State {
    pub file: Option<PathBuf>,
    pub text: Option<String>,
    pub minimal: bool,
    pub scale: f64,
    pub answer: Option<Result<SurveyBaselinesAdjusted, String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Pick,
    Picked(Option<PathBuf>),
    Minimal(bool),
    Scale(f64),
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
        let head = row![
            words::ghost(OPEN, Some(msg(Event::Pick))),
            label::caption(file),
            Switch::new(s.minimal, |on| msg(Event::Minimal(on))).label("Minimal datum"),
            label::body("Varyans çarpanı"),
            NumberInput::new(s.scale, |v| msg(Event::Scale(v))).step(0.5).decimals(2).point().width(90),
        ]
        .spacing(12)
        .align_y(Center);
        let mut body = Column::new().spacing(10).push(head);
        let mut lines = vec![words::text_line(
            Kind::Info,
            "Satırlar: K;ad;X;Y;Z[;sX;sY;sZ] bilinen, N;ad yeni istasyon, B;başlangıç;bitiş;dX;dY;dZ;xx;xy;xz;yy;yz;zz baz (ECEF, m, m²). Hiçbir şey yazılmaz.",
        )];
        match &s.answer {
            Some(Ok(a)) => {
                let test = match (a.m0, a.passed) {
                    (Some(m0), Some(true)) => (Kind::Ok, format!("m0 = {m0:.3}; χ² testi geçti (vᵀPv {:.2} ≤ {:.2}); serbestlik {}.", a.omega, a.chi2.unwrap_or(0.0), a.f)),
                    (Some(m0), Some(false)) => (Kind::Warn, format!("m0 = {m0:.3}; χ² testi geçmedi (vᵀPv {:.2} > {:.2}): ağırlıklar ya da kaba hata.", a.omega, a.chi2.unwrap_or(0.0))),
                    _ => (Kind::Warn, "Fazlalık yok: m0 ve χ² testi yok, bazlar denetlenemez.".to_owned()),
                };
                lines.push(words::text_line(test.0, test.1));
                if let Some(h) = &a.held {
                    lines.push(words::text_line(Kind::Info, format!("Minimal datum: “{h}” tutuldu.")));
                }
                for st in a.stations.iter().filter(|st| !st.held) {
                    lines.push(words::text_line(
                        Kind::Ok,
                        format!("{}: σ doğu {} mm, kuzey {} mm, yukarı {} mm", st.name, mm(st.senu[0]), mm(st.senu[1]), mm(st.senu[2])),
                    ));
                }
                for (i, b) in a.baselines.iter().enumerate() {
                    let len = (b.v[0] * b.v[0] + b.v[1] * b.v[1] + b.v[2] * b.v[2]).sqrt();
                    let w = b.w.map_or("denetlenemez".to_owned(), |w| format!("w {w:.2}"));
                    let kind = if b.flag == "blunder" { Kind::Error } else { Kind::Ok };
                    let worst = if a.worst == Some(i as u32) { " — en kötü baz" } else { "" };
                    lines.push(words::text_line(kind, format!("{} → {}: artık {} mm, {w}{worst}", b.from, b.to, mm(len))));
                }
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => {}
        }
        body = body.push(words::summary(lines));
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
