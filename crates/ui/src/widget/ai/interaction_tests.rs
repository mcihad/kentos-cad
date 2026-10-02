//! Real keyboard and pointer events exercise the composed native surfaces.

use std::sync::{Arc, Mutex};

use iced::advanced::widget::{Id, Operation};
use iced::keyboard::{self, key};
use iced::{Element, Event, Point, Rectangle, Size};

use crate::snapshot::{Input, Snapshot};

use super::*;

struct FindText {
    label: &'static str,
    position: Arc<Mutex<Option<Point>>>,
}
impl Operation for FindText {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }
    fn text(&mut self, _: Option<&Id>, bounds: Rectangle, text: &str) {
        if text == self.label {
            *self.position.lock().unwrap() = Some(bounds.center());
        }
    }
}
fn find<Message>(
    snapshot: &mut Snapshot,
    view: Element<'_, Message>,
    label: &'static str,
) -> Point {
    let position = Arc::new(Mutex::new(None));
    snapshot.operate(
        view,
        Box::new(FindText {
            label,
            position: position.clone(),
        }),
    );
    position.lock().unwrap().expect(label)
}
fn enter(modifiers: keyboard::Modifiers) -> Event {
    Event::Keyboard(keyboard::Event::KeyPressed {
        key: keyboard::Key::Named(key::Named::Enter),
        modified_key: keyboard::Key::Named(key::Named::Enter),
        physical_key: key::Physical::Unidentified(key::NativeCode::Unidentified),
        location: keyboard::Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    })
}

#[derive(Default)]
struct AiState {
    conversation: Conversation,
    sent: Vec<AiRequest>,
    stopped: Vec<u64>,
}
fn prompt_view(state: &AiState) -> Element<'_, AiEvent> {
    AiPrompt::new(&state.conversation.prompt, |e| e)
        .busy(state.conversation.pending().is_some())
        .into()
}
fn ai_update(state: &mut AiState, event: AiEvent) {
    match state.conversation.update(event) {
        Some(AiAction::Send(request)) => state.sent.push(request),
        Some(AiAction::Stop(id)) => state.stopped.push(id),
        _ => {}
    }
}

#[test]
fn narrow_prompt_sends_multiline_text_and_stops_without_losing_the_next_draft() {
    let mut snapshot = Snapshot::software(Size::new(420.0, 230.0)).unwrap();
    let mut state = AiState::default();
    snapshot.input(
        &mut state,
        prompt_view,
        &mut ai_update,
        Input::Click(Point::new(40.0, 24.0)),
    );
    snapshot.input(
        &mut state,
        prompt_view,
        &mut ai_update,
        Input::Type("Çizimi incele 🦀".into()),
    );
    snapshot.step(
        &mut state,
        prompt_view,
        &mut ai_update,
        &[enter(keyboard::Modifiers::SHIFT)],
    );
    snapshot.input(
        &mut state,
        prompt_view,
        &mut ai_update,
        Input::Type("Alanları hesapla".into()),
    );
    snapshot.input(
        &mut state,
        prompt_view,
        &mut ai_update,
        Input::Key(key::Named::Enter),
    );
    assert_eq!(state.sent.len(), 1);
    assert_eq!(state.sent[0].prompt, "Çizimi incele 🦀\nAlanları hesapla");
    snapshot.input(
        &mut state,
        prompt_view,
        &mut ai_update,
        Input::Type("Sonraki taslak".into()),
    );
    snapshot.input(
        &mut state,
        prompt_view,
        &mut ai_update,
        Input::Key(key::Named::Enter),
    );
    assert_eq!(state.sent.len(), 1);
    let stop = find(&mut snapshot, prompt_view(&state), "Durdur");
    assert!(stop.x < 420.0 && stop.y < 230.0);
    snapshot.input(&mut state, prompt_view, &mut ai_update, Input::Click(stop));
    assert_eq!(state.stopped, [state.sent[0].id]);
    assert!(
        state
            .conversation
            .prompt
            .content
            .text()
            .contains("Sonraki taslak")
    );
}

struct QuestionState {
    question: Question,
    answers: Vec<Answer>,
}
fn question_view(state: &QuestionState) -> Element<'_, QuestionEvent> {
    AiQuestion::new(&state.question, |event| event).into()
}
fn question_update(state: &mut QuestionState, event: QuestionEvent) {
    if let Some(answer) = state.question.update(event) {
        state.answers.push(answer);
    }
}

