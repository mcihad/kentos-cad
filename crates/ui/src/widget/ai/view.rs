use std::rc::Rc;

use iced::advanced::text::{LineHeight, Wrapping};
use iced::keyboard::{Key, key::Named};
use iced::widget::text_editor::{Binding, Status};
use iced::widget::{
    Column, button, column, container, responsive, row, scrollable, space, text, text_editor,
    text_input,
};
use iced::{Border, Element, Fill, Length, Padding, Theme};

use crate::icon::{Icon, icon};
use crate::style;
use crate::theme::{Tokens, shape, typography};
use crate::widget::animated_surface::{Activity, Surface, status_color};
use crate::widget::python::font as code_font;

use super::disclosure::Disclosure;
use super::{
    AiActivity, AiContent, AiEvent, AiStream, Attachment, AttachmentKind, Conversation,
    MessageData, Phase, PromptState, Question, QuestionEvent, Role, Thought, ToolCall, ToolPhase,
    Vote,
};

type Events<'a, Message> = Rc<dyn Fn(AiEvent) -> Message + 'a>;

fn small<'a, Message: Clone + 'a>(
    glyph: Icon,
    label: &'a str,
    message: Message,
) -> Element<'a, Message> {
    button(
        row![
            icon(glyph).size(12.0),
            text(label).size(typography::caption())
        ]
        .spacing(5)
        .align_y(iced::Center),
    )
    .on_press(message)
    .padding([5, 8])
    .style(style::button::flat)
    .into()
}

fn tag<'a, Message: 'a>(label: impl iced::widget::text::IntoFragment<'a>) -> Element<'a, Message> {
    container(
        text(label)
            .size(typography::caption())
            .style(style::text::muted),
    )
    .padding([3, 7])
    .style(|theme: &Theme| {
        let t = Tokens::of(theme);
        container::Style {
            background: Some(t.surface_alt.into()),
            border: Border {
                color: t.border,
                width: 1.0,
                radius: shape::sm().into(),
            },
            ..container::Style::default()
        }
    })
    .into()
}

fn attachment<'a, Message: Clone + 'a>(
    data: &'a Attachment,
    remove: Option<Message>,
) -> Element<'a, Message> {
    let glyph = match data.kind {
        AttachmentKind::Image => Icon::Grid,
        AttachmentKind::Code => Icon::Terminal,
        AttachmentKind::Drawing => Icon::Layers,
        AttachmentKind::Document => Icon::Document,
    };
    let mut line = row![
        icon(glyph).size(14.0),
        text(data.name.as_str()).size(typography::body()),
        text(data.detail.as_str())
            .size(typography::caption())
            .style(style::text::muted)
    ]
    .spacing(7)
    .align_y(iced::Center);
    if let Some(remove) = remove {
        line = line.push(small(Icon::Close, "Kaldır", remove));
    }
    container(line)
        .padding([5, 8])
        .style(style::container::bordered)
        .into()
}

/// Multiline prompt composer, attachments, model label and keyboard shortcuts.
/// Enter sends; Shift+Enter inserts a newline. During an active request the
/// composer keeps a next-message draft and exposes a separate Stop action.
pub struct AiPrompt<'a, Message> {
    state: &'a PromptState,
    on_event: Events<'a, Message>,
    busy: bool,
    model: &'a str,
    placeholder: &'a str,
    attach: bool,
}

