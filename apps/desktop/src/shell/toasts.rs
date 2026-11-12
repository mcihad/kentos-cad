//! The window's notices (KentOS UI's `Toaster`): short messages in the
//! window's lower right corner that close themselves. A save that closes its
//! window says so here, since the window that would have said it is gone
//! (“Sembol kaydedildi”); what went wrong stays in the window, by its field.

use iced::Element;
use kentos_interaction::Level;
use kentos_ui::widget::{Toast, Toaster};

use crate::app::{App, Message};

impl App {
    /// Shows a notice; the same one again counts up instead of stacking.
    pub(crate) fn toast(&mut self, toast: Toast<Message>) {
        self.toasts.push(toast);
    }

    /// A save done: “… kaydedildi.” with where it went.
    pub(crate) fn toast_saved(&mut self, title: impl Into<String>, body: impl Into<String>) {
        let body = body.into();
        let toast = Toast::success(title);
        self.toast(if body.is_empty() {
            toast
        } else {
            toast.body(body)
        });
    }

    /// What a window that closed on its Kaydet did: in the message log as before, and as a notice,
    /// since the window that would have said it is gone. A warning stays longer; an error waits.
    pub(crate) fn announce(&mut self, level: Level, text: impl Into<String>) {
        let text = text.into();
        self.say(level, text.clone());
        // The first sentence is the title, the rest its body: “Tablo kaydedildi” · “Ctrl+Z geri alır.”
        let (title, body) = match text.split_once(". ") {
            Some((first, rest)) => (first.to_owned(), rest.to_owned()),
            None => (
                text.strip_suffix('.').unwrap_or(&text).to_owned(),
                String::new(),
            ),
        };
        let toast = match level {
            Level::Warn => Toast::warning(title),
            Level::Error => Toast::error(title),
            _ => Toast::success(title),
        };
        self.toast(if body.is_empty() {
            toast
        } else {
            toast.body(body)
        });
    }

    /// The window's content with its notices over everything in it (the dialogs too).
    pub(crate) fn with_toasts<'a>(&'a self, content: Element<'a, Message>) -> Element<'a, Message> {
        // Clear of the status bar; over a window, clear of its foot's buttons too (Uygula, Tamam).
        let bottom = if self.dialog.is_some() { 104.0 } else { 36.0 };
        Toaster::new(content, &self.toasts, Message::ToastClosed)
            .padding(iced::Padding::new(8.0).bottom(bottom))
            .into()
    }
}
