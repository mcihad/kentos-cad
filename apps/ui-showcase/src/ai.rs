//! Interactive AI surface gallery. This is a deterministic provider simulator;
//! it never sends the user's draft, attachments or drawing to a remote model.

use std::time::Duration;

use iced::Task;
use iced::time::Instant;
use kentos_ui::theme::motion;
use kentos_ui::widget::ai::{
    AiAction, AiEvent, AnswerOption, Attachment, AttachmentKind, Conversation, MessageData,
    PromptState, Question, QuestionEvent, Role, Source, Thought, ToolCall, ToolPhase, Usage,
};

use crate::message::Message;

pub const OUTPUT: &str = "gallery-ai-output";
pub const RESPONSE: &str = "## Parsel analizi hazır\n\nÜç örnek parseli inceledim. Toplam alan **1545.25 m²**; en büyük parsel **1244 / 7**.\n\n| Parsel | Alan |\n| --- | --- |\n| 1244 / 7 | 642.50 m² |\n| 1244 / 8 | 384.00 m² |\n| 1244 / 9 | 518.75 m² |\n\nAynı hesabı Python ile de yapabilirsiniz:\n\n```python\nareas = [642.5, 384.0, 518.75]\ntotal = sum(areas)\nprint(f\"Toplam: {total:.2f} m²\")\n```\n\n- Geometriler değişmedi.\n- Hesap, örnek katmanın alan değerlerini kullanıyor.\n\nİsterseniz sonraki adımda parselleri alanlarına göre sıralayabiliriz.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    Stream,
    Question,
    Approval,
    Error,
}
impl Scenario {
    pub const ALL: [Self; 4] = [Self::Stream, Self::Question, Self::Approval, Self::Error];
    pub fn label(self) -> &'static str {
        match self {
            Self::Stream => "Metin akışı",
            Self::Question => "Kullanıcı sorusu",
            Self::Approval => "Araç onayı",
            Self::Error => "Hata",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Conversation(AiEvent),
    Scenario(Scenario),
    Tick(Instant),
    Speed(usize),
    Reset,
    Question(QuestionEvent),
    Thought,
    Tool,
    ReducedMotion(bool),
}

pub struct Studio {
    pub conversation: Conversation,
    pub scenario: Scenario,
    pub speed: usize,
    pub question: Question,
    pub thought: Thought,
    pub tool: ToolCall,
    pub notice: String,
    started: Option<Instant>,
    stream_started: Option<Instant>,
    offset: usize,
    waiting: bool,
}

impl Default for Studio {
    fn default() -> Self {
        let mut conversation = Conversation::default();
        conversation.push(MessageData::new(
            1,
            Role::User,
            "Parsellerin toplam alanını hesapla ve bir Python örneği göster.",
        ));
        let mut assistant = MessageData::new(2, Role::Assistant, RESPONSE);
        assistant.thought = Some(Thought { summary: "Örnek katmanın alan değerleri karşılaştırıldı; toplam ve en büyük parsel hesaplandı.".into(), elapsed: Duration::from_millis(1240), expanded: false, complete: true });
        assistant.tools.push(ToolCall {
            id: 1,
            name: "Parsel alanlarını hesapla".into(),
            detail: "3 örnek kayıt incelendi.".into(),
            phase: ToolPhase::Complete,
            output: "toplam: 1545.25 m²\nen büyük: 1244 / 7".into(),
            elapsed: Some(Duration::from_millis(42)),
            expanded: false,
        });
        assistant.sources.push(Source {
            title: "Parseller · 3 örnek kayıt".into(),
            location: "drawing://sample/parcels".into(),
        });
        assistant.usage = Some(Usage {
            input_tokens: 36,
            output_tokens: 184,
            elapsed: Duration::from_millis(2480),
        });
        conversation.push(assistant);
        Self {
            conversation, scenario: Scenario::Stream, speed: 1,
            question: question(77),
            thought: Thought { summary: "Katman yapısını ve seçili nesneleri inceliyorum. Hesap için gerekli alanlar hazırlanıyor.".into(), elapsed: Duration::from_millis(840), expanded: true, complete: false },
            tool: ToolCall { id: 9, name: "Katmanı incele".into(), detail: "Parseller · alan ve geometri".into(), phase: ToolPhase::Running, output: "3 kayıt bulundu.\nKoordinat sistemi: EPSG:5254".into(), elapsed: None, expanded: false },
            notice: String::new(), started: None, stream_started: None, offset: 0, waiting: false,
        }
    }
}

pub fn question(id: u64) -> Question {
    Question::new(id, "Hangi parselleri inceleyelim?")
        .detail("Hesabın kapsamını seçin. Katmanda değişiklik yapılmayacak.")
        .option(
            AnswerOption::new("Seçili parseller", "Çizimde seçili 3 parseli incele.").recommended(),
        )
        .option(AnswerOption::new(
            "Bütün katman",
            "Katmandaki tüm parselleri incele.",
        ))
        .allow_skip(true)
}

impl Studio {
    pub fn playing(&self) -> bool {
        self.started.is_some() && !self.waiting && self.conversation.pending().is_some()
    }