impl<'a, Message: Clone + 'a> AiPrompt<'a, Message> {
    pub fn new(state: &'a PromptState, on_event: impl Fn(AiEvent) -> Message + 'a) -> Self {
        Self {
            state,
            on_event: Rc::new(on_event),
            busy: false,
            model: "Asistan",
            placeholder: "Bir soru sorun veya yapmak istediğinizi anlatın…",
            attach: false,
        }
    }
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }
    pub fn model(mut self, model: &'a str) -> Self {
        self.model = model;
        self
    }
    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }
    /// The host receives `AiAction::Attach` and chooses the actual file flow.
    pub fn attachments(mut self, enabled: bool) -> Self {
        self.attach = enabled;
        self
    }

    fn view(self) -> Element<'a, Message> {
        let events = self.on_event.clone();
        let keys = self.on_event.clone();
        let busy = self.busy;
        let ready = self.state.ready();
        let mut body = Column::new().spacing(8);
        if !self.state.attachments.is_empty() {
            let mut attachments = Column::new().spacing(5);
            for data in &self.state.attachments {
                attachments = attachments.push(attachment(
                    data,
                    Some((self.on_event)(AiEvent::RemoveAttachment(data.id))),
                ));
            }
            body = body.push(container(attachments).padding(Padding::new(10.0).bottom(0.0)));
        }
        let size = typography::body() + 2.0;
        let input = text_editor(&self.state.content)
            .on_action(move |action| events(AiEvent::Edit(action)))
            .font(typography::ui())
            .size(size)
            .line_height(LineHeight::Relative(1.6))
            .placeholder(self.placeholder)
            .padding([12, 14])
            .height(
                (self.state.content.line_count().clamp(2, 6) as f32 * size * 1.6 + 24.0).round(),
            )
            .key_binding(move |kp| {
                if !matches!(kp.status, Status::Focused { .. }) {
                    return None;
                }
                match kp.key.as_ref() {
                    Key::Named(Named::Enter)
                        if !kp.modifiers.shift() && !kp.modifiers.alt() && !busy && ready =>
                    {
                        Some(Binding::Custom(keys(AiEvent::Submit)))
                    }
                    Key::Character(ch)
                        if kp.modifiers.command() && ch.eq_ignore_ascii_case("z") =>
                    {
                        Some(Binding::Custom(keys(if kp.modifiers.shift() {
                            AiEvent::Redo
                        } else {
                            AiEvent::Undo
                        })))
                    }
                    _ => Binding::from_key_press(kp),
                }
            })
            .style(|theme: &Theme, _| {
                let t = Tokens::of(theme);
                text_editor::Style {
                    background: iced::Color::TRANSPARENT.into(),
                    border: Border::default(),
                    placeholder: t.muted,
                    value: t.text,
                    selection: t.selection(),
                }
            });
        body = body.push(input);
        let controls = responsive(move |bounds| {
            let mut model = row![tag(self.model)].spacing(8).align_y(iced::Center);
            if self.attach {
                model = model.push(small(Icon::Plus, "Ekle", (self.on_event)(AiEvent::Attach)));
            }
            let hint = text("Shift+Enter yeni satır")
                .size(typography::caption())
                .style(style::text::muted);
            let send = button(
                row![
                    icon(if busy { Icon::Stop } else { Icon::ChevronUp }).size(13.0),
                    text(if busy { "Durdur" } else { "Gönder" }).size(typography::body())
                ]
                .spacing(6)
                .align_y(iced::Center),
            )
            .on_press_maybe(
                (busy || ready)
                    .then(|| (self.on_event)(if busy { AiEvent::Stop } else { AiEvent::Submit })),
            )
            .padding([7, 12])
            .style(if busy {
                style::button::secondary
            } else {
                style::button::primary
            });
            if bounds.width < typography::scaled(530.0) {
                column![
                    model,
                    row![hint, space::horizontal(), send].align_y(iced::Center)
                ]
                .spacing(8)
                .into()
            } else {
                model.push(space::horizontal()).push(hint).push(send).into()
            }
        })
        .height(Length::Shrink);
        body = body.push(
            container(controls)
                .width(Fill)
                .padding(Padding::new(0.0).left(12.0).right(12.0).bottom(12.0)),
        );
        Surface::new(container(body).width(Fill).padding(1), Activity::Ready, 0).into()
    }
}
impl<'a, Message: Clone + 'a> From<AiPrompt<'a, Message>> for Element<'a, Message> {
    fn from(prompt: AiPrompt<'a, Message>) -> Self {
        prompt.view()
    }
}

