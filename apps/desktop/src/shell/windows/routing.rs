//! Which window an event came from (docs/adr/8060 §3). iced gives every subscription the events of every window;
//! the shell's listeners were written for one window and would take a secondary window's keys as the main window's.
//! Here the main window's events still go to them unchanged, and a secondary window's go to its own router:
//!
//! - a key press no widget took becomes [`Event::Key`] for that window, never a shortcut or a drawing command;
//! - a key a field took stays the field's (typing goes to that window's fields);
//! - its modifier keys and its focus changes do not reach the main window (Shift held there does not turn ortho on);
//! - its close button, moves and resizes are its own; its scale change is not the main window's (the brand's
//!   pictures follow the main window's screen).

use iced::keyboard;
use iced::{Subscription, event, window};

use super::Event;
use crate::app::Message;
use crate::keys::KeyPress;

/// A listener of the shell, written for the main window (`keys::key_event`, `sources::dropped` …).
pub(crate) type Listener = fn(iced::Event, event::Status, window::Id) -> Option<Message>;

/// `listener` for the main window's events; a secondary window's keys to its own router ([`secondary_key`]).
pub(crate) fn keys(main: Option<window::Id>, listener: Listener) -> Subscription<Message> {
    event::listen_with(every_event)
        .with((main, listener))
        .filter_map(|((main, listener), (event, status, id))| {
            if is_main(main, id) {
                listener(event, status, id)
            } else {
                secondary_key(event, status, id)
            }
        })
}

/// `listener` for the main window's events only (a file dropped on the drawing).
pub(crate) fn main_only(main: Option<window::Id>, listener: Listener) -> Subscription<Message> {
    event::listen_with(every_event)
        .with((main, listener))
        .filter_map(|((main, listener), (event, status, id))| {
            is_main(main, id)
                .then(|| listener(event, status, id))
                .flatten()
        })
}

/// The windows' own events, each to the main window's message (as with one window) or to the windows' event.
pub(crate) fn window_events(main: Option<window::Id>) -> Subscription<Message> {
    event::listen_with(window_event)
        .with(main)
        .filter_map(|(main, (id, event))| route(main, id, event))
}

fn is_main(main: Option<window::Id>, id: window::Id) -> bool {
    main.is_none_or(|main| main == id)
}

fn every_event(
    event: iced::Event,
    status: event::Status,
    id: window::Id,
) -> Option<(iced::Event, event::Status, window::Id)> {
    Some((event, status, id))
}

fn window_event(
    event: iced::Event,
    _: event::Status,
    id: window::Id,
) -> Option<(window::Id, window::Event)> {
    match event {
        iced::Event::Window(event) => Some((id, event)),
        _ => None,
    }
}

/// A window event of window `id` as the app's message. The main window's keep the messages they had with one window
/// (`CloseRequested`, `WindowResized`, `DeviceScale`); its moves are not asked for (each would be an update).
pub(crate) fn route(
    main: Option<window::Id>,
    id: window::Id,
    event: window::Event,
) -> Option<Message> {
    let main = is_main(main, id);
    match event {
        window::Event::CloseRequested if main => Some(Message::CloseRequested(id)),
        window::Event::CloseRequested => Some(Message::Windows(Event::CloseRequested(id))),
        window::Event::Resized(size) if main => Some(Message::WindowResized(size)),
        window::Event::Resized(size) => Some(Message::Windows(Event::Resized(id, size))),
        // The brand's pictures are chosen by the main window's screen (kentos_ui's brand.rs).
        window::Event::Rescaled(scale) if main => Some(Message::DeviceScale(scale)),
        window::Event::Moved(at) if !main => Some(Message::Windows(Event::Moved(id, at))),
        window::Event::Closed if !main => Some(Message::Windows(Event::Closed(id))),
        _ => None,
    }
}

/// A secondary window's event as its router's message: a key press no widget took. What a field took (typing, its
/// Esc) stays the field's; modifiers and focus changes stay in that window.
pub(crate) fn secondary_key(
    event: iced::Event,
    status: event::Status,
    id: window::Id,
) -> Option<Message> {
    let iced::Event::Keyboard(keyboard::Event::KeyPressed {
        modified_key,
        physical_key,
        modifiers,
        text,
        repeat,
        ..
    }) = event
    else {
        return None;
    };
    if status == event::Status::Captured {
        return None;
    }
    Some(Message::Windows(Event::Key(
        id,
        KeyPress {
            key: modified_key,
            physical: physical_key,
            modifiers,
            text: text.map(|t| t.to_string()),
            repeat,
        },
    )))
}