#[test]
fn clicking_a_recommended_option_does_not_submit_the_question() {
    let mut snapshot = Snapshot::software(Size::new(420.0, 380.0)).unwrap();
    let mut state = QuestionState {
        question: Question::new(9, "Hangi katman?")
            .option(AnswerOption::new("Parseller", "Seçili katmanı incele.").recommended()),
        answers: Vec::new(),
    };
    snapshot.settle(&mut state, question_view, &mut question_update);
    let option = find(&mut snapshot, question_view(&state), "Parseller");
    snapshot.input(
        &mut state,
        question_view,
        &mut question_update,
        Input::Click(option),
    );
    assert!(state.answers.is_empty());
    assert_eq!(state.question.selected, [0]);
    let submit = find(&mut snapshot, question_view(&state), "Yanıtı gönder");
    snapshot.input(
        &mut state,
        question_view,
        &mut question_update,
        Input::Click(submit),
    );
    assert_eq!(state.answers.len(), 1);
    assert_eq!(state.answers[0].choices, [0]);
    snapshot.input(
        &mut state,
        question_view,
        &mut question_update,
        Input::Click(submit),
    );
    assert_eq!(state.answers.len(), 1);
}

struct ScrollPosition {
    result: Arc<Mutex<(Rectangle, f32, f32)>>,
}
impl Operation for ScrollPosition {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }
    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        content: Rectangle,
        translation: iced::Vector,
        _: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        if id == Some(&Id::new("conversation-output")) {
            *self.result.lock().unwrap() = (
                bounds,
                translation.y,
                (content.height - bounds.height).max(0.0),
            );
        }
    }
}
fn conversation_view(state: &Conversation) -> Element<'_, AiEvent> {
    AiConversation::new(state, |event| event)
        .height(420)
        .output_id("conversation-output")
        .into()
}
fn conversation_update(state: &mut Conversation, event: AiEvent) {
    let _ = state.update(event);
}
fn scroll_position(snapshot: &mut Snapshot, state: &Conversation) -> (Rectangle, f32, f32) {
    let result = Arc::new(Mutex::new((Rectangle::default(), 0.0, 0.0)));
    snapshot.operate(
        conversation_view(state),
        Box::new(ScrollPosition {
            result: result.clone(),
        }),
    );
    *result.lock().unwrap()
}

#[test]
fn growing_content_follows_the_end_and_manual_scrolling_preserves_the_reading_position() {
    let mut snapshot = Snapshot::software(Size::new(600.0, 420.0)).unwrap();
    let mut state = Conversation::default();
    let body = "Parsel alanı ve çevresi incelendi.\n\n".repeat(20);
    state.push(MessageData::new(1, Role::Assistant, &body));
    snapshot.settle(&mut state, conversation_view, &mut conversation_update);
    let (bounds, offset, end) = scroll_position(&mut snapshot, &state);
    assert!(end > 100.0);
    assert!((offset - end).abs() < 1.0);
    assert!(state.following());
    state.push(MessageData::new(2, Role::Assistant, &body));
    snapshot.settle(&mut state, conversation_view, &mut conversation_update);
    let (_, offset, end) = scroll_position(&mut snapshot, &state);
    assert!((offset - end).abs() < 1.0);
    assert!(state.following());

    snapshot.input(
        &mut state,
        conversation_view,
        &mut conversation_update,
        Input::Scroll(bounds.center(), 4.0),
    );
    assert!(!state.following());
    let (_, reading, _) = scroll_position(&mut snapshot, &state);
    state.push(MessageData::new(3, Role::Assistant, &body));
    snapshot.settle(&mut state, conversation_view, &mut conversation_update);
    let (_, offset, _) = scroll_position(&mut snapshot, &state);
    assert!((offset - reading).abs() < 1.0);
    assert!(!state.following());

    snapshot.input(
        &mut state,
        conversation_view,
        &mut conversation_update,
        Input::Scroll(bounds.center(), -1000.0),
    );
    assert!(state.following());
    state.push(MessageData::new(4, Role::Assistant, &body));
    snapshot.settle(&mut state, conversation_view, &mut conversation_update);
    let (_, offset, end) = scroll_position(&mut snapshot, &state);
    assert!((offset - end).abs() < 1.0);
}