/// A selectable question card with descriptions, optional multiple choice,
/// custom text, an explicit submit button and a retained answer summary.
pub struct AiQuestion<'a, Message> {
    question: &'a Question,
    on_event: Rc<dyn Fn(QuestionEvent) -> Message + 'a>,
    enabled: bool,
}
impl<'a, Message: Clone + 'a> AiQuestion<'a, Message> {
    pub fn new(question: &'a Question, on_event: impl Fn(QuestionEvent) -> Message + 'a) -> Self {
        Self {
            question,
            on_event: Rc::new(on_event),
            enabled: true,
        }
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    fn view(self) -> Element<'a, Message> {
        let q = self.question;
        let enabled = self.enabled && q.answer.is_none();
        let mut body = column![
            row![
                icon(if q.answer.is_some() {
                    Icon::Check
                } else {
                    Icon::Help
                })
                .size(16.0),
                text(q.title.as_str())
                    .font(typography::ui_strong())
                    .size(typography::body() + 2.0)
                    .width(Fill)
            ]
            .spacing(8)
            .align_y(iced::Center)
        ]
        .spacing(10);
        if !q.detail.is_empty() {
            body = body.push(
                text(q.detail.as_str())
                    .size(typography::body())
                    .style(style::text::muted),
            );
        }
        if let Some(answer) = &q.answer {
            let summary = if answer.skipped {
                "Bu soru atlandı.".into()
            } else if !answer.text.is_empty() {
                answer.text.clone()
            } else {
                answer
                    .choices
                    .iter()
                    .filter_map(|index| q.options.get(*index).map(|option| option.label.as_str()))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            body = body.push(
                row![
                    tag("Yanıtlandı"),
                    text(summary).size(typography::body() + 1.0)
                ]
                .spacing(10)
                .align_y(iced::Center),
            );
        } else {
            for (index, option) in q.options.iter().enumerate() {
                let selected = q.selected.contains(&index);
                let mark: Element<'a, Message> = if selected {
                    icon(Icon::Check).size(12.0).into()
                } else {
                    text((index + 1).to_string())
                        .size(typography::caption())
                        .into()
                };
                let indicator =
                    container(mark)
                        .width(25)
                        .height(25)
                        .center(25)
                        .style(move |theme: &Theme| {
                            let t = Tokens::of(theme);
                            container::Style {
                                text_color: Some(if selected { t.on_accent } else { t.muted }),
                                background: Some(
                                    if selected { t.accent } else { t.surface_alt }.into(),
                                ),
                                border: Border {
                                    color: if selected { t.accent } else { t.border },
                                    width: 1.0,
                                    radius: if q.multiple { shape::sm() } else { 13.0 }.into(),
                                },
                                ..container::Style::default()
                            }
                        });
                let mut title = row![
                    text(option.label.as_str())
                        .font(typography::ui_strong())
                        .size(typography::body() + 1.0)
                ]
                .spacing(8)
                .align_y(iced::Center);
                if option.recommended {
                    title = title.push(tag("Önerilen"));
                }
                let mut words = column![title].spacing(3).width(Fill);
                if !option.description.is_empty() {
                    words = words.push(
                        text(option.description.as_str())
                            .size(typography::body())
                            .style(style::text::muted),
                    );
                }
                body =
                    body.push(
                        button(row![indicator, words].spacing(10).align_y(iced::Center))
                            .on_press_maybe(enabled.then(|| {
                                (self.on_event)(QuestionEvent::Choose { id: q.id, index })
                            }))
                            .width(Fill)
                            .padding([10, 12])
                            .style(style::button::list_item(selected)),
                    );
            }
            if enabled {
                let events = self.on_event.clone();
                let id = q.id;
                let mut custom = text_input("Kendi yanıtınızı yazabilirsiniz…", &q.custom)
                    .on_input(move |text| events(QuestionEvent::Custom { id, text }))
                    .font(typography::ui())
                    .size(typography::body() + 1.0)
                    .padding([9, 12])
                    .style(style::field::input);
                if q.ready() {
                    custom = custom.on_submit((self.on_event)(QuestionEvent::Submit { id }));
                }
                body = body.push(custom);
                let mut actions = row![
                    text(if q.multiple {
                        "Birden fazla seçenek seçebilirsiniz."
                    } else {
                        "Bir seçenek seçin veya kendi yanıtınızı yazın."
                    })
                    .size(typography::caption())
                    .width(Fill)
                    .style(style::text::muted)
                ]
                .spacing(8)
                .align_y(iced::Center);
                if q.allow_skip {
                    actions = actions.push(small(
                        Icon::ChevronRight,
                        "Atla",
                        (self.on_event)(QuestionEvent::Skip { id }),
                    ));
                }
                actions = actions.push(
                    button(text("Yanıtı gönder").size(typography::body()))
                        .on_press_maybe(
                            q.ready()
                                .then(|| (self.on_event)(QuestionEvent::Submit { id })),
                        )
                        .padding([7, 12])
                        .style(style::button::primary),
                );
                body = body.push(actions);
            } else {
                body = body.push(
                    text("Bu soru kapatıldı.")
                        .size(typography::caption())
                        .style(style::text::muted),
                );
            }
        }
        Surface::new(
            container(body).width(Fill).padding(16),
            if q.answer.is_some() {
                Activity::Success
            } else {
                Activity::Ready
            },
            u64::from(q.answer.is_some()),
        )
        .into()
    }
}
impl<'a, Message: Clone + 'a> From<AiQuestion<'a, Message>> for Element<'a, Message> {
    fn from(question: AiQuestion<'a, Message>) -> Self {
        question.view()
    }
}

