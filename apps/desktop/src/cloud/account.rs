//! Signing in and out (docs/adr/0041). The sign-in window takes the server's
//! address, the login and the password; the server's own Turkish message
//! goes into the window. The address is the user's preference
//! (`cloud.server`, docs/adr/0023), checked by kentos-cloud before anything
//! is sent (plain http only to this computer). The password lives in the
//! window only while it is open; the session in the connection's memory only.
//!
//! Signing out leaves an open cloud project: its unsent work goes to the
//! device draft first (leaving.rs), and the drawing stays on screen as a
//! local drawing, as the web leaves it.

use iced::Task;
use iced::task::Handle;
use kentos_cloud::{ApiFailure, Cloud as Client};
use kentos_contracts::Me;
use serde_json::Value;

use crate::app::{App, Dialog, Message, Then};
use crate::cloud::Event;
use crate::document::Source;

/// What waits for the sign-in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    Catalog,
    Upload,
}

/// The sign-in window.
pub struct SignIn {
    pub server: String,
    pub login: String,
    /// In memory while the window is open; gone with it.
    pub password: String,
    /// The server's message, or why the address or the fields are refused.
    pub error: Option<String>,
    /// The server is fixed while a cloud project is open: its changes go there.
    pub server_fixed: bool,
    pub next: Option<Next>,
    /// The request on its way and the connection it uses.
    busy: Option<(u64, Client, Handle)>,
}

impl SignIn {
    pub fn busy(&self) -> bool {
        self.busy.is_some()
    }

    /// The request on its way (tests answer it).
    #[cfg(test)]
    pub(super) fn request(&self) -> Option<u64> {
        self.busy.as_ref().map(|(id, ..)| *id)
    }
}

impl App {
    /// Opens the sign-in window; `next` goes on after a sign-in.
    pub(crate) fn open_sign_in(&mut self, next: Option<Next>) {
        let open_server = self
            .document
            .as_ref()
            .and_then(|d| d.cloud_source())
            .and(self.cloud.client.as_ref())
            .map(|c| c.server().to_owned());
        self.cloud.sign_in = Some(SignIn {
            server: open_server
                .clone()
                .unwrap_or_else(|| self.settings.text("cloud.server")),
            login: String::new(),
            password: String::new(),
            error: None,
            server_fixed: open_server.is_some(),
            next,
            busy: None,
        });
        self.dialog = Some(Dialog::SignIn);
    }

    pub(crate) fn account_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::SignInServer(text) => {
                if let Some(s) = self.cloud.sign_in.as_mut().filter(|s| !s.server_fixed) {
                    s.server = text;
                    s.error = None;
                }
            }
            Event::SignInLogin(text) => {
                if let Some(s) = self.cloud.sign_in.as_mut() {
                    s.login = text;
                    s.error = None;
                }
            }
            Event::SignInPassword(text) => {
                if let Some(s) = self.cloud.sign_in.as_mut() {
                    s.password = text;
                    s.error = None;
                }
            }
            Event::SignInSubmit => return self.submit_sign_in(),
            Event::SignedIn { id, result } => return self.signed_in(id, result),
            Event::SignedOut(Err(failure)) => self.warn(format!(
                "Oturum sunucuda kapatılamadı ({}); bu bilgisayarda kapatıldı, sunucudaki oturum süresi dolunca kendiliğinden kapanır.",
                failure.message
            )),
            _ => {}
        }
        Task::none()
    }

    fn submit_sign_in(&mut self) -> Task<Message> {
        let id = self.cloud.next_id();
        let Some(s) = self.cloud.sign_in.as_mut() else {
            return Task::none();
        };
        if s.busy.is_some() {
            return Task::none();
        }
        if s.login.trim().is_empty() || s.password.is_empty() {
            s.error = Some("Giriş adını ve parolayı yazın.".to_owned());
            return Task::none();
        }
        // The address is checked before anything is sent.
        let client = match Client::new(&s.server) {
            Ok(client) => client,
            Err(failure) => {
                s.error = Some(failure.message.clone());
                return Task::none();
            }
        };
        s.error = None;
        let (task, handle) = Task::perform(client.sign_in(&s.login, &s.password), move |result| {
            crate::cloud::msg(Event::SignedIn { id, result })
        })
        .abortable();
        s.busy = Some((id, client, handle.abort_on_drop()));
        task
    }

    fn signed_in(&mut self, id: u64, result: Result<Me, ApiFailure>) -> Task<Message> {
        let Some(s) = self
            .cloud
            .sign_in
            .as_mut()
            .filter(|s| s.busy.as_ref().is_some_and(|(b, ..)| *b == id))
        else {
            return Task::none();
        };
        let Some((_, client, _)) = s.busy.take() else {
            return Task::none();
        };
        match result {
            Err(failure) => {
                s.error = Some(failure.message.clone());
                Task::none()
            }
            Ok(me) => {
                let next = s.next;
                self.cloud.sign_in = None;
                if self.dialog == Some(Dialog::SignIn) {
                    self.dialog = None;
                }
                // The address that worked is the preference from now on.
                if self.settings.text("cloud.server") != client.server() {
                    let _ = self
                        .settings
                        .choose(&[("cloud.server", Value::from(client.server()))]);
                }
                self.say(
                    kentos_interaction::Level::Success,
                    format!("{} olarak giriş yapıldı.", me.user.display_name),
                );
                // Its id (not a secret) opens its projects on this device without a connection.
                self.keep_account(&me);
                self.cloud.client = Some(client);
                self.cloud.me = Some(me);
                // Work that waited for a session (it had ended, or the project opened offline) goes now.
                self.cloud.link = crate::cloud::copy::Link::Online;
                let online = self.resume();
                let next = match next {
                    Some(Next::Catalog) => self.open_catalog(),
                    Some(Next::Upload) => self.leave(Then::Upload),
                    None => Task::none(),
                };
                Task::batch([online, next])
            }
        }
    }

    /// Signs out: the session is forgotten here at once and ended on the
    /// server; an open cloud project is left, its drawing stays as a local one.
    pub(crate) fn sign_out(&mut self) -> Task<Message> {
        self.detach_cloud();
        self.cloud.me = None;
        let client = self.cloud.client.take();
        self.output("Bulut oturumu kapatıldı.");
        match client {
            Some(client) => Task::perform(client.sign_out(), |result| {
                crate::cloud::msg(Event::SignedOut(result))
            }),
            None => Task::none(),
        }
    }

    /// The drawing is no longer a cloud project: nothing more is sent or
    /// heard; it stays on screen as a local drawing without a file.
    pub(crate) fn detach_cloud(&mut self) {
        // The project's copy is written whole and let go.
        self.close_cloud_project();
        self.cloud.file_conflict = None;
        if let Some(doc) = self.document.as_mut()
            && doc.cloud_source().is_some()
        {
            doc.source = Source::Local;
        }
    }
}