    pub fn update(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Conversation(event) => {
                if matches!(event, AiEvent::Retry(_)) {
                    self.scenario = Scenario::Stream;
                }
                match self.conversation.update(event) {
                    Some(AiAction::Send(_)) => self.start(),
                    Some(AiAction::Stop(_)) => {
                        self.started = None;
                        self.notice = "Akış durduruldu; yazılmış yanıt ve taslak korunuyor.".into();
                    }
                    Some(AiAction::Answer { .. }) => {
                        self.waiting = false;
                        self.stream_started = Some(Instant::now());
                        self.notice = "Yanıtınız alındı; akış devam ediyor.".into();
                    }
                    Some(AiAction::ToolApproval {
                        request,
                        tool,
                        approved,
                    }) => {
                        self.waiting = false;
                        self.conversation.tool(
                            request,
                            ToolCall {
                                id: tool,
                                name: "Parsel alanlarını hesapla".into(),
                                detail: if approved {
                                    "Araç onaylandı."
                                } else {
                                    "Araç reddedildi; örnek veriyle devam ediliyor."
                                }
                                .into(),
                                phase: if approved {
                                    ToolPhase::Complete
                                } else {
                                    ToolPhase::Error
                                },
                                output: String::new(),
                                elapsed: Some(Duration::from_millis(42)),
                                expanded: false,
                            },
                        );
                        self.stream_started = Some(Instant::now());
                    }
                    Some(AiAction::Copy(text)) => {
                        self.notice = "Metin panoya kopyalandı.".into();
                        return iced::clipboard::write(text);
                    }
                    Some(AiAction::Attach) => {
                        let id = self
                            .conversation
                            .prompt
                            .attachments
                            .iter()
                            .map(|a| a.id)
                            .max()
                            .unwrap_or(0)
                            + 1;
                        self.conversation.prompt.attachments.push(Attachment {
                            id,
                            name: "parseller.geojson".into(),
                            kind: AttachmentKind::Drawing,
                            detail: "Örnek ek · 3 kayıt".into(),
                        });
                    }
                    Some(AiAction::Feedback { vote, .. }) => {
                        self.notice = format!(
                            "Geri bildirim alındı: {}.",
                            if vote == kentos_ui::widget::ai::Vote::Helpful {
                                "yararlı"
                            } else {
                                "yararlı değil"
                            }
                        )
                    }
                    Some(AiAction::OpenSource(location)) => {
                        self.notice = format!("Örnek kaynak: {location}")
                    }
                    None => {}
                }
            }
            Event::Scenario(scenario) => {
                self.scenario = scenario;
                self.conversation = Conversation::default();
                self.conversation.prompt =
                    PromptState::with_text("Parselleri incele ve sonucu açıkla.");
                return self.update(Event::Conversation(AiEvent::Submit));
            }
            Event::Tick(now) => {
                self.tick(now);
            }
            Event::Speed(speed) => self.speed = speed.min(2),
            Event::Reset => *self = Self::default(),
            Event::Question(event) => {
                if let Some(answer) = self.question.update(event) {
                    self.notice = if answer.skipped {
                        "Örnek soru atlandı.".into()
                    } else {
                        "Örnek sorunun yanıtı alındı.".into()
                    };
                }
            }
            Event::Thought => self.thought.expanded = !self.thought.expanded,
            Event::Tool => self.tool.expanded = !self.tool.expanded,
            Event::ReducedMotion(reduced) => motion::set_reduced(reduced),
        }
        Task::none()
    }