/// Collapsible provider supplied reasoning summary, with a live particle orbit.
pub struct AiThinking<'a, Message> {
    thought: &'a Thought,
    active: bool,
    toggle: Option<Message>,
}
impl<'a, Message: Clone + 'a> AiThinking<'a, Message> {
    pub fn new(thought: &'a Thought) -> Self {
        Self {
            thought,
            active: !thought.complete,
            toggle: None,
        }
    }
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
    pub fn on_toggle(mut self, message: Message) -> Self {
        self.toggle = Some(message);
        self
    }
    fn view(self) -> Element<'a, Message> {
        let thought = self.thought;
        let label = if self.active {
            "Düşünüyor"
        } else {
            "Düşünme özeti"
        };
        let header = row![
            AiActivity::new(self.active).size(18.0),
            text(label)
                .size(typography::body())
                .font(typography::ui_strong()),
            text(format!("{:.1} sn", thought.elapsed.as_secs_f64()))
                .size(typography::caption())
                .style(style::text::muted),
            space::horizontal(),
            icon(if thought.expanded {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            })
            .size(12.0)
        ]
        .spacing(8)
        .align_y(iced::Center);
        let mut body = Column::new().spacing(8).push(
            button(header)
                .on_press_maybe(self.toggle)
                .width(Fill)
                .padding([7, 9])
                .style(style::button::flat),
        );
        if !thought.summary.is_empty() {
            body = body.push(Disclosure::new(
                container(
                    AiStream::new(&thought.summary)
                        .active(self.active)
                        .size(typography::body()),
                )
                .padding(Padding::new(0.0).left(35.0).right(10.0).bottom(8.0)),
                thought.expanded,
            ));
        }
        container(body)
            .width(Fill)
            .style(|theme: &Theme| container::Style {
                background: Some(Tokens::of(theme).surface.into()),
                border: Border {
                    radius: shape::md().into(),
                    ..Border::default()
                },
                ..container::Style::default()
            })
            .into()
    }
}
impl<'a, Message: Clone + 'a> From<AiThinking<'a, Message>> for Element<'a, Message> {
    fn from(thought: AiThinking<'a, Message>) -> Self {
        thought.view()
    }
}

/// Tool progress, input/output details and explicit approval actions. The host
/// receives events and owns the actual tool execution and permissions.
pub struct AiToolCall<'a, Message> {
    call: &'a ToolCall,
    toggle: Option<Message>,
    approve: Option<Message>,
    reject: Option<Message>,
}
impl<'a, Message: Clone + 'a> AiToolCall<'a, Message> {
    pub fn new(call: &'a ToolCall) -> Self {
        Self {
            call,
            toggle: None,
            approve: None,
            reject: None,
        }
    }
    pub fn on_toggle(mut self, message: Message) -> Self {
        self.toggle = Some(message);
        self
    }
    pub fn on_approve(mut self, message: Message) -> Self {
        self.approve = Some(message);
        self
    }
    pub fn on_reject(mut self, message: Message) -> Self {
        self.reject = Some(message);
        self
    }
    fn view(self) -> Element<'a, Message> {
        let call = self.call;
        let (label, state) = match call.phase {
            ToolPhase::Pending => ("Bekliyor", Activity::Ready),
            ToolPhase::Running => ("Çalışıyor", Activity::Running),
            ToolPhase::Approval => ("Onay gerekiyor", Activity::Ready),
            ToolPhase::Complete => ("Tamamlandı", Activity::Success),
            ToolPhase::Stopped => ("Durduruldu", Activity::Ready),
            ToolPhase::Error => ("Başarısız", Activity::Error),
        };
        let mut header = row![
            AiActivity::new(call.phase == ToolPhase::Running).size(18.0),
            text(call.name.as_str())
                .font(typography::ui_strong())
                .size(typography::body())
                .width(Fill),
            text(label)
                .size(typography::caption())
                .style(move |theme: &Theme| text::Style {
                    color: Some(status_color(state, &Tokens::of(theme)))
                })
        ]
        .spacing(8)
        .align_y(iced::Center);
        if let Some(elapsed) = call.elapsed {
            header = header.push(
                text(format!("{:.0} ms", elapsed.as_secs_f64() * 1000.0))
                    .font(code_font())
                    .size(typography::caption())
                    .style(style::text::muted),
            );
        }
        header = header.push(
            icon(if call.expanded {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            })
            .size(12.0),
        );
        let mut body = Column::new().spacing(8).push(
            button(header)
                .on_press_maybe(self.toggle)
                .width(Fill)
                .padding([8, 10])
                .style(style::button::flat),
        );
        {
            let mut details = Column::new().spacing(8);
            if !call.detail.is_empty() {
                details = details.push(
                    container(
                        text(call.detail.as_str())
                            .size(typography::body())
                            .style(style::text::muted),
                    )
                    .padding([0, 12]),
                );
            }
            if !call.output.is_empty() {
                details = details.push(
                    container(
                        scrollable(
                            text(call.output.as_str())
                                .font(code_font())
                                .size(typography::body())
                                .wrapping(Wrapping::WordOrGlyph),
                        )
                        .height(
                            (call.output.lines().count().clamp(1, 7) as f32
                                * typography::body()
                                * 1.5)
                                .max(30.0),
                        ),
                    )
                    .padding([5, 12]),
                );
            }
            body = body.push(Disclosure::new(details, call.expanded));
        }
        if call.phase == ToolPhase::Approval && (self.approve.is_some() || self.reject.is_some()) {
            body = body.push(
                container(
                    row![
                        text(call.detail.as_str())
                            .size(typography::body())
                            .width(Fill),
                        button(text("Reddet").size(typography::body()))
                            .on_press_maybe(self.reject)
                            .padding([6, 10])
                            .style(style::button::secondary),
                        button(text("Onayla").size(typography::body()))
                            .on_press_maybe(self.approve)
                            .padding([6, 10])
                            .style(style::button::primary)
                    ]
                    .spacing(8)
                    .align_y(iced::Center),
                )
                .padding([5, 12]),
            );
        }
        Surface::new(
            container(body).width(Fill).padding(3),
            state,
            u64::from(call.expanded),
        )
        .into()
    }
}
impl<'a, Message: Clone + 'a> From<AiToolCall<'a, Message>> for Element<'a, Message> {
    fn from(tool: AiToolCall<'a, Message>) -> Self {
        tool.view()
    }
}