    fn start(&mut self) {
        self.started = Some(Instant::now());
        self.stream_started = None;
        self.offset = 0;
        self.waiting = false;
        self.notice.clear();
    }

    fn tick(&mut self, now: Instant) {
        let Some(started) = self.started else {
            return;
        };
        let Some(id) = self.conversation.pending() else {
            return;
        };
        let elapsed = now.saturating_duration_since(started);
        if self.waiting {
            return;
        }
        if self.stream_started.is_none() {
            self.conversation.thought(
                id,
                if elapsed < Duration::from_millis(700) {
                    "Örnek katmanın alan değerlerini inceliyorum."
                } else {
                    "Toplam alanı ve en büyük parseli karşılaştırıyorum."
                },
                elapsed,
            );
            if elapsed > Duration::from_millis(700) {
                self.conversation.tool(
                    id,
                    ToolCall {
                        id: 1,
                        name: "Parsel alanlarını hesapla".into(),
                        detail: "3 örnek kayıt".into(),
                        phase: ToolPhase::Running,
                        output: String::new(),
                        elapsed: None,
                        expanded: false,
                    },
                );
            }
            if elapsed < Duration::from_millis(1300) {
                return;
            }
            if self.scenario == Scenario::Error {
                self.conversation.fail(id, "Örnek model bağlantısı kesildi. Yazdığınız mesaj korunuyor; Yeniden dene ile akışı başlatabilirsiniz.");
                self.started = None;
                return;
            }
            self.conversation.tool(
                id,
                ToolCall {
                    id: 1,
                    name: "Parsel alanlarını hesapla".into(),
                    detail: "3 örnek kayıt".into(),
                    phase: if self.scenario == Scenario::Approval {
                        ToolPhase::Approval
                    } else {
                        ToolPhase::Complete
                    },
                    output: "Toplam: 1545.25 m²".into(),
                    elapsed: Some(Duration::from_millis(42)),
                    expanded: false,
                },
            );
            if self.scenario == Scenario::Question {
                self.conversation.ask(id, question(1));
                self.waiting = true;
                return;
            }
            if self.scenario == Scenario::Approval {
                self.waiting = true;
                return;
            }
            self.stream_started = Some(now);
        }
        let count = [2, 6, 16][self.speed];
        let end = RESPONSE[self.offset..]
            .char_indices()
            .nth(count)
            .map_or(RESPONSE.len(), |(offset, _)| self.offset + offset);
        if end > self.offset {
            self.conversation.append(id, &RESPONSE[self.offset..end]);
            self.offset = end;
        }
        if self.offset == RESPONSE.len() {
            self.conversation.finish(
                id,
                Some(Usage {
                    input_tokens: 36,
                    output_tokens: 184,
                    elapsed,
                }),
            );
            self.started = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_ui::widget::ai::Phase;
    #[test]
    fn gallery_streams_questions_resume_and_stop_rejects_late_ticks() {
        let mut studio = Studio::default();
        let _ = studio.update(Event::Scenario(Scenario::Question));
        let start = studio.started.unwrap();
        studio.tick(start + Duration::from_secs(2));
        assert_eq!(studio.conversation.phase(), Phase::Waiting);
        assert!(!studio.playing());
        let id = studio.conversation.pending().unwrap();
        let _ = studio.update(Event::Conversation(AiEvent::Question {
            message: id,
            event: QuestionEvent::Choose { id: 1, index: 0 },
        }));
        let _ = studio.update(Event::Conversation(AiEvent::Question {
            message: id,
            event: QuestionEvent::Submit { id: 1 },
        }));
        assert!(studio.playing());
        studio.tick(start + Duration::from_secs(3));
        assert!(!studio.conversation.message(id).unwrap().body.is_empty());
        let _ = studio.update(Event::Conversation(AiEvent::Stop));
        let body = studio.conversation.message(id).unwrap().body.clone();
        studio.tick(start + Duration::from_secs(4));
        assert_eq!(studio.conversation.message(id).unwrap().body, body);
    }

    #[test]
    #[ignore = "writes AI gallery images; KENTOS_SNAPSHOT_BACKEND=wgpu enables GPU capture"]
    fn screens() {
        use crate::{app::Showcase, gallery::Page, message::RibbonTab};
        use iced::{Event as NativeEvent, Size, window};
        use kentos_ui::{
            snapshot::Snapshot,
            theme::{Mode, shape},
        };

        let directory = std::path::Path::new(".run/shots/ai");
        std::fs::create_dir_all(directory).unwrap();
        let mut app = Showcase::new();
        app.ribbon_tab = RibbonTab::Gallery;
        app.gallery.page = Page::Ai;
        motion::set_reduced(true);
        shape::set(shape::Shape {
            corners: shape::Corners::Round,
            ..shape::Shape::default()
        });
        let mut snapshot = Snapshot::new(Size::new(1440.0, 1150.0)).unwrap();
        let mut update = |app: &mut Showcase, event| {
            let _ = app.update(event);
        };
        snapshot.settle(&mut app, Showcase::view, &mut update);
        for (mode, name) in [
            (Mode::Dark, "dark"),
            (Mode::Light, "light"),
            (Mode::Night, "night"),
            (Mode::HighContrast, "contrast"),
        ] {
            let _ = app.update(Message::ThemeSelected(mode));
            snapshot
                .render(app.view(), &app.theme())
                .save(directory.join(format!("conversation-{name}.png")))
                .unwrap();
        }
        let _ = app.update(Message::ThemeSelected(Mode::Dark));
        for (scenario, name) in [
            (Scenario::Question, "question"),
            (Scenario::Approval, "approval"),
            (Scenario::Error, "error"),
        ] {
            let _ = app.ai_studio.update(Event::Scenario(scenario));
            let start = app.ai_studio.started.unwrap();
            app.ai_studio.tick(start + Duration::from_secs(2));
            snapshot.settle(&mut app, Showcase::view, &mut update);
            snapshot
                .render(app.view(), &app.theme())
                .save(directory.join(format!("scenario-{name}.png")))
                .unwrap();
        }

        // Actual animation stays enabled for these captures: real native redraw
        // events advance the paragraph cache, orbit and accordion independently.
        motion::set_reduced(false);
        let _ = app.ai_studio.update(Event::Scenario(Scenario::Stream));
        let start = app.ai_studio.started.unwrap();
        for frame in 0..110 {
            app.ai_studio
                .tick(start + Duration::from_millis(1500 + frame * 30));
            snapshot.step(
                &mut app,
                Showcase::view,
                &mut update,
                &[NativeEvent::Window(window::Event::RedrawRequested(
                    Instant::now(),
                ))],
            );
            if [2, 35, 100].contains(&frame) {
                // Give visual reveal its own frames while the provider pauses.
                std::thread::sleep(Duration::from_millis(32));
                snapshot.step(
                    &mut app,
                    Showcase::view,
                    &mut update,
                    &[NativeEvent::Window(window::Event::RedrawRequested(
                        Instant::now(),
                    ))],
                );
                snapshot
                    .render(app.view(), &app.theme())
                    .save(directory.join(format!("stream-{frame}.png")))
                    .unwrap();
            }
        }
        let _ = app.ai_studio.update(Event::Tool);
        snapshot.settle(&mut app, Showcase::view, &mut update);
        std::thread::sleep(Duration::from_millis(120));
        snapshot.step(
            &mut app,
            Showcase::view,
            &mut update,
            &[NativeEvent::Window(window::Event::RedrawRequested(
                Instant::now(),
            ))],
        );
        snapshot
            .render(app.view(), &app.theme())
            .save(directory.join("tool-expanding.png"))
            .unwrap();
        println!("{}: {}", directory.display(), snapshot.renderer_name());
        shape::set(shape::Shape::default());
        motion::set_reduced(false);
    }
}