/// An individual message, usable in a custom dock, inspector or transcript.
pub struct AiMessage<'a, Message> {
    data: &'a MessageData,
    on_event: Events<'a, Message>,
    assistant: &'a str,
}
impl<'a, Message: Clone + 'a> AiMessage<'a, Message> {
    pub fn new(data: &'a MessageData, on_event: impl Fn(AiEvent) -> Message + 'a) -> Self {
        Self {
            data,
            on_event: Rc::new(on_event),
            assistant: "KentOS Asistan",
        }
    }
    pub fn assistant(mut self, name: &'a str) -> Self {
        self.assistant = name;
        self
    }
    fn view(self) -> Element<'a, Message> {
        let data = self.data;
        let id = data.id;
        if data.role == Role::User {
            let mut body = column![
                text("Siz")
                    .font(typography::ui_strong())
                    .size(typography::caption())
                    .style(style::text::muted),
                text(data.body.as_str())
                    .font(typography::ui())
                    .size(typography::body() + 2.0)
                    .line_height(LineHeight::Relative(1.6))
                    .width(Fill)
            ]
            .spacing(8);
            for file in &data.attachments {
                body = body.push(attachment(file, None));
            }
            return container(container(body).max_width(680).padding([14, 18]).style(
                |theme: &Theme| {
                    let t = Tokens::of(theme);
                    container::Style {
                        background: Some(t.surface_alt.into()),
                        border: Border {
                            color: t.border,
                            width: 1.0,
                            radius: shape::lg().into(),
                        },
                        ..container::Style::default()
                    }
                },
            ))
            .padding(Padding::new(0.0).left(48.0))
            .width(Fill)
            .align_right(Fill)
            .into();
        }
        if data.role == Role::System {
            return container(
                text(data.body.as_str())
                    .size(typography::caption())
                    .style(style::text::muted),
            )
            .padding([8, 0])
            .into();
        }
        let phase = data.phase;
        let mut body = column![
            row![
                AiActivity::new(phase.active()).size(26.0),
                text(self.assistant)
                    .font(typography::ui_strong())
                    .size(typography::body() + 1.0),
                space::horizontal(),
                text(phase.label())
                    .size(typography::caption())
                    .style(move |theme: &Theme| text::Style {
                        color: Some(status_color(phase, &Tokens::of(theme)))
                    })
            ]
            .spacing(9)
            .align_y(iced::Center)
        ]
        .spacing(12)
        .width(Fill);
        if let Some(thought) = &data.thought {
            body = body.push(
                AiThinking::new(thought)
                    .active(phase == Phase::Thinking)
                    .on_toggle((self.on_event)(AiEvent::ToggleThought(id))),
            );
        }
        for tool in &data.tools {
            let mut view = AiToolCall::new(tool).on_toggle((self.on_event)(AiEvent::ToggleTool {
                message: id,
                tool: tool.id,
            }));
            if !matches!(phase, Phase::Stopped | Phase::Error | Phase::Complete) {
                view = view
                    .on_approve((self.on_event)(AiEvent::ApproveTool {
                        message: id,
                        tool: tool.id,
                        approved: true,
                    }))
                    .on_reject((self.on_event)(AiEvent::ApproveTool {
                        message: id,
                        tool: tool.id,
                        approved: false,
                    }));
            }
            body = body.push(view);
        }
        if !data.body.is_empty() || phase == Phase::Answering {
            let links = self.on_event.clone();
            let copies = self.on_event.clone();
            body = body.push(
                AiContent::new(&data.body)
                    .streaming(phase == Phase::Answering)
                    .on_link(move |location| links(AiEvent::OpenSource(location)))
                    .on_copy(move |code| copies(AiEvent::CopyText(code))),
            );
        }
        if let Some(failure) = &data.failure {
            body = body.push(
                container(
                    row![
                        icon(Icon::Close).size(16.0),
                        text(failure.as_str()).size(typography::body()).width(Fill)
                    ]
                    .spacing(8)
                    .align_y(iced::Center),
                )
                .padding(12)
                .width(Fill)
                .style(|theme: &Theme| {
                    let t = Tokens::of(theme);
                    container::Style {
                        text_color: Some(t.danger),
                        background: Some(t.danger.scale_alpha(0.07).into()),
                        border: Border {
                            color: t.danger.scale_alpha(0.45),
                            width: 1.0,
                            radius: shape::md().into(),
                        },
                        ..container::Style::default()
                    }
                }),
            );
        }
        for question in &data.questions {
            let events = self.on_event.clone();
            body = body.push(
                AiQuestion::new(question, move |event| {
                    events(AiEvent::Question { message: id, event })
                })
                .enabled(!matches!(
                    phase,
                    Phase::Stopped | Phase::Error | Phase::Complete
                )),
            );
        }
        if !data.sources.is_empty() {
            let mut sources = Column::new().spacing(4).push(
                text("Kaynaklar")
                    .font(typography::ui_strong())
                    .size(typography::caption())
                    .style(style::text::muted),
            );
            for (index, source) in data.sources.iter().enumerate() {
                sources = sources.push(
                    button(
                        row![
                            text(format!("[{}]", index + 1))
                                .font(code_font())
                                .size(typography::caption()),
                            icon(Icon::Link).size(12.0),
                            text(source.title.as_str()).size(typography::body())
                        ]
                        .spacing(7)
                        .align_y(iced::Center),
                    )
                    .on_press((self.on_event)(AiEvent::OpenSource(
                        source.location.clone(),
                    )))
                    .padding([4, 0])
                    .style(style::button::flat),
                );
            }
            body = body.push(sources);
        }
        if matches!(phase, Phase::Complete | Phase::Stopped | Phase::Error) {
            let mut actions = row![
                small(Icon::Copy, "Kopyala", (self.on_event)(AiEvent::Copy(id))),
                small(
                    Icon::Retry,
                    "Yeniden dene",
                    (self.on_event)(AiEvent::Retry(id))
                )
            ]
            .spacing(4)
            .align_y(iced::Center);
            for (vote, label, glyph) in [
                (Vote::Helpful, "Yararlı", Icon::Check),
                (Vote::Unhelpful, "Yararlı değil", Icon::Close),
            ] {
                actions = actions.push(
                    button(
                        row![
                            icon(glyph).size(12.0),
                            text(label).size(typography::caption())
                        ]
                        .spacing(4)
                        .align_y(iced::Center),
                    )
                    .on_press((self.on_event)(AiEvent::Feedback { message: id, vote }))
                    .padding([5, 8])
                    .style(style::button::toggle(data.vote == Some(vote))),
                );
            }
            if let Some(usage) = data.usage {
                actions = actions.push(space::horizontal()).push(
                    text(format!(
                        "{} token · {:.1} sn",
                        usage.output_tokens,
                        usage.elapsed.as_secs_f64()
                    ))
                    .font(code_font())
                    .size(typography::caption())
                    .style(style::text::muted),
                );
            }
            body = body.push(actions);
        }
        container(body).width(Fill).padding([4, 0]).into()
    }
}
impl<'a, Message: Clone + 'a> From<AiMessage<'a, Message>> for Element<'a, Message> {
    fn from(message: AiMessage<'a, Message>) -> Self {
        message.view()
    }
}

/// Complete AI workspace with transcript and prompt composer. Its events are
/// the same ones exposed by the smaller standalone components.
pub struct AiConversation<'a, Message> {
    state: &'a Conversation,
    on_event: Events<'a, Message>,
    title: &'a str,
    model: &'a str,
    output_id: Option<iced::widget::Id>,
    height: Length,
    attachments: bool,
    suggestions: Vec<(&'a str, Message)>,
}
impl<'a, Message: Clone + 'a> AiConversation<'a, Message> {
    pub fn new(state: &'a Conversation, on_event: impl Fn(AiEvent) -> Message + 'a) -> Self {
        Self {
            state,
            on_event: Rc::new(on_event),
            title: "KentOS Asistan",
            model: "Çizim asistanı",
            output_id: None,
            height: Fill,
            attachments: false,
            suggestions: Vec::new(),
        }
    }
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }
    pub fn model(mut self, model: &'a str) -> Self {
        self.model = model;
        self
    }
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
    pub fn output_id(mut self, id: impl Into<iced::widget::Id>) -> Self {
        self.output_id = Some(id.into());
        self
    }
    pub fn attachments(mut self, enabled: bool) -> Self {
        self.attachments = enabled;
        self
    }
    pub fn suggestion(mut self, label: &'a str, message: Message) -> Self {
        self.suggestions.push((label, message));
        self
    }
    fn view(self) -> Element<'a, Message> {
        let phase = self.state.phase();
        let header = row![
            AiActivity::new(phase.active()).size(28.0),
            column![
                text(self.title)
                    .font(typography::ui_strong())
                    .size(typography::body() + 3.0),
                text(self.model)
                    .size(typography::caption())
                    .style(style::text::muted)
            ]
            .spacing(3),
            space::horizontal(),
            tag(phase.label())
        ]
        .spacing(10)
        .align_y(iced::Center);
        let mut messages = Column::new().spacing(22).padding([18, 20]).width(Fill);
        if self.state.messages().len() == 0 {
            messages = messages.push(
                container(
                    column![
                        AiActivity::new(false).size(44.0),
                        text("Birlikte başlayalım.")
                            .font(typography::ui_strong())
                            .size(typography::body() + 8.0),
                        text("Sorunuzu yazın veya bir örnek seçin.")
                            .size(typography::body() + 1.0)
                            .style(style::text::muted)
                    ]
                    .spacing(12),
                )
                .padding([25, 4]),
            );
            for (label, message) in self.suggestions {
                messages = messages.push(
                    button(
                        row![
                            text(label).size(typography::body() + 1.0).width(Fill),
                            icon(Icon::ChevronRight).size(13.0)
                        ]
                        .spacing(10),
                    )
                    .on_press(message)
                    .width(Fill)
                    .padding([12, 14])
                    .style(style::button::secondary),
                );
            }
        }
        for data in self.state.messages() {
            let events = self.on_event.clone();
            messages = messages
                .push(AiMessage::new(data, move |event| events(event)).assistant(self.title));
        }
        let mut output = scrollable(messages)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(5).scroller_width(5),
            ))
            .width(Fill)
            .height(Fill);
        if let Some(id) = self.output_id {
            output = output.id(id);
        }
        let follow = self.on_event.clone();
        let output =
            super::scroll::ScrollFollow::new(output, self.state.following(), move |following| {
                follow(AiEvent::Follow(following))
            });
        let events = self.on_event.clone();
        let composer = AiPrompt::new(&self.state.prompt, move |event| events(event))
            .busy(self.state.pending().is_some())
            .model(self.model)
            .attachments(self.attachments);
        let content = column![
            container(header).width(Fill).padding([14, 18]),
            crate::widget::horizontal_divider(),
            output,
            container(composer).padding([12, 16]).width(Fill)
        ]
        .height(self.height);
        Surface::new(
            container(content)
                .width(Fill)
                .height(self.height)
                .padding(1),
            phase,
            0,
        )
        .into()
    }
}
impl<'a, Message: Clone + 'a> From<AiConversation<'a, Message>> for Element<'a, Message> {
    fn from(conversation: AiConversation<'a, Message>) -> Self {
        conversation.view()
    }
}
